use super::reference_api as cerune_lang;
use cerune_lang::{
    ir,
    runtime::output::{Failure, Output},
};
use std::io::{self, Write};

struct Writer {
    bytes: Vec<u8>,
    limit: usize,
    chunk: usize,
    mode: &'static str,
    interrupted: bool,
    interrupted_after: bool,
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if !self.interrupted_after && !self.bytes.is_empty() {
            self.interrupted_after = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.bytes.len() == self.limit {
            return match self.mode {
                "zero" => Ok(0),
                "invalid" => Ok(bytes.len() + 1),
                _ => Err(io::ErrorKind::BrokenPipe.into()),
            };
        }
        let n = bytes
            .len()
            .min(self.chunk)
            .min(self.limit - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn reference(
    program: &ir::Program,
    route: usize,
    limit: usize,
    chunk: usize,
    mode: &'static str,
    trace: usize,
) -> (Vec<u8>, Option<Failure>) {
    let mut output = Output::with_trace_limit(
        Writer {
            bytes: vec![],
            limit,
            chunk,
            mode,
            interrupted: false,
            interrupted_after: false,
        },
        trace,
    );
    let ok = match route {
        0 => cerune_lang::ir_executor::run_with_output(program, &mut output).is_ok(),
        1 => cerune_lang::mir_executor::run_with_output(
            &cerune_lang::mir::lower(program).unwrap(),
            &mut output,
        )
        .is_ok(),
        2 => cerune_lang::ssa_executor::run_with_output(
            &cerune_lang::mir::ssa::construct(&cerune_lang::mir::lower(program).unwrap()).unwrap(),
            &mut output,
        )
        .is_ok(),
        _ => cerune_lang::run_bytecode_with_output(
            &cerune_lang::bytecode::lower(program).unwrap(),
            &mut output,
        )
        .is_ok(),
    };
    let failure = output.failure();
    assert_eq!(ok, failure.is_none());
    (output.into_inner().bytes, failure)
}
