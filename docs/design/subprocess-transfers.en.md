# Subprocess transfers in flight and cancellation

[日本語](subprocess-transfers.ja.md)

This specifies the transfer portion of the [subprocess contract](subprocesses.en.md). The [OS probe](../../experiments/subprocess/README.en.md) showed that transfers on multiple streams can remain in flight when a deadline or failure occurs.

## Implementation status and responsibilities

| Component | Status and responsibility |
| --- | --- |
| [`Transfers`](../../src/runtime/process/transfers.rs) | Implemented Rust state component: transfer identity, buffer ownership, cancellation, confirmed counts, partial output, and reuse conditions |
| [Transition tests](../../src/runtime/process/transfers/tests.rs) | Inject ordering, races, and invalid responses without launching OS processes |
| `Processes::run` / `Backend` | Integrated: the common runtime controls owned transfers, deadlines, exit monitoring, cancellation, and bounded cleanup |
| Production OS connections | Both [Windows](subprocess-windows.en.md) and [Linux](subprocess-linux.en.md) integrate the common runtime; language APIs and generated routes follow |
| Language API, IR, and execution routes | Not implemented. This adds neither an execution route nor separate language semantics |

`Transfers` alone does not launch processes, read clocks, or reclaim OS resources; the integrated `Processes` requests these operations through `Backend`. The Rust API is used by the Windows/Linux OS connections but is not the public language API.

## Transfer units and ownership

Create one `Transfers` per call with a nonzero call identifier never reused within the connection. Transfer numbers increase monotonically within the call; exhaustion rejects new transfers without wrapping.

| Item | Rule |
| --- | --- |
| Concurrency | At most one operation each on stdin, stdout, and stderr. The next operation on a stream follows its previous terminal notification |
| Stdin | Send the unaccepted prefix in chunks of at most 4096 bytes. Close through a separate operation after confirming acceptance of all input |
| Stdout/stderr | Preserve each stream's byte order; no byte ordering between streams |
| Read limits | Request remaining retained capacity plus at most one probe byte, capped at 4096 per read. Count the probe as read but exclude it from returned contents |
| Before launch | Check and reserve both retained output limits plus three 4096-byte buffers. Distinguish arithmetic overflow/capacity errors from allocation failure |
| Buffer lifetime | Move a `Transfer` into the connection. The connection owns it until the OS no longer references it, then returns it with a terminal notification |
| Result lifetime | Move partial stdout/stderr into `Settlement`. The connection continues owning unfinished buffers |

There is no buffer cloning or resizing API. Moving ownership preserves the internal address. Rust types alone cannot detect the end of OS references: the connection must confirm this before returning the buffer. Capacity bounds cover requested byte buffers, not allocator overhead or physical memory guarantees including Cerune array storage.

## State transitions

| Operation | Transition / meaning |
| --- | --- |
| `prepare_*` | `Prepared`: buffer preparation only, no external transfer |
| `submit(ticket)` | Move to `Submitted` immediately before external submission; external progress is now possible |
| `withdraw(transfer)` | Reclaim only a `Prepared` buffer into `Terminal`, including after stopping; no external effect |
| `complete(transfer, n, result)` | Terminal notification for a `Submitted` operation; record confirmed progress and move to `Terminal` |
| `stop(failure)` | Preserve the first failure and block new prepare/submit operations; keep pending operations and buffers |
| `request_cancel(ticket)` | Request cancellation once for a stopped call's `Submitted` operation; repeated requests do not request another external action |
| `cancel_reply(ticket, reply)` | Record requested, too late, or failed cancellation; this is not a terminal transfer notification |

`submit` does not mean the OS accepted submission. Return `NotSubmitted` only when non-submission is certain. If submission is uncertain, retain `Submitted` while stopping, cancelling, and cleaning up. Do not convert ambiguity into non-submission.

Cancellation replies may arrive after transfer completion. Successful, partially successful, and failed terminal notifications after cancellation still count. Failed cancellation does not invalidate subsequent transfer completion.

## Terminal notifications and counts

`confirmed` is the accepted/read byte count for that operation. Accepted stdin bytes do not mean the child read those bytes.

| Terminal result | Counts and behavior |
| --- | --- |
| `Done` | Add actual progress. Zero-byte transfers fail as zero progress; a zero-byte stdin close completes |
| `Eof` | Reads only, zero bytes; mark the output stream ended |
| `Failed` | Transfer/close failure retaining confirmed partial progress |
| `Cancelled` | Requires a previous cancellation request; retain confirmed partial progress. Zero does not imply EOF |
| `NotSubmitted` | Only when no external operation began; confirmed count must be zero |
| `ProgressUnknown` | Retain a confirmed lower bound and content prefix. Further progress is unknown; do not reuse the connection |

