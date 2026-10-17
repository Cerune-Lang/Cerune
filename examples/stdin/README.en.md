# Standard input

[日本語](README.md) / [Examples](../README.en.md)

[lines.ceru](lines.ceru) flushes a prompt, reads a UTF-8 line, and displays its contents and byte count. The 64-byte limit includes LF/CRLF. Blank lines are lines; EOF ends the loop. An unterminated final line is displayed once.

| Input | Displayed contents / byte count |
| --- | --- |
| Cerune + LF | `["Cerune\n"]` / 7 |
| LF only | `["\n"]` / 1 |
| 日本語 + LF | `["日本語\n"]` / 10 |
| EOF | end |

Windows/Linux files and pipes are supported. Direct Windows consoles stop with unsupported-connection. Linux terminal settings are unchanged. This example waits for input and is excluded from the top-level batch examples.

## Run

Prepare a UTF-8 input file and run from the repository root.

~~~sh
cargo build
target/debug/cerune run examples/stdin/lines.ceru < input.txt
target/debug/cerune run-vm examples/stdin/lines.ceru < input.txt
~~~

In PowerShell use `cmd /c "target\debug\cerune.exe run examples/stdin/lines.ceru < input.txt"` to avoid text-pipeline encoding conversion.

## Inspect generation and run Native

~~~sh
cargo run -- emit-ir examples/stdin/lines.ceru -o target/lines.ceir
cargo run -- emit-lir examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu -o target/lines.lir
cargo run -- emit-asm examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/lines.s
cargo run -- emit-obj examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/lines.o
cc target/lines.o -o target/lines
target/lines < input.txt
~~~

For Windows use target `x86_64-pc-windows-msvc`, object path `target/lines.obj`, and link with `clang target/lines.obj -o target/lines.exe`. Run using the same cmd input redirection.

See the [connection/diagnostic contract](../../docs/design/stdin-connections.en.md) and [WAT host instructions](../../docs/design/wat-input-host.en.md). Tests compare identical inputs across executors, C, LLVM, QBE, WAT, and Native. Limit overflow and invalid UTF-8 stop without including input contents in diagnostics.

[retained_lines.ceru](retained_lines.ceru) reads two lines before displaying them. Input `x + LF`, then `y + LF`, displays `["x\n"]`, then `["y\n"]`, retaining the first line during the second read. Each limit is 64 bytes; EOF displays `end`. Substitute this filename in the commands above.
