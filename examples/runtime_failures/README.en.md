# Expected runtime failures

[日本語](README.md)

These six examples must stop for the specified reason. They are excluded from the root directory's successful-example batch.

| Example | Output before failure | Reason and source location |
| --- | --- | --- |
| [hello_cerune.ceru](hello_cerune.ceru) | `Hello, Cerune!`, `0.300000012`, `0.30000000000000004` | `integer-overflow` at `limit + 1` after floating-point output |
| [constant_overflow.ceru](constant_overflow.ceru) | `7` | `integer-overflow`, `limit + 1`; unused results must still fail at runtime |
| [overflow.ceru](overflow.ceru) | `カウンタを更新` | `integer-overflow`, `counter + 1` |
| [array_update.ceru](array_update.ceru) | `4` | `array-index-out-of-bounds`, assignment's `[2]`; the right-hand side does not execute |
| [function_division.ceru](function_division.ceru) | `false`, `除算を開始` | `division-by-zero`, `value / divisor` inside the function; the first call is short-circuited |
| [call_sequence.ceru](call_sequence.ceru) | `除算する値`, `2`, `5`, `false`, `除算する値`, `0` | `division-by-zero`, `10 / value` inside the same function; a successful call is followed by a skipped call and a failing call |

```sh
cargo run -- run examples/runtime_failures/array_update.ceru --diagnostic-format runtime-v1
```

`run` / `run-ir` / `run-vm` exit with code 1 and emits one stderr line: `cerune: runtime-v1 code=... node=... bytes=.....`. The byte range addresses the original UTF-8 source, with an exclusive end. Use the NodeId and range to locate the operation in `emit-ir`. Changing CRLF/LF changes byte offsets.

`constant_overflow` establishes a baseline for the [constant-folding design](../../docs/design/ssa-constant-folding.en.md). `cargo test --test ssa_executor --test ssa_native` compares HIR, MIR, SSA, reconstructed MIR, VM, and available Native ASM/Object execution. The optimization pass is not implemented yet.

`cargo test --test runtime_routes` generates and executes the existing four examples and shared failure cases through C, LLVM, QBE, and WAT, comparing these records and prior output. See [diagnostic verification](../../docs/design/runtime-diagnostics.en.md#verification) for external-tool configuration. An unavailable, unconfigured route is skipped rather than counted as execution coverage. WAT hosts must implement `cerune.write_error_byte`.

Compare with the internal encoder on Windows, using a new output directory:

```powershell
node scripts/observe-native.cjs --source examples/runtime_failures/array_update.ceru --target x86_64-pc-windows-msvc --cerune target/debug/cerune.exe --cc clang --objdump llvm-objdump --output-dir target/observe-array-failure --encoder cerune --run --expect-trap
```

On Linux explicitly select `x86_64-unknown-linux-gnu`, `target/unix/debug/cerune`, `cc`, and `objdump`. Select `--encoder external` to use an external assembler. Verification requires matching reasons, NodeIds, byte ranges, prior output, and the intended native illegal-instruction termination. See the [runtime diagnostic contract](../../docs/design/runtime-diagnostics.en.md).
