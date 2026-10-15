use super::ir::{BinaryOp, Expr, ExprKind, Module, PrintFormat, Statement, Type, UnaryOp};
use std::fmt::Write;

pub fn emit(module: &Module) -> String {
    let mut output = String::new();
    let support = required_support(module);
    fn has_flush(s: &Statement) -> bool {
        match s {
            Statement::Flush { .. } => true,
            Statement::If {
                then_body,
                else_body,
                ..
            } => then_body.iter().chain(else_body).any(has_flush),
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(has_flush)
            }
            _ => false,
        }
    }
    if module
        .statements
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.body))
        .any(has_flush)
    {
        output.push_str("#include <stdlib.h>\n#include <signal.h>\n");
    }
    let reads = support.input;
    let writes = super::output::used(module);
    let strings = module.uses_strings || support.strings || super::string::uses_type(module);
    if !support.conversions.is_empty() || support.nonfinite {
        output.push_str("#include <math.h>\n#include <float.h>\n");
    }

    if module_uses_bool(module) || strings || module.array_heap_limit.is_some() {
        output.push_str("#include <stdbool.h>\n");
    }

    output.push_str("#include <stdint.h>\n");
    if module.array_heap_limit.is_some() {
        output.push_str("#include <stddef.h>\n#include <assert.h>\n");
    }
    output.push_str("#include <stdio.h>\n");
    if strings {
        output.push_str("#include <stddef.h>\n#include <string.h>\n");
    }
    if writes || strings || module.array_heap_limit.is_some() {
        output.push_str("#ifdef _WIN32\n#include <io.h>\n#include <fcntl.h>\n#endif\n");
    }
    if module.string_heap_limit.is_some() || !module.array_types.is_empty() || support.any_numeric()
    {
        output.push_str("#include <stdlib.h>\n");
    }
    output.push('\n');

    if module.string_heap_limit.is_some() || !module.array_types.is_empty() || support.any_numeric()
    {
        output.push_str(super::failure::SUPPORT);
    }
    emit_i64_operation_support(support, &mut output);
    if strings {
        output.push_str(super::string::SUPPORT);
        if let Some(limit) = module.string_heap_limit {
            output.push_str(&super::heap::support(limit));
        }
    }

    if reads {
        output.push_str(&super::input::support(
            module
                .string_heap_limit
                .expect("input uses string ownership"),
        ));
    }
    if let Some(limit) = module.array_heap_limit {
        output.push_str(&super::array::support(limit));
    }
    if writes {
        output.push_str(&super::output::support(strings));
    }
    let mut emitted_array_types = Vec::new();
    for ty in &module.array_types {
        let (Type::Array { element, .. } | Type::DynamicArray { element }) = ty else {
            unreachable!("array type collection only contains arrays")
        };
        if !type_uses_named(element) {
            emit_array_support_recursive(ty, module, &mut emitted_array_types, &mut output);
        }
    }

    for definition in &module.type_definitions {
        for field in &definition.fields {
            emit_array_support_recursive(&field.ty, module, &mut emitted_array_types, &mut output);
        }
        output.push_str("typedef struct cerune_type_");
        output.push_str(&definition.name);
        output.push('_');
        output.push_str(&definition.id.to_string());
        output.push_str(" {\n");
        for field in &definition.fields {
            output.push_str("    ");
            output.push_str(&c_type(&field.ty, module));
            output.push(' ');
            output.push_str(&field.name);
            output.push_str(";\n");
        }
        output.push_str("} cerune_type_");
        output.push_str(&definition.name);
        output.push('_');
        output.push_str(&definition.id.to_string());
        output.push_str(";\n\n");
    }

    for ty in &module.array_types {
        emit_array_support_recursive(ty, module, &mut emitted_array_types, &mut output);
    }

    for ty in &module.array_types {
        if let Type::DynamicArray { element } = ty {
            emit_dynamic_array_access(element, module, &mut output);
        }
    }
    for function in &module.functions {
        emit_function_signature(function, module, &mut output);
        output.push_str(";\n");
    }
    if !module.functions.is_empty() {
        output.push('\n');
    }

    for function in &module.functions {
        emit_function_signature(function, module, &mut output);
        output.push_str(" {\n");
        emit_temporaries(&function.temporaries, module, &mut output);
        emit_integer_scratch(&function.body, &mut output);
        for statement in &function.body {
            emit_statement(statement, 1, module, &mut output);
        }
        output.push_str("}\n\n");
    }

    output.push_str("int main(void) {\n");
    if writes || strings || module.array_heap_limit.is_some() {
        // WindowsのテキストモードによるLFの書き換えを防ぎ、VMと同じバイトを出力します。
        output.push_str("#ifdef _WIN32\n    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {\n        fputs(\"cerune: cannot set stdout to binary mode\\n\", stderr);\n        return 1;\n    }\n#endif\n");
    }
    emit_temporaries(&module.temporaries, module, &mut output);
    emit_integer_scratch(&module.statements, &mut output);

    for statement in &module.statements {
        emit_statement(statement, 1, module, &mut output);
    }

    if let Some(function_id) = module.explicit_main {
        let function = &module.functions[function_id];
        output.push_str("    ");
        output.push_str(&function_name(function.id, &function.name));
        output.push_str("();\n");
    }

    output.push_str("    return 0;\n");
    output.push_str("}\n");

    output
}

