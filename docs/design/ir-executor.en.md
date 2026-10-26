# Cerune IR Executor

[日本語](ir-executor.ja.md)

## Role and input

IR Executor is the official reference execution path for existing Cerune IR semantics. It reuses IR definitions, construction, type/name resolution, and common expansion. It neither lowers to Bytecode or backend IR nor delegates execution to the VM. There is no Whitebase-specific contract.

```text
Existing frontend and common expansion
                   ↓
             ir::Program
              ├─ IR Executor
              ├─ Bytecode → VM
              ├─ C / LLVM / QBE / WAT
              └─ Assembly / Native Object
```

Input is the completed program returned by compile_to_ir or modules::Compilation::to_ir. Existing generic, enum/match, aggregate display/equality, and string ownership expansion remain shared. IR construction, including constant evaluation, retains its current Bytecode/VM use. Execution after receiving IR is independent. Parsing textual .ceir files is a separate feature.

Use `run_with_output` for incremental delivery; existing `run` retains captured output. The CLI uses incremental delivery without printing output again. See [standard I/O](standard-io.en.md) for connections, partial writes, I/O failures, and read-only records.

## API and execution responsibilities

The entry point is `ir_executor::run(&ir::Program) -> Result<String, ExecutionError>`. A constructed IR can be run repeatedly without recompilation, with fresh state per call. Execution does not mutate IR, perform hidden optimization, or build a separate executable IR.

CLI `run <file>` and its alias `run-ir <file>` select direct execution; `run-vm <file>` selects the VM route. All three reuse the frontend for imports, diagnostics, and string budgets and support `--diagnostic-format runtime-v1`. Source API `run_ir(source) -> Result<String, IrRunError>` distinguishes compilation from execution failures. Execution paths are never selected silently.

- Walk statements and expressions directly; identify per-function bindings by BindingId.
- Execute structured if/while/for and distinguish return/break/continue. For-loop continue still executes its update.
- Preserve top-level and explicit-main entry rules.
- Evaluate arguments, operators, array elements, and constructor fields once from left to right; skip short-circuited operands.
- Check each assignment index before later indices or the right-hand side.

Private values contain typed numbers, bools, immutable string handles, and value arrays/products, without VM Value or bytecode Type. Keep nested mutable aggregate values independent while sharing immutable strings. Dynamic array element copies run through explicit common-IR loops rather than an opaque host deep copy. Execute enums as the common IR's tags and fields rather than introducing another language interpretation.

Execution retains references to original statements/expressions, BindingId bindings, intermediate values, and continuation positions in explicit frames. Function calls do not add host-recursive Rust frames. See the [recursion design](recursion.en.md) for allocation-failure handling and remaining resource limits.

## Shared semantics and ownership

Numeric conversion, float formatting, and string allocation management are extracted into small runtime components. The VM uses compatible adapters, preserving its public APIs and errors. Do not share instruction dispatch, stacks, slots, or function execution.

Preserve integer ranges, u64, rounding/saturation, NaNs and signed zeros, UTF-8/NUL/CR/LF, and quoted display. Execute explicit string.retain/release independently of temporary Rust clones. Match live string-byte limits, old/new coexistence, and reclamation on failure.

Use known expected values, boundaries, and independent generated implementations as well as VM comparisons, so sharing code cannot by itself establish correctness.

## Failures and observability

Separate language failures from invalid-IR/internal errors. Language failures carry common FailureCode, NodeId, Span, and prior output. Do not replace a failing child expression/function's origin with its caller's origin, or treat invalid IR as an expected language failure.

Execute generated loops, checks, retention, and release as ordinary IR. Observe through existing emit-ir and source-aware failures. Do not expose mutation of running bindings or ownership through observation APIs. Accepting prebuilt IR permits future execution-time comparisons separately from compilation; no speed advantage is promised.

## Implementation and verification order

1. Shared components, scalar operations, and structured control.
2. Functions, arrays, products, and expanded enum/match.
3. Dynamic strings, ownership/budgets, error origins, CLI/API.
4. Compare existing examples and success/failure fixtures with known expectations, VM, and generated routes; update bilingual capability tables.

Intermediate stages are not feature completion. Establish current-feature parity through tests before returning to new language features or dynamic-array implementation.

## Implemented coverage and limits

The stages above are implemented for current completed IR.

| Support | Contents |
| --- | --- |
| Values and operations | All integer types, f32/f64, bool, strings; range checks, explicit conversion, rounding and saturation |
| Control and functions | if/while/for, short-circuiting, break/continue/return, top-level/main entry |
| Aggregates | Fixed/dynamic arrays, products, enums, nesting, value copies, indexed updates, equality and display |
| Common expansion | Imports, constants, type/length generics, array iteration, match expressions and guards |
| Strings | concat/byte_len, immutable contents, explicit retain/release, budgets and failure cleanup |
| Observation | emit-ir, NodeId, file-local Span, prior output and runtime-v1 |

`tests/ir_executor.rs` compares direct execution with VM bytecode lowered from the same completed IR and checks IR immutability. `tests/examples.rs` and shared fixtures check known expectations. `runtime_routes` and `string_heap_routes` also compare generated execution, origins, and budgets. `ir_execution.ceru` participates in C, LLVM, QBE, WAT, assembly, and object comparisons.

Textual IR loading, an embedding API for arbitrary function invocation, and pause/resume are unsupported. No intervention API is introduced. Recursion uses explicit frames and common call-depth checks. [Dynamic arrays](owned-arrays.en.md) were added subsequently to IR/VM; allocation, copy loops, and release execute from common IR. The public Rust API expects frontend-completed IR, not arbitrary hand-built IR requiring full validation. Detected structure/type/ownership inconsistencies produce `InvalidIr`; unexpanded `Let`/`Conditional` nodes are not silently expanded or delegated to the VM.

Implementation is split into `src/ir_executor/machine.rs` (frames, continuation positions, structured control), `src/ir_executor.rs` (evaluated-operand operations and diagnostics), `src/runtime/value.rs` (private values and atomic operators shared with MIR), and `src/runtime/{numeric,float_output,string_heap,array_heap}.rs` (shared components). VM adapters remain under `src/vm`; direct execution does not depend on VM values or instruction sequences.
