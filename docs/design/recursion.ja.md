# 再帰と呼出し領域

[English](recursion.en.md)

状態：WATの呼出し領域分離、HIR・MIR・SSA・VMの明示上限API・深さ超過診断、4実行器の明示frame、C・LLVM・QBE・WAT・Nativeの明示上限API・深さ診断を実装。CLI共通設定（既定128・範囲1〜128）とNative build/releaseへの保存も実装済みです。ソースの直接・相互再帰とgeneric具体化の再利用も実装済みです。

## 実装済み：ソース再帰とgenericの再利用

直接再帰・相互再帰・generic関数の再帰を受け付けます。既存の関数・call・returnをそのまま使い、再帰専用IR、暗黙のinline化、末尾呼出し最適化は追加しません。再帰的な値型と循環importの禁止は別の規則として維持します。

| 処理 | 契約 |
| --- | --- |
| 名前・型の検査 | 全関数の宣言を登録して本文を検査。再帰呼出しも引数・戻り値を通常どおり検査 |
| genericの登録 | 型・配列長を解決した組合せを、本文の処理前に登録。同じ組合せなら処理中でも既存の関数を参照 |
| generic本文の処理 | 処理待ち一覧から登録順に具体化。コンパイラの再帰callへ展開の進行を積まない |
| 展開の上限 | 1コンパイル256具体化・新しい具体化の経路深さ64。型を変え続ける展開は診断して停止。実行時の呼出し深さとは別 |
| 出自 | 具体化した引数・定義Span・再帰を含む各呼出しSpanを保存。展開失敗には具体化の経路を添付。本文のNodeIdは具体化ごとに割当て |
| 実行 | 引数を左から評価後、ソース関数の深さを検査。親の途中値・所有値・戻り先を保持し、return後は深さを戻す |

既定値から呼ぶ関数、配列操作・変換・構造体更新式の内側からの呼出しも同じ規則です。具体化の登録順は観測可能ですが、識別子は一回のコンパイルの対応を表し、以前の版との番号の固定を保証しません。

[実行例](../../examples/recursion/README.md)は階乗・相互再帰・所有値を渡すgeneric再帰です。4実行器と生成C・LLVM・QBE・WAT・ASM・Objectで、上限前後、先行出力、call-v1の全履歴・停止位置、128段、反復後の所有領域解放を比較します。Linux／Windowsを対象にします。

Rustの上限未指定APIは引き続き上限を選びません。無限再帰を実行するときは必ず明示上限APIを使ってください。CLIは既定128・公開範囲1〜128です。物理stack・memory不足の安全保証とは区別します。

## 調査結果と責務

既存のCerune IRの関数・call・returnを使います。再帰専用のIRや別の意味論は追加しません。

| 経路／段階 | 現在の状態 | 確認事項 |
| --- | --- | --- |
| frontend | 直接・間接再帰を受付 | 再帰的な値型の禁止とは区別 |
| generic展開 | 登録した具体化を再利用。本文は処理待ち一覧から展開 | 256具体化・展開深さ64を維持 |
| HIR | 元HIRへの参照と明示frameで実行。ソース関数・生成補助関数の呼出しをホスト再帰へ積まない | 再帰ソースの全経路比較 |
| VM | 元bytecodeの命令位置・operand stack・slotを明示frameで保持 | 追加の資源検証と再帰ソース比較 |
| MIR / SSA | 元CFGのblock・命令位置・局所値／SSA値／slotを明示frameで保持 | 再帰ソースで共通設定を維持 |
| C | Rust APIで明示上限とcall-v1を生成。localは実行環境のstack上に配置 | 追加の物理資源検証と再帰ソース比較 |
| LLVM / QBE / Native | 明示上限とcall-v1を生成。localは実行環境のstack上に配置 | 追加の物理資源検証と再帰ソース比較 |
| WAT | 集約値・一時領域と呼出し履歴をprivate memoryへ保持。明示上限とcall-v1に対応 | 追加の物理資源検証と再帰ソース比較 |

## 合意した停止条件

共通の「同時に有効な関数呼出し数」に上限を設けます。引数を左から評価し終えてから、関数へ入る直前に検査します。超過した呼出しの本体は実行せず、呼出し元のNodeId/Spanとcall stackを記録して停止します。

| 区分 | 意味 |
| --- | --- |
| 共通の深さ上限 | 同じソース・設定・入力に対する意味上の停止条件。観測の有無で変えない |
| 物理的なstack／memory不足 | 実行環境の資源不足。深さ超過として報告しない |
| 観測 | call stackとソース位置の読取り。実行中のframeを書き換えるAPIにしない |

