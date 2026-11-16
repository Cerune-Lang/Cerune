# Native subprocess ABI probe

[日本語](README.md)

**This probe verifies common-runtime calls from ASM and directly encoded Objects produced from typed x86-64 instructions.** It has been executed on Windows and Linux. [Execution from Cerune sources and startup configuration](../../../docs/design/subprocess-native.en.md) are also implemented. Additional-runtime inputs for build/release follow.

Following the [generated-route contract](../../../docs/design/subprocess-generated.en.md), verify value passing and linking before connecting language operations. The probe uses the same runtime archive and independent C expectations as the [C ABI probe](../c_abi/README.en.md). Runnable Cerune examples remain under [process_execution](../../../examples/process_execution/README.en.md); this probe is not a language example.

## Running

Use Rust 1.98, Node, Clang, and the target OS linker/development libraries. Prepare the archive with the C ABI probe and the supervisor executable on Linux. Specify target and archive explicitly and select a new output directory.

~~~text
node experiments/subprocess/native_abi/check.cjs x86_64-pc-windows-msvc clang target/subprocess-native-abi target/subprocess-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
node experiments/subprocess/native_abi/check.cjs x86_64-unknown-linux-gnu clang target/subprocess-native-abi target/subprocess-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

[emit.rs](emit.rs) uses the existing pinned SASHIMI API to build typed Programs for both targets, then emits ASM and Objects from the same Program. Clang also assembles the ASM. Only the explicitly selected host-compatible target is linked and executed; the other target is also assembled. The running OS does not supply an implicit generation target.

[driver.c](driver.c) reuses the C ABI assertions, but all seven runtime calls pass through typed instructions. The C compiler does not generate those runtime calls. Both directly encoded and externally assembled Objects run with the C driver at O0 and O2, giving four combinations per OS. These optimization flags apply to the C test driver, not Cerune.

## Coverage

| Subject | Checks |
| --- | --- |
| create | The 16-byte Bytes value: a pointer to a copy on Windows, two integer registers on Linux SysV |
| prepare | Copy the 120-byte request into caller stack storage; verify retained runtime data and the original storage lifetime |
| copy | Six arguments: Windows stack arguments 5/6 and Linux's six integer registers |
| Other calls | execute/report/release/destroy, 32-bit statuses, 64-bit handles, lengths, and origins |
| Execution and failures | Raw bytes, exit code 7, unknown version, invalid requests, unconfigured/unregistered targets, reservation failure, output limits, deadlines, cleanup, calls after release |
| Precision | High u64 bits in limits and Report roundtrips for NodeId/SourceId/Span values above 32 bits |
| Observation | Object and origin records are unchanged by ASM rendering; each of seven external calls retains its relocation and probe-operation ID |

Clang's create call generated from the canonical C header is saved as reference.s for inspection, never linked. Probe IDs identify call operations, not Cerune source NodeIds. They are separate from NodeId/SourceId/Span fields in the Report.

## Records and remaining work

commands.json preserves commands, arguments, exit status, and output. inputs.json records the explicit target and SHA-256 hashes of the archive, emit.rs, and Cargo.lock. The windows/linux directories retain typed instructions, ASM, Objects, and external-call origin mappings. Configuration JSON contains local absolute paths and is excluded from CI artifacts. Existing experiments/subprocess attributes exclude this test's C/Rust/JS files from GitHub language statistics.

ProcessRun [retention in LIR](../../../docs/design/lir-observation.en.md) is implemented. Next, lower it to machine instructions and connect request conversion, array reservation, Report conversion, ownership release, and startup configuration. Compare baseline/SSA and ASM/Object with existing Cerune execution examples, then update Native build/release's explicit link inputs and dependency records. This probe alone does not verify language-array reservation/release or generated source mappings. Runtime and supervisor distribution also remain subsequent work.
