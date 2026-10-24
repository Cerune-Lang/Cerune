# Integer parsing

[日本語](README.md)

A basic example of `import "std:num" as num;` and `num::parse_i64`.

`values.ceru` parses `0042`, i64::MIN, an out-of-range value and invalid syntax. Complete syntax validation means a huge digit sequence ending in a letter is still invalid syntax.

Run: `cargo run -- run examples/integer_parsing/values.ceru`

Results are `Value { value }`, `InvalidFormat {}` and `OutOfRange {}`. Invalid input is a value that can be handled with `match`.

[Contract](../../docs/design/integer-parsing.en.md) / [CLI arguments and seed](../arguments/README.en.md)

