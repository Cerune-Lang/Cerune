# Line-input retained capacity and logical budgets

[日本語](input-memory.ja.md)

Initial measurements for #154 / #133 on 2026-10-18, against the implementation merged in PR #164 (`12ddad3`). Input behavior, budgets, and failure semantics are unchanged.

## Findings

**Short lines retain their reservation capacity, exceeding logical charges.** Measurements of the shared Rust runtime and generated C on Windows/Linux retained eight two-byte lines.

| Per-line limit | Total length / logical charge | Rust total capacity | Live generated-C malloc requests |
| --- | ---: | ---: | ---: |
| 2 bytes | 16 | 16 | 208 |
| 1 MiB | 16 | 8,388,608 | 8,388,800 |

C includes 24 bytes of ownership metadata per line. These are requested sizes, excluding allocator rounding and metadata. Rust capacity excludes ownership metadata and HashMap storage.

Three retain/release cycles each returned owner count, logical charge, and tracked live allocations to zero. This case does not show a leak; it does not prove general leak freedom or bound whole-process memory.

## Separate OS measurements

The generated-C probe records startup, retention, and release after compilation. These are measured values for the first cycle with a 1 MiB limit and `-O2`. They include probe printing and OS queries, and are informational, never CI thresholds.

| Environment / metric | Before | Eight retained | Released |
| --- | ---: | ---: | ---: |
| Windows working set (bytes) | 4,001,792 | 4,038,656 | 4,005,888 |
| Windows private commit (bytes) | 561,152 | 8,998,912 | 561,152 |
| Linux/WSL VmSize (kB) | 10,548 | 18,772 | 10,548 |
| Linux/WSL VmRSS (kB) | 580 | 640 | 612 |

Measurements use x86-64. Windows: 10.0.26100, Clang 22.1.8. Linux: WSL `4.4.0-26100-Microsoft`, GCC 9.4.0. Rust: 1.98.0. C allocation, accounting, and release checks also passed at `-O0`.

Windows private commit is PrivateUsage in [PROCESS_MEMORY_COUNTERS_EX](https://learn.microsoft.com/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex). It is not interchangeable with [Linux /proc metrics](https://www.kernel.org/doc/html/v6.1/filesystems/proc.html). This WSL environment returned zero VmHWM/VmData; those are not reported as measured peak RSS or commit. Additional measurements on native Linux remain. An 8 MiB capacity increase does not imply 8 MiB of additional physical RAM.

## Coverage by route

| Route | Evidence in this change |
| --- | --- |
| HIR / MIR / SSA / VM | Unit measurements of shared StringHeap length, capacity, accounting, and release; identical source example executed through each executor |
| C | Real emitted input/ownership functions called from a probe main; malloc/free requests, live owners, logical charges, and OS metrics recorded |
| LLVM / QBE / ASM / self-encoded Object | Example output and logical budgets compared; source inspection shows transfer of limit + 24 storage. Individual capacity / OS measurements remain |
| WAT | Same example output/budget comparison; source inspection confirms full-limit allocation followed by length-only accounting. Retained storage, free lists, and linear-memory pages remain unmeasured |

[retained_lines.ceru](../../examples/stdin/retained_lines.ceru) reads two lines before displaying both. Tests compare 64-byte and 1 MiB limits with a heap budget of “limit + the first line's two bytes,” allowing the second read. This also detects an unintended switch to capacity-based charging.

The C probe main does not measure standard-module resolution or enum lowering; the source-level route comparison covers those. Probe code stays in tests, without a product observation API or internal-memory export.

## Alternatives and decision

| Alternative | Benefit | Contract implications / limits |
| --- | --- | --- |
| Incremental growth while reading | Lower allocations for short lines | Allocation can fail after consuming input, changing the current pre-read allocation-failure contract |
| Mandatory shrinking on completion | Less excess capacity in successful strings | Reallocation for shrinking may introduce a new failure after reading |
| Optional shrinking on completion | Preserve results/accounting by keeping original storage on failure | No memory bound; copying, temporary peaks, ownership transfer, and WAT reuse need separate validation |
| Charge capacity to the logical budget | Account for retained capacity | Changes where the same source exceeds its budget; allocator-dependent sizes would break route consistency |
| Separate host memory limit | Can cover metadata, output, compiler memory, and more | OS/host specific; needs separate configuration, termination, and diagnostic design rather than treating it as an ordinary io-v1 failure |

**Keep logical budgets and pre-read reservations for now. Optional shrinking on completion is the next candidate to evaluate.** It is neither adopted nor implemented. Review the current promise of no additional allocation on success, and verify that injected shrinking failures preserve results, consumption, and subsequent output before deciding.

#154 remains open: LLVM/QBE/Native/WAT capacity measurements, native-Linux OS metrics, and a shrinking prototype with failure/ownership comparisons remain. This study excludes #133 array/output copies/compiler peaks and the cost of one-byte read calls.

## Reproduce

~~~sh
cargo test --lib retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_lines_example -- --nocapture
~~~

Set `CERUNE_TEST_CC` explicitly for C measurement (Windows: clang; Linux: cc). Route comparisons also require valid `CERUNE_TEST_LLVM_CLANG`, `CERUNE_TEST_ASM_CLANG`, `CERUNE_TEST_QBE`, `CERUNE_TEST_NODE`, and `CERUNE_TEST_WAT2WASM_JS`. Existing helpers skip unavailable unconfigured tools; an explicitly configured missing tool fails.

Generated probe C and executables are in `target/stdin-input-memory-<pid>`. Each run is finite: three cycles, eight lines, at most 1 MiB per line, and 48 bytes read. Live-allocation assertions record today's implementation baseline and should be intentionally updated when shrinking is implemented. Preserve the separate [input contract](line-input.en.md).
