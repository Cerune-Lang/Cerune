# 子プロセス結果型の使い方

[English](README.en.md)

[outcome.ceru](outcome.ceru)は`std:process`の型を使い、**自分で構築した結果値**を処理します。子プロセスは起動しません。実行操作は別の[example](../process_execution/README.md)で扱います。

| 確認すること | 表現 |
| --- | --- |
| 操作完了と非0終了の区別 | `Failure::None`と32ビット最大の`Exit::Code` |
| 失敗後の部分出力と継続 | `Timeout`、`Transfer { Stdout }`、最後の`continued` |
| 本文の保持 | `[255, 0, 13, 10]`（不正UTF-8・NUL・CR/LFをバイト列として保持） |
| 後始末と進行状態 | 未完了の後始末、生報告の欠落、`progress_known` |
| コピーの独立性 | コピーを65へ変更しても元の先頭は255 |

リポジトリのルートから実行します。

```sh
cargo run --quiet -- run examples/process_results/outcome.ceru
cargo run --quiet -- run-vm examples/process_results/outcome.ceru
cargo run --quiet -- emit-sources examples/process_results/outcome.ceru
cargo run --quiet -- emit-ir examples/process_results/outcome.ceru
```

[期待出力](outcome.stdout)は4実行器と生成C・LLVMで比較します。宣言・出自は`emit-sources`、enum・配列・コピーの生成過程は`emit-ir`以降で確認できます。C/LLVMの実行比較には対応するClangが必要です。

```sh
cargo test --test modules process_types
```

要求のフィールド、結果の意味、実行機能への残りの接続は[公開型の設計](../../docs/design/subprocess-values.ja.md)を参照してください。
