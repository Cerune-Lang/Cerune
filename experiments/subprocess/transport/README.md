# 子プロセス接続の通信・取消し試作

[English](README.en.md)

[実OS接続設計](../../../docs/design/subprocess-os.ja.md)の成立条件を、[起動・管理範囲の試作](../README.md)と分けて検証します。**本体Backend・言語APIではありません。** 製品の制御プロトコルや数値上限を確定する変更でもありません。

## 実行

リポジトリ直下で、未作成の出力ディレクトリを指定します。Rust 1.98.0、Windows x86-64／Linux x86-64で検証しています。

```text
cargo test --example subprocess_transport_probe
cargo run --example subprocess_transport_probe -- target/subprocess-transport-probe
```

出力は`observations.jsonl`です。件数・確認済み転送量・OSエラー・資源回収の根拠を残し、本文・引数・環境・絶対パスは記録しません。CIは両OSで実行してJSONLを保存します。期待した失敗を正しく検出したケースも成功です。

## Linux：6ケース

| ケース | 確認すること |
| --- | --- |
| descriptor-roundtrip | 別workerへ制御接続を渡し、SCM_RIGHTSでstdin/stdout/stderrの親側3fdを受信。方向・pipe種別・close-on-execを検査し、日本語・NUL・CR/LFを保持。複製を閉じてEOFを確認し、終了7とworkerの回収を確認 |
| invalid-control-packets | 版・要求番号・宣言長・短い本文・余分なfd・補助情報の切詰め・過大本文・メッセージ種別の8異常を拒否。各回のfd数が変わらない |
| duplicate-control-identity | 次の要求に対し、古い要求番号を再使用した通知を拒否 |
| control-peer-closed | 制御相手の切断をEOFとして検出 |
| sigpipe-default-handler | SIGPIPEの既定動作でも閉じたpipeへのwriteで終了せずEPIPEを返す。handlerとthread maskを維持 |
| sigpipe-pending-preserved | 呼出し前から保留されたSIGPIPEを消費せず、元のmaskを復元 |

試作用フレームは24バイト固定、little-endianです。`CSPT`、版u16、種別u16、要求番号u64、宣言長u32、値u32を持ちます。種別はHELLO／READY／FINISH／EXIT、要求番号は固定fixtureの1です。HELLO検査後にだけ対象を起動します。任意の登録対象・引数・環境は送信しません。

`SOCK_SEQPACKET`を使い、受信本文は25バイト、補助情報も固定容量です。受信済みfdは先に所有し、その後の検査で失敗しても閉じます。親側pipeの複製をworkerで閉じ、workerの制御fdも対象へ継承しない設定へ戻します。[SCM_RIGHTS](https://man7.org/linux/man-pages/man7/unix.7.html)

2026-11-02のWSL（kernel release `4.4.0-26100-Microsoft`）では、152バイト送信／24バイト受信の単独例で`MSG_TRUNC`が返りませんでした。受信に余分な1バイトを設け、フラグに加えて実際の受信長でも拒否します。この観測だけで通常のLinux kernelにも同じ問題があるとは判断しません。[recvmsgの契約](https://man7.org/linux/man-pages/man2/recvmsg.2.html)

pipe書込み時は呼出しthreadでSIGPIPEを一時的に遮断し、先に保留されていなかった場合だけEPIPEに伴うsignalを回収してmaskを戻します。制御socketへの送信は`MSG_NOSIGNAL`を指定します。試験の既定handler設定は独立fixture内だけです。同時に外部から送られるSIGPIPEとの合流は未検証です。

## Windows：9ケース

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
| buffer-ownership-settled | 全ケース後に未回収の隔離保持がない |

各操作は固定アドレスのheap領域にbuffer・OVERLAPPEDを置き、handleとeventを終端まで所有します。`CancelIoEx`の成功を転送完了とは扱いません。`GetOverlappedResult`成功時だけ受理数を確定し、エラー時は不明（JSONのnull）を0へ変えません。[取消し](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)、[完了と転送量](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)

pipe名には128bit乱数を使い、first-instance、remote拒否、明示ACLを指定します。別ユーザーへの偽装やremote接続による侵入試験は行っていません。[CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew)

## 検証結果と残る作業

2026-11-02のローカル実行でLinux 6ケース、Windows 9ケースが成功しました。通常の通信・完了待機は5秒、WindowsのDrop回収待機は250msです。これらは試作の見張り用であり、言語の実行時間上限ではありません。

Windowsでは終端未確認のままDrop期限を過ぎた場合、OS参照中の領域を解放せず隔離保持し、検証を失敗させます。この保持の発生・資源枯渇からの回復は未検証で、恒久的な製品実装ではありません。

次は既存の起動・管理範囲の試作と今回の転送を接続します。Linuxでは不要なhost fdの継承防止、監督切断時の管理対象回収、制御状態と期限の統合を検証します。Windowsでは子へ渡す標準I/O、Job終了と未完了操作、最終所有者の破棄を合わせて検証します。その後、共通runtimeのBackendへ接続して結果・観測を比較します。OS資源枯渇や任意の外部強制終了まで回収成功を保証する試作ではありません。
