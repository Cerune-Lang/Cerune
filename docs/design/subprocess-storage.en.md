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
| Struct/enum storage needed even for failure values | **Next integration step**: reserve the fixed return shape before calling the common runtime |

Finalization first checks that both output lengths fit. On overflow, publish neither array and release both. The immediate-value conversion function must not allocate; it is intended for existing executor integer representations. Preserve NUL, CR/LF, and invalid UTF-8 as bytes without changing the original failure, cleanup, or origin.

Reservation exclusively borrows the array heap and never exposes uninitialized storage as a user array. Finalized arrays return to existing retain/release/free and ownership lowering. Observation settings/capacity do not change reservation invocation count, outcomes, or OS action sequences.

## Verification and remaining integration

[Array reservation tests](../../src/runtime/array_heap/byte_outputs.rs) cover zero/exact/short results, capacity/allocation/size failures, cancellation after second-allocation failure, and unwinding during conversion. [Common-runtime tests](../../src/runtime/process/tests/result_storage.rs) cover no launch, validation precedence, subsequent calls, partial results, single ownership release, and observation enabled/disabled.

[Windows/Linux OS checks](../../experiments/subprocess/README.en.md) now include reservation failure followed by explicit execution. They retain nonzero exits and raw bytes and compare storage addresses/capacities before and after finalization. These are Rust connection checks, not `.ceru` execution support.

```sh
cargo test --lib runtime::process
cargo test --lib runtime::array_heap
```

Next prepare the public Outcome's fixed shape and connect requests, return values, and origins through one common IR operation. Request string/array conversion into Rust also needs storage before launch. Failure to construct even a failure value remains Cerune resource exhaustion, never a disguised child exit code. Keep each executor's control flow independent and reject unsupported generated routes rather than simulating success.
