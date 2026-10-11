# Verification experiments

[日本語](README.md)

Models, transformations, and external-tool verification live separately from the language implementation and ordinary Cerune examples. Each folder documents scope, tools, and commands.

| Experiment | Purpose |
| --- | --- |
| [lean](lean/README.md) | Finite-fixture HIR/MIR/SSA correspondence with Lean models, program properties, and execution comparisons |
| [ssa](ssa/README.en.md) | Rust API examples for SSA construction, observation, and direct execution; not a separate language specification replacing public CLI behavior |

There is no LIR direct-execution experiment. The [overall inventory](../docs/design/development-status.en.md) distinguishes implementation, observation, and proof coverage.

GitHub Languages reports source-language statistics, not runtime requirements. [.gitattributes](../.gitattributes) excludes Lean verification files, CJS helpers, and generated-output comparison fixtures from statistics while keeping their diffs visible.

A future Lean emitter implemented in Rust remains implementation code; Lean comparison files belong to verification.
