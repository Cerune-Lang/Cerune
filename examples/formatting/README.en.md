# Optional let and formatting

[日本語](README.md)

[bindings.ceru](bindings.ceru) uses `let` for ordinary declarations, for initializers and array iteration. Types, `mut`, evaluation order, short circuiting and array-copy semantics are unchanged. The colon remains required. Initialized bindings may omit `infer`, as shown below.

Run from the repository root.

```sh
cargo run --quiet -- run examples/formatting/bindings.ceru
cargo run --quiet -- fmt examples/formatting/bindings.ceru --let hide -o target/bindings.ceru
cargo run --quiet -- run target/bindings.ceru
cargo run --quiet -- fmt target/bindings.ceru --let hide --check
```

Both executions produce the same output.

```text
6
2
5
[9, 4]
[2, 4]
false
```

| Selection | Behavior |
| --- | --- |
| `--let preserve` (default) | Keep original prefix choices |
| `--let show` / `hide` | Add/remove declaration prefixes; keep identifiers named let |
| No destination | Print formatted text to stdout |
| `--check` | Exit 1 when changes are needed; do not modify files |
| `--write` / `-o <file>` | Replace input/save to another file; mutually exclusive |

Comments and literals are retained. Formatting does not load imports or execute code. Compiling the result gives locations corresponding to the formatted source. See the [contract and limits](../../docs/design/formatting.en.md).

## Omitting infer

[infer.ceru](infer.ceru) uses inference through `value: = expression;`. It covers ordinary bindings, for initializers, u8 array copying, u64, floating point, strings and short circuiting.

```sh
cargo run --quiet -- run examples/formatting/infer.ceru
cargo run --quiet -- fmt examples/formatting/infer.ceru --let show -o target/infer.ceru
cargo run --quiet -- run target/infer.ceru
cargo run --quiet -- emit-ir target/infer.ceru
```

Both executions produce the following output. IR displays the same resolved types as explicit inference.

```text
6
[9, 4]
[2, 4]
18446744073709551615
1.5
日本語
false
```

Explicit `infer` uses the same inference. Formatting retains its presence/absence; `--let` changes only let prefixes. Array iteration remains `value: infer in values`; omission does not extend to parameters, results, constants or fields. See the [design](../../docs/design/infer-omission.en.md) for scope and origins.
