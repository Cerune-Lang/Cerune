# Language capabilities and next steps

[日本語](language-roadmap.ja.md)

This roadmap covers the Cerune compiler as of 2026-11-24. See the [language reference](../reference/language.en.md) for implemented syntax and the [project inventory](ecosystem.en.md) for uses outside the compiler.

## Available in v0.2.0

| Task | Implementation and examples |
| --- | --- |
| Write programs | [Optional let and fmt](formatting.en.md), [inferred binding shorthand](infer-omission.en.md), and [recursion](../../examples/recursion/README.en.md) |
| Handle input and output | [Arguments](../../examples/arguments/README.en.md), [integer parsing](../../examples/integer_parsing/README.en.md), [line input](line-input.en.md), and [UTF-8 file reads and exclusive creation](../../examples/files/README.en.md) |
| Run a child process | [Start a registered program](../../examples/process_execution/README.en.md) and inspect its output, termination status and failures. Tests cover four executors and generated C, LLVM, QBE, WAT and Native on Windows and Linux |
| Build a native executable | [Sashimi](native-encoder.en.md) emits assembly and objects. [Native build](native-build.en.md) links with the selected Clang. [Native release](native-release.en.md) packages the files and records for distribution |

Compiler downloads cover Windows x64 and both x64 and ARM64 on Linux and macOS. The child-process runtime for linking generated programs ships for Windows x64 and Linux x64. [v0.2.0 release notes](../distribution/v0.2.0-notes.en.md) · [Distribution tests](../distribution/release-readiness.en.md)

See the [generated-route design](subprocess-generated.en.md) for subprocess tests by route and the [October 11 inventory](development-status.en.md) for the earlier baseline.

## Completion criteria for v0.3.0

