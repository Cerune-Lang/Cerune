# Bounded UTF-8 line input

[日本語](line-input.ja.md)

This specifies line input for [standard I/O](standard-io.en.md). The initial language API returns a line or EOF and stops with origin-bearing diagnostics on failure. Reading results remain separate from the stopping policy to allow a future recoverable API.

## Implementation scope

The [virtual std:io module](standard-modules.en.md) and common IR input operation are implemented. HIR/MIR/SSA/VM accept explicit input connections through Rust run_with_io APIs. [CLI and generated C/LLVM/QBE/Native stdin bindings](stdin-connections.en.md) are also implemented. [Versioned WAT input hosts](wat-input-host.en.md) are also implemented. Unconfigured input stops with a diagnostic rather than EOF.

Existing enums can represent a string-bearing line or EOF. A blank line contains LF or CRLF. No generic Result type is required. The public API is `import "std:io" as io;` and `io::read_line(max_bytes)`, returning `io::ReadLine`. The Rust connection API below is an execution component.

## Bytes and termination

Input must be strict UTF-8. Preserve Japanese, NUL, standalone CR, and combining characters without normalization, replacement, or trimming. LF delimits a line; CR alone does not. The byte limit includes LF and any preceding CR.

| Input / limit | Result | Bytes consumed by this operation |
| --- | --- | --- |
| Empty stream / 3 | EOF | 0 |
| LF / 1 | Blank line containing LF | 1 |
| CRLF / 2 | Blank line containing CRLF | 2 |
| abc EOF / 3 | Unterminated final line once, then EOF | 3 |
| abc LF / 4 | Complete line | 4 |
| abc LF / 3 | Limit exceeded | 4 |
| abcd followed by more / 3 | Limit exceeded; do not drain the remainder | 4 |
| CR EOF / 1 | Final line containing CR | 1 |

Initially request one byte at a time. A full buffer without LF requires one additional request to distinguish EOF from overflow. Consumption can reach limit + 1. Count that extra byte on overflow; never pretend to rewind it. A line ending in LF exactly at the limit needs no additional request.

Once EOF is confirmed, retain that state and do not access the OS/reader on subsequent calls, including after an unterminated final line. Restarting the stream or clearing EOF is outside the initial API.

## Validation, budgets, and failure order

Create a connection per execution. A failed connection returns its original failure without validating new arguments, reading, or adding records. A new operation follows this order:

1. Evaluate the limit once. The common range is 1 through 2147483647 bytes; reject zero, negatives, and larger values.
2. Require an input connection. Unconfigured input is not EOF.
3. If EOF is already known, return EOF without reading or reserving storage.
4. Reserve the whole requested line limit against the same byte limit as live dynamic strings. Budget exhaustion and physical allocation failure stop before reading.
5. Read through LF, EOF, overflow, or a read failure. Retry interruptions, which consume no bytes; do not retry other failures.
6. Strictly validate UTF-8 only after framing a complete line. Transfer ownership into an immutable string and reduce the accounted bytes to actual length. Release scratch storage on failure or empty EOF.

Framing precedes UTF-8 validation: an oversized invalid line reports limit-exceeded, and a read failure during an incomplete line reports read-failed. Failure while checking EOF after the last byte does not return a successful partial line.

Attempt [optional compaction](input-compaction.en.md) after completion; keep the original buffer on failure. Rust temporarily copies the actual line length when shrinking. Accounted string lengths differ from physical capacity and allocator metadata. Lines outside the compaction conditions, failed compaction, and WAT retain the reserved capacity, so this budget does not guarantee a physical memory limit. A large requested limit can exhaust the budget even for a short actual line. Unknown EOF still requires reservation; known EOF does not.

The Rust component's read_line checks the caller-supplied available_bytes budget. The four executors use StringHeap's remaining budget, reserving the full limit and ownership metadata before reading. Success transfers the compacted or original buffer and counts its length toward the memory limit; EOF/failure releases the reservation. Exclusive heap borrowing prevents intervening string allocations. Existing retain/release operations manage copied values and final release.

Failures have distinct codes. Budget exhaustion is not reported as an OS read failure.

