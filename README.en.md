<div align="center">

# Cerune

[日本語](README.md) | English

Cerune is an experimental programming language with static typing, direct IR execution, VM execution, and code generation in multiple formats.

[![CI](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml/badge.svg)](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Cerune-Lang/Cerune?label=release)](https://github.com/Cerune-Lang/Cerune/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Prebuilt: Windows, Linux, macOS](https://img.shields.io/badge/prebuilt-Windows%20%7C%20Linux%20%7C%20macOS-555)](https://github.com/Cerune-Lang/Cerune/releases/latest)

[Examples by type and purpose](examples/README.en.md) · [Language](docs/reference/language.en.md) · [CLI](docs/reference/cli.en.md)  
[Design docs](docs/README.en.md) · [Roadmap](docs/design/language-roadmap.en.md) · [Migrating from Primer](docs/design/naming.en.md)

</div>

## Try It

Prebuilt Cerune v0.1.0 binaries for Windows, Linux, and macOS are available from [GitHub Releases](https://github.com/Cerune-Lang/Cerune/releases/tag/v0.1.0).

### Install the release

**Windows (PowerShell)**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/Cerune-Lang/Cerune/releases/download/v0.1.0/cerune-lang-installer.ps1 | iex"
```

**Linux / macOS (Shell)**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Cerune-Lang/Cerune/releases/download/v0.1.0/cerune-lang-installer.sh | sh
```

### Try the latest master branch

With Rust and Cargo, you can build and try the latest implementation from the `master` branch.

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

To install from source:

```sh
cargo install --path .
```

### Compare execution and semantics

```cerune
print("Hello, Cerune!");
a: f32 = 0.1 + 0.2;
b: f64 = 0.1 + 0.2;
print(a);
print(b);

limit: u8 = 255;
unused: u8 = limit + 1;
print(99);
```

```sh
cerune run examples/runtime_failures/hello_cerune.ceru
cerune run-mir examples/runtime_failures/hello_cerune.ceru
cerune run-vm examples/runtime_failures/hello_cerune.ceru
cerune emit-ir examples/runtime_failures/hello_cerune.ceru
```

The program first prints `Hello, Cerune!` and the results for two floating-point types. Every execution path then stops with `integer-overflow` at the unused `unused` assignment, before printing `99`.

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
| Variables | Optional `let`, explicit types or inference with `infer` / `: =`; immutable by default, reassignable with `mut` |
| Operators | Arithmetic, remainder, comparisons, bit operations, logical negation, short-circuit evaluation; aggregate `==` / `!=` |
| Numeric conversion | `convert<T>(x)` / `T(x)` preserve values; float-to-integer conversions offer explicit rounding and saturation |
| Structs | Named types, field access and defaults, updates that create new values, nesting, value copies |
| Sum types | `enum` variants with payloads; exhaustive `match` statements/expressions with `if` guards |
| Arrays | Fixed length (including named constants), element counts with `array_len`, nesting, element access and updates, value copies |
| Dynamic arrays | `[T]`, `array_copy`, `array_copy_range`, `array_repeat`, independent copies, display, equality, iteration |
| Functions | Typed parameters/results, `void`, `return`, type/array-length parameters, direct/mutual recursion; aggregate values can be passed and returned |
| Entry point | Top-level statements or `fn main() -> void` (not both) |
| Control flow | `if` / `else`, `while`, `for`, array `for … in`, `break` / `continue` |
| Constants | Typed `const`, compile-time evaluation, shared with `pub const` |
| Modules | Explicit imports, namespaces, `pub` visibility |
| Standard operations | Line input in `std:io`, arguments in `std:env`, i64 parsing in `std:num`, reads/new-file creation in `std:fs` (in-memory Rust API connections) |
| Output and diagnostics | `print(expr);` including aggregates, error reasons, source locations, output produced before failure |

**Arithmetic rules:** No implicit numeric conversions. Integer overflow, invalid integer division, out-of-bounds access, failed exact conversions, and out-of-range rounded results stop execution. Explicit saturation clamps to the type's endpoints. Floating-point arithmetic rounds.

**Main limitations / not yet implemented:**

- **Language:** Public string indexing, character counting and numeric parsing beyond i64, dynamic-array growth and borrowing, catching runtime stops, and generic `Option`/`Result`.
- **I/O and modules:** OS file connections, generated file I/O, network I/O APIs, module re-exports, and package management.
- **Build and distribution:** IR/VM `build` and `release` (Native executable construction and distribution directories are supported).

## Execution and Output

| Command | Result / artifact | Use / target |
| --- | --- | --- |
| `build native` | Executable and generation/link records | Explicit Windows/Linux target and Clang path |
| `release native` | Distribution directory, requirements and SHA-256 | Package a successful build without regeneration |
| `fmt` | Source formatting | Keep comments; preserve/show/hide let; check or explicitly save |
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

Text goes to stdout; use `-o` to save it. LLVM / QBE require `--target`. WAT uses host functions for output and diagnostics.

## Examples and Development

| Task | Command |
| --- | --- |
| Run all examples (PowerShell) | `.\scripts\run-examples.ps1` |
| Run all examples (WSL / Bash) | `bash scripts/run-examples.sh` |
| Filter examples | PowerShell: `-Pattern "matrix*.ceru"`; Bash: `--pattern 'matrix*.ceru'` |
| Check expected example output | `cargo test --test examples` |
| Run fmt, Clippy, and all tests (WSL / Bash) | `bash scripts/test.sh` |

WSL needs its own Rust installation. Shell scripts default to `target/unix` for builds.

## Related Repositories

[Organization](https://github.com/Cerune-Lang) · [Project responsibilities](docs/design/ecosystem.en.md)

| Purpose | Repository | Role |
| --- | --- | --- |
| Native generation | [Sashimi](https://github.com/Cerune-Lang/Sashimi) | Generate ASM/objects from typed instructions and map origins to byte ranges |
| Development tools | [Tint\*](https://github.com/Cerune-Lang/Tint-St) / [Cerune-vscode](https://github.com/Cerune-Lang/Cerune-vscode) | Inspect source and generated representations / VS Code syntax highlighting and snippets |
| Testing and research | [Parallax](https://github.com/Cerune-Lang/Parallax) / [Fracta](https://github.com/Cerune-Lang/Fracta) | Differential testing across execution routes / safety research |
| Library experiments | [Quantelia](https://github.com/Cerune-Lang/Quantelia) / [Nerune](https://github.com/Cerune-Lang/Nerune) | Numerical computing / machine learning |
| Programs written in Cerune | [Stelle](https://github.com/Cerune-Lang/stelle) / [Cisa](https://github.com/Cerune-Lang/Cisa) | A tiny CLI RPG / a small 32-bit virtual CPU; both early experiments |
| Documentation and navigation | [website](https://github.com/Cerune-Lang/website) / [.github](https://github.com/Cerune-Lang/.github) | Documentation site / Organization overview |
| Purpose not documented | [Symnexa](https://github.com/Cerune-Lang/Symnexa) | No published description of purpose or implementation yet |
| External measurement environment | [Whitebase](https://github.com/Hokutaka/Whitebase) | Measure and compare Rust, C++, Assembly, and Cerune execution |

## License

Cerune is dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

at your option.
