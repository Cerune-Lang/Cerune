# Common runtime failure records

[日本語](runtime-diagnostics.ja.md)

## Purpose and current scope

A nonzero exit alone cannot distinguish an intended language stop from an access violation or startup failure. Cerune compares failure reasons and source locations across routes as part of observability, without exposing state mutation or execution intervention.

| Route | Current runtime diagnostics |
| --- | --- |
| IR Executor | `run` / `run-ir`: source-aware diagnostics or explicit `runtime-v1`; CLI exits with code 1 |
| VM | `run-vm`: default human-readable diagnostics and explicit `runtime-v1` format |
| Windows/Linux direct assembly | Write `runtime-v1` to stderr, then fast-fail on Windows or execute an illegal instruction on Linux |
| Internal COFF/ELF encoder | Encodes the same assembly lowering and diagnostics |
| C | `runtime-v1` on C standard stderr, followed by `abort` |
| LLVM | `runtime-v1` using the explicit Windows/Linux output ABI, followed by a trap |
| QBE | `runtime-v1` using the explicit Windows/Linux output ABI, followed by `abort` |
| WAT | `runtime-v1` bytes through `cerune.write_error_byte`, followed by `unreachable` |

IR Executor, VM, C, LLVM, QBE, WAT, direct assembly, and internal objects expose comparable reasons and source locations for language check failures. Verification requires both the record and each route's intended termination; OS and host exit codes remain route-specific.

## Record

```text
cerune: runtime-v1 code=division-by-zero node=1 bytes=6..11
```

This identifies division in `print(1 / 0);`. Each ASCII line contains:

- `code`: A failure reason from the table below.
- `node`: The corresponding Cerune IR NodeId. A failure inside a callee identifies the failing expression, not the caller's expression.
- `file` (optional): A positive registered `SourceMap` file ID between `node` and `bytes`. Absence means the existing anonymous source 0.
- `bytes`: The original UTF-8 source range, zero-based with an exclusive end. These are not line numbers or displayed character counts.

The [file-identity API](source-files.en.md) produces records such as `cerune: runtime-v1 code=division-by-zero node=1 file=2 bytes=6..11`, with a range local to that file. Existing single-source APIs and CLI inputs without import/pub retain their records. Readers must support this optional field; older strict readers cannot accept registered-file records. The bundled observation script and WAT validation host validate both forms, rejecting `file=0`, leading zeros, duplicate fields, and invalid numbers. CLI inputs using [modules](modules.en.md) report file IDs, including the entry, resolved against the inventory and text from `emit-sources`.

Records exclude paths, source text, and runtime values. Interpret them with the same source and compiler version. NodeIds are not persistent across edits or versions. Native observation manifests also record the source SHA-256 and tools used.

| Code | Meaning |
| --- | --- |
| `integer-overflow` | Addition, subtraction, multiplication, negation, or left shift exceeds the type's range |
| `division-by-zero` | Integer division by zero |
| `division-overflow` | Integer division result exceeds the type's range |
| `remainder-by-zero` | Integer remainder with a zero divisor |
| `invalid-shift-count` | Shift count is negative or at least the type's bit width |
| `integer-conversion-out-of-range` | Explicit integer-to-integer conversion exceeds the range |
| `conversion-out-of-range` | Conversion involving floating point exceeds the range |
| `conversion-inexact` | Conversion changes the value |
| `conversion-not-finite` | NaN or infinity cannot convert to an integer |
| `conversion-nan` | Changing float types cannot preserve NaN |
| `conversion-negative-zero` | Integer conversion cannot preserve negative zero's sign |
| `array-range-out-of-bounds` | Invalid array-copy range |
| `array-length-out-of-range` | Negative `array_repeat` count |
| `allocation-size-overflow` | String/array length or allocation layout is unrepresentable |
| `allocation-limit-exceeded` | Live string contents or array element storage exceeds its respective budget |
| `allocation-failed` | Allocation reports failure after validation |
| `array-index-out-of-bounds` | Array index is outside its bounds |

For array assignments, the NodeId identifies the assignment and the range identifies the failing `[...]`. Nested indices are checked left to right; failure prevents right-hand-side evaluation. An integer-to-float conversion that rounds beyond a range still reports `conversion-inexact`: it cannot preserve the original integer value.

This format does not classify compilation diagnostics, invalid-bytecode internals, OS resource failures, or output I/O failures as language check failures. The VM uses existing diagnostics for those cases; unexpected generated-code crashes must not receive fabricated common records.

## Usage and retained output

```sh
cerune run examples/runtime_failures/function_division.ceru --diagnostic-format runtime-v1
```

`run` / `run-ir` show the reason and source location; `run-vm` keeps the existing source/bytecode-position diagnostics. `--diagnostic-format runtime-v1` selects common records for language check failures. The VM exits with code 1. `ExecutionError::runtime_failure()` exposes structured reasons and ranges.

Previously executed `print` operations are not rolled back. The VM retains their output in `VmError::output()`, and the CLI writes it to stdout. Direct ASM/native objects flush existing stdout buffers and write the stderr record. Linux then executes `ud2`; Windows uses `FAST_FAIL_FATAL_APP_EXIT` (rcx=7, `int 0x29`, status `0xc0000409`). This does not guarantee the display order of merged stdout/stderr. Windows programs containing strings retain their existing binary stdout behavior.

