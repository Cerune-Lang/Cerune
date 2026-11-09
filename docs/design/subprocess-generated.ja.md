# 生成経路の子プロセス接続契約

[English](subprocess-generated.en.md)

**状態：2026-11-09の設計案。以下のABI、runtimeライブラリ、段階APIは未実装です。** 現在は[4実行器](subprocess-execution.ja.md)と[Windows/LinuxのCLI接続](subprocess-cli.ja.md)まで実装済みです。C・LLVM・QBE・WAT・Nativeは実行操作を含むIRを拒否します。以下の名前・数値配置は初版実装で検証し、公開前に固定します。

## 調査結果と方針

| 既存実装で確認したこと | 生成経路で必要な対応 |
| --- | --- |
| [共通runtime](../../src/runtime/process.rs)が要求検査・転送・期限・回収・接続停止を判定 | このRust実装を再利用し、backendごとに意味処理を移植しない |
| `run_with_reservation`は要求検査、転送buffer確保、言語の結果領域予約、時計取得・起動の順 | 予約と起動を分ける共通の段階APIへ整理し、既存APIも同じ処理を包む |
| [返却配列](../../src/runtime/array_heap/byte_outputs.rs)のu8は意味上1要素8バイトを使用 | ABIの生バイトbufferと区別し、生成側の配列へ明示的に変換する |
| [生成Cの配列確保](../../src/codegen/c/array.rs)は失敗すると言語を停止 | 子プロセス結果の予約には失敗を返せる処理を追加。通常配列確保の停止規則は変えない |
| [Rustの失敗enum](../../src/runtime/process/records.rs)と[標準ソース](../../src/standard/process.ceru)では`ProgressUnknown`の宣言位置が違う | Rustの判別値・ソースのタグ番号をABIへ流用せず、名前と明示した番号で変換する |
| [WATファイルhost](wat-file-host.ja.md)は内部memoryを公開しない | 子プロセスも版付きの値・chunk転送を使い、memory公開やhostからの配列書換えを要求しない |
| [Nativeリンク](../../src/native_build/link.rs)とruntime記録は現状、自前Object＋システムruntimeを前提とする | 追加ライブラリを明示したリンク入力・互換性・配布記録が必要 |

C・LLVM・QBE・Nativeは、共通runtimeを包む**Cerune提供の静的ライブラリとC ABI**を使う方針です。WATの参照hostも同じruntimeの段階APIを使います。IR ExecutorやVMを生成物の内部で動かす設計にはしません。

| 担当 | 責務 |
| --- | --- |
| 共通IR・各lowering | 評価順・短絡・型・所有・出自、外部操作の位置を維持 |
| 生成コード | 要求値の変換、固定結果の準備、言語配列の予約・確定・解放、Reportから公開enumへの変換 |
| 共通runtime | 要求と接続の検査順、転送、期限、失敗選択、後始末、停止した接続の維持 |
| Windows/Linux Backend | 既存の明示登録とOS資源管理。Linux監督はOSの親子関係・回収だけを担当 |
| ABIアダプタ | 幅・版・状態・所有を検査して共通runtimeへ渡す。別の時間切れや再試行規則を持たない |

既存ファイルI/Oや出力runtime全体の分離は今回の前提にしません。子プロセスを使わない生成物へ新しいリンク依存を追加しません。実行モジュールの未使用関数も保持する現行IRでは、importだけでも操作を含みます。この判定を変える暗黙の未使用削除は行いません。

## 検査・予約・起動の順序

生成コードで次の処理を追えるようにします。各名前は概念上の操作で、現行CLIのコマンドではありません。

```text
要求を一度評価・コピー
固定Reportと失敗時の空配列を準備
prepare(要求, 転送領域上限, 出自)
  ├─ 結果確定済み → Reportと空配列を受け取る
  └─ 予約待ち     → 言語のstdout/stderr配列を予約
                    execute(予約成功 / 容量不足 / 確保失敗)
Reportと保持済み生バイトを、予約済みの言語値へ変換
結果領域を解放し、呼出し元の処理を続ける
```

