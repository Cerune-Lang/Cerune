# Branching on input values

[日本語](README.md)

[input_flow.ceru](input_flow.ceru) selects operations using values scripted in the source. **It does not read stdin.** Cerune owns the state and operation rules.

| Value or operation | Behavior checked |
| --- | --- |
| `Line` | attack reduces HP by 3; heal adds 2 |
| LF, CRLF, no line ending | The program explicitly distinguishes strings retaining their line endings |
| Empty line | Distinct from EOF; does not change HP |
| `End`, `Failed` | Stop processing without executing later operations |
| Japanese, NUL, CR/LF | Preserve byte counts |
| Composed and combining characters | No Unicode normalization |

Expected output is [input_flow.stdout](input_flow.stdout). `Input` and failure number 17 are local to this example, not standard-library API or an OS error code.

```sh
cargo run --quiet -- run examples/io_protocol/input_flow.ceru
cargo run --quiet -- run-mir examples/io_protocol/input_flow.ceru
cargo run --quiet -- run-mir examples/io_protocol/input_flow.ceru --ssa
cargo run --quiet -- run-vm examples/io_protocol/input_flow.ceru
cargo test --test io_protocol
```

The [standard I/O design](../../docs/design/standard-io.en.md) covers actual input, flushing, failure, and observation. This example is a baseline for branching after receiving input values; it does not test OS reads or interactive output order.
