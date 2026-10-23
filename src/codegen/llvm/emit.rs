use std::fmt::Write;

use super::ir::{
    BinaryOp, CompareOp, Function, Instruction, Label, Module, Operand, Origin, Slot, SlotId, Temp,
    Type,
};

pub fn emit(module: &Module) -> String {
    emit_with_origins(module, false)
}

pub fn emit_with_origins(module: &Module, annotate_origins: bool) -> String {
    let mut output = String::new();
    if annotate_origins {
        output.push_str("; cerune-origins v1: UTF-8 byte ranges, end exclusive\n");
    }
    emit_origin(Origin::Synthetic, annotate_origins, &mut output);
    let i64_operations = i64_operations(module);
    let binary_stdout =
        module.uses_strings || module.array_heap_limit.is_some() || super::output::used(module);
    let runtime_failures = module.array_heap_limit.is_some()
        || module.string_heap_limit.is_some()
        || super::failure::first_failure_span(module).is_some();
    if let Some(target) = module.target {
        writeln!(output, "target triple = \"{}\"\n", target.triple()).unwrap();
    }
    if runtime_failures {
        super::failure::emit_data(module, &mut output);
    }
    if module.uses_strings {
        super::string::emit_data(module, &mut output);
    }

    if module.array_heap_limit.is_some() {
        output.push_str("%cerune.array = type { ptr, i64 }\n%cerune.array.owner = type { ptr, i64, i64, i64 }\n");
    }
    for definition in &module.type_definitions {
        write!(
            output,
            "%cerune.type.{}.{} = type {{ ",
            definition.name, definition.id
        )
        .unwrap();
        for (index, field) in definition.fields.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            output.push_str(&type_name(field, module));
        }
        output.push_str(" }\n");
    }

    output.push('\n');

    let array_types = array_types(module);
    let array_set_types = array_set_types(module);
    if module.array_heap_limit.is_some()
        || module.string_heap_limit.is_some()
        || !array_types.is_empty()
        || i64_operations.any()
    {
        output.push_str("declare void @llvm.trap()\n");
    }

    if i64_operations.add {
        output.push_str("declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)\n");
    }
    if i64_operations.subtract {
        output.push_str("declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)\n");
    }
    if i64_operations.multiply {
        output.push_str("declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)\n");
    }

    output.push('\n');

    if runtime_failures {
        super::failure::emit_support(module, &mut output);
    }
    emit_i64_operation_support(i64_operations, &mut output);
    if module.uses_strings {
        super::string::emit_support(module, &mut output);
        if let Some(limit) = module.string_heap_limit {
            output.push_str(&super::heap::support(limit));
        }
    }

    if let Some(limit) = module.array_heap_limit {
        if !module.uses_strings || module.string_heap_limit.is_none() {
            output.push_str("declare ptr @malloc(i64)\ndeclare void @free(ptr)\n");
        }
        if !module.uses_strings && module.target == Some(super::Target::X86_64PcWindowsMsvc) {
            output.push_str("declare i32 @_setmode(i32, i32)\n");
        }
        output.push_str(&super::array::support(limit));
    }
    if binary_stdout
        && !module.uses_strings
        && module.array_heap_limit.is_none()
        && module.target == Some(super::Target::X86_64PcWindowsMsvc)
    {
        output.push_str("declare i32 @_setmode(i32, i32)\n");
    }
    super::output::emit_support(module, &mut output);
    super::arguments::emit_support(module, &mut output);
    super::input::emit_support(module, &mut output);
    for ty in &array_types {
        emit_array_get(ty, module, &mut output);
        output.push('\n');
        if array_set_types.contains(ty) {
            emit_array_set(ty, module, &mut output);
            output.push('\n');
        }
    }

    for function in &module.functions {
        emit_function(function, module, annotate_origins, &mut output);
        output.push('\n');
    }

    emit_origin(Origin::Synthetic, annotate_origins, &mut output);
    output.push_str(if super::arguments::used(module) {
        "define internal i32 @cerune.program() {\nentry:\n"
    } else {
        "define i32 @main() {\nentry:\n"
    });
    if binary_stdout && module.target == Some(super::Target::X86_64PcWindowsMsvc) {
        // CRTの標準出力(記述子1)を、最初の出力より前にバイナリモードにします。
        output.push_str("  %stdout.mode = call i32 @_setmode(i32 1, i32 32768)\n");
        output.push_str("  %stdout.failed = icmp eq i32 %stdout.mode, -1\n");
        output.push_str("  br i1 %stdout.failed, label %stdout_error, label %stdout_ready\n");
        output.push_str("stdout_error:\n  ret i32 1\nstdout_ready:\n");
    }

    for slot in &module.slots {
        writeln!(
            output,
            "  %cerune_{} = alloca {}",
            slot.name,
            type_name(&slot.ty, module),
        )
        .unwrap();
    }

    for instruction in &module.instructions {
        emit_origin(instruction.origin, annotate_origins, &mut output);
        emit_instruction(
            &instruction.instruction,
            instruction.origin,
            &module.slots,
            module,
            &mut output,
        );
    }

    emit_origin(Origin::Synthetic, annotate_origins, &mut output);
    if let Some(function_id) = module.explicit_main {
        let function = &module.functions[function_id];
        writeln!(output, "  call void @{}()", function_name(function)).unwrap();
    }

    output.push_str("  ret i32 0\n");
    output.push_str("}\n");

    super::arguments::emit_entry(module, &mut output);
    crate::codegen::output::llvm(module, &mut output);
    output
}

