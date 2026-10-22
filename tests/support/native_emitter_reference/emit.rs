use super::failure::Reporter;
use super::ir::{
    Argument, ArgumentLocation, BinaryOp, CompareOp, Function, Instruction, Module, Type,
};
use crate::runtime::FailureCode as Failure;

pub fn emit(module: &Module) -> String {
    emit_with_origins(module, false)
}

pub fn emit_with_origins(module: &Module, annotate: bool) -> String {
    let mut reporter = Reporter::new(module.target);
    let mut output = super::data::render(module);
    if annotate {
        output.insert_str(0, "# cerune-asm-origins v1: UTF-8 byte ranges, end exclusive\n# cerune-origin: synthetic\n");
    }

    output.push_str("\n.text\n");
    if module.uses_write {
        output.push_str(&super::output::support(module.target.is_linux()));
    }
    if module.uses_strings {
        output.push_str(&super::machine::render_support(
            module.target,
            super::string::append,
        ));
    }

    if let Some(limit) = module.string_heap_limit {
        output.push_str(&super::heap::support(limit, module.target.is_linux()));
    }

    if let Some(limit) = module.array_heap_limit {
        output.push_str(&super::array::support(limit, module.target.is_linux()));
    }

    if super::input::used(module) || super::arguments::used(module) {
        output.push_str(&super::machine::render_support(
            module.target,
            super::utf8::append,
        ));
    }
    if super::arguments::used(module) {
        output.push_str(&super::arguments::support(module));
    }
    if super::input::used(module) {
        output.push_str(&super::input::support(module.target.is_linux()));
    }

    for function in &module.functions {
        emit_function(function, module, annotate, &mut reporter, &mut output);
        output.push('\n');
    }

    if annotate {
        output.push_str("# cerune-origin: synthetic\n");
    }
    let entry = if super::arguments::used(module) {
        "cerune_program"
    } else {
        "main"
    };
    if entry == "main" {
        output.push_str(".globl main\n");
    }

    output.push_str(".p2align 4\n");

    output.push_str(&format!("{entry}:\n"));

    output.push_str("  pushq %rbp\n");

    output.push_str("  movq %rsp, %rbp\n");

    emit_stack_allocation(module.frame_size, module.target, &mut output);
    if (module.uses_strings || module.uses_write) && !module.target.is_linux() {
        // 固定ターゲットのWindows CRTで、最初のCerune処理より前にLF変換を止めます。
        output.push_str("  movl $1, %ecx\n  movl $32768, %edx\n  callq _setmode\n  cmpl $-1, %eax\n  jne .Lstdout_ready\n  movl $1, %eax\n");
        emit_epilogue(module.frame_size, false, &mut output);
        output.push_str(".Lstdout_ready:\n");
    }

    assert_eq!(module.instructions.len(), module.origins.len());
    assert_eq!(module.instructions.len(), module.mir_origins.len());
    for (index, instruction) in module.instructions.iter().enumerate() {
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

    if annotate {
        output.push_str("# cerune-origin: synthetic\n");
    }

    emit_epilogue(module.frame_size, true, &mut output);

    if super::arguments::used(module) {
        output.push_str(&super::arguments::entry(module.target.is_linux()));
    }
    if !reporter.data.is_empty() {
        output.push_str(if module.target.is_linux() {
            "\n.section .rodata\n"
        } else {
            "\n.section .rdata,\"dr\"\n"
        });
        output.push_str(&reporter.data);
    }

    if module.target.is_linux() {
        output.push_str("\n.section .note.GNU-stack,\"\",@progbits\n");
    }
    output
}

fn emit_function(
    function: &Function,
    module: &Module,
    annotate: bool,
    reporter: &mut Reporter,
    output: &mut String,
) {
    if annotate {
        output.push_str("# cerune-origin: synthetic\n");
    }
    output.push_str(".p2align 4\n");
    output.push_str(&format!("{}:\n", function_name(function)));
    output.push_str("  pushq %rbp\n");
    output.push_str("  movq %rsp, %rbp\n");
    // 大きなフレームでは__chkstkの引数にRAXを使うため、集約戻り値の保存先を退避します。
    // 一時領域は16バイトとし、呼び出し時のスタック整列を保ちます。
    let save_result_pointer = !module.target.is_linux()
        && function.frame_size >= 4096
        && function
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::StoreAggregateReturnPointer { .. }));
    if save_result_pointer {
        output.push_str("  subq $16, %rsp\n  movq %rax, (%rsp)\n");
    }
    emit_stack_allocation(function.frame_size, module.target, output);
    if save_result_pointer {
        output.push_str(&format!(
            "  movq {}(%rsp), %rax\n  addq $16, %rsp\n",
            function.frame_size
        ));
    }
    assert_eq!(function.instructions.len(), function.origins.len());
    assert_eq!(function.instructions.len(), function.mir_origins.len());
    for (index, instruction) in function.instructions.iter().enumerate() {
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
fn emit_stack_allocation(frame_size: usize, target: super::Target, output: &mut String) {
    if target.is_linux() {
        // ガード領域を飛び越さず、引数レジスタを保持して各ページへ触れます。
        for _ in 0..frame_size / 4096 {
            output.push_str("  subq $4096, %rsp\n  testb $0, (%rsp)\n");
        }
        output.push_str(&format!("  subq ${}, %rsp\n", frame_size % 4096));
        return;
    }
    if frame_size >= 4096 {
        output.push_str(&format!(
            "  movq ${frame_size}, %rax\n  callq __chkstk\n  subq %rax, %rsp\n"
        ));
    } else {
        output.push_str(&format!("  subq ${frame_size}, %rsp\n"));
    }
}

fn function_name(function: &Function) -> String {
    format!(
        "cerune_fn_{}_{}",
        function.name.replace('$', "_"),
        function.id
    )
}

fn emit_epilogue(frame_size: usize, zero_result: bool, output: &mut String) {
    if zero_result {
        output.push_str("  xorl %eax, %eax\n");
    }
    output.push_str(&format!("  addq ${frame_size}, %rsp\n"));
    output.push_str("  popq %rbp\n");
    output.push_str("  retq\n");
}

fn emit_instruction(
    instruction: &Instruction,
    frame_size: usize,
    label_prefix: &str,
    module: &Module,
    reporter: &mut Reporter,
    output: &mut String,
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
            output.push_str(if module.target.is_linux() {
                "  movq %rax, %rdi\n"
            } else {
                "  movq %rax, %rcx\n"
            });
            output.push_str(if *retain {
                "  callq cerune_string_retain\n"
            } else {
                "  callq cerune_string_release\n"
            });
        }
        Instruction::StringConcat { left_offset, label } => {
            if module.target.is_linux() {
                output.push_str(&format!(
                    "  movq %rax, %rsi\n  movq {left_offset}(%rbp), %rdi\n"
                ));
            } else {
                output.push_str(&format!(
                    "  movq %rax, %rdx\n  movq {left_offset}(%rbp), %rcx\n"
                ));
            }
            output.push_str(&format!("  callq cerune_string_concat\n  testq %rdx, %rdx\n  je .L{label_prefix}_concat_ok_{label}\n  cmpq $1, %rdx\n  jne .L{label_prefix}_concat_limit_{label}\n"));
            reporter.emit(Failure::AllocationSizeOverflow, output);
            output.push_str(&format!(".L{label_prefix}_concat_limit_{label}:\n  cmpq $2, %rdx\n  jne .L{label_prefix}_concat_failed_{label}\n"));
            reporter.emit(Failure::AllocationLimitExceeded, output);
            output.push_str(&format!(".L{label_prefix}_concat_failed_{label}:\n"));
            reporter.emit(Failure::AllocationFailed, output);
            output.push_str(&format!(".L{label_prefix}_concat_ok_{label}:\n"));
        }
        Instruction::CompareU64(op) => {
            output.push_str("  cmpq %rcx, %rax\n");
            output.push_str(match op {
                CompareOp::Equal => "  sete %al\n",
                CompareOp::NotEqual => "  setne %al\n",
                CompareOp::Less => "  setb %al\n",
                CompareOp::LessEqual => "  setbe %al\n",
                CompareOp::Greater => "  seta %al\n",
                CompareOp::GreaterEqual => "  setae %al\n",
            });
            output.push_str("  movzbq %al, %rax\n");
        }
        Instruction::LoadStringLength => output.push_str("  movq (%rax), %rax\n"),
        Instruction::LoadStringConstant(id) => {
            output.push_str(&format!("  leaq .Lcerune_string_{id}(%rip), %rax\n"))
        }
        Instruction::CompareString { left_offset, equal } => {
            let left = integer_argument_register(0, module.target);
            let right = integer_argument_register(1, module.target);
            output.push_str(&format!("  movq %rax, {right}\n  movq {left_offset}(%rbp), {left}\n  callq cerune_string_equal\n"));
            if !equal {
                output.push_str("  xorl $1, %eax\n");
            }
        }
        Instruction::ConvertNumeric { conversion, label } => {
            super::conversion::emit(*conversion, *label, label_prefix, reporter, output)
        }
        Instruction::BitNot { mask } => {
            output.push_str(&format!("  movabsq ${mask}, %r11\n  xorq %r11, %rax\n"));
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
                IntegerBinaryOp::BitAnd => output.push_str("  andq %rcx, %rax\n"),
                IntegerBinaryOp::BitOr => output.push_str("  orq %rcx, %rax\n"),
                IntegerBinaryOp::BitXor => output.push_str("  xorq %rcx, %rax\n"),
                IntegerBinaryOp::Remainder => {
                    let divide = format!(".Lcerune_{label_prefix}_integer_rem_{label}");
                    output.push_str(&format!("  testq %rcx, %rcx\n  je {bad}\n  cmpq $-1, %rcx\n  jne {divide}\n  xorq %rax, %rax\n  jmp {done}\n{divide}:\n  cqto\n  idivq %rcx\n  movq %rdx, %rax\n  jmp {done}\n{bad}:\n"));
                    reporter.emit(Failure::RemainderByZero, output);
                    output.push_str(&format!("{done}:\n"));
                }
                IntegerBinaryOp::ShiftLeft | IntegerBinaryOp::ShiftRight => {
                    output.push_str(&format!(
                        "  testq %rcx, %rcx\n  js {bad}\n  cmpq ${}, %rcx\n  jge {bad}\n",
                        ty.bit_width()
                    ));
                    if *op == IntegerBinaryOp::ShiftLeft {
                        output.push_str(&format!("  movabsq ${}, %r11\n  sarq %cl, %r11\n  cmpq %r11, %rax\n  jl {bad}_overflow\n  movabsq ${}, %r11\n  shrq %cl, %r11\n  cmpq %r11, %rax\n  jg {bad}_overflow\n  shlq %cl, %rax\n  jmp {done}\n{bad}_overflow:\n", ty.minimum(), ty.maximum()));
                        reporter.emit(Failure::IntegerOverflow, output);
                    } else {
                        output.push_str("  sarq %cl, %rax\n");
                    }
                    output.push_str(&format!("  jmp {done}\n{bad}:\n"));
                    reporter.emit(Failure::InvalidShiftCount, output);
                    output.push_str(&format!("{done}:\n"));
                }
            }
        }
        Instruction::CheckIntegerRange { ty, label, failure } => {
            let bad = format!(".Lcerune_{label_prefix}_range_bad_{label}");
            let done = format!(".Lcerune_{label_prefix}_range_ok_{label}");
            output.push_str(&format!("  # semantic {}, storage i64\n  movabsq ${}, %r11\n  cmpq %r11, %rax\n  jl {bad}\n  movabsq ${}, %r11\n  cmpq %r11, %rax\n  jle {done}\n{bad}:\n", ty.name(), ty.minimum(), ty.maximum()));
            reporter.emit(*failure, output);
            output.push_str(&format!("{done}:\n"));
        }
        Instruction::Label { id, name } => {
            output.push_str(&format!("{}: # {name}\n", block_label(label_prefix, *id)));
        }

        Instruction::JumpIfZero(label) => {
            output.push_str("  testq %rax, %rax\n");
            output.push_str(&format!("  je {}\n", block_label(label_prefix, *label)));
        }

        Instruction::Jump(label) => {
            output.push_str(&format!("  jmp {}\n", block_label(label_prefix, *label)));
        }

        Instruction::MovI64ImmediateToRax(value) => {
            output.push_str(&format!("  movabsq ${value}, %rax\n"));
        }

        Instruction::LoadI64FromStack(offset) | Instruction::LoadI64ScratchToRax(offset) => {
            output.push_str(&format!("  movq {offset}(%rbp), %rax\n"));
        }

        Instruction::StoreI64ToStack(offset) => {
            output.push_str(&format!("  movq %rax, {offset}(%rbp)\n"));
        }

        Instruction::LoadF32FromStack(offset) | Instruction::LoadF32ScratchToXmm0(offset) => {
            output.push_str(&format!("  movss {offset}(%rbp), %xmm0\n"));
        }

        Instruction::StoreF32ToStack(offset) => {
            output.push_str(&format!("  movss %xmm0, {offset}(%rbp)\n"));
        }

        Instruction::LoadF64FromStack(offset) | Instruction::LoadF64ScratchToXmm0(offset) => {
            output.push_str(&format!("  movsd {offset}(%rbp), %xmm0\n"));
        }

        Instruction::StoreF64ToStack(offset) => {
            output.push_str(&format!("  movsd %xmm0, {offset}(%rbp)\n"));
        }

        Instruction::CheckedArrayLoad {
            ty,
            base_offset,
            length,
            label,
        } => {
            let trap = format!(".Lcerune_{label_prefix}_array_oob_{label}");
            let done = format!(".Lcerune_{label_prefix}_array_done_{label}");
            output.push_str("  testq %rax, %rax\n");
            output.push_str(&format!("  js {trap}\n"));
            output.push_str(&format!("  cmpq ${length}, %rax\n"));
            output.push_str(&format!("  jge {trap}\n"));
            output.push_str("  negq %rax\n");
            match ty {
                Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
                    output.push_str(&format!("  movq {base_offset}(%rbp,%rax,8), %rax\n"));
                }
                Type::F32 => {
                    output.push_str(&format!("  movss {base_offset}(%rbp,%rax,8), %xmm0\n"));
                }
                Type::F64 => {
                    output.push_str(&format!("  movsd {base_offset}(%rbp,%rax,8), %xmm0\n"));
                }
            }
            output.push_str(&format!("  jmp {done}\n"));
            output.push_str(&format!("{trap}:\n"));
            reporter.emit(Failure::ArrayIndexOutOfBounds, output);
            output.push_str(&format!("{done}:\n"));
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
            output.push_str("  testq %rax, %rax\n");
            output.push_str(&format!("  js {trap}\n"));
            output.push_str(&format!("  cmpq ${length}, %rax\n"));
            output.push_str(&format!("  jge {trap}\n"));
            output.push_str(&format!("  imulq $-{}, %rax\n", element_slots));
            for slot in 0..*element_slots {
                let source = base_offset - 8 * slot as isize;
                let destination = destination_offset - 8 * slot as isize;
                output.push_str(&format!("  movq {source}(%rbp,%rax,8), %rcx\n"));
                output.push_str(&format!("  movq %rcx, {destination}(%rbp)\n"));
            }
            output.push_str(&format!("  jmp {done}\n"));
            output.push_str(&format!("{trap}:\n"));
            reporter.emit(Failure::ArrayIndexOutOfBounds, output);
            output.push_str(&format!("{done}:\n"));
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
            output.push_str("  testq %rax, %rax\n");
            output.push_str(&format!("  js {trap}\n"));
            output.push_str(&format!("  cmpq ${length}, %rax\n"));
            output.push_str(&format!("  jge {trap}\n"));
            output.push_str(&format!("  imulq $-{}, %rax\n", element_slots));
            if *base_is_pointer {
                output.push_str(&format!("  movq {base_offset}(%rbp), %rcx\n"));
                output.push_str("  leaq (%rcx,%rax,8), %rcx\n");
            } else {
                output.push_str(&format!("  leaq {base_offset}(%rbp,%rax,8), %rcx\n"));
            }
            output.push_str(&format!("  movq %rcx, {destination_offset}(%rbp)\n"));
            output.push_str(&format!("  jmp {done}\n"));
            output.push_str(&format!("{trap}:\n"));
            reporter.emit(Failure::ArrayIndexOutOfBounds, output);
            output.push_str(&format!("{done}:\n"));
        }

        Instruction::StoreI64ToPointer(pointer_offset) => {
            output.push_str(&format!("  movq {pointer_offset}(%rbp), %rcx\n"));
            output.push_str("  movq %rax, (%rcx)\n");
        }
        Instruction::StoreF32ToPointer(pointer_offset) => {
            output.push_str(&format!("  movq {pointer_offset}(%rbp), %rcx\n"));
            output.push_str("  movss %xmm0, (%rcx)\n");
        }
        Instruction::StoreF64ToPointer(pointer_offset) => {
            output.push_str(&format!("  movq {pointer_offset}(%rbp), %rcx\n"));
            output.push_str("  movsd %xmm0, (%rcx)\n");
        }
        Instruction::CopyToPointer {
            source_offset,
            slots,
            pointer_offset,
        } => {
            output.push_str(&format!("  movq {pointer_offset}(%rbp), %r11\n"));
            for slot in 0..*slots {
                let source = source_offset - 8 * slot as isize;
                let destination = -8 * slot as isize;
                output.push_str(&format!("  movq {source}(%rbp), %r10\n"));
                output.push_str(&format!("  movq %r10, {destination}(%r11)\n"));
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
                    output.push_str(&format!("  movq {}(%rbp), %r11\n", offset + 16));
                    "%r11"
                }
            };
            for slot in 0..*slots {
                let source = -8 * slot as isize;
                let destination = destination_offset - 8 * slot as isize;
                output.push_str(&format!("  movq {source}({register}), %r10\n"));
                output.push_str(&format!("  movq %r10, {destination}(%rbp)\n"));
            }
        }

        Instruction::StoreAggregateReturnPointer { offset } => {
            output.push_str(&format!("  movq %rax, {offset}(%rbp)\n"));
        }

        Instruction::CopyToAggregateReturn {
            source_offset,
            slots,
            pointer_offset,
        } => {
            output.push_str(&format!("  movq {pointer_offset}(%rbp), %r11\n"));
            for slot in 0..*slots {
                let source = source_offset - 8 * slot as isize;
                let destination = -8 * slot as isize;
                output.push_str(&format!("  movq {source}(%rbp), %r10\n"));
                output.push_str(&format!("  movq %r10, {destination}(%r11)\n"));
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
                    Argument::Aggregate { offset } => {
                        match location {
                            ArgumentLocation::Register(index) => {
                                let register = integer_argument_register(index, module.target);
                                output.push_str(&format!("  leaq {offset}(%rbp), {register}\n"));
                            }
                            ArgumentLocation::Stack(destination) => {
                                output.push_str(&format!("  leaq {offset}(%rbp), %r10\n  movq %r10, {destination}(%rsp)\n"));
                            }
                        }
                    }
                }
            }
            if let Some(offset) = aggregate_result_offset {
                output.push_str(&format!("  leaq {offset}(%rbp), %rax\n"));
            }
            output.push_str(&format!(
                "  callq {}\n",
                function_name(&module.functions[*function_id])
            ));
        }

        Instruction::ObserveOnly => {}
        Instruction::Unreachable => output.push_str("  ud2\n"),
        Instruction::Return => emit_epilogue(frame_size, false, output),

        Instruction::LoadF32Constant(id) => {
            output.push_str(&format!("  movss .Lcerune_f32_{id}(%rip), %xmm0\n"));
        }

        Instruction::LoadF64Constant(id) => {
            output.push_str(&format!("  movsd .Lcerune_f64_{id}(%rip), %xmm0\n"));
        }

        Instruction::NegI64 => {
            output.push_str("  negq %rax\n");
        }

        Instruction::TrapIfOverflow(label) => {
            let done = format!(".Lcerune_{label_prefix}_integer_ok_{label}");
            output.push_str(&format!("  jno {done}\n"));
            reporter.emit(Failure::IntegerOverflow, output);
            output.push_str(&format!("{done}:\n"));
        }

        Instruction::NotBool => {
            output.push_str("  xorq $1, %rax\n");
        }

        Instruction::NegF32 => {
            output.push_str("  xorps .Lcerune_sign_f32(%rip), %xmm0\n");
        }

        Instruction::NegF64 => {
            output.push_str("  xorpd .Lcerune_sign_f64(%rip), %xmm0\n");
        }

        Instruction::MoveRaxToRcx => {
            output.push_str("  movq %rax, %rcx\n");
        }

        Instruction::I64Binary(op) => {
            output.push_str(match op {
                BinaryOp::Add => "  addq %rcx, %rax\n",
                BinaryOp::Subtract => "  subq %rcx, %rax\n",
                BinaryOp::Multiply => "  imulq %rcx, %rax\n",
                BinaryOp::Divide => {
                    unreachable!("i64 division is lowered to sign-extend + divide")
                }
            });
        }

        Instruction::CompareI64(op) => {
            output.push_str("  cmpq %rcx, %rax\n");
            output.push_str(compare_setcc(*op));
            output.push_str("  movzbq %al, %rax\n");
        }

        Instruction::SignExtendRax => {
            output.push_str("  cqto\n");
        }

        Instruction::TrapIfInvalidI64Division(label) => {
            let trap = format!(".Lcerune_{label_prefix}_division_trap_{label}");
            let done = format!(".Lcerune_{label_prefix}_division_ok_{label}");
            output.push_str("  testq %rcx, %rcx\n");
            output.push_str(&format!("  je {trap}\n"));
            output.push_str("  cmpq $-1, %rcx\n");
            output.push_str(&format!("  jne {done}\n"));
            output.push_str("  movabsq $-9223372036854775808, %rdx\n");
            output.push_str("  cmpq %rdx, %rax\n");
            output.push_str(&format!("  jne {done}\n"));
            reporter.emit(Failure::DivisionOverflow, output);
            output.push_str(&format!("{trap}:\n"));
            reporter.emit(Failure::DivisionByZero, output);
            output.push_str(&format!("{done}:\n"));
        }

        Instruction::DivideRaxByRcx => {
            output.push_str("  idivq %rcx\n");
        }

        Instruction::CopyXmm0ToXmm1F32 => {
            output.push_str("  movaps %xmm0, %xmm1\n");
        }

        Instruction::CopyXmm0ToXmm1F64 => {
            output.push_str("  movapd %xmm0, %xmm1\n");
        }

        Instruction::F32Binary(op) => {
            output.push_str(match op {
                BinaryOp::Add => "  addss %xmm1, %xmm0\n",
                BinaryOp::Subtract => "  subss %xmm1, %xmm0\n",
                BinaryOp::Multiply => "  mulss %xmm1, %xmm0\n",
                BinaryOp::Divide => "  divss %xmm1, %xmm0\n",
            });
        }

        Instruction::F64Binary(op) => {
            output.push_str(match op {
                BinaryOp::Add => "  addsd %xmm1, %xmm0\n",
                BinaryOp::Subtract => "  subsd %xmm1, %xmm0\n",
                BinaryOp::Multiply => "  mulsd %xmm1, %xmm0\n",
                BinaryOp::Divide => "  divsd %xmm1, %xmm0\n",
            });
        }

        Instruction::CompareF32(op) => {
            output.push_str("  ucomiss %xmm1, %xmm0\n");
            emit_float_comparison(*op, output);
        }

        Instruction::CompareF64(op) => {
            output.push_str("  ucomisd %xmm1, %xmm0\n");
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
    output: &mut String,
) {
    let ArgumentLocation::Register(index) = location else {
        let ArgumentLocation::Stack(source) = location else {
            unreachable!()
        };
        emit_stack_argument_copy(
            ty,
            &format!("{}(%rbp)", source + 16),
            &format!("{offset}(%rbp)"),
            output,
        );
        return;
    };
    match ty {
        Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
            let register = integer_argument_register(index, target);
            output.push_str(&format!("  movq {register}, {offset}(%rbp)\n"));
        }
        Type::F32 => {
            output.push_str(&format!("  movss %xmm{index}, {offset}(%rbp)\n"));
        }
        Type::F64 => {
            output.push_str(&format!("  movsd %xmm{index}, {offset}(%rbp)\n"));
        }
    }
}

