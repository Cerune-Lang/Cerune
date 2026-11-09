# Subprocess contracts for generated routes

[日本語](subprocess-generated.ja.md)

**Status: the Rust prepare/execute API and fixed Report v1 codec are implemented.** [Four executors](subprocess-execution.en.md) and [Windows/Linux CLI connections](subprocess-cli.en.md) use the same runtime. The C ABI and static linking are verified by an [independent C probe](../../experiments/subprocess/c_abi/README.en.md). [Generated C integration](subprocess-c.en.md) is also implemented. LLVM/QBE/WAT/Native still reject IR containing the execution operation. The implemented Rust API is distinguished below from future generated-route contracts.

## Findings and direction

| Finding in existing code | Required generated-route work |
| --- | --- |
| The [common runtime](../../src/runtime/process.rs) decides validation, transfers, deadlines, cleanup, and disabling | Reuse this Rust implementation instead of porting semantics into each backend |
| `run_with_reservation` orders validation, transfer buffers, language result reservation, then clock/launch | Extract shared preparation/execution stages; keep existing APIs as wrappers |
| Returned [u8 arrays](../../src/runtime/array_heap/byte_outputs.rs) account for eight bytes per element | Distinguish packed ABI buffers and explicitly convert into generated arrays |
| [Generated C array allocation](../../src/codegen/c/array.rs) stops on failure | Add fallible subprocess-result reservation without changing ordinary allocation failure semantics |
| `ProgressUnknown` occurs at different positions in the [Rust enum](../../src/runtime/process/records.rs) and [standard source](../../src/standard/process.ceru) | Map explicit numbers and names; never reuse Rust discriminants or source tag ordinals |
| The [WAT file host](wat-file-host.en.md) does not expose internal memory | Use versioned values/chunks without public memory or host mutation of language arrays |
| [Native linking](../../src/native_build/link.rs) and runtime records assume owned objects plus system runtime | Explicitly record additional link inputs, compatibility, and distribution requirements |

Use a **Cerune-provided static library with a C ABI** around the common runtime for C/LLVM/QBE/Native. The WAT reference host uses the same staged runtime API. Generated programs do not execute an IR Executor or VM internally.

| Component | Responsibility |
| --- | --- |
| Common IR and lowering | Evaluation order, short circuiting, types, ownership, origins, and external-operation placement |
| Generated code | Request conversion, fixed result preparation, language array reservation/finalization/release, Report-to-public-enum conversion |
| Common runtime | Validation precedence, transfers, deadlines, failure selection, cleanup, and disabled connections |
| Windows/Linux Backend | Existing explicit registration and OS resources; Linux supervisor owns OS parentage/reclamation only |
| ABI adapter | Validate widths, versions, state, and ownership; no independent timeout or retry policy |

Separating all existing file/output runtimes is not a prerequisite. Programs without subprocess operations gain no new link dependency. Current IR retains unused execution-module functions, so an import alone can introduce the operation. Do not introduce implicit dead-code elimination to change that rule.

## Validation, reservation, and launch order

Keep these steps visible in generated code. The names below describe operations, not existing CLI commands.

```text
evaluate and copy the request once
prepare fixed Report and empty failure arrays
prepare(request, transfer allowance, origin)
  ├─ result ready       → obtain Report and empty arrays
  └─ reservation needed → reserve language stdout/stderr arrays
                           execute(success / capacity / allocation failure)
convert Report and retained raw bytes into reserved language values
release result storage and continue the caller
```

1. Request-copy or fixed-result construction failure remains language resource exhaustion. No child starts.
2. `prepare` advances the call identity once and preserves precedence: disabled, invalid request, request limits, unconfigured, transfer-storage allowance/allocation. Registered-name lookup remains in the launch stage.
3. `prepare` performs no child launch, OS transfer, or operation-deadline clock read. Pending state owns the request, frozen connection settings, and buffers; configuration cannot change midway.
4. Generated code reserves both arrays at their limits. A second-allocation failure releases new reservations while preserving the fixed Report and empty failure arrays. Short results retain reserved-capacity accounting until release.
5. `execute` accepts exactly one reservation result. Capacity failure returns `OutputBudget`; allocation failure returns `AllocationFailed` at `Reserve`, without launch. Only success reaches existing clock/deadline/launch/transfer/cleanup logic. Do not revalidate and change precedence.
6. After launch, constructing Report, public enums, outer Outcome, and finalized arrays requires no new allocation. Copy packed bytes into reserved elements and finalize retained lengths.
7. Discarding pending preparation releases unpublished storage only. After execution, existing runtime ownership preserves unreclaimed resources and disabled-state/cleanup reports.

