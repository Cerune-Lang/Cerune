# 限定的なファイル入出力

[English](file-io.en.md)

**状態：std:fs・共通IR・HIR／MIR／SSA／VMを実装済み。Windows/Linuxの実OS接続とCLIの接続指定も実装済み。生成Cも実装済み。LLVMは意味処理の試験段階で、実接続・公開生成は未対応。QBE／WAT／Nativeは未実装。** 2026-10-26に「明示した作業範囲での上限付きUTF-8全文読取り＋新規保存」から進めることを合意しました。既存ファイルの置換は初版に含めません。生成実行ファイルの起動形式は合意済みです。WATの接続方法は提案段階です。[#89](https://github.com/Cerune-Lang/Cerune/issues/89)・[#173](https://github.com/Cerune-Lang/Cerune/issues/173)の保存・再読込みを扱い、比較時の外部作用は[#104](https://github.com/Cerune-Lang/Cerune/issues/104)に従って分離します。

## 実装済みの共通部品

[runtime::files](../../src/runtime/files.rs)の`Files<B>`に、実行前に選んだBackendを渡します。`std:fs`の操作は4実行器からこの部品へ接続します。Windows/Linuxでは`Directory::open(&Path)`で選んだ作業ディレクトリを接続できます。 既存の行入力・出力・引数のAPIは引き続き使えます。

| 項目 | 実装した契約 |
| --- | --- |
| 読取り | `read_text(name, max_bytes, available_bytes, origin)`。上限0〜2147483647、上限分をopen前に予約、最大4096バイトの転送単位、上限後の確認は別の1バイト。EOF後のcloseに成功してからUTF-8を検証 |
| 新規保存 | `write_new(name, text, max_bytes, origin)`。長さを作成前に確認し、空内容でも作成。短い書込みは残りから続け、割込み以外の失敗を再試行しない |
| 接続 | Backendが固定した作業範囲、通常ファイルの検査、排他的作成を担当。返すHandleは先頭位置。通常のopenエラーは作成・変更なしを保証し、不明な応答はInvalidResponse。実行時のOS接続を明示して渡す。生成物のtargetをホストOSから暗黙に選ばない |
| close | Handleを消費して一度だけ呼ぶ。先行失敗を主診断に保ち、close失敗はcleanup=failed。再試行・削除・再送をしない。hostのpanic等はこの結果契約の対象外 |
| 停止 | 最初のFailureを保持し、後続呼出しでは引数再検査・open・転送・close・追加記録をせず同じFailureを返す。内部の転送結果と停止方針は分離 |
| 所有 | available_bytesは呼出し側が渡す文字列使用量の残量。open前の所有情報準備に対応し、失敗時に破棄、成功時に文字列とともに返す。返却後の寿命・使用量管理は呼出し側。capacityの縮小は未実装 |
| 観測 | `with_trace_limit`、`events()`、`trace_truncated()`、`failure()`。本文・名前・OSエラー文なし。記録なし／上限到達／確保失敗でも実行・転送結果を変えない。into_innerは追加操作なしで接続を返す |

名前の共通規則は、UTF-8で1〜255バイト、直下の1要素です。NUL〜U+001F、U+007F、`< > : " / \ | ? *`、末尾のASCII空白・ピリオド、`.`・`..`を拒否します。先頭のピリオド、内部の空白、日本語・結合文字は保持します。

ASCIIの大文字小文字を区別せず、CON／PRN／AUX／NUL／CONIN$／CONOUT$、COM・LPTに1〜9または¹／²／³を付けた名前を拒否します。最初のピリオドより前の末尾空白を除いて同じ検査を行い、拡張子付きも拒否します。これは検査のための比較であり、受け付けた名前を変換しません。[Windowsの命名規則](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)を参照し、OSに依存しない初版の名前集合を定義しています。全ファイルシステムで作成できる保証や、作業範囲の保証ではありません。

Windowsの新規一時ディレクトリとLinuxの/tmpで、日本語・合成済み／結合文字・補助平面文字・隠し名・空白・255バイト境界の8例を作成・読戻ししました。これは名前集合の初期試作です。現在のdirectory相対openの検証は後述します。

FailureはcodeとDetailsを保持し、Eventは順序番号とDetails・成功／失敗を持ちます。転送数は接続が確認した消費／受理数であり、永続保存量ではありません。readのrequestedはEOFまでunknown、writeでは本文の長さです。不正な転送応答のprogressはunknownとし、transferredはその前までの確認済み数です。openの不正応答で新規作成の有無が不明ならcreated=unknownとします。

```text
file-v1 operation=write-new phase=write code=write-failed limit=3 requested=3 transferred=1 progress=known created=yes cleanup=closed node=7 bytes=10..22
```

phaseはvalidate／reserve／open／read／write／close／decode、createdはno／yes／unknown、cleanupはnot-attempted／closed／failedです。匿名SourceIdではfileを省略し、名前付きソースでは`file=N`を付けます。このRust部品は必ず操作のOriginを受け取り、起動時の接続診断は`origin=host`で区別します。

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

[経路比較テスト](../../tests/file_io/mod.rs)は独立したメモリ接続でexample、評価順、短絡、所有、標準入出力・引数との組合せ、部分転送・異常停止と出自を比較します。生成CはWindows/Linuxの読取り・新規保存に対応します。LLVM／QBE／WAT／Nativeは未使用関数内の操作も含め、生成前に位置付きの未対応診断を返します。`emit-ir`／`emit-mir`／`emit-bytecode`は操作を実行せず観測できます。CLIの`--file-root <directory>`で実ファイルを接続できます。

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

| 項目 | 現在の契約と後続の設計 |
| --- | --- |
| ファイル名 | 初版は作業ディレクトリ直下の1要素。空・`.`・`..`・絶対パス・区切り・NUL・Windowsのstream/device名を拒否する。Unicodeは正規化しない。共通の禁止文字・255 UTF-8バイト上限は前述のとおり実装済み |
| 名前の同一性 | 大文字小文字・Unicodeの照合規則はファイルシステムにも依存する。文字列が異なれば別ファイルという保証はしない。同一名衝突は新規作成の応答で判定する |
| 対象 | 通常ファイルだけ。symlink・reparse point・directory・device・FIFOは専用の未対応診断で拒否する。検査のためにFIFO等のopenで待ち続けないことも確認する |
| 解決 | 作業ディレクトリのhandleに基づく相対openを実装した。文字列prefix検査や、存在検査→通常openだけで範囲制限が成立したとしない |
| CLI／埋込み | CLIの`--file-root`と、Rust APIへの明示接続注入を実装。既存APIは未設定のまま。記録を読むAPIから接続を変更しない |
| 生成実行ファイル | 先頭マーカー付きの起動設定を合意し、生成Cへ実装した。マーカーなしのargvを維持し、生成時の絶対パスや環境変数から暗黙設定しない。Native build/releaseには必要な接続契約の版を記録する案 |
| WAT | 版付きhost接続で名前とバイトを転送する。内部memory・所有ポインタを公開しない。import名・状態値・転送手順は実装前に別途固定する |

Linuxではdirectory fdを用いる`openat`と、新規作成の`O_CREAT | O_EXCL`を採用しました。`O_NOFOLLOW`の効果は末尾要素に限られるため、多段パスまで安全と見なしません。[Linux open(2)](https://man7.org/linux/man-pages/man2/open.2.html)

Windowsではユーザーモードの`NtCreateFile`のRootDirectory・排他的作成・reparseを追わない指定をRust接続で採用しました。生成CのClangからも利用します。LLVM／QBE／自前Objectへの接続は後続です。[Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)

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

1〜3と4の生成Cを実装しました。LLVMの意味処理と試験接続での比較も追加しました。次はLLVMの実接続・起動契約、その後QBE／NativeとWAT契約の具体化です。各段階の開始前に設計・試作結果を明示します。

## Windows/Linux接続とCLI

実装は[Directory](../../src/runtime/files/directory.rs)にあります。実行前に開いたディレクトリを保持し、後から元のパスが移動・置換されても接続先を変えません。rootの最終要素のリンク／reparse pointを拒否します。明示したrootの親要素の解決はOSに従い、親全体のリンク禁止やOS全体の隔離は保証しません。

| OS | 読取り | 新規保存 |
| --- | --- | --- |
| Linux | O_PATH・O_NOFOLLOW・O_CLOEXECで対象を固定し、通常ファイルと確認後、保持中のfdを/proc/self/fd経由で読取り用に開く。inodeとdeviceの同一性も検査 | openatのO_CREAT・O_EXCL・O_NOFOLLOW。modeは0666をumaskで制限。既存対象には書き込まない |
| Windows | NtCreateFileのRootDirectoryとFILE_OPEN_REPARSE_POINTを使い、非directory・disk handle・非reparseを確認 | FILE_CREATEで排他的に作成。共有はread/write/deleteを許可し、既定のセキュリティ属性を継承 |

Linuxの読取りには利用可能なprocfsが必要です。再openに失敗しても元の名前を通常openする代替処理はありません。通常ファイル確認前にFIFOやdeviceをデータ入出力用に開きません。[proc_pid_fd(5)](https://man7.org/linux/man-pages/man5/proc_pid_fd.5.html) Windowsの属性確認は[GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex)を使います。closeは所有権を取り出して一度だけ呼び、失敗でも再試行しません。root用handleは接続の破棄時に解放します。

CLIの指定例は次のとおりです。sample-outputは事前に用意したディレクトリです。

~~~sh
cerune run examples/files/roundtrip.ceru --file-root sample-output
cerune run-mir examples/files/roundtrip.ceru --ssa --file-root another-empty-output
~~~

run/run-ir/run-mir（SSAを含む）/run-vmが対応します。相対rootは接続時の作業ディレクトリから解決し、その後はhandleを使います。省略時は未設定です。重複・値なしは起動前に拒否します。引数区切りの後に置いた--file-rootはプログラム引数であり、接続を設定しません。emit/check/build/releaseにはこのオプションを追加しません。

rootの不在・不正指定・未対応対象・open失敗は、操作のNodeIdを作らず、実行前にfile-v1 operation=connect phase=open-root code=... origin=hostで停止します。ソース操作の失敗には従来のNodeId/Spanを残します。診断にrootの絶対パスやOSエラー本文を追加しません。Windows/Linux以外の接続はunsupported-platformです。

[OS接続テスト](../../tests/file_io/os.rs)は独立した一時ディレクトリで、exampleの16バイト、空／上限ちょうど／上限+1、不正UTF-8、出力後の停止と出自を4実行器で比較します。同名作成の競合、rootの移動・置換、directory・link・Windows junction・Linux FIFOの拒否も検証します。子プロセスには20秒の上限を設けます。部分転送・割込み・close失敗の注入は共通部品のテストで検証し、実OSでclose失敗を発生させた検証とは区別します。Linuxの特殊ファイル試験はLinuxの一時領域を使います。WSLのWindowsドライブではFIFO作成を利用できないためです。


## 生成実行ファイルの起動契約（C実装済み）

2026-10-27に、先頭の専用マーカーがある場合だけランタイム設定を解釈する方式を合意しました。

~~~sh
program --cerune-runtime-v1 --file-root output -- user-argument
program --cerune-runtime-v1 -- --cerune-runtime-v1
~~~

- マーカーなしでは全引数を従来どおりプログラムへ渡し、ファイル接続は未設定にします。
- マーカーありでは終端の--が必須です。その前は--file-rootと値だけを受け付け、重複・欠落・空値・未知の設定を起動時に拒否します。root省略は未設定を表します。
- --以降は空文字列・マーカー・オプション風文字列もプログラム引数として保持します。設定部分は引数snapshotの個数・バイト数へ含めません。
- 設定を全て検証してからrootを接続します。rootの存在確認や範囲制限は引数解析の責務ではなく、既存のOS接続の責務です。cwdや環境変数から接続を補いません。
- 初版ではファイル操作を含む生成物がこの起動契約を持ちます。ファイル操作を含まない既存生成物の起動・argvの意味は変更しません。

生成Cへ接続し、Windows/Linux、最適化なし／あり、独立した作業ディレクトリで4実行器と比較しました。LLVM・QBE・Nativeは同じ起動契約へ順次接続します。WATはargvの専用マーカーではなく、別途定義する版付きhost契約で接続を明示します。C以外の生成経路の実装完了を意味しません。


## 生成Cの検査と比較

[生成Cのruntime](../../src/codegen/c/files.rs)に名前・上限・予約・転送・close・UTF-8検査を展開し、[OS接続](../../src/codegen/c/files/connection.rs)と[起動解析](../../src/codegen/c/files/entry.rs)を分離しています。起動設定→プログラム引数の検証とsnapshot→root接続→ソース実行の順です。設定エラーはfile-v1のphase=configure、root接続エラーはphase=open-rootで、いずれもorigin=hostです。引数snapshotの失敗は従来のarguments-v1を使います。

設定エラーはunknown-setting・duplicate-setting・invalid-root・missing-delimiterに分かれます。起動設定を省略したソース操作はunconfiguredです。Cコンパイル対象の_WIN32／__linux__で接続を選び、生成器を動かしたOSからtargetを推定しません。他OSの明示接続はunsupported-platformです。Linuxの読取りはRust接続同様に/proc/self/fdを必要とします。

読取りは上限分をopen前に予約し、返却値を既存の文字列所有管理へ移します。書込み引数は順番を持つ局所値へ展開し、Cの引数評価順に任せません。各操作に元のNodeId/Spanを残します。出力済みの内容をflushして診断し、後続のソース操作を実行せず終了します。観測用の生成操作ではファイルを開きません。

[比較テスト](../../tests/file_io/c.rs)はO0/O2でexample、UTF-8と上限、既存ファイル、名前、使用量上限、未設定、起動設定、プログラム引数、評価順・短絡・再帰・コピー後の独立性を確認します。独立した接続へ短い転送・割込み・0進行・不正受理数・転送失敗・close失敗を注入し、4実行器とfile-v1全体・先行出力・残存内容を比較します。WindowsのjunctionとLinuxのlink/FIFOも拒否します。LinuxではAddressSanitizer/UndefinedBehaviorSanitizerでも比較します。Cに成功操作の共通観測APIを追加したものではありません。

## LLVMの意味処理と試験接続

LLVMへのlowering・名前検査・転送・所有管理・file-v1診断を実装しました。[比較テスト](../../src/codegen/llvm/files/tests.rs)では、既存のroundtrip.ceruをLLVMへ直接展開し、独立した試験接続で4実行器と比較します。C生成物やVMを経由して実行する方式ではありません。

**公開LLVM生成は引き続き未対応診断を返します。** 今回はOS非依存の意味処理を検証する段階です。Windows/Linuxの実接続と、合意済みの起動マーカー・引数snapshotへの接続は次の変更で実装します。試験接続を利用者向けAPIや完成したファイル接続として公開しません。

内部の接続関数は次の責務だけを持ちます。これは安定した外部ABIの宣言ではありません。

| 接続関数 | 応答と責務 |
| --- | --- |
| configured | 接続の有無。省略時にcwdや環境変数から補わない |
| open | 名前のUTF-8バイト列・長さ・read/write指定を受ける。0は成功、1〜4は欠落・既存・未対応対象・open失敗、それ以外は契約外応答。成功時だけ所有handleを返す |
| read / write | 要求バイト数に対する転送数を返す。-2は未消費の割込み、その他の負値は失敗。要求を超える応答は進捗不明。再試行と停止の判断は生成LLVMが行う |
| close | handleを一度だけ消費する。0は成功、それ以外は失敗。先行失敗の主診断を変更しない |

読取りは4096バイト以下で要求し、上限到達後の1バイト確認を独立した領域へ行います。書込みは残り全体を要求します。EOF前のrequestedはunknownであり、UTF-8検査はclose成功後です。読取り結果は既存のLLVM文字列所有管理へ移し、コピー・再帰・反復後に解放します。引数は左から一度ずつ評価し、NodeId/SourceId/Spanを保持します。

比較対象は通常の出力だけでなく、要求列・close回数・残存内容・停止前出力・file-v1全文です。短い転送、割込み、0進行、契約外応答、転送とcloseの複合失敗、空内容、上限ちょうど／+1、4096バイトをまたぐ転送、不正UTF-8、未設定、名前検査、使用量上限を含みます。LLVM内の確保失敗も注入し、open前に停止することを確認します。O0/O2、両ターゲットのObject生成とWindows/Linuxでの実行を検証します。成功後は文字列の使用量と所有者リストが空であることも試験します。

試験用Cコードは固定データの転送・応答列・試験ログだけを担当します。ファイル名の意味検査、UTF-8検査、予算、診断、再試行を代行しません。これは実OSのdirectory相対openやリンク拒否の検証ではなく、それらは後続の実接続試験で確認します。
