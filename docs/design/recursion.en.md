# Recursion and invocation storage

[日本語](recursion.ja.md)

Status: WAT invocation storage and explicit depth-limit APIs/diagnostics for HIR, MIR, SSA, and VM are implemented. CLI configuration, common limits in generated routes, and recursive source programs remain pending.

## Findings and responsibilities

Use existing Cerune IR functions, calls, and returns. No recursion-specific IR or separate semantics is introduced.

| Route / stage | Current state | Prerequisite for enabling recursion |
| --- | --- | --- |
| Frontend | Rejects direct and indirect recursion | Lift the restriction after execution routes are ready; keep recursive value-type validation separate |
| Generic expansion | Rejects re-entry into the same template | Reuse an existing specialization for identical concrete arguments; retain bounds on expansion that keeps creating new types |
| HIR / VM | Independent values per call; explicit depth-limit APIs implemented; execution uses Rust recursion | CLI configuration; verify structures and ranges that stop before physical stack exhaustion |
| MIR / SSA | Independent frames; explicit depth-limit APIs implemented; reject re-entry into the same function | Validate resources and prepare all routes before allowing re-entry |
| C / LLVM / QBE / Native | Locals use the execution environment's stack | Retain limits, call stacks, and origins; compare aggregates and ownership across routes |
| WAT | This change moves aggregates and temporaries to private per-invocation memory | Add common limits and call-stack diagnostics |

## Agreed stopping condition

Bound the number of simultaneously active function calls. Evaluate arguments from left to right, then check immediately before entering the callee. On exceeding the bound, do not execute the callee body; stop with the caller NodeId/Span and call stack.

| Category | Meaning |
| --- | --- |
| Common depth limit | A semantic stopping condition for the same source, settings, and input; observation does not change it |
| Physical stack / memory exhaustion | An execution-environment resource failure, not a depth-limit failure |
| Observation | Read call stacks and source locations without an API to mutate active frames |

Choose defaults and supported ranges after measuring all routes. Count each actual invocation of a source-declared function, including standard modules, generic specializations, and explicit main, as one level. Exclude shared-IR/backend-generated helpers and the implicit top-level entry. Backend implementation details must not change semantic depth. No implicit tail-call optimization is introduced.

## Implemented: explicit limits in four executors

HIR, MIR, SSA, and VM expose the Rust API `run_with_call_limit(program, NonZeroU32)`. Allow calls exactly at the limit; reject the next before executing its body. Return restores the depth. Bytecode retains generated-helper identity rather than guessing from function names.

| Item | Current contract |
| --- | --- |
| Configuration | Caller supplies a positive limit; ordinary run, CLI, and I/O connection APIs retain their behavior |
| Access | Error call_failure() exposes the limit, rejected call, and active source-function frames ordered outermost first |
| Origins | Each call retains FunctionId and caller NodeId/Span; explicit main starts with origin=host |
| Record | record() starts with call-v1 reason=depth-limit limit=N function=F; subsequent lines use call-v1 frame=I function=F, each followed by origin fields |
| Content protection | Origins use node=N [file=S] bytes=A..B; omit file for anonymous SourceId. No paths, source text, arguments, or runtime values |
| Observation/control | Configure limits before execution. Failure records are read-only snapshots; reading/rendering them does not resume execution |

This increment does not bound physical stack or memory use. Generated helpers still consume resources. Existing heap allocation failures remain separate runtime diagnostics, but capturing Rust executor OS stack exhaustion or general memory exhaustion as common diagnostics is not implemented. Large limits are not guaranteed safe. Measure supported ranges and address resource exhaustion before enabling recursion.

Attaching stacks to every other failure, matching stopping conditions in generated routes, CLI configuration, and MIR/SSA re-entry remain pending. Frontend recursion rejection stays in place.

`cargo test --test ir_executor call_depth` compares limit boundaries, left-to-right arguments, short circuiting, main, generics, standard modules, helper exclusion, normal/early returns, origins, prior output, and stack order across four executors. It also checks that heap-limit failures remain distinct from depth failures, and includes the [depth example](../../examples/call_frames/depth.ceru).

## Physical stack validation and next steps

The [call-depth experiment](../../experiments/call_depth/README.en.md) compares Windows/Linux debug and release builds. With a requested 2 MiB at depth 32, aggregate calls exhaust the stack in all four debug executors before producing depth diagnostics. These finite measurements do not prove a safe configuration range.

Before extending limits through the CLI, start with the standard HIR executor and replace host recursion with explicit frames and continuation positions. Include generated helpers in physical frame management and distinguish allocation failure from depth overflow. Preserve evaluation order, ownership, and origins, extend the approach to the other executors, then align generated routes, supported ranges, and CLI configuration. This execution-structure change is not implemented yet.

## Implemented: WAT invocation storage

| Item | Mechanism |
| --- | --- |
| Layout | Keep string literals and output formatting storage static; represent aggregate parameters, locals, and temporaries as offsets within a frame |
| Calls | Allocate callee storage after evaluating arguments and pass its base as a private final parameter; public library parameter types remain unchanged |
| Returns | Copy aggregate results into the caller destination and complete ownership operations before releasing the frame, including early returns |
| Reuse | A 16-byte header and 8-byte-aligned payload; reuse a sufficiently large block from a dedicated free list, otherwise grow memory |
| Live bytes | Share the allocation cursor with string/array heaps, but keep free lists and lifetimes separate; frames do not count toward language string/array live-byte limits |
| Allocation failure | Ordinary calls stop with runtime-v1 allocation-failed and caller NodeId/Span; direct host entry uses origin=host |
| Observation | Emitted WAT retains function IDs, frame sizes, bases, offsets, and allocation/release operations; memory remains unexported |

Allocate top-level storage before execution and associate failure with the first statement's origin. Physical memory does not shrink; blocks are neither split nor coalesced. Trap rollback/resumption is not guaranteed, and this change introduces no contract for reusing a failed execution instance.

Aggregate programs that now need allocation-failure diagnostics require the existing import cerune.write_error_byte. Hosts without it fail instantiation. Existing versioned output/flush contracts and public function types are retained.

## Validation and implementation order

cargo test --test wat_output introduces recursion only in a test IR fixture and executes it to check parent arrays and return destinations. Tests also cover 10,000 calls with memory capped at one page, reuse after normal/early returns, allocation-failure origins/prior output, and owned values under a 22-byte string live-byte limit. Recursive public source remains rejected.

The [call_frames example](../../examples/call_frames/README.en.md) uses current syntax for nested calls, value updates, and early returns. Its output is compared across HIR, MIR, SSA, VM, and WAT.

1. Establish independent WAT invocation storage (implemented).
2. Align common limits across routes. Counting and explicit APIs in four executors are implemented; measurements make explicit frame management, starting with HIR, the next prerequisite before configuration ranges, CLI settings, and generated-route diagnostics.
3. Reuse generic specializations and lift frontend restrictions.
4. Compare direct/mutual recursion, ownership/cleanup, argument order, limit boundaries, and failure origins across routes; add recursive examples.

Complete steps 2–4 before enabling recursion. File I/O and subprocesses follow as later language features.