`prepare` and `execute` control execution; they are not observation APIs. Existing Rust `run` / `run_with_reservation` remain compatible wrappers over consecutive stages. Scripted Backends verify agreement between existing and staged entry points in results, OS operation sequences, and observations.

## Implemented Rust API

The [staged API](../../src/runtime/process/preparation.rs) has these responsibilities.

| API | Behavior |
| --- | --- |
| `Processes::prepare` | Validate and allocate transfer buffers; return a completed result or pending reservation |
| `Prepared::report` | Read the current record without clock reads, OS actions, or state changes |
| `Prepared::execute` | Accept language-side reservation status and launch only on success; consume preparation and return the result and reserved storage |
| `run` / `run_with_reservation` | Existing entry points calling the same consecutive stages |

The Rust API borrows the request immutably and the connection exclusively, while owning transfer buffers. Existing language bridges copy requests before the call. No additional owned copy or Box allocation is introduced. The C ABI prototype now owns request copies and contexts across foreign calls and checks protocol states. WAT integration follows later. Borrowing Rust structures are not exported as an ABI.

Dropping an unexecuted pending state releases only unsubmitted buffers. It advances no clock, launch, or cleanup; observations retain Validate/Reserve. The consumed call identity is not rolled back, and the next call receives the next identity. Compile-fail tests reject duplicate execution and connection reuse while preparation remains live.

[ReportV1](../../src/runtime/process/report_codec.rs) converts the 32 words below and their little-endian bytes. It rejects unknown versions, invalid tags, reserved fields, width overflow, reversed spans, and sub-millisecond rounding without changing partial counts or uncertain progress. Restoring a SourceId number does not validate membership in a source table: consumers must associate it with the original compilation's SourceMap. This is record conversion, not exported C functions or a native memory-layout contract.

[Staged tests](../../src/runtime/process/tests/preparation.rs) compare results, actions, observations, reservation failures, and reuse after dropping preparation. [Codec tests](../../src/runtime/process/report_codec/tests.rs) check independent expected values, all failure codes, distinct cleanup records, u64 precision, and malformed representations. In addition to the four executors, the [real-OS probe](../../experiments/subprocess/execution.rs) launches a fixture through the staged API and checks raw bytes, nonzero exit, origins, and Report round trips.

## Generated stages and observation

Preserve unoptimized common IR and subsequent representations. Generated C/LLVM/QBE expose request extraction, prepare, array reservation, execute, element conversion, and release. Native preserves the same correspondence in MIR/LIR, annotated ASM, and Object origins. An external runtime call must not erase source locations.

Report NodeId/Span continues to identify the operation inside the standard wrapper, separately from the user's Call position. SourceId refers to the same compilation's source table. Carry ABI origins into shared runtime operation records; never automatically include arguments, environment, or payloads in diagnostics.

Positioned result reads and read-only observation must not advance deadlines, transfers, subsequent calls, or release. Observation capacity/allocation failure must not alter control. General record/replay and a common success-observation API for every generated route are not additional prerequisites for this change.

## ABI values and ownership

Initial native targets are `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`. Runtime and generated artifacts use the same target/calling convention. Require explicit target selection for C subprocess emission while preserving existing C invocation forms without subprocess operations.

