# WATのファイル接続契約 v1

[English](wat-file-host.en.md)

**状態：emit-watのstd:fsと版付きhost接続を実装済み。実OSのファイルhostは未提供です。**
[共通ファイル契約](file-io.ja.md)の読取り・新規保存を接続します。既存の共通IR・名前規則・UTF-8・使用量上限・停止方針は変えません。生成WATの意味処理を独立したメモリ接続で4実行器と比較しています。OS接続の検証は別途行います。

## 責務と選択理由

| 担当 | 責務 |
| --- | --- |
| 生成WAT | 引数の左からの評価、名前・上限・使用量の検査、所有領域の予約、再試行、転送数、close、UTF-8検証、出自付き停止 |
| 実行用host | 開始前に明示した作業範囲、通常ファイルの検査、排他的作成、バイト転送、handleの消費 |
| 観測 | 生成過程と診断を読む。接続を選ぶ・応答を注入する機構は実行制御であり、観測APIではない |

内部memory、address、所有者情報をimport/exportせず、既存の[出力契約](wat-output-host.ja.md)と同じ最大8バイトの値渡しを使います。本文全体をhostへ複製してから書く方式、hostからWasmへ再入して本文を取得する方式は採用しません。名前だけを最大255バイトの一時領域へ渡します。生成物の実行exportはmainだけとし、接続試験の転送用exportは製品ABIへ追加しません。

hostはinstanceごとに接続・handle・準備中の名前を保持します。cwd・環境変数や生成時のOSから接続を補いません。生成物はargvマーカーを解釈せず、hostを構成する側が実行前に接続を選びます。同期再入・並行実行・実行中の接続差替えは契約外です。mainは一度だけ実行し、失敗後も同じinstanceを再開しません。

## importと値

ファイル操作を含む生成物は、未使用関数内の操作も含め、ceruneから次の7関数を要求します。v1の契約として扱い、互換性を壊す変更では別の版を使います。

| import | 型 | 意味 |
| --- | --- | --- |
| files_status_v1 | () → i32 | 0=接続済み、1=未設定。その他は契約外応答。消費・openを行わない |
| file_name_begin_v1 | (i32 length) → i32 | length=1〜255。前の準備を破棄し、新しい名前の転送を開始 |
| file_name_chunk_v1 | (i64 bits, i32 count) → i32 | 名前を順番に1〜8バイト渡す。合計は宣言長以下 |
| file_open_v1 | (i32 mode) → (i32 handle, i32 status) | 準備済みの名前でopen。mode=0はread、1はwrite-new |
| file_read_v1 | (i32 handle, i32 requested) → (i64 bits, i32 count) | requested=1〜8。先頭から順に読み、受け取ったバイト数と値を返す |
| file_write_v1 | (i32 handle, i64 bits, i32 count) → i32 | count=1〜8を渡し、受理数または失敗を返す |
| file_close_v1 | (i32 handle) → i32 | handleを一度だけ消費。0=成功、それ以外=close失敗 |

bitsは下位バイトから順に使います。符号付きi64の符号を文字データとして解釈せず、64ビットの並びを保持します。末尾の不要ビットは無視します。UTF-8の途中・NUL・CR/LF・結合文字・不正UTF-8もバイトとして保持し、hostで正規化や置換をしません。内容のUTF-8検証は生成WATの責務です。

### 名前の準備とopen

begin/chunkの0は全量受理、1は準備失敗です。部分受理・再試行はありません。1では準備を破棄し、生成WATはphase=open、code=open-failed、created=noで停止します。未知の応答はinvalid-responseですが、名前の準備自体はファイルを作成・変更しないためcreated=noです。準備用host領域の不足をCerune文字列の使用量超過へ置き換えません。

共通の名前検証・読取り領域の予約を先に終え、その後にbegin/chunkを呼びます。名前が全長届いた場合だけopenを呼びます。openは成功・失敗にかかわらず準備状態を消費します。次のbeginも前の名前を破棄します。途中失敗後に暗黙open・再送を行いません。

| open status | handle | 意味 |
| --- | --- | --- |
| 0 | 1〜2147483647 | 成功。readは先頭位置、新規保存は空ファイル |
| 1 | 0 | not-found |
| 2 | 0 | already-exists |
| 3 | 0 | unsupported-target |
| 4 | 0 | open-failed |
| その他・上記以外の組合せ | 不定 | invalid-response。write-newのcreated=unknown |

通常のopenエラーは作成・変更なしを保証します。失敗時にhost内部で取得したhandleの回収もhostの責務です。成功handleはinstance内で現在有効な他のhandleと重複させず、OSのfd・ポインタを直接渡しません。生成WATは正しい成功handleだけを使用し、未知のhandleでcloseを試みません。契約外応答で作成有無・消費量を推測しません。

hostは通常ファイルと作業範囲をOS接続側で保証します。名前の文字検査や文字列prefix比較だけを範囲保証に使いません。今回のメモリ上の試験hostはOSパスを開かず、Windows/Linuxの作業範囲の安全性を検証したものではありません。

### 転送・終了

| 応答 | read | write |
| --- | --- | --- |
| 1〜要求数 | そのバイト数を消費。短いreadだけではEOFにならない | prefixを受理。残りだけを続ける |
| 0 | EOF | zero-progressで停止 |
| -1 | read-failedで停止。今回の消費0 | write-failedで停止。今回の受理0 |
| -2 | 今回の消費0の割込み。同じ要求を再試行 | 今回の受理0の割込み。同じ残りを再試行 |
| 要求数超過・その他の負値 | invalid-response、progress=unknown | invalid-response、progress=unknown |