fn emit_function(
    function: &Function,
    module: &Module,
    annotate_origins: bool,
    output: &mut String,
) {
    emit_origin(Origin::Synthetic, annotate_origins, output);
    write!(
        output,
        "define {} @{}(",
        function
            .return_type
            .as_ref()
            .map_or_else(|| "void".into(), |ty| type_name(ty, module)),
        function_name(function)
    )
    .unwrap();
    for (index, parameter) in function.parameters.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "{} %arg{index}", type_name(&parameter.ty, module)).unwrap();
    }
    output.push_str(") {\nentry:\n");

    for slot in &function.slots {
        writeln!(
            output,
            "  %cerune_{} = alloca {}",
            slot.name,
            type_name(&slot.ty, module),
        )
        .unwrap();
    }
    for (index, parameter) in function.parameters.iter().enumerate() {
        writeln!(
            output,
            "  store {} %arg{index}, ptr %cerune_{}",
            type_name(&parameter.ty, module),
            slot_by_id(&function.slots, parameter.slot).name,
        )
        .unwrap();
    }
    for instruction in &function.instructions {
        emit_origin(instruction.origin, annotate_origins, output);
        emit_instruction(
            &instruction.instruction,
            instruction.origin,
            &function.slots,
            module,
            output,
        );
    }
    output.push_str("}\n");
}

