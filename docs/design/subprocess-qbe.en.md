# Generated QBE subprocess integration

[日本語](subprocess-qbe.ja.md)

**2026-11-16: implemented and verified on Windows x86-64 MSVC and Linux x86-64 GNU.** Connect common-IR process.run to the shared Rust runtime through the seven calls verified by the [ABI probe](../../experiments/subprocess/qbe_abi/README.en.md). Launch, transfer, deadlines, and cleanup follow the [shared contract](subprocess-generated.en.md), without QBE-specific execution semantics.

## Operations visible in generated code

| Operation | QBE implementation |
| --- | --- |
| Request evaluation | Existing lowering evaluates once; preserve ordinary ownership copying and source call depth for the standard run function |
| Request conversion | Resolve storage offsets by field name/type; place string data/length, argument table, raw stdin, limits, and origins in the 120-byte ABI request |
| prepare | Release temporary argument/stdin copies after the shared runtime owns the request; distinguish pending reservation and completed results |
| Reserve both outputs | Reserve stdout/stderr up to their limits before launch; return OutputBudget or AllocationFailed and roll back partial reservation |
| execute | Supply the reservation result; only success starts runtime clock reads, launch, transfers, and cleanup |
| Restore results | Validate the fixed 32-word version, size, and reserved words; map enum tags by name and read output in 4096-byte chunks |
| Dispose | Explicitly release the ABI result; return arrays to ordinary ownership management without re-enabling disabled connections |

A QBE string points to length followed by bytes; a dynamic array points to its owner. Convert argument string references into ABI data/length pairs and pack stdin's eight-byte element slots into raw bytes. Preserve original language values until prepare copies them. Widen output bytes back into eight-byte u8 slots.

The 48-byte array owner stores data, length, references, accounted bytes, initialized elements, and stride. Even short or empty results retain their reservation charge until owner disposal. Result conversion requires no new heap allocation. Ordinary array-allocation failure still stops the language as before.

Retain ProcessRun in QBE-specific instructions and carry NodeId/SourceId/Span into ABI requests and failure diagnostics. Unoptimized common IR remains observable. QBE input IL exposes each stage; this does not claim that QBE's own optimizations can be disabled.

## Generate, link, and run

Supply a matching-target runtime archive explicitly. On Linux, process.json uses the [CLI configuration](subprocess-cli.en.md), including the absolute supervisor path and registered executable.

~~~sh
cargo build --example subprocess_linux_supervisor
cargo rustc --lib --features process-c-abi --target x86_64-unknown-linux-gnu --crate-type staticlib -- --print native-static-libs
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.ssa
qbe -t amd64_sysv -o target/process.s target/process.ssa
clang --target=x86_64-unknown-linux-gnu target/process.s target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

On Windows, use MSVC/Windows SDK link libraries.

~~~powershell
cargo rustc --lib --features process-c-abi --target x86_64-pc-windows-msvc --crate-type staticlib -- --print native-static-libs
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-pc-windows-msvc -o target/process.ssa
qbe -t amd64_win -o target/process.s target/process.ssa
clang --target=x86_64-pc-windows-msvc target/process.s target/x86_64-pc-windows-msvc/debug/cerune_lang.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -ladvapi32 -lbcrypt -o target/process.exe
target/process.exe --cerune-runtime-v1 --process-config process.json --
~~~

Never choose a target from the running OS. IR containing subprocess operations requires an explicit target and additional linking. Type-only programs acquire no subprocess runtime dependency. emit-qbe remains emit-only without implicit linking or archive discovery.

Parse process-config only inside a leading runtime marker. Programs containing file operations may also specify file-root. Preserve argv after the delimiter; without a marker, return Unconfigured. Missing/duplicate/unknown settings and malformed, empty, or over-1-MiB JSON stop before user code. Never automatically embed configuration JSON, registered paths, or environment values.

An invalid ABI state emits cerune-process-abi-v1 with its status and operation source position, flushes prior output, and exits with status 1. It is distinct from recoverable subprocess outcomes such as launch failure.

## Verification and remaining work

Use --qbe with the [shared comparison runner](../../experiments/subprocess/c_abi/generated.cjs) and an explicit CERUNE_TEST_QBE executable. The same .ceru/configuration produced 76 comparisons against HIR/MIR/SSA/VM on each of Windows/Linux. Generated C/LLVM run the same comparisons.

- Existing registered.ceru, unconfigured/unregistered connections, launch failure, capacity/deadline limits, copies, short-circuit/evaluation order, origins, and the complete Report.
- 4097-byte chunk transfers, empty results, retained reservation capacity, and final array usage zero.
- Four output-allocation failures, plus ABI status 13 preserving prior output/source position while stopping.
- Independently injected false/true cleanup flags, startup configuration, and coexistence with file-root/argv.

Real OS timeouts can differ in partial transfers and exit timing, so compare stable results. For completed cleanup, exclude termination_requested only from the comparison copy when there is no failure, progress is known, cleanup is complete, and the raw report agrees. Preserve raw output in commands.json and reject other differences. Test the comparison rule separately from exact boolean conversion.

CI runs pinned QBE 1.3. Existing QBE Windows workarounds remain; this integration does not rewrite assembly. Native LIR/ASM/Object and build/release, WAT hosts, and runtime/Linux supervisor distribution remain later work.