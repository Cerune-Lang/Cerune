# QBEのWindows対応

[English](qbe-windows.en.md)

## ターゲットと責務

| Ceruneの明示ターゲット | QBE 1.3の指定 | 検証するリンク環境 |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `-t amd64_sysv` | Linux x86-64、Clang / cc |
| `x86_64-pc-windows-msvc` | `-t amd64_win` | Windows x64、Clang、MSVC CRT・リンカ |

`emit-qbe`はSSAを生成するだけです。外部QBEとリンカは利用側が明示的に呼びます。ホストOSからターゲットを選びません。文字列・動的配列・標準入出力には指定が必須で、既存の数値専用・未指定出力はLinux/SysV前提を保ちます。

CIは[公式QBE 1.3](https://c9x.me/compile/release/qbe-1.3.html)をSHA-256で確認します。WindowsジョブではMinGW/UCRTでQBE本体をビルドしますが、生成プログラムにはClang/MSVCを使います。QBEへのパッチは加えません。

## 出力と停止

Windowsでは、最初のCerune操作より前に`_setmode(1, 32768)`と`_setmode(2, 32768)`を呼びます。初期化失敗時は終了コード1で止めます。stdout/stderrはバイナリモードとなり、数値・真偽値もLF、文字列も日本語・NUL・CR/LFをそのまま出力します。

文字列と診断の静的データは読み取り専用の`.rdata`に置きます。言語エラー時は既存stdoutをflushし、`_write`へ不変の診断片を渡します。Windowsのcount引数は32bitです。診断片は短い固定形式なので、QBEの64bit長から幅を明示して渡します。

言語エラーは診断後に`abort`で止めます。その直前に`_set_abort_behavior(0, 3)`でCRTの追加メッセージとクラッシュ収集を無効にし、終了コード3を返します。[Microsoftの仕様](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/set-abort-behavior?view=msvc-170)に沿った停止であり、テストはコード3だけでなく理由・NodeId・SourceId・byte範囲・先行出力の一致も要求します。OSの書き込み失敗時に診断全体が届く保証はありません。

## QBE 1.3で確認した引数の制約

`examples/function_arguments.ceru`で、Windowsの第5引数以降にある`f32`・`f64`が不正なAssemblyになることを再現しました。QBE 1.3の`amd64/winabi.c`はこのスタック引数を整数クラスで読み、浮動小数点比較へ整数レジスタが渡る場合があります。

Ceruneの内部関数間では、この位置にある`f32`を`w`、`f64`を`l`のビット列として渡し、入口で同幅の`cast`により復元します。隠れた集約戻り値ポインタも引数位置に数えます。QBEの[cast](https://c9x.me/compile/doc/il.html#Cast-and-Copy)はビット列を保持し、丸めや数値変換を行いません。引数評価後の受け渡しだけが変わり、評価順やCerune IRの型は変わりません。

この処理はWindows向けの内部呼び出しに限定します。生成SSAの関数引数・呼び出し・castを直接観察できます。外部関数との一般的なFFI互換性を保証するものではありません。

非有限浮動小数点定数は、生のビット定数で生成します。QBEをビルドしたCRTの`scanf`が`inf`などの表記を読めるかに依存しないためです。有限値の既存表記は維持します。

## 実行比較と観測

[CLIの手順](../reference/cli.ja.md#qbeのターゲット指定)でexampleからSSA・Assembly・実行ファイルを順に作れます。自動比較はWindowsで次のように実行します。

```powershell
$env:CERUNE_TEST_QBE = 'C:\tools\qbe.exe'
$env:CERUNE_TEST_QBE_CLANG = 'C:\Program Files\LLVM\bin\clang.exe'
cargo test --test qbe_windows -- --include-ignored
```

外部ツールのパスは両方必須です。通常のテストでは外部実行を`ignored`と表示し、専用の`windows-qbe` CIジョブで必ず実行します。

| 比較対象 | 確認内容 |
| --- | --- |
| `examples/*.ceru` | 同じ完成済みIRからIR Executor・VM・QBEを実行 |
| 共通の値fixture | 既知の期待出力、文字列、u64、複合値、所有権、評価順 |
| 実行時失敗fixture | 理由、出自、先行出力、意図した終了コード |
| 文字列の小さい予算 | 所有・借用・コピー・解放・上限超過 |
| モジュールexample | 名前解決後の実行と別ファイル由来の失敗位置 |
| CLI | 明示ターゲット、誤指定時の既存出力保護 |

改行や浮動小数点表示を正規化せず、stdout/stderrの全バイトを比較します。`target/qbe-windows-observations/<case>/`に`.ceir`・`.cebc`・`.ssa`・`.s`・実行ファイル・期待出力・各プロセスの出力を保存します。CIでも同じフォルダをartifactとして残します。

[動的配列](owned-arrays.ja.md)もIR・VM・C・LLVMと実行結果・停止理由・出自・先行出力を比較します。`cargo test --test dynamic_arrays`で確保・独立コピー・範囲・所有解放とWindowsの引数境界を検証し、上記のQBE用環境変数を設定したCIでも実行します。

[行入力](stdin-connections.ja.md)もQBE 1.3の両ターゲットで実装済みです。Windowsは同期file/pipeの生バイトをReadFileで読み、consoleは消費0のunsupported-connectionで停止します。入力失敗はio-v1診断後に_Exit(1)で停止し、言語エラーのabortとは区別します。入力による暗黙flushは行いません。

CIは `cargo test --test stdin_routes` も実行し、実file/pipe、EINTR、部分応答、EOF、入力上限・予算・UTF-8・消費数・出自を比較します。生成IL・assemblyと実行記録は `target/stdin-*/` に残し、Windows QBEジョブのartifactへ含めます。
