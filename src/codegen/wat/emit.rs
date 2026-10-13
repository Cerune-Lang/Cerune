use std::fmt::Write;

use super::ir::{Function, Instruction, LoopKind, Module, Origin, Type};

pub fn emit(module: &Module) -> String {
    let mut output = String::new();
    let mut helpers = std::collections::BTreeMap::new();
    let mut uses_failures = module.uses_flush;
    for instruction in module.instructions.iter().chain(
        module
            .functions
            .iter()
            .flat_map(|function| &function.instructions),
    ) {
        collect_helpers(instruction, &mut helpers, &mut uses_failures);
    }

    writeln!(output, "(module").unwrap();
    if module.uses_write {
        output.push_str(&crate::codegen::display::wat_imports());
    }
    if uses_failures {
        output.push_str(
            "  (import \"cerune\" \"write_error_byte\" (func $write_error_byte (param i32)))\n",
        );
    }
    if module.uses_strings {
        output.push_str("  (import \"cerune\" \"write_byte\" (func $write_byte (param i32)))\n");
    }

    if module.uses_flush {
        output.push_str(
            "  (import \"cerune\" \"flush_stdout_v1\" (func $flush_stdout_v1 (result i32)))\n",
        );
    }
    if module_uses_u64_print(module) {
        output.push_str("  (import \"cerune\" \"print_u64\" (func $print_u64 (param i64)))\n");
    }
    // print() is provided by the host.
    if module_uses_bool_print(module) {
        writeln!(
            output,
            "  (import \"cerune\" \"print_bool\" (func $print_bool (param i32)))"
        )
        .unwrap();
    }

    writeln!(
        output,
        "  (import \"cerune\" \"print_i64\" (func $print_i64 (param i64)))"
    )
    .unwrap();

    writeln!(
        output,
        "  (import \"cerune\" \"print_f32\" (func $print_f32 (param f32)))"
    )
    .unwrap();

    writeln!(
        output,
        "  (import \"cerune\" \"print_f64\" (func $print_f64 (param f64)))"
    )
    .unwrap();

    writeln!(output).unwrap();

    for (name, (instruction, origin)) in helpers {
        emit_helper(instruction, origin, &name, &mut output);
    }

    if module.memory_pages > 0 || module.uses_strings {
        writeln!(output, "  (memory {})", module.memory_pages).unwrap();
        writeln!(output).unwrap();
    }

    if module.uses_strings {
        super::string::emit(module, &mut output);
        if let Some(limit) = module.string_heap_limit {
            output.push_str(&super::heap::support(limit, module.string_heap_start));
        }
    }

    if let Some(limit) = module.array_heap_limit {
        output.push_str(&super::array::support(
            limit,
            module.string_heap_start,
            !(module.uses_strings && module.string_heap_limit.is_some()),
        ));
    }
    if module.uses_write {
        output.push_str(&crate::codegen::display::wat());
    }
    for function in &module.functions {
        emit_function(function, module, &mut output);
        writeln!(output).unwrap();
    }

    writeln!(output, "  (func $main").unwrap();

    for local in &module.locals {
        writeln!(
            output,
            "    (local $cerune_{} {})",
            local.name,
            wat_type(local.ty),
        )
        .unwrap();
    }

    if !module.locals.is_empty() {
        writeln!(output).unwrap();
    }

    for instruction in &module.instructions {
        emit_instruction(instruction, 2, module, &mut output);
    }

    if let Some(function_id) = module.explicit_main {
        writeln!(
            output,
            "    call ${}",
            function_name(&module.functions[function_id])
        )
        .unwrap();
    }

    writeln!(output, "  )").unwrap();

    writeln!(output, "  (export \"main\" (func $main))").unwrap();

    writeln!(output, ")").unwrap();

    output
}