fn emit_instruction(
    instruction: &Instruction,
    origin: Origin,
    slots: &[Slot],
    module: &Module,
    output: &mut String,
) {
    let failure = super::failure::argument(instruction, origin);
    match instruction {
        Instruction::Flush => {
            let Origin::Source { node_id, .. } = origin else {
                unreachable!("flush origin")
            };
            writeln!(output, "  call void @cerune.flush.{}()", node_id.0).unwrap();
        }
        Instruction::ArrayAllocate {
            dest,
            element,
            length,
            element_width,
        } => {
            let ty = type_name(element, module);
            writeln!(output, "  {} = call %cerune.array @cerune.array.allocate.elements(i64 {}, i64 {element_width}, i64 ptrtoint (ptr getelementptr ({ty}, ptr null, i64 1) to i64), ptr {failure})", temp(*dest), operand(*length)).unwrap();
        }
        Instruction::ArrayInitialize {
            scratch,
            element,
            array,
            value,
        } => {
            let prefix = temp(*scratch);
            let ty = type_name(element, module);
            writeln!(
                output,
                "  {prefix}.owner = extractvalue %cerune.array {}, 0",
                operand(*array)
            )
            .unwrap();
            writeln!(output, "  {prefix}.count.ptr = getelementptr %cerune.array.owner, ptr {prefix}.owner, i32 0, i32 3").unwrap();
            writeln!(
                output,
                "  {prefix}.index = load i64, ptr {prefix}.count.ptr"
            )
            .unwrap();
            writeln!(output, "  {prefix}.data = load ptr, ptr {prefix}.owner").unwrap();
            writeln!(
                output,
                "  {prefix}.element = getelementptr {ty}, ptr {prefix}.data, i64 {prefix}.index"
            )
            .unwrap();
            writeln!(
                output,
                "  store {ty} {}, ptr {prefix}.element",
                operand(*value)
            )
            .unwrap();
            writeln!(output, "  {prefix}.next = add i64 {prefix}.index, 1").unwrap();
            writeln!(output, "  store i64 {prefix}.next, ptr {prefix}.count.ptr").unwrap();
        }
        Instruction::ArrayManage { value, retain } => {
            writeln!(
                output,
                "  call void @cerune.array.{}(%cerune.array {})",
                if *retain {
                    "retain.owner"
                } else {
                    "free.elements"
                },
                operand(*value)
            )
            .unwrap();
        }
        Instruction::ArrayReleaseOwner { dest, value } => {
            writeln!(
                output,
                "  {} = call i1 @cerune.array.release.owner.last(%cerune.array {})",
                temp(*dest),
                operand(*value)
            )
            .unwrap();
        }
        Instruction::ArrayRangeCheck { length, start, end } => {
            writeln!(
                output,
                "  call void @cerune.array.check.range(i64 {}, i64 {}, i64 {}, ptr {failure})",
                operand(*length),
                operand(*start),
                operand(*end)
            )
            .unwrap();
        }
        Instruction::StringByteAt { dest, value, index } => {
            writeln!(
                output,
                "  {} = call i64 @cerune.string.byte_at(%cerune.string {}, i64 {})",
                temp(*dest),
                operand(*value),
                operand(*index)
            )
            .unwrap();
        }
        Instruction::ArgumentCount { dest } => {
            writeln!(
                output,
                "  {} = call i64 @cerune.arguments.count(ptr {})",
                temp(*dest),
                super::arguments::origin_name(origin)
            )
            .unwrap();
        }
        Instruction::ArgumentGet { dest, index } => {
            writeln!(
                output,
                "  {} = call %cerune.string @cerune.arguments.get(i64 {}, ptr {})",
                temp(*dest),
                operand(*index),
                super::arguments::origin_name(origin)
            )
            .unwrap();
        }
        Instruction::ReadLine { dest, max_bytes } => {
            writeln!(
                output,
                "  {} = call %cerune.string @cerune.input.read_line(i64 {}, ptr {})",
                temp(*dest),
                operand(*max_bytes),
                super::input::origin_name(origin)
            )
            .unwrap();
        }
        Instruction::StringConcat { dest, left, right } => {
            writeln!(output, "  {} = call %cerune.string @cerune.string.concat(%cerune.string {}, %cerune.string {}, ptr {failure})",
                temp(*dest), operand(*left), operand(*right)).unwrap();
        }
        Instruction::StringManage { value, retain } => {
            writeln!(
                output,
                "  call void @cerune.string.{}(%cerune.string {})",
                if *retain { "retain" } else { "release" },
                operand(*value)
            )
            .unwrap();
        }
        Instruction::Output {
            kind,
            value,
            newline,
        } => {
            writeln!(
                output,
                "  call void @cerune.output.{kind}({} {}, i1 {newline}, ptr {})",
                crate::codegen::display::llvm_type(kind),
                operand(*value),
                super::output::origin_name(origin)
            )
            .unwrap();
        }
        Instruction::ConvertNumeric {
            dest,
            value,
            conversion,
        } => {
            writeln!(
                output,
                "  {} = call {} @{}({} {}, ptr {failure})",
                temp(*dest),
                super::conversion::type_name(conversion.to),
                conversion.helper(),
                super::conversion::type_name(conversion.from),
                operand(*value)
            )
            .unwrap();
        }
        Instruction::IntegerBinary {
            dest,
            op,
            ty,
            left,
            right,
        } => {
            writeln!(
                output,
                "  {} = call i64 @{}(i64 {}, i64 {}, ptr {failure})",
                temp(*dest),
                op.helper(*ty),
                operand(*left),
                operand(*right)
            )
            .unwrap();
        }
        Instruction::CheckIntegerRange {
            dest,
            value,
            ty,
            failure: code,
        } => {
            let code = super::failure::index(*code);
            writeln!(
                output,
                "  {} = call i64 @cerune_check_{}(i64 {}, ptr {failure}, i64 {code})",
                temp(*dest),
                ty.name(),
                operand(*value)
            )
            .unwrap();
        }
        Instruction::Label { id, name } => {
            writeln!(output, "{}: ; {name}", label(*id)).unwrap();
        }

        Instruction::Branch {
            condition,
            then_label,
            else_label,
        } => {
            writeln!(
                output,
                "  br i1 {}, label %{}, label %{}",
                operand(*condition),
                label(*then_label),
                label(*else_label),
            )
            .unwrap();
        }

        Instruction::Jump { label: target } => {
            writeln!(output, "  br label %{}", label(*target)).unwrap();
        }

        Instruction::Store { ty, value, slot } => {
            writeln!(
                output,
                "  store {} {}, ptr %cerune_{}",
                type_name(ty, module),
                operand(*value),
                slot_by_id(slots, *slot).name,
            )
            .unwrap();
        }

        Instruction::Load { dest, ty, slot } => {
            writeln!(
                output,
                "  {} = load {}, ptr %cerune_{}",
                temp(*dest),
                type_name(ty, module),
                slot_by_id(slots, *slot).name,
            )
            .unwrap();
        }

        Instruction::InsertValue {
            dest,
            ty,
            aggregate,
            value_ty,
            value,
            field,
        } => {
            writeln!(
                output,
                "  {} = insertvalue {} {}, {} {}, {}",
                temp(*dest),
                type_name(ty, module),
                operand(*aggregate),
                type_name(value_ty, module),
                operand(*value),
                field,
            )
            .unwrap();
        }

        Instruction::ExtractValue {
            dest,
            ty,
            aggregate,
            field,
        } => {
            writeln!(
                output,
                "  {} = extractvalue {} {}, {}",
                temp(*dest),
                type_name(ty, module),
                operand(*aggregate),
                field,
            )
            .unwrap();
        }

        Instruction::ArrayGet {
            dest,
            element,
            length,
            array,
            index,
        } => {
            let array_ty = array_type(element, *length);
            writeln!(
                output,
                "  {} = call {} @{}({} {}, i64 {}, ptr {failure})",
                temp(*dest),
                type_name(element, module),
                array_get_name(element, *length, module),
                type_name(&array_ty, module),
                operand(*array),
                operand(*index),
            )
            .unwrap();
        }

        Instruction::ArraySet {
            dest,
            element,
            length,
            array,
            index,
            value,
        } => {
            let array_ty = array_type(element, *length);
            writeln!(
                output,
                "  {} = call {} @{}({} {}, i64 {}, {} {}, ptr {failure})",
                temp(*dest),
                type_name(&array_ty, module),
                array_set_name(element, *length, module),
                type_name(&array_ty, module),
                operand(*array),
                operand(*index),
                type_name(element, module),
                operand(*value),
            )
            .unwrap();
        }

        Instruction::Call {
            dest,
            function_id,
            return_type,
            arguments,
        } => {
            output.push_str("  ");
            if let Some(dest) = dest {
                write!(output, "{} = ", temp(*dest)).unwrap();
            }
            let function = &module.functions[*function_id];
            write!(
                output,
                "call {} @{}(",
                return_type
                    .as_ref()
                    .map_or_else(|| "void".into(), |ty| type_name(ty, module)),
                function_name(function)
            )
            .unwrap();
            for (index, (ty, argument)) in arguments.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write!(output, "{} {}", type_name(ty, module), operand(*argument)).unwrap();
            }
            output.push_str(")\n");
        }

        Instruction::Return { value } => match value {
            Some((ty, value)) => {
                writeln!(
                    output,
                    "  ret {} {}",
                    type_name(ty, module),
                    operand(*value)
                )
                .unwrap();
            }
            None => output.push_str("  ret void\n"),
        },

        Instruction::Binary {
            dest,
            op,
            ty,
            left,
            right,
        } => {
            if let Some(helper) = checked_i64_helper(*op) {
                writeln!(
                    output,
                    "  {} = call i64 @{helper}(i64 {}, i64 {}, ptr {failure})",
                    temp(*dest),
                    operand(*left),
                    operand(*right),
                )
                .unwrap();
            } else {
                writeln!(
                    output,
                    "  {} = {} {} {}, {}",
                    temp(*dest),
                    binary_name(*op),
                    type_name(ty, module),
                    operand(*left),
                    operand(*right),
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
                    format!("{name}.equal")
                } else {
                    assert_eq!(*op, CompareOp::Equal);
                    name.clone()
                };
                writeln!(output, "  {result} = call i1 @cerune.string.equal(%cerune.string {}, %cerune.string {})", operand(*left), operand(*right)).unwrap();
                if *op == CompareOp::NotEqual {
                    writeln!(output, "  {name} = xor i1 {result}, true").unwrap();
                }
                return;
            }
            writeln!(
                output,
                "  {} = {} {} {}, {}",
                temp(*dest),
                if *unsigned {
                    unsigned_compare(*op)
                } else {
                    compare_name(*op, operand_ty)
                },
                type_name(operand_ty, module),
                operand(*left),
                operand(*right),
            )
            .unwrap();
        }

        Instruction::FNeg { dest, ty, value } => {
            writeln!(
                output,
                "  {} = fneg {} {}",
                temp(*dest),
                type_name(ty, module),
                operand(*value),
            )
            .unwrap();
        }
    }
}

