# LLVMの子プロセス接続

[English](subprocess-llvm.en.md)

**2026-11-16：Windows/LinuxのLLVM生成・リンク・実行を実装しました。** 明示したtargetの共通runtimeを呼び、既存exampleと失敗時の継続を4実行器と比較します。[生成経路の契約](subprocess-generated.ja.md)に沿い、生成Cに続いてLLVMへ接続します。

## 確認した現状

| 対象 | 現状・今回の判断 |
| --- | --- |
| Issue・ロードマップ | LLVMまで実装済み。次はQBE/Native→WAT・配布。実行トレース（#282）、RV64実験（#284）、配布アーカイブ検証（#272）、バイナリI/O（#243）、自己起動の上限（#249）は別作業 |
| 共通runtime | prepare/execute・固定Report・C ABI・Windows/Linux接続を再利用。LLVM用の期限・回収規則は追加しない |
| [LLVM lowering](../../src/codegen/llvm/lower.rs) | HIRのProcessRunをLLVM用命令へ変換し、要求・出自を保持 |
| LLVMの型情報 | フィールドの型・名前・enum選択肢を保持し、固定Reportから名前を使って変換 |
| LLVM配列管理 | 通常確保は失敗時停止。子プロセスの両出力には、失敗を値で返して取り消せる予約処理を追加 |
| LLVM起動処理 | argv・file-root・process-configに対応。実行前にcontextを作り、実行後に破棄 |