CLIの既定値128・公開範囲1〜128を採用します。物理資源の安全範囲ではありません。ソースで宣言された関数（標準モジュール、generic具体化、明示mainを含む）を、実際の呼出しごとに1段として数えます。共通IR・backendの生成補助関数と暗黙のトップレベル入口は数えません。backendの実装都合で深さが変わらないようにします。末尾呼出しの暗黙最適化は導入しません。

## 実装済み：CLIの共通設定

通常実行・生成経路・Native buildは `--call-depth-limit N` を使います。既定128、指定範囲1〜128です。上限ちょうどの呼出しは実行し、次のソース関数へ入る直前に停止します。引数は左から先に評価します。

| 対象 | 設定の扱い |
| --- | --- |
| run / run-ir / run-mir（SSAを含む）/ run-vm | 同じ上限を入出力・プログラム引数と一緒に実行器へ渡す。runtime-v1指定時の深さ超過はcall-v1の全履歴を出力 |
| emit-c / emit-llvm / emit-qbe / emit-wat | 同じ上限の検査と値を生成物に残す。LLVM・QBEはスカラーのみでも明示targetが必須 |
| emit-lir / emit-asm / emit-obj / build native | 同じ上限付きLIRから生成。LIRに値、ASM・Objectに検査と診断用の上限値を保持。buildのmanifestにもcall_depth_limitを保存 |
| observe | 同じ設定でbaseline／SSA由来のLIR・ASMを保存。manifestへ値を記録し、観測による実行や設定変更は行わない |
| release native | buildに記録された設定を変更せず保持。以前のbuildで記録がない場合は追加せず、release側の値はnull（不明）とする |
| check / fmt / emit-sources / emit-ir / emit-mir / emit-bytecode | この実行設定の指定を受け付けない。ソース・共通IR・bytecodeの表現自体には設定を埋め込まない |

**1〜128は意味上の公開設定範囲であり、物理stack・memoryの安全保証ではありません。** 大きな局所値・集約値、生成補助関数も物理資源を消費します。検出できる確保失敗はallocation-failed、OS／エンジン側のstack不足はその失敗として残し、depth-limitへ変換しません。すべての物理的不足を共通診断で捕捉する保証はありません。上限の拡張は各経路の検証結果を基に別変更で判断します。

既存のRust APIの既定動作は維持します。入出力付き実行には `run_with_io_arguments_and_call_limit`、生成には `emit_*_with_runtime_limits`／`lower_mir_with_runtime_limits` を追加し、引数上限と呼出し深さを同時に渡せます。Rust APIのNonZeroU32範囲はCLIの公開範囲と区別し、大きな値の安全性は保証しません。

破壊的変更はCLI生成物の既定深さ検査、LLVM／QBEのtarget必須化、WATの既存診断import `cerune.write_error_byte` の要求です。対応しないWAT hostはinstantiate時に失敗します。内部memoryの非公開と版付きI/O契約は維持します。ソースの再帰も同じ深さ検査で実行します。

## 実装済み：4実行器の明示上限API

HIR・MIR・SSA・VMにRust API `run_with_call_limit(program, NonZeroU32)`を追加しました。上限ちょうどの呼出しは許可し、次の呼出しを本体の実行前に拒否します。return後は深さを戻します。bytecodeにも生成補助関数の区分を保持し、関数名で推測しません。

| 項目 | 現在の契約 |
| --- | --- |
| 設定 | 呼出し側が正の上限を明示。既存Rust APIの既定動作は維持。CLIは前述の共通設定を使う |
| 取得 | エラーの `call_failure()` から、上限、拒否した呼出し、外側から順に並ぶ有効なソース関数の履歴を読める |
| 出自 | 各callのFunctionIdと呼出し元NodeId/Span。明示mainの起動元は `origin=host` |
| 記録 | `record()` の先頭は `call-v1 reason=depth-limit limit=N function=F`。後続行は `call-v1 frame=I function=F`。各行に出自を付ける |
| 内容の保護 | 位置は `node=N [file=S] bytes=A..B`。匿名SourceIdはfileを省略。パス・本文・引数・実行値を含めない |
| 観測と制御 | 上限は実行前に設定。失敗記録は読取り専用のsnapshotであり、記録の読取り・描画は実行を再開しない |

この段階の上限は物理stack・memory消費を保証しません。生成補助関数も資源を消費します。既存のheap確保失敗は別のruntime診断のままですが、Rust実行器のOSスタック枯渇・汎用メモリ不足を共通診断として捕捉する処理は未実装です。大きな上限を安全な設定とみなしてはいけません。公開範囲と物理資源条件を区別し、再帰する有限例の検証も継続します。

