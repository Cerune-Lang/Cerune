# Linux subprocess Backend

[日本語](subprocess-linux.ja.md)

The Linux OS connection now implements the common runtime as Rust's `Processes<Linux>`. **Language standard modules, common IR, CLI, generated routes, and supervisor distribution are not connected yet.** A separate Cargo example provides the verification supervisor.

## Responsibilities and launch

| Component | Responsibility |
| --- | --- |
| Common runtime | Requests, limits, deadlines, counts, first failure, cancellation, cleanup decisions |
| [Linux Backend](../../src/runtime/process/linux.rs) | Explicit registration, control transport, nonblocking transfers, call identity and NodeId/Span, reaping the supervisor |
| [Dedicated supervisor](../../src/runtime/process/linux/supervisor.rs) | Subreaper configuration, target process group, direct child and descendants in that group |
| [Execution checks](../../experiments/subprocess/linux_backend.rs) | Fixtures through the common runtime; results, failures, observation, host isolation |

Linux::new(supervisor_path, targets, trace_limit) takes an absolute supervisor path and registrations containing a name, absolute executable, absolute working directory, and explicit environment. There is no PATH search, bulk parent environment inheritance, or automatic supervisor restart. Relative paths are rejected. Paths retain OS bytes; arguments and environment retain UTF-8 without Unicode normalization. Environment names are case sensitive.

1. Start one supervisor per call; the host owns only that supervisor as its direct child.
2. Verify protocol version and call identity before transmitting a bounded registration and arguments. A mismatch prevents target launch.
3. The supervisor starts the target in its own process group, passes the three parent stdio descriptors, and closes its copies.
4. The host transfers body bytes directly. The supervisor reports the direct child's exit without reaping the leader before group termination is requested.
5. Request group termination and reap the direct child and descendants in that group. The Backend requires cleanup evidence, successful supervisor exit and reap, terminal transfers, and closed descriptors.

The host's subreaper, process group, and SIGCHLD handler are preserved. The host never reaps unrelated children with waitpid(-1). Only the control descriptor is inherited by the supervisor; only stdio is inherited by the target. Inheritance uses close_range(CLOEXEC), with a fixed-buffer /proc/self/fd scan on unsupported kernels.

## Initial bounds and protocol

These are Rust connection limits, separate from the eventual language API and distribution format.

| Item | Bound or contract |
| --- | --- |
| Registrations | 64 targets, 256 UTF-8 bytes per name, 128 environment entries per target |
| Paths | 4095 OS bytes each, absolute, no NUL |
| Paths and environment per target | 32768 bytes total; all registrations including names fit within 1 MiB |
| Call arguments | 4096 entries, 16384 UTF-8 bytes total, no NUL |
| Launch request | 65536 body bytes including length fields; unsigned little-endian lengths followed by bytes |
| Control transport | SOCK_SEQPACKET; 24-byte header containing CSRP, version 1, kind, nonzero call identity, full size, value |
| Descriptor transfer | Exactly three on READY; check close-on-exec, pipe type, direction |
| Supervisor setup | Five seconds; watchdog after START uses received remaining time plus five seconds; abnormal cleanup has at most five seconds |
| Accepted remaining launch time | 1–86400000 milliseconds; the common runtime deadline still applies independently |
| OS records | 0–4096 entries; zero disables recording; no automatic body, arguments, environment, or absolute paths |

The protocol checks HELLO exchange, START, READY or FAILED, exit notification, FINISH, and CLEANED. It rejects duplicates, wrong versions or call identities, inconsistent sizes, extra descriptors, and truncation, closing received descriptors on rejection. Untrusted lengths cannot trigger unbounded allocation. Communication and polling do not extend the common absolute deadline.

The supervisor watchdog assists reclamation after parent loss; it does not decide program success. This does not guarantee forcibly interrupting an OS launch call at a specified instant. Polling optimization and performance comparisons remain future work.

## Transfers, failures, and ownership

| Condition | Treatment |
| --- | --- |
| Positive count | Report the confirmed count, including short transfers |
| Zero read | EOF, distinct from a successful zero write |
| EAGAIN / EINTR | Retain the buffer and return to deadline checking; never report zero progress |
| Cancellation while waiting | The synchronous syscall has returned, so remove the transfer safely; report cancellation reply and terminal completion separately |
| EPIPE or another transfer failure | Preserve confirmed counts and report failure without terminating the host through its default SIGPIPE handler |
| Confirmed target launch failure and supervisor reap | NotStarted; the caller may explicitly make a subsequent call |
| Launch-time disconnect or supervisor failure | Uncertain or an execution failure; never claim unconfirmed scope cleanup |
| Cleanup deadline exceeded | The common runtime disables reuse; the Backend retains remaining supervisor ownership |

SIGPIPE is temporarily blocked only on the writing thread. Existing pending signals and the original mask are preserved; the process-wide handler is unchanged. A failed synchronous Linux write can provide a known progress count where Windows cancellation may leave progress unknown. The Backend preserves that difference in OS evidence instead of hiding uncertainty to produce identical values.

events() and has_unreclaimed_resources() are read-only. reclaim(timeout) explicitly controls execution without rewriting earlier Outcomes. A failed Linux close is recorded; the same numeric descriptor is not closed again.

Final Drop closes control to trigger finite supervisor cleanup and checks supervisor exit once. It creates neither an unlimited wait nor a background thread. If the supervisor remains unreaped, its management information is retained until process exit and counted by retained_connections(). This is not recovery or successful reclamation; repeated occurrences consume resources, including unreaped processes. Prefer normal cleanup and explicit reclaim.

Reclamation is not guaranteed after forced supervisor termination, kernel-blocked work, descendants leaving the managed group, external wait/signal interference, or allocator failure. After supervisor loss, the host never signals a numeric PGID whose identity it cannot verify.

## Verification and next work

Run on Linux with a new output directory:

    cargo build --example subprocess_linux_supervisor
    cargo run --example subprocess_linux_backend -- "$PWD/target/debug/examples/subprocess_linux_supervisor" target/subprocess-linux-backend

The 23 cases cover normal execution, arguments/environment, NUL/CR/LF, simultaneous transfers, nonzero exit, output limits, timeout, descendant cleanup, continuation after failure, observation, cancellation, and explicit reclaim. Fault supervisors exercise version mismatch, launch disconnect, loss after READY, and cleanup deadline expiry without launching a real target. Checks also preserve the host's subreaper, group, SIGCHLD, unrelated-child wait ownership, descriptors, default SIGPIPE behavior, and existing pending signals. OS notification order and operation identities are not fixed.

[Public data types](subprocess-values.en.md) are available. Next reserve result storage and connect the operation through common IR. The production supervisor binary, manifest, and build/release packaging belong to distribution work.

OS contracts: [wait/waitid](https://man7.org/linux/man-pages/man2/waitpid.2.html), [Unix sockets and SCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html), and [why Linux close must not be retried](https://man7.org/linux/man-pages/man2/close.2.html).
