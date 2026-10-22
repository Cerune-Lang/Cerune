use super::failure::Reporter;
use super::ir::{
    Argument, ArgumentLocation, BinaryOp, CompareOp, Function, Instruction, Module, Type,
};
use super::machine::{EntryKind, Linkage, Opcode, Operand, Scale, Section, r8, r32, r64};
use super::program::{NativeProgram, address, xmm};
use crate::runtime::FailureCode as Failure;

pub fn emit(module: &Module) -> String {
    emit_with_origins(module, false)
}

pub fn emit_with_origins(module: &Module, annotate: bool) -> String {
    build(module, annotate)
        .render_asm()
        .expect("compiler-generated Native Program")
}
pub(super) fn build(module: &Module, annotate: bool) -> NativeProgram {
    let mut reporter = Reporter::new(module.target);
    let mut output = NativeProgram::new(module.target);
    if annotate {
        output.comment("cerune-asm-origins v1: UTF-8 byte ranges, end exclusive");
        output.comment("cerune-origin: synthetic");
    }
    if module.target.is_linux() {
        output.comment("target: x86_64-unknown-linux-gnu");
    }
    super::data::append(&mut output.program, module);
    output.push(EntryKind::Section(Section::Text));
    if module.uses_write {
        super::output::append(&mut output.program);
    }
    if module.uses_strings {
        super::string::append(&mut output.program);
    }
    if let Some(limit) = module.string_heap_limit {
        super::heap::append(&mut output.program, limit);
    }
    if let Some(limit) = module.array_heap_limit {
        super::array::append(&mut output.program, limit);
    }
    if super::input::used(module) || super::arguments::used(module) {
        super::utf8::append(&mut output.program);
    }
    if super::arguments::used(module) {
        super::arguments::append(&mut output, module);
    }
    if super::input::used(module) {
        super::input::append(&mut output);
    }
    for function in &module.functions {
        emit_function(function, module, annotate, &mut reporter, &mut output);
    }

    output.synthetic();
    if annotate {
        output.comment("cerune-origin: synthetic");
    }
    let entry = if super::arguments::used(module) {
        "cerune_program"
    } else {
        "main"
    };

    output.push(EntryKind::Align(4));

    if entry == "main" {
        output.global_label(entry);
    } else {
        output.label(entry);
    }

    output.emit(Opcode::Push, [r64(5)]);

    output.emit(Opcode::Mov64, [r64(4), r64(5)]);

    emit_stack_allocation(module.frame_size, module.target, &mut output);
    if (module.uses_strings || module.uses_write) && !module.target.is_linux() {
        // 固定ターゲットのWindows CRTで、最初のCerune処理より前にLF変換を止めます。

        output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(1)]);
        output.emit(Opcode::Mov32, [Operand::Immediate(32768), r32(2)]);

        let symbol = output.symbol("_setmode", Linkage::External);
        output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        output.emit(Opcode::Cmp32, [Operand::Immediate(-1), r32(0)]);

        let symbol = output.symbol(".Lstdout_ready", Linkage::Local);
        output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

        output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(0)]);

        emit_epilogue(module.frame_size, false, &mut output);

        output.label(".Lstdout_ready");
    }

    assert_eq!(module.instructions.len(), module.origins.len());
    assert_eq!(module.instructions.len(), module.mir_origins.len());
    for (index, instruction) in module.instructions.iter().enumerate() {
        output.source(super::program::Origin {
            source: module.origins[index],
            mir: module.mir_origins[index],
            function: None,
            instruction: index,
        });
        reporter.origin = module.origins[index];
        emit_origin(module.origins[index], "main", index, annotate, &mut output);
        emit_mir_origin(
            module.mir_origins[index],
            "main",
            index,
            annotate,
            &mut output,
        );
        emit_instruction(
            instruction,
            module.frame_size,
            "main",
            module,
            &mut reporter,
            &mut output,
        );
    }

    output.synthetic();
    if annotate {
        output.comment("cerune-origin: synthetic");
    }

    emit_epilogue(module.frame_size, true, &mut output);

    if super::arguments::used(module) {
        super::arguments::append_entry(&mut output);
    }
    if !reporter.data.is_empty() {
        output.push(EntryKind::Section(Section::ReadOnlyData));
        for (name, value, provenance) in reporter.data {
            let symbol = output.symbol(&name, Linkage::Local);
            output
                .program
                .push(EntryKind::Label(symbol), provenance.clone());
            output.program.push(EntryKind::CString(value), provenance);
        }
    }
    output
}

