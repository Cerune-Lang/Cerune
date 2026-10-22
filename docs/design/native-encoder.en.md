# Cerune's x86-64 encoder

[日本語](native-encoder.ja.md)

## Scope

`emit-obj` uses pinned SASHIMI in the same process to encode x86-64 instructions and write Linux ELF64 or Windows COFF objects. It does not require an external assembler or launch a JIT or executable. Linking remains the responsibility of an explicitly selected external linker.

```text
HIR → MIR (optionally through explicit SSA) → x86-64 LIR → internal ASM
                                                          ├→ emit-asm → external assembler
                                                          └→ SASHIMI: restricted ASM reader → encoder → ELF/COFF
```

Language semantics and instruction selection are shared with the existing path. This first boundary is a small assembler for internally generated ASM, rather than a public general-purpose assembler; there is no arbitrary-ASM input command. It does not repeat type inference, failure checks, evaluation order, or value-copy rules. A future typed machine-instruction IR can replace this internal text boundary.

The encoder covers the forms needed by all current language features: eight integer kinds, f32/f64, booleans, strings, arrays, products, functions, and control flow. General x86-64 assembly, AVX, short-branch optimization, and an internal linker are outside its scope. Newly generated unsupported instruction forms produce diagnostics; there is no automatic external-assembler fallback.

## SASHIMI connection (2026-10-23)

Object generation now calls the [SASHIMI](https://github.com/Cerune-Lang/SASHIMI/tree/7068cade33213d9d188f0f6b767aaca6ffdba293) Rust library. Cargo.toml pins the full commit, and Cargo.lock records it. A neighboring SASHIMI clone is for development/review; building Cerune has no local path dependency. The initial build needs dependency sources, but generated executables require no separate SASHIMI process or dynamic library.

| Owner | Responsibility |
| --- | --- |
| Cerune | MIR→LIR, instruction selection, ABI, startup/runtime, origin labels, CLI/observation bundles, and linked language-behavior tests |
| SASHIMI | Restricted ASM parsing, instruction encoding, label resolution, relocations, ELF64/COFF writing |
| Adapter | Explicit Target and diagnostic conversion; preserve diagnostic messages and object bytes |

[SASHIMI PR #5](https://github.com/Cerune-Lang/SASHIMI/pull/5) implements ASM, Object and OriginId provenance mappings from one typed Program. Cerune pins that PR's commit and verifies compatibility with the existing route. Production still passes assembly text and retains Cerune-created origin labels as ordinary symbols. [Typed construction for UTF-8 and string equality](native-typed-runtime.en.md) is implemented. Whole-program instruction selection and other runtime helpers remain to migrate; final object generation still reparses assembly. A stable public ABI remains unimplemented. The legacy text API still rejects unresolved cerune_ labels; typed Programs use explicit internal/external declarations instead of name prefixes.

The [old encoder](../../tests/support/native_object_reference/README.en.md) is a frozen test-only migration reference, excluded from production builds and not a place to add features. [Connection tests](../../tests/support/sashimi_connection.rs) compare complete objects and diagnostics against it, covering 135 runnable examples, both targets, baseline MIR/SSA and origins on/off. Equivalence demonstrates preservation during the switch, not absence of shared defects. Retain external-assembler comparisons, independent expectations and linked execution tests.

SASHIMI maps OriginId to multiple ranges, zero-byte operations and synthetic instructions. Positions are object section offsets, distinct from linked addresses. Next connect Cerune's typed construction and NodeId/SourceId/Span mappings so ASM/Object share one Program. [Porting SASHIMI to Cerune](https://github.com/Cerune-Lang/SASHIMI/issues/2) is a separate stage.

See the [SASHIMI Program API](https://github.com/Cerune-Lang/SASHIMI/blob/7068cade33213d9d188f0f6b767aaca6ffdba293/docs/design/typed-program.en.md) for contracts and examples. The new API uses ELF PLT32 for external call/jmp/jcc, renders @PLT in assembly, and compares ASM/Object execution on Windows/Linux. The legacy text API retains PC32 external jumps for compatibility. Current Cerune output and failure-injection objects use ABI-compliant calls.

## Encoding and relocations

- Instructions contain the required legacy prefixes, REX, ModR/M, SIB, displacements, and immediates. Register classes and widths, address scales, and immediate ranges are checked.
- Local branches use fixed 32-bit relative displacements. References within `.text` are resolved; displacements outside their representable range are rejected.
- Constants and strings occupy a read-only section. ELF uses PC32 for RIP-relative data and PLT32 for external calls; COFF uses REL32. Addends account for immediates following a displacement.
- Dynamic string management state occupies non-executable writable `.data`; static strings remain read-only.
- ELF includes an empty `.note.GNU-stack` and does not request executable stack memory. COFF timestamps are fixed at zero. Symbol and section ordering is deterministic.
- COFF supports up to 65,535 relocations. Positions, values, and displacements are not silently truncated into narrower fields.

Cerune may use longer immediate/branch forms and single-byte NOP alignment. Instruction bytes and addresses can differ from external assemblers. Comparisons establish language behavior and execution results. Identical input, target, and options produce identical Cerune objects.

`--annotate-origins` retains source-origin labels as symbols. It does not change `.text`, constant/string data, or relocation meaning. The symbol table itself changes. Generated objects are observations, not interfaces for external mutation of compiler state.

## Commands

Both `--target` and `-o` are required so binary objects are never implicitly printed to a terminal. Output is a relocatable object, not an executable.

```powershell
cargo build
target/debug/cerune.exe emit-obj examples/packet_counter.ceru --target x86_64-pc-windows-msvc --annotate-origins -o target/packet.obj
clang target/packet.obj -o target/packet.exe
target/packet.exe
```

```sh
cargo build
target/debug/cerune emit-obj examples/packet_counter.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/packet.o
cc target/packet.o -o target/packet
target/packet
```

With `CARGO_TARGET_DIR=target/unix` in WSL, use `target/unix/debug/cerune`. Encoding can target either OS from either host. Execution and linked libraries must match the chosen target.

Select `--encoder cerune` in the observation script; the existing `external` default remains available for comparison.

```powershell
node scripts/observe-native.cjs --source examples/packet_counter.ceru --target x86_64-pc-windows-msvc --cerune target/debug/cerune.exe --cc clang --objdump llvm-objdump --output-dir target/packet-own-windows --encoder cerune --run
```

`manifest.json` records `encoder: cerune` and an `encode-object` stage. The C driver performs linking only. IR, corresponding ASM, the Cerune object, disassembly, and VM output comparison are saved through the [native observation workflow](native-code.en.md).

## Validation

All examples, u64/string boundaries, and large aggregate arguments and returns are linked and executed on Windows/Linux against the VM and external-assembler path. Expected illegal-instruction traps are checked for both encoders. Lower-level tests cover REX/SIB, displacement boundaries, SSE2, local branches, and RIP-relative addends. Instruction-byte expectations were cross-checked with the independent LLVM assembler. CLI tests clear PATH to verify object generation requires no external tools.

References: [Intel instruction manuals](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html), [Microsoft PE/COFF specification](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format), [ELF64 format](https://uclibc.org/docs/elf-64-gen.pdf), and [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI).
