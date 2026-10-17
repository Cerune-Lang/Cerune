# Reducing spare capacity in completed input lines

[日本語](input-compaction.ja.md)

Based on [capacity measurements](input-memory.en.md), the four Rust executors and C/LLVM/QBE/Native compact completed string storage. Full-limit allocation before reading and memory accounting by string length remain unchanged.

## Conditions

Attempt compaction only for a nonempty, valid UTF-8 line meeting both conditions:

- Length is at most half the requested limit.
- The requested limit exceeds length by at least 4096 bytes.

This implementation policy avoids copying or allocator calls for small savings. It does not change string semantics or public limits.

| Stage | Behavior |
| --- | --- |
| Before reading | Reserve full-limit storage, accounted bytes, and ownership metadata. Failure consumes no input |
| After completion | Attempt compaction after UTF-8 validation, before publishing the owner |
| Compaction succeeds | Transfer the smaller storage into the completed string |
| Compaction fails | Keep original storage; add no diagnostic, reread, or implicit retry |
| EOF / input failure | Preserve cleanup, consumption, stopping reason, and NodeId/Span |

## Implementations

Rust uses fallible try_reserve_exact for length-sized storage and copies the contents. Failure preserves the original String. Ownership metadata is prepared before reading; registration needs no additional allocation.

C/LLVM/QBE/Native attempt realloc including ownership metadata. Success initializes and registers ownership at the new address; failure uses the original address. No other value holds the unpublished owner's old pointer. Generated C/LLVM/QBE/ASM expose the conditions, realloc, and selection; self-encoded Object uses the same operations.

**WAT keeps its existing free-storage reuse without splitting in this change.** Splitting alone can leave fragments too small for the next full-limit reservation and grow pages on repeated execution. WAT compaction needs a coalescing design and tests as well. String semantics, accounted bytes, input, output, and diagnostics remain identical across routes; retained capacity is not a shared language value.

## Guarantees

No public observation/control API for compaction is added. Input events, consumption, and read counts are unchanged; enabling observation does not select a different compaction policy. Failure injection and allocation probes remain confined to test connections and test artifacts.

Subsequent reads still require full-limit reservation. Failed shrinking or allocator behavior can retain excess capacity, so this is not a hard whole-process memory bound. Temporary Rust copies and realloc-internal storage are separate from string length.

## Validation

- Boundary, adjacent, and accepted values for 4096-byte savings and the half-size condition.
- Simulated Rust compaction failure preserves contents, input events, EOF, and the first failure.
- Normal, always-failing, and always-moving realloc across C/LLVM/QBE and MIR/SSA-derived ASM/Object; Japanese, NUL, CR/LF, retained contents, output, and final release.
- The [two-line example](../../examples/stdin/retained_lines.ceru) compares subsequent reads, unterminated final lines, EOF, and subsequent invalid UTF-8 across routes.
- WAT live/free capacity and pages continue to reuse storage as before.

For eight two-byte lines with a 1 MiB limit, successful compaction retains 16 bytes of Rust payload capacity or 208 bytes including malloc-route ownership metadata. Peak malloc requests are 1,048,782 bytes. Forced shrink failure retains the original 8,388,800 bytes while returning identical results; full release returns both cases to zero.
