# Standard I/O: execution, control, and observation

[日本語](standard-io.ja.md)

**Status: Output connections in all four executors, incremental CLI output, and explicit flush across execution/generated routes are implemented. Ordinary generated C/LLVM/QBE/WAT/Native writes are also supported. std:io line input is implemented in four executor Rust APIs. CLI and generated C/LLVM/QBE connect actual stdin. WAT supports [versioned input hosts](wat-input-host.en.md). Native input and shared generated-route records including successful operations remain unimplemented.** The first use case for [#89](https://github.com/Cerune-Lang/Cerune/issues/89) is a small interaction using bounded UTF-8 line input. Public names/types are defined by [virtual standard modules](standard-modules.en.md), with detailed failures in the [line-input contract](line-input.en.md).

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
| IR/MIR/SSA/VM and CLI | Retain capture APIs, add incremental output APIs; CLI delivers output during execution and flushes at completion; stdin connected | Windows console input and shared generated-route records |
| C/LLVM/QBE/Native | Per-operation requested/accepted counts and origins; partial writes, interruption, and termination | Shared recording including successful operations remains future work |
| WAT | Versioned write/format imports, partial failure, retry, and termination; private memory | Input host contract and shared recording including successful operations |
| Strings | Owned valid UTF-8; four executors and C validate input and account for budgets/ownership | Connect the remaining generated routes to the same input rules |
| Process tests | Verify output arrival/no duplication and CLI/C/LLVM/QBE dialogue that sends input only after the prompt | Extend dialogue comparisons to remaining generated routes |

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

Output failures use `ErrorKind::Output` / `VmErrorKind::Output`, not fabricated language-check `runtime-v1` records. Ordinary writes and explicit flush now cover existing routes. This does not imply complete stdin or successful-operation recording support.

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

Ordinary C/LLVM/QBE/WAT/Native writes support partial failure/retry handling. std:io line input is implemented in four executor Rust APIs. CLI and generated C/LLVM/QBE connect actual stdin. WAT supports [versioned input hosts](wat-input-host.en.md). Native input and shared generated-route records including successful operations remain unimplemented.

## Generated C/LLVM/QBE/Native writes (implemented)

Each common-IR output operation carries requested/accepted byte counts and immutable NodeId/Span. C/LLVM/QBE IR and Native LIR/ASM retain the origin, and arguments are evaluated once before output. Aggregate printing uses the existing common-IR operation sequence.

| Item | Contract and implementation |
| --- | --- |
| Acceptance/retry | Send only the remainder after a short write; retry only EINTR. Stop on zero progress, a response exceeding the request, or any other error |
| Termination | `io-v1 operation=write` retains operation-wide counts and origin, including a prefix ending inside UTF-8. No later execution, retransmission, or additional flush |
| Formatting | Format numbers with `snprintf` in a fixed 128-byte buffer. Use existing string data; count quoted output before sending it through a fixed 256-byte buffer. Do not copy the entire output into heap storage |
| Connection | The C compilation target or LLVM/QBE/Native target selects Linux `write` or Windows CRT `_write`. Limit each call to 65536 bytes. Never infer the generated target from the OS running Cerune |
| Windows | Binary mode also covers numeric-only output; do not translate LF into CR/LF. Preserve Japanese/NUL/CR/LF as bytes |
| Linux | Temporarily ignore SIGPIPE while writing and restore it on success. The same single-threaded artifact scope as explicit flush applies |

Count bytes for a common-IR output operation without resetting at OS-call splits. Acceptance does not guarantee display or durable storage. Ordinary stdout output bypasses stdio buffering, so C/LLVM/QBE/Native can detect a failure at `print`. WAT implements the same operation contract through its versioned host connection. `flush();` remains an explicit operation with its existing failure contract.

[Shared comparison tests](../../tests/support/generated_output.rs) for [C](../../tests/c_output.rs), [LLVM](../../tests/llvm_output.rs), and [QBE](../../tests/qbe_output.rs) supply each executor with an independent writer returning identical controlled responses, and generated C/LLVM/QBE/Native with a test execution connection. Compare prefixes, requested/accepted counts, origins, and termination. No observation API can replace responses. Cases include interruption, short writes, zero progress, invalid responses, quoted values, numbers, and multiple files. A real disconnected pipe is tested separately.

LLVM/QBE require an explicit target even for numeric-only `print(1);`. Generated `cerune.output.*` (LLVM) / `cerune_output_*` (QBE) helpers expose operation state, formatting, connection, retries, and termination. Shared successful-operation recording remains unimplemented; origin annotations do not affect execution. A failure in `snprintf` itself stops with an origin-bearing formatting diagnostic rather than inventing requested/accepted counts for text that was never produced.

Native retains LIR `Output { kind, newline }` with origins and uses the same `cerune_output_*` instructions for ASM and internal objects. [ASM tests](../../tests/native_output.rs) and [object tests](../../src/codegen/x86_64/output.rs) cover ordinary MIR, the SSA path, and origin annotations. Windows selects binary mode even for numeric-only output. The internal encoder supports `movb` for output bytes, checked against independent assembler bytes.

OS connections follow Linux [write](https://man7.org/linux/man-pages/man2/write.2.html) and Windows CRT [_write](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/write?view=msvc-170).

## WAT writes (implemented)

The [WAT output host contract v1](wat-output-host.en.md) routes ordinary output through `write_stdout_v1`. The host receives up to eight bytes by value; generated WAT distinguishes partial acceptance, interruption, zero progress, invalid responses, and failure. It preserves requested/accepted counts and NodeId/Span, stopping with an `io-v1` diagnostic on failure.

WAT formats integers and Booleans internally. Versioned float-format imports return the complete length and bytes into a fixed 128-byte buffer before writing, making requested counts exact. Memory remains private and `flush_stdout_v1` is unchanged. Old hosts missing required imports fail instantiation. See the [compatibility, formatting, and test details](wat-output-host.en.md).

## Initial input and output order

The public std:io read_line API takes an explicit byte limit and returns the ordinary ReadLine enum. The following contract is implemented in four executors; CLI/C/LLVM/QBE stdin is also connected within the [initial scope](stdin-connections.en.md); remaining generated routes follow.

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

Verify UTF-8/NUL/CR/LF preservation on Windows and Linux, and document console versus pipe behavior. The [output host contract](wat-output-host.en.md) specifies WAT migration; input imports need a separate design. C and similar backends remain emit-only; no implicit build route is introduced.

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

Stage 1 output connections, explicit flush, and ordinary C/LLVM/QBE/WAT/Native writes are implemented. The [bounded UTF-8 line-input contract](line-input.en.md) and shared Rust connection are implemented. std:io, string ownership, and input operations are connected to four executors. CLI/C/LLVM/QBE actual stdin is connected; versioned WAT input hosts are implemented; Native input comes next. Shared generated-route records including successful operations remain separate follow-up work.

## Details to resolve before implementation

The following design and integration work remains. Distinguish the settled line-input contract from routes that are not implemented.

- Remaining stage 1 work: shared generated-route I/O records including successful operations. Ordinary-write termination contracts cover all routes. Embedding partial-write/retry/record-limit behavior is implemented as specified above.
- Stage 2: [std:io](standard-modules.en.md), its public API, ordinary enum, and virtual source resolution are implemented. CLI/C/LLVM/QBE OS bindings are also implemented; versioned WAT input hosts are implemented. Connect the remaining Native route.
- The [line-input contract](line-input.en.md) fixes EOF checks, at most one extra consumed byte, stopping on errors, and classified diagnostics. A separate recoverable API remains future work.
- Whole-limit reservation and ownership transfer are connected to the four executors, C, LLVM, QBE, and WAT. Native input still needs matching budgets and release.
- Remaining console support and Native OS-error mappings. WAT input host ABI v1 and compatibility are defined in the [contract](wat-input-host.en.md).

## Runnable baseline today

[explicit_flush.ceru](../../examples/io_protocol/explicit_flush.ceru) covers flushing inside functions/loops and Japanese/NUL/CR/LF. [Generated-route tests](../../tests/explicit_flush.rs) compare output, controlled flush failures/origins, and output arrival before termination; on Linux they also check diagnostics with a disconnected socket.

[output_sequence.ceru](../../examples/io_protocol/output_sequence.ceru) checks function evaluation/output order and Japanese/NUL/CR/LF preservation. [Execution API tests](../../tests/execution_output.rs) compare observation settings, capacities, and partial writes; [CLI tests](../../tests/execution_output_cli.rs) verify output arrival before termination and prevent duplicate output.

The [input-value branching example](../../examples/io_protocol/README.en.md) keeps state and operation rules in Cerune using existing features. Values are scripted in the source; **it uses neither stdin nor flush**. Compare expected results for empty lines versus completion/failure, suppression of later operations, Japanese/NUL/newlines, and non-normalized text. Actual read and conversion failures belong to stage 2 and later tests.
