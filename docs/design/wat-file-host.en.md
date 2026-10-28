# Proposed WAT file host contract v1

[日本語](wat-file-host.ja.md)

**Status: contract proposal and a Wasm transport prototype. std:fs in emit-wat is still unimplemented.**
Connect reads and new-file creation under the [common file contract](file-io.en.md). Keep the common IR, filename rules, UTF-8, usage limits, and stopping policy. This change checks ABI feasibility; generated WAT semantics and OS connections require subsequent validation.

## Responsibilities and rationale

| Component | Responsibility |
| --- | --- |
| Generated WAT | Left-to-right evaluation, name/limit/usage checks, ownership reservation, retries, transfer accounting, close, UTF-8 validation, and source-located stopping |
| Execution host | Explicit scope selected before execution, regular-file checks, exclusive creation, byte transfer, and handle consumption |
| Observation | Read generation and diagnostics. Selecting connections or injecting responses is execution control, not an observation API |

Do not import/export internal memory, addresses, or ownership information. Pass at most eight bytes as values, as in the [output contract](wat-output-host.en.md). Do not copy the complete body into the host before writing or reenter Wasm from the host to fetch it. Only names use temporary storage bounded to 255 bytes. Generated programs export only main; transport exports in the probe are not product ABI.

Keep connection, handles, and pending name per instance. Do not infer a connection from cwd, environment variables, or the emitter OS. Generated WAT does not interpret the argv marker: the embedding host selects its connection before execution. Synchronous reentry, concurrent execution, and connection replacement during execution are outside the contract. Run main once; do not resume the instance after failure.

## Imports and values

The proposal requires these seven imports from cerune for programs containing file operations, including unused functions. Names are proposals before implementation, not a published stable ABI.

| Import | Type | Meaning |
| --- | --- | --- |
| files_status_v1 | () → i32 | 0=configured, 1=unconfigured; other values are invalid responses. No consumption or open |
| file_name_begin_v1 | (i32 length) → i32 | length=1–255. Discard previous preparation and begin a new name |
| file_name_chunk_v1 | (i64 bits, i32 count) → i32 | Append 1–8 name bytes in order, without exceeding declared length |
| file_open_v1 | (i32 mode) → (i32 handle, i32 status) | Open the prepared name; mode=0 for read, 1 for write-new |
| file_read_v1 | (i32 handle, i32 requested) → (i64 bits, i32 count) | requested=1–8. Return sequential bytes and their count |
| file_write_v1 | (i32 handle, i64 bits, i32 count) → i32 | Supply 1–8 bytes and return acceptance or failure |
| file_close_v1 | (i32 handle) → i32 | Consume the handle once. 0=success; any nonzero value=close failure |

Use low-order bytes first. Preserve the 64-bit pattern rather than interpreting the sign of i64 as character data. Ignore unused trailing bits. Preserve split UTF-8, NUL, CR/LF, combining characters, and invalid UTF-8 as bytes without host normalization or replacement. Generated WAT validates content UTF-8.

### Name preparation and open

For begin/chunk, 0 accepts all supplied data and 1 means preparation failure. There is no partial acceptance or retry. On 1, discard preparation; WAT stops with phase=open, code=open-failed, created=no. Unknown responses become invalid-response, still with created=no because preparation never creates or changes files. Do not classify host preparation allocation failure as Cerune string usage exhaustion.

Complete common name validation and read-buffer reservation before begin/chunk. Call open only after the full name has arrived. Open consumes preparation on both success and failure. A new begin also discards the previous name. Do not implicitly open or resend after preparation failure.

| Open status | Handle | Meaning |
| --- | --- | --- |
| 0 | 1–2147483647 | Success: read starts at zero; new-file creation starts empty |
| 1 | 0 | not-found |
| 2 | 0 | already-exists |
| 3 | 0 | unsupported-target |
| 4 | 0 | open-failed |
| Other values or combinations | Unspecified | invalid-response; write-new has created=unknown |

Ordinary open errors guarantee no creation or modification. The host also reclaims internal handles obtained during failed opens. Successful handles are distinct from other live handles in the instance, not raw OS descriptors or pointers. Generated WAT uses only valid successful handles and does not attempt close on unknown handles. Do not guess creation or consumption after invalid responses.

The OS connection must enforce scope and regular-file requirements. Character validation and string-prefix comparisons do not establish containment. The in-memory test host opens no OS paths and does not validate Windows/Linux scope enforcement.

### Transfer and close

| Response | Read | Write |
| --- | --- | --- |
| 1–requested | Consume that many bytes; short reads alone are not EOF | Accept that prefix; continue only the remainder |
| 0 | EOF | Stop with zero-progress |
| -1 | Stop with read-failed; no consumption in this call | Stop with write-failed; no acceptance in this call |
| -2 | Interrupted with no consumption; retry the same request | Interrupted with no acceptance; retry the same remainder |
| Greater than requested or other negative values | invalid-response, progress=unknown | invalid-response, progress=unknown |

