# Language capabilities and next steps

[日本語](language-roadmap.ja.md)

This inventory records capabilities, execution routes, and next steps as of 2026-10-23. Implemented work is distinguished from unimplemented plans; each design stage determines candidate syntax and adoption. See the [language reference](../reference/language.en.md) for the exact specification.

The [earlier audit](development-status.en.md) preserves the 2026-10-11 baseline. This page tracks current status.

## Core development order

**Argument acquisition is merged across existing routes. Before integer parsing, prioritize assembler separation in [#183](https://github.com/Cerune-Lang/Cerune/issues/183).** Extend Cerune in this order so programs can use external values to select computations, then save and reuse results.

| Order | Work | First usable outcome | Status |
| --- | --- | --- | --- |
| Preceding work | [SASHIMI connection / assembler separation](native-encoder.en.md) (#183) | Preserve pinned-object equivalence and define typed machine instructions and origin ranges | Rust connection, migration comparisons and SASHIMI typed single-instruction API implemented; typed Program/object and OriginId remain next |
| 1 | [Program arguments](https://github.com/Cerune-Lang/Cerune/issues/174) → [integer parsing](https://github.com/Cerune-Lang/Cerune/issues/175) | Distinguish empty/missing values, invalid syntax and range errors; compare a seed-argument example across routes | Arguments implemented in four executors, CLI and generated C/LLVM/QBE/WAT/Native; integer parsing is next |
| 2 | [Optional let + fmt](https://github.com/Cerune-Lang/Cerune/issues/90) | Preserve/show/hide let, retain comments, and verify formatting idempotence and semantics; align with [infer omission and related syntax](https://github.com/Cerune-Lang/Cerune/issues/105) | Design and implement together; unimplemented |
| 3 | [Native build and initial distribution](https://github.com/Cerune-Lang/Cerune/issues/81) | Build an executable with explicit target/linker; distribute dependencies, execution requirements, and origins | Responsibilities outlined; unimplemented |
| 4 | [Recursion](functions.en.md) | Align per-call storage, ownership/cleanup, limits, failure diagnostics, call stacks and origins across routes | Design and implementation pending |
| 5 | Limited file I/O → subprocess operations ([#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#173](https://github.com/Cerune-Lang/Cerune/issues/173)) | Save/reload first, then explicit shell-free execution and output/exit-status collection; define resources, capacity/time limits, and failures | Follow-up design; unimplemented |

This implementation order was agreed on 2026-10-20. In step 1, complete argument acquisition across routes, then add pure integer parsing and the seed-argument example. Resolve public names, limits, and connection contracts in the [proposal](dogfooding-inputs.en.md); use small standard modules without requiring generic Option/Result or package management.

Add optional let and fmt together in step 2. Preserve types, mutability, and evaluation order, and keep original and formatted sources observable with their respective source locations. Recursion need not wait for file I/O; start with independent call frames, including WAT temporary storage.

Step 3 distributes programs written in Cerune. Compiler binary updates can proceed separately. Initial Native distribution does not wait for IR/VM Images; those routes require separate format, validator, and loader work. Release does not imply optimization.

The [fixed-seed baseline example](../../examples/deterministic_seed.ceru) is added in PR #176. An [argument-display example](../../examples/arguments/README.en.md) runs in four executors, CLI and generated C/LLVM/QBE/WAT/Native. Integer parsing remains unimplemented. Parallax seed selection is one validation use case for general Cerune features. Retain Parallax's independent Python oracle.

At each step, preserve observation of original unoptimized forms, transformations, inputs, outputs, stop reasons, and origins; compare identical inputs across existing routes. Observation APIs must not mutate execution state.

Organization repository responsibilities and progress have a [separate inventory](ecosystem.en.md). Finishing surrounding tools, websites, or experiments is not a prerequisite for these language additions.

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
| Strings | `string`, printing, `==`, `!=`, `byte_len`, `concat` | No indexing, character count, or numeric conversion | [string_byte_length](../../examples/string_byte_length.ceru), [string_lookup](../../examples/string_lookup.ceru) |
| Fixed arrays | `[T; N]`, `array_len`, `for … in`, nesting, value passing, updates through `mut` | Length belongs to the type; indices are `i64`; use `[T]` for dynamic lengths; no borrowed slices | [array_iteration](../../examples/array_iteration.ceru), [heat_diffusion](../../examples/heat_diffusion.ceru) |
| Dynamic arrays | `[T]`, explicit/range/repeat copies, independent nesting, display/equality/iteration | IR/MIR/VM/C/LLVM/QBE/WAT/ASM/native objects supported; `array_repeat` expands into common IR | [copy](../../examples/dynamic_arrays/copy.ceru), [nested](../../examples/dynamic_arrays/nested.ceru) |
| Named product types | Fields, defaults, update expressions, nesting, value passing | No direct field assignment; construct a new value and reassign the whole binding | [product-point](../../examples/product-point.ceru), [packet_counter](../../examples/packet_counter.ceru) |
| Functions and control flow | Typed parameters/results, explicit type/length parameters, `void`, `if`/`else`, `while`/`for`, `break`/`continue`/`return` | No fixed parameter-count limit; no recursion | [function_values](../../examples/function_values.ceru), [loop_control](../../examples/loop_control.ceru) |
| Bindings and conversions | Immutable by default, `mut`, explicit `infer`, `T(value)` and `convert<T>(value)` | `infer` is not a runtime type; five float-to-integer rounding modes and explicit saturation are supported | [integer_conversions](../../examples/integer_conversions.ceru) |
| Sum types | `enum` payload variants, exhaustive `match` statements/expressions and guards | No generic Option/Result | [sum_lookup](../../examples/sum_lookup.ceru) |
| Compile-time constants | Typed `const`, dependency evaluation, array type lengths, `pub const` | No function calls or block-local declarations | [constants](../../examples/constants.ceru) |
| Modules | Explicit imports, namespaces, function/type/constant visibility | Cycles/private access diagnosed; no re-exports, module variables, or package distribution | [modules](../../examples/modules/README.en.md) |

The [example type tables](../../examples/README.en.md) list ranges and applications. Whole-array, whole-product, and whole-sum [printing and equality](aggregate-values.en.md) are supported.

## Separate language features from output routes

All features above, including dynamic arrays, are supported by the [IR Executor](ir-executor.en.md), [MIR Executor](mir-executor.en.md), VM, generated C, LLVM, QBE, WAT, Windows/Linux direct assembly, and objects from Cerune's own encoder. Windows/Linux distinguish targets; assembly/objects distinguish artifacts. Use the [route and target table](targets.en.md) when counting them.

The native encoder generates x86-64 instructions and COFF/ELF objects. It shares assembly lowering and currently reads an internal assembly representation to encode it. Linking uses external tools. A typed machine-instruction IR or an internal linker would be compiler implementation work, not new language features. See the [native encoder design](native-encoder.en.md).

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
| Distributed | Compiler binaries v0.1.0 | Published for Windows x64, Linux x64/arm64, and macOS x64/arm64. Separate from Cerune program build/release and a Darwin Native backend |
| Planned, unimplemented | [IR/VM/Native build / release](owned-routes.en.md) | Follow [#81](https://github.com/Cerune-Lang/Cerune/issues/81), keeping completed artifacts and distribution separate from optimization |

## Implementation order, technical dependencies, and later candidates

The opening table sets development priorities. Technical prerequisites are listed below. Features outside that sequence, such as array growth, do not yet have an assigned implementation order.

| Start condition | Work and status | Completion criteria and decisions |
| --- | --- | --- |
| Priority 3; technically independent | [Native build](owned-routes.en.md). Responsibilities outlined; CLI/link management unimplemented | An executable is the proposed first artifact. Define explicit target, external linker, dependencies, failures, and observation records; build/link current examples and compare with IR/VM |
| After each artifact stabilizes | IR/VM Images and per-route release; initial Native distribution is priority 3. Unimplemented | Validate versions, types, references, ownership, and budgets before loading; compare saved/reloaded execution. As each route's artifact stabilizes, verify distribution contents, compatibility, and failure origins |
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
| Standard input/arguments | io::read_line and the line/EOF enum work in four executors, CLI, generated C/LLVM/QBE/Native, and versioned WAT hosts | Windows console support remains; arguments work in four executors, CLI and generated C/LLVM/QBE/WAT/Native; files, FFI and networking follow separately |
| Standard library | std:io and std:env are bundled virtual sources with observable types, text, and origins; no file lookup | Distinguish language built-ins, runtime connections to external resources, and libraries written in Cerune |

The [standard I/O design](standard-io.en.md) records responsibilities and implementation order. The [input-value branching example](../../examples/io_protocol/README.en.md) runs today but does not use stdin. The [line-input contract and shared Rust connection](line-input.en.md) are added. The initial API returns a line or EOF and stops on errors; existing enums suffice without generic Result. Virtual std:io read_line/ReadLine are connected to four executors. CLI/C/LLVM/QBE/Native stdin is implemented within the [initial scope](stdin-connections.en.md); versioned WAT input hosts are implemented; Native input is implemented. Compare with identical inputs and controlled output destinations, testing real connections separately. A general record/replay or effect system is not a prerequisite.

The [fixed-capacity used-length](../../examples/bounded_sequence.ceru) and [range-copy](../../examples/array_window.ceru) examples run with current features; they do not implement dynamic arrays or borrowed slices.

Existing foundations are [common failure records](runtime-diagnostics.en.md), [file origins](source-files.en.md), [modules](modules.en.md), [constants](constants.en.md), [functions](functions.en.md), [product updates](product-updates.en.md), [sums](sum-types.en.md), and [fixed arrays](fixed-arrays.en.md). The [mixed-argument](../../examples/function_arguments.ceru) and [array-length](../../examples/array_length.ceru) examples check value passing and evaluation order.

This is not a commitment to implement everything together. GPU experiments need not await every stage. Inheritance, implicit shared mutable references, automatic GPU dispatch, and general async facilities are not prioritized without evidence from current examples.

## Issue inventory and remaining work

On 2026-10-20, 17 open issues, recent merges and Organization progress were checked; argument acquisition #174 and integer parsing #175 were split out (19 open issues afterward). This table records remaining core work and recent completions; use the opening table for implementation order. Investigation issues are not confirmed defects.

| Issues | Purpose | Next work and limits |
| --- | --- | --- |
| [#156](https://github.com/Cerune-Lang/Cerune/issues/156) | Floating-point semantic consistency | Completed in PR #163: Clang/GCC controls, example, regression tests, and bilingual contract. CI including macOS passed |
| [#161](https://github.com/Cerune-Lang/Cerune/issues/161) | Typed calls from JS/TS | Initial scope implemented. [WAT library Rust API](wat-library.en.md) implemented; [generated checked JS/TS](wat-js-bindings.en.md) and an HTML example are also implemented; assess additional types and IR/VM exposure against real uses |
| [#130](https://github.com/Cerune-Lang/Cerune/issues/130) | Documentation consistency | Merged in PR #136: old URLs, licensing, historical audits, and bilingual pairs |
| [#134](https://github.com/Cerune-Lang/Cerune/issues/134) | String ownership correctness | Merged in PR #137: validate heap membership/owner counts before mutation, separately from performance |
| [#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#104](https://github.com/Cerune-Lang/Cerune/issues/104) | Standard I/O and effect comparisons | Output connections merged. Explicit flush implemented; C/LLVM/QBE/WAT/Native write failures also implemented; virtual std:io, line input, and ownership connected to four executors; CLI/C/LLVM/QBE/Native stdin and dialogue tests added; versioned WAT input hosts implemented; Native input implemented. Next are argument acquisition (#174) and integer parsing (#175). Shared successful-operation records remain follow-up work |
| [#153](https://github.com/Cerune-Lang/Cerune/issues/153) / [#135](https://github.com/Cerune-Lang/Cerune/issues/135) | Parser and source-loading bounds | Initial scope implemented: common frontend diagnostics, bounded subprocess and CLI boundary checks. Import isolation, generated C, and CI candidates remain separate |
| [#131](https://github.com/Cerune-Lang/Cerune/issues/131) | Generated well-typed differential tests | [Parallax](ecosystem.en.md) implements fixed seeds, expressions, branches, loops, arrays and IR/MIR/VM comparisons against independent expectations. SSA, compiled routes and general origin comparison remain |
| [#173](https://github.com/Cerune-Lang/Cerune/issues/173) / [#174](https://github.com/Cerune-Lang/Cerune/issues/174) / [#175](https://github.com/Cerune-Lang/Cerune/issues/175) | Select missing capabilities from real uses | [Responsibility inventory and argument design](dogfooding-inputs.en.md). Arguments/parsing are priority 1; fmt, Native distribution, and recursion precede file I/O/subprocesses at priority 5. Retain the independent Python oracle |
| [#154](https://github.com/Cerune-Lang/Cerune/issues/154) / [#133](https://github.com/Cerune-Lang/Cerune/issues/133) | Input capacity and memory measurement | Initial measurements and policy decisions are complete. Distinguish memory limit, capacity, RSS, and WAT pages. Measure large limits with short input; do not claim unconfirmed leaks or regressions |
| [#132](https://github.com/Cerune-Lang/Cerune/issues/132) | Actions Node.js warning | Completed in PR #157. Distinguish action-runtime Node 24 from WAT-test Node 20.20.2 |
| [#81](https://github.com/Cerune-Lang/Cerune/issues/81) | build / release and route responsibilities | Priority 3 covers Native build and initial distribution. IR/VM Image formats, validation, and loading follow separately. show/via/optional backends await use cases and detailed design |
| [#60](https://github.com/Cerune-Lang/Cerune/issues/60) / [#94](https://github.com/Cerune-Lang/Cerune/issues/94) | Observation, transformations, proofs | Partly complete through SSA/LIR observation. More bundled artifacts, general proofs, and optimization remain; preserve unoptimized forms |
| [#88](https://github.com/Cerune-Lang/Cerune/issues/88) | Integer arithmetic policies | Keep checked arithmetic; prioritize demonstrated wrapping uses. Define saturation, overflow reporting, widening, bit discard, and division separately |
| [#90](https://github.com/Cerune-Lang/Cerune/issues/90) / [#105](https://github.com/Cerune-Lang/Cerune/issues/105) | Longer-program syntax | Priority 2 adds optional let and fmt together; align with infer omission, then else if. Design conditional semantics and for coverage separately |
| [#92](https://github.com/Cerune-Lang/Cerune/issues/92) | QBE Windows | Retain workaround; revalidate against official upstream changes. Preserve the research branch unmerged |
| [#103](https://github.com/Cerune-Lang/Cerune/issues/103) / [#118](https://github.com/Cerune-Lang/Cerune/issues/118) | Code organization and observation artifacts | Refactor affected responsibilities; distinguish local records from public fixtures and review shared source/path/environment data |
| [#93](https://github.com/Cerune-Lang/Cerune/issues/93) | FPGA | Investigate another realization of the same IR semantics; not a prerequisite for language features |

License issue [#119](https://github.com/Cerune-Lang/Cerune/issues/119) is complete: PR #129 adopted **MIT OR Apache-2.0** ([MIT](../../LICENSE-MIT) / [Apache-2.0](../../LICENSE-APACHE)). LIR #123 and array_repeat #122 are also complete, not reopened backlog.

Experiment topics belong to the [idea catalog](../idea/idea-catalog.en.md), not the adoption/order commitments. Issues hold details/discussion, the roadmap tracks progress/dependencies, and references describe implemented behavior.

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
