# WAT subprocess host contract v1

[日本語](wat-process-host.ja.md)

**Implemented: eight host calls and generated-WAT integration, compared with four executors using existing examples on Windows/Linux x86-64. Explicit reference-host CLI configuration and standard-I/O/file composition are also implemented.** Use the requests, failures and Report from the [common subprocess contract](subprocess-generated.en.md), without defining a smaller WAT language subset.

## Responsibilities

| Component | Work |
| --- | --- |
| Generated WAT | Left-to-right argument evaluation, request transport, prelaunch reservation of both output arrays, result conversion, ownership release and source-located diagnostics |
| Host | Own requests/results per instance and call the existing Rust runtime prepare/execute API; share Windows/Linux connections and JSON parsing |
| Observation | Read Report and output by position. Repeated/reverse-order reads change no launch, clock, consumption, release or connection state |

WAT stays emit-only. Before execution, the host explicitly selects connection JSON, target OS and Linux supervisor; it infers no connection from cwd or environment. The host does not reenter WAT to reserve arrays. Internal memory, addresses and owners are never imported/exported.

## Imports

All imports use module `cerune`. These are the v1 types. Treat i64 as u64 bits, not a signed quantity; JavaScript must not round values through Number.

| Name | Parameters → results |
| --- | --- |
| `process_begin_v1` | (i64 argument_count, i64 stdout_limit, i64 stderr_limit, i64 timeout_ms, i64 node, i64 source, i64 start, i64 end) → i32 status |
| `process_field_v1` | (i32 kind, i64 index, i64 byte_length) → i32 status |
| `process_chunk_v1` | (i64 bits, i32 count) → i32 status |
| `process_prepare_v1` | () → i32 status |
| `process_execute_v1` | (i32 reservation) → i32 status |
| `process_report_v1` | (i32 word_index) → (i64 word, i32 status) |
| `process_copy_v1` | (i32 channel, i64 offset, i32 requested) → (i64 bits, i32 copied, i32 status) |
| `process_release_v1` | () → i32 status |

Status values are 0=accepted, 1=reservation pending, 2=result ready. Only prepare returns 1/2; execute returns 2; other successful calls return 0. Errors match the existing C ABI: 10=version, 11=shape, 12=state, 13=range, 14=UTF-8, 15=physical allocation, 16=configuration, 17=internal failure, 18=target mismatch. Never silently accept unknown success values.

## Request transport order

Call begin only after explicitly releasing the previous request/result. Declare fields in this order: target(kind=0,index=0), arguments(kind=1,index=0 through argument_count-1), stdin(kind=2,index=0). Each field declares its byte length; chunks carry 1–8 low-order bytes first, until exactly that length. Declare empty fields but send no chunks for them.

Reject duplicates, skipped fields, missing/excess data, zero-length chunks and repeated prepare. Target/arguments are UTF-8; stdin is arbitrary bytes. Preserve Japanese, NUL, CR/LF and combining characters without conversion. Semantic invalidity such as NUL yields common-runtime InvalidRequest, never host-invented success or Unconfigured.

Wasm32 body lengths and argument counts must fit u32. The host checks allocation failure and does not preempt common validation with connection execution limits. Timeout, output limits and origins retain u64 bits when passed to the runtime. Physical transport-storage failure is separate from semantic RequestLimit/OutputBudget. Reject start > end.

## Launch and reservation

| State | Allowed operations |
| --- | --- |
| Idle | begin |
| Receiving | Ordered field/chunk, prepare after all bodies arrive, release to discard |
| Pending | report, execute, release to cancel before launch |
| Ready | report/copy, release |

Prepare runs common validation and transfer-storage reservation once, without launching or acquiring clocks. A ready result skips language-array reservation and execute. Pending requires both output arrays to be reserved; execute receives 0=reserved, 1=capacity failure, 2=physical allocation failure. Values 1/2 never launch: the common runtime produces OutputBudget/AllocationFailed.

