# Calling Cerune functions from a browser

[日本語](README.md)

Generate the [.ceru examples](../../../examples/host_calls/README.en.md) through the [production WAT API](../../../docs/design/wat-js-bindings.en.md) and repeatedly call functions on the same instance. HTML and host helpers stay here, separately from the Cerune executable example catalog.

## Start

Use Rust 1.98.0 and Node 20.20.2. Run each command successfully from the repository root. Tools and generated artifacts stay under target.

```sh
npm install --prefix target/host-call-tools --no-audit --no-fund wabt@1.0.39 typescript@5.9.3 playwright@1.64.1
cargo run --locked --example wat_library -- examples/host_calls/library.ceru target/wat-library step sum_to checked
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/wat-library/library.wat -o target/wat-library/library.wasm
node experiments/host_calls/browser/serve.cjs target/wat-library
```

Open the printed URL. The server listens only on 127.0.0.1; stop it with Ctrl+C. Switch languages at the top of the page. Opening HTML directly through a file URL is not supported.

## Try it

| Action | Result |
| --- | --- |
| Advance one step | step(position, 2, 0.5) reuses the same function: 10→11→12 |
| Sum through limit | sum_to(4) returns 10. Reject non-i32 input such as fractions before execution |
| Pass a string | Reject checked("1.5") by type; the handle remains usable |
| Convert a fraction exactly | checked(1.5) stops and displays runtime-v1 with dependency source position |
| Call after stopping | closed; create a fresh instance to continue |
| Read export metadata | Read-only; no execution or state changes |

Calls are synchronous, so very large loop counts can make the browser wait. No execution time limit is added.

Generation links expose original IR, bytecode, ordinary/library WAT, JS, declarations and export metadata separately. sources.json is an explicitly generated observation file containing source text and is served by this server.

## Automated checks

```sh
node experiments/host_calls/browser/check-types.cjs target/wat-library target/host-call-tools/node_modules/typescript/bin/tsc
node target/host-call-tools/node_modules/playwright/cli.js install chromium
node experiments/host_calls/browser/test.cjs target/wat-library target/host-call-tools/node_modules/playwright
```

CI uses `install --with-deps chromium` to install required Linux libraries too. On Windows, append `msedge` to the browser test command to use existing Edge and skip downloading Chromium.

Type checks verify generated library.d.mts names, arities, value types and readonly declarations. Browser tests exercise both languages, repeated calls, observation, argument rejection, failure origins, closed handles and recreation, saving browser-ja.png and browser-en.png beside the artifacts. Local Edge has been verified; CI uses Chromium.
