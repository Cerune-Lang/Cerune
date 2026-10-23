# Explicit integer parsing

[日本語](integer-parsing.ja.md)

## Public contract

Import the reserved virtual standard module with `import "std:num" as num;` and call `num::parse_i64(text)`. The result uses the existing enum facility: `num::ParseI64`.

| Result | Meaning |
| --- | --- |
| `Value { value: i64 }` | Success |
| `InvalidFormat {}` | The complete input does not match ASCII `-?[0-9]+` |
| `OutOfRange {}` | Valid syntax, outside the i64 range |

- Leading zeros are accepted; `-0` is zero. The range is -9223372036854775808 through 9223372036854775807.
- Empty input, lone signs, `+`, whitespace, CR/LF, NUL, non-ASCII digits, fractions, exponents, separators and radix prefixes are rejected. No trimming or Unicode normalization occurs.
- Complete syntax validation precedes range checking. Even an enormous digit sequence ending in `x` returns `InvalidFormat`.
- Parsing does not mutate its input, perform I/O, allocate additional strings or retry. Invalid input returns a value; existing resource-limit failures remain execution failures.
- Parsing u64, other integer types and floating-point values, implicit conversion, function evaluation in constants and a generic Result type are outside this change.

## Implementation and observation

Syntax and range checks are implemented once in bundled [`src/standard/num.ceru`](../../src/standard/num.ceru). The normal frontend processes this module, leaving its loops, comparisons, arithmetic and enum construction visible from HIR onward. Negative accumulation with checks before multiplication and subtraction accommodates i64::MIN without intermediate overflow.

`std:num` is a version-1 virtual source. Aliased imports share its type identity and SourceId within a compilation. Existing source observation APIs expose the body and distinguish its origin from user sources. Relative imports are unchanged; APIs such as `compile(&str)` do not search files or environment variables. SourceIds identify sources within a compilation, not persistent fixed numbers.

The only new common IR primitive is internal, pure `StringByteAt(value, index)`.

| Property | Contract |
| --- | --- |
| Arguments | string and i64, evaluated once each in that order |
| In range | Return one UTF-8 byte as i64 in 0..255 |
| Out of range | Negative or length-or-greater indices return -1 without a memory read |
| String | Raw bytes including NUL; no decoding, normalization, mutation or allocation |
| Origin | Preserve call NodeId/Span through HIR, MIR, SSA, Bytecode, LIR and code generation |

Only the body of `std:num` resolves `__cerune_string_byte_at` to this operation. It is not a public user-source API. Executors and generated routes implement byte access, not separate numeric grammars or range algorithms. They do not call libc integer parsers. WAT needs no public memory or new host contract. Observation does not change input or execution state.

## Verification and examples

Independent ASCII grammar checks and Rust i64 parsing supply expected values for HIR, MIR, SSA, VM, C, LLVM, QBE, WAT and Native ASM/Object comparisons. Cases cover bounds, long leading zeros, invalid grammar, UTF-8 bytes, negative/large indices, short circuiting, single argument evaluation, array copies and repeated calls under a small string limit. Native is checked from both MIR and SSA. Existing tool-required CI executes external toolchains.

- [Basic example](../../examples/integer_parsing/README.en.md): success, invalid syntax and range errors.
- [Seed argument example](../../examples/arguments/README.en.md): integer parsing followed by domain checks.

The seed example restricts seeds to 0..2147483646 and counts to 0..100. These are application constraints, not limits of `parse_i64`. Its diagnostics do not automatically echo argument contents.
