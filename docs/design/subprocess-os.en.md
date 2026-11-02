# Subprocess OS connections

[日本語](subprocess-os.ja.md)

This designs OS connections for the [common runtime](subprocesses.en.md) and [transfer contract](subprocess-transfers.en.md). As of 2026-11-02, the common runtime and independent OS probes exist; **production OS Backends and the language API remain unimplemented**.

## Status and selected architecture

| Area | Status |
| --- | --- |
| Common runtime | Scripted tests cover launch, three-stream transfers, deadlines, cancellation, exit monitoring, and cleanup |
| Independent OS probes | Nine cases on both OSes plus measured Linux supervisor isolation |
| Connection design | Use Jobs and asynchronous I/O on Windows; a dedicated supervisor and caller-side nonblocking I/O on Linux |
| Next verification | Linux control transport, descriptor transfer, SIGPIPE; Windows OVERLAPPED lifetime, cancellation, inherited handles |
| Later | Production Backends, registration, standard module/IR, routes/artifacts, language examples |

Keep the common runtime as the sole owner of semantic decisions. Do not put Cerune IR/VM execution or a second timeout semantics in the Linux supervisor. Its responsibilities are OS parentage and resource reclamation.

| Responsibility | Common runtime | Windows connection | Linux connection and supervisor |
| --- | --- | --- | --- |
| Requests, limits, origin, first failure | Decide and retain | Convert to OS representations | Convert to OS representations |
| Buffers and completion | Transfer ownership, reclaim at termination | Retain per OVERLAPPED operation | Caller-side connection owns descriptors and buffers |
| Child and managed scope | Interpret launch/cleanup reports | Manage Job and direct child | Supervisor manages direct child and its process group |
| Cancellation | Request once; distinguish reply from termination | Separate CancelIoEx from completion | Remove waiting operations; preserve completed progress |
| Observation | Read-only history/final state | Separate from execution controls | Do not expose control IPC through observation APIs |

## Explicit connection configuration

Before launch, provide a registration table mapping target names to absolute executable paths, absolute working directories, and explicit environments. Do not search PATH, inherit the entire parent environment, depend on the caller's current directory, or interpret target names as paths.

Before launch, validate NUL in arguments/environment, empty/invalid environment names, duplicate names, lengths after OS encoding, and arithmetic overflow. Connections handle Windows environment-name comparison and UTF-16 conversion, or Linux bytes and descriptors. Bound registry, OS metadata, and IPC storage separately from common-runtime transfer buffers.

Explicitly configure the Linux supervisor path and protocol version. A missing or incompatible supervisor must not launch user code. Match artifact/connection targets and capabilities rather than silently choosing a generation target from the executing OS. Public configuration type names, CLI syntax, and numeric limits remain undecided.

## Linux: a dedicated supervisor

Subreaper configuration changes process-wide orphan adoption and reaping responsibility. Do not apply it directly to an embedding host. [Linux man-pages](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html)

Initially use **one supervisor per call**, without reuse or a resident daemon. The caller waits only for the supervisor as its direct child and does not reap unrelated children with `waitpid(-1)`. Do not change global SIGCHLD handlers or the caller's process group.

The intended sequence is:

1. Launch the explicitly configured supervisor; check version, request identity, and capacity before launching user code.
2. Establish the supervisor's subreaper state/control connection and launch the target in a dedicated process group.
3. Transfer the parent ends of target stdin/stdout/stderr to the caller; close unnecessary supervisor duplicates.
4. The caller performs nonblocking I/O. The supervisor reports direct-child exit status but retains the leader unreaped until group termination is requested.
5. Reap the direct child and descendants in the managed group after termination. The caller confirms terminal transfers, descriptor closure, supervisor reports, and termination/reaping of the supervisor itself.

