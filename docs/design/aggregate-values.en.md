# Aggregate comparison, display, and match expressions

[日本語](aggregate-values.ja.md)

**Status: implemented**

## Operations

| Operation | Contract | Example |
| --- | --- | --- |
| `==` / `!=` | Compare arrays, products, and sums of the same type by contents | [aggregate_comparison](../../examples/aggregate_comparison.ceru) |
| `print(value);` | Display complete, nested values | [aggregate_display](../../examples/aggregate_display.ceru) |
| Match expressions | Return the selected arm's expression | [match_values](../../examples/match_values.ceru) |
| Guards | Explicit `if` condition following the pattern | [match_guards](../../examples/match_guards.ceru) |

```cerune
enum Lookup { Found { text: string }, Missing }
value: Lookup = Lookup::Found { text: "空" };
label: string = match value {
    Lookup::Found { text: text } if byte_len(text) > 0 => text,
    Lookup::Found { text: _ } => "空欄",
    Lookup::Missing {} => "未登録",
};
print(value); // Found{text: "空"}
print(label); // 空
```

## Equality

Evaluate both operands once, left to right, then compare their copies. Arrays compare in index order; products compare in declaration order, stopping at the first unequal member. Sums compare tags first and then only the active payload. Inactive internal storage is not part of the value's meaning. `!=` negates this result.

Existing numeric, boolean, and exact UTF-8 string equality applies recursively. A value containing NaN remains unequal to itself; positive and negative zero compare equal. Unicode is not normalized.

Array element types and lengths must match, and named types retain nominal identity. Existing contextual typing of untyped literals still applies. Aggregate ordering is not introduced. Equality also works in constant expressions and explicitly instantiated generic functions.

## Display

| Value | Format |
| --- | --- |
| Array | `[1, 2]` |
| Product | `{x: 1, y: 2}` |
| Sum | `Found{text: "空"}`, `Missing{}` |
| Nested values | `[{x: 1}, {x: 2}]` |

Type names are omitted; product field names and sum variant names remain visible. Arrays use index order and fields use declaration order. Inactive sum storage is omitted. This is a readable value format, not serialization that reconstructs types.

Nested strings are quoted. Quote, backslash, NUL, LF, CR, and TAB become `\"`, `\\`, `\0`, `\n`, `\r`, and `\t`. Remaining ASCII controls use two lowercase hex digits, such as `\u{01}`, `\u{1b}`, and `\u{7f}`. Other UTF-8 bytes, including Japanese text, are unchanged. Values are not modified; standalone `print(string)` still writes the original bytes.

Evaluate and copy the entire argument before writing punctuation, and append one LF at the end. An argument failure emits no partial new aggregate display; output from earlier calls remains. Numeric formatting, including the existing limitations on special floating-point spellings, follows the [language reference](../reference/language.en.md).

## Match statements, expressions, and guards

Evaluate and copy the subject once. Try arms in source order, evaluating a `bool` guard only when its tag matches. A false guard continues to the next arm; a true guard selects only that body or result. Later guards and results are not evaluated eagerly.

Every variant requires an unguarded arm; even `if true` does not establish coverage. An arm following an unguarded arm for the same variant is unreachable and diagnosed. Bind every field or discard it with `field: _`. Bindings are immutable copies scoped to that arm's guard and body/result.

Statement arms use a block after `=>`; expression arms use a single expression. All result types must agree. An expected type from the surrounding context is passed to each arm; otherwise the first arm establishes the type. There are no implicit numeric conversions. Expressions support nesting, function results, generic functions, constants, and constant array lengths.

In statement arms, `return`, `break`, and `continue` retain their surrounding function/loop targets. Block expressions, whole-arm wildcards, nested patterns, and implicit error propagation are not introduced.

## Observe generation

All routes share expansion into typed IR before backend lowering.

| IR marker | Visible operations |
| --- | --- |
| `lower aggregate-equality` | Argument copies, index loops, field/tag comparisons, early returns |
| `lower aggregate-display` | Copies, field extraction, punctuation, `write` / `write.quoted`, final newline |
| `lower match-binding` | Local subject and pattern bindings |
| `lower match-selection` | Conditional evaluation of only the selected result |

Generated functions carry `generated` and their original source ranges. Outer values used by a match expression are copied through parameters; guard/result calculations stay inside their corresponding functions and branches. Constants retain their initializer and evaluated result. Bytecode and annotated LLVM/assembly retain origins; failures inside helpers preserve the original failing expression's location, reason, and prior output.

`write` is an internal operation without a newline, not a new source builtin. C/LLVM/QBE/assembly use the existing numeric formats and byte output. String quoting introduces no dynamic allocation. WAT sends formatted bytes through the [versioned output contract](wat-output-host.en.md), retaining per-operation requested/accepted counts, origins, and partial failures without exporting memory. Targets remain explicit rather than selected from the running host OS.

Observation adds no external mutation interface. Independent copies and short-circuit evaluation remain intact while the generated steps are inspectable.

## Try it

```sh
cargo run --quiet -- run examples/match_values.ceru
cargo run --quiet -- emit-ir examples/aggregate_comparison.ceru
cargo run --quiet -- emit-bytecode examples/match_guards.ceru
cargo run --quiet -- emit-llvm examples/aggregate_display.ceru --target x86_64-unknown-linux-gnu --annotate-origins
cargo run --quiet -- emit-asm examples/match_values.ceru --target x86_64-unknown-linux-gnu --annotate-origins
```

The [module example](../../examples/modules/aggregate_match.ceru) combines a public enum/constant with a match expression in another file. Tests compare VM, generated C/LLVM, QBE, WAT, Windows/Linux assembly, and objects from Cerune's encoder. Cases cover NaN, signed zero, maximum u64, NUL/CR/LF, Unicode non-normalization, independent copies, evaluation order, and failures in display arguments, guards, and results.

The [observation fixture](../../tests/fixtures/observation/aggregate-values/source.ceru) retains outputs for each representation and annotated LLVM/assembly, checking the path from typed IR to generated artifacts.
