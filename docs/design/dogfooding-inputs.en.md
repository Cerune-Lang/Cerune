# Arguments and integer parsing driven by real uses

[日本語](dogfooding-inputs.ja.md)

2026-10-20. This is the next design proposal for [#173](https://github.com/Cerune-Lang/Cerune/issues/173) and [#89](https://github.com/Cerune-Lang/Cerune/issues/89). **The argument and parsing APIs described here are not implemented.** Existing stdin and WAT function exports remain in place.

## Smallest use

Produce the same integer sequence from the same seed and count. First accept the seed as a program argument; file storage and subprocesses are separate stages.

[deterministic_seed.ceru](../../examples/deterministic_seed.ceru) is an executable baseline with source-defined seeds 0, 1, 1337 and 2147483646, plus zero iterations. It uses no OS randomness and is not cryptographic.

```text
next = (state * 48271) % 2147483647
0 <= state <= 2147483646
```

The maximum product, 103661183076066, fits i64. Known expectations are compared across HIR/MIR/SSA/VM, including reads of the unchanged IR representation. This is a small computation baseline for argument support, not a copy of Parallax's generator or output protocol.

## Reproducing the missing capability

On master 200c1ec, `cerune run examples/deterministic_seed.ceru -- 1337` exits 1 with a usage diagnostic. A source containing only `import "std:env" as env;` also exits 1 with unknown standard module. Do not present these as working argument APIs.

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

## Proposed argument contract

A reserved virtual `std:env` module would expose argument count and indexed argument retrieval. This does not introduce environment-variable access or file searches. API names must be settled before implementation.

| Item | Proposed contract |
| --- | --- |
| CLI | `cerune run source.ceru [Cerune options] -- [program arguments...]`; same separation for run-ir/run-mir/--ssa/run-vm |
| Splitting order | Separate the first delimiter before any option parsing; subsequent `--ssa` or `--string-heap-limit` remain program values |
| Contents | Only program arguments; omit executable/source names and compiler options |
| Empty/missing | Empty string is present. Out-of-range uses an existing enum such as Missing; negative index is diagnosed |
| Text | Preserve UTF-8, order, whitespace and CR/LF; no normalization, splitting or quote reinterpretation. Propose rejecting NUL across connections because OS arguments cannot represent it |
| Connection | Inject a snapshot fixed before execution; never reread OS state or an externally mutable list per call |
| Embedding | Explicit host injection; distinguish unconfigured from configured empty. Existing APIs do not read host-process arguments implicitly |
| Ownership/limits | Bound snapshot count and total UTF-8 bytes. Returned strings follow existing immutable contents, ownership and string-heap limits |
| Observation | Trace common IR operations, types, NodeId/Span, index and result kind. Do not include argument text in default diagnostics or bundles |

The current CLI reads `env::args()`, and `parse_heap_limits` scans all arguments. Program arguments must be separated before this scan. OS string conversion must be explicit so invalid Unicode does not panic. Startup errors must not invent NodeIds/Spans; distinguish them from failures in source operations.

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

1. Design declarations, snapshot and common IR; explicitly connect HIR/MIR/SSA/VM. Add a small argument-display example.
2. Connect C/LLVM/QBE/WAT/Native and compare equal argument lists. Unconnected routes must not silently use an empty list or succeed.
3. Add pure integer parsing across routes and complete the argument-driven seed example.
4. Separately design file I/O within an explicit working scope, then subprocesses without a shell.

Retain original HIR, non-SSA MIR, SSA, generated artifacts and origins throughout. Optimization, Native build and package management are not prerequisites.

Before implementation, settle public/enum names, concrete snapshot limits, allocation failure timing, Windows command-line splitting/UTF-16 conversion and the versioned WAT host ABI. Do not export WAT memory for argument transfer. C/LLVM/QBE/Native must explicitly select target-specific acquisition rather than infer it from the emitter host OS.

Regression cases include zero arguments, empty strings, Unicode, CR/LF, order, option-like text after the delimiter, invalid Unicode, exact/exceeded limits and unconfigured connections. Parsing tests cover both i64 bounds and neighboring values, syntax/range precedence, stopping behavior, omitted input text in diagnostics and observation-independent results.
