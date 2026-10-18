# Functions for host calls

[日本語](README.md)

These are ordinary Cerune examples. JS/TS and Rust comparison code lives in a [separate experiment](../../experiments/host_calls/README.en.md).

| File | Purpose |
| --- | --- |
| [math.ceru](math.ceru) | Position updates, exact numeric conversion, i32/u64/f32 functions |
| [demo.ceru](demo.ceru) | Imports math and calls the same functions from Cerune |

Run from the repository root.

```sh
cargo run --quiet -- run examples/host_calls/demo.ceru
cargo run --quiet -- run-vm examples/host_calls/demo.ceru
cargo run --quiet -- emit-ir examples/host_calls/demo.ceru
```

Both execution routes produce the following output.

```text
11
0
42
42
18446744073709551615
1.25
```

`checked(1.5)` stops instead of silently dropping the fraction. The experiment also compares that failure record between VM and WAT. `pub` controls visibility within Cerune; ordinary emit-wat does not automatically export these functions to JS.

## Library export example

[library.ceru](library.ceru) exposes functions calling a dependency and an integer summation. Running [library_demo.ceru](library_demo.ceru) with run/run-vm prints 11, 10 and 42.

```sh
cargo run --quiet -- run examples/host_calls/library_demo.ceru
cargo run --quiet -- run-vm examples/host_calls/library_demo.ceru
```

The [WAT library API](../../docs/design/wat-library.en.md) explicitly exports step, sum_to and checked. The Rust generator example lives in the [experiment directory](../../experiments/host_calls/README.en.md).

The [browser example and instructions](../../experiments/host_calls/browser/README.en.md) repeatedly call these functions through generated JS/TS and expose failure origins and generation artifacts.
