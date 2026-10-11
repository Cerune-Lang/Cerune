# First optimization pass: SSA constant folding

[日本語](ssa-constant-folding.ja.md) · [SSA](mir-ssa.en.md) · [Issue #94](https://github.com/Cerune-Lang/Cerune/issues/94)

## Status and choice

**Design only. The optimization pass, pass-selection CLI, and optimized bundle are not implemented.** This change adds executable Cerune fixtures to establish baseline semantics. SSA construction and SSA→MIR remain representation changes without implicit optimization.

Start with `ssa-const-fold-v1`: replace successful operations on known integer/bool values within one block by literals at the same instruction positions. Use typed known values and existing arithmetic rules, without inferring values from algebraic identities.

The candidates differ in the preservation conditions they require.

| Candidate | Required preservation | Order |
| --- | --- | --- |
| Constant folding | Typed ranges, operation success/failure, instruction position, failure origin | First; can preserve instruction counts, CFG, and IDs |
| Copy propagation/removal | Dominance, loop re-execution, parallel transfer, mappings for removed instructions | Later; v1 retains copies |
| Dead instruction elimination | Classify failures, output, allocation, and cleanup even when results are unused | Later; unused results alone do not justify deletion |
| Branch/CFG simplification | Short-circuiting, reachability, edge arguments, records of removed blocks/origins | Later; keep branches even with constant conditions |

The existing `const` feature requires frontend evaluation of constant expressions. This pass is optional optimization of already typed runtime IR; it must not turn runtime failures into compilation errors.

## Initial transformation scope

This is a limit on transformations, not on accepted language features. Valid SSA containing other operations remains accepted, preserving those operations.

| Input | v1 behavior |
| --- | --- |
| Literals of all eight integer types and bool | Record block-local known values, retaining width and signedness |
| Scalar Copy of a known value | Retain the instruction and record the destination's known value |
| Integer addition, subtraction, multiplication | Replace only when both inputs are known and the operation succeeds in the same integer type |
| Six integer comparisons and bool equality/inequality | Replace with a bool literal when both inputs are known |
| Function parameters, block arguments, definitions from another block | Unknown; no propagation across joins, loops, or calls |
| Unary operations, division/remainder, bit operations, conversions, floats | Retain operations; do not record their results as known in v1 |
| Calls, output, strings, aggregates, residual slots, heap operations | Retain execution; do not evaluate their values or effects during optimization |

Process each function's blocks once in instruction order, starting each block with an empty known-value table. Even a literal defined elsewhere is unknown in v1. Results do not depend on CFG traversal order or fixed-point iteration. Do not execute loops statically; perform only local substitutions valid on every block visit.

Use the existing typed scalar values and `runtime::value::binary` rather than reimplementing IR/MIR/SSA arithmetic. Whitelist operations/types before evaluation and pass only integer/bool values. A language failure leaves the original instruction intact. Invalid types or other internal inconsistencies are pass errors, distinct from an operation that cannot be folded.

## Preserved semantics and representation

Preserve output bytes, return values, failure codes, NodeId/SourceId/Span, prior output, evaluation order, short-circuiting, ownership, and heap budgets. Leave float operations, NaNs, and negative zero untouched.

The first pass has these constraints.

- Replace only proven successful supported operations, in place.
- Preserve instruction count/order, ValueId, original InstructionId, Origin, and type.
- Retain constant input instructions and Copy; do not alter branches, edges, block arguments, or retained unreachable records.
- Do not mutate input SSA or its non-SSA `original` MIR.
- Leave known failing operations in place, retaining prior output and failing at the original location only if executed.

This schematic is explanatory, not current output or finalized text syntax.

```text
before: v0 = u8 6; v1 = u8 7; v2 = mul v0 v1 [mir-i2, origin=P]
after:  v0 = u8 6; v1 = u8 7; v2 = u8 42    [mir-i2, origin=P]
```

Keep `u8 255 + 1` as addition because it fails. Do not replace `x * 0` when x is unknown. Do not delete unused additions that may fail. Never evaluate the call in `false && would_overflow()` while optimizing; preserve the already lowered short-circuit CFG.

The SSA validator currently requires input-MIR instruction count/ID/Origin correspondence. This pass can preserve that contract without weakening validation. Structural validity is not semantic preservation; execution comparisons and proofs remain separate obligations.

## Observing the transformation

Keep baseline MIR, constructed SSA, optimized SSA, and reconstructed MIR as separate snapshots; do not overwrite a single “current IR” file.

```text
HIR → baseline MIR ──────────────────────────→ baseline Native
       └ SSA before → ssa-const-fold-v1 → SSA after → reconstructed MIR → Native
```

Use a small Rust API that borrows validated input and returns transformed SSA plus replacement records, without introducing a general pass manager. Store records structurally so text output and bundles share them. Concrete Rust types and public commands will be finalized during implementation.

Records contain the following information.

| Record | Contents |
| --- | --- |
| Pass | `ssa-const-fold-v1`, version, options, application order, input/output snapshots |
| Replacement | Function, SSA block, original MIR block, original InstructionId, ValueId, type, single source origin |
| Evidence | Original operation/operands, typed known values and the instruction/Copy chains establishing them, replacement literal |
| Result | Replacement count; zero replacements still records an explicitly requested pass |

Complete retained instructions remain comparable in before/after snapshots. Unsupported operations, unknown operands, and operations retained due to language failure are not recorded as deleted instructions. Detailed non-application reason lists are a later consideration.

The CLI must run only explicitly requested passes. `--ssa` alone must not optimize. Reject unknown pass names and unsupported routes rather than silently ignoring them. Do not add implicit repetitions or extra passes. Initially allow v1 once; define order and duplicate selection when multiple-pass support is introduced.

An optimized bundle will use a new schema with before/after SSA, pass records, mappings, and MIR/ASM derived from that SSA, declaring each artifact's input in the manifest. Retain existing v1 (non-SSA) and v3 (SSA-only) behavior. Exact flags, filenames, and schema number remain undecided and are not added to today's CLI.

## Executable fixtures and verification

The fixtures use current language features and establish a baseline; they do not yet produce optimized output.

| Fixture | Semantics checked | Baseline result |
| --- | --- | --- |
| [constant_candidates.ceru](../../examples/ir_stages/constant_candidates.ceru) | u8 multiplication, u64 maximum, comparisons, short-circuiting, outputting call order, loop updates | Eight lines: `42 / 18446744073709551615 / true / false / true / 4 / 10 / 7` |
| [constant_overflow.ceru](../../examples/runtime_failures/constant_overflow.ceru) | Execute checked addition even for an unused result, preserving prior output and origin | Print `7`, then `integer-overflow` at `limit + 1`; never print 99 |

Compare the baseline across HIR, MIR, SSA, reconstructed MIR, and VM, plus Native ASM/Object. Pass implementation adds these requirements.

1. Test boundaries of all eight integer types, large u64 multiplication, intermediate overflow, and bool comparisons.
2. Check input immutability, determinism, IR idempotence, stable IDs/origins/order/edges, and replacement-record validation. An empty second replacement list is distinct from not running the pass.
3. Compare examples, failure cases, and heap budgets across baseline MIR, SSA before/after, reconstructed MIR, VM, and Native. Confirm calls, floats, and ownership operations remain.
4. Use the independent Lean SSA model to check concrete before/after examples and completion, initially including successful u8 addition and overflow; reject wrong types, values, and origins. Multiplication and other operations exceed today's Lean exporter, so extend it with explicit scope.
5. Test unchanged CLI defaults, explicit pass recording, and distinct artifact inputs. Do not generalize execution comparisons or finite-fixture proofs to proofs of all types or the whole transformer.

## Next implementation units

Implement the Rust pass API, replacement records, and tests first; then CLI/bundle and Native integration, followed by finite-fixture Lean checks. Treat instruction deletion, CFG simplification, and cross-block propagation as separate passes/versions. Completion requires tracing each substitution while keeping the original operations and branches observable.
