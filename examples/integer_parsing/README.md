# 整数読取り

[English](README.en.md)

`import "std:num" as num;` と `num::parse_i64` の基本例です。

`values.ceru` は `0042`・i64最小値・範囲外・書式不正を順に読み取ります。全文を検査するため、巨大な数字列の末尾に文字があっても書式不正になります。

実行：`cargo run -- run examples/integer_parsing/values.ceru`

結果は `Value { value }`・`InvalidFormat {}`・`OutOfRange {}`。不正な入力も値なので、`match` で分岐できます。

[詳細契約](../../docs/design/integer-parsing.ja.md) / [CLI引数とseed](../arguments/README.md)

