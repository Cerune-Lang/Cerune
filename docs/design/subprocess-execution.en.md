# Subprocess common IR and four executors

[日本語](subprocess-execution.ja.md)

`run` from `std:process/run` is connected to HIR, MIR, SSA, and the Bytecode VM, using [existing types](subprocess-values.en.md) and the [common runtime](subprocesses.en.md).

## Public API and support

Use `import "std:process" as types;` for types and `import "std:process/run" as process;` for `process::run(request)`. The result is the same `types::Outcome`.

| Component | Current behavior |
| --- | --- |
| HIR, MIR, SSA, VM | Execute the common `process.run` operation |
| Ordinary CLI run commands | [Explicit configuration](subprocess-cli.en.md) connects Windows/Linux; without it, return `Unconfigured` |
| Each Rust executor | `run_with_processes(program, &mut Processes<B>, transfer_bytes)` explicitly supplies connection and capacity |
| Generated C | Windows/Linux execution with [explicit target and runtime linking](subprocess-c.en.md) |
| LLVM, QBE, WAT, Native | Reject IR containing the execution module with a source diagnostic; existing type-only programs still emit |
| Remaining generated execution, Linux supervisor distribution | Subsequent work |

Unused functions are retained and emitted. Separate imports preserve existing type-only generation without introducing implicit unused-function removal. Even an unused execution-module import requires a target/runtime link in C and introduces an unsupported operation in the other generated routes.

Both modules are reserved virtual standard sources at contract version 1. The execution module imports the original types through normal resolution without duplication. Aliases and import order resolve to the same definitions. Standard dependencies perform no filesystem search and do not change user relative-import rules.

The Rust API `run_with_io_files_processes_arguments_and_call_limit` supplies the process connection and transfer-storage allowance alongside existing I/O, files, arguments, and call depth. Existing APIs remain available.

## Execution and failures

1. Ordinary calls and ownership lowering evaluate the request once. The standard-source `run` counts toward logical call depth.
2. Copy the request into independent storage; prepare fixed Report storage and empty failure arrays.
3. The common runtime validates request, connection, and capacity; clock reads and launch occur only after output-array reservation.
4. Preserve partial output, nonzero exit, failures, and cleanup in Outcome; returned arrays follow normal release rules.

Launch/transfer failures, deadlines, and capacity failures are values. A later call on a disabled connection returns `ConnectionDisabled` without implicit restart. Resource exhaustion preventing even a failure value stops with a source diagnostic such as `allocation-failed`, never disguised as child exit status.

MIR/SSA and VM validate Report shape using retained type metadata. Bytecode retains enum alternative names, avoiding fixed type IDs or Rust discriminants. Metadata preparation precedes launch. Recovery from allocator abort in existing Rust metadata cloning and similar operations is not guaranteed.

## Observation and verification

`emit-sources` exposes both standard sources. `emit-ir`, `emit-mir`, `emit-mir --ssa`, and `emit-bytecode` expose the operation and construction. Report NodeId/Span identifies the internal operation in the standard wrapper, not a rewritten user call site. SourceId identifies the virtual source.

Observation remains read-only. Identical backend responses with zero/nonzero trace capacity must yield identical results and OS actions.

- [Differential tests](../../src/runtime/process/tests/execution.rs): evaluation order, short circuit, nonzero exit, bytes, copies, explicit retry after launch failure, partial output, disabled connections, prelaunch capacity failure, and missing configuration.
- [Module tests](../../tests/process_types/mod.rs): nominal identity, virtual origins, IR text, private-operation rejection, existing emission, and unsupported diagnostics.
- [OS connections](../../experiments/subprocess/execution.rs): four executors send input to a registered fixture on Windows/Linux, comparing nonzero exit, bytes, and continuation. This dedicated comparison fixture runs four times; arbitrary user programs are not automatically replayed.
- [Runnable example](../../examples/process_execution/README.en.md): receive missing configuration as a value and continue.

[CLI registration, limits, and Linux supervisor paths](subprocess-cli.en.md) are connected. Staged runtime APIs, fixed Report, C ABI, and [generated C](subprocess-c.en.md) are also implemented. Next follow the [generated-route sequence](subprocess-generated.en.md) for LLVM/QBE/Native/WAT and distribution.
