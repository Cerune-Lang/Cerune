# Recursion and invocation storage

[日本語](recursion.ja.md)

Status: WAT invocation storage is separated. Recursive source programs, the common depth limit, and call-stack diagnostics remain unimplemented.

## Findings and responsibilities

Use existing Cerune IR functions, calls, and returns. No recursion-specific IR or separate semantics is introduced.

| Route / stage | Current state | Prerequisite for enabling recursion |
| --- | --- | --- |
| Frontend | Rejects direct and indirect recursion | Lift the restriction after execution routes are ready; keep recursive value-type validation separate |
| Generic expansion | Rejects re-entry into the same template | Reuse an existing specialization for identical concrete arguments; retain bounds on expansion that keeps creating new types |
| HIR / VM | Independent values per call; execution uses Rust recursion | Common limits/diagnostics; verify structures and ranges that stop before physical stack exhaustion |
| MIR / SSA | Independent frames; reject re-entry into the same function | Implement common limits/diagnostics before allowing re-entry |
| C / LLVM / QBE / Native | Locals use the execution environment's stack | Retain limits, call stacks, and origins; compare aggregates and ownership across routes |
| WAT | This change moves aggregates and temporaries to private per-invocation memory | Add common limits and call-stack diagnostics |

## Agreed stopping condition (not implemented)

Bound the number of simultaneously active function calls. Evaluate arguments from left to right, then check immediately before entering the callee. On exceeding the bound, do not execute the callee body; stop with the caller NodeId/Span and call stack.

| Category | Meaning |
| --- | --- |
| Common depth limit | A semantic stopping condition for the same source, settings, and input; observation does not change it |
| Physical stack / memory exhaustion | An execution-environment resource failure, not a depth-limit failure |
| Observation | Read call stacks and source locations without an API to mutate active frames |

Choose defaults and supported ranges after measuring all routes. The next common contract must also define counting for entry points, shared-IR-generated functions, and backend helpers. Backend implementation details must not change semantic depth. No implicit tail-call optimization is introduced.

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

1. Establish independent WAT invocation storage (this change).
2. Define common counting, configuration, diagnostics, and call stacks; implement them across executors and generated routes.
3. Reuse generic specializations and lift frontend restrictions.
4. Compare direct/mutual recursion, ownership/cleanup, argument order, limit boundaries, and failure origins across routes; add recursive examples.

Complete steps 2–4 before enabling recursion. File I/O and subprocesses follow as later language features.
