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