Use `WNOWAIT` to separate status observation from reaping and avoid signaling a reused numeric PID. [waitpid/waitid](https://man7.org/linux/man-pages/man2/waitpid.2.html)

A control socketpair with SCM_RIGHTS descriptor transfer is the candidate transport. It keeps data out of the supervisor control channel and lets OS byte counts reach the common runtime directly. [Unix domain sockets](https://man7.org/linux/man-pages/man7/unix.7.html) Final framing and implementation follow in the next probe.

| Transport/descriptor requirement | Verification |
| --- | --- |
| Version, request ID, length | Reject mismatch, duplication, disconnect, and oversized frames; validate length before variable allocation |
| Descriptor transfer | Exactly three descriptors with fixed directions/roles; reject extras/truncated ancillary data without leaking already received descriptors |
| Descriptor lifetime | Set close-on-exec on received descriptors; inherit no unnecessary host descriptors; close duplicates that would prevent EOF |
| SIGPIPE | Writing closed stdin must not terminate the host; verify a solution without changing process-wide signal configuration |
| Waiting | Do not interpret EAGAIN as zero-progress failure; distinguish interruption from accepted bytes and return for common-runtime deadline checks |

Supervisor watchdogs assist resource reclamation. The caller's common runtime decides program success, failure, and deadlines; communication or repeated waits must not extend the operation deadline.

## Windows: Jobs and asynchronous I/O

Establish Job membership before target execution and explicitly select inherited handles. Do not inherit the Job/control events or permit breakaway. Build on the existing Job List and suspended-launch/resume probe. [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

Move connection-side pipes to overlapped I/O rather than copying the blocking workers into production. Each operation gets its own OVERLAPPED and event. Do not move, reuse, or release OS-referenced structures/buffers. [Overlapped pipe I/O](https://learn.microsoft.com/en-us/windows/win32/ipc/synchronous-and-overlapped-input-and-output)

`CancelIoEx` requests cancellation without waiting for transfer completion. Normal completion or another error may still follow, so correlate cancellation replies separately from terminal results. [CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)

The next probe checks pipe creation/access restrictions, fixed OVERLAPPED addresses, short transfers, cancellation races, duplicate-completion prevention, outstanding I/O after Job termination, and handle closure. Target-facing standard I/O must support ordinary reads/writes.

## Failure and outstanding resources

| Situation | Common-runtime treatment |
| --- | --- |
| Confirmed absence of target and held resources | `NotStarted`; distinguish unregistered, missing, denied, and other causes |
| Uncertain target launch or reclamation | `Uncertain`; do not hide unreclaimed resources even if only the supervisor launched |
| Transfer failure with confirmed progress | Include partial counts/content in the terminal notification |
| Unknown additional progress | Retain `ProgressUnknown` or the pending operation; never turn it into zero-byte success |
| Supervisor disconnect/crash | No automatic restart; distinguish known and unknown launch/progress/reclamation |
| Missing termination, cancellation replies, or cleanup | Prevent reuse; retain the connection report separately from the common-runtime verdict |

On control-channel closure, the supervisor should enter bounded cleanup. This does not guarantee reclamation after forced supervisor termination, kernel-stalled operations, or descendants escaping the managed scope. After supervisor loss, do not signal numeric PIDs/PGIDs whose identities cannot be established.

Connection Drop must not free OS-referenced buffers. Before publishing a production Backend, prototype explicit return of unreclaimed ownership and retention when its final owner is dropped. Do not use unbounded join, detach-as-success, or handle closure alone as proof of completion.

## Distribution and observation

Distribute the Linux supervisor as a Cerune-owned auxiliary artifact. Record target, protocol version, and required capabilities in manifests; use explicitly configured placement. Do not silently relaunch the ordinary `cerune` CLI or fall back to a namesake on PATH. Adding the actual binary and build/release packaging remains later work.

Observation retains call/operation identity, NodeId/Span/SourceId, OS phase, offered/accepted/retained bytes, exit status, and cleanup evidence. Do not automatically publish arguments, environment, contents, or local absolute paths in diagnostics. Keep control IPC separate from observation; compile/check/emit/observation must not launch children.

## Measurements and next milestone

On 2026-11-02, the [Linux isolation probe](../../experiments/subprocess/isolation.rs) ran the existing nine cases in a dedicated worker, then verified a tenth `supervisor-isolation` case:

- Supervisor subreaper state changed 0→1; caller state remained 0→0.
- Caller process group and SIGCHLD configuration were preserved.
- The supervisor reaped a grandchild; the caller reaped the supervisor.
- The caller recovered exit status 255 from an unrelated child.

This verifies **supervisor placement** only. It does not verify descriptor transfer, versioned control messages, cleanup on supervisor disconnection, production Backend integration, or all OS failures. The probe's worker-selection argument is not a product CLI contract.

The next milestone is a Linux control-transport/descriptor-ownership probe and a Windows overlapped-I/O probe. Verify both before production Backend integration, then compare the same observations used by common-runtime tests.