Counts exceeding the request, foreign-call/stale/duplicate responses, invalid EOF, and unsolicited cancellation notifications violate the contract. Do not double-count; disable reuse. Valid terminal notifications on other streams can still be collected.

`Progress.known` means the reported count is known as a total. It is false while a `Submitted` operation remains. A valid known terminal notification restores it; `ProgressUnknown` or invalid counts leave it false. Missing notifications never establish a confirmed zero count.

## Deadlines, cleanup, and later calls

The caller checks deadlines. Upon observing the deadline or later, call `stop(Timeout)` first, then account for counts and contents in terminal notifications received afterward. Subsequent transfer or cleanup failures do not replace the first failure. This component never reads a clock.

After bounded cleanup, `finish(cleanup)` settles the result. Keep the connection's cleanup report unchanged and set `reusable` only when all conditions hold:

- No unfinished or unreclaimed prepared operation remains.
- All stream counts are known.
- No protocol violation occurred.
- The connection reports completion of managed-scope, child, and handle cleanup.

Failure-free transfers additionally require stdin close and both output EOFs. The common runtime checks child exit status outside this component. After failure, reuse can be allowed without EOF if progress and reclamation are certain.

Pending operations prevent reuse even if the connection reports `Cleanup::Complete`. Preserve pending identifiers and unknown counts in the result. The connection retains unfinished buffers and OS management state. `finish` does not release them, wait implicitly, or retry.

## Observation and verification

`snapshot()` copies call identity, NodeId/Span/SourceId, limits, operation identities, offered/confirmed/retained counts, states, cancellation replies, terminal results, and the first failure. It includes no contents and performs no allocation, clock reading, or I/O. Observation is separate from execution APIs. `Transfers` retains only the latest operation per stream. The common runtime saves bounded snapshots at each stage in `events()`; disabling, exhausting, or failing to allocate history does not change execution. `Outcome.transfers` retains final state independently of history settings.

Tests cover all six completion orders across three streams, three cancellation replies racing with terminal outcomes, late cancellation replies, missing notifications, partial input failure, zero/exact/exceeded limits, zero progress versus EOF, unknown progress, withdrawal before submission, duplicate/foreign/stale responses, budgets, cleanup, and observation enabled/disabled.

## Integration with the common runtime

`Processes::run` connects this component to launch, deadlines, exit monitoring, and cleanup. It never reuses call identities and submits the next operation only after that stream's previous operation terminates. Advance stdin by confirmed acceptance only, then submit close after all input is accepted.

The connection owns buffers through `submit` and returns fixed-capacity `Batch` values from `poll(deadline)`. A batch contains at most three terminal transfers, three cancellation replies, one exit status, and one cleanup report. Simultaneous notifications follow this order:

1. Clock regression, then deadline expiry. Record failure first, without discarding received notifications.
2. Terminal stdin, stdout, then stderr transfers. Inspect invalid multiple notifications on one stream in identifier order.
3. Cancellation replies, including replies after the associated terminal transfer.
4. Child exit status. Duplicate exits and signal zero violate the contract.
5. Cleanup reports. Reports before cleanup begins violate the contract.

Empty ordinary batches and premature `Deadline` notifications are also invalid. `Interrupted` permits another wait. Array positions do not define operation order; collecting returned contents is distinct from performing external effects.

After failure, submit no new transfers. Request cancellation once per remaining operation and call `begin_cleanup` once. Successful calls also reclaim their managed scope. Control operations do not wait; cleanup polling uses a separate finite deadline. Account for partial results returned at or beyond that deadline before stopping.

`Report.cleanup_reported` retains the connection's raw report; `Report.cleanup` records the clock/deadline-aware verdict. Late complete reports retain reclaimed-resource flags but fail the deadline and prevent reuse. Missing cancellation replies also violate the contract and prevent reuse even if all transfers terminated.

Keep the old synchronous model only under `cfg(test)` to compare shared cases. This migration changes the Rust `Backend` contract; language CLI and generated-route contracts have not yet been introduced. Windows OS launch now receives `CallContext`, retaining call identity/origin even for failures before transfers.

## Next integration steps

1. Both [Windows](subprocess-windows.en.md) and [Linux](subprocess-linux.en.md) OS connections are implemented. [Public data types](subprocess-values.en.md) are also available.
2. Next design result storage and connect the operation through common IR and execution routes, preserving registration, capacity, cleanup, and observation.

Cross-route language support remains incomplete. Do not add unimplemented `.ceru` APIs to the executable example catalog.
