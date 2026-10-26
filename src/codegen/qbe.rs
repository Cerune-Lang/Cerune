mod arguments;
mod array;
mod call;
mod conversion;
mod emit;
mod failure;
mod heap;
mod input;
mod integer;
pub mod ir;
mod lower;
mod output;
mod rounding;
mod string;
mod unsigned;
mod utf8;

pub use emit::emit;
use lower::lower;

use crate::{diagnostic::Diagnostic, ir as cerune_ir};

/// QBEの文字列・動的配列・診断で検証する実行環境です。ホストOSからは選びません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    X86_64UnknownLinuxGnu,
    X86_64PcWindowsMsvc,
}

impl Target {
    pub const fn triple(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
        }
    }
    pub const fn qbe_name(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "amd64_sysv",
            Self::X86_64PcWindowsMsvc => "amd64_win",
        }
    }
    const fn read_only_section(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => ".rodata",
            Self::X86_64PcWindowsMsvc => ".rdata",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "x86_64-unknown-linux-gnu" => Some(Self::X86_64UnknownLinuxGnu),
            "x86_64-pc-windows-msvc" => Some(Self::X86_64PcWindowsMsvc),
            _ => None,
        }
    }
}

pub fn emit_qbe(program: &cerune_ir::Program) -> Result<String, Diagnostic> {
    emit_qbe_with_target(program, None)
}

pub fn emit_qbe_with_target(
    program: &cerune_ir::Program,
    target: Option<Target>,
) -> Result<String, Diagnostic> {
    emit_qbe_with_argument_limits(program, target, Default::default())
}

