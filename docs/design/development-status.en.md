# Execution routes and unfinished work inventory

[日本語](development-status.ja.md) · [Roadmap](language-roadmap.en.md)

This page records the audit baseline below. Since then, [LIR observation and structural validation](lir-observation.en.md) have been implemented and direct execution deferred. See the [roadmap](language-roadmap.en.md) for current priorities.

## Audit baseline and conclusion

On 2026-10-11, compare master `763a9fb` (merged PR #120) with PR #121 at `9904461`. The constant-folding design, baseline examples, and priority changes remain PR changes at this audit baseline. No optimization pass is implemented.

**LIR is implemented as an internal Native generation stage. Direct LIR execution, a separate public observation format, and a dedicated structural validator are not implemented.** Native execution, LIR display, and direct LIR evaluation are separate milestones. The previous priority table did not make this distinction clear.

## Current flow

Solid arrows below represent implemented transformations and execution. Each executor consumes its representation, not the result of the preceding executor.

```text
Source → shared frontend → HIR ─────────────→ IR Executor (run / run-ir)
                            ├→ Bytecode ───→ VM (run-vm)
                            ├→ C / LLVM / QBE / WAT → emit (external execution)
                            └→ non-SSA MIR → MIR Executor (run-mir)
                                  ├→ SSA ──→ SSA Executor (run-mir --ssa)
                                  │    └→ MIR with copies ─┐
                                  └────────────────────────┴→ x86-64 LIR
                                                                ↓
                                                               ASM
                                                                ├→ external assembler → Object
                                                                └→ internal encoder ──→ Object
                                                                                          ↓
                                                                                external link → CPU
```

Users, tests, or observation scripts currently perform external linking and Native launch, not a public build command. SSA is a MIR form, Windows/Linux are targets, and ASM/Object are artifacts; they should not all be added into one route count.

| Stage | Representation/transformation | Separate observation | Direct evaluation/execution |
| --- | --- | --- | --- |
| Source/AST | Frontend parsing and expansion | `emit-sources` contains text/file mappings, not a dedicated AST dump | No AST Executor; the study recommends deferring adoption |
| HIR | Resolved types/names/expansion/ownership | `emit-ir` | IR Executor |
| Non-SSA MIR | Temporaries, blocks, explicit control flow | `emit-mir` | MIR Executor |
| SSA | Scalar SSA with residual slots and MIR mappings | `emit-mir --ssa`, mappings, bundles | SSA Executor; Native generation reconstructs MIR with copies |
| Bytecode | Stack VM instructions and origins | `emit-bytecode` | VM, also exposing a Rust function-call API |
| x86-64 LIR | Target, frames, stack/register operations and pseudo-instructions | Rust Module and ASM annotations; no dedicated text output or bundle snapshot | No LIR Executor/`run-lir` |
| ASM/Object | Windows/Linux ASM and internal COFF/ELF | `emit-asm`/`emit-obj`, annotations, symbols, disassembly | CPU execution after linking |
| C/LLVM/QBE/WAT | HIR lowered to backend representations | Respective `emit-*` commands | External toolchains/runtimes; current CLI does not manage launch |
| Lean | Experimental exporters for finite models/theorems | Artifacts in `experiments/lean` | External Lean checks; no public `emit-lean` or whole-language proof |

MIR/SSA execution does not establish new build artifacts or distribution routes. Current build/release plans cover IR, VM, and Native.

## Implemented and missing LIR work

Evidence: [LIR types](../../src/codegen/x86_64/ir.rs), [Rust API](../../src/codegen/x86_64.rs), [lowering](../../src/codegen/x86_64/lower.rs), [emitter](../../src/codegen/x86_64/emit.rs), and [Native design](native-mir.en.md).

| Area | Current state |
| --- | --- |
| MIR→LIR separation | `x86_64::lower_mir(&mir, target)` returns a Module; validates MIR and selects layout/ABI for the explicit target |
| LIR→ASM separation | Emitter consumes Module; this is not a single MIR-to-string procedure |
| Origins | Per-instruction Origin/MirOrigin, propagated to ASM `cerune-mir` annotations and Object symbols |
| Instruction granularity | Register operations such as `MoveRaxToRcx` coexist with `CheckIntegerRange`, `ArrayAllocate`, and `Call`; emitter expands checks, ABI, and runtime operations further |
| Validation | MIR validation, some emitter consistency assertions, generated-artifact and Native execution tests; no independent whole-LIR structural validator |
| Public observation | No `emit-lir`, LIR text schema, or LIR bundle snapshot; instruction numbers in ASM annotations do not display LIR bodies |
| Independent execution | No direct LIR evaluator; MIR execution or Native launch is not an LIR Executor |
| Complete machine-instruction IR | Unimplemented; the encoder parses the internal ASM emitted by the emitter |

LIR is neither untouched nor complete through independent observation/execution. The transformation stage is separated, but public observation and validation remain unfinished.

### Value and cost of direct LIR execution

Independent evaluation could distinguish MIR→LIR errors from LIR→ASM/Object errors. It requires deciding how to model registers, frames, memory, branches, checks, calling conventions, and runtime operations.

Correct pseudo-instruction evaluation does not establish correctness of expanded x86-64 instructions, encoding, relocations, the real linker, or OS ABI. Native execution comparisons remain necessary. Delegating back to MIR/VM would not provide this independent check.

**First design LIR observation and structural validation; decide direct execution separately.** Track this in [#123](https://github.com/Cerune-Lang/Cerune/issues/123). Do not bundle LIR execution, typed machine-instruction IR, an internal linker, and JIT into one implementation project.

## Revised priorities

Group work by purpose instead of requiring a large research project to finish before new language features.

| Priority | Work | Dependencies/completion |
| --- | --- | --- |
| This audit | Correct status tables and conflicting descriptions | Distinguish implemented SSA, absent LIR observation/execution, and absent build |
| Next design check | LIR observation/validation and execution adoption (#123) | Define saving instruction bodies, target, frames, and origins from the same Module; do not gate other work on executor completion |
| Next language feature | `array_repeat` (#122) | Confirm existing proposal; compare zero/invalid counts, budgets, nested copying, and lowering across existing routes |
| Practical priority | Standard I/O and effect comparisons (#89/#104), necessary library helpers | Interactive output/flushing, EOF/failures/budgets, identical inputs and operation sequences; start with a small interactive example |
| Independently prioritized | Native build (#81) | Completed artifact, explicit target/linker/runtime dependencies, failures, manifest; start with current programs |
| Alongside affected work | Observation sharing (#118), focused refactoring (#103), QBE maintenance (#92) | Preserve outputs/failures/origins; identify shared artifacts and tool revisions |
| Use-driven candidates | Explicit wrapping policies (#88), bindings/conditional syntax (#90/#105) | Uses are recorded in issues; preserve ordinary checked arithmetic and distinguish syntax from new semantics |
| Larger dependent work | IR/VM Images/release, recursion, growth/borrowing, string operations, module distribution | Design validation, ownership, resources, and compatibility individually; Native release need not await IR/VM completion |
| Deferred/research | Optimization, general proofs, AST execution, LIR executor implementation, GPU/FPGA | Decide scope/adoption separately; none is a prerequisite for language features |

LIR observation directly serves Cerune's observability goal and is restored explicitly to the roadmap. Evidence does not yet justify making a large LIR executor implementation the next mandatory task. #122 remains the next language feature and can proceed independently of LIR design review.

## Main language-level gaps

Route agreement does not mean all language capabilities are present. The [current reference](../reference/language.en.md) and individual designs identify these gaps by use. This is not a decision to adopt all of them immediately.

| Area | Main missing capabilities |
| --- | --- |
| I/O and libraries | stdin, CLI arguments, general stderr writes, files/external interaction, standard-module layout/lookup/compatibility |
| Data operations | array_repeat, dynamic-array growth/borrowing, string indexing/character counts/numeric parsing, zero-length fixed arrays/empty literals |
| Functions and types | Recursion, function values, generic type definitions/traits/type-argument inference, recursive value types |
| Failure handling | Generic Option/Result, implicit error propagation, catching runtime traps; current enums can represent success/failure values |
| Longer source programs | Optional let, omitted infer, else if, conditional expressions, formatter; not all require new semantics |
| Distribution/integration | Re-exports/package management, IR/VM Images, build/release, show/via, optional backends |

AST/LIR execution or optimization alone cannot supply these capabilities. Select work that broadens use separately from work observing and comparing transformations.

## Mapping every issue

At the audit baseline, 14 issues were open and #2 was the only closed issue, excluding pull requests.

| Category | Issues | Position |
| --- | --- | --- |
| Observation/transformation | [#60](https://github.com/Cerune-Lang/Cerune/issues/60), [#94](https://github.com/Cerune-Lang/Cerune/issues/94) | Partly complete; record SSA bundles/finite proofs and LIR gaps; defer optimization |
| Completed artifacts | [#81](https://github.com/Cerune-Lang/Cerune/issues/81) | build/release, show/via, Images, and optional backends remain distinct unimplemented work |
| Input/comparison | [#89](https://github.com/Cerune-Lang/Cerune/issues/89), [#104](https://github.com/Cerune-Lang/Cerune/issues/104) | stdin first; design ordinary and comparison connections together |
| Language features | [#122](https://github.com/Cerune-Lang/Cerune/issues/122), [#88](https://github.com/Cerune-Lang/Cerune/issues/88) | array_repeat next; wrapping policies are separate use-driven candidates |
| Syntax | [#90](https://github.com/Cerune-Lang/Cerune/issues/90), [#105](https://github.com/Cerune-Lang/Cerune/issues/105) | Distinguish let, omitted infer, else if, and conditional expressions; formatter also unimplemented |
| Maintenance | [#92](https://github.com/Cerune-Lang/Cerune/issues/92), [#103](https://github.com/Cerune-Lang/Cerune/issues/103), [#118](https://github.com/Cerune-Lang/Cerune/issues/118) | QBE, focused separation, local observations versus published fixtures |
| Distribution policy | [#119](https://github.com/Cerune-Lang/Cerune/issues/119) | License investigation; no decision to change current MIT |
| Experiment | [#93](https://github.com/Cerune-Lang/Cerune/issues/93) | FPGA/Cisa ISA concepts, not implemented routes |
| Completed historical plan | [#2](https://github.com/Cerune-Lang/Cerune/issues/2) | Initial function/control/aggregate IR plan, not a current missing-feature inventory |

This audit adds #123. QBE patch information was checked as recorded issue comments and retained research-branch material. No latest-QBE upgrade, workaround removal, or research-branch merge was performed.

## Sources checked and corrections

The inventory covered all 83 tracked docs documents (Japanese/English), README/SECURITY, 12 example guides, four experiment guides, four scripts, and relevant CLI/IR APIs, Native lowering/emitter/encoder, bundles, tests, and CI at the audit baseline. Implementation-relevant statements were cross-checked against code; this is not a new review of every source line or a rerun of every test.

| Source | Finding |
| --- | --- |
| Architecture | Correct stale “SSA planned/unimplemented” entry and the diagram omitting SSA |
| Stage/Native/target/CLI/bundle docs | Internal LIR already described; clarify absent independent observation/execution |
| Native encoder | Update old diagram omitting HIR→MIR |
| Roadmap | Connect to this inventory including LIR gaps and separate wrapping/syntax candidates |
| Docs/experiment indexes | Add status, constant folding, source files, and SSA experiment entries; separate Japanese/English prose |
| Lean/SSA experiments | Distinguish finite checks from a public backend; do not extend proof claims to LIR/machine code |
| GitHub | Check all issue bodies/comments, open PR #121, and merged PR history; release list empty. Wiki remote unavailable, so Wiki contents were not checked |
| local-notes | Unagreed private drafts include old names/status; not current specification evidence and not copied into public documents |

A real `observe --ssa` bundle from `native_calls.ceru` with an explicit Linux target contained ten files and MIR→LIR ASM annotations, but no LIR snapshot. This checks generation/observation, not a new Linux execution test.

Maintain current priorities in the [roadmap](language-roadmap.en.md), implemented commands in the [CLI reference](../reference/cli.en.md), and adoption/completion criteria in issues. This document records evidence and decisions at the audit baseline.
