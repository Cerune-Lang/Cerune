use super::{execute, hir, snapshot};
use cerune_lang::{
    format::{LetStyle, format},
    ir,
};

const EXAMPLE: &str = include_str!("../../examples/formatting/bindings.ceru");
const EXPECTED: &str = "6\n2\n5\n[9, 4]\n[2, 4]\nfalse\n";

pub(super) fn cases() -> Vec<(ir::Program, &'static str)> {
    let inferred = include_str!("../../examples/formatting/infer.ceru");
    let explicit = inferred.replace(": =", ": infer =");
    let inferred_output = "6\n[9, 4]\n[2, 4]\n18446744073709551615\n1.5\n日本語\nfalse\n";
    let mut programs = Vec::new();
    for (input, expected) in [
        (EXAMPLE, EXPECTED),
        (inferred, inferred_output),
        (&explicit, inferred_output),
    ] {
        programs.push((hir(input), expected));
        for style in [LetStyle::Preserve, LetStyle::Show, LetStyle::Hide] {
            let source = format(input, style).unwrap();
            assert_eq!(format(&source, style).unwrap(), source);
            programs.push((hir(&source), expected));
        }
    }
    programs
}
pub(super) fn check(output: &std::process::Output, expected: &str) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(output.stdout, expected.as_bytes());
}
#[test]
fn optional_let_formatting_matches_four_executors() {
    for (program, expected) in cases() {
        for route in 0..4 {
            assert_eq!(
                execute(route, &program, &snapshot(&[])).unwrap(),
                expected,
                "route {route}"
            );
        }
    }
}
