# Contributing

[日本語](CONTRIBUTING.md) | English

Thank you for your interest in Cerune. Bug reports, tests, documentation fixes, and small improvements are welcome. Cerune is an experimental language under active development, so its specification and implementation may change.

## Getting Started

- See the [README](README.en.md) for usage and the [examples](examples/README.en.md) for language features.
- The [language reference](docs/reference/language.en.md) and [design docs](docs/README.en.md) describe the current specification and design decisions.
- Check [existing issues](https://github.com/Cerune-Lang/Cerune/issues) before starting. For larger features or design changes, please discuss the approach in an issue first.

## Reporting Bugs and Proposing Changes

When opening an issue, please include as much of the following as possible:

- What you ran (the command and a minimal `.ceru` reproducer)
- Expected behavior versus actual output or error messages
- Cerune version or commit, operating system, and architecture
- Relevant execution route or target, if known

Cerune has several execution routes, including `run` / `run-ir`, `run-mir`, and `run-vm`. If their results differ, describing which route produced which result helps us investigate. You do not need to test every route.

For security concerns, do not open a public issue. Follow [SECURITY.md](SECURITY.md) to report them privately.

## Development and Checks

Development uses Rust and Cargo. The toolchain version is pinned in [rust-toolchain.toml](rust-toolchain.toml).

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

Basic checks before submitting changes:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

Please also run tests relevant to your changes. Some integration tests require external tools; if you cannot run them locally, mention that in your PR. CI covers a broader set of environments and execution routes.

For documentation changes, update both Japanese and English where appropriate. With Node.js installed, use `node scripts/check-docs.cjs` to check links and translation pairs.

## Pull Requests

- Keep each PR focused on one purpose where possible.
- Explain what changed and why, which commands or tests you ran, and any related issues.
- For changes affecting execution or code generation, add or update tests or examples for the relevant routes.
- Be clear about checks you could not run and any known limitations.

Small fixes and questions are welcome, too.