readのcountが0以下のときbitsは使いません。確認済みtransferredだけを残し、契約外応答が示す数を加算しません。hostは一部進行後の失敗を「消費0の-1」として返してはいけません。確認済みprefixを成功応答として返すか、進捗を確定できなければ契約外応答を返します。host例外はこの数値応答の外であり、空内容・EOF・成功へ変換しません。

EOFまたは転送失敗後、取得済みhandleを一度だけcloseします。closeは失敗時もhandleを消費し、再試行しません。先行失敗があれば主診断を保ってcleanup=failed、先行失敗がなければclose-failedです。部分ファイルを削除せず、先行出力も取り消しません。UTF-8検証はEOFとclose成功の後です。

## 転送分割と全経路の意味

C・LLVM・QBE等はreadを最大4096バイト、writeを残り全体として接続へ要求します。WATではprivate memoryを保つため、read/writeのhost要求を最大8バイトへ分割します。これを言語機能や最大ファイル容量の違いにはしません。

| 比較対象 | 一致させるもの |
| --- | --- |
| ソースの1操作 | 引数評価、要求全長、保存・読戻し内容、停止理由と出自、前後の入出力順 |
| 上限 | readは残量以内の要求。ちょうど上限に達した後は別の1バイトでEOFを確認。余分な1バイトは消費済みとして記録 |
| 失敗時 | 確認済みの消費・受理数、進捗が不明か、作成有無、残存内容、closeと主診断の優先順 |
| 接続呼出し | 実際の要求・応答を経路ごとに記録。分割・回数の一致は要求しない |

file-v1のwrite requestedはソース操作の本文全長であり、8へ縮めません。read requestedはEOF確認まではunknown、成功したEOF時に実長へ確定します。max_bytes=0でもreadのEOF確認を行い、空内容のwriteでも新規作成とcloseを行います。文字列の使用量上限やwasm32の物理確保失敗は既存の意味のままです。

差分試験は「第N回呼出しで失敗」だけに依存させず、共通のバイト位置で短い転送・割込み・失敗を注入して確認済みprefixを揃えます。転送単位に依存する契約違反の試験は各ABIで別途行い、同一OS呼出し列・任意の外部変更時の結果一致は主張しません。

## 停止・観測・互換性

生成WATにFileReadText／FileWriteNew、名前・上限検査、転送、所有、NodeId/SourceId/Spanを残します。失敗時は既存のflush_stdout_v1へ先行出力を渡し、write_error_byteでfile-v1を配送してunreachableで停止します。本文・名前・OSエラー文は診断へ含めません。診断配送はbest effortで、二次的な失敗で主診断を変更しません。

成功操作の共通観測APIは今回追加しません。試験hostのeventsはコピーを返す読取り専用記録であり、記録の有無・容量は転送を変更しません。応答注入設定は開始前にコピーする別の試験制御です。

| 組合せ | 動作 |
| --- | --- |
| ファイル操作なし | 新規importを要求しない。既存hostを維持 |
| ファイル操作あり＋旧host | 必須import欠落でinstantiate失敗。成功する代替処理を用意しない |
| 新hostで未設定 | files_status_v1=1。操作時にunconfiguredで停止 |
| std:io／std:env／通常出力併用 | 既存import・起動時の引数snapshot・明示flushの意味は維持 |

WAT libraryは純粋な公開関数を対象とする既存の制限を維持します。WASI、OSファイルhost、汎用ファイルhandle、安定した埋込みAPIはこの変更で導入しません。

## 実装・検証と残件

[生成処理](../../src/codegen/wat/files.rs)は共通IRのFileReadText／FileWriteNewを直接WATへ展開します。既存の文字列allocatorとUTF-8検査を共有し、readはopen前に上限分を予約します。失敗・空内容では解放し、成功時は使用量と長さを実長へ調整して所有を返します。mainの再実行は最初に拒否し、引数やファイルを再読取りしません。

[比較テスト](../../tests/file_io/wat.rs)は独立したメモリ接続で4実行器と出力・file-v1全文・残存内容・close回数を比較します。名前、空・0・ちょうど・+1、4096を超える転送、不正UTF-8、部分転送、割込み、0進行、不正応答、closeとの複合失敗、使用量上限、評価順・短絡・再帰・コピーと反復後の解放を含みます。物理memory.grow失敗は試験用memory上限を指定し、使用量上限の失敗とは別に、open前のallocation-failedを確認します。標準入力・引数・flushとの併用、旧hostでの必須import欠落、未使用関数にもimportが必要なことも検証します。

[試験host](../../tests/support/wat_files.cjs)はバイト転送と応答注入だけを担当し、名前・UTF-8・使用量・file-v1意味処理を代行しません。初期データと応答設定を明示的に渡し、実OSファイルへは接続しません。試験記録のeventsは本文を含まない読取り専用コピー、contentsは期待値照合用の別のコピーです。[ABI試験](../../tests/wat_files_protocol.rs)も維持し、全256バイト値、既知i64ビット列、handle消費、記録の不干渉、host例外を検証します。試験用の転送exportは製品生成物に追加しません。

~~~sh
cargo test --test wat_files_protocol
cargo test --test standard_input file_io::wat
~~~

NodeとWABTが必要です。CIのWindows/Linux/macOS WAT jobではCERUNE_TEST_NODEとCERUNE_TEST_WAT2WASM_JSを指定して必須実行します。[roundtrip.ceruの実行手順](../../examples/files/README.md)からも保存・読戻しを試せます。

実OSのWAT hostは未提供です。作業範囲・排他的作成・リンク拒否を保証する接続を別途検証してから提供します。Nodeの通常パスopenだけで既存のOS契約を満たすとは扱いません。成功操作の生成経路共通観測APIも残件です。
