# 生成QBEの子プロセス接続

[English](subprocess-qbe.en.md)

**2026-11-16：Windows x86-64 MSVC・Linux x86-64 GNUで実装・検証済みです。** [ABI試験](../../experiments/subprocess/qbe_abi/README.md)で確認した7関数を使い、共通IRのprocess.runを既存Rust runtimeへ接続します。起動・転送・期限・回収の判定は[共通契約](subprocess-generated.ja.md)に従い、QBE専用の意味処理を作りません。

## 生成物で追える処理

| 処理 | QBEでの実装 |
| --- | --- |
| 要求の評価 | 既存のloweringで一度評価。標準run関数への通常の所有コピーと呼出し深さを維持 |
| 要求の変換 | フィールド名と型から格納位置を決定。文字列の長さと本文、引数表、生バイトstdin、上限と出自を120バイトのABI要求へ保存 |
| prepare | 共通runtimeが要求を所有した後、引数表とstdinの一時コピーを解放。予約待ちと結果確定済みを区別 |
| 両出力の予約 | 起動前にstdout/stderrを上限分予約。容量不足はOutputBudget、確保失敗はAllocationFailedを返し、片側の追加確保も取り消す |
| execute | 予約結果を渡し、成功時だけ共通runtimeが時計取得・起動・転送・回収を行う |
| 結果の復元 | 固定32 wordの版・サイズ・予約wordを検査。enumは名前からタグを選び、出力は4096バイトずつ読み取る |
| 解放 | ABI結果を明示解放。返却配列は通常の所有処理へ戻し、停止した接続を勝手に再開しない |

QBEの文字列は長さと本文を指すポインタ、動的配列は所有者を指すポインタです。引数の文字列参照をABIのdata/len対へ変換し、stdinの8バイト格納要素を生バイトへ詰めます。prepareがコピーするまで元の言語値を保持します。出力の生バイトはu8の8バイト格納領域へ広げます。

配列所有者はdata／length／references／使用バイト数／初期化済み要素数／strideの48バイトです。短い結果や空結果でも、予約容量に対する使用バイト数は所有者の解放まで保持します。結果変換の途中で新しいヒープ領域を確保しません。通常の配列確保失敗による言語停止規則は変更しません。

ProcessRunはQBE用命令にも残し、NodeId・SourceId・SpanをABI要求と失敗診断へ渡します。非最適化の共通IRは従来どおり観測できます。QBEへ渡すILには各段階を明示しますが、QBE自身の最適化を無効にできるとは主張しません。

## 生成・リンク・起動

targetと同じruntime archiveを明示します。Linuxの例です。process.jsonは[CLIの設定](subprocess-cli.ja.md)で、監督の絶対パスと実行対象を登録します。

~~~sh
cargo build --example subprocess_linux_supervisor
cargo rustc --lib --features process-c-abi --target x86_64-unknown-linux-gnu --crate-type staticlib -- --print native-static-libs
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.ssa
qbe -t amd64_sysv -o target/process.s target/process.ssa
clang --target=x86_64-unknown-linux-gnu target/process.s target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

WindowsではMSVC/Windows SDKのリンク用ライブラリを使います。

~~~powershell
cargo rustc --lib --features process-c-abi --target x86_64-pc-windows-msvc --crate-type staticlib -- --print native-static-libs
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-pc-windows-msvc -o target/process.ssa
qbe -t amd64_win -o target/process.s target/process.ssa
clang --target=x86_64-pc-windows-msvc target/process.s target/x86_64-pc-windows-msvc/debug/cerune_lang.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -ladvapi32 -lbcrypt -o target/process.exe
target/process.exe --cerune-runtime-v1 --process-config process.json --
~~~

ターゲットは実行中のOSから選びません。子プロセス操作を含むIRには明示targetと追加リンクが必要です。既存の型だけを使う生成物へ子プロセスruntimeへの依存は追加しません。emit-qbeはemit-onlyのままで、リンクやarchive探索を暗黙に実行しません。

先頭の専用マーカー内だけprocess-configを解析します。ファイル操作も含む場合はfile-rootを併記できます。区切り以降のargvは維持し、マーカーなしでは未接続としてUnconfiguredを返します。欠落・重複・未知設定、不正・空・1 MiB超のJSONは利用者プログラムより前に停止します。設定JSON・登録パス・環境値を生成物へ自動埋込みしません。

壊れたABI状態はcerune-process-abi-v1と状態番号・操作のソース位置を出し、先行出力をflushして終了1で停止します。起動失敗等の回復可能な子プロセス結果と混同しません。

## 検証と残る作業

[共通比較試験](../../experiments/subprocess/c_abi/generated.cjs)に--qbeを指定し、CERUNE_TEST_QBEで実行ファイルを明示します。同じ.ceruと登録設定を使い、Windows/Linux各76比較でHIR/MIR/SSA/VMと照合しました。生成C/LLVMも同じ比較を実行しています。

- 既存registered.ceru、未接続、未登録、起動失敗、容量・時間上限、コピー、短絡・評価順、出自、Report全体。
- 4097バイトのchunk転送、空結果、予約容量の維持と最後の配列使用量0。
- 4箇所の出力確保失敗と、ABI状態13による先行出力・ソース位置付き停止。
- cleanupのfalse/trueを個別に注入した公開enum変換、起動設定とfile-root／argvの共存。

実OSの時間切れは部分転送や終了時点が変わるため、固定できる結果を比較します。正常回収のtermination_requestedも、失敗なし・進行既知・回収完了・生報告一致の場合だけ比較用の写しから除きます。生出力はcommands.jsonへ残し、他の差は拒否します。比較規則と真偽値の変換は別々に検査します。

CIでもQBE 1.3を固定して実行します。既存のQBE Windows回避処理は維持し、今回の接続のためにASMを書き換えません。NativeのLIR/ASM/Object・build/release、WAT host、runtimeとLinux監督の配布は後続です。