#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{compile_to_c, compile_to_wat, compile_to_x86_64_win_asm};

const SOURCE: &str = include_str!("fixtures/observation/float-output/source.ceru");

#[test]
fn tiny_and_large_values_remain_observable_through_the_vm() {
    assert_eq!(
        run_both(SOURCE).unwrap(),
        concat!(
            "9.99999968e-21\n",
            "9.9999999999999995e-21\n",
            "true\n",
            "1.40129846e-45\n",
            "4.9406564584124654e-324\n",
            "3.40282347e+38\n",
            "1.7976931348623157e+308\n",
            "-0\n",
            "-0\n",
            "0\n",
            "0\n",
            "9.99999975e-05\n",
            "0.0001\n",
            "1e+09\n",
            "1e+17\n",
        )
    );
}

#[test]
fn generated_backends_keep_significant_digit_formats_and_typed_host_calls() {
    for emit in [
        compile_to_c,
        llvm::compile,
        compile_to_qbe,
        compile_to_x86_64_win_asm,
    ] {
        let artifact = emit(SOURCE).unwrap();
        assert!(artifact.contains("%.9g"));
        assert!(artifact.contains("%.17g"));
    }
    let wat = compile_to_wat(SOURCE).unwrap();
    assert!(wat.contains("(func $format_f32_v1 (param f32 i32) (result i64 i32))"));
    assert!(wat.contains("(func $format_f64_v1 (param f64 i32) (result i64 i32))"));
    assert!(wat.contains("call $cerune_output_f32"));
    assert!(wat.contains("call $cerune_output_f64"));
}

#[test]
fn printing_does_not_change_the_value_used_by_later_computation() {
    let source = "
        fn show(value: f64) -> f64 { print(value); return value; }
        small: f64 = 1e-20;
        print(show(small) == small);
        print(show(small) * 1e20);
    ";
    assert_eq!(
        run_both(source).unwrap(),
        "9.9999999999999995e-21\ntrue\n9.9999999999999995e-21\n1\n"
    );
}

fn run_both(source: &str) -> Result<String, cerune_lang::RunError> {
    let output = cerune_lang::run_vm(source)?;
    assert_eq!(cerune_lang::run_ir(source).unwrap(), output);
    Ok(output)
}

#[test]
fn separate_rounding_and_signed_zero_match_known_bits_in_all_executors() {
    let source = include_str!("../examples/float_rounding.ceru");
    let hir = cerune_lang::compile_to_ir(source).unwrap();
    let mir = cerune_lang::mir::lower(&hir).unwrap();
    let ssa = cerune_lang::mir::ssa::construct(&mir).unwrap();
    let lowered = cerune_lang::mir::ssa::lower(&ssa).unwrap();
    for output in [
        cerune_lang::ir_executor::run(&hir).unwrap(),
        cerune_lang::mir_executor::run(&mir).unwrap(),
        cerune_lang::ssa_executor::run(&ssa).unwrap(),
        cerune_lang::mir_executor::run(&lowered.program).unwrap(),
        cerune_lang::run_vm(source).unwrap(),
    ] {
        assert_eq!(output, "0\n0\n0\n0\n-0\n-0\ntrue\ntrue\n");
        let lines: Vec<_> = output.lines().collect();
        for index in [0, 2, 4] {
            assert_eq!(
                lines[index].parse::<f32>().unwrap().to_bits(),
                if index == 4 { 0x8000_0000 } else { 0 }
            );
        }
        for index in [1, 3, 5] {
            assert_eq!(
                lines[index].parse::<f64>().unwrap().to_bits(),
                if index == 5 { 0x8000_0000_0000_0000 } else { 0 }
            );
        }
    }
    // Independently known fused answers demonstrate that this fixture detects
    // replacing two rounded operations with a single rounded operation.
    assert_eq!(
        (1.0f32 + 2.0f32.powi(-23))
            .mul_add(1.0 - 2.0f32.powi(-23), -1.0)
            .to_bits(),
        0xa880_0000
    );
    assert_eq!(
        (1.0f64 + 2.0f64.powi(-52))
            .mul_add(1.0 - 2.0f64.powi(-52), -1.0)
            .to_bits(),
        0xb970_0000_0000_0000
    );
}
