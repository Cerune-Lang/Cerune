# Lean verification experiment

[日本語](README.md)

This precedes a public backend. It verifies the u8 functions `increment`, `choose`, `advance`, and `swap` from actual completed HIR/MIR and SSA/reconstructed MIR, not printing callers or other functions. [Design](../../docs/design/lean-verification.en.md).

## Files

Proofs, inputs, and generation tools live here; `examples/` remains the Cerune program list.

| File | Purpose |
| --- | --- |
| [increment.ceru](increment.ceru) | Checked u8 increment |
| [Model.lean](Model.lean) | HIR parameters, constants, addition, overflow, and origins |
| [MirModel.lean](MirModel.lean) | Independent typed locals, instructions, and CFG semantics |
| [branch.ceru](branch.ceru) · [BranchModel.lean](BranchModel.lean) | If/short-circuit fixture and HIR reference model |
| [branch.rs](branch.rs) · [BranchProperties.lean](BranchProperties.lean) | HIR branch exporter and independent properties |
| [emit.rs](emit.rs) | HIR reference terms and direct Lean code |
| [mir.rs](mir.rs) | Exports the supplied MIR snapshot, without rebuilding it from HIR |
| [Properties.lean](Properties.lean) | Independent properties and transfer to HIR/MIR |
| [loop.ceru](loop.ceru) · [LoopModel.lean](LoopModel.lean) | Four increments and structured HIR while/local semantics |
| [LoopCorrespondence.lean](LoopCorrespondence.lean) | Shared loop correspondence and completion theorems |
| [loops.rs](loops.rs) · [LoopProperties.lean](LoopProperties.lean) | Exporter with explicit bounds and plain-while specification |
| [loop_control.ceru](loop_control.ceru) · [LoopControlProperties.lean](LoopControlProperties.lean) | Break/continue inside if; body/return failure origins |
| [nested_loop_control.ceru](nested_loop_control.ceru) · [NestedControlProperties.lean](NestedControlProperties.lean) | Innermost-loop behavior |
| [for_control.ceru](for_control.ceru) · [ForProperties.lean](ForProperties.lean) | Initialization, continue through update, break skipping update |
| [for_update_failure.ceru](for_update_failure.ceru) · [ForFailureProperties.lean](ForFailureProperties.lean) | Expected failure distinguishing update and body overflow |
| [main.rs](main.rs) · [lean-toolchain](lean-toolchain) | Development tool and pinned checker |

## Commands

Run from the repository root; the example prints `1 → 42 → 255`.

`branch.ceru` prints `1 → 127 → 129 → 130 → 255`. The condition `value < 128 && value + 128 < 255` skips an overflowing RHS for inputs ≥128. The complete input specification follows.

| Input | Result |
| --- | --- |
| 0–126 | `input + 1` |
| 127–253 | `input + 2` |
| 254–255 | Overflow at the else-side `value + 2` |

`loop.ceru` prints `4 → 5 → 254 → 255`. `advance` increments four times: inputs 0–251 return `input + 4`; inputs 252–255 overflow during `current + 1`.

The added control fixtures have the following complete input specifications and sample outputs.

| Fixture | Full specification | Output |
| --- | --- | --- |
| `loop_control.ceru` | +6 through 249; return overflow at 250–253; body overflow at 254–255 | `6 → 7 → 254 → 255` |
| `nested_loop_control.ceru` | +2 through 253; inner-body overflow at 254–255 | `2 → 3 → 254 → 255` |

The two for fixtures have the following expected results.

| Fixture | Specification | Sample |
| --- | --- | --- |
| `for_control.ceru` | +5 through 250; return overflow at 251–253; body overflow at 254–255 | `5 → 6 → 254 → 255` |
| `for_update_failure.ceru` | Update overflow after continue through 254; body overflow at 255 | Input 1 fails in update, no output, exit 1 |

