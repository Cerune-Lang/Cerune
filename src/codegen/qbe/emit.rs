use std::fmt::Write;

use super::ir::{
    BinaryOp, CompareOp, Function, Instruction, Module, Operand, ParameterPassing, Slot, Temp, Type,
};

pub fn emit(module: &Module) -> String {
    let mut output = String::new();
    if let Some(target) = module.target {
        writeln!(
            output,
            "# target: {} (qbe -t {})\n",
            target.triple(),
            target.qbe_name()
        )
        .unwrap();
    }
    super::call::support(module, &mut output);
    super::output::emit(module, &mut output);
    super::input::emit(module, &mut output);
    super::arguments::emit_support(module, &mut output);
    let i64_operations = i64_operations(module);
    super::failure::emit(module, &mut output);
    if module.uses_strings {
        super::string::emit(module, &mut output);
        if let Some(limit) = module.string_heap_limit {
            output.push_str(&super::heap::support(
                limit,
                module
                    .target
                    .unwrap_or(super::Target::X86_64UnknownLinuxGnu)
                    .read_only_section(),
            ));
        }
    }

    if let Some(limit) = module.array_heap_limit {
        output.push_str(&super::array::support(limit));
    }

    output.push('\n');

    emit_i64_operation_support(i64_operations, &mut output);

    for function in &module.functions {
        emit_function(function, module, &mut output);
        output.push('\n');
    }

    output.push_str(if super::arguments::used(module) {
        "function w $cerune_program() {\n"
    } else {
        "export function w $main() {\n"
    });
    output.push_str("@start\n");
    if module.target == Some(super::Target::X86_64PcWindowsMsvc) {
        // CRTの改行変換を止め、文字列と診断のバイト列をそのまま渡します。
        output.push_str("  %stdout_mode =w call $_setmode(w 1, w 32768)\n  %stderr_mode =w call $_setmode(w 2, w 32768)\n  %stdout_bad =w csltw %stdout_mode, 0\n  %stderr_bad =w csltw %stderr_mode, 0\n  %io_bad =w or %stdout_bad, %stderr_bad\n  jnz %io_bad, @io_failed, @io_ready\n@io_failed\n  ret 1\n@io_ready\n");
    }

    for slot in &module.slots {
        writeln!(output, "  %slot_{} =l alloc8 {}", slot.name, slot.size).unwrap();
    }

    for (index, instruction) in module.instructions.iter().enumerate() {
        emit_instruction(instruction, &module.slots, module, index, &mut output);
    }

    if let Some(function_id) = module.explicit_main {
        super::call::enter(module, function_id, None, &mut output);
        writeln!(
            output,
            "  call ${}()",
            function_name(&module.functions[function_id])
        )
        .unwrap();
    }

    if let Some(id) = module.explicit_main {
        super::call::leave(module, id, &mut output);
    }
    if module.call_limit.is_some() {
        output.push_str("  call $cerune_call_finish()\n");
    }
    output.push_str("  ret 0\n");
    output.push_str("}\n");

    super::arguments::emit_entry(module, &mut output);
    crate::codegen::output::qbe(module, &mut output);
    output
}

#[derive(Clone, Default)]
struct I64Operations {
    conversions: std::collections::BTreeSet<crate::codegen::NumericConversion>,
    integer_binary:
        std::collections::BTreeSet<(crate::codegen::IntegerBinaryOp, crate::types::IntegerType)>,
    range_checks: std::collections::BTreeSet<crate::types::IntegerType>,
    add: bool,
    subtract: bool,
    multiply: bool,
    divide: bool,
    negate: bool,
}

