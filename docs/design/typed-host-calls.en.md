# Typed function calls from JS/TS

[日本語](typed-host-calls.ja.md)

This is the comparison design for [#161](https://github.com/Cerune-Lang/Cerune/issues/161). **Generated WAT function exports are recommended for the first production implementation.** This change adds a fixed-function experiment; a general public API/CLI is not implemented. IR Executor remains the normal execution entry.

## Existing capabilities

The audit covers Cerune at `e0d7270` and the [Tint* Wasm wrapper](https://github.com/Cerune-Lang/Tint-St/blob/ddb676a6d24c6a628203b9cabb877462b5fc01a4/experiments/wasm/src/lib.rs). Tint was not modified.

| Route | Implemented | Missing for this purpose |
| --- | --- | --- |
| [VM embedding](vm-embedding.en.md) | `embedding::resolve_function` retains a function bound to bytecode; `invoke_function` calls it repeatedly. f64 arguments, f64/void returns, output and failure origins | JS/TS exposure, other host value types, a user-facing public declaration catalog |
| IR Executor | Direct completed-IR execution, input/output and diagnostics | Public named-function invocation with typed arguments |
| Tint* | Source checking, representation generation, IR/MIR/VM execution | `execute(source, path)` compiles each time and returns output text; no reusable typed JS function handle |
| WAT | Compiles Cerune functions to Wasm; exports main and keeps memory private | Export selection, external type adaptation, checked JS and TS declarations |

VM embedding does not need to be invented. Each existing invocation creates independent execution state without running top-level statements or main. Functions may perform I/O; this API does not validate purity. Another call after failure is supported.

## Comparing the same functions

[Ordinary Cerune examples](../../examples/host_calls/README.en.md) are separate from the [experiment instructions](../../experiments/host_calls/README.en.md). They cover `step(position, velocity, delta)`, a failing conversion, and i32/u64/f32 transfers.

| Item | VM compiled to Wasm | Generated WAT |
| --- | --- | --- |
| Preparation | Includes the Rust implementation and fixed source; compiles and resolves once | Generates WAT from the same source ahead of time, then assembles with external wabt |
| Repeated call | Uses existing VM embedding | Calls a generated function directly |
| Shared checks | f64 results, argument count/types, NaN, failure reason and NodeId/SourceId/Span, execution in a fresh instance | Same |
| Additional checks | Reuses the Rust f64 API contract | i32 width adaptation, u64 BigInt range, f32 rounding, private memory, required imports |
| Experimental limitation | Fixed source/functions. Rust Wasm exports memory, but the JS adapter does not expose it | Adds fixed wrappers to output WAT; not a production emitter API |

The following single measurements were taken on 2026-10-18, Node 20.20.2, x86-64. Linux/WSL used the same Wasm binaries built on Windows.

| Metric | VM Wasm | Wasm from generated WAT |
| --- | ---: | ---: |
| Binary bytes | 979,282 | 3,180 |
| 10,000 calls on Windows | 12.71 ms | 1.88 ms |
| 10,000 calls on Linux/WSL | 11.71 ms | 2.63 ms |
| Source preparation on Windows | 21.59 ms | Ahead of time, not measured |
| Source preparation on Linux/WSL | 22.87 ms | Ahead of time, not measured |

Measurements follow 1,000 warmup calls and include JS argument checks and result handling. Wasm compile/instantiate times are recorded separately. Source compilation and assembly for generated WAT are excluded from this table. These are measurements of a small fixture comparing an included compiler with ahead-of-time generation, not guarantees about general speed, browser startup, or Cisa/Stelle workloads. CI has no performance threshold.

## Proposed first production implementation

Start with WAT exports for ahead-of-time application logic used repeatedly: the destination need not include a compiler. VM embedding suits live source editing and provides an independent comparison here. Neither route needs a new implementation of language semantics.

| Item | Initial proposal |
| --- | --- |
| Export selection | Explicitly select named, nongeneric pub functions in the entry module. pub alone does not imply a Wasm export |
| Declaration catalog | Preserve public names, types, origins and their IR function mapping in the frontend. Do not infer visibility from internal FunctionId or `module_N_` names |
| Library initialization | Reject top-level executable statements and main. Do not implicitly run user code at instantiate time |
| Initial values/bodies | Public arguments/results are i32/f64. Bodies allow scalar computation, local mutation, branches, loops and calls. Diagnose libraries containing I/O, heap allocation or aggregate values |
| Validation scope | Conservatively check the whole library initially, including private helpers. Do not silently remove unused functions to make it qualify |
| Artifacts | WAT, a versioned public-name/type/origin catalog, checked JS and TS declarations. Exact CLI/API names remain to be settled during implementation |
| State | Synchronous sequential calls within an instance. No shared application state, reentrancy or concurrent calls initially |
| After failure | Close the JS handle after a language stop, trap or host exception during execution. Argument rejection does not close it because execution has not started |
| Capabilities | No implicit DOM/network/arbitrary host callbacks. Connect diagnostic output explicitly |

This restricts the **initial host API**, not ordinary Cerune programs or language support in other routes. Strings, arrays, products, enums and ownership need subsequent design. Scalar functions can still stop on conversions/overflow, and loops are not guaranteed to terminate.

Current module resolution checks pub, but completed IR functions do not retain that visibility information. The next implementation must retain declarations during resolution and preserve their mapping to the resulting IR.

### Value transfer

Internal WAT uses i64 for Cerune integers, including i32. Exposing those functions unchanged would require BigInt from JS. External wrappers map Cerune types to the Wasm ABI.

| Cerune type | JS value and checks | Wasm transfer |
| --- | --- | --- |
| f64 | Number only; NaN/Infinity/-0 allowed. No implicit string/object conversion | f64 |
| i32 | Integral Number within range. Reject -0 instead of silently losing its sign | Sign-extend external i32 to internal i64; wrap the result back to i32 |
| u64 (later candidate, experiment only) | BigInt in 0 through 2^64−1 | i64 bit pattern; explicitly restore unsigned return interpretation |
| f32 (later candidate, raw experiment only) | Undecided whether the API permits rounding or accepts only exactly representable Numbers | Tests f32 rounding separately from the f64 contract |

Raw calls in the [WebAssembly JS API](https://webassembly.github.io/spec/js-api/) convert arguments. The experiment confirms fractional truncation, integer wrapping and NaN for omitted arguments. TS declarations do not validate runtime values; JS must check argument count, types and ranges. The existing [floating-point contract](float-rounding.en.md) for per-operation rounding remains unchanged.

### Failure and observation

The production API should distinguish compilation, export resolution, instantiation, argument rejection, language stops, Wasm traps and host exceptions. Preserve existing `runtime-v1` records and NodeId/SourceId/Span for language stops. Do not invent a Cerune failure reason for an unexplained trap.

The experimental JS adapter distinguishes `arguments/language/trap/host/closed` during calls. Compile/instantiate failures still use standard JS exceptions. VM failure records are retrieved separately so valid NaN results are not confused with failure. Closing the experimental wrapper after failure follows WAT instance-discard policy; it does not change the existing Rust VM API's ability to invoke again.

The experiment saves original IR, bytecode, ordinary WAT, WAT with wrappers, and failure records. Production declaration selection, wrappers and conversions must remain observable without observation invoking functions or changing state. Completed IR and ordinary unoptimized output remain available. WAT stays emit-only: external assembly does not become a new build/release responsibility.

## Sequence and remaining work

1. **Completed here:** inventory, fixed-function A/B comparison, ordinary examples, TS type checks, bilingual design, CI comparisons of results/failures/artifacts.
2. **Next:** retain public declarations and origins from the frontend; implement explicit selection, library validation and WAT wrappers as a production API.
3. Generate checked JS and TS declarations; test invalid inputs, stops and reuse; add an HTML repeated-call example and verify it in a browser.
4. Use real requirements to assess additional scalar types, IR/VM exposure to JS, aggregate values and ownership.

Internal-name lookup, textual WAT additions and handwritten TS declarations are specific to this experiment, not a mechanism to generalize into a production API. General FFI, a JS backend, package management, a new sandbox and proof of the whole implementation are outside this change.
