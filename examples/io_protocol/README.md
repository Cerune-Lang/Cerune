# 入出力の動作例

[English](README.en.md)

[output_sequence.ceru](output_sequence.ceru)は、関数引数の評価と出力順、日本語・NUL・CR/LFの保持を確認します。[期待値](output_sequence.stdout)は各経路で同じです。CLIは実行中に出力先へ渡します。標準入力とソース内のflushは使いません。

```sh
cargo run --quiet -- run examples/io_protocol/output_sequence.ceru
cargo test --test execution_output --test execution_output_cli
```

[explicit_flush.ceru](explicit_flush.ceru)は関数とループ内で`flush();`を呼び、出力の順序と日本語・NUL・CR/LFを保持します。[期待値](explicit_flush.stdout)を各経路で比較します。flush失敗時は後続処理を実行しません。

```sh
cargo run --quiet -- run examples/io_protocol/explicit_flush.ceru
cargo run --quiet -- emit-mir examples/io_protocol/explicit_flush.ceru
cargo test --test explicit_flush --test execution_output
```

[output_values.ceru](output_values.ceru)は、引数を一度評価してからu64最大値・引用付き文字列・真偽値・負のゼロ・小さな数値を表示します。[期待値](output_values.stdout)を4実行器と生成C・LLVM・QBE・Nativeで比較し、部分書込みや失敗は`cargo test --test c_output --test llvm_output --test qbe_output --test native_output`で確認します。自前Objectの同じ比較は`cargo test --lib native_object_output`で実行します。

[input_flow.ceru](input_flow.ceru)は、ソース内で用意した入力値で操作を選ぶ例です。**標準入力は読みません。** 状態と操作の規則はCerune側に置きます。

| 値・操作 | 確認すること |
| --- | --- |
| `Line` | attackでHPを3減らし、healで2増やす |
| LF・CRLF・改行なし | 改行を保持した文字列を、プログラムが明示的に判別する |
| 空行 | EOFと区別し、HPを変えない |
| `End`・`Failed` | 処理を止め、後続の操作を実行しない |
| 日本語・NUL・CR/LF | バイト数を保持する |
| 合成済み文字と結合文字 | Unicodeを正規化しない |

実行結果は[input_flow.stdout](input_flow.stdout)です。`Input`と失敗番号17はこの例の独自定義で、標準ライブラリのAPIやOSのエラー番号ではありません。

```sh
cargo run --quiet -- run examples/io_protocol/input_flow.ceru
cargo run --quiet -- run-mir examples/io_protocol/input_flow.ceru
cargo run --quiet -- run-mir examples/io_protocol/input_flow.ceru --ssa
cargo run --quiet -- run-vm examples/io_protocol/input_flow.ceru
cargo test --test io_protocol
```

[標準入出力の設計](../../docs/design/standard-io.ja.md)では、実際の入力、flush、失敗、観測を扱います。この例は入力値を受けた後の分岐の基準であり、OSからの読取りや対話時の出力順を検証するものではありません。