fn emit_function_signature(function: &super::ir::Function, module: &Module, output: &mut String) {
    output.push_str(
        &function
            .return_type
            .as_ref()
            .map_or_else(|| "void".into(), |ty| c_type(ty, module)),
    );
    output.push(' ');
    output.push_str(&function_name(function.id, &function.name));
    output.push('(');
    if function.parameters.is_empty() {
        output.push_str("void");
    } else {
        for (index, parameter) in function.parameters.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            output.push_str(&c_type(&parameter.ty, module));
            output.push_str(" cerune_");
            output.push_str(&parameter.name);
        }
    }
    output.push(')');
}

fn function_name(id: usize, name: &str) -> String {
    format!("cerune_fn_{}_{id}", name.replace('$', "_"))
}

fn emit_temporaries(types: &[Type], module: &Module, output: &mut String) {
    for (id, ty) in types.iter().enumerate() {
        output.push_str(&format!("    {} _cerune_eval_{id};\n", c_type(ty, module)));
    }
}

fn emit_statement(statement: &Statement, indent: usize, module: &Module, output: &mut String) {
    let prefix = "    ".repeat(indent);

    match statement {
        Statement::ArrayInitialize { array, value } => {
            // 配列、値、初期化の順をCの引数評価順に委ねません。
            writeln!(output, "{prefix}{{").unwrap();
            write!(output, "{prefix}    cerune_dynamic_array array = ").unwrap();
            emit_expr(array, module, output);
            write!(
                output,
                ";\n{prefix}    {} value = ",
                c_type(&value.ty, module)
            )
            .unwrap();
            emit_expr(value, module, output);
            writeln!(output, ";\n{prefix}    assert(array.owner && array.owner->references && array.owner->initialized < array.length);").unwrap();
            writeln!(output, "{prefix}    (({} *)array.owner->data)[array.owner->initialized++] = value;\n{prefix}}}", c_type(&value.ty, module)).unwrap();
        }
        Statement::ArrayManage { value, retain } => {
            write!(
                output,
                "{prefix}cerune_array_{}(",
                if *retain {
                    "retain_owner"
                } else {
                    "free_elements"
                }
            )
            .unwrap();
            emit_expr(value, module, output);
            output.push_str(");\n");
        }
        Statement::ArrayRangeCheck {
            origin,
            length,
            start,
            end,
        } => {
            writeln!(output, "{prefix}{{").unwrap();
            for (name, expr) in [("length", length), ("start", start), ("end", end)] {
                write!(output, "{prefix}    int64_t {name} = ").unwrap();
                emit_expr(expr, module, output);
                output.push_str(";\n");
            }
            write!(
                output,
                "{prefix}    cerune_array_check_range(length, start, end"
            )
            .unwrap();
            super::failure::argument(*origin, output);
            writeln!(output, ");\n{prefix}}}").unwrap();
        }
        Statement::StringManage { value, retain } => {
            write!(
                output,
                "{prefix}cerune_string_{}(",
                if *retain { "retain" } else { "release" }
            )
            .unwrap();
            emit_expr(value, module, output);
            output.push_str(");\n");
        }
        Statement::Binding { name, ty, value } => {
            output.push_str(&prefix);
            output.push_str(&c_type(ty, module));
            output.push_str(" cerune_");
            output.push_str(name);
            output.push_str(" = ");

            emit_expr(value, module, output);

            output.push_str(";\n");
        }

        Statement::Assignment { target, value } => {
            if target.projections.is_empty() {
                output.push_str(&prefix);
                output.push_str("cerune_");
                output.push_str(&target.name);
                output.push_str(" = ");
                emit_expr(value, module, output);
                output.push_str(";\n");
            } else {
                output.push_str(&prefix);
                output.push_str("{\n");
                for (index, projection) in target.projections.iter().enumerate() {
                    output.push_str(&prefix);
                    output.push_str("    ");
                    output.push_str(&c_type(&projection.element, module));
                    output.push_str(" *cerune_assignment_target_");
                    output.push_str(&index.to_string());
                    output.push_str(" = ");
                    output.push_str(&match projection.length {
                        Some(length) => array_at_name(&projection.element, length, module),
                        None => dynamic_access_name(&projection.element, "at", module),
                    });
                    output.push('(');
                    if index == 0 {
                        output.push_str("&cerune_");
                        output.push_str(&target.name);
                    } else {
                        output.push_str("cerune_assignment_target_");
                        output.push_str(&(index - 1).to_string());
                    }
                    output.push_str(", ");
                    emit_expr(&projection.index, module, output);
                    super::failure::argument(projection.origin, output);
                    output.push(')');
                    output.push_str(";\n");
                }
                output.push_str(&prefix);
                output.push_str("    ");
                output.push_str(&c_type(&target.ty, module));
                output.push_str(" cerune_assignment_value = ");
                emit_expr(value, module, output);
                output.push_str(";\n");
                output.push_str(&prefix);
                output.push_str("    *cerune_assignment_target_");
                output.push_str(&(target.projections.len() - 1).to_string());
                output.push_str(" = cerune_assignment_value;\n");
                output.push_str(&prefix);
                output.push_str("}\n");
            }
        }

        Statement::Write {
            kind,
            value,
            origin,
        } => {
            emit_output(kind, value, false, *origin, &prefix, module, output);
        }
        Statement::Flush { origin } => {
            let record =
                crate::runtime::output::flush_failure_record(crate::runtime::output::Origin {
                    node_id: origin.node_id,
                    span: origin.span,
                });
            writeln!(output, "{prefix}{{\n{prefix}    int cerune_flush_status;\n#ifdef SIGPIPE\n{prefix}    void (*cerune_prior_sigpipe)(int) = signal(SIGPIPE, SIG_IGN);\n{prefix}    cerune_flush_status = cerune_prior_sigpipe == SIG_ERR ? EOF : fflush(stdout);\n{prefix}    if (cerune_flush_status == 0 && signal(SIGPIPE, cerune_prior_sigpipe) == SIG_ERR) cerune_flush_status = EOF;\n#else\n{prefix}    cerune_flush_status = fflush(stdout);\n#endif\n{prefix}    if (cerune_flush_status != 0) {{ fputs(\"cerune: {record}\\n\", stderr); fflush(stderr); _Exit(1); }}\n{prefix}}}").unwrap();
        }
        Statement::Print {
            format,
            value,
            origin,
        } => {
            let kind = match format {
                PrintFormat::String => "string",
                PrintFormat::Bool => "bool",
                PrintFormat::U64 => "u64",
                PrintFormat::I64 => "i64",
                PrintFormat::F32 => "f32",
                PrintFormat::F64 => "f64",
            };
            emit_output(kind, value, true, *origin, &prefix, module, output);
        }

        Statement::Call {
            evaluation,
            function_id,
            function_name,
            arguments,
        } => {
            for (id, value) in evaluation {
                output.push_str(&format!("{prefix}_cerune_eval_{id} = "));
                emit_expr(value, module, output);
                output.push_str(";\n");
            }
            output.push_str(&prefix);
            emit_call(*function_id, function_name, arguments, module, output);
            output.push_str(";\n");
        }

        Statement::Return(value) => {
            output.push_str(&prefix);
            output.push_str("return");
            if let Some(value) = value {
                output.push(' ');
                emit_expr(value, module, output);
            }
            output.push_str(";\n");
        }

        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            output.push_str(&prefix);
            output.push_str("if ");
            emit_condition(condition, module, output);
            output.push_str(" {\n");

            for statement in then_body {
                emit_statement(statement, indent + 1, module, output);
            }

            output.push_str(&prefix);
            output.push('}');

            if else_body.is_empty() {
                output.push('\n');
            } else {
                output.push_str(" else {\n");
                for statement in else_body {
                    emit_statement(statement, indent + 1, module, output);
                }
                output.push_str(&prefix);
                output.push_str("}\n");
            }
        }

        Statement::While { condition, body } => {
            output.push_str(&prefix);
            output.push_str("while ");
            emit_condition(condition, module, output);
            output.push_str(" {\n");

            for statement in body {
                emit_statement(statement, indent + 1, module, output);
            }

            output.push_str(&prefix);
            output.push_str("}\n");
        }

        Statement::For {
            initializer,
            condition,
            update,
            body,
        } => {
            // ループを抜けた直後の所有解放から初期化束縛を参照できるようにします。
            // 名前はBindingIdで一意なので、Ceruneの可視範囲は広がりません。
            let lifted = (module.string_heap_limit.is_some() || module.array_heap_limit.is_some())
                && matches!(initializer.as_ref(), Statement::Binding { .. });
            if lifted {
                emit_statement(initializer, indent, module, output);
            }
            output.push_str(&prefix);
            output.push_str("for (");
            if !lifted {
                emit_for_clause(initializer, module, output);
            }
            output.push_str("; ");
            emit_expr(condition, module, output);
            output.push_str("; ");
            emit_for_clause(update, module, output);
            output.push_str(") {\n");

            for statement in body {
                emit_statement(statement, indent + 1, module, output);
            }

            output.push_str(&prefix);
            output.push_str("}\n");
        }

        Statement::Break => {
            output.push_str(&prefix);
            output.push_str("break;\n");
        }

        Statement::Continue => {
            output.push_str(&prefix);
            output.push_str("continue;\n");
        }
    }
}