1. 要求コピー・固定結果の構築失敗は、既存の言語側の資源不足として停止します。子は起動しません。
2. `prepare`は呼出し番号を一度だけ進め、現在の検査順を維持します。接続停止 → 不正要求 → 要求容量 → 未接続 → 転送領域上限・確保の順です。登録名の検索は現状どおり起動段階に残します。
3. `prepare`には子の起動・OS転送・操作期限の時計取得を入れません。要求・接続設定・bufferを予約待ち状態が保持し、途中で接続設定を変更できないようにします。
4. 生成側は両配列を上限分予約します。片側だけの確保失敗では追加分を解放し、固定Reportと失敗用空配列は保持します。予約容量は短い結果でも解放まで使用量に残します。
5. `execute`は一度だけ呼べます。予約不足は`OutputBudget`、確保失敗は`AllocationFailed`を`Reserve`段階で返し、起動しません。成功時にだけ現在の時計取得・期限計算・起動・転送・回収へ進みます。二度目の要求検査で優先順位を変えません。
6. 起動後のReport・公開enum・外側のOutcome・配列確定に、新しい領域確保を要求しません。runtimeの生バイトを予約済み要素へコピーし、出力の保持長だけを確定します。
7. 予約待ちの破棄は未公開の領域だけを解放します。実行後に未回収資源がある場合は、既存runtimeが所有を保持し、接続停止と後始末結果を隠しません。

`prepare`と`execute`は実行制御です。観測APIではありません。既存Rust APIの`run` / `run_with_reservation`は、この段階処理を連続して呼ぶ入口として互換性を保ちます。まず試験用Backendで、従来APIと段階APIの結果・OS操作列・観測記録を比較します。

## 生成過程と観測

非最適化の共通IRと各段階の表現は従来どおり保存します。生成C・LLVM・QBEでは要求の取出し、prepare、配列予約、execute、要素変換、解放を明示し、Nativeでは同じ対応をMIR/LIR・注釈付きASM・Objectの出自に残します。外部runtimeを呼ぶことを理由にソース位置を消しません。

ReportのNodeId/Spanは既存の標準ラッパー内操作を指し、利用者のCallの位置とは分けて保持します。SourceIdは同じコンパイルのソース表に対応させます。ABIから渡した出自を共通runtimeの操作記録へ引き継ぎ、診断へ引数・環境・本文を自動掲載しません。

結果の位置指定読取りと読取り専用の観測は、期限・転送・次の呼出し・解放を進めません。記録容量や記録確保失敗による制御変更を禁止します。汎用の記録再生機能や、全生成経路の成功操作を扱う共通観測APIまで、この変更の完了条件に追加しません。

## ABIへ渡す値と所有

Native系の初版対象は`x86_64-pc-windows-msvc`と`x86_64-unknown-linux-gnu`です。runtimeと生成物は同じtarget・呼出し規約を使います。生成Cの子プロセス利用にもtargetの明示を追加する方針です。子プロセスを使わない既存C出力の指定方法は維持します。

| 値 | 表現案・寿命 |
| --- | --- |
| ABIヘッダ | 版と構造のサイズを明示。未知版・サイズ不一致は起動前に拒否 |
| 要求 | targetのUTF-8列、引数ごとのUTF-8列、stdinの生バイト列、それぞれの長さ。NUL終端に依存しない |
| 数値 | 件数・長さ・上限・ミリ秒・出自はu64。タグ・状態は明示した整数。Rustの`usize`・`bool`・enum・`Vec`をABIへ出さない |
| Native参照 | ポインタと長さ。呼出し中の有効性・非重複・読取り権限は呼出し元の契約。無効ポインタを安全に検査できるとは主張しない |
| 接続 | 明示生成した不透明なcontextが所有。同時に1操作、再入・並行操作・設定の途中変更は禁止 |
| 要求の保持 | `prepare`が返るまでに必要なコピーを確保。生成側の一時値を後から参照しない |
| 結果 | contextが固定Reportと生バイトを保持。生成側の値へ変換し終えるまで有効。次の要求前に明示解放 |
| 未回収OS資源 | 結果の解放とは別にcontext/runtimeが保持。結果解放で接続停止を解除しない |

