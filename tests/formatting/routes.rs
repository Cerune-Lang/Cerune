use super::{execute, hir, snapshot};
use cerune_lang::{
    format::{LetStyle, format},
    ir,
};

const EXAMPLE: &str = include_str!("../../examples/formatting/bindings.ceru");
const EXPECTED: &str = "6\n2\n5\n[9, 4]\n[2, 4]\nfalse\n";

pub(super) fn cases() -> Vec<(ir::Program, &'static str)> {
    let mut programs = vec![(hir(EXAMPLE), EXPECTED)];
    for style in [LetStyle::Preserve, LetStyle::Show, LetStyle::Hide] {
        let source = format(EXAMPLE, style).unwrap();
        assert_eq!(format(&source, style).unwrap(), source);
        programs.push((hir(&source), EXPECTED));
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
