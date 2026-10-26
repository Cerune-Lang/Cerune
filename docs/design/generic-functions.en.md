# Functions with type and length parameters

[日本語](generic-functions.ja.md)

## Syntax

```cerune
fn first<T, const N: i64>(values: [T; N]) -> T {
    return values[0];
}
const COUNT: u8 = 2;
print(first::<string, COUNT>(["日本語", "予備"]));
```

| Parameter or argument | Meaning |
| --- | --- |
| `T` | Type parameter accepting numbers, bool, string, named types, or fixed arrays |
| `const N: i64` | Positive length parameter usable in `[T; N]` and as an i64 value in the body |
| `::<string, COUNT>` | Supply every argument in declaration order; lengths accept unsuffixed positive integer literals, integer constant names, or enclosing length parameters |
| `pub fn` | Ordinary function visibility; call with `alias::first::<T, N>(values)` |

Type-argument inference/defaults, traits or constraint syntax, generic types and first-class functions are outside this increment. Lengths range from 1 to i64's maximum; using them in arrays also applies existing storage limits. Compute arbitrary length expressions in named constants rather than directly in generic arguments.

## Type checking and value semantics

Parameter names cannot repeat or collide with built-in types, types/functions/constants in the same scope, or import aliases. Value parameters and local bindings cannot shadow generic parameters. Using a type parameter as a value or a length parameter as a type is diagnosed.

Instantiation substitutes parameter, return, and body type annotations, then applies ordinary function checking. A body containing `a + b` may work for i64 but fails for string, which has no concatenation. Use `convert<T>(value)` for conversions to a type parameter. `T(value)` and `T { ... }` are not construction syntax for type parameters.

Unused templates produce no executable functions. Their syntax and generic declarations are checked, but unused bodies are not guaranteed to be fully type-checked. Module name resolution and visibility checks still run during loading. A call skipped by runtime short-circuiting is still instantiated and checked. Calls from constant expressions remain prohibited.

Type/length arguments are compile-time information. Ordinary value arguments evaluate once, left to right; skipping a call through short-circuiting also skips its arguments. Parameters/results retain value-copy semantics, so changing an array or product copy cannot modify the caller's value. String bytes and immutability, exact numeric conversions, and failure conditions are unchanged.

## Specialization and observation

The common frontend instantiates ordinary typed functions before resolving array lengths, lowering sums, checking types, and building Cerune IR. The VM, C, LLVM, QBE, WAT, Windows/Linux assembly, and native objects consume the same instantiated IR.

Instances are reused for the same template and resolved type/length arguments. `[i64; COUNT]` and `[i64; 2]` are equal when COUNT is 2. Named types retain their module identity. Internal `$generic_*` names cannot collide with source identifiers; backends translate unsupported characters and distinguish symbols using function IDs.

```text
; instantiate first::<u64, 2> [definition source=0 bytes=0..72]
;   call [source=0 bytes=100..146]
;   call [source=0 bytes=185..209]
fn %$generic_0_first@0(%values@0: [u64; 2]) -> u64 {
  ...
}
```

IR retains the definition span, concrete arguments, and every call span. Structured IR also retains each type/length argument's source span. A type argument such as `[i64; COUNT]` retains its constant-length reference and remains queryable from the original AST. Body expression spans remain at their definitions, with separate NodeIds for each instance. Type errors include relevant instantiations and call locations; multiple instances sharing the same definition span are listed as candidates. Runtime failures map to the original body expression and preserve prior output. Observation grants no authority to modify running values.

Recursion is supported. Register resolved type/length combinations before processing bodies and reuse pending instances. Process bodies from a queue in registration order without compiler-recursive expansion calls. Calls through used defaults follow ordinary runtime depth checks. A compilation allows at most 256 instances and expansion depth 64. Existing per-type storage limits also apply to unused type arguments.

## Examples and validation

| Example | Checks |
| --- | --- |
| [generic_functions.ceru](../../examples/generic_functions.ceru) | Aggregation, string-array replacement, u64/f64, products, sums, nested arrays |
| [generic_evaluation_order.ceru](../../examples/generic_evaluation_order.ceru) | Left-to-right arguments, type/length forwarding, short-circuiting |
| [modules/generic_functions.ceru](../../examples/modules/generic_functions.ceru) | Public functions/types/constants and private implementation sizes |
| [Observation fixture](../../tests/fixtures/observation/generic-functions/source.ceru) | IR, bytecode, C, LLVM, QBE, WAT, and assembly from one source |

`cargo test --test generic_functions --test modules` checks semantics and diagnostics. Shared cases compare exact successful output bytes across routes and compare failure codes, source locations, and prior output.
