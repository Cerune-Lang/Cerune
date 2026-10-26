# ファイルの新規保存と読戻し

[English](README.en.md)

[roundtrip.ceru](roundtrip.ceru)は日本語・NUL・CR/LF・結合文字を保存し、読戻した文字列の一致とバイト数を表示します。期待する出力は`true`、`16`です。

**現在はRust APIから明示したメモリ接続で実行します。CLIの実ファイル接続は未実装です。** 下のテストがHIR・MIR・SSA・VMそれぞれに空の独立接続を用意し、出力・保存内容・操作記録を比較します。同じ接続で再実行すると、既存ファイルを変えず`already-exists`で停止します。

```sh
cargo test --test standard_input file_io::roundtrip_example
cargo run --quiet -- emit-ir examples/files/roundtrip.ceru
cargo run --quiet -- emit-mir examples/files/roundtrip.ceru
cargo run --quiet -- emit-bytecode examples/files/roundtrip.ceru
```

通常の`run`では`unconfigured`診断で停止します。C・LLVM・QBE・WAT・Native生成は未対応の診断を返します。接続を暗黙に選んだり、ソースのあるディレクトリへ保存したりしません。[設計とRust API](../../docs/design/file-io.ja.md)に現在の対応範囲をまとめています。
