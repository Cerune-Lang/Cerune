[![CI](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml/badge.svg)](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml)

# Cerune

[日本語](README.md) | English

Cerune is an experimental programming language with static typing, direct IR execution, VM execution, and code generation in multiple formats.

## Try It

Requires Rust (rustup and Cargo).

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo install --path .
cerune run examples/floating_point.ceru
```

The same addition produces different results depending on the type. Comments show the output.

```cerune
a: f32 = 0.1 + 0.2;
b: f64 = 0.1 + 0.2;
c: infer = 0.1 + 0.2;

print(a); // 0.300000012
print(b); // 0.30000000000000004
print(c); // 0.30000000000000004 (infer resolves to f64)
```

```sh
cerune emit-ir examples/floating_point.ceru
cerune emit-c examples/floating_point.ceru -o floating_point.c
```

During development, replace `cerune` with `cargo run --quiet --`.

## Capabilities

### Types

| Category | Types / operations |
| --- | --- |
| Signed integers | `i8`, `i16`, `i32`, `i64` |
| Unsigned integers | `u8`, `u16`, `u32`, `u64` |
| Floating point | `f32`, `f64` |
| Boolean | `bool` (`true` / `false`) |
| Strings | Immutable UTF-8 `string`; printing, equality, `byte_len`, concatenation with `concat` |

### Syntax and Operations

| Feature | Support |
| --- | --- |
| Variables | Explicit types or inference with `infer`; immutable by default, reassignable with `mut` |
| Operators | Arithmetic, remainder, comparisons, bit operations, logical negation, short-circuit evaluation; aggregate `==` / `!=` |
| Numeric conversion | `convert<T>(x)` / `T(x)` preserve values; float-to-integer conversions offer explicit rounding and saturation |
| Structs | Named types, field access and defaults, updates that create new values, nesting, value copies |
| Sum types | `enum` variants with payloads; exhaustive `match` statements/expressions with `if` guards |
| Arrays | Fixed length (including named constants), element counts with `array_len`, nesting, element access and updates, value copies |
| Dynamic arrays | `[T]`, `array_copy`, `array_copy_range`, `array_repeat`, independent copies, display, equality, iteration |
| Functions | Typed parameters/results, `void`, `return`, type/array-length parameters; aggregate values can be passed and returned |
| Entry point | Top-level statements or `fn main() -> void` (not both) |
| Control flow | `if` / `else`, `while`, `for`, array `for … in`, `break` / `continue` |
| Constants | Typed `const`, compile-time evaluation, shared with `pub const` |
| Modules | Explicit imports, namespaces, `pub` visibility |
| Output and diagnostics | `print(expr);` including aggregates, error reasons, source locations, output produced before failure |

**Arithmetic rules:** No implicit numeric conversions. Integer overflow, invalid integer division, out-of-bounds access, failed exact conversions, and out-of-range rounded results stop execution. Explicit saturation clamps to the type's endpoints. Floating-point arithmetic rounds.

**Not implemented:** Recursion, string indexing, catching runtime stops.

## Execution and Output

| Command | Result / artifact | Use / target |
| --- | --- | --- |
| `check` | Syntax and type checking | Validate Cerune source (`.ceru`) |
| `run` / `run-ir` | Direct IR execution | Execute Cerune source (`.ceru`) from common IR |
| `run-mir` | MIR execution | Compare lowered control flow; `--ssa` executes SSA directly |
| `run-vm` | VM execution | Run Cerune source (`.ceru`) |
| `emit-sources` | Source manifest (JSON) | Export loaded file names and contents |
| `emit-ir` | Cerune IR (`.ceir`) | Inspect types and operations |
| `observe` | Sources, HIR, MIR, annotated ASM, manifest | Save one compilation (explicit target required); `--ssa` includes the SSA path; `--lir` also saves LIR |
| `emit-mir` | MIR observation text | Inspect temporaries, control, ownership, and origins; `--ssa` shows original MIR and SSA |
| `emit-lir` | Native LIR observation text | Explicit target; instructions, frames, ABI, and origins; `--ssa` selects the SSA path |
| `emit-bytecode` | Bytecode text (`.cebc`) | Inspect instructions |
| `emit-c` | C (`.c`) | GCC, Clang, or another C compiler |
| `emit-llvm` | LLVM IR (`.ll`) | LLVM / Clang; Windows / Linux x86-64 |
| `emit-qbe` | QBE IR (`.ssa`) | QBE 1.3; Windows x64 / Linux x86-64 (explicit target) |
| `emit-wat` | WebAssembly Text (`.wat`) | WebAssembly tools and a host |
| `emit-asm` | Assembly (`.s`) | Windows / Linux x86-64; `--ssa` selects the SSA route |
| `emit-obj` | ELF / COFF object (`.o` / `.obj`) | External linker; requires `--target` and `-o`, supports `--ssa` |

Text goes to stdout; use `-o` to save it. LLVM / QBE strings and dynamic arrays, and LLVM checked numeric operations require `--target`. WAT uses host functions for output and diagnostics.

## Examples and Development

| Task | Command |
| --- | --- |
| Run all examples (PowerShell) | `.\scripts\run-examples.ps1` |
| Run all examples (WSL / Bash) | `bash scripts/run-examples.sh` |
| Filter examples | PowerShell: `-Pattern "matrix*.ceru"`; Bash: `--pattern 'matrix*.ceru'` |
| Check expected example output | `cargo test --test examples` |
| Run fmt, Clippy, and all tests (WSL / Bash) | `bash scripts/test.sh` |

WSL needs its own Rust installation. Shell scripts default to `target/unix` for builds.

## Documentation and Related Tools

- [Examples by type and purpose](examples/README.en.md) · [Language](docs/reference/language.en.md) · [CLI](docs/reference/cli.en.md)
- [Design docs](docs/README.en.md) · [Roadmap](docs/design/language-roadmap.en.md) · [Migrating from Primer](docs/design/naming.en.md)
- [Tint\*](https://github.com/Hokutaka/Tint-St.): inspect source and generated representations side by side.
- [Whitebase](https://github.com/Hokutaka/Whitebase): measure and compare Rust, C++, Assembly, and Cerune execution paths.
- [Stelle](https://github.com/Hokutaka/stelle): an early dogfooding experiment building a tiny CLI RPG entirely in Cerune.
- [Cisa](https://github.com/Hokutaka/Cisa): an early dogfooding experiment building a small 32-bit virtual CPU in Cerune.

[MIT License](LICENSE)
