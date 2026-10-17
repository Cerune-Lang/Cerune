# Owned dynamic arrays and range copies

[日本語](owned-arrays.ja.md)

**`[T]`, `array_copy`, `array_copy_range`, `array_repeat`, and the array budget are implemented in common IR, IR Executor/VM, generated C, LLVM, QBE, WAT, Windows/Linux ASM, and self-encoded COFF/ELF.** Runnable examples cover [copies](../../examples/dynamic_arrays/copy.ceru) and [nesting, functions, and enums](../../examples/dynamic_arrays/nested.ceru). It builds on [implemented string ownership](dynamic-data.en.md), but mutable array storage must be copied independently.

## Implementation progress

| Stage | Status |
| --- | --- |
| Argument ownership transfer | Implemented: pass caller-prepared ownership once; the callee releases it. IR distinguishes ownership from internal reading |
| Reads and owned-value preparation | Implemented: common IR marks `read` bindings and preserves temporary projection/release order |
| Dynamic array types, allocation, and independent copies | Implemented in IR/VM/C/LLVM/QBE/WAT/ASM/native objects, including ranges, display/equality/iteration, aggregates, and functions |
| `array_repeat::<T>` | Implemented: independent copies from a runtime count, expanded into common IR allocation, loops, and release |
| Dynamic arrays across backends and the completion criteria below | C, LLVM, QBE, WAT, Windows/Linux ASM, and self-encoded COFF/ELF implemented |

