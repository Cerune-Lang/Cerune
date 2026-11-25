# Contributing

[日本語](../CONTRIBUTING.md) | English

Thank you for your interest in Cerune. Bug reports, tests, documentation fixes, small improvements, and verification experiments are welcome. Cerune is an experimental language developed as a personal project, so its specification and implementation may change.

Please also read the [Code of Conduct](CODE_OF_CONDUCT.en.md) before participating. You do not need to understand every execution route or internal representation before contributing.

## Getting Started

- See the [README](../README.en.md) for usage and the [examples](../examples/README.en.md) for language features.
- The [language reference](../docs/reference/language.en.md), [CLI reference](../docs/reference/cli.en.md), and [design docs](../docs/README.en.md) describe the current specification and design decisions.
- Check the [roadmap](../docs/design/language-roadmap.en.md), [milestones](https://github.com/Cerune-Lang/Cerune/milestones), and [existing issues](https://github.com/Cerune-Lang/Cerune/issues) before starting. For larger features or design changes, please discuss the approach in an issue first.

## Development Principles

Cerune prioritizes keeping the meaning of a program and its transformation into executable representations observable. Preserve the following properties where relevant to your change.

- Resolve language semantics and types in the common frontend. Make backend-specific decisions at explicit lowering boundaries.
- Keep evaluation order, arithmetic rules, value copying and updates, stop conditions, and external operations consistent across supported execution routes. Document differences in supported features.
- Preserve the correspondence between source code, IR, and generated code. Do not use `Synthetic` to replace lost origin information.
- Separate observation from control. Observing a program must not change its semantics or execution state.
- Treat optimization as an explicit, observable stage.
- Distinguish implemented features, design proposals, and the scope of verification.

See the [compiler architecture](../docs/design/architecture.en.md), [observability contract](../docs/design/observability.en.md), and [output routes and targets](../docs/design/targets.en.md) for details.

## Reporting Bugs and Proposing Changes

When opening an issue, please include as much of the following as possible:

- What you tried (the command, a minimal `.ceru` reproducer, and required inputs)
- Expected behavior versus actual output, exit status, and error messages
- Cerune version or commit, operating system, and architecture
- Relevant execution route, target, options, and external tool versions, if known
- For differences between routes, the observed difference and relevant diagnostics, IR, or traces, if available

Cerune has four executors: direct HIR execution (`run` / `run-ir`), MIR (`run-mir`), SSA (`run-mir --ssa`), and VM (`run-vm`). It also has C, LLVM, QBE, WAT, and Native code generation routes. You do not need to test every route. The [CLI reference](../docs/reference/cli.en.md) explains stop diagnostics, `--trace`, `observe`, and the generation routes.

Observation artifacts may contain source text, paths, or other data. Check them before sharing and remove secrets or data that cannot be made public. For security concerns, do not open a public issue. Follow [SECURITY.md](../SECURITY.md) to report them privately.

## Development Environment

Development uses Git, Rust, and Cargo. The toolchain version is pinned in [rust-toolchain.toml](../rust-toolchain.toml).

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

Basic execution does not require an external C compiler. Depending on the generation route or verification task, you may need Clang/GCC, QBE, WABT, Node.js, and the target OS linker or SDK. See the [external tools in the README](../README.en.md#external-tools) for details.

Tests using LLVM require Clang with opaque-pointer (`ptr`) support. On Linux/WSL, for example, check Clang 18 as follows. Adjust the path to your environment.

```sh
export CERUNE_TEST_LLVM_CLANG=/usr/bin/clang-18
bash scripts/check-llvm-toolchain.sh
cargo test --lib
```

The [preflight check](../scripts/check-llvm-toolchain.sh) prints the version and compiles a small LLVM IR module. `bash scripts/test.sh` also runs it when Clang is available or explicitly configured. An incompatible compiler or a missing explicitly configured executable is not treated as a passing test or silently skipped. The environment variable applies to the current shell; set it again in a new shell.

## Tests and CI

Basic checks before and after code changes:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

Run tests relevant to your changes.

| Purpose | Command |
| --- | --- |
| Expected results of examples | `cargo test --test examples` |
| CLI output, including execution traces | `cargo test --test execution_output_cli` |
| Formatting, Clippy, and all test targets (Bash/WSL) | `bash scripts/test.sh` |

For changes affecting execution or code generation, compare expected results across the relevant routes, including stop reasons, source locations, and execution traces where appropriate. For bug fixes, add a regression test that detects the original problem where possible. When comparing runs with external side effects, use equivalent inputs and environments and isolate output locations and other affected resources.

[CI](../.github/workflows/ci.yml) covers supported routes on Linux, Windows, and macOS, as well as Lean verification, host calls, subprocesses, runtime distribution checks, and more. The `ci-gate` aggregates the results of the required job groups within CI. [Dependency Review](../.github/workflows/dependency-review.yml) checks for newly introduced known vulnerabilities of high or critical severity.

Some integration tests require external tools or specific operating systems. You do not need to set up every environment. Describe checks you could not run locally, skipped checks, and the reasons in your PR.

## Verification and Technical Experiments

In addition to language implementation changes, differential testing across execution routes, comparisons with semantic models, observation of generated code, and performance measurements are welcome. [Verification experiments](../experiments/README.en.md) are kept separate from ordinary executable examples.

Record the target commit, operating system and architecture, tool versions, inputs and seeds (where used), reproduction steps, expected results or baselines, observed results, and verification scope. Results that contradict a hypothesis or cannot be reproduced are also useful for future work.

Do not generalize verification of finite examples into a guarantee for the entire compiler. State what was actually verified. Keep detailed reproduction instructions in each experiment's README.

## Documentation

When changing specifications or design decisions, update the corresponding Japanese and English documents, and examples and expected results where appropriate. The [documentation index](../docs/README.en.md) describes translation pairs and link checks.

With Node.js installed, run the following from the repository root:

```sh
node scripts/check-docs.cjs
node --test scripts/check-docs.test.cjs
```

Automated checks cover local links, translation pairs, and related conventions. Also check that the Japanese and English content agrees and accurately describes the current implementation.

## Pull Requests

- Keep each PR focused on one purpose where possible.
- Explain what changed and why, which commands or tests you ran, and any related issues.
- For changes affecting execution or code generation, add or update relevant tests, examples, and documentation.
- Be clear about checks you could not run and any known limitations.
- Share only necessary observation artifacts and experiment logs, and check that unintended generated files or secrets are not committed.

Small fixes and questions are welcome, too.
