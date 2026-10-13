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
    assert!(wat.contains("(func $print_f32 (param f32))"));
    assert!(wat.contains("(func $print_f64 (param f64))"));
    assert!(wat.contains("call $print_f32"));
    assert!(wat.contains("call $print_f64"));
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