fn slot_by_id(slots: &[Slot], id: SlotId) -> &Slot {
    &slots[id.0]
}

fn function_name(function: &Function) -> String {
    format!(
        "cerune.fn.{}.{}",
        function.name.replace('$', "_"),
        function.id
    )
}

fn temp(temp: Temp) -> String {
    format!("%tmp{}", temp.0)
}

fn label(label: Label) -> String {
    format!("block{}", label.0)
}

fn type_name(ty: &Type, module: &Module) -> String {
    match ty {
        Type::DynamicArray { .. } => "%cerune.array".into(),
        Type::String => "%cerune.string".into(),
        Type::Bool => "i1".into(),
        Type::I64 => "i64".into(),
        Type::Float => "float".into(),
        Type::Double => "double".into(),
        Type::Named(id) => {
            let definition = &module.type_definitions[*id];
            format!("%cerune.type.{}.{}", definition.name, id)
        }
        Type::Array { element, length } => {
            format!("[{length} x {}]", type_name(element, module))
        }
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::CheckedI64Add
        | BinaryOp::CheckedI64Sub
        | BinaryOp::CheckedI64Mul
        | BinaryOp::CheckedI64Div => {
            unreachable!("checked integer operations are emitted as helper calls")
        }
        BinaryOp::FAdd => "fadd",
        BinaryOp::FSub => "fsub",
        BinaryOp::FMul => "fmul",
        BinaryOp::FDiv => "fdiv",
        BinaryOp::Xor => "xor",
    }
}