The failure example is expected to fail at runtime. Tests pass only when its failure code and origin match the specification.

`Cargo --example lean_verification` names a Rust development tool, not a language backend. Normal Cerune execution needs no Lean. The `+toolchain` form uses elan; a preinstalled matching binary can be selected with `CERUNE_TEST_LEAN`. Tests never auto-install Lean.

```sh
cargo run --quiet -- run experiments/lean/increment.ceru
cargo run --quiet -- run-mir experiments/lean/increment.ceru
cargo run --quiet -- run-vm experiments/lean/increment.ceru
cargo run --quiet -- run experiments/lean/branch.ceru
cargo run --quiet -- run-mir experiments/lean/branch.ceru
cargo run --quiet -- run-vm experiments/lean/branch.ceru
cargo run --quiet -- run experiments/lean/loop.ceru
cargo run --quiet -- run-mir experiments/lean/loop.ceru
cargo run --quiet -- run-vm experiments/lean/loop.ceru
cargo run --quiet -- run experiments/lean/loop_control.ceru
cargo run --quiet -- run-mir experiments/lean/loop_control.ceru
cargo run --quiet -- run-vm experiments/lean/loop_control.ceru
cargo run --quiet -- run experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run-mir experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run-vm experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run experiments/lean/for_control.ceru
cargo run --quiet -- run-mir experiments/lean/for_control.ceru
cargo run --quiet -- run-vm experiments/lean/for_control.ceru
# Expected exit 1.
cargo run --quiet -- run experiments/lean/for_update_failure.ceru
cargo run --quiet --example lean_verification
lean +leanprover/lean4:v4.34.1 target/lean-verification/Verified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/BranchVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/LoopVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/LoopControlVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/NestedControlVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/ForVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/ForFailureVerified.lean
cargo test --test lean_verification
cargo test --test lean_verification -- --include-ignored
```

Outputs under `target/lean-verification` are `Generated.lean` (models, reference terms, generated function, correspondence), `Verified.lean` (independent properties appended), `increment.ceir`, `increment.mir.txt`, and `lean-toolchain`. Inspect source representations, instruction order/locals, and theorem assumptions together. Neither the generated function nor the MIR evaluator calls HIR `eval`.

The same directory also contains `BranchGenerated.lean`, `BranchVerified.lean`, `branch.ceir`, and `branch.mir.txt`, exposing structured HIR conditions alongside their MIR block/jump/branch lowering.

The loop fixture adds `LoopGenerated.lean`, `LoopVerified.lean`, `loop.ceir`, and `loop.mir.txt`; generated definitions retain `hirFuel = 16` and `mirFuel = 11`.

Control fixtures also save `LoopControl{Generated,Verified}.lean` / `loop_control.{ceir,mir.txt}` and `NestedControl{Generated,Verified}.lean` / `nested_loop_control.{ceir,mir.txt}`. Their definitions record the following bounds.

| Fixture | Statement visits | Block visits |
| --- | --- | --- |
| Plain while | 16 | 11 |
| break/continue | 22 | 22 |
| Nested loops | 28 | 23 |

For artifacts are `For{Generated,Verified}.lean` / `for_control.{ceir,mir.txt}` and `ForFailure{Generated,Verified}.lean` / `for_update_failure.{ceir,mir.txt}`. Their bounds are 23 HIR statements/25 MIR blocks and 8/5 respectively.

Each definition is checked with `LoopCorrespondence.lean` appended. Execution and below-bound tests use the exact definition prefix from that checked file, omitting redundant correspondence checks; they never regenerate a different proof subject.

## Proofs and tests

Proof subjects and dependencies are explicit.

