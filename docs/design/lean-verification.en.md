# Semantic correspondence and program proofs with Lean

[日本語](lean-verification.ja.md)

## Purpose

The Lean backend should support both:

1. **Translation correspondence:** completed Cerune IR and generated Lean code have the same meaning.
2. **Program properties:** the program satisfies a stated specification under explicit assumptions.

Once Cerune self-hosts, the compiler written in Cerune can itself become a subject of (2).
Self-hosting is not a correctness proof: compiler specifications, input domains, and output semantics still need definitions and proofs.
It also does not automatically verify today's Rust compiler or other backends.

**Current status: design and a small verification experiment. The public Lean backend, `emit-lean`, and a formal semantics for the whole language are not implemented.**
The [experiment](../../experiments/lean/README.md) checks actual `increment: u8 → u8` HIR against direct Lean and lowered MIR, together with user properties. A second function, `choose: u8 → u8`, adds if/short-circuiting and an independent MIR model of typed locals, instructions, jumps, branches, and returns. Five fixtures of `advance: u8 → u8` cover while/for, mutable locals/cyclic CFGs, break/continue inside if, innermost-loop behavior, and for initialization/updates/update overflow. Each fixture checks values or overflow/origins for all 256 inputs. Heap and general loop termination remain outside the proof scope.

The MIR branch evaluator uses block count as fuel; the exporter rejects cycles including unreachable blocks. The all-256-input correspondence also checks completion within that bound. `completed (.error …)` (language failure), `invalid` (invalid model state), and `exhausted` (insufficient fuel) are distinct; matching fuel exhaustion is not proof success. The loop fixture uses separate explicit bounds and additionally proves completion for every input within 16 HIR statement visits/11 MIR block visits for plain while, 22/22 for break/continue, 28/23 for nesting, 23/25 for for-control, and 8/5 for update failure. Completion includes language-level failure; the update-failure fixture overflows for every input. Checks reject incorrect iteration counts, updates, backedges, failure origins, break/continue targets, inner-to-outer jumps, skipped for initialization/updates, changed update origins, and setting both fuels to zero. These bounds are fixture-specific, not a proof of general termination or a bound-discovery algorithm.

The HIR model retains for initialization, condition, update, and body as `forLess`. Internal `forNext` represents repetition after initialization: body completion and continue pass through the update once before testing again, while break skips it. HIR is not reconstructed from MIR targets. Shared correspondence/completion theorems live in `LoopCorrespondence.lean`; execution comparisons use the exact same proved definitions.

