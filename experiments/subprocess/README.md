# 子プロセスのOS検証

[English](README.en.md)

Windows/Linuxで、shellを介さない起動、引数、同時入出力、期限、子孫の回収を調べる実験です。[子プロセス契約](../../docs/design/subprocesses.ja.md)の実OS接続を作る前の成立条件を確認します。**Ceruneの言語API・通常の実行exampleではありません。共通runtimeのBackendへもまだ接続していません。**

## 実行

Rust 1.98.0とWindows 10以降、またはLinuxを使います。検証したローカル環境はWindows x86-64とUbuntu（WSL）のx86-64です。実験以外の任意のコマンドは実行せず、同じ実験用バイナリを子・孫のfixtureとして使います。

リポジトリ直下で次を実行します。出力先は存在しないディレクトリを指定してください。再実行時は別名にし、既存の成果物を上書きしません。

```text
cargo test --example subprocess_probe
cargo run --example subprocess_probe -- target/subprocess-os-probe
```

出力先には実験用バイナリのコピー、作業ディレクトリ、`observations.jsonl`を作ります。結果ファイルには成否、要求・受理・保持数、終了状態、後始末の根拠と経過時間を残し、引数・環境・入出力の本文や絶対パスは書きません。CIもWindows/Linuxで実行し、JSONLだけをartifactとして保存します。

## ケースと結果

2026-11-01のローカル実行で、両OSとも次の9ケースが成功しました。ここでの成功は、期待した失敗を正しく検出できたことも含みます。

| ケース | 確認したこと |
| --- | --- |
| context | 空引数・空白・引用符・末尾backslash・日本語・CR/LF・shellの特殊文字をargvとして保持。空白/日本語を含む実行パスとcwd、明示環境、親のPATHを継承しないこと |
| duplex | 子がstdout/stderrを先に各128 KiB書いてからstdinを読む。stdin 128 KiB、stdout 256 KiB、stderr 128 KiBを欠落なく回収 |
| exit | 非0終了を保持。`exit(-1)`はWindowsで4294967295、Linuxで255。OSの値を同じ値に偽装しない |
| early-close | 子がstdinを読まずに終了したときの書込み失敗と、APIが受理を返したバイト数 |
| limit | stdout上限7バイトで8バイトを読んで超過を検出し、保持は7バイトだけ |
| exact | 7バイトの出力に上限7を指定し、追加読取りでEOFを確認して成功 |
| sleep | 2秒の期限後に停止させ、停止前のNUL・CR/LFを含む部分出力を回収 |
| descendant | 直接の子が終了0でも孫がpipeを保持するため未完了。2秒の期限後、孫も終了・回収 |
| missing-executable | 存在しない実行ファイルの起動を成功扱いしない |

`stdin_accepted`はwrite APIが成功として返した数であり、子が実際に読み取った数ではありません。早期closeの受理数や同時に発生する失敗の観測順、経過時間はOS・実行負荷で変わり、同じ数値を要求する比較にはしていません。

## OSごとの方法

| OS | 起動と管理 | 後始末の確認 |
| --- | --- | --- |
| Windows | `CreateProcessW`でJob Listと継承handle一覧を指定。停止状態で生成してから明示resume。Job handleは子に渡さず、breakawayを許可しない。ウィンドウは表示しない | `TerminateJobObject`後、直接の子の終了とJobの活動プロセス数0を確認。送受信workerの終了後にjoin |
| Linux | この実験専用のプロセスをsubreaperにし、子は専用process groupへ配置。`waitid(WNOWAIT)`で終了を観測し、終了要求まではleaderをreapしない | groupへSIGKILLを送ってから直接の子と引き取った子孫をwait。未回収の子がいないこととworker終了を確認 |

WindowsのJob Listは初期threadが動く前にJobへの所属を決める方法です。[Microsoftの説明](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812)に基づきます。単なる「起動してからJobに入れる」競合を避けますが、起動APIの任意の途中で監督プロセスがクラッシュした場合まで今回検証したものではありません。

Linuxでは[WNOWAIT](https://man7.org/linux/man-pages/man2/waitpid.2.html)により終了状態の取得と回収を分けます。groupへの終了要求より先にleaderをreapせず、PIDが再利用された別のプロセスへ終了要求を送らないようにします。[subreaper](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html)の指定は呼出しプロセスへ影響するため、Ceruneを埋め込むhostへそのまま適用する設計にはしません。

後始末の待機は3秒です。workerが完了した場合だけjoinし、完了しなければ失敗として記録します。Dropの終了要求は例外経路の補助であり、それだけで回収成功とは判定しません。

## 本体へ接続する前に残ること

1. **進行中の転送を扱う接続契約。** この実験は3つのworkerで同時に転送し、各workerの終了時に結果を受け取ります。共通runtimeの`poll`は一度に一事象を返す契約です。実装をそのまま移すと、未報告の転送や期限後に完了する転送を取りこぼすため、開始・保留・完了・取消しと受理数の確定を明示する必要があります。
2. **監督プロセスの置き場所。** Linuxのsubreaperは専用の補助プロセス内で扱う案を検討します。その起動・失敗・終了、残る資源の所有、配布方法を決めます。今回、処理系の埋め込みhostへ副作用を追加する方式は採用していません。
3. **後始末失敗と再実行。** 今回は実OSで正常に回収できるfixtureを検証しました。終了要求失敗、回収不能、handle close失敗、監督プロセス自体の異常終了を実接続で分類し、共通runtimeの接続停止・部分結果へ対応づける作業が残ります。
4. **公開機能への接続。** 登録形式、固定する数値上限、`std:process`の型、IR命令、各経路のABI、言語exampleは後続です。一般的なOS接続としての性能や、観測の有無による意味の一致も未検証です。

管理範囲を離れる子孫や未信頼コードのsandboxは対象外です。外部からの強制終了やOS資源枯渇を含めて、指定時間内の完了を保証する実験ではありません。
