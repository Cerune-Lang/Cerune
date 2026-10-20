# Virtual standard modules

[日本語](standard-modules.ja.md)

The public line-input API uses `import "std:io" as io;` and `io::read_line(4096)`. Standard modules are virtual sources bundled into the compiler, never searched on disk. print and flush are unchanged.

## Resolution and identity

- Classify `std:` imports as standard modules. The exact identifiers `std:io` and `std:env` are recognized; unknown identifiers fail without filesystem fallback.
- Preserve existing relative `.ceru` resolution. Add no environment variables, search paths, network downloads, or general package manager.
- Register each standard module once per compilation. Aliases and imports from multiple user files share one ordinary enum definition across all backends.
- `modules::from_source` resolves in-memory source and bundled modules only, rejecting relative file imports. compile(&str) and compile_to_ir(&str) use this path without implicit file I/O. APIs taking a single SourceFile preserve their existing behavior without resolving imports; use modules::from_source when the full SourceMap including standard sources is needed.

## SourceId and observation

SourceIds are compilation-local registration-order identifiers: register the entry first, then traverse dependencies in declaration order. Do not reserve a fixed numerical ID for std:io or assume IDs match across compilations.

SourceMap distinguishes user and standard sources. Standard sources use identifiers std:io or std:env, both with contract version 1. emit-sources includes its text, SourceId, kind, identifier, and version, without inventing a filesystem path. Preserve anonymous spans and manifests for existing single-source inputs without standard modules.

The input operation's Span refers to its location in the bundled function. The caller's HIR Call retains the user's Span, allowing observation of the relationship between caller, function, and input operation. Never render a virtual-source diagnostic against the same offsets in a user file.

## Line/EOF and common IR

Public enum ReadLine has Line { text: string } and End. Its bundled function body remains observable. Only reading itself becomes a common IR operation; ordinary enum/match and ownership lowering are reused.

The internal operation returns strict UTF-8 text; an empty string represents confirmed EOF. Blank lines remain nonempty because LF/CRLF are preserved, allowing the bundled wrapper to convert this representation into Line/End. The internal operation is not a public user name; resolve the dedicated call in bundled source into an internal AST name.

Preserve the [line-input contract](line-input.en.md), including limits, consumption, failures, budgets, and origins. That document tracks route support; unsupported routes must not substitute success or EOF.

## Execution and checks

Pass Input and Output explicitly to HIR/MIR/SSA `run_with_io` or VM `run_bytecode_with_io`. Existing capture APIs keep input unconfigured. [CLI and generated C/LLVM/QBE/Native](stdin-connections.en.md) connect actual stdin. [Versioned WAT input hosts](wat-input-host.en.md) are also implemented. Native lowers input from ordinary/SSA MIR through LIR, ASM, and self-encoded Object. Unused functions are retained, so importing std:io alone includes input support in generated artifacts.

The [Cerune line-input fixture](../../tests/fixtures/line_input.ceru) executes print → flush → read_line → match. [Differential tests](../../tests/standard_input.rs) supply independent identical inputs to four executors and check [expected output](../../tests/fixtures/line_input.stdout), operation order, failures, and ownership. A [real stdin example](../../examples/stdin/README.en.md) is also tested across all routes.

```sh
cargo test --test standard_input
cargo run -- emit-sources tests/fixtures/line_input.ceru
cargo run -- emit-ir tests/fixtures/line_input.ceru
```

## Program arguments: std:env

Bundled arg_count/arg/Argument declarations reuse the same SourceMap, name resolution and enum lowering. HIR/MIR/SSA/VM, CLI and generated C are connected; LLVM/QBE/WAT/Native diagnose unsupported operations. See the [contract and remaining work](dogfooding-inputs.en.md) and [example](../../examples/arguments/README.en.md).