fn checked_i64_helper(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::CheckedI64Add => Some("cerune_i64_add"),
        BinaryOp::CheckedI64Sub => Some("cerune_i64_sub"),
        BinaryOp::CheckedI64Mul => Some("cerune_i64_mul"),
        BinaryOp::CheckedI64Div => Some("cerune_i64_div"),
        BinaryOp::FAdd | BinaryOp::FSub | BinaryOp::FMul | BinaryOp::FDiv | BinaryOp::Xor => None,
    }
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
}

impl I64Operations {
    fn any(&self) -> bool {
        !self.conversions.is_empty()
            || !self.integer_binary.is_empty()
            || !self.range_checks.is_empty()
            || self.add
            || self.subtract
            || self.multiply
            || self.divide
    }

    fn include(&mut self, instruction: &Instruction) {
        if let Instruction::ConvertNumeric { conversion, .. } = instruction {
            self.conversions.insert(*conversion);
        }
        if let Instruction::IntegerBinary { op, ty, .. } = instruction {
            self.integer_binary.insert((*op, *ty));
        }
        if let Instruction::CheckIntegerRange { ty, .. } = instruction
            && *ty != crate::types::IntegerType::I64
        {
            self.range_checks.insert(*ty);
        }
        let Instruction::Binary { op, .. } = instruction else {
            return;
        };
        match op {
            BinaryOp::CheckedI64Add => self.add = true,
            BinaryOp::CheckedI64Sub => self.subtract = true,
            BinaryOp::CheckedI64Mul => self.multiply = true,
            BinaryOp::CheckedI64Div => self.divide = true,
            BinaryOp::FAdd | BinaryOp::FSub | BinaryOp::FMul | BinaryOp::FDiv | BinaryOp::Xor => {}
        }
    }
}

