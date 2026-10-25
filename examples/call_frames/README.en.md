# Values and result destinations across calls

[日本語](README.md)

[main.ceru](main.ceru) passes a product containing a string and an array through nested calls. Updating a copy preserves the original, and the value returned early remains usable. This example does not use recursion.

```sh
cargo run --quiet -- run examples/call_frames/main.ceru
cargo run --quiet -- run-vm examples/call_frames/main.ceru
cargo run --quiet -- emit-wat examples/call_frames/main.ceru
```

The output matches [main.stdout](main.stdout).

```text
[2, 4]
Cerune
[2, 4]
Cerune!
[12, 4]
```

Inspect private-frame-bytes, cerune.frame.base, and cerune_frame_allocate / cerune_frame_release in emitted WAT to follow per-function sizes, allocation at calls, and release on return. This is internal layout, not an API for mutating memory from Cerune. See the [recursion design](../../docs/design/recursion.en.md).

cargo test --test wat_output frames::call_frames_example compares HIR, MIR, SSA, VM, and WAT output. Executing WAT requires Node and wabt.

[depth.ceru](depth.ceru) evaluates the left and right arguments before entering pair. Run it with `cargo run --quiet -- run examples/call_frames/depth.ceru`; output matches [depth.stdout](depth.stdout).

Depth limits are explicit Rust API settings. Four executors and C/LLVM/QBE/WAT/Native emission APIs support them. See the [artifact commands](../../experiments/call_depth/README.en.md) to inspect checks, history, and origins. Limit 2 completes the example; limit 1 stops immediately before outer enters the first mark(10). CLI configuration is pending. `cargo test --test ir_executor call_depth` compares prior output, failure origins, and call snapshots across four executors.

[pending_values.ceru](pending_values.ceru) retains the result of price(1) while calling subtotal, then adds the returned value. Run `cargo run --quiet -- run examples/call_frames/pending_values.ceru`; output matches [pending_values.stdout](pending_values.stdout). Each executor retains its own representation in explicit frames. run-mir, run-mir --ssa, and run-vm produce the same output. emit-ir/emit-mir/emit-bytecode representations and origins remain unchanged.

[return_paths.ceru](return_paths.ceru) uses floating-point results from both early and normal returns. Limit 2 matches the [expected output](return_paths.stdout); limit 1 stops before the first leaf call.
