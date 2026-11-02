# Subprocess execution contract proposal

[日本語](subprocesses.ja.md)

This is a design proposal as of 2026-11-01. **Language-level subprocess execution is not implemented.** Related issues are [#89](https://github.com/Cerune-Lang/Cerune/issues/89), [#173](https://github.com/Cerune-Lang/Cerune/issues/173), and [#104](https://github.com/Cerune-Lang/Cerune/issues/104) for comparisons without duplicating external effects.

## Direction and current status

The proposed initial operation synchronously invokes a previously registered executable without a shell, sends bounded stdin, and collects bounded stdout, stderr, and termination status.

The user confirmed that **subprocess failures must be observable and the caller should be able to continue**. The design therefore returns launch, transfer, capacity, and timeout failures as results, as well as nonzero exits. It does not copy the existing file-I/O policy of stopping Cerune on an operation failure.

| Category | Status |
| --- | --- |
| Existing implementation | [Limited file I/O](file-io.en.md): reading and creation, with connections in four executors and existing generated routes |
| Existing separate uses | Linker invocation in `src/native_build/link.rs`; test execution in `tests/support/process.rs` |
| This proposal | Registered targets, explicit argv, capacity/time limits, recoverable operation results, read-only observation |
| Decisions before implementation | Public types/parameters, numeric limits, registration format, measured OS cleanup behavior, generated-route ABIs |
| Outside the initial scope | Shell syntax, interactive terminals, background execution, pipeline syntax, general job management, sandboxing untrusted code |

The existing linker and test helpers collect output into temporary files and enforce timeouts. They are not a common language runtime satisfying output-capacity contracts, recoverable result types, and complete cleanup records. Even if low-level code is shared, build/test operations remain separate from source-program semantics.

## Partial common-runtime implementation

`runtime::process::Processes<B>` now integrates [in-flight transfer state](subprocess-transfers.en.md). A [scripted connection](../../src/runtime/process/tests.rs) verifies requests, launch, transfers, deadlines, exit monitoring, and cleanup. **Production OS connections, `std:process`, IR instructions, executors, and generated routes remain unimplemented.** Cerune programs cannot yet launch subprocesses.

| Component | Current behavior |
| --- | --- |
| `Request` / `Limits` | Explicit target, arguments, input, output limits, and `Duration` deadlines; no default limits yet |
| `Processes::run` | Synchronously returns an `Outcome` while progressing three streams concurrently; ordinary failures with confirmed reclamation allow a later explicit call |
| `Outcome` | Stdout/stderr contents, a `Report` with origin/counts/exit/failure/cleanup, and transfer state; `completed()` does not mean child exit code zero |
| `Backend::start` | Distinguishes started, confirmed not started, and uncertain launch; uncertain launch still receives one cleanup attempt |
| `Backend::submit` | Takes ownership of a buffer of at most 4096 bytes; even submission failure returns a terminal notification |
| `Backend::poll` | Waits until the deadline, returning transfers, cancellation replies, child exit, and cleanup in a fixed-capacity `Batch` |
| `Backend::cancel` | Requested once per pending operation; cancellation replies and transfer termination are separate |
| `Backend::begin_cleanup` | Begins once for started/uncertain calls, on success and failure; poll continues collecting late notifications during cleanup |
| `events()` | Read-only call/phase/origin, offered/confirmed/retained counts per operation, states, cancellation and terminal outcomes; no contents, targets, or arguments |

The Rust connection contract changed. The old borrowed-buffer `Poll`/`PollEvent` interface is removed from production; the [old synchronous model](../../src/runtime/process/synchronous.rs) remains only for comparison tests. This is not a stable embedding ABI; no existing language CLI or production OS connection is broken. Registration and OS configuration remain connection responsibilities, and the scripted connection is not a user-facing execution route.

Validation precedence is disabled connection, invalid request, request capacity, unconfigured connection, buffer allowance, allocation, representable deadline addition, then launch. Request capacity includes argument count, combined UTF-8 bytes of target and arguments, stdin length, combined output limits, and timeout. Empty targets, NUL in targets/arguments, and zero operation/cleanup durations fail before launch. Exhausted call identities disable further launch without wrapping.

Reserve both retained output buffers plus three 4096-byte transfer buffers before launch. `available_bytes` bounds their sum, not language array storage. Lifetime management after return and reservations for conversion to `[u8]` follow with IR integration. Short results retain reserved capacity.

Check clock regression/deadlines before notifications, then process stdin, stdout, stderr, cancellation replies, exit status, and cleanup reports in that order. Even at the deadline, retain accepted counts and partial contents without replacing the first failure. Excessive counts, stale operations, duplicate exits, and empty ordinary event batches violate the contract. Direct-child exit alone is insufficient: normal completion also requires stdin close and both output EOFs. Interrupted waiting waits again; launch and close are not implicitly retried.

Do not reuse connections after incomplete cleanup, clock regression, cleanup deadline expiry, unknown progress, protocol violations, or missing transfer/cancellation notifications. Preserve the connection's report in `cleanup_reported` and the deadline-aware verdict in `cleanup`, including reclaimed-resource flags and whether termination was requested. Later calls on disabled connections perform no clock reads, launch, transfer, or cleanup.

Control operations do not wait; poll performs bounded waiting. The common runtime cannot forcibly interrupt a connection that ignores the contract and never returns. Waiting, fairness, ownership of unreclaimed resources, and shutdown behavior require real-OS verification. Disabling, truncating, or failing to allocate observation history changes neither results nor control actions.

Run the tests with:

```text
cargo test --lib runtime::process
```

Tests compare all six notification-array orders, late partial transfers, delayed/missing cancellation replies, EOF/limits, cleanup, later calls, and observation enabled/disabled. Normal exit, output, timeout, and capacity failure also compare results against the old synchronous model.

The [Windows/Linux OS probe](../../experiments/subprocess/README.en.md) already verifies nine cases including argv, concurrent transfer, deadlines, and descendants. Windows confirms Job membership before execution and zero active processes; Linux confirms descendant reaping in a dedicated supervisor. The [OS connection design](subprocess-os.en.md) now includes measured Linux supervisor isolation from caller configuration and unrelated children. [Control transport/descriptor transfer and Windows asynchronous I/O/cancellation](../../experiments/subprocess/transport/README.en.md) are also verified separately. Next combine launch/scope management with transfers and verify ownership during disconnection/termination, then connect production Backends, public result types, standard modules, and common IR.

## Calls and connections

Candidate names are the virtual standard module `std:process` and operation `run`. Use the reserved names, virtual sources, and SourceIds of [existing standard modules](standard-modules.en.md), without changing relative imports or introducing implicit file search.

The proposed request contains the following information. Finalize its function signature after checking representation with existing structs and enums.

| Request | Proposal |
| --- | --- |
| Target | A name registered in the connection. Registration specifies an absolute executable path, working directory, environment, and argument-decoding convention |
| Arguments | `[string]`, evaluated left to right; preserve empty arguments, spaces, quotes, and Japanese text. Reject NUL before launch |
| Stdin | `[u8]`; empty means immediate EOF. No appended newline, encoding conversion, or Unicode normalization |
| Output limits | Separate maximum byte counts for stdout/stderr; zero is valid and any output then exceeds the limit |
| Timeout | A finite elapsed-time limit using a monotonic clock. Fix units and supported range after measurements |

Connections also set maximum argument count, aggregate argument bytes, stdin size, output size, and time; requests exceeding these return a failure value before launch. Fix numeric values and validation order when implementing the common runtime. Operations are synchronous and sequential; a parallel-launch API is outside this change.

Raw bytes are the proposed input/output representation. Invalid UTF-8 can therefore be retained as child output. Consider explicit string conversion as a separate pure operation; do not silently insert replacement characters or discard output. The public scope of raw-byte APIs and string conversion remains a pre-implementation decision.

Missing registration returns an unconnected failure value. Do not implicitly search PATH, use the caller's current directory, or inherit the entire parent environment. Register required environment entries explicitly and validate OS differences such as environment-name case rules. Registration provides permission/configuration, not executable-content immutability or restrictions on the child's file access. The `std:fs` working area is not a subprocess sandbox.

Initially target Windows executables using conventional argv decoding; custom command-line parsers require a separate connection. Reject `.bat`/`.cmd` and implicit shell fallback. Do not fall back to a shell on Linux either. Scripts would use an explicitly registered interpreter. [Rust's documentation](https://doc.rust-lang.org/std/process/index.html) describes Windows argument decoding and special batch-file handling.

## Results and recovery

A generic Result type is not a prerequisite. Represent the following using existing enums, structs, and owned arrays.

| Result | Retained information | Caller behavior |
| --- | --- | --- |
| Completed | Termination status, stdout, stderr, stdin accepted count, cleanup outcome | Return nonzero codes too; the caller decides what constitutes success |
| Operation failed | Cause/phase/origin, launch state, partial stdout/stderr, stdin accepted count, any available termination status, cleanup outcome | Use `match` to log, choose another action, or make another call |
| Cerune execution failed | Existing runtime failure and source location, such as inability to construct a value | Do not disguise this as a child exit code |

“Completed” means Cerune finished transfer and collection. Distinguish normal exit codes, Linux signal termination, and unavailable status. Preserve Windows 32-bit exit values without signed truncation or guessing POSIX signals.

Proposed operation-failure categories follow. Public names remain undecided.

| Cause | Distinctions |
| --- | --- |
| Request/connection | Invalid limits/NUL, unregistered, unconnected, unsupported connection, or connection disabled after incomplete cleanup |
| Launch | Missing target, execution denied, OS resource exhaustion, other launch errors. Record unknown launch state when it cannot be determined |
| Transfer | Stdin write/close, stdout/stderr read, zero progress, connection-contract violation |
| Capacity | Stdout limit, stderr limit, pre-launch request/result-storage limits |
| Time | Operation deadline; distinguish this from a cleanup deadline |
| Collection | Wait, status retrieval, termination request, or handle-reclamation failure |

Finalize each call independently and permit another call on the same connection after an ordinary failure. Do not reuse the file runtime's state that stops every operation after its first failure. Only an explicit new call by the program retries execution. Previous external effects have not been rolled back.

If cleanup cannot be confirmed, return the original failure and cleanup outcome and let the caller continue. The proposal disables new launches through that process connection; subsequent requests return a connection-disabled result without effects. The runtime retains outstanding management information and uses finite cleanup procedures at execution shutdown. Never report an unreaped child as terminated. OS prototypes must validate this state transition and resource lifetime.

## Transfer, limits, and termination

1. After evaluating argument expressions, validate the request, connection, and capacity. Do not launch on failure. Check space for output and result values, distinguishing transfer-byte limits from Cerune array-storage limits. Do not assume that each `[u8]` element physically occupies one byte.
2. Start the deadline immediately before the launch attempt. Progress stdin transmission, stdout/stderr draining, and termination monitoring concurrently to avoid mutually blocked pipes. Do not finish all input before reading output.
3. Close the write end after sending all stdin. Accepted bytes mean acceptance by the OS connection, not consumption by the child. Early child closure is a transfer failure retaining the unsent count. Retry interrupted transfers only when progress is known; never automatically retry launching.
4. Preserve order within stdout and stderr without promising a total order between them. Do not automatically forward them to Cerune stdout or implicitly run parent print/flush operations. An exact-limit result still needs an EOF check. Read at most one additional byte per stream to detect overflow; exclude that probe byte from result arrays and record the distinction between read and retained counts.
5. Normal collection requires completed stdin transmission/close, EOF on both outputs, and the direct child's termination status. A direct-child exit is insufficient while descendants retain pipes. The deadline remains active during collection. This follows the [pipe EOF conditions](https://man7.org/linux/man-pages/man7/pipe.7.html).
6. If the observed clock reaches or exceeds the deadline before completion is finalized, report timeout. Fix the choice among simultaneously available events in the common state machine and verify it with scripted connections. Blocking OS operations and scheduling delays mean this is not a guarantee that the call returns within the requested duration.
7. On failure, stop ordinary transfers, request termination of managed processes, reap the direct child, and close handles. Give cleanup a separate finite limit. Preserve partial output and accepted counts; never replace the original cause with a cleanup error.

Design successful completion to leave no managed descendants running either. Record any necessary termination of remaining descendants instead of silently turning the operation into background execution.

Candidate mechanisms are a dedicated Linux process group and Windows Job Object. Linux descendants can leave a group using [setsid](https://man7.org/linux/man-pages/man2/setsid.2.html), so this is not containment of all descendants. On Windows, establish Job membership before user code executes and specify [inheritance and breakaway behavior](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects). Neither approach is a sandbox for arbitrary untrusted code. Cleanup records distinguish reaping the direct child, requesting termination of the managed scope, and unconfirmed outcomes.

## Observability and execution routes

Add an explicit effectful operation to common IR, retaining request/result and NodeId/Span/SourceId correspondence through HIR→MIR→SSA→bytecode/LIR and generated code. Track both the standard-module operation location and the user's call site. Optimization must not eliminate, duplicate, or move the call; original unoptimized forms remain available.

Proposed observation records contain a request number, registered-target identifier, phases, configured limits, requested/accepted/retained byte counts, termination status, failure, cleanup, and origin. Do not automatically include arguments, environment values, stdin/stdout/stderr content, or absolute paths in diagnostics. Programs can use result contents, but observation APIs cannot change read positions, deadlines, launching, retries, or termination requests.

Separate mandatory operation results from optional read-only history. Disabling history, reaching its capacity, or failing to allocate a history record must not change transfers or result values. Real-OS outcomes near a deadline can depend on load and measurement overhead; different executions do not promise identical elapsed times or stop reasons.

| Route | Responsibility |
| --- | --- |
| IR/MIR/SSA/VM | Share request/result/limit rules; separate scripted connections from OS connections |
| C/LLVM/QBE/Native | Lower the same rules to explicit runtime calls, with explicit targets and connection configuration |
| WAT | Transfer requests/results through a versioned host contract; keep internal memory private and do not treat missing imports as success |
| build/release | Record required connection contracts, targets, and limits; do not automatically embed local paths or environment secrets |
| compile/check/emit/observation | Never launch a subprocess written in source. Explicit Native linking remains a separate build operation |

Preserve the program-argument rules of `--cerune-runtime-v1`. Finalize registration files/CLI options, host ABI, and artifact versions separately, listing existing-host compatibility changes before implementation. A general effect system or record/replay infrastructure is not required.

## Implementation order and completion criteria

| Stage | Work | Evidence |
| --- | --- | --- |
| 1 | Use minimal OS prototypes to establish feasibility; define result types, requests, limits, cleanup states, a common runtime, and a scripted connection | Continuation after failure, disabled connections, partial results, no duplicate launch or reclamation |
| 2 | Connect `std:process`, common IR, and four executors | Identical requests/responses produce matching values, failures, origins, and subsequent execution |
| 3 | Prototype and validate Windows/Linux OS connections | Argv, concurrent transfer, deadlines, descendants, cleanup failures, handle/fd lifetimes |
| 4 | Align C/LLVM/QBE/WAT/Native and artifact contracts | Support matrix, host compatibility, unoptimized/SSA paths, execution comparisons |
| 5 | Add runnable examples and paired documentation | Collect output, branch on nonzero exit, and continue with another action after failure |

During staged implementation, document unsupported routes instead of substituting successful no-op implementations. Do not claim a completed common feature until cross-route comparisons pass. Do not add unimplemented `.ceru` APIs to the runnable-example list now.

Tests must cover the following cases.

- Empty arguments, quotes, spaces, Japanese text, rejected NUL; explicit environment/cwd and no launch when unconnected.
- Zero/nonzero exit, OS-specific abnormal termination, missing target, execution denied; printing and another call after handling a failure value.
- Empty input/output, NUL/CR/LF/invalid UTF-8, early stdin close, short transfers, interruption, zero progress, and contract violations.
- Large simultaneous stdout/stderr, limits of zero/exact size/one byte over, insufficient result storage, and failure on only one output stream.
- Timeout, deadline/termination races, pipes surviving direct-child exit, descendants, failed termination/reaping, and disabled connections.
- Meaning unchanged with observation off or history exhausted; independent copies of partial result arrays.
- Comparisons using identical scripted responses, plus OS tests in isolated working areas. Do not repeat externally effectful real programs once per route without authorization.

OS fixtures should be independent programs controlling argv, output volume, exit, and waiting without a shell. Preserve independent expected-value calculations in projects such as Parallax. A common observation API for successful file operations in generated routes remains [separate work](file-io.en.md), not part of claiming subprocess completion.
