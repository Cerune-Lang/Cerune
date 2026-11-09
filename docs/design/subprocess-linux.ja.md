# Linux子プロセスBackend

[English](subprocess-linux.en.md)

共通runtimeへLinuxの実OS接続を追加しました。Rustの`Processes<Linux>`から利用できます。[標準モジュール・共通IR・4実行器](subprocess-execution.ja.md)と[CLI登録](subprocess-cli.ja.md)も接続しました。**生成経路への操作接続・Linux監督の配布同梱は後続です。** 登録・OS上限はこの接続で検査します。検証用監督はCargo exampleとして別プロセスに構築します。

## 責務と起動

| 部品 | 担当 |
| --- | --- |
| 共通runtime | 要求・上限・期限・転送数・最初の失敗・取消し・回収判定 |
| [Linux Backend](../../src/runtime/process/linux.rs) | 明示登録、監督との通信、非ブロッキング転送、呼出し番号とNodeId/Span、監督自身のreap |
| [専用監督](../../src/runtime/process/linux/supervisor.rs) | subreaper設定、対象のprocess group、直接の子と同じgroupの子孫の回収 |
| [実行検証](../../experiments/subprocess/linux_backend.rs) | 共通runtimeからfixtureを実行し、結果・失敗・観測・host隔離を確認 |

Linux::new(supervisor_path, targets, trace_limit)で監督の絶対パスと対象登録を明示します。登録は対象名・絶対実行パス・絶対cwd・明示環境です。PATH探索、親環境の一括継承、監督の自動再起動はありません。相対パスは拒否します。パスはOSのバイト列、引数・環境はUTF-8で保持し、Unicode正規化をしません。環境名は大文字小文字を区別します。

1. 1呼出しにつき1監督を起動します。呼出し元は監督だけを直接の子として管理します。
2. 版・呼出し番号を照合した後、有限の登録・引数を送ります。不一致では対象を起動しません。
3. 監督が対象を専用process groupで起動し、標準I/Oの親側fdを3個渡して自身の複製を閉じます。
4. 呼出し元が本文を直接転送します。監督は終了状態を通知しますが、groupへの終了要求前にはleaderをreapしません。
5. 終了時はgroupへの終了要求、直接の子と同じgroupの子孫の回収を行います。Backendは回収通知と監督の正常終了・reap、転送終端とfd閉鎖をすべて確認します。

呼出し元のsubreaper・process group・SIGCHLD handlerを変更せず、無関係な子をwaitpid(-1)で回収しません。監督へは制御fdだけ、対象へは標準I/Oだけを継承させます。close_range(CLOEXEC)を利用し、未対応時は固定bufferによる/proc/self/fd走査を使います。

## 初版の容量と通信

次の値はRust接続の制限です。言語APIの公開上限や配布形式とは区別します。

| 対象 | 上限・契約 |
| --- | --- |
| 登録 | 64対象、名前256 UTF-8バイト、環境128組/対象 |
| パス | 各4095 OSバイト、絶対パス、NUL禁止 |
| 対象ごとのパス・環境 | 合計32768バイト。登録名を含む全登録は1 MiB |
| 呼出し引数 | 4096件、合計16384 UTF-8バイト。NUL禁止 |
| 起動要求 | 長さ欄を含む本文65536バイト。符号なしlittle-endian長さ＋バイト列 |
| 制御通信 | SOCK_SEQPACKET、CSRP・版1・種別・非0呼出し番号・全長・値の24バイトheader |
| fd受渡し | READYだけ3個。close-on-exec、pipe種別、読書き方向を検査 |
| 監督の準備 | 5秒。対象起動後のwatchdogは受信した残時間＋5秒、異常時の回収は最大5秒 |
| 接続が受け付ける起動残時間 | 1〜86400000ミリ秒。共通runtimeの期限は別途そのまま適用 |
| OS記録 | 0〜4096件。0で無効化。本文・引数・環境・絶対パスは自動掲載しない |

