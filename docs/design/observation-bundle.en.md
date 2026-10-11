# Observation bundle

[日本語](observation-bundle.ja.md)

## Purpose and scope

The first implementation of [#60](https://github.com/Hokutaka/Cerune/issues/60) saves HIR, non-SSA MIR, and annotated Native ASM from one frontend invocation. The manifest is an index, not a new IR.

Use `cerune observe <file> [--ssa] --target <triple> -o <new-directory>`. An explicit Windows/Linux x86-64 target is required. Existing heap-budget options apply.

| File | Contents |
| --- | --- |
| `manifest.json` | `cerune-observation-v1`, Cerune version, target, both heap budgets, optimization passes (currently empty), no-execution status, relative artifact paths |
| `sources.json` | File names, exact text, and SourceIds in the existing `emit-sources` format |
| `program.ceir` | Completed HIR |
| `program.mir.txt` | MIR lowered and validated once from the same HIR |
| `program.origins.s` | ASM from that MIR, annotated with HIR origins and MIR→LIR correspondence |

## Selecting SSA

`--ssa` constructs SSA from the same non-optimized MIR and adds five files. The original four artifacts remain byte-identical to the default bundle.

| Added file | Contents |
| --- | --- |
| `program.ssa.txt` | Same observation text as `emit-mir --ssa`, with separate complete original-MIR and SSA sections |
| `program.ssa-map.txt` | `Cerune SSA mapping v0.1`: functions, parameters, values/slots, blocks, instructions, terminators, edge transfers, and retained unreachable records |
| `program.ssa-lowered.mir.txt` | MIR reconstructed from SSA with dedicated temporaries and parallel copies |
| `program.ssa-lowered-map.txt` | SSA→MIR value/block/edge-copy/parameter/unreachable mappings |
| `program.ssa.origins.s` | Annotated Native ASM generated from reconstructed MIR |

Value IDs are scoped by function; instruction indexes by block. Record then/else separately even with the same target, identifying incoming values, destination arguments, and original locals. Original instruction IDs and origins refer back to the original MIR snapshot. The mapping is neither a semantic-preservation proof nor a loading format.

SSA manifests use `cerune-observation-v3`, extending v2's SSA observation with SSA→MIR/Native. The default retains existing v1 output.

| v3 additions | Contents |
| --- | --- |
| `transformations` | Zero-based order, `kind=representation`, versioned pass names, `options={}`. `scalar-ssa-v1`: `mir`→`ssa` / `ssa_mapping`; `ssa-lower-v1`: `ssa`→`ssa_lowered_mir` / `ssa_lowering_mapping` artifact keys |
| `artifact_inputs` | Each artifact's input: baseline `assembly` comes from `mir`; added `ssa_assembly` comes from `ssa_lowered_mir` |
| Additional `artifacts` keys | `ssa`, `ssa_mapping`, `ssa_lowered_mir`, `ssa_lowering_mapping`, and `ssa_assembly`, referencing the files above |

`optimization_passes` stays empty and `executed` stays false. Observe baseline Native and SSA-derived Native separately. Objects are not bundled; use `emit-obj --ssa` to emit them individually.

## One compilation

Read entry/imports once and run the common frontend once. Pass the resulting HIR and MIR snapshots to generation rather than collecting repeated CLI invocations. Do not reinterpret types, names, or ownership rules, or optimize implicitly.

The same version, input names/text, target, and heap budgets produce identical contents. Do not record timestamps, output-directory names, or environment variables. Bundles include input names and text. Different input path spellings need not produce identical bytes.

## Saving and failures

The parent directory must exist. The output must be a new directory; never overwrite existing files, directories, or symlinks. Generate all artifacts in memory before writing, then write the manifest last.

Report write failures as failures. A directory without a manifest is not a complete bundle. Ordinary write failures clean up only files created by this operation. Forced termination or cleanup failures can leave an incomplete directory; this is not a transaction or crash-durability guarantee.

## Execution and distribution

Do not run the program, launch external tools, or link. No repeated external effects or record/replay from [#104](https://github.com/Hokutaka/Cerune/issues/104) are introduced. This is not a completed `build`/`release` artifact and adds no IR/MIR loader.

LIR bodies are also absent. ASM contains MIR→LIR correspondence numbers, not a dedicated LIR snapshot. [Remaining LIR observation work](development-status.en.md) is separate from optimization.

Objects, disassembly, bytecode, C/LLVM/QBE/WAT, Lean, execution results, and optimization comparisons remain future extensions. Omission is neither generation failure nor implemented coverage. Individual `emit-*` commands remain available.

## Validation

Use the [IR-stage examples](../../examples/ir_stages/README.en.md) to compare each file byte-for-byte with individual emits using identical options. Check both targets, multiple files, string bytes, heap budgets, determinism, input errors, and output collisions. Observing a program that would trap must not execute it.
