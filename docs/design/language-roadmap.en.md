# Language capabilities and next steps

[日本語](language-roadmap.ja.md)

This inventory records capabilities, execution routes, and next steps as of 2026-10-13. Implemented work is distinguished from unimplemented plans; each design stage determines candidate syntax and adoption. See the [language reference](../reference/language.en.md) for the exact specification.

The [earlier audit](development-status.en.md) preserves the 2026-10-11 baseline. This page tracks current status.

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
| Planned, unimplemented | [IR/VM/Native build / release](owned-routes.en.md) | Follow [#81](https://github.com/Cerune-Lang/Cerune/issues/81), keeping completed artifacts and distribution separate from optimization |

## Proposed priorities

Pause SSA expansion after construction, direct execution, Native lowering, and finite-fixture Lean verification. Retain the [constant-folding design](ssa-constant-folding.en.md) and baseline examples, but defer pass implementation. Keep baseline MIR, explicit SSA selection, existing observations, and comparison tests.

Prioritize the programs users can write and the artifacts they can use. This is a work priority, not a chain in which every item requires the previous one to finish.

| Priority | Work and current status | Initial completion criteria |
| --- | --- | --- |
| Completed milestone | [LIR observation, validation, and executor decision](lir-observation.en.md) ([#123](https://github.com/Cerune-Lang/Cerune/issues/123)) | Baseline/SSA LIR display, saving, structural validation, and example comparisons implemented. Defer the executor and return to the next language feature |
| Implemented feature | [Dynamic-array `array_repeat`](owned-arrays.en.md) ([#122](https://github.com/Cerune-Lang/Cerune/issues/122)) | Runtime counts, zero/negative counts, evaluation order, budget failures, and independent nested copies implemented; examples, observable lowering, and existing-route comparisons added |
| Complete | Documentation/issue consistency ([#130](https://github.com/Cerune-Lang/Cerune/issues/130)) | Separate current status from historical audits; automate local-link and translation-pair checks |
| Complete | String-heap owner validation ([#134](https://github.com/Cerune-Lang/Cerune/issues/134)) | Reject another heap’s value even with the same ID; failed retain/release must preserve ownership counts, live bytes, and contents. Reachability from ordinary source remains unconfirmed |
| 1: language features | [Standard I/O design](standard-io.en.md) ([#89](https://github.com/Cerune-Lang/Cerune/issues/89), [#104](https://github.com/Cerune-Lang/Cerune/issues/104)). Incremental output implemented in four executors and CLI; explicit flush also implemented across routes; stdin unimplemented | C/LLVM/QBE/Native write failures are also implemented; next extend the contract to WAT, then bounded UTF-8 line input and interaction. Compare identical input/operation sequences; keep observation read-only |
| 3 | Small standard library. Standard module layout/APIs undecided | Select text/numeric parsing helpers required by I/O. Separate functions expressible in Cerune from runtime operations; use existing imports and explicit failure handling. Do not require generic Option/Result or package management first |
| Independently actionable | [Native build](owned-routes.en.md). Responsibilities outlined; CLI/link management unimplemented | An executable is the proposed first artifact. Define explicit target, external linker, dependencies, failures, and observation records; build/link current examples and compare with IR/VM |
| After stable build | IR/VM Images and release for three routes. Unimplemented | Validate versions, types, references, ownership, and budgets before loading; compare saved/reloaded execution. As each route's artifact stabilizes, verify distribution contents, compatibility, and failure origins |
| Later | Recursion, array growth/borrowing, more string operations, module distribution | Design call storage/limits/cleanup, borrowing lifetimes/mutation, re-exports, dependency versions, and reproducibility around actual uses |
| Deferred/experimental | Optimization passes, broader proofs, added AST/LIR execution, GPU/FPGA | Resume for concrete needs; these do not gate language features, I/O, or build |

Native build can serve current programs without new input or standard-library features. `array_repeat` is implemented. The new investigations do not all become prerequisites for I/O. Release is not another name for optimization. Do not treat observation text as a runnable artifact by bypassing IR/VM Image validation.

### Standard I/O and standard-library status

Separate existing behavior from decisions for the next design.

| Area | Current behavior | Next decisions |
| --- | --- | --- |
| Standard output | `print` displays values; `run`/`run-mir`/`run-vm` deliver output during execution and flush at completion; captured Rust APIs remain | `flush();` is implemented; C/LLVM/QBE/Native write failures are also implemented; WAT remain, and tests showing prompts before waiting for input |
| Standard error | Used for compiler/runtime diagnostics; no general program-facing write API | Distinguish program output from runtime diagnostics and define write failures |
| Standard input/arguments | No general program-facing API | Use stdin as the first use case; scope CLI arguments, files, FFI, and networking separately later |
| Standard library | Built-ins and user modules exist; standard-module distribution, lookup, and compatibility policies do not | Distinguish language built-ins, runtime connections to external resources, and libraries written in Cerune |

The [standard I/O design](standard-io.en.md) records responsibilities and implementation order. The [input-value branching example](../../examples/io_protocol/README.en.md) runs today but does not use stdin. API names, result types, and which errors are returned as values remain undecided. Distinguish EOF and read failures from existing runtime traps, and consider representation using current enums. Compare with identical inputs and controlled output destinations, testing real connections separately. A general record/replay or effect system is not a prerequisite.

The [fixed-capacity used-length](../../examples/bounded_sequence.ceru) and [range-copy](../../examples/array_window.ceru) examples run with current features; they do not implement dynamic arrays or borrowed slices.

Existing foundations are [common failure records](runtime-diagnostics.en.md), [file origins](source-files.en.md), [modules](modules.en.md), [constants](constants.en.md), [functions](functions.en.md), [product updates](product-updates.en.md), [sums](sum-types.en.md), and [fixed arrays](fixed-arrays.en.md). The [mixed-argument](../../examples/function_arguments.ceru) and [array-length](../../examples/array_length.ceru) examples check value passing and evaluation order.

This is not a commitment to implement everything together. GPU experiments need not await every stage. Inheritance, implicit shared mutable references, automatic GPU dispatch, and general async facilities are not prioritized without evidence from current examples.

### Current issue inventory

On 2026-10-13, 16 open issues and two recently completed issues were checked. The table records priorities and purposes; investigation issues are not confirmed defects.

| Issues | Purpose | Next work and limits |
| --- | --- | --- |
| [#130](https://github.com/Cerune-Lang/Cerune/issues/130) | Documentation consistency | Merged in PR #136: old URLs, licensing, historical audits, and bilingual pairs |
| [#134](https://github.com/Cerune-Lang/Cerune/issues/134) | String ownership correctness | Merged in PR #137: validate heap membership/owner counts before mutation, separately from performance |
| [#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#104](https://github.com/Cerune-Lang/Cerune/issues/104) | Standard I/O and effect comparisons | Output connections merged. Explicit flush implemented; C/LLVM/QBE/Native write failures also implemented; remaining WAT work → bounded UTF-8 input → interactive example |
| [#135](https://github.com/Cerune-Lang/Cerune/issues/135) | Deep parser inputs | Unreproduced investigation. Use bounded child processes; design diagnostics/depth limits if needed |
| [#131](https://github.com/Cerune-Lang/Cerune/issues/131) | Generated well-typed differential tests | Start with fixed seeds, finite execution, and known expectations; separate from parser and internal-API checks |
| [#133](https://github.com/Cerune-Lang/Cerune/issues/133) | Memory measurement | Distinguish logical budgets, actual allocations, RSS, and WAT pages. No confirmed leak/regression; measure before changing representation |
| [#132](https://github.com/Cerune-Lang/Cerune/issues/132) | Actions Node.js warning | Independent maintenance; distinguish action-runtime Node from WAT-test Node. Not an I/O prerequisite |
| [#81](https://github.com/Cerune-Lang/Cerune/issues/81) | build / release and route responsibilities | Native build can start independently; then Image validation/distribution. show/via/optional backends await use cases and detailed design |
| [#60](https://github.com/Cerune-Lang/Cerune/issues/60) / [#94](https://github.com/Cerune-Lang/Cerune/issues/94) | Observation, transformations, proofs | Partly complete through SSA/LIR observation. More bundled artifacts, general proofs, and optimization remain; preserve unoptimized forms |
| [#88](https://github.com/Cerune-Lang/Cerune/issues/88) | Integer arithmetic policies | Keep checked arithmetic; prioritize demonstrated wrapping uses. Define saturation, overflow reporting, widening, bit discard, and division separately |
| [#90](https://github.com/Cerune-Lang/Cerune/issues/90) / [#105](https://github.com/Cerune-Lang/Cerune/issues/105) | Longer-program syntax | Align optional let and omitted infer, then else if. Design conditional semantics, formatter, and for coverage separately |
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
