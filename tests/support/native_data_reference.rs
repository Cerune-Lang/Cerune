//! Frozen constant-data emission from Cerune 0eca28a; test-only migration reference.
use crate::codegen::x86_64::{
    Target,
    ir::{FloatConstant, Module},
};
use std::fmt::Write;
pub(super) fn render(module: &Module) -> String {
    let mut output = initial_data(module.target);
    if module.uses_strings {
        emit_data(module, &mut output);
    }
    for constant in &module.float_constants {
        emit_float_constant(constant, &mut output);
    }
    output
}
fn initial_data(target: Target) -> String {
    let mut output = String::new();

    output.push_str(if target.is_linux() {
        "# target: x86_64-unknown-linux-gnu\n.section .rodata\n"
    } else {
        ".section .rdata,\"dr\"\n"
    });

    // Unary minus masks.
    output.push_str(".p2align 4\n");

    output.push_str(".Lcerune_sign_f32:\n");

    output.push_str("  .long 0x80000000\n");

    output.push_str("  .long 0\n");

    output.push_str("  .long 0\n");

    output.push_str("  .long 0\n");

    output.push_str(".p2align 4\n");

    output.push_str(".Lcerune_sign_f64:\n");

    output.push_str("  .quad 0x8000000000000000\n");

    output.push_str("  .quad 0\n");

    output
}

fn emit_float_constant(constant: &FloatConstant, output: &mut String) {
    match constant {
        FloatConstant::F32 { id, bits } => {
            output.push_str(".p2align 2\n");

            output.push_str(&format!(".Lcerune_f32_{id}:\n"));

            output.push_str(&format!("  .long 0x{bits:08X}\n"));
        }

        FloatConstant::F64 { id, bits } => {
            output.push_str(".p2align 3\n");

            output.push_str(&format!(".Lcerune_f64_{id}:\n"));

            output.push_str(&format!("  .quad 0x{bits:016X}\n"));
        }
    }
}

fn emit_data(module: &Module, output: &mut String) {
    for (id, value) in module.strings.iter().enumerate() {
        writeln!(
            output,
            ".p2align 3\n.Lcerune_string_{id}:\n  .quad {}",
            value.len()
        )
        .unwrap();
        for byte in value.bytes() {
            writeln!(output, "  .byte {byte}").unwrap();
        }
    }
}
