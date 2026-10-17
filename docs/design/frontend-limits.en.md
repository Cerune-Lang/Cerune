# Compiler input limits

[日本語](frontend-limits.ja.md)

## Purpose and scope

Reject excessively deep syntax or large dependencies with ordinary diagnostics that retain source locations. Checks run in the shared frontend, so the choice of executor/backend does not change them. Evaluation order, types, numeric semantics, and relative-import resolution remain unchanged.

These are compiler acceptance limits, separate from runtime string/array budgets, generic expansion, and import depth. They do not bound all compiler memory, time, or stack usage, promise operation on arbitrarily small host stacks, or validate every externally constructed AST.

## Syntax and expression depth

| Check | Limit and counting | Error location |
| --- | --- | --- |
| Open delimiters | 16 combined levels of `(`, `[`, and `{`; closed delimiters no longer count | Opening token that exceeds the limit |
| Recursive expression parsing | 16 active expression entries plus containing active blocks | Token starting the next expression |
| Expression AST | 32 root-to-leaf nodes plus containing blocks; a leaf counts as 1 | Span of the expression exceeding the limit |
| File-import depth | Existing limit of 128 levels | Import declaration |

Symbols inside strings/comments do not count as delimiters. Array element and argument counts measure width, not expression depth. Parentheses count as delimiters but add no AST node. Function bodies are blocks too.

Delimiter checks alone miss match scrutinees that recurse before opening braces, and deep trees such as `1+1+…` or `a.field.field…`. Unary operators are read iteratively and constructed from the right. Expression trees are checked without recursion, including during binary/index/field/unary construction, rather than only after constructing a huge tree.

Limits are checked using both the Parser and debug CLI through IR generation on Windows/Linux. Containing blocks count because independently allowing both maxima increases actual depth. A failed compilation does not leave counters for another compilation.

Existing ordinary examples remain valid. Previously accepted deep input may now produce a diagnostic. Users can explicitly split long expressions into typed intermediates or functions; the compiler introduces no hidden rewrite or reassociation.

## Filesystem-source budgets

`modules::load` and the CLI use these defaults. `modules::load_with_limits` accepts an explicit `LoadLimits` chosen before loading. No CLI flags or environment overrides are added.

| Quantity | Default |
| --- | --- |
| Individual file | 8 MiB (8,388,608 bytes) |
| Total source text | 64 MiB (67,108,864 bytes) |
| Distinct files | 1,024, including the entry |

Count UTF-8 bytes, including comments, whitespace, NUL, LF, and CRLF, without normalization or newline translation. Charge each canonical path once; diamond dependencies are not reread. Different hard-link paths remain distinct files.

Read at most the smaller of the per-file limit and total remaining budget, plus one probe byte. Exact-limit input is accepted after confirming EOF; another byte causes a limit diagnostic. File-size metadata alone is not trusted. Text that exceeds limits, fails reading, or is invalid UTF-8 is not registered in SourceMap; loading does not continue with later files.

| Failure | Diagnostic |
| --- | --- |
| Entry read/limit failure | Identify its path; do not invent SourceId/Span for unregistered text |
| Imported read/limit failure | Retain the importing source's SourceId and declaration Span |
| Syntax limit in successfully read text | Its SourceId/Span; preserve anonymous-source behavior for single-file CLI inputs |
| CLI failure with an output path | Preserve the existing artifact |

Bundled `std:io` is a virtual source without filesystem lookup, so it does not spend this file budget. Text/origin observation and type identity remain unchanged. Already-supplied memory input to `compile(&str)`, `modules::from_source`, and similar APIs is outside the filesystem budget; syntax/expression checks still apply. Embedders can limit their input acceptance before calling these APIs.

```rust
use cerune_lang::modules::{load_with_limits, LoadLimits};
use std::path::Path;

let compilation = load_with_limits(
    Path::new("main.ceru"),
    LoadLimits {
        max_file_bytes: 1024 * 1024,
        max_total_bytes: 8 * 1024 * 1024,
        max_files: 64,
    },
);
```

A zero-byte budget accepts empty files. A zero-file budget rejects the entry. These budgets measure logical source bytes, not allocator capacity, tokens/AST/IR overhead, OS RSS, or blocking-read time. They do not add sandbox restrictions on ordinary parent-directory imports.


File traversal also uses an explicit stack. Parsing a leaf in a 128-level import chain does not retain a Rust call frame for every ancestor. Registration order remains depth-first.

## Validation and follow-up

`tests/frontend_limits/mod.rs`, included by `tests/modules.rs`, runs child processes with ten-second limits. It checks boundaries and excess for parentheses, types, blocks, arrays, unary/binary/logical operators, and fields/indexes. Wide arrays, symbols inside strings/comments, recursion without nested delimiters, and combined block/expression depth are included.

File tests cover UTF-8/CRLF, exact-limit EOF, one-byte excess, individual/total/count limits, duplicate paths, standard modules, invalid UTF-8, diagnostic origins, and preservation of CLI artifacts. [Existing module examples](../../examples/modules/README.en.md) and single-file examples retain execution comparisons.

This addresses syntax/source-loading limits in [#135](https://github.com/Cerune-Lang/Cerune/issues/135) and [#153](https://github.com/Cerune-Lang/Cerune/issues/153). Import isolation, generated-C NDEBUG checks, CI dependency pinning, and ongoing fuzzing remain separate #153 work. Parser limits alone do not complete resource controls for later semantic processing or synthesized ASTs.
