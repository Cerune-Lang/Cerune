# Windows子プロセスBackend

[English](subprocess-windows.en.md)

[共通runtime](subprocesses.ja.md)へWindowsの実OS接続を追加しました。Rustの`Processes<Windows>`から利用できます。**言語の`std:process`、IR命令、CLI、生成経路への接続は未実装**です。登録形式・数値上限はこのRust初版のもので、言語の公開契約ではありません。

## 実装と責務

| 部品 | 担当 |
| --- | --- |
| `Processes` / `Transfers` | 要求・容量・共通期限、要求/操作番号、出自、受理数、最初の失敗、取消し、後始末の判定 |
| [Windows](../../src/runtime/process/windows.rs) | 明示登録、OS通知の変換、期限内の待機、保持資源、読取り専用OS記録 |
| [scope](../../src/runtime/process/windows/scope.rs) | 絶対パス・明示環境で停止状態の子をJob所属済みで生成し、期限確認後にresume。子とJob活動数を確認 |
| [pipe](../../src/runtime/process/windows/pipe.rs) | 親はoverlapped I/O、子は同期標準I/O。所有したTransferのbufferを直接使用し、終端まで固定アドレスのOVERLAPPEDとeventを保持 |
| [実行検証](../../experiments/subprocess/windows_backend.rs) | 本体Backendでfixtureを実行し、共通の結果・観測・資源回収を検査 |

起動・転送・取消し・終了・回収の意味をBackendで再定義しません。`submit`・`cancel`・`begin_cleanup`は待機せず、`poll`は共通runtimeから渡された絶対期限まで待ちます。初版は最大1msごとに非ブロッキング確認を行います。最適な待機方式や性能比較は後続です。

## 登録と起動

`Windows::new(Vec<Target>, trace_limit)`で、対象名・絶対実行パス・絶対cwd・明示環境を渡します。対象名をパスとして解釈せず、PATH探索、親環境の一括継承、cwd補完はしません。環境名はWindowsの大文字小文字を無視するordinal比較で重複検査・整列し、値は正規化しません。

この初版は次の容量を起動前に検査します。

| 対象 | 上限 |
| --- | --- |
| 登録 | 64対象、対象名256 UTF-8バイト、環境128組/対象 |
| 登録の合計 | 名前・環境のUTF-8バイト数とパスのUTF-16バイト数の合計1 MiB |
| パス | 各32766 UTF-16 code unit、終端NULは別 |
| 環境block・引用後command line | 終端NUL込み32767 UTF-16 code unit |
| 呼出し引数 | 4096件、合計16384 UTF-8バイト。共通runtimeの上限も適用 |
| OS記録 | 明示した0〜4096件。0は無効化 |

pipe名は128bit乱数を含み、first-instance・owner/system向けACL・remote拒否を指定します。子には標準I/Oの3handleだけを継承させます。対象が起動する前の失敗は`NotStarted`、子を生成した後のresume失敗や期限到達は資源を保持して`Uncertain`へ分けます。OSの起動API内部を指定時刻で強制中断できる保証ではありません。

## 転送・取消し・失敗

| OSの結果 | 共通runtimeへの通知 |
| --- | --- |
| 成功 | 確認済み数と`Done`。短い転送も実数を返す |
| 読取りのbroken pipe | `Eof`、0バイト |
| 0バイト成功 | EOFへ変えず`Done`。共通runtimeが0進行と判定 |
| 取消し要求成功 / 間に合わない / 失敗 | `Requested` / `TooLate` / `Failed`。終端通知とは別 |
| 終端した取消し・転送失敗で総数不明 | `ProgressUnknown`。確認済み下限を保持し、0バイト確定へ変えない |
| 完了問合せの予期しないエラー | 終端の証拠とせずbufferを保持。OSエラーを記録し、共通期限に従う |
| 提出前の期限到達・event確保失敗 | `NotSubmitted`。OSがbufferを参照していないことを確認 |

取消し後も正常完了はあり得ます。終了コード0以外は操作失敗にせず値として返します。成功にはstdin close、両出力EOF、直接の子の終了が必要です。後始末では必要に応じてJob全体を終了させ、子の終了・活動数0・全転送の終端・handle閉鎖を確認します。

`Backend::start`に`CallContext { call, origin }`を追加しました。OS記録は、起動失敗も含め呼出し番号・NodeId/Span、転送ではTicketも保持します。これはRust接続契約の変更です。既存CLIや言語構文の変更ではありません。共通記録とOS記録はいずれも本文・引数・環境・絶対パスを自動掲載せず、上限到達や記録確保失敗で実行制御を変えません。

## 未回収資源の所有

期限切れ・進行不明・回収未完了では、共通runtimeが接続を再利用不可にします。資源回収と転送量の確定は別であり、handleを回収できても転送量不明を成功へ変えません。

1. `Processes::into_inner()`でBackendの所有を回収できます。
2. `has_unreclaimed_resources()`と`events()`は読取り専用です。
3. `reclaim(timeout)`は明示的な実行制御です。取消し・終了要求・有限の回収を行いますが、過去のOutcomeを書き換えません。
4. 最後の所有者のDropでは、終了・取消しと一度の完了確認を行い、待機threadを作りません。なお未回収なら、buffer・OVERLAPPED・event・handle・Job情報をprocess終了まで保持し、`retained_connections()`へ加算します。

最後の方式はuse-after-freeを避けるための例外時の保持です。自動回復・成功回収ではなく、繰り返せば資源を消費します。通常は共通runtimeの後始末、必要なら明示`reclaim`を使います。Rust allocatorの失敗、OS資源枯渇、管理範囲を離れる子孫、任意の外部干渉まで回復を保証しません。

## 検証と次の作業

`cargo run --example subprocess_windows_backend -- target/subprocess-windows-backend`で、未作成の出力先へJSONLを保存します。18ケースでargv・環境・出自、NUL/CR/LF、128 KiB同時転送、非0終了、上限ちょうどのEOF、容量超過、時間切れ、孫の回収、早期close、未登録/対象なし後の継続、観測上限、提出期限、明示回収を検査します。別々のOS実行では通知順・操作番号が変わり得るため、観測有無の比較は結果・出力・停止理由を対象にします。

単体試験ではOS問合せ失敗を注入し、未完了bufferと最終所有者の保持も検査します。この試験は意図的に約4 KiBとevent等を試験process終了まで保持します。

[Linux Backend](subprocess-linux.ja.md)も接続済みです。次は言語の公開型・標準モジュール・共通IRへ進みます。

参照するOS契約は[CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)、[GetOverlappedResult](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)、[ReadFileのpipe規則](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile)、[CompareStringOrdinal](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal)です。
