# Recursion and invocation storage

[日本語](recursion.ja.md)

Status: WAT invocation storage, explicit depth-limit APIs/diagnostics for HIR, MIR, SSA, and VM, explicit frames in all four executors, and explicit C/LLVM/QBE/WAT/Native limits/diagnostics are implemented. Common CLI configuration (default 128, range 1–128) and Native build/release records are also implemented. Direct/mutual source recursion and generic reuse are also implemented.

## Implemented: source recursion and generic reuse

Direct, mutual, and generic recursion use existing functions, calls, and returns. No recursion-specific IR, implicit inlining, or tail-call optimization is introduced. Recursive value types and cyclic imports remain separately prohibited.

| Processing | Contract |
| --- | --- |
| Names and types | Register all function declarations before checking bodies; recursive calls retain ordinary parameter/result checks |
| Generic registration | Register resolved type/length combinations before processing bodies; reuse the same function even while its body is pending |
| Generic bodies | Process the pending queue in registration order, without adding compiler-recursive expansion frames |
| Expansion limits | 256 instances per compilation and depth 64 along a path of new instances. Ever-changing types stop with a diagnostic. These limits are separate from runtime call depth |
| Origins | Retain resolved arguments, definition spans, and all call spans including recursive sites. Expansion errors include their instantiation path. Body NodeIds are allocated per instance |
| Execution | Evaluate arguments left to right, then check source-function depth. Preserve caller intermediates, owned values, and return destinations; restore depth on return |

Calls through used defaults, array operations, conversions, and product updates follow the same rules. Registration order is observable; identifiers describe one compilation and are not promised to retain their numbers across compiler versions.

The [examples](../../examples/recursion/README.en.md) cover factorial, mutual recursion, and generic recursion with owned values. Tests compare four executors with C, LLVM, QBE, WAT, ASM, and objects: limits, prior output, full call-v1 stacks/origins, depth 128, and ownership cleanup across repeated execution on Linux/Windows.

Unbounded Rust APIs still select no limit. Use explicit-limit APIs when executing infinite recursion. CLI defaults to 128 with public range 1–128; this is distinct from physical stack/memory safety.

## Findings and responsibilities

Use existing Cerune IR functions, calls, and returns. No recursion-specific IR or separate semantics is introduced.

| Route / stage | Current state | Validation focus |
| --- | --- | --- |
| Frontend | Accepts direct and indirect recursion | Recursive value-type validation remains separate |
| Generic expansion | Reuse registered specializations; process queued bodies | Retain 256 instances / expansion depth 64 |
| HIR | Execute references into original HIR with explicit frames; source and generated calls do not add host-recursive frames | Cross-route comparisons of recursive source |
| VM | Explicit frames retain original bytecode PCs, operand stacks, and slots | Additional resource validation and recursive-source comparisons |
| MIR / SSA | Explicit frames retain original CFG blocks/instruction positions and locals/SSA values/slots | Retain common settings for recursive source |
| C | Rust API emits explicit limits and call-v1; locals use the execution environment's stack | Additional physical resource and recursive-source validation |
| LLVM / QBE / Native | Explicit limits and call-v1; locals use the execution environment's stack | Additional physical resource and recursive-source validation |
| WAT | Private invocation storage and call history; explicit limits and call-v1 | Additional physical resource and recursive-source validation |

## Agreed stopping condition

Bound the number of simultaneously active function calls. Evaluate arguments from left to right, then check immediately before entering the callee. On exceeding the bound, do not execute the callee body; stop with the caller NodeId/Span and call stack.

| Category | Meaning |
| --- | --- |
| Common depth limit | A semantic stopping condition for the same source, settings, and input; observation does not change it |
| Physical stack / memory exhaustion | An execution-environment resource failure, not a depth-limit failure |
| Observation | Read call stacks and source locations without an API to mutate active frames |

The CLI uses default 128 and public range 1–128, distinct from physical safety. Count each actual invocation of a source-declared function, including standard modules, generic specializations, and explicit main, as one level. Exclude shared-IR/backend-generated helpers and the implicit top-level entry. Backend implementation details must not change semantic depth. No implicit tail-call optimization is introduced.

