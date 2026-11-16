# Nativeの子プロセスABI試験

[English](README.en.md)

**型付きx86-64命令から作ったASMと自前Objectで、共通runtimeの呼出しを検証する試験です。** Windows/Linuxの両方で実行済みです。[Ceruneソースからの実行と起動設定](../../../docs/design/subprocess-native.ja.md)も実装済みです。build/releaseへの追加runtime指定は後続です。

[生成経路の契約](../../../docs/design/subprocess-generated.ja.md)に従い、言語機能を接続する前に値の渡し方とリンクを確かめます。[C ABI試験](../c_abi/README.md)と同じruntime archive・C側の期待値を使います。実行可能なCeruneの例は[process_execution](../../../examples/process_execution/README.md)に置き、この試験を言語exampleへ混ぜません。

## 実行

Rust 1.98、Node、Clang、対象OSのリンカ・開発用ライブラリが必要です。C ABI試験でarchiveを用意し、Linuxでは監督実行ファイルも用意します。targetとarchiveを明示し、出力先は未作成ディレクトリにします。

~~~text
node experiments/subprocess/native_abi/check.cjs x86_64-pc-windows-msvc clang target/subprocess-native-abi target/subprocess-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
node experiments/subprocess/native_abi/check.cjs x86_64-unknown-linux-gnu clang target/subprocess-native-abi target/subprocess-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

[emit.rs](emit.rs)が既存の固定版SASHIMI APIで両targetの型付きProgramを作り、同じProgramからASMとObjectを生成します。ASMはClangでも組み立てます。選択した実行OSと一致するtargetだけリンク・実行し、他方のtargetもASMの組立てまで確認します。実行中OSから生成targetを補うことはしません。

[driver.c](driver.c)はC ABIの既存assertionを呼び出しますが、runtimeへの7種類の呼出しは型付き命令を通します。Cコンパイラがruntime呼出しを代行する試験ではありません。C側のO0/O2それぞれで、自前ObjectとASM経由のObjectを実行するため、1 OSにつき4通りを確認します。O0/O2はC試験側の設定であり、Ceruneの最適化ではありません。

## 確認する内容

| 対象 | 試験内容 |
| --- | --- |
| create | 16バイトのBytes値。Windowsではコピーへのポインタ、Linux SysVでは2本の整数レジスタで渡す |
| prepare | 120バイトの要求を呼出し側stackへコピー。runtimeが保持した要求と元領域の寿命を検証 |
| copy | 6引数。Windowsの第5・6引数のstack受渡し、Linuxの6本の整数レジスタを確認 |
| その他の呼出し | execute/report/release/destroy、32ビット状態、64ビットhandle・長さ・出自 |
| 実行と失敗 | 生バイト、終了コード7、未知版、不正要求、未接続・未登録、予約失敗、出力上限、期限、回収、解放後の再呼出し |
| 値の精度 | u64上位ビットを含む上限と、32ビットを超えるNodeId/SourceId/SpanのReport往復 |
| 観測 | ASMの表示前後でObjectと出自記録が一致。7個の外部呼出しそれぞれの再配置と試験内IDの対応を検査 |

Clangが正規のCヘッダから生成したcreate呼出しもreference.sへ保存します。参考資料でありリンク入力にはしません。試験内IDは呼出し操作の番号で、CeruneソースのNodeIdではありません。Report内のNodeId/SourceId/Spanとは区別します。

## 保存するものと残件

commands.jsonにコマンド・引数・終了状態・出力を保存します。inputs.jsonには明示target、archiveとemit.rsとCargo.lockのSHA-256を保存します。windows/linuxディレクトリには型付き命令、ASM、Object、外部呼出しの出自対応を残します。設定JSONはローカルの絶対パスを含むためCI artifactに収録しません。試験用C/Rust/JSは既存のexperiments/subprocess属性によりGitHubの言語集計から除外されます。

ProcessRunの[LIRへの保持](../../../docs/design/lir-observation.ja.md)は実装済みです。次は機械命令へ変換し、要求変換、配列予約・Report変換・所有解放、起動設定を接続します。非最適化/SSAとASM/Objectを既存のCerune実行例で比較した後、Native build/releaseの明示リンク入力と依存記録を更新します。この試験だけでは言語配列の予約・解放や生成ソースの出自対応を検証したことにはなりません。runtimeや監督の配布も後続です。
