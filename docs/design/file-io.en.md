# Limited file I/O

[日本語](file-io.ja.md)

**Status: std:fs, common IR, and HIR/MIR/SSA/VM execution implemented. Windows/Linux OS connections and CLI configuration are also implemented. Generated C/LLVM OS connections and startup configuration are also implemented. QBE/WAT/Native remain unimplemented.** On 2026-10-26, we agreed to start with bounded whole-file UTF-8 reads and new-file creation in an explicit working directory. Replacement is outside the first version. The generated-executable startup format is agreed. WAT connections remain proposals. This covers saving/reloading in [#89](https://github.com/Cerune-Lang/Cerune/issues/89) and [#173](https://github.com/Cerune-Lang/Cerune/issues/173), isolating comparison effects according to [#104](https://github.com/Cerune-Lang/Cerune/issues/104).

## Implemented common components

[runtime::files](../../src/runtime/files.rs) provides `Files<B>` with a Backend selected before execution. All four executors connect `std:fs` operations to this component. `Directory::open(&Path)` connects an explicitly selected directory on Windows/Linux. Existing input, output, and argument APIs remain available.

| Item | Implemented contract |
| --- | --- |
| Reading | `read_text(name, max_bytes, available_bytes, origin)`: limit 0–2147483647, reserve before open, transfer in units of at most 4096 bytes, use a separate one-byte probe after the limit; validate UTF-8 after EOF and successful close |
| Creation | `write_new(name, text, max_bytes, origin)`: check length before creation, create even for empty content, continue short writes from the remaining bytes, retry only interruption |
| Connection | Backend owns the fixed scope, regular-file checks, and exclusive creation. Returned Handles start at offset zero. Ordinary open errors guarantee no creation/change; indeterminate responses use InvalidResponse. Explicitly inject the running OS connection; never infer a generated target from the host OS |
| Close | Consume the Handle once. Preserve an earlier primary failure and record close failure as cleanup=failed. No retry, deletion, or retransmission. Host panics are outside this result contract |
| Stopping | Retain the first Failure; subsequent calls return it without revalidation, open, transfer, close, or additional recording. Keep internal transfer outcomes separate from the stopping policy |
| Ownership | available_bytes is the caller's remaining string-usage allowance. Support ownership preparation before open, discard it on failure, and return it with the string on success. The caller manages returned lifetimes/usage; capacity compaction is unimplemented |
| Observation | `with_trace_limit`, `events()`, `trace_truncated()`, `failure()`: no content, names, or OS error text. Disabled/full/unallocatable recording does not change execution or transfer results. into_inner returns the connection without further operations |

The common name rule accepts one direct component of 1–255 UTF-8 bytes. Reject NUL–U+001F, U+007F, `< > : " / \ | ? *`, trailing ASCII space/period, dot, and dot-dot. Preserve leading periods, internal spaces, Japanese, and combining characters.

Ignore ASCII case when rejecting CON/PRN/AUX/NUL/CONIN$/CONOUT$ and COM/LPT followed by 1–9 or ¹/²/³. Apply the same check to the portion before the first period after trimming trailing ASCII spaces, rejecting extensions too. This comparison does not transform accepted names. The [Windows naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) inform this OS-independent initial name set; it is not a guarantee of creation on every filesystem or of directory containment.

A prototype created and read back eight names in new temporary directories on Windows and Linux /tmp: Japanese, composed/combining text, supplementary characters, hidden names, spaces, and the 255-byte boundary. That initial prototype checked the name set. Verification of the implemented directory-relative opening is described below.

Failure holds a code and Details; Event adds a sequence number and success/failure. Counts describe consumption/acceptance confirmed by the connection, not durable storage. requested is unknown until read EOF, and is the content length for writes. For invalid transfer responses, progress=unknown and transferred retains only the previously confirmed count. An invalid open response with uncertain creation uses created=unknown.

```text
file-v1 operation=write-new phase=write code=write-failed limit=3 requested=3 transferred=1 progress=known created=yes cleanup=closed node=7 bytes=10..22
```

phase is validate/reserve/open/read/write/close/decode; created is no/yes/unknown; cleanup is not-attempted/closed/failed. Omit file for anonymous SourceId, otherwise include `file=N`. This Rust component always receives an operation Origin; startup connection diagnostics use origin=host separately.

[Tests](../../src/runtime/files/tests.rs) use independent in-memory connections to check every UTF-8 split, 4096-byte boundaries, failure during the limit probe, close precedence, existing/partial files, provenance/privacy, ownership preparation cleanup/transfer, and observation noninterference on Windows/Linux.

```sh
cargo test --lib runtime::files
```

## Public API and executor connections

Save generated text to a new file, read it back, and compare it with the original value. Preserve UTF-8, NUL, CR/LF, and combining characters exactly; do not normalize, translate newlines, or remove a BOM.

| Public API | Success | Failure |
| --- | --- | --- |
| `fs::read_text(name: string, max_bytes: i64) -> string` | Read from the opened file's beginning to EOF; an empty file returns an empty string | Stop on missing files, limit overflow, invalid UTF-8, read/close failures, etc. |
| `fs::write_new(name: string, text: string, max_bytes: i64) -> void` | Create a new file, transfer all bytes, and close it | Leave an existing target unchanged; stop on creation/write/close failures |

Explicitly import the reserved virtual standard module with `import "std:fs" as fs;`. The first version does not add a Result type, public file handles, seek, append, deletion, replacement, directory creation, or subprocesses. Separate the public stop-on-failure policy from internal operations returning progress, leaving room for a later recovery API.

The API below is implemented. [roundtrip.ceru](../../examples/files/roundtrip.ceru) compares all four executors with an in-memory connection, checking equality and the 16-byte length.

```cerune
import "std:fs" as fs;
text: string = "日本語\0\r\né\n";
fs::write_new("sample.txt", text, 4096);
saved: string = fs::read_text("sample.txt", 4096);
print(saved == text);
```

Expect `true` plus LF and file bytes exactly matching the original string. Give each execution an empty directory; a second execution against the same directory must stop without changing the existing file.

The executor Rust APIs are as follows.

| API | Connections and result |
| --- | --- |
| `run_with_files(program, &mut Files<B>)` | Explicit file connection; capture stdout as a returned string; stdin/arguments remain unconfigured |
| `run_with_io_files_arguments_and_call_limit` | Explicit stdin, stdout, files, arguments, and call depth in one execution |
| Existing `run` APIs | Files remain unconfigured; stop with a `file-v1` diagnostic with `code=unconfigured` when used. Never infer a directory from cwd/environment |

HIR `FileReadText` returns a string; `FileWriteNew` is a void statement. MIR/SSA/bytecode retain `file.read_text.utf8` and `file.write_new.utf8`. Evaluate arguments left to right once and preserve operation NodeId/Span in the standard source. Transfer read results into StringHeap, reserving before open and releasing on failure. Borrow file names and write content without an extra content copy for the operation. Count `std:fs` source-function calls toward call depth, excluding generated helpers.

[Cross-route tests](../../tests/file_io/mod.rs) compare the example, evaluation order, short circuits, ownership, composed input/output/arguments, partial transfers, failures, and origins with independent in-memory connections. Generated C/LLVM support Windows/Linux reads and creation. QBE/WAT/Native reject file operations before generation with source locations, including unused functions. `emit-ir`/`emit-mir`/`emit-bytecode` remain available without executing operations. CLI `--file-root <directory>` supplies an explicit real-filesystem connection.

```sh
cargo test --test standard_input file_io
```

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

| Item | Current contract and remaining design |
| --- | --- |
| File name | Start with one component directly inside the directory. Reject empty names, dot/dot-dot, absolute paths, separators, NUL, and Windows stream/device names. Do not normalize Unicode. Common forbidden characters and the 255 UTF-8 byte limit are implemented as described above |
| Name identity | Case and Unicode comparisons also depend on the filesystem. Distinct strings are not guaranteed to identify different files; detect collisions through exclusive creation results |
| Targets | Regular files only. Reject symlinks, reparse points, directories, devices, and FIFOs as unsupported. Verify that opening a FIFO for validation does not block indefinitely |
| Resolution | Directory-handle-relative opening is implemented. A string-prefix check or a separate existence check followed by ordinary open does not establish containment |
| CLI / embedding | Implemented CLI `--file-root` and explicit injection into Rust execution APIs. Existing APIs remain unconfigured. Observation APIs cannot change connections |
| Generated executables | The leading-marker startup configuration is agreed and implemented in C/LLVM. Preserve marker-free argv semantics; do not silently embed generation-time absolute paths or obtain configuration from environment variables. Propose recording the required connection contract version in Native build/release |
| WAT | Transfer names and bytes through versioned host connections without exposing internal memory or ownership pointers. Fix import names, status values, and transfer protocol before implementation |

Linux uses directory-fd-relative `openat` and `O_CREAT | O_EXCL` for creation. `O_NOFOLLOW` only protects the final component; it does not establish safety for arbitrary multi-component paths. [Linux open(2)](https://man7.org/linux/man-pages/man2/open.2.html)

Windows uses user-mode `NtCreateFile` with RootDirectory, exclusive creation, and no-reparse-following options in the Rust connection. Generated C/LLVM also use this. QBE/custom Object integration remain later work. [Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)

The directory is not an OS sandbox. Hard links, concurrent processes, network filesystems, and OS permissions are separate conditions. Initial OS comparisons use managed local temporary directories; identical results under concurrent third-party changes are not guaranteed. Reject connections that cannot enforce the required scope instead of silently relaxing it.

## Common read and create sequence

Language integration evaluates arguments once, left to right. The common component validates the supplied limit, name, and connection in that order. The byte limit is 0–2147483647, with zero allowing only empty content. Distinguish failure before any OS operation from failure after external state has changed.

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

The Rust component implements the new `file-v1` diagnostic schema. Preserve existing io-v1 write/flush/read-line records.

| Record field | Meaning |
| --- | --- |
| operation / phase / code | Distinguish read-text/write-new, validation/open/read/write/close, and reason |
| limit / requested / transferred | Configured limit, logical requested amount, and confirmed consumed/accepted bytes. The total read length is unknown until EOF; do not display the limit as a known total |
| progress / created / cleanup | Whether progress is complete or unknown, whether creation occurred, and close outcome. Invalid responses may also leave creation status unknown |
| NodeId / SourceId / Span | Original common-IR operation origin; startup configuration errors use origin=host |

Implemented distinctions include unconfigured, invalid name/limit, budget/allocation failure, missing/existing target, unsupported target, open/read/write/close failure, zero progress, invalid response, invalid UTF-8, and limit overflow. Common-component failure tests fix the codes and validation precedence. Creation collisions report no change without following an existing target to classify its referent.

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

Steps 1–3 and C/LLVM in step 4 are implemented. Semantics are compared using controlled connections, and Windows/Linux OS connections/startup are verified. QBE is next, followed by the WAT contract and Native integration. Publish designs and prototype findings before those stages.

## Windows/Linux connections and CLI

[Directory](../../src/runtime/files/directory.rs) retains the directory opened before execution. Renaming or replacing its original path does not redirect later operations. Reject a link/reparse point at the root's final component. Parent components of the explicitly selected root resolve according to the OS; this does not prohibit every parent link or isolate the whole OS.

| OS | Reading | Creation |
| --- | --- | --- |
| Linux | Pin with O_PATH, O_NOFOLLOW, and O_CLOEXEC; verify a regular file, then reopen the retained fd through /proc/self/fd for reading. Verify matching inode/device | openat with O_CREAT, O_EXCL, O_NOFOLLOW; mode 0666 restricted by umask; never write an existing target |
| Windows | NtCreateFile with RootDirectory and FILE_OPEN_REPARSE_POINT; require a non-directory disk handle without reparse attributes | Exclusive FILE_CREATE; allow read/write/delete sharing and inherit default security attributes |

Linux reads require accessible procfs. A failed reopen never falls back to opening the original name. FIFOs/devices are not opened for data I/O before regular-file verification. [proc_pid_fd(5)](https://man7.org/linux/man-pages/man5/proc_pid_fd.5.html) Windows checks attributes using [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex). Consume handle ownership and call close exactly once, without retry on failure. Drop the root handle when the connection is dropped.

The following CLI commands use directories prepared beforehand.

~~~sh
cerune run examples/files/roundtrip.ceru --file-root sample-output
cerune run-mir examples/files/roundtrip.ceru --ssa --file-root another-empty-output
~~~

Supported commands are run/run-ir/run-mir (including SSA)/run-vm. Resolve an explicit relative root against cwd at connection time, then use the handle. Omission means unconfigured. Reject duplicates/missing values before execution. A --file-root after the argument delimiter is a program argument and does not configure files. The option is not added to emit/check/build/release.

Missing/invalid/unsupported roots and open failures stop before execution with file-v1 operation=connect phase=open-root code=... origin=host, without inventing an operation NodeId. Program operations retain their NodeId/Span. Do not add absolute root paths or OS error text to diagnostics. Connections outside Windows/Linux return unsupported-platform.

[OS tests](../../tests/file_io/os.rs) use independent temporary directories to compare the 16-byte example, empty/exact/over-limit reads, invalid UTF-8, prior output, and failure origins across four executors. They also verify competing creation, root rename/replacement, directories, links, Windows junctions, and Linux FIFOs. Child processes have a 20-second limit. Partial transfers, interruptions, and close failures are injected in common-component tests, not claimed as actual OS close-failure tests. Linux special-file tests use Linux temporary storage because WSL Windows drives do not support FIFO creation.


## Generated-executable startup contract (C/LLVM implemented)

On 2026-10-27 we agreed to interpret runtime configuration only when the dedicated marker is the first argument.

~~~sh
program --cerune-runtime-v1 --file-root output -- user-argument
program --cerune-runtime-v1 -- --cerune-runtime-v1
~~~

- Without the marker, preserve all arguments as before and leave files unconfigured.
- With the marker, require a terminating --. Before it accept only --file-root and its value; reject duplicates, missing/empty values, and unknown settings at startup. Omitting root means unconfigured.
- After --, preserve empty strings, the marker itself, and option-like strings as program arguments. Configuration is excluded from snapshot count/byte limits.
- Validate all settings before connecting root. Existence and scope are enforced by the OS connection, not the argument parser. Do not supply implicit cwd/environment defaults.
- Initially only generated programs containing file operations use this startup contract. Preserve startup/argv behavior of existing programs without file operations.

C/LLVM are connected and compared across Windows/Linux, unoptimized/optimized builds, and four executors in independent directories. QBE/Native will adopt the same startup contract in later changes. WAT will use a separate versioned host contract rather than this argv marker. This section does not claim completed support for other generated routes.


## Generated C validation and comparisons

The [generated C runtime](../../src/codegen/c/files.rs) exposes name/limit checks, reservations, transfers, close, and UTF-8 validation, separating [OS connections](../../src/codegen/c/files/connection.rs) and [startup parsing](../../src/codegen/c/files/entry.rs). Startup proceeds through configuration validation, program-argument validation/snapshot, root connection, then source execution. Configuration errors use file-v1 phase=configure; root errors use phase=open-root, both with origin=host. Argument snapshot failures retain arguments-v1.

Configuration codes are unknown-setting, duplicate-setting, invalid-root, and missing-delimiter. Source operations with omitted configuration report unconfigured. Select connections using the C compilation target's _WIN32/__linux__, never the emitter's OS. Other OS connections report unsupported-platform. Linux reads require /proc/self/fd, as in the Rust connection.

Reserve the read limit before open and transfer returned values into existing string ownership. Expand write arguments into ordered locals rather than relying on C argument evaluation order. Retain original NodeId/Span for each operation. Flush prior output before diagnosing and exit without executing later source operations. Emission itself never opens program files.

[Real-connection comparisons](../../tests/file_io/generated.rs) and [injected-response tests](../../tests/file_io/c.rs) cover O0/O2 examples, UTF-8/limits, existing files, names, usage limits, unconfigured access, startup settings, arguments, order, short circuits, recursion, and independent copies. Inject short transfers, interruptions, zero progress, invalid counts, transfer failures, and close failures into independent connections; compare full file-v1 records, prior output, and remaining content with four executors. Reject Windows junctions and Linux links/FIFOs. Linux comparisons also run with AddressSanitizer/UndefinedBehaviorSanitizer. This does not add a common successful-operation observation API to C.

## LLVM semantics and OS connections

LLVM lowering, name validation, transfers, ownership, and file-v1 diagnostics are implemented. The [comparison tests](../../src/codegen/llvm/files/tests.rs) lower the existing roundtrip.ceru directly to LLVM and compare independent test connections with all four executors. Execution does not go through generated C or the VM.

**emit-llvm generates real connections for explicitly selected x86_64-unknown-linux-gnu / x86_64-pc-windows-msvc targets.** [OS connections](../../src/codegen/llvm/files/connection.rs) and [startup](../../src/codegen/llvm/files/entry.rs) are separate from language semantics. The emitter host OS does not select the connection. Generated programs containing file operations, including unused functions, connect startup settings to the existing argument snapshot. Configuration validation → argument validation → root connection → execution, and exclusion of settings from argument limits, match generated C.

The internal connection functions have only the following responsibilities. This is not a stable external ABI declaration.

| Connection function | Response and responsibility |
| --- | --- |
| configured | Whether a connection exists; do not fill in omitted configuration from cwd or environment |
| open | Accept UTF-8 name bytes, length, and read/write selection. 0 means success; 1–4 mean missing, existing, unsupported target, or open failure; other values are invalid responses. Return an owned handle only on success |
| read / write | Return the transferred byte count. -2 means interruption without consumption; other negative values mean failure. Counts exceeding the request leave progress unknown. Generated LLVM decides whether to retry or stop |
| close | Consume the handle once. 0 means success; other values mean failure. Preserve any earlier primary failure |

Reads request at most 4096 bytes and use separate storage for the one-byte probe at the limit. Writes request the entire remainder. requested stays unknown until EOF; UTF-8 validation follows successful close. Read results enter existing LLVM string ownership and are released after copying, recursion, and iteration. Arguments are evaluated once from left to right, preserving NodeId/SourceId/Span.

Comparisons include request sequences, close counts, retained content, prior output, and complete file-v1 records. Cases include short transfers, interruption, zero progress, invalid responses, transfer failures combined with close failures, empty content, exact/over limits, transfers crossing 4096 bytes, invalid UTF-8, missing configuration, name rules, and usage limits. Injected LLVM allocation failure also verifies stopping before open. Verification covers O0/O2, object generation for both targets, and execution on Windows/Linux. Successful completion additionally checks that string usage and the owner list are empty.

Test-only C code supplies fixed bytes, response sequences, and test logs. It does not implement name semantics, UTF-8 validation, budgets, diagnostics, or retries. Real OS connections are verified separately from these injected responses.

### LLVM OS connections and verification scope

OS operations remain visible in generated LLVM. Windows explicitly resolves NtCreateFile from ntdll, opens the UTF-16 root, then opens children relative to its handle. Linux uses the openat and /proc/self/fd sequence above. Target ABI layouts, including x86-64 GNU stat and Win64 NT structures, are fixed in the connection module and are not reused for other ABIs.

[Shared real-connection tests](../../tests/file_io/generated.rs) apply to C and LLVM. They compare O0/O2 save/readback, existing-file protection, Unicode names and empty files, missing/duplicate/invalid startup settings, and argument limits excluding configuration. They also verify that excess argument counts and invalid Unicode stop before root connection; moving/replacing root while waiting on stdin still accesses the retained directory; and Windows junctions / Linux links and FIFOs are rejected. Read/write/close failure injection continues through independent test connections.

Normal completion releases the root handle and argument snapshot. Root release ends the connection lifetime and is separate from close diagnostics for source read/write operations. As with existing I/O, generated routes do not yet expose the common successful-operation observation API. QBE/WAT/Native file support and recording the connection contract in Native build/release remain follow-up work.
