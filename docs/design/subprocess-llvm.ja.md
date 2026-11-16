# LLVMの子プロセス接続

[English](subprocess-llvm.en.md)

**2026-11-16：外部runtimeを呼ぶABI試験まで完了。本体のLLVM loweringは未実装です。** 現在のemit-llvmは子プロセス操作を含むIRを引き続き診断します。[生成経路の契約](subprocess-generated.ja.md)に沿い、生成Cに続いてLLVMへ接続します。

## 確認した現状

| 対象 | 現状・今回の判断 |
| --- | --- |
| Issue・ロードマップ | 本筋はLLVM→QBE/Native→WAT・配布。実行トレース（#282）、RV64実験（#284）、配布アーカイブ検証（#272）、バイナリI/O（#243）、自己起動の上限（#249）は別作業 |
| 共通runtime | prepare/execute・固定Report・C ABI・Windows/Linux接続を再利用。LLVM用の期限・回収規則は追加しない |
| [LLVM lowering](../../src/codegen/llvm/lower.rs) | HIRから独自のLLVM用命令へ変換。ProcessRunは入口で拒否され、命令表現もまだない |
| LLVMの型情報 | フィールドの型だけを保持。要求の取出しとReport変換には名前・enum選択肢も必要 |
| LLVM配列管理 | 通常確保は失敗時停止。子プロセスの両出力には、失敗を値で返して取り消せる予約処理が必要 |
| LLVM起動処理 | argv・file-rootに対応。process-configとcontextの寿命は未接続 |

[#268](https://github.com/Cerune-Lang/Cerune/issues/268)の重複処理の指摘を踏まえ、意味の判定は共通runtimeへ集約します。一方、Cerune値の配置からABI値へ変換する処理はLLVM生成物へ残します。全backendの構成変更や一般FFIの導入は今回の前提にしません。

## ABI試験で確定した呼出し方

[独立試験](../../experiments/subprocess/llvm_abi/README.md)では、手書きLLVMから既存Rust staticlibを呼びました。C frontendから生成した参考LLVM・ASMを別途保存し、型配置と呼出し方を照合します。参考Cの関数はLLVM試験へリンクしません。

createの設定はCでは16バイトのBytes値ですが、LLVMへCの構造体引数をそのまま書くだけでは両OSの呼出規約を表せません。

| 明示target | createのLLVM宣言 | 設定の渡し方 |
| --- | --- | --- |
| x86_64-pc-windows-msvc | i32 (i64, i64, ptr, ptr) | 呼出し元がBytesを一時領域へコピーし、そのポインタを渡す |
| x86_64-unknown-linux-gnu | i32 (i64, i64, ptr, i64, ptr) | データポインタと長さを別の引数へ分ける |

ここでの引数は版・target・設定・出力handleです。残る6関数は整数とポインタだけを渡します。copyは6引数を使うため、Windowsのスタック上の引数も試験対象です。LLVM内の補助関数は設定をポインタ＋長さで受け、外部呼出しの箇所だけをtarget別にします。既存C ABIを変更する必要はありません。

Bytesは16バイト、Requestは120バイトです。LLVMで計算したサイズ2件と全13フィールドのoffsetをCのsizeof/offsetofと比較しました。長さ・時間はi64のビット列として扱い、符号付き整数への範囲縮小はしません。UINT64_MAX、最上位bit付きの上限、32ビットを超える出自もReportへ保持できました。SourceIdの数値の往復は、ソース表への存在確認とは別です。

WindowsではClangが指定したMSVC targetへMSVC環境の版を追記します。試験は指定targetとの一致を検査し、同じtargetへの数値の版追記だけを認めて、その表記をLLVMへ書きます。他のOS・ABIへの切替えや警告の一括抑制はしません。

## 本体へ接続する手順（未実装）

| 順序 | 追加する処理 | 保持する条件 |
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

この試験は**言語exampleをLLVM経路で実行したものではありません**。失敗可能な言語配列予約、公開Outcomeへの変換、起動引数、通常のLLVM emitterへの組込みは次の変更です。OSで再現できない失敗分類は共通runtime/codecの既存試験に任せ、全失敗を実OSで検証したとは扱いません。Nativeの配布・WATのhost・汎用観測APIも別段階です。
