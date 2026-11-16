# WAT子プロセスhost契約 v1

[English](wat-process-host.en.md)

**段階：host呼出しと独立したABI試験を定義します。生成WATへの接続は後続です。** [共通の子プロセス契約](subprocess-generated.ja.md)と同じ要求・失敗・Reportを扱います。言語機能をWAT専用に減らしません。

## 誰が何を行うか

| 担当 | 処理 |
| --- | --- |
| 生成WAT | 引数を左から評価し、要求転送、両出力配列の起動前予約、結果変換、所有解放、元の位置付き診断を行う |
| host | インスタンスごとに要求と結果を保持し、既存Rust runtimeのprepare/executeを呼ぶ。Windows/Linux接続とJSON解析も共有する |
| 観測 | Reportと結果本文は位置を指定して読む。繰返し・逆順の読取りで、起動・時計・消費数・解放・接続状態を変えない |

WATはemit-onlyのままです。hostは起動前に接続JSON・対象OS・Linux監督を明示し、cwdや環境変数から接続を推測しません。hostからWATへ再入して予約を呼び出しません。内部memory・アドレス・所有者をimport/exportしません。

## import一覧

すべてmodule名は `cerune` です。以下は版1の型です。i64は符号ではなくu64のビット列として扱い、JavaScriptではNumberへ丸めません。

| 名前 | 引数 → 結果 |
| --- | --- |
| `process_begin_v1` | (i64 argument_count, i64 stdout_limit, i64 stderr_limit, i64 timeout_ms, i64 node, i64 source, i64 start, i64 end) → i32 status |
| `process_field_v1` | (i32 kind, i64 index, i64 byte_length) → i32 status |
| `process_chunk_v1` | (i64 bits, i32 count) → i32 status |
| `process_prepare_v1` | () → i32 status |
| `process_execute_v1` | (i32 reservation) → i32 status |
| `process_report_v1` | (i32 word_index) → (i64 word, i32 status) |
| `process_copy_v1` | (i32 channel, i64 offset, i32 requested) → (i64 bits, i32 copied, i32 status) |
| `process_release_v1` | () → i32 status |

statusは0=受理、1=予約待ち、2=結果確定です。prepareだけが1または2、executeは2、その他は0を正常値とします。失敗値は既存C ABIと同じ10=版不一致、11=形状不正、12=順序不正、13=表現範囲外、14=UTF-8不正、15=物理確保失敗、16=接続設定不正、17=内部失敗、18=対象OS不一致です。新しい正常値を黙って受理しません。

## 要求の転送順

beginは前の要求・結果を明示的に解放した後だけ呼べます。fieldは target(kind=0,index=0)、arguments(kind=1,index=0からargument_count未満)、stdin(kind=2,index=0)の順に呼びます。各fieldで長さを宣言し、chunkの下位バイトから1〜8バイトずつ、宣言長ちょうどまで転送します。空のfieldも宣言しますがchunkは呼びません。

重複・飛越し・不足・超過・ゼロ長chunk・二度目のprepareを拒否します。targetとargumentsはUTF-8、stdinは任意のバイト列です。日本語・NUL・CR/LF・結合文字は変換せず保持します。NUL等の意味上の要求違反は共通runtimeのInvalidRequestであり、host独自の成功・Unconfiguredには置き換えません。

wasm32の本文長・引数個数はu32で表現できる値に限ります。hostは確保失敗を検査し、接続の実行上限による検査を先回りしません。timeout、出力上限、出自はu64のまま保持し、共通runtimeへ渡します。転送領域を持てない場合の物理的失敗は、意味上のRequestLimitやOutputBudgetとは別です。start > endは拒否します。

## 起動と予約

| 状態 | 許可する操作 |
| --- | --- |
| Idle | begin |
| Receiving | 順序どおりのfield/chunk、全本文受領後のprepare、releaseによる破棄 |
| Pending | report、execute、releaseによる起動前取消し |
| Ready | report/copy、release |

prepareは共通runtimeの検査と転送領域予約を一度だけ行います。ここでは子を起動せず、時計も取得しません。結果確定なら生成WATは出力配列の予約・executeを飛ばします。予約待ちなら両出力を予約し、executeへ0=予約成功、1=容量不足、2=物理確保失敗を渡します。1/2では子を起動せず、共通runtimeがOutputBudget/AllocationFailedの結果を作ります。

配列の片側だけを確保できた場合は取り消し、予約を再試行しません。起動後は予約済み領域へ結果を格納し、追加の結果領域確保を要求しません。残ったOS資源や無効化された接続は共通runtimeが保持し、releaseで再利用可能に戻しません。

## 結果と停止

reportは[固定Report v1の32語](subprocess-generated.ja.md)を返します。indexは0〜31です。copyはchannel=1 stdout / 2 stderr、requested=1〜8です。offsetが末尾ならcopied=0、途中ならmin(requested,残量)、末尾より先なら範囲エラーです。使わない上位バイトは0にし、失敗時のword/bits/copiedは使用しません。読取りはcursorを進めません。

生成WATはReportの版・語数・予約語・enum・出自・数量を検証し、enumは宣言順でなく名前で変換します。子の非0終了や回復可能な失敗は通常のOutcomeとして呼出し元へ返します。ABI異常は先行出力を既存flush契約で処理し、元のNodeId/SourceId/Spanとstatusを持つ `cerune-process-abi-v1` 診断を出して停止します。要求本文・登録先・環境値は診断に含めません。host例外やWasm engine停止を成功・EOF・深さ上限に偽装せず、停止したインスタンスは再実行しません。

## 互換性と実装順

子プロセス操作を含む生成物は未使用関数内も含め8 importを要求し、古いhostではinstantiate時に失敗させます。操作がない生成物には追加importを要求しません。出力・flush・入力・引数・ファイルの既存importと、mainを一度だけ実行する制約は維持します。

1. [独立したABI試験](../../experiments/subprocess/wat_abi/README.md)で、私有memory、状態順、u64、Reportの読取り、既存runtimeとの一致を検証する。
2. 生成WATへ要求転送、配列予約、Report変換・解放・診断を接続し、既存の.ceru exampleを4実行器と比較する。
3. 参照hostのCLIへ明示接続を追加し、Windows/Linuxの実際の子・出力・時間超過・回収を比較する。その後にruntime・監督の公式配布へ進む。

段階1だけでは生成WATで言語exampleを実行できたことにはしません。試験用WATはexperimentsに置き、未対応の.ceru exampleは追加しません。
