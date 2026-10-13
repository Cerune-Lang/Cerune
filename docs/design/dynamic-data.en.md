# Dynamic data: string ownership and lifetime

[日本語](dynamic-data.ja.md)

**String concatenation is implemented across routes. [Dynamic arrays](owned-arrays.en.md) support IR/VM/C/LLVM/QBE/WAT/ASM/native-object execution; borrowed slices remain pending.** For string ownership below, the IR Executor, VM, C, LLVM, QBE, WAT, Windows/Linux ASM, and native COFF/ELF routes share value semantics and ownership lowering.

## String concatenation

`concat(left, right) -> string` is a built-in accepting exactly two strings. It adds neither implicit numeric formatting nor a new meaning for `+`.

1. Evaluate left, then right, once each. A failing left operand prevents evaluation of the right.
2. Check the length sum, target layout size, and dynamic string budget.
3. Allocate a result and copy all left bytes followed by all right bytes.
4. Transfer the immutable result and release temporary values.

Japanese text, NUL, CR/LF, and combining sequences retain their exact bytes. No normalization, newline conversion, or terminating NUL is added. Equality, `byte_len`, standalone printing, and quoted aggregate display retain their existing meanings. An empty result uses static storage; every nonempty result allocates once, even with an empty operand. Operand evaluation is never skipped.

Explicit `const` evaluation applies the same rules, then freezes the result into a static string. IR retains both initializer and result; compilation does not carry runtime ownership into the artifact.

## Copies and lifetime

| Value or operation | Behavior |
| --- | --- |
| Static strings | Share storage; retain/release has no effect |
| Dynamic strings | Share immutable content using logical reference counts |
| Arrays and products | Copy independent values; immutable string fields may share content |
| Enums | Retain/release only the active payload |
| Reassignment | Finish the RHS before releasing the previous value and saving the result |
| Function return | Secure result ownership before releasing locals and parameters |
| Scope exit, return, break, continue | Release relevant bindings in reverse definition order |
| Loop conditions/updates and short circuit | Create and release temporaries at their original evaluation position |

Both `s = concat(s, suffix)` and `s = s` are valid. Previously saved copies retain their contents. There is no user-facing `free`, implicit move, pointer type, GC, or shared mutable reference. Observation does not expose mutation of ownership metadata.

The IR Executor and VM release locals on success and reclaims remaining content at the call boundary on failure before returning to its host. Existing embedding API value types remain unchanged. Native failure terminates the process, whose remaining allocations the OS reclaims. Successful WAT calls reuse released storage; discard an instance after a trap instead of reusing it.

## Ownership across calls

Arguments to user functions are evaluated from left to right, including the retains needed to prepare each owned value. Ownership passes once: the callee does not retain parameters again at entry, and the caller does not release transferred arguments after the call. The callee also releases unused arguments and acquires the return value before releasing locals and parameters. A binding's last use does not silently become a move.

After ownership expansion, IR shows `ownership arguments=owned` / `borrowed` together with explicit retains and releases. Executors run those body operations without adding extra lifetime operations from the classification. Generated comparison, display, match, and expression functions only borrow their arguments during the call. Generated release and replacement functions process transferred ownership. This is logical ownership of dynamic storage, separate from aggregate copies required by a backend ABI; it does not add a source-language borrow type.

[owned_arguments.ceru](../../examples/owned_arguments.ceru) demonstrates independent fixed arrays, argument order, returned-value lifetime, and early returns with unused arguments. It completes 100 iterations with a 22-byte string budget. At 15 bytes, construction of the first argument fails; at 19, the next argument fails. IR Executor, VM, and compiled routes compare these cases. Dynamic array allocation is not included yet.

## Reads and owned-value preparation

Reading an existing binding for `array_len(a)`, display, comparison, indexing, or field access does not acquire ownership. Common IR marks these internal bindings as `read %...`, avoiding whole-array walks whose only purpose was retaining and releasing strings. Saving or passing a value still acquires the required ownership. Fresh array literals and products without a base receive the ownership already prepared for their elements or fields.

Temporaries remain alive until their read completes. For a projection such as `make().field`, retain the selected value before releasing the original temporary. Keeping unrelated fields alive until the next expression would change budget failures. If a product contains two dynamic strings of two bytes each, `make().value == make().value` continues to succeed with six bytes rather than requiring eight.

This distinction describes logical ownership. Physical fixed-array copies may remain in executor value representations or backend ABIs. Executors run explicit lifetime operations in the body without inferring additional work from the `read` marker. No source-language borrow type or dynamic-array storage management is added yet.

[borrowed_reads.ceru](../../examples/borrowed_reads.ceru) covers reads, temporary projections, operand order, short circuit, and independent updates after copying. It succeeds with a 30-byte string budget; at 29 bytes, concatenation in the right-hand product construction fails. IR/bytecode fixtures and every execution route check output, the original failure location, and prior output.

## Budget and failures

The default budget is **64 MiB (67108864 bytes) of live dynamic string payload**. Shared allocations count once; static literals do not count. Old and new values both count while reassignment keeps them alive.

Use `--string-heap-limit <bytes>` with `run`, `run-ir`, `run-vm`, or code generation commands to change the runtime budget; zero is valid. Compile-time evaluation has an independent 64 MiB budget, unaffected by this option. Rust callers set `ir::Program::string_heap_limit` before emitting or lowering to bytecode. This configures generation rather than intervening in a running program.