fn collect_helpers<'a>(
    instruction: &'a Instruction,
    helpers: &mut std::collections::BTreeMap<String, (&'a Instruction, Origin)>,
    uses_failures: &mut bool,
) {
    match instruction {
        Instruction::Located {
            origin,
            instruction,
        } => {
            *uses_failures = true;
            helpers.insert(
                super::failure::helper_name(instruction, *origin),
                (instruction, *origin),
            );
        }
        Instruction::Failure(_) => *uses_failures = true,
        Instruction::If {
            then_instructions,
            else_instructions,
        }
        | Instruction::IfBool {
            then_instructions,
            else_instructions,
        } => {
            for child in then_instructions.iter().chain(else_instructions) {
                collect_helpers(child, helpers, uses_failures);
            }
        }
        Instruction::Loop {
            condition_instructions,
            body_instructions,
            update_instructions,
            ..
        } => {
            for child in condition_instructions
                .iter()
                .chain(body_instructions)
                .chain(update_instructions)
            {
                collect_helpers(child, helpers, uses_failures);
            }
        }
        _ => {}
    }
}

fn emit_helper(instruction: &Instruction, origin: Origin, name: &str, output: &mut String) {
    match instruction {
        Instruction::ArrayAllocate { .. }
        | Instruction::ArrayAddress
        | Instruction::ArrayRangeCheck => super::array::checked(instruction, origin, name, output),
        Instruction::StringConcat => super::heap::concat(origin, name, output),
        Instruction::ConvertNumeric { conversion } => {
            super::conversion::emit_support(*conversion, origin, name, output)
        }
        Instruction::IntegerBinary { op, ty } => {
            super::integer::emit_support(*op, *ty, origin, name, output)
        }
        Instruction::CheckIntegerRange { ty, failure } => {
            writeln!(output, "  (func ${name} (param $value i64) (result i64)").unwrap();
            super::failure::emit_if(
                &format!(
                    "    local.get $value\n    i64.const {}\n    i64.lt_s\n    local.get $value\n    i64.const {}\n    i64.gt_s\n    i32.or\n",
                    ty.minimum(),
                    ty.maximum()
                ),
                *failure,
                origin,
                output,
            );
            output.push_str("    local.get $value\n  )\n\n");
        }
        Instruction::CheckedI64Add
        | Instruction::CheckedI64Sub
        | Instruction::CheckedI64Mul
        | Instruction::CheckedI64DivS => {
            super::integer::emit_signed(instruction, origin, name, output)
        }
        _ => unreachable!("only checked operations have specialized helpers"),
    }
}

fn emit_function(function: &Function, module: &Module, output: &mut String) {
    write!(output, "  (func ${}", function_name(function)).unwrap();
    for parameter in &function.parameters {
        write!(
            output,
            " (param $cerune_{} {})",
            parameter.name,
            wat_type(parameter.ty)
        )
        .unwrap();
    }
    if let Some(return_type) = function.return_type {
        write!(output, " (result {})", wat_type(return_type)).unwrap();
    }
    writeln!(output).unwrap();

    for local in &function.locals {
        writeln!(
            output,
            "    (local $cerune_{} {})",
            local.name,
            wat_type(local.ty)
        )
        .unwrap();
    }
    if !function.locals.is_empty() {
        writeln!(output).unwrap();
    }
    for instruction in &function.instructions {
        emit_instruction(instruction, 2, module, output);
    }
    write!(output, "  )").unwrap();
}

fn function_name(function: &Function) -> String {
    format!(
        "cerune_fn_{}_{}",
        function.name.replace('$', "_"),
        function.id
    )
}

