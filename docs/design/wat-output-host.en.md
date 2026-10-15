# WAT output host contract v1

[日本語](wat-output-host.ja.md)

Generated WAT manages ordinary output counts, retries, and termination. Hosts exchange byte values and responses; internal memory is neither imported nor exported. The only exported execution function remains `main`.

## Writes and flushing

Artifacts import the following functions from `cerune` only when needed.

| Import | Purpose |
| --- | --- |
| `write_stdout_v1(bits: i64, count: i32) -> i32` | Ordinary output: count bytes (1–8), starting at the least significant byte |
| `flush_stdout_v1() -> i32` | Existing explicit-flush contract: zero succeeds, nonzero fails |
| `write_error_byte(byte: i32)` | Existing ASCII diagnostic delivery |

Write responses have distinct meanings.

| Response | Meaning and generated WAT behavior |
| --- | --- |
| 1–count | Accept that prefix; send only the remainder |
| 0 | Stop for zero progress |
| -1 | Stop for a write error |
| -2 | Interrupted with no bytes accepted in this call (EINTR equivalent); retry the same remainder |
| Above count or other negative values | Stop for an invalid acceptance response |

At most eight bytes per host call allow partial writes without exposing mutable memory or internal addresses. OS-call boundaries need not match other routes. Hosts interpret the signed i64 as bits, ignore bits beyond count, and preserve encoding and line endings.

Requested/accepted counts cover **one common-IR output operation**. This includes numbers, Booleans, strings, and aggregate display; arrays still expand into the existing sequence of IR operations. Failure emits the following diagnostic and traps without later execution, retransmission, or additional flushing. Accepted prefixes, including partial UTF-8, are not rolled back.

```text
cerune: io-v1 operation=write requested=N accepted=M node=N[ file=F] bytes=S..E
```

Brackets denote the optional multi-file field and are not printed. Generated code distinguishes response classes without adding OS-specific numbers to the existing `io-v1` format. Acceptance does not guarantee display or durability.

## Formatting and exact byte counts

WAT formats integers and Booleans internally. Strings use existing storage; quoted strings first count the escaped length. No whole-output heap copy is introduced.

Only artifacts displaying the corresponding floating-point type require these formatting imports.

| Import | Policy |
| --- | --- |
| `format_f32_v1(value: f32, offset: i32) -> (i64, i32)` | Nine significant digits |
| `format_f64_v1(value: f64, offset: i32) -> (i64, i32)` | Seventeen significant digits |

The result is `(bits, length)`. Length is the **complete formatted length without a newline** (1–128 bytes). Bits contain up to eight bytes starting at offset, least significant byte first. Offsets advance through 0, 8, 16… below length; unused final bits are ignored.

Formatting returns the same result for the same value and performs no output, flush, or executor reentry. Follow the [existing numeric display rules](../reference/language.en.md#output) without locale-dependent separators. WAT snapshots every chunk into a fixed, private 128-byte buffer, checking consistent lengths and printable ASCII before any write. The requested count is the completed length plus any trailing LF; accepted counts come from the write connection.

Negative, zero, or excessive lengths, inconsistent lengths, and invalid text stop with an origin-bearing `cerune: output formatting failed` diagnostic. Do not invent counts for unformatted text. WAT does not prove the host's formatting computation correct; hosts remain responsible for numeric display semantics.

## Migration and compatibility

New ordinary output no longer imports legacy `print_*`, `write_i64/u64/f32/f64`, or `write_byte`. Existing hosts need updating.

| Host and artifact | Behavior |
| --- | --- |
| New artifact, old host | Missing required imports fail instantiation; no legacy fallback |
| New artifact, updated host | Execute with the new response contract |
| Old artifact | Retains its old ABI; an updated host must retain legacy imports to run it |
| Flush only / no output | No ordinary-write or formatting imports required |
| No float display | No float-formatting imports required |

The repository's common host is `tests/support/run_wasm.cjs`, used by string, dynamic-array, multi-file, runtime-diagnostic, and flush tests. Legacy imports remain for old diagnostic fixtures; they never turn new output failures into success. External hosts must add the new imports before adopting new artifacts.

Hosts translate OS errors into the responses above. Exceptions thrown by the host itself are outside the ordinary write-failure contract; acceptance counts are not guessed. Diagnostic delivery is best effort. Synchronous reentry and concurrent execution of one instance are unsupported; do not reuse a stopped instance.

## Observation and verification

WAT `Output { kind, newline, origin }` retains the path to generated `cerune_output_*` counting, formatting, retry, and termination. Selecting an execution host is separate from reading observations; no observation API replaces connection responses. Shared generated-route records including successful operations remain future work.

[Comparison tests](../../tests/wat_output.rs) use the same controlled responses as four executors to check partial writes, interruption, zero progress, invalid responses, partial failure, origins, and independence from observation capacity. They also cover old-host rejection, unused imports, formatting failure, partial UTF-8, NUL/CR/LF, long strings, and float extremes. [output_values.ceru](../../examples/io_protocol/output_values.ceru) runs unchanged against its existing expected output. The test host rounds integer ratios to even and is no longer restricted to a small set of exact finite values.