要求の意味上の不正と、壊れたABIの表現を分けます。空対象・NUL・時間0など、正しいCerune値で表せる不正は共通runtimeの失敗値です。未知版・タグ・壊れた状態・長さの物理表現不能・不正UTF-8等は接続契約または資源の失敗です。`Unconfigured`や成功した空結果に置換しません。元の操作位置付きで停止し、子の終了コードへ偽装しません。

panic・unwindをC呼出し元へ越境させません。捕捉可能な内部失敗では接続を再利用せず、残る資源の所有を保持します。allocator abortやOSによる終了まで正常なOutcomeで回復できるとは保証しません。

## 固定Reportの表現案

C ABI・WAT host・比較試験で同じフィールド表を使います。配列本体・診断文字列・ポインタは含めません。初版案は**32個のu64（256バイト）**です。Nativeでは対象ABIのu64配列、バイト列へ保存する場合はlittle-endian、WATのi64は同じ64ビット列として扱います。

| word | 内容 |
| --- | --- |
| 0–1 | 版=1、word数=32 |
| 2–5 | NodeId、SourceId、Span.start、Span.end（UTF-8バイト範囲） |
| 6 | Started：0=No、1=Yes、2=Unknown |
| 7–11 | stdin_requested、stdin_accepted、stdout_limit、stderr_limit、timeout_ms |
| 12–16 | stdout_read、stderr_read、stdout_retained、stderr_retained、progress_known（0/1） |
| 17–18 | Exit：0=Absent、1=Code、2=Signalとu32の値。Absentの値は0 |
| 19–21 | 失敗コード、段階、転送方向。失敗なしは全て0 |
| 22–24 | 判定後cleanupの種別、flags、error |
| 25–27 | 接続の生報告cleanup_reportedの種別、flags、error |
| 28–31 | 予約。初版は0。未知の拡張を黙って読む用途にしない |

失敗コードは0=なし、1から順に`InvalidRequest, RequestLimit, Unconfigured, Unregistered, ConnectionDisabled, OutputBudget, AllocationFailed, NotFound, Denied, Resources, Unsupported, StartFailed, TransferFailed, CloseFailed, WaitFailed, ZeroProgress, InvalidResponse, OutputLimit, Timeout, CleanupFailed, ProgressUnknown`とします。言語のenum順が変わってもこの表から名前で変換します。

段階は0=なし、1=Validate、2=Reserve、3=Start、4=Transfer、5=CloseStdin、6=Wait、7=Cleanupです。転送方向はTransfer時のみ1=Stdin、2=Stdout、3=Stderrとし、それ以外は0です。

cleanup種別は0=Absent、1=Complete、2=Incompleteです。Completeではflagsのbit0だけがtermination_requestedを表し、errorは0です。Incompleteではbit1=child_reaped、bit2=handles_closed、bit3=scope_finishedとし、errorは1=Timeout、2=Terminate、3=Reap、4=Close、5=InvalidResponseです。Absentはflags/errorとも0です。未知タグ・無効な組合せ・予約bit・逆転Span・時間の丸めを拒否します。

Rust enumの宣言順やホスト整数幅に依存するcastは禁止です。コード化・復元の全選択肢を検査し、cleanupとcleanup_reported、転送数と保持数を混同しないことを独立した期待値で確認します。

## WAT hostとの対応

同じ段階APIを値転送のアダプタで包みます。hostが生成WATへ予約callbackで再入する方式は採りません。

| 段階 | 生成WATとhostの契約案 |
| --- | --- |
| 要求受渡し | 宣言した長さと最大8バイトのchunkを版付きimportで渡す。欠落・超過・二重終端を拒否 |
| prepare | 全要求を受けて共通runtimeのprepareを一度呼ぶ。結果確定済み／予約待ちを区別 |
| 予約・execute | private memory内で言語配列を予約し、結果をhostへ渡す。成功時だけ起動 |
| 結果読取り | 固定Reportのwordと出力chunkを位置指定で読む。読取り位置を進めず、何度読んでも結果を変えない |
| 解放 | 生成コードから明示した制御呼出しで結果を解放。観測読取りで自動解放しない |