fn emit_instruction(
    instruction: &Instruction,
    indent: usize,
    module: &Module,
    output: &mut String,
) {
    let prefix = "  ".repeat(indent);

    match instruction {
        Instruction::Flush { origin } => {
            let record =
                crate::runtime::output::flush_failure_record(crate::runtime::output::Origin {
                    node_id: origin.node_id,
                    span: origin.span,
                });
            writeln!(output, "{prefix}call $flush_stdout_v1\n{prefix}if").unwrap();
            for b in format!("cerune: {record}\n").bytes() {
                writeln!(
                    output,
                    "{prefix}  i32.const {b}\n{prefix}  call $write_error_byte"
                )
                .unwrap();
            }
            writeln!(output, "{prefix}  unreachable\n{prefix}end").unwrap();
        }
        Instruction::Located {
            origin,
            instruction,
        } => writeln!(
            output,
            "{prefix}call ${}",
            super::failure::helper_name(instruction, *origin)
        )
        .unwrap(),
        Instruction::Failure(record) => super::failure::emit(*record, &prefix, output),
        Instruction::Write { kind } => {
            writeln!(output, "{prefix}call $cerune_write_{kind}").unwrap()
        }
        Instruction::CallPrintU64 => writeln!(output, "{prefix}call $print_u64").unwrap(),
        Instruction::I64LtU => writeln!(output, "{prefix}i64.lt_u").unwrap(),
        Instruction::I64LeU => writeln!(output, "{prefix}i64.le_u").unwrap(),
        Instruction::I64GtU => writeln!(output, "{prefix}i64.gt_u").unwrap(),
        Instruction::I64GeU => writeln!(output, "{prefix}i64.ge_u").unwrap(),
        Instruction::ArrayAllocate { .. }
        | Instruction::ArrayAddress
        | Instruction::ArrayRangeCheck => unreachable!("checked arrays carry their source origin"),
        Instruction::ArrayInitAddress => {
            writeln!(output, "{prefix}call $cerune_array_init_address").unwrap()
        }
        Instruction::ArrayInitialized => {
            writeln!(output, "{prefix}call $cerune_array_initialized").unwrap()
        }
        Instruction::ArrayLength => writeln!(output, "{prefix}call $cerune_array_length").unwrap(),
        Instruction::ArrayRetain => writeln!(output, "{prefix}call $cerune_array_retain").unwrap(),
        Instruction::ArrayReleaseOwner => {
            writeln!(output, "{prefix}call $cerune_array_release_owner").unwrap()
        }
        Instruction::ArrayFree => writeln!(output, "{prefix}call $cerune_array_free").unwrap(),
        Instruction::StringConcat => unreachable!("concat carries its source origin"),
        Instruction::StringManage { retain } => {
            writeln!(
                output,
                "{prefix}call $cerune_string_{}",
                if *retain { "retain" } else { "release" }
            )
            .unwrap();
        }
        Instruction::StringEqual => writeln!(output, "{prefix}call $cerune_string_equal").unwrap(),
        Instruction::StringNotEqual => {
            writeln!(output, "{prefix}call $cerune_string_equal\n{prefix}i32.eqz").unwrap()
        }
        Instruction::ConvertNumeric { conversion } => {
            writeln!(output, "{prefix}call ${}", conversion.helper()).unwrap();
        }
        Instruction::IntegerBinary { op, ty } => {
            writeln!(output, "{prefix}call ${}", op.helper(*ty)).unwrap();
        }
        Instruction::CheckIntegerRange { ty, .. } => {
            writeln!(output, "{prefix}call $cerune_check_{}", ty.name()).unwrap();
        }
        Instruction::I32Const(value) => {
            writeln!(output, "{prefix}i32.const {value}").unwrap();
        }

        Instruction::I64Const(value) => {
            writeln!(output, "{prefix}i64.const {value}").unwrap();
        }

        Instruction::F32Const(text) => {
            writeln!(output, "{prefix}f32.const {text}").unwrap();
        }

        Instruction::F64Const(text) => {
            writeln!(output, "{prefix}f64.const {text}").unwrap();
        }

        Instruction::LocalGet(name) => {
            writeln!(output, "{prefix}local.get $cerune_{name}").unwrap();
        }

        Instruction::LocalSet(name) => {
            writeln!(output, "{prefix}local.set $cerune_{name}").unwrap();
        }

        Instruction::I32Load { offset } => emit_memory("i32.load", *offset, &prefix, output),
        Instruction::I64Load { offset } => emit_memory("i64.load", *offset, &prefix, output),
        Instruction::F32Load { offset } => emit_memory("f32.load", *offset, &prefix, output),
        Instruction::F64Load { offset } => emit_memory("f64.load", *offset, &prefix, output),
        Instruction::I32Store { offset } => emit_memory("i32.store", *offset, &prefix, output),
        Instruction::I64Store { offset } => emit_memory("i64.store", *offset, &prefix, output),
        Instruction::F32Store { offset } => emit_memory("f32.store", *offset, &prefix, output),
        Instruction::F64Store { offset } => emit_memory("f64.store", *offset, &prefix, output),

        Instruction::I32WrapI64 => emit_simple("i32.wrap_i64", &prefix, output),
        Instruction::I32Add => emit_simple("i32.add", &prefix, output),
        Instruction::I32Mul => emit_simple("i32.mul", &prefix, output),
        Instruction::Unreachable => emit_simple("unreachable", &prefix, output),

        Instruction::If {
            then_instructions,
            else_instructions,
        }
        | Instruction::IfBool {
            then_instructions,
            else_instructions,
        } => {
            let result = if matches!(instruction, Instruction::IfBool { .. }) {
                " (result i32)"
            } else {
                ""
            };
            writeln!(output, "{prefix}if{result}").unwrap();
            for instruction in then_instructions {
                emit_instruction(instruction, indent + 1, module, output);
            }
            if !else_instructions.is_empty() {
                writeln!(output, "{prefix}else").unwrap();
                for instruction in else_instructions {
                    emit_instruction(instruction, indent + 1, module, output);
                }
            }
            writeln!(output, "{prefix}end").unwrap();
        }

        Instruction::Loop {
            kind,
            id,
            condition_instructions,
            body_instructions,
            update_instructions,
        } => {
            let name = loop_name(*kind);
            writeln!(output, "{prefix}block ${name}_end_{id}").unwrap();
            writeln!(output, "{prefix}  loop ${name}_condition_{id}").unwrap();
            for instruction in condition_instructions {
                emit_instruction(instruction, indent + 2, module, output);
            }
            writeln!(output, "{prefix}    i32.eqz").unwrap();
            writeln!(output, "{prefix}    br_if ${name}_end_{id}").unwrap();
            writeln!(output, "{prefix}    block ${name}_continue_{id}").unwrap();
            for instruction in body_instructions {
                emit_instruction(instruction, indent + 3, module, output);
            }
            writeln!(output, "{prefix}    end").unwrap();
            for instruction in update_instructions {
                emit_instruction(instruction, indent + 2, module, output);
            }
            writeln!(output, "{prefix}    br ${name}_condition_{id}").unwrap();
            writeln!(output, "{prefix}  end").unwrap();
            writeln!(output, "{prefix}end").unwrap();
        }

        Instruction::Break { kind, id } => {
            writeln!(output, "{prefix}br ${}_end_{id}", loop_name(*kind)).unwrap();
        }

        Instruction::Continue { kind, id } => {
            writeln!(output, "{prefix}br ${}_continue_{id}", loop_name(*kind)).unwrap();
        }

        Instruction::Call { function_id } => {
            writeln!(
                output,
                "{prefix}call ${}",
                function_name(&module.functions[*function_id])
            )
            .unwrap();
        }

        Instruction::Return => emit_simple("return", &prefix, output),

        Instruction::CheckedI64Add => emit_simple("call $cerune_i64_add", &prefix, output),
        Instruction::CheckedI64Sub => emit_simple("call $cerune_i64_sub", &prefix, output),
        Instruction::CheckedI64Mul => emit_simple("call $cerune_i64_mul", &prefix, output),
        Instruction::CheckedI64DivS => emit_simple("i64.div_s", &prefix, output),
        Instruction::I64Eq => emit_simple("i64.eq", &prefix, output),
        Instruction::I64Ne => emit_simple("i64.ne", &prefix, output),
        Instruction::I64LtS => emit_simple("i64.lt_s", &prefix, output),
        Instruction::I64LeS => emit_simple("i64.le_s", &prefix, output),
        Instruction::I64GtS => emit_simple("i64.gt_s", &prefix, output),
        Instruction::I64GeS => emit_simple("i64.ge_s", &prefix, output),

        Instruction::I32Eq => emit_simple("i32.eq", &prefix, output),
        Instruction::I32Ne => emit_simple("i32.ne", &prefix, output),
        Instruction::I32Eqz => emit_simple("i32.eqz", &prefix, output),

        Instruction::F32Add => emit_simple("f32.add", &prefix, output),
        Instruction::F32Sub => emit_simple("f32.sub", &prefix, output),
        Instruction::F32Mul => emit_simple("f32.mul", &prefix, output),
        Instruction::F32Div => emit_simple("f32.div", &prefix, output),
        Instruction::F32Neg => emit_simple("f32.neg", &prefix, output),
        Instruction::F32Eq => emit_simple("f32.eq", &prefix, output),
        Instruction::F32Ne => emit_simple("f32.ne", &prefix, output),
        Instruction::F32Lt => emit_simple("f32.lt", &prefix, output),
        Instruction::F32Le => emit_simple("f32.le", &prefix, output),
        Instruction::F32Gt => emit_simple("f32.gt", &prefix, output),
        Instruction::F32Ge => emit_simple("f32.ge", &prefix, output),

        Instruction::F64Add => emit_simple("f64.add", &prefix, output),
        Instruction::F64Sub => emit_simple("f64.sub", &prefix, output),
        Instruction::F64Mul => emit_simple("f64.mul", &prefix, output),
        Instruction::F64Div => emit_simple("f64.div", &prefix, output),
        Instruction::F64Neg => emit_simple("f64.neg", &prefix, output),
        Instruction::F64Eq => emit_simple("f64.eq", &prefix, output),
        Instruction::F64Ne => emit_simple("f64.ne", &prefix, output),
        Instruction::F64Lt => emit_simple("f64.lt", &prefix, output),
        Instruction::F64Le => emit_simple("f64.le", &prefix, output),
        Instruction::F64Gt => emit_simple("f64.gt", &prefix, output),
        Instruction::F64Ge => emit_simple("f64.ge", &prefix, output),

        Instruction::CallPrint(ty) => {
            let function = match ty {
                Type::String => "$cerune_print_string",
                Type::Bool => "$print_bool",
                Type::I64 => "$print_i64",
                Type::F32 => "$print_f32",
                Type::F64 => "$print_f64",
                Type::DynamicArray | Type::Pointer => {
                    unreachable!("pointers are not printable Cerune values")
                }
            };

            writeln!(output, "{prefix}call {function}").unwrap();
        }
    }
}

