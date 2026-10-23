//! Test-only text assembler for fault injection and frozen parser comparisons.
//! Production uses NativeProgram::assemble directly.
use super::Target;
use crate::diagnostic::Diagnostic;

pub(super) fn assemble(assembly: &str, target: Target) -> Result<Vec<u8>, Diagnostic> {
    let target = match target {
        Target::X86_64PcWindowsMsvc => sashimi_assembler::Target::X86_64PcWindowsMsvc,
        Target::X86_64UnknownLinuxGnu => sashimi_assembler::Target::X86_64UnknownLinuxGnu,
    };
    sashimi_assembler::assemble(&legacy_text(assembly), target)
        .map_err(|error| Diagnostic::without_span(error.message))
}

#[cfg(test)]
#[path = "../../../tests/support/sashimi_connection.rs"]
mod tests;

// Test-only fault injection still substitutes external calls in saved ASM.
// The frozen parser predates explicit external declarations and @PLT display.
pub(super) fn legacy_text(assembly: &str) -> String {
    assembly
        .lines()
        .filter(|line| !line.starts_with(".extern "))
        .map(|line| line.strip_suffix("@PLT").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}
