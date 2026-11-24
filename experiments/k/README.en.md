# Checking short circuits with K

[日本語](README.md)

Run the [branch fixture](../lean/branch.ceru)'s `choose(value: u8)` on every u8 input. At 128 the left operand `value < 128` is false, so the function skips the addition on the right and returns 130. At 254 it skips the same operand and overflows in the selected else arm's `value + 2`.

| Input | Selected arm | Outcome |
| --- | --- | --- |
| 0–126 | then | input + 1 |
| 127–253 | else | input + 2 |
| 254–255 | else | `integer-overflow` at the else addition's NodeId and span |

## Comparison

1. The [translator](main.rs) reads the completed HIR for the body of `choose`. It translates the parameter, literals, addition, comparison, short circuit, branch and returns into a K term and carries their source sites from HIR.
2. The [K rules](choose.k) execute the term for inputs 0 through 255. Binary arithmetic evaluates left to right. A true left operand of `andThen` selects the right operand. An overflow discards the pending computation and records the addition site.
3. The [comparison](verify.py) checks values or stop codes and sites against the evaluated [Lean specification](../lean/BranchProperties.lean). HIR, MIR, SSA and VM run the same function. The comparison reads their stdout and matches their `trace-v1` branch, return and stop sites with K.
4. A second run changes a copy of the short-circuit rule to evaluate the right operand when the left operand is false. At 128 this produces overflow at `value + 128`.

## Reproduce

Use K 7.1.337, Lean 4.34.1, the project's Rust toolchain and Python 3.10 or later on Linux. Run from the repository root. Set `CERUNE_TEST_LEAN` to select a particular Lean executable.

```sh
python3 experiments/k/verify.py
```

Each run writes a fresh directory under `target/k-choose-*` containing the HIR term, final K configurations, the Lean table and JSON Lines for all four executors. Normal compiler use requires only the built Cerune binary. [Linux verification steps](../../.github/workflows/k-choose.yml)
