# Reference host for real OS connections from WAT

[日本語](README.md)

This independent reference implementation connects Cerune-generated WAT/Wasm to files, standard I/O, arguments, and subprocesses on Windows/Linux. WAT remains emit-only; this adds no Cerune CLI run/build/release route.

## Run

Run from the repository root. Use a new data directory for each comparison.

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files.wat
mkdir target/wat-files-data
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/files.wat --file-root target/wat-files-data --
~~~

Expected output is true and 16. sample.txt retains 16 bytes including Japanese, NUL, CR/LF, and combining characters, then reads them back unchanged. Repeating the run stops with already-exists without overwriting. A precompiled .wasm is also accepted in the module position.

| Argument | Meaning |
| --- | --- |
| First argument | .wat or .wasm module |
| `--trace <new-file>` | Save calls, branches, returns and stops after running a module built with `emit-wat --with-trace`. [Factorial example](../../docs/reference/cli.en.md) |
| `--trace-event-limit <count>` / `--trace-byte-limit <bytes>` | Limit recorded events and file size. Defaults to 4096 events and 1 MiB |
| --file-root directory | Working directory opened before execution; omitted means unconfigured |
| --process-target triple | Required for subprocess imports: explicit x86-64 Windows MSVC or Linux GNU |
| --process-config json | Existing cerune-process-v1 connection JSON; requires target. Omitted means Unconfigured |
| -- | Required end of host options, even with no program arguments |
| After -- | Program arguments for std:env; marker-like strings remain ordinary arguments |

Duplicate/unknown settings and missing values/delimiters fail startup. No root is inferred from cwd or environment variables. Explicit relative paths resolve from startup cwd. This host CLI differs from the Native artifact's --cerune-runtime-v1 syntax. Settings, root connection, and module loading are checked before execution; generated WAT checks program argument limits.

## Subprocesses

Run the unconfigured example on Linux x86-64 as follows. On Windows, replace the target with x86_64-pc-windows-msvc. No target or connection is inferred from the running OS.

~~~sh
cargo run --quiet -- emit-wat examples/process_execution/unconfigured.ceru -o target/process.wat
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/process.wat --process-target x86_64-unknown-linux-gnu --
~~~

Generate [registered.ceru](../../examples/process_execution/README.en.md) similarly and add --process-config process.json to launch registered children. JSON follows the [existing contract](../../docs/design/subprocess-cli.en.md), including an explicit absolute Linux supervisor path. Empty/invalid/over-1-MiB JSON and mismatched targets fail before execution; diagnostics omit bodies and environment values.

Each Wasmi Store owns one process connection. Releasing a request/result never clears a disabled connection. Normal completion, traps, and instantiation failures hand ownership back to the existing runtime destructor once. After timeout, unknown transfer progress or missing cancellation replies may disable further launches even when cleanup completed. There is no success fallback or implicit reconnection.

Supported hosts are x86-64 Windows MSVC / Linux GNU. Other host builds explicitly reject subprocess settings. Wasm memory stays private. Official runtime/Linux-supervisor distribution is separate work.

## Components and scope

| Component | Responsibility |
| --- | --- |
| Wasmi 2.0.0 | Wasm validation and execution |
| src/main.rs | Versioned imports, argument snapshot, standard I/O, and one main invocation |
| src/process.rs | Versioned subprocess transport and request/result ownership; execution, failures and cleanup use the shared runtime |
| src/files.rs | Transfers of at most eight bytes and instance-local handles |
| Cerune runtime::files::Directory | OS scope, regular-file checks, exclusive creation, and close |
| Generated WAT | Common-IR checks, ownership, retries, UTF-8, accounting, and file-v1 diagnostics |

[Wasmi](https://docs.rs/wasmi/2.0.0/wasmi/) is pinned in this crate's Cargo.lock and is not a dependency of Cerune itself. The relative cerune-lang dependency stays inside the repository. Float formatting shares the executors' pure functions. File transport avoids complete-body copies; subprocess transport owns complete requests before prepare. Neither accesses internal memory. Only the main export is accepted.

Windows uses the existing directory-handle-relative connection; Linux uses the existing openat/procfs connection. The opened directory survives root renames, and symlinks/reparse points and non-regular targets are rejected. See the [common contract](../../docs/design/file-io.en.md) for OS requirements.

Stdin uses the existing raw-byte connection. Direct Windows console input is unsupported; use pipes/files. Output writes directly through duplicated OS handles and returns accepted counts/failures. Input content is not normalized.

## Stopping and observation

Generated WAT delivers semantic file failures as file-v1 with NodeId/SourceId/Span and stops with unreachable. Host/engine failures stop separately; they are not converted to EOF, success, or depth-limit. The host does not reenter, rerun main, or resume after stopping.

Close consumes its handle even on failure and is not retried. Remaining handles after an engine failure are reclaimed once at shutdown. Partial files and prior output remain; this reclamation does not recreate source close diagnostics.

Generated WAT and existing source-located diagnostics remain observable. Response injection belongs to test execution control. This host adds no observation API, public memory, or stable embedding API. It is not a sandbox for arbitrary untrusted Wasm or a general WASI runner. Calling WAT library exports remains a separate experiment.

## Validation

~~~sh
cargo fmt --manifest-path experiments/wat_host/Cargo.toml --check
cargo clippy --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host --all-targets -- -D warnings
cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host
~~~

Windows/Linux CI requires these checks. Tests compare output, complete file-v1 diagnostics, and remaining files against four executors, covering empty/exact/exceeded limits, invalid UTF-8, usage limits, evaluation order, short circuits, recursion, and copies. They also cover stdin/arguments/scalar/aggregate output, root movement during execution, and link rejection.

Short transfers, interruption, invalid responses, close consumption, and all 256 byte values use explicit test connections to the transport adapter. This does not claim those failures occurred naturally on the OS. Generated WAT response handling is also compared by the existing [in-memory tests](../../tests/file_io/wat.rs) under the [host contract](../../docs/design/wat-file-host.en.md).

The [subprocess ABI probe](../subprocess/wat_abi/README.en.md), [generated-WAT tests](tests/process_generated.rs), and [CLI comparisons](tests/process_cli.rs) cover existing examples, standard I/O/files/arguments together, timeouts, continuation and disabled connections. Linux cleanup failure uses a supervisor fixture that delays notifications. Report injection on both OSes checks enum conversion, not a naturally occurring OS cleanup failure.

On Linux, include the normally ignored child/supervisor tests with the following explicit paths. Windows/Linux CI requires these comparisons.

~~~sh
cargo build --locked --example subprocess_linux_supervisor --example subprocess_linux_backend
CERUNE_TEST_PROCESS_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_supervisor" CERUNE_TEST_PROCESS_FAULT_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_backend" cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- --include-ignored
~~~
