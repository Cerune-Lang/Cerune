# Dynamic array examples

[日本語](README.md)

Run with IR/MIR/SSA Executors, VM, generated C, LLVM, QBE, WAT, Windows/Linux ASM, or self-encoded COFF/ELF.

| Example | Checks |
| --- | --- |
| [repeat.ceru](repeat.ceru) | Build a table from a runtime row count; functions, updates, saved copies, and iteration |
| [repeat_empty.ceru](repeat_empty.ceru) | Evaluate value then count even for zero; release the temporary and check empty length/iteration |
| [lowering_order.ceru](lowering_order.ceru) | Short-circuiting, continue/break, argument copies; succeeds at budget 48, stops before the call at 47 ([IR stages](../../docs/design/ir-stages.en.md)) |
| [coordinates.ceru](coordinates.ceru) | Dynamic arrays of fixed coordinate pairs; translation, independent copies, and element updates |
| [labels.ceru](labels.ceru) | Transform a string array in a function; compare the original, saved copy, and reassigned value |
| [batches.ceru](batches.ceru) | Return two ranges in a fixed array and compare updates with a saved copy |
| [readings.ceru](readings.ceru) | Return a range of records, edit a copy, and total its values |
| [window.ceru](window.ceru) | Choose a runtime-sized range; update the returned array independently |
| [copy.ceru](copy.ceru) | Independent copies, ranges and emptiness, updates, display/equality/iteration |
| [nested.ceru](nested.ceru) | Nesting, string NUL/CR/LF, functions/enums/match, reassignment during iteration |

```sh
cargo run --quiet -- run examples/dynamic_arrays/copy.ceru
cargo run --quiet -- run-vm examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-ir examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-bytecode examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-c examples/dynamic_arrays/window.ceru -o window.c
clang -std=c11 window.c -o window
cargo run --quiet -- run examples/dynamic_arrays/copy.ceru --array-heap-limit 24
```

The final command intentionally stops: the original three elements use 24 bytes, leaving no storage for `saved`. The default budget succeeds. Generated IR exposes copy loops, allocation, and release.

Build generated C with an external C compiler, then run `./window` on Linux or `.\window.exe` on Windows. Copy/release loops remain visible in both IR and generated C.

Generate and execute LLVM (Linux x86-64):

```sh
cargo run --quiet -- emit-llvm examples/dynamic_arrays/readings.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o readings.ll
clang readings.ll -o readings
./readings
```

On Windows, use target `x86_64-pc-windows-msvc`, then `clang readings.ll -o readings.exe` and `.\readings.exe`. The selected range stays `[{valid: false, value: 0}, {valid: true, value: 20}]`; the edited copy contains 15 and 20 and prints a total of 35.

Generate and execute with QBE 1.3 (Linux x86-64):

```sh
cargo run --quiet -- emit-qbe examples/dynamic_arrays/batches.ceru --target x86_64-unknown-linux-gnu -o batches.ssa
qbe -t amd64_sysv -o batches.s batches.ssa
clang batches.s -o batches
./batches
```

On Windows, select Cerune target `x86_64-pc-windows-msvc` and QBE `-t amd64_win`, then link with `clang --target=x86_64-pc-windows-msvc batches.s -o batches.exe`. Run `.\batches.exe`: it prints the updated `[[99, 20], []]`, saved `[[10, 20], [30]]`, outer length 2, and empty-array length 0.

Generate and run WAT with the development host (Node and WABT):

```sh
cargo run --quiet -- emit-wat examples/dynamic_arrays/labels.ceru -o labels.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm labels.wat -o labels.wasm
node tests/support/run_wasm.cjs labels.wasm
```

Install WABT with `npm install --prefix target/wasm-tools --no-audit --no-fund wabt@1.0.39`. Output is `["月", "火"]`, `["予定:月", "予定:火"]`, `["休み", "予定:火"]`, then `false`. Tests compare the same output through IR/VM/C/LLVM/QBE. Copies, releases, and checks remain visible in generated WAT; array memory is not exposed to the host.

Native generation and execution (Linux x86-64):

```sh
cargo run --quiet -- emit-asm examples/dynamic_arrays/coordinates.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o coordinates.s
clang coordinates.s -o coordinates-asm
./coordinates-asm
cargo run --quiet -- emit-obj examples/dynamic_arrays/coordinates.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o coordinates.o
clang coordinates.o -o coordinates-native
./coordinates-native
```

On Windows, select `x86_64-pc-windows-msvc`, link with `clang --target=x86_64-pc-windows-msvc coordinates.o -o coordinates.exe`, then run `.\coordinates.exe`. Link ASM with the same target. Output is the original `[[1, 2], [3, 4]]`, translated saved value `[[11, 1], [13, 3]]`, edited value `[[99, 1], [13, 3]]`, and `false`. All routes check the same result.