HELLO往復、START、READYまたはFAILED、終了通知、FINISH、CLEANEDの順序を検査します。重複・版違い・呼出し番号違い・長さ不一致・余分なfd・切詰めを拒否し、受信済みfdを閉じます。可変長の長さを信用して無制限の領域を確保しません。通信と待機で共通の絶対期限を延長しません。

監督のwatchdogは親喪失時の回収補助であり、プログラムの成否を判定しません。OS起動APIの内部を指定時刻に強制中断する保証ではありません。待機方式の最適化・性能比較は後続です。

## 転送・失敗・所有

| 状況 | 扱い |
| --- | --- |
| 正の転送数 | 確認した実数を通知。短い転送も保持 |
| readの0 | EOF。writeの0成功とは区別 |
| EAGAIN / EINTR | bufferを保持して共通期限の再検査へ戻る。0進行に変換しない |
| 待機中の取消し | 同期syscallは既に戻っているため、安全に転送を外し、取消し応答と終端を別に通知 |
| EPIPE等の転送失敗 | 確認済み数を維持して失敗を通知。既定SIGPIPE handlerでhostを終了させない |
| 対象起動失敗の確認＋監督回収 | NotStarted。失敗を返した後、明示的に次の呼出しを行える |
| 起動中の通信喪失・監督異常終了 | Uncertainまたは実行中の失敗。未確認の管理範囲を正常回収にしない |
| 回収期限超過 | 共通runtimeが接続を無効化。Backendは残る監督の所有を保持 |

SIGPIPEは書込みthreadだけで一時的にblockし、既存のpending signalを消費せず元のmaskへ戻します。プロセス全体のhandlerを変更しません。Linuxの失敗した同期writeでは進行数が確定できる場合があり、Windowsの取消し後の進行不明とはOSの証拠が異なります。不明を隠して同じ値を返すことはしません。

events()とhas_unreclaimed_resources()は読取り専用です。reclaim(timeout)は明示的な実行制御であり、過去のOutcomeを書き換えません。Linuxのclose失敗後に同じ数値fdを再closeせず、失敗を記録します。

最終Dropでは制御接続を閉じ、監督へ有限回収を促し、一度だけ監督の終了を確認します。無期限waitや背景threadは作りません。監督がまだ終了していない場合は管理情報をprocess終了まで保持し、retained_connections()へ加算します。自動回復・成功回収ではなく、繰り返せば未回収process等の資源を消費します。通常の後始末と明示reclaimを優先してください。

監督を強制終了した場合、kernel内で停止した処理、管理groupを離れた子孫、外部のwaitやsignal操作、allocator失敗まで回収を保証しません。監督喪失後に身元を確認できない数値PGIDへ終了要求を送りません。

## 検証と次の作業

Linuxで次を実行します。出力先は未作成ディレクトリを指定します。

    cargo build --example subprocess_linux_supervisor
    cargo run --example subprocess_linux_backend -- "$PWD/target/debug/examples/subprocess_linux_supervisor" target/subprocess-linux-backend

24ケースで通常実行・引数と環境・NUL/CR/LF・同時転送・非0終了・容量超過・時間切れ・孫の回収・失敗後の継続・観測・取消しと明示回収を確認します。さらに、検証用の不正監督で版違い・起動中切断・READY後喪失・回収期限超過を検査します。不正監督は実対象を起動しません。hostのsubreaper・group・SIGCHLD・無関係な子の回収権・fdと、既定SIGPIPE・既存pending signalの維持も確認します。OSの通知順・操作番号は固定しません。

[公開データ型](subprocess-values.ja.md)は追加済みです。[返却領域の起動前予約](subprocess-storage.ja.md)も追加しました。次は公開結果の固定形状と共通IRへの操作接続です。監督の本番binary、manifest、build/releaseへの同梱は配布作業で扱います。

OS契約は[wait/waitid](https://man7.org/linux/man-pages/man2/waitpid.2.html)、[Unix socketとSCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html)、[Linux closeの再試行禁止](https://man7.org/linux/man-pages/man2/close.2.html)を参照します。
