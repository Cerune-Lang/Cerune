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

/// 出力の実行環境を明示します。コンパイラを動かすOSからは選びません。
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

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "x86_64-unknown-linux-gnu" => Some(Self::X86_64UnknownLinuxGnu),
            "x86_64-pc-windows-msvc" => Some(Self::X86_64PcWindowsMsvc),
            _ => None,
        }
    }
}

pub fn emit_llvm(program: &cerune_ir::Program) -> Result<String, Diagnostic> {
    emit_llvm_with_target(program, None)
}

pub fn emit_llvm_with_target(
    program: &cerune_ir::Program,
    target: Option<Target>,
) -> Result<String, Diagnostic> {
    emit_llvm_with_options(
        program,
        Options {
            target,
            annotate_origins: false,
        },
    )
}

/// 注釈は読み取り専用で、命令の選択や評価順には影響しません。
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub target: Option<Target>,
    pub annotate_origins: bool,
}

pub fn emit_llvm_with_options(
    program: &cerune_ir::Program,
    options: Options,
) -> Result<String, Diagnostic> {
    emit_llvm_with_argument_limits(program, options, Default::default())
}

/// Explicit startup limits; existing emission APIs retain their defaults.
pub fn emit_llvm_with_argument_limits(
    program: &cerune_ir::Program,
    options: Options,
    limits: crate::runtime::arguments::Limits,
) -> Result<String, Diagnostic> {
    emit_configured(program, options, limits, None)
}
/// 明示ターゲット上でソース関数の呼出し深さを検査します。
pub fn emit_llvm_with_call_limit(
    program: &cerune_ir::Program,
    target: Target,
    limit: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    emit_configured(
        program,
        Options {
            target: Some(target),
            annotate_origins: false,
        },
        Default::default(),
        Some(limit),
    )
}
/// 引数と呼出し深さの制約を同時に指定します。既存APIの既定動作は維持します。
pub fn emit_llvm_with_runtime_limits(
    program: &cerune_ir::Program,
    options: Options,
    arguments: crate::runtime::arguments::Limits,
    calls: std::num::NonZeroU32,
) -> Result<String, Diagnostic> {
    if options.target.is_none() {
        return Err(Diagnostic::without_span(
            "LLVM call depth checks require an explicit --target",
        ));
    }
    emit_configured(program, options, arguments, Some(calls))
}
fn emit_configured(
    program: &cerune_ir::Program,
    options: Options,
    limits: crate::runtime::arguments::Limits,
    call_limit: Option<std::num::NonZeroU32>,
) -> Result<String, Diagnostic> {
    super::support::reject_files(program, "LLVM")?;
    let target = options.target;
    if let Some(span) = program.first_argument_span() {
        if target.is_none() {
            return Err(Diagnostic::new(
                "LLVM program arguments require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
                span,
            ));
        }
        if limits.max_count > i64::MAX as u64 {
            return Err(Diagnostic::new(
                "LLVM argument count limit must not exceed i64::MAX",
                span,
            ));
        }
    }
    if let Some(span) = program.first_input_span()
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "LLVM line input requires an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    if target.is_none()
        && let Some(span) = super::support::first_flush_span(program)
    {
        return Err(Diagnostic::new(
            "LLVM flush requires an explicit --target",
            span,
        ));
    }
    if let Some(span) = program.first_dynamic_array_span()
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "LLVM dynamic arrays require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    let string_span = super::support::first_string_span(program);
    if let Some(span) = string_span
        && target.is_none()
    {
        return Err(Diagnostic::new(
            "LLVM string values require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    let mut module = lower(program);
    if target.is_none()
        && let Some(span) = failure::first_failure_span(&module)
    {
        return Err(Diagnostic::new(
            "LLVM runtime diagnostics require an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    if target.is_none()
        && let Some(span) = output::first_span(&module)
    {
        return Err(Diagnostic::new(
            "LLVM output requires an explicit --target: x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc",
            span,
        ));
    }
    module.call_limit = call_limit;
    module.target = target;
    module.argument_limits = limits;
    module.uses_strings = string_span.is_some();

    Ok(emit::emit_with_origins(&module, options.annotate_origins))
}

#[cfg(test)]
mod tests {
    use crate::compile_to_ir;

    use super::{ir::Instruction, lower};

    fn emit_llvm(program: &crate::ir::Program) -> Result<String, crate::diagnostic::Diagnostic> {
        super::emit_llvm_with_target(program, Some(super::Target::X86_64UnknownLinuxGnu))
    }

    #[test]
    fn output_and_runtime_diagnostics_require_an_explicit_target() {
        let infallible = crate::compile_to_ir("x: i64 = 1;").unwrap();
        assert!(super::emit_llvm(&infallible).is_ok());
        let output = crate::compile_to_ir("print(1);").unwrap();
        assert!(
            super::emit_llvm(&output)
                .unwrap_err()
                .message()
                .contains("output requires an explicit --target")
        );
        let program = crate::compile_to_ir("print(1 / 0);").unwrap();
        let error = super::emit_llvm(&program).unwrap_err();
        assert!(
            error
                .message()
                .contains("runtime diagnostics require an explicit --target")
        );
        for (target, writer) in [
            (
                super::Target::X86_64UnknownLinuxGnu,
                "call i64 @write(i32 2",
            ),
            (super::Target::X86_64PcWindowsMsvc, "call i32 @_write(i32 2"),
        ] {
            let artifact = super::emit_llvm_with_target(&program, Some(target)).unwrap();
            assert!(artifact.contains(writer));
            assert!(artifact.contains("call i32 @fflush(ptr null)"));
            assert!(artifact.contains("call void @llvm.trap()"));
        }
    }

    #[test]
    fn lowers_i64_add() {
        let program = compile_to_ir("x: i64 = 1 + 2; print(x);").unwrap();
        let module = lower(&program);

        assert!(module.instructions.iter().any(|instruction| {
            matches!(
                instruction.instruction,
                Instruction::Binary {
                    op: super::ir::BinaryOp::CheckedI64Add,
                    ..
                }
            )
        }));
    }

    #[test]
    fn nested_expression_origins_restore_the_parent_and_mark_generated_returns() {
        let source = "fn noop() -> void {} print(\"日\" == \"日\");";
        let program = compile_to_ir(source).unwrap();
        let module = lower(&program);
        assert_eq!(
            module.functions[0].instructions[0].origin,
            super::ir::Origin::Synthetic
        );
        let comparison = module
            .instructions
            .iter()
            .find(|item| matches!(item.instruction, Instruction::Compare { .. }))
            .unwrap();
        let super::ir::Origin::Source { node_id, span } = comparison.origin else {
            panic!("source origin required")
        };
        assert_eq!(&source[span.start()..span.end()], "\"日\" == \"日\"");
        let crate::ir::StatementKind::Print { value } = &program.statements[0].kind else {
            panic!()
        };
        assert_eq!(node_id, value.id);
        for item in module
            .instructions
            .iter()
            .filter(|item| matches!(item.instruction, Instruction::Output { .. }))
        {
            assert_eq!(
                item.origin,
                super::ir::Origin::Source {
                    node_id: program.statements[0].id,
                    span: program.statements[0].span
                }
            );
        }
    }

    #[test]
    fn emits_i64_add() {
        let program = compile_to_ir("x: i64 = 1 + 2; print(x);").unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("@llvm.sadd.with.overflow.i64"));
        assert!(llvm.contains("call i64 @cerune_i64_add(i64 1, i64 2, ptr @cerune.failure."));
        assert!(llvm.contains("br i1 %overflow, label %trap, label %ok"));
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
        let llvm = emit_llvm(&program).unwrap();

        for helper in [
            "@cerune_i64_add",
            "@cerune_i64_sub",
            "@cerune_i64_mul",
            "@cerune_i64_div",
        ] {
            assert!(llvm.contains(helper));
        }
        assert!(llvm.contains("@llvm.sadd.with.overflow.i64"));
        assert!(llvm.contains("@llvm.ssub.with.overflow.i64"));
        assert!(llvm.contains("@llvm.smul.with.overflow.i64"));
        assert!(llvm.contains("%result = sdiv i64 %left, %right"));
    }

    #[test]
    fn emits_f32_add() {
        let program = compile_to_ir("x: f32 = 0.1 + 0.2; print(x);").unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("fadd float"));
        assert!(llvm.contains("fpext float"));
    }

    #[test]
    fn emits_f64_add() {
        let program = compile_to_ir("x: f64 = 0.1 + 0.2; print(x);").unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("fadd double"));
    }

    #[test]
    fn emits_llvm_22_compatible_float_literals() {
        let program = compile_to_ir("x: f32 = 0.1 + 0.2; print(x);").unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("fadd float 0x3FB99999A0000000, 0x3FC99999A0000000"));
    }

    #[test]
    fn emits_all_integer_comparisons() {
        let program = compile_to_ir(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let llvm = emit_llvm(&program).unwrap();

        for instruction in [
            "icmp eq i64",
            "icmp ne i64",
            "icmp slt i64",
            "icmp sle i64",
            "icmp sgt i64",
            "icmp sge i64",
        ] {
            assert!(llvm.contains(instruction));
        }
    }

    #[test]
    fn emits_product_construction_and_field_access() {
        let program = compile_to_ir(
            "type Point { x: f64 = 0.0, y: f64, }
             point: Point = Point { y: 2.0, };
             print(point.x);",
        )
        .unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("%cerune.type.Point.0 = type { double, double }"));
        assert!(llvm.contains("insertvalue %cerune.type.Point.0 poison, double"));
        assert!(llvm.contains("extractvalue %cerune.type.Point.0"));
    }

    #[test]
    fn allocates_loop_bindings_once_in_entry() {
        let program = compile_to_ir(
            "mut count: i64 = 0;
             while count < 2 {
                 marker: bool = true;
                 print(marker);
                 count = count + 1;
             }",
        )
        .unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert_eq!(llvm.matches("%cerune_marker = alloca i1").count(), 1);
        assert!(
            llvm.find("%cerune_marker = alloca i1").unwrap()
                < llvm.find("while_condition").unwrap()
        );
    }

    #[test]
    fn emits_typed_functions_and_calls() {
        let program = compile_to_ir(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);
             print(answer);",
        )
        .unwrap();
        let llvm = emit_llvm(&program).unwrap();

        assert!(llvm.contains("define i64 @cerune.fn.add.0(i64 %arg0, i64 %arg1)"));
        assert!(llvm.contains("store i64 %arg0, ptr %cerune_left"));
        assert!(llvm.contains("call i64 @cerune.fn.add.0(i64 20, i64 22)"));
        assert!(llvm.contains("ret i64"));
    }
}
