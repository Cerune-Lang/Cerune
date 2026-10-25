# Call depth and physical stack experiment

[日本語](README.md)

**This validates prerequisites for choosing defaults; it is neither the normal Cerune CLI nor a recursive language example.** The same source-function depth can exhaust the host stack before the common depth check, depending on executor, value shape, and Rust build settings.

## Method

probe.cjs generates linear, non-recursive call chains. At depth N it tries limits N−1 and N, expecting a depth failure and completion respectively. Cases use scalar integers or products containing owned strings and fixed arrays. Generated helpers do not consume logical depth.

| Item | Treatment |
| --- | --- |
| Isolation | One process per route, shape, depth, and limit; 30-second timeout per case |
| Preparation | Parse and construct HIR, MIR, SSA, and bytecode outside the measured thread. Validation inside execution APIs and execution-state cleanup remain measured |
| Stack | Explicit request to thread::Builder::stack_size; neither actual OS allocation nor used stack bytes are measured |
| Records | Preparation/execution start and completion, depth diagnostics, expected output, exit code/signal, stdout, stderr |
| Matching | Require zero exit status, complete protocol, and expected output; depth failures also require matching limit, active-frame count, and call-v1 |
| Resource failure | Classify host-stack-exhausted only with abnormal termination after execution starts and Rust stack-overflow evidence; otherwise distinguish process-terminated with unknown cause, launch failure, timeout, etc. |
| Exit status | The driver exits 1 if any case differs from expectation. Stack exhaustion is not a successful test |

A physical abort cannot be converted into a common Cerune diagnostic here. Captured prior program output may also be unrecoverable. These are observations by an external experiment driver, not an implementation of language-level resource handling.

## Running

Use Rust and Node.js 20 or newer. Choose a **new output directory** whose parent exists; previous observations are never overwritten.

```sh
cargo build --example call_depth_probe
# Append .exe to the executable name on Windows.
node experiments/call_depth/probe.cjs target/debug/examples/call_depth_probe target/depth-debug 8,32,64 2097152
cargo build --release --example call_depth_probe
node experiments/call_depth/probe.cjs target/release/examples/call_depth_probe target/depth-release 8,32,64 2097152
```

On Linux and similar systems, first run `ulimit -c 0` in the same shell to suppress core dumps from intentional failures. Windows children use the existing test helper to suppress crash dialogs. Normal CI does not run the large probes.

| File | Contents |
| --- | --- |
| environment.json | OS, architecture, Node, current rustc, executable SHA-256, requested stack bytes. rustc describes the measurement environment, not build attestation |
| scalar-N.ceru / aggregate-N.ceru | Identical source for all routes; inspect transformations using normal CLI emit-ir and other emit commands |
| results.jsonl | Exact command arguments, expected result, classification, exit status |
| Per-case .stdout / .stderr | Raw evidence; stdout also includes OS, debug_assertions, helper count, and call-v1 diagnostics |

Individual invocation is `cargo run --example call_depth_probe -- hir <file.ceru> <limit> <stack-request-bytes>`, but the process itself may abort. Prefer the driver.

## Results on 2026-10-25

