# 子プロセス接続の通信・取消し試作

[English](README.en.md)

[実OS接続設計](../../../docs/design/subprocess-os.ja.md)の成立条件を検証します。[起動・管理範囲の試作](../README.md)と部品を共有し、個別検証に続いて起動・転送・終了を接続しました。**本体Backend・言語APIではありません。** 製品の制御プロトコルや数値上限を確定する変更でもありません。

## 実行

リポジトリ直下で、未作成の出力ディレクトリを指定します。Rust 1.98.0、Windows x86-64／Linux x86-64で検証しています。

```text
cargo test --example subprocess_transport_probe
cargo run --example subprocess_transport_probe -- target/subprocess-transport-probe
```

出力は`observations.jsonl`です。件数・確認済み転送量・OSエラー・資源回収の根拠を残し、本文・引数・環境・絶対パスは記録しません。CIは両OSで実行してJSONLを保存します。期待した失敗を正しく検出したケースも成功です。

## Linux：9ケース

| ケース | 確認すること |
| --- | --- |
| descriptor-roundtrip | 別workerへ制御接続を渡し、SCM_RIGHTSでstdin/stdout/stderrの親側3fdを受信。方向・pipe種別・close-on-execを検査し、日本語・NUL・CR/LFを保持。複製を閉じてEOFを確認し、終了7とworkerの回収を確認 |
| invalid-control-packets | 版・要求番号・宣言長・短い本文・余分なfd・補助情報の切詰め・過大本文・メッセージ種別の8異常を拒否。各回のfd数が変わらない |
| duplicate-control-identity | 次の要求に対し、古い要求番号を再使用した通知を拒否 |
| control-peer-closed | 制御相手の切断をEOFとして検出 |
| sigpipe-default-handler | SIGPIPEの既定動作でも閉じたpipeへのwriteで終了せずEPIPEを返す。handlerとthread maskを維持 |
| sigpipe-pending-preserved | 呼出し前から保留されたSIGPIPEを消費せず、元のmaskを復元 |
| managed-finish | 直接の子が終了0でも孫がpipeを保持。明示終了要求後に孫・監督を回収し、既読の部分出力とEOFを確認 |
| managed-disconnect | 同じ状態で制御接続を閉じても、監督が管理範囲を回収。workerの検査成功とpipeのEOFを確認 |
| managed-invalid-control | 制御接続を開いたまま不正な版を送信しても、監督が管理範囲を回収。fd継承制限の走査方式も明示的に選んで検証 |

試作用フレームは24バイト固定、little-endianです。`CSPT`、版u16、種別u16、要求番号u64、宣言長u32、値u32を持ちます。種別はHELLO／READY／FINISH／EXITと、管理範囲の回収確認用CLEANEDです。要求番号は固定fixtureの1です。HELLO検査後にだけ対象を起動します。任意の登録対象・引数・環境は送信しません。

`SOCK_SEQPACKET`を使い、受信本文は25バイト、補助情報も固定容量です。受信済みfdは先に所有し、その後の検査で失敗しても閉じます。親側pipeの複製をworkerで閉じ、workerの制御fdも対象へ継承しない設定へ戻します。[SCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html)

