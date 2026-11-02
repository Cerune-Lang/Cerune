# 子プロセスの実OS接続

[English](subprocess-os.en.md)

[共通runtime](subprocesses.ja.md)と[転送契約](subprocess-transfers.ja.md)を実OSへ接続する設計です。2026-11-02時点では、共通runtimeと独立したOS試作まで実装済みで、**本体の実OS Backend・言語APIは未実装**です。

## 現在地と採用する構成

| 区分 | 状態 |
| --- | --- |
| 共通runtime | 起動、3streamの転送、期限、取消し、終了監視、後始末を試験用接続で検証済み |
| 独立OS試作 | 両OSの9ケースに加え、Linuxの監督分離を実測済み |
| 接続の設計 | WindowsはJobと非同期I/O、Linuxは専用監督＋呼出し元の非ブロッキングI/Oを採用方針とする |
| 次の検証 | Linuxの制御通信・fd受渡し・SIGPIPE処理、WindowsのOVERLAPPED寿命・取消し・継承handle |
| 後続 | 実OS Backend、登録設定、標準モジュール・IR、各経路・成果物、言語example |

共通runtimeを、意味の判定を担当する唯一の場所にします。Linux監督へCerune IR・VMや別のタイムアウト意味論を持ち込みません。監督はOSの親子関係と資源回収だけを担当します。

| 責務 | 共通runtime | Windows接続 | Linux接続と監督 |
| --- | --- | --- | --- |
| 要求・上限・出自・最初の失敗 | 判定・保持 | OS表現へ変換 | OS表現へ変換 |
| 転送buffer・完了 | 所有権を渡し、終端で回収 | OVERLAPPEDごとに保持 | 呼出し元の接続がfdとbufferを保持 |
| 子・管理範囲 | 起動結果と回収報告を判定 | Job・直接の子を管理 | 監督が直接の子と同一process groupを管理 |
| 取消し | 一度だけ要求し、応答と終端を区別 | CancelIoExと完了通知を区別 | 待機中の操作を外し、実行済み数を保持 |
| 観測 | 読取り専用の履歴・最終状態 | 実行用の操作と分離 | 制御通信を観測APIへ公開しない |

## 明示する接続設定

実行対象名から、絶対実行パス・絶対cwd・明示環境へ対応づける登録表を、起動前に接続へ渡します。PATH探索、親環境の一括継承、現在ディレクトリ依存は設けません。対象名をファイルパスとして解釈しません。

引数・環境のNUL、空または不正な環境名、重複名、OS表現へ変換後の長さ、数値の加算overflowを起動前に検査します。Windowsの環境名比較とUTF-16変換、Linuxのバイト列とfd設定は、それぞれの接続が担当します。登録表・OS構造体・IPCの容量は、共通runtimeの転送buffer上限とは別に制限します。

Linux監督の実行パスとプロトコル版も明示します。監督が見つからない・版が違う場合は、利用者のプログラムを起動しません。実行中OSから生成targetを黙って選ぶことはせず、成果物・実接続のtargetと能力を照合します。公開設定の型名・CLI構文・数値上限はまだ確定していません。

## Linux：専用の監督プロセス

subreaperの指定は呼出しプロセス全体へ作用し、孤児化した子孫の親と回収担当を変えます。そのため埋め込みhostで直接設定しません。[Linux man-pages](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html)

初版は**1呼出しにつき1監督**とし、監督の再利用や常駐daemonは導入しません。呼出し元は監督だけを直接の子として待ち、無関係な子を`waitpid(-1)`で回収しません。SIGCHLDの大域ハンドラやprocess groupも変更しません。

進める順序です。

1. 明示パスで監督を起動し、版・要求識別子・容量を確認する。利用者のプログラムはまだ起動しない。
2. 監督がsubreaper設定と制御接続を確立し、専用process groupへ対象を起動する。
3. 対象のstdin/stdout/stderrの親側fdを呼出し元へ渡し、監督側の不要な複製を閉じる。
4. 呼出し元の接続が非ブロッキングI/Oを進める。監督は直接の子の終了状態を報告し、groupへの終了要求まではleaderをreapしない。
5. 終了要求後に直接の子と管理範囲内の子孫を回収する。呼出し元は転送終端、fd閉鎖、監督の報告、監督自身の終了・回収を確認する。