[#268](https://github.com/Cerune-Lang/Cerune/issues/268)の重複処理の指摘を踏まえ、意味の判定は共通runtimeへ集約します。一方、Cerune値の配置からABI値へ変換する処理はLLVM生成物へ残します。全backendの構成変更や一般FFIの導入は今回の前提にしません。

## ABI試験で確定した呼出し方

[独立試験](../../experiments/subprocess/llvm_abi/README.md)では、手書きLLVMから既存Rust staticlibを呼びました。C frontendから生成した参考LLVM・ASMを別途保存し、型配置と呼出し方を照合します。参考Cの関数はLLVM試験へリンクしません。

createの設定はCでは16バイトのBytes値ですが、LLVMへCのBytes型の引数をそのまま書くだけでは両OSの呼出規約を表せません。

| 明示target | createのLLVM宣言 | 設定の渡し方 |
| --- | --- | --- |
| x86_64-pc-windows-msvc | i32 (i64, i64, ptr, ptr) | 呼出し元がBytesを一時領域へコピーし、そのポインタを渡す |
| x86_64-unknown-linux-gnu | i32 (i64, i64, ptr, i64, ptr) | データポインタと長さを別の引数へ分ける |

ここでの引数は版・target・設定・出力handleです。残る6関数は整数とポインタだけを渡します。copyは6引数を使うため、Windowsのスタック上の引数も試験対象です。LLVM内の補助関数は設定をポインタ＋長さで受け、外部呼出しの箇所だけをtarget別にします。既存C ABIを変更する必要はありません。

Bytesは16バイト、Requestは120バイトです。LLVMで計算したサイズ2件と全13フィールドのoffsetをCのsizeof/offsetofと比較しました。長さ・時間はi64のビット列として扱い、符号付き整数への範囲縮小はしません。UINT64_MAX、最上位bit付きの上限、32ビットを超える出自もReportへ保持できました。SourceIdの数値の往復は、ソース表への存在確認とは別です。

WindowsではClangが指定したMSVC targetへMSVC環境の版を追記します。試験は指定targetとの一致を検査し、同じtargetへの数値の版追記だけを認めて、その表記をLLVMへ書きます。他のOS・ABIへの切替えや警告の一括抑制はしません。

## 実装した生成処理

| 順序 | 生成する処理 | 保持する条件 |
| --- | --- | --- |
| 1 | LLVM用ProcessRun命令と型の名前情報 | 要求を一度評価。短絡・左からの評価順・NodeId/SourceId/Spanを維持 |
| 2 | 要求変換とprepare | UTF-8と生バイトを明示変換。prepareが要求を所有してから一時領域を解放 |
| 3 | 両出力の予約→execute | u8は意味上1要素8バイト。両方を起動前に予約し、片方の失敗で追加領域を取り消す。容量不足/確保失敗を共通runtimeへ渡す |
| 4 | Report・出力の変換とrelease | 固定wordをenum名から解決。保持数だけ要素を初期化し、短い結果でも予約容量は所有者の解放まで計上 |
| 5 | 起動設定とdestroy | 先頭の専用マーカー内だけprocess-config/file-rootを解釈。区切り以降のargvを保持。contextを実行後に破棄 |
| 6 | 言語example・異常系比較 | registered.ceru、所有コピー、評価順、出自、予約失敗、設定異常をHIR/MIR/SSA/VM・生成Cと照合 |

比較用の実OS接続は専用fixtureと新規出力先に限定します（#104）。利用者が登録した一般のプログラムを比較のために再実行する機能は追加しません。CI必須チェックの集約（#262）も別作業で、今回は既存のWindows/Linux子プロセスジョブへ試験を追加します。

ABI違反は操作位置と状態番号付きで停止し、回復可能な子プロセスの失敗値へ置換しません。report/copyによる読取りは時計・転送・解放を進めません。生成物には非最適化の要求変換・予約・実行・解放を残し、外部archiveの探索やリンクは暗黙に追加しません。

## 検証済みの範囲と残件

Windows/Linuxの-O0と明示的な追加検証の-O2で、7関数の呼出し、型配置、u64、要求の所有、出自、生バイト、非0終了、位置指定の反復読取り、不正入力・二重操作、予約失敗、未登録・起動失敗、出力上限、時間切れ、解放後の再利用を検査しました。実OSの時間切れ後に進行不明となった場合は、接続停止の維持を確認します。

独立ABI試験に加え、[比較試験](../../experiments/subprocess/c_abi/generated.cjs)の--llvmで既存registered.ceruを実行します。Windows/Linux各76比較で、Report全体、コピーの独立性、評価順、4097バイトの出力、空結果、上限、起動設定を4実行器と照合します。4箇所の出力確保失敗を注入し、起動しないことと最終配列使用量0を確認します。ABI状態番号13の注入では、先行出力とNodeId/Spanを保持して終了1となり、continuedへ進まないことを検査します。OSで再現できない失敗分類は共通runtime/codecの既存試験に任せ、全失敗を実OSで検証したとは扱いません。Nativeの配布・WATのhost・汎用観測APIも別段階です。


Report全体の実OS比較は、失敗なし・進行が既知・回収完了・cleanupとcleanup_reportedの一致を必須にします。独立した実行ではJob／process groupが空になったことを確認する時点が異なるため、この条件を満たす場合だけtermination_requestedを比較用の写しから除きます。commands.jsonの生出力は変更しません。出自・転送数・出力・終了コード・回収失敗などの差は引き続き拒否します。[比較規則の回帰試験](../../experiments/subprocess/c_abi/completed-report.cjs)で除外範囲を検査し、生成C/LLVMへfalseとtrueを個別に注入して、公開enumへの変換自体は両方とも正確に検査します。

## 生成・リンク・起動

同じtargetのruntime archiveを明示します。Linuxでは次の手順です。process.jsonには[CLIと同じ設定](subprocess-cli.ja.md)を使い、監督の絶対パスを指定します。

~~~sh
cargo build --example subprocess_linux_supervisor
cargo rustc --lib --features process-c-abi --target x86_64-unknown-linux-gnu --crate-type staticlib -- --print native-static-libs
cerune emit-llvm examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.ll
clang --target=x86_64-unknown-linux-gnu -O0 target/process.ll target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

WindowsではMSVC/Windows SDKのリンク用ライブラリを使います。

~~~powershell
cargo rustc --lib --features process-c-abi --target x86_64-pc-windows-msvc --crate-type staticlib -- --print native-static-libs
cerune emit-llvm examples/process_execution/registered.ceru --target x86_64-pc-windows-msvc -o target/process.ll
clang --target=x86_64-pc-windows-msvc -O0 target/process.ll target/x86_64-pc-windows-msvc/debug/cerune_lang.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -ladvapi32 -lbcrypt -o target/process.exe
target/process.exe --cerune-runtime-v1 --process-config process.json --
~~~

子プロセス操作にはtarget指定が必須です。ファイル操作を含む場合は同じマーカーの内側で--file-rootも指定できます。マーカーなしでは接続を作らずUnconfiguredを返し、argvは従来どおり扱います。欠落・重複・未知設定、空・不正・大きすぎるJSONは利用者プログラムの実行前に停止します。JSONは1 MiB以下とし、生成物へ自動埋込みしません。

要求の文字列・引数表は、同じ{ptr,i64}配置を使う読取り専用の値としてprepareへ渡します。prepareがコピーするまで有効に保ちます。stdinのu8要素はi64の格納領域から生バイトへ変換し、prepare後に一時領域を解放します。結果の生バイトは4096バイトずつ読み取り、u8の格納領域へ広げます。生成したLLVMにこの処理を残します。

[生成QBE](subprocess-qbe.ja.md)も接続済みです。次はNativeへの接続、WAT、runtime配布です。
