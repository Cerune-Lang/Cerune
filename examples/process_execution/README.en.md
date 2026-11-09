# Receiving subprocess-operation failures

[日本語](README.md)

[unconfigured.ceru](unconfigured.ceru) calls `std:process/run`, receives missing configuration as a value, and reaches `continued`. Ordinary CLI registration is not implemented, so no OS child launches. `std:process` provides types; `std:process/run` provides execution. `report.failure` retains failure category/phase, and `stdout`/`stderr` retain raw bytes.

```sh
cargo run --quiet -- run examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru --ssa
cargo run --quiet -- run-vm examples/process_execution/unconfigured.ceru
cargo test --test modules process_types
```

[Expected output](unconfigured.stdout) is compared across four executors. See the [execution design](../../docs/design/subprocess-execution.en.md) for Rust OS connections, unsupported generated routes, origins, and allocation.