## Implemented: common CLI configuration

Normal execution, generated routes, and Native build accept `--call-depth-limit N`: default 128, public range 1–128. A call at the limit is allowed; the next source-function entry stops after left-to-right argument evaluation.

| Operation | Configuration |
| --- | --- |
| run / run-ir / run-mir (including SSA) / run-vm | Pass the same limit alongside I/O and program arguments. runtime-v1 diagnostic mode emits complete call-v1 records for depth failures |
| emit-c / emit-llvm / emit-qbe / emit-wat | Retain the limit and checks in generated artifacts. LLVM/QBE require an explicit target even for scalar-only programs |
| emit-lir / emit-asm / emit-obj / build native | Generate from the same limited LIR. LIR records the value; ASM/Object retain checks and the diagnostic limit. Build manifests record call_depth_limit |
| observe | Save baseline/SSA LIR and ASM using the same setting and record it in the manifest, without execution or changing settings through observation |
| release native | Preserve the build setting unchanged. Older builds without a recorded limit remain unchanged; the release field is null (unknown) |
| check / fmt / emit-sources / emit-ir / emit-mir / emit-bytecode | Reject this execution setting; source, common IR, and bytecode representations do not embed it |

**1–128 is the public semantic configuration range, not a physical stack/memory safety guarantee.** Large locals, aggregates, and generated helpers consume physical resources. Detected allocation failures remain allocation-failed; OS/engine stack exhaustion remains a separate failure and is never relabeled depth-limit. General physical exhaustion is not guaranteed to produce a common diagnostic. Range extensions require a separate change based on validation across routes.

Existing Rust APIs retain their defaults. `run_with_io_arguments_and_call_limit` combines execution settings; `emit_*_with_runtime_limits` / `lower_mir_with_runtime_limits` combine argument limits and call depth. Rust NonZeroU32 APIs are distinct from the public CLI range and do not guarantee large values are safe.

Compatibility changes are default CLI depth checks, mandatory LLVM/QBE targets, and the existing diagnostic import `cerune.write_error_byte` now required by generated WAT. Hosts lacking it fail at instantiation. Private memory and versioned I/O contracts remain unchanged. Source recursion uses the same depth checks.

## Implemented: explicit limits in four executors

HIR, MIR, SSA, and VM expose the Rust API `run_with_call_limit(program, NonZeroU32)`. Allow calls exactly at the limit; reject the next before executing its body. Return restores the depth. Bytecode retains generated-helper identity rather than guessing from function names.

| Item | Current contract |
| --- | --- |
| Configuration | Caller supplies a positive limit; existing Rust APIs retain their defaults; CLI uses the common setting above |
| Access | Error call_failure() exposes the limit, rejected call, and active source-function frames ordered outermost first |
| Origins | Each call retains FunctionId and caller NodeId/Span; explicit main starts with origin=host |
| Record | record() starts with call-v1 reason=depth-limit limit=N function=F; subsequent lines use call-v1 frame=I function=F, each followed by origin fields |
| Content protection | Origins use node=N [file=S] bytes=A..B; omit file for anonymous SourceId. No paths, source text, arguments, or runtime values |
| Observation/control | Configure limits before execution. Failure records are read-only snapshots; reading/rendering them does not resume execution |

This increment does not bound physical stack or memory use. Generated helpers still consume resources. Existing heap allocation failures remain separate runtime diagnostics, but capturing Rust executor OS stack exhaustion or general memory exhaustion as common diagnostics is not implemented. Large limits are not guaranteed safe. Keep the public range distinct from physical resource conditions and continue validating finite recursive programs.

Attaching stacks to every other failure remains pending. Internal MIR/SSA re-entry is supported with explicit frames. Source recursion uses the same depth checks.

