# optional letと整形

[English](README.en.md)

[bindings.ceru](bindings.ceru)は、通常の宣言・for初期化・配列反復に`let`を付けた例です。型・`mut`・評価順・短絡・配列コピーの意味は従来と同じです。`:`は必須です。初期値を持つ束縛では、下の例のように`infer`だけ省略できます。

リポジトリのルートから試します。

```sh
cargo run --quiet -- run examples/formatting/bindings.ceru
cargo run --quiet -- fmt examples/formatting/bindings.ceru --let hide -o target/bindings.ceru
cargo run --quiet -- run target/bindings.ceru
cargo run --quiet -- fmt target/bindings.ceru --let hide --check
```

両方の実行結果は同じです。

```text
6
2
5
[9, 4]
[2, 4]
false
```

| 選択 | 動作 |
| --- | --- |
| `--let preserve`（既定） | 元のlet有無を保持 |
| `--let show` / `hide` | 宣言のletを追加／除去。識別子のletは保持 |
| 保存指定なし | 整形結果を標準出力へ表示 |
| `--check` | 変更が必要なら終了1。ファイルを変更しない |
| `--write` / `-o <file>` | 入力を置換／別ファイルへ保存。併用不可 |

コメントとリテラルを保持し、import先の読込み・実行はしません。整形後はそのソースに対応した位置情報でコンパイルされます。[契約と制限](../../docs/design/formatting.ja.md)を参照してください。

## inferを省略する

[infer.ceru](infer.ceru)は、`value: = expression;`で型推論を使う例です。通常の束縛とfor初期化、u8配列のコピー、u64・小数・文字列・短絡を確認します。

```sh
cargo run --quiet -- run examples/formatting/infer.ceru
cargo run --quiet -- fmt examples/formatting/infer.ceru --let show -o target/infer.ceru
cargo run --quiet -- run target/infer.ceru
cargo run --quiet -- emit-ir target/infer.ceru
```

両方の実行結果は次のとおりです。IRには省略前と同じ解決型が表示されます。

```text
6
[9, 4]
[2, 4]
18446744073709551615
1.5
日本語
false
```

`infer`を明示しても同じ推論になります。fmtはその有無を保持し、`--let`だけを切り替えます。配列反復は`value: infer in values`のままで、関数引数・戻り値・定数・フィールドへ省略を広げません。[設計](../../docs/design/infer-omission.ja.md)に範囲と出自の扱いをまとめています。