2026-11-02のWSL（kernel release `4.4.0-26100-Microsoft`）では、152バイト送信／24バイト受信の単独例で`MSG_TRUNC`が返りませんでした。受信に余分な1バイトを設け、フラグに加えて実際の受信長でも拒否します。この観測だけで通常のLinux kernelにも同じ問題があるとは判断しません。[recvmsgの契約](https://man7.org/linux/man-pages/man2/recvmsg.2.html)

pipe書込み時は呼出しthreadでSIGPIPEを一時的に遮断し、先に保留されていなかった場合だけEPIPEに伴うsignalを回収してmaskを戻します。制御socketへの送信は`MSG_NOSIGNAL`を指定します。試験の既定handler設定は独立fixture内だけです。同時に外部から送られるSIGPIPEとの合流は未検証です。

管理範囲のケースは、専用workerでsubreaperとprocess groupを管理し、既存の起動・回収処理を再利用します。直接の子の終了状態はWNOWAITで保持し、groupへの終了要求後に孫を回収します。workerの終了0は「回収検査が成功した」証拠であり、通信や対象の成功を表しません。切断時は制御経路で回収報告を受け取れないため、workerの検査結果とデータpipeのEOFを別々に確認します。

監督起動前の子側でfd 3以上をclose-on-execへ設定し、制御fdだけを許可します。close_rangeが未対応なら、fork後の単一threadで固定bufferのgetdents64とfcntlを使って/proc/self/fdを走査します。親側の列挙結果には依存せず、走査失敗時は起動を失敗させます。高い番号の継承可能fdが監督へ届かず、親のfd設定が維持されることを検証します。WSLでは走査方式を実測し、CIでは通常選択と強制走査を両方実行します。[close_range](https://man7.org/linux/man-pages/man2/close_range.2.html)

## Windows：13ケース

| ケース | 確認すること |
| --- | --- |
| restricted-pipe-pair | 非継承handle、同名の先行instance重複拒否。owner/system向けACLとremote拒否を設定 |
| short-read-binary | 64バイト要求に対する短い読取り、日本語・NUL・CR/LF、成功時の正確な転送量 |
| pending-read-cancel | 未完了readを取消し、要求応答とは別に中止の終端を確認。OVERLAPPEDアドレスを保持 |
| completion-before-cancel | OS完了後・結果取得前に取消しても、成功と確認済み6バイトを失わない |
| read-write-cancel-race | 32回の競合で成功または中止を区別し、終端を二重計上しない。両結果の発生回数は固定しない |
| pending-write-cancel | pipe容量を超える64 KiB書込みを取消し、終端を確認 |
| peer-closed | 相手の閉鎖をbroken pipeとして保持 |
| pending-drop | 未完了操作のDropで取消し・終端確認を行い、所有handleを解放 |
| job-stdio-roundtrip | 同期標準I/Oを使う子と非同期の親を接続。日本語・NUL・CR/LF、終了7、Job活動数0を確認。継承可能でも一覧にないhost handleは子へ渡さない |
| job-terminate-pending | read・64 KiB writeが未完了の状態でJobを終了。Jobの回収とI/O終端を別々に確認 |
| job-descendant-pending | 直接の子の終了0後も孫がpipeを保持。Jobを終了し、活動数0と残るreadの終端を確認 |
| job-owner-drop | pipeの所有を未完了操作だけに残し、Jobの最終所有者を破棄。直接の子の終了とI/O終端を確認 |
| buffer-ownership-settled | 全ケース後に未回収の隔離保持がない |

各操作は固定アドレスのheap領域にbuffer・OVERLAPPEDを置き、handleとeventを終端まで所有します。`CancelIoEx`の成功を転送完了とは扱いません。`GetOverlappedResult`成功時だけ受理数を確定し、エラー時は不明（JSONのnull）を0へ変えません。[取消し](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)、[完了と転送量](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)

pipe名には128bit乱数を使い、first-instance、remote拒否、明示ACLを指定します。別ユーザーへの偽装やremote接続による侵入試験は行っていません。[CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew)

## 検証結果と残る作業

2026-11-02のローカル実行でLinux 9ケース、Windows 13ケースが成功しました。通常の通信・完了待機は5秒、WindowsのDrop回収待機は250msです。これらは試作の見張り用であり、言語の実行時間上限ではありません。

Windowsでは終端未確認のままDrop期限を過ぎた場合、OS参照中の領域を解放せず隔離保持し、検証を失敗させます。この保持の発生・資源枯渇からの回復は未検証で、恒久的な製品実装ではありません。

起動・管理範囲との接続は上記ケースで確認しました。次は共通runtimeのBackendへ接続し、要求・操作番号、共通期限、取消し応答と終端、後始末報告を対応づけます。公開前に、実接続の登録設定・容量制限と、終端未確認の資源を最後の所有者が保持する方式を確定する必要があります。監督の強制終了、起動途中の切断、回収不能、OSエラーの網羅的注入は未検証です。OS資源枯渇や任意の外部強制終了まで回収成功を保証する試作ではありません。
