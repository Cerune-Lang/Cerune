# Arguments and integer parsing driven by real uses

[日本語](dogfooding-inputs.ja.md)

2026-10-24. This is the next design proposal for [#173](https://github.com/Cerune-Lang/Cerune/issues/173) and [#89](https://github.com/Cerune-Lang/Cerune/issues/89). **Argument acquisition works in HIR/MIR/SSA/VM, CLI and generated C/LLVM/QBE/WAT/Native; integer parsing is also implemented in [std:num](integer-parsing.en.md).** Existing stdin and WAT function exports remain in place.

## Smallest use

Produce the same integer sequence from the same seed and count. First accept the seed as a program argument; file storage and subprocesses are separate stages.

[deterministic_seed.ceru](../../examples/deterministic_seed.ceru) is an executable baseline with source-defined seeds 0, 1, 1337 and 2147483646, plus zero iterations. It uses no OS randomness and is not cryptographic.

```text
next = (state * 48271) % 2147483647
0 <= state <= 2147483646
```

The maximum product, 103661183076066, fits i64. Known expectations are compared across HIR/MIR/SSA/VM, including reads of the unchanged IR representation. This is a small computation baseline for argument support, not a copy of Parallax's generator or output protocol.

## Baseline before implementation

On master 200c1ec, `cerune run examples/deterministic_seed.ceru -- 1337` exits 1 with a usage diagnostic. A source containing only `import "std:env" as env;` also exits 1 with unknown standard module. This is historical evidence; the public API and CLI described below now support arguments.

## Responsibilities in Parallax

The inspected revision is [Parallax 356f357](https://github.com/Cerune-Lang/Parallax/tree/356f357cb2bf347ef8ec7462e6bdd77410a6e7d7).

| Operation | Current owner | Work for Cerune |
| --- | --- | --- |
| Generate values/operations from fixed seeds | Also implemented in .ceru | Replace source constants with program arguments |
| Expected results | Independent Python oracle | Retain it; do not rely solely on Cerune to judge itself |
| Build/reduce typed programs | Python; Cerune emits some generation parameters | Try bounded existing structures and split demonstrated string/recursion needs |
| Parse numeric arguments and output protocol | Python | Define explicit string-to-integer rules |
| Store/reload files | Python and temporary directories | Design limited file I/O later |
| Execute/reap/compare | Python subprocess, timeouts, stdout/stderr/status | Design subprocess APIs last; observation must not execute programs |

Python's existing random generator and the Cerune LCG use different algorithms. Equal seeds do not promise equal sequences across generators. Record generator/version, seed, count and Cerune commit for reproduction.

Track argument acquisition in [#174](https://github.com/Cerune-Lang/Cerune/issues/174) and integer parsing in [#175](https://github.com/Cerune-Lang/Cerune/issues/175).

## Common argument contract

A reserved virtual `std:env` module would expose argument count and indexed argument retrieval. This does not introduce environment-variable access or file searches. Public names are now arg_count / arg / Argument; generated connections follow separately.

| Item | Proposed contract |
| --- | --- |
| CLI | `cerune run source.ceru [Cerune options] -- [program arguments...]`; same separation for run-ir/run-mir/--ssa/run-vm |
| Splitting order | Separate the first delimiter before any option parsing; subsequent `--ssa` or `--string-heap-limit` remain program values |
| Contents | Only program arguments; omit executable/source names and compiler options |
| Empty/missing | Empty string is present. Out-of-range uses an existing enum such as Missing; negative index is diagnosed |
| Text | Preserve UTF-8, order, whitespace and CR/LF; no normalization, splitting or quote reinterpretation. Reject NUL across connections because OS arguments cannot represent it |
| Connection | Inject a snapshot fixed before execution; never reread OS state or an externally mutable list per call |
| Embedding | Explicit host injection; distinguish unconfigured from configured empty. Existing APIs do not read host-process arguments implicitly |
| Ownership/limits | Bound snapshot count and total UTF-8 bytes. Returned strings follow existing immutable contents, ownership and string-heap limits |
| Observation | Trace common IR operations, types, NodeId/Span, index and result kind. Do not include argument text in default diagnostics or bundles |

CLI now strictly converts `env::args_os()` and separates program arguments before `parse_heap_limits`. Invalid Unicode produces a diagnostic rather than a panic. Startup errors must not invent NodeIds/Spans; distinguish them from failures in source operations.

`compile(&str)`, emit and observe do not acquire arguments and construct the same IR. Standard declarations/source use the existing SourceMap and std:io resolution, type identity and origin mechanisms, without separate frontend semantics.

## Integer parsing as a separate pure operation

`std:num::parse_i64` implements string-to-i64 parsing. It is not seed-specific or an implicit extension to numeric `convert<T>`.

| Input | Result |
| --- | --- |
| ASCII `-?[0-9]+` | i64 value; allow leading zeros, parse `-0` as integer zero |
| Empty, sign only, `+1`, whitespace, newline, fullwidth digits, fractions, NUL, trailing text | Invalid format |
| Valid grammar outside i64 limits | Out of range |
| Result type | Existing enum for success/invalid-format/out-of-range; no prerequisite generic Result |
| Precedence | Validate the whole syntax before range: huge digits followed by x are invalid format |

Parsing and application-level seed validation are separate: valid negative i64 values or seeds at least 2147483647 are rejected by the example. Returning a format error as a value is distinct from execution failures such as allocation failure. Parsing does not consume or mutate input.

General string indexing is not public. An internal pure byte-access operation in common IR lets bundled Cerune source share syntax and range checks. See the [final API and internal operation](integer-parsing.en.md).

## Implementation units and validation

1. Implemented: public declarations, snapshot and common IR connected to HIR/MIR/SSA/VM and CLI, with an argument-display example and comparison tests.
2. Implemented: generated C/LLVM/QBE/WAT/Native connections with equal argument lists. Native includes baseline MIR/SSA, assembly and self-encoded objects.
3. Implemented: pure integer parsing across routes and an example accepting seed and count arguments.
4. After arguments and integer parsing, follow the [core roadmap](language-roadmap.en.md): optional let + fmt, Native build/initial distribution, then recursion. Subsequently design limited file I/O and shell-free subprocess operations separately.

Retain original HIR, non-SSA MIR, SSA, generated artifacts and origins throughout. Optimization, Native build and package management are not prerequisites.

C Windows startup, UTF-16 conversion and limits are settled below. Native startup and limits are also implemented; the versioned WAT host ABI is implemented. Do not export WAT memory for argument transfer. C/LLVM/QBE/Native must explicitly select target-specific acquisition rather than infer it from the emitter host OS.

Regression cases include zero arguments, empty strings, Unicode, CR/LF, order, option-like text after the delimiter, invalid Unicode, exact/exceeded limits and unconfigured connections. Parsing tests cover both i64 bounds and neighboring values, syntax/range precedence, stopping behavior, omitted input text in diagnostics and observation-independent results.

## Argument storage and validation: first implemented component

`runtime::arguments::Arguments` now stores and validates arguments on the Rust side. The immutable component is connected to the public API, common IR, CLI and four executors described below.

| Area | Implemented contract |
| --- | --- |
| Injection | Explicitly pass separated byte strings and `Limits { max_count, max_bytes }` to `capture`; no OS, environment-variable, or file reads |
| Contents | Strict UTF-8; retain empty strings, Japanese text, spaces, CR/LF, combining characters and order without splitting or normalization. Reject NUL |
| Count | Allow a configured zero-element list. `max_count` ranges from 0 to i64::MAX; counts and indices use i64 |
| Byte limit | Sum of UTF-8 contents, including CR/LF. An empty argument costs zero bytes but one entry. Terminators and allocator bookkeeping are excluded |
| Validation order | Limit validity → total count → each argument in order: cumulative byte limit → UTF-8 → NUL. Report the first violation in this order |
| Allocation | Validate all entries, then copy the table and strings with fallible reservations. Never return a partial snapshot. The connection owns allocation of the original host input |
| Ownership | Changing or dropping the host input after capture does not affect the snapshot. No mutation API or unconditional Clone |
| Reads | Unconfigured returns `Unconfigured`; configured negative indices return `InvalidIndex`; missing entries return `None`; present empty strings return `Some("")`. Check configuration first |
| Observation | Counts, byte usage and Debug exclude contents. Reads allocate nothing, consume nothing and do not contact the OS. Only explicit `get` returns contents |
| Capture failures | `arguments-v1 phase=capture` records a reason and applicable argument index only, without contents or fabricated NodeId/Span |

Snapshot storage and the heap limit for strings returned during execution are separate. CLI limits, copying into the string heap and source diagnostics with NodeId/Span are implemented below; chronological success-event records follow separately. Validation distinguishes count excess, byte-limit excess, invalid UTF-8, NUL, and allocation failures.

Failure tests cover precedence, exact/exceeded limits, and failed table/string allocations. Success tests cover non-normalized Unicode, independence from host input, empty/missing/unconfigured states, and non-interference of repeated reads and Debug. Public standard declarations and common IR operations now explicitly inject this snapshot into executors.

## Public API, CLI and four executors

Explicitly import `std:env` and use `env::arg_count() -> i64` and `env::arg(index: i64) -> env::Argument`. The ordinary enum has `Argument::Value { text: string }` and `Argument::Missing {}`. Empty text is Value, a nonnegative out-of-range index is Missing, and a negative index stops. Type identity and virtual-source origins use the same mechanism as std:io.

| Area | Implemented contract |
| --- | --- |
| Routes | HIR/MIR/SSA/VM; CLI run/run-ir/run-mir/run-mir --ssa/run-vm |
| CLI delimiter | Only values before the first `--` are parsed as Cerune options. Subsequent empty strings, `--ssa`, `--string-heap-limit`, etc. remain program arguments |
| CLI limits | Defaults: 4096 entries and 1048576 total bytes. Override with `--argument-count-limit` / `--argument-byte-limit` before the delimiter. Zero is valid; duplicates and invalid numbers fail |
| Startup | Strictly convert already separated OS strings from Rust `args_os`; no additional splitting or quote interpretation. Invalid Unicode produces a redacted capture diagnostic |
| No arguments | CLI supplies configured zero entries. Existing embedding APIs remain unconfigured. Compilation and emission do not acquire arguments |
| Rust APIs | Explicitly inject immutable Arguments through each executor's `run_with_arguments` / `run_with_io_and_arguments`. VM exposes these too; lib adds `run_bytecode_with_io_and_arguments` |
| Common representation | HIR ArgumentCount/ArgumentGet and MIR/SSA/bytecode arguments.count/arguments.get. Ordinary lowering handles Missing branching and enum construction from bundled Cerune source |
| Ownership | Copy each nonempty result into the existing heap, then release through ordinary ownership lowering. Empty text costs zero bytes. Snapshot and returned-value heap limits are separate |
| Execution failures | arguments-v1 phase=execute records operation, reason, index and NodeId/SourceId/Span, then stops. Distinguish unconfigured, negative index, heap-limit excess and allocation failure; omit contents |
| Origins | The internal operation is located in std:env; HIR Call retains the user's call site. Never substitute a fabricated user span |

Argument failures use arguments-v1 even with `--diagnostic-format runtime-v1`. Capture failures have no source-operation position. OS argument acquisition and temporary CLI conversion storage are separate from the snapshot storage limit.

The [example](../../examples/arguments/README.en.md) and [comparison tests](../../tests/program_arguments/mod.rs) verify expected output, short-circuiting, function/array transfers, independent copies, prior output, stop reason/origin, and unchanged original IR.

Native now generates argument-using LIR, assembly, self-encoded objects and observe bundles. Preserve original HIR, non-SSA MIR, SSA and origins. A chronological event log of successful accesses remains unimplemented. Pure integer parsing is implemented in [std:num](integer-parsing.en.md).

## Generated C argument connection

The same common IR operations generate C acquisition, diagnostics and ownership code. Emitting C never reads OS arguments.

| Area | Implemented contract |
| --- | --- |
| Startup | Selected by the C compilation target's `_WIN32`. Windows strictly converts `wmain` UTF-16 arguments with `WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS)`; other targets validate `main` arguments as UTF-8. Never select from the emitter's OS |
| Contents | Omit only the executable name from arguments already split by the CRT. No resplitting or normalization. Generated executables treat every argument as a value, so the Cerune CLI separator `--` is unnecessary |
| Startup validation | Validate OS Unicode, then validate limits, count, cumulative bytes, UTF-8 and NUL in the same order as the common snapshot; copy all entries before execution. Invalid OS text reports `invalid-unicode`. Original/temporary conversion storage is separate from snapshot limits |
| Limits | Set `CERUNE_ARGUMENT_COUNT_LIMIT` and `CERUNE_ARGUMENT_BYTE_LIMIT` when compiling C. Defaults: 4096 entries and 1048576 bytes. No implicit configuration through environment variables or executable arguments |
| Ownership | Snapshot contents remain immutable during execution. Retrieval copies into the existing string heap; normal ownership lowering releases values passed through functions, arrays, etc. Release the snapshot on normal termination |
| Failures | `arguments-v1` distinguishes startup from source operations. Negative indices, string-heap excess and allocation failures flush prior output, record NodeId/SourceId/Span and exit 1 without subsequent execution. Never include contents |
| Tools | Normal linking for Windows MSVC-target Clang. MinGW targets require `-municode` for Unicode startup. No extra libraries beyond the C runtime and basic Windows APIs |

[Generated C comparisons](../../tests/program_arguments/c.rs) run at `-O0` and `-O2`, covering the example, empty/Japanese/CR/LF/combining/supplementary-plane text, quotes, option-like values, evaluation order, short-circuiting, copying and release. They also check limits, mutation of original input, unconfigured snapshots, NUL, invalid UTF-8/UTF-16 and allocation failure. Input and arguments share the same emitted UTF-8 validator.

Static C helpers are not a published stable embedding ABI.

## LLVM argument connection

Lower the same snapshot, ownership and diagnostic contract directly to LLVM IR. LLVM emission does not internally generate C or invoke an external compiler.

| Area | Implemented contract |
| --- | --- |
| Target | Argument operations require `--target x86_64-unknown-linux-gnu` or `x86_64-pc-windows-msvc`. Never choose from the emitter's OS |
| Startup | Linux validates UTF-8 from `main(argc, argv)`; Windows strictly converts UTF-16 from `wmain(argc, argv)`. Validate and copy all entries before executing Cerune |
| Limits | `emit-llvm --argument-count-limit` / `--argument-byte-limit` emit visible IR constants. Defaults: 4096 entries and 1048576 bytes. Rust callers use `emit_llvm_with_argument_limits`; existing emission APIs retain defaults |
| Retrieval/ownership | Lower arguments.count/get through dedicated LLVM instructions to internal helpers. Copy results into the existing string heap and release through ordinary ownership lowering. Empty text allocates nothing. Release the snapshot on normal termination |
| Failures/origins | Startup diagnostics omit contents and source positions; execution uses arguments-v1 with operation, reason, index and NodeId/SourceId/Span. Flush prior output and exit 1. Distinguish negative indices, heap-limit excess and allocation failure |
| Observation | `emit-llvm --annotate-origins` shows source correspondence without changing instructions or results. Startup, validation, copying, release and limits remain visible in IR |

[LLVM comparisons](../../tests/program_arguments/llvm.rs) compile both targets into objects at `-O0`/`-O2`. On supported Windows/Linux hosts they compare the same example, normal/failing inputs and prior output against four executors. macOS validates both target objects without adding a macOS LLVM execution target. Generated C arguments also run on macOS; C stdin coexistence checks its existing unsupported-connection diagnostic there.

`emit-llvm` does not accept execution arguments after `--`. Pass program arguments to the linked executable. Integer parsing is next.

## QBE argument connection

Lower common IR argument operations through dedicated QBE instructions to internal helpers. Validation, copying and release remain visible in generated IL, without going through C or LLVM.

| Area | Implemented contract |
| --- | --- |
| Target/startup | Explicit Linux main or Windows wmain. Validate UTF-8 on Linux; strictly convert UTF-16 to UTF-8 on Windows. Validate and copy every entry according to the common contract before execution |
| Limits | emit-qbe --argument-count-limit / --argument-byte-limit produce IL constants; defaults are 4096 entries and 1048576 bytes. Rust uses emit_qbe_with_argument_limits; existing APIs retain defaults |
| Internal values | Snapshot entries hold a data pointer and byte length. Retrieval copies into existing length-prefixed strings and owner storage. OS conversion writes into caller-provided storage without depending on an aggregate-return ABI |
| Failures/origins | arguments-v1 distinguishes capture/execute. Execution retains operation, index and NodeId/SourceId/Span, flushes prior output, and exits 1. Omit contents |
| Ownership/observation | Empty values allocate nothing; ordinary ownership lowering releases retrieved strings, and normal termination releases the snapshot. Preserve common IR and expose startup, validation order, limits and ownership in IL |

[QBE comparisons](../../tests/program_arguments/qbe.rs) generate assembly with QBE 1.3 amd64_sysv and amd64_win. Native Windows/Linux execution is compared with four executors and generated C, covering the example, Japanese/empty/CR/LF/combining/supplementary-plane text, short-circuiting and evaluation order, independent copies, limits, invalid inputs, allocation failures, prior output and stdin coexistence. Older Linux Clang versions reject QBE's quoted COFF section syntax; Windows object assembly and execution are therefore required in Windows CI.

emit-qbe does not accept execution arguments after the delimiter. Internal helpers are not a stable embedding ABI. Integer parsing is next.

## WAT argument connection

The [versioned host contract](wat-arguments-host.en.md) is implemented. Startup validates frozen input and copies it into private memory; retrieval never rereads the host during execution. Compare returned-string ownership, limits, origins and prior output against four executors. Argument-using instances run once; discarding the instance reclaims the snapshot. Three new imports are required; missing imports fail instantiation on old hosts. WAT library restrictions remain in place.

## Native argument connection

Lower common MIR directly to LIR ArgumentCount/ArgumentGet. Baseline MIR and SSA use the same assembly and self-encoded object generation, without internally using C, LLVM or QBE.

| Area | Implemented contract |
| --- | --- |
| Startup | Explicit x86_64-unknown-linux-gnu main validates UTF-8; x86_64-pc-windows-msvc wmain strictly converts UTF-16. OS validation precedes common limits/UTF-8/NUL checks and a complete copy before execution |
| Limits/API | emit-lir/emit-asm/emit-obj accept --argument-count-limit / --argument-byte-limit, defaulting to 4096 entries / 1048576 bytes. Rust lower_mir_with_argument_limits stores limits in LIR; existing APIs retain defaults |
| Ownership | Copy the table and contents into one private allocation, freed on normal termination. Temporary OS conversion and returned-string heap limits are separate. Ordinary ownership releases retrieved strings; empty values allocate nothing |
| Failures | arguments-v1 distinguishes capture/execute. Execution retains operation, index, NodeId/SourceId/Span and prior output, then exits 1. Never include contents in diagnostics |
| Observation | Retain instructions, limits and MIR correspondence in LIR; expose startup, validation and copying in assembly. Emission never acquires actual arguments. Preserve main for artifacts without arguments |

[Native comparisons](../../tests/program_arguments/native.rs) compare the same example against four executors through baseline MIR/SSA, assembly and self-encoded objects. Windows/Linux CI require these tests. Cover unconfigured/empty/missing values, Unicode/CR/LF, limits, validation order, allocation failures, independent copies/release, short-circuit/evaluation order and stdin coexistence. Strict UTF-8 validation is shared by input and arguments.

Internal helpers are not a stable embedding ABI. Integer parsing is implemented in [std:num](integer-parsing.en.md). Chronological success records remain follow-up work. [Native build](native-build.en.md) and [initial release](native-release.en.md) are implemented.

PR #182 merged Native support into master. ASM/Object tests cover allocation failures during startup, OS conversion, snapshot capture and string retrieval, including prior output and arguments-v1. The [SASHIMI object connection](native-encoder.en.md) does not change argument semantics.

## Next: saving and reloading

[Limited file I/O design](file-io.en.md) covers whole-file UTF-8 reads and new-file creation in an explicit working directory. As of 2026-10-27, std:fs, common IR, four executors, Windows/Linux connections, and CLI connection options are implemented. Generated routes follow next. Replacement and subprocesses follow later.