Writing uses the explicitly selected target's Linux `write` or Windows `_write`. Diagnostic comparisons normalize CRLF/LF. If stderr is closed or a write fails partway through, code still attempts the target-specific stop, but complete diagnostic delivery is not guaranteed. Observation tools reject incomplete records.

The internal Windows ASM/COFF routes do not yet emit unwind information. We reproduced an illegal-instruction exception followed by a different OS failure from a deeper generated stack. Language checks now explicitly emit [fast-fail](https://learn.microsoft.com/en-us/cpp/intrinsics/fastfail?view=msvc-170), which terminates without invoking exception handlers (Windows 8 or later). Code 7 describes the OS termination; `runtime-v1` carries Cerune's specific reason and origin. Tests require matching records and prior output in addition to the exit status. Generating [unwind information](https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64?view=msvc-170) for debugger stack recovery remains separate work.

## Implementation decisions

C passes immutable location strings into checked helpers, which report codes and locations through standard C stderr on failure. QBE and LLVM likewise use read-only data and helper arguments. Successful paths do not mutate a global current-location variable, and caller locations do not overwrite failures inside called functions.

LLVM requires `--target` for potentially failing operations, conversions, or bounds checks, as well as strings. Even `print(1 + 2);` emits a checked addition. Missing targets produce a diagnostic rather than inferring Windows/Linux from the host. QBE also selects its diagnostic ABI through the explicit target. Strings require a target; numeric-only calls without one retain the previous Linux/SysV assumptions. Windows writes the record with `_write`, disables additional CRT messages and crash collection, then calls `abort` for exit code 3. See [QBE Windows support](qbe-windows.en.md).

WAT emits ASCII bytes from statically specialized checked helpers through `cerune.write_error_byte(i32) -> void`. Modules with diagnostics require this import. Hosts retain/write these bytes as stderr and preserve previous stdout when a trap occurs. Neither memory nor mutable diagnostic state is exported. Complete delivery is not guaranteed if the host discards output, throws from an import, or encounters an I/O error.

Each source-derived failure site has a read-only record. This increases artifact size, but reporting runs only on failure branches, without introducing mutable “current location” state on successful paths. Records require neither dynamic allocation nor externally mutable diagnostic variables. Checks needed to distinguish numeric conversion reasons are always generated as part of preserving operation semantics.

`--annotate-origins` still only controls observational comments and symbols. It does not disable runtime diagnostics. Tests compare instruction/data contents and behavior with and without annotations. Failure ABIs and output functions depend on the explicit compilation target, not the host OS.

## Verification

`cargo test --test runtime_routes` uses these external tools. Configured but unavailable tools fail the test. This test runs QBE on Linux x86-64. Windows uses `cargo test --test qbe_windows -- --include-ignored`, requiring exact stdout/stderr bytes even for numeric-only programs. For the existing C and LLVM routes in `runtime_routes`, Windows numeric-only CRT output normalizes CRLF to LF for comparison; output containing strings requires exact bytes, including NUL and CR/LF.

| Environment variable | Tool |
| --- | --- |
| `CERUNE_TEST_CC` | C compiler and linker for QBE-generated assembly, such as `clang` |
| `CERUNE_TEST_LLVM_CLANG` | Clang for generated LLVM |
| `CERUNE_TEST_QBE` | QBE 1.3 executable |
| `CERUNE_TEST_NODE` | Node.js |
| `CERUNE_TEST_WAT2WASM_JS` | Path to WABT's `bin/wat2wasm` script |

CI configures all tools used by its platform. Locally, unavailable unconfigured tools or a missing WAT compiler configuration produce an explicit skip; skipped routes are not execution coverage.

[Four expected-failure examples](../../examples/runtime_failures/README.en.md) trace overflow, nested array assignment, division inside a function, and successful, short-circuited, and failing calls to the same function. They are separate from normal examples and list their intended failure reasons.

`observe-native.cjs --run --expect-trap` runs the VM with common records and compares reason, NodeId, byte range, and prior output with native execution. It also requires SIGILL / Windows fast-fail termination. Matching diagnostics with successful termination, unreported crashes, additional errors, and timeouts do not pass. The manifest's `runtimeFailure` stores the matched record.

Tests compare 49 failure cases with known reasons, matching Windows/Linux assembly and internal objects against the VM. They include Unicode, CRLF, short-circuiting, function calls, default-field expressions, conversion boundaries, and retained output. The 50 normal examples and existing test suite also remain part of validation.

C, LLVM, QBE, and WAT run these 49 cases plus the expected-failure examples and additional evaluation-order cases. C and LLVM use both `-O0` and `-O2`; QBE-generated assembly is linked with both flags.

This failure test limits linking to 30 seconds and generated execution to 10 seconds. A timeout fails with the case name and captured output. Stdout/stderr use temporary files, separating child termination from pipe EOF. `CERUNE_TEST_TRACE=1` reports each case's start and end. Linux CI limits the full test command to eight minutes (with a ten-second termination grace period), limits the job to fifteen minutes, and preserves the test log as an artifact.

Linux CI sets the disposable runner's `kernel.core_pattern` to a file pattern and applies `ulimit -c 0` in the test shell. Core size limits alone do not disable piped crash collection. This configures the CI observation environment; generated `ud2` instructions and diagnostic checks remain unchanged. Core files are not used by these assertions. Tracing also records successful comparisons and temporary-directory cleanup. The eight-minute deadline covers the entire pipeline, including the log collector `tee`.
