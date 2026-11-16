# LLVM subprocess integration

[日本語](subprocess-llvm.ja.md)

**2026-11-16: external runtime ABI probes are complete; production LLVM lowering is unimplemented.** emit-llvm still diagnoses IR containing subprocess execution. Integration follows the [generated-route contract](subprocess-generated.en.md), after generated C.

## Current findings

| Area | Current state and decision |
| --- | --- |
| Issues and roadmap | Continue LLVM → QBE/Native → WAT and distribution. Execution traces (#282), RV64 experiments (#284), distribution archive checks (#272), binary I/O (#243), and self-launch limits (#249) remain separate work |
| Shared runtime | Reuse prepare/execute, fixed Report, C ABI, and Windows/Linux connections. Add no LLVM-specific deadline or cleanup semantics |
| [LLVM lowering](../../src/codegen/llvm/lower.rs) | Converts HIR to LLVM-specific instructions. ProcessRun is rejected at the entry point and has no instruction representation yet |
| LLVM type metadata | Retains field types only. Request extraction and Report conversion also need field and variant names |
| LLVM arrays | Ordinary allocation stops on failure. Both subprocess outputs need fallible reservation with rollback |
| LLVM startup | argv and file-root are supported. process-config and context lifetime are not connected |

In line with the duplication concerns in [#268](https://github.com/Cerune-Lang/Cerune/issues/268), semantic decisions stay in the shared runtime. Converting Cerune value layouts into ABI values remains visible in emitted LLVM. Restructuring all backends or introducing general FFI is not a prerequisite.

## Calls verified by the ABI probe

The [independent probe](../../experiments/subprocess/llvm_abi/README.en.md) calls the existing Rust staticlib from handwritten LLVM. Separate reference LLVM and assembly produced by the C frontend provide evidence for layout and calls. The reference C function is never linked into the LLVM probe.

create accepts a 16-byte Bytes value in C. Writing that aggregate directly as an LLVM argument does not describe both OS calling conventions.

| Explicit target | LLVM create declaration | Configuration passing |
| --- | --- | --- |
| x86_64-pc-windows-msvc | i32 (i64, i64, ptr, ptr) | Copy Bytes into caller-owned temporary storage and pass its address |
| x86_64-unknown-linux-gnu | i32 (i64, i64, ptr, i64, ptr) | Split data pointer and length into separate arguments |

These arguments represent version, target, configuration, and output handle. The other six functions accept only integers and pointers. copy has six arguments, exercising Windows stack arguments too. An internal LLVM helper accepts configuration as pointer plus length and selects the external call by target. No change to the existing C ABI is required.

Bytes occupies 16 bytes and Request 120. Two sizes and all 13 field offsets computed by LLVM are compared with C sizeof/offsetof. Lengths and time retain their i64 bit patterns without narrowing to signed ranges. UINT64_MAX, limits with the top bit set, and origins beyond 32 bits survive the Report round trip. Restoring a SourceId number does not establish its membership in a source map.

On Windows, Clang appends an MSVC environment version to the selected MSVC triple. The probe checks that it still matches the requested target, allows only a numeric version suffix for that target, and writes that spelling into LLVM. It neither changes OS/ABI nor suppresses all warnings.

## Production integration steps (unimplemented)

| Order | Work | Conditions to retain |
| --- | --- | --- |
| 1 | LLVM ProcessRun instruction and type names | Evaluate the request once; retain short-circuiting, left-to-right order, and NodeId/SourceId/Span |
| 2 | Request conversion and prepare | Convert UTF-8 and raw bytes explicitly. Free temporary storage only after prepare owns the request |
| 3 | Reserve both outputs, then execute | A language u8 element accounts for 8 bytes. Reserve both before launch; roll back additional storage if either fails. Pass capacity/allocation failures to the shared runtime |
| 4 | Convert Report/output, then release | Resolve fixed words through variant names. Initialize retained elements only; account for reserved capacity until owner disposal even for short results |
| 5 | Startup settings and destroy | Interpret process-config/file-root only inside the leading runtime marker, retain argv after the separator, and destroy the context after execution |
| 6 | Language examples and failure comparisons | Compare registered.ceru, owned copies, order, origins, reservation failures, and invalid settings with HIR/MIR/SSA/VM and generated C |

Real-OS comparisons use dedicated fixtures and fresh output directories (#104). They do not add a feature to rerun arbitrary user registrations for comparison. Required-check aggregation (#262) is separate work; this change adds tests to the existing Windows/Linux subprocess jobs.

ABI violations stop with an operation origin and status, instead of becoming recoverable subprocess outcomes. report/copy reads do not advance clocks, transfers, or disposal. Keep unoptimized request conversion, reservation, execution, and disposal visible. Do not add implicit archive discovery or linking.

## Verified scope and remaining work

Windows/Linux at -O0 and explicit additional -O2 checks cover seven calls, layouts, u64, request ownership, origins, raw bytes, nonzero exit, repeated positional reads, malformed inputs and duplicate operations, reservation failures, unregistered/missing children, output limits, timeout, and reuse after release. If real OS timeout cleanup leaves progress unknown, the probe checks that the connection stays disabled.

This probe **does not execute a language example through the LLVM route**. Fallible language array reservation, public Outcome conversion, startup arguments, and integration into the ordinary LLVM emitter are the next change. Existing shared-runtime/codec tests cover failure classes that cannot be induced reliably on a real OS; this is not a claim that every failure was tested on an OS. Native packaging, WAT hosts, and a general observation API remain separate stages.