Ignore bits when read count is nonpositive. Retain confirmed transferred bytes only; never add an invalid response count. A host must not report partial progress as zero-consumption -1. Return a confirmed prefix as success, or an invalid response if progress cannot be established. Host exceptions are outside the numeric response contract; never translate them into empty content, EOF, or success.

After EOF or transfer failure, close an acquired handle once. Close consumes the handle even on failure and is never retried. Retain an earlier primary failure with cleanup=failed; otherwise report close-failed. Do not remove partial files or undo prior output. Validate UTF-8 only after EOF and successful close.

## Transfer splitting and route consistency

C, LLVM, QBE, and other connections request reads of up to 4096 bytes and writes of the whole remainder. WAT splits host read/write requests into at most eight bytes to retain private memory. This does not change language features or the maximum file size.

| Comparison | Required agreement |
| --- | --- |
| One source operation | Argument evaluation, total requested length, stored/read content, stopping reason and origin, and surrounding I/O order |
| Limits | Read within remaining capacity, then probe EOF with a separate one-byte request at the exact limit. Record an extra byte as consumed |
| Failures | Confirmed consumption/acceptance, unknown progress, creation status, residual content, and close/primary-failure precedence |
| Connection calls | Record actual requests/responses per route; splitting and call counts need not match |

In file-v1, write requested is the complete source-operation body length, never eight. Read requested is unknown until EOF, then becomes actual length. A zero limit still probes EOF; an empty write still creates and closes a file. Existing string usage limits and wasm32 physical allocation failures retain their meanings.

Differential tests must not depend solely on failing the Nth call. Inject short transfers, interruptions, and failures at common byte positions to align confirmed prefixes. Test transport-dependent invalid responses separately for each ABI. Do not claim identical OS call sequences or identical outcomes under arbitrary external mutation.

## Stopping, observation, and compatibility

Retain FileReadText/FileWriteNew, name/limit checks, transfers, ownership, and NodeId/SourceId/Span in generated WAT. On failure, pass prior output through existing flush_stdout_v1, deliver file-v1 through write_error_byte, then stop with unreachable. Do not include bodies, names, or OS error text. Diagnostic delivery is best effort; secondary failures do not replace the primary diagnosis.

This change adds no common observation API for successful generated operations. The test host returns copies from events; recording availability and capacity do not alter transfers. Response injection is separate test control copied before execution.

| Combination | Behavior |
| --- | --- |
| No file operations | No new imports; preserve existing hosts |
| File operations with an old host | Missing required imports fail instantiation; no successful fallback |
| New host, unconfigured | files_status_v1=1; the operation stops as unconfigured |
| Combined std:io/std:env/output | Preserve existing imports, startup argument snapshot, and explicit flush semantics |

Keep the existing WAT library restrictions for pure public functions. This change does not add WASI, an OS file host, general file handles, or a stable embedding API.

## Prototype validation and subsequent implementation

The [test host](../../tests/support/wat_files.cjs) provides independent in-memory connections. The [ABI test](../../tests/wat_files_protocol.rs) assembles probe WAT with WABT and invokes real Wasm imports in Node. The product emitter remains unchanged and still rejects std:fs operations.

Verified coverage includes the 16-byte UTF-8/NUL/CR/LF/combining-character payload and all 256 byte values; short transfers, interruptions, zero progress, failures, and invalid response transport; EOF; empty creation; existing-content protection; handle consumption after close failure; observation independence; copying initial state; missing imports; host exception propagation; and private memory. Probe calling loops do not implement Cerune name/UTF-8/usage checks or file-v1 semantics.

~~~sh
cargo test --test wat_files_protocol
~~~

Run with Node and WABT available. Windows/Linux/macOS WAT CI jobs require execution with CERUNE_TEST_NODE and CERUNE_TEST_WAT2WASM_JS configured.

Subsequent changes connect generated WAT in this order:

1. Lower common IR and implement name/limit/reservation checks using existing string ownership and UTF-8 support.
2. Transfer through these imports; compare complete file-v1 records, prior output, stopping, and close precedence against four executors.
3. Compare byte-position-based response injection, zero/exact/+1/over-4096 cases, split UTF-8, evaluation order, short circuits, recursion, copying/release, and allocation failures.
4. Run examples/files/roundtrip.ceru as generated WAT and update Japanese/English instructions and support tables.

Provide a real OS host only after separately validating scope, exclusive creation, and link rejection. Ordinary Node path-based open alone does not establish the existing OS contract.
