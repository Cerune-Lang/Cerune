# JS/TS API for WAT libraries

[日本語](wat-js-bindings.ja.md)

Generate checked JS and TS declarations from selected [WAT library](wat-library.en.md) exports. The initial argument/result types are i32/f64. General host calls, aggregates, a dedicated CLI and IR/VM exposure to JS are not added here.

## Generation and use

Save Rust `Library::javascript()` as `library.mjs` and `Library::typescript()` as `library.d.mts`. Original IR, ordinary WAT, library WAT and export metadata remain separately available. Generation performs no file I/O or implicit optimization.

```js
import {createLibrary, metadata, CeruneHostError} from './library.mjs';
const response = await fetch('./library.wasm');
if (!response.ok) throw new Error('Wasm load failed');
const library = await createLibrary(new Uint8Array(await response.arrayBuffer()));
console.log(library.functions.step(10, 2, 0.5)); // 11
console.log(metadata); // Read-only; does not execute the program.
```

The caller loads Wasm explicitly. `createLibrary` accepts Uint8Array and compiles/instantiates on each creation. Repeated function calls on the same handle do not recompile. WAT stays emit-only; assembly remains an external tool's responsibility.

## Call contract

| Subject | Check/behavior |
| --- | --- |
| Argument count | Exact declaration count; reject extra/missing arguments |
| i32 | Integer Number in −2147483648…2147483647; reject −0 |
| f64 | Number, including NaN, Infinity and −0 |
| Implicit coercion | Reject strings, BigInt, boxed Numbers and convertible objects; never call conversion methods |
| Results | Check the same type/range. Valid NaN is not a failure sentinel |
| TS declarations | Generate selected names, arities and number types. JS runtime checks enforce integer ranges |
| Execution | Synchronous; no termination or time-limit guarantee |
| Public handle | Frozen `functions` only; no instance/module/memory or arbitrary host callbacks |

The function catalog has no prototype and supports names such as `__proto__` and `constructor`. Wrapping it in `functions` avoids treating an export named `then` as a Promise callback.

## Failures and reuse

API failures use `CeruneHostError`. Stops after execution begins close the handle; subsequent calls fail with `closed`. A new handle can be created independently.

| phase | Meaning | Reuse same handle |
| --- | --- | --- |
| compile | Invalid input representation or Wasm compilation failure | Not created |
| link | Export names/kinds/count or imports violate the contract | Not created |
| instantiate | Instantiation failure or unexpected startup diagnostic | Not created |
| arguments | Invalid count/type/range; Wasm is not invoked | Yes |
| language | Wasm stop with a runtime-v1 diagnostic | No |
| trap | Wasm trap without a diagnostic | No |
| host | Host exception, invalid diagnostic or invalid result | No |
| closed | Call on a stopped handle | No |

Errors retain `functionName` for calls and `cause` for underlying exceptions. Language errors include the runtime-v1 `record` and `failure` with code, NodeId, SourceId and byte range. Anonymous SourceId is 0. No source text is included; use sources.json from the same generation to resolve origins. Diagnostics are collected privately with a 4096-byte ASCII limit; invalid values produce a host failure. There is no mutable observation callback.

## Observation and limits

`metadata` contains the same public declarations, conversions and origins as exports.json and is deeply frozen. Reading it neither executes nor changes the program. Ordinary IR/WAT and unused functions remain; generated JS and declarations are inspectable too.

**Pair Wasm and JS from the same generation.** WebAssembly reflection checks export/import names, kinds and counts. It does not prove function-signature or body equivalence for arbitrary Wasm, nor sandbox modified artifacts. Calling raw Wasm directly bypasses JS checks and stopped-handle management.

## Validation

The [browser example](../../experiments/host_calls/browser/README.en.md) exercises repeated calls, catalog reads, invalid arguments, dependency failures and recreation. Node tests compare results/failure records with VM and check all phases, i32 ranges, f64 special values, unusual export names and private memory. Generated TS is checked in strict mode; CI operates both Japanese and English HTML in Chromium and saves screenshots.
