# Subprocess C ABI and static-link probe

[日本語](README.md)

**This probe statically links the shared Rust runtime into an independent C program.** [Generated C](../../../docs/design/subprocess-c.en.md) is integrated. LLVM/QBE/Native/WAT integration and Native build/release packaging remain unimplemented. Language examples stay in [process_execution](../../../examples/process_execution/README.en.md).

## Build and run

Use Rust 1.98, Node, Clang, and the matching OS system linker/development libraries. Specify the target and Clang explicitly; the probe rejects targets that differ from its host. Build the Linux supervisor first. Choose an output directory that does not exist.

~~~text
node experiments/subprocess/c_abi/check.cjs x86_64-pc-windows-msvc clang target/process-c-abi
cargo build --example subprocess_linux_supervisor
node experiments/subprocess/c_abi/check.cjs x86_64-unknown-linux-gnu clang target/process-c-abi /absolute/path/to/subprocess_linux_supervisor
~~~

The script enables process-c-abi to build a staticlib and links the [independent C test](probe.c). Default builds add no C ABI symbols. System libraries are explicit per target, and Rust's native-static-libs output is recorded. Ordinary emit/build commands gain no implicit library search.

commands.json records commands, target, link inputs, and exit statuses. config.json contains local absolute paths for the probe and is not automatically distributed or uploaded as a CI artifact. This archive is a verification artifact built from the compiler crate, not a completed runtime distribution package.

## Call contract

The [header](../../../src/runtime/process/abi/cerune_process_v1.h) corresponds to the [implementation](../../../src/runtime/process/abi.rs). Version 1 is currently a prototype contract.

| Operation | State and responsibility |
| --- | --- |
| create | Specify ABI version 1 and target. Pass UTF-8 JSON to the shared parser; empty configuration explicitly means unconfigured. Failure clears the output handle |
| prepare | Idle only. Copy/retain the request, then invoke shared validation and transfer allocation. Return 1=Pending or 2=Ready |
| execute | Once in Pending. Reservation 0=success, 1=capacity exceeded, 2=allocation failed. Uses shared Prepared::execute and becomes Ready |
| report | Copy Pending/Ready Report v1 into exactly 32 u64 words, retaining NodeId/SourceId/Span |
| copy | Copy Ready stdout/stderr at an explicit offset. No internal pointers or advancing read cursor |
| release | Drop unsubmitted Pending storage or Ready results and return to Idle. Preserve disabled connections and unreclaimed OS ownership |
| destroy | Destroy context and clear the caller's handle. Existing Backend ownership/quarantine rules govern unreclaimed resources |

prepare/execute/release/destroy are control operations. report/copy advance no clock, launch, transfer, or release. Changing or dropping original request/configuration storage after preparation does not change execution. Context-owned requests and connections remain private and cannot be replaced in Pending.

Request is a 120-byte C structure and Bytes is 16 bytes. Pointers use this 64-bit target's addresses; lengths, time, and origins use u64. Reports use the [fixed Report v1](../../../docs/design/subprocess-generated.en.md) layout. Restoring SourceId does not validate membership in the original SourceMap.

C callers guarantee valid initialized input ranges, valid output ranges, and no overlap between them or with the context. NULL with length zero is allowed. Non-null pointer validity, freed handles, and concurrent access cannot be safely checked. Calls on a context must be serial and non-reentrant; destroy it exactly once. Representable semantic errors such as embedded NUL and zero timeout remain shared-runtime failure values.

## Failures and ownership

| ABI status | Meaning |
| --- | --- |
| 0 / 1 / 2 | Success / pending reservation / completed result |
| 10 / 11 / 12 | Unsupported version / invalid shape / invalid call state |
| 13 / 14 / 15 | Invalid width/range / invalid UTF-8 / request-copy allocation failure |
| 16 / 17 / 18 | Invalid configuration / caught internal panic / target mismatch |

Statuses 10 and above are connection-contract or physical-resource failures, separate from subprocess Outcomes. Callers use the original operation position and status to diagnose a stop. Generated-route diagnostic lowering follows later. Invalid state never silently retries a launch, and errors do not copy request bodies or environments into messages.

Catchable panics do not unwind into C; the context becomes Poisoned and only destroy remains available. Rust's panic hook is unchanged. Allocator abort and forced process termination are not recoverable guarantees; context Box allocation can also abort.

Request strings, arguments, stdin, and argument-view storage are copied/allocated during prepare. Private String storage remains immutable, and internal views are exposed only for the borrow lifetime of OwnedRequest. Rust/C tests verify validity after moves and destruction of original storage. execute does not reallocate argument views.

## Verification scope

Independent C expectations check sizes/offsets/Report fields, copied requests, origins, arbitrary bytes, nonzero exit, repeatable positioned reads, invalid version/shape/UTF-8/Span, duplicate prepare/execute, reservation failures, unregistered/missing targets, output limits, timeouts, and calls after releasing results. If Windows cancellation leaves progress unknown, subsequent ConnectionDisabled is checked. Rust tests additionally check poisoning/destruction after panic and internal-view lifetimes.

Shared-runtime tests continue verifying that observation does not change execution. Existing codec tests cover all failure numbers; the C probe does not claim to reproduce every OS fault. It does not exercise invalid non-null pointers or data races.

Generated C request conversion, array reservation, Report conversion, and startup configuration are implemented. The [comparison probe](generated.cjs) uses the same archive. Distribution archive names, ABI compatibility guarantees, manifests, and Linux supervisor placement remain later work.