fn emit_for_clause(statement: &Statement, module: &Module, output: &mut String) {
    match statement {
        Statement::Binding { name, ty, value } => {
            output.push_str(&c_type(ty, module));
            output.push_str(" cerune_");
            output.push_str(name);
            output.push_str(" = ");
            emit_expr(value, module, output);
        }
        Statement::Assignment { target, value } => {
            debug_assert!(target.projections.is_empty());
            output.push_str("cerune_");
            output.push_str(&target.name);
            output.push_str(" = ");
            emit_expr(value, module, output);
        }
        _ => unreachable!("for clauses are validated by the parser"),
    }
}

fn c_type(ty: &Type, module: &Module) -> String {
    match ty {
        Type::DynamicArray { .. } => "cerune_dynamic_array".into(),
        Type::Bool => "bool".into(),
        Type::String => "cerune_string".into(),
        Type::U64 => "uint64_t".into(),
        Type::I64 => "int64_t".into(),
        Type::Float => "float".into(),
        Type::Double => "double".into(),
        Type::Named(id) => {
            let definition = module
                .type_definitions
                .iter()
                .find(|definition| definition.id == *id)
                .expect("named C type must have a definition");
            format!("cerune_type_{}_{}", definition.name, id)
        }
        Type::Array { element, length } => array_type_name(element, *length, module),
    }
}

