# Standard I/O: execution, control, and observation

[日本語](standard-io.ja.md)

**Status: Output connections in all four executors, incremental CLI output, and explicit flush across execution/generated routes are implemented. Ordinary generated-C writes are also supported. Stdin and LLVM/QBE/Native/WAT write-failure contracts remain unimplemented.** The first use case for [#89](https://github.com/Cerune-Lang/Cerune/issues/89) is a small interaction using bounded UTF-8 line input. API names, types, and detailed failure rules remain open below.

## Acceptance principles

These three conditions apply to APIs, IR, implementation, and tests.

| Condition | Requirement |
| --- | --- |
| Observability | Trace input, output, failure, and transformation through operation order and source origins |
| Separate observation from control | Observation APIs are read-only; they neither supply input nor replace responses or mutate state |
| Consistent semantics | HIR, MIR, SSA, VM, and generated routes agree given the same input and external responses |

Enabling observation must not change language results, external operation sequences, or failure reasons. It need not take identical time. Unoptimized representations and explicitly transformed representations remain separately observable.

## Current implementation and gaps

The investigation covers `src/ir_executor.rs`, `src/mir_executor/`, `src/ssa_executor.rs`, `src/vm.rs`, `src/main.rs`, `src/codegen/`, and `tests/support/`.

| Area | Current behavior | Required change |
| --- | --- | --- |
| IR/MIR/SSA/VM and CLI | Retain capture APIs, add incremental output APIs; CLI delivers output during execution and flushes at completion | A write-failure contract shared with generated routes |
| LLVM/QBE/Native | Use existing runtime output functions; output-failure checks are not consistent | Align partial writes and retries; explicit flush is implemented |
| C | Per-operation requested/accepted counts and origins; partial writes, interruption, and termination | Shared recording including successful operations remains future work |
| WAT | Host output imports; the test host captures output; memory stays private | Flush-status import is implemented; input and write-failure host contracts remain |
| Strings | Owned values containing valid UTF-8 | Strict input-byte validation, budgets, and ownership rules |
| Process tests | Verify output arrival before termination and no duplicate output | Interactive tests that send input only after receiving the prompt |

Existing language-failure diagnostics do not specify OS read/write failure semantics. Extend the contract and diagnostic coverage explicitly.

## Separate responsibilities

The connection that performs I/O is distinct from the facility that reads its records.

```text
Cerune program → explicit I/O operation → execution I/O connection → OS or controlled response
                         ↓ request/result records
                    read-only observation
```

- Execution connections return input and accept output. Select controlled comparison connections explicitly before execution.
- Observers read records. Observation callbacks do not replace input or decide whether execution continues.
- Share operation contracts, value conversion, and record formats. Do not delegate independent executors' control flow to the VM or another executor.
- Retain existing capture APIs without implicitly connecting to process stdin. Unconfigured input is distinct from EOF.
- A CLI delivering output incrementally must not print captured output again on success or failure.
- Compilation, constant evaluation, emit operations, and current observe operations do not perform program I/O.

## Implemented output connections

All four executors provide `run_with_output(program, &mut Output<W>)`. The VM entry retaining source origins is `run_bytecode_with_output`. Existing `run` APIs still capture output in their return values or errors' `output()`.

| Item | Current contract |
| --- | --- |
| Connection | `runtime::output::Output<W>` writes to a `std::io::Write` selected before execution; it has no access to stdin or program state |
| Incremental execution | `run_with_output` returns `()` on success, without collecting output internally; error `output()` is empty. Accepted bytes remain at the destination |
| Writes | Send the remainder after short writes; retry only `Interrupted`. Zero progress, invalid accepted counts, and other errors stop execution |
| Partial failure | Retain operation, `ErrorKind`, requested/accepted byte counts, and NodeId/Span, even within a UTF-8 character. No later operations, retransmission, or additional flush |
| Flushing | The embedding caller explicitly invokes `Output::flush()`. Failure is returned without retry. This host operation has origin `None`; language `flush();` carries NodeId/Span |
| CLI | `run`, `run-ir`, `run-mir` (including SSA), and `run-vm` deliver output incrementally. Unless the connection itself failed, flush at both success and language failure. Report both a program failure and a subsequent flush failure |
| Records | Explicitly enable with `with_trace_limit(writer, n)`. Read completed logical output/flush operations, order, origins, requested/accepted counts, and failures through read-only `events()` |
| Record limit | 0 disables recording. Reaching the limit or failing to allocate a record sets `trace_truncated()` and execution continues. Records contain neither content nor OS error messages |

Records correspond to common-IR output operations. Aggregate printing expands into multiple operations. OS write chunk counts are not the recording unit. There is no live notification of pending operations. To compare output content, collect bytes in an explicitly selected writer.

I/O failures use `ErrorKind::Output` / `VmErrorKind::Output`, not fabricated language-check `runtime-v1` records. Do not claim parity across all routes until generated-route failure contracts agree.

Embedding code can select a destination and record limit before execution.

```rust
use cerune_lang::{compile_to_ir, ir_executor, runtime::output::Output};

let program = compile_to_ir("print(42);").unwrap();
let mut output = Output::with_trace_limit(Vec::<u8>::new(), 32);
ir_executor::run_with_output(&program, &mut output).unwrap();
output.flush().unwrap();
assert_eq!(output.events().len(), 2); // write, host flush
assert_eq!(output.into_inner(), b"42\n");
```

`Output` owns neither program values nor the program and has no observation callbacks. The writer is an execution connection, not an observation API. Its provider is responsible for any external effects performed by that writer.

## Explicit flush (implemented)

`flush();` is a built-in statement with no arguments or result; `flush` is reserved. It hands pending stdout output to the execution connection at that point. It does not guarantee screen rendering or durable storage.

Operations and failures follow these rules.

| Item | Contract |
| --- | --- |
| Order and origin | Retain `flush.stdout` in HIR/MIR/SSA/bytecode and the operation with NodeId/Span in LIR/generated code. Observation does not change operation counts or order |
| Success | Continue execution. Capture APIs succeed because output has already been handed to the capture destination |
| Failure | Stop before later operations, without retransmission or another flush. Diagnose the operation and source origin. Prior output cannot be rolled back; bytes delivered to the OS during flushing are unknown |
| Routes | HIR/MIR/SSA/VM, C/LLVM/QBE/WAT, and Windows/Linux ASM/native objects, including the SSA path |
| Target | LLVM/QBE require an explicit target even for a program containing only `flush();`. Never infer it from the compiler host |
| Observation | The four executors record success/failure through `Output`. Shared success-operation recording for generated routes remains future work |

Stable failure records use `io-v1`, separately from language-check `runtime-v1`.

```text
cerune: io-v1 operation=flush requested=0 accepted=0 node=N[ file=F] bytes=S..E
```

`[ file=F]` denotes an optional multi-file origin field; literal brackets are not emitted. Zero requested/accepted counts mean flush accepts no new payload, not that zero bytes reached the OS. The record excludes OS-specific error classifications; the four executors retain `ErrorKind` in their Rust API. Host `Output::flush()` has no source origin and records `origin=host`.

The existing CLI option `--diagnostic-format runtime-v1` selects `runtime-v1` for language failures and `io-v1` for I/O failures. Write failures retain `operation=write` and requested/accepted counts. Default CLI diagnostics remain human-readable with locations. Generated code writes `io-v1` to stderr before terminating. Delivery of that diagnostic cannot be guaranteed if stderr itself fails.

### Generated-route connections

C/LLVM/QBE/Native flush only stdout and stop on failure without stdio flushing again during process exit. On Linux they temporarily ignore SIGPIPE while flushing and restore its prior disposition on success. This is a process-wide setting, so the contract covers current single-threaded executable artifacts; embedding in an external multithreaded host is not defined. See the Linux [fflush](https://man7.org/linux/man-pages/man3/fflush.3.html) and [signal](https://man7.org/linux/man-pages/man7/signal.7.html) rules.

WAT artifacts containing explicit flush add `cerune.flush_stdout_v1() -> i32`. The host returns 0 on success and any nonzero status on failure. Failure writes the record through existing `write_error_byte` and traps. Missing required imports fail instantiation rather than silently succeeding. Internal memory stays private. The test host synchronously captures output and reports successful flushing.

Ordinary C writes support partial failure/retry handling; LLVM/QBE/Native/WAT do not yet implement it. Completed explicit-flush support does not imply completed I/O-failure parity. Stdin remains unimplemented.

## Generated-C writes (implemented)

Each common-IR output operation carries requested/accepted byte counts and immutable NodeId/Span. C IR retains the origin, and arguments are evaluated once before output. Aggregate printing uses the existing common-IR operation sequence.

| Item | Contract and implementation |
| --- | --- |
| Acceptance/retry | Send only the remainder after a short write; retry only EINTR. Stop on zero progress, a response exceeding the request, or any other error |
| Termination | `io-v1 operation=write` retains operation-wide counts and origin, including a prefix ending inside UTF-8. No later execution, retransmission, or additional flush |
| Formatting | Format numbers with `snprintf` in a fixed 128-byte buffer. Use existing string data; count quoted output before sending it through a fixed 256-byte buffer. Do not copy the entire output into heap storage |
| Connection | The C compilation target selects Linux `write` or Windows CRT `_write`. Limit each call to 65536 bytes. Never infer the generated target from the OS running Cerune |
| Windows | Binary mode also covers numeric-only output; do not translate LF into CR/LF. Preserve Japanese/NUL/CR/LF as bytes |
| Linux | Temporarily ignore SIGPIPE while writing and restore it on success. The same single-threaded artifact scope as explicit flush applies |

Count bytes for a common-IR output operation without resetting at OS-call splits. Acceptance does not guarantee display or durable storage. Ordinary stdout output bypasses stdio buffering, so C can detect a failure at `print`. Other generated routes will adopt this contract in subsequent steps. `flush();` remains an explicit operation with its existing failure contract.

[Comparison tests](../../tests/c_output.rs) supply each executor with an independent writer returning identical controlled responses, and generated C with a test execution connection. Compare prefixes, requested/accepted counts, origins, and termination. No observation API can replace responses. Cases include interruption, short writes, zero progress, invalid responses, quoted values, numbers, and multiple files. A real disconnected pipe is tested separately.

OS connections follow Linux [write](https://man7.org/linux/man-pages/man2/write.2.html) and Windows CRT [_write](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/write?view=msvc-170).

## Initial input and output order

The intended user-facing operation reads a UTF-8 line with an explicit byte limit. The following defines the semantic direction, not finalized public function names or result types.

| Item | Direction |
| --- | --- |
| Strings | Strict UTF-8; no replacement of invalid bytes, Unicode normalization, or NUL termination |
| Line boundary | Read through LF and retain LF or CRLF; a standalone CR is content |
| EOF | Return an unterminated final line once, then EOF; distinguish empty lines from EOF |
| Limit | Explicit byte count; validate invalid limits before external reads |
| Failure | Distinguish read, UTF-8 conversion, limit, and budget failures; do not pretend consumed input was rewound |
| Interaction | Write the prompt, complete flushing, then wait for input; failed flushing must prevent the read |
| Partial output | Distinguish requested bytes, accepted bytes, and flush results; externally written output is not rolled back |

Successful flushing hands output to the connection; it does not guarantee screen rendering or durable storage. Do not rely on implicit flushing from newlines or reads; keep the order traceable in IR.

## Connecting each route

Executors will support captured output, real OS connections, and controlled comparison connections. Use existing frontend expansion and ownership processing rather than a separate semantic resolver for input.

| Route | Implementation condition |
| --- | --- |
| HIR/MIR/SSA/VM | Retain operations and origins; SSA and other transformations preserve I/O order and count |
| C/LLVM/QBE | Express the same contract in generated code; do not infer LLVM/QBE targets from the compiler's host OS |
| LIR/ASM/native objects | Use the explicit target's ABI and I/O helpers, retaining visibility into generation |
| WAT | Supply input through imports returning values/status; do not expose internal heap pointers or mutable memory |

Verify UTF-8/NUL/CR/LF preservation on Windows and Linux, and document console versus pipe behavior. Resolve WAT host compatibility before changing imports. C and similar backends remain emit-only; no implicit build route is introduced.

## Observation and comparison

Following [#104](https://github.com/Cerune-Lang/Cerune/issues/104), give each route its own identical controlled input/responses and isolated output destination. Do not hand the stream consumed by one route to the next. Test real OS connections separately.

Operation records associate sequence numbers, operation kinds, requests/results, and source NodeId/Span. Sequence numbers distinguish repeated execution. NodeIds are not assumed stable across different compilations.

Compare language operations and results, not identical OS-call chunk counts. Preserve the effects of partial reads/writes and failures on results. Input-content recording is opt-in and is not automatically added to current failure diagnostics. If observation capacity is exhausted, indicate missing records rather than stopping the program or changing I/O.

## Implementation order and completion criteria

The order follows dependencies of interaction. General record/replay, an effect system, and a complete standard library are not prerequisites.

| Stage | Work | Checks |
| --- | --- | --- |
| 1 | Separate captured output from execution I/O connections; align output/flush/failure contracts | Existing-output compatibility, incremental delivery, no duplicate output, partial failure, equal behavior with observation on/off |
| 2 | Finalize bounded line-input details and implement from common IR through all routes | EOF/empty/final lines, strict UTF-8, budgets, failures/origins, ownership and release |
| 3 | Add real interactive examples and route comparisons | Do not send input until the prompt arrives; compare identical input/responses/order on Windows/Linux and the WAT host |

Stage 1 output connections are implemented. Explicit flush is also implemented. Ordinary C writes are also implemented. Next, extend partial-write/retry/termination contracts to LLVM/QBE/Native/WAT. Identify unsupported routes during partial implementation; parity across all existing routes is the final requirement.

## Details to resolve before implementation

The following remain undecided. Align the design and expected test results before implementing the relevant stage.

- Remaining stage 1 work: generated-write failure classification/termination and shared I/O records including successful operations. Embedding partial-write/retry/record-limit behavior is implemented as specified above.
- Stage 2: API names, result types, and standard-module distribution. Check whether existing enums/imports suffice; do not add implicit file lookup.
- EOF detection exactly at the limit, lookahead ownership, and recovery after exceeding the limit. Do not assume lookahead leaves OS input unconsumed.
- Budgets and peak simultaneous usage for scratch buffers, UTF-8 conversion, and completed strings. An existing `[u8]` element does not necessarily occupy one logical budget byte.
- Stable OS-error classification and diagnostics, supported console/pipe behavior, and WAT host ABI versioning/compatibility.

## Runnable baseline today

[explicit_flush.ceru](../../examples/io_protocol/explicit_flush.ceru) covers flushing inside functions/loops and Japanese/NUL/CR/LF. [Generated-route tests](../../tests/explicit_flush.rs) compare output, controlled flush failures/origins, and output arrival before termination; on Linux they also check diagnostics with a disconnected socket.

[output_sequence.ceru](../../examples/io_protocol/output_sequence.ceru) checks function evaluation/output order and Japanese/NUL/CR/LF preservation. [Execution API tests](../../tests/execution_output.rs) compare observation settings, capacities, and partial writes; [CLI tests](../../tests/execution_output_cli.rs) verify output arrival before termination and prevent duplicate output.

The [input-value branching example](../../examples/io_protocol/README.en.md) keeps state and operation rules in Cerune using existing features. Values are scripted in the source; **it uses neither stdin nor flush**. Compare expected results for empty lines versus completion/failure, suppression of later operations, Japanese/NUL/newlines, and non-normalized text. Actual read and conversion failures belong to stage 2 and later tests.