/// Explicit startup limits; existing emission APIs retain their defaults.
pub fn emit_qbe_with_argument_limits(
    program: &cerune_ir::Program,
    target: Option<Target>,
    limits: crate::runtime::arguments::Limits,
) -> Result<String, Diagnostic> {
    emit_configured(program, target, limits, None)
}
/// ターゲットとソース関数の深さ上限を明示します。物理stackの安全範囲は保証しません。
pub fn emit_qbe_with_call_limit(
    program: &cerune_ir::Program,
    target: Target,
    limit: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    emit_configured(program, Some(target), Default::default(), Some(limit))
}
/// 引数と呼出し深さの制約を同時に指定します。既存APIの既定動作は維持します。
pub fn emit_qbe_with_runtime_limits(
    program: &cerune_ir::Program,
    target: Option<Target>,
    arguments: crate::runtime::arguments::Limits,
    calls: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    if target.is_none() {
        return Err(Diagnostic::without_span(
            "QBE call depth checks require an explicit --target",
        ));
    }
    emit_configured(program, target, arguments, Some(calls))
}
fn emit_configured(
    program: &cerune_ir::Program,
    target: Option<Target>,
    limits: crate::runtime::arguments::Limits,
    call_limit: Option<std::num::NonZeroU32>,
) -> Result<String, Diagnostic> {
    if let Some(span) = program.first_argument_span() {
        if target.is_none() {
            return Err(Diagnostic::new(
                "QBE program arguments require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
                span,
            ));
        }
        if limits.max_count > i64::MAX as u64 {
            return Err(Diagnostic::new(
                "QBE argument count limit must not exceed i64::MAX",
                span,
            ));
        }
    }
    if let Some(span) = program.first_input_span()
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "QBE line input requires an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    if target.is_none()
        && let Some(span) = super::support::first_flush_span(program)
    {
        return Err(Diagnostic::new(
            "QBE flush requires an explicit --target",
            span,
        ));
    }
    if let Some(span) = program.first_dynamic_array_span()
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "QBE dynamic arrays require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    if let Some(span) = super::support::first_string_span(program)
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "QBE string values require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }

    let mut module = lower(program);
    if target.is_none()
        && let Some(span) = output::first_span(&module)
    {
        return Err(Diagnostic::new(
            "QBE output requires an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    module.call_limit = call_limit;
    module.target = target;
    module.argument_limits = limits;

    Ok(emit(&module))
}

#[cfg(test)]
mod tests {
    use crate::compile_to_ir;

    use super::{ir::Instruction, lower};
    fn emit_qbe(program: &crate::ir::Program) -> Result<String, crate::diagnostic::Diagnostic> {
        super::emit_qbe_with_target(program, Some(super::Target::X86_64UnknownLinuxGnu))
    }

    #[test]
    fn lowers_i64_add() {
        let program = compile_to_ir("x: i64 = 1 + 2; print(x);").unwrap();
        let module = lower(&program);

        assert!(module.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                Instruction::Binary {
                    op: super::ir::BinaryOp::CheckedI64Add,
                    ..
                }
            )
        }));
    }

    #[test]
    fn emits_i64_add() {
        let program = compile_to_ir("x: i64 = 1 + 2; print(x);").unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("function l $cerune_i64_add"));
        assert!(qbe.contains("=l call $cerune_i64_add(l 1, l 2, l $cerune_origin_"));
        assert!(qbe.contains("jnz %overflow, @trap, @ok"));
        assert!(qbe.contains("storel %tmp0, %slot_x"));
        assert!(qbe.contains("call $cerune_output_i64(l "));
    }

    #[test]
    fn emits_checked_i64_arithmetic_helpers() {
        let program = compile_to_ir(
            "value: i64 = 8;
             print(value + 1);
             print(value - 1);
             print(value * 2);
             print(value / 2);
             print(-value);",
        )
        .unwrap();
        let qbe = emit_qbe(&program).unwrap();

        for helper in [
            "$cerune_i64_add",
            "$cerune_i64_sub",
            "$cerune_i64_mul",
            "$cerune_i64_div",
            "$cerune_i64_neg",
        ] {
            assert!(qbe.contains(helper));
        }
        assert!(qbe.contains("call $abort()"));
    }

    #[test]
    fn emits_f32_add() {
        let program = compile_to_ir("x: f32 = 0.1 + 0.2; print(x);").unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("=s add s_0.1, s_0.2"));
        assert!(qbe.contains("=d exts %value"));
        assert!(qbe.contains("call $cerune_output_f32(s "));
    }

    #[test]
    fn emits_f64_add() {
        let program = compile_to_ir("x: f64 = 0.1 + 0.2; print(x);").unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("=d add d_0.1, d_0.2"));
        assert!(qbe.contains("call $cerune_output_f64(d "));
    }

    #[test]
    fn inferred_f32_uses_single() {
        let program = compile_to_ir("a: f32 = 0.1 + 0.2; b: infer = a + a;").unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("stores %tmp"));
    }

    #[test]
    fn emits_all_integer_comparisons() {
        let program = compile_to_ir(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let qbe = emit_qbe(&program).unwrap();

        for instruction in ["ceql", "cnel", "csltl", "cslel", "csgtl", "csgel"] {
            assert!(qbe.contains(instruction));
        }
    }

    #[test]
    fn lowers_product_values_to_stack_memory() {
        let program = compile_to_ir(
            "type Point { x: f64 = 0.0, y: f64, }
             point: Point = Point { y: 2.0, };
             print(point.x);",
        )
        .unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("%slot_point =l alloc8 16"));
        assert!(qbe.contains("%slot_aggregate_tmp0 =l alloc8 16"));
        assert!(qbe.contains("blit %slot_aggregate_tmp0, %slot_point, 16"));
        assert!(qbe.contains("loadd %slot_point"));
    }

    #[test]
    fn emits_typed_functions_and_calls() {
        let program = compile_to_ir(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);
             print(answer);",
        )
        .unwrap();
        let qbe = emit_qbe(&program).unwrap();

        assert!(qbe.contains("function l $cerune_fn_add_0(l %arg0, l %arg1)"));
        assert!(qbe.contains("storel %arg0, %slot_left"));
        assert!(qbe.contains("call $cerune_fn_add_0(l 20, l 22)"));
        assert!(qbe.contains("ret %tmp"));
    }
}