fn emit_function(
    function: &Function,
    module: &Module,
    annotate: bool,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    output.synthetic();
    if annotate {
        output.comment("cerune-origin: synthetic");
    }

    output.push(EntryKind::Align(4));

    output.label(&function_name(function).to_string());

    output.emit(Opcode::Push, [r64(5)]);

    output.emit(Opcode::Mov64, [r64(4), r64(5)]);

    // 大きなフレームでは__chkstkの引数にRAXを使うため、集約戻り値の保存先を退避します。
    // 一時領域は16バイトとし、呼び出し時のスタック整列を保ちます。
    let save_result_pointer = !module.target.is_linux()
        && function.frame_size >= 4096
        && function
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::StoreAggregateReturnPointer { .. }));
    if save_result_pointer {
        output.emit(Opcode::Sub64, [Operand::Immediate(16), r64(4)]);
        output.emit(Opcode::Mov64, [r64(0), address(4, None, 0)]);
    }
    emit_stack_allocation(function.frame_size, module.target, output);
    if save_result_pointer {
        output.emit(
            Opcode::Mov64,
            [
                address(4, None, (function.frame_size).to_owned() as i128),
                r64(0),
            ],
        );
        output.emit(Opcode::Add64, [Operand::Immediate(16), r64(4)]);
    }
    assert_eq!(function.instructions.len(), function.origins.len());
    assert_eq!(function.instructions.len(), function.mir_origins.len());
    for (index, instruction) in function.instructions.iter().enumerate() {
        output.source(super::program::Origin {
            source: function.origins[index],
            mir: function.mir_origins[index],
            function: Some(function.id),
            instruction: index,
        });
        reporter.origin = function.origins[index];
        emit_origin(
            function.origins[index],
            &format!("fn_{}", function.id),
            index,
            annotate,
            output,
        );
        emit_mir_origin(
            function.mir_origins[index],
            &format!("fn_{}", function.id),
            index,
            annotate,
            output,
        );
        emit_instruction(
            instruction,
            function.frame_size,
            &format!("fn_{}", function.id),
            module,
            reporter,
            output,
        );
    }
}

// Windowsのガードページを飛び越さないよう、確保前に各ページを検査します。
// __chkstkは引数レジスタとraxを保持するため、引数を保存する前でも呼び出せます。
fn emit_stack_allocation(frame_size: usize, target: super::Target, output: &mut NativeProgram) {
    if target.is_linux() {
        // ガード領域を飛び越さず、引数レジスタを保持して各ページへ触れます。
        for _ in 0..frame_size / 4096 {
            output.emit(Opcode::Sub64, [Operand::Immediate(4096), r64(4)]);
            output.emit(Opcode::Test8, [Operand::Immediate(0), address(4, None, 0)]);
        }

        output.emit(
            Opcode::Sub64,
            [
                Operand::Immediate((frame_size % 4096).to_owned() as i128),
                r64(4),
            ],
        );

        return;
    }
    if frame_size >= 4096 {
        output.emit(
            Opcode::Mov64,
            [Operand::Immediate((frame_size).to_owned() as i128), r64(0)],
        );

        let symbol = output.symbol("__chkstk", Linkage::External);
        output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        output.emit(Opcode::Sub64, [r64(0), r64(4)]);
    } else {
        output.emit(
            Opcode::Sub64,
            [Operand::Immediate((frame_size).to_owned() as i128), r64(4)],
        );
    }
}

fn function_name(function: &Function) -> String {
    format!(
        "cerune_fn_{}_{}",
        function.name.replace('$', "_"),
        function.id
    )
}

fn emit_epilogue(frame_size: usize, zero_result: bool, output: &mut NativeProgram) {
    if zero_result {
        output.emit(Opcode::Xor32, [r32(0), r32(0)]);
    }

    output.emit(
        Opcode::Add64,
        [Operand::Immediate((frame_size).to_owned() as i128), r64(4)],
    );

    output.emit(Opcode::Pop, [r64(5)]);

    output.emit(Opcode::Ret, []);
}

