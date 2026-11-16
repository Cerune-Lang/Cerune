# Native subprocess execution

[日本語](subprocess-native.ja.md)

**Implemented for Windows/Linux x86-64 ASM and directly encoded Objects.** Baseline and SSA-lowered MIR feed the same LIR. Validation, transfers, deadlines, cleanup and failure selection use the [common runtime](subprocess-generated.en.md). Additional-runtime selection, records and distribution for Native build/release remain unimplemented; subprocess build requests stop with an explanatory diagnostic. Dependency records required for release packaging also follow.

## Generated operations

The [implementation](../../src/codegen/x86_64/process.rs) constructs typed SASHIMI instructions. It does not reparse ASM, translate through C, or execute an IR Executor or VM internally. Existing emit-ir / emit-mir / emit-lir / emit-asm operations and origin records expose the generation steps.

| Stage | Native operation |
| --- | --- |
| Request | Prepare argument data/length pairs and packed stdin from the evaluated Request. Store limits and NodeId/SourceId/Span in the 120-byte ABI request |
| prepare | Release temporary descriptors and stdin after the common runtime owns the request. Skip launch reservation when a result is already ready |
| Reserve | Check combined stdout/stderr capacity and allocate both before launch. Return OutputBudget or AllocationFailed; cancel any partial reservation |
| execute | Pass the reservation result to the common runtime. Clock acquisition, launch, transfers and cleanup occur only after success |
| Result | Validate the 32-word version, size, reserved words and enum indices. Select tags by declaration names and copy output into existing arrays in 1024-byte chunks |
| Release | Explicitly release the ABI result. Ordinary ownership releases returned arrays without re-enabling a disabled connection |

Native arrays allocate their 48-byte owner and payload together, storing eight-byte elements from high to low addresses. Stdin is packed into raw bytes; output is widened in reverse. Short and empty results retain the reserved-capacity accounting until owner release. Result conversion performs no additional heap allocation after launch. A 1024-byte scratch buffer keeps the conversion helper's stack frame below 4096 bytes.

Malformed request/result types in modified LIR are rejected before instruction selection. Report declaration validation is shared with the existing runtime. Invalid ABI responses flush prior output, then stop with a cerune-process-abi-v1 diagnostic carrying the original operation's location. Failure to allocate temporary request copies stops with runtime-v1 code=allocation-failed. These stops are distinct from recoverable subprocess outcomes.

## Generation, linking and startup

Explicitly link the matching process-c-abi archive and required system libraries. See the [C ABI probe](../../experiments/subprocess/c_abi/README.en.md) for archive construction. Linux example:

~~~sh
cerune emit-obj examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.o
clang --target=x86_64-unknown-linux-gnu target/process.o target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

For Windows, use x86_64-pc-windows-msvc, cerune_lang.lib and kernel32/ntdll/userenv/ws2_32/dbghelp/advapi32/bcrypt. Clang-assembled emit-asm output uses the same archive. Explicitly pass --ssa to emit-obj / emit-asm for the SSA path; ordinary unoptimized IR observation remains available.

process.json uses the [shared CLI configuration](subprocess-cli.en.md), including an explicit Linux supervisor path. Settings are parsed only after a leading --cerune-runtime-v1; everything after -- remains a program argument. Programs also using file operations can combine --file-root. Without the marker, the connection is unconfigured. JSON is bounded to 1 MiB; Windows opens configuration paths as UTF-16. Missing, duplicate or unknown settings, read failures and invalid JSON stop before program execution.

## Verification and remaining work

The [common comparison script](../../experiments/subprocess/c_abi/generated.cjs) accepts --native for baseline MIR/ASM, or --native --native-object --native-ssa for SSA/direct Object. Both Windows and Linux perform 76 comparisons of existing [execution examples](../../examples/process_execution/README.en.md) against four executors. Coverage includes copies, evaluation order, short circuiting, 4097-byte output, nonzero exit, timeout, output limits, reservation failures, complete array release, program arguments and file connections. Allocation/ABI failures and cleanup values are injected in the ASM variant; commands and raw results are preserved.

Programs without subprocess operations gain no archive dependency. Explicit additional-runtime inputs for Native build/release, runtime/Linux-supervisor distribution and the WAT host follow separately.
