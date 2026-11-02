# Subprocess transport and cancellation probe

[日本語](README.md)

This experiment checks prerequisites for the [OS connection design](../../../docs/design/subprocess-os.en.md), sharing components with the [launch/scope probe](../README.en.md). Following individual checks, it now combines launch, transfers, and termination. **It is neither a production Backend nor a language API.** It does not freeze a product control protocol or public numeric limits.

## Running

Run from the repository root with a new output directory. Verification uses Rust 1.98.0 on Windows x86-64 and Linux x86-64.

```text
cargo test --example subprocess_transport_probe
cargo run --example subprocess_transport_probe -- target/subprocess-transport-probe
```

The output is `observations.jsonl`: counts, confirmed progress, OS errors, and reclamation evidence, without content, arguments, environment, or absolute paths. CI runs both OSes and retains JSONL. Correctly detecting an expected failure counts as a passing case.

## Linux: nine cases

| Case | Evidence |
| --- | --- |
| descriptor-roundtrip | Transfer a control endpoint to a separate worker; receive three parent-side stdin/stdout/stderr descriptors via SCM_RIGHTS. Check direction, pipe type, and close-on-exec; preserve Japanese text, NUL, and CR/LF. Close duplicates, confirm EOF and exit 7, and reap the worker |
| invalid-control-packets | Reject eight faults: version, call identity, declared length, short payload, extra descriptors, truncated ancillary data, oversized payload, and message kind. Descriptor count remains unchanged after each |
| duplicate-control-identity | Reject an old call identity while expecting the next call |
| control-peer-closed | Detect control-peer closure as EOF |
| sigpipe-default-handler | Return EPIPE without process termination even with the default SIGPIPE disposition; preserve handler and thread mask |
| sigpipe-pending-preserved | Preserve a preexisting pending SIGPIPE and restore the original mask |
| managed-finish | Leader exits 0 while a grandchild retains pipes; after explicit termination, reap grandchild/supervisor and confirm prior output plus EOF |
| managed-disconnect | Close the control connection in the same state; the supervisor reclaims the scope. Confirm worker checks and data-pipe EOF |
| managed-invalid-control | Send an invalid version while keeping control open; the supervisor reclaims the scope. Explicitly select descriptor scanning too |

The experimental frame is 24 bytes, little-endian: `CSPT`, u16 version, u16 kind, u64 call identity, u32 declared length, and u32 value. Kinds are HELLO/READY/FINISH/EXIT plus CLEANED for scope-reclamation evidence; the fixed fixture uses identity 1. Launch only after HELLO validation. Arbitrary registered targets, arguments, and environments are not transmitted.

Use `SOCK_SEQPACKET` with a 25-byte payload buffer and bounded ancillary storage. Take ownership of received descriptors before validation so failures close them. Close worker copies of parent pipe endpoints, and restore close-on-exec on the worker control descriptor before target launch. [SCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html)

On 2026-11-02, WSL kernel release `4.4.0-26100-Microsoft` returned no `MSG_TRUNC` in a standalone 152-byte send / 24-byte receive. One extra receive byte lets actual length reject oversized frames independently of that flag. This observation does not establish the same behavior on ordinary Linux kernels. [recvmsg contract](https://man7.org/linux/man-pages/man2/recvmsg.2.html)

Pipe writes temporarily block SIGPIPE in the calling thread, drain the signal associated with EPIPE only if none was previously pending, and restore the mask. Control sends use `MSG_NOSIGNAL`. Default-handler setup occurs only inside a separate fixture. Coalescing with concurrently externally delivered SIGPIPE remains unverified.

Managed cases reuse existing launch/reclamation logic in a dedicated worker with subreaper/process-group management. Retain the leader with WNOWAIT until group termination, then reap the grandchild. Worker exit 0 means its reclamation checks passed, not that control communication or the target succeeded. Disconnection prevents a control-channel cleanup report, so separately confirm worker checks and data-pipe EOF.

Before supervisor exec, set descriptors 3 and above close-on-exec and allow only the control descriptor. If close_range is unsupported, scan /proc/self/fd with fixed-buffer getdents64/fcntl in the single post-fork thread. Do not rely on a parent-side snapshot; fail launch if scanning fails. Verify exclusion of a high-numbered inheritable descriptor while preserving the parent's flags. WSL exercises scanning; CI runs both normal selection and forced scanning. [close_range](https://man7.org/linux/man-pages/man2/close_range.2.html)

## Windows: thirteen cases

| Case | Evidence |
| --- | --- |
| restricted-pipe-pair | Noninheritable handles and duplicate first-instance rejection; configure owner/system ACL and remote rejection |
| short-read-binary | Short read against a 64-byte request, Japanese text/NUL/CR/LF preservation, exact successful counts |
| pending-read-cancel | Cancel a pending read and separately confirm its aborted terminal result; retain the OVERLAPPED address |
| completion-before-cancel | Cancel after OS completion but before result collection without losing success or six confirmed bytes |
| read-write-cancel-race | Across 32 races, distinguish success from cancellation and deliver each terminal result once; do not require fixed frequencies |
| pending-write-cancel | Cancel a 64 KiB write exceeding pipe capacity and confirm termination |
| peer-closed | Preserve broken-pipe failure when the peer closes |
| pending-drop | Cancel and settle an outstanding operation during Drop, releasing owned handles |
| job-stdio-roundtrip | Connect synchronous child stdio to an asynchronous parent; preserve Japanese/NUL/CR/LF, exit 7, and zero active Job processes. Exclude an inheritable host handle absent from the explicit list |
| job-terminate-pending | Terminate the Job with pending reads and a 64 KiB write; separately confirm Job reclamation and I/O termination |
| job-descendant-pending | After leader exit 0, a grandchild retains pipes; terminate the Job, confirm zero active processes and terminal reads |
| job-owner-drop | Leave pipe ownership only with the pending operation and drop the final Job owner; confirm direct-child termination and terminal I/O |
| buffer-ownership-settled | No quarantined, unreclaimed operations after all cases |

Each operation holds its buffer and OVERLAPPED at fixed heap addresses and owns handles/events until termination. Successful `CancelIoEx` is not completion. Confirm counts only on successful `GetOverlappedResult`; errors retain unknown counts (JSON null), not zero. [Cancellation](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex), [completion/counts](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)

Pipe names use 128 random bits, first-instance enforcement, remote rejection, and an explicit ACL. Cross-user impersonation and remote intrusion tests are not performed. [CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew)

## Results and remaining work

Local runs on 2026-11-02 passed nine Linux cases and thirteen Windows cases. Normal transport/completion waits use five seconds; Windows Drop reclamation waits 250 ms. These are probe watchdogs, not language execution limits.

If Windows Drop reaches its deadline before terminal confirmation, it quarantines OS-referenced storage instead of freeing it and makes verification fail. Triggering this retention and recovering from exhaustion remain unverified; this is not a permanent production implementation.

The cases above now combine launch/scope management with transfers. The [Windows Backend](../../../docs/design/subprocess-windows.en.md) now integrates the common runtime with explicit registration, capacity, and unreclaimed ownership. Next connect Linux supervisor registration/control transport to the same contract. Forced supervisor termination, disconnection during launch, unreclaimable resources, and comprehensive OS-error injection remain unverified. The probe does not guarantee reclamation under resource exhaustion or arbitrary external forced termination.
