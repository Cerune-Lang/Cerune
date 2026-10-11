# HIR, MIR, LIR, and observable transformations

[日本語](ir-stages.ja.md) · [Issue #94](https://github.com/Hokutaka/Cerune/issues/94)

## Status and direction

The inspected baseline is `f39f55e`, merging Native dynamic arrays. **Common MIR types, validation, HIR lowering, and `emit-mir` are implemented. The [MIR interpreter](mir-executor.en.md) and `run-mir` are also implemented. [Native lowering from MIR](native-mir.en.md) is also implemented. SSA representation, structural validation, observation text, automatic construction, and direct execution through the Rust API are implemented. Public SSA CLI/bundle support is also implemented. SSA→MIR/Native is also explicitly selectable. Optimization passes follow later.**

| Stage | Current implementation | Direction |
| --- | --- | --- |
| AST | Source syntax and positions, feeding common name/type/generic processing | Do not introduce another AST executor |
| HIR | The completed `ir::Program` already serves this role: resolved types/references, ownership operations, structured control | Reuse Cerune IR rather than define its semantics twice |
| MIR | `mir::Program`, HIR lowering, validation, observation text, independent execution | Target-independent execution representation with typed temporaries, basic blocks, and explicit control flow |
| LIR | `codegen::x86_64::ir` serves a similar role: register operations, stack slots, frames, target | Organize Native lowering around it; do not claim it is already a fully lowered machine-instruction IR |
| Artifacts | Bytecode, C, LLVM, QBE, WAT, ASM, self-encoded COFF/ELF | Distinguish representations from completed artifacts in the [route design](owned-routes.en.md) |

`run` / `run-ir` continue to execute completed Cerune IR directly; `run-vm` executes bytecode. This classification does not rename the `ir` module, `emit-ir`, or `.ceir`. “HIR” names the existing IR's role.

SSA here means scalar SSA construction for common MIR, distinct from SSA representations in external LLVM/QBE output.

LIR generation exists, but dedicated observation text, structural validation, and direct execution do not. Tracing MIR→LIR through ASM annotations is distinct from independently observing or evaluating LIR bodies. See the [status inventory](development-status.en.md) and [#123](https://github.com/Cerune-Lang/Cerune/issues/123).

## Findings from the code

| Implementation | Current responsibility | Do not duplicate or misclassify |
| --- | --- | --- |
| [IR builder](../../src/ir/builder.rs) | Generic expansion, array-length resolution, sum/match expansion, semantic analysis, typed IR construction, aggregate and ownership lowering | Type inference, name resolution, and ownership rules inside MIR |
| [Common IR](../../src/ir/mod.rs) | Types, BindingId/FunctionId, NodeId/Span, if/while/for, array allocation/initialization/retain/release | OS calling conventions or physical pointer widths |
| [IR Executor](../../src/ir_executor.rs) | Direct structured statement/expression execution | Implementing MIR execution by returning to the HIR executor |
| [Bytecode](../../src/bytecode.rs) | Operand stack, local slots, jumps, per-instruction origins | Renaming bytecode as common MIR |
| [Native lowering](../../src/codegen/x86_64/lower.rs), [emitter](../../src/codegen/x86_64/emit.rs) | Stack/ABI layout and instruction selection; the emitter still expands checks and prologues | Claiming the current x86-64 IR already fixes every machine instruction |
| [Runtime](../../src/runtime) | Shared numeric rules, string/array storage, display, failure codes | A new universal execution engine that makes comparison routes identical |

File-based module resolution happens in [modules](../../src/modules.rs), preserving file origins before the common frontend. Required constant evaluation and generic expansion establish language semantics; they remain necessary without optimization and are distinct from optional optimization passes.

## Baseline and migration

HIR feeds direct execution, bytecode, and external backends; Native now goes through MIR→LIR. The following diagram describes the current implementation. Each executor receives a representation, not the previous executor's result.

```text
Source → frontend → HIR ─────────────→ IR Executor (reference)
                     │
                     └→ MIR (no optimization) ─→ MIR Interpreter
                              │
                              └→ x86-64 LIR → ASM → self-encoded object
                     └→ existing bytecode/C/LLVM/QBE/WAT (migration references)
```

After comparing independent MIR execution with existing routes, Native lowering was migrated to MIR. Known outputs, failures, origins, and ABI behavior were tested before and after migration on Windows/Linux, and the old HIR→Native conversion was removed. Unoptimized HIR→MIR→LIR remains the baseline.

Keep the unoptimized baseline snapshot and separate before/after snapshots for selected transformations. The future selectable path is shown below. Passes are optional, and SSA construction itself is distinct from optimization.

```text
Baseline: HIR → MIR → LIR → artifacts
Selected: HIR → HIR pass → MIR → SSA construction → MIR pass → LIR → LIR pass → artifacts
```

Do not hide selection and order behind a single opaque `-O2`. Record pass selection/order and correspondence, and reject unsupported combinations.

Evaluate VM and external-backend migration individually. C/WAT need a verified way to express structured control; LLVM/QBE need verified mappings to their external IRs. Their backend IRs are not all machine-dependent LIR. Reject MIR-pass selections on routes that do not consume them rather than silently ignoring them.

## Initial MIR contract

Start without SSA, allowing explicit assignments to typed locals and temporaries. The current Rust APIs are `mir::lower(&ir)`, `mir::validate(&mir)`, and `mir::text::emit(&mir)`.

| Element | What it records |
| --- | --- |
| Program/function | Correspondence to HIR types/functions, entry point, array/string budgets, and source string usage retained after constant evaluation (`source-strings`) |
| Block | Deterministic BlockId and instructions, ending in exactly one terminator such as jump/branch/return |
| Operand/place | Typed constants, temporaries, and locals; no physical addresses or CPU registers |
| Computation | Integer width/signedness, f32/f64, explicit conversions, existing aggregate types |
| Call/return | Resolved callee and left-to-right argument preparation; ownership transfer versus internal reads |
| Effects | Output, allocation, initialization, retain/release, free, and updates in execution order |
| Checks | Distinct checked arithmetic, conversions, indexing, ranges, and allocation, retaining reasons/origins |

Checked operations may remain typed MIR instructions initially. They need not all become machine comparisons and trap branches. A failing instruction stops subsequent computation; only success continues. Do not introduce recovery or catch semantics absent from the language.

Reading a local or saving a temporary must not secretly deep-copy a value. Transform the copy loops and retain/release already present in completed HIR. Implementation-level storage and Cerune ownership operations are distinct. Array assignment checks the target and each index in order before evaluating the RHS; prepare the new element before releasing the old one.

For-loop continue targets the update block; while-loop continue targets the condition; break targets the appropriate exit. Preserve HIR's explicit cleanup ordering, including return. Do not move a short-circuit RHS or argument copies before its branch.

The current validator checks references, types, terminators, definite initialization before reads, language-level argument types/ownership, and failure origins. Initialization intersects all reachable predecessors to a fixed point. Array stores also require prior checks for every index-path prefix on every incoming path, without intervening reassignment to the root or indices. Tests fix the concrete lowering order that checks indices before evaluating the RHS.

**Path validation of heap aliases, partial initialization, and retain/release lifetimes remains unimplemented.** Validation alone does not establish ownership safety or execution equivalence with HIR. The MIR interpreter compares existing routes and detects live owned storage after successful execution. Static lifetime validation remains future work, distinct from validation for loading external Images.

The implementation and observation data are organized as follows.

| Implementation | Contents |
| --- | --- |
| [mir/mod.rs](../../src/mir/mod.rs) | Reuses HIR types/function IDs; typed locals/temporaries, atomic operations, blocks and terminators |
| [mir/lower.rs](../../src/mir/lower.rs) | Translates all completed HIR execution operations; explicitly rejects unexpanded ArrayCopy/Let/Conditional |
| [mir/validate.rs](../../src/mir/validate.rs) | Structure, types, initialization, preceding index checks, and call ownership categories |
| [mir/text.rs](../../src/mir/text.rs) | Deterministic v0.1 observation text with budgets, types, locals, operations, edges, and origins |

`BlockId` / `LocalId` / `InstructionId` are function-local IDs. Instruction IDs include terminators and are separate from HIR NodeIds. Origins distinguish original `Source` operations, `Derived` helper edges with their source, and `Synthetic` function entry/end and explicit main calls. Failing execution operations retain a single primary origin. Unreachable statements/helper blocks remain present: there is no implicit dead-code elimination.

Declaration metadata such as field defaults, constant declarations, and generic expansion origins stays in HIR. MIR holds expanded execution operations; NodeIds and HIR retain access to their declaration context. The Rust API returns an independent snapshot whose mutation does not affect the input HIR.

## Semantic preservation and resources

For the same source and explicit budgets, preserve more than final values:

- Normal completion, return values, and output bytes, including Japanese, NUL, CR/LF, without Unicode normalization.
- Language failure codes, original NodeId/SourceId/Span, and prior output.
- Evaluation order, short-circuiting, independent copies, and reading/releasing only initialized elements.
- Separate logical array/string budgets, including old/new values coexisting and when budgets are returned.
- Distinctions among success, language stops, internal inconsistencies, unsupported features, tool failures, and test timeouts.

An unused copy may still exceed its allocation budget. Dead-code elimination that removes it changes the program's failure semantics. Treat allocation, checks, cleanup, and output as effects; equal final values alone do not justify moving or deleting them. Floating-point evaluation order, NaN, and negative zero also matter.

Logical budgets are separate from physical exhaustion. Different OS allocators need not fail at identical points. Inject fixed allocation failures and verify each stage's appropriate reason, origin, and prior output. Divergence is not success; future proofs need an execution relation covering divergence or an explicitly bounded fuel claim.

## Origins and observation

HIR NodeId continues to identify the originating statement/expression. MIR block/instruction IDs and LIR instruction IDs use separate namespaces within snapshots, without promising stable numbering across source edits or passes.

Each transformation records input-to-output correspondence, including one-to-many, many-to-one, and removal. Distinguish compiler-generated helpers from lost origins. A future pass may combine operations only if it can still diagnose the original operation that actually fails; arbitrarily selecting one origin from a set is insufficient.

| Observation | Information retained |
| --- | --- |
| HIR→MIR | Structured control becoming blocks/edges, evaluation-order temporaries, original checks/ownership operations |
| MIR→LIR | Typed operations becoming target/ABI/stack decisions |
| Each pass | Input/output snapshots, pass name/version/order/explicit options, element correspondence |
| LIR→artifacts | Existing ASM annotations/object symbols, including expansions still performed by the emitter |

Observations are detached, read-only information. They do not expose mutable compiler state or runtime heaps. Version new public formats. Include source text/paths only on explicit request, and exclude timestamps, random IDs, and environment secrets.

## CLI, bundles, and Lean

`emit-mir <file> [-o <output.txt>]` is implemented. `run-mir` independently executes MIR. `emit-hir` / `emit-lir` and pass selection remain **unimplemented candidates**. Observation text is versioned as `Cerune MIR v0.1`; a dedicated extension, loading format, and distribution snapshot remain undecided. Preserve `emit-ir` compatibility; an interpreter does not automatically create another distribution route or build format.

The [#60 observation bundle](observation-bundle.en.md) saves sources, HIR, MIR, and annotated ASM from one frontend invocation. Its manifest indexes target, heap budgets, passes (currently empty), and artifacts; it is not another semantic IR. Objects and other backends remain future extensions.

The [#81](https://github.com/Hokutaka/Cerune/issues/81) show/emit/build/run/via/release responsibilities are distinct from representation stages. Release is not an optimization alias. Select passes and their order explicitly, and record external-tool optimization separately from Cerune passes.

[Lean verification](lean-verification.en.md) can first address unoptimized HIR→MIR correspondence, then individual passes. Matching interpreter runs is testing, not formal proof. The actual u8 functions increment, choose, and advance have HIR→MIR correspondence/property proofs using independent typed MIR locals, instructions, and CFGs. These check if/short-circuiting, while/for and mutable locals, innermost-loop break/continue behavior, initialization/updates, and overflow origins. The loop fixture also proves completion within separate explicit HIR/MIR bounds, not general loop termination, heap, the general lowering algorithm, or the whole compiler.

## Baseline example and implementation units

Start with [control_flow.ceru](../../examples/ir_stages/README.en.md) to inspect short-circuit and loop edges. [lowering_order.ceru](../../examples/dynamic_arrays/lowering_order.ceru) combines short-circuiting, for-loop continue/break, and dynamic-array argument copies.

```sh
cargo run --quiet -- run examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- run-vm examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-ir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48 -o target/lowering_order.mir.txt
cargo run --quiet -- emit-bytecode examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-asm examples/dynamic_arrays/lowering_order.ceru --target x86_64-unknown-linux-gnu --annotate-origins --array-heap-limit 48
```

Output is four lines: `begin`, `2`, `40`, `[10, 20, 30]`. At i=0 the call short-circuits; i=1 continues through update; only i=2 copies the argument and calls positive; i=3 exits before indexing. The 48-byte budget covers the original 24 bytes plus the 24-byte argument copy. At 47 bytes, `values` at the call fails with `allocation-limit-exceeded` and only `begin` is printed.

The MIR control flow is related to source constructs in the sketch below. **This is explanatory structure, not a generated MIR dump.** The sketch omits ownership-helper calls; adding MIR does not implicitly inline them. Actual conditions, copies, and cleanup need instructions/blocks within their respective functions.

```text
condition → continue check → break check → short-circuit LHS
                 │              │            ├ true → add → update → condition
                 └→ update      └→ exit      └ false → argument copy → call → result branch
condition false → exit
call result: true → add / false → update
```

Implement and compare in these units, aiming for current feature parity rather than a permanently restricted language subset:

| Order | Work | Acceptance |
| --- | --- | --- |
| 1 (implemented) | MIR types/blocks/instructions/validator, HIR→MIR, `emit-mir` | Validate every example's lowering/determinism, order, short-circuit/loop edges, origins, and rejection of malformed MIR. Execution comparisons follow in the next stage; static heap lifetime guarantees remain future work. |
| 2 (implemented) | [Independent MIR interpreter](mir-executor.en.md), `run-mir` | Executes neither HIR nor VM internally; compares current features, failures, origins, and lifetimes. Dynamic arrays are compared across nine routes on Windows/Linux |
| 3 (implemented) | [Native lowering from MIR](native-mir.en.md) | Match existing ASM/COFF/ELF known outputs, failures, origins, and ABI behavior on Windows/Linux |
| 4 (partially implemented) | Observation bundles, Lean correspondence, other backend migration as needed | sources/HIR/MIR/ASM bundle implemented; u8 straight-line/if/short-circuit/while/for/break/continue/nesting/update-origin proofs and completion within explicit bounds also added; general loops/heap follow. Extend artifacts and proof coverage separately |
| 5 (partly implemented) | [SSA construction](mir-ssa.en.md) and individual optimization passes | Scalar SSA with residual slots, structural validation, observation text, and MIR→SSA construction and direct execution implemented. Public CLI/bundle integration is implemented; SSA→MIR/Native is also implemented; finite-fixture Lean correspondence is also implemented; the [first constant-folding pass](ssa-constant-folding.en.md) is designed but implementation is deferred |

The first MIR PR should not combine SSA, optimization, every backend migration, and Image loading. MIR execution covers types, functions, modules, generics, arrays, strings, ownership, and runtime diagnostics. Native lowering from MIR is now implemented. The initial observation bundle is also implemented. Lean HIR→MIR verification now covers u8 straight-line, branch, short-circuit, and while fixtures with completion within explicit bounds; branches, break/continue, and nesting are also checked. For initialization, body completion/continue passing through updates, break skipping updates, and update-overflow origins are now checked. The [SSA design](mir-ssa.en.md) now uses this unoptimized MIR baseline. The dedicated SSA representation, structural validator, and deterministic observation text are implemented. Automatic construction preserving original MIR is implemented too. Direct SSA execution is also implemented, comparing output, failure, and ownership semantics with baseline routes using runnable examples, failure cases, and heap budgets. The public CLI and bundle now support explicit `--ssa` selection. `emit-asm --ssa` / `emit-obj --ssa` reconstruct MIR with explicit copies for Native generation; bundles keep baseline and SSA-derived ASM. Finite-fixture SSA Lean correspondence is also implemented. Retain the [constant-folding design](ssa-constant-folding.en.md) and baseline examples, deferring implementation. Prioritize language features, I/O, and completed artifacts following the [roadmap](language-roadmap.en.md).
