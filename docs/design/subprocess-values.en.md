# Public subprocess data types

[日本語](subprocess-values.ja.md)

`import "std:process" as process;` provides request and result types. **Only data types are implemented. `process::run`, subprocess IR instructions, and CLI registration are not implemented.** Conversion from the [Rust common runtime and Windows/Linux connections](subprocesses.en.md) into language values is also subsequent work.

## Types and retained information

The [bundled source](../../src/standard/process.ceru) declares ordinary structs and enums, without a separate value system or generic Result type.

| Type | Contents |
| --- | --- |
| `Request` | `target: string`, `arguments: [string]`, `stdin: [u8]`, `stdout_limit: u64`, `stderr_limit: u64`, `timeout_ms: u64` |
| `Outcome` | `report: Report` and owned `stdout: [u8]` / `stderr: [u8]`, including partial output |
| `Report` | Origin, launch status, configured limits, requested/confirmed/retained byte counts, progress certainty, exit status, original failure, cleanup verdict and raw report |
| `Origin` | `node_id`, `source_id`, `start`, and `end` as `u64` |
| `Started` | `No`, `Yes`, `Unknown` |
| `Exit` | `Absent`, `Code { value: u32 }`, `Signal { value: u32 }` |
| `Failure` | `None`, `Reason { code: FailureCode, phase: Phase }` |
| `Phase` / `Channel` | Validation, reservation, start, transfer, stdin closure, wait, cleanup; transfers distinguish stdin/stdout/stderr |
| `FailureCode` | The same names as the common runtime's 21 categories; unconfigured, capacity exceeded, timeout, unknown progress, etc. remain distinct |
| `Cleanup` / `CleanupError` | Absent, complete, incomplete; retain termination requests, child reclamation, handle closure, scope completion, and cleanup errors |

Programs can construct these ordinary values. Their existence does not prove that an OS operation occurred. The current [example](../../examples/process_results/README.en.md) constructs values and processes them.

## Correspondence with the common runtime

Future integration must preserve the following mapping. Conversion and execution instructions remain unimplemented.

| Rust representation | Public meaning |
| --- | --- |
| `Report.failure: Option<Failure>` | `None` means operation completion, independently of child exit code zero; cleanup failure must not overwrite the first failure |
| `Report.exit: Option<Exit>` | `Absent` means unavailable; preserve all 32 bits of Windows exit codes without guessing signal equivalents |
| `Report.cleanup` / `cleanup_reported` | Common-runtime verdict including deadlines / raw connection report, separately retained; no report is `Absent` |
| `stdin_requested` / `stdin_accepted` | Requested / confirmed accepted byte counts |
| `stdout_read` / `stderr_read` | Bytes confirmed read by the connection |
| `stdout_retained` / `stderr_retained` | Bytes retained in result arrays; not necessarily the read counts |
| `progress_known` | When false, transfer counts are confirmed lower bounds; do not replace unknown progress with zero or pretend rollback |
| `Origin` | NodeId and Span's SourceId / UTF-8 byte range `[start, end)`; IDs are compilation-local, SourceId 0 denotes existing anonymous source; no absolute paths or content |
| `Outcome.transfers` and events | Individual transfer state remains Rust-side read-only observation; public Outcome carries no operation handles or control API |

Public enum order is not a Rust discriminant mapping or external ABI. Future conversion checks names and contents explicitly. Tests check all `FailureCode` categories against Rust.

## Requests, arrays, and allocation

- Public time values use whole milliseconds. Language requests convert exactly to `Duration`; no conversion silently rounds arbitrary submillisecond Rust durations into public values.
- Being representable as `u64` is distinct from being accepted for execution. Validate target names, NUL, capacity, time, and host integer width before launch. Currently only constructing values is supported, so even invalid requests are constructible. Accepted ranges, defaults, and connection configuration follow separately.
- Contents are byte arrays: retain invalid UTF-8, NUL, and CR/LF without adding newlines, normalizing, or implicitly converting to strings. Copies, argument passing, and updates reuse existing value semantics and ownership lowering.
- **[Prelaunch reservation and array finalization/cancellation](subprocess-storage.en.md) are implemented. Language-instruction integration follows.** Current `u8` arrays use 64-bit element storage in the common representation; payload bytes alone cannot account for language array usage. Distinguish ownership and accounting for retained output buffers, transfer buffers, language arrays, and conversion temporaries, reserving required storage before launch.
- Runtime resource exhaustion that prevents constructing a value is not a child exit code or successful operation. Do not repeat execution after such a failure.

## Observation and verification

The standard source registers as `std:process`, contract version 1. Aliases and imports from multiple files share nominal type identity; `compile(&str)` adds no implicit file I/O. `emit-sources` exposes declarations and origins; `emit-ir` and later forms expose ordinary enum, ownership, and array lowering.

C output prefixes source fields with `cerune_field_` and enum internals with a separate `cerune_internal_field_` name, avoiding header macros such as `stdin` and C keywords. Common IR retains source field names and NodeId/Span.

[Differential tests](../../tests/process_types/mod.rs) check four executors and generated C/LLVM output, nominal identity, failure categories, independent copies, and rejection of unimplemented `run`. QBE/WAT/ASM/Object generation is checked separately from execution comparisons for this example. LLVM execution comparisons run only on Windows/Linux x86-64, specifying the same target during emission and linking. Other hosts retain four-executor/C comparisons and generation checks. This does not validate conversion of actual OS results into language values.

```sh
cargo test --test modules process_types
```

Next, prepare the public result's fixed shape and connect `run` to four executors through one common IR operation. Generated routes, host contracts, CLI registration, and distribution follow. Observation APIs must not control execution; unsupported operations must not become successful no-ops.