fn emit_instruction(
    instruction: &Instruction,
    frame_size: usize,
    label_prefix: &str,
    module: &Module,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    if super::array::emit(instruction, label_prefix, module.target, reporter, output) {
        return;
    }
    match instruction {
        Instruction::ArgumentCount => reporter.argument(false, output),
        Instruction::ArgumentGet => reporter.argument(true, output),
        Instruction::ReadLine => reporter.input(output),
        Instruction::Flush => reporter.flush(output),
        Instruction::ArrayAllocate { .. }
        | Instruction::ArrayRangeCheck { .. }
        | Instruction::DynamicArrayAddress { .. }
        | Instruction::ArrayInitAddress { .. }
        | Instruction::ArrayInitialized { .. }
        | Instruction::ArrayLength
        | Instruction::ArrayRetain
        | Instruction::ArrayReleaseOwner
        | Instruction::ArrayFree
        | Instruction::LoadFromPointer { .. }
        | Instruction::CopyFromPointer { .. } => unreachable!(),
        Instruction::Output { kind, newline } => reporter.output(kind, *newline, output),
        Instruction::StringManage { retain } => {
            if module.target.is_linux() {
                output.emit(Opcode::Mov64, [r64(0), r64(7)]);
            } else {
                output.emit(Opcode::Mov64, [r64(0), r64(1)]);
            };
            if *retain {
                let symbol = output.symbol("cerune_string_retain", Linkage::Local);
                output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
            } else {
                let symbol = output.symbol("cerune_string_release", Linkage::Local);
                output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
            };
        }
        Instruction::StringConcat { left_offset, label } => {
            if module.target.is_linux() {
                output.emit(Opcode::Mov64, [r64(0), r64(6)]);
                output.emit(
                    Opcode::Mov64,
                    [address(5, None, (left_offset).to_owned() as i128), r64(7)],
                );
            } else {
                output.emit(Opcode::Mov64, [r64(0), r64(2)]);
                output.emit(
                    Opcode::Mov64,
                    [address(5, None, (left_offset).to_owned() as i128), r64(1)],
                );
            }

            let symbol = output.symbol("cerune_string_concat", Linkage::Local);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Test64, [r64(2), r64(2)]);

            let symbol = output.symbol(
                &format!(".L{label_prefix}_concat_ok_{label}"),
                Linkage::Local,
            );
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cmp64, [Operand::Immediate(1), r64(2)]);

            let symbol = output.symbol(
                &format!(".L{label_prefix}_concat_limit_{label}"),
                Linkage::Local,
            );
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::AllocationSizeOverflow, output);

            output.label(&format!(".L{label_prefix}_concat_limit_{label}"));
            output.emit(Opcode::Cmp64, [Operand::Immediate(2), r64(2)]);

            let symbol = output.symbol(
                &format!(".L{label_prefix}_concat_failed_{label}"),
                Linkage::Local,
            );
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::AllocationLimitExceeded, output);

            output.label(&format!(".L{label_prefix}_concat_failed_{label}"));

            reporter.emit(Failure::AllocationFailed, output);

            output.label(&format!(".L{label_prefix}_concat_ok_{label}"));
        }
        Instruction::CompareU64(op) => {
            output.emit(Opcode::Cmp64, [r64(1), r64(0)]);

            match op {
                CompareOp::Equal => {
                    output.emit(Opcode::SetE, [r8(0)]);
                }
                CompareOp::NotEqual => {
                    output.emit(Opcode::SetNe, [r8(0)]);
                }
                CompareOp::Less => {
                    output.emit(Opcode::SetB, [r8(0)]);
                }
                CompareOp::LessEqual => {
                    output.emit(Opcode::SetBe, [r8(0)]);
                }
                CompareOp::Greater => {
                    output.emit(Opcode::SetA, [r8(0)]);
                }
                CompareOp::GreaterEqual => {
                    output.emit(Opcode::SetAe, [r8(0)]);
                }
            };

            output.emit(Opcode::ZeroExtend8To64, [r8(0), r64(0)]);
        }
        Instruction::LoadStringLength => {
            output.emit(Opcode::Mov64, [address(0, None, 0), r64(0)]);
        }
        Instruction::LoadStringConstant(id) => {
            let symbol = output.symbol(&format!(".Lcerune_string_{id}"), Linkage::Local);
            output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);
        }
        Instruction::CompareString { left_offset, equal } => {
            let left = integer_argument_register(0, module.target);
            let right = integer_argument_register(1, module.target);

            output.emit(Opcode::Mov64, [r64(0), r64(right)]);
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (left_offset).to_owned() as i128),
                    r64(left),
                ],
            );

            let symbol = output.symbol("cerune_string_equal", Linkage::Local);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            if !equal {
                output.emit(Opcode::Xor32, [Operand::Immediate(1), r32(0)]);
            }
        }
        Instruction::ConvertNumeric { conversion, label } => {
            super::conversion::emit(*conversion, *label, label_prefix, reporter, output)
        }
        Instruction::BitNot { mask } => {
            output.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((mask).to_owned() as i128), r64(11)],
            );
            output.emit(Opcode::Xor64, [r64(11), r64(0)]);
        }
        Instruction::IntegerBinary { op, ty, label } => {
            if *ty == crate::types::IntegerType::U64 {
                return super::unsigned::emit_binary(*op, *label, label_prefix, reporter, output);
            }
            use crate::codegen::IntegerBinaryOp;
            let bad = format!(".Lcerune_{label_prefix}_integer_bad_{label}");
            let done = format!(".Lcerune_{label_prefix}_integer_done_{label}");
            match op {
                IntegerBinaryOp::Add
                | IntegerBinaryOp::Subtract
                | IntegerBinaryOp::Multiply
                | IntegerBinaryOp::Divide => unreachable!("u64 operations are emitted above"),
                IntegerBinaryOp::BitAnd => {
                    output.emit(Opcode::And64, [r64(1), r64(0)]);
                }
                IntegerBinaryOp::BitOr => {
                    output.emit(Opcode::Or64, [r64(1), r64(0)]);
                }
                IntegerBinaryOp::BitXor => {
                    output.emit(Opcode::Xor64, [r64(1), r64(0)]);
                }
                IntegerBinaryOp::Remainder => {
                    let divide = format!(".Lcerune_{label_prefix}_integer_rem_{label}");

                    output.emit(Opcode::Test64, [r64(1), r64(1)]);

                    let symbol = output.symbol(&bad, Linkage::Local);
                    output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

                    output.emit(Opcode::Cmp64, [Operand::Immediate(-1), r64(1)]);

                    let symbol = output.symbol(&divide, Linkage::Local);
                    output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

                    output.emit(Opcode::Xor64, [r64(0), r64(0)]);

                    let symbol = output.symbol(&done, Linkage::Local);
                    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

                    output.label(&divide);
                    output.emit(Opcode::SignExtendRax, []);
                    output.emit(Opcode::SignedDiv64, [r64(1)]);
                    output.emit(Opcode::Mov64, [r64(2), r64(0)]);

                    let symbol = output.symbol(&done, Linkage::Local);
                    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

                    output.label(&bad);

                    reporter.emit(Failure::RemainderByZero, output);

                    output.label(&done);
                }
                IntegerBinaryOp::ShiftLeft | IntegerBinaryOp::ShiftRight => {
                    output.emit(Opcode::Test64, [r64(1), r64(1)]);

                    let symbol = output.symbol(&bad, Linkage::Local);
                    output.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

                    output.emit(
                        Opcode::Cmp64,
                        [
                            Operand::Immediate((ty.bit_width()).to_owned() as i128),
                            r64(1),
                        ],
                    );

                    let symbol = output.symbol(&bad, Linkage::Local);
                    output.emit(Opcode::JumpGe, [Operand::Symbol(symbol)]);

                    if *op == IntegerBinaryOp::ShiftLeft {
                        output.emit(
                            Opcode::MovAbs64,
                            [Operand::Immediate((ty.minimum()).to_owned()), r64(11)],
                        );
                        output.emit(Opcode::ShiftArithmeticRight64, [r8(1), r64(11)]);
                        output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

                        let symbol = output.symbol(&format!("{bad}_overflow"), Linkage::Local);
                        output.emit(Opcode::JumpL, [Operand::Symbol(symbol)]);

                        output.emit(
                            Opcode::MovAbs64,
                            [Operand::Immediate((ty.maximum()).to_owned()), r64(11)],
                        );
                        output.emit(Opcode::ShiftRight64, [r8(1), r64(11)]);
                        output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

                        let symbol = output.symbol(&format!("{bad}_overflow"), Linkage::Local);
                        output.emit(Opcode::JumpG, [Operand::Symbol(symbol)]);

                        output.emit(Opcode::ShiftLeft64, [r8(1), r64(0)]);

                        let symbol = output.symbol(&done, Linkage::Local);
                        output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

                        output.label(&format!("{bad}_overflow"));

                        reporter.emit(Failure::IntegerOverflow, output);
                    } else {
                        output.emit(Opcode::ShiftArithmeticRight64, [r8(1), r64(0)]);
                    }

                    let symbol = output.symbol(&done, Linkage::Local);
                    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

                    output.label(&bad);

                    reporter.emit(Failure::InvalidShiftCount, output);

                    output.label(&done);
                }
            }
        }
        Instruction::CheckIntegerRange { ty, label, failure } => {
            let bad = format!(".Lcerune_{label_prefix}_range_bad_{label}");
            let done = format!(".Lcerune_{label_prefix}_range_ok_{label}");

            output.comment(&format!("semantic {}, storage i64", ty.name()));
            output.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((ty.minimum()).to_owned()), r64(11)],
            );
            output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpL, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((ty.maximum()).to_owned()), r64(11)],
            );
            output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::JumpLe, [Operand::Symbol(symbol)]);

            output.label(&bad);

            reporter.emit(*failure, output);

            output.label(&done);
        }
        Instruction::Label { id, name } => {
            output.label(&block_label(label_prefix, *id).to_string());
            output.comment(name.as_ref());
        }

        Instruction::JumpIfZero(label) => {
            output.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = output.symbol(
                &block_label(label_prefix, *label).to_string(),
                Linkage::Local,
            );
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);
        }

        Instruction::Jump(label) => {
            let symbol = output.symbol(
                &block_label(label_prefix, *label).to_string(),
                Linkage::Local,
            );
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);
        }

        Instruction::MovI64ImmediateToRax(value) => {
            output.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((value).to_owned() as i128), r64(0)],
            );
        }

        Instruction::LoadI64FromStack(offset) | Instruction::LoadI64ScratchToRax(offset) => {
            output.emit(
                Opcode::Mov64,
                [address(5, None, (offset).to_owned() as i128), r64(0)],
            );
        }

        Instruction::StoreI64ToStack(offset) => {
            output.emit(
                Opcode::Mov64,
                [r64(0), address(5, None, (offset).to_owned() as i128)],
            );
        }

        Instruction::LoadF32FromStack(offset) | Instruction::LoadF32ScratchToXmm0(offset) => {
            output.emit(
                Opcode::Movss,
                [address(5, None, (offset).to_owned() as i128), xmm(0)],
            );
        }

        Instruction::StoreF32ToStack(offset) => {
            output.emit(
                Opcode::Movss,
                [xmm(0), address(5, None, (offset).to_owned() as i128)],
            );
        }

        Instruction::LoadF64FromStack(offset) | Instruction::LoadF64ScratchToXmm0(offset) => {
            output.emit(
                Opcode::Movsd,
                [address(5, None, (offset).to_owned() as i128), xmm(0)],
            );
        }

        Instruction::StoreF64ToStack(offset) => {
            output.emit(
                Opcode::Movsd,
                [xmm(0), address(5, None, (offset).to_owned() as i128)],
            );
        }

        Instruction::CheckedArrayLoad {
            ty,
            base_offset,
            length,
            label,
        } => {
            let trap = format!(".Lcerune_{label_prefix}_array_oob_{label}");
            let done = format!(".Lcerune_{label_prefix}_array_done_{label}");

            output.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::Cmp64,
                [Operand::Immediate((length).to_owned() as i128), r64(0)],
            );

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpGe, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Neg64, [r64(0)]);

            match ty {
                Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
                    output.emit(
                        Opcode::Mov64,
                        [
                            address(5, Some((0, Scale::Eight)), (base_offset).to_owned() as i128),
                            r64(0),
                        ],
                    );
                }
                Type::F32 => {
                    output.emit(
                        Opcode::Movss,
                        [
                            address(5, Some((0, Scale::Eight)), (base_offset).to_owned() as i128),
                            xmm(0),
                        ],
                    );
                }
                Type::F64 => {
                    output.emit(
                        Opcode::Movsd,
                        [
                            address(5, Some((0, Scale::Eight)), (base_offset).to_owned() as i128),
                            xmm(0),
                        ],
                    );
                }
            }

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&trap);

            reporter.emit(Failure::ArrayIndexOutOfBounds, output);

            output.label(&done);
        }

        Instruction::CheckedArrayCopy {
            base_offset,
            length,
            element_slots,
            destination_offset,
            label,
        } => {
            let trap = format!(".Lcerune_{label_prefix}_array_oob_{label}");
            let done = format!(".Lcerune_{label_prefix}_array_done_{label}");

            output.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::Cmp64,
                [Operand::Immediate((length).to_owned() as i128), r64(0)],
            );

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpGe, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::SignedMul64,
                [
                    Operand::Immediate(-((element_slots).to_owned() as i128)),
                    r64(0),
                ],
            );

            for slot in 0..*element_slots {
                let source = base_offset - 8 * slot as isize;
                let destination = destination_offset - 8 * slot as isize;

                output.emit(
                    Opcode::Mov64,
                    [
                        address(5, Some((0, Scale::Eight)), (source).to_owned() as i128),
                        r64(1),
                    ],
                );

                output.emit(
                    Opcode::Mov64,
                    [r64(1), address(5, None, (destination).to_owned() as i128)],
                );
            }

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&trap);

            reporter.emit(Failure::ArrayIndexOutOfBounds, output);

            output.label(&done);
        }

        Instruction::CheckedArrayAddress {
            base_offset,
            base_is_pointer,
            length,
            element_slots,
            destination_offset,
            label,
        } => {
            let trap = format!(".Lcerune_{label_prefix}_array_oob_{label}");
            let done = format!(".Lcerune_{label_prefix}_array_done_{label}");

            output.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::Cmp64,
                [Operand::Immediate((length).to_owned() as i128), r64(0)],
            );

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpGe, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::SignedMul64,
                [
                    Operand::Immediate(-((element_slots).to_owned() as i128)),
                    r64(0),
                ],
            );

            if *base_is_pointer {
                output.emit(
                    Opcode::Mov64,
                    [address(5, None, (base_offset).to_owned() as i128), r64(1)],
                );

                output.emit(
                    Opcode::Lea64,
                    [address(1, Some((0, Scale::Eight)), 0), r64(1)],
                );
            } else {
                output.emit(
                    Opcode::Lea64,
                    [
                        address(5, Some((0, Scale::Eight)), (base_offset).to_owned() as i128),
                        r64(1),
                    ],
                );
            }

            output.emit(
                Opcode::Mov64,
                [
                    r64(1),
                    address(5, None, (destination_offset).to_owned() as i128),
                ],
            );

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&trap);

            reporter.emit(Failure::ArrayIndexOutOfBounds, output);

            output.label(&done);
        }

        Instruction::StoreI64ToPointer(pointer_offset) => {
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(1),
                ],
            );

            output.emit(Opcode::Mov64, [r64(0), address(1, None, 0)]);
        }
        Instruction::StoreF32ToPointer(pointer_offset) => {
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(1),
                ],
            );

            output.emit(Opcode::Movss, [xmm(0), address(1, None, 0)]);
        }
        Instruction::StoreF64ToPointer(pointer_offset) => {
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(1),
                ],
            );

            output.emit(Opcode::Movsd, [xmm(0), address(1, None, 0)]);
        }
        Instruction::CopyToPointer {
            source_offset,
            slots,
            pointer_offset,
        } => {
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(11),
                ],
            );

            for slot in 0..*slots {
                let source = source_offset - 8 * slot as isize;
                let destination = -8 * slot as isize;

                output.emit(
                    Opcode::Mov64,
                    [address(5, None, (source).to_owned() as i128), r64(10)],
                );

                output.emit(
                    Opcode::Mov64,
                    [r64(10), address(11, None, (destination).to_owned() as i128)],
                );
            }
        }

        Instruction::StoreParameter {
            location,
            ty,
            offset,
        } => {
            emit_store_parameter(*location, *ty, *offset, module.target, output);
        }

        Instruction::StoreAggregateParameter {
            location,
            slots,
            destination_offset,
        } => {
            let register = match location {
                ArgumentLocation::Register(index) => {
                    integer_argument_register(*index, module.target)
                }
                ArgumentLocation::Stack(offset) => {
                    output.emit(
                        Opcode::Mov64,
                        [address(5, None, (offset + 16).to_owned() as i128), r64(11)],
                    );

                    11
                }
            };
            for slot in 0..*slots {
                let source = -8 * slot as isize;
                let destination = destination_offset - 8 * slot as isize;

                output.emit(
                    Opcode::Mov64,
                    [
                        address(register, None, (source).to_owned() as i128),
                        r64(10),
                    ],
                );

                output.emit(
                    Opcode::Mov64,
                    [r64(10), address(5, None, (destination).to_owned() as i128)],
                );
            }
        }

        Instruction::StoreAggregateReturnPointer { offset } => {
            output.emit(
                Opcode::Mov64,
                [r64(0), address(5, None, (offset).to_owned() as i128)],
            );
        }

        Instruction::CopyToAggregateReturn {
            source_offset,
            slots,
            pointer_offset,
        } => {
            output.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(11),
                ],
            );

            for slot in 0..*slots {
                let source = source_offset - 8 * slot as isize;
                let destination = -8 * slot as isize;

                output.emit(
                    Opcode::Mov64,
                    [address(5, None, (source).to_owned() as i128), r64(10)],
                );

                output.emit(
                    Opcode::Mov64,
                    [r64(10), address(11, None, (destination).to_owned() as i128)],
                );
            }
        }

        Instruction::Call {
            function_id,
            arguments,
            aggregate_result_offset,
        } => {
            let mut locations = super::abi::Arguments::new(module.target);
            for argument in arguments {
                let location = locations.next(matches!(
                    argument,
                    Argument::Scalar {
                        ty: Type::F32 | Type::F64,
                        ..
                    }
                ));
                match argument {
                    Argument::Scalar { ty, offset } => {
                        emit_load_argument(location, *ty, *offset, module.target, output)
                    }
                    Argument::Aggregate { offset } => match location {
                        ArgumentLocation::Register(index) => {
                            let register = integer_argument_register(index, module.target);

                            output.emit(
                                Opcode::Lea64,
                                [address(5, None, (offset).to_owned() as i128), r64(register)],
                            );
                        }
                        ArgumentLocation::Stack(destination) => {
                            output.emit(
                                Opcode::Lea64,
                                [address(5, None, (offset).to_owned() as i128), r64(10)],
                            );
                            output.emit(
                                Opcode::Mov64,
                                [r64(10), address(4, None, (destination).to_owned() as i128)],
                            );
                        }
                    },
                }
            }
            if let Some(offset) = aggregate_result_offset {
                output.emit(
                    Opcode::Lea64,
                    [address(5, None, (offset).to_owned() as i128), r64(0)],
                );
            }

            let symbol = output.symbol(
                &function_name(&module.functions[*function_id]).to_string(),
                Linkage::Local,
            );
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
        }

        Instruction::ObserveOnly => {}
        Instruction::Unreachable => {
            output.emit(Opcode::Undefined, []);
        }
        Instruction::Return => emit_epilogue(frame_size, false, output),

        Instruction::LoadF32Constant(id) => {
            let symbol = output.symbol(&format!(".Lcerune_f32_{id}"), Linkage::Local);
            output.emit(Opcode::Movss, [Operand::RipRelative(symbol), xmm(0)]);
        }

        Instruction::LoadF64Constant(id) => {
            let symbol = output.symbol(&format!(".Lcerune_f64_{id}"), Linkage::Local);
            output.emit(Opcode::Movsd, [Operand::RipRelative(symbol), xmm(0)]);
        }

        Instruction::NegI64 => {
            output.emit(Opcode::Neg64, [r64(0)]);
        }

        Instruction::TrapIfOverflow(label) => {
            let done = format!(".Lcerune_{label_prefix}_integer_ok_{label}");

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::JumpNo, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::IntegerOverflow, output);

            output.label(&done);
        }

        Instruction::NotBool => {
            output.emit(Opcode::Xor64, [Operand::Immediate(1), r64(0)]);
        }

        Instruction::NegF32 => {
            let symbol = output.symbol(".Lcerune_sign_f32", Linkage::Local);
            output.emit(Opcode::Xorps, [Operand::RipRelative(symbol), xmm(0)]);
        }

        Instruction::NegF64 => {
            let symbol = output.symbol(".Lcerune_sign_f64", Linkage::Local);
            output.emit(Opcode::Xorpd, [Operand::RipRelative(symbol), xmm(0)]);
        }

        Instruction::MoveRaxToRcx => {
            output.emit(Opcode::Mov64, [r64(0), r64(1)]);
        }

        Instruction::I64Binary(op) => {
            match op {
                BinaryOp::Add => {
                    output.emit(Opcode::Add64, [r64(1), r64(0)]);
                }
                BinaryOp::Subtract => {
                    output.emit(Opcode::Sub64, [r64(1), r64(0)]);
                }
                BinaryOp::Multiply => {
                    output.emit(Opcode::SignedMul64, [r64(1), r64(0)]);
                }
                BinaryOp::Divide => {
                    unreachable!("i64 division is lowered to sign-extend + divide")
                }
            };
        }

        Instruction::CompareI64(op) => {
            output.emit(Opcode::Cmp64, [r64(1), r64(0)]);

            output.emit(compare_setcc(*op), [r8(0)]);

            output.emit(Opcode::ZeroExtend8To64, [r8(0), r64(0)]);
        }

        Instruction::SignExtendRax => {
            output.emit(Opcode::SignExtendRax, []);
        }

        Instruction::TrapIfInvalidI64Division(label) => {
            let trap = format!(".Lcerune_{label_prefix}_division_trap_{label}");
            let done = format!(".Lcerune_{label_prefix}_division_ok_{label}");

            output.emit(Opcode::Test64, [r64(1), r64(1)]);

            let symbol = output.symbol(&trap, Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cmp64, [Operand::Immediate(-1), r64(1)]);

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            output.emit(
                Opcode::MovAbs64,
                [Operand::Immediate(-9223372036854775808), r64(2)],
            );

            output.emit(Opcode::Cmp64, [r64(2), r64(0)]);

            let symbol = output.symbol(&done, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::DivisionOverflow, output);

            output.label(&trap);

            reporter.emit(Failure::DivisionByZero, output);

            output.label(&done);
        }

        Instruction::DivideRaxByRcx => {
            output.emit(Opcode::SignedDiv64, [r64(1)]);
        }

        Instruction::CopyXmm0ToXmm1F32 => {
            output.emit(Opcode::Movaps, [xmm(0), xmm(1)]);
        }

        Instruction::CopyXmm0ToXmm1F64 => {
            output.emit(Opcode::Movapd, [xmm(0), xmm(1)]);
        }

        Instruction::F32Binary(op) => {
            match op {
                BinaryOp::Add => {
                    output.emit(Opcode::Addss, [xmm(1), xmm(0)]);
                }
                BinaryOp::Subtract => {
                    output.emit(Opcode::Subss, [xmm(1), xmm(0)]);
                }
                BinaryOp::Multiply => {
                    output.emit(Opcode::Mulss, [xmm(1), xmm(0)]);
                }
                BinaryOp::Divide => {
                    output.emit(Opcode::Divss, [xmm(1), xmm(0)]);
                }
            };
        }

        Instruction::F64Binary(op) => {
            match op {
                BinaryOp::Add => {
                    output.emit(Opcode::Addsd, [xmm(1), xmm(0)]);
                }
                BinaryOp::Subtract => {
                    output.emit(Opcode::Subsd, [xmm(1), xmm(0)]);
                }
                BinaryOp::Multiply => {
                    output.emit(Opcode::Mulsd, [xmm(1), xmm(0)]);
                }
                BinaryOp::Divide => {
                    output.emit(Opcode::Divsd, [xmm(1), xmm(0)]);
                }
            };
        }

        Instruction::CompareF32(op) => {
            output.emit(Opcode::Ucomiss, [xmm(1), xmm(0)]);

            emit_float_comparison(*op, output);
        }

        Instruction::CompareF64(op) => {
            output.emit(Opcode::Ucomisd, [xmm(1), xmm(0)]);

            emit_float_comparison(*op, output);
        }
    }
}

