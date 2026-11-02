# Subprocess OS probe

[日本語](README.md)

This experiment checks shell-free launching, arguments, concurrent I/O, deadlines, and descendant reclamation on Windows/Linux before implementing OS connections for the [subprocess contract](../../docs/design/subprocesses.en.md). **It is not a Cerune language API or ordinary runnable example, and it is not yet connected to the common runtime Backend.**

## Running

Use Rust 1.98.0 with Windows 10 or later, or Linux. Local verification used Windows x86-64 and Ubuntu x86-64 under WSL. The probe runs its own copied executable as child/grandchild fixtures, not arbitrary external commands.

Run from the repository root. Supply a directory that does not exist; use another name for subsequent runs rather than overwriting observations.

```text
cargo test --example subprocess_probe
cargo run --example subprocess_probe -- target/subprocess-os-probe
```

The directory contains a fixture-executable copy, a working directory, and `observations.jsonl`. Records include pass/fail, requested/accepted/retained counts, termination status, cleanup evidence, and elapsed time, without argument/environment/I/O contents or absolute paths. CI runs on Windows/Linux and uploads only JSONL.

## Cases and results

On 2026-11-02, all nine common cases passed locally on Windows; Linux passed ten cases including supervisor isolation. A passing case includes correctly detecting an expected failure.

| Case | Evidence |
| --- | --- |
| context | Preserve empty arguments, spaces, quotes, trailing backslashes, Japanese text, CR/LF, and shell metacharacters as argv; executable/cwd paths contain spaces and Japanese text; explicit environment without parent PATH inheritance |
| duplex | Child writes 128 KiB each to stdout/stderr before reading stdin; collect 128 KiB stdin, 256 KiB stdout, and 128 KiB stderr without loss |
| exit | Preserve nonzero exit: `exit(-1)` yields 4294967295 on Windows and 255 on Linux; do not pretend OS values are identical |
| early-close | Detect write failure when the child exits without reading stdin; retain counts acknowledged by the write API |
| limit | With stdout limit 7, read 8 bytes to detect overflow and retain only 7 |
| exact | With output length and limit both 7, confirm EOF with an additional read and succeed |
| sleep | Stop after a two-second deadline and collect prior output containing NUL/CR/LF |
| descendant | Direct-child exit code 0 does not finish collection while a grandchild retains pipes; terminate and reclaim the grandchild after two seconds |
| missing-executable | Do not treat a nonexistent executable as successfully launched |
| supervisor-isolation (Linux) | Worker subreaper state changes 0→1 while caller remains 0→0; preserve caller SIGCHLD/process-group configuration and recover an unrelated child's exit 255 plus supervisor termination |

`stdin_accepted` counts bytes successfully acknowledged by the write API, not bytes consumed by the child. Early-close counts, ordering among concurrent failures, and elapsed times vary with OS/load; comparisons do not require identical numbers.

On Linux, the normal invocation splits into a caller-side checker and a dedicated worker. The worker runs nine cases; the caller checks configuration, the unrelated child, and supervisor reclamation before appending case ten. Observation reads are capped at 64 KiB and supervisor waiting at 60 seconds. Exceptional termination and a wait of at most two seconds are probe fallbacks, not guarantees of managed-scope cleanup after forced supervisor termination. Descriptor transfer and versioned control messages are checked separately by the [transport/cancellation probe](transport/README.en.md).

## OS mechanisms

| OS | Launch and management | Cleanup evidence |
| --- | --- | --- |
| Windows | `CreateProcessW` with Job List and explicit inherited handles; create suspended, then resume; do not pass the Job handle to children or permit breakaway; show no window | After `TerminateJobObject`, confirm direct-child termination and zero active Job processes; join I/O workers only after they finish |
| Linux | Make this dedicated probe process a subreaper; place the child in its own process group; observe exit with `waitid(WNOWAIT)` without reaping the leader before termination requests | Send SIGKILL to the group, then wait for the direct child and adopted descendants; confirm no remaining waitable children and completed workers |

Windows Job List establishes membership before the initial thread runs, following [Microsoft's explanation](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812). This avoids the race in launching first and assigning the Job later. The experiment does not validate supervisor crashes at every point inside the launch API.

On Linux, [WNOWAIT](https://man7.org/linux/man-pages/man2/waitpid.2.html) separates status observation from reaping. Retaining the leader until group termination prevents sending a later termination request to a reused PID. [Subreaper](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html) configuration affects the calling process, so this is not a proposal to apply it directly to a Cerune embedding host.

Cleanup waits at most three seconds in the polling loop. Join only finished workers; incomplete cleanup is a failed observation. Drop-time termination is an exceptional fallback, not evidence that reclamation succeeded.

## Before integration into Cerune

1. **A contract for transfers in flight.** Three workers transfer concurrently and report when they finish. Directly copying these workers into the original synchronous poll would lose unreported transfers. The runtime now uses an [ownership-based contract and state component](../../docs/design/subprocess-transfers.en.md), integrated with launch, deadlines, exit monitoring, and cleanup. This experiment's workers are not yet connected to that contract.
2. **Supervisor placement.** The [OS connection design](../../docs/design/subprocess-os.en.md) selects one dedicated supervisor per call. Following isolation, [transport/cancellation combined with launch/scope management](transport/README.en.md) is measured. Cleanup after control disconnection is verified; ownership after forced supervisor termination and distribution integration remain future work. Do not add process-wide subreaper effects to embedding hosts.
3. **Cleanup failures and later calls.** Real-OS fixtures here can be reclaimed normally. Classify failed termination, unreapable children, handle-close failures, and supervisor failure in real connections, then map these to disabled connections and partial results in the common runtime.
4. **Public integration.** Registration format, fixed numeric limits, `std:process` types, IR instructions, route ABIs, and language examples remain future work. General connection performance and semantic equivalence with observation enabled/disabled are also unverified.

Descendants escaping the managed scope and sandboxing untrusted code are out of scope. The experiment does not guarantee completion within a requested duration under external forced termination or OS resource exhaustion.

## Common-runtime integration checks

Windows now has 18 cases using the [production-code Backend](../../docs/design/subprocess-windows.en.md). These exercise common requests, results, deadlines, and observation separately from the independent probes.

```text
cargo run --example subprocess_windows_backend -- target/subprocess-windows-backend
```

This requires Windows and a new output directory. CI retains its JSONL too. Cerune language examples remain later work.

## Linux common-runtime verification

The [Linux Backend](../../docs/design/subprocess-linux.en.md) has 23 cases. Specify an absolute supervisor path and a new output directory. CI runs the cases and retains JSONL. The supervisor is a verification Cargo example; production distribution follows.

    cargo build --example subprocess_linux_supervisor
    cargo run --example subprocess_linux_backend -- "$PWD/target/debug/examples/subprocess_linux_supervisor" target/subprocess-linux-backend
