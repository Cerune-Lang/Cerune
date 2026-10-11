# Native lowering from MIR

[日本語](native-mir.ja.md) · [IR stages](ir-stages.en.md)

## Implemented pipeline

Windows/Linux ASM and self-encoded COFF/ELF now use completed HIR→non-SSA MIR→x86-64 LIR→ASM. The old HIR→Native conversion is removed. Native does not reinterpret names, types, generics, match, or ownership rules.

| Stage | Responsibility |
| --- | --- |
| HIR→MIR | Left-to-right evaluation, short circuit, loop edges, ownership order, using the existing [common lowering](../../src/mir/lower.rs) |
| MIR→LIR | Validate MIR; determine local stack layout, argument/result ABI for the explicit target, and machine operations |
| LIR→ASM | Expand register operations, prologue/epilogue, checks/traps, and runtime helpers |
| ASM→Object | Encode the same ASM into COFF/ELF, relocations, and symbols. Linking remains separate |

Checks in LIR still expand into multiple machine instructions in the emitter. A LIR instruction index is not a final machine-instruction index. Existing target rules are retained; the host OS does not choose a target.

[Dedicated LIR display, bundle saving, and structural validation](lir-observation.en.md) are implemented. `emit-lir` / `observe --lir` expose instructions, frames, and provenance from the same Module. A direct LIR executor is deferred.

## Explicit SSA route

`emit-asm --ssa` / `emit-obj --ssa` use HIR→original MIR→SSA→MIR with copies→LIR. Defaults retain the non-optimized baseline. Pass the `program` returned by [SSA lowering](mir-ssa.en.md), `mir::ssa::lower(&ssa)`, to existing MIR-input APIs.

`observe --ssa` saves original MIR, SSA, reconstructed MIR, both mappings, and baseline/SSA-derived ASM separately. Follow added temporaries, edge copies, and parameter preparation through LIR/Object. No optimization or implicit ownership copies are introduced.

## Semantics and storage

All MIR blocks are laid out deterministically, preserving jump/branch destinations and unreachable blocks. The explicit main call is taken from MIR; Native does not add a call based on a function name.

| Operation | Preserved behavior |
| --- | --- |
| Locals and temporaries | Typed stack storage; fixed arrays/products copy their field representations |
| Dynamic-array/string reads | Store internal references without implicit deep copies or retain |
| CheckIndex→Store | Check after each index evaluation and save the element address; store there after the RHS, without moving checks past it |
| Allocate/initialize/retain/release/free | Preserve MIR order and separate logical string/array budgets |
| Numeric operations/output/failure | Reuse Native instructions/runtime, preserving width, sign, rounding, bytes, failure codes, and original origins |
| Calls/results | Reuse Windows/System V register and stack arguments; the aggregate result pointer in RAX remains Cerune-internal |

No slot reuse, register allocation, or implicit optimization is introduced. Explicit temporaries can increase stack use and instruction counts relative to the old conversion. Execution comparisons check semantics; this migration does not promise identical bytes or performance.

## API and observation

Existing HIR APIs and CLI commands use this pipeline internally. Rust APIs also accept the same MIR snapshot directly.

| API | Result |
| --- | --- |
| `x86_64::lower_mir(&mir, target)` | LIR with target, frames, instructions, source origins, and MIR correspondence |
| `x86_64::emit_asm_from_mir(&mir, target, annotate_origins)` | ASM from that MIR snapshot |
| `x86_64::emit_object_from_mir(&mir, target, annotate_origins)` | Self-encoded object from that MIR snapshot |

MIR `uses_strings` retains source string usage even when constant evaluation removes runtime string operations. `emit-mir` exposes it as `source-strings`, preserving Windows binary stdout initialization. Detection lives in [shared IR code](../../src/ir/string_usage.rs) and is reused by existing backends.

Generation does not mutate input snapshots. MIR validation errors return Diagnostic. The validator does not statically prove every heap alias/lifetime; these APIs are not a safe loader for arbitrary external MIR.

`--annotate-origins` retains source origins and adds correspondence:

- `# cerune-mir: v1 fn_0 bb2 i17 -> lir 45 (source)`: function/block/MIR instruction to the function's LIR instruction index. These numbers are illustrative.
- `cerune_origin_mir_fn_0_bb2_i17_lir45`: the same correspondence retained as an object symbol.
- Block entries use `block` instead of `iN`. Comments retain Derived/Synthetic reasons.
- ABI entry/exit use `synthetic ABI setup/exit`, distinguishing helper operations with no MIR instruction.

A MIR instruction can produce several LIR instructions. Zero-width operations expressible through the MIR API retain an `ObserveOnly` marker that emits no bytes. This does not add zero-length fixed arrays to the source language. Annotations do not change machine-instruction bytes and add no source text, paths, or timestamps.

## Example and verification

[native_calls.ceru](../../examples/ir_stages/native_calls.ceru) exercises seven mixed arguments, continue/break, and a single main call. See the [example README](../../examples/ir_stages/README.en.md) for commands and output.

Before/after Native tests on Windows/Linux cover existing examples, numeric boundaries, failure codes/NodeId/SourceId/Span/prior output, string bytes, and ABI behavior. [Dynamic-array tests](../../tests/dynamic_arrays.rs) compare HIR, MIR, VM, C, LLVM, QBE, WAT, ASM, and self-encoded objects. [Native MIR tests](../../tests/native_mir.rs) also check changed MIR execution, instruction provenance, and invalid-MIR rejection.

ASM fixtures are updated for the new layout. Known-output/failure execution comparisons and generated-text snapshots are separate checks. Formal proof, SSA, optimization, and observation bundles are outside this migration. Subsequent [observation bundles](observation-bundle.en.md) and SSA integration are implemented, saving baseline/SSA-derived representations separately from one compilation. [SSA Native tests](../../tests/ssa_native.rs) compare parallel transfers, arguments, ownership, and failure origins against ASM/self-encoded Object execution.