```sh
cerune run examples/string_concat.ceru --string-heap-limit 1024
cerune emit-c examples/string_concat.ceru --string-heap-limit 1024 -o concat.c
```

| Failure code | Cause |
| --- | --- |
| `allocation-size-overflow` | Length sum or target layout size cannot be represented |
| `allocation-limit-exceeded` | Live payload would exceed the configured budget |
| `allocation-failed` | Allocation fails after validation; includes exhausted usable linear address space in WAT |

Logical byte lengths are `i64`. Header/layout sizes are checked; wasm32 never truncates upper bits before validation. Failures retain the original concatenation NodeId/Span in the [shared runtime record](runtime-diagnostics.en.md). Prior output remains visible and later expressions do not execute.

The budget is not a physical-memory cap. Metadata, stacks, fragmentation, and host conditions can cause allocation failure below the payload limit. Unexpected OS termination or access violations are not accepted as valid `allocation-failed` diagnostics.

## Observable lowering and storage

Common IR `string.concat.allocate-copy` requests checked allocation and copying. Generated `ownership-expression` / `ownership-replace` functions and explicit `string.retain` / `string.release` expose evaluation order and lifetime. Array traversal, product fields, and enum tag branches become ordinary IR. Original spans remain attached; additional statements/expressions receive unique NodeIds.

Backend IR and artifacts retain size calculations, budget checks, allocation calls, copies, and release. `emit-ir` and `emit-bytecode` also show the runtime budget. Existing origin annotations cover LLVM, ASM, and objects. Storage layouts and runtime addresses need not match across routes.

| Route | Dynamic storage |
| --- | --- |
| IR Executor / VM | Separate shared handles from logical reference counts; reclaim bytes at last release or a failed call boundary |
| C / LLVM | Preserve pointer-plus-length values; identify dynamic allocations through a list and use `malloc` / `free` |
| QBE / ASM / native objects | Preserve references to length headers; maintain ownership headers, counts, and a list |
| WAT | Reuse free blocks in private memory; check `memory.grow` only when expansion is needed |

Native headers contain link, reference count, and length. WAT headers contain link, reference count, capacity, and length. Static strings are never identified by reading a presumed header before their data. Initial retain/release uses linear list searches. WAT reuses the first sufficiently large free block; splitting, coalescing, and shrinking are not implemented. Freed capacity does not count toward live payload.

LLVM uses explicit targets rather than the compiler host OS. Native COFF/ELF puts management state in non-executable writable `.data` while retaining read-only literals. WAT neither exports nor imports memory. Tool execution and linking remain the artifact consumer's responsibility.

## Internal Rust string-owner validation

StringHeap retain/release checks both the allocation ID and shared-handle identity (`Rc::ptr_eq`). IDs are assigned per heap, so an ID alone does not establish membership. Foreign handles, missing IDs, zero owner counts, and retain overflow return `InvalidStringOwnership` without changing counts, live bytes, or contents.

This hardens a crate-internal invariant. Reachability from ordinary Cerune source has not been established; it adds neither a language-level stop condition nor value copying. Static and empty strings still have no dynamic owner count. Regression tests cover colliding IDs across heaps, invalid counts, double release, and successful cleanup after rejection.

## Executable checks

[string_concat.ceru](../../examples/string_concat.ceru) covers concatenation, returns, reassignment, array copies, byte preservation, and loops. Shared `concat_cases` additionally cover defaults, enums, guarded matches, generic arguments, constants, side effects, self-assignment, and early return.

`string_heap_routes` passes the same small budget through every route and compares known output, budget failures, prior output, origins, and index-check order. Internal WAT tests cap memory at one page to verify reuse and repeat successful calls on the same instance. C is also checked with ASan/UBSan. Internal tests distinguish fixed allocation-failure conditions from size overflow.

## Dynamic arrays and future borrowed slices

Implement owned dynamic arrays and range copies before borrowed slices. The [owned-array design](owned-arrays.en.md) specifies copy points, read-only access, common budget accounting, failure order, and cross-route acceptance criteria. Owned arrays work in IR/VM/C/LLVM/QBE/WAT/ASM/native objects; borrowed slices remain pending.

- `[T]` differs from `[T; N]`, permits zero length, and maintains `0 <= length <= capacity`. Capacity counts elements; initially allocate only the requested amount.
- Copies own independent nested mutable storage. Immutable string content can be shared. Lower typed copy/release into common IR and check size products/sums.
- `array_copy_range(values, start, end)` evaluates its three operands once from left to right, checks `0 <= start <= end <= length`, and copies half-open `[start, end)` into an independent owner, including an empty end range.
- Design access, update, parameters, results, equality, display, and iteration together. Fixed-array conversions are explicit. Reservation and growth policies are later operations.
- Before borrowing, define owner lifetime, mutation, and reallocation rules. Resolve escaping-owner returns and mutation during a borrow explicitly. Keep byte ranges distinct from UTF-8 character boundaries.

[bounded_sequence.ceru](../../examples/bounded_sequence.ceru) and [array_window.ceru](../../examples/array_window.ceru) remain fixed-capacity baselines. Their `append` and `copy_range` are example-defined functions, not dynamic-array built-ins. Example functions validate used lengths; unused storage remains part of the fixed value. Recursion and external I/O require separate call-storage, resource-limit, failure, and cleanup contracts.