fn emit_output(
    kind: &str,
    expr: &Expr,
    newline: bool,
    origin: super::ir::Origin,
    prefix: &str,
    module: &Module,
    output: &mut String,
) {
    write!(output, "{prefix}cerune_output_{kind}(").unwrap();
    emit_expr(expr, module, output);
    write!(output, ", {}", u8::from(newline)).unwrap();
    super::failure::argument(origin, output);
    output.push_str(");\n");
}

fn emit_expr(expr: &Expr, module: &Module, output: &mut String) {
    match &expr.kind {
        ExprKind::ArrayAllocate {
            length,
            element_width,
        } => {
            let Type::DynamicArray { element } = &expr.ty else {
                unreachable!()
            };
            output.push_str("cerune_array_allocate_elements(");
            emit_expr(length, module, output);
            write!(
                output,
                ", UINT64_C({element_width}), sizeof({})",
                c_type(element, module)
            )
            .unwrap();
            super::failure::argument(expr.origin, output);
            output.push(')');
        }
        ExprKind::ArrayReleaseOwner { value } => {
            output.push_str("cerune_array_release_owner_last(");
            emit_expr(value, module, output);
            output.push(')');
        }
        ExprKind::ArrayLength { value, length } => {
            if let Some(length) = length {
                // sizeofでは評価が消えるため、カンマ式で引数の評価を残します。
                output.push_str("((void)(");
                emit_expr(value, module, output);
                write!(output, "), INT64_C({length}))").unwrap();
            } else {
                output.push('(');
                emit_expr(value, module, output);
                output.push_str(").length");
            }
        }
        ExprKind::ReadLine { max_bytes } => {
            output.push_str("cerune_input_read_line(");
            emit_expr(max_bytes, module, output);
            super::failure::argument(expr.origin, output);
            output.push(')');
        }
        ExprKind::StringConcat { left, right } => {
            output.push_str("cerune_string_concat(");
            emit_expr(left, module, output);
            output.push_str(", ");
            emit_expr(right, module, output);
            super::failure::argument(expr.origin, output);
            output.push(')');
        }
        ExprKind::StringByteLength { value } => {
            output.push_str("((int64_t)(");
            emit_expr(value, module, output);
            output.push_str(").length)");
        }
        ExprKind::String(value) => super::string::literal(value, output),
        ExprKind::Temporary(id) => output.push_str(&format!("_cerune_eval_{id}")),
        ExprKind::Sequence { bindings, value } => {
            output.push('(');
            for (id, value) in bindings {
                output.push_str(&format!("_cerune_eval_{id} = "));
                emit_expr(value, module, output);
                output.push_str(", ");
            }
            emit_expr(value, module, output);
            output.push(')');
        }
        ExprKind::ConvertNumeric { value, conversion } => {
            output.push_str(&conversion.helper());
            output.push('(');
            emit_expr(value, module, output);
            super::failure::argument(expr.origin, output);
            output.push(')');
        }
        ExprKind::IntegerBinary {
            scratch,
            op,
            ty,
            left,
            right,
        } => {
            output.push_str(&format!("(_cerune_bit_left_{scratch} = "));
            emit_expr(left, module, output);
            output.push_str(&format!(", _cerune_bit_right_{scratch} = "));
            emit_expr(right, module, output);
            output.push_str(&format!(
                ", {}(_cerune_bit_left_{scratch}, _cerune_bit_right_{scratch}",
                op.helper(*ty)
            ));
            super::failure::argument(expr.origin, output);
            output.push_str("))");
        }
        ExprKind::CheckIntegerRange { value, ty, code } => {
            output.push_str(&format!("cerune_check_{}(", ty.name()));
            emit_expr(value, module, output);
            write!(output, ", \"{}\"", code.name()).unwrap();
            super::failure::argument(expr.origin, output);
            output.push(')');
        }
        ExprKind::Boolean(value) => {
            output.push_str(if *value { "true" } else { "false" });
        }

        ExprKind::Integer(value) => {
            if expr.ty == Type::U64 {
                write!(output, "UINT64_C({})", *value as u64).unwrap();
            } else if *value == i64::MIN as i128 {
                output.push_str("INT64_MIN");
            } else {
                output.push_str(&value.to_string());
            }
        }

        ExprKind::Float { text, suffix_f32 } => {
            if let Some(special) = match text.as_str() {
                "nan" => Some("NAN"),
                "inf" => Some("INFINITY"),
                "-inf" => Some("(-INFINITY)"),
                _ => None,
            } {
                output.push_str(special);
                return;
            }
            output.push_str(text);

            if *suffix_f32 {
                output.push('f');
            }
        }

        ExprKind::Variable(name) => {
            output.push_str("cerune_");
            output.push_str(name);
        }

        ExprKind::Construct {
            type_id,
            base,
            fields,
            copy_temp,
        } => {
            if let Some(base) = base {
                let temp = copy_temp.expect("update has a C sequencing temporary");
                write!(output, "(_cerune_eval_{temp} = ").unwrap();
                emit_expr(base, module, output);
                for field in fields {
                    write!(output, ", _cerune_eval_{temp}.{} = ", field.name).unwrap();
                    emit_expr(&field.value, module, output);
                }
                write!(output, ", _cerune_eval_{temp})").unwrap();
                return;
            }
            output.push('(');
            output.push_str(&c_type(&Type::Named(*type_id), module));
            output.push_str("){ ");
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                output.push('.');
                output.push_str(&field.name);
                output.push_str(" = ");
                emit_expr(&field.value, module, output);
            }
            output.push_str(" }");
        }

        ExprKind::FieldAccess { field_name, base } => {
            output.push('(');
            emit_expr(base, module, output);
            output.push_str(").");
            output.push_str(field_name);
        }

        ExprKind::Array(values) => {
            output.push('(');
            output.push_str(&c_type(&expr.ty, module));
            output.push_str("){ .items = { ");
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                emit_expr(value, module, output);
            }
            output.push_str(" } }");
        }

        ExprKind::Index { base, index } => {
            output.push_str(&match &base.ty {
                Type::Array { element, length } => array_get_name(element, *length, module),
                Type::DynamicArray { element } => dynamic_access_name(element, "get", module),
                _ => unreachable!("indexed expression must have an array base"),
            });
            output.push('(');
            emit_expr(base, module, output);
            output.push_str(", ");
            emit_expr(index, module, output);
            super::failure::argument(expr.origin, output);
            output.push(')');
        }

        ExprKind::Call {
            function_id,
            function_name,
            arguments,
        } => emit_call(*function_id, function_name, arguments, module, output),

        ExprKind::Unary { op, value } => {
            if *op == UnaryOp::CheckedI64Negate {
                output.push_str("cerune_i64_neg(");
                emit_expr(value, module, output);
                super::failure::argument(expr.origin, output);
                output.push(')');
            } else {
                output.push('(');
                output.push_str(match op {
                    UnaryOp::CheckedI64Negate => unreachable!(),
                    UnaryOp::Negate => "-",
                    UnaryOp::Not => "!",
                });
                emit_expr(value, module, output);
                output.push(')');
            }
        }

        ExprKind::Logical { op, left, right } => {
            output.push('(');
            emit_expr(left, module, output);
            output.push_str(match op {
                super::ir::LogicalOp::And => " && ",
                super::ir::LogicalOp::Or => " || ",
            });
            emit_expr(right, module, output);
            output.push(')');
        }
        ExprKind::Binary { op, left, right } => {
            if left.ty == Type::String {
                output.push('(');
                if *op == BinaryOp::NotEqual {
                    output.push('!');
                } else {
                    assert_eq!(
                        *op,
                        BinaryOp::Equal,
                        "string ordering is rejected by semantics"
                    );
                }
                output.push_str("cerune_string_equal(");
                emit_expr(left, module, output);
                output.push_str(", ");
                emit_expr(right, module, output);
                output.push_str("))");
                return;
            }
            let helper = match op {
                BinaryOp::CheckedI64Add => Some("cerune_i64_add"),
                BinaryOp::CheckedI64Subtract => Some("cerune_i64_sub"),
                BinaryOp::CheckedI64Multiply => Some("cerune_i64_mul"),
                BinaryOp::CheckedI64Divide => Some("cerune_i64_div"),
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => None,
            };
            if let Some(helper) = helper {
                output.push_str(helper);
                output.push('(');
                emit_expr(left, module, output);
                output.push_str(", ");
                emit_expr(right, module, output);
                super::failure::argument(expr.origin, output);
                output.push(')');
                return;
            }

            output.push('(');

            emit_expr(left, module, output);

            output.push(' ');

            output.push_str(match op {
                BinaryOp::CheckedI64Add
                | BinaryOp::CheckedI64Subtract
                | BinaryOp::CheckedI64Multiply
                | BinaryOp::CheckedI64Divide => unreachable!(),
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
                BinaryOp::Equal => "==",
                BinaryOp::NotEqual => "!=",
                BinaryOp::Less => "<",
                BinaryOp::LessEqual => "<=",
                BinaryOp::Greater => ">",
                BinaryOp::GreaterEqual => ">=",
            });

            output.push(' ');

            emit_expr(right, module, output);

            output.push(')');
        }
    }
}

