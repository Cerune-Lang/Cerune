//! HIRの実行が終了してから、上限内に収まるJSON Linesを新規ファイルへ保存します。
use cerune_lang::{
    ir::Program,
    runtime::trace::{Event, Incomplete, Site, Trace},
};
use serde::Serialize;
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

pub fn parse(args: Vec<String>) -> Result<(Vec<String>, Option<Config>), String> {
    if !args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--trace" | "--trace-event-limit" | "--trace-byte-limit"
        )
    }) {
        return Ok((args, None));
    }
    let mut path = None;
    let mut events = None;
    let mut bytes = None;
    let mut rest = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if !matches!(
            arg.as_str(),
            "--trace" | "--trace-event-limit" | "--trace-byte-limit"
        ) {
            let paired = matches!(
                arg.as_str(),
                "--file-root"
                    | "--diagnostic-format"
                    | "--process-config"
                    | "--argument-count-limit"
                    | "--argument-byte-limit"
                    | "--string-heap-limit"
                    | "--array-heap-limit"
                    | "--call-depth-limit"
            );
            if paired {
                let val = args
                    .next()
                    .filter(|s| !s.starts_with("--"))
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                rest.extend([arg, val]);
            } else {
                rest.push(arg);
            }
            continue;
        }
        let value = args
            .next()
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .ok_or_else(|| format!("missing value for {arg}"))?;
        if arg == "--trace" {
            if path.replace(PathBuf::from(value)).is_some() {
                return Err(format!("duplicate {arg}"));
            }
        } else {
            let limit = if arg == "--trace-event-limit" {
                &mut events
            } else {
                &mut bytes
            };
            if limit.is_some() {
                return Err(format!("duplicate {arg}"));
            }
            if !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("{arg} expects an unsigned decimal integer"));
            }
            *limit = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| format!("{arg} exceeds usize"))?,
            );
        }
    }
    if bytes.is_some_and(|n| n < MIN_BYTES) {
        return Err(format!("--trace-byte-limit must be at least {MIN_BYTES}"));
    }
    if path.is_none() && (events.is_some() || bytes.is_some()) {
        return Err("trace limits require --trace <new-file>".into());
    }
    Ok((
        rest,
        path.map(|path| Config {
            path,
            events: events.unwrap_or(DEFAULT_EVENTS),
            bytes: bytes.unwrap_or(DEFAULT_BYTES),
        }),
    ))
}

#[derive(Serialize)]
struct Row<'a> {
    schema: &'static str,
    seq: usize,
    #[serde(flatten)]
    kind: Kind<'a>,
    node: Option<usize>,
    file: usize,
    start: usize,
    end: usize,
}
#[derive(Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum Kind<'a> {
    Call { function: &'a str },
    Return { function: &'a str },
    Branch { taken: bool },
    Stop { code: &'a str },
}

fn row(program: &Program, event: Event, seq: usize) -> Row<'_> {
    let (kind, site) = match event {
        Event::Call { function, site } => (
            Kind::Call {
                function: &program.function_definitions[function.0].name,
            },
            site,
        ),
        Event::Return { function, site } => (
            Kind::Return {
                function: &program.function_definitions[function.0].name,
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
    Row {
        schema: "trace-v1",
        seq,
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

fn write_trace(
    out: &mut impl Write,
    program: &Program,
    trace: &Trace,
    mut remaining: usize,
) -> io::Result<()> {
    let end = footer(match trace.incomplete() {
        Some(Incomplete::AllocationFailed) => "allocation-failed",
        Some(Incomplete::EventLimit) => "event-limit",
        None => "complete",
    });
    let byte_limit = footer("byte-limit");

    for (index, &event) in trace.events().iter().enumerate() {
        let record = row(program, event, index + 1);
        // 最後の行は実際の終端まで含めて判定し、上限ちょうどでも保存する。
        let last = index + 1 == trace.events().len();
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
        let save = || {
            let file = File::create_new(&self.path)?;
            write_trace(&mut BufWriter::new(file), program, trace, self.bytes)
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
        let program = cerune_lang::compile_to_ir("fn main()->void {}").unwrap();
        let (_, trace) = cerune_lang::ir_executor::run_with_trace(&program, 2);
        for (remaining, expected) in [(7, "write failed"), (usize::MAX, "flush failed")] {
            let err =
                write_trace(&mut FailingWriter { remaining }, &program, &trace, 1024).unwrap_err();
            assert_eq!(err.to_string(), expected);
        }
    }
}
