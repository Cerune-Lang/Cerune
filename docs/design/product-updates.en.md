# Product update expressions

[日本語](product-updates.ja.md) | English

**Status: implemented**

## Purpose and syntax

Create a new struct value by replacing selected fields of an existing value.

```cerune
type Point { x: i64, y: i64, }
before: Point = Point { x: 1, y: 2 };
after: Point = Point { ..before, x: 3 };
print(before.x); // 1
print(after.x);  // 3
print(after.y);  // 2
```

Use `Type { ..base, field: expression, ... }`. A copy without replacements, `Type { ..base }`, and a trailing comma are supported. Qualified module type names work as well. This expression neither assigns directly to fields nor creates a new type.

## Evaluation contract

1. Evaluate `base` once and copy its value. It must have the same nominal type as the specified type.
2. Evaluate explicit field expressions once each, in source order.
3. Inherit unspecified fields from the copy. Do not re-evaluate field defaults.
4. Return the new value. It can be assigned to a `mut` binding or an array element.

The base is still evaluated when every field is replaced. Later changes to arrays or nested values do not affect the original. Strings retain their immutable bytes.

Failure stops evaluation of subsequent expressions and prevents the assignment. Earlier `print` output remains, and diagnostics identify the failing expression. Short-circuiting skips the entire update, including its base. Array assignments retain their existing order: target bounds checks precede the right-hand update expression.

Exactly one base is allowed, before all fields. A late or duplicate base, unknown or duplicate fields, and type mismatches are compilation errors. A distinct type with identical fields is still rejected. Empty `Type {}` literals and direct field assignment remain unsupported. Base expressions may contain recursive calls.

## Syntax rationale

Adding `..base` to the existing constructor keeps the type and replacements together. No new keyword is introduced for the earlier `with` proposal.

Putting the base first aligns reading order and evaluation order. There is no rule that evaluates a trailing base before preceding expressions, or precedence for merging multiple bases. Re-running defaults could change inherited values or repeat effects, so the expression inherits stored values instead.

## Lowering

| Route | Operation |
| --- | --- |
| Cerune IR | `Construct` retains the base and explicit fields. Text shows `base = ... [copy]` with each expression's NodeId and Span |
| Bytecode / VM | Evaluate base, then replacements; `construct.from_base` creates the new value |
| C | Copy into a typed temporary; comma expressions enforce field evaluation and assignment order |
| LLVM | Apply `insertvalue` to the base aggregate to produce a new aggregate |
| QBE / WAT / x86-64 | Copy into separate temporary storage, then replace selected fields |
| Internal object encoder | Use the shared assembly lowering and encode ELF / COFF |

Physical copy counts and layout are not language guarantees. Independent values, evaluation order, and diagnostics are shared across routes. Constructors without a base keep their existing behavior: explicit fields first, then defaults for omitted fields.

## Examples and validation

- [product_update](../../examples/product_update.ceru): update u64, strings, arrays, and nested structs; compare with the original.
- [product_update_order](../../examples/product_update_order.ceru): show base/replacement order, defaults, short circuiting, and loops.
- [modules/product_update](../../examples/modules/product_update.ceru): update an imported public type using imported functions.
- [modules/product_update_failure](../../examples/modules/product_update_failure.ceru): a failure in the base skips later expressions and preserves the imported source location and prior output.
- [packet_counter](../../examples/packet_counter.ceru): replace an existing constructor with an update while preserving output.

Tests cover syntax and type errors, recursion through a base, and IR/bytecode origins. Success and failure cases are compared across VM, C, LLVM, QBE, WAT, Windows/Linux assembly, and internal objects, including failure reasons, locations, and output before failure.
