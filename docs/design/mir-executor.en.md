# Independent MIR execution

[日本語](mir-executor.ja.md) · [IR stages](ir-stages.en.md)

## Role and current coverage

`run-mir` lowers completed Cerune IR into non-SSA MIR, then directly executes its locals, instructions, and blocks. It supports current types, functions, modules, generics, match, fixed/dynamic arrays, strings, ownership operations, and runtime diagnostics. The common frontend still resolves names, types, and ownership.

| Command | Executed representation | Role |
| --- | --- | --- |
| `run` / `run-ir` | Completed Cerune IR (HIR) | Default direct execution |
| `run-mir` | Non-SSA MIR | Independent route for comparing HIR→MIR semantics |
| `run-mir --ssa` | Scalar SSA with residual slots | Direct execution after explicit SSA construction |
| `run-vm` | Bytecode | VM instruction execution |
| `emit-mir` | None | Observe MIR operations, edges, and origins |

MIR execution does not reconstruct HIR or bytecode for execution. It does not introduce another build/release artifact. [Native consumes the same MIR](native-mir.en.md). [SSA construction and direct execution](mir-ssa.en.md) are available through Rust APIs and explicit `--ssa` selection. Optimization and external MIR loading remain unimplemented.

Use `run_with_output` for incremental delivery; existing `run` retains captured output. The CLI uses incremental delivery without printing output again. See [standard I/O](standard-io.en.md) for connections, partial writes, I/O failures, and read-only records.

## Implementation and sharing

Control flow is separate from atomic value operations.

| Implementation | Responsibility |
| --- | --- |
| [mir_executor.rs](../../src/mir_executor.rs) | Local frames, instructions, branch/jump/return, calls, and MIR failure locations |
| [mir_executor/semantics.rs](../../src/mir_executor/semantics.rs) | Individual instructions, operations, and ownership shared by MIR/SSA; each executor owns its CFG and value references |
| [runtime/value.rs](../../src/runtime/value.rs) | Value representation, arithmetic/comparisons/bit operations, and display shared by HIR/MIR/SSA; no control flow |
| [runtime/numeric.rs](../../src/runtime/numeric.rs) | Numeric conversion and rounding rules |
| [runtime/string_heap.rs](../../src/runtime/string_heap.rs), [array_heap.rs](../../src/runtime/array_heap.rs) | Logical ownership counts, allocation budgets, reclamation |
| [mir/validate.rs](../../src/mir/validate.rs) | Structure, types, initialization, and preceding index checks before execution |

Rust clones for reads and temporaries do not retain logical ownership or perform independent dynamic copies. Only explicit MIR allocation/copy loops/retain/release/free change language ownership. No ABI or host-OS inference switches execution behavior.

Lowering inserts a `synthetic=entry-main-call` instruction for the `main` call after the entry body. The MIR executor does not infer calls from function names. A callee failure retains the callee instruction and original source location.

Calls retain original MIR references, blocks, next instructions, locals, and return destinations in explicit frames rather than the host call stack. The [depth and allocation-failure contract](recursion.en.md) is shared by all four executors.

## API and failures

Use `run_mir(source)` for source text, or `modules::load(path)?.to_ir()`→`mir::lower(&hir)`→`mir_executor::run(&mir)` for imports. Every run has independent state and leaves HIR/MIR unchanged.

`MirRunError` separates construction from execution errors. Execution errors expose the following data.

| API | Contents |
| --- | --- |
| `kind()` | Validation failure, language failure code, or execution-time internal inconsistency |
| `origin()` | Original HIR NodeId and Span including SourceId |
| `location()` | MIR FunctionId (None for entry), BlockId, InstructionId; not overwritten by callers |
| `output()` | Prior output in captured APIs; empty in incremental APIs |
| `runtime_failure()` | Shared failure record for language failures only |

Validation errors precede execution and have empty output. CLI `--diagnostic-format runtime-v1` preserves the same code, original location, and prior output as other routes. Default diagnostics locate the source file; the Rust API exposes MIR instruction IDs for comparison with `emit-mir`.

## Ownership checks and limits

Successful execution must leave no live string/array heap storage. Leftover allocations produce an internal error instead of success. Failed execution reclaims its state, including partially initialized storage, when that state is dropped.

This is not a static proof of every lifetime path. Static alias/partial-initialization/retain/release path verification remains unimplemented. The API accepts compiler-constructed MIR and is not a sandbox or execution-time limiter for hostile input. The frontend still rejects recursive source programs. MIR re-entry uses independent explicit frames, with run_with_call_limit checking test-only IR cycles. Unbounded APIs do not prevent infinite execution of invalid IR.

## Comparisons and examples

[control_flow.ceru](../../examples/ir_stages/control_flow.ceru) illustrates short-circuit and loop edges. [owned_values.ceru](../../examples/ir_stages/owned_values.ceru) exercises functions, dynamic arrays, strings, and cleanup through continue.

```sh
cargo run --quiet -- run-mir examples/ir_stages/owned_values.ceru
cargo run --quiet -- emit-mir examples/ir_stages/owned_values.ceru -o target/owned_values.mir.txt
cargo run --quiet -- run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 47 --diagnostic-format runtime-v1
```

The last command intentionally exceeds the budget: only `begin` is printed, followed by `allocation-limit-exceeded` at the original argument-copy site. A passing failure test checks that stop against its expected result.

The [IR comparison tests](../../tests/ir_executor.rs) and [dynamic-array comparisons](../../tests/dynamic_arrays.rs) include MIR and check known output, codes, origins, and prior output. [MIR-specific tests](../../tests/mir_executor.rs) check that changing MIR itself changes execution, retain callee locations, and detect missing cleanup. Windows/Linux dynamic-array comparisons cover nine routes: HIR, MIR, VM, C, LLVM, QBE, WAT, ASM, and self-encoded objects.

HIR/MIR comparisons alone cannot detect bugs in their shared value operations, so tests also use VM/generated routes and known expectations. Execution agreement is testing, not formal proof or a performance-equivalence guarantee.
