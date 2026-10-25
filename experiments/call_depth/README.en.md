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

This is not a search for minimum safe stack sizes or maximum safe depths. OS, compiler, optimization, expression shapes, and generated helpers affect results. Release success does not establish a universally safe limit; debug builds will not get different language depth semantics. Physical-resource measurements for C, LLVM, QBE, WAT, and Native are still pending.

## Next implementation steps

1. Start with the standard HIR execution route: replace host-recursive function calls with frames and continuation positions explicitly managed by the executor. Keep logical counting restricted to source functions while including generated helpers in physical frame management.
2. Diagnose frame allocation failure separately from depth overflow; compare argument order, ownership cleanup, NodeId/Span, and early returns against existing tests. Apply the approach to MIR, SSA, and VM in turn.
3. Validate generated-route resource conditions, then establish common ranges, CLI configuration, and diagnostics before enabling recursion.

This change adds measurements and establishes the order only. Explicit frame execution and new resource diagnostics remain unimplemented. No duplicate IR, execution control through observation APIs, or implicit tail-call optimization is introduced.

CI runs `node --test experiments/call_depth/probe.test.cjs` to prevent false success classifications, plus 16 shallow cases at depth 4 with a requested 16 MiB to compare fixtures, four executors, and known output.