impl I64Operations {
    fn include(&mut self, instruction: &Instruction) {
        if let Instruction::ConvertNumeric { conversion, .. } = instruction {
            self.conversions.insert(*conversion);
        }
        if let Instruction::IntegerBinary { op, ty, .. } = instruction {
            self.integer_binary.insert((*op, *ty));
        }
        match instruction {
            Instruction::CheckIntegerRange { ty, .. } => {
                if *ty != crate::types::IntegerType::I64 {
                    self.range_checks.insert(*ty);
                }
            }
            Instruction::CheckedI64Negate { .. } => self.negate = true,
            Instruction::Binary { op, .. } => match op {
                BinaryOp::CheckedI64Add => self.add = true,
                BinaryOp::CheckedI64Subtract => self.subtract = true,
                BinaryOp::CheckedI64Multiply => self.multiply = true,
                BinaryOp::CheckedI64Divide => self.divide = true,
                BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {}
            },
            _ => {}
        }
    }
}

fn i64_operations(module: &Module) -> I64Operations {
    let mut operations = I64Operations::default();
    for instruction in &module.instructions {
        operations.include(instruction);
    }
    for function in &module.functions {
        for instruction in &function.instructions {
            operations.include(instruction);
        }
    }
    operations
}

fn emit_i64_operation_support(operations: I64Operations, output: &mut String) {
    for &conversion in &operations.conversions {
        super::conversion::emit_support(conversion, output);
    }
    for &(op, ty) in &operations.integer_binary {
        super::integer::emit_support(op, ty, output);
    }
    for ty in &operations.range_checks {
        output.push_str(&format!("function l $cerune_check_{}(l %value, l %code, l %code_len, l %origin, l %origin_len) {{\n@start\n  %below =w csltl %value, {}\n  %above =w csgtl %value, {}\n  %bad =w or %below, %above\n  jnz %bad, @trap, @ok\n@trap\n  call $cerune_runtime_failure(l %code, l %code_len, l %origin, l %origin_len)\n  hlt\n@ok\n  ret %value\n}}\n\n", ty.name(), ty.minimum(), ty.maximum()));
    }

    for (enabled, name, operation, check) in [
        (
            operations.add,
            "add",
            "add",
            "  %left_changed =l xor %result, %left\n  %right_changed =l xor %result, %right\n  %sign_changed =l and %left_changed, %right_changed\n",
        ),
        (
            operations.subtract,
            "sub",
            "sub",
            "  %operands_differ =l xor %left, %right\n  %result_changed =l xor %left, %result\n  %sign_changed =l and %operands_differ, %result_changed\n",
        ),
    ] {
        if !enabled {
            continue;
        }
        writeln!(
            output,
            "function l $cerune_i64_{name}(l %left, l %right, l %origin, l %origin_len) {{"
        )
        .unwrap();
        output.push_str("@start\n");
        writeln!(output, "  %result =l {operation} %left, %right").unwrap();
        output.push_str(check);
        output.push_str("  %overflow =w csltl %sign_changed, 0\n");
        output.push_str("  jnz %overflow, @trap, @ok\n@trap\n");
        output.push_str("  call $cerune_fail_integer_overflow(l %origin, l %origin_len)\n  hlt\n@ok\n  ret %result\n}\n\n");
    }

    if operations.multiply {
        output.push_str(
            "function l $cerune_i64_mul(l %left, l %right, l %origin, l %origin_len) {\n\
             @start\n\
             \x20 %left_zero =w ceql %left, 0\n\
             \x20 jnz %left_zero, @zero, @special\n\
             @special\n\
             \x20 %left_negative_one =w ceql %left, -1\n\
             \x20 %right_min =w ceql %right, -9223372036854775808\n\
             \x20 %first_special =w and %left_negative_one, %right_min\n\
             \x20 %right_negative_one =w ceql %right, -1\n\
             \x20 %left_min =w ceql %left, -9223372036854775808\n\
             \x20 %second_special =w and %right_negative_one, %left_min\n\
             \x20 %special_overflow =w or %first_special, %second_special\n\
             \x20 jnz %special_overflow, @trap, @multiply\n\
             @multiply\n\
             \x20 %result =l mul %left, %right\n\
             \x20 %quotient =l div %result, %left\n\
             \x20 %overflow =w cnel %quotient, %right\n\
             \x20 jnz %overflow, @trap, @ok\n\
             @zero\n\
             \x20 ret 0\n\
             @trap\n\
             \x20 call $cerune_fail_integer_overflow(l %origin, l %origin_len)\n\
             \x20 hlt\n\
             @ok\n\
             \x20 ret %result\n\
             }\n\n",
        );
    }

    if operations.divide {
        output.push_str("function l $cerune_i64_div(l %left, l %right, l %origin, l %origin_len) {\n@start\n  %is_zero =w ceql %right, 0\n  jnz %is_zero, @zero, @bounds\n@bounds\n  %is_min =w ceql %left, -9223372036854775808\n  %is_negative_one =w ceql %right, -1\n  %overflows =w and %is_min, %is_negative_one\n  jnz %overflows, @overflow, @ok\n");
        super::failure::block("zero", crate::runtime::FailureCode::DivisionByZero, output);
        super::failure::block(
            "overflow",
            crate::runtime::FailureCode::DivisionOverflow,
            output,
        );
        output.push_str("@ok\n  %result =l div %left, %right\n  ret %result\n}\n\n");
    }

    if operations.negate {
        output.push_str(
            "function l $cerune_i64_neg(l %value, l %origin, l %origin_len) {\n\
             @start\n\
             \x20 %overflow =w ceql %value, -9223372036854775808\n\
             \x20 jnz %overflow, @trap, @ok\n\
             @trap\n\
             \x20 call $cerune_fail_integer_overflow(l %origin, l %origin_len)\n\
             \x20 hlt\n\
             @ok\n\
             \x20 %result =l neg %value\n\
             \x20 ret %result\n\
             }\n\n",
        );
    }
}

