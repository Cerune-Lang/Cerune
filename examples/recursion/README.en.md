# Recursion

[日本語](README.md)

| Example | Demonstrates | Required depth |
| --- | --- | --- |
| [factorial.ceru](factorial.ceru) | Multiply after returning from recursion | 5 |
| [mutual.ceru](mutual.ceru) | Mutual even/odd calls | 7 |
| [owned_generic.ceru](owned_generic.ceru) | Independent owned strings/dynamic arrays and recursive generic reuse | 5 |

```sh
cargo run --quiet -- run examples/recursion/factorial.ceru
cargo run --quiet -- run-mir examples/recursion/mutual.ceru --ssa
cargo run --quiet -- run-vm examples/recursion/owned_generic.ceru
cargo run --quiet -- run examples/recursion/factorial.ceru --call-depth-limit 4 --diagnostic-format runtime-v1
cargo run --quiet -- emit-ir examples/recursion/owned_generic.ceru
cargo run --quiet -- emit-c examples/recursion/factorial.ceru --call-depth-limit 5 -o target/factorial.c
```

The last run exits 1 with call-v1 depth failure, NodeId/Span, and the call stack. Factorial succeeds at limit 5. Expected output is saved in each `.stdout` file. Default 128/range 1–128 describes semantic call depth, not physical stack/memory safety.

emit-ir retains generic arguments, definition/recursive-call origins, ordinary calls/returns, and ownership operations. MIR, bytecode, and generated routes retain the same calls. Recursion is not implicitly rewritten to loops. See the [contract and validation](../../docs/design/recursion.en.md).