fn block_label(prefix: &str, id: usize) -> String {
    if prefix == "main" {
        format!(".Lcerune_block_{id}")
    } else {
        format!(".Lcerune_{prefix}_block_{id}")
    }
}

fn emit_store_parameter(
    location: ArgumentLocation,
    ty: Type,
    offset: isize,
    target: super::Target,
    output: &mut NativeProgram,
) {
    let ArgumentLocation::Register(index) = location else {
        let ArgumentLocation::Stack(source) = location else {
            unreachable!()
        };
        emit_stack_argument_copy(
            ty,
            address(5, None, (source + 16) as i128),
            address(5, None, offset as i128),
            output,
        );
        return;
    };
    match ty {
        Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
            let register = integer_argument_register(index, target);

            output.emit(
                Opcode::Mov64,
                [r64(register), address(5, None, (offset).to_owned() as i128)],
            );
        }
        Type::F32 => {
            output.emit(
                Opcode::Movss,
                [xmm(index), address(5, None, (offset).to_owned() as i128)],
            );
        }
        Type::F64 => {
            output.emit(
                Opcode::Movsd,
                [xmm(index), address(5, None, (offset).to_owned() as i128)],
            );
        }
    }
}

fn emit_load_argument(
    location: ArgumentLocation,
    ty: Type,
    offset: isize,
    target: super::Target,
    output: &mut NativeProgram,
) {
    let ArgumentLocation::Register(index) = location else {
        let ArgumentLocation::Stack(destination) = location else {
            unreachable!()
        };
        emit_stack_argument_copy(
            ty,
            address(5, None, offset as i128),
            address(4, None, destination as i128),
            output,
        );
        return;
    };
    match ty {
        Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
            let register = integer_argument_register(index, target);

            output.emit(
                Opcode::Mov64,
                [address(5, None, (offset).to_owned() as i128), r64(register)],
            );
        }
        Type::F32 => {
            output.emit(
                Opcode::Movss,
                [address(5, None, (offset).to_owned() as i128), xmm(index)],
            );
        }
        Type::F64 => {
            output.emit(
                Opcode::Movsd,
                [address(5, None, (offset).to_owned() as i128), xmm(index)],
            );
        }
    }
}