fn emit_function(function: &Function, module: &Module, output: &mut String) {
    output.push_str("function ");
    if let Some(return_type) = function.return_type {
        write!(output, "{} ", type_name(return_type)).unwrap();
    }
    write!(output, "${}(", function_name(function)).unwrap();
    let mut has_parameter = false;
    if function.aggregate_return_size.is_some() {
        output.push_str("l %result");
        has_parameter = true;
    }
    for (index, parameter) in function.parameters.iter().enumerate() {
        if has_parameter {
            output.push_str(", ");
        }
        let ty = match parameter.passing {
            ParameterPassing::Scalar(ty) => ty,
            ParameterPassing::Aggregate { .. } => Type::Pointer,
        };
        let position = index + usize::from(function.aggregate_return_size.is_some());
        if let Some(bits) = stack_float_bits(module, position, ty) {
            write!(output, "{bits} %bits_arg{index}").unwrap();
        } else {
            write!(output, "{} %arg{index}", type_name(ty)).unwrap();
        }
        has_parameter = true;
    }
    output.push_str(") {\n@start\n");

    for slot in &function.slots {
        writeln!(output, "  %slot_{} =l alloc8 {}", slot.name, slot.size).unwrap();
    }
    for (index, parameter) in function.parameters.iter().enumerate() {
        match parameter.passing {
            ParameterPassing::Scalar(ty) => {
                let position = index + usize::from(function.aggregate_return_size.is_some());
                if stack_float_bits(module, position, ty).is_some() {
                    writeln!(
                        output,
                        "  %arg{index} ={} cast %bits_arg{index}",
                        type_name(ty)
                    )
                    .unwrap();
                }
                writeln!(
                    output,
                    "  {} %arg{index}, %slot_{}",
                    store_name(ty),
                    function.slots[parameter.slot].name
                )
                .unwrap();
            }
            ParameterPassing::Aggregate { size } => {
                writeln!(
                    output,
                    "  blit %arg{index}, %slot_{}, {size}",
                    function.slots[parameter.slot].name
                )
                .unwrap();
            }
        }
    }
    for (index, instruction) in function.instructions.iter().enumerate() {
        emit_instruction(instruction, &function.slots, module, index, output);
    }
    output.push_str("}\n");
}