fn i64_operations(module: &Module) -> I64Operations {
    let mut operations = I64Operations::default();
    for instruction in &module.instructions {
        operations.include(&instruction.instruction);
    }
    for function in &module.functions {
        for instruction in &function.instructions {
            operations.include(&instruction.instruction);
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
        output.push_str(&format!("define internal i64 @cerune_check_{}(i64 %value, ptr %failure, i64 %code) {{\nentry:\n  %below = icmp slt i64 %value, {}\n  %above = icmp sgt i64 %value, {}\n  %bad = or i1 %below, %above\n  br i1 %bad, label %trap, label %ok\ntrap:\n  call void @cerune.runtime.fail(ptr %failure, i64 %code)\n  unreachable\nok:\n  ret i64 %value\n}}\n\n", ty.name(), ty.minimum(), ty.maximum()));
    }

    for (enabled, name, intrinsic) in [
        (operations.add, "add", "sadd"),
        (operations.subtract, "sub", "ssub"),
        (operations.multiply, "mul", "smul"),
    ] {
        if !enabled {
            continue;
        }
        writeln!(
            output,
            "define internal i64 @cerune_i64_{name}(i64 %left, i64 %right, ptr %failure) {{"
        )
        .unwrap();
        output.push_str("entry:\n");
        writeln!(
            output,
            "  %checked = call {{ i64, i1 }} @llvm.{intrinsic}.with.overflow.i64(i64 %left, i64 %right)"
        )
        .unwrap();
        output.push_str("  %result = extractvalue { i64, i1 } %checked, 0\n");
        output.push_str("  %overflow = extractvalue { i64, i1 } %checked, 1\n");
        output.push_str("  br i1 %overflow, label %trap, label %ok\n\n");
        output.push_str("trap:\n");
        super::failure::emit_trap(crate::runtime::FailureCode::IntegerOverflow, output);
        output.push_str("ok:\n  ret i64 %result\n}\n\n");
    }

    if operations.divide {
        output.push_str("define internal i64 @cerune_i64_div(i64 %left, i64 %right, ptr %failure) {\nentry:\n  %is_zero = icmp eq i64 %right, 0\n  br i1 %is_zero, label %zero, label %bounds\nzero:\n");
        super::failure::emit_trap(crate::runtime::FailureCode::DivisionByZero, output);
        output.push_str("bounds:\n  %is_min = icmp eq i64 %left, -9223372036854775808\n  %is_negative_one = icmp eq i64 %right, -1\n  %overflows = and i1 %is_min, %is_negative_one\n  br i1 %overflows, label %overflow, label %ok\noverflow:\n");
        super::failure::emit_trap(crate::runtime::FailureCode::DivisionOverflow, output);
        output.push_str("ok:\n  %result = sdiv i64 %left, %right\n  ret i64 %result\n}\n\n");
    }
}

fn compare_name(op: CompareOp, ty: &Type) -> &'static str {
    match (op, ty) {
        (_, Type::String) => unreachable!("string comparisons use a content helper"),
        (CompareOp::Equal, Type::Bool | Type::I64) => "icmp eq",
        (CompareOp::NotEqual, Type::Bool | Type::I64) => "icmp ne",
        (CompareOp::Less, Type::I64) => "icmp slt",
        (CompareOp::LessEqual, Type::I64) => "icmp sle",
        (CompareOp::Greater, Type::I64) => "icmp sgt",
        (CompareOp::GreaterEqual, Type::I64) => "icmp sge",

        (CompareOp::Equal, Type::Float | Type::Double) => "fcmp oeq",
        (CompareOp::NotEqual, Type::Float | Type::Double) => "fcmp une",
        (CompareOp::Less, Type::Float | Type::Double) => "fcmp olt",
        (CompareOp::LessEqual, Type::Float | Type::Double) => "fcmp ole",
        (CompareOp::Greater, Type::Float | Type::Double) => "fcmp ogt",
        (CompareOp::GreaterEqual, Type::Float | Type::Double) => "fcmp oge",

        (
            CompareOp::Less | CompareOp::LessEqual | CompareOp::Greater | CompareOp::GreaterEqual,
            Type::Bool,
        ) => {
            unreachable!("semantic analysis rejects boolean ordering")
        }
        (_, Type::DynamicArray { .. } | Type::Named(_) | Type::Array { .. }) => {
            unreachable!("semantic analysis rejects aggregate comparison")
        }
    }
}