| Value | Proposed representation and lifetime |
| --- | --- |
| ABI header | Explicit version and structure size; reject unsupported versions/sizes before launch |
| Request | UTF-8 target, individual UTF-8 arguments, packed stdin bytes, each with a length; no NUL termination dependency |
| Numbers | Counts, lengths, limits, milliseconds, and origins use u64; tags/status use explicit integers. No Rust `usize`, `bool`, enums, or `Vec` cross the ABI |
| Native references | Pointer plus length; validity, non-aliasing, and readable storage are caller obligations, not safely validated arbitrary pointers |
| Connection | Explicit opaque context; at most one operation, no reentry/concurrency/mid-operation configuration changes |
| Request retention | Allocate required copies before `prepare` returns; never retain generated temporary references |
| Result | Context owns fixed Report and packed bytes until conversion into language values; explicitly release before the next request |
| Unreclaimed OS resources | Retained separately by context/runtime; releasing a result does not reset a disabled connection |

Distinguish semantic invalid requests from broken ABI representations. Empty targets, NUL, or zero timeouts are representable Cerune values and yield common-runtime failures. Unsupported versions/tags, invalid state, unrepresentable physical lengths, and invalid UTF-8 are connection-contract or resource failures. Never substitute `Unconfigured` or successful empty output. Stop with the original operation's source position, without disguising the error as child exit status.

Do not unwind across C callers. A caught internal failure disables reuse and preserves remaining resource ownership. Allocator abort or OS termination is not guaranteed to recover as an Outcome.

## Fixed Report v1 representation (codec implemented)

Use the same field table for C ABI, WAT hosts, and comparisons. No array bodies, diagnostic strings, or pointers appear in the record. The initial proposal has **32 u64 words (256 bytes)**. Native uses a target-ABI u64 array; byte serialization uses little-endian; WAT i64 carries the same 64 bits.

| Word | Contents |
| --- | --- |
| 0–1 | Version=1, word count=32 |
| 2–5 | NodeId, SourceId, Span.start, Span.end (UTF-8 byte range) |
| 6 | Started: 0=No, 1=Yes, 2=Unknown |
| 7–11 | stdin_requested, stdin_accepted, stdout_limit, stderr_limit, timeout_ms |
| 12–16 | stdout_read, stderr_read, stdout_retained, stderr_retained, progress_known (0/1) |
| 17–18 | Exit: 0=Absent, 1=Code, 2=Signal; u32 payload, zero when Absent |
| 19–21 | Failure code, phase, transfer channel; all zero without failure |
| 22–24 | Evaluated cleanup kind, flags, error |
| 25–27 | Raw cleanup_reported kind, flags, error |
| 28–31 | Reserved, zero in v1; never silently interpret extensions |

Failure code 0 means none. Codes 1–21 are `InvalidRequest, RequestLimit, Unconfigured, Unregistered, ConnectionDisabled, OutputBudget, AllocationFailed, NotFound, Denied, Resources, Unsupported, StartFailed, TransferFailed, CloseFailed, WaitFailed, ZeroProgress, InvalidResponse, OutputLimit, Timeout, CleanupFailed, ProgressUnknown`. Map by name even if language enum order changes.

Phases are 0=none, 1=Validate, 2=Reserve, 3=Start, 4=Transfer, 5=CloseStdin, 6=Wait, 7=Cleanup. Only Transfer carries channel 1=Stdin, 2=Stdout, or 3=Stderr; other phases use zero.

Cleanup kinds are 0=Absent, 1=Complete, 2=Incomplete. Complete permits only flag bit0=termination_requested and error zero. Incomplete permits bit1=child_reaped, bit2=handles_closed, bit3=scope_finished, with error 1=Timeout, 2=Terminate, 3=Reap, 4=Close, 5=InvalidResponse. Absent has zero flags/error. Reject unknown tags, invalid combinations, reserved bits, reversed spans, and rounded durations.

No casts based on Rust declaration order or host integer width. Test every encoding/decoding alternative against independent expectations; distinguish cleanup from cleanup_reported and transferred bytes from retained bytes.

## WAT host mapping

Wrap the same staged API with value transport. Do not reenter generated WAT through a host reservation callback.

| Stage | Proposed generated-WAT/host contract |
| --- | --- |
| Request transport | Versioned imports accept declared lengths and chunks of up to eight bytes; reject omissions, excess, and duplicate completion |
| prepare | Invoke shared prepare once after receiving the complete request; distinguish result-ready and reservation-needed |
| Reservation/execute | Reserve language arrays in private memory and pass the reservation result to the host; launch only on success |
| Result reads | Read Report words and output chunks by position, without advancing cursors or changing results on repeated reads |
| Release | Generated code explicitly releases the result through a control call; observation reads never release it |

