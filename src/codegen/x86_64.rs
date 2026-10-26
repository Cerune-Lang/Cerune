mod abi;
mod arguments;
mod array;
mod call;
mod conversion;
mod data;
mod emit;
#[cfg(test)]
#[path = "../../tests/support/native_emitter_reference/mod.rs"]
mod emitter_reference;
mod failure;
mod heap;
mod input;
pub mod ir;
pub mod lir;
mod lower;
mod machine;
#[cfg(test)]
mod object;
mod program;
pub use program::{NativeProgram, Origin as NativeOrigin};
#[cfg(test)]
#[path = "../../tests/support/native_object_reference/mod.rs"]
mod object_reference;
mod output;
mod rounding;
mod string;
#[cfg(test)]
#[path = "../../tests/support/process.rs"]
mod test_process;
mod unsigned;
mod utf8;

pub use emit::emit;
/// MIRから直接Native LIRを作ります。ターゲットは必須です。
pub use lower::{lower as lower_mir, lower_with_argument_limits as lower_mir_with_argument_limits};
/// 同じ上限付きLIRからASM・自前Objectの両方を生成できます。
pub fn lower_mir_with_call_limit(
    program: &crate::mir::Program,
    target: Target,
    limit: std::num::NonZeroU32,
) -> Result<ir::Module, crate::diagnostic::Diagnostic> {
    let mut module = lower_mir(program, target)?;
    module.call_limit = Some(limit);
    Ok(module)
}

/// 引数とソース関数の深さ上限を保持する共通LIRを構築します。
pub fn lower_mir_with_runtime_limits(
    program: &crate::mir::Program,
    target: Target,
    arguments: crate::runtime::arguments::Limits,
    calls: std::num::NonZeroU32,
) -> Result<ir::Module, crate::diagnostic::Diagnostic> {
    let mut module = lower_mir_with_argument_limits(program, target, arguments)?;
    module.call_limit = Some(calls);
    Ok(module)
}
#[cfg(test)]
fn lower(program: &crate::ir::Program) -> ir::Module {
    lower_mir(
        &crate::mir::lower(program).unwrap(),
        Target::X86_64PcWindowsMsvc,
    )
    .unwrap()
}
fn lower_with_target(
    program: &crate::ir::Program,
    target: Target,
) -> Result<ir::Module, crate::diagnostic::Diagnostic> {
    lower_mir(
        &crate::mir::lower(program).map_err(|e| e.diagnostic())?,
        target,
    )
}

use crate::{diagnostic::Diagnostic, ir as cerune_ir};

/// 実行中のOSから補わず、呼び出し側が生成先を選びます。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    X86_64PcWindowsMsvc,
    X86_64UnknownLinuxGnu,
}

impl Target {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "x86_64-pc-windows-msvc" => Some(Self::X86_64PcWindowsMsvc),
            "x86_64-unknown-linux-gnu" => Some(Self::X86_64UnknownLinuxGnu),
            _ => None,
        }
    }

    pub fn triple(self) -> &'static str {
        match self {
            Self::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
        }
    }

    fn is_linux(self) -> bool {
        self == Self::X86_64UnknownLinuxGnu
    }
}

/// 既に構築したMIR snapshotから生成します。観測した値と同じ入力を使えます。
pub fn emit_asm_from_mir(
    program: &crate::mir::Program,
    target: Target,
    annotate_origins: bool,
) -> Result<String, Diagnostic> {
    emit_asm_from_lir(&lower_mir(program, target)?, annotate_origins)
}
/// 表示・保存したものと同じLIRを検証してASMを生成します。
pub fn emit_asm_from_lir(
    module: &ir::Module,
    annotate_origins: bool,
) -> Result<String, Diagnostic> {
    build_program_from_lir(module, annotate_origins)?.render_asm()
}

/// LIRを検証し、ASM・Object・出自対応を共有する型付きProgramを構築します。
pub fn build_program_from_lir(
    module: &ir::Module,
    annotate_origins: bool,
) -> Result<NativeProgram, Diagnostic> {
    lir::validate(module)?;
    Ok(emit::build(module, annotate_origins))
}

/// 同じ型付きProgramから、ASM文字列を経由せずObjectを生成します。
pub fn emit_object_from_lir(
    module: &ir::Module,
    annotate_origins: bool,
) -> Result<Vec<u8>, Diagnostic> {
    Ok(build_program_from_lir(module, annotate_origins)?
        .assemble()?
        .object)
}

pub fn emit_object_from_mir(
    program: &crate::mir::Program,
    target: Target,
    annotate_origins: bool,
) -> Result<Vec<u8>, Diagnostic> {
    emit_object_from_lir(&lower_mir(program, target)?, annotate_origins)
}

pub fn emit_asm(program: &cerune_ir::Program, target: Target) -> Result<String, Diagnostic> {
    emit_asm_from_lir(&lower_with_target(program, target)?, false)
}

/// 自前の符号化・オブジェクト生成です。リンクや外部プロセス起動は行いません。
pub fn emit_object(
    program: &cerune_ir::Program,
    target: Target,
    annotate_origins: bool,
) -> Result<Vec<u8>, Diagnostic> {
    emit_object_from_lir(&lower_with_target(program, target)?, annotate_origins)
}

pub fn emit_asm_with_origins(
    program: &cerune_ir::Program,
    target: Target,
) -> Result<String, Diagnostic> {
    emit_asm_from_lir(&lower_with_target(program, target)?, true)
}