fn operand(operand: Operand) -> String {
    match operand {
        Operand::String { id, length } => format!("{{ ptr @cerune.string.{id}, i64 {length} }}"),
        Operand::Boolean(value) => i32::from(value).to_string(),

        Operand::Integer(value) => value.to_string(),

        Operand::Float32(bits) => {
            // LLVM 22 legacy hexadecimal syntax represents
            // float constants using the corresponding exact
            // double representation.
            let as_double = f32::from_bits(bits) as f64;

            format!("0x{:016X}", as_double.to_bits())
        }

        Operand::Float64(bits) => format!("0x{bits:016X}"),

        Operand::Temp(id) => temp(id),
        Operand::Poison => "poison".into(),
    }
}

fn array_types(module: &Module) -> Vec<Type> {
    fn add(ty: &Type, result: &mut Vec<Type>) {
        let (Type::Array { element, .. } | Type::DynamicArray { element }) = ty else {
            return;
        };
        add(element, result);
        if !result.contains(ty) {
            result.push(ty.clone());
        }
    }

    let mut result = Vec::new();
    for instruction in module.instructions.iter().chain(
        module
            .functions
            .iter()
            .flat_map(|function| function.instructions.iter()),
    ) {
        match &instruction.instruction {
            Instruction::ArrayGet {
                element, length, ..
            }
            | Instruction::ArraySet {
                element, length, ..
            } => {
                add(&array_type(element, *length), &mut result);
            }
            _ => {}
        }
    }
    result
}

fn array_set_types(module: &Module) -> Vec<Type> {
    let mut result = Vec::new();
    for instruction in module.instructions.iter().chain(
        module
            .functions
            .iter()
            .flat_map(|function| function.instructions.iter()),
    ) {
        if let Instruction::ArraySet {
            element, length, ..
        } = &instruction.instruction
        {
            let ty = array_type(element, *length);
            if !result.contains(&ty) {
                result.push(ty);
            }
        }
    }
    result
}

fn emit_array_get(ty: &Type, module: &Module, output: &mut String) {
    if let Type::DynamicArray { element } = ty {
        return emit_dynamic_access(element, false, module, output);
    }
    let Type::Array { element, length } = ty else {
        unreachable!("array getter requires an array type")
    };
    let element_ty = type_name(element, module);
    let array_ty = format!("[{length} x {element_ty}]");
    writeln!(
        output,
        "define internal {element_ty} @{}({array_ty} %value, i64 %index, ptr %failure) {{",
        array_get_name(element, Some(*length), module)
    )
    .unwrap();
    output.push_str("entry:\n");
    output.push_str("  %index.low = icmp slt i64 %index, 0\n");
    writeln!(output, "  %index.high = icmp sge i64 %index, {length}").unwrap();
    output.push_str("  %index.outside = or i1 %index.low, %index.high\n");
    output.push_str("  br i1 %index.outside, label %out_of_bounds, label %in_bounds\n");
    output.push_str("out_of_bounds:\n");
    super::failure::emit_trap(crate::runtime::FailureCode::ArrayIndexOutOfBounds, output);
    output.push_str("in_bounds:\n");
    writeln!(output, "  %array = alloca {array_ty}").unwrap();
    writeln!(output, "  store {array_ty} %value, ptr %array").unwrap();
    writeln!(
        output,
        "  %element = getelementptr inbounds {array_ty}, ptr %array, i64 0, i64 %index"
    )
    .unwrap();
    writeln!(output, "  %result = load {element_ty}, ptr %element").unwrap();
    writeln!(output, "  ret {element_ty} %result").unwrap();
    output.push_str("}\n");
}

