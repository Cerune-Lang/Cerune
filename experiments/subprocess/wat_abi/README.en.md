# WAT subprocess ABI probe

[日本語](README.md)

Verify [eight versioned imports](../../../docs/design/wat-process-host.en.md) using Wasmi and the existing Rust common runtime. host.rs transports requests and checks call order; the existing C ABI owns prepare/execute, failure selection, Report and retained resources. No C generation or compilation is involved.

~~~sh
cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host --test process_protocol
~~~

Targets are Windows x86-64 MSVC / Linux x86-64 GNU. Enable process-c-abi only as a test dependency of the existing WAT host crate; add no execution route to the regular host or Cerune. Existing Windows/Linux WAT CI also runs this test.

| Check | Method |
| --- | --- |
| Import types and old hosts | Actually instantiate probe.wat; missing required imports fail |
| Bodies and numbers | All 256 bytes, split UTF-8, NUL/CR/LF/combining characters, u64 maximum and origins beyond 2^53 |
| Order and lengths | Reject duplicates, missing/excess data, wrong order, repeated prepare/execute and invalid UTF-8 |
| Common failure selection | InvalidRequest/RequestLimit precede Unconfigured; reservation failures yield OutputBudget/AllocationFailed without launch |
| Observation and release | Repeated/reverse Report reads, empty/output-range copy, cancellation, partial-request discard and subsequent operations |

Probe exports are never added to generated products. No memory is exposed; bodies use at most eight bytes per value. Configured tests have no registered targets and launch neither children nor a Linux supervisor.

[Generated-WAT comparisons](../../wat_host/tests/process_generated.rs) reuse host.rs for existing .ceru examples, array reservation/release, located diagnostics and real Windows/Linux child I/O. They are separate from the independent ABI tests above. **Remaining:** reference-host CLI connection JSON and generated-WAT timeout/cleanup-failure/other-I/O comparisons.