`WNOWAIT`で終了状態の確認とreapを分けます。数値PIDの再利用後に誤って終了要求を送らないためです。[waitpid/waitid](https://man7.org/linux/man-pages/man2/waitpid.2.html)

制御用socketpairとSCM_RIGHTSによるfd受渡しを候補にします。これにより本文の転送を監督の制御通信へ重ねず、OSの受理数を共通runtimeへ直接返せます。[Unix domain sockets](https://man7.org/linux/man-pages/man7/unix.7.html) 具体的なフレーム形式と実装は次の検証で確定します。

| 通信・fdの条件 | 検証すること |
| --- | --- |
| 版・要求番号・長さ | 不一致、重複、切断、過大フレームを拒否。長さ検査前に可変長領域を確保しない |
| fd受渡し | ちょうど3個、用途・方向を固定。余分なfd、切り詰められた制御情報、部分失敗で受理済みfdを漏らさない |
| fd寿命 | 受信fdにclose-on-execを設定。対象や監督へ不要なhost fdを継承しない。複製を残してEOFを妨げない |
| SIGPIPE | 閉じたstdinへの書込みでhostを終了させない。プロセス全体のsignal設定を変更しない方法を検証する |
| 待機 | EAGAINを0進行と誤認しない。中断と受理済み数を分け、共通runtimeの期限再検査へ戻す |

監督の見張り用期限は資源回収の補助です。プログラムの成功・失敗・期限の判定は呼出し元の共通runtimeが行い、通信や再待機のたびに操作期限を延長しません。

## Windows：Jobと非同期I/O

対象の実行開始前にJob所属を確定し、子へ継承するhandleを明示します。Jobや制御用eventは継承させず、breakawayを許可しません。既存試作のJob Listと停止状態からのresumeを基にします。[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

ブロッキングworkerをそのまま本体へ移さず、接続側のpipeをoverlapped I/Oへ移す方針です。各操作に固有のOVERLAPPEDとeventを割り当て、OSが参照中の構造体・bufferを移動、再利用、解放しません。[Overlapped pipe I/O](https://learn.microsoft.com/en-us/windows/win32/ipc/synchronous-and-overlapped-input-and-output)

`CancelIoEx`は取消し要求であり、転送完了を待ちません。取消し要求後にも正常完了や別のエラーが届くため、要求の応答と終端結果を別々に対応づけます。[CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)

次の試作では、pipeの生成とアクセス制限、OVERLAPPEDの固定アドレス、短い転送、取消し競合、完了通知の二重計上防止、Job終了後の残るI/O、handle閉鎖を検証します。対象へ渡す標準I/Oは通常の読書きに使える接続にします。

## 失敗と残る資源

| 状況 | 共通runtimeへ返す扱い |
| --- | --- |
| 対象も保持資源もないと確認 | `NotStarted`。未登録・対象なし・権限等を区別 |
| 対象の起動または資源回収が不明 | `Uncertain`。監督だけが起動した場合も、未回収資源を隠さない |
| 確認済み部分のある転送失敗 | 部分受理数と本文を終端通知へ付ける |
| それ以上の進行が不明 | `ProgressUnknown`または未完了操作を保持。0バイト成功に変換しない |
| 監督切断・異常終了 | 自動再起動しない。起動・受理数・回収の確定済み部分と不明部分を分ける |
| 終端・取消し応答・回収の未確認 | 接続を再利用しない。接続の生の報告と共通runtimeの判定を分ける |

hostとの通信が閉じた場合、監督は有限の後始末へ移る設計です。ただし監督自身の強制終了、kernel内で停止した処理、管理範囲を離れた子孫まで回収できる保証はしません。監督喪失後に、身元を確認できない数値PID/PGIDへ終了要求を送りません。

OSが参照中のbufferを接続のDropで解放してはいけません。未回収状態を所有者へ返す方式と、最後の所有者を破棄した場合の保持方法は、実Backend公開前に試作で確定します。無期限join、成功扱いの切離し、単にhandleを閉じて完了とみなす方式は採用しません。

## 配布と可観測性

Linux監督はCeruneが管理する補助成果物として配布する方針です。ビルド対象、プロトコル版、必要な能力をmanifestへ記録し、hostが明示した配置を使用します。通常の`cerune`を暗黙に再起動する方式やPATH上の同名実行ファイルへのfallbackは設けません。実際のbinary追加・build/releaseへの同梱は後続です。

観測には呼出し・操作番号、NodeId/Span/SourceId、OS段階、要求/受理/保持数、終了状態、後始末の根拠を残します。引数・環境・本文・ローカル絶対パスを診断へ自動掲載しません。制御用IPCと観測APIを分け、compile/check/emit/観測で子を起動しません。

## 実測と次の区切り

2026-11-02の[Linux隔離試作](../../experiments/subprocess/isolation.rs)は、専用workerで従来9ケースを実行し、10番目の`supervisor-isolation`で次を確認しました。

- 監督のsubreaperは0→1、呼出し元は0→0。
- 呼出し元のprocess groupとSIGCHLD設定を維持。
- 監督が孫を回収し、呼出し元が監督自身を回収。
- 無関係な子の終了255を呼出し元が回収。

これは**監督配置の成立条件**の検証です。fd受渡し、版付き制御通信、監督切断時の回収、本体Backendへの接続、OS異常系全般を検証したものではありません。試作のworker選択引数は製品のCLI契約ではありません。

次の区切りは、Linuxの制御通信とfd所有の試作、およびWindowsのoverlapped I/O試作です。両方の成立条件を確認してから本体Backendへ接続し、共通runtimeの試験と同じ観測項目で比較します。
