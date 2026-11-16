# Subprocess QBE ABI probe

[日本語](README.md)

**2026-11-16: calls from QBE 1.3 to the shared Rust runtime passed on Windows/Linux.** [Production emit-qbe](../../../docs/design/subprocess-qbe.en.md) is also connected. This independent probe continues to verify external calls.

## Verified calls

Pass the unchanged [probe.ssa](probe.ssa) to amd64_win and amd64_sysv. The C frontend compiles only the assertion caller. Handwritten QBE calls all seven runtime functions without using the IR Executor or VM internally.

| Operation | QBE IL representation | Verification |
| --- | --- | --- |
| create | Pass the 16-byte `:bytes = { l, l }` aggregate by value | Windows passes a copy address; Linux passes pointer and length as two integers. Target and version are explicit |
| prepare | alloc8 and blit copy 120 bytes to temporary storage, then pass its pointer | The runtime owns arguments/stdin before prepare returns; later mutation of original data does not affect execution |
| execute | l context/reservation arguments, w status return | Launch only on reservation success; distinguish capacity and allocation failures |
| report | Context, u64 array, word count | Preserve 32 words, origins, high-bit limits, and UINT64_MAX |
| copy | Six pointer/l arguments | Register/stack positions, repeated reads, arbitrary bytes, written count |
| release / destroy | Context/context-storage pointer | Reject duplicate operations; result disposal does not re-enable a disabled connection; destruction clears the handle |

In create assembly, Windows passes version/target/Bytes-copy address/output through rcx/rdx/r8/r9 and allocates 32 bytes for the call. Linux passes version/target/data/len/output through rdi/rsi/rdx/rcx/r8. The probe also saves reference assembly produced by Clang from the canonical C header.

These seven calls assemble, link, and execute without assembly rewriting or additional workarounds. This neither validates every QBE feature nor resolves previous Windows investigations. The research branch and existing emitter workarounds remain unchanged.

## Run

Use a matching-target archive built with Rust 1.98, QBE 1.3, Clang, Node, and target OS development libraries. First build the archive and Linux supervisor using the [C ABI probe](../c_abi/README.en.md). Select a new output directory.

~~~powershell
node experiments/subprocess/qbe_abi/check.cjs x86_64-pc-windows-msvc target/qbe-1.3/qbe.exe clang target/process-qbe-abi target/process-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
~~~

~~~sh
node experiments/subprocess/qbe_abi/check.cjs x86_64-unknown-linux-gnu target/qbe-1.3/qbe clang target/process-qbe-abi target/process-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

A target/host mismatch is rejected. Generate and assemble both target assemblies, then link/execute only the explicit target. QBE has no -O0/-O2 switch; it processes the IL normally. Probe O0/O2 settings apply only to C assertions and do not compare QBE with and without optimization. Each command has a 120-second timeout and 8 MiB output limit; any command failure, timeout, or output overflow fails the probe.

CI pins the QBE 1.3 release archive SHA-256 to d587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320. The script uses the supplied QBE executable and records its hash; it does not claim to detect its version number.

## Assertions and artifacts

[driver.c](driver.c) independently checks Bytes=16 and Request=120 against the C header. QBE IL lacks sizeof, so this is not an LLVM-style computation of field offsets from types. The [existing C assertions](../c_abi/probe.c) verify all Request fields, request ownership, origins, NUL/CR/LF/non-UTF-8 output, nonzero exit, invalid versions/states, reservation failure, unregistered/missing children, limits, and timeout. Complete cleanup does not require a fixed termination_requested value across independent OS runs. Unknown progress after timeout requires the connection to stay disabled.

| Artifact | Contents |
| --- | --- |
| probe.ssa | Identical IL used for both targets |
| amd64_win.s / amd64_sysv.s | QBE output, assembled without rewriting |
| reference.s | C frontend create call, not linked into the executable |
| commands.json | Tool information, commands, exit status, stdout/stderr |
| inputs.json | Explicit target, QBE/archive paths, SHA-256 of QBE executable and IL |
| config-O0.json / config-O2.json | Fixture registration containing local absolute paths; excluded from CI artifacts |

## Production execution comparisons

[Generated QBE](../../../docs/design/subprocess-qbe.en.md) now implements request conversion, both output reservations, Report conversion, startup, and disposal. The [comparison runner](../c_abi/generated.cjs) with --qbe compares existing registered.ceru and failures against four executors; specify QBE through CERUNE_TEST_QBE. Keep ABI probes outside language examples. Native, WAT, and distribution follow later.
