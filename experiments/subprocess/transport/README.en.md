# Subprocess transport and cancellation probe

[日本語](README.md)

This experiment checks prerequisites for the [OS connection design](../../../docs/design/subprocess-os.en.md), separately from the [launch/scope probe](../README.en.md). **It is neither a production Backend nor a language API.** It does not freeze a product control protocol or public numeric limits.

## Running

Run from the repository root with a new output directory. Verification uses Rust 1.98.0 on Windows x86-64 and Linux x86-64.

```text
cargo test --example subprocess_transport_probe
cargo run --example subprocess_transport_probe -- target/subprocess-transport-probe
```

The output is `observations.jsonl`: counts, confirmed progress, OS errors, and reclamation evidence, without content, arguments, environment, or absolute paths. CI runs both OSes and retains JSONL. Correctly detecting an expected failure counts as a passing case.

## Linux: six cases

| Case | Evidence |
| --- | --- |
| descriptor-roundtrip | Transfer a control endpoint to a separate worker; receive three parent-side stdin/stdout/stderr descriptors via SCM_RIGHTS. Check direction, pipe type, and close-on-exec; preserve Japanese text, NUL, and CR/LF. Close duplicates, confirm EOF and exit 7, and reap the worker |
| invalid-control-packets | Reject eight faults: version, call identity, declared length, short payload, extra descriptors, truncated ancillary data, oversized payload, and message kind. Descriptor count remains unchanged after each |
| duplicate-control-identity | Reject an old call identity while expecting the next call |
| control-peer-closed | Detect control-peer closure as EOF |
| sigpipe-default-handler | Return EPIPE without process termination even with the default SIGPIPE disposition; preserve handler and thread mask |
| sigpipe-pending-preserved | Preserve a preexisting pending SIGPIPE and restore the original mask |

The experimental frame is 24 bytes, little-endian: `CSPT`, u16 version, u16 kind, u64 call identity, u32 declared length, and u32 value. Kinds are HELLO/READY/FINISH/EXIT; the fixed fixture uses identity 1. Launch only after HELLO validation. Arbitrary registered targets, arguments, and environments are not transmitted.

Use `SOCK_SEQPACKET` with a 25-byte payload buffer and bounded ancillary storage. Take ownership of received descriptors before validation so failures close them. Close worker copies of parent pipe endpoints, and restore close-on-exec on the worker control descriptor before target launch. [SCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html)

On 2026-11-02, WSL kernel release `4.4.0-26100-Microsoft` returned no `MSG_TRUNC` in a standalone 152-byte send / 24-byte receive. One extra receive byte lets actual length reject oversized frames independently of that flag. This observation does not establish the same behavior on ordinary Linux kernels. [recvmsg contract](https://man7.org/linux/man-pages/man2/recvmsg.2.html)

Pipe writes temporarily block SIGPIPE in the calling thread, drain the signal associated with EPIPE only if none was previously pending, and restore the mask. Control sends use `MSG_NOSIGNAL`. Default-handler setup occurs only inside a separate fixture. Coalescing with concurrently externally delivered SIGPIPE remains unverified.

## Windows: nine cases

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
| buffer-ownership-settled | No quarantined, unreclaimed operations after all cases |

Each operation holds its buffer and OVERLAPPED at fixed heap addresses and owns handles/events until termination. Successful `CancelIoEx` is not completion. Confirm counts only on successful `GetOverlappedResult`; errors retain unknown counts (JSON null), not zero. [Cancellation](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex), [completion/counts](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)

Pipe names use 128 random bits, first-instance enforcement, remote rejection, and an explicit ACL. Cross-user impersonation and remote intrusion tests are not performed. [CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew)

## Results and remaining work

Local runs on 2026-11-02 passed six Linux cases and nine Windows cases. Normal transport/completion waits use five seconds; Windows Drop reclamation waits 250 ms. These are probe watchdogs, not language execution limits.

If Windows Drop reaches its deadline before terminal confirmation, it quarantines OS-referenced storage instead of freeing it and makes verification fail. Triggering this retention and recovering from exhaustion remain unverified; this is not a permanent production implementation.

Next combine the existing launch/scope probe with these transfers. On Linux, verify prevention of unrelated host-descriptor inheritance, managed-target cleanup on supervisor disconnection, and integrated control states/deadlines. On Windows, combine target-facing standard I/O, Job termination with pending operations, and final-owner destruction. Then integrate the common-runtime Backend and compare results/observations. The probe does not guarantee reclamation under resource exhaustion or arbitrary external forced termination.
