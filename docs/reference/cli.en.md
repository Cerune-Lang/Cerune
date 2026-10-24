# Cerune CLI reference

[日本語](cli.ja.md)

This document defines the command-line interface of Cerune v0.1.

IR/VM/Native [build / release](../design/owned-routes.en.md) and [Lean output](../design/lean-verification.en.md) are at the design/experiment stage and are not current CLI commands.

`emit-lir` displays Native LIR with an explicit target and optional `--ssa`. See [LIR observation and validation](../design/lir-observation.en.md). `run-lir` is deferred; independent execution covers HIR, MIR, SSA, and Bytecode.

## Save observations together

`observe` [saves sources, HIR, MIR, annotated ASM, and a manifest from one compilation](../design/observation-bundle.en.md). Both `--target` and `-o` are required; both heap budgets are supported. Existing output paths are rejected; create the parent directory first. Input names and source text are included. No program execution, external tools, or linking occurs. Objects and other backend artifacts are not yet bundled.

## Commands

The current CLI provides the following commands:

```text
cerune fmt <file> [--let preserve|show|hide] [--check | --write | -o <file>]
cerune check <file>
cerune observe <file> [--ssa] [--lir] --target <triple> -o <new-directory>
cerune emit-sources <file> [-o <sources.json>]
cerune emit-ir <file> [-o <output.ceir>]
cerune emit-mir <file> [-o <output.txt>]
cerune emit-lir <file> [--ssa] --target <triple> [-o <output.lir.txt>]
cerune emit-c <file> [-o <output.c>]
cerune emit-llvm <file> [--target <triple>] [-o <output.ll>]
cerune emit-wat <file> [-o <output.wat>]
cerune emit-qbe <file> [--target <triple>] [-o <output.ssa>]
cerune emit-asm <file> [--target <triple>] [--annotate-origins] [-o <output.s>]
cerune emit-obj <file> --target <triple> [--annotate-origins] -o <output.o>
cerune emit-bytecode <file> [-o <output.cebc>]
cerune run <file> [--diagnostic-format runtime-v1]
cerune run-ir <file> [--diagnostic-format runtime-v1]
cerune run-mir <file> [--diagnostic-format runtime-v1]
cerune run-vm <file> [--diagnostic-format runtime-v1]
cerune --version
```

## Source formatting

`fmt` validates and formats syntax in one explicitly selected file. It does not load imports, type-check or execute code. By default it writes to stdout without changing files.

| Selection | Behavior |
| --- | --- |
| `--let preserve` (default) | Keep original prefix choices |
| `--let show` / `hide` | Add/remove binding prefixes; preserve identifiers named let |
| `--check` | Exit 0 when formatted, 1 when different; no stdout or file changes |
| `--write` / `-o <file>` | Replace input/save to the selected path; mutually exclusive with each other and check |

Use four-space indentation and LF endings while retaining comments and literals. Keep the original presence/absence of `infer` on initialized bindings; `--let` does not change it. Input is limited to 8 MiB; syntax, UTF-8 and save errors also exit 1. Saving uses a temporary file and refuses replacement of links, directories and read-only files. Configuration files, import sorting and recursive formatting are unsupported.

See the [example](../../examples/formatting/README.en.md) and [design](../design/formatting.en.md) for semantic and origin checks.


## Standard output during execution

`run`, `run-ir`, `run-mir` (including `--ssa`), and `run-vm` deliver output during execution. Destination buffering still applies; this does not guarantee delivery of every byte before program termination. Unless the connection failed, the CLI flushes at both normal completion and language failure, without printing prior output again.

Write/flush failures produce diagnostics and exit code 1. If both language execution and the final flush fail, both are reported. Source `flush();` flushes at that point and stops later execution on failure. Read stdin through std:io read_line. Synchronous Windows/Linux pipes and files are supported; Windows console stops with unsupported-connection. See the [input contract and runnable fixture](../design/stdin-connections.en.md). See the [output connection contract](../design/standard-io.en.md) for APIs and coverage.

