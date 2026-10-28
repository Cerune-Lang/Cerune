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

テストは実ディレクトリとメモリ接続を分け、出力・保存内容・操作記録・失敗を比較します。--file-root省略時はunconfiguredで停止します。Native生成は未対応の診断を返します。WATは下記の版付きテストhostで実行できます。保存先を暗黙に選びません。Linuxの読取りには/proc/self/fdが必要です。[契約と対応範囲](../../docs/design/file-io.ja.md)も参照してください。


生成Cでも同じ例を実行できます。Cのコンパイル対象でWindows/Linuxを選びます。

~~~sh
cargo run --quiet -- emit-c examples/files/roundtrip.ceru -o roundtrip.c
clang -std=c11 roundtrip.c -o roundtrip
mkdir c-output
./roundtrip --cerune-runtime-v1 --file-root c-output --
~~~

WindowsのPowerShellでは最後を .\roundtrip.exe に読み替えます。MinGW向けClangではwmainを使うため-municodeも指定します。設定の後の--は必須です。省略した接続を作業ディレクトリで補いません。以降の引数はプログラムへ渡されます。

生成LLVMでも同じ例を実行できます。対象を明示し、生成後はClangでリンクします。Linuxでは次のコマンドを使います。

~~~sh
cargo run --quiet -- emit-llvm examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.ll
clang --target=x86_64-unknown-linux-gnu roundtrip.ll -o roundtrip
mkdir llvm-output
./roundtrip --cerune-runtime-v1 --file-root llvm-output --
~~~

Windowsでは両コマンドのtargetをx86_64-pc-windows-msvcにし、出力をroundtrip.exe、実行を.\roundtrip.exeにします。ClangとMSVC互換のリンク環境が必要です。起動設定はCと同じです。実接続比較に加え、短い転送やclose失敗を注入する開発用テストも用意しています。

~~~sh
cargo test --lib codegen::llvm::files::tests
~~~

QBE 1.3でも同じ例を実行できます。Linuxではターゲットをamd64_sysvに明示します。

~~~sh
cargo run --quiet -- emit-qbe examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.ssa
qbe -t amd64_sysv -o roundtrip.s roundtrip.ssa
clang --target=x86_64-unknown-linux-gnu roundtrip.s -o roundtrip
mkdir qbe-output
./roundtrip --cerune-runtime-v1 --file-root qbe-output --
~~~

WindowsではCeruneとClangのtargetをx86_64-pc-windows-msvc、QBEの-tをamd64_winにし、出力をroundtrip.exe、実行を.\roundtrip.exeにします。ClangとMSVC互換のリンク環境が必要です。起動設定はC・LLVMと同じです。CERUNE_TEST_QBEにQBEのパスを指定すると、実接続と応答注入の比較を必須実行できます。

~~~sh
cargo test --test standard_input file_io::generated::qbe_tests
cargo test --lib codegen::qbe::files::tests
~~~

WATでも同じCeruneソースを実行できます。以下は実ファイルを開かないメモリ上のテストhostです。設定ファイルは初期状態と確認対象を指定し、保存内容は任意の試験記録へコピーします。

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files-roundtrip.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm target/files-roundtrip.wat -o target/files-roundtrip.wasm
CERUNE_TEST_WAT_FILES_FILE=tests/fixtures/file_io/wat_host.json CERUNE_TEST_WAT_FILES_TRACE=target/files-trace.json node tests/support/run_wasm_input.cjs target/files-roundtrip.wasm
~~~

NodeとWABT 1.0.39を使います。WindowsのPowerShellでは最後の行の前に、$env:CERUNE_TEST_WAT_FILES_FILE='tests/fixtures/file_io/wat_host.json' と $env:CERUNE_TEST_WAT_FILES_TRACE='target/files-trace.json' を設定し、node以降を実行します。期待出力はtrue、16です。files-trace.jsonのcontents.sample.txtに16バイトの配列が残ります。毎回独立したメモリ接続を作るため、実OSディレクトリへsample.txtを保存した検証ではありません。

[WATのファイルhost契約](../../docs/design/wat-file-host.ja.md)に従うhostが必要です。旧hostはimport欠落で停止します。未設定はconfigured=falseで表し、操作時にunconfiguredで停止します。実OSのファイルhostは別途必要で、--file-rootやcwdを自動で使いません。

~~~sh
cargo test --test standard_input file_io::wat
~~~