The [milestone](https://github.com/Cerune-Lang/Cerune/milestone/1) tracks three issues. Each feature PR records the bilingual specification, a runnable example, and comparisons using the same inputs.

| Order | Work | Acceptance tests |
| --- | --- | --- |
| 1 | Merge checks [#262](https://github.com/Cerune-Lang/Cerune/issues/262) | Pass `ci-gate` when every CI job, including the Linux, Windows and macOS matrices, succeeds. Test failed, cancelled and skipped prerequisite jobs. OS-specific steps can be skipped within a successful job. Require the gate and Dependency Review on `master` and record the CodeQL decision. |
| 2 | Execution traces [#282](https://github.com/Cerune-Lang/Cerune/issues/282) | Compare event order, NodeId and source locations for calls, branches, returns and stops in the same example across four executors and generated C, LLVM, QBE, WAT and Native. Compare output, exit status and external operation counts with tracing on and off. Test event and byte limits and a failed trace write. |
| 3 | Binary file I/O [#243](https://github.com/Cerune-Lang/Cerune/issues/243) | Read bytes containing NUL and invalid UTF-8 under the configured root and compare the newly saved file byte for byte. Check empty input, the exact limit, oversized input, read failures and an existing destination across routes. Include a small example reading known fields from an ELF header. |

**Publication**. Extract and run archives for five compiler targets and two child-process runtime targets at the candidate commit that passed the three checks above. Record changes, upgrade instructions and supported hosts in both languages. Verify the tag workflow and the version and SHA-256 of the published files before closing the milestone. [Previous distribution tests](../distribution/release-readiness.en.md)

Tint-St tracks trace display and Symnexa tracks ELF linking. Record both repositories’ commits when testing them together. The next packaging feature is [saving and loading IR and VM images (#81)](https://github.com/Cerune-Lang/Cerune/issues/81).

## K and Cranelift experiments

Use the [Lean branch example](../../experiments/lean/branch.ceru) throughout. The experiments inform a future adoption decision. The release criteria above govern v0.3.0.

| Order | Work | Evidence for the next decision |
| --- | --- | --- |
| 1 | Record VM execution alongside HIR, MIR and SSA. | Compare output, exit status and source events for short circuits, successful returns and overflow. Save these results for the two experiments. |
| 2 | Translate the completed HIR of `choose` to terms for an independent K definition. Start with checked u8 addition, short circuits, branches and return. | Compare all 256 inputs against the Lean specification and the executors. Input 128 returns 130. Input 254 skips the right operand and stops in the addition in the else branch. Check the stop code, NodeId and source span. Record which rules had to be written and how a change to a rule is detected before expanding coverage. |
| 3 | Take completed HIR as input, as the C, LLVM, QBE and WAT emitters do. Save the generated CLIF and run that artifact with a separate Cranelift driver. | Record the target, runtime ABI and tool version. Compare outcomes for all 256 inputs with the four executors and trace branches and stops back to the source. Estimate the cost of maintaining the HIR translation and driver before deciding whether to offer an optional `emit-clif`. |

Step 1 is implemented. Reproduce the four-executor comparison, including all 256 inputs to `choose`, with `cargo test --test execution_output_cli executors_match_hir_events_output_and_stops`.

Step 2 is complete. K ran every input to `choose` and agreed with Lean and the four executors. Changing the short-circuit rule produced a difference at input 128. Next, #282 will record the branch taken and the return or stop site when generated C runs. [K experiment results](../../experiments/k/README.en.md)

Lean continues to check properties of programs and correspondence between representations. A later self-hosting test can run a compiler written in Cerune via Native and Cranelift, then compare the IR each emits for the same source and the behavior of the resulting programs.

## Properties Cerune should preserve

Cerune prioritizes explaining a computation's meaning and its transformation into executable representations. New features should preserve these properties:

- Explicit types, evaluation order, short-circuiting, and failure conditions. No value loss through implicit numeric conversions.
- Independent values after copying. Updates through `mut` must not secretly change another value.
- Immutable UTF-8 strings preserving Japanese text, NUL, CR, and LF without Unicode normalization.
- Observable correspondence between source, Cerune IR, and artifacts. Observation must not expose an interface for changing running state.
- Explicit targets, artifact-consuming tools, and supported capabilities. The host OS must not silently select language behavior.

## Current language capabilities

| Area | Implemented | Limits and boundaries | Runnable examples |
| --- | --- | --- | --- |
| Signed integers | `i8`, `i16`, `i32`, `i64`, arithmetic, comparisons, bit operations | Out-of-range arithmetic stops; small types currently also occupy 64-bit storage | [sensor_calibration](../../examples/sensor_calibration.ceru), [integer_limits](../../examples/integer_limits.ceru) |
| Unsigned integers | `u8`, `u16`, `u32`, `u64` | `u64` covers 0–18446744073709551615; no implicit signedness changes or wrapping | [u64_values](../../examples/u64_values.ceru), [packet_counter](../../examples/packet_counter.ceru) |
| Floating point | `f32`, `f64` | Arithmetic rounds at the selected precision; `convert` preserves values; float-to-integer rounding and saturation are explicit | [floating_point](../../examples/floating_point.ceru) |
| Booleans | `bool`, comparisons, `!`, short-circuiting `&&` and `||` | No implicit numeric conversion | [short_circuit](../../examples/short_circuit.ceru) |
| Strings | `string`, printing, `==`, `!=`, `byte_len`, `concat` | std:num::parse_i64 parses integers. Indexing and character counts remain open | [string_byte_length](../../examples/string_byte_length.ceru), [string_lookup](../../examples/string_lookup.ceru) |
| Fixed arrays | `[T; N]`, `array_len`, `for … in`, nesting, value passing, updates through `mut` | Length belongs to the type; indices are `i64`; use `[T]` for dynamic lengths; no borrowed slices | [array_iteration](../../examples/array_iteration.ceru), [heat_diffusion](../../examples/heat_diffusion.ceru) |
| Dynamic arrays | `[T]`, explicit/range/repeat copies, independent nesting, display/equality/iteration | IR/MIR/VM/C/LLVM/QBE/WAT/ASM/native objects supported; `array_repeat` expands into common IR | [copy](../../examples/dynamic_arrays/copy.ceru), [nested](../../examples/dynamic_arrays/nested.ceru) |
| Named product types | Fields, defaults, update expressions, nesting, value passing | No direct field assignment; construct a new value and reassign the whole binding | [product-point](../../examples/product-point.ceru), [packet_counter](../../examples/packet_counter.ceru) |
| Functions and control flow | Typed parameters/results, explicit type/length parameters, `void`, `if`/`else if`/`else`, `while`/`for`, `break`/`continue`/`return` | No fixed parameter-count limit; recursive CLI calls default to depth 128 | [function_values](../../examples/function_values.ceru), [loop_control](../../examples/loop_control.ceru), [else_if](../../examples/else_if.ceru) |
| Bindings and conversions | Immutable by default, `mut`, explicit `infer`, `T(value)` and `convert<T>(value)` | `infer` is not a runtime type; five float-to-integer rounding modes and explicit saturation are supported | [integer_conversions](../../examples/integer_conversions.ceru) |
| Sum types | `enum` payload variants, exhaustive `match` statements/expressions and guards | No generic Option/Result | [sum_lookup](../../examples/sum_lookup.ceru) |
| Compile-time constants | Typed `const`, dependency evaluation, array type lengths, `pub const` | No function calls or block-local declarations | [constants](../../examples/constants.ceru) |
| Modules | Explicit imports, namespaces, function/type/constant visibility | Cycles/private access diagnosed; no re-exports, module variables, or package distribution | [modules](../../examples/modules/README.en.md) |

The [example type tables](../../examples/README.en.md) list ranges and applications. Whole-array, whole-product, and whole-sum [printing and equality](aggregate-values.en.md) are supported.

## Separate language features from output routes

All features above, including dynamic arrays, are supported by the [IR Executor](ir-executor.en.md), [MIR Executor](mir-executor.en.md), VM, generated C, LLVM, QBE, WAT, Windows/Linux direct assembly, and objects from Cerune's own encoder. Windows/Linux distinguish targets; assembly/objects distinguish artifacts. Use the [route and target table](targets.en.md) when counting them.

Native constructs a SASHIMI typed Program from LIR and generates ASM and COFF/ELF from that same Program, without reparsing ASM for Object generation. Cerune retains instruction selection, ABI, runtime and source-provenance semantics; SASHIMI handles encoding, relocations and object formats. Linking uses external tools. A future internal linker would be compiler implementation work, not a new language feature. See the [native encoder design](native-encoder.en.md).

Language support does not imply equal observation detail. Language check failures have comparable reasons, source locations, and prior output across all routes. LLVM/assembly origin annotations still do not promise debugging information across all routes and optimization stages.

## Execution, distribution, and proof foundations

Language features are tracked separately from compiler stages, distribution, and proof. A new classification or proof experiment does not complete missing language features or route support.

| Status | Foundation | Progress and remaining work |
| --- | --- | --- |
| Implemented | [MIR](ir-stages.en.md) and [independent interpreter](mir-executor.en.md) | Non-SSA types, lowering, validation, `emit-mir`, and `run-mir`; output, failures, origins, and ownership lifetimes compared with existing routes |
| Implemented | [Native lowering from MIR](native-mir.en.md) | Before/after execution comparisons and MIR→LIR→ASM/Object provenance verified |
| Initial scope implemented | [Observation bundle](observation-bundle.en.md) | Saves sources, HIR, MIR, annotated ASM, and manifest from one compilation; `--ssa` adds SSA, reconstructed MIR, mappings, and SSA-derived ASM. Additional artifacts remain future work ([#60](https://github.com/Cerune-Lang/Cerune/issues/60)) |
| Partial experiment | [Lean verification](lean-verification.en.md) | HIR→Lean/HIR→MIR correspondence and properties checked for u8 increment/choose/advance, including if/short-circuiting/while/for/break/continue/nesting, update-failure origins, and completion within explicit bounds; no whole-language, general-lowering, or whole-compiler proof, or public `emit-lean` |
| Partly implemented | [SSA construction](mir-ssa.en.md) and later optimization passes | Scalar SSA/residual-slot representation, structural validator, observation text, automatic construction, and direct execution through the Rust API implemented; public SSA CLI/bundle support also implemented; SSA→MIR/Native also implemented; finite-fixture Lean checks for MIR→SSA→MIR implemented; constant folding is designed but unimplemented and deferred; general proofs also remain future work |
| Implemented | [LIR observation and validation](lir-observation.en.md) | `emit-lir`, `observe --lir`, validator, and example comparisons for both targets. Direct execution investigated and deferred |
| Distributed | Compiler binaries v0.2.0 | Published for Windows x64, Linux x64/arm64, and macOS x64/arm64. Separate from Cerune program build/release and a Darwin Native backend |
| Partially implemented | [IR/VM/Native build / release](owned-routes.en.md); Native build/initial release implemented | Follow [#81](https://github.com/Cerune-Lang/Cerune/issues/81), keeping completed artifacts and distribution separate from optimization |

## Implementation order, technical dependencies, and later candidates

The opening table sets development priorities. Technical prerequisites are listed below. Features outside that sequence, such as array growth, do not yet have an assigned implementation order.

| Start condition | Work and status | Completion criteria and decisions |
| --- | --- | --- |
| Build implemented | [Native build](native-build.en.md); explicit target/Clang produce an executable | Compare examples with IR/MIR/VM; preserve generation stages, origins, and link records. Initial release saves distribution contents, direct runtime requirements and hashes |
| After each artifact stabilizes | IR/VM Images/release remain unimplemented. Initial Native distribution is implemented | Validate versions, types, references, ownership, and budgets before loading; compare saved/reloaded execution. As each route's artifact stabilizes, verify distribution contents, compatibility, and failure origins |
| Later; order undecided | Array growth/borrowing, more string operations, module distribution | Design borrowing lifetimes/mutation, re-exports, dependency versions, and reproducibility around actual uses |
| Deferred/experimental | Optimization passes, broader proofs, added AST/LIR execution, GPU/FPGA | Resume for concrete needs; these do not gate language features, I/O, or build |

Native build can serve current programs without new input or standard-library features. `array_repeat` is implemented. The new investigations do not all become prerequisites for I/O. Release is not another name for optimization. Do not treat observation text as a runnable artifact by bypassing IR/VM Image validation.

Pause SSA expansion after construction, direct execution, Native lowering, and finite-fixture Lean verification. Retain the [constant-folding design](ssa-constant-folding.en.md) and baseline examples, but defer pass implementation. Keep baseline MIR, explicit SSA selection, existing observations, and comparison tests.

## Completed milestones

These items are not the next work queue. Completing an initial scope does not close the entire issue; remaining work is recorded below and in the issue inventory.

| Status | Work | Completed scope and remaining work |
| --- | --- | --- |
| Complete | [LIR observation, validation, and executor decision](lir-observation.en.md) ([#123](https://github.com/Cerune-Lang/Cerune/issues/123)) | Baseline/SSA LIR display, saving, structural validation, and example comparisons implemented. Defer the executor and return to the next language feature |
| Implemented | [Dynamic-array `array_repeat`](owned-arrays.en.md) ([#122](https://github.com/Cerune-Lang/Cerune/issues/122)) | Runtime counts, zero/negative counts, evaluation order, budget failures, and independent nested copies implemented; examples, observable lowering, and existing-route comparisons added |
| Complete | Documentation/issue consistency ([#130](https://github.com/Cerune-Lang/Cerune/issues/130)) | Separate current status from historical audits; automate local-link and translation-pair checks |
| Complete | String-heap owner validation ([#134](https://github.com/Cerune-Lang/Cerune/issues/134)) | Reject another heap’s value even with the same ID; failed retain/release must preserve ownership counts, live bytes, and contents. Reachability from ordinary source remains unconfirmed |
| Initial scope complete | [Standard I/O](standard-io.en.md) ([#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#104](https://github.com/Cerune-Lang/Cerune/issues/104)) | Output, flush, and line input in four executors; CLI/C/LLVM/QBE/WAT/Native connections, failure and dialogue tests implemented. Windows console and shared generated-route success records remain follow-up work |
| Complete | [Floating-point rounding](float-rounding.en.md) ([#156](https://github.com/Cerune-Lang/Cerune/issues/156)) | Merged in PR #163. Minimal cases, bits, and assembly isolate FMA; generated C controls Clang/GCC contraction with regression tests |
| Initial scope implemented | [Compiler input limits](frontend-limits.en.md) ([#153](https://github.com/Cerune-Lang/Cerune/issues/153) / [#135](https://github.com/Cerune-Lang/Cerune/issues/135)) | Diagnose syntax/expression depth and per-file/total-byte/file-count excess. Iterative import traversal preserves order. Other #153 isolation/C/CI candidates remain |
| Initial decision and implementation complete | [Input capacity measurements](input-memory.en.md). Line-input allocation capacity and memory limits ([#154](https://github.com/Cerune-Lang/Cerune/issues/154) / [#133](https://github.com/Cerune-Lang/Cerune/issues/133)) | Capacity, accounted bytes, and release measured across routes; the same example output compared. [Rust/C/LLVM/QBE/Native compaction](input-compaction.en.md) implemented; [WAT policy comparisons](wat-input-storage.en.md) support retaining existing reuse without adopting splitting/coalescing. Broader memory work remains in #133 |
| i32/f64 exports implemented | [Typed calls from JS/TS](typed-host-calls.en.md) ([#161](https://github.com/Cerune-Lang/Cerune/issues/161)) | [WAT library API](wat-library.en.md) retains declarations, selects and validates exports, and emits i32/f64 wrappers with observable IR/conversions. [Checked JS/TS generation](wat-js-bindings.en.md) and an HTML example are also implemented |

### Standard I/O and standard-library status

Separate existing behavior from decisions for the next design.

| Area | Current behavior | Next decisions |
| --- | --- | --- |
| Standard output | `print` displays values; `run`/`run-mir`/`run-vm` deliver output during execution and flush at completion; captured Rust APIs remain | `flush();` is implemented; C/LLVM/QBE/WAT/Native write failures are also implemented; dialogue tests already verify prompts before waiting for input |
| Standard error | Used for compiler/runtime diagnostics; no general program-facing write API | Distinguish program output from runtime diagnostics and define write failures |
| Standard input/arguments | io::read_line and the line/EOF enum work in four executors, CLI, generated C/LLVM/QBE/Native, and versioned WAT hosts | Windows console support remains; arguments work in four executors, CLI and generated C/LLVM/QBE/WAT/Native; see the v0.2.0 table above for file I/O; FFI and networking follow separately |
| Standard library | std:io, std:env, std:num, std:fs, std:process and std:process::run are bundled virtual sources with observable types, text, and origins; no file lookup | Distinguish language built-ins, runtime connections to external resources, and libraries written in Cerune |

The [standard I/O design](standard-io.en.md) records responsibilities and implementation order. The [input-value branching example](../../examples/io_protocol/README.en.md) runs today but does not use stdin. The [line-input contract and shared Rust connection](line-input.en.md) are added. The initial API returns a line or EOF and stops on errors; existing enums suffice without generic Result. Virtual std:io read_line/ReadLine are connected to four executors. CLI/C/LLVM/QBE/Native stdin is implemented within the [initial scope](stdin-connections.en.md); versioned WAT input hosts are implemented; Native input is implemented. Compare with identical inputs and controlled output destinations, testing real connections separately. A general record/replay or effect system is not a prerequisite.

The [fixed-capacity used-length](../../examples/bounded_sequence.ceru) and [range-copy](../../examples/array_window.ceru) examples run with current features; they do not implement dynamic arrays or borrowed slices.

Existing foundations are [common failure records](runtime-diagnostics.en.md), [file origins](source-files.en.md), [modules](modules.en.md), [constants](constants.en.md), [functions](functions.en.md), [product updates](product-updates.en.md), [sums](sum-types.en.md), and [fixed arrays](fixed-arrays.en.md). The [mixed-argument](../../examples/function_arguments.ceru) and [array-length](../../examples/array_length.ceru) examples check value passing and evaluation order.

This is not a commitment to implement everything together. GPU experiments need not await every stage. Inheritance, implicit shared mutable references, automatic GPU dispatch, and general async facilities are not prioritized without evidence from current examples.

## Open issues

This inventory covers the 31 issues open on 2026-11-24. Some track follow-up work on features already shipped.

| Area | Issues | Remaining work |
| --- | --- | --- |
| Compilation and execution records | [#60](https://github.com/Cerune-Lang/Cerune/issues/60), [#94](https://github.com/Cerune-Lang/Cerune/issues/94), [#282](https://github.com/Cerune-Lang/Cerune/issues/282) | `observe` saves compiler representations. #282 proposes recording the path taken at run time. |
| I/O and repeatable inputs | [#89](https://github.com/Cerune-Lang/Cerune/issues/89), [#104](https://github.com/Cerune-Lang/Cerune/issues/104), [#173](https://github.com/Cerune-Lang/Cerune/issues/173), [#243](https://github.com/Cerune-Lang/Cerune/issues/243) | UTF-8 files and child processes are available. Binary I/O and comparisons that avoid repeating external effects remain open. |
| Arithmetic and source syntax | [#88](https://github.com/Cerune-Lang/Cerune/issues/88), [#105](https://github.com/Cerune-Lang/Cerune/issues/105), [#198](https://github.com/Cerune-Lang/Cerune/issues/198), [#306](https://github.com/Cerune-Lang/Cerune/issues/306) | Proposals for explicit wrapping, inference in array iteration and conditional expressions, block comments, and indentation-based blocks. |
| Artifacts and targets | [#81](https://github.com/Cerune-Lang/Cerune/issues/81), [#92](https://github.com/Cerune-Lang/Cerune/issues/92), [#284](https://github.com/Cerune-Lang/Cerune/issues/284) | IR/VM artifacts, rechecking Windows QBE argument passing, and RV64 Linux instruction selection. |
| Compiler maintenance and testing | [#103](https://github.com/Cerune-Lang/Cerune/issues/103), [#118](https://github.com/Cerune-Lang/Cerune/issues/118), [#131](https://github.com/Cerune-Lang/Cerune/issues/131), [#133](https://github.com/Cerune-Lang/Cerune/issues/133), [#153](https://github.com/Cerune-Lang/Cerune/issues/153), [#262](https://github.com/Cerune-Lang/Cerune/issues/262), [#268](https://github.com/Cerune-Lang/Cerune/issues/268) | Code organization, observation files, generated differential tests, memory use, input checks, CI and compiler costs. |
| Calls from the web | [#161](https://github.com/Cerune-Lang/Cerune/issues/161), [#258](https://github.com/Cerune-Lang/Cerune/issues/258) | WAT exports i32/f64. Strings, arrays and a dedicated CLI are candidates. Tarot owns its UI and application tests. |
| Research | [#93](https://github.com/Cerune-Lang/Cerune/issues/93), [#235](https://github.com/Cerune-Lang/Cerune/issues/235), [#244](https://github.com/Cerune-Lang/Cerune/issues/244), [#249](https://github.com/Cerune-Lang/Cerune/issues/249), [#254](https://github.com/Cerune-Lang/Cerune/issues/254), [#255](https://github.com/Cerune-Lang/Cerune/issues/255), [#256](https://github.com/Cerune-Lang/Cerune/issues/256), [#257](https://github.com/Cerune-Lang/Cerune/issues/257) | FPGA, testing a self-hosted compiler across routes, hardware I/O for an OS, and GUI APIs and event handling. |

## GPU direction

GPU execution is worth including as a future target. Observing how sequential CPU computations become parallel element computations fits Cerune's purpose. GPU generation and execution are currently unimplemented. This proposal concerns computation, not a graphics API.

### Decide types and execution contracts first

Selecting an LLVM GPU backend does not make existing programs run unchanged. NVPTX distinguishes host-launchable kernels from device functions and defines GPU address spaces. CPU output helpers also cannot simply be assumed. See the [LLVM NVPTX guide](https://llvm.org/docs/NVPTXUsage.html).

The API and hardware have not been selected:

| Candidate | Considerations |
| --- | --- |
| WebGPU / WGSL | A possible starting point for a limited 32-bit numeric experiment. WGSL runtime scalar types do not include `u64` or `f64`, so this would not support all current Cerune types |
| Vulkan / SPIR-V | Query features such as `shaderInt64` and `shaderFloat64`; do not assume 64-bit support on every device |
| LLVM NVPTX / CUDA | An NVIDIA-targeted option requiring kernel calling conventions, memory rules, and host launch/result handling |

These constraints follow from [WGSL scalar types](https://www.w3.org/TR/WGSL/#scalar-types) and [Vulkan features](https://docs.vulkan.org/spec/latest/chapters/features.html). Unsupported `u64` values must never silently become `f32`. Reject unsupported capabilities explicitly, or separately verify an implementation that preserves their meaning.

At minimum, GPU support should expose:

- Input/output element types, lengths, layouts, host/device ownership boundaries, transfers, and readback.
- The relationship between logical elements and work, dispatch bounds, and synchronization. Observation APIs must not enable external mutation during execution.
- Integer range and index checks, with a contract for recovering failure reason, source location, and logical element index.
- Floating-point rounding, operation grouping, and comparison policy. Declare any tolerance in advance without silently weakening existing CPU semantics.
- Separate compilation, transfer, kernel execution, and readback timings. Do not assume small examples become faster.

Do not equate global GPU execution order with CPU `print` order. Initially prohibit output inside kernels and explicitly print results in element order on the host after readback. See the [Vulkan compute tutorial](https://docs.vulkan.org/tutorial/latest/11_Compute_Shader.html) for synchronization requirements.

### First experiment and completion criteria

1. Choose elementwise addition/transformation with read-only input arrays and disjoint output writes. Initially require a restricted form whose ranges and indices can be checked for safety.
2. Generate for explicit types, hardware, and tools, and compare against known CPU answers. Reject unsupported types or computations whose safety is not established.
3. Before supporting general computation, implement failure reporting. Define an execution-order-independent rule for multiple failures, such as selecting the smallest logical element index.
4. Separately verify element counts not divisible by the workgroup size, bounds failures, overflow, missing capabilities, and transfer failures.
5. Then expand to matrix operations and heat diffusion. [matrix_vector_product](../../examples/matrix_vector_product.ceru) and [heat_diffusion](../../examples/heat_diffusion.ceru) are existing CPU comparison examples, not implemented GPU examples. Replacing the latter's `f64` with `f32` for a limited route would be a separately typed experiment.

The initial experiment excludes conflicting shared writes, atomics, parallel reductions, graphics, and an internal GPU machine-code encoder.

## Completion criteria for language additions

Update Japanese design rationale, matching English documentation, a small example categorized by type, known expected results, and successful/failing execution comparisons together. Distinguish normal completion, expected diagnostics/stops, unexpected failures, and unexecuted checks. A nonzero exit alone is not evidence of success.

For common language additions, preserve semantics across existing routes and verify evaluation order, short-circuiting, independent copies, string bytes, and origins. Limited experiments such as GPU support must declare their scope; do not claim full route parity while features remain unsupported.
