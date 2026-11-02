# Windows subprocess Backend

[日本語](subprocess-windows.ja.md)

The Windows OS connection now integrates with the [common runtime](subprocesses.en.md) through Rust `Processes<Windows>`. **Language `std:process`, IR instructions, CLI, and generated-route integration remain unimplemented.** Registration and numeric limits below belong to this initial Rust interface, not a public language contract.

## Implementation and responsibilities

| Component | Responsibility |
| --- | --- |
| `Processes` / `Transfers` | Requests/capacity/common deadlines, call/operation identity, origins, accepted counts, first failure, cancellation, cleanup verdict |
| [Windows](../../src/runtime/process/windows.rs) | Explicit registration, OS notification conversion, bounded waiting, resource ownership, read-only OS records |
| [scope](../../src/runtime/process/windows/scope.rs) | Create a suspended child already in its Job with explicit paths/environment, resume after deadline checks, inspect child/Job activity |
| [pipe](../../src/runtime/process/windows/pipe.rs) | Asynchronous parent and synchronous child stdio; directly use the owned Transfer buffer and retain fixed-address OVERLAPPED/event until termination |
| [Execution verification](../../experiments/subprocess/windows_backend.rs) | Run fixtures through the production-code Backend and inspect common results, observations, and cleanup |

The Backend does not redefine launch, transfer, cancellation, exit, or cleanup semantics. `submit`, `cancel`, and `begin_cleanup` do not wait; `poll` waits within the common runtime's absolute deadline. Initial waiting checks nonblocking status at intervals of at most 1 ms. More efficient waiting and performance comparisons remain later work.

## Registration and launch

`Windows::new(Vec<Target>, trace_limit)` receives names, absolute executable paths, absolute working directories, and explicit environments. Names are not paths; there is no PATH search, bulk parent-environment inheritance, or cwd completion. Environment names use Windows case-insensitive ordinal comparison for duplicate checking/sorting; values are not normalized.

The initial implementation checks these capacities before launch.

| Item | Bound |
| --- | --- |
| Registration | 64 targets, 256 UTF-8 bytes per name, 128 environment pairs per target |
| Registration total | 1 MiB of UTF-8 names/environment plus UTF-16 path bytes |
| Paths | 32766 UTF-16 code units each, excluding terminal NUL |
| Environment block / quoted command line | 32767 UTF-16 code units including terminal NUL |
| Call arguments | 4096 entries / 16384 UTF-8 bytes total; common-runtime limits also apply |
| OS records | Explicit 0–4096 entries; zero disables recording |

Pipe names include 128 random bits; enforce first-instance creation, owner/system ACLs, and remote rejection. Only three stdio handles are inherited. Failures before target creation produce `NotStarted`; resume failure or deadline expiry after child creation retains resources and produces `Uncertain`. This does not guarantee preemption inside OS launch APIs.

## Transfers, cancellation, and failures

| OS result | Common-runtime notification |
| --- | --- |
| Success | Confirmed count and `Done`, including short transfers |
| Broken pipe on read | `Eof`, zero bytes |
| Zero-byte success | `Done`, not EOF; the common runtime reports zero progress |
| Cancellation requested / too late / failed | `Requested` / `TooLate` / `Failed`, separately from termination |
| Terminal cancellation/failure with unknown total | `ProgressUnknown`; retain confirmed lower bounds, not a fabricated exact zero |
| Unexpected completion-query failure | Retain the buffer without claiming termination; record the OS error and follow the common deadline |
| Deadline before submission / event allocation failure | `NotSubmitted`; confirm no OS reference to the buffer |

Normal completion can follow cancellation. Nonzero exit codes are values, not operation failures. Success requires stdin closure, both output EOFs, and direct-child termination. Cleanup terminates the Job when necessary, then confirms child termination, zero active processes, all terminal transfers, and handle closure.

`Backend::start` now receives `CallContext { call, origin }`. OS records retain call identity and NodeId/Span even for launch failures, plus Ticket for transfers. This changes the Rust connection contract, not language syntax or CLI. Common/OS records do not automatically include content, arguments, environment, or absolute paths. Recording limits/allocation failures do not change execution control.

## Ownership of unreclaimed resources

Deadline expiry, unknown progress, or incomplete cleanup prevents common-runtime connection reuse. Reclamation and exact progress are distinct: closing handles does not turn unknown progress into success.

1. `Processes::into_inner()` returns Backend ownership.
2. `has_unreclaimed_resources()` and `events()` are read-only.
3. `reclaim(timeout)` explicitly controls cancellation, termination, and bounded reclamation without changing past Outcomes.
4. Final-owner Drop requests termination/cancellation and checks completion once without creating a waiting thread. If still unresolved, retain buffers, OVERLAPPEDs, events, handles, and Job state until process exit and increment `retained_connections()`.

This exceptional retention avoids use-after-free; it is not automatic recovery or successful reclamation. Repeated retention consumes resources. Prefer common-runtime cleanup and explicit `reclaim` when necessary. Recovery is not guaranteed for Rust allocator failure, OS resource exhaustion, descendants leaving the managed scope, or arbitrary external interference.

## Verification and next work

Run `cargo run --example subprocess_windows_backend -- target/subprocess-windows-backend` with a new output directory to save JSONL. Eighteen cases verify argv/environment/origins, NUL/CR/LF, concurrent 128 KiB transfers, nonzero exits, exact-limit EOF, overflow, timeout, descendant reclamation, early close, continued execution after unregistered/missing targets, bounded observation, submission deadlines, and explicit reclamation. Separate OS runs can produce different notification order/operation identities; observation comparisons check results, output, and failure reasons.

A unit test injects completion-query failure and verifies pending-buffer/final-owner retention. It intentionally retains about 4 KiB plus an event and related resources until the test process exits.

The [Linux Backend](subprocess-linux.en.md) is integrated too. Next add public language types, standard modules, and common IR.

OS references: [CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex), [GetOverlappedResult](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult), [ReadFile pipe behavior](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile), and [CompareStringOrdinal](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal).