| Code | Condition |
| --- | --- |
| invalid-limit | Limit outside the common range |
| unsupported-connection | Actual connection unsupported in the initial scope, such as Windows console |
| unconfigured | No input connection |
| budget-exceeded | Insufficient logical reservation budget |
| allocation-failed | Cannot physically reserve scratch storage |
| read-failed | Reader error other than interruption |
| invalid-response | Connection contract violation, such as returning more than one for a one-byte request |
| invalid-utf8 | Completed line is not strict UTF-8 |
| limit-exceeded | Consumed limit + 1 bytes, confirming overflow |

## Connection, stopping, and recovery responsibilities

Internal reading returns a line, EOF, or classified failure with consumption metadata. Input applies the stopping policy, retaining the first failure and never using the connection afterward. A future separate API can reuse internal results, but must specify resumption positions, explicit draining, and budgets. Observation does not expose an Input reset operation.

The reader must return zero (EOF) or one for a one-byte request; errors and interruptions consume no bytes. It must not secretly consume beyond the request. Invalid counts retain the previously confirmed consumption and mark consumption by that response as unknown. Do not invent an exact count or a rewind.

Synchronous Windows/Linux pipe/file stdin is implemented first. Windows console stops with a dedicated diagnostic; character conversion follows later. Never infer the generated target from the host OS. WAT uses a [versioned value/status input contract](wat-input-host.en.md) without exposing internal memory. Reading does not flush output implicitly.

## Diagnostics and observation

Failure records are ASCII and exclude input text, OS error messages, and paths. Input adds its own operation fields without changing existing write/flush diagnostics.

~~~text
io-v1 operation=read-line code=limit-exceeded limit=3 consumed=4 consumption=known node=7 file=2 bytes=10..22
~~~

Omit file for anonymous sources. bytes identifies the source Span, not the input text. consumed counts confirmed bytes for this operation. Invalid responses use consumption=unknown; other cases use known. Sequence numbers and individual consumption counts identify successive operations.

Events retain sequence, NodeId/Span, limit, confirmed consumption, completeness of consumption, line/EOF/failure, read-call count, and interruption count. read_calls includes EOF checks and interrupted requests; generated routes need not have identical OS call counts. Observation APIs return immutable references. Recording capacity or allocation failure sets truncated without changing execution. Do not record contents or read via observation callbacks.

## Failure tests and route completion criteria

The [shared connection tests](../../tests/line_input.rs) establish these inputs and states first. Unit tests inject physical reservation failure without huge allocations or relying on OS behavior.

| Area | Required expectations |
| --- | --- |
| LF, CRLF, CR, NUL, Japanese, combining characters | Preserved bytes, blank lines distinct from EOF, no normalization |
| Exact/adjacent limits, final line, EOF | Exact consumption, one extra probe, no reading after EOF |
| Invalid UTF-8, mid-read errors, interruptions | Error precedence, no partial successful line, retry interruptions only |
| Zero, negative, excessive limit, budget/allocation failure | Zero reads, separate codes, retained origin |
| Invalid counts, calls after failure | Unknown consumption explicit, original failure retained, no further effects |
| Disabled, sufficient, and truncated observation | Identical results, consumption, and requests |
| Multiple source files and secret input | Correct NodeId/Span; no input in diagnostics |

After integration, give each route independent identical input and responses. Do not reuse a stream consumed by another route.

| Route | Comparison | Current state |
| --- | --- | --- |
| HIR, MIR, SSA, VM | Line/EOF, prior output, failure reason/origin, consumption, ownership/release | Implemented and compared through Rust APIs |
| C / LLVM / QBE | Identical input, prior output/diagnostics, ownership, actual file/pipe, C/LLVM at -O0/-O2 | Implemented |
| WAT | Same input, requests/responses, prior output/diagnostics, no later execution | Implemented and compared with versioned hosts |
| LIR, ASM, native Object (ordinary/SSA) | Input origin, explicit target, same failures and prior output | Implemented and compared on Windows/Linux |
| CLI / actual OS | Print then flush the prompt before input; failed flush means zero reads | Windows/Linux pipe/file implemented; Windows console unsupported |

A [real stdin example](../../examples/stdin/README.en.md) is available. Do not describe the existing scripted input_flow example as actual stdin support. Keep observation separate from supplying input through execution connections.

## Retained-capacity measurements

See [measurements and improvement candidates](input-memory.en.md), separating accounted bytes, retained capacity, and OS memory metrics. Short lines with large limits retain excess capacity. The string memory limit is not a physical-memory bound.