要求の受渡し領域の確保は起動前です。hostは要求の宣言長を無検査で切り詰めず、接続の実行上限を使った独自の先行判定で共通runtimeの優先順位を変えません。wasm32の物理領域制約は意味上の上限と分けます。数値はBigIntまたはi64のビット表現を使い、JavaScript Numberを経由してu64を丸めません。

内部memoryは公開しません。既存の出力・`flush_stdout_v1`・入力・引数・ファイルimportは維持します。子プロセス操作を含む新しい生成物は新しい版付きimportを要求し、旧hostならinstantiateで失敗します。子プロセスを含まない生成物には追加importを要求しません。正確なimport名とシグネチャはWAT実装前の契約書で固定します。

## 起動設定・リンク・配布への影響

生成実行ファイルでは、既存の先頭マーカーの内側へ設定を追加する案です。これは未実装の起動形式です。

```text
program --cerune-runtime-v1 --process-config process.json --file-root directory -- program-arguments
```

マーカーがなければ従来のargvを維持し、接続は未設定です。`--`以降は解釈しません。欠落・重複・未知設定はプログラム開始前の診断です。JSONはCLIと同じ`cerune-process-v1`を使い、解析器を共有します。Linux監督のパスも明示し、実行中OSから別target用の設定へ切り替えません。

| 経路 | 必要な変更 |
| --- | --- |
| emit-c / emit-llvm / emit-qbe | 版付き外部呼出しと必要ABI・targetを生成物で明示。外部ツールによるcompile/linkは利用者の責務。runtimeを黙って探してリンクしない |
| emit-asm / emit-obj | LIRに要求・出自・段階呼出しを残し、SASHIMIは従来どおり命令符号化と再配置を担当 |
| build native | runtime archiveを明示入力に追加。target・ABI版・hash・システム依存・リンク引数と結果を記録 |
| release native | buildの依存記録を検査・保持。静的リンク済みruntimeと別プロセスのLinux監督を区別 |
| WAT | 生成はemit-onlyのまま。hostのABI版・必要import・runtime実装を検証し、配布条件を明示 |

子プロセス使用時の新しいリンク依存は互換性変更です。既存の`target-system-runtime` / `cerune_runtime: in-object`だけの記録では追加依存を表せません。Nativeのmanifest・runtime記録・release検証は版付きの変更で扱い、古い成果物へ存在しない依存情報を書き足しません。

ローカルの登録先絶対パス・環境値・設定JSONを配布物へ自動埋込みしません。Linux監督の同梱は、必要な実行ファイル・版・target・hashを記録した上で扱います。runtimeライブラリのファイル名、公開ビルドオプション、配布ディレクトリ構成はリンク試作で確定します。

## 実装順と検証

| 順序 | 変更単位 | 完了条件 |
| --- | --- | --- |
| 1 | 共通runtimeのprepare/execute分離と固定Report codec | 従来APIと段階APIの結果・OS操作列・観測が一致。予約失敗後に起動せず、二重実行・予約待ち破棄を検証 |
| 2 | Native系C ABIのアダプタと静的リンク試作 | Windows/Linuxで型幅・配置・寿命・不正版・失敗・回収を検証。Rustと独立したC側の期待値と比較 |
| 3 | 生成Cのloweringと起動設定 | [registered.ceru](../../examples/process_execution/registered.ceru)を4実行器と比較。未接続・部分出力・非0終了・継続・所有・失敗位置が一致 |
| 4 | LLVM・QBE・Native、Native build/release | 同じABIを使用。非最適化/SSA、明示target・リンク・依存記録を比較 |
| 5 | WATの版付きhost、Linux監督の配布仕上げ | private memory、旧hostの拒否、同じ要求・結果・回収、配置変更後の配布実行を検証 |

生成C対応までは、未対応の生成経路を成功する空実装へ変えません。将来APIの未実装`.ceru`を実行example一覧へ追加せず、既存の例を各経路で実行できるようになった段階で説明を更新します。

各段階で、全失敗分類・上限0/ちょうど/超過・u64精度・NodeId/SourceId/Span・コピー後の独立性を比較します。短い転送・中断・契約違反・後始末不能は試験用応答で揃え、実OSでは専用fixtureを使います。観測の有無で制御を変えず、期限付近の実時間結果が別実行で必ず同じになるとは主張しません。
