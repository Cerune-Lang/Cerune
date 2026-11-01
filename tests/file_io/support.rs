use super::language::runtime::files::*;
use super::language::{bytecode, ir, ir_executor, mir, mir_executor, modules, ssa_executor, vm};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    io::{self, Read, Write},
    rc::Rc,
};
#[derive(Clone, Copy)]
pub(super) enum Step {
    Count(usize),
    Interrupted,
    Fail,
    Invalid,
    Zero,
}
#[derive(Default)]
pub(super) struct State {
    pub(super) files: BTreeMap<String, Vec<u8>>,
    pub(super) calls: Vec<String>,
    pub(super) reads: VecDeque<Step>,
    pub(super) writes: VecDeque<Step>,
    pub(super) open_error: Option<OpenError>,
    pub(super) close_error: bool,
}
#[derive(Clone, Default)]
pub(super) struct Memory(pub(super) Rc<RefCell<State>>);
pub(super) struct MemoryHandle {
    memory: Memory,
    name: String,
    position: usize,
}
impl Backend for Memory {
    type Handle = MemoryHandle;
    fn open(&mut self, name: &str, operation: Operation) -> Result<MemoryHandle, OpenError> {
        let mut s = self.0.borrow_mut();
        s.calls.push(format!("open:{operation:?}:{name}"));
        if let Some(error) = s.open_error {
            return Err(error);
        }
        match operation {
            Operation::ReadText if !s.files.contains_key(name) => return Err(OpenError::NotFound),
            Operation::WriteNew => {
                if s.files.contains_key(name) {
                    return Err(OpenError::AlreadyExists);
                }
                s.files.insert(name.to_owned(), Vec::new());
            }
            _ => {}
        }
        Ok(MemoryHandle {
            memory: self.clone(),
            name: name.to_owned(),
            position: 0,
        })
    }
}
fn count(step: Option<Step>, requested: usize) -> io::Result<usize> {
    match step {
        Some(Step::Count(n)) => Ok(n.min(requested)),
        Some(Step::Interrupted) => Err(io::ErrorKind::Interrupted.into()),
        Some(Step::Fail) => Err(io::Error::other("private OS error")),
        Some(Step::Invalid) => Ok(requested + 1),
        Some(Step::Zero) => Ok(0),
        None => Ok(requested),
    }
}
impl Read for MemoryHandle {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push(format!("read:{}", into.len()));
        let n = count(s.reads.pop_front(), into.len())?;
        if n > into.len() {
            return Ok(n);
        }
        let data = &s.files[&self.name];
        let n = n.min(data.len() - self.position);
        into[..n].copy_from_slice(&data[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}
impl Write for MemoryHandle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push(format!("write:{}", bytes.len()));
        let n = count(s.writes.pop_front(), bytes.len())?;
        if n <= bytes.len() {
            s.files
                .get_mut(&self.name)
                .unwrap()
                .extend_from_slice(&bytes[..n]);
        }
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("file operations must not implicitly flush stdout")
    }
}
impl Handle for MemoryHandle {
    fn close(self) -> io::Result<()> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push("close".to_owned());
        if s.close_error {
            Err(io::ErrorKind::Interrupted.into())
        } else {
            Ok(())
        }
    }
}

pub(super) fn hir(source: &str) -> ir::Program {
    modules::from_source(source).unwrap().to_ir().unwrap()
}
pub(super) fn execute<B: Backend>(
    route: usize,
    hir: &ir::Program,
    files: &mut Files<B>,
) -> Result<String, (Failure, String)> {
    match route {
        0 => ir_executor::run_with_files(hir, files).map_err(|e| {
            let ir_executor::ErrorKind::Files(f) = e.kind() else {
                panic!("{e:?}")
            };
            assert_eq!(e.origin().unwrap().span, f.details.origin.span);
            (**f, e.output().into())
        }),
        1 => mir_executor::run_with_files(&mir::lower(hir).unwrap(), files).map_err(|e| {
            let mir_executor::ErrorKind::Files(f) = e.kind() else {
                panic!("{e:?}")
            };
            assert_eq!(e.origin().unwrap().span, f.details.origin.span);
            assert!(e.location().is_some());
            (**f, e.output().into())
        }),
        2 => ssa_executor::run_with_files(
            &mir::ssa::construct(&mir::lower(hir).unwrap()).unwrap(),
            files,
        )
        .map_err(|e| {
            let ssa_executor::ErrorKind::Files(f) = e.kind() else {
                panic!("{e:?}")
            };
            assert_eq!(e.origin().unwrap().span, f.details.origin.span);
            assert!(e.location().is_some());
            (**f, e.output().into())
        }),
        _ => vm::run_with_files(&bytecode::lower(hir).unwrap(), files).map_err(|e| {
            let vm::VmErrorKind::Files(f) = e.kind() else {
                panic!("{e:?}")
            };
            let code = bytecode::lower(hir).unwrap();
            let instructions = &code.functions[e.function_id().unwrap()].instructions;
            let bytecode::InstructionOrigin::Source { node_id, span } =
                instructions[e.instruction_index()].origin
            else {
                panic!()
            };
            assert_eq!(f.details.origin, Origin { node_id, span });
            (*f, e.output().into())
        }),
    }
}
