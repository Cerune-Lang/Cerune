//! Pinned SASHIMI connection. Cerune owns lowering, runtime and source origins.
use super::Target;
use crate::diagnostic::Diagnostic;

pub(super) fn assemble(assembly: &str, target: Target) -> Result<Vec<u8>, Diagnostic> {
    let target = match target {
        Target::X86_64PcWindowsMsvc => sashimi_assembler::Target::X86_64PcWindowsMsvc,
        Target::X86_64UnknownLinuxGnu => sashimi_assembler::Target::X86_64UnknownLinuxGnu,
    };
    sashimi_assembler::assemble(assembly, target)
        .map_err(|error| Diagnostic::without_span(error.message))
}

#[cfg(test)]
#[path = "../../../tests/support/sashimi_connection.rs"]
mod tests;