#[derive(Clone, Default)]
struct RuntimeSupport {
    input: bool,
    strings: bool,
    nonfinite: bool,
    conversions: std::collections::BTreeSet<crate::codegen::NumericConversion>,
    scratch: std::collections::BTreeSet<usize>,
    integer_binary:
        std::collections::BTreeSet<(crate::codegen::IntegerBinaryOp, crate::types::IntegerType)>,
    range_checks: std::collections::BTreeSet<crate::types::IntegerType>,
    add: bool,
    subtract: bool,
    multiply: bool,
    divide: bool,
    negate: bool,
}

impl RuntimeSupport {
    fn any_numeric(&self) -> bool {
        !self.conversions.is_empty()
            || !self.integer_binary.is_empty()
            || !self.range_checks.is_empty()
            || self.add
            || self.subtract
            || self.multiply
            || self.divide
            || self.negate
    }

    fn include_expr(&mut self, expr: &Expr) {
        self.strings |= expr.ty == Type::String;
        self.nonfinite |= matches!(&expr.kind, ExprKind::Float { text, .. } if matches!(text.as_str(), "nan" | "inf" | "-inf"));
        match &expr.kind {
            ExprKind::ReadLine { max_bytes } => {
                self.input = true;
                self.include_expr(max_bytes);
            }
            ExprKind::ArrayAllocate { length: value, .. }
            | ExprKind::ArrayReleaseOwner { value }
            | ExprKind::ArrayLength { value, .. }
            | ExprKind::StringByteLength { value } => self.include_expr(value),
            ExprKind::Sequence { bindings, value } => {
                for (_, value) in bindings {
                    self.include_expr(value);
                }
                self.include_expr(value);
            }
            ExprKind::ConvertNumeric { value, conversion } => {
                self.conversions.insert(*conversion);
                self.include_expr(value);
            }
            ExprKind::IntegerBinary {
                scratch,
                op,
                ty,
                left,
                right,
            } => {
                self.scratch.insert(*scratch);
                self.integer_binary.insert((*op, *ty));
                self.include_expr(left);
                self.include_expr(right);
            }
            ExprKind::CheckIntegerRange { value, ty, .. } => {
                self.include_expr(value);
                if *ty != crate::types::IntegerType::I64 {
                    self.range_checks.insert(*ty);
                }
            }
            ExprKind::Unary { op, value } => {
                self.include_expr(value);
                if *op == UnaryOp::CheckedI64Negate {
                    self.negate = true;
                }
            }
            ExprKind::Binary { op, left, right } => {
                self.include_expr(left);
                self.include_expr(right);
                match op {
                    BinaryOp::CheckedI64Add => self.add = true,
                    BinaryOp::CheckedI64Subtract => self.subtract = true,
                    BinaryOp::CheckedI64Multiply => self.multiply = true,
                    BinaryOp::CheckedI64Divide => self.divide = true,
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {}
                }
            }
            ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    self.include_expr(base);
                }
                for field in fields {
                    self.include_expr(&field.value);
                }
            }
            ExprKind::FieldAccess { base, .. } => self.include_expr(base),
            ExprKind::Array(values) => {
                for value in values {
                    self.include_expr(value);
                }
            }
            ExprKind::StringConcat {
                left: base,
                right: index,
            }
            | ExprKind::Logical {
                left: base,
                right: index,
                ..
            }
            | ExprKind::Index { base, index } => {
                self.include_expr(base);
                self.include_expr(index);
            }
            ExprKind::Call { arguments, .. } => {
                for argument in arguments {
                    self.include_expr(argument);
                }
            }
            ExprKind::Boolean(_)
            | ExprKind::String(_)
            | ExprKind::Temporary(_)
            | ExprKind::Integer(_)
            | ExprKind::Float { .. }
            | ExprKind::Variable(_) => {}
        }
    }

    fn include_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Flush { .. } => {}
            Statement::ArrayInitialize { array, value } => {
                self.include_expr(array);
                self.include_expr(value);
            }
            Statement::ArrayRangeCheck {
                length, start, end, ..
            } => {
                for value in [length, start, end] {
                    self.include_expr(value);
                }
            }
            Statement::ArrayManage { value, .. }
            | Statement::Binding { value, .. }
            | Statement::StringManage { value, .. }
            | Statement::Write { value, .. }
            | Statement::Print { value, .. } => self.include_expr(value),
            Statement::Assignment { target, value } => {
                self.include_expr(value);
                for projection in &target.projections {
                    self.include_expr(&projection.index);
                }
            }
            Statement::Call {
                evaluation,
                arguments,
                ..
            } => {
                for (_, value) in evaluation {
                    self.include_expr(value);
                }
                for argument in arguments {
                    self.include_expr(argument);
                }
            }
            Statement::Return(value) => {
                if let Some(value) = value {
                    self.include_expr(value);
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                self.include_expr(condition);
                for statement in then_body.iter().chain(else_body) {
                    self.include_statement(statement);
                }
            }
            Statement::While { condition, body } => {
                self.include_expr(condition);
                for statement in body {
                    self.include_statement(statement);
                }
            }
            Statement::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.include_statement(initializer);
                self.include_expr(condition);
                self.include_statement(update);
                for statement in body {
                    self.include_statement(statement);
                }
            }
            Statement::Break | Statement::Continue => {}
        }
    }
}

