# Comparing VM embedding and WAT exports

[日本語](README.md)

This fixed-function experiment supports the [#161 design](../../docs/design/typed-host-calls.en.md). **The fixed-function comparison itself is not a production public API.** The final section also contains a usage example for the separately implemented production Rust API. Cerune programs live in [examples/host_calls](../../examples/host_calls/README.en.md); supporting code lives here.

## Reproduction

Use Rust 1.98.0 with wasm32-unknown-unknown, Node 20.20.2, wabt 1.0.39 and TypeScript 5.9.3. Run each command successfully in order from the repository root. Tools and generated artifacts stay under target.

```sh
rustup target add wasm32-unknown-unknown
npm install --prefix target/host-call-tools --no-audit --no-fund wabt@1.0.39 typescript@5.9.3
cargo build --manifest-path experiments/host_calls/vm/Cargo.toml --locked --target wasm32-unknown-unknown --release --target-dir target/host-calls-vm
cargo run --locked --example host_calls -- target/host-calls
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/host-calls/exports.wat -o target/host-calls/exports.wasm
node experiments/host_calls/compare.cjs target/host-calls target/host-calls-vm/wasm32-unknown-unknown/release/cerune_host_call_probe.wasm
node target/host-call-tools/node_modules/typescript/bin/tsc --strict --noEmit --target ES2022 --module NodeNext experiments/host_calls/types.test.mts
```

The comparison has run in Node on Windows and Linux/WSL. CI's “Typed host call comparison” builds both Wasm artifacts on Linux, runs them and uploads observations. Execution inside a browser has not yet been verified.

## Code and artifacts

| Location | Purpose |
| --- | --- |
| [main.rs](main.rs) | Saves original IR, bytecode, ordinary WAT and wrapped WAT. Runs the ordinary demo in IR/VM against known expected output |
| [vm](vm/Cargo.toml) | Compiles the same fixed source once and uses existing VM embedding in Wasm; no new interpreter |
| [host.mjs](host.mjs) | Argument checks, typed results and handle closure on execution failure. Does not return raw instance/memory |
| [host.d.mts](host.d.mts) / [types.test.mts](types.test.mts) | Handwritten experimental TS declarations and type/arity/route-specific function checks |
| [compare.cjs](compare.cjs) | Repeated calls to both routes in one Node process; results, invalid arguments, failure records, fresh instances and raw Wasm conversions |
| target/host-calls | source.ceir, source.cebc, plain.wat, exports.wat/wasm, failure.txt, demo.stdout, measurements.json |

Records size, Wasm compile, instantiate, VM source preparation and 10,000 calls after 1,000 warmup calls separately. It validates result sums without time-based pass/fail thresholds. Offline WAT generation/assembly is not included in call timing.

## Interpretation and limitations

`api.step(10, 2, 0.5)` on the result of `create(bytes, 'vm' | 'wat')` returns 11. Compiled code is retained without recompiling source for every call.

- Shared f64 calls use known expected results; `checked(1.5)` must produce the same runtime-v1 record and origins. Valid NaN is distinct from failure.
- WAT also exercises an i32 wrapper and u64 range checks. f32 rounding is tested only on a raw Wasm export, not supported by the common API.
- JS runtime checks and TS static checks are separate. Raw Wasm coercions do not automatically become the public contract.
- Call failures distinguish arguments/language/trap/host/closed. Compile/instantiate failures do not yet have a common error representation.
- Rust VM Wasm exports memory, hidden by the experiment adapter. Generated WAT does not export memory. This does not prove isolation from a malicious host.
- Fixed internal-name lookup, textual WAT additions and handwritten TS declarations are experimental only. General selection, retained pub declarations, purity validation, generated JS/TS and HTML examples remain future work.
- Closing the JS handle after a language stop does not change the existing Rust VM API's independent subsequent invocations.

The [design](../../docs/design/typed-host-calls.en.md) records measurements and the rationale for starting with WAT exports. JS/TS helpers are excluded from GitHub Languages statistics without hiding their diffs.

## Using the production WAT library API

[library.rs](library.rs) uses the [production Rust API](../../docs/design/wat-library.en.md), separately from the fixed-wrapper experiment above. This is not a dedicated Cerune CLI command.

```sh
cargo run --quiet --example wat_library -- examples/host_calls/library.ceru target/wat-library step sum_to checked
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/wat-library/library.wat -o target/wat-library/library.wasm
cargo test --test wat_output library_exports
```

library.wat and exports.json record selected names, conversions and origins. source.ceir, source.cebc, plain.wat and sources.json retain ordinary representations for comparison; sources.json explicitly includes source text. Validation finishes before any output files are written.

Artifacts expose a raw Wasm ABI. JS/TS argument checks and stopped-handle management are not generated yet. Tests pass correctly typed arguments repeatedly and discard failed instances.