Dynamic arrays are also compared across all routes for results, failure reasons, source origins, and prior output. See [call ownership](dynamic-data.en.md#ownership-across-calls), [read preparation](dynamic-data.en.md#reads-and-owned-value-preparation), and the [argument](../../examples/owned_arguments.ceru) and [read](../../examples/borrowed_reads.ceru) examples.

## Types and initial operations

| Spelling | Meaning |
| --- | --- |
| `[T]` | An owned array with a runtime length; length is not part of its type |
| `[T; N]` | The existing fixed array, with its syntax and meaning preserved |
| `array_copy(values)` | Create a new `[T]` from a fixed or dynamic array of the same element type |
| `array_copy_range(values, start, end)` | Copy the half-open range `[start, end)` into a new `[T]` |
| `array_repeat::<T>(value, count)` | Evaluate value once and create count independent copies |
| `array_len(values)` | Return a fixed or dynamic array's element count as i64 |
| `values[index]` | Read through an i64 index; a mut array binding permits element updates |

Implemented operation names follow `array_len`. Existing example functions named `copy_range` and `append` remain ordinary functions. Definitions colliding with `array_copy`, `array_copy_range`, or `array_repeat` are diagnosed.

There is no implicit fixed/dynamic conversion. `[1, 2]` remains a fixed-array literal. Initially no dynamic-array literal is added. Empty values come from an empty range. `array_repeat::<T>(seed, 0)` also creates an empty array, but evaluates seed even for zero elements. Existing restrictions on `[T; 0]` and empty literals are unchanged.

The following runs through all routes:

```text
mut values: [i64] = array_copy([10, 20, 30]);
saved: [i64] = values;
part: [i64] = array_copy_range(values, 1, 3);
values[1] = 99;
print(saved); // [10, 20, 30]
print(part);  // [20, 30]
empty: [i64] = array_copy_range(values, 3, 3);
print(empty); // []
```

Equality compares lengths and corresponding elements of dynamic arrays with the same element type. Comparing with a fixed array requires an explicit copy; ordering comparisons are not added. Display uses `[elements, ...]`, or `[]` when empty, with existing nested-string quoting. A zero-length `for … in` skips the body.

Length ranges from 0 through i64::MAX. Initially capacity always equals length. A binding can be reassigned an array of a different length with the same `[T]` type. Append, reserve, exposed capacity, growth policy, and borrowed slices are later work.

## Value copies and observation

| Context | Generated behavior |
| --- | --- |
| Assignment, binding initialization, insertion into an array/struct/enum | Copy an existing value, including independent storage for nested dynamic arrays |
| Function arguments and returns | Pass by value; copy existing values at the boundary, with the receiving function or caller owning the result |
| A newly created temporary used for storage, argument, or return | Transfer the temporary's ownership without a second copy |
| Length, display, equality, intermediate index/field access | Do not copy the whole array to read it; evaluate the target once and keep it alive |
| `for (item: T in values)` | Create an independent snapshot at loop entry and pass each element as a value |
| `a[i] = value` | Change only that destination element; prepare the new value before releasing the old one |
| Self-assignment `a = a` | Finish copying before releasing the old value; both allocations count toward the budget |

Do not turn a binding's last use into a move: that changes allocation failures, budgets, and origins. Distinguish temporary ownership transfers from binding copies in IR. The first implementation does not use copy-on-write.

Apply the same rule recursively to fixed arrays and structs containing `[T]`. Copy and release only the active enum payload. Immutable string contents may be shared through explicit retain/release. Nested arrays in separate `array_repeat` elements must also be independent.

Evaluate and complete any argument copy from left to right, before evaluating the next argument. A failed copy skips later arguments. Transfer an argument's owned value to the callee once, without copying again on function entry. Returning a binding copies the result before releasing locals and parameters.

Match saves its subject once as an owned value and copies selected payload bindings as values. A false guard releases that arm's bindings before trying the next arm. Struct defaults and update expressions keep their existing evaluation order; overwritten fields are released after their replacements are ready.

Internal read access does not introduce source-level references or shared mutable APIs. For example, `array_len(make())` executes make, reads its owned result, then releases it. Evaluate both equality operands in their original order; only the element comparison walk may stop early.

## Ordering of ranges, updates, and failures

For `array_copy_range`, evaluate target, start, and end once from left to right. Read the target as the copy source without first making a full dynamic copy. Keep both existing and temporary sources alive until the operation finishes.

1. After all three arguments succeed, check `0 <= start <= end <= array_len(values)`.
2. Calculate and check end - start, accounting size, and target layout size.
3. Allocate the result and copy elements in ascending source-index order.
4. Transfer the result and release a temporary source.

A negative start value does not skip evaluating end if the start expression itself succeeded. A failure in end therefore precedes a range diagnostic. Empty ranges at the beginning or end are valid and allocate no result element storage.

`array_repeat::<T>(value, count)` requires an explicit element type `T`, a value of type `T`, and an `i64` count. It does not infer `T` or implicitly convert other integer types.

1. Read value once as the copy source, then evaluate count once. Keep the source alive until completion without an extra preliminary deep copy.
2. Check that count is nonnegative, validate logical/physical sizes and the budget, and allocate result storage.
3. Copy elements independently in ascending order, including nested dynamic arrays; retain immutable string contents.
4. Transfer result ownership and release a temporary source.

Zero elements still evaluate both expressions and release temporaries, but allocate no result storage and perform no element copies. Failure in value skips count; failure in count preserves its own diagnostic. A budget failure during element copying retains the repeat expression origin and prior output. See the [table example](../../examples/dynamic_arrays/repeat.ceru) and [zero-count example](../../examples/dynamic_arrays/repeat_empty.ceru).

For indexed assignment, evaluate and check each index before later indices or the right-hand side. In `a[bad][later()] = rhs()`, a failed first check skips both later and rhs. Do not release the previous element before right-hand evaluation and copying succeed.

| Diagnostic | Cause and origin |
| --- | --- |
| `array-index-out-of-bounds` (existing) | The indexed access expression |
| `array-range-out-of-bounds` | An invalid range at the array_copy_range expression |
| `array-length-out-of-range` | A negative count at the array_repeat expression |
| `allocation-size-overflow` (existing) | An unrepresentable size product/sum, length, or layout size |
| `allocation-limit-exceeded` (existing) | An explicit array or string budget is exceeded |
| `allocation-failed` (existing) | Allocator or memory.grow failure despite passing the budget check |

An allocation introduced by copying points to the source expression requesting that copy. Generated helper nodes have unique NodeIds and preserve the original Span. Preserve earlier output, and never count an unrelated access violation or trap as a successful diagnostic.

## Budgets and lifetimes

`--array-heap-limit <bytes>` is implemented, defaulting to 64 MiB and accepting decimal integers from zero through i64::MAX. Preserve the meaning of `--string-heap-limit`. Both are settings made before execution/generation and recorded in IR and bytecode. Generated C, LLVM, QBE, WAT, ASM, and native objects use the same setting. Initially dynamic arrays are not allowed in compile-time constants; explicitly copy fixed-array constants at runtime.

To keep pointer width and padding from changing where budget failures occur, **count live array element storage using common accounting widths**. This is not a physical-memory limit.

| Element accounting width | Bytes |
| --- | --- |
| bool, every integer, f32, f64 | 8 |
| string or dynamic-array descriptor | 16 |
| Fixed array | Length times element width |
| Struct | Sum of field widths in declaration order |
| Enum | Tag of 8 bytes plus all variants' field widths; do not allocate inactive dynamic payloads |

Charge length × width(T) per allocation, plus separate storage for nested dynamic arrays. Exclude static storage, local descriptors, and management headers. String contents count once under the existing string budget. Empty arrays cost zero. Preserve the length of zero-width element types and avoid division by zero in size checks. Check **both** common accounting size and physical target layout size; inspect high bits before narrowing to wasm32.

Old and new values coexist during reassignment. Self-assignment of three i64 elements raises the array charge from 24 to 48 bytes and back to 24 after release. Reading array_len(a) or print(a) keeps it at 24. Test this difference to prevent hidden copies or changes in failure timing.

Release bindings in reverse order at block exit, return, break, and continue; release array elements in reverse order. Track the initialized element count during partial copies so failures never read uninitialized storage. The VM reclaims all remaining allocations before returning a failure to its host. Preserve the existing native process-termination boundary and discard failed WAT instances.

## What the generation process exposes

`array_copy`, `array_copy_range`, `array_repeat`, and value copies now expand into ordinary common-IR loops and branches. Primitive spellings are `array.check-range`, `array.allocate-elements width=N`, `array.initialize-next`, `array.release-owner-last`, and `array.free-elements`. Lowering exposes:

- Once-only source evaluation, distinguishing reads, value copies, and temporary ownership transfers.
- Index, range, length, size, and budget checks.
- Element allocation, ascending copy loops, and initialized-element count updates.
- Type-directed processing of nested values, fields, and active enum payloads.
- String retention and release of old values, temporaries, and elements in reverse order.

Preserve correspondence between AST types, common IR, backend IR, bytecode, and artifacts. Element traversal must remain visible as ordinary common-IR loops and branches, rather than disappear into one opaque host call. Backends implement layout, allocation, load/store, and release without redefining value-copy semantics separately. Keep existing LLVM/ASM/object origin annotations.

IR Executor and VM share primitive typed storage management per execution. Execution remains independent: elements are copied by common-IR or bytecode loops. C uses the standard allocator of the target selected by the downstream C compiler. LLVM/QBE and ASM/native COFF/ELF use the explicit target's allocator and ABI; WAT uses private memory. Do not expose shared storage addresses through the language or observation APIs. Recursive types, dynamic arrays as external host values, borrowing, and FFI are outside this change.

### Ownership handoff from a temporary

Copying a binding always allocates independent storage. Extracting an owned temporary such as `make().field` briefly retains the selected array before releasing the parent aggregate. Only this handoff uses `array.retain-owner` and the owner count; it is not copy-on-write. The last release enters an ordinary reverse IR loop to release elements, followed by `array.free-elements`. Unrelated fields do not stay alive and consume the next argument's budget.

IR Executor and VM reclaim partially initialized regions at the execution boundary. Allocation failure, size overflow, and budget exhaustion are tested separately. Invalid ownership is an internal error, not a language failure.

## C support

C uses an owner pointer and length descriptor. The owner tracks references, initialized elements, and logical bytes; separately allocated element storage has the target C type's alignment. Empty arrays allocate nothing. Accounting width and physical `sizeof(T)` are checked independently, before budget checks and allocation.

Element copying, equality, display, and reverse release remain common-IR loops emitted as C. Helpers only manage storage and typed loads/stores. Tests check live accounting after success, allocator failure, and budget failure during partial copies. A runtime stop emits the existing diagnostic and terminates the process; the OS reclaims remaining storage. No recoverable execution API is added.

Compare the same [source](../../tests/fixtures/dynamic-arrays/source.ceru), [IR](../../tests/fixtures/dynamic-arrays/ir.ceir), [bytecode](../../tests/fixtures/dynamic-arrays/bytecode.cebc), and [C](../../tests/fixtures/dynamic-arrays/c.c). The [returned-range example](../../examples/dynamic_arrays/window.ceru) also runs through IR/VM/C/LLVM/QBE/WAT/ASM/native objects. C comparisons use `-O0` and `-O2`, plus ASan/UBSan on Linux. These are external compiler test settings, not implicit Cerune optimization.

## LLVM support

LLVM stores the owner and length as `%cerune.array = { ptr, i64 }`. The owner tracks element storage, reference count, logical bytes, and initialized count. Copy, display, equality, and reverse-release loops remain common-IR lowering; LLVM implements allocation and typed loads/stores.

Logical accounting uses common-IR `width`; physical allocation uses the target type's `getelementptr` stride. Even when `bool` or `f32` uses fewer than eight physical bytes, its budget matches other routes. Both products and their signed 64-bit limits are checked before allocation. Zero length allocates nothing; zero width preserves length.

Require `--target x86_64-unknown-linux-gnu` or `--target x86_64-pc-windows-msvc` without choosing from the compiler host OS. Use the target's `malloc/free` and existing output/diagnostic ABI. External Clang owns compilation and linking after emission.

The [LLVM fixture](../../tests/fixtures/dynamic-arrays/llvm.ll) and `--annotate-origins` expose the common-IR correspondence. Compare IR/VM/C/LLVM/QBE/WAT/ASM/native objects output, including the [record-array example](../../examples/dynamic_arrays/readings.ceru). LLVM tests at `-O0`/`-O2` cover range/index/budget/allocation failure codes, NodeId/Span, and prior output. An internal ownership trap alone is not an accepted language failure. Successful execution must return the live array budget to zero.

## QBE support

QBE passes dynamic arrays as pointers to owners. Each owner has six eight-byte fields: element pointer, length, references, logical bytes, initialized count, and stride. Empty arrays are null. This is internal storage; no user-facing shared mutable pointer API is added.

Following the existing QBE layout, string and dynamic-array pointers physically occupy eight bytes but count as sixteen logical bytes. Check physical element stride and common-IR accounting width separately. Allocate owner and elements with `malloc`; free the owner first if element allocation fails.

The [SSA fixture](../../tests/fixtures/dynamic-arrays/qbe.ssa) retains copy/equality/display/reverse-release loops, element store/blit, and initialization completion. Checks receive the original operation's NodeId/Span. An internal ownership `hlt` alone is not a language diagnostic. Tests verify zero live budget after success, size overflow, and both allocator failures.

Explicitly select `amd64_sysv` for Linux or `amd64_win` for Windows. QBE 1.3 tests compare IR/VM/C/LLVM/QBE/WAT/ASM/native objects, including the [returned-batches example](../../examples/dynamic_arrays/batches.ceru) and arrays combined with the [Windows argument rules](qbe-windows.en.md). `array_repeat` uses the same common IR and storage management.

## WAT support

WAT uses private Wasm32 linear memory and passes dynamic arrays as i32 owner references. Empty arrays use zero. Owners have this layout:

| Byte offset | Content |
| --- | --- |
| 0 / 4 | i32 next-block reference / freed flag |
| 8 / 16 / 24 | i64 reference count / initialized count / length |
| 32 / 40 / 48 | i64 logical bytes / element stride / physical capacity |
| 56 onward | Element storage |

Following the existing WAT layout, scalar storage slots occupy eight bytes. String and array references are i32 values, but count as sixteen bytes in the common byte accounting. Allocate owner and elements together; validate length, logical width, and Wasm32 physical size using i64 before narrowing addresses. Check indices at full i64 width as well.

Strings and arrays share one physical allocation cursor to prevent overlap, with independent live budgets and reuse lists. Reuse a sufficiently large freed block or grow only the required pages with `memory.grow`. Blocks are not split, coalesced, or shrunk; fragmentation or exhausted linear address space can cause allocation failure. Return budgets on success; discard the instance after a trap.

Zero references and freed storage are distinct. Only `array.free`, after common IR's reverse element cleanup, permits reuse. Common-IR copy, equality, display, and cleanup loops remain visible in [generated WAT](../../tests/fixtures/dynamic-arrays/wat.wat). Checked helpers retain the original NodeId/SourceId/Span and pass only output bytes to the host. Neither memory nor mutable diagnostic state is exported.

Common tests compare IR/VM/C/LLVM/QBE/WAT/ASM/native objects, including the [string-array example](../../examples/dynamic_arrays/labels.ceru). Primitive tests distinguish size overflow, actual `memory.grow` failure, zero width, empty arrays, and indices with nonzero upper 32 bits. Repeated successful calls on one instance verify zero live array/string budgets and storage reuse within one page.

## Windows/Linux ASM and self-encoded objects

Native passes dynamic arrays as eight-byte references to ownership headers. One `malloc` allocates the header and element storage; empty arrays use null. Common-IR copy, equality, display, and reverse-cleanup loops remain visible in lowering.

| Byte offset | Content |
| --- | --- |
| 0 / 8 / 16 | Element base pointer / length / owner count |
| 24 / 32 / 40 | Logical bytes / initialized count / element stride |
| 48 onward | Element storage |

Following the existing Native ABI, element slots and aggregate fields descend from the base address in eight-byte steps. An internal array reference occupies eight bytes but counts as sixteen bytes toward the memory limit. Logical size and physical size including the 48-byte header are checked separately before allocation. The budget increases only after successful allocation and returns on `array.free-elements` after the final owner releases the elements. This does not expose a pointer API to users.

Explicit Windows/Linux targets select argument placement and OS allocation, output, and failure conventions. Shared lowering produces ASM and self-encoded COFF/ELF; both use external linking. Inspect [Linux ASM](../../tests/fixtures/dynamic-arrays/linux.s), [Windows ASM](../../tests/fixtures/dynamic-arrays/windows.s), and `--annotate-origins` to follow allocation, element loads/stores, checks, and cleanup.

Tests compare output and failure records across all routes, including the [coordinate-pair example](../../examples/dynamic_arrays/coordinates.ceru). Primitive tests cover size overflow, forced malloc failure, empty and zero-width arrays, owner counts, and zero live budgets/empty management lists after repeated successful calls. Windows diagnostic CRLF is normalized to LF; string output preserves NUL, CR, and LF byte-for-byte. Internal ownership traps do not count as successful language diagnostics.

## Implementation acceptance criteria

| Area | Compare |
| --- | --- |
| Basics | Empty, singleton, runtime lengths, explicit conversion, reassignment to a different length |
| Value semantics | Nested arrays, strings, structs, enums, self-assignment, independent repeated elements |
| Existing features | Parameters/returns, explicit generic type arguments, modules, comparison/display/iteration |
| Evaluation order | Target/bounds/count, short circuit, staged index checks, skipping expressions after copy failure |
| Lifetime | Block/return/break/continue, iteration snapshots, partial-copy failure |
| Budget | Same semantic failure for the same explicit limit, no read-only copies, old/new coexistence, reuse after release |
| Allocation failure | Size boundaries, fixed allocator failure, actual WAT memory.grow failure |
| Observability | Common-IR/bytecode/backend fixtures and annotations identifying the original expressions |

Compare known output bytes, diagnostics, and prior output across IR Executor, VM, generated C, LLVM, QBE, WAT, Windows/Linux ASM, and native COFF/ELF. Add C ASan/UBSan checks and verify successful cleanup and partial-failure reclamation. Mark the supported routes at each stage in the reference and feature tables; do not claim full route coverage until they agree.

## Executable baseline today

`array_copy_lifetimes.ceru` combines nested fixed arrays of capacity three with a used length, and copies, returns, and replaces dynamically concatenated strings. It checks survival after function locals disappear, independent nested copies, range argument order, an empty end range, invalid ranges, and short circuit. Invalid ranges return **the example's enum values**, not the proposed runtime diagnostics.

```sh
cargo run --quiet -- run examples/array_copy_lifetimes.ceru --string-heap-limit 24
cargo run --quiet -- emit-ir examples/array_copy_lifetimes.ceru
cargo run --quiet -- emit-bytecode examples/array_copy_lifetimes.ceru
cargo run --quiet -- emit-llvm examples/array_copy_lifetimes.ceru --target x86_64-unknown-linux-gnu --annotate-origins
```

It succeeds with a 24-byte string budget. The source and selection stay at `["保存", "二"]` while only the edited array becomes `["変更", "二"]`. A used length of zero still has array_len(storage) equal to three. This example does not test empty dynamic values, array allocation failures, or runtime-sized storage. Current IR exposes fixed-array value copies, traversal, and string retention/release.