pub fn emit_x86_64_win_asm(program: &cerune_ir::Program) -> Result<String, Diagnostic> {
    let module = lower_with_target(program, Target::X86_64PcWindowsMsvc)?;

    emit_asm_from_lir(&module, false)
}

#[cfg(test)]
mod tests {
    use crate::compile_to_ir;

    use super::{
        emit_x86_64_win_asm,
        ir::{BinaryOp, FloatConstant, Instruction},
        lower,
    };

    #[test]
    fn probes_stack_allocations_of_a_page_or_more_in_main_and_functions() {
        let program =
            compile_to_ir("fn value(x: i64) -> i64 { return x; } print(value(7));").unwrap();
        for frame_size in [4080, 4096, 4112, 32768] {
            let mut module = lower(&program);
            module.frame_size = frame_size;
            module.functions[0].frame_size = frame_size;
            let asm = super::emit(&module);
            if frame_size >= 4096 {
                let allocation =
                    format!("movq ${frame_size}, %rax\n  callq __chkstk\n  subq %rax, %rsp");
                assert_eq!(asm.matches(&allocation).count(), 2);
            } else {
                assert!(!asm.contains("__chkstk"));
                assert_eq!(asm.matches(&format!("subq ${frame_size}, %rsp")).count(), 2);
            }
            assert!(asm.contains(&format!("addq ${frame_size}, %rsp")));
        }
    }

    #[test]
    fn lowers_i64_add_to_asm_ir() {
        let program = compile_to_ir("x: i64 = 1 + 2;").unwrap();
        let module = lower(&program);

        assert!(
            module
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, Instruction::I64Binary(BinaryOp::Add)))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, Instruction::TrapIfOverflow(_)))
        );
    }

    #[test]
    fn lowers_f32_constant_to_asm_ir() {
        let program = compile_to_ir("x: f32 = 0.1;").unwrap();
        let module = lower(&program);

        assert!(matches!(
            module.float_constants.first(),
            Some(FloatConstant::F32 { .. })
        ));
    }

    #[test]
    fn emits_i64_arithmetic() {
        let program = compile_to_ir(
            "x: i64 = 1 + 2;
             print(x);",
        )
        .unwrap();

        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("addq %rcx, %rax"));
        assert!(asm.contains("jno .Lcerune_main_integer_ok_"));
        assert!(asm.contains("movl $7, %ecx\n  int $41"));

        assert!(asm.contains("callq cerune_output_i64"));
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
        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("jno .Lcerune_main_integer_ok_"));
        assert!(asm.contains("je .Lcerune_main_division_trap_"));
        assert!(asm.contains("movabsq $-9223372036854775808, %rdx"));
        assert!(asm.contains("movl $7, %ecx\n  int $41"));
    }

    #[test]
    fn emits_f32_arithmetic() {
        let program = compile_to_ir(
            "x: f32 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("addss %xmm1, %xmm0"));

        assert!(asm.contains("cvtss2sd %xmm0, %xmm0"));
    }

    #[test]
    fn emits_f64_arithmetic() {
        let program = compile_to_ir(
            "x: f64 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("addsd %xmm1, %xmm0"));

        assert!(asm.contains("movapd %xmm0, %xmm3"));
    }

    #[test]
    fn emits_all_integer_comparisons() {
        let program = compile_to_ir(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let asm = emit_x86_64_win_asm(&program).unwrap();

        for instruction in [
            "sete %al",
            "setne %al",
            "setl %al",
            "setle %al",
            "setg %al",
            "setge %al",
        ] {
            assert!(asm.contains(instruction));
        }
    }

    #[test]
    fn lowers_product_values_to_stack_fields() {
        let program = compile_to_ir(
            "type Point { x: f64 = 0.0, y: f64, }
             point: Point = Point { y: 2.0, };
             print(point.x);",
        )
        .unwrap();
        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("movsd %xmm0, -8(%rbp)"));
        assert!(asm.contains("movsd %xmm0, -16(%rbp)"));
        assert!(asm.contains("movsd -8(%rbp), %xmm0"));
        assert!(asm.contains("callq cerune_output_f64"));
    }

    #[test]
    fn emits_typed_functions_with_windows_register_arguments() {
        let program = compile_to_ir(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);
             print(answer);",
        )
        .unwrap();
        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert!(asm.contains("cerune_fn_add_0:"));
        assert!(asm.contains("movq %rcx, -8(%rbp)"));
        assert!(asm.contains("movq %rdx, -16(%rbp)"));
        assert!(asm.contains("callq cerune_fn_add_0"));
    }

    #[test]
    fn preserves_nested_calls_and_mixed_argument_registers() {
        let program = compile_to_ir(
            "fn twice(value: i64) -> i64 { return value * 2; }
             fn select(a: i64, b: f64, c: i64, d: f32) -> f64 { return b; }
             result: f64 = select(twice(1), 2.5, twice(3), 4.5f32);
             print(result);",
        )
        .unwrap();
        let asm = emit_x86_64_win_asm(&program).unwrap();

        assert_eq!(asm.matches("callq cerune_fn_twice_0").count(), 2);
        assert!(asm.contains("%rcx"));
        assert!(asm.contains("%xmm1"));
        assert!(asm.contains("%r8"));
        assert!(asm.contains("%xmm3"));
    }
}
