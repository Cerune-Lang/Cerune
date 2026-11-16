# Subprocess LLVM ABI probe

[日本語](README.md)

Handwritten LLVM calls seven shared Rust runtime functions and runs the unchanged expectations from the [existing C probe](../c_abi/probe.c). This tests the external connection separately from language examples. See the [design and unsupported work](../../../docs/design/subprocess-llvm.en.md).

## Run

Use Rust 1.98, Node, Clang supporting opaque ptr, and the target OS linker/development libraries. Build the runtime archive with the [C ABI probe](../c_abi/README.en.md) first. Explicitly specify target, Clang, archive, and Linux supervisor; select a new output directory.

~~~powershell
node experiments/subprocess/llvm_abi/check.cjs x86_64-pc-windows-msvc clang target/process-llvm-abi target/process-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
~~~

~~~sh
node experiments/subprocess/llvm_abi/check.cjs x86_64-unknown-linux-gnu clang-18 target/process-llvm-abi target/process-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

A target that differs from the host is rejected. Each command is bounded by 120 seconds and 8 MiB captured output; failure, timeout, or excess output fails the probe. Configuration registers only explicit fixtures and inherits no child environment.

## Tests and saved files

| File | Purpose |
| --- | --- |
| common.ll, windows.ll / linux.ll | Handwritten layouts, request copying, and external calls; only create differs by OS |
| driver.c | Reuses C assertions with every runtime call redirected to LLVM; independently checks sizes, offsets, and u64 |
| reference.c | Produces reference LLVM/ASM from the canonical C header; never linked into the test executable |
| Output probe.ll, probe-O0.s / probe-O2.s | Executed LLVM and assembly for each explicit optimization setting |
| reference.ll / reference.s | C frontend calling convention; create declaration and target checked automatically |
| commands.json | Clang version, arguments, link inputs, exit status, stdout/stderr |
| config-O0.json / config-O2.json | Test configuration containing local absolute paths; excluded from CI artifacts |

CI retains commands.json and LLVM/ASM only. Reference LLVM includes checkout source paths, but registration settings, environment, and input contents are not automatically included. Reuse the built archive and consult the C ABI probe's records for runtime build options.

Windows/Linux -O0/-O2 passed on 2026-11-16. Tests cover 15 sizes/offsets, 64-bit values, independent request copies, origins, NUL/CR/LF/non-UTF-8 output, nonzero exit, reservation failures, unregistered/missing children, output limits, timeout, and state transitions. Raw exit results are retained; uncertain progress after timeout is not substituted with success.

Production LLVM integration is also implemented. Pass --llvm to the [generated-artifact comparison](../c_abi/generated.cjs) to verify language array reservation, Outcome conversion, startup, and existing .ceru execution. This independent ABI probe continues to test external calling conventions.
