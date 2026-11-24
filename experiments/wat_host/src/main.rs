mod files;
mod options;
mod trace;
use options::options;
#[cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
mod process;
use cerune_lang::runtime::{files::Directory, float_output, input};
use files::{Files, ProtocolError};
use std::{
    fs,
    io::{self, Read, Write},
};
use wasmi::{Caller, Engine, Error, Linker, Module, Store, TrapCode};

struct State {
    trace: Option<trace::Capture>,
    #[cfg(all(
        target_arch = "x86_64",
        any(
            all(windows, target_env = "msvc"),
            all(target_os = "linux", target_env = "gnu")
        )
    ))]
    process: Option<process::Process>,
    files: Files<Directory>,
    input: input::Stdin,
    input_failed: bool,
    output: fs::File,
    error: fs::File,
    diagnostics: bool,
    arguments: Vec<Vec<u8>>,
}
fn protocol(_: ProtocolError) -> Error {
    Error::new("invalid file host protocol")
}
fn chunk(text: String, offset: i32) -> Result<(i64, i32), Error> {
    let offset = usize::try_from(offset).map_err(|_| Error::new("invalid format offset"))?;
    if offset % 8 != 0 || offset >= text.len() {
        return Err(Error::new("invalid format offset"));
    }
    let mut bytes = [0; 8];
    let remaining = &text.as_bytes()[offset..];
    let n = remaining.len().min(8);
    bytes[..n].copy_from_slice(&remaining[..n]);
    Ok((i64::from_le_bytes(bytes), text.len() as i32))
}
fn imports(linker: &mut Linker<State>) -> Result<(), Error> {
    linker.func_wrap(
        "cerune",
        "trace_event_v1",
        |mut c: Caller<'_, State>, id: i32| {
            if let Some(trace) = &mut c.data_mut().trace {
                trace.record(id)?;
            }
            Ok::<(), Error>(())
        },
    )?;
    linker.func_wrap("cerune", "files_status_v1", |c: Caller<'_, State>| {
        c.data().files.status().map_err(protocol)
    })?;
    linker.func_wrap(
        "cerune",
        "file_name_begin_v1",
        |mut c: Caller<'_, State>, n: i32| c.data_mut().files.begin(n).map_err(protocol),
    )?;
    linker.func_wrap(
        "cerune",
        "file_name_chunk_v1",
        |mut c: Caller<'_, State>, b: i64, n: i32| c.data_mut().files.chunk(b, n).map_err(protocol),
    )?;
    linker.func_wrap(
        "cerune",
        "file_open_v1",
        |mut c: Caller<'_, State>, m: i32| c.data_mut().files.open(m).map_err(protocol),
    )?;
    linker.func_wrap(
        "cerune",
        "file_read_v1",
        |mut c: Caller<'_, State>, h: i32, n: i32| c.data_mut().files.read(h, n).map_err(protocol),
    )?;
    linker.func_wrap(
        "cerune",
        "file_write_v1",
        |mut c: Caller<'_, State>, h: i32, b: i64, n: i32| {
            c.data_mut().files.write(h, b, n).map_err(protocol)
        },
    )?;
    linker.func_wrap(
        "cerune",
        "file_close_v1",
        |mut c: Caller<'_, State>, h: i32| c.data_mut().files.close(h).map_err(protocol),
    )?;
    linker.func_wrap(
        "cerune",
        "write_stdout_v1",
        |mut c: Caller<'_, State>, bits: i64, n: i32| -> Result<i32, Error> {
            let n = files::count_bytes(n).map_err(protocol)?;
            Ok(files::response(
                c.data_mut().output.write(&bits.to_le_bytes()[..n]),
                n,
            ))
        },
    )?;
    linker.func_wrap("cerune", "flush_stdout_v1", |mut c: Caller<'_, State>| {
        if c.data_mut().output.flush().is_ok() {
            0i32
        } else {
            -1
        }
    })?;
    linker.func_wrap("cerune", "format_f32_v1", |v: wasmi::F32, o: i32| {
        chunk(float_output::f32(v.into()), o)
    })?;
    linker.func_wrap("cerune", "format_f64_v1", |v: wasmi::F64, o: i32| {
        chunk(float_output::f64(v.into()), o)
    })?;
    linker.func_wrap(
        "cerune",
        "write_error_byte",
        |mut c: Caller<'_, State>, byte: i32| -> Result<(), Error> {
            if !(0..=127).contains(&byte) {
                return Err(Error::new("invalid diagnostic byte"));
            }
            c.data_mut().diagnostics = true;
            let _ = c.data_mut().error.write_all(&[byte as u8]);
            Ok(())
        },
    )?;
    linker.func_wrap("cerune", "stdin_status_v1", |c: Caller<'_, State>| match c
        .data()
        .input
        .connection_error()
    {
        None => 0i32,
        Some(input::Code::UnsupportedConnection) => 2,
        Some(_) => 3,
    })?;
    linker.func_wrap(
        "cerune",
        "read_stdin_byte_v1",
        |mut c: Caller<'_, State>| -> Result<(i32, i32), Error> {
            if c.data().input_failed {
                return Err(Error::new("stdin read after failure"));
            }
            let mut b = [0];
            Ok(match c.data_mut().input.read(&mut b) {
                Ok(0) => (0, 1),
                Ok(1) => (i32::from(b[0]), 0),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => (0, 2),
                _ => {
                    c.data_mut().input_failed = true;
                    (0, 3)
                }
            })
        },
    )?;
    linker.func_wrap("cerune", "arguments_count_v1", |c: Caller<'_, State>| {
        (c.data().arguments.len() as i64, 0i32)
    })?;
    linker.func_wrap(
        "cerune",
        "argument_length_v1",
        |c: Caller<'_, State>, i: i64| {
            usize::try_from(i)
                .ok()
                .and_then(|i| c.data().arguments.get(i))
                .map_or((0i64, 2i32), |a| (a.len() as i64, 0))
        },
    )?;
    linker.func_wrap(
        "cerune",
        "argument_byte_v1",
        |c: Caller<'_, State>, i: i64, o: i64| {
            usize::try_from(i)
                .ok()
                .and_then(|i| c.data().arguments.get(i))
                .and_then(|a| usize::try_from(o).ok().and_then(|o| a.get(o)))
                .map_or((0i32, 2i32), |b| (i32::from(*b), 0))
        },
    )?;
    Ok(())
}
fn output_files() -> io::Result<(fs::File, fs::File)> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        Ok((
            io::stdout().as_fd().try_clone_to_owned()?.into(),
            io::stderr().as_fd().try_clone_to_owned()?.into(),
        ))
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsHandle;
        Ok((
            io::stdout().as_handle().try_clone_to_owned()?.into(),
            io::stderr().as_handle().try_clone_to_owned()?.into(),
        ))
    }
}
fn run() -> Result<(), String> {
    let options = options(std::env::args_os().skip(1))?;
    let bytes = fs::read(options.source).map_err(|_| "cannot read Wasm module")?;
    let engine = Engine::default();
    let module = Module::new(&engine, bytes).map_err(|_| "invalid Wasm module")?;
    if module.exports().map(|e| e.name()).collect::<Vec<_>>() != ["main"] {
        return Err(
            "expected only the generated main export; public memory is not supported".into(),
        );
    }
    let trace = trace::Capture::open(&module, options.trace)?;
    let needs_process = module
        .imports()
        .any(|i| i.module() == "cerune" && i.name().starts_with("process_"));
    if needs_process && options.process_target.is_none() {
        return Err("subprocess imports require explicit --process-target".into());
    }
    #[cfg(all(
        target_arch = "x86_64",
        any(
            all(windows, target_env = "msvc"),
            all(target_os = "linux", target_env = "gnu")
        )
    ))]
    let processes = options
        .process_target
        .map(|target| {
            let config = options::configuration(options.process_config.as_deref())?;
            process::Process::new(target, &config)
                .map_err(|status| format!("process host configuration failed: status={status}"))
        })
        .transpose()?;
    #[cfg(not(all(
        target_arch = "x86_64",
        any(
            all(windows, target_env = "msvc"),
            all(target_os = "linux", target_env = "gnu")
        )
    )))]
    if needs_process || options.process_target.is_some() || options.process_config.is_some() {
        return Err("subprocess host requires x86-64 Windows MSVC or Linux GNU".into());
    }
    // Select the backend before execution. No root is inferred from cwd or environment.
    let directory = options
        .root
        .as_deref()
        .map(Directory::open)
        .transpose()
        .map_err(|e| e.record())?;
    let (output, error) = output_files().map_err(|_| "host stdout/stderr connection failed")?;
    let state = State {
        trace,
        #[cfg(all(
            target_arch = "x86_64",
            any(
                all(windows, target_env = "msvc"),
                all(target_os = "linux", target_env = "gnu")
            )
        ))]
        process: processes,
        files: Files::new(directory),
        input: input::stdin().into_inner().expect("OS reader"),
        input_failed: false,
        output,
        error,
        diagnostics: false,
        arguments: options.arguments,
    };
    let mut store = Store::new(&engine, state);
    let mut linker = Linker::new(&engine);
    #[cfg(all(
        target_arch = "x86_64",
        any(
            all(windows, target_env = "msvc"),
            all(target_os = "linux", target_env = "gnu")
        )
    ))]
    if store.data().process.is_some() {
        process::imports(&mut linker).map_err(|_| "cannot define subprocess imports")?;
    }
    imports(&mut linker).map_err(|_| "cannot define host imports")?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| format!("Wasm instantiation failed: {e}"))?;
    let main = instance
        .get_typed_func::<(), ()>(&store, "main")
        .map_err(|_| "invalid main signature")?;
    let result = main.call(&mut store, ());
    store.data_mut().files.shutdown();
    if let Some(trace) = &store.data().trace
        && let Err(error) = trace.save()
    {
        let _ = writeln!(io::stderr().lock(), "cerune-wat-host: {error}");
    }
    if let Err(error) = result {
        return Err(
            if store.data().diagnostics
                && error.as_trap_code() == Some(TrapCode::UnreachableCodeReached)
            {
                String::new()
            } else {
                format!("Wasm execution failed: {error}")
            },
        );
    }
    if store.data().diagnostics {
        return Err("diagnostic without a stop".into());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        if !error.is_empty() {
            eprintln!("cerune-wat-host: {error}");
        }
        std::process::exit(1);
    }
}

#[cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
impl AsRef<process::Process> for State {
    fn as_ref(&self) -> &process::Process {
        self.process
            .as_ref()
            .expect("imports require explicit process target")
    }
}
#[cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
impl AsMut<process::Process> for State {
    fn as_mut(&mut self) -> &mut process::Process {
        self.process
            .as_mut()
            .expect("imports require explicit process target")
    }
}
