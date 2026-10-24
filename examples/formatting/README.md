# optional letと整形

[English](README.en.md)

[bindings.ceru](bindings.ceru)は、通常の宣言・for初期化・配列反復に`let`を付けた例です。型・`mut`・評価順・短絡・配列コピーの意味は従来と同じです。`:`と型指定（`infer`を含む）は必須です。

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