fn emit_simple(instruction: &str, prefix: &str, output: &mut String) {
    writeln!(output, "{prefix}{instruction}").unwrap();
}

fn emit_memory(instruction: &str, offset: u32, prefix: &str, output: &mut String) {
    if offset == 0 {
        writeln!(output, "{prefix}{instruction}").unwrap();
    } else {
        writeln!(output, "{prefix}{instruction} offset={offset}").unwrap();
    }
}

fn wat_type(ty: Type) -> &'static str {
    match ty {
        Type::DynamicArray | Type::String | Type::Bool => "i32",
        Type::I64 => "i64",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::Pointer => "i32",
    }
}

fn instruction_uses_bool_print(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::CallPrint(Type::Bool) => true,
        Instruction::If {
            then_instructions,
            else_instructions,
        }
        | Instruction::IfBool {
            then_instructions,
            else_instructions,
        } => {
            then_instructions.iter().any(instruction_uses_bool_print)
                || else_instructions.iter().any(instruction_uses_bool_print)
        }
        Instruction::Loop {
            condition_instructions,
            body_instructions,
            update_instructions,
            ..
        } => {
            condition_instructions
                .iter()
                .any(instruction_uses_bool_print)
                || body_instructions.iter().any(instruction_uses_bool_print)
                || update_instructions.iter().any(instruction_uses_bool_print)
        }
        _ => false,
    }
}

