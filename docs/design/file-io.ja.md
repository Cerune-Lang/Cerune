# 限定的なファイル入出力

[English](file-io.en.md)

**状態：std:fs・共通IR・HIR／MIR／SSA／VMを実装済み。実OS接続・CLIの接続指定・生成経路は未実装。** 2026-10-26に「明示した作業範囲での上限付きUTF-8全文読取り＋新規保存」から進めることを合意しました。既存ファイルの置換は初版に含めません。OSと生成経路の接続方法は提案段階です。[#89](https://github.com/Cerune-Lang/Cerune/issues/89)・[#173](https://github.com/Cerune-Lang/Cerune/issues/173)の保存・再読込みを扱い、比較時の外部作用は[#104](https://github.com/Cerune-Lang/Cerune/issues/104)に従って分離します。

## 実装済みの共通部品

[runtime::files](../../src/runtime/files.rs)の`Files<B>`に、実行前に選んだBackendを渡します。`std:fs`の操作は4実行器からこの部品へ接続します。**実ファイルへの接続はまだありません。** 既存の行入力・出力・引数のAPIは引き続き使えます。

| 項目 | 実装した契約 |
| --- | --- |
| 読取り | `read_text(name, max_bytes, available_bytes, origin)`。上限0〜2147483647、上限分をopen前に予約、最大4096バイトの転送単位、上限後の確認は別の1バイト。EOF後のcloseに成功してからUTF-8を検証 |
| 新規保存 | `write_new(name, text, max_bytes, origin)`。長さを作成前に確認し、空内容でも作成。短い書込みは残りから続け、割込み以外の失敗を再試行しない |
| 接続 | Backendが固定した作業範囲、通常ファイルの検査、排他的作成を担当。返すHandleは先頭位置。通常のopenエラーは作成・変更なしを保証し、不明な応答はInvalidResponse。OSを暗黙に選ばない |
| close | Handleを消費して一度だけ呼ぶ。先行失敗を主診断に保ち、close失敗はcleanup=failed。再試行・削除・再送をしない。hostのpanic等はこの結果契約の対象外 |
| 停止 | 最初のFailureを保持し、後続呼出しでは引数再検査・open・転送・close・追加記録をせず同じFailureを返す。内部の転送結果と停止方針は分離 |
| 所有 | available_bytesは呼出し側が渡す文字列使用量の残量。open前の所有情報準備に対応し、失敗時に破棄、成功時に文字列とともに返す。返却後の寿命・使用量管理は呼出し側。capacityの縮小は未実装 |
| 観測 | `with_trace_limit`、`events()`、`trace_truncated()`、`failure()`。本文・名前・OSエラー文なし。記録なし／上限到達／確保失敗でも実行・転送結果を変えない。into_innerは追加操作なしで接続を返す |

名前の共通規則は、UTF-8で1〜255バイト、直下の1要素です。NUL〜U+001F、U+007F、`< > : " / \ | ? *`、末尾のASCII空白・ピリオド、`.`・`..`を拒否します。先頭のピリオド、内部の空白、日本語・結合文字は保持します。

ASCIIの大文字小文字を区別せず、CON／PRN／AUX／NUL／CONIN$／CONOUT$、COM・LPTに1〜9または¹／²／³を付けた名前を拒否します。最初のピリオドより前の末尾空白を除いて同じ検査を行い、拡張子付きも拒否します。これは検査のための比較であり、受け付けた名前を変換しません。[Windowsの命名規則](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)を参照し、OSに依存しない初版の名前集合を定義しています。全ファイルシステムで作成できる保証や、作業範囲の保証ではありません。

Windowsの新規一時ディレクトリとLinuxの/tmpで、日本語・合成済み／結合文字・補助平面文字・隠し名・空白・255バイト境界の8例を作成・読戻ししました。これはRust標準ライブラリによる名前の試作で、予定するdirectory相対openの実装完了ではありません。

FailureはcodeとDetailsを保持し、Eventは順序番号とDetails・成功／失敗を持ちます。転送数は接続が確認した消費／受理数であり、永続保存量ではありません。readのrequestedはEOFまでunknown、writeでは本文の長さです。不正な転送応答のprogressはunknownとし、transferredはその前までの確認済み数です。openの不正応答で新規作成の有無が不明ならcreated=unknownとします。

```text
file-v1 operation=write-new phase=write code=write-failed limit=3 requested=3 transferred=1 progress=known created=yes cleanup=closed node=7 bytes=10..22
```

phaseはvalidate／reserve／open／read／write／close／decode、createdはno／yes／unknown、cleanupはnot-attempted／closed／failedです。匿名SourceIdではfileを省略し、名前付きソースでは`file=N`を付けます。このRust部品は必ず操作のOriginを受け取り、将来の起動時接続診断は別のorigin=hostとして設計します。

[テスト](../../src/runtime/files/tests.rs)ではメモリ上の独立接続を使い、UTF-8の各位置での分割、4096バイト前後、上限確認時の失敗、close失敗の優先順、既存・部分ファイル、出自と情報非表示、所有準備の破棄・移動、観測の不干渉をWindows/Linuxで確認します。

```sh
cargo test --lib runtime::files
```

## 公開APIと実行器への接続

生成したテキストを新しいファイルに保存し、読み戻した文字列を元の値と比較します。内容のUTF-8・NUL・CR/LF・結合文字をそのまま保持し、正規化・改行変換・BOM除去をしません。

| 公開API | 成功 | 失敗 |
| --- | --- | --- |
| `fs::read_text(name: string, max_bytes: i64) -> string` | 開いたファイルの先頭からEOFまで読む。空ファイルは空文字列 | 欠落、上限超過、不正UTF-8、読取り／close失敗等で停止 |
| `fs::write_new(name: string, text: string, max_bytes: i64) -> void` | 新規作成し、全バイトを渡してcloseする | 同名の対象が既にあれば変更しない。作成／書込み／close失敗で停止 |

`import "std:fs" as fs;`で予約済み仮想標準モジュールを明示します。新しいResult型、公開ファイルhandle、seek、追記、削除、置換、ディレクトリ作成、子プロセスは初版に含めません。異常時停止の公開APIと、進捗を返す内部処理は分離し、将来の回復APIを妨げません。

次は実装済みAPIの例です。[roundtrip.ceru](../../examples/files/roundtrip.ceru)はメモリ接続で4実行器を比較し、真偽値と16バイトの長さを確認します。

```cerune
import "std:fs" as fs;
text: string = "日本語\0\r\né\n";
fs::write_new("sample.txt", text, 4096);
saved: string = fs::read_text("sample.txt", 4096);
print(saved == text);
```

期待値は`true`とLF、および元の文字列と完全一致するファイル内容です。実行ごとに空の保存先を用意し、2回目を同じ保存先で実行した場合は既存ファイルを変えず停止させます。

各実行器のRust APIは次のとおりです。

| API | 接続と結果 |
| --- | --- |
| `run_with_files(program, &mut Files<B>)` | ファイル接続を明示し、stdoutを文字列で返す。stdinと引数は未設定 |
| `run_with_io_files_arguments_and_call_limit` | stdin・stdout・ファイル・引数・呼出し深さを同じ実行へ明示指定 |
| 既存の`run`等 | ファイル接続は未設定。操作時に`file-v1 code=unconfigured`相当の診断で停止。cwd・環境変数から補わない |

HIRの`FileReadText`はstringを返し、`FileWriteNew`は値を返さない文です。MIR／SSA／bytecodeにも`file.read_text.utf8`・`file.write_new.utf8`を残します。引数を左から一度ずつ評価し、操作と標準ソースのNodeId／Spanを保持します。読取り結果はStringHeapへ移し、予約はopen前、失敗時は解放します。ファイル名・保存内容は借用し、操作のための本文コピーを増やしません。生成補助関数は呼出し深さに数えず、`std:fs`のソース関数は数えます。

[経路比較テスト](../../tests/file_io/mod.rs)は独立したメモリ接続でexample、評価順、短絡、所有、標準入出力・引数との組合せ、部分転送・異常停止と出自を比較します。C／LLVM／QBE／WAT／Nativeは未使用関数内の操作も含め、生成前に位置付きの未対応診断を返します。`emit-ir`／`emit-mir`／`emit-bytecode`は操作を実行せず観測できます。CLIからの実ファイル保存はまだできません。

```sh
cargo test --test standard_input file_io
```

## 既存実装から共有するもの

| 部品 | 現在の実装 | ファイル操作での方針 |
| --- | --- | --- |
| 標準宣言・型・出自 | [modules.rs](../../src/modules.rs)、同梱Ceruneソース、SourceMap | `std:fs`にも同じ解決・SourceId規則を使う。import時に対象ファイルを読まない |
| 入力・所有 | [input.rs](../../src/runtime/input.rs)、StringHeap | UTF-8検査と領域予約を共有候補とする。行／EOFの状態は流用しない。空ファイルは成功値 |
| 出力・進捗 | [output.rs](../../src/runtime/output.rs) | 短い書込みと確認済み受理数の規則を揃える。stdoutへ転送する実装にはしない |
| 共通IR・実行 | [HIR](../../src/ir/mod.rs)、[MIR](../../src/mir/mod.rs)、bytecode | read／write-newを明示操作にし、4実行器はそれぞれ元の表現を実行する |
| 生成 | C・LLVM・QBE・WAT・LIR→ASM／Object | 同じ検査・順序・所有・停止を直接loweringする。NativeはCを経由しない |

標準ソース内の操作はそのSourceId/Spanを保持し、利用者の呼出し位置は通常のCallに残します。元HIR・非SSA MIR・SSAと対応表を保存し、ファイル操作を省略・複製・並べ替え・定数化しません。`check`・`compile`・`emit-*`・`observe`はプログラムのファイル操作を実行しません。ソースimportの既存の明示読込みとは区別します。

## 作業範囲と実行接続

作業範囲は実行前にホストが明示して固定します。未設定をカレントディレクトリに置き換えません。相対import、ソースの場所、標準モジュールの探索規則とは独立です。

| 項目 | 提案／実装前に必要な確認 |
| --- | --- |
| ファイル名 | 初版は作業ディレクトリ直下の1要素。空・`.`・`..`・絶対パス・区切り・NUL・Windowsのstream/device名を拒否する。Unicodeは正規化しない。共通の禁止文字・255 UTF-8バイト上限は前述のとおり実装済み |
| 名前の同一性 | 大文字小文字・Unicodeの照合規則はファイルシステムにも依存する。文字列が異なれば別ファイルという保証はしない。同一名衝突は新規作成の応答で判定する |
| 対象 | 通常ファイルだけ。symlink・reparse point・directory・device・FIFOは専用の未対応診断で拒否する。検査のためにFIFO等のopenで待ち続けないことも確認する |
| 解決 | 作業ディレクトリのhandleに基づく相対openを検討する。文字列prefix検査や、存在検査→通常openだけで範囲制限が成立したとしない |
| CLI／埋込み | CLIの`--file-root`案と、Rust APIへの明示接続注入。既存APIは未設定のまま。記録を読むAPIから接続を変更しない |
| 生成実行ファイル | runtimeへの作業範囲の渡し方は未決。既存argvの意味を変えず、生成時の絶対パスや環境変数から暗黙設定しない。Native build/releaseには必要な接続契約の版を記録する案 |
| WAT | 版付きhost接続で名前とバイトを転送する。内部memory・所有ポインタを公開しない。import名・状態値・転送手順は実装前に別途固定する |

Linuxではdirectory fdを用いる`openat`と、新規作成の`O_CREAT | O_EXCL`が候補です。`O_NOFOLLOW`の効果は末尾要素に限られるため、多段パスまで安全と見なしません。[Linux open(2)](https://man7.org/linux/man-pages/man2/open.2.html)

Windowsではユーザーモードの`NtCreateFile`のRootDirectory・新規作成・reparse処理の指定が調査候補です。採用は未確定で、Rust／Clang／自前ObjectからのABI、handleの後始末、再解析点の拒否を試作で確認します。[Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)

作業ディレクトリはOS全体のsandboxではありません。hard link、他プロセスによる変更、ネットワークファイルシステム、OS権限は別の条件です。初版の実OS比較は管理されたローカルの一時ディレクトリで行い、同時に第三者が変更した場合の同一結果を保証しません。必要な範囲制限を満たせない接続は未対応とし、制約を黙って緩めません。

## 共通部品の読取りと新規保存

言語接続では引数を左から一度ずつ評価します。共通部品は受け取った値を上限→名前→接続の順に検査します。上限は0〜2147483647バイトです。0は空内容だけを許可します。OS操作前の失敗と、外部状態が変わった後の失敗を区別します。

| 操作 | 順序と条件 |
| --- | --- |
| read_text | 文字列上限分の使用量・領域を予約してからopenする。EOFまで読み、上限ちょうどなら1バイトを追加要求してEOF／超過を判定する。最大消費は上限+1。ファイルサイズの事前取得だけで成功を決めない |
| 読取り成功 | EOF→close→厳密UTF-8検査の順。全て成功した場合に所有文字列を返し、使用量を実長へ縮める。読取りごとに新しくopenし、過去のEOFを別操作へ持ち越さない |
| write_new | 内容のバイト数が上限以下であることを作成前に検査する。排他的な新規作成→順次書込み→close。空文字列でも新規ファイルを作る |
| 部分書込み | 正の受理数は残りから続ける。消費0の割込みだけ再試行。0進行・要求超過の数・書込み失敗は別コードで停止する |
| 後始末 | open成功後は成功／失敗ともcloseを一度行う。closeを盲目的に再試行しない。先行失敗があれば主診断を維持し、close失敗は付帯結果にする |
| 失敗後 | 後続のソース操作、再open、書込みの再送、暗黙の削除はしない。作成済みファイルと受理済み内容が残り得る。巻戻し・原子的公開を保証しない |

内部接続は短い読取りをEOFと扱わず、0バイトをEOFとして返します。読取りの割込みは消費0の場合だけ再試行します。上限超過・読取り失敗・close失敗はUTF-8検査より先に決まります。不正な応答では、その応答の消費／受理数を推測せず不明と記録します。

成功は全バイトの受理とcloseの成功を意味し、障害後の永続保存を保証しません。closeの再試行と永続化は別問題です。[Linux close(2)](https://man7.org/linux/man-pages/man2/close.2.html) `flush();`は引き続き標準出力だけを対象とします。

読取り上限・文字列ヒープの使用量・物理確保失敗を分けます。既存文字列ヒープ同様、使用量は物理capacityの保証ではありません。書込みのmax_bytesは1回の量を制限し、総ディスク使用量や実行時間を保証しません。同期OS操作の時間上限と回復は後続設計であり、比較テストは外側のprocess timeoutで管理します。

## 失敗と観測

Rust共通部品に`file-v1`の診断形式を実装しました。既存io-v1のwrite／flush／read-lineは変更しません。

| 記録項目 | 意味 |
| --- | --- |
| operation / phase / code | read-text／write-new、検査・open・read/write・close、失敗理由を区別 |
| limit / requested / transferred | 指定上限、論理的な要求量、確認できた消費／受理数。読取りの全長はEOFまで未知であり、上限を既知の全長と表示しない |
| progress / created / cleanup | 進捗数が完全か不明か、新規作成の有無、closeの結果。契約外応答は作成状態も不明になり得る |
| NodeId / SourceId / Span | 元の共通IR操作の出自。起動時の接続設定失敗はorigin=hostとして区別 |

未設定・不正名・不正上限・予算超過・確保失敗・欠落・既存対象・未対応対象・open/read/write/close失敗・0進行・契約外応答・不正UTF-8・上限超過を区別します。コード集合と検査優先順は共通部品の異常系テストで固定しています。既存対象の作成失敗は、その実体を追跡して種類を判定せず、変更なしで報告します。

診断・通常の記録にファイル内容、絶対パス、OSエラー本文を入れません。順序番号と出自から操作を追い、本文比較には実行前に用意した専用接続・保存先を使います。観測は不変記録の取得だけとし、入力差替え・再実行・接続変更を行いません。記録なし／上限到達／記録用確保失敗でも実行結果を変えません。全生成経路の成功操作の共通記録は既存I/Oでも未実装であり、今回の計画と実装済み機能を混同しません。

## 実装単位と検証の完了条件

| 順番 | 作業 | 次へ進む条件 |
| --- | --- | --- |
| 1 | この設計と、純粋な名前検査・進捗／失敗契約 | API・名前の範囲・異常停止・診断fixtureを確定。OS接続を必須にしない |
| 2 | 共通runtime部品、std:fs、HIR→MIR／SSA／bytecode、4実行器 | 独立したメモリ上の接続で結果・作用順・失敗・所有・出自を比較。既存APIは未設定。動くexampleを追加 |
| 3 | Windows/Linuxの実接続とCLI | directory相対open、通常ファイル検査、同名競合・リンク・Unicode・close失敗の扱いを検証。元の入力・出力契約を維持 |
| 4 | C・LLVM・QBE・WAT・Native | 起動時の接続指定とWAT ABIを先に確定し、各経路に同じ検査・停止を追加。未対応を成功に偽装しない |
| 5 | Native build/release・全経路のexample | 元HIR／非SSA MIR／SSA／LIR／生成物と出自を確認し、完成状態をロードマップへ反映 |

必須ケースは、空内容・日本語・NUL・CR/LF・結合文字・上限ちょうど／+1・0上限・不正UTF-8・短いI/O・割込み・0進行・契約外応答・全バイト受理後のclose失敗・作成後の部分失敗です。名前と接続の検証順、未設定、同名既存ファイルの不変性、予算不足で外部操作0回、所有値のコピー後の独立性、早期return、再帰、短絡も含めます。

比較は同じ初期内容・応答列を持つ独立接続／ディレクトリで行い、stdoutだけでなく要求順・転送数・停止理由・残ったファイルを照合します。同じ実保存先で全経路を順次実行しません。OS接続試験と意味論の注入試験を分け、独立したRust／Python等の期待値計算を維持します。汎用record/replayやeffect systemは前提にしません。

1と2を実装しました。次は3のWindows/Linux実接続とCLIです。実OSのpath解決方式と生成実行ファイルの接続指定は未決のまま実装へ流さず、各段階の開始前に設計・試作結果を明示します。
