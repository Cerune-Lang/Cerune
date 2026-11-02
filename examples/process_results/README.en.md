# Using subprocess result types

[日本語](README.md)

[outcome.ceru](outcome.ceru) uses `std:process` types to process **explicitly constructed result values**. It launches no subprocess. `process::run` is not implemented yet.

| Check | Representation |
| --- | --- |
| Operation completion versus nonzero exit | `Failure::None` with the largest 32-bit `Exit::Code` |
| Partial output and continuation after failure | `Timeout`, `Transfer { Stdout }`, and final `continued` |
| Content preservation | `[255, 0, 13, 10]`: invalid UTF-8, NUL, and CR/LF retained as bytes |
| Cleanup and progress | Incomplete cleanup, absent raw report, and `progress_known` |
| Independent copies | Changing a copy to 65 leaves the original first byte at 255 |

Run from the repository root.

```sh
cargo run --quiet -- run examples/process_results/outcome.ceru
cargo run --quiet -- run-vm examples/process_results/outcome.ceru
cargo run --quiet -- emit-sources examples/process_results/outcome.ceru
cargo run --quiet -- emit-ir examples/process_results/outcome.ceru
```

The [expected output](outcome.stdout) is compared across four executors and generated C/LLVM. `emit-sources` exposes declarations/origins; `emit-ir` and later forms expose enum, array, and copy lowering. C/LLVM execution comparisons require a compatible Clang.

```sh
cargo test --test modules process_types
```

See the [public-type design](../../docs/design/subprocess-values.en.md) for request fields, result meanings, and remaining execution integration.
