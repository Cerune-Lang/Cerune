# 新規保存と読戻し

[English](README.en.md)

[roundtrip.ceru](roundtrip.ceru)は日本語・NUL・CR/LF・結合文字を保存し、読戻し結果の一致とバイト数を表示します。期待する出力はtrue、16です。

Windows/Linuxで空のディレクトリを用意して実行します。

~~~sh
mkdir sample-output
cargo run --quiet -- run examples/files/roundtrip.ceru --file-root sample-output
~~~

sample.txtを作成します。同じディレクトリで再実行すると、既存の内容を変えずalready-existsで停止します。run-ir、run-mir、run-mir --ssa、run-vmでも実行できます。比較には経路ごとに空の独立ディレクトリを使ってください。

~~~sh
cargo test --test standard_input file_io
cargo run --quiet -- emit-ir examples/files/roundtrip.ceru
cargo run --quiet -- emit-mir examples/files/roundtrip.ceru
cargo run --quiet -- emit-bytecode examples/files/roundtrip.ceru
~~~

テストは実ディレクトリとメモリ接続を分け、出力・保存内容・操作記録・失敗を比較します。--file-root省略時はunconfiguredで停止します。C・LLVM・QBE・WAT・Native生成は未対応の診断を返します。保存先を暗黙に選びません。Linuxの読取りには/proc/self/fdが必要です。[契約と対応範囲](../../docs/design/file-io.ja.md)も参照してください。
