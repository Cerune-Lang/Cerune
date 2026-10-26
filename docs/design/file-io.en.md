# Limited file I/O

[日本語](file-io.ja.md)

**Status: design in progress; unimplemented.** On 2026-10-26, we agreed to start with bounded whole-file UTF-8 reads and creation of new files within an explicitly supplied working directory. Replacing existing files is outside the first version. API names, connections, and detailed rules below are proposals for implementation. This covers saving/reloading in [#89](https://github.com/Cerune-Lang/Cerune/issues/89) and [#173](https://github.com/Cerune-Lang/Cerune/issues/173), with comparison effects isolated according to [#104](https://github.com/Cerune-Lang/Cerune/issues/104).

## First use and proposed API

Save generated text to a new file, read it back, and compare it with the original value. Preserve UTF-8, NUL, CR/LF, and combining characters exactly; do not normalize, translate newlines, or remove a BOM.

| Proposed API | Success | Failure |
| --- | --- | --- |
| `fs::read_text(name: string, max_bytes: i64) -> string` | Read from the opened file's beginning to EOF; an empty file returns an empty string | Stop on missing files, limit overflow, invalid UTF-8, read/close failures, etc. |
| `fs::write_new(name: string, text: string, max_bytes: i64) -> void` | Create a new file, transfer all bytes, and close it | Leave an existing target unchanged; stop on creation/write/close failures |

Propose `import "std:fs" as fs;` as a reserved virtual standard module. The first version does not add a Result type, public file handles, seek, append, deletion, replacement, directory creation, or subprocesses. Separate the public stop-on-failure policy from internal operations returning progress, leaving room for a later recovery API.

This is an **unimplemented usage sketch**. Do not register it as an executable example under `examples/` until it works.

```cerune
import "std:fs" as fs;
text: string = "日本語\0\r\né\n";
fs::write_new("sample.txt", text, 4096);
saved: string = fs::read_text("sample.txt", 4096);
print(saved == text);
```

Expect `true` plus LF and file bytes exactly matching the original string. Give each execution an empty directory; a second execution against the same directory must stop without changing the existing file.

## Reuse from the current implementation

| Component | Current implementation | File operation plan |
| --- | --- | --- |
| Standard declarations, types, origins | [modules.rs](../../src/modules.rs), bundled Cerune sources, SourceMap | Reuse resolution and SourceId rules for std:fs; importing it does not read the target file |
| Input and ownership | [input.rs](../../src/runtime/input.rs), StringHeap | Consider sharing UTF-8 validation and reservations, but not line/EOF state; an empty file is a successful value |
| Output and progress | [output.rs](../../src/runtime/output.rs) | Align short-write and confirmed-byte rules without implementing file output through stdout |
| Common IR and execution | [HIR](../../src/ir/mod.rs), [MIR](../../src/mir/mod.rs), bytecode | Represent read/write-new explicitly; each of the four executors executes its own original representation |
| Generation | C, LLVM, QBE, WAT, LIR → ASM/Object | Lower the same checks, order, ownership, and stopping rules directly; Native does not go through C |

Operations in standard sources retain their own SourceId/Span; ordinary Calls retain user call sites. Preserve original HIR, non-SSA MIR, SSA, and mappings. Do not eliminate, duplicate, reorder, or constant-evaluate file operations. `check`, `compile`, `emit-*`, and `observe` do not execute program file operations; existing explicit source-import reads are separate.

## Working directory and execution connections

The host explicitly supplies and fixes the working directory before execution. An unconfigured connection does not default to the current directory. This is independent of relative imports, source locations, and standard-module lookup.

| Item | Proposal / prerequisite investigation |
| --- | --- |
| File name | Start with one component directly inside the directory. Reject empty names, dot/dot-dot, absolute paths, separators, NUL, and Windows stream/device names. Do not normalize Unicode. Finalize common forbidden characters and length limits in OS prototypes |
| Name identity | Case and Unicode comparisons also depend on the filesystem. Distinct strings are not guaranteed to identify different files; detect collisions through exclusive creation results |
| Targets | Regular files only. Reject symlinks, reparse points, directories, devices, and FIFOs as unsupported. Verify that opening a FIFO for validation does not block indefinitely |
| Resolution | Investigate directory-handle-relative opening. A string-prefix check or a separate existence check followed by ordinary open does not establish containment |
| CLI / embedding | Proposed CLI `--file-root` and explicit injection into Rust execution APIs. Existing APIs remain unconfigured. Observation APIs cannot change connections |
| Generated executables | Supplying the directory to the runtime is unresolved. Preserve existing argv semantics; do not silently embed generation-time absolute paths or obtain configuration from environment variables. Propose recording the required connection contract version in Native build/release |
| WAT | Transfer names and bytes through versioned host connections without exposing internal memory or ownership pointers. Fix import names, status values, and transfer protocol before implementation |

Linux candidates are directory-fd-relative `openat` and `O_CREAT | O_EXCL` for creation. `O_NOFOLLOW` only protects the final component; it does not establish safety for arbitrary multi-component paths. [Linux open(2)](https://man7.org/linux/man-pages/man2/open.2.html)

Investigate Windows user-mode `NtCreateFile` with RootDirectory, exclusive creation, and explicit reparse behavior. Adoption is undecided; prototype ABI use from Rust, Clang, and self-encoded objects, handle cleanup, and reparse rejection. [Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)

The directory is not an OS sandbox. Hard links, concurrent processes, network filesystems, and OS permissions are separate conditions. Initial OS comparisons use managed local temporary directories; identical results under concurrent third-party changes are not guaranteed. Reject connections that cannot enforce the required scope instead of silently relaxing it.

## Proposed read and create sequence

Evaluate arguments once, left to right, then validate limit, name, and connection in that order. Propose a byte limit of 0–2147483647, with zero allowing only empty content. Distinguish failure before any OS operation from failure after external state has changed.

| Operation | Sequence and conditions |
| --- | --- |
| read_text | Reserve the maximum string usage/storage before opening. Read to EOF; at the exact limit, request one additional byte to distinguish EOF from overflow. Consume at most limit+1. Do not determine success solely from a prior file-size query |
| Read success | EOF → close → strict UTF-8 validation. Return an owned string only if all succeed; reduce accounted usage to actual length. Each operation opens afresh rather than carrying EOF state from a prior read |
| write_new | Check content length against the limit before creation. Exclusively create, write sequentially, then close. Empty content still creates a new file |
| Partial writes | Continue from the positive accepted count. Retry only interruptions reporting zero consumption. Zero progress, excessive accepted counts, and write failure stop with distinct codes |
| Cleanup | Close once after a successful open, on success or failure. Do not blindly retry close. Retain an earlier primary failure and attach any close failure as cleanup information |
| After failure | No subsequent source operations, reopening, retransmission, or implicit deletion. The newly created file and accepted content may remain. No rollback or atomic publication guarantee |

An internal connection does not treat a short read as EOF; zero bytes means EOF. Retry reads only for zero-consumption interruptions. Limit overflow, read failure, and close failure take precedence over UTF-8 validation. For invalid responses, mark the response's consumption/acceptance unknown rather than guessing.

Success means all bytes were accepted and close succeeded, not durability after a system failure. Close retries and durability are separate concerns. [Linux close(2)](https://man7.org/linux/man-pages/man2/close.2.html) `flush();` continues to address stdout only.

Distinguish read limits, string-heap accounted usage, and physical allocation failures. As with the existing heap, accounted usage is not a physical-capacity bound. Write max_bytes bounds one operation, not total disk use or execution time. Time bounds for synchronous OS operations and recovery remain later design work; comparison tests use an outer process timeout.

## Failure and observation

Propose a new `file-v1` diagnostic schema. Preserve existing io-v1 write/flush/read-line records.

| Record field | Meaning |
| --- | --- |
| operation / phase / code | Distinguish read-text/write-new, validation/open/read/write/close, and reason |
| limit / requested / transferred | Configured limit, logical requested amount, and confirmed consumed/accepted bytes. The total read length is unknown until EOF; do not display the limit as a known total |
| progress / created / cleanup | Whether progress is complete or unknown, whether creation occurred, and close outcome. Invalid responses may also leave creation status unknown |
| NodeId / SourceId / Span | Original common-IR operation origin; startup configuration errors use origin=host |

Proposed distinctions include unconfigured, invalid name/limit, budget/allocation failure, missing/existing target, unsupported target, open/read/write/close failure, zero progress, invalid response, invalid UTF-8, and limit overflow. Fix exact codes and validation precedence in failure fixtures. Creation collisions report no change without following an existing target to classify its referent.

Do not include file content, absolute paths, or OS error text in ordinary diagnostics/records. Track operations by sequence and origin; compare content through connections/directories explicitly selected before execution. Observation retrieves immutable records only: it cannot replace input, re-execute, or change connections. Disabled/full/unallocatable recording must not change execution results. Common success-operation records are not yet implemented across generated routes even for existing I/O; distinguish this plan from delivered features.

## Implementation units and completion criteria

| Order | Work | Gate |
| --- | --- | --- |
| 1 | This design, pure name validation, and progress/failure contracts | Finalize API, name scope, stopping policy, and diagnostic fixtures without requiring an OS connection |
| 2 | Common runtime components, std:fs, HIR → MIR/SSA/bytecode, four executors | Compare results, effects, failures, ownership, and origins with independent in-memory connections; retain unconfigured defaults; add a working example |
| 3 | Windows/Linux OS connections and CLI | Verify directory-relative opening, regular-file checks, collisions, links, Unicode, and close failures; preserve existing input/output contracts |
| 4 | C, LLVM, QBE, WAT, Native | First finalize startup configuration and WAT ABI, then add the same checks/stops to each route; never report unsupported behavior as success |
| 5 | Native build/release and cross-route examples | Inspect original HIR/non-SSA MIR/SSA/LIR, generated artifacts, and origins; update the roadmap to reflect completed coverage |

Required cases include empty content, Japanese, NUL, CR/LF, combining characters, exact/+1/zero limits, invalid UTF-8, short I/O, interruption, zero progress, invalid responses, close failure after all bytes were accepted, and partial failure after creation. Also cover validation order, unconfigured access, unchanged existing files, no external operation on budget failure, independent owned copies, early return, recursion, and short circuiting.

Compare independent connections/directories with identical initial content and responses. Check request order, transferred counts, failure reasons, and residual files as well as stdout. Do not run every route sequentially against the same real directory. Separate OS connection tests from injected semantic tests and retain independent Rust/Python oracles. General record/replay or an effect system is not a prerequisite.

The immediate steps are 1 → 2. Do not silently implement unresolved OS path resolution or generated-executable startup configuration; publish the design and prototype findings before their respective stages.
