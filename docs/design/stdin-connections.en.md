# Actual stdin connections: CLI and generated C/LLVM

[日本語](stdin-connections.ja.md)

Connect the [shared std:io line-input contract](line-input.en.md) to actual stdin in CLI HIR/MIR/SSA/VM execution and generated C/LLVM. Input lowering for QBE/WAT/Native follows separately.

## Initial scope

| Connection | Behavior |
| --- | --- |
| Synchronous Windows/Linux pipes and files | Read raw UTF-8 bytes; preserve NUL, CR/LF, and Ctrl-Z |
| Linux terminal | Read bytes delivered by the OS; do not change terminal settings, echo, line editing, or encoding |
| Windows console | Stop before reading with unsupported-connection; UTF-16 conversion and source/converted consumption accounting remain future work |
| Other OSes | Actual stdin reports unsupported-connection; explicitly supplied Rust readers remain separately available |

The contract covers synchronous connections, excluding asynchronous handles and external stdin replacement. Existing capture APIs never implicitly attach stdin. CLI run/run-ir/run-mir (including --ssa)/run-vm select execution connections. emit and observe still do not execute programs.

## Reads and failures

Linux uses `read(0, ..., 1)`; Windows uses `ReadFile(..., 1, ...)` on a borrowed standard handle. No stdio read-ahead or close is introduced. Generated C selects the target using its C compiler's `_WIN32` / `__linux__`, never the OS running Cerune. LLVM requires an explicit `--target x86_64-unknown-linux-gnu` or `--target x86_64-pc-windows-msvc`; omission is diagnosed. The Rust connection follows the CLI binary's target OS.

Windows console/handle classification is obtained once. Broken pipes mean EOF; message-pipe ERROR_MORE_DATA preserves the returned byte. Successful zero-byte pipe responses are empty messages, so continue reading rather than inventing EOF. Files confirm EOF with zero or ERROR_HANDLE_EOF. Linux retries only EINTR. OS-call counts need not equal shared-connection request counts.

See [POSIX read](https://pubs.opengroup.org/onlinepubs/9799919799/functions/read.html), [ReadFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile), and [ReadConsole](https://learn.microsoft.com/en-us/windows/console/readconsole). The shared buffer provided by [Rust Stdin](https://doc.rust-lang.org/std/io/struct.Stdin.html) is not used here.

Check limits, connection, known EOF, budget/allocation, reads, then UTF-8 in that order. unsupported-connection consumes zero and retains NodeId/Span. Input origins point into virtual std:io; callers remain traceable in IR. Reads never implicitly flush. Write interactive code as print → flush → read_line.

Generated C/LLVM allocates the limit-sized storage and ownership metadata together. Failure/empty EOF frees it; success transfers the same allocation into the existing owner list and charges actual length. io-v1 diagnostics exclude input and OS-error text and stop subsequent execution. A shared API observing successful generated operations is not added yet.

LLVM lowers the common IR ReadLine to a dedicated instruction, emitting `cerune.input.read_line`, the OS connection, UTF-8 validation, budget/ownership, and diagnostics as visible LLVM. Input sites pass immutable NodeId/Span data. `--annotate-origins` only adds comments; plain artifacts retain identical instructions and diagnostics.

## Runnable fixture

The [source](../../tests/fixtures/line_input.ceru), [input](../../tests/fixtures/line_input.stdin), and [expected output](../../tests/fixtures/line_input.stdout) cover blank lines, Japanese, NUL, CRLF, combining characters, and an unterminated final line. Keep these separate from ordinary examples until the remaining generated routes support input.

Run `cargo build` first. From Windows PowerShell, use cmd file redirection to avoid PowerShell text-pipeline encoding conversions.

```powershell
cmd /c "target\debug\cerune.exe run tests\fixtures\line_input.ceru < tests\fixtures\line_input.stdin"
```

On Linux:

```sh
target/debug/cerune run tests/fixtures/line_input.ceru < tests/fixtures/line_input.stdin
cargo run -- emit-c tests/fixtures/line_input.ceru -o target/line-input.c
clang -std=c11 -O2 target/line-input.c -o target/line-input
target/line-input < tests/fixtures/line_input.stdin
cargo run -- emit-llvm tests/fixtures/line_input.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/line-input.ll
clang --target=x86_64-unknown-linux-gnu -O2 target/line-input.ll -o target/line-input-llvm
target/line-input-llvm < tests/fixtures/line_input.stdin
```

[Route tests](../../tests/stdin_routes.rs) compare CLI routes with C/LLVM at -O0/-O2. Interactive tests send input only after receiving the prompt and verify EOF by closing the pipe. Shared file positions verify absence of read-ahead; controlled execution connections inject errors, invalid responses, interruptions, and allocation failure. LLVM OS-response tests also cover EINTR, empty messages, ERROR_MORE_DATA, broken pipes, file EOF, and console rejection. Tests cover console-rejection reasons and preflight order, not visible-console interaction or completed UTF-16 conversion support.