fn emit_load_argument(
    location: ArgumentLocation,
    ty: Type,
    offset: isize,
    target: super::Target,
    output: &mut String,
) {
    let ArgumentLocation::Register(index) = location else {
        let ArgumentLocation::Stack(destination) = location else {
            unreachable!()
        };
        emit_stack_argument_copy(
            ty,
            &format!("{offset}(%rbp)"),
            &format!("{destination}(%rsp)"),
            output,
        );
        return;
    };
    match ty {
        Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
            let register = integer_argument_register(index, target);
            output.push_str(&format!("  movq {offset}(%rbp), {register}\n"));
        }
        Type::F32 => {
            output.push_str(&format!("  movss {offset}(%rbp), %xmm{index}\n"));
        }
        Type::F64 => {
            output.push_str(&format!("  movsd {offset}(%rbp), %xmm{index}\n"));
        }
    }
}

// XMMや引数レジスタを壊さず、f32も数値変換せずにビットをコピーします。
fn emit_stack_argument_copy(ty: Type, source: &str, destination: &str, output: &mut String) {
    let (instruction, register) = if ty == Type::F32 {
        ("movl", "%r10d")
    } else {
        ("movq", "%r10")
    };
    output.push_str(&format!(
        "  {instruction} {source}, {register}\n  {instruction} {register}, {destination}\n"
    ));
}

