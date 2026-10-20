# WAT program-argument host contract v1

[日本語](wat-arguments-host.ja.md)

Connect the [common argument contract](dogfooding-inputs.en.md) to WAT. The host explicitly supplies execution input; generated WAT validates, copies, retrieves and owns values. Internal memory, string addresses and owner metadata remain private.

## Imports and compatibility

Artifacts containing argument operations require these imports. Unused functions are retained, so importing std:env alone also requires them.

| cerune import | Type | Successful result |
| --- | --- | --- |
| arguments_count_v1 | () → (i64 count, i32 status) | Number of program arguments, excluding executable names and similar metadata |
| argument_length_v1 | (i64 index) → (i64 length, i32 status) | UTF-8 byte length; zero for a present empty value |
| argument_byte_v1 | (i64 index, i64 offset) → (i32 byte, i32 status) | One byte, from 0 through 255 |

Status 0 means success and 2 means connection failure. Only count permits status 1 with count 0 to represent an unconfigured connection. Other statuses, negative counts/lengths and out-of-range bytes produce invalid-response. Value fields for status 2 are unused. Requests use indices and offsets within the acquired count and lengths. There is no EOF or interruption retry protocol.

Hosts freeze input before startup and return stable responses to repeated startup reads. Do not normalize or split contents again. Hosts accepting JavaScript strings must reject invalid UTF-16 instead of replacing it. Mutating original arrays or Buffers must not affect the snapshot. Semantics are not guaranteed for hosts that violate the contract by changing values during acquisition. Host exceptions remain host failures, never empty or missing values.

Argument-free artifacts retain their existing imports. Argument artifacts require all three imports plus existing flush_stdout_v1 and write_error_byte; missing imports fail instantiation. Unconfigured input is distinct from an empty list and is diagnosed with source origin at the actual count/get operation. Existing output and stdin contracts remain unchanged.

## Generated WAT processing

1. Check the connection and count at main startup.
2. Validate limits, count, then each argument's cumulative byte count, UTF-8 and NUL in common order. UTF-8 transition rules are shared with stdin.
3. After validating every entry, reserve and copy the table and contents into private memory. Validation and copying read the frozen host input multiple times. Never publish a partial snapshot.
4. Start Cerune execution. Count/get read only the private snapshot, never the host. Retrieval copies into the existing string heap, released by ordinary ownership lowering. Empty values allocate nothing.

`emit-wat --argument-count-limit` / `--argument-byte-limit` emit visible WAT constants. Defaults are 4096 entries and 1048576 total UTF-8 bytes, including CR/LF and excluding terminators/bookkeeping. Rust exposes emit_wat_with_argument_limits; existing emit_wat retains defaults. Emission neither reads host input nor accepts execution arguments after the delimiter.

Snapshot and returned-string limits are separate. They physically share private linear memory, but snapshot bytes do not count as live returned strings. wasm32 address exhaustion or failed memory.grow produces allocation-failed. Main on an argument-using instance runs only once, including after failure. Create a new instance to run again. Discarding the instance reclaims snapshot storage.

## Diagnostics and remaining work

Startup reports arguments-v1 phase=capture with a reason and applicable index, without contents or fabricated source positions. Connection failures use connection-failed; invalid responses use invalid-response. Execution reports phase=execute with operation, reason, index and NodeId/SourceId/Span, passes prior output to the existing flush connection and stops with unreachable. Diagnostic delivery errors and host exceptions remain host failures.

Preserve original IR and expose argument operations, origins, startup validation, limits, copying and ownership in generated artifacts. Chronological success records remain unimplemented. This first scope covers WAT main execution; WAT library restrictions for pure exported functions remain in place. No WASI, OS argument connection or stable embedding ABI is added. Native arguments are also implemented; integer parsing follows separately.

## Validation and example

The [argument example](../../examples/arguments/README.en.md) and [comparisons](../../tests/program_arguments/wat.rs) verify output, stop reasons, origins and prior output against four executors, plus ownership/release, unconfigured/empty/missing states, invalid input, limits, allocation failures, old-host rejection, private memory and stdin coexistence. The test host checks original-input mutation and invalid UTF-16, and rejects argument reads after ordinary output begins. Windows, Linux and macOS WAT CI require these tests.

The [test host](../../tests/support/run_wasm_input.cjs) explicitly injects JSON from CERUNE_TEST_WAT_ARGUMENTS. Each values entry is a string or UTF-8 byte array. This is test-host configuration, not language-level environment-variable discovery. Omitting it adds no argument imports, allowing old-host compatibility checks.

For large test input, save the same JSON in a file and specify its path with CERUNE_TEST_WAT_ARGUMENTS_FILE. Supplying both settings is rejected. This avoids Windows environment-variable length limits while testing page growth, physical allocation failure and separate returned-string limits.