fn emit_array_set(ty: &Type, module: &Module, output: &mut String) {
    if let Type::DynamicArray { element } = ty {
        return emit_dynamic_access(element, true, module, output);
    }
    let Type::Array { element, length } = ty else {
        unreachable!("array setter requires an array type")
    };
    let element_ty = type_name(element, module);
    let array_ty = format!("[{length} x {element_ty}]");
    writeln!(
        output,
        "define internal {array_ty} @{}({array_ty} %value, i64 %index, {element_ty} %replacement, ptr %failure) {{",
        array_set_name(element, Some(*length), module)
    )
    .unwrap();
    output.push_str("entry:\n");
    output.push_str("  %index.low = icmp slt i64 %index, 0\n");
    writeln!(output, "  %index.high = icmp sge i64 %index, {length}").unwrap();
    output.push_str("  %index.outside = or i1 %index.low, %index.high\n");
    output.push_str("  br i1 %index.outside, label %out_of_bounds, label %in_bounds\n");
    output.push_str("out_of_bounds:\n");
    super::failure::emit_trap(crate::runtime::FailureCode::ArrayIndexOutOfBounds, output);
    output.push_str("in_bounds:\n");
    writeln!(output, "  %array = alloca {array_ty}").unwrap();
    writeln!(output, "  store {array_ty} %value, ptr %array").unwrap();
    writeln!(
        output,
        "  %element = getelementptr inbounds {array_ty}, ptr %array, i64 0, i64 %index"
    )
    .unwrap();
    writeln!(output, "  store {element_ty} %replacement, ptr %element").unwrap();
    writeln!(output, "  %result = load {array_ty}, ptr %array").unwrap();
    writeln!(output, "  ret {array_ty} %result").unwrap();
    output.push_str("}\n");
}

fn array_get_name(element: &Type, length: Option<usize>, module: &Module) -> String {
    let length = length.map_or_else(|| "dynamic".into(), |n| n.to_string());
    format!(
        "cerune.array.get.{}.{length}",
        array_element_name(element, module)
    )
}

fn array_set_name(element: &Type, length: Option<usize>, module: &Module) -> String {
    let length = length.map_or_else(|| "dynamic".into(), |n| n.to_string());
    format!(
        "cerune.array.set.{}.{length}",
        array_element_name(element, module)
    )
}

fn array_element_name(element: &Type, module: &Module) -> String {
    match element {
        Type::DynamicArray { element } => {
            format!("dynamic.{}", array_element_name(element, module))
        }
        Type::String => "string".into(),
        Type::Bool => "bool".into(),
        Type::I64 => "i64".into(),
        Type::Float => "f32".into(),
        Type::Double => "f64".into(),
        Type::Named(id) => {
            let definition = &module.type_definitions[*id];
            format!("type.{}.{}", definition.name, id)
        }
        Type::Array { element, length } => {
            format!("array.{}.{length}", array_element_name(element, module))
        }
    }
}

fn emit_origin(origin: Origin, enabled: bool, output: &mut String) {
    if !enabled {
        return;
    }
    match origin {
        Origin::Source { node_id, span } => writeln!(
            output,
            "; cerune-origin: #{}{} bytes {}..{}",
            node_id.0,
            span.source_id().record_field(),
            span.start(),
            span.end()
        )
        .unwrap(),
        Origin::Synthetic => output.push_str("; cerune-origin: synthetic\n"),
    }
}

fn unsigned_compare(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Equal => "icmp eq",
        CompareOp::NotEqual => "icmp ne",
        CompareOp::Less => "icmp ult",
        CompareOp::LessEqual => "icmp ule",
        CompareOp::Greater => "icmp ugt",
        CompareOp::GreaterEqual => "icmp uge",
    }
}

fn array_type(element: &Type, length: Option<usize>) -> Type {
    let element = Box::new(element.clone());
    match length {
        Some(length) => Type::Array { element, length },
        None => Type::DynamicArray { element },
    }
}
// 動的配列の境界検査と型付きload/store。要素走査は共通IRのループのままです。
fn emit_dynamic_access(element: &Type, set: bool, module: &Module, output: &mut String) {
    let ty = type_name(element, module);
    if set {
        writeln!(output, "define internal %cerune.array @{}(%cerune.array %value, i64 %index, {ty} %replacement, ptr %failure) {{", array_set_name(element, None, module)).unwrap();
    } else {
        writeln!(
            output,
            "define internal {ty} @{}(%cerune.array %value, i64 %index, ptr %failure) {{",
            array_get_name(element, None, module)
        )
        .unwrap();
    }
    output.push_str("entry:\n  %data = call ptr @cerune.array.checked.data(%cerune.array %value, i64 %index, ptr %failure)\n");
    writeln!(
        output,
        "  %element = getelementptr {ty}, ptr %data, i64 %index"
    )
    .unwrap();
    if set {
        writeln!(
            output,
            "  store {ty} %replacement, ptr %element\n  ret %cerune.array %value\n}}"
        )
        .unwrap();
    } else {
        writeln!(
            output,
            "  %result = load {ty}, ptr %element\n  ret {ty} %result\n}}"
        )
        .unwrap();
    }
}