Allocate request staging before launch. Never silently truncate declared lengths or prevalidate using connection execution limits in a way that changes common-runtime precedence. Distinguish wasm32 physical capacity from semantic limits. Use BigInt/i64 bits without rounding u64 through JavaScript Number.

Do not expose internal memory. Preserve existing output, `flush_stdout_v1`, input, argument, and file imports. New artifacts containing subprocess operations require versioned imports and fail instantiation on old hosts. Artifacts without subprocess operations require no additional imports. Freeze exact import names/signatures in the WAT contract before implementation.

## Startup, linking, and distribution impact

Generated C extends the existing leading runtime marker as shown below. Other generated routes follow later.

```text
program --cerune-runtime-v1 --process-config process.json --file-root directory -- program-arguments
```

Without the marker, preserve existing argv and leave the connection unconfigured. Never interpret tokens after `--`. Missing/duplicate/unknown settings fail before program entry. Share parsing of the CLI's `cerune-process-v1` JSON. Require an explicit Linux supervisor path; never select another target's configuration from the running OS.

| Route | Required change |
| --- | --- |
| emit-c / emit-llvm / emit-qbe | Expose versioned external calls, required ABI and target. Users own external compilation/linking; no implicit runtime discovery/linking |
| emit-asm / emit-obj | Preserve requests, origins, and staged calls in LIR; SASHIMI continues to encode instructions and relocations |
| build native | Explicit runtime archive input; record target, ABI version, hash, system dependencies, link arguments, and results |
| release native | Validate/preserve build dependency records; distinguish statically linked runtime from the separate Linux supervisor |
| WAT | Remains emit-only; validate host ABI/imports/runtime implementation and document distribution requirements |

A new subprocess link dependency changes compatibility. Existing `target-system-runtime` / `cerune_runtime: in-object` records cannot represent it alone. Version Native manifest/runtime/release validation changes; do not invent missing dependency records for older artifacts.

Never automatically embed local registered absolute paths, environment values, or configuration JSON in distribution artifacts. Supervisor bundling records its executable, version, target, and hash. Runtime filenames, public build options, and distribution layout will be finalized through linking probes.

## Implementation order and verification

| Order | Change | Completion criterion |
| --- | --- | --- |
| 1 (implemented) | Split shared prepare/execute and implement fixed Report codec | Existing/staged APIs agree on results, OS calls, and observations; failed reservation never launches; test duplicate execution and discarded preparation |
| 2 (prototype implemented) | Native C ABI adapter and static linking probe | Windows/Linux widths, layout, lifetime, invalid version, failure, and reclamation; independent C expectations |
| 3 (implemented) | [Generated C lowering and startup](subprocess-c.en.md) | Compare [registered.ceru](../../examples/process_execution/registered.ceru) with four executors: unconfigured, partial output, nonzero exit, continuation, ownership, origins |
| 4 | LLVM/QBE/Native and Native build/release | Same ABI; baseline/SSA, explicit target/linking/dependency records |
| 5 | Versioned WAT host and supervisor distribution completion | Private memory, old-host rejection, matching requests/results/cleanup, relocated distribution execution |

Never replace remaining unsupported emission with successful stubs. Do not add unimplemented future-API `.ceru` files to executable examples; update existing example instructions as each route becomes executable.

At every stage compare all failure categories, zero/exact/exceeded limits, u64 precision, NodeId/SourceId/Span, and independent copies. Use scripted responses for short transfers, interruption, protocol errors, and unreclaimed cleanup; use dedicated fixtures for real OS tests. Observation must not change control flow, and separate real-time executions near deadlines are not guaranteed to agree.

The C ABI prototype has [dedicated documentation](../../experiments/subprocess/c_abi/README.en.md) for signatures, states, and verification. Its JSON parser is shared with the CLI; tests also verify that releasing results never re-enables a disabled connection.
