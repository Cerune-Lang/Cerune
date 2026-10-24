# Optional let and formatting

[日本語](README.md)

[bindings.ceru](bindings.ceru) uses `let` for ordinary declarations, for initializers and array iteration. Types, `mut`, evaluation order, short circuiting and array-copy semantics are unchanged. The colon and type specification (including `infer`) remain required.

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