// XMMや引数レジスタを壊さず、f32も数値変換せずにビットをコピーします。
fn emit_stack_argument_copy(
    ty: Type,
    source: Operand,
    destination: Operand,
    output: &mut NativeProgram,
) {
    let (instruction, register) = if ty == Type::F32 {
        (Opcode::Mov32, r32(10))
    } else {
        (Opcode::Mov64, r64(10))
    };
    output.emit(instruction, [source, register.clone()]);
    output.emit(instruction, [register, destination]);
}

fn integer_argument_register(index: usize, target: super::Target) -> u8 {
    if target.is_linux() {
        [7, 6, 2, 1, 8, 9][index]
    } else {
        [1, 2, 8, 9][index]
    }
}

fn compare_setcc(op: CompareOp) -> Opcode {
    match op {
        CompareOp::Equal => Opcode::SetE,
        CompareOp::NotEqual => Opcode::SetNe,
        CompareOp::Less => Opcode::SetL,
        CompareOp::LessEqual => Opcode::SetLe,
        CompareOp::Greater => Opcode::SetG,
        CompareOp::GreaterEqual => Opcode::SetGe,
    }
}

fn emit_float_comparison(op: CompareOp, output: &mut NativeProgram) {
    match op {
        CompareOp::Equal => {
            output.emit(Opcode::SetE, [r8(0)]);

            output.emit(Opcode::SetNp, [r8(1)]);

            output.emit(Opcode::And8, [r8(1), r8(0)]);
        }
        CompareOp::NotEqual => {
            output.emit(Opcode::SetNe, [r8(0)]);

            output.emit(Opcode::SetP, [r8(1)]);

            output.emit(Opcode::Or8, [r8(1), r8(0)]);
        }
        CompareOp::Less => {
            output.emit(Opcode::SetB, [r8(0)]);

            output.emit(Opcode::SetNp, [r8(1)]);

            output.emit(Opcode::And8, [r8(1), r8(0)]);
        }
        CompareOp::LessEqual => {
            output.emit(Opcode::SetBe, [r8(0)]);

            output.emit(Opcode::SetNp, [r8(1)]);

            output.emit(Opcode::And8, [r8(1), r8(0)]);
        }
        CompareOp::Greater => {
            output.emit(Opcode::SetA, [r8(0)]);
        }
        CompareOp::GreaterEqual => {
            output.emit(Opcode::SetAe, [r8(0)]);
        }
    }

    output.emit(Opcode::ZeroExtend8To64, [r8(0), r64(0)]);
}

