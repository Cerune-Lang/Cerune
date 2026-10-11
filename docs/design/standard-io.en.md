# Standard I/O: execution, control, and observation

[日本語](standard-io.ja.md)

**Status: output connections for HIR/MIR/SSA/VM and incremental CLI output are implemented. Stdin, language-level explicit flush, and generated-route I/O failure contracts remain unimplemented.** The first use case for [#89](https://github.com/Cerune-Lang/Cerune/issues/89) is a small interaction using bounded UTF-8 line input. API names, types, and detailed failure rules remain open below.

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
| IR/MIR/SSA/VM and CLI | Retain capture APIs, add incremental output APIs; CLI delivers output during execution and flushes at completion | Language-level flush and a contract shared with generated routes |
| C/LLVM/QBE/Native | Use existing runtime output functions; output-failure checks are not consistent | Common write/flush/failure contract with target-specific implementations |
| WAT | Host output imports; the test host captures output; memory stays private | Host contract returning input/flush/failure results; keep memory private |
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
| Flushing | The embedding caller explicitly invokes `Output::flush()`. Failure is returned without retry. This is not a language instruction and its origin is `None` |
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

Stage 1 output connections are implemented. Next, align language-level flush and write/flush/failure contracts for generated routes. Identify unsupported routes during partial implementation; parity across all existing routes is the final requirement.

## Details to resolve before implementation

The following remain undecided. Align the design and expected test results before implementing the relevant stage.

- Remaining stage 1 work: language-level flush syntax, failure classification/termination including generated routes, and shared I/O records. Embedding partial-write/retry/record-limit behavior is implemented as specified above.
- Stage 2: API names, result types, and standard-module distribution. Check whether existing enums/imports suffice; do not add implicit file lookup.
- EOF detection exactly at the limit, lookahead ownership, and recovery after exceeding the limit. Do not assume lookahead leaves OS input unconsumed.
- Budgets and peak simultaneous usage for scratch buffers, UTF-8 conversion, and completed strings. An existing `[u8]` element does not necessarily occupy one logical budget byte.
- Stable OS-error classification and diagnostics, supported console/pipe behavior, and WAT host ABI versioning/compatibility.

## Runnable baseline today

[output_sequence.ceru](../../examples/io_protocol/output_sequence.ceru) checks function evaluation/output order and Japanese/NUL/CR/LF preservation. [Execution API tests](../../tests/execution_output.rs) compare observation settings, capacities, and partial writes; [CLI tests](../../tests/execution_output_cli.rs) verify output arrival before termination and prevent duplicate output.

The [input-value branching example](../../examples/io_protocol/README.en.md) keeps state and operation rules in Cerune using existing features. Values are scripted in the source; **it uses neither stdin nor flush**. Compare expected results for empty lines versus completion/failure, suppression of later operations, Japanese/NUL/newlines, and non-normalized text. Actual read and conversion failures belong to stage 2 and later tests.