fn integer_argument_register(index: usize, target: super::Target) -> &'static str {
    if target.is_linux() {
        ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"][index]
    } else {
        ["%rcx", "%rdx", "%r8", "%r9"][index]
    }
}

fn compare_setcc(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Equal => "  sete %al\n",
        CompareOp::NotEqual => "  setne %al\n",
        CompareOp::Less => "  setl %al\n",
        CompareOp::LessEqual => "  setle %al\n",
        CompareOp::Greater => "  setg %al\n",
        CompareOp::GreaterEqual => "  setge %al\n",
    }
}

fn emit_float_comparison(op: CompareOp, output: &mut String) {
    match op {
        CompareOp::Equal => {
            output.push_str("  sete %al\n");
            output.push_str("  setnp %cl\n");
            output.push_str("  andb %cl, %al\n");
        }
        CompareOp::NotEqual => {
            output.push_str("  setne %al\n");
            output.push_str("  setp %cl\n");
            output.push_str("  orb %cl, %al\n");
        }
        CompareOp::Less => {
            output.push_str("  setb %al\n");
            output.push_str("  setnp %cl\n");
            output.push_str("  andb %cl, %al\n");
        }
        CompareOp::LessEqual => {
            output.push_str("  setbe %al\n");
            output.push_str("  setnp %cl\n");
            output.push_str("  andb %cl, %al\n");
        }
        CompareOp::Greater => output.push_str("  seta %al\n"),
        CompareOp::GreaterEqual => output.push_str("  setae %al\n"),
    }

    output.push_str("  movzbq %al, %rax\n");
}

fn emit_mir_origin(
    origin: Option<super::ir::MirOrigin>,
    prefix: &str,
    index: usize,
    annotate: bool,
    output: &mut String,
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
        output.push_str(&format!("# cerune-mir: v1 {prefix} bb{} {item} -> lir {index} ({reason})\ncerune_origin_mir_{prefix}_bb{}_{item}_lir{index}:\n", o.block.0, o.block.0));
    } else {
        output.push_str("# cerune-mir: v1 synthetic ABI setup/exit\n");
    }
}

fn emit_origin(
    origin: super::ir::Origin,
    prefix: &str,
    index: usize,
    annotate: bool,
    output: &mut String,
) {
    if !annotate {
        return;
    }
    match origin {
        super::ir::Origin::Synthetic => output.push_str("# cerune-origin: synthetic\n"),
        super::ir::Origin::Source { node_id, span } => {
            output.push_str(&format!(
                "# cerune-origin: #{}{} bytes {}..{}\ncerune_origin_n{}_{prefix}_{index}:\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end(),
                node_id.0
            ));
        }
    }
}
