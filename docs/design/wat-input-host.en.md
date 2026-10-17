# WAT line-input host contract v1

[日本語](wat-input-host.ja.md)

Connect the [shared line-input contract](line-input.en.md) to WAT. The host supplies an execution connection, not an observation API. Internal memory, string addresses, and ownership metadata remain private.

## Imports and compatibility

Only artifacts containing input instructions require these imports. Unused functions are retained, so importing std:io alone also requires them.

| cerune import | Type | Meaning |
| --- | --- | --- |
| stdin_status_v1 | () → i32 | Non-consuming connection check: 0=ready, 1=unconfigured, 2=unsupported connection, 3=connection failure |
| read_stdin_byte_v1 | () → (i32 byte, i32 status) | Request one byte: status 0=one byte, 1=EOF, 2=interrupted, 3=read failure |

Successful byte values are 0–255; every other status requires byte=0. Invalid bytes or unknown statuses produce invalid-response with prior confirmed consumption and consumption=unknown. Unknown connection statuses also produce invalid-response. Failures and interruptions consume no bytes. Only interruptions retry; no read-ahead, rewind, or implicit flush occurs. Keep the connection stable during instance execution and do not reenter main from a host call.

Artifacts without input still work with existing hosts. Input artifacts require both imports and fail instantiation when either is missing. There is no compatibility fallback treating unconfigured input as EOF. A new host reports unconfigured input by returning 1 from stdin_status_v1. Existing write_stdout_v1, numeric formatting, flush_stdout_v1, and write_error_byte remain unchanged.

## Generated WAT responsibilities

Check limit, connection, known EOF, budget/allocation, reads, then UTF-8. Reserve the maximum using the existing string allocator; on success shrink the same allocation's length and accounted bytes to the actual length. Release on failure or empty EOF. Physical allocation failure, including memory.grow failure, reports allocation-failed.

Failures emit an io-v1 reason, limit, consumption, and NodeId/Span through write_error_byte and stop with unreachable. Diagnostics exclude input text. Shared records of successful generated operations are not added here. Host exceptions that never return to main and diagnostic delivery failures remain host failures, never successful language input results.

EOF is retained within the instance. Resuming a failed instance is outside the public contract; an input-failure latch prevents subsequent reads. Observation does not provide response replacement.

## Validation

Supply identical input to the executors, C, LLVM, QBE, and WAT, comparing line/EOF, output, failure reason, consumption, and origins. Separate actual file/pipe tests from injected read failures, interruptions, invalid responses, and allocation failure. Also verify rejection of old hosts, compatibility of input-free artifacts, private memory, and no reads after failure. The test host is not a general WASI runtime or standard library.

## Runnable fixture with the test host

Use the [source](../../tests/fixtures/line_input.ceru) and [input file](../../tests/fixtures/line_input.stdin) to compare [expected output](../../tests/fixtures/line_input.stdout). Use WABT 1.0.39 and Node.js 20. The test host reads synchronous files/pipes and rejects Windows consoles with unsupported-connection.

~~~sh
cargo run -- emit-wat tests/fixtures/line_input.ceru -o target/line-input.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm target/line-input.wat -o target/line-input.wasm
node tests/support/run_wasm_input.cjs target/line-input.wasm < tests/fixtures/line_input.stdin
~~~

In Windows PowerShell, run the last line as `cmd /c "node tests/support/run_wasm_input.cjs target/line-input.wasm < tests/fixtures/line_input.stdin"` to avoid text-pipeline conversion. [Route comparisons](../../tests/stdin_routes.rs) and [host contract tests](../../tests/support/wat_input.rs) cover this fixture and failures.
