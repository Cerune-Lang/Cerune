mod arguments;
mod array;
mod call;
mod conversion;
mod emit;
mod failure;
mod frame;
mod heap;
mod input;
mod integer;
pub mod ir;
pub mod library;
mod lower;
mod utf8;
pub use library::emit_library;
mod output;
mod rounding;
mod string;
mod unsigned;

pub use emit::emit;
use lower::lower;

use crate::{diagnostic::Diagnostic, ir as cerune_ir};

pub fn emit_wat(program: &cerune_ir::Program) -> Result<String, Diagnostic> {
    emit_wat_with_argument_limits(program, Default::default())
}
pub fn emit_wat_with_argument_limits(
    program: &cerune_ir::Program,
    limits: crate::runtime::arguments::Limits,
) -> Result<String, Diagnostic> {
    emit_configured(program, limits, None)
}
/// 非公開memoryに呼出し履歴を保持します。Wasmエンジンの物理stackは制限しません。
pub fn emit_wat_with_call_limit(
    program: &cerune_ir::Program,
    limit: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    emit_configured(program, Default::default(), Some(limit))
}
/// 引数と呼出し深さの制約を同時に指定します。既存APIの既定動作は維持します。
pub fn emit_wat_with_runtime_limits(
    program: &cerune_ir::Program,
    arguments: crate::runtime::arguments::Limits,
    calls: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    emit_configured(program, arguments, Some(calls))
}
fn emit_configured(
    program: &cerune_ir::Program,
    limits: crate::runtime::arguments::Limits,
    call_limit: Option<std::num::NonZeroU32>,
) -> Result<String, Diagnostic> {
    super::support::reject_files(program, "WAT")?;
    if let Some(span) = program.first_argument_span()
        && limits.max_count > i64::MAX as u64
    {
        return Err(Diagnostic::new(
            "WAT argument count limit must not exceed i64::MAX",
            span,
        ));
    }
    let mut module = lower(program);
    module.argument_limits = limits;
    module.call_limit = call_limit;
    if call_limit.is_some() {
        module.string_heap_start = module.string_heap_start.max(8).next_multiple_of(8);
        module.memory_pages = module
            .memory_pages
            .max(module.string_heap_start.div_ceil(65_536) as u32);
    }

    Ok(emit(&module))
}

#[cfg(test)]
mod tests {
    use crate::compile_to_ir;

    use super::{emit_wat, ir::Instruction, lower};

    #[test]
    fn lowers_i64_negation_to_stack_instructions() {
        let program = compile_to_ir("x: i64 = -1;").unwrap();
        let module = lower(&program);

        assert!(module.instructions.windows(3).any(|instructions| {
            matches!(
                instructions,
                [
                    Instruction::I64Const(0),
                    Instruction::I64Const(1),
                    Instruction::Located { instruction, .. }
                ]
                if matches!(**instruction, Instruction::CheckedI64Sub)
            )
        }));
    }

    #[test]
    fn lowers_print_type() {
        let program = compile_to_ir("print(1);").unwrap();
        let module = lower(&program);

        assert!(matches!(
            module.instructions.last(),
            Some(Instruction::Output {
                kind: "i64",
                newline: true,
                ..
            })
        ));
    }

    #[test]
    fn emits_i64_add() {
        let program = compile_to_ir(
            "x: i64 = 1 + 2;
             print(x);",
        )
        .unwrap();

        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains("(func $cerune_i64_add"));
        assert!(wat.contains("call $cerune_i64_add"));
        assert!(wat.contains("unreachable"));

        assert!(wat.contains("(local $cerune_x i64)"));

        assert!(wat.contains("call $cerune_output_i64"));
    }

    #[test]
    fn emits_checked_i64_arithmetic() {
        let program = compile_to_ir(
            "value: i64 = 8;
             print(value + 1);
             print(value - 1);
             print(value * 2);
             print(value / 2);
             print(-value);",
        )
        .unwrap();
        let wat = emit_wat(&program).unwrap();

        for helper in ["$cerune_i64_add", "$cerune_i64_sub", "$cerune_i64_mul"] {
            assert!(wat.contains(helper));
        }
        assert!(wat.contains("i64.div_s"));
        assert!(wat.contains("unreachable"));
    }

    #[test]
    fn emits_f32_add() {
        let program = compile_to_ir(
            "x: f32 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains("f32.const 0.1"));

        assert!(wat.contains("f32.const 0.2"));

        assert!(wat.contains("f32.add"));

        assert!(wat.contains("call $cerune_output_f32"));
    }

    #[test]
    fn emits_f64_add() {
        let program = compile_to_ir(
            "x: f64 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains("f64.add"));

        assert!(wat.contains("call $cerune_output_f64"));
    }

    #[test]
    fn inferred_f32_uses_f32() {
        let program = compile_to_ir(
            "a: f32 = 0.1 + 0.2;
             b: infer = a + a;",
        )
        .unwrap();

        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains("(local $cerune_b f32)"));

        assert!(wat.contains("f32.add"));
    }

    #[test]
    fn emits_all_integer_comparisons() {
        let program = compile_to_ir(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let wat = emit_wat(&program).unwrap();

        for instruction in [
            "i64.eq", "i64.ne", "i64.lt_s", "i64.le_s", "i64.gt_s", "i64.ge_s",
        ] {
            assert!(wat.contains(instruction));
        }
    }

    #[test]
    fn lowers_product_values_to_linear_memory() {
        let program = compile_to_ir(
            "type Point { x: f64 = 0.0, y: f64, }
             point: Point = Point { y: 2.0, };
             print(point.x);",
        )
        .unwrap();
        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains("(memory 1)"));
        assert!(wat.contains("f64.store"));
        assert!(wat.contains("f64.load"));
        assert!(wat.contains("call $cerune_output_f64"));
    }

    #[test]
    fn emits_typed_functions_and_calls() {
        let program = compile_to_ir(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);
             print(answer);",
        )
        .unwrap();
        let wat = emit_wat(&program).unwrap();

        assert!(wat.contains(
            "(func $cerune_fn_add_0 (param $cerune_left i64) (param $cerune_right i64) (param $cerune.frame.base i32) (result i64)"
        ));
        assert!(wat.contains("call $cerune_fn_add_0"));
        assert!(wat.contains("return"));
    }
}
