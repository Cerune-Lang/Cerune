# Explicit function exports for WAT libraries

[日本語](wat-library.ja.md)

This is the initial Rust API based on the [comparison design](typed-host-calls.en.md) for [#161](https://github.com/Cerune-Lang/Cerune/issues/161). Export selection, validation and WAT generation are implemented. Generated checked JS/TS, a dedicated CLI and an HTML example remain future work.

## API and generation

```rust
use cerune_lang::{modules, codegen::wat};

let compilation = modules::from_source(
    "pub fn increment(x:i32)->i32{return x+1;}"
).unwrap();
let library = wat::emit_library(&compilation, &["increment"]).unwrap();
println!("{}", library.wat());
println!("{}", library.export_manifest());
```

Use `modules::load` for relative imports. `from_source` continues to perform no file I/O. Select exports using original function names in the entry module.

| Stage/API | Information retained or validated |
| --- | --- |
| Module resolution | Original/resolved names, declaration/name Spans, pub visibility, entry membership and generic status |
| `Compilation::function_declarations()` | Read-only declaration catalog. Nonmodular input retains anonymous SourceId=0 and private visibility |
| Common IR construction | Reuses existing name/type resolution without introducing host-specific language semantics |
| `wat::emit_library` | Validates declaration/AST correspondence, the whole library and selected signatures |
| `Library::program()` | Ordinary completed IR, including unused functions, for observation and comparisons |
| `Library::exports()` | Selected-order names, FunctionIds, resolved argument/result types and name Spans |
| `Library::wat()` | Ordinary function lowering plus wrappers with explicit type conversions |
| `Library::export_manifest()` | `cerune-wat-library-v1`: public names, types, internal types, wrappers, function mapping, SourceId and byte ranges |

Internal names, FunctionIds and SourceIds are compilation-local observations, not persistent external identifiers. The external name is the explicitly selected original name. Save `source_manifest()` from the same Compilation separately to map origins to source text; export metadata contains no source text.

Declaration metadata has no public setter. If callers mutate the existing public AST and break its correspondence with retained names, positions or generic status, compilation diagnoses it. Artifact IR and export metadata are read-only too.

## Initial accepted scope

| Item | Contract |
| --- | --- |
| Exports | Explicitly select at least one unique entry-module pub function. Reject private/missing names and direct dependency selection |
| Export types | i32/f64 arguments/results; zero arguments allowed. No void result or other public value types yet |
| i32 ABI | External Wasm i32, existing internal i64. Wrappers sign-extend arguments and wrap results back to i32 |
| f64 ABI | Wasm f64; NaN, Infinity and negative zero follow existing arithmetic |
| Bodies | bool, all integers, f32/f64, local mutation, conditions, loops, ordinary calls, numeric conversions and scalar constants |
| Whole library | Check unused functions and dependencies too. Initially reject type declarations, generics, aggregate values, I/O and ownership/allocation operations |
| Initialization | Reject top-level executable statements and entry main. Generate no start section or synthetic main |
| Observation | Obtain original IR, ordinary WAT and wrapped WAT separately. No unused-function removal or implicit optimization |

To expose a dependency operation, write an ordinary entry function that calls it, as in [library.ceru](../../examples/host_calls/library.ceru). pub alone never implies a Wasm export.

## Runtime responsibilities

**The artifact exposes a raw Wasm ABI. It does not yet generate JS argument count/type/range checks.** The checked JS layer proposed by the comparison design, including prevention of implicit Number-to-i32 coercion, comes next. Callers must currently provide inputs of the correct types and ranges.

Imports `cerune.write_error_byte` only when needed. Language stops emit existing runtime-v1 records with NodeId/SourceId/Span and trap. Memory stays private; no DOM, network or general host calls are added. Callers must discard stopped instances: raw exports do not enforce closed-handle checks. Loops are not guaranteed to terminate.

WAT remains emit-only. External tools assemble and execute Wasm; this does not expand build/release responsibilities.

## Running and testing

[Ordinary Cerune examples](../../examples/host_calls/README.en.md) remain separate from the [Rust API generator example](../../experiments/host_calls/README.en.md).

```sh
cargo test --lib codegen::wat::library
cargo test --test modules --test wat_output
cargo run --quiet -- run examples/host_calls/library_demo.ceru
```

The ordinary example prints 11, 10 and 42. Tests cover same-named dependency functions, visibility, unused I/O, unsupported types, stale declaration metadata and preservation of ordinary IR/WAT. With Node and wabt available, tests compare repeated i32/f64 calls with known results and compare f64 results/dependency failure records with VM. Windows/Linux/macOS WAT jobs require these tools in CI.
