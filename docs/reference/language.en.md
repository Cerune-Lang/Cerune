# Cerune language reference

[日本語](language.ja.md)

This document defines the syntax and semantics of Cerune v0.1.

## Grammar

`let` is optional on ordinary bindings, for initializers and each array-iteration binding. `let mut total: infer = 0;` and `mut total: infer = 0;` have the same meaning. The colon remains required; constants, parameters, fields and for updates do not accept the prefix. It is contextual: only an identifier or `mut` following it in a binding position makes it a prefix. Existing identifiers such as `let: i64 = 1;` remain valid. See [fmt](cli.en.md#source-formatting) for preserve/show/hide policies.

```text
program     := item* EOF

item        := enum_definition
             | type_definition
             | function_definition
             | constant_definition
             | statement

type_definition :=
    "type" IDENT "{" field_definition ("," field_definition)* ","? "}"

enum_definition := "enum" IDENT "{" variant ("," variant)* ","? "}"
variant     := IDENT ("{" payload_fields? "}")?
payload_fields := (IDENT ":" type_ref) ("," IDENT ":" type_ref)* ","?
variant_path := IDENT "::" IDENT | IDENT "::" IDENT "::" IDENT
match_statement := "match" expression "{" match_arm ("," match_arm)* ","? "}"
match_arm   := variant_path "{" pattern_fields? "}" ("if" expression)? "=>" block
match_expression := "match" expression "{" match_value_arm ("," match_value_arm)* ","? "}"
match_value_arm := variant_path "{" pattern_fields? "}" ("if" expression)? "=>" expression
pattern_fields := (IDENT ":" IDENT) ("," IDENT ":" IDENT)* ","?

constant_definition := "const" IDENT ":" type_ref "=" expression ";"

field_definition := IDENT ":" type_ref ("=" expression)?

function_definition :=
    "fn" IDENT generic_parameters? "(" parameters? ")" "->" return_type block

generic_parameters := "<" generic_parameter ("," generic_parameter)* ","? ">"
generic_parameter := IDENT | "const" IDENT ":" "i64"
generic_arguments := "::<" generic_argument ("," generic_argument)* ","? ">"
generic_argument := type_ref | INTEGER
function_path := IDENT ("::" IDENT)?
function_call := function_path generic_arguments? "(" arguments? ")"

parameters  := parameter ("," parameter)*

parameter   := IDENT ":" type_ref

return_type := type_ref | "void"

statement   := binding
             | assignment
             | "print" "(" expression ")" ";"
             | "flush" "(" ")" ";"
             | function_call ";"
             | "return" expression? ";"
             | match_statement
             | if_statement
             | while_statement
             | for_statement
             | for_each_statement
             | "break" ";"
             | "continue" ";"

if_statement := "if" expression block ("else" block)?

while_statement := "while" expression block

for_statement :=
    "for" "(" (binding_clause | binding_assignment_clause) ";"
    expression ";" binding_assignment_clause ")" block

for_each_statement :=
    "for" "(" ("let"? IDENT ":" type_spec ",")?
    "let"? "mut"? IDENT ":" type_spec "in" expression ")" block

block       := "{" statement* "}"

binding     := "let"? "mut"? IDENT ":" type_spec? "=" expression ";"

binding_clause := "let"? "mut"? IDENT ":" type_spec? "=" expression

assignment  := assignment_target "=" expression ";"

assignment_target := IDENT ("[" expression "]")*

binding_assignment_clause := IDENT "=" expression

type_spec   := "i8"
             | "u8"
             | "i16"
             | "u16"
             | "i32"
             | "u32"
             | "i64"
             | "u64"
             | "f32"
             | "f64"
             | "bool"
             | "string"
             | fixed_array_type
             | IDENT
             | "infer"

type_ref    := "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "bool" | "string" | fixed_array_type | IDENT

fixed_array_type := "[" type_ref ";" array_length "]"
array_length := INTEGER | IDENT ("::" IDENT)?

expression  := logical_or

logical_or  := logical_and ("||" logical_and)*

logical_and := bit_or ("&&" bit_or)*

bit_or      := bit_xor ("|" bit_xor)*

bit_xor     := bit_and ("^" bit_and)*

bit_and     := equality ("&" equality)*

equality    := comparison (("==" | "!=") comparison)*

comparison  := shift (("<" | "<=" | ">" | ">=") shift)*

shift       := additive (("<<" | ">>") additive)*

additive    := multiply (("+" | "-") multiply)*

multiply    := unary (("*" | "/" | "%") unary)*

unary       := ("-" | "!" | "~") unary
             | postfix

postfix     := primary (("." IDENT) | ("[" expression "]"))*

primary     := match_expression
             | "true"
             | ("i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64") "(" expression ","? ")"
             | "convert" "<" type_ref ">" "(" expression ","? ")"
             | "false"
             | INTEGER
             | FLOAT
             | STRING
             | "[" expression ("," expression)* ","? "]"
             | IDENT
             | function_call
             | variant_path "{" (field_value ("," field_value)* ","?)? "}"
             | IDENT "{" field_value ("," field_value)* ","? "}"
             | IDENT "{" ".." expression ("," field_value)* ","? "}"
             | "(" expression ")"

arguments   := expression ("," expression)*

field_value := IDENT ":" expression
```

Bindings are immutable by default. Only a binding declared with `mut` can be reassigned. Expressions may only refer to bindings declared earlier in the file.

A type specifier is always required.

```cerune
count: i64 = 42;
single: f32 = 0.1 + 0.2;
double: f64 = 0.1 + 0.2;
value: infer = count * 2;
```

`infer` explicitly requests type inference. It is not itself a runtime type.

Ordinary bindings and for initializers may use `value: = expression;` with exactly the same inference. This combines with `mut` and optional `let`; the declaration colon remains required. Array iteration still needs a type specification such as `value: infer in values`. Omission does not extend to constants, parameters, results or fields. Formatting preserves inference spelling. See the [contract and verification](../design/infer-omission.en.md).

## Strings

`string` is a UTF-8 string, written in double quotes. Empty strings (`""`) and Japanese text are supported.

```cerune
mut text: string = "こんにちは";
saved: infer = text;
text = "こんばんは";
print(saved);                  // こんにちは
print(text);                   // こんばんは
print(saved == "\u{3053}んにちは"); // true
```

String contents cannot be changed in place. `mut` permits assigning a different string to that name; previously saved values remain unchanged. Strings can be function parameters and results, struct fields and defaults, and fixed-array elements. Elements of a `mut` string array can be replaced with other strings.

Supported escapes inside strings are:

| Spelling | Value |
| --- | --- |
| `\"` | Double quote |
| `\\` | Backslash |
| `\n` / `\r` / `\t` | Line feed (LF) / carriage return (CR) / tab |
| `\0` | NUL (part of the value, not a string terminator) |
| `\u{...}` | Unicode scalar value written as 1 to 6 hexadecimal digits |

`\u{...}` accepts `0` through `10FFFF`, excluding the surrogate range `D800` through `DFFF`. Invalid escapes and unclosed quotes are diagnosed at the original source location. Physical LF and CR characters are not allowed between quotes in source; use escapes such as `\n` instead.

`==` and `!=` compare the entire contents. Comparison is case-sensitive and does not normalize Unicode. Visually identical text with different character sequences remains distinct. `"a\0b"` is not treated as `"a"`.

String indexing, character counts, ordering, and numeric conversions are not implemented. Strings do not implicitly convert to or from other types.

`print` writes the contents unchanged and appends LF. In contrast, textual Cerune IR and bytecode escape line breaks and control characters. Decoded values are kept distinct from the UTF-8 byte range (Span) of the original quoted spelling.

Strings are supported by every output route. LLVM and QBE require [CLI target selection](cli.en.md#llvm-target-selection); omitting it produces a source-located diagnostic before lowering, including strings in unused types, functions, and branches. Direct assembly supports Windows x64 and Linux x86-64; native objects require an explicit target. WAT uses the WebAssembly output host contract.

C emission pairs a data pointer with a byte count. Static content lasts until process exit; dynamic content lasts until its final release. Generated programs using strings set standard output to binary mode on Windows, preventing automatic LF or CR translation. See [String design](../design/strings.en.md) for representation and lifetime details.

### Concatenating strings

`concat(left, right)` evaluates exactly two strings once from left to right and returns their concatenated immutable contents. NUL, CR/LF, and Unicode spellings retain their exact bytes. It works with function results, arrays, products, enums, defaults, and `const`. The built-in name cannot be redefined as a function.

```cerune
mut text: string = concat("こんにちは、", "世界");
saved: string = text;
text = concat(text, "！");
print(saved); // こんにちは、世界
print(text);  // こんにちは、世界！
```

An empty result uses static storage; every nonempty result allocates. Reassignment releases the old value after evaluating the RHS, allowing self-assignment and use of the old value as an argument. The default live payload budget is 64 MiB, configurable before execution with `--string-heap-limit <bytes>`. Constant evaluation has an independent 64 MiB budget. See the [ownership, lifetime, and failure contract](../design/dynamic-data.en.md).

### UTF-8 byte length

`byte_len(text)` accepts one `string` and returns its stored UTF-8 byte count as `i64`. Its argument is evaluated once.

```cerune
print(byte_len(""));          // 0
print(byte_len("日本語"));    // 9
print(byte_len("\0\r\n"));  // 3
print(byte_len("\u{e9}"));    // 2
print(byte_len("e\u{301}"));  // 3
```

NUL and line endings count as data. No Unicode normalization occurs. This does not count Unicode scalar values, displayed characters, or display width. The result is always `i64`; use an existing explicit conversion to assign it to a smaller integer type.

`byte_len` is reserved as a built-in operation name: defining a function with that name is rejected. Wrong argument counts/types and discarding its result in a call statement produce source-located diagnostics. Variable and function namespaces remain separate. The operation accepts function results, array elements, fields, and defaults.

`byte_len(f())` evaluates `f()` exactly once. Short-circuited operands remain unevaluated. If evaluating the argument fails, for example through an out-of-bounds access, execution stops before reading the length.

## Named product types

`type` defines a named product type that groups several fields into one value. Types have nominal identity, so two types with the same fields are still distinct.

```cerune
type Point {
    x: f64 = 0.0,
    y: f64,
}

point: Point = Point {
    y: 2.0,
};

print(point.x);
```

Every field type is explicit and cannot use `infer`. A field without a default is required when constructing a value. Fields are named, so construction order may differ from definition order. Trailing commas are accepted.

For construction without a base, explicit field expressions are evaluated in source order. Defaults for omitted fields are then evaluated in definition order. Cerune IR exposes this order and whether each value was explicit or came from a default.

`.` accesses a field and may be chained as in `segment.start.x`. Fields cannot be assigned directly. To make a change, construct a new value and reassign the whole `mut` binding.

```cerune
mut point: Point = Point { x: 1.0, y: 2.0, };
point = Point { x: 3.0, y: point.y, };
```

Reassigning the original binding after copying a product value into another binding does not change the earlier value. This is the language-level value rule. Each backend's physical placement and copying remain observable in emitted artifacts.

The `{` immediately after an `if` or `while` condition starts its block. Parenthesize a construction expression when accessing one of its fields in a condition.

```cerune
type Flags { enabled: bool, }

if (Flags { enabled: true, }).enabled {
    print(true);
}
```

Empty product types, empty construction expressions, and infinitely sized recursion by value are unsupported. Whole-product equality and `print` follow the [aggregate rules](../design/aggregate-values.en.md).

See [Named product type design](../design/product-types.en.md) for the detailed design and backend representations.

### Product update expressions

`Point { ..original, x: 3 }` creates a new `Point` by replacing only `x` in `original`.

- Specify exactly one base first. It is evaluated once and copied, and must have the specified nominal type.
- Evaluate subsequent field expressions in source order. Inherit omitted fields from the base without re-evaluating defaults.
- `Point { ..original }` and trailing commas are supported. The base is evaluated even when all fields are replaced.
- Unknown/duplicate fields, type mismatches, and late/duplicate bases are diagnosed. Direct field assignment is not permitted.
- The original and copied arrays remain independent. Failure skips subsequent expressions and assignment, retaining prior output and the failing expression's location.

See the [syntax and evaluation design](../design/product-updates.en.md) and [example](../../examples/product_update.ceru).

## Dynamic arrays

`[T]` is an owned value with a runtime length. Execute it with `run` / `run-ir` / `run-vm`; inspect it with `emit-ir` / `emit-bytecode` / `emit-c` / `emit-llvm` / `emit-qbe` / `emit-wat`. Build generated C/LLVM/QBE with external tools; LLVM and QBE require an explicit `--target`. Convert WAT to Wasm and run it with output host functions. Use `emit-asm` / `emit-obj` with an explicit Windows/Linux target, then link with external tools.

| Operation | Meaning |
| --- | --- |
| `array_copy(values)` | Create an independent `[T]` from a fixed or dynamic array |
| `array_copy_range(values, start, end)` | Copy a half-open range with i64 bounds, including empty ranges |
| `array_repeat::<T>(value, count)` | Create `[T]` with an `i64` count of independent copies of an explicitly typed value |
| `array_len(values)`, indexing, display, equality, `for … in` | Existing array rules; compare the same type and iterate an independent entry snapshot |

`[1, 2]` remains fixed; no implicit conversion is added. Create an empty array with `array_copy_range([1], 0, 0)`. Binding, argument, return, and aggregate copies are independent even when nested. A `mut` binding permits element updates and reassignment to a different length. Generic type arguments, modules, structs, and enums can contain dynamic arrays.

Range copies evaluate source, start, and end once from left to right, then check `0 <= start <= end <= length`. Violations stop with `array-range-out-of-bounds`. Indexed assignment checks each index before later indices or the RHS.

`array_repeat` evaluates value and count once, in that order, then checks that count is nonnegative. Negative counts report `array-length-out-of-range`. Zero still evaluates both expressions and releases temporaries without allocating result storage. Nested dynamic arrays in each element are independent.

`--array-heap-limit <bytes>` limits logical element storage, defaulting to 64 MiB independently of strings. Copies require old and new storage simultaneously. Display, length, and equality do not copy solely to read. Dynamic arrays in `const`, growth operations, and borrowed slices remain unsupported. See [examples](../../examples/dynamic_arrays/README.en.md) and [ownership and generation design](../design/owned-arrays.en.md).

## Fixed arrays

A fixed array is a value containing a known number of boxes of the same type. `[i64; 4]` means an array with four `i64` boxes.

```cerune
values: [i64; 4] = [2, 4, 6, 8];
print(values[2]);
```

The length is part of the type, so `[i64; 3]` and `[i64; 4]` are different types. Cerune reports an error when a literal has the wrong number of elements or its element types differ. Empty array literals are not currently available because they provide no element type to infer.

An index has type `i64`, and the first index is `0`. In the example above, `values[2]` reads the third value, `6`. A negative index or an index greater than or equal to the length stops execution in both the Cerune VM and generated programs. Every backend leaves this bounds check visible in its artifact.

An array is copied as one value. Reassigning the original `mut` binding after a copy does not change the earlier copy.

```cerune
mut first: [i64; 2] = [10, 20];
second: [i64; 2] = first;
first = [30, 40];
print(second[0]); // 10
```

An element type may be `bool`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`, `string`, a named product type, a sum type, or another fixed array. A fixed array may also be used as a field of a product type.

```cerune
type Point {
    x: i64,
    y: i64,
}

type Path {
    points: [Point; 4],
}

path: Path = Path {
    points: [
        Point { x: 0, y: 0, },
        Point { x: 1, y: 1, },
        Point { x: 2, y: 4, },
        Point { x: 3, y: 9, },
    ],
};

print(path.points[2].y);
```

Fixed arrays may be nested directly. In `matrix[row][column]`, the two indices are checked in sequence.

```cerune
matrix: [[i64; 3]; 2] = [[1, 2, 3], [4, 5, 6]];
print(matrix[1][2]); // 6
```

Fixed arrays may be used as function parameters and results. They remain values and are copied across the function boundary.

An element of a `mut` array can be updated with `values[index] = value;`. Nested arrays support forms such as `matrix[row][column] = value;`. Indices are evaluated from left to right and each is bounds-checked immediately. The right-hand side is evaluated only after every check succeeds, followed by one write. If a check fails, the right-hand side is not evaluated and the array is unchanged.

Updating one copy of an array does not change another copy. The assigned value must have the declared element type. Updating through an immutable binding is an error, just like reassigning the complete array.

See [Fixed array design](../design/fixed-arrays.en.md) for the detailed design and bounds-check representation in each backend.


### Constants in type lengths

```cerune
const COUNT: u32 = 2;
values: [i64; COUNT] = [7, 9];
same: [i64; 2] = values;
print(array_len(same)); // 2
```

Specify a positive integer literal, an integer constant name, or `alias::COUNT`. Every integer kind is accepted, but the value must be positive and at most `i64::MAX`. Declaration order is unrestricted. Resolved lengths determine type identity; mismatched initializer counts remain errors.

Write calculations in definitions such as `const COUNT: i64 = BASE + 1;`. `[T; COUNT + 1]`, calls in type lengths, and runtime variables are unsupported. Cycles through types are diagnosed. Public types may use local private size constants; importing a constant name requires `pub`.

The shared resource limit is 100,000 scalar units per aggregate value and type-computation depth 128, applying equally to literals, constant names, and `infer`. IR retains the constant's computation and its type-reference locations. See [evaluation and limits](../design/constant-array-lengths.en.md) and the [example](../../examples/constant_array_lengths.ceru).

### Array element count

`array_len(values)` accepts one array and returns its outermost element count as `i64`, regardless of the element type: `N` for `[T; N]`, or the runtime length for `[T]` (dynamic arrays currently support IR/VM/C/LLVM/QBE/WAT/ASM/native objects). Use `byte_len` for string bytes.

```cerune
matrix: [[i64; 3]; 2] = [[1, 2, 3], [4, 5, 6]];
print(array_len(matrix));    // 2
print(array_len(matrix[0])); // 3
```

Evaluate the argument exactly once. A known length does not remove calls in `array_len(make())` or evaluation of array elements. If the argument fails, report the original failing expression without returning a length. Short-circuited operands remain unevaluated; a loop condition evaluates the argument each time the condition is checked.

Constant expressions support this operation, subject to constant-expression rules for the entire argument. Failures in unused constants are still diagnosed, and ordinary function calls remain forbidden. Integer results can be stored in constants and used as array type lengths.

Functions, constants, and import aliases cannot use this name. Variable and function namespaces remain separate. Wrong argument counts/types and discarded call results are diagnosed. See the [aggregation example](../../examples/array_length.ceru) and [evaluation-order example](../../examples/array_length_order.ceru).


### Fixed-array iteration

Dynamic arrays use the same iteration rules in IR/VM/C/LLVM/QBE/WAT/ASM/native objects; zero length skips the body.

```cerune
values: [i64; 3] = [4, 7, 9];
for (value: infer in values) { print(value); }
for (index: i64, mut value: infer in values) {
    value = value + index;
    print(value);
}
print(values[1]); // 7
```

Use `for (element: type in subject)` or `for (index: type, element: type in subject)` to visit a fixed array from the beginning. Every binding requires an explicit type or `infer`. The index is immutable `i64`; the subject determines the element type. `for (v: u8 in [1])` is a type error; specify the subject as e.g. `[1u8]`. `in` is reserved.

Evaluate and copy the subject once on entry, in the outer scope. Changes to the original array inside the body do not affect this snapshot. Each element is also copied; `mut` permits changing that iteration's copy only. Bindings are body-local; duplicate declarations in that body and shadowing constants/import aliases are diagnosed.

`continue` advances to the next element; `break`/`return` retain their existing meaning. An immediate `break` still follows complete subject evaluation. Subject/body failures stop at the original failing expression. A `return` inside iteration alone does not establish that the function returns on every path. Empty arrays, string iteration, references, and destructuring patterns are unsupported.

Cerune IR exposes the snapshot, length, and cursor as internal `$for_in_*` bindings and an existing `for`. Subject/body source locations are retained; generated control expressions point to the header. See the [design](../design/array-iteration.en.md) and [aggregation/search example](../../examples/array_iteration.ceru).

## Sum types and match

`enum Lookup { Found { text: string }, Missing }` declares a payload variant and an empty variant. Construct them with `Lookup::Found { text: "空" }` and `Lookup::Missing {}`. Types are nominal and can appear in bindings, arguments, returns, arrays, products, and constants.

```cerune
enum Lookup { Found { text: string }, Missing }
value: Lookup = Lookup::Found { text: "空" };
match value {
    Lookup::Found { text: text } => { print(text); },
    Lookup::Missing {} => { print("未登録"); },
}
```

`match` supports statements and expressions. Evaluate and copy the subject once, then try arms in source order. An optional `if condition` after a pattern evaluates its `bool` condition only for a matching tag; false continues to the next arm. Every variant requires an unguarded arm, after which another arm for that variant is unreachable. Evaluate only the selected body/result. Bind each field as an immutable local with `field: binding` or discard it with `field: _`. Parenthesize constructors used directly as subjects. Constructor fields evaluate in source order; reassigning a copy does not alter other values.

`enum` and `match` are keywords. `pub enum` exports all variants and fields; importers use `alias::Enum::Variant`. Enum payload defaults, recursive value types, direct field access, product updates, whole-arm wildcards, and nested patterns are unsupported. Whole-enum display/equality follow the [aggregate rules](../design/aggregate-values.en.md). Return, break, and continue retain their ordinary function/loop targets. Match does not catch runtime stops. See the [design and representation](../design/sum-types.en.md) and [example](../../examples/sum_lookup.ceru).

Statement arms use `=> { statements... }`; expression arms use `=> expression`. All result types agree. An expected type applies to every arm; otherwise the first result establishes the type. No implicit conversions or block expressions are added. Pattern bindings are scoped to their guard and arm.

```cerune
label: string = match value {
    Lookup::Found { text: text } if byte_len(text) > 0 => text,
    Lookup::Found { text: _ } => "空欄",
    Lookup::Missing {} => "未登録",
};
```

## Compile-time constants

`const LIMIT: u64 = 64 * 2;` evaluates a typed expression during compilation. Declarations belong at file scope and can be used in functions and field defaults. Forward references are allowed; cycles and dependency depths beyond 128 are diagnosed.

Numbers, booleans, strings, arrays, structs, and sums are supported. Constant expressions cannot reference runtime variables or call ordinary functions. Short circuiting is preserved, while every constant is type-checked and evaluated even if unused. Evaluation failures are compilation errors.

Constants cannot be assigned or shadowed by local bindings. Export with `pub const` and access through `alias::LIMIT`. Integer constants can also specify array type lengths; block-local declarations remain unsupported. See the [evaluation and observation contract](../design/constants.en.md) and [example](../../examples/constants.ceru).

## Modules and visibility

```cerune
import "values.ceru" as values;
item: values::Reading = values::reading(7);
print(item.amount);
```

Use external types, functions, and constants through `alias::name`. Imports precede definitions and statements. Definitions are private to their file unless marked `pub fn`, `pub type`, `pub enum`, or `pub const`. Public types expose all fields; public parameter/result types and fields cannot contain private types.

Imported files allow only imports, types, functions, and constants at top level. Loading executes no initialization and never invokes a dependency's `main`. Relative `.ceru` paths resolve against the declaring file's directory, using `/` separators. Cycles, private access, duplicate aliases, and alias collisions with definitions or bindings are diagnosed. `import`, `as`, `pub`, and `const` are keywords.

Re-exports, wildcards, imported variables, and mutable module state are unsupported. See the [rules and loading limits](../design/modules.en.md) and [executable examples](../../examples/modules/README.en.md).

### Line input through a virtual standard module

Explicitly import the bundled module with `import "std:io" as io;` and call `io::read_line(max_bytes)`. The result is the ordinary enum `io::ReadLine::Line { text: string }` / `io::ReadLine::End {}`. Preserve LF/CRLF and distinguish blank lines from EOF. Limits range from 1 to 2147483647 bytes including line endings. Read, UTF-8, limit, budget, and other errors diagnose and stop execution.

Currently available through HIR/MIR/SSA/VM Rust input connection APIs. [CLI and generated C/LLVM/QBE/Native stdin](../design/stdin-connections.en.md) are also available. [Versioned WAT input hosts](../design/wat-input-host.en.md) are also implemented. **Windows console input remains unsupported.** Standard modules are not searched through files or environment variables; emit-sources exposes their text, types can be inspected in IR, and source origins remain observable. See [resolution and execution examples](../design/standard-modules.en.md) and the [consumption/failure contract](../design/line-input.en.md).

### Explicit integer parsing

Import `"std:num" as num` and call `num::parse_i64(text: string) -> num::ParseI64`. Results are `Value { value: i64 }`, `InvalidFormat {}` and `OutOfRange {}`. Validate the complete ASCII grammar `-?[0-9]+` before checking the i64 range. Leading zeros are allowed and `-0` is zero. `+`, whitespace, line endings, NUL and non-ASCII digits are rejected without mutation or normalization.

All existing routes support it. Bundled Cerune source implements the checks and keeps their generation observable. See the [contract](../design/integer-parsing.en.md) and [basic example](../../examples/integer_parsing/README.en.md).

### Program arguments

Import `std:env` explicitly to use `env::arg_count() -> i64` and `env::arg(index: i64) -> env::Argument`. Results are `Value { text: string }` / `Missing {}`. Empty text is Value, nonnegative out-of-range indices are Missing, and negative indices stop with source diagnostics. Contents are neither normalized nor split again.

HIR/MIR/SSA/VM, CLI and generated C/LLVM/QBE/WAT are supported. Values after the first CLI `--` are program arguments, excluding executable/source names. Defaults are 4096 entries and 1048576 total UTF-8 bytes; override with `--argument-count-limit` / `--argument-byte-limit` before the delimiter. Invalid Unicode, NUL and limit excess stop startup preparation. Returned strings separately obey the string-heap limit.

Existing embedding APIs never read OS arguments implicitly and report unconfigured access. Inject arguments using `run_with_arguments` or related APIs. See the [contract](../design/dogfooding-inputs.en.md) and [example](../../examples/arguments/README.en.md).

### File reads and new-file creation

Import `"std:fs" as fs` to use `fs::read_text(name, max_bytes) -> string` and `fs::write_new(name, text, max_bytes) -> void`. Whole-file UTF-8 limits range from 0 to 2147483647 bytes. Existing files are never overwritten. Preserve NUL, CR/LF, and combining characters, and stop with source diagnostics on errors.

**HIR/MIR/SSA/VM support explicit Rust API connections and Windows/Linux CLI `--file-root <directory>`. Generated C/LLVM/QBE also support Windows/Linux. WAT supports versioned host connections; a real-OS host is not provided. Native remains unimplemented.** Unconfigured access stops with `unconfigured`. See the [contract and connection APIs](../design/file-io.en.md) and [example](../../examples/files/README.en.md).

## Functions and entrypoint

`fn` defines a named computation. Every parameter and the return type are explicit.

```cerune
fn add(left: i64, right: i64) -> i64 {
    return left + right;
}

answer: i64 = add(20, 22);
```

A value-returning function uses an explicit `return expression;`. A trailing expression is not an implicit result. Cerune reports an error when it cannot prove that every path returns a value.

A function without a value uses `-> void`. It may reach the end of its block or exit early with `return;`. A value-returning call is used as an expression, while a `void` call is a statement.

```cerune
fn show(value: i64) -> void {
    print(value);
}

show(answer);
```

Function names are resolved across the whole file, so a call may precede its definition. Parameters and local bindings are not visible outside their function. A function also cannot read a top-level runtime binding.

Top-level executable statements receive a compiler-generated entrypoint. A program may instead define `fn main() -> void`, but an explicit `main` cannot be combined with top-level executable statements. `main` takes no parameters.

Function parameters and results may use `bool`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`, `string`, named product types, and fixed arrays. Products and arrays are passed as values, so the received value and the caller's value do not share a mutable location. There is no fixed language-level parameter-count limit. Direct and mutual recursion are supported. CLI call depth defaults to 128, range 1–128, distinct from physical stack safety. Unsupported forms produce diagnostics instead of silently changing meaning.

Cerune IR and bytecode expose function IDs, parameter binding IDs, calls, and returns. Backend artifacts expose how those entities become function symbols, arguments, local storage, and ABI registers or memory. See [Function design](../design/functions.en.md) for details.

### Functions with type and length arguments

```cerune
fn first<T, const N: i64>(values: [T; N]) -> T { return values[0]; }
print(first::<string, 2>(["日本語", "予備"]));
```

`T` is a type parameter and `const N: i64` is a positive length parameter. Supply every argument in declaration order using `::<...>`. Types are existing concrete types; lengths are positive integer literals, integer constant names, or enclosing length parameters. A length parameter is also an i64 body value. Convert to a type parameter with `convert<T>(value)`. `pub fn` and `alias::first::<string, COUNT>(values)` are supported.

Equal resolved types/lengths share one ordinary function instance. Bodies are checked for each used concrete type; unused templates do not guarantee complete body type checking. Calls skipped by short-circuiting are still checked. Value-argument order, copying, and failure rules remain those of ordinary functions. Recursive calls reuse the same specialization. Type-argument inference, traits, and generic type definitions are unsupported. See the [design](../design/generic-functions.en.md) for limits and observation formats and [generic_functions.ceru](../../examples/generic_functions.ceru) for a runnable example.

## Mutable bindings and reassignment

A binding that needs to change is declared with `mut` before its name:

```cerune
mut count: i64 = 40;
count = count + 2;
print(count);
```

`mut` is not a type. It specifies that the name `count` may be reassigned. For arrays, it also permits element updates through that binding. Reassignment does not contain `: type_spec`; this distinction separates a new declaration from assignment to an existing binding.

Reassigning a binding without `mut` is a type-checking error:

```cerune
count: i64 = 40;
count = 42; // error
```

The assigned value must have the type resolved when the binding was declared. With `infer`, inference happens only at the declaration:

```cerune
mut value: infer = 1; // resolved as i64
value = 2;            // OK
value = 0.5;          // error
```

Cerune IR preserves initialization and reassignment as different statements. Bytecode likewise distinguishes initialization `store` from reassignment `assign`.

## Conditionals, loops, and block scope

`if` executes statements according to a `bool` condition. The `else` block is optional.

```cerune
if value < 10 {
    print(value);
} else {
    print(10);
}
```

A condition that is not `bool` is a type-checking error. An `if` is currently a statement and does not produce a value.

`while` repeats its body while its condition is `true`. The condition is evaluated before the body on every iteration, so a condition that starts as `false` executes the body zero times.

```cerune
mut count: i64 = 0;

while count < 3 {
    print(count);
    count = count + 1;
}
```

The condition of a `while` must also be `bool`. A `while` is a statement and does not produce a value.

`for` groups a start statement, a `bool` continuation condition, an update statement, and a body. The start statement may declare a new binding or assign to an existing `mut` binding:

```cerune
mut sum: i64 = 0;

for (mut i: i64 = 0; i < 6; i = i + 1) {
    sum = sum + i;
}
```

The start statement runs once. Before every iteration, the continuation condition is evaluated. After every completed iteration, the update statement runs and control returns to the condition. All three header parts are required in the current syntax.

A binding declared by the start statement is visible in the condition, update, and body, but not after the `for`. When the start statement assigns to an existing binding, that binding remains visible after the `for`. The body creates a nested block scope.

`break;` exits the innermost loop. In a `while`, `continue;` proceeds directly to its condition. In a `for`, it proceeds to the update and then the condition. Neither may be used outside a loop.

```cerune
while value < 10 {
    value = value + 1;

    if value < 3 {
        continue;
    }

    if value > 5 {
        break;
    }
}
```

Cerune currently has no labeled `break` or `continue` for naming an outer loop.

Each braced block creates a new scope. Bindings declared inside a block are not visible outside it. An inner block can read an outer binding and can reassign it when it is `mut`.

An inner block may declare a distinct binding with the same name as an outer binding.

```cerune
mut value: i64 = 1;

if true {
    value = 2;          // updates the outer value
    value: bool = true; // a distinct value local to this block
    print(value);       // the bool value
}

print(value);           // the i64 value
```

Cerune IR assigns deterministic IDs to bindings so references remain unambiguous when names are reused. Structured `if`, `while`, `for`, `break`, and `continue` statements remain visible in Cerune IR. A `for` keeps its initializer, condition, body, and update as distinct parts. During lowering into Bytecode and backend IRs, structured loops become condition, body, update when applicable, and exit paths. `break` and `continue` become jumps to the correct path of their target loop.

## Types

Cerune v0.1 has one boolean type, ten numeric types, a string type, fixed arrays, and user-defined named product types:

```text
bool
i8
u8
i16
u16
i32
u32
i64
u64
f32
f64
string
fixed arrays
named product types
```

Backends map supported types to their own representations during lowering. Strings are supported by every output route.

For example, the C backend maps them as follows:

```text
Cerune    C
bool      bool
i8        int64_t
u8        int64_t
i16       int64_t
u16       int64_t
i32       int64_t
u32       int64_t
i64       int64_t
u64       uint64_t
f32       float
f64       double
```

Integer ranges are:

| Type | Minimum | Maximum |
| --- | --- | --- |
| `i8` | -128 | 127 |
| `u8` | 0 | 255 |
| `i16` | -32768 | 32767 |
| `u16` | 0 | 65535 |
| `i32` | -2147483648 | 2147483647 |
| `u32` | 0 | 4294967295 |
| `i64` | -9223372036854775808 | 9223372036854775807 |
| `u64` | 0 | 18446744073709551615 |

Range and storage width are separate. All integer types currently use 64-bit locations in generated targets. Arrays and products do not pack them into 8, 16, or 32 bits, so changing from `i64` to a smaller type does not reduce memory use. Direct external byte I/O is not implemented.

## Booleans and comparisons

`bool` has two values, `true` and `false`. The `!` operator negates a boolean value.

```cerune
enabled: bool = true;
disabled: bool = !enabled;
```

`==` and `!=` compare numbers, booleans, strings, arrays, products, or sums of the same type. Evaluate operands once, left to right. Aggregates compare recursively in index/declaration order; sums compare tags and only the active payload. The first unequal member ends comparison. Numeric types additionally support `<`, `<=`, `>`, and `>=`. A comparison always produces `bool`.

```cerune
same: bool = enabled == true;
small: bool = 1 + 2 < 4;
different: bool = 0.1f32 != 0.2f32;
```

Boolean ordering and arithmetic are not supported. Comparisons do not perform implicit numeric conversion.

## Combining conditions with logical operators

`&&` returns `true` when both operands are `true`; `||` returns `true` when at least one is `true`. Both operands must have type `bool`, and the result is `bool`. Numbers such as 0 and 1 are not treated as Booleans.

Evaluate the left operand once, then decide whether to evaluate the right operand. This is short-circuit evaluation:

| Operation | Evaluate the right operand when | Result when skipped |
| --- | --- | --- |
| `left && right` | The left operand is `true` | `false` |
| `left \|\| right` | The left operand is `false` | `true` |

```cerune
count: i64 = 0;
print(count != 0 && 12 / count > 2); // false; the division is not executed
values: [i64; 2] = [4, 9];
index: i64 = 2;
print(index == 2 || values[index] == 9); // true; the out-of-bounds element is not read
```

When needed, the right operand is evaluated exactly once. Calls, effects such as `print`, and runtime checks in a skipped operand are not executed. Failure in the left operand or a required right operand still stops execution normally; errors are not suppressed.

Name resolution and type checking still apply to both operands. `false && missing` and `true || 1` are compile errors even though the right operand would not execute.

Precedence from strongest to weakest is unary operations, multiplication/division/remainder, addition/subtraction, shifts, ordering comparisons, equality comparisons, `&`, `^`, `|`, `&&`, then `||`. Thus `a < b && c == d || ready` means `((a < b) && (c == d)) || ready`. Repeated operators associate to the left; parentheses change grouping. `a < b < c` is not a range comparison.

Logical operators work anywhere a `bool` expression is accepted, including bindings, function arguments/results, array elements, and product fields, not only conditions. The [short-circuit example](../../examples/short_circuit.ceru) includes array traversal.

Cerune IR retains `and.short_circuit.bool` and `or.short_circuit.bool`. Lowering uses conditional jumps in bytecode, `&&`/`||` in C, branches in LLVM/QBE/Windows x86-64, and a Boolean-producing `if` in WAT. It never evaluates the right operand eagerly before selecting a result.

## Numeric literals

Integer literals use an explicit suffix if present, otherwise the expected integer type from context, and default to `i64` only when no type information is available.

```cerune
x: i64 = 42;
```

An integer literal remains a sequence of decimal digits until its type is known. Once its type is resolved, that type's range is checked and an out-of-range value is a compilation error. The sign is parsed as unary `-`, but `-9223372036854775808` is accepted as the minimum `i64` value.

Integer suffixes are `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, and `u64`. Unsuffixed numbers receive expected types from declarations, assignments, arguments, returns, fields, and array elements. Without an outer expected type, already typed values in the same arithmetic expression supply the type.

```cerune
count: i32 = 4;
first: infer = count + 1;
second: infer = (1 + 2) + count;
explicit: infer = 3000000000u32;
default: infer = 1 + 2; // no type information, so i64
```

`first` and `second` have type `i32`, independent of operand order. Already typed variables and suffixed literals are not reinterpreted; operations between different types fail type checking. Integer literals are not reinterpreted as floating-point values. Comparison results are `bool`, separately from their integer operand types.

An array declared with `infer` still derives its element type from the first element. `[1i32, 2]` is `[i32; 2]`, but `[1, 2i32]` is an error because the first element defaults to `i64`. An explicitly typed `[i32; 2]` accepts `[1, 2]`. Array indices remain `i64`; use `values[i64(index)]` for a `u32` position.

Floating-point literals without a suffix are contextually typed when an explicit floating-point type is available.

```cerune
a: f32 = 0.1 + 0.2;
b: f64 = 0.1 + 0.2;
```

That distinction is resolved in Cerune IR before backend lowering.

For example, the C backend may emit:

```c
float cerune_a = (0.1f + 0.2f);
double cerune_b = (0.1 + 0.2);
```

When no expected floating-point type is available, an unsuffixed floating-point literal defaults to `f64`.

```cerune
x: infer = 0.1 + 0.2;
```

Here `x` is inferred as `f64`.

A literal suffix can explicitly select its type:

```cerune
a: infer = 0.1f32 + 0.2f32;
b: infer = 0.1f64 + 0.2f64;
```

Scientific notation is also accepted for floating-point literals:

```cerune
x: f64 = 1.5e-3;
```

## Type checking

The four arithmetic operations require both operands to have the same type. Remainder and bit operations are restricted to integer types.

```text
i8 op i8 -> i8
u8 op u8 -> u8
i16 op i16 -> i16
u16 op u16 -> u16
i32 op i32 -> i32
u32 op u32 -> u32
i64 op i64 -> i64
f32 op f32 -> f32
f64 op f64 -> f64
```

Each floating-point operation rounds to its type using round-to-nearest, ties-to-even. Multiplication and addition must not contract into FMA when this changes the result. Signed zeros are preserved; NaN payloads need not match across routes. See the [rounding contract, external compiler conditions, and examples](../design/float-rounding.en.md).

Integer `+`, `-`, `*`, and signed integer unary `-` stop execution when their result is outside that integer type's range. They do not silently wrap from one end of the range to the other. Integer division by zero and division of the minimum signed integer value by `-1` also stop execution. Unary minus is rejected for unsigned `u8`, `u16`, and `u32`, including `-0u8` and `-0u32`. Integer division rounds toward zero.

The Cerune VM diagnoses the failing operation kind, type, bytecode instruction index, and source location. Generated C, LLVM IR, QBE IR, WebAssembly Text, and Windows x86-64 assembly retain corresponding checks or traps, making the enforcement point observable.

Cerune v0.1 performs no implicit numeric conversion.

For example, the following expression is a type error because its operands are `i64` and `f64`:

```cerune
x: infer = 1 + 0.1;
```

Explicit binding types are checked against the resolved expression type:

```cerune
x: f32 = 0.1 + 0.2;
```

The `f32` binding supplies the expected type to unsuffixed floating-point literals, so the expression is evaluated as `f32`.

This decision is recorded in Cerune IR and is not recomputed by individual backends.

Comparison operands must also have the same type. Cerune IR exposes the operand type separately from the resulting `bool` type.

## Remainder and bit operations

Bits are the zeros and ones that represent a number. A `u8` can be viewed as eight switches. Remainder and bit operations support all eight implemented integer kinds, not Boolean or floating-point values.

| Operation | Meaning | Example |
| --- | --- | --- |
| `a % b` | Integer remainder | `7 % 3` is `1` |
| `a & b` | Keep bits set in both values | `6u8 & 3` is `2u8` |
| `a \| b` | Set bits present in either value | `6u8 \| 3` is `7u8` |
| `a ^ b` | Set bits present in exactly one value | `6u8 ^ 3` is `5u8` |
| `~a` | Complement bits within the original type's width | `~0u8` is `255u8` |
| `a << b` | Shift left by b bits; fail outside the type's range | `3u8 << 2` is `12u8` |
| `a >> b` | Shift right by b bits | `12u8 >> 2` is `3u8` |

Binary operands must have the same integer kind, including the shift count, and the result retains that kind. Context can type unsuffixed literals, so `value: u8 = 1 << 3;` is valid. Already typed values are never implicitly converted.

Remainder agrees with division truncated toward zero. A nonzero remainder has the dividend's sign: `-7 % 3` is `-1`, and `7 % -3` is `1`. A zero divisor stops execution. The minimum signed value modulo `-1` is `0` and succeeds, unlike the corresponding division.

Shift counts must be nonnegative and less than the original type's bit width. Even `0u8 << 8` fails. Left shift succeeds only if the mathematical product by 2 to the power b fits the original type. `128u8 << 1` fails instead of wrapping to zero. A deliberately bit-discarding left shift is a separate operation and is not implemented.

Signed bit operations use two's complement semantics. Signed right shift preserves the sign: `-3i8 >> 1` is `-2i8`. Unsigned right shift fills high bits with zeros. Right shift discards low bits, and its rounding for negative values differs from integer division.

These binary operations evaluate the left operand and then the right operand, each once. If the left fails, the right is not executed. `&` and `|` do not short-circuit: `0u8 & (1u8 % 0)` fails. An enclosing `&&` or `||` can still skip an entire right operand containing these operations.

Comparisons bind more tightly than bit operations; write `(flags & mask) != 0` to test bits. See [bit flags](../../examples/bit_flags.ceru), [ring buffer](../../examples/ring_buffer.ceru), and [subset sum](../../examples/subset_sum_bits.ceru).

Cerune IR and bytecode retain the operation and original kind, such as `rem.u8`, `bit_and.u8`, `bit_or.u8`, `bit_xor.u8`, `bit_not.u8`, `shl.checked.u8`, and `shr.u8`. VM failures retain the source `NodeId` and `Span`. Out-of-range results, invalid shift counts, and a zero remainder divisor have distinct diagnoses.

## Explicit numeric conversions

Exact, value-preserving numeric conversion has two equivalent spellings:

```cerune
value: i64 = 42;
compact: infer = i64(value);
explicit: infer = convert<i64>(value);
```

All pairs among `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32`, and `f64` support explicit conversion. Conversion succeeds only if the destination preserves the value; otherwise execution stops. It does not truncate, round, or wrap. Conversions involving `bool`, arrays, or product types are not supported.

```cerune
count: u32 = 3000000000;
wide: i64 = i64(count);
back: u32 = convert<u32>(wide);
```

Every `i32` fits in `i64`, but `u32` to `i32` can fail despite equal bit widths. VM conversion failures retain both source and destination types.
`value: i32 = 2147483648;` is a compile-time literal error; `i32(2147483648)` evaluates an `i64` value and then fails conversion at runtime.

Both spellings evaluate the expression inside parentheses exactly once. The destination type is not passed into the input expression to change its arithmetic. A nonnumeric input is a compile-time error. Exactly one argument is required; a trailing comma is allowed. Conversion produces a value and cannot be used as a standalone statement.

### Floating-point conversions

| Conversion | Success condition |
| --- | --- |
| Integer to float | The integer is exactly representable in the destination |
| Float to integer | The value is finite, has no fractional part, is in range, and is not negative zero |
| `f32` to `f64` | All values except NaN; infinity and zero signs are preserved |
| `f64` to `f32` | The value is exactly representable and is not NaN; infinity and zero signs are preserved |
| Same type | The original value is kept, including NaN payload and sign bits |

```cerune
print(f64(1 / 2));       // 0: integer division happens first
print(f64(1) / f64(2));  // 0.5: floating-point division
print(i32(3.0));         // 3
print(f32(1.5));         // 1.5
// print(i32(3.7));      // fails: fractional part would be lost
// print(f32(16777217)); // fails: integer precision would be lost
// print(f32(0.1));      // fails: the input f64 value is not exactly representable as f32
// print(i32(-0.0));     // fails: an integer cannot preserve the zero sign
```

Exactness checks apply to the already evaluated input, not to an ideal mathematical result. Ordinary floating-point arithmetic and literal parsing still use floating-point rounding. These conversions do not add arbitrary precision or rounding operations. `f32(0.1f32)` preserves its already-rounded input; `f64(0.1f32)` widens that exact value and therefore differs from the `f64` literal `0.1`.

Floating-point conversion failures record source/destination types, reason, and source origin. Reasons distinguish range, precision, nonfinite input for integer conversion, NaN when changing float types, and negative zero for integer conversion. Explicit truncation uses the separate `trunc` operation below.

### Truncating the fractional part

`trunc<T>(value)` converts an `f32` or `f64` toward zero to integer type `T`: `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, or `u64`.

| Input | Result |
| --- | --- |
| `trunc<i64>(3.7)` / `trunc<i64>(-3.7)` | `3` / `-3` |
| `trunc<u8>(255.9)` / `trunc<i8>(-128.9)` | `255` / `-128` |
| `trunc<u64>(-0.9)` / `trunc<u64>(-0.0)` | Both yield `0` |
| `trunc<u8>(256.0)` / `trunc<u8>(-1.0)` | Stop with `conversion-out-of-range` |
| NaN or either infinity | Stop with `conversion-not-finite` |

The input is evaluated once, then checked for nonfinite values and for the range of the truncated integer, in that order. The destination does not change input type inference. Integer inputs and floating-point destinations are compile-time errors. Exactness checks for `convert<T>` and `T(value)` remain unchanged.

It works in `const` evaluation and functions with explicit type arguments. `trunc` is not a keyword; ordinary function and variable names remain available. Built-in `trunc<T>(value)` is distinct from user-defined `trunc::<T>(value)`.

IR and bytecode retain `convert.trunc`; backend IR retains `Rounded { rounding: Truncate, overflow: Checked }` along with types and source origins. See the [design](../design/truncating-conversions.en.md) and [example](../../examples/truncating_conversions.ceru).

### Choosing rounding and saturation

These operations accept the same inputs and destinations as `trunc<T>`. Evaluate the input once, round it, then check the integer range.

| Operation | Rule | `2.5` / `-2.5` |
| --- | --- | --- |
| `floor<T>(x)` | Toward negative infinity | 2 / -3 |
| `ceil<T>(x)` | Toward positive infinity | 3 / -2 |
| `round<T>(x)` | Nearest; ties away from zero | 3 / -3 |
| `round_ties_even<T>(x)` | Nearest; ties to even | 2 / -2 |

Ordinary variants stop with `conversion-not-finite` for NaN/infinity and `conversion-out-of-range` when the rounded result exceeds the range. Negative zero becomes integer zero. Invalid types or argument counts are compile-time errors.

All five operations also have `saturating_` variants. For example, `saturating_round<u8>(300.0)` yields 255. Values below the minimum and negative infinity become the minimum; values above the maximum and positive infinity become the maximum; NaN becomes zero. Failures while evaluating the input are not caught.

Rounding and overflow policies remain in AST, IR, bytecode, and backend IR; textual output distinguishes `convert.round` from `convert.saturating_round`. Constant IR retains both the original expression and its evaluated result. See the [generation and observation design](../design/rounding-conversions.en.md), [rounding example](../../examples/rounding_conversions.ceru), and [saturation example](../../examples/saturating_conversions.ceru). Names are not keywords and remain available for ordinary functions, comparisons, and explicit generic calls.

### Observing conversions

If input evaluation fails, the diagnostic points to that operation. For example, `i64(1 / 0)` stops at division before conversion is reached.

Cerune IR retains source/destination types, input, original spelling, and source location. Both spellings use the same operation kind; spelling is origin information. Integer-only conversions retain `ConvertInteger` and bytecode such as `convert.checked i32 -> u32`. Conversions involving floats use `ConvertNumeric` and `convert.exact i64 -> f64`. Both instructions retain the corresponding Cerune IR `NodeId` and `Span`.

C, LLVM, QBE, WAT, and Windows x86-64 retain integer values in 64-bit storage and check narrower integer destinations. Floating-point conversions retain a typed operation in backend IR and generate checks before accepting a changed representation. Same-type conversions need no native instruction; they remain explicit in Cerune IR and bytecode. Integer-to-`i64` conversions also need no extra native operation; float-to-`i64` conversions still require checks.

Functions and types cannot be defined with the built-in type names `bool`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`, or `string`; these are diagnosed at the definition. `convert` is not a keyword: ordinary calls such as `convert(value)` and comparisons such as `convert < limit` remain available. The `convert<type>(expression)` form is a built-in conversion whose meaning does not change when a user function named `convert` exists. User-defined generic functions use the call syntax `name::<type>(expression)`.

## Output

`flush();` is a built-in statement that hands pending stdout output to its connection. It has no arguments or result, and `flush` is reserved. Failure stops subsequent execution and diagnoses the operation and source origin. It does not guarantee screen rendering or durable storage. HIR/MIR/SSA/VM and all generated routes support it; the operation is observable as `flush.stdout`. See the [contract and coverage](../design/standard-io.en.md) and [example](../../examples/io_protocol/explicit_flush.ceru).

`print(expression);` accepts booleans, numbers, strings, fixed arrays, products, and sums across all routes. Aggregate formats include `[1, 2]`, `{x: 1}`, and `Found{text: "空"}`, quoting and escaping nested strings. Evaluate the entire argument before display, then append one LF. See [formats and generation](../design/aggregate-values.en.md).

Cerune keeps floating-point output precise enough to expose the behavior being observed.

Negative zero is printed as `-0`, preserving its sign rather than normalizing it to `0`.

The current formatting policy is:

```text
bool   `true` or `false`
i8 / u8 / i16 / u16 / i32 / u32 / i64 / u64    integer output
f32    9 significant digits
f64    17 significant digits
```

Significant digits include the integer part, not just digits after the decimal point. Finite values are printed with enough precision to recover the original value when parsed as the same type. Unnecessary trailing fractional zeros are omitted.

After rounding to the significant-digit limit, a decimal exponent below `-4` or at least the precision selects scientific notation using `e`. Other values use fixed notation. Exponents include a sign and at least two digits. For example, `1e-20` prints as `9.9999999999999995e-21`, not `0`. This means approximately ten to the power minus twenty; the longer digits expose the approximation stored in `f64`.

Printing does not change the value. `1.0 + 1e-20 == 1.0` is `true` because of arithmetic rounding, not because printing discards small values. See the [small-values example](../../examples/small_values.ceru).

The VM formats values by these rules. Generated C, LLVM, QBE, and Native use `snprintf` with `%.9g` and `%.17g`. WAT receives formatted bytes and complete lengths using `format_f32_v1` and `format_f64_v1`, validates them, then writes through `write_stdout_v1`.

WAT formats integers and Booleans in generated code. All ordinary output, including strings, quoting, and punctuation, uses the [versioned host contract](../design/wat-output-host.en.md), preserving requested/accepted counts and origins on partial failure without exposing internal memory.

The VM prints infinities as `inf` and `-inf`, and NaN as `NaN`. Generated-code spellings of special values depend on the target runtime. Decimal `print` output does not distinguish NaN payloads.

For example, an `f32` calculation such as:

```cerune
x: f32 = 0.1 + 0.2;
print(x);
```

may visibly produce the floating-point approximation rather than a shortened decimal representation.

Backends are responsible for preserving the observable print behavior while implementing it according to their own target conventions.


## Compiler input limits

Syntax/expression depth and per-file, total-byte, and file-count budgets are checked. Excess stops compilation with a diagnostic and preserves existing artifacts. See [compiler input limits](../design/frontend-limits.en.md) for defaults and Rust API configuration.
