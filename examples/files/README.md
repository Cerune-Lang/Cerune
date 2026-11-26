# 新規保存と読戻し

[English](README.en.md)

## ELF64 ヘッダを読む

[elf_header.ceru](elf_header.ceru) は `std:fs::bytes` で `input.o` を読み、先頭64バイトが揃っていることと ELF の識別子・ビット幅・バイト順を確認します。次に16・32・64ビットのリトルエンディアンのフィールドを整数に変換し、`copy.o` に全バイトを保存します。

```sh
mkdir elf-sample
cargo run --quiet -- emit-obj examples/recursion/factorial.ceru --target x86_64-unknown-linux-gnu -o elf-sample/input.o
cargo run --quiet -- run examples/files/elf_header.ceru --file-root elf-sample
```

結果は順に、オブジェクトの種類（1 = ET_REL）、版（1）、入口アドレス（0）です。64バイトに満たない入力ではメッセージを表示して終了します。経路を比較するときは、各ディレクトリに同じ `input.o` を置いてください。

生成Cでも同じ入力を使えます。次の Linux の例では、`cmp` で元のオブジェクトと保存したファイルの全バイトを照合します。

```sh
mkdir elf-c
cp elf-sample/input.o elf-c/input.o
cargo run --quiet -- emit-c examples/files/elf_header.ceru -o elf-c/reader.c
clang -std=c11 -O2 elf-c/reader.c -lm -o elf-c/reader
./elf-c/reader --cerune-runtime-v1 --file-root elf-c --
cmp elf-c/input.o elf-c/copy.o
```

生成LLVMを試す場合も、同じ `input.o` を使います。Linux x86-64 では次のようにターゲットを指定します。

```sh
mkdir elf-llvm
cp elf-sample/input.o elf-llvm/input.o
cargo run --quiet -- emit-llvm examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-llvm/reader.ll
clang --target=x86_64-unknown-linux-gnu -O2 elf-llvm/reader.ll -lm -o elf-llvm/reader
./elf-llvm/reader --cerune-runtime-v1 --file-root elf-llvm --
cmp elf-llvm/input.o elf-llvm/copy.o
```

QBE 1.3では `.ssa` からアセンブリを生成します。Linux x86-64 には `amd64_sysv`、Windows x86-64 には `amd64_win` を指定します。Linuxでの手順を示します。

```sh
mkdir elf-qbe
cp elf-sample/input.o elf-qbe/input.o
cargo run --quiet -- emit-qbe examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-qbe/reader.ssa
qbe -t amd64_sysv -o elf-qbe/reader.s elf-qbe/reader.ssa
clang --target=x86_64-unknown-linux-gnu elf-qbe/reader.s -lm -o elf-qbe/reader
./elf-qbe/reader --cerune-runtime-v1 --file-root elf-qbe --
cmp elf-qbe/input.o elf-qbe/copy.o
```

生成WATを[参照ホスト](../../experiments/wat_host/README.md)で実行するときは、`--file-root` に `input.o` を置いたディレクトリを指定します。Linuxでは次のコマンドでコピーの全バイトを照合できます。

```sh
mkdir elf-wat
cp elf-sample/input.o elf-wat/input.o
cargo run --quiet -- emit-wat examples/files/elf_header.ceru -o elf-wat/reader.wat
cargo run --quiet --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- elf-wat/reader.wat --file-root elf-wat --
cmp elf-wat/input.o elf-wat/copy.o
```

Linux x86-64 で同じ例を Native 実行するには、次のコマンドを使います。`cmp` の終了コード0は全バイトの一致を表します。

```sh
mkdir elf-native
cp elf-sample/input.o elf-native/input.o
cargo run --quiet -- emit-obj examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-native/reader.o
clang elf-native/reader.o -o elf-native/reader
./elf-native/reader --cerune-runtime-v1 --file-root elf-native --
cmp elf-native/input.o elf-native/copy.o
```

## テキストを保存して読み戻す

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

テストは実ディレクトリとメモリ接続を分け、出力・保存内容・操作記録・失敗を比較します。--file-root省略時はunconfiguredで停止します。Nativeも明示したWindows/Linux targetで利用できます。WATは下記の版付きテストhostで実行できます。保存先を暗黙に選びません。Linuxの読取りには/proc/self/fdが必要です。[契約と対応範囲](../../docs/design/file-io.ja.md)も参照してください。


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

## WATを実ファイルへ接続する

Windows/Linuxでは独立した[参照host](../../experiments/wat_host/README.md)を使えます。リポジトリ直下で実行します。

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files.wat
mkdir target/wat-files-data
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/files.wat --file-root target/wat-files-data --
~~~

出力はtrueと16です。指定したディレクトリのsample.txtへ16バイトを保存して読み戻します。同じ保存先での再実行はalready-existsで停止します。末尾の--はhost設定の終端で、その後がプログラム引数です。root省略時は未設定になります。内部memoryは公開しません。WATはemit-onlyのままで、参照hostはCeruneのbuild/releaseに含めません。

## Nativeで実行

Linuxでは、Objectを生成して明示したClangでリンクします。保存先は新規ディレクトリにします。

```sh
cargo run --quiet -- emit-obj examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.o
clang --target=x86_64-unknown-linux-gnu roundtrip.o -o roundtrip
mkdir native-output
./roundtrip --cerune-runtime-v1 --file-root native-output --
```

Windowsではtargetを`x86_64-pc-windows-msvc`、Objectを`roundtrip.obj`、実行ファイルを`roundtrip.exe`にし、`./roundtrip.exe`で起動します。MSVC互換のリンク環境が必要です。`emit-asm`で生成したASMからのリンクも可能です。`--ssa`でSSA経由の生成を選べます。

出力は`true`と`16`です。保存内容の日本語・NUL・CR/LF・結合文字をそのまま読み戻します。同じ保存先で再実行すると既存ファイルの上書きを拒否するため、比較ごとに別ディレクトリを使います。

## buildして配布する

同じroundtripを完成成果物として保存・配布できます。Linuxの例です。

```sh
cargo run --quiet -- build native examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o target/files-build
cargo run --quiet -- release native target/files-build -o target/files-release
mkdir target/files-data
./target/files-release/build/program --cerune-runtime-v1 --file-root target/files-data --
```

Windowsではbuildのtargetを`x86_64-pc-windows-msvc`、linkerを実際のClangパスへ変更し、`./target/files-release/build/program.exe`で起動します。各出力先は新規に用意します。`--ssa`をbuildに付けるとSSA経由になります。

出力と保存内容は上の例と同じです。配布ディレクトリを別の場所へ移しても、起動時に明示したrootを使います。build/release自体はsample.txtを作らず、実行ファイルの再リンクや最適化もreleaseでは行いません。

`manifest.json`・`runtime.json`の`file_connection`に起動契約とOS要件が記録され、release.json・runtime-requirements.jsonへ引き継がれます。実際の保存先は埋め込みません。[build](../../docs/design/native-build.ja.md)・[release](../../docs/design/native-release.ja.md)の契約も参照してください。