fn module_uses_bool_print(module: &Module) -> bool {
    module.instructions.iter().any(instruction_uses_bool_print)
        || module.functions.iter().any(|function| {
            function
                .instructions
                .iter()
                .any(instruction_uses_bool_print)
        })
}

fn loop_name(kind: LoopKind) -> &'static str {
    match kind {
        LoopKind::While => "while",
        LoopKind::For => "for",
    }
}

fn instruction_uses_u64_print(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::CallPrintU64 => true,
        Instruction::If {
            then_instructions,
            else_instructions,
        }
        | Instruction::IfBool {
            then_instructions,
            else_instructions,
        } => {
            then_instructions.iter().any(instruction_uses_u64_print)
                || else_instructions.iter().any(instruction_uses_u64_print)
        }
        Instruction::Loop {
            condition_instructions,
            body_instructions,
            update_instructions,
            ..
        } => {
            condition_instructions
                .iter()
                .any(instruction_uses_u64_print)
                || body_instructions.iter().any(instruction_uses_u64_print)
                || update_instructions.iter().any(instruction_uses_u64_print)
        }
        _ => false,
    }
}

fn module_uses_u64_print(module: &Module) -> bool {
    module.instructions.iter().any(instruction_uses_u64_print)
        || module
            .functions
            .iter()
            .any(|function| function.instructions.iter().any(instruction_uses_u64_print))
}
