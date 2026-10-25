# Recursion and invocation storage

[日本語](recursion.ja.md)

Status: WAT invocation storage, explicit depth-limit APIs/diagnostics for HIR, MIR, SSA, and VM, and explicit frames in all four executors are implemented. CLI configuration, common limits in generated routes, and recursive source programs remain pending.

## Findings and responsibilities

Use existing Cerune IR functions, calls, and returns. No recursion-specific IR or separate semantics is introduced.

| Route / stage | Current state | Prerequisite for enabling recursion |
| --- | --- | --- |
| Frontend | Rejects direct and indirect recursion | Lift the restriction after execution routes are ready; keep recursive value-type validation separate |
| Generic expansion | Rejects re-entry into the same template | Reuse an existing specialization for identical concrete arguments; retain bounds on expansion that keeps creating new types |
| HIR | Execute references into original HIR with explicit frames; source and generated calls do not add host-recursive frames | Cross-route agreement, CLI configuration, and common supported ranges |
| VM | Explicit frames retain original bytecode PCs, operand stacks, and slots | Generated-route agreement, resource validation, and CLI configuration |
| MIR / SSA | Explicit frames retain original CFG blocks/instruction positions and locals/SSA values/slots | Align generated routes, supported ranges, and CLI before enabling recursive source |
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

The [call-depth experiment](../../experiments/call_depth/README.en.md) compares Windows/Linux debug and release builds. Before this change, commit 143aae0 exhausted the stack in all four debug executors for aggregate calls at depth 32 with a requested 2 MiB, before producing depth diagnostics. These finite measurements do not prove a safe configuration range.

Explicit frames are implemented in HIR, MIR, SSA, and VM. Next validate generated-route resources and depth diagnostics, then supported ranges and CLI configuration.

## Implemented: explicit HIR frames

HIR function calls and statement/expression progression now use a loop instead of host-recursive dispatch. Each frame retains references into the original HIR, continuation positions, BindingId bindings, intermediate values, and the return destination. No execution IR or bytecode is constructed; source IR and origins remain unchanged.

| Item | Behavior |
| --- | --- |
| Calls | Evaluate arguments left to right, check logical depth, then add the callee frame |
| Generated helpers | Use the same physical frame management without consuming source-function depth |
| Return | Check the result, transfer it to caller intermediate values, and discard callee continuations and bindings; early returns follow the same rule |
| Control | Advance only the selected branch of if/short circuiting; for-continue runs the update; break/return discard irrelevant continuations |
| Indexed assignment | Evaluate and bounds-check each index before advancing to later indices or the right-hand side |
| Allocation failure | Check frame, continuation, intermediate-value, and binding container growth with try_reserve. Report returned failures as runtime-v1 allocation-failed, separate from call-v1 depth overflow |
| Failure | Stop subsequent source operations and retain origins/prior output. Unwinding internal depth counters is not source rollback or re-execution |

Shared call-history and depth-failure snapshot vectors now also return allocation failures, mapped to allocation-failed by all four executors. This does not make every Rust allocation recoverable: ordinary value copies, diagnostic Box/String allocations, nested-value processing, OS termination, and general memory exhaustion are not universally covered.

On Windows/Linux debug builds, tests execute depth-128 scalar and owned-string/array-product chains on a thread requesting 2 MiB. Limits 8 and 127 stop with the original call origin; limit 128 and execution without a limit complete. These finite fixtures verify the call structure improvement, not defaults or safe ranges across all routes.

The [pending-values example](../../examples/call_frames/pending_values.ceru), four-route comparisons of order/loops/index checks, and injected frame-allocation failures for ordinary/generated functions are included. A test-only IR cycle also checks owned parent values and cleanup across 100 runs. The same validation now covers MIR, SSA, and VM. Recursive source programs, CLI configuration for common limits, and generated-route limits remain unavailable.

## Implemented: explicit MIR, SSA, and VM frames

MIR, SSA, and VM calls now push an explicit callee frame and resume the saved position on return, without host-recursive dispatch. Each executor directly uses its original representation; it does not delegate execution or construct another intermediate representation.

| Route | State retained per invocation | Resume |
| --- | --- | --- |
| MIR | Function, block, next instruction, locals, return destination | Store the result in the caller local and advance |
| SSA | Function, block, next instruction, SSA values, retained slots, return destination | Preserve caller values/slots; edge transfer reads every input before assigning block arguments |
| VM | Function, PC, operand stack, slots, initializer positions | Push the result onto the caller stack and continue after call |

Generated helpers have physical frames but do not consume logical depth. Frame/local/argument/SSA-edge containers and VM stack growth use try_reserve and report returned allocation failures as allocation-failed. Ordinary callee failures retain the failing instruction location; callee frame-allocation failures retain the caller's call location. Failure stops subsequent execution and unwinds remaining internal depth counters.

Tests compare all four executors at depth 128 on a requested 2 MiB thread, with limits 8/127/128, suspended values, loops/early returns, test-only re-entry with owned values, and injected frame-allocation failures for source/generated functions. The internal MIR/SSA re-entry rejection is removed; recursive source programs remain prohibited. Unbounded APIs are not a sandbox against infinite execution in invalid IR. General allocation recovery and safe cross-route ranges are not guaranteed.

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
2. Align common limits across routes. Counting and explicit APIs in four executors are implemented; all four executors now use explicit frames; next align configuration ranges, CLI settings, and generated-route diagnostics.
3. Reuse generic specializations and lift frontend restrictions.
4. Compare direct/mutual recursion, ownership/cleanup, argument order, limit boundaries, and failure origins across routes; add recursive examples.

Complete steps 2–4 before enabling recursion. File I/O and subprocesses follow as later language features.