深さ超過以外のすべての失敗にcall stackを添付する機能は後続です。MIR/SSAの内部再入は明示frameで対応済みです。ソースの再帰も同じ深さ検査で実行します。

`cargo test --test ir_executor call_depth` は4実行器の上限前後、引数の左からの評価、短絡、main、generic、標準モジュール、生成補助関数の除外、通常／早期return、出自、先行出力、履歴順を比較します。heap上限の失敗が深さ超過へ変わらないことも検証します。[depthの実行例](../../examples/call_frames/depth.ceru)を含みます。

## 物理スタックの検証と次の作業

[呼出し深さの実験](../../experiments/call_depth/README.md)でWindows/Linuxのdebug・releaseを比較しました。変更前のcommit 143aae0では、要求2 MiB・深さ32の構造体受渡しでdebugの4実行器すべてが深さ診断より先にstack枯渇で停止しました。これは有限例の実測であり、安全な設定範囲の証明ではありません。

HIR・MIR・SSA・VMの明示frame化を実装しました。Cの明示上限と資源条件の有限例を先行検証しました。生成経路にも同じ停止条件を実装しました。CLIの共通設定も実装しました。ソース再帰も実装しました。

## 実装済み：HIRの明示frame

HIRの関数呼出し・文・式の進行を、Rustの再帰呼出しからループによる処理へ移しました。各frameは元HIRへの参照、次に処理する位置、BindingIdごとの束縛、評価途中の値、戻り先を保持します。実行用IR・bytecodeを生成せず、元のIRと出自は変えません。

| 項目 | 動作 |
| --- | --- |
| 呼出し | 引数を左から評価し終え、論理的な深さを検査してからcalleeのframeを追加 |
| 生成補助関数 | 同じframe管理で実行するが、ソース関数の深さには加算しない |
| return | 戻り値を検査してcallerの途中値へ渡し、calleeの継続位置と束縛を外す。早期returnでも同じ |
| 制御 | if・短絡は選ばれた側だけ進む。continueはforの更新部を経由。break・returnは不要な継続を外す |
| 添字代入 | 各添字の評価と範囲検査を済ませてから、次の添字や右辺を評価 |
| 確保失敗 | frame・継続・途中値・束縛のコンテナ拡張をtry_reserveで検査。返された失敗はruntime-v1 allocation-failedとして扱い、call-v1の深さ超過とは区別 |
| 失敗後 | 後続のソース処理を実行せず、出自と先行出力を保持する。内部の深さ計数を戻すことを、ソースの巻戻し・再実行とは扱わない |

共通の呼出し履歴と深さ超過snapshotのベクタ確保も失敗を返す形にし、4実行器でallocation-failedへ対応付けます。通常の値コピー、診断用Box/String等を含むすべてのRust allocationを回復可能にしたわけではありません。入れ子値の処理、OSからの強制終了、一般的なメモリ枯渇をすべて捕捉する保証はありません。

Windows/Linuxのdebugで、整数・所有文字列と配列を含む構造体の深さ128を要求2 MiBのthread上で検証します。上限8・127は元callの出自付きで停止し、上限128と上限未指定では完走します。これは関数呼出しの構造改善を確認する有限例であり、全経路の既定値・安全範囲を決めるものではありません。

[途中値の実行例](../../examples/call_frames/pending_values.ceru)と、4経路の評価順・ループ・添字検査比較、通常関数／生成補助関数の確保失敗注入を追加しました。テスト用IRに限って同じ関数への再入も作り、所有値を保持した親frameと100回実行後の領域解放を検証します。MIR・SSA・VMにも同じ検証を適用しています。ソースの再帰も同じ深さ検査で実行します。CLI設定は前述のとおりです。生成経路の明示上限は後述します。

## 実装済み：MIR・SSA・VMの明示frame

MIR・SSA・VMも関数呼出しをホストの再帰へ積まず、呼出し先のframeを追加し、return時に保存した位置へ戻ります。元のMIR・SSA・bytecodeを直接実行し、別の実行器への委譲や新たな中間表現は加えません。

| 経路 | 呼出しごとに保持する状態 | 戻るときの処理 |
| --- | --- | --- |
| MIR | 関数、block、次の命令、局所値、戻り値の格納先 | 呼出し元の局所値へ格納して次の命令へ |
| SSA | 関数、block、次の命令、SSA値、残存slot、戻り値の格納先 | 元のSSA値・slotを保持して再開。辺の引数は全入力を読んでから同時に受け渡す |
| VM | 関数、PC、operand stack、slot、初期化位置 | 呼出し元のstackへ戻り値を積み、callの次へ |