// 一時変数は関数内に置き、短絡評価やループ内で式が実行される位置は変えません。
fn emit_integer_scratch(statements: &[Statement], output: &mut String) {
    let mut operations = RuntimeSupport::default();
    for statement in statements {
        operations.include_statement(statement);
    }
    for scratch in operations.scratch {
        output.push_str(&format!(
            "    int64_t _cerune_bit_left_{scratch}, _cerune_bit_right_{scratch};\n"
        ));
    }
}

fn required_support(module: &Module) -> RuntimeSupport {
    let mut operations = RuntimeSupport::default();
    for statement in &module.statements {
        operations.include_statement(statement);
    }
    for function in &module.functions {
        for statement in &function.body {
            operations.include_statement(statement);
        }
    }
    operations
}

fn emit_i64_operation_support(operations: RuntimeSupport, output: &mut String) {
    for &conversion in &operations.conversions {
        super::conversion::emit_support(conversion, output);
    }
    for &(op, ty) in &operations.integer_binary {
        super::integer::emit_support(op, ty, output);
    }
    for ty in &operations.range_checks {
        output.push_str(&format!("static int64_t cerune_check_{}(int64_t value, const char *code, const char *origin) {{\n    if (value < {}LL || value > {}LL) cerune_runtime_fail(code, origin);\n    return value;\n}}\n\n", ty.name(), ty.minimum(), ty.maximum()));
    }

    if !operations.any_numeric() {
        return;
    }

    if operations.add {
        output.push_str(
            "static int64_t cerune_i64_add(int64_t left, int64_t right, const char *origin) {\n",
        );
        output.push_str("    if ((right > 0 && left > INT64_MAX - right) ||\n");
        output.push_str("        (right < 0 && left < INT64_MIN - right)) {\n");
        output.push_str("        cerune_runtime_fail(\"integer-overflow\", origin);\n    }\n");
        output.push_str("    return left + right;\n}\n\n");
    }

    if operations.subtract {
        output.push_str(
            "static int64_t cerune_i64_sub(int64_t left, int64_t right, const char *origin) {\n",
        );
        output.push_str("    if ((right < 0 && left > INT64_MAX + right) ||\n");
        output.push_str("        (right > 0 && left < INT64_MIN + right)) {\n");
        output.push_str("        cerune_runtime_fail(\"integer-overflow\", origin);\n    }\n");
        output.push_str("    return left - right;\n}\n\n");
    }

    if operations.multiply {
        output.push_str(
            "static int64_t cerune_i64_mul(int64_t left, int64_t right, const char *origin) {\n",
        );
        output.push_str("    if ((left > 0 && right > 0 && left > INT64_MAX / right) ||\n");
        output.push_str("        (left > 0 && right < 0 && right < INT64_MIN / left) ||\n");
        output.push_str("        (left < 0 && right > 0 && left < INT64_MIN / right) ||\n");
        output.push_str("        (left < 0 && right < 0 && left < INT64_MAX / right)) {\n");
        output.push_str("        cerune_runtime_fail(\"integer-overflow\", origin);\n    }\n");
        output.push_str("    return left * right;\n}\n\n");
    }

    if operations.divide {
        output.push_str(
            "static int64_t cerune_i64_div(int64_t left, int64_t right, const char *origin) {\n",
        );
        output.push_str("    if (right == 0) {\n");
        output.push_str("        cerune_runtime_fail(\"division-by-zero\", origin);\n    }\n");
        output.push_str("    if (left == INT64_MIN && right == -1) {\n");
        output.push_str("        cerune_runtime_fail(\"division-overflow\", origin);\n    }\n");
        output.push_str("    return left / right;\n}\n\n");
    }

    if operations.negate {
        output.push_str("static int64_t cerune_i64_neg(int64_t value, const char *origin) {\n");
        output.push_str("    if (value == INT64_MIN) {\n");
        output.push_str("        cerune_runtime_fail(\"integer-overflow\", origin);\n    }\n");
        output.push_str("    return -value;\n}\n\n");
    }
}