`cargo test --test ir_executor call_depth` compares limit boundaries, left-to-right arguments, short circuiting, main, generics, standard modules, helper exclusion, normal/early returns, origins, prior output, and stack order across four executors. It also checks that heap-limit failures remain distinct from depth failures, and includes the [depth example](../../examples/call_frames/depth.ceru).

## Physical stack validation and next steps

The [call-depth experiment](../../experiments/call_depth/README.en.md) compares Windows/Linux debug and release builds. Before this change, commit 143aae0 exhausted the stack in all four debug executors for aggregate calls at depth 32 with a requested 2 MiB, before producing depth diagnostics. These finite measurements do not prove a safe configuration range.

Explicit frames are implemented in HIR, MIR, SSA, and VM. Explicit C limits and finite resource fixtures are now verified first. Generated routes now share the stopping condition. Common CLI configuration is implemented; source recursion is also implemented.

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

The [pending-values example](../../examples/call_frames/pending_values.ceru), four-route comparisons of order/loops/index checks, and injected frame-allocation failures for ordinary/generated functions are included. A test-only IR cycle also checks owned parent values and cleanup across 100 runs. The same validation now covers MIR, SSA, and VM. Recursive source programs are supported. Common CLI configuration is described above. Generated-route limits are described below.

## Implemented: explicit MIR, SSA, and VM frames

MIR, SSA, and VM calls now push an explicit callee frame and resume the saved position on return, without host-recursive dispatch. Each executor directly uses its original representation; it does not delegate execution or construct another intermediate representation.

| Route | State retained per invocation | Resume |
| --- | --- | --- |
| MIR | Function, block, next instruction, locals, return destination | Store the result in the caller local and advance |
| SSA | Function, block, next instruction, SSA values, retained slots, return destination | Preserve caller values/slots; edge transfer reads every input before assigning block arguments |
| VM | Function, PC, operand stack, slots, initializer positions | Push the result onto the caller stack and continue after call |

Generated helpers have physical frames but do not consume logical depth. Frame/local/argument/SSA-edge containers and VM stack growth use try_reserve and report returned allocation failures as allocation-failed. Ordinary callee failures retain the failing instruction location; callee frame-allocation failures retain the caller's call location. Failure stops subsequent execution and unwinds remaining internal depth counters.

Tests compare all four executors at depth 128 on a requested 2 MiB thread, with limits 8/127/128, suspended values, loops/early returns, test-only re-entry with owned values, and injected frame-allocation failures for source/generated functions. The internal MIR/SSA re-entry rejection is removed; recursive source programs are supported. Unbounded APIs are not a sandbox against infinite recursion or loops. General allocation recovery and safe cross-route ranges are not guaranteed.

## Implemented: explicit limits in generated C

The Rust API `codegen::c::emit_c_with_call_limit(&program, NonZeroU32)` emits C with the same counting and call-v1 records as the four executors. Ordinary Rust emit_c retains its default behavior. CLI uses the common limit above.

| Item | Behavior |
| --- | --- |
| Arguments/calls | Evaluate all source-function arguments into temporaries from left to right, then check enter in the caller before calling the C function |
| Results | Save the result in a caller temporary before leave. Void calls, early returns, and aggregate returns decrement depth exactly once after the callee returns |
| Classification | Preserve source/generated classification in C IR. Exclude generated helpers; count explicit main with host origin |
| History | Retain only FunctionId and immutable origin strings. Grow on demand rather than allocating the full limit up front; release on successful completion |
| History allocation failure | Stop with runtime-v1 allocation-failed at the call site. History is runtime storage separate from language string/array live-size limits |
| Depth overflow | Write the rejected call and outermost-first active calls to stderr, then exit with code 1. Preserve prior output and skip the callee body and subsequent operations |
| Observation | Emitted C retains the limit, temporaries, enter/leave, history allocation and call-v1 rendering. No external history/control API is added |

History construction and execution control are internal. Records contain no paths, source text, arguments, or values. C stderr follows the target C runtime: comparisons normalize CRLF/LF and experiments retain raw stderr. String contents and stdout bytes are not normalized.

