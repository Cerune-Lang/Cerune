# Reserving subprocess results before launch

[日本語](subprocess-storage.ja.md)

Discovering insufficient result storage after launch cannot undo external effects. The [common runtime](subprocesses.en.md) now supports prelaunch return-storage reservation, and the internal array heap supports reservation, finalization, and cancellation. **Language `std:process::run` and common IR instructions remain unimplemented.**

## Current implementation

| Component | Implemented behavior |
| --- | --- |
| `Processes::run_with_reservation` | After request validation and transfer-buffer allocation, invoke the reservation function at most once before reading the clock or launching; return the outcome and reserved owner |
| `ReservationError` | Capacity/representation limits become `OutputBudget`; allocation failure becomes `AllocationFailed`, both at `Reserve` with no launch |
| `ArrayHeap::reserve_byte_outputs` | Reserve stdout/stderr limits in the existing array heap; a second-allocation failure releases the first |
| `ReservedByteOutputs::finish` | Convert confirmed bytes to existing immediate values and finalize lengths without reallocating arrays |
| Reservation Drop | Cancel unpublished arrays, including partial initialization, discarded results, and unwinding |

Existing Rust `Processes::run` uses a reservation function that adds no storage, preserving request/result behavior and validation order. Reservation is part of the execution API, not the read-only observation API. Its contract permits storage allocation only, without OS operations, retries, or extra execution control.

## Validation and ownership order

1. Assign the call identity; check disabled connection, request validity, request capacity, and missing configuration in the existing order.
2. Check and allocate retained output buffers and three transfer buffers.
3. Reserve return storage needed by the language or other caller.
4. Read the monotonic clock, calculate the deadline, and launch the registered target exactly once.
5. Run transfers, termination, and cleanup through the common runtime; return the outcome and reserved owner.
6. The caller finalizes the result in reserved storage, or cancels it if unused.

Failures through step 3 perform no clock reads, launch, or cleanup. Reservation failure alone does not disable the connection; a later explicit call remains possible. After successful reservation, the same owner is returned even for launch failures and partial results. A `Some` reservation does not imply successful launch.

The deadline starts from the clock read after reservation. As before, validation and allocation time are outside the subprocess execution deadline. This API does not guarantee completion when the reservation function never returns or the allocator aborts.

## Accounting and finalization

| Storage | Accounting and lifetime |
| --- | --- |
| Runtime raw bytes | Bound both output limits plus 3×4096 bytes with `available_bytes`; Rust Outcome owns returned raw output |
| Language arrays | Charge the existing ArrayHeap for 8 bytes per common u8 storage element times both limits; separately check and allocate actual Rust element storage |
| Finalized short results | Visible length is retained bytes; reserved-capacity accounting persists until array release. Even zero-length output retains a nonzero reservation until release |
| During conversion | Raw output and destination arrays coexist; do not hide one by claiming it has already been consumed |
| Report structs/enums needed even for failure values | `PreparedReport` allocates all storage before the common runtime call, using Vec storage in the existing value representations, separate from ArrayHeap byte-array accounting |
| Outer Outcome and request conversion | **Next integration step**: prepare storage holding Report and both arrays, plus request string/array conversion into Rust, before launch |

Finalization first checks that both output lengths fit. On overflow, publish neither array and release both. The immediate-value conversion function must not allocate; it is intended for existing executor integer representations. Preserve NUL, CR/LF, and invalid UTF-8 as bytes without changing the original failure, cleanup, or origin.

Reservation exclusively borrows the array heap and never exposes uninitialized storage as a user array. Finalized arrays return to existing retain/release/free and ownership lowering. Observation settings/capacity do not change reservation invocation count, outcomes, or OS action sequences.

## Fixed Report storage

The [internal Report conversion](../../src/runtime/process/report_value.rs) is implemented and used by unit and common-runtime integration tests. Language instructions do not call it yet.

| Stage | Behavior |
| --- | --- |
| Preparation | From the standard-module Report type ID supplied by the caller, validate common-IR field names/order/types and enum alternatives |
| Allocation | Use `try_reserve_exact` for every nested aggregate, including inactive enum payloads; release prepared storage on failure or abandonment |
| Finalization | Write the runtime Report into existing storage without allocation, string formatting, or OS operations; resolve tags by alternative names in common IR |

Both existing value representations are supported: the shared HIR/MIR/SSA value and the VM value. No fixed type IDs or Rust enum ordinals are assumed. Inactive payloads start with type-correct zero values.

Conversion preserves all 21 failure codes, transfer directions, exit codes/signals, confirmed byte counts, unknown progress, cleanup verdicts and raw reports, and NodeId/SourceId/Span. The public time unit is milliseconds: a Rust Duration outside exact u64 milliseconds produces a conversion error instead of rounding. Language integration must convert request `timeout_ms` exactly.

Layout mismatches are internal type-integration errors; allocation failures are Cerune resource failures. Inability to build a failure value must not become a subprocess failure value or exit code. Prepare Report before calling `run_with_reservation`, since request-validation failures also need a result. Reserve output arrays in the existing callback after request validation.

## Verification and remaining integration

[Array reservation tests](../../src/runtime/array_heap/byte_outputs.rs) cover zero/exact/short results, capacity/allocation/size failures, cancellation after second-allocation failure, and unwinding during conversion. [Common-runtime tests](../../src/runtime/process/tests/result_storage.rs) cover no launch, validation precedence, subsequent calls, partial results, single ownership release, and observation enabled/disabled.

[Windows/Linux OS checks](../../experiments/subprocess/README.en.md) now include reservation failure followed by explicit execution. They retain nonzero exits and raw bytes and compare storage addresses/capacities before and after finalization. These are Rust connection checks, not `.ceru` execution support.

```sh
cargo test --lib runtime::process
cargo test --lib runtime::array_heap
```

Report tests cover every field and alternative in both representations, reordered tags, failure and release at every allocation point, stable storage addresses, exact durations, and virtual-source origins. Common-runtime integration tests cover request rejection, reservation failure, the next explicit call, and identical results with observation enabled/disabled.

Next connect the outer Outcome with both arrays, then connect requests, return values, and origins through one common IR operation. Request string/array conversion into Rust also needs storage before launch. Failure to construct even a failure value remains Cerune resource exhaustion, never a disguised child exit code. Keep each executor's control flow independent and reject unsupported generated routes rather than simulating success.
