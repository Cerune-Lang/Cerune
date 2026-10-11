# Standard I/O: execution, control, and observation

[日本語](standard-io.ja.md)

**Status: design direction. Stdin, explicit flushing, and output delivery during execution are not implemented.** The first use case for [#89](https://github.com/Cerune-Lang/Cerune/issues/89) is a small interaction using bounded UTF-8 line input. API names, types, and detailed failure rules remain open below.

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
| IR/MIR/SSA/VM and CLI | Collect output as a string, displayed after completion or failure | Deliver output during execution and flush; retain embedding APIs that capture output |
| C/LLVM/QBE/Native | Use existing runtime output functions; output-failure checks are not consistent | Common write/flush/failure contract with target-specific implementations |
| WAT | Host output imports; the test host captures output; memory stays private | Host contract returning input/flush/failure results; keep memory private |
| Strings | Owned values containing valid UTF-8 | Strict input-byte validation, budgets, and ownership rules |
| Process tests | Closed stdin and final-output capture | Interactive tests that send input only after receiving the prompt |

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

Stage 1 is the next implementation unit. Identify unsupported routes during partial implementation; parity across all existing routes is the final requirement.

## Details to resolve before implementation

The following remain undecided. Align the design and expected test results before implementing the relevant stage.

- Stage 1: output failures as values versus termination, partial-write/retry rules, public flush syntax, I/O record granularity/capacity, and migration from existing output APIs.
- Stage 2: API names, result types, and standard-module distribution. Check whether existing enums/imports suffice; do not add implicit file lookup.
- EOF detection exactly at the limit, lookahead ownership, and recovery after exceeding the limit. Do not assume lookahead leaves OS input unconsumed.
- Budgets and peak simultaneous usage for scratch buffers, UTF-8 conversion, and completed strings. An existing `[u8]` element does not necessarily occupy one logical budget byte.
- Stable OS-error classification and diagnostics, supported console/pipe behavior, and WAT host ABI versioning/compatibility.

## Runnable baseline today

The [input-value branching example](../../examples/io_protocol/README.en.md) keeps state and operation rules in Cerune using existing features. Values are scripted in the source; **it uses neither stdin nor flush**. Compare expected results for empty lines versus completion/failure, suppression of later operations, Japanese/NUL/newlines, and non-normalized text. Actual read and conversion failures belong to stage 2 and later tests.