The [generated-C experiment](../../experiments/call_depth/README.en.md) explicitly selects target, compiler, and stack size. It checks scalar and owned-string/array-product chains at depths 32/128, limits N−1/N, and O0/O2, never accepting abnormal process termination as successful depth limiting. Tests also compare every diagnostic line with four executors, short circuiting, argument-failure precedence, generics, standard modules, helper exclusion, 100 owned-value runs, and test-only direct/mutual IR re-entry.

This API does not bound physical stack use. Argument copies, caller temporaries, compiler layout/optimization, and generated helpers consume resources. OS stack exhaustion is not converted into call-v1, and no common physical safety range is guaranteed; semantic CLI defaults are defined above. LLVM, QBE, WAT, and Native also validate logical depth and physical resource failures separately.

## Implemented: shared stopping conditions in generated routes

The following Rust APIs explicitly select a depth limit. Ordinary Rust emit APIs retain their defaults; CLI and build/release use the common setting above. Native uses the same limited LIR to produce assembly and self-encoded objects.

| Route | API | History storage |
| --- | --- | --- |
| LLVM | `emit_llvm_with_call_limit(program, target, limit)` | Grow an array of immutable call-site pointers on demand |
| QBE | `emit_qbe_with_call_limit(program, target, limit)` | Same; explicitly select amd64_sysv or amd64_win |
| WAT | `emit_wat_with_call_limit(program, limit)` | Linked records allocated through existing private frames; release and reuse on return |
| Native | `lower_mir_with_call_limit(program, target, limit)` → `emit_asm_from_lir` / `emit_object_from_lir` | Array of immutable site pointers; checks and history use typed machine instructions |

All routes check after argument evaluation and before entering the callee, then restore depth on return. Generated helpers are excluded. LLVM/QBE retain results in existing temporaries, WAT on its operand stack, and Native in return registers. Comparisons require every call-v1 line and prior output to match. LLVM/QBE/Native terminate with exit code 1; WAT emits diagnostics and traps with unreachable.

History is runtime storage separate from language string/array live-byte budgets. Allocation failure stops with an origin-bearing runtime-v1 allocation-failed record, never a call-v1 overflow. WAT keeps memory private; limited artifacts require the existing write_error_byte import when needed. Resuming a failed instance is not supported. This change does not translate general OS stack exhaustion or a Wasm engine's physical call-stack exhaustion into common diagnostics.

`cargo test --test llvm_output --test qbe_output --test wat_output --test native_output call_depth` compares four executors with finite depth-32 chains, depth-128 internal-IR re-entry using few functions, ownership, floating-point and stack arguments, main/generics/standard modules, short circuits, argument failures, and allocation failures. Native executes both assembly and self-encoded objects; LIR observations retain limits and function classification. Source-recursion cases also compare the same checks after frontend compilation.

The [experiment emitter](../../experiments/call_depth/README.en.md) saves each representation. Common CLI configuration is implemented. Source recursion uses the same checks; additional physical resource validation remains ongoing.


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

cargo test --test wat_output introduces recursion only in a test IR fixture and executes it to check parent arrays and return destinations. Tests also cover 10,000 calls with memory capped at one page, reuse after normal/early returns, allocation-failure origins/prior output, and owned values under a 22-byte string live-byte limit. Public recursive source is also covered by the common depth tests.

The [call_frames example](../../examples/call_frames/README.en.md) uses current syntax for nested calls, value updates, and early returns. Its output is compared across HIR, MIR, SSA, VM, and WAT.

1. Establish independent WAT invocation storage (implemented).
2. Align common limits across routes. Counting and explicit APIs in four executors are implemented; all four executors now use explicit frames; generated-route limits and diagnostics are also implemented. Common CLI configuration is also implemented. Source recursion and generic reuse are also implemented.
3. Reuse generic specializations and lift frontend restrictions (implemented).
4. Compare direct/mutual recursion, ownership/cleanup, argument order, limits, and origins across routes; add recursive examples (implemented).

Steps 1–4 are implemented. Limited file I/O design is next, followed by subprocesses.
