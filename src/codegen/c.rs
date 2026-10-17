mod array;
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
mod sequence;
mod string;
mod unsigned;

pub use emit::emit;
use lower::lower;

use crate::{diagnostic::Diagnostic, ir as cerune_ir};

pub fn emit_c(program: &cerune_ir::Program) -> Result<String, Diagnostic> {
    let module = lower(program);

    Ok(emit(&module))
}

#[cfg(test)]
mod tests {
    use crate::compile_to_ir;

    use super::{
        emit_c,
        ir::{BinaryOp, ExprKind, Statement},
        lower,
    };

    #[test]
    fn lowers_binding_to_c_ir() {
        let program = compile_to_ir("x: f32 = 0.1 + 0.2;").unwrap();
        let module = lower(&program);

        assert!(matches!(
            &module.statements[0],
            Statement::Binding {
                ty: super::ir::Type::Float,
                ..
            }
        ));
    }

    #[test]
    fn emits_contextual_f32_literals() {
        let program = compile_to_ir(
            "x: f32 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let c = emit_c(&program).unwrap();

        assert!(c.contains("float cerune_binding_0_x = (0.1f + 0.2f);"));
    }

    #[test]
    fn generated_c_disables_clang_and_gcc_floating_point_contraction() {
        let program = compile_to_ir("x: f64 = 0.1 * 2.0 + 0.3; print(x);").unwrap();
        let c = emit_c(&program).unwrap();
        let directive = "#if defined(__clang__)\n#pragma clang fp contract(off)\n#elif defined(__GNUC__)\n#pragma GCC optimize (\"fp-contract=off\")\n#endif\n";
        let start = c
            .find(directive)
            .expect("Clang/GCC FP contraction directive");
        assert!(start < c.find("int main(void)").unwrap());
    }

    #[test]
    fn emits_contextual_f64_literals() {
        let program = compile_to_ir(
            "x: f64 = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let c = emit_c(&program).unwrap();

        assert!(c.contains("double cerune_binding_0_x = (0.1 + 0.2);"));
    }

    #[test]
    fn inferred_float_defaults_to_f64() {
        let program = compile_to_ir(
            "x: infer = 0.1 + 0.2;
             print(x);",
        )
        .unwrap();

        let c = emit_c(&program).unwrap();

        assert!(c.contains("double cerune_binding_0_x = (0.1 + 0.2);"));
    }

    #[test]
    fn emits_all_comparison_operators() {
        let program = compile_to_ir(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let c = emit_c(&program).unwrap();

        for operator in ["==", "!=", "<", "<=", ">", ">="] {
            assert!(c.contains(operator));
        }
    }

    #[test]
    fn emits_typed_functions_and_calls() {
        let program = compile_to_ir(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);
             print(answer);",
        )
        .unwrap();
        let c = emit_c(&program).unwrap();

        assert!(c.contains("int64_t cerune_fn_add_0(int64_t cerune_binding_0_left, int64_t cerune_binding_1_right);"));
        assert!(c.contains(
            "return cerune_i64_add(cerune_binding_0_left, cerune_binding_1_right, \" node="
        ));
        assert!(c.contains("cerune_fn_add_0(20, 22)"));
    }

    #[test]
    fn emits_i64_minimum_without_an_out_of_range_c_literal() {
        let program = compile_to_ir("x: i64 = -9223372036854775808;").unwrap();

        let c = emit_c(&program).unwrap();

        assert!(c.contains("int64_t cerune_binding_0_x = INT64_MIN;"));
    }

    #[test]
    fn keeps_checked_i64_arithmetic_visible_in_c_ir() {
        let program = compile_to_ir("x: i64 = 1 + 2;").unwrap();
        let module = lower(&program);

        assert!(matches!(
            &module.statements[0],
            Statement::Binding {
                value: super::ir::Expr {
                    kind: ExprKind::Binary {
                        op: BinaryOp::CheckedI64Add,
                        ..
                    },
                    ..
                },
                ..
            }
        ));
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
        let c = emit_c(&program).unwrap();

        for helper in [
            "cerune_i64_add",
            "cerune_i64_sub",
            "cerune_i64_mul",
            "cerune_i64_div",
            "cerune_i64_neg",
        ] {
            assert!(c.contains(helper));
        }
        assert!(c.contains("INT64_MAX"));
        assert!(c.contains("INT64_MIN"));
    }
}