fn emit_call(
    function_id: usize,
    name: &str,
    arguments: &[Expr],
    module: &Module,
    output: &mut String,
) {
    output.push_str(&function_name(function_id, name));
    output.push('(');
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        emit_expr(argument, module, output);
    }
    output.push(')');
}

fn emit_condition(expr: &Expr, module: &Module, output: &mut String) {
    if matches!(&expr.kind, ExprKind::Unary { .. } | ExprKind::Binary { .. }) {
        emit_expr(expr, module, output);
    } else {
        output.push('(');
        emit_expr(expr, module, output);
        output.push(')');
    }
}

fn module_uses_bool(module: &Module) -> bool {
    module.temporaries.iter().any(type_uses_bool)
        || module
            .type_definitions
            .iter()
            .flat_map(|definition| &definition.fields)
            .any(|field| type_uses_bool(&field.ty))
        || module.array_types.iter().any(type_uses_bool)
        || module.statements.iter().any(statement_uses_bool)
        || module.functions.iter().any(|function| {
            function.temporaries.iter().any(type_uses_bool)
                || function.return_type.as_ref().is_some_and(type_uses_bool)
                || function
                    .parameters
                    .iter()
                    .any(|parameter| type_uses_bool(&parameter.ty))
                || function.body.iter().any(statement_uses_bool)
        })
}

fn type_uses_bool(ty: &Type) -> bool {
    match ty {
        Type::Bool => true,
        Type::Array { element, .. } | Type::DynamicArray { element } => type_uses_bool(element),
        Type::String | Type::U64 | Type::I64 | Type::Float | Type::Double | Type::Named(_) => false,
    }
}

fn type_uses_named(ty: &Type) -> bool {
    match ty {
        Type::DynamicArray { .. } => false,
        Type::Named(_) => true,
        Type::Array { element, .. } => type_uses_named(element),
        Type::Bool | Type::String | Type::U64 | Type::I64 | Type::Float | Type::Double => false,
    }
}

fn emit_array_support_recursive(
    ty: &Type,
    module: &Module,
    emitted: &mut Vec<Type>,
    output: &mut String,
) {
    let Type::Array { element, .. } = ty else {
        return;
    };

    emit_array_support_recursive(element, module, emitted, output);
    if !emitted.contains(ty) {
        emit_array_support(ty, module, output);
        emitted.push(ty.clone());
    }
}