`--diagnostic-format runtime-v1` retains `runtime-v1` for language failures and uses `io-v1` for write/flush failures. I/O records contain the operation, requested/accepted counts, and NodeId/Span; final host flushing uses `origin=host`. Zero flush counts do not measure bytes delivered to the OS.

## Dynamic string budget

`observe`, `run`, `run-ir`, `run-mir`, `run-vm`, `emit-ir`, `emit-mir`, `emit-bytecode`, `emit-c`, `emit-llvm`, `emit-qbe`, `emit-wat`, `emit-asm`, and `emit-obj` accept `--string-heap-limit <bytes>`. The default is 67108864 (64 MiB). Values are decimal integers from 0 through 9223372036854775807; missing, duplicate, and negative values are rejected. `fmt`, `check` and `emit-sources` do not accept this option.

The budget counts live dynamic string payload, shared allocations once, and static strings never. Old and new values both count while reassignment keeps them alive. Compile-time evaluation has an independent 64 MiB budget unaffected by this setting. This is not a physical-memory cap. Generated artifacts fix the value; no API changes it during execution.

```sh
cerune run examples/string_concat.ceru --string-heap-limit 1024
cerune emit-llvm examples/string_concat.ceru --target x86_64-unknown-linux-gnu --string-heap-limit 1024 -o concat.ll
```

## Dynamic array budget

The same commands also accept `--array-heap-limit <bytes>`, with the same default and numeric syntax as the string option. The budgets are independent. Array accounting measures live element storage using logical widths, not total physical memory. `fmt`, `check` and `emit-sources` reject this option.

Dynamic arrays support `run` / `run-ir` / `run-mir` / `run-vm` and IR/MIR/bytecode/C/LLVM/QBE/WAT/ASM/native-object output. Each artifact records the array budget in use. LLVM and QBE require `--target`, including unused array types. Select a target for ASM and objects, then link externally. `emit-asm` retains its fixed Windows default.

```sh
cerune run examples/dynamic_arrays/copy.ceru --array-heap-limit 1024
cerune run-vm examples/dynamic_arrays/copy.ceru --array-heap-limit 1024
```

## Validation

Except for `fmt`, `<file>` is the entry file for each command. [Module imports](../design/modules.en.md) resolve from the declaring file's directory, and all dependencies are checked. Changing the CLI working directory does not change resolution. Existing backend `--target` requirements remain in effect.

### Observing dependency sources

```sh
cerune emit-sources examples/modules/main.ceru -o sources.json
```

This explicitly outputs names and source contents. JSON schema `cerune-sources-v1` contains a registration-ordered `files` array with `id`, `name`, and `text`. JSON escaping preserves the exact UTF-8 text without normalization. Inputs without import/pub retain anonymous source ID 0; module inputs start with entry ID 1. Resolve runtime `file` and file-local `bytes` against these contents. This does not embed source text in generated programs. Without `-o`, the JSON goes to stdout.

Default `run`, `run-ir`, and `run-vm` diagnostics show the dependency filename, line, and column. `--diagnostic-format runtime-v1` emits numeric file IDs. Compilation failures do not replace output artifacts.

### Syntax and type validation

```text
cerune check <file>
```

`cerune check` parses the input source file and performs semantic validation and type checking.

A successful `check` does not guarantee that every output route supports the program. Strings work through every route. Omitting the LLVM or QBE target diagnoses string-containing type definitions or expressions at their source location without producing an artifact. This diagnostic also leaves an existing file specified by `-o` unchanged.

## Direct Cerune IR execution

```sh
cerune run examples/ir_execution.ceru
cerune run-ir examples/modules/failure.ceru --diagnostic-format runtime-v1
```

`run` builds common IR from source (`.ceru`) and directly executes its statements and expressions. `run-ir` is an alias with identical behavior and options. It supports current language features without lowering to Bytecode or delegating to the VM. It does not parse textual `.ceir` files or accept `--target` or `-o`.

Successful output goes to stdout. Failures preserve prior output, write a diagnostic to stderr, and exit with code 1. Default diagnostics report the reason and source location; `runtime-v1` uses the same FailureCode, NodeId, and Span as other routes. There is no instruction index.

