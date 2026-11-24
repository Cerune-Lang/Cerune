//! JSON Lines shared by the executors and the WAT host.
use super::{Event, Incomplete, Site, Trace};
use crate::ir::Program;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};
const DEFAULT_EVENTS: usize = 4096;
const DEFAULT_BYTES: usize = 1024 * 1024;
const END_PREFIX: &str = r#"{"schema":"trace-v1","event":"end","status":""#;
const END_SUFFIX: &str = "\"}\n";
const MIN_BYTES: usize = END_PREFIX.len() + "allocation-failed".len() + END_SUFFIX.len();
pub struct Config {
    path: PathBuf,
    pub events: usize,
    bytes: usize,
}

#[derive(Default)]
pub struct Options {
    path: Option<PathBuf>,
    events: Option<usize>,
    bytes: Option<usize>,
}
impl Options {
    pub fn set(&mut self, arg: &str, value: &std::ffi::OsStr) -> Result<(), String> {
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err(format!("missing value for {arg}"));
        }
        if arg == "--trace" {
            if self.path.replace(value.into()).is_some() {
                return Err(format!("duplicate {arg}"));
            }
        } else {
            let limit = match arg {
                "--trace-event-limit" => &mut self.events,
                "--trace-byte-limit" => &mut self.bytes,
                _ => return Err(format!("unknown trace option {arg}")),
            };
            if limit.is_some() {
                return Err(format!("duplicate {arg}"));
            }
            let value = value
                .to_str()
                .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| format!("{arg} expects an unsigned decimal integer"))?;
            *limit = Some(value.parse().map_err(|_| format!("{arg} exceeds usize"))?);
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Option<Config>, String> {
        if self.bytes.is_some_and(|n| n < MIN_BYTES) {
            return Err(format!("--trace-byte-limit must be at least {MIN_BYTES}"));
        }
        if self.path.is_none() && (self.events.is_some() || self.bytes.is_some()) {
            return Err("trace limits require --trace <new-file>".into());
        }
        Ok(self.path.map(|path| Config {
            path,
            events: self.events.unwrap_or(DEFAULT_EVENTS),
            bytes: self.bytes.unwrap_or(DEFAULT_BYTES),
        }))
    }
}

#[derive(Serialize)]
struct Row<T> {
    schema: &'static str,
    seq: usize,
    #[serde(flatten)]
    fields: T,
}
#[derive(Serialize, Deserialize)]
pub struct Fields<S> {
    #[serde(flatten)]
    kind: Kind<S>,
    node: Option<usize>,
    file: usize,
    start: usize,
    end: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum Kind<S> {
    Call { function: S },
    Return { function: S },
    Branch { taken: bool },
    Stop { code: S },
}
fn row(program: &Program, event: Event) -> Fields<&str> {
    let (kind, site) = match event {
        Event::Call { function, site } => (
            Kind::Call {
                function: program.function_definitions[function.0].name.as_str(),
            },
            site,
        ),
        Event::Return { function, site } => (
            Kind::Return {
                function: program.function_definitions[function.0].name.as_str(),
            },
            site,
        ),
        Event::Branch { taken, site } => (Kind::Branch { taken }, site),
        Event::Stop(stop) => (
            Kind::Stop {
                code: stop.code.name(),
            },
            Site {
                node: Some(stop.node_id),
                span: stop.span,
            },
        ),
    };
    Fields {
        kind,
        node: site.node.map(|n| n.0),
        file: site.span.source_id().index(),
        start: site.span.start(),
        end: site.span.end(),
    }
}
fn footer(status: &str) -> String {
    format!("{END_PREFIX}{status}{END_SUFFIX}")
}

// 先にシリアライズ長を数え、収まる行だけを出力する。行全体をメモリに複製しない。
#[derive(Default)]
struct ByteCount(usize);
impl Write for ByteCount {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_trace<T: Serialize>(
    out: &mut impl Write,
    count: usize,
    incomplete: Option<Incomplete>,
    mut remaining: usize,
    fields: impl Fn(usize) -> T,
) -> io::Result<()> {
    let end = footer(match incomplete {
        Some(Incomplete::AllocationFailed) => "allocation-failed",
        Some(Incomplete::EventLimit) => "event-limit",
        None => "complete",
    });
    let byte_limit = footer("byte-limit");

    for index in 0..count {
        let record = Row {
            schema: "trace-v1",
            seq: index + 1,
            fields: fields(index),
        };
        // 最後の行は実際の終端まで含めて判定し、上限ちょうどでも保存する。
        let last = index + 1 == count;
        let footer_size = if last { end.len() } else { byte_limit.len() };
        let available = remaining.saturating_sub(footer_size);
        let mut length = ByteCount::default();
        serde_json::to_writer(&mut length, &record)?;
        if length.0 >= available {
            out.write_all(byte_limit.as_bytes())?;
            return out.flush();
        }
        serde_json::to_writer(&mut *out, &record)?;
        out.write_all(b"\n")?;
        remaining -= length.0 + 1;
    }
    out.write_all(end.as_bytes())?;
    out.flush()
}

impl Config {
    pub fn save(&self, program: &Program, trace: &Trace) -> Result<(), String> {
        self.save_rows(trace.events().len(), trace.incomplete(), |i| {
            row(program, trace.events()[i])
        })
    }
    pub fn save_rows<T: Serialize>(
        &self,
        count: usize,
        incomplete: Option<Incomplete>,
        fields: impl Fn(usize) -> T,
    ) -> Result<(), String> {
        let save = || {
            let file = File::create_new(&self.path)?;
            write_trace(
                &mut BufWriter::new(file),
                count,
                incomplete,
                self.bytes,
                fields,
            )
        };
        save().map_err(|e| format!("trace-v1: could not save {}: {e}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailingWriter {
        remaining: usize,
    }
    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("write failed"));
            }
            let n = bytes.len().min(self.remaining);
            self.remaining -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }
    #[test]
    fn trace_writer_reports_partial_writes_and_flush_failures() {
        let program = crate::compile_to_ir("fn main()->void {}").unwrap();
        let (_, trace) = crate::ir_executor::run_with_trace(&program, 2);
        for (remaining, expected) in [(7, "write failed"), (usize::MAX, "flush failed")] {
            let err = write_trace(
                &mut FailingWriter { remaining },
                trace.events().len(),
                trace.incomplete(),
                1024,
                |i| row(&program, trace.events()[i]),
            )
            .unwrap_err();
            assert_eq!(err.to_string(), expected);
        }
    }
}
