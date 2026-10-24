# Omitting infer in bindings

[日本語](infer-omission.ja.md)

## Implemented scope

`value: = expression;` abbreviates `value: infer = expression;`. The colon remains required to distinguish declaration from assignment; optional `let` and `mut` can be used. Explicit types and explicit `infer` remain supported.

| Position | Spelling and support |
| --- | --- |
| Ordinary binding (top level, function or block) | Accept `let mut value: = 1;` |
| For initializer | Accept `for (mut i: = 0; i < 3; i = i + 1) {}` |
| Array iteration | No initializer `=`: retain `value: infer in values`. Reject `value: in` and `value: = in` |
| Constants, parameters, results and fields | Keep required types; do not extend omission |
| For update | Assignment only, as before; reject declarations |

Ordinary bindings and for initializers share `name: type = expression`, so they share the omission rule. Iteration inference comes from its subject and remains separate from omitting infer before an initializer. Else-if and conditional expressions are excluded.

## Semantics and observation

At a binding position, the parser returns existing `TypeSpec::Infer` when `=` follows the colon. Leave `=` for initializer parsing; do not invent an infer token or Span. Type resolution, HIR, MIR, SSA, VM and backends gain no separate semantics for omission.

Default integer types, numeric suffixes, aggregates, evaluation order, short circuiting, copying, mutability and existing type errors are identical to explicit infer. Inspect resolved types with `emit-ir` as before, and original spelling with `emit-sources`. Preserve each source’s NodeId/Span and failure locations.

Formatting preserves both omitted and explicit spelling. `--let` only changes let prefixes; it neither inserts nor removes infer. An `--infer` policy or writing resolved types back to source is not implemented.

## Verification

- Replace explicit infer with equal-width spaces to isolate semantics from offset changes, then compare complete AST and IR.
- Compare the [example](../../examples/formatting/infer.ceru), its explicit-infer counterpart and each formatter policy across HIR, MIR, SSA, VM, C, LLVM, QBE, WAT, ASM and Object.
- Test invalid omission positions, type errors, prior output and stop reasons, original/formatted diagnostic positions, retained comments and formatting idempotence.

This implements infer omission from [#105](https://github.com/Cerune-Lang/Cerune/issues/105). Else-if, conditional expressions and other follow-up items remain; the entire issue is not complete.