Update scripts and performance measurements that used the previous VM behavior of `run` to use `run-vm`. Language output and failure records are shared, but only the VM route includes bytecode instruction indices in default diagnostics.

Rust callers use `run_ir(source)` or `ir_executor::run(&program)` for completed IR. `IrRunError` distinguishes construction and execution failures; execution errors expose `output()`, `origin()`, and `runtime_failure()`. For imports, use `modules::load(path)?.to_ir()`. See [responsibilities and limits](../design/ir-executor.en.md).

## Cerune IR emission

```text
cerune emit-ir <file> [-o <output.ceir>]
```

`cerune emit-ir` emits the backend-independent Cerune IR after semantic and type resolution.

## Independent MIR execution

```sh
cerune run-mir examples/ir_stages/owned_values.ceru
cerune run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 47 --diagnostic-format runtime-v1
```

`run-mir` builds HIR→MIR from source, validates it, and directly executes MIR instructions/blocks. It supports current language features without internally executing HIR or VM. The default meaning of `run` / `run-ir` is unchanged.

It accepts `--string-heap-limit`, `--array-heap-limit`, and `--diagnostic-format runtime-v1`. Success exits 0; failures exit 1, preserving original locations and prior output for language failures. Validation/internal errors remain distinct from language stops. `-o`, `--target`, and SSA/pass flags are rejected. This is not a MIR text loader or a new build artifact. See [API and validation limits](../design/mir-executor.en.md).

## Observing MIR

```sh
cerune emit-mir examples/ir_stages/control_flow.ceru -o target/control_flow.mir.txt
```

Lower completed IR into unoptimized, non-SSA MIR, validate it, then emit observation text. Without `-o`, output goes to stdout. No target is needed; `--target`, `--ssa`, and pass flags are rejected. Both heap budgets are recorded.

`Cerune MIR v0.1` contains typed locals, explicit evaluation order, basic blocks/edges, ownership operations, and HIR NodeId/SourceId/Span. Full source contents and paths are not included automatically. String literals and identifiers are displayed as program contents. There is no dedicated extension or loader; this is not an executable distribution artifact. `run` still executes HIR directly. See [lowering and validation limits](../design/ir-stages.en.md).

## Output artifact emission

```text
cerune emit-c <file> [-o <output.c>]
cerune emit-llvm <file> [--target <triple>] [-o <output.ll>]
cerune emit-qbe <file> [--target <triple>] [-o <output.ssa>]
cerune emit-wat <file> [-o <output.wat>]
cerune emit-asm <file> [--target <triple>] [--annotate-origins] [-o <output.s>]
cerune emit-obj <file> --target <triple> [--annotate-origins] -o <output.o>
cerune emit-bytecode <file> [-o <output.cebc>]
```

Each command emits the following artifact:

| Command | Output route | Current target | Artifact |
| --- | --- | --- | --- |
| `emit-c` | C | not selected by Cerune | `.c` |
| `emit-llvm` | LLVM IR | unspecified, or explicit Windows x64 / Linux x86-64 | `.ll` |
| `emit-qbe` | QBE IR | unspecified, or explicit Windows x64 / Linux x86-64 | `.ssa` |
| `emit-wat` | WebAssembly Text | WebAssembly | `.wat` |
| `emit-asm` | native assembly | x86-64, Windows / Linux, respective calling conventions | `.s` |
| `emit-obj` | Native object encoded by Cerune | explicit Windows x64 / Linux x86-64 | `.obj` / `.o` |
| `emit-bytecode` | Cerune bytecode | Cerune VM | `.cebc` |

Text-producing `emit-*` commands write their observations to standard output by default. With `-o`, the caller chooses the output path. Binary `emit-obj` requires `-o`.

`emit-asm --target x86_64-unknown-linux-gnu` selects Linux; `--target x86_64-pc-windows-msvc` selects Windows. Omission preserves the fixed Windows default without inferring the host OS. `--annotate-origins` adds source comments and labels. See [native code observation](../design/native-code.en.md).