fn emit_array_support(ty: &Type, module: &Module, output: &mut String) {
    let Type::Array { element, length } = ty else {
        unreachable!("array support requires an array type")
    };
    let name = array_type_name(element, *length, module);
    output.push_str("typedef struct ");
    output.push_str(&name);
    output.push_str(" {\n    ");
    output.push_str(&c_type(element, module));
    output.push_str(" items[");
    output.push_str(&length.to_string());
    output.push_str("];\n} ");
    output.push_str(&name);
    output.push_str(";\n\n");

    output.push_str("static ");
    output.push_str(&c_type(element, module));
    output.push(' ');
    output.push_str(&array_get_name(element, *length, module));
    output.push('(');
    output.push_str(&name);
    output.push_str(" value, int64_t index, const char *origin) {\n");
    output.push_str("    if (index < 0 || index >= ");
    output.push_str(&length.to_string());
    output.push_str(") {\n");
    output.push_str("        cerune_runtime_fail(\"array-index-out-of-bounds\", origin);\n    }\n");
    output.push_str("    return value.items[index];\n}\n\n");

    if module.array_assignment_types.contains(ty) {
        output.push_str("static ");
        output.push_str(&c_type(element, module));
        output.push_str(" *");
        output.push_str(&array_at_name(element, *length, module));
        output.push('(');
        output.push_str(&name);
        output.push_str(" *value, int64_t index, const char *origin) {\n");
        output.push_str("    if (index < 0 || index >= ");
        output.push_str(&length.to_string());
        output.push_str(") {\n");
        output.push_str(
            "        cerune_runtime_fail(\"array-index-out-of-bounds\", origin);\n    }\n",
        );
        output.push_str("    return &value->items[index];\n}\n\n");
    }
}

fn array_type_name(element: &Type, length: usize, module: &Module) -> String {
    format!(
        "cerune_array_{}_{}",
        array_element_name(element, module),
        length
    )
}

fn array_get_name(element: &Type, length: usize, module: &Module) -> String {
    format!(
        "cerune_array_get_{}_{}",
        array_element_name(element, module),
        length
    )
}

fn array_at_name(element: &Type, length: usize, module: &Module) -> String {
    format!(
        "cerune_array_at_{}_{}",
        array_element_name(element, module),
        length
    )
}

fn array_element_name(element: &Type, module: &Module) -> String {
    match element {
        Type::DynamicArray { element } => {
            format!("dynamic_{}", array_element_name(element, module))
        }
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::U64 => "u64".into(),
        Type::I64 => "i64".into(),
        Type::Float => "f32".into(),
        Type::Double => "f64".into(),
        Type::Named(id) => {
            let definition = module
                .type_definitions
                .iter()
                .find(|definition| definition.id == *id)
                .expect("C IR should contain the referenced product type");
            format!("type_{}_{}", definition.name, id)
        }
        Type::Array { element, length } => {
            format!("array_{}_{}", array_element_name(element, module), length)
        }
    }
}

fn statement_uses_bool(statement: &Statement) -> bool {
    match statement {
        Statement::Flush { .. } => false,
        Statement::ArrayInitialize { array, value } => {
            array.ty == Type::Bool || value.ty == Type::Bool
        }
        Statement::ArrayRangeCheck { .. } => false,
        Statement::ArrayManage { value, .. } => value.ty == Type::Bool,
        Statement::Binding { ty, value, .. } => *ty == Type::Bool || value.ty == Type::Bool,
        Statement::StringManage { value, .. }
        | Statement::Assignment { value, .. }
        | Statement::Write { value, .. } => value.ty == Type::Bool,
        Statement::Print { format, value, .. } => {
            *format == PrintFormat::Bool || value.ty == Type::Bool
        }
        Statement::Call { arguments, .. } => arguments.iter().any(|value| value.ty == Type::Bool),
        Statement::Return(value) => value.as_ref().is_some_and(|value| value.ty == Type::Bool),
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            condition.ty == Type::Bool
                || then_body.iter().any(statement_uses_bool)
                || else_body.iter().any(statement_uses_bool)
        }
        Statement::While { condition, body } => {
            condition.ty == Type::Bool || body.iter().any(statement_uses_bool)
        }
        Statement::For {
            initializer,
            condition,
            update,
            body,
        } => {
            statement_uses_bool(initializer)
                || condition.ty == Type::Bool
                || statement_uses_bool(update)
                || body.iter().any(statement_uses_bool)
        }
        Statement::Break | Statement::Continue => false,
    }
}

// 型付きのload/storeだけを生成し、要素のコピー規則は共通IRに残します。
fn dynamic_access_name(element: &Type, operation: &str, module: &Module) -> String {
    format!(
        "cerune_dynamic_{operation}_{}",
        array_element_name(element, module)
    )
}
fn emit_dynamic_array_access(element: &Type, module: &Module, output: &mut String) {
    let ty = c_type(element, module);
    let get = dynamic_access_name(element, "get", module);
    let at = dynamic_access_name(element, "at", module);
    writeln!(
        output,
        r#"static {ty} {get}(cerune_dynamic_array value, int64_t index, const char *origin) {{
    if (index < 0 || index >= value.length)
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    assert(value.owner && index < value.owner->initialized);
    return (({ty} *)value.owner->data)[index];
}}
static {ty} *{at}(cerune_dynamic_array *value, int64_t index, const char *origin) {{
    if (index < 0 || index >= value->length)
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    assert(value->owner && index < value->owner->initialized);
    return &(({ty} *)value->owner->data)[index];
}}
"#
    )
    .unwrap();
}
