# Arguments and integer parsing driven by real uses

[日本語](dogfooding-inputs.ja.md)

2026-10-20. This is the next design proposal for [#173](https://github.com/Cerune-Lang/Cerune/issues/173) and [#89](https://github.com/Cerune-Lang/Cerune/issues/89). **Argument acquisition works in HIR/MIR/SSA/VM and CLI; generated-route connections and integer parsing remain unimplemented.** Existing stdin and WAT function exports remain in place.

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

Initially propose string-to-i64 parsing, rather than a seed-specific operation or an implicit extension to numeric `convert<T>`.

| Input | Proposed result |
| --- | --- |
| ASCII `-?[0-9]+` | i64 value; allow leading zeros, parse `-0` as integer zero |
| Empty, sign only, `+1`, whitespace, newline, fullwidth digits, fractions, NUL, trailing text | Invalid format |
| Valid grammar outside i64 limits | Out of range |
| Result type | Existing enum for success/invalid-format/out-of-range; no prerequisite generic Result |
| Precedence | Validate the whole syntax before range: huge digits followed by x are invalid format |

Parsing and application-level seed validation are separate: valid negative i64 values or seeds at least 2147483647 are rejected by the example. Returning a format error as a value is distinct from execution failures such as allocation failure. Parsing does not consume or mutate input.

General string indexing is currently unavailable. Compare the necessary pure IR operation and backend implementations before attempting to implement parsing solely in bundled Cerune source.

## Implementation units and validation

1. Implemented: public declarations, snapshot and common IR connected to HIR/MIR/SSA/VM and CLI, with an argument-display example and comparison tests.
2. Connect C/LLVM/QBE/WAT/Native and compare equal argument lists. Unconnected routes must not silently use an empty list or succeed.
3. Add pure integer parsing across routes and complete the argument-driven seed example.
4. After arguments and integer parsing, follow the [core roadmap](language-roadmap.en.md): optional let + fmt, Native build/initial distribution, then recursion. Subsequently design limited file I/O and shell-free subprocess operations separately.

Retain original HIR, non-SSA MIR, SSA, generated artifacts and origins throughout. Optimization, Native build and package management are not prerequisites.

Before generated-route implementation, settle Windows startup argument acquisition/UTF-16 conversion, the versioned WAT host ABI and per-route limit configuration. Do not export WAT memory for argument transfer. C/LLVM/QBE/Native must explicitly select target-specific acquisition rather than infer it from the emitter host OS.

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

C/LLVM/QBE/WAT/Native connections remain unimplemented and reject generation containing argument operations with source locations. Unused functions are retained, so importing std:env alone is sufficient. emit-sources/emit-ir/emit-mir/emit-bytecode remain observable; observe bundles requiring Native generation are unsupported. A chronological event log of successful accesses remains unimplemented. Connect generated routes next, then proceed to integer parsing once route support is complete.
