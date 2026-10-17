# Line-input retained capacity and memory limits

[日本語](input-memory.ja.md)

Measurements and shrinking prototype for #154 / #133 on 2026-10-18, against the implementation merged in PR #164 (`12ddad3`). The tables below record the pre-compaction baseline. See [input compaction](input-compaction.en.md) for the current implementation.

## What the memory budget counts

The memory budget limits accounted bytes of dynamic strings created by input or concat. Accounted usage sums the lengths of live owned strings. Copies sharing the same owner are counted once; final release subtracts the string length.

A line read reserves its entire requested limit before reading, then counts only the resulting length. Reading `x + LF` with a 1 MiB limit leaves two accounted bytes. Allocation capacity and OS memory metrics are separate.

## Findings

**Short lines retain their reservation capacity, exceeding accounted bytes.** Measurements of the shared Rust runtime and generated C on Windows/Linux retained eight two-byte lines.

| Per-line limit | Total length / accounted bytes | Rust total capacity | Live generated-C malloc requests |
| --- | ---: | ---: | ---: |
| 2 bytes | 16 | 16 | 208 |
| 1 MiB | 16 | 8,388,608 | 8,388,800 |

C includes 24 bytes of ownership metadata per line. These are requested sizes, excluding allocator rounding and metadata. Rust capacity excludes ownership metadata and HashMap storage.

Three retain/release cycles each returned owner count, accounted bytes, and tracked live allocations to zero. This case does not show a leak; it does not prove general leak freedom or bound whole-process memory.

## Separate OS measurements

The generated-C probe records startup, retention, and release after compilation. These are measured values for the first cycle with a 1 MiB limit and `-O2`. They include probe printing and OS queries, and are informational, never CI thresholds.

| Environment / metric | Before | Eight retained | Released |
| --- | ---: | ---: | ---: |
| Windows working set (bytes) | 4,001,792 | 4,038,656 | 4,005,888 |
| Windows private commit (bytes) | 561,152 | 8,998,912 | 561,152 |
| Linux/WSL VmSize (kB) | 10,548 | 18,772 | 10,548 |
| Linux/WSL VmRSS (kB) | 580 | 640 | 612 |

Measurements use x86-64. Windows: 10.0.26100, Clang 22.1.8. Linux: WSL `4.4.0-26100-Microsoft`, GCC 9.4.0. Rust: 1.98.0. C allocation, accounting, and release checks also passed at `-O0`.

