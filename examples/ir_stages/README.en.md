# Observing IR stages

[日本語](README.md)

| Example | What to observe | Output |
| --- | --- | --- |
| [lir_layout.ceru](lir_layout.ceru) | Seven integer arguments and an aggregate return; inspect target-specific argument locations, frame, and copies in LIR | `6`, `22`, `LIR → ASM` |
| [native_calls.ceru](native_calls.ceru) | MIR→Native, seven mixed arguments, continue/break, main call | Header, arguments for i=0 and 2, result `4` |
| [owned_values.ceru](owned_values.ceru) | Independent dynamic-array arguments, immutable strings, cleanup through continue | Original array, decorated array, `["反復"]` |
| [ssa_values.ceru](ssa_values.ceru) | Branch-selected values, updates after continue, and swaps across iterations; SSA construction and execution comparisons | Four lines: `14`, `16`, `20`, `10` |
| [constant_candidates.ceru](constant_candidates.ceru) | Unoptimized baseline: constant arithmetic, short-circuiting, call order, loop updates | `42`, u64 maximum, `true`, `false`, `true`, `4`, `10`, `7` |
| [control_flow.ceru](control_flow.ceru) | Short circuit skips a call; for-continue enters the update and break enters the exit | Two lines: `2`, `4` |

Run the same source and inspect both representations with these commands.

```sh
cargo run --quiet -- run examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-vm examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru --ssa
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru -o target/ssa_values.mir.txt
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru --ssa -o target/ssa_values.ssa.txt
cargo run --quiet -- observe examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu -o target/ssa-observation
cargo run --quiet -- run examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/owned_values.ceru
cargo run --quiet -- run-vm examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/native_calls.ceru
cargo run --quiet -- emit-mir examples/ir_stages/native_calls.ceru -o target/native_calls.mir.txt
cargo run --quiet -- emit-asm examples/ir_stages/native_calls.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/native_calls.s
cargo run --quiet -- emit-obj examples/ir_stages/native_calls.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/native_calls.o
cargo run --quiet -- emit-ir examples/ir_stages/control_flow.ceru
cargo run --quiet -- emit-mir examples/ir_stages/control_flow.ceru -o target/control_flow.mir.txt
```

`native_calls` prints the header `MIR → Native`, then `0, 18446744073709551615, 7, 9, 4`, followed by `2, 18446744073709551615, 7, 9, 4`, one value per line. ASM `cerune-mir` annotations point back to MIR block/instruction IDs. For Windows output, select `x86_64-pc-windows-msvc` instead. Emitting an object does not link or execute it.

In `ssa_values`, `choose_and_count` adds either 2 or 4 to 10, then adds 0 and 2 in the loop. `swap_rounds` swaps 10 and 20 three times. This compares non-SSA routes, direct SSA execution, and SSA-derived Native execution. `emit-mir --ssa` prints original MIR alongside automatically constructed SSA. The [SSA design](../../docs/design/mir-ssa.en.md) uses it to explain merge values and parallel transfer.

In `control_flow`, at `i=0`, `accept` is skipped. At `i=1`, continue enters the update. Only `i=2` calls `accept` and prints `2`. At `i=3`, break skips the update. The final total is `4`.

In MIR, `bbN` identifies a block, `%N` a typed local/temporary, and `iN` an instruction within a function. Follow jump/branch and `derived=short-circuit-rhs`, `derived=for-update`, `derived=loop-exit` to inspect control flow. Each operation retains `hir=#N source=N bytes=A..B` to locate its input IR and source span. Emitting MIR does not execute the program.

For array copies and allocation failure, see [lowering_order.ceru](../dynamic_arrays/lowering_order.ceru). `run-mir` uses the [independent MIR executor](../../docs/design/mir-executor.en.md). [Native now consumes MIR](../../docs/design/native-mir.en.md). Use `--ssa` to select SSA construction, execution, and observation. SSA bundles separately retain original MIR, SSA, reconstructed MIR, and baseline/SSA-derived ASM. No optimization occurs. See the [stage design](../../docs/design/ir-stages.en.md) for types and validation limits.

## Observe constant arithmetic

`constant_candidates` includes `6 * 7`, addition reaching the u64 maximum, comparisons, and short-circuiting. `reported(4)` prints 4 before the outer addition prints 10. The loop starts at 1 and adds 2 three times to reach 7. A skipped function contains overflowing arithmetic that remains unexecuted.

```sh
cargo run --quiet -- run examples/ir_stages/constant_candidates.ceru
cargo run --quiet -- run-mir examples/ir_stages/constant_candidates.ceru --ssa
cargo run --quiet -- emit-mir examples/ir_stages/constant_candidates.ceru --ssa -o target/constant_candidates.ssa.txt
```

No optimization is performed yet. The [constant-folding design](../../docs/design/ssa-constant-folding.en.md) separates preserving original operations/origins from selecting operations to replace with literals. The [unused-result overflow example](../runtime_failures/constant_overflow.ceru) intentionally stops after printing 7.

## Generate Native through SSA

This example emits a Linux Object, links it with Clang, and executes it. On Windows, select `x86_64-pc-windows-msvc` and name the executable `target/ssa_values.exe` and omit `-lm` when linking. The output is the same four lines.

```sh
cargo run --quiet -- emit-asm examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu --annotate-origins -o target/ssa_values.ssa.s
cargo run --quiet -- emit-obj examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu --annotate-origins -o target/ssa_values.ssa.o
clang target/ssa_values.ssa.o -o target/ssa_values -lm
./target/ssa_values
```

In an `observe --ssa` bundle, `program.ssa-lowered.mir.txt` shows input copies followed by destination copies in edge helper blocks. `program.ssa-lowered-map.txt` connects SSA values/edges to MIR instructions. `program.origins.s` comes from baseline MIR; `program.ssa.origins.s` comes from reconstructed MIR. Neither overwrites the other.

## Save one compilation

Prepare the parent directory and choose a bundle path that does not exist. This command saves five files without executing the program.

```sh
cargo run --quiet -- observe examples/ir_stages/owned_values.ceru --target x86_64-unknown-linux-gnu --string-heap-limit 512 --array-heap-limit 1024 -o target/owned_values.observation
```

The files are `sources.json`, `program.ceir`, `program.mir.txt`, `program.origins.s`, and `manifest.json`. For Windows, select `x86_64-pc-windows-msvc`. Choose a different new directory on subsequent runs.

Follow `hir=#N` in MIR and `cerune-mir` ASM annotations to inspect how the same expression becomes instructions. Outputs match individual `emit-ir`, `emit-mir`, and `emit-asm --annotate-origins` commands with identical target/budgets. Compare execution separately with `run`, `run-mir`, and `run-vm`. See [format and scope](../../docs/design/observation-bundle.en.md).

## Compare LIR

Use `emit-lir` for instruction bodies and `observe --lir` for ASM from the same LIR. `--ssa --lir` saves baseline and SSA-derived snapshots separately, retaining the default generation path.

```sh
cargo run --quiet -- run examples/ir_stages/lir_layout.ceru
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-pc-windows-msvc
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-unknown-linux-gnu
cargo run --quiet -- observe examples/ir_stages/lir_layout.ceru --ssa --lir --target x86_64-unknown-linux-gnu -o target/lir-observation
```

Windows stack arguments start at the fifth integer argument; SysV starts at the seventh. LIR indices are local to each function and match annotated ASM. See [validation scope and the decision to defer direct execution](../../docs/design/lir-observation.en.md).