SSA now has [finite-fixture checks](mir-ssa.en.md#finite-fixture-lean-verification): baseline MIR→SSA→reconstructed MIR values, failure origins, and completion within explicit bounds for all u8 inputs of the seven existing fixtures plus joins and swaps. SSA control has an independent model; individual operations share MIR semantics. Independent join/swap properties are also checked and transferred to reconstructed MIR by correspondence. SSA theorems depend on standard `propext`. General SSA conversion, heap, and Native remain unproved.

## Connecting the contracts

```text
Cerune source → shared frontend → completed Cerune IR
                                    ├─ IR model in Lean (reference semantics)
                                    └─ generated Lean code + correspondence proof
                                                └─ user specification + property proof
```

Conceptually, translation correspondence requires
`observe(evalIR(P, s)) = observe(evalLean(emit(P), encode(s)))`
for valid inputs/states s.
A property Q of the generated program can then transfer to IR through that correspondence.
Putting the same generator on both sides of an identity, or comparing output for successful examples alone, does not establish this contract.

The reference model formalizes the meaning of existing `ir::Program`.
It does not introduce another language/frontend or redo name resolution, generics, match expansion, or ownership lowering.
The Rust-to-Lean representation boundary and the model's fidelity to the existing specification still require review and validation.

## Runtime semantics

Observations include return values, output bytes, failure codes, NodeId/SourceId/Span, and output preceding failure.
Proofs involving intermediate states or resource budgets must include those states in the relation.

| Area | Meaning to preserve |
| --- | --- |
| Integers | Type bounds and overflow, division, shift, and explicit-conversion failures; no silent replacement by unbounded or wrapping arithmetic |
| Floating point | IEEE widths, rounding, NaN, infinity, negative zero, and conversions; not Lean real numbers |
| Strings | Immutable bytes, Japanese text, NUL, CR/LF, no normalization, display and equality |
| Control | Left-to-right order, short-circuiting, functions, loops, break/continue/return |
| Aggregates/ownership | Independent copies and the allocation, retain/release, freeing, and budgets explicit in common IR |
| External conditions | Explicit assumptions for future I/O and real allocation failure; an ideal heap proof does not guarantee OS behavior |

Finite-fuel agreement proves agreement only within that fuel. It does not establish general termination or equivalence of infinite executions.
Total execution claims need termination proofs or an execution relation admitting divergence.
Cerune resource-limit failures are distinct from proof-checker/test timeouts.

## Backend and checker boundary

Lean is [emit-only](owned-routes.en.md). A future `emit-lean` emits Lean source and tracking metadata; the ordinary Cerune CLI does not invoke Lean.
Development tests and CI use a pinned Lean version to check code and theorems.

Start with per-program translation validation.
There is no fallback that turns a failed proof into success. A later proof of the general translation algorithm is a separate result.
User properties live in a separate file so a generator cannot rewrite the specification to make it pass.
Proofs and execution must reference the same generated definitions.

Tie source, completed IR, generated definitions, theorems, origin maps, and semantics/Lean versions to verification results.
Do not remove unsupported types, values, or ownership operations and call the remainder verified.
Accepting a whole program requires coverage checks including unused definitions.
A function-only experiment must name the selected function and explicitly exclude unverified callers.

## Distinguishing tests and proofs

- Compare known expectations with IR Executor, VM, and generated-route execution.
- Prove correspondence between the Lean model and output, reviewing the theorem statement and assumptions.
- Prove independently authored user properties of the same generated definitions.
- Mutate generated behavior, failure origins, and specifications; require rejection.
- Distinguish unsupported, unexecuted, timed-out, and failed proofs from success.

Lean kernel acceptance is relative to theorem statements and axioms.
Inspect dependencies with `#print axioms`; do not silently accept `sorryAx`, unreviewed custom axioms, or extra native-evaluation trust.
The four increment HIR→Lean/property theorems and the independent branch specification theorem have empty axiom dependencies. Lean's standard `propext` is used by the three increment MIR theorems, two branch correspondence/transfer theorems, and four theorems per loop fixture (correspondence, completion, independent specification, transfer). With frames added to the loop model, the existing loop-specification theorem now also has this dependency. The kernel checks proofs produced by `decide`, and tests require the exact axiom list. No `native_decide` or custom axioms are used.
See Lean's [proof validation](https://lean-lang.org/doc/reference/latest/ValidatingProofs/) and [axiom reference](https://lean-lang.org/doc/reference/latest/Axioms/).

## Stages and completion criteria

| Stage | Content | Completion criterion |
| --- | --- | --- |
| Implemented experiment | Increment HIR→Lean/HIR→MIR, choose if/short-circuiting and acyclic CFGs, plus advance while/for, mutable locals, break/continue inside if, nesting, initialization/updates, cyclic CFGs, and completion within explicit bounds | Prove all-u8 correspondence/properties and reject value/origin/return/branch/short-circuit/condition/update/backedge/fuel/specification mutations. This proves individual snapshots, not the general lowering algorithm |
| Backend foundation | Common IR input, origins, results/traces, generated definitions separated from user specifications | Settle `emit-lean` and support table; port existing success/failure tests |
| Control/numbers | All integer types, booleans, order, functions, branches, loops, explicit conversions | Compare failures, prior output, and termination conditions as well as values; add floats with an explicit model |
| Aggregates/resources | Strings, arrays, products, sums, ownership/budgets | Target existing language semantics and track gaps explicitly |
| User proofs | Preconditions, postconditions, invariants, and termination where needed | Examples trace the subject and assumptions from source through IR to Lean |
| After self-hosting | Treat the compiler as a Cerune program | Specifications/proofs for frontend, IR transformations, and backends, with an explicit bootstrap trust boundary |

The small experiment is not a permanently restricted language subset for Lean.
Its success does not prove the entire Rust frontend, executors, or generator, nor Native output.