Windows private commit is PrivateUsage in [PROCESS_MEMORY_COUNTERS_EX](https://learn.microsoft.com/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex). It is not interchangeable with [Linux /proc metrics](https://www.kernel.org/doc/html/v6.1/filesystems/proc.html). This WSL environment returned zero VmHWM/VmData; those are not reported as measured peak RSS or commit. Additional native-Linux CI measurements are recorded below. An 8 MiB capacity increase does not imply 8 MiB of additional physical RAM.

## Coverage by route

| Route | Evidence in this change |
| --- | --- |
| HIR / MIR / SSA / VM | Unit measurements of shared StringHeap length, capacity, accounting, and release; identical source example executed through each executor |
| C | Real emitted input/ownership functions called from a probe main; malloc/free requests, live owners, accounted bytes, and OS metrics recorded |
| LLVM / QBE / ASM / self-encoded Object | Malloc/free references in objects generated from the same source instrumented; ASM and Object output/allocation/release measured separately |
| WAT | Live/free capacity and pages measured; full release and reuse across three cycles verified |

[retained_lines.ceru](../../examples/stdin/retained_lines.ceru) reads two lines before displaying both. Tests compare 64-byte and 1 MiB limits with a heap budget of “limit + the first line's two bytes,” allowing the second read. This also detects an unintended switch to counting allocation capacity toward the limit.

The C probe main does not measure standard-module resolution or enum lowering; the source-level route comparison covers those. Probe code stays in tests, without a product observation API or internal-memory export.

## Additional generated-route measurements

The same Cerune source retains eight two-byte lines, prints them, and releases them when leaving the block, repeated three times. Plain and instrumented executions produce identical output.

| Route (Windows/Linux x86-64) | Results with a 1 MiB line limit |
| --- | --- |
| C / LLVM / QBE | Peak live malloc requests: 8,388,800 bytes. 24 allocations / 24 frees, zero live bytes at exit |
| MIR/SSA-derived ASM / self-encoded Object | Same results; external assembly and directly linked self-encoded objects measured separately |
| WAT | Live capacity 8,388,608 bytes, accounted usage 16 bytes. After full release: live capacity zero, free capacity 8,388,608 bytes |
| WAT repetition | 129 pages at first full retention (64 KiB per page). Pages remain 129 after release; cycles two and three reuse storage without growth |

With a two-byte limit, malloc-based routes peak at 208 bytes. WAT has eight bytes of payload capacity per line due to alignment, 64 total, within one memory page. WAT capacity totals exclude ownership headers; page counts include static data and other storage.

For malloc-based routes, objcopy redirects only malloc/free references in test objects to measurement functions. The external tool handles instructions and relocations. OS metrics are saved at new live-request peaks; this does not measure all allocator-internal memory.

Instrumented WAT scans its ownership list and passes usage, live/free capacity, owner count, and page count as values to the test host. Memory is never exported. Snapshots occur after successful reads and final releases; this is not a product observation API.

## Optional-shrinking prototype

Only a test copy of generated C adds shrinking after UTF-8 validation and before owner-list registration. Failed realloc keeps the original allocation. At that stage, product emitters and runtimes were unchanged.

| Mode | Shrinks across 24 lines | Peak requested storage | Output / release |
| --- | --- | ---: | --- |
| Normal realloc | 24 successful | 1,048,782 bytes | Matches baseline; zero at exit |
| Always fail shrinking | 0 successful | 8,388,800 bytes | Matches baseline; zero at exit |
| Always copy to another allocation | 24 successful | Tracked storage 1,048,782 bytes; including temporary copy 1,048,808 bytes | Eight distinct line contents preserved; zero at exit |

These checks passed on Windows/Linux. Approximately 1 MiB remains at peak because the next read still reserves its full limit before consuming input. Internal temporary allocations made by normal realloc are unmeasured.

The prototype showed that C can tolerate shrinking failure without a new input failure. The current input-compaction design records the adopted conditions, implementation by route, and failure tests.

## Alternatives and decision

| Alternative | Benefit | Contract implications / limits |
| --- | --- | --- |
| Incremental growth while reading | Lower allocations for short lines | Allocation can fail after consuming input, changing the current pre-read allocation-failure contract |
| Mandatory shrinking on completion | Less excess capacity in successful strings | Reallocation for shrinking may introduce a new failure after reading |
| Optional shrinking on completion | Preserve results/accounting by keeping original storage on failure | No memory bound; copying, temporary peaks, ownership transfer, and WAT reuse need separate validation |
| Count allocation capacity toward the memory limit | Account for retained capacity | Changes where the same source exceeds its budget; allocator-dependent sizes would break route consistency |
| Separate host memory limit | Can cover metadata, output, compiler memory, and more | OS/host specific; needs separate configuration, termination, and diagnostic design rather than treating it as an ordinary io-v1 failure |

**Rust/C/LLVM/QBE/Native now compact completed input while retaining pre-read reservations.** See the [current design](input-compaction.en.md) for conditions, failure handling, and why WAT does not split blocks.

The #154 WAT decision is to keep existing reuse, based on the [policy comparison](wat-input-storage.en.md). This study excludes #133 array/output copies/compiler peaks and the cost of one-byte read calls.

## Reproduce with the current implementation

~~~sh
cargo test --lib retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_lines_example -- --nocapture
cargo test --test stdin_routes input_memory_routes -- --nocapture
~~~

Set `CERUNE_TEST_CC` explicitly for C measurement (Windows: clang; Linux: cc). Route comparisons also require valid `CERUNE_TEST_LLVM_CLANG`, `CERUNE_TEST_ASM_CLANG`, `CERUNE_TEST_QBE`, `CERUNE_TEST_NODE`, and `CERUNE_TEST_WAT2WASM_JS`. Existing helpers skip unavailable unconfigured tools; an explicitly configured missing tool fails.

Generated probe C and executables are in `target/stdin-input-memory-<pid>`. The isolated C probe runs three cycles of eight lines, at most 1 MiB reserved per line and 48 bytes read. Route comparisons also exercise both sides of the compaction conditions, reading 24 lines of at most 4097 bytes. Expected live allocations now reflect compaction and differ from the historical tables above. Preserve the separate [input contract](line-input.en.md).

Additional probes require `CERUNE_TEST_OBJCOPY` (Windows: llvm-objcopy; Linux: objcopy), also required in CI. Reports are saved to `target/stdin-input-memory-routes-<pid>/*-metrics.txt`, WAT to `target/stdin-input-memory-wat-<pid>/*-metrics.json`. Normal, failed, and moving compaction produce `*-normal-metrics.txt`, `*-fail-metrics.txt`, and `*-move-metrics.txt`. The former C prototype reports remain in the PR #166 CI artifact.

## Native-Linux CI measurements

The linux-cross-io-observations artifact from [CI run 39075433193](https://github.com/Cerune-Lang/Cerune/actions/runs/39075433193) was inspected. Baseline LLVM with eight 1 MiB reservations reported VmSize 11,872 kB, VmRSS 2,000 kB, and 8,388,800 bytes of live malloc requests. The normal C shrinking prototype reported VmSize 4,704 kB, VmRSS 2,000 kB, and peak requests of 1,048,782 bytes. These are informational values from separate processes; capacity and RSS measure different quantities. Allocation/release/output checks passed.
