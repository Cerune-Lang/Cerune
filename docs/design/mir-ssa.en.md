# SSA construction for MIR

[日本語](mir-ssa.ja.md) · [IR stages](ir-stages.en.md) · [Issue #94](https://github.com/Cerune-Lang/Cerune/issues/94)

## Status and decisions

**The SSA representation, structural validator, observation text, and automatic MIR→SSA construction are implemented. Direct SSA execution, public CLI selection with `--ssa`, SSA→MIR/Native, and observation bundle integration are also implemented.** This document distinguishes the implemented scope from subsequent plans. Non-SSA MIR and its independent interpreter remain the baseline. Runnable examples, including [ssa_values.ceru](../../examples/ir_stages/ssa_values.ceru), compare IR, non-optimized MIR, SSA, and VM results.

SSA gives each value a single definition. Source `mut` remains valid: each reassignment gets a different value ID. At a merge, the selected edge supplies values to block arguments. Select construction explicitly and preserve the input MIR.

| Decision | Direction | Reason |
| --- | --- | --- |
| Role | Optional MIR representation conversion | SSA construction alone performs no optimization |
| Merge values | Typed block arguments and edge arguments | Show which values arrive from each if, short-circuit, or loop edge |
| Initial promotion | bool, all integer types, f32/f64 locals and temporaries | Handle scalar reassignment without changing heap lifetimes |
| Other values | Retain typed slots and current operations | Keep programs with strings, fixed/dynamic arrays, and named types within the same language |
| Execution baseline | Keep non-SSA MIR; compare with evaluation that directly consumes SSA | Do not establish agreement by running HIR or the input MIR instead |
| Native | After validation, explicitly convert SSA back to non-SSA MIR for existing LIR lowering | Observe edge transfers without rewriting Native at the same time |

This is **scalar SSA with residual slots**, not Memory SSA or SSA for individual array elements. The output header and local inventory must state that slots remain. Language-feature support and the range of promoted values are separate questions.

Block arguments also appear in the [MLIR structure](https://mlir.llvm.org/docs/LangRef/#blocks). They express merge values differently from [LLVM phi instructions](https://llvm.org/docs/LangRef.html#phi-instruction). This choice does not introduce either compiler as a dependency.

## Mapping from existing implementation

The inspected code establishes the contracts to preserve.

| Current implementation | SSA treatment |
| --- | --- |
| `Assign` and empty-path `Store` in [mir/mod.rs](../../src/mir/mod.rs) | Every write to a promoted local defines a new ValueId; retain reassignment as an explicit copy |
| `Operation::Copy` | Representation read, not a deep copy or retain; do not remove during SSA construction |
| Nonempty-path `Store`, `CheckIndex` | Use residual slots and SSA indices; preserve checks, RHS evaluation, and update order |
| `Call`, checked operations, allocation, retain/release, output | Retain their block and order, types, ownership mode, and failure origins |
| [validate.rs](../../src/mir/validate.rs) | Validate original MIR types, definite initialization, and preceding index checks before construction |
| [mir_executor.rs](../../src/mir_executor.rs) | Share numeric/heap semantics where appropriate; SSA evaluates its own control and value references |
| [text.rs](../../src/mir/text.rs), [observation bundle](observation-bundle.en.md) | Preserve the original MIR format; add separate SSA snapshots and transformation mappings |

Evaluate an assignment's RHS before registering the new definition. In `x = x + 1`, the RHS refers to the old x. Function arguments provide entry definitions and retain their ownership mode. Preparing residual-slot arguments must not introduce hidden ownership copies.

An SSA value ID identifies a static definition. Revisiting an instruction in a loop produces that iteration's dynamic value; it does not mean the instruction may execute only once.

## Representation and construction

Place the new responsibilities under `mir::ssa`. Reuse types, FunctionId, SourceOrigin, and operation semantics. Use a dedicated representation distinguishing ValueId, slot references, block arguments, and edge arguments, rather than marking an unchanged `mir::Program` as SSA. Refactor shared operand types only where implementation would otherwise require duplication.

Entry points and status follow. Unimplemented Rust API names will be finalized during implementation.

| Proposed entry point | Responsibility |
| --- | --- |
| `mir::ssa::construct(&mir)` (implemented) | Validate input MIR; return an independent original-MIR snapshot, SSA, and mappings. Record `scalar-ssa-v1` with no options |
| `mir::ssa::validate(&ssa)` (implemented) | Check definitions, uses, types, edges, residual slots, and origins |
| `mir::ssa::mapping::emit(&ssa)` (implemented) | Return observation text containing only correspondence with original MIR; saved separately in bundles |
| `mir::ssa::text::emit(&ssa)` (implemented) | Validate, then return `Result<String, mir::Error>` with `Cerune scalar SSA v0.1` observation text, not a loading format |
| `ssa_executor::run(&ssa)` (implemented) | Directly evaluate SSA blocks, values, and slots; return `Result<String, ExecutionError>` under existing output/failure contracts |
| `mir::ssa::lower(&ssa)` (implemented) | Return `Lowered { program, mapping }`: non-SSA MIR with parallel edge copies and observation mapping text |

The implemented `ssa::Program` holds an independent original-MIR snapshot and SSA functions, values, and blocks. It distinguishes `Operand::Value` from `Operand::Slot` while sharing `mir::Operation<R>` / `InstructionKind<R>`. Existing MIR uses the default `R = LocalId`; operation semantics and existing observation text stay unchanged.

Structural validation also validates original MIR and checks reachable-block coverage, instruction order, original IDs/origins, and retained unreachable blocks. After SSA dominance checks, it shares existing MIR type, slot-initialization, and index-check validation. The internal reference projection exists only for validation: it is neither SSA→MIR lowering with parallel copies nor an execution route. Block arguments become initialized on entry and invalidate prior-iteration index-check facts involving those values.

These checks establish structural and mapping consistency. They do not guarantee that operation contents or edge selection preserve the original semantics; execution comparisons and proofs follow automatic construction. Initial mappings preserve one original block and its instruction order. Recording deleted or combined instructions for optimization is unimplemented.

Implemented automatic construction proceeds in the following order.

1. Validate MIR and compute CFG reachability from each function entry. Follow both edges even for constant conditions; do not fold constants.
2. Classify promoted locals and residual slots. Collect definitions, reads, and values required along edges.
3. Compute dominance (every entry-to-use path passes through the definition) and dominance frontiers. Place arguments in the iterated dominance frontier of each local's definitions where that local is live on entry. Include backedges and iterate to a fixed point.
4. Rename definitions along the dominator tree and supply arguments on each edge. Process instructions in their original order. Fix argument/local/block traversal order; numbering must not depend on hash iteration.
5. Structurally validate the result. Tests independently compare original MIR, operations, edges, origins, and the latest definition at each read. Direct SSA execution is also compared with HIR, non-SSA MIR, and VM output, failure origins, and prior output.

Avoiding arguments for locals not live on entry prevents invented undefined values; it does not delete source computations or copies. Initially, do not simplify created arguments even when all incoming values are identical.

### Direct SSA execution

[ssa_executor.rs](../../src/ssa_executor.rs) directly evaluates SSA blocks, edges, ValueIds, and residual slots. It does not execute the original MIR body or delegate to the VM. The original snapshot supplies type definitions, signatures, slot types, heap budgets, and origin mappings; execution leaves it unchanged.

Individual numeric, string, array, and ownership operations share [operation semantics](../../src/mir_executor/semantics.rs) with the MIR interpreter. Each executor controls its own CFG, value references, and function dispatch. Read all incoming edge values before binding destination arguments simultaneously, without adding ownership copies or allocations.

The diagnostic API exposes the following information.

| Information | Meaning |
| --- | --- |
| `ErrorKind` | Shared MIR categories: validation failure, language runtime failure, or internal inconsistency (`InvalidMir`) |
| `origin()` / `runtime_failure()` | Failing operation's NodeId, SourceId, Span, and failure code; callers do not overwrite callee origins |
| `location()` | SSA FunctionId/BlockId and original MIR BlockId/InstructionId |
| `output()` | Output preceding failure |

Validate the entire SSA program before producing output. Each run uses fresh frames and heaps, and successful completion checks for unreleased owned storage. Finite-fixture SSA Lean proofs are covered by the experiment below; general proofs and individual optimization passes follow later. Execution comparisons are not a general proof of the transformation.

### Unreachable blocks

Current MIR retains unreachable statements and helper blocks, without the same initialization guarantees as reachable code. Do not invent zero, undef, or poison values to make these blocks satisfy SSA.

Convert the entry-reachable portion and retain unreachable blocks in a separate section of the same snapshot as **unconverted original MIR records**. Keep all original instructions, edges, origins, and references into the reachable portion. Mark mappings as “retained because unreachable,” distinct from deletion. Accordingly, do not call the entire snapshot fully SSA.

SSA evaluation executes only the reachable SSA section. An executable edge into the unconverted records is a validation error. A future CFG pass activating such a section must recheck initialization and convert it. Conversion back to MIR restores unreachable records too, remapping ID references.

### Edge arguments

For `jump join(a, b)`, read **both** outgoing values before simultaneously binding the destination arguments. Edge arguments are existing value IDs, never embedded expressions, calls, or slot reads. A branch transfers along the selected edge only. Keep edge identities and arguments distinct even when then/else target the same block.

Sequential assignments for a loop's `jump head(right, left)` can destroy the old values needed for a swap. When returning to non-SSA MIR, implement parallel copies using temporaries. Allocate MIR Temporary destinations for SSA definitions and block arguments. Do not misrepresent repeatedly written block arguments as Binding initializers; preserve source binding names in the mapping. If the source branches to multiple destinations, place copies in a helper block specific to the selected edge. Do not add output, allocation, retain, or release there.

### SSA→MIR and Native

[lower.rs](../../src/mir/ssa/lower.rs) validates SSA, converts it into per-value temporaries and parallel edge copies, and validates the resulting MIR with the ordinary validator. It does not select the original MIR body for execution. Input snapshots remain unchanged.

| Input | Lowering and observation |
| --- | --- |
| SSA values/block arguments | Allocate a dedicated Temporary per value, recording its original LocalId |
| Function parameters | Keep signature Bindings; copy into SSA temporaries in an added entry block, without extra ownership operations |
| Edges with arguments | Use a helper block per edge: capture every input, write every destination, then jump to the original target |
| Original instructions/failures | Retain original InstructionId and SourceOrigin; identify added copies as `ssa-edge-copy` / `ssa-parameter-copy` |
| Unreachable blocks | Restore original block numbers, instructions, and references; reachable SSA blocks occupy their original MIR block positions |
| Native | Pass `Lowered.program` to existing MIR→LIR→ASM/Object generation |

Argument-free edges need no helper. Every argument-carrying jump or branch uses parallel copies; then/else remain distinct even with the same target. No storage reuse or copy removal occurs, so stack usage and instruction counts can increase. Logical string/array budgets and ownership operations stay unchanged.

The mapping text is `Cerune SSA lowering mapping v0.1`, recording `ssa-lower-v1`. Follow SSA values to MIR locals, SSA blocks to original-numbered MIR blocks, edges to helper/read/write instructions, and parameters to entry copies. Existing MIR→LIR annotations and Object symbols continue the correspondence.

## Example representation

[ssa_values.ceru](../../examples/ir_stages/ssa_values.ceru) chooses a value with if, adds 0 and 2 in a loop, and swaps two values three times in a separate loop. IR/non-SSA MIR/SSA/VM routes should print `14\n16\n20\n10\n`.

The following is **explanatory shorthand, not current generated output or a finalized text grammar**. It omits literal/copy operations and origin annotations. Actual SSA construction will retain the original operations.

```text
Non-SSA:
  then: total = checked_add(total, 2); jump join
  else: total = checked_add(total, 4); jump join
  join: ... use total ...

Proposed SSA:
  then: v_then = checked_add(v_seed, 2); jump join(v_then)
  else: v_else = checked_add(v_seed, 4); jump join(v_else)
  join(v_total: u8): ... use v_total ...

Loop backedge:
  head(v_left: u8, v_right: u8, v_index: u8):
    ...
    jump head(v_right, v_left, v_next_index)
```

The [Rust API example](../../experiments/ssa/README.en.md) prints hand-authored SSA without arguments, or automatically constructs SSA from actual MIR when given a source file. Both modes display separate `original-mir` and `ssa` sections. Explicit `--run source.ceru` directly executes SSA.

```sh
cargo run --quiet --example ssa_model -- examples/ir_stages/ssa_values.ceru
cargo run --quiet --example ssa_model -- examples/ir_stages/owned_values.ceru
cargo run --quiet --example ssa_model -- --run examples/ir_stages/ssa_values.ceru
```

For the current `ssa_values`, both if edges supply `bb3(v13)`; the loop condition `bb4(v16, v17)` receives total and index, and update block `bb7(v25)` receives the iteration's total. Follow `mir-iN` and `original-local` back to original MIR. These IDs describe this source and construction version, not a promise of stable IDs across future versions.

These commands are available for Cerune source today.

```sh
cargo run --quiet -- run examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru --ssa
cargo run --quiet -- run-vm examples/ir_stages/ssa_values.ceru
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru -o target/ssa_values.mir.txt
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru --ssa -o target/ssa_values.ssa.txt
cargo run --quiet -- observe examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu -o target/ssa-observation
```

## Validation and semantic preservation

Require these structural checks.

| Check | Rejected examples |
| --- | --- |
| Unique definitions and function-local references | Duplicate ValueId, another function's value, unknown value |
| Dominance and within-block order | Using a branch-only value directly after a merge, reading a later definition first |
| Arguments and edges | Wrong argument count/type, entry/signature mismatch, unknown block |
| Initialization and residual slots | Uninitialized slot read, indexed store before checks, ownership-mode mismatch |
| Origins and mappings | Missing failing source operation, arbitrarily selecting one origin from a set |
| Unreachable section | Executable edge into an unconverted record |

A block argument is defined at block entry. Incoming arguments are used at the predecessor's terminator. Apply the same rules to self-loops rather than assuming all arguments dominate automatically. Residual slots retain existing MIR type, initialization, and index-check obligations. Structural SSA validation does not prove heap alias or lifetime safety.

Compare return values, output bytes, failure codes, NodeId/SourceId/Span, prior output, evaluation order, short circuit, and copy independence. Preserve separate logical string/array budgets and allocation/release timing. Do not change floating-point association, NaN, or negative zero. Keep unused checked operations, calls, allocations, and copies in place.

The initial conversion does not speed up programs by adding, moving, or deleting operations. Except for block-argument transfer, it should correspond to the original dynamic operation sequence. Value/block IDs may change; source failure origins must not. SSA definition history must not become an API for externally mutating runtime state.

## Observation, CLI, and implementation order

Mappings record input/output snapshots, pass name/version/options/order, and original FunctionId/BlockId/InstructionId/LocalId to SSA values/blocks/edges. Auxiliary arguments identify the merged local; incoming values identify their edges. Keep a failing operation's single source origin separate from many-to-one provenance.

`emit-mir --ssa` displays original MIR and SSA; `run-mir --ssa` executes SSA directly. Heap budgets, default diagnostics, and `runtime-v1` diagnostics match the non-SSA route. Preserve defaults and reject the flag on unsupported execution/emit routes, duplicate flags, and values such as `--ssa false`. Do not enable SSA or optimization implicitly.

`emit-asm --ssa` / `emit-obj --ssa` explicitly pass through SSA→MIR. `observe --ssa` separately saves original MIR, SSA, reconstructed MIR, both mappings, and baseline/SSA-derived assembly. Manifest v3 records `scalar-ssa-v1`→`ssa-lower-v1` with inputs, outputs, and mappings; the optimization list stays empty. Default v1 output is unchanged. See [observation bundles](observation-bundle.en.md) for the format. General pass selection follows later.

Implement in these units.

| Order | Work | Completion criteria |
| --- | --- | --- |
| 1 (implemented) | Representation, structural validator, deterministic text | Validate hand-built branches, loops, parallel arguments, and residual slots; reject broken definitions/edges/initialization |
| 2 (implemented) | MIR→SSA and mappings | Straight-line, branch/short-circuit, loop, and residual-slot conversion. Check baseline preservation, operations/edges/origins/latest definitions, unreachable records, and determinism for all runnable examples |
| 3 (implemented) | Direct SSA evaluation, CLI, and bundle integration | Compare HIR/MIR/SSA/VM using baseline examples and existing MIR/runtime tests, including heap budgets, prior output, and failure origins |
| 4 (implemented) | SSA→MIR and Native | Observe parallel copies/helper edge blocks; compare ASM/Object execution on Windows/Linux |
| 5 (finite fixtures implemented) | Lean correspondence | Check concrete MIR→SSA→MIR fixtures with independent SSA control semantics; see below |
| 6 (designed, implementation deferred) | [First constant-folding pass](ssa-constant-folding.en.md) | Replace successful block-local integer/bool operations; retain original MIR, before/after SSA, and replacement evidence. Resume with the Rust API and records; see the [roadmap](language-roadmap.en.md) for current priorities |

While intermediate implementations lack operations, diagnose them explicitly rather than silently falling back to input MIR execution. Public SSA execution should target current language features, including comparisons involving residual slots.

## Finite-fixture Lean verification

The [proof experiment](../../experiments/lean/README.en.md#ssa-verification) exports actual baseline MIR, SSA, and reconstructed MIR separately. The SSA evaluator independently handles values, selected edges, and binding after reading all incoming arguments; individual operations and instruction execution share the existing MIR model. SSA evaluation calls neither baseline MIR nor lowering.

The implemented checks have the following scope.

| Item | Coverage |
| --- | --- |
| Programs | Seven existing fixtures (increment, if/short-circuit, while, break/continue, nesting, for, update overflow), plus a join and three swaps; one additional SSA API fixture replaces the swap backedge with direct exchanges between header arguments |
| Correspondence and completion | All 256 u8 inputs for each fixture: baseline MIR = SSA = reconstructed MIR, including return values or overflow NodeId/SourceId/Span. Separate completion theorems require all cases to finish within explicit bounds |
| Bounds | 64 block visits for baseline MIR/SSA, 128 for reconstructed MIR. This proves neither bound inference nor general termination. Equal exhaustion in all routes fails the completion theorem |
| Independent properties | Join returns +3 for 0–127, +2 for 128–253, and fails at the return addition for 254–255. Swapping returns the original input. Correspondence transfers SSA properties to reconstructed MIR |
| Rejected mutations | Wrong join argument, missing loop update, sequential SSA binding, sequential lowered copies, failure origin, fuel, and user property |
| Execution comparison | Execute all three Lean models and compare every input with HIR, baseline MIR, SSA, reconstructed MIR, and VM using the same definitions as the proofs |
| Observations | Separately retain source, HIR, baseline MIR, SSA, reconstructed MIR, both mappings, Lean definitions/proofs, execution output, and rejected cases |

Pinned Lean 4.34.1 checks kernel proofs with exactly the standard `propext` axiom dependency; no `sorry`, custom axioms, or `native_decide`. The Rust exporters and shared operation semantics remain trusted parts requiring review and execution comparison.

Proofs cover selected pure u8 functions. Printing callers, general calls, residual slots, other numeric types, heap, output traces, Native, and general transformation correctness remain unproved. Unsupported operations are rejected rather than omitted. Default non-SSA routes, observation bundles, and optimization lists stay unchanged. Execution comparison, finite-fixture proofs, and general semantic-preservation proofs are separate results.