Measured runtime commit 143aae0 (PR #200), Rust 1.98.0, Windows x86_64 MSVC and Linux x86_64 GNU. Each depth used both N−1 and N limits. [Recorded observations](observations.json) retain the details.

| Conditions | Scalars | Products with strings and arrays |
| --- | --- | --- |
| Windows / Linux, debug, requested 2 MiB, depth 8 | Expected results in all four routes | Expected results in all four routes |
| Same, depth 32 | HIR exhausts stack; MIR, SSA, VM match expectations | All four exhaust stack |
| Same, depth 64 | All four exhaust stack | All four exhaust stack |
| Windows, debug, requested 16 MiB, depth 8/32/64 | Expected results in all four routes | Expected results in all four routes |
| Windows / Linux, release, requested 2 MiB, depth 8/32/64 | Expected results in all four routes | Expected results in all four routes |

This is not a search for minimum safe stack sizes or maximum safe depths. OS, compiler, optimization, expression shapes, and generated helpers affect results. Release success does not establish a universally safe limit; debug builds will not get different language depth semantics. These earlier measurements do not include generated routes; the C experiment below records a later measurement.

## Implementation order and current progress

1. Replace host-recursive calls in all four executors with explicit frames and continuation positions (implemented). Keep logical counting restricted to source functions while including generated helpers in physical frame management.
2. Diagnose frame allocation failure separately from depth overflow; compare argument order, ownership cleanup, NodeId/Span, and early returns against existing tests. Applied to MIR, SSA, and VM as well.
3. Validate generated-route resource conditions, then establish common ranges, CLI configuration, and diagnostics before enabling recursion.

The table and observations.json retain measurements of the earlier runtime commit 143aae0. A subsequent implementation introduced explicit frames in all four executors, tests at depth 128 with a requested 2 MiB, and frame-allocation-failure diagnostics. See the [current status](../../docs/design/recursion.en.md). Explicit limits and depth diagnostics are also implemented in generated routes. Physical resource conditions, common ranges, and CLI settings are next. No duplicate IR, execution control through observation APIs, or implicit tail-call optimization is introduced.

CI runs `node --test experiments/call_depth/probe.test.cjs` to prevent false success classifications, plus 16 shallow cases at depth 4 with a requested 16 MiB to compare fixtures, four executors, and known output.

## Generated C limits and resource conditions

`call_depth_emit_c` demonstrates the Rust API `codegen::c::emit_c_with_call_limit`. It does not add depth configuration to the normal CLI. Output files must be new.

```powershell
cargo run --example call_depth_emit_c -- examples/call_frames/depth.ceru 2 target/depth-limit-2.c
clang --target=x86_64-pc-windows-msvc -std=c11 -O0 target/depth-limit-2.c -o target/depth-limit-2.exe
.\target\depth-limit-2.exe
```

Limit 2 matches [depth.stdout](../../examples/call_frames/depth.stdout). Emit a separate C file with limit 1 to stop before outer enters mark(10), writing call-v1 to stderr and exiting with code 1 without program output. Emitted C retains argument temporaries, enter/leave, history allocation, and the limit.

The resource experiment explicitly selects target, C compiler, and bounds. Windows uses the linker's `/STACK` reservation; Linux uses the OS limit through `prlimit --stack`. These are experiment controls, not Cerune observation APIs.

```powershell
cargo build --example call_depth_emit_c
node experiments/call_depth/probe_c.cjs target/debug/examples/call_depth_emit_c.exe clang windows-msvc target/c-depth-windows 32,128 2097152
```

```sh
cargo build --example call_depth_emit_c
node experiments/call_depth/probe_c.cjs target/debug/examples/call_depth_emit_c clang linux-gnu target/c-depth-linux 32,128 2097152
```

`probe_c.cjs` never infers a target from the execution environment: a mismatch is rejected. Linux requires prlimit. A new output directory retains source, C, executables, exact commands, compiler version, artifact SHA-256 hashes, raw stdout/stderr, exit status, and comparisons with expectations. Child processes have 30-second emission, 120-second compilation, and 30-second execution timeouts. Interruption or abnormal exit is not accepted as successful depth limiting; records survive failures.

The [2026-10-25 observations](observations-c.json) are summarized below. Each OS matched 16 cases: limit N−1 produced the expected call-v1/prior output; limit N returned expected values.

| Conditions | Values and depth | Result |
| --- | --- | --- |
| Windows x86_64 MSVC, Clang 22.1.8, 2 MiB reserved stack, O0/O2 | Scalar and owned-string/array products, depth 32/128 | 16 matching cases |
| Linux x86_64 GNU, Clang 10.0.0, 2 MiB stack limit, O0/O2 | Same | 16 matching cases |

This is not a measurement of maximum stack consumption or a proof of safety for arbitrary programs. Large fixed arrays, other compilers/ABIs, and generated helpers require separate validation. Logical depth and physical stack exhaustion remain different events; no common default is selected.

`cargo test --test c_output call_depth` compares generated C with four executors. `node --test experiments/call_depth/probe_c.test.cjs` rejects abnormal termination, extra output, and incomplete histories as successes.

## Try the same limit in each generated route

`call_depth_emit` emits LLVM/QBE/WAT from the same completed HIR. Native follows MIR → LIR → typed Program to produce assembly or objects; LIR itself can also be saved. It invokes no external tools and never overwrites an existing file. Depth configuration in the normal CLI remains follow-up work.

```sh
cargo run --example call_depth_emit -- llvm x86_64-pc-windows-msvc examples/call_frames/depth.ceru 2 target/depth.ll
cargo run --example call_depth_emit -- qbe x86_64-pc-windows-msvc examples/call_frames/depth.ceru 2 target/depth.ssa
cargo run --example call_depth_emit -- wat wasm32 examples/call_frames/depth.ceru 2 target/depth.wat
cargo run --example call_depth_emit -- lir x86_64-pc-windows-msvc examples/call_frames/depth.ceru 2 target/depth.lir
cargo run --example call_depth_emit -- asm x86_64-pc-windows-msvc examples/call_frames/depth.ceru 2 target/depth.s
cargo run --example call_depth_emit -- obj x86_64-pc-windows-msvc examples/call_frames/depth.ceru 2 target/depth.obj
```

Select x86_64-unknown-linux-gnu for Linux artifacts. Execute LLVM with Clang, QBE with the selected QBE target followed by Clang, WAT with wabt and the existing Node test host, and ASM/objects by linking with Clang. Limit 2 matches [depth.stdout](../../examples/call_frames/depth.stdout); limit 1 stops with call-v1. [return_paths.ceru](../../examples/call_frames/return_paths.ceru) also exercises floating-point results through early and normal returns.

```sh
cargo test --test llvm_output --test qbe_output --test wat_output --test native_output call_depth
```

Use the existing CERUNE_TEST_LLVM_CLANG, CERUNE_TEST_QBE, CERUNE_TEST_QBE_CLANG, CERUNE_TEST_ASM_CLANG, CERUNE_TEST_NODE, and CERUNE_TEST_WAT2WASM_JS environment settings. Unavailable explicitly configured tools fail tests. Artifacts remain in target/call-depth-<route>-<pid>-<number>.

Finite 32-function chains and depth-128 internal-IR re-entry using few functions are separate tests. The latter retains owned strings and compares limits 8/127/128. This avoids repeating costly Native generation for many-function fixtures in routine CI while retaining history growth and deep-execution coverage. These comparisons use ordinary OS resource conditions; they do not establish a safe physical stack range.