fn function_name(function: &Function) -> String {
    format!(
        "cerune_fn_{}_{}",
        function.name.replace('$', "_"),
        function.id
    )
}

// QBE 1.3 amd64_winはスタックの浮動小数点引数を整数として読み込みます。
// 内部呼び出しの第5引数以降を同幅の整数ビット列として渡し、入口で復元します。
// 隠れた集約戻り値ポインタも引数位置に含めます。数値変換は行いません。
fn stack_float_bits(module: &Module, position: usize, ty: Type) -> Option<&'static str> {
    if module.target != Some(super::Target::X86_64PcWindowsMsvc) || position < 4 {
        return None;
    }
    match ty {
        Type::Single => Some("w"),
        Type::Double => Some("l"),
        _ => None,
    }
}

fn emit_instruction(
    instruction: &Instruction,
    slots: &[Slot],
    module: &Module,
    instruction_index: usize,
    output: &mut String,
) {
    match instruction {
        Instruction::Flush { origin } => {
            writeln!(output, "  call $cerune_flush_{}()", origin.node.0).unwrap();
        }

        Instruction::ArrayAllocate {
            dest,
            length,
            width,
            stride,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_array_allocate(l {}, l {width}, l {stride}, {})",
                temp(*dest),
                operand(length, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::ArrayRangeCheck {
            length,
            start,
            end,
            origin,
        } => {
            writeln!(
                output,
                "  call $cerune_array_check_range(l {}, l {}, l {}, {})",
                operand(length, slots),
                operand(start, slots),
                operand(end, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::ArrayAddress {
            dest,
            value,
            index,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_array_checked_address(l {}, l {}, {})",
                temp(*dest),
                operand(value, slots),
                operand(index, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::ArrayInitAddress { dest, value }
        | Instruction::ArrayLength { dest, value }
        | Instruction::ArrayReleaseOwner { dest, value } => {
            let (ty, name) = match instruction {
                Instruction::ArrayInitAddress { .. } => ("l", "init_address"),
                Instruction::ArrayLength { .. } => ("l", "length"),
                _ => ("w", "release_owner_last"),
            };
            writeln!(
                output,
                "  {} ={ty} call $cerune_array_{name}(l {})",
                temp(*dest),
                operand(value, slots)
            )
            .unwrap();
        }
        Instruction::ArrayInitialized { value } | Instruction::ArrayManage { value, .. } => {
            let name = match instruction {
                Instruction::ArrayInitialized { .. } => "initialized",
                Instruction::ArrayManage { retain: true, .. } => "retain_owner",
                _ => "free_elements",
            };
            writeln!(
                output,
                "  call $cerune_array_{name}(l {})",
                operand(value, slots)
            )
            .unwrap();
        }
        Instruction::StringByteAt { dest, value, index } => {
            writeln!(
                output,
                "  {} =l call $cerune_string_byte_at(l {}, l {})",
                temp(*dest),
                operand(value, slots),
                operand(index, slots)
            )
            .unwrap();
        }
        Instruction::ArgumentCount { dest, origin } => {
            writeln!(
                output,
                "  {} =l call $cerune_arguments_count(l ${})",
                temp(*dest),
                super::arguments::origin_name(*origin)
            )
            .unwrap();
        }
        Instruction::ArgumentGet {
            dest,
            index,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_arguments_get(l {}, l ${})",
                temp(*dest),
                operand(index, slots),
                super::arguments::origin_name(*origin)
            )
            .unwrap();
        }
        Instruction::ReadLine {
            dest,
            max_bytes,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_input_read_line(l {}, l ${})",
                temp(*dest),
                operand(max_bytes, slots),
                super::input::origin_name(*origin)
            )
            .unwrap();
        }
        Instruction::StringConcat {
            dest,
            left,
            right,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_string_concat(l {}, l {}, {})",
                temp(*dest),
                operand(left, slots),
                operand(right, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::StringManage { value, retain } => {
            writeln!(
                output,
                "  call $cerune_string_{}(l {})",
                if *retain { "retain" } else { "release" },
                operand(value, slots)
            )
            .unwrap();
        }
        Instruction::Output {
            kind,
            value,
            newline,
            origin,
        } => {
            writeln!(
                output,
                "  call $cerune_output_{kind}({} {}, w {}, l ${})",
                crate::codegen::display::qbe_type(kind),
                operand(value, slots),
                u8::from(*newline),
                super::output::origin_name(*origin)
            )
            .unwrap();
        }
        Instruction::ConvertNumeric {
            dest,
            value,
            conversion,
            origin,
        } => {
            writeln!(
                output,
                "  {} ={} call ${}({} {}, {})",
                temp(*dest),
                super::conversion::type_name(conversion.to),
                conversion.helper(),
                super::conversion::type_name(conversion.from),
                operand(value, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::IntegerBinary {
            dest,
            op,
            ty,
            left,
            right,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call ${}(l {}, l {}, {})",
                temp(*dest),
                op.helper(*ty),
                operand(left, slots),
                operand(right, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::CheckIntegerRange {
            dest,
            value,
            ty,
            origin,
            failure,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_check_{}(l {}, {}, {})",
                temp(*dest),
                ty.name(),
                operand(value, slots),
                super::failure::code_arguments(*failure),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }
        Instruction::Label { id, name } => {
            writeln!(output, "@block{id} # {name}").unwrap();
        }

        Instruction::Branch {
            condition,
            then_label,
            else_label,
        } => {
            writeln!(
                output,
                "  jnz {}, @block{then_label}, @block{else_label}",
                operand(condition, slots)
            )
            .unwrap();
        }

        Instruction::Jump(label) => {
            writeln!(output, "  jmp @block{label}").unwrap();
        }

        Instruction::Store { address, ty, value } => {
            writeln!(
                output,
                "  {} {}, {}",
                store_name(*ty),
                operand(value, slots),
                operand(address, slots),
            )
            .unwrap();
        }

        Instruction::Load { dest, address, ty } => {
            writeln!(
                output,
                "  {} ={} {} {}",
                temp(*dest),
                type_name(*ty),
                load_name(*ty),
                operand(address, slots),
            )
            .unwrap();
        }

        Instruction::Address { dest, base, offset } => {
            writeln!(
                output,
                "  {} =l add {}, {}",
                temp(*dest),
                operand(base, slots),
                offset
            )
            .unwrap();
        }

        Instruction::Blit {
            source,
            destination,
            size,
        } => {
            writeln!(
                output,
                "  blit {}, {}, {size}",
                operand(source, slots),
                operand(destination, slots)
            )
            .unwrap();
        }

        Instruction::Abort { origin } => {
            writeln!(
                output,
                "  call $cerune_runtime_failure({}, {})\n  hlt",
                super::failure::code_arguments(crate::runtime::FailureCode::ArrayIndexOutOfBounds),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }

        Instruction::Call {
            origin,
            dest,
            function_id,
            return_type,
            arguments,
        } => {
            super::call::enter(module, *function_id, Some(*origin), output);
            for (index, (ty, argument)) in arguments.iter().enumerate() {
                if let Some(bits) = stack_float_bits(module, index, *ty) {
                    writeln!(
                        output,
                        "  %call_{instruction_index}_arg_{index} ={bits} cast {}",
                        operand(argument, slots)
                    )
                    .unwrap();
                }
            }
            output.push_str("  ");
            if let (Some(dest), Some(return_type)) = (dest, return_type) {
                write!(output, "{} ={} ", temp(*dest), type_name(*return_type)).unwrap();
            }
            write!(
                output,
                "call ${}(",
                function_name(&module.functions[*function_id])
            )
            .unwrap();
            for (index, (ty, argument)) in arguments.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                if let Some(bits) = stack_float_bits(module, index, *ty) {
                    write!(output, "{bits} %call_{instruction_index}_arg_{index}").unwrap();
                } else {
                    write!(output, "{} {}", type_name(*ty), operand(argument, slots)).unwrap();
                }
            }
            output.push_str(")\n");
            super::call::leave(module, *function_id, output);
        }

        Instruction::Return { value } => match value {
            Some((_, value)) => {
                writeln!(output, "  ret {}", operand(value, slots)).unwrap();
            }
            None => output.push_str("  ret\n"),
        },

        Instruction::Negate { dest, ty, value } => {
            writeln!(
                output,
                "  {} ={} neg {}",
                temp(*dest),
                type_name(*ty),
                operand(value, slots),
            )
            .unwrap();
        }

        Instruction::CheckedI64Negate {
            dest,
            value,
            origin,
        } => {
            writeln!(
                output,
                "  {} =l call $cerune_i64_neg(l {}, {})",
                temp(*dest),
                operand(value, slots),
                super::failure::arguments(*origin)
            )
            .unwrap();
        }

        Instruction::Not { dest, value } => {
            writeln!(
                output,
                "  {} =w ceqw {}, 0",
                temp(*dest),
                operand(value, slots)
            )
            .unwrap();
        }

        Instruction::Binary {
            dest,
            op,
            ty,
            left,
            right,
            origin,
        } => {
            if let Some(helper) = checked_i64_helper(*op) {
                writeln!(
                    output,
                    "  {} =l call ${helper}(l {}, l {}, {})",
                    temp(*dest),
                    operand(left, slots),
                    operand(right, slots),
                    super::failure::arguments(*origin)
                )
                .unwrap();
            } else {
                writeln!(
                    output,
                    "  {} ={} {} {}, {}",
                    temp(*dest),
                    type_name(*ty),
                    binary_name(*op),
                    operand(left, slots),
                    operand(right, slots),
                )
                .unwrap();
            }
        }

        Instruction::Compare {
            unsigned,
            dest,
            op,
            operand_ty,
            left,
            right,
        } => {
            if *operand_ty == Type::String {
                let name = temp(*dest);
                let result = if *op == CompareOp::NotEqual {
                    format!("{name}_equal")
                } else {
                    assert_eq!(*op, CompareOp::Equal);
                    name.clone()
                };
                writeln!(
                    output,
                    "  {result} =w call $cerune_string_equal(l {}, l {})",
                    operand(left, slots),
                    operand(right, slots)
                )
                .unwrap();
                if *op == CompareOp::NotEqual {
                    writeln!(output, "  {name} =w xor {result}, 1").unwrap();
                }
                return;
            }
            writeln!(
                output,
                "  {} =w {} {}, {}",
                temp(*dest),
                if *unsigned {
                    unsigned_compare(*op)
                } else {
                    compare_name(*op, *operand_ty)
                },
                operand(left, slots),
                operand(right, slots),
            )
            .unwrap();
        }
    }
}

fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::Bool => "w",
        Type::DynamicArray | Type::String | Type::I64 => "l",
        Type::Single => "s",
        Type::Double => "d",
        Type::Pointer => "l",
    }
}

fn store_name(ty: Type) -> &'static str {
    match ty {
        Type::Bool => "storew",
        Type::DynamicArray | Type::String | Type::I64 => "storel",
        Type::Single => "stores",
        Type::Double => "stored",
        Type::Pointer => unreachable!("pointers are passed without scalar stores"),
    }
}

fn load_name(ty: Type) -> &'static str {
    match ty {
        Type::Bool => "loadw",
        Type::DynamicArray | Type::String | Type::I64 => "loadl",
        Type::Single => "loads",
        Type::Double => "loadd",
        Type::Pointer => unreachable!("pointers are passed without scalar loads"),
    }
}

fn compare_name(op: CompareOp, ty: Type) -> &'static str {
    match (op, ty) {
        (CompareOp::Equal, Type::Bool) => "ceqw",
        (CompareOp::NotEqual, Type::Bool) => "cnew",

        (CompareOp::Equal, Type::I64) => "ceql",
        (CompareOp::NotEqual, Type::I64) => "cnel",
        (CompareOp::Less, Type::I64) => "csltl",
        (CompareOp::LessEqual, Type::I64) => "cslel",
        (CompareOp::Greater, Type::I64) => "csgtl",
        (CompareOp::GreaterEqual, Type::I64) => "csgel",

        (CompareOp::Equal, Type::Single) => "ceqs",
        (CompareOp::NotEqual, Type::Single) => "cnes",
        (CompareOp::Less, Type::Single) => "clts",
        (CompareOp::LessEqual, Type::Single) => "cles",
        (CompareOp::Greater, Type::Single) => "cgts",
        (CompareOp::GreaterEqual, Type::Single) => "cges",

        (CompareOp::Equal, Type::Double) => "ceqd",
        (CompareOp::NotEqual, Type::Double) => "cned",
        (CompareOp::Less, Type::Double) => "cltd",
        (CompareOp::LessEqual, Type::Double) => "cled",
        (CompareOp::Greater, Type::Double) => "cgtd",
        (CompareOp::GreaterEqual, Type::Double) => "cged",

        (
            CompareOp::Less | CompareOp::LessEqual | CompareOp::Greater | CompareOp::GreaterEqual,
            Type::Bool,
        ) => {
            unreachable!("semantic analysis rejects boolean ordering")
        }
        (_, Type::DynamicArray | Type::String | Type::Pointer) => {
            unreachable!("string comparison uses its own helper")
        }
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "sub",
        BinaryOp::Multiply => "mul",
        BinaryOp::Divide => "div",
        BinaryOp::CheckedI64Add
        | BinaryOp::CheckedI64Subtract
        | BinaryOp::CheckedI64Multiply
        | BinaryOp::CheckedI64Divide => {
            unreachable!("checked integer operations are emitted as helper calls")
        }
    }
}

fn checked_i64_helper(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::CheckedI64Add => Some("cerune_i64_add"),
        BinaryOp::CheckedI64Subtract => Some("cerune_i64_sub"),
        BinaryOp::CheckedI64Multiply => Some("cerune_i64_mul"),
        BinaryOp::CheckedI64Divide => Some("cerune_i64_div"),
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => None,
    }
}

fn operand(value: &Operand, slots: &[Slot]) -> String {
    match value {
        Operand::String(id) => format!("$cerune_string_{id}"),
        Operand::Boolean(value) => i32::from(*value).to_string(),
        Operand::Integer(value) => value.to_string(),
        // 非有限値はQBEをビルドしたCRTのscanf表記に依存しないビット定数にします。
        Operand::Float32(text) => match text.parse::<f32>() {
            Ok(value) if !value.is_finite() => value.to_bits().to_string(),
            _ => format!("s_{text}"),
        },
        Operand::Float64(text) => match text.parse::<f64>() {
            Ok(value) if !value.is_finite() => (value.to_bits() as i64).to_string(),
            _ => format!("d_{text}"),
        },
        Operand::Temp(temp) => self::temp(*temp),
        Operand::Slot(slot) => format!("%slot_{}", slots[*slot].name),
        Operand::ReturnPointer => "%result".into(),
    }
}

fn temp(temp: Temp) -> String {
    format!("%tmp{}", temp.0)
}

fn unsigned_compare(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Equal => "ceql",
        CompareOp::NotEqual => "cnel",
        CompareOp::Less => "cultl",
        CompareOp::LessEqual => "culel",
        CompareOp::Greater => "cugtl",
        CompareOp::GreaterEqual => "cugel",
    }
}
