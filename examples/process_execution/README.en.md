# Receive subprocess output and failures

[日本語](README.md)

`std:process` provides types; `std:process/run` provides execution. Failures return in `Outcome` so the caller can continue.

| Example | Behavior | Connection |
| --- | --- | --- |
| [unconfigured.ceru](unconfigured.ceru) | Receive missing configuration and reach `continued` | No configuration needed. [Expected output](unconfigured.stdout) |
| [registered.ceru](registered.ceru) | Pass arguments/raw bytes; display failure, exit, stdout/stderr, and accepted input count | Register `tool` in [explicit JSON configuration](../../docs/design/subprocess-cli.en.md). [Verification output](registered.stdout) |

## Unconfigured example

```sh
cargo run --quiet -- run examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru --ssa
cargo run --quiet -- run-vm examples/process_execution/unconfigured.ceru
```

## Launch an actual child

```sh
cerune run examples/process_execution/registered.ceru --process-config process.json
cerune run-mir examples/process_execution/registered.ceru --ssa --process-config process.json
cerune run-vm examples/process_execution/registered.ceru --process-config process.json
```

Without configuration, `registered.ceru` also displays `Unconfigured` and continues. `registered.stdout` assumes the verification child is connected.

[CLI tests](../../tests/process_cli/mod.rs) build the [verification child](../../tests/support/process_child.rs), explicitly configure its absolute path, working directory, and environment, and run this example. The child verifies empty/Japanese/quoted arguments, echoes `[255, 0, 13, 10]` to stdout, writes `err\0\r\n` to stderr, and exits with code 7. Cerune reaches `continued` and exits successfully. The child fixture remains separate from the execution-example list.

On Windows, compare four executors and the `run-ir` alias with:

```sh
cargo test --test modules process_cli
```

On Linux, explicitly supply the supervisor (with Cargo's default output directory):

```sh
cargo build --example subprocess_linux_supervisor
CERUNE_TEST_PROCESS_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_supervisor" cargo test --test modules process_cli -- --include-ignored
```

Generated C also runs on Windows/Linux. Specify a target and explicitly link its matching runtime archive.

~~~sh
cerune emit-c examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.c
~~~

See [build/startup instructions](../../docs/design/subprocess-c.en.md) and the [four-executor comparison](../../experiments/subprocess/c_abi/generated.cjs). [LLVM generation and execution](../../docs/design/subprocess-llvm.en.md) are also supported. [QBE generation and execution](../../docs/design/subprocess-qbe.en.md) are also supported. [Native generation and execution](../../docs/design/subprocess-native.en.md) are also supported. WAT follows later. See the [execution design](../../docs/design/subprocess-execution.en.md) for support and preserved origins.

LLVM generation uses the same example and process.json.

~~~sh
cerune emit-llvm examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.ll
~~~

QBE uses the same example. Match the explicit target with the QBE target, then link the assembly with the matching runtime archive.

~~~sh
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.ssa
qbe -t amd64_sysv -o process.s process.ssa
~~~

## Inspect the representation before Native generation

The same examples expose request/result storage, type-field positions, and original source locations in LIR.

~~~sh
cerune emit-lir examples/process_execution/unconfigured.ceru --target x86_64-pc-windows-msvc
cerune emit-lir examples/process_execution/registered.ceru --ssa --target x86_64-unknown-linux-gnu
~~~

LIR observation does not start a child. [Native ASM/direct Objects](../../docs/design/subprocess-native.en.md) execute these examples after explicitly linking the common runtime. Additional-runtime inputs for Native build/release follow.
