# 標準入力

[English](README.en.md) / [サンプル一覧](../README.md)

[lines.ceru](lines.ceru)は、promptをflushしてからUTF-8の行を読み、内容とバイト数を表示します。行の上限は64バイトで、LF・CRLFも含みます。空行は行として返り、EOFで終了します。改行なしの最終行も一度表示します。

| 入力 | 表示される内容／バイト数 |
| --- | --- |
| Cerune + LF | `["Cerune\n"]` / 7 |
| LFだけ | `["\n"]` / 1 |
| 日本語 + LF | `["日本語\n"]` / 10 |
| EOF | end |

Windows/Linuxのfile・pipeに対応します。Windowsの直接コンソール入力はunsupported-connectionで停止します。Linux端末ではOSの端末設定を変更しません。入力待ちを含むため、ディレクトリ直下の一括実行には含めません。

## 実行

UTF-8の入力ファイルを用意し、リポジトリルートで実行します。

~~~sh
cargo build
target/debug/cerune run examples/stdin/lines.ceru < input.txt
target/debug/cerune run-vm examples/stdin/lines.ceru < input.txt
~~~

PowerShellでは文字コード変換を避け、`cmd /c "target\debug\cerune.exe run examples/stdin/lines.ceru < input.txt"`とします。

## 生成過程とNative実行

~~~sh
cargo run -- emit-ir examples/stdin/lines.ceru -o target/lines.ceir
cargo run -- emit-lir examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu -o target/lines.lir
cargo run -- emit-asm examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/lines.s
cargo run -- emit-obj examples/stdin/lines.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/lines.o
cc target/lines.o -o target/lines
target/lines < input.txt
~~~

Windowsではtargetを`x86_64-pc-windows-msvc`、Object名を`target/lines.obj`に変え、`clang target/lines.obj -o target/lines.exe`でリンクします。実行は同じcmdの入力リダイレクトを使います。

[接続・診断の契約](../../docs/design/stdin-connections.ja.md)と[WAT hostでの実行](../../docs/design/wat-input-host.ja.md)も参照できます。テストは同じ入力で実行器・C・LLVM・QBE・WAT・Nativeを比較します。上限超過や不正UTF-8では入力本文を診断に含めず停止します。

[retained_lines.ceru](retained_lines.ceru)は2行を読み終えてから表示します。例えば `x + LF`、`y + LF` を渡すと `["x\n"]`、`["y\n"]` を表示し、2回目の入力中も最初の行を保持します。各行の上限は64バイト、EOFは `end` と表示します。上の実行コマンドのファイル名を置き換えて試せます。
