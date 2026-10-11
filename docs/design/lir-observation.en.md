# LIR observation and structural validation

[日本語](lir-observation.ja.md)

Expose and validate the existing x86-64 LIR used by Native generation. This adds neither another semantic representation nor another execution route. Tracked in [#123](https://github.com/Cerune-Lang/Cerune/issues/123).

## Implemented scope

Text observation and ASM generation consume the same Module. Baseline and SSA-derived artifacts remain separate.

| Operation | Input and result |
| --- | --- |
| `emit-lir <file> --target <triple> [-o <file>]` | HIR→MIR→LIR observation text |
| `emit-lir <file> --ssa --target <triple>` | HIR→original MIR→SSA→MIR with copies→LIR observation text |
| `observe <file> --lir --target <triple> -o <new-directory>` | Adds baseline LIR to the existing bundle, using schema v4 |
| `observe <file> --ssa --lir --target <triple> -o <new-directory>` | Adds both baseline and SSA-derived LIR, using schema v4 |
| `x86_64::lir::validate(&module)` | Validates an internal snapshot |
| `x86_64::lir::emit(&module)` | Validates and emits observation text |
| `emit_asm_from_lir` / `emit_object_from_lir` | Validates and generates ASM / self-encoded objects from that Module |

An explicit Windows/Linux x86-64 target is required; it is never inferred from the host. Existing heap-budget options apply. MIR→LIR lowering also validates its result. The existing low-level `x86_64::emit` API expects a validated Module.

The text header is `cerune-lir-observation-v1`. It records target, heap budgets, runtime flags, strings with UTF-8 byte lengths, floating-point constant bits, function IDs/names/frame sizes, instruction indices/operands, SourceId/NodeId/Span, and MIR blocks/instructions/derivation reasons. Instructions, operands, and provenance use Debug notation of internal Rust types. Output is deterministic for the same compiler and snapshot, without timestamps or environment data. This is not stable serialization, a loading format, or a distribution Image; internal type changes may change its presentation.

Instruction indices are local to each function and match the LIR indices in ASM `cerune-mir` annotations. One LIR instruction may expand into several ASM instructions. ABI preparation without a MIR counterpart has `mir: None`; operations without direct source provenance use `Synthetic`. Zero-width values retain `ObserveOnly` without emitting machine bytes.

## Checks and limits

Validation catches malformed internal snapshots early.

| Area | Checks |
| --- | --- |
| Provenance | Instruction and both provenance-array lengths; source origin agrees with MIR provenance |
| Functions/branches | Function ID/index agreement, callees, duplicate/referenced block labels, duplicate expansion label IDs |
| Constants | String indices and distinct f32/f64 constant IDs |
| Frames | 16-byte alignment, signed 32-bit displacement range, downward 8-byte slots |
| Stack operands | Scalar/aggregate/fixed-array extents, arithmetic overflow, overlap with outgoing arguments |
| ABI | Target-specific incoming register indices and stack alignment/shadow space/displacement |
| Runtime | Required string/array/write settings, write kinds, SysV output instruction target |

This does not prove initialization, pointer validity, ownership lifetimes, full callee type/arity agreement, existence of provenance IDs in the original MIR, or semantic correctness of ASM expansion, encoding, and the real ABI. Original MIR and source contents are not duplicated inside LIR; validating actual provenance references and UTF-8 spans requires other bundle stages. This is not a safe loader for untrusted executable input.

## Comparison example

[Seven arguments and an aggregate return](../../examples/ir_stages/lir_layout.ceru) prints `6`, `22`, and `LIR → ASM`. Inspect register/stack arguments, aggregate return storage, and source provenance with these commands.

```sh
cargo run --quiet -- run examples/ir_stages/lir_layout.ceru
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-pc-windows-msvc
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-unknown-linux-gnu
cargo run --quiet -- observe examples/ir_stages/lir_layout.ceru --ssa --lir --target x86_64-unknown-linux-gnu -o target/lir-observation
```

For seven integer arguments, Windows places the fifth onward on the stack; SysV places the seventh there. Cerune's internal convention passes an aggregate return pointer in RAX. This example does not declare a public C ABI.

Tests cover structural validity and determinism for every runnable example, both targets, baseline and SSA paths, plus ASM identity from the same LIR. They also check malformed snapshots, preservation of old bundle artifacts, and CLI equivalence. [Native MIR tests](../../tests/native_mir.rs) and [SSA Native tests](../../tests/ssa_native.rs) compare output, intentional failures, prior output, and provenance. Actual Windows/Linux execution belongs to tests on each OS.

## Decision on direct LIR execution

**Defer it for now. Observation and validation provide the stopping point before returning to language features.**

Direct execution would need independent models for integer/XMM registers, stack frames, pointers and memory, internal call ABI, and runtime helper operations. Delegating back to the MIR executor would not validate LIR. Current LIR includes pseudo-instructions for checks, arrays, and strings; the ASM emitter expands prologues and runtime details. A matching LIR Executor would not establish correctness of ASM expansion, self-encoding, object relocations, external linking, or the real OS ABI.

An independent executor could compare targets without hardware and localize MIR→LIR differences. Existing independent MIR/SSA evaluation and Native execution comparisons currently cover the concrete needs; no specific remaining gap justifies maintaining another machine/runtime model. Reconsider if an LIR-specific failure cannot be localized through current comparisons, or when designing a representation closer to machine instructions.

Typed machine IR, a general emulator, JIT, and a self-hosted linker are separate work. A LIR Executor or optimization completion is not a prerequisite for `array_repeat`, standard I/O, or build/release.