fn emit_mir_origin(
    origin: Option<super::ir::MirOrigin>,
    prefix: &str,
    index: usize,
    annotate: bool,
    output: &mut NativeProgram,
) {
    if !annotate {
        return;
    }
    if let Some(o) = origin {
        let item = o
            .instruction
            .map_or_else(|| "block".into(), |i| format!("i{}", i.0));
        let reason = match o.origin {
            crate::mir::Origin::Source(_) => "source",
            crate::mir::Origin::Derived { reason, .. }
            | crate::mir::Origin::Synthetic { reason } => reason,
        };

        output.comment(&format!(
            "cerune-mir: v1 {prefix} bb{} {item} -> lir {index} ({reason})",
            o.block.0
        ));
        output.label(&format!(
            "cerune_origin_mir_{prefix}_bb{}_{item}_lir{index}",
            o.block.0
        ));
    } else {
        output.comment("cerune-mir: v1 synthetic ABI setup/exit");
    }
}

fn emit_origin(
    origin: super::ir::Origin,
    prefix: &str,
    index: usize,
    annotate: bool,
    output: &mut NativeProgram,
) {
    if !annotate {
        return;
    }
    match origin {
        super::ir::Origin::Synthetic => {
            output.comment("cerune-origin: synthetic");
        }
        super::ir::Origin::Source { node_id, span } => {
            output.comment(&format!(
                "cerune-origin: #{}{} bytes {}..{}",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ));
            output.label(&format!("cerune_origin_n{}_{prefix}_{index}", node_id.0));
        }
    }
}