生成補助関数も物理frameを持ちますが、論理的な深さには含めません。frame・局所領域・引数・SSA辺引数のコンテナ確保とVM stack拡張にはtry_reserveを使い、返された確保失敗はallocation-failedにします。呼出し先の通常の失敗はその命令位置、calleeのframe確保失敗は呼出し元のcall位置を保持します。失敗後は後続処理を実行せず、残る深さ計数を外します。

4実行器の深さ128／要求2 MiB、上限8・127・128、評価途中の値、loop・早期return、所有値を持つテスト用の再入、通常／生成関数の確保失敗を比較します。MIR・SSAだけにあった内部の再入拒否は取り除きましたが、ソースの再帰も同じ深さ検査で実行します。上限なしのAPIは不正なIRの無限実行を防ぐsandboxではありません。一般的なメモリ不足をすべて捕捉する保証や、全経路の安全範囲はまだありません。

## 実装済み：生成Cの明示上限

Rust APIの `codegen::c::emit_c_with_call_limit(&program, NonZeroU32)` が、4実行器と同じ数え方・call-v1記録を持つCを生成します。通常のRust API emit_cの既定動作は維持します。CLIは前述の共通上限を使います。

| 項目 | 動作 |
| --- | --- |
| 引数・呼出し | ソース関数の全引数を左から一時値へ評価し終えてから、callerでenterを検査して実際のC関数へ入る |
| 戻り値 | callerの一時値に保存してからleaveする。void、早期return、集約戻り値でもcalleeの復帰後に一度だけ深さを戻す |
| 分類 | C用IRへ元関数のsource/generated区分を保持。生成補助関数は数えず、明示mainはhost出自で数える |
| 履歴 | FunctionIdと不変の位置文字列だけを保持。履歴の確保量は必要に応じて拡張し、上限分を先に一括確保しない。正常終了時に解放 |
| 履歴の確保失敗 | runtime-v1 allocation-failedとしてcall位置で停止。文字列／配列の言語上の生存量とは別の処理系領域 |
| 深さ超過 | stderrへ拒否したcall、外側から順の有効なcallを出し、終了コード1で停止。先行出力を保持し、callee本体・後続処理を実行しない |
| 観測 | 生成Cに上限、一時値、enter／leave、履歴確保、call-v1の描画を残す。履歴や実行制御の外部APIは追加しない |

履歴の生成と実行制御は内部の処理です。診断にはパス・本文・引数・実行値を含めません。Cのstderrは対象のC runtimeに従うため、記録の照合ではCRLF/LFを正規化し、生のstderrも実験に保存します。文字列本文やstdoutのバイトを正規化するものではありません。

[生成Cの実験](../../experiments/call_depth/README.md)はターゲット・コンパイラ・stack量を明示します。整数と所有文字列・配列入り構造体の深さ32/128、上限N−1/N、O0/O2を検証し、プロセスの異常終了を深さ超過の成功に数えません。テストでは4実行器との診断全行、短絡・引数失敗の優先順、generic・標準モジュール、生成補助関数、所有値の100回実行とテスト用IRの直接／相互再入も比較します。

このAPIは物理stackを制限しません。引数のコピー、callerの一時値、Cコンパイラの配置・最適化、生成補助関数も資源を消費します。OSによるstack枯渇をcall-v1へ変換せず、物理的な安全範囲は保証しません。意味上の公開範囲・既定値は前述のとおりです。LLVM・QBE・WAT・Nativeも、意味上の深さ検査と物理資源の失敗を分けて検証します。

## 実装済み：生成経路で同じ停止条件

深さを明示するRust APIは次のとおりです。通常のRust emit APIは既定動作を維持し、CLI・build/releaseは前述の共通設定を使います。Nativeは同じ上限付きLIRからASMと自前Objectを生成します。

| 経路 | API | 履歴の保持 |
| --- | --- | --- |
| LLVM | `emit_llvm_with_call_limit(program, target, limit)` | 不変の呼出し位置を指す配列を必要時に拡張 |
| QBE | `emit_qbe_with_call_limit(program, target, limit)` | 同上。`amd64_sysv` / `amd64_win`を明示 |
| WAT | `emit_wat_with_call_limit(program, limit)` | 既存のprivate frame確保を使う双方向リスト。復帰時に返却して再利用 |
| Native | `lower_mir_with_call_limit(program, target, limit)` → `emit_asm_from_lir` / `emit_object_from_lir` | 不変の位置を指す配列。型付き機械命令で検査・履歴を生成 |

