//! Cerune instruction selection and immutable source mapping for one typed Program.
use super::{ir, machine::*};
use crate::diagnostic::Diagnostic;
use sashimi_assembler::program::{Artifact, OriginId, SymbolId};

/// One LIR operation's source and MIR identity; IDs are local to this Program.
#[derive(Clone, Debug)]
pub struct Origin {
    pub source: ir::Origin,
    pub mir: Option<ir::MirOrigin>,
    pub function: Option<usize>,
    pub instruction: usize,
}

/// ASM and Object are rendered from the same immutable typed instructions.
/// SASHIMI OriginId indexes origins(); it is not a Cerune NodeId.
#[derive(Clone, Debug)]
pub struct NativeProgram {
    pub(super) program: Program,
    origins: Vec<Origin>,
    comments: std::collections::BTreeMap<usize, String>,
    preamble: String,
    provenance: Provenance,
}
impl NativeProgram {
    pub(super) fn new(target: super::Target) -> Self {
        Self {
            program: Program::new(super::machine::target(target)),
            origins: vec![],
            comments: Default::default(),
            preamble: String::new(),
            provenance: Provenance::synthetic(),
        }
    }
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn origins(&self) -> &[Origin] {
        &self.origins
    }
    pub fn assemble(&self) -> Result<Artifact, Diagnostic> {
        sashimi_assembler::program::assemble_program(&self.program).map_err(|e| self.diagnostic(e))
    }
    pub fn render_asm(&self) -> Result<String, Diagnostic> {
        let text = sashimi_assembler::program::render_asm(&self.program)
            .map_err(|e| self.diagnostic(e))?;
        let mut out = self.preamble.clone();
        // Read SASHIMI's entry delimiters only to place Cerune's source comments.
        // No instruction, operand or directive is parsed or used to produce Object.
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("# entry ") {
                let id: usize = rest.split_whitespace().next().unwrap().parse().unwrap();
                if let Some(comment) = self.comments.get(&id) {
                    out.push_str(comment);
                }
            } else {
                if !line.starts_with('.') && !line.ends_with(':') {
                    out.push_str("  ");
                }
                out.push_str(line);
                out.push('\n');
            }
        }
        Ok(out)
    }
    fn diagnostic(&self, error: sashimi_assembler::program::ProgramDiagnostic) -> Diagnostic {
        for id in &error.origins {
            if let Some(Origin {
                source: ir::Origin::Source { span, .. },
                ..
            }) = self.origins.get(id.0 as usize)
            {
                return Diagnostic::new(error.to_string(), *span);
            }
        }
        Diagnostic::without_span(error.to_string())
    }
    pub(super) fn source(&mut self, origin: Origin) {
        let synthetic = matches!(origin.source, ir::Origin::Synthetic);
        let id = OriginId(self.origins.len() as u64);
        self.origins.push(origin);
        self.provenance = Provenance {
            origins: vec![id],
            synthetic,
        };
        self.push(EntryKind::Marker);
    }
    pub(super) fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }
    pub(super) fn synthetic(&mut self) {
        self.provenance = Provenance::synthetic();
    }
    pub(super) fn push(&mut self, kind: EntryKind) {
        self.program.push(kind, self.provenance.clone());
    }
    pub(super) fn emit(&mut self, opcode: Opcode, operands: impl Into<Vec<Operand>>) {
        self.push(EntryKind::Instruction(Instruction::new(opcode, operands)));
    }
    pub(super) fn symbol(&mut self, name: &str, linkage: Linkage) -> SymbolId {
        if let Some((id, s)) = self
            .program
            .symbols
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == name)
        {
            assert_eq!(s.linkage, linkage, "conflicting linkage for {name}");
            SymbolId(id)
        } else {
            self.program.declare(name, linkage)
        }
    }
    pub(super) fn label(&mut self, name: &str) {
        let s = self.symbol(name, Linkage::Local);
        self.push(EntryKind::Label(s));
    }
    pub(super) fn global_label(&mut self, name: &str) {
        let s = self.symbol(name, Linkage::Global);
        self.push(EntryKind::Label(s));
    }
    pub(super) fn comment(&mut self, text: &str) {
        if self.program.entries.is_empty() {
            self.preamble.push_str(&format!("# {text}\n"));
            return;
        }
        self.comments
            .insert(self.program.entries.len(), format!("# {text}\n"));
        self.push(EntryKind::Marker);
    }
}

pub(super) fn xmm(n: usize) -> Operand {
    Register::xmm(n.try_into().expect("XMM number"))
        .expect("XMM register")
        .into()
}
pub(super) fn address(base: u8, index: Option<(u8, Scale)>, displacement: i128) -> Operand {
    Memory::new(
        Register::gpr(base, Width::Qword).expect("base"),
        index.map(|(n, s)| (Register::gpr(n, Width::Qword).expect("index"), s)),
        displacement.try_into().expect("validated LIR displacement"),
    )
    .expect("LIR address")
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoding_failure_uses_the_source_span_not_a_machine_entry_number() {
        let hir = crate::compile_to_ir("print(7);").unwrap();
        let mir = crate::mir::lower(&hir).unwrap();
        let lir =
            super::super::lower_mir(&mir, super::super::Target::X86_64UnknownLinuxGnu).unwrap();
        let mut program = super::super::build_program_from_lir(&lir, false).unwrap();
        let entry = program
            .program
            .entries
            .iter_mut()
            .find(|e| !e.provenance.synthetic && matches!(e.kind, EntryKind::Instruction(_)))
            .unwrap();
        let id = entry.provenance.origins[0];
        entry.kind = EntryKind::Instruction(Instruction::new(Opcode::Mov64, [r32(0), r64(1)]));
        let ir::Origin::Source { span, .. } = program.origins[id.0 as usize].source else {
            panic!("source");
        };
        let error = program.assemble().unwrap_err();
        assert_eq!(error.primary_span(), Some(span));
        assert_eq!(program.render_asm().unwrap_err(), error);
    }
}