Cancel a partial array reservation and never retry it implicitly. After launch, fill pre-reserved result storage without additional result allocation. The common runtime retains unreclaimed OS resources and disabled connections; release never re-enables them.

## Results and stopping

Report exposes the [32-word fixed Report v1](subprocess-generated.en.md#fixed-report-v1-representation-codec-implemented), indices 0–31. Copy uses channel=1 stdout / 2 stderr and requested=1–8. At the end, copied=0; otherwise it is min(requested,remaining); offsets beyond the end fail. Unused high bytes are zero. Ignore word/bits/copied on failure. Reads never advance a cursor.

Generated WAT validates Report version, count, reserved words, enums, origins and quantities, mapping enums by name rather than declaration order. Nonzero child exits and recoverable failures return ordinary Outcomes. ABI failures flush prior output through the existing contract, deliver a `cerune-process-abi-v1` diagnostic containing the operation's NodeId/SourceId/Span and status, then stop. Do not include request bodies, registered paths or environment values. Host exceptions/engine stops are not success, EOF or depth-limit, and stopped instances are not resumed.

## Compatibility and implementation order

Subprocess operations, including those in uncalled functions, require all eight imports; older hosts fail instantiation. Programs without these operations require no new imports. Preserve existing output/flush/input/argument/file imports and one main invocation.

1. Verify private memory, state order, u64, Report reads and common-runtime agreement with the [independent ABI probe](../../experiments/subprocess/wat_abi/README.en.md).
2. Connect request transport, array reservation, Report conversion, release and diagnostics to generated WAT; compare existing .ceru examples with four executors.
3. Add explicit configuration to the reference-host CLI and compare real Windows/Linux children, output, timeouts and cleanup. Then complete official runtime/supervisor distribution.

Host implementation/comparisons for stages 1–3 are complete. The [reference-host CLI](../../experiments/wat_host/README.en.md) requires --process-target and accepts common connection JSON through optional --process-config. Omitted JSON yields Unconfigured. Empty/invalid/over-1-MiB JSON and mismatched targets fail before program execution. No target is inferred from the running OS and no internal memory is exposed. Official distribution follows.

## Generation and validation

[src/codegen/wat/process.rs](../../src/codegen/wat/process.rs) evaluates/transports the request once and receives Report into 32 private locals. It checks combined output capacity before reserving both arrays and cancels partial reservation on allocation failure. After launch, no additional result storage is allocated; retained bytes become array length. Short/empty results remain charged at reserved capacity until release.

[Generated-artifact tests](../../experiments/wat_host/tests/process_generated.rs) use the existing [subprocess examples](../../examples/process_execution/README.en.md).

| Check | Compared or verified behavior |
| --- | --- |
| Four executors | Unconfigured/unregistered/invalid requests, capacity failure, evaluation order, short circuits, Report origins |
| Real Windows/Linux children | Japanese/empty/quoted/trailing-backslash arguments, binary stdin, exit code 7, multiple chunks, repeated calls and copies |
| Reservation/ownership | Inject second-allocation failure before launch; check array usage returns to zero at normal completion |
| Invalid hosts | Located stops for malformed versions/enums/payloads/origins/counts/reserved words, short copies, excess bytes and transfer failures |
| Compatibility | Unused calls still reject old hosts; no extra imports without operations; private memory and repeated-main rejection |

[CLI comparisons](../../experiments/wat_host/tests/process_cli.rs) cover timeouts/continuation on both OSes together with stdin/output/flush/arguments/file creation. Unknown progress after cancellation keeps the connection disabled even if cleanup completed. Linux cleanup failure uses delayed notifications from a supervisor fixture and compares the result and next-launch rejection with four executors. Report injection on both OSes checks enum conversion, not naturally occurring OS faults. Physical Wasm-engine memory exhaustion remains separate from common capacity limits; these limits are not a physical safety guarantee.
