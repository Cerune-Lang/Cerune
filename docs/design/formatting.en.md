# Optional let and source formatting

[日本語](formatting.ja.md)

## Scope and contract

`let` is optional spelling that makes bindings easier to find. Accept it on ordinary bindings, for initializers and each array-iteration binding, before optional `mut`. The colon remains mandatory. Bindings with initializers also support the [infer-omission follow-up](infer-omission.en.md); array iteration still requires a type specification (including `infer`). Else-if and conditional expressions remain unimplemented.

`let` is contextual, not reserved: at a binding position it is a prefix only when followed by an identifier or `mut`. Existing identifiers in `let: i64 = 1;`, `let(...)` and `let = ...` remain valid; `let let: i64 = 1;` is valid too. Update clauses, constants, parameters and fields do not accept the prefix.

The parser builds the existing binding AST. `parse_module_with_syntax` separately returns binding-prefix spelling and original Spans. Semantic analysis, IR and executors do not branch on the presence of let.

## Initial formatter

Use `cerune fmt <file> [--let preserve|show|hide] [--check | --write | -o <file>]` on one file.

| Selection | Behavior |
| --- | --- |
| Default | Write formatted source to stdout without changing the input |
| `--let preserve` | Keep original prefix choices (default) |
| `--let show` / `hide` | Add/remove binding prefixes; never change identifiers named let |
| `--check` | Exit 1 if formatting differs, without changing files or stdout |
| `--write` | Replace the explicitly named input |
| `-o` | Save to the chosen output; incompatible with check/write |

The first version standardizes four-space indentation, spacing around punctuation/operators, block line breaks and a final newline. Keep constructors, match patterns and enum payloads compact; distinguish type-argument angle brackets from comparison operators. Empty input stays empty. It offers no line-width selection, configuration files, import sorting or recursive formatting. Preserve at most one blank line.

Comments and string/numeric literal spellings are copied from source. Japanese text, escapes, NUL and CR/LF inside strings are unchanged. Code uses LF line endings. Line-comment contents are preserved; a terminal CR is treated as a line ending.

Formatting validates syntax only: it does not read imports, infer types or execute code. Files with unavailable dependencies can be formatted. Invalid syntax/UTF-8, excessive input and invalid options do not write output files. File input uses the existing loader default per-file limit (8 MiB). Saving uses a temporary file in the same directory and refuses to replace symbolic links.

## Semantics and origins

Re-lex and re-parse the result before returning it, then compare token streams after excluding only actual binding prefixes. Preserve comments from the original gaps between tokens. Do not add another interpretation of the language or regenerate source from an AST that has lost original spelling.

Never reuse original Spans for formatted text. Compile each source separately to observe its own SourceId/Span and generation. Annotated artifacts and locations need not be byte-identical; semantics, types, order, short circuiting, copies and failure reasons must agree. Formatting is an explicit transformation; observation APIs such as emit-sources remain read-only.

Format existing examples under all three policies and check idempotence, comments/tokens, IR meaning, all execution/generated routes and failure positions in both sources. CLI tests cover explicit writes, check mode and preservation of existing files on errors.
