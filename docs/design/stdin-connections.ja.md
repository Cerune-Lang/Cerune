# 実stdin接続：CLIと生成C・LLVM

[English](stdin-connections.en.md)

std:ioのread_lineの[共通契約](line-input.ja.md)を、CLIのHIR・MIR・SSA・VMと生成C・LLVMの実stdinへ接続します。QBE・WAT・Nativeへの入力loweringは後続です。

## 初版の範囲

| 入力先 | 扱い |
| --- | --- |
| Windows/Linuxの同期pipe・file | UTF-8の生バイトを読む。NUL・CR/LF・Ctrl-Zを変換しない |
| Linux端末 | OSがreadへ渡すバイトを読む。端末設定・echo・行編集・文字コード設定を変更しない |
| Windows console | 読取り前にunsupported-connectionで停止。UTF-16変換と元入力／変換後の消費数の設計は後続 |
| その他のOS | 初版の実stdin接続はunsupported-connection。既存の明示Rust readerは別途利用可能 |

入力は同期接続を対象とし、非同期handleや外部からのstdin差替えは今回の契約に含めません。既存のRust収集型実行APIはstdinへ暗黙接続しません。CLIのrun/run-ir/run-mir（--ssaを含む）/run-vmだけが実行用接続を選びます。emitやobserveは引き続き非実行です。

## 読取りと失敗

Linuxは`read(0, ..., 1)`、Windowsは借用した標準handleへ`ReadFile(..., 1, ...)`を使います。stdioの先読みを挟まず、stdinをcloseしません。CはCコンパイラの`_WIN32`／`__linux__`で対象を選ぶため、Ceruneを起動したOSから生成先を決めません。LLVMは明示した`--target x86_64-unknown-linux-gnu`／`--target x86_64-pc-windows-msvc`で選び、省略時は診断します。Rust接続はCLIバイナリの対象OSで選びます。

Windowsのconsole判定とhandle情報は一度だけ取得します。pipe切断はEOF、message pipeのERROR_MORE_DATAは返されたバイトを保持します。pipeの成功応答で0バイトなら空メッセージとして読み進め、EOFを捏造しません。ファイル終端は0またはERROR_HANDLE_EOFで確認します。LinuxはEINTRのみ再試行します。OS呼出し回数と共通接続の要求回数の一致は求めません。

根拠は[POSIX read](https://pubs.opengroup.org/onlinepubs/9799919799/functions/read.html)、[ReadFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile)、[ReadConsole](https://learn.microsoft.com/en-us/windows/console/readconsole)です。[Rust Stdin](https://doc.rust-lang.org/std/io/struct.Stdin.html)の共有バッファは今回の接続に使いません。

検査順は上限→接続→既知EOF→予算・確保→読取り→UTF-8です。unsupported-connectionも消費0でNodeId/Spanを保持します。入力命令の出自は仮想std:io本体を指し、呼出し元はIRから辿れます。入力は暗黙にflushしません。質問を出す場合はprint→flush→read_lineと書きます。

生成C・LLVMでは上限分の領域と所有情報を一度に確保します。失敗・空EOFでは解放し、成功時は同じ領域を既存の文字列所有リストへ渡して実長を課金します。終了診断はio-v1で、入力本文やOSエラー本文を含めず、後続処理を実行しません。生成経路の成功操作を読む共通APIはまだ追加していません。

LLVMは共通IRのReadLineを専用命令へloweringし、`cerune.input.read_line`、OS接続、UTF-8検査、予算・所有、診断を生成LLVMに展開します。入力地点には不変のNodeId/Spanを渡します。`--annotate-origins`はコメントだけを追加し、注釈なしでも同じ命令・診断を保ちます。

## 実行例

[ソース](../../tests/fixtures/line_input.ceru)、[入力](../../tests/fixtures/line_input.stdin)、[期待出力](../../tests/fixtures/line_input.stdout)は、空行・日本語・NUL・CRLF・結合文字・改行なし最終行を確認します。残る生成経路が揃うまでは通常のexamples一覧と分けて置きます。

まず`cargo build`を実行します。WindowsのPowerShellではcmdのファイルリダイレクトを使い、PowerShellのテキストpipeによる文字コード変換を挟みません。

```powershell
cmd /c "target\debug\cerune.exe run tests\fixtures\line_input.ceru < tests\fixtures\line_input.stdin"
```

Linuxでは次のように実行できます。

```sh
target/debug/cerune run tests/fixtures/line_input.ceru < tests/fixtures/line_input.stdin
cargo run -- emit-c tests/fixtures/line_input.ceru -o target/line-input.c
clang -std=c11 -O2 target/line-input.c -o target/line-input
target/line-input < tests/fixtures/line_input.stdin
cargo run -- emit-llvm tests/fixtures/line_input.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/line-input.ll
clang --target=x86_64-unknown-linux-gnu -O2 target/line-input.ll -o target/line-input-llvm
target/line-input-llvm < tests/fixtures/line_input.stdin
```

[経路テスト](../../tests/stdin_routes.rs)はCLI各経路とC・LLVMの-O0/-O2を比較します。対話テストは質問を受信するまで入力を送らず、pipeを閉じてEOFも確認します。実fileの共有位置から先読みがないことを検証し、入力失敗・契約外応答・割込み・確保失敗は比較用接続へ注入します。LLVMではOS呼出しへの試験応答によりEINTR、空message、ERROR_MORE_DATA、pipe切断、ファイル終端、console拒否も検証します。Windows consoleの拒否理由と検査順はテストしますが、画面を開くconsole試験やUTF-16変換の対応完了を意味しません。