全経路で、引数の評価後・calleeへ入る前に上限を検査し、復帰後に深さを戻します。生成補助関数は数えません。戻り値や所有状態を壊さないよう、LLVM/QBEは既存の一時値、WATはoperand stack、Nativeは戻り値レジスタを保持します。深さ超過では同じcall-v1全行と先行出力を比較します。LLVM・QBE・Nativeは終了コード1、WATは診断後のunreachableで停止します。

履歴は言語の文字列・配列の生存量とは別の処理系領域です。確保失敗は位置付きruntime-v1 allocation-failedで停止し、call-v1には変えません。WATはmemoryを公開せず、上限付き生成物だけが必要に応じて既存のwrite_error_byte importを要求します。失敗したinstanceの再開は保証しません。一般のOS stack枯渇・Wasmtime/ブラウザ等の物理call stack不足まで、この変更が共通診断へ変換するものではありません。

`cargo test --test llvm_output --test qbe_output --test wat_output --test native_output call_depth`で4実行器と照合します。有限の32段、少数の関数を使う内部IRの128段再入、所有値、浮動小数点とstack引数、main/generic/標準モジュール、短絡、引数失敗、確保失敗を対象にします。NativeはASMと自前Objectを実行し、上限と関数分類はLIR表示にも残します。ソース再帰の例でも、frontendを通した後の同じ検査を比較します。

[実験用の生成コマンド](../../experiments/call_depth/README.md)から各表現を保存できます。CLI設定は実装済みです。ソース再帰にも同じ検証を適用しています。物理資源条件の追加検証は継続します。


## 実装済み：WATの呼出し領域

| 項目 | 仕組み |
| --- | --- |
| 配置 | 静的文字列と出力整形領域を共有の静的領域に残す。集約引数・local・temporaryはframe内offsetで表現 |
| 呼出し | 引数の評価後にcallee用の領域を確保し、非公開の末尾引数で基点を渡す。公開ライブラリ関数の引数型は変更しない |
| return | 集約戻り値をcallerの戻り先へコピーし、所有の処理を完了してからframeを返却。早期returnも同じ |
| 再利用 | 16バイトのheaderと8バイト整列のpayload。専用の空き領域リストから十分な大きさのblockを再利用し、足りなければmemoryを拡張 |
| 生存量 | 文字列・動的配列とは確保cursorを共有するが、空きリストと寿命は分離。frameは言語の文字列／配列生存量へ加算しない |
| 確保失敗 | 通常callでは呼出し元のNodeId/Span付きruntime-v1 `allocation-failed`を出して停止。hostからの直接entryは`origin=host` |
| 観測 | 生成WATに関数番号・frameサイズ・基点・offset・確保／返却を残す。memoryをexportしない |

トップレベル文の領域は実行前に確保し、失敗位置には最初の文の出自を保持します。物理memoryは縮小せず、blockの分割・併合は行いません。trap後の巻戻し・再開は保証せず、失敗した実行のinstanceを再利用する契約は追加しません。

新たに確保失敗が起こり得る集約値のプログラムは、診断用の既存import `cerune.write_error_byte`を要求します。対応しないhostではinstantiateが失敗します。既存の版付き出力・flush契約と公開関数の型は維持します。

## 検証と次の順序

`cargo test --test wat_output`で、テスト用IRだけに再帰callを作り、親の配列・戻り先の独立性を実行検証します。1ページ制限下の1万回呼出し、通常／早期return後の再利用、物理確保失敗時の位置・先行出力、22バイトの文字列生存量での所有値受渡しも確認します。ソースの再帰も同じ深さ検査で実行します。

[call_framesの実行例](../../examples/call_frames/README.md)は現在の構文だけで入れ子呼出し・値更新・早期returnを試せます。HIR・MIR・SSA・VMとWATの出力を比較します。

1. WAT領域の独立性を整える（実装済み）。
2. 共通上限を全経路へ揃える。4実行器の明示APIと数え方は実装済み。物理stackの実測を受け、4実行器は明示frame管理へ移行済み。生成経路の明示上限・深さ診断とCLI共通設定も実装済み。
3. generic具体化の再利用とfrontendの禁止解除（実装済み）。
4. 直接／相互再帰、所有・解放、評価順、上限前後と停止位置を全経路で比較し、再帰exampleを追加（実装済み）。

1〜4を実装しました。次は限定的なファイル入出力、その後に子プロセスの設計へ進みます。