### LLVM target selection

`--target` accepts `x86_64-unknown-linux-gnu` or `x86_64-pc-windows-msvc`. Programs using strings or dynamic arrays require it, including unused types and functions. Numeric-only programs also require it when checked operations, conversions, or array indexing emit runtime diagnostics. Even numeric-only `print(1);` requires a target to select the write-failure connection. `flush();` and std:io line input also require an explicit target. The host OS never supplies a default. `--target` and `-o` (also `--output`) may appear in either order; duplicate options, missing values, and unsupported targets are errors.

On Linux x86-64:

```sh
cerune emit-llvm examples/string_lookup.ceru --target x86_64-unknown-linux-gnu -o target/string_lookup.ll
clang --target=x86_64-unknown-linux-gnu target/string_lookup.ll -o target/string_lookup
./target/string_lookup
```

On Windows x64 with the MSVC CRT and linker available:

```powershell
cerune emit-llvm examples/string_lookup.ceru --target x86_64-pc-windows-msvc -o target/string_lookup.ll
clang --target=x86_64-pc-windows-msvc target/string_lookup.ll -o target/string_lookup.exe
.\target\string_lookup.exe
```

Cerune only generates LLVM; it does not launch Clang or the executable. Selection is recorded in `target triple`. Pass the same target to downstream tools. Windows programs also initialize numeric-only standard output in binary mode to preserve NUL, CR, and LF. See [string design](../design/strings.en.md#llvm-representation-and-targets).

Library callers can use `compile_to_llvm_with_target(source, Some(codegen::llvm::Target::X86_64UnknownLinuxGnu))`, or `X86_64PcWindowsMsvc`. Existing `compile_to_llvm(source)` remains the unspecified-target API.

### QBE target selection

QBE accepts `--target x86_64-unknown-linux-gnu` or `x86_64-pc-windows-msvc`. Strings, dynamic arrays, stdout operations (including `print(1);` and `flush();`), and std:io line input require a target. Missing or unsupported targets and duplicate options produce diagnostics without changing existing output files. Programs using none of these may omit the target. Never infer it from the OS running Cerune.

```sh
cerune emit-qbe examples/string_lookup.ceru --target x86_64-unknown-linux-gnu -o target/string_lookup.ssa
qbe -t amd64_sysv -o target/string_lookup.s target/string_lookup.ssa
cc target/string_lookup.s -o target/string_lookup
./target/string_lookup
```

On Windows x64, install QBE 1.3, Clang, and the MSVC CRT and linker.

```powershell
cerune emit-qbe examples/function_arguments.ceru --target x86_64-pc-windows-msvc -o target/function_arguments.ssa
qbe -t amd64_win -o target/function_arguments.s target/function_arguments.ssa
clang --target=x86_64-pc-windows-msvc target/function_arguments.s -o target/function_arguments.exe
./target/function_arguments.exe
```

The artifact records the target and QBE `-t` option in a comment. Invoking QBE and the linker belongs to the consumer. Library callers use `compile_to_qbe_with_target(source, Some(codegen::qbe::Target::X86_64UnknownLinuxGnu))`, or `X86_64PcWindowsMsvc`. See [Windows ABI and validation](../design/qbe-windows.en.md).

### WAT and direct assembly strings

`emit-wat` remains fixed to WebAssembly. `emit-asm` accepts explicit Windows/Linux selection and preserves Windows as its default.

WAT with ordinary output imports `cerune.write_stdout_v1(i64, i32) -> i32`, exchanging up to eight bytes by value and acceptance/failure responses without exposing memory. Hosts implement the [versioned output contract](../design/wat-output-host.en.md). `emit-wat` does not launch a host.

WAT containing argument operations requires the [versioned argument host contract](../design/wat-arguments-host.en.md). Use `emit-wat --argument-count-limit <count> --argument-byte-limit <bytes>` to emit explicit startup limits; inject execution argument values separately through the host.

Float display also requires `format_f32_v1` or `format_f64_v1`. WAT receives the complete length and bytes into private formatting storage, then uses the common write connection. Old hosts missing required imports fail instantiation.

WAT with runtime checks or ordinary output also imports `cerune.write_error_byte(i32) -> void`. Hosts write these ASCII diagnostics to stderr and preserve previous stdout when `unreachable` traps. See the [runtime diagnostic contract](../design/runtime-diagnostics.en.md).

Generate Windows x64 direct assembly with `cerune emit-asm examples/string_lookup.ceru -o target/string_lookup.s` and build it with `clang --target=x86_64-pc-windows-msvc target/string_lookup.s -o target/string_lookup.exe`. Programs using strings switch standard output to binary mode before output.

## Native input and origins

`emit-asm` / `emit-obj` generate through HIR→MIR→x86-64 LIR. `--annotate-origins` retains original NodeId/source byte ranges and adds MIR block/instruction-to-LIR correspondence. Objects retain corresponding symbols. See [Native lowering from MIR](../design/native-mir.en.md). Existing target selection and artifact formats are retained.

## VM execution

```text
cerune run-vm <file>
```

`cerune run-vm` lowers the program to Cerune bytecode and executes the resulting `BytecodeProgram` in the Cerune VM.

Runtime output is useful for validation and experiments, but it is distinct from observations of HIR, MIR, and artifacts defined in the [compiler design](../design/architecture.en.md).

When a runtime error occurs in a bytecode instruction derived from source, the diagnostic includes both the source location and the bytecode instruction index:

```text
cerune: cannot divide an integer by zero at 1:7 (bytecode instruction 0002)
```

The bytecode instruction index is still displayed when no source location is available. Compact diagnostics do not include source text or the input file path.

`run-vm --diagnostic-format runtime-v1` emits language check failures as a single record containing the reason, NodeId, and UTF-8 byte range. Compilation diagnostics and VM internal errors retain their existing format. Previously executed `print` output remains on stdout. C, LLVM, QBE, WAT, Windows/Linux assembly, and internal objects use the same failure records. See the [common diagnostic contract](../design/runtime-diagnostics.en.md) and [expected-failure examples](../../examples/runtime_failures/README.en.md).

## Version

```text
cerune --version
```

`cerune --version` prints the Cerune version.

## External settings not controlled by Cerune

Cerune does not choose external experiment policy such as:

- GCC versus Clang;
- optimization levels for external compilers;
- CPU targets for external toolchains;
- benchmark settings;
- measurement policy;
- comparison policy.

Those choices belong to the caller and should be recorded when necessary.

## Following LLVM origins

Run `cargo run -- emit-ir examples/string_origins.ceru`, then `cargo run -- emit-llvm examples/string_origins.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o string-origins.ll`. Use `x86_64-pc-windows-msvc` for Windows. Run the example with `cargo run -- run examples/string_origins.ceru`.

`--annotate-origins` is optional and LLVM-only. Ordinary output is unchanged. The API is `compile_to_llvm_with_options(source, llvm::Options { target, annotate_origins: true })`. See the [annotation contract](../design/observability.en.md#llvm-origin-annotations).

## WAT u64 output

Artifacts printing `u64` import `cerune.print_u64(i64) -> void`. The host writes unsigned 64-bit decimal digits followed by LF. JavaScript hosts use `BigInt.asUintN(64, value).toString()`. See the [u64 design](../design/u64.en.md).

## Cerune object generation

`emit-obj` requires `--target x86_64-pc-windows-msvc` or `x86_64-unknown-linux-gnu` and `-o`. It generates COFF/ELF without external tools and never writes binary data to stdout. `--annotate-origins` retains origin labels. Invalid options and compilation diagnostics preserve existing output. Linking and execution remain separate explicit operations. See the [native encoder](../design/native-encoder.en.md). The library API `compile_to_native_object(source, target, annotate_origins)` returns `Vec<u8>`.

## Explicit WAT flush

Artifacts containing `flush();` import `cerune.flush_stdout_v1() -> i32`. Hosts return 0 for success and nonzero for failure. Failure sends an `io-v1` record through `write_error_byte` and traps. Hosts missing the import fail instantiation. Memory stays private.
