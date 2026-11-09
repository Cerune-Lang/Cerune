# 子プロセス操作の失敗を受け取る

[English](README.en.md)

[unconfigured.ceru](unconfigured.ceru)は`std:process/run`を呼び、未接続の失敗を受け取って`continued`まで進みます。通常CLIの実行対象登録は未実装なので、OS上の子は起動しません。`std:process`は型、`std:process/run`は実行操作です。`report.failure`に失敗の分類・段階、`stdout`と`stderr`に生バイトを保持します。

```sh
cargo run --quiet -- run examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru --ssa
cargo run --quiet -- run-vm examples/process_execution/unconfigured.ceru
cargo test --test modules process_types
```

[期待出力](unconfigured.stdout)を4実行器で比較します。Rust APIの実OS接続、未対応の生成経路、出自と領域確保は[実行設計](../../docs/design/subprocess-execution.ja.md)を参照してください。
