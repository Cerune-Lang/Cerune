# 子プロセスLLVM ABI試験

[English](README.en.md)

手書きLLVMから共通Rust runtimeの7関数を呼び、[既存C試験](../c_abi/probe.c)の期待値をそのまま検査します。言語exampleとは別の接続試験です。[設計と未対応部分](../../../docs/design/subprocess-llvm.ja.md)を参照してください。

## 実行

Rust 1.98、Node、opaque ptrに対応するClang、対象OSのリンカ・開発ライブラリを使います。先に[C ABI試験](../c_abi/README.md)でruntime archiveを構築してください。target・Clang・archive・Linux監督を明示し、出力先は新規ディレクトリを指定します。

~~~powershell
node experiments/subprocess/llvm_abi/check.cjs x86_64-pc-windows-msvc clang target/process-llvm-abi target/process-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
~~~

~~~sh
node experiments/subprocess/llvm_abi/check.cjs x86_64-unknown-linux-gnu clang-18 target/process-llvm-abi target/process-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

指定targetがhostと異なる場合は実行しません。実行コマンドは1件120秒、記録出力は8 MiBを上限とし、失敗・時間超過・出力超過は試験失敗です。設定JSONは明示したfixtureだけを登録し、子に環境を継承しません。

## 試験と保存物

| ファイル | 役割 |
| --- | --- |
| common.ll、windows.ll / linux.ll | 手書きの型配置・要求コピー・外部呼出し。createだけOS別 |
| driver.c | C試験を再利用し、runtimeへの全呼出しをLLVM関数へ接続。サイズ・offset・u64の独立期待値も検査 |
| reference.c | 正規Cヘッダからcreateの参考LLVM/ASMを生成。試験実行ファイルへはリンクしない |
| 出力先のprobe.ll、probe-O0.s / probe-O2.s | 実際に実行したLLVMと、明示した最適化設定ごとのASM |
| reference.ll / reference.s | C frontendが選んだ呼出規約。create宣言とtargetを自動照合 |
| commands.json | Clang版、引数、リンク入力、終了状態、stdout/stderr |
| config-O0.json / config-O2.json | ローカルの絶対パスを含む試験設定。CI artifactに含めない |

CIはcommands.jsonとLLVM/ASMだけを保存します。参考LLVMにはcheckout内のソースパスが入りますが、登録設定・環境・入力本文を成果物へ自動収録しません。構築済みのarchiveを再利用し、runtimeのビルド条件はC ABI試験側の記録で追います。

2026-11-16にWindows/Linuxの-O0/-O2で成功しました。15個のサイズ/offset、64ビット値、要求の独立コピー、出自、NUL/CR/LF/非UTF-8出力、非0終了、予約失敗、未登録・起動失敗、上限・時間切れ、状態遷移を検証しています。生の終了結果は記録し、時間切れ後の不確かな進行を成功に置換しません。

本体のLLVM emitterへの接続も実装しました。[生成物の比較試験](../c_abi/generated.cjs)に--llvmを指定すると、言語配列の予約・Outcomeへの変換・起動設定・既存.ceruの実行を検証できます。この独立ABI試験は外部関数の呼出し方の検証として維持します。