| Subject | Claim | Axioms |
| --- | --- | --- |
| HIR→direct Lean | Correspondence for every valid input, exact addition below 255 and transfer to HIR, overflow at 255 | Existing four: none |
| HIR→MIR | `∀ x : Fin 256, evalMir mirReference x.val = .completed (eval reference x.val)` | `propext` |
| properties | Transfer the same exact addition and overflow | `propext` |
| Branch HIR→MIR | For all 256 inputs, `evalMir = .completed (evalBranch …)` | `propext` |
| Branch specification | Prove the input table above for HIR | None |
| Branch MIR property | Transfer the input-table specification through correspondence | `propext` |
| Loop correspondence, completion, MIR property | Same result and completion within each explicit bound for every input | `propext` |
| Loop specifications | Input-specific while, break/continue, nesting, and for specifications | `propext` |

With frames in the HIR model, the plain-while specification theorem also uses `propext`, whereas it previously had no dependencies. Checks still require the exact current axiom list.

The MIR theorem uses `decide` over the finite domain and is checked by Lean's kernel. `propext` is Lean's standard propositional-extensionality axiom used by finite quantification/equality decision procedures. Tests require the exact declared dependencies; `sorryAx`, custom axioms, and additional native-evaluation trust are not accepted. Runtime output comparison alone does not produce the proof.

Tests compare all 256 inputs across HIR, MIR, VM, direct Lean, the Lean MIR model, and known expectations, including overflow FailureCode/NodeId/SourceId/Span and empty prior output. With HIR fixed, changing MIR literals, NodeIds, SourceIds, spans, or returns breaks the proof. Invalid instruction order and unsupported operations are rejected before export. Existing generated-value/origin/user-specification mutation tests remain.

The branch fixture also compares all 256 inputs across HIR, MIR, VM, Lean HIR/MIR models, and known expectations. Checks reject swapped branch targets/comparison operands, eager RHS execution, altered failure NodeIds, and changing the independent specification's 127 boundary. A variant using `||` also checks correspondence.

The loop fixture also compares all 256 inputs. Checks reject changed iteration counts, update stores, backedges, failure NodeIds, a self-looping condition block, and the independent specification's 252 boundary. Reducing either bound by one or setting both to zero is rejected; matching exhaustion is not success. Mutations run only in bounded models, never as potentially nonterminating executables.

Both control fixtures compare all 256 inputs. Checks reject continue-to-exit, break-to-condition, inner-to-outer transfers, swapped if targets, and changed specification boundaries. They also check exhaustion immediately below each bound and invalid break/continue outside a loop.

The for fixtures also compare values/failure origins for all 256 inputs. Checks reject skipped initialization, continue/body completion bypassing update, break executing update, altered update failure NodeIds, and changed specification boundaries. Finishing mutants are checked with an increased MIR bound.

Lean execution is explicitly ignored in ordinary runs and mandatory in the dedicated CI job. Proofs, rejected mutations, and MIR observations remain in `target/lean-verification-test`.

## Limits and trust

`Nat` is a mathematical domain: inputs are `Fin 256`, constants are 0–255, and each addition checks its range. This does not turn `u8` into an unbounded integer. MIR exports u8/bool literals, copy/scalar store/add/less, jump/branch/return/unreachable. Straight-line/branch exporters reject cyclic CFGs, including unreachable blocks. Only the loop experiment allows cycles, with separate explicit bounds and a completion proof.

HIR frames store the innermost loop first, with its condition restart and statements after the loop. An if adds no frame. While body completion and continue resume the condition. For a for-loop they execute the update once before retesting. Break skips the update and resumes after that loop. Frame restoration is administrative and consumes no statement fuel. HIR frames are not derived from MIR targets.

`forLess` retains initialization, condition, update, and body. Internal `forNext` repeats after initialization without rerunning it. HIR fuel counts for entry, initialization, each condition test, and update separately.

For acyclic fixtures, the MIR model uses block count as fuel, which bounds every path. Correspondence checks that every input reaches `.completed`. `.completed (.error …)` is a Cerune failure, `.invalid` an invalid state, and `.exhausted` insufficient fuel. The last two are neither language failures nor proof success.

The loop fixtures count HIR statement visits (including while/if tests and break/continue) and MIR block visits. Expressions and finite block instruction lists are evaluated structurally. Bounds 16 and 11 have different units, not a shared fuel scale or time measurement. Completion is proved for all 256 inputs, not for arbitrary loops or an algorithm that discovers bounds.

General loop termination, calls, output traces, heap/ownership, other integers, floats, and other features are not covered. Use these finite fixtures as an unoptimized MIR baseline; the experiment below checks finite SSA fixtures. Track heap and other proof extensions separately, without fixing a permanent language subset.

The Rust HIR/MIR-to-model correspondence, exporters, Lean kernel, and declared axioms remain trusted. This is translation validation of individual actual lowering results, not proof of the general lowering algorithm, whole compiler, or Native output.

## SSA verification

Actual baseline MIR, SSA, and reconstructed MIR snapshots are exported separately. Independent SSA control reads all incoming values before binding block arguments; operations and instruction evaluation share the MIR model.

The added files have these roles.

| File | Purpose |
| --- | --- |
| [ssa.rs](ssa.rs) · [SsaModel.lean](SsaModel.lean) | Actual SSA exporter and independent control semantics |
| [SsaCorrespondence.lean](SsaCorrespondence.lean) | Correspondence and completion for every input |
| [ssa_join.ceru](ssa_join.ceru) · [SsaJoinProperties.lean](SsaJoinProperties.lean) | Join and independent specification; prints 3 → 130 → 130 → 255 |
| [ssa_swap.ceru](ssa_swap.ceru) · [SsaSwapProperties.lean](SsaSwapProperties.lean) | Three swaps and independent specification; prints 0 → 127 → 255 |
| [lean_ssa.rs](../../tests/lean_ssa.rs) | All-input proofs, execution comparisons, and mutation rejection |

Ten cases (seven existing fixtures, join, swap, and direct header-argument swapping via the SSA API) each cover all 256 inputs. Bounds are 64 block visits for baseline MIR/SSA and 128 for reconstructed MIR; a separate theorem requires success or language-level overflow by proving baseline completion and transferring it through the checked correspondences, retaining failure code and NodeId/SourceId/Span. Printing callers, heap, general termination, and the entire transformer are outside the proof scope.

Run from the repository root.

```sh
cargo run --quiet -- run experiments/lean/ssa_join.ceru
cargo run --quiet -- run-mir experiments/lean/ssa_join.ceru --ssa
cargo run --quiet -- run experiments/lean/ssa_swap.ceru
cargo run --quiet -- run-mir experiments/lean/ssa_swap.ceru --ssa
cargo test --test lean_ssa
# Requires the pinned Lean toolchain.
cargo test --test lean_ssa -- --include-ignored
```

Normal tests check export scope and sample execution across five routes without Lean. The ignored tests use Lean 4.34.1 to check proofs and rejected mutations, then compare execution of all three Lean models against HIR, baseline MIR, SSA, reconstructed MIR, and VM for every input. CI explicitly runs them. Every Lean process has a 120-second limit; timeouts fail validation.

Observations live under `target/lean-verification-test/ssa`: each case's `.ceru` source, `.ceir` HIR, baseline MIR, SSA, reconstructed MIR, both mappings, `Generated.lean`, `Verified.lean`, `Execute.lean`, rejected cases, process output, and toolchain version. Only `Parallel` changes the generated SSA backedge to an equivalent direct exchange through the API; the SSA snapshot and mapping expose that change.

Correspondence, completion, and added property theorems use `decide`, kernel checking, and exact `#print axioms` list checks (standard `propext` only). No `native_decide` or custom axioms. Rust exporters and shared operation semantics remain trusted; these finite-fixture proofs are distinct from general semantic preservation. See the [SSA design](../../docs/design/mir-ssa.en.md#finite-fixture-lean-verification).
