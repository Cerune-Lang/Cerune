# 利用例から進める引数と数値の読取り

[English](dogfooding-inputs.en.md)

2026-10-24。[#173](https://github.com/Cerune-Lang/Cerune/issues/173)と[#89](https://github.com/Cerune-Lang/Cerune/issues/89)の次の設計案です。**引数取得はHIR・MIR・SSA・VM・CLI・生成C・LLVM・QBE・WAT・Nativeに対応済み。整数読取りも[std:num](integer-parsing.ja.md)として実装済み**です。実装済みの標準入力・WAT関数公開を作り直しません。

## 最小の用途

同じseedと件数から、同じ整数列を得ます。まずseedを引数から受け取る例を目標にし、ファイル保存・子プロセス起動は別段階にします。

[deterministic_seed.ceru](../../examples/deterministic_seed.ceru)は現在動く基準例です。seedはソースに固定し、0・1・1337・2147483646と0回の反復を確認します。次の計算はOSの乱数を使わず、暗号用途ではありません。

```text
next = (state * 48271) % 2147483647
0 <= state <= 2147483646
```

積の最大値103661183076066はi64に収まります。既知の期待値とHIR/MIR/SSA/VMを照合し、読取り専用のIR表示を挟んでも同じ結果を得ます。Parallaxの生成器・出力プロトコルを複製する例ではなく、引数対応の前後で計算を比較するための小さな例です。

## 実装前の基準

master 200c1ecでは、`cerune run examples/deterministic_seed.ceru -- 1337`はusage診断で終了1になります。`import "std:env" as env;`だけのソースもunknown standard moduleで終了1です。これは実装前の履歴です。現在は下記の公開APIとCLIで引数取得に対応しています。

## Parallaxの担当処理を分ける

確認対象は[Parallax 356f357](https://github.com/Cerune-Lang/Parallax/tree/356f357cb2bf347ef8ec7462e6bdd77410a6e7d7)です。

| 処理 | 現在の担当 | Cerune側で進めること |
| --- | --- | --- |
| 固定seedから値・操作を生成 | .ceruでも実装済み | 固定値をプログラム引数へ置き換える |
| 期待値計算 | 独立したPython参照実装 | 維持する。Cerune側だけで正しさを判定しない |
| 型付きソース構築・縮小 | Python。Ceruneは生成条件の一部を出力 | 現行の固定長・有限構造で可能な範囲を試し、必要な文字列操作・再帰等を分割 |
| 引数の数値化・プロトコル検査 | Python | 明示した文字列→整数の規則を設計 |
| 保存・再読込み | Pythonと一時ディレクトリ | 限定的なファイルI/Oを後続設計 |
| 実行・回収・比較 | Pythonのsubprocess、timeout、stdout/stderr/終了状態 | 子プロセス操作は最後に別設計。観測から実行しない |

既存のPython乱数生成器とCeruneのLCGは異なるアルゴリズムです。同じseedだけで双方の生成列が一致すると約束しません。生成器・版・seed・件数・対象Ceruneコミットを再現条件として記録します。

引数取得は[#174](https://github.com/Cerune-Lang/Cerune/issues/174)、整数の読取りは[#175](https://github.com/Cerune-Lang/Cerune/issues/175)で個別に追跡します。

## 引数取得の共通契約

`std:env`という予約済み仮想モジュールに、引数数と指定位置の引数を読む操作を置く案です。環境変数やファイル探索の追加は含めません。公開名はarg_count／arg／Argumentに確定しました。生成経路への接続も実装済みです。

| 項目 | 提案する契約 |
| --- | --- |
| CLI | `cerune run source.ceru [Cerune側オプション] -- [プログラム引数...]`。run-ir/run-mir/--ssa/run-vmも同じ分離 |
| 分割順 | 最初の区切りを全オプション解析より先に分離。以後の`--ssa`・`--string-heap-limit`等もプログラムの値 |
| 対象 | プログラムの引数だけ。実行ファイル名、ソース名、コンパイラオプションは含めない |
| 空と欠落 | 空文字列は存在する引数。範囲外は既存enumによるMissing等で表現。負の添字は不正として診断 |
| 内容 | UTF-8、順序、空白、CR/LFを保持。正規化・分割・quoteの再解釈なし。OSで表せないNULは全接続共通で拒否 |
| 接続 | 実行開始前に固定した引数列を渡す。呼出すたびにOSや外部の可変配列を読まない |
| 埋込み | ホストが明示注入。未設定と設定済み空列を区別。既存APIは暗黙にホストプロセスの引数を読まない |
| 所有・上限 | 引数列の件数・総UTF-8バイト数を制限。返した文字列は既存の不変内容・所有・文字列ヒープ上限に従う |
| 観測 | 共通IRの引数操作、型、NodeId/Span、取得位置と結果種別を追跡。本文は既定の診断や観測bundleへ入れない |

CLIは`env::args_os()`を厳密変換し、区切り以後を`parse_heap_limits`へ渡さない構造に変更しました。不正Unicodeはpanicにせず診断します。起動時の不正入力には存在しないNodeId/Spanを付けず、ソース内の操作に起因する停止と区別します。

`compile(&str)`やemit/observeは引数を取得せず、同じIRを構築します。標準宣言と本文は通常のSourceMapに登録し、std:ioと同じ名前解決・型同一性・出自の仕組みを使います。別のfrontend意味論は作りません。

## 数値の読取りを別の純粋操作にする

文字列からi64を読む`std:num::parse_i64`を実装しました。seedだけの特別処理や、既存の数値型間の`convert<T>`への暗黙追加にはしません。

| 入力 | 結果 |
| --- | --- |
| ASCIIの`-?[0-9]+` | i64の値。先頭ゼロを許可、`-0`は整数0 |
| 空、符号だけ、`+1`、空白、改行、全角数字、小数、NUL、末尾の文字 | 不正な書式 |
| 文法に合うがi64の上下限を超える | 範囲外 |
| 結果型 | 既存enumの成功／不正書式／範囲外。汎用Resultを前提にしない |
| 判定順 | 全体の書式を確認してから範囲を判定。`巨大な桁列+x`を単なる範囲外にしない |

整数への読取りと、seedの用途上の範囲検査は分けます。i64として正しくても負値や2147483647以上のseedは例側が拒否します。書式の誤りを値で返すことと、確保失敗などの実行時停止も区別します。入力文字列は消費・変更しません。

一般の文字列添字APIは公開していません。内部の純粋なバイト参照を共通IRに追加し、書式と範囲の検査を同梱Ceruneソースで共有します。[確定したAPIと内部操作](integer-parsing.ja.md)を参照してください。

## 実装単位と検証

1. 実装済み：公開宣言・snapshot・共通IRをHIR/MIR/SSA/VMとCLIへ接続。引数を表示するexampleと比較テストを追加。
2. 実装済み：生成C・LLVM・QBE・WAT・Nativeへ接続し、同じ引数列で比較。Nativeは通常MIR／SSA経由のASM・自前Objectを含む。
3. 実装済み：純粋な整数読取りを全経路へ追加し、seedと回数を引数から受け取る例を追加。
4. 引数・整数読取りの完了後は、[本体ロードマップ](language-roadmap.ja.md)に従ってoptional let＋fmt、Native build・配布初版、再帰へ進む。その後、明示した作業範囲でのファイルI/Oとshellを経由しない子プロセス操作を個別設計する。

各段階で元HIR・非SSA MIR・SSA・生成物・ソース対応を維持します。最適化・Native build・パッケージ管理は前提にしません。

CのWindows起動・UTF-16変換・上限設定は下記で確定しました。Nativeの起動接続と上限設定も実装済みです。WATの版付きhost契約は実装済みです。WATの内部memoryを引数転送のために公開しません。C/LLVM/QBE/Nativeでもターゲット固有の入力方法を明示し、生成実行中のOSから黙って選びません。

回帰テストは引数0件・空文字・Unicode・CR/LF・順序・区切り以後のオプション風文字列・不正Unicode・上限ちょうど／超過・未設定接続を含めます。読取りではi64両端・その外側・書式と範囲の競合を確認し、失敗後の停止・本文非表示・観測有無での一致も比較します。

## 引数列の保持・検証：実装済みの最初の部品

`runtime::arguments::Arguments`を追加しました。この不変な保持部品を、下記の公開API・共通IR・CLI・4実行器へ接続しています。

| 項目 | 実装した契約 |
| --- | --- |
| 注入 | 区切り済みバイト列と`Limits { max_count, max_bytes }`を明示して`capture`。OS・環境変数・ファイルは読まない |
| 内容 | 厳密なUTF-8。空文字、日本語、空白、CR/LF、結合文字、順序を保存。再分割・正規化しない。NULは拒否 |
| 件数 | 0件の設定済み列を許可。`max_count`は0〜i64::MAX。返す件数・添字はi64 |
| バイト上限 | 全引数のUTF-8内容の合計。空文字は0バイトだが1件。CR/LFも含む。終端文字やallocator管理領域は含めない |
| 検証順 | 上限の妥当性→総件数→各引数を順に、累積バイト上限→UTF-8→NUL。複数の違反があればこの順で一つを返す |
| 確保 | 全件検証後に、引数表と各文字列を失敗可能な確保でコピー。失敗時に部分的な列を返さない。ホスト側の元入力の確保は接続側の責務 |
| 所有 | コピー後にホストが元入力を変更・破棄しても影響しない。変更APIや無条件のCloneは持たない |
| 読取り | 未設定は`Unconfigured`。設定済みの負添字は`InvalidIndex`、範囲外は`None`、存在する空文字は`Some("")`。未設定判定を先に行う |
| 観測 | 件数・使用バイト数・Debugは本文を含まない。読取りは確保・消費・OS接続を行わない。本文は明示した`get`だけで返す |
| 起動準備の失敗 | `arguments-v1 phase=capture`に理由と該当する引数位置だけを記録。入力本文や存在しないNodeId/Spanを付けない |

引数列の保存容量と、実行中に返す文字列のヒープ上限は別です。CLI上限、文字列ヒープへのコピー、NodeId/Span付き診断は下記の接続で実装しました。成功した操作の時系列記録は後続です。初版の検証コードは、件数超過・バイト上限超過・不正UTF-8・NUL・確保失敗を区別します。

異常系テストは検証順、上限ちょうど／超過、引数表と各文字列の確保失敗を確認します。成功系はUnicodeの非正規化、元入力からの独立性、空／欠落／未設定、繰返し読取りとDebugによる非干渉を確認します。公開標準宣言と共通IRから、この引数列を実行器へ明示注入します。

## 公開API・CLI・4実行器の接続

`import "std:env" as env;`を明示し、`env::arg_count() -> i64`と`env::arg(index: i64) -> env::Argument`を使います。`Argument::Value { text: string }`と`Argument::Missing {}`は既存enumです。空文字はValue、0以上の範囲外はMissing、負添字は停止です。型と仮想ソースの出自はstd:ioと同じ仕組みで解決します。

| 項目 | 実装した契約 |
| --- | --- |
| 経路 | HIR・MIR・SSA・VM。CLIはrun／run-ir／run-mir／run-mir --ssa／run-vm |
| CLI区切り | 最初の`--`より前だけをCeruneオプションとして解析。以後の空文字・`--ssa`・`--string-heap-limit`等はそのままプログラム引数 |
| CLI上限 | 既定4096件・合計1048576バイト。`--argument-count-limit`／`--argument-byte-limit`で明示変更。区切りの前に指定。0を許可し、重複・不正数値は拒否 |
| 起動処理 | Rustの`args_os`で得た既に区切られたOS文字列を厳密変換。再分割やquote再解釈をしない。不正Unicodeは本文を含めず起動準備の診断にする |
| 引数なし | CLIでは設定済み0件。既存の埋込み実行APIは未設定のまま。compile・emitは引数を取得しない |
| Rust API | 各実行器の`run_with_arguments`／`run_with_io_and_arguments`へ不変のArgumentsを明示注入。VMにも同じAPIがあり、libには`run_bytecode_with_io_and_arguments`を追加 |
| 共通表現 | HIRのArgumentCount／ArgumentGet、MIR・SSA・bytecodeのarguments.count／arguments.get。Missingへの分岐とenum構築は同梱Cerune本文から通常の展開を行う |
| 所有 | 取得した非空文字列を既存ヒープへ確保・コピーし、通常の所有展開で解放。空文字は0バイト。snapshotの上限と返却値のヒープ上限を分離 |
| 実行時失敗 | arguments-v1 phase=executeで操作・理由・添字・NodeId/SourceId/Spanを記録して停止。未設定、負添字、ヒープ上限超過、確保失敗を区別。本文は含めない |
| 出自 | 内部操作の位置はstd:env内。利用者の呼出し位置はHIR Callに保持。架空の利用者Spanに置き換えない |

`--diagnostic-format runtime-v1`でも引数操作の失敗はarguments-v1として出します。起動準備の失敗にはソース操作の位置を付けません。CLIのOS文字列取得・一時変換の領域はsnapshot保存の上限とは別です。

[実行例](../../examples/arguments/README.md)と[比較テスト](../../tests/program_arguments/mod.rs)で正常出力、短絡評価、関数・配列への受渡し、コピー後の独立性、停止前の出力、停止理由・出自、元IRが変わらないことを確認します。

Nativeも対応し、引数操作を含むLIR・ASM・自前Objectとobserve bundleを生成できます。元HIR・非SSA MIR・SSA・出自を維持します。成功した各取得の時系列イベント記録は未実装です。純粋な整数読取りは[std:num](integer-parsing.ja.md)に実装しました。

## 生成Cの引数接続

同じ共通IR操作からCの引数取得・診断・所有処理を生成します。Cソースを作る時点ではOS引数を読みません。

| 項目 | 実装した契約 |
| --- | --- |
| 起動 | Cコンパイル対象の`_WIN32`で選択。Windowsは`wmain`のUTF-16引数を`WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS)`で厳密変換。他OSは`main`の引数をUTF-8検査。生成器を動かすOSには依存しない |
| 内容 | CRTが分割した引数から実行ファイル名だけを除外。再分割・正規化なし。生成した実行ファイルは全引数を値として扱うため、Cerune CLI用の区切り`--`は不要 |
| 起動時検証 | OS文字列のUnicode検査後、共通snapshotと同じ上限・件数・累積バイト数・UTF-8・NULの順で検証し、全件コピーしてから実行。不正OS文字列は`invalid-unicode`。元入力・一時変換領域はsnapshot上限とは別 |
| 上限 | Cコンパイル時の`CERUNE_ARGUMENT_COUNT_LIMIT`と`CERUNE_ARGUMENT_BYTE_LIMIT`で指定。既定4096件・1048576バイト。環境変数や実行ファイルの引数から暗黙設定しない |
| 所有 | snapshotの内容は実行中不変。取得時は既存文字列ヒープにコピーし、関数・配列等へ渡して通常の所有展開で解放。snapshotは正常終了時に解放 |
| 失敗 | 起動準備とソース操作を区別する`arguments-v1`。負添字・文字列ヒープ超過・確保失敗では既出力をflushし、NodeId/SourceId/Spanを記録して終了1。本文は含めず、後続処理を行わない |
| ツール | WindowsのMSVC対象Clangでは通常のリンク。MinGW対象ではUnicode起動用に`-municode`を指定。C標準ライブラリとWindowsの基本API以外のライブラリは追加しない |

[生成Cの比較テスト](../../tests/program_arguments/c.rs)は`-O0`と`-O2`でexample、空・日本語・CR/LF・結合文字・補助平面文字・quote・オプション風文字列、評価順・短絡・コピー・解放を比較します。上限、元入力変更、未設定、NUL、不正UTF-8/UTF-16、確保失敗も検証します。入力と引数のUTF-8検査は同じ生成ヘルパーを使います。

C内の静的補助関数は安定した埋込みABIとして公開するものではありません。

## LLVMの引数接続

生成Cと同じsnapshot・所有・診断契約を、LLVM IRへ直接展開します。C生成や外部コンパイラをLLVMの生成に内部利用しません。

| 項目 | 実装した契約 |
| --- | --- |
| ターゲット | 引数操作には`--target x86_64-unknown-linux-gnu`または`x86_64-pc-windows-msvc`が必須。生成器のOSから選ばない |
| 起動 | Linuxは`main(argc, argv)`のUTF-8検査、Windowsは`wmain(argc, argv)`の厳密なUTF-16→UTF-8変換。全件の検証・コピー後にCerune本体を実行する |
| 上限 | `emit-llvm`の`--argument-count-limit`／`--argument-byte-limit`で指定し、生成IR内の定数として表示。既定4096件・1048576バイト。Rustでは`emit_llvm_with_argument_limits`から明示指定。既存の生成APIは既定値を維持 |
| 取得と所有 | `arguments.count`／`arguments.get`を専用LLVM命令から内部ヘルパーへ展開。取得文字列は既存ヒープへコピーし、通常の所有展開で解放。空文字は確保しない。snapshotは正常終了時に解放 |
| 失敗と出自 | 起動準備は本文・ソース位置なし、実行中は操作・理由・添字・NodeId/SourceId/Span付きの`arguments-v1`。既出力をflushして終了1。負添字・予算超過・確保失敗を区別 |
| 観測 | `emit-llvm --annotate-origins`で元ソースとの対応を表示。注釈の有無で命令・生成結果は変えない。起動・検証・コピー・解放の処理と上限をIR上に残す |

[LLVM比較テスト](../../tests/program_arguments/llvm.rs)は両ターゲットのIRを`-O0`／`-O2`でオブジェクトへコンパイルし、実行可能なWindows/Linuxでは4実行器と同じexample・正常値・異常値・停止前出力を比較します。macOSではLLVM実行ターゲットを追加せず、両ターゲットのオブジェクト生成を検証します。Cの引数はmacOSでも実行できますが、Cの標準入力併用は既存の未対応診断を検証します。

`emit-llvm`は実行用の`-- [引数...]`を受け取りません。プログラム引数はリンク後の実行ファイルへ渡します。整数読取りは[std:num](integer-parsing.ja.md)に対応しています。

## QBEの引数接続

共通IRの引数操作をQBE ILの専用命令から内部ヘルパーへ展開します。CやLLVMを経由せず、検査・コピー・解放も生成ILで追跡できます。

| 項目 | 実装済みの契約 |
| --- | --- |
| ターゲットと起動 | 明示したLinuxのmain／Windowsのwmainから起動。LinuxはUTF-8検査、Windowsは厳密なUTF-16→UTF-8変換を行い、共通契約に従って全件検査・コピーしてから実行 |
| 上限 | emit-qbeの--argument-count-limit／--argument-byte-limitでILの定数へ設定。既定4096件・1048576バイト。Rust APIはemit_qbe_with_argument_limits、既存APIは既定値を維持 |
| 内部表現 | snapshotはデータポインタとバイト数。取得値は既存の長さ付き文字列と所有領域へコピー。OS変換の戻り値は呼出側の保存先に書き、集約値のABIへ依存しない |
| 失敗と出自 | arguments-v1でcapture／executeを区別。実行時は操作・添字・NodeId/SourceId/Spanを保持し、先行出力をflushして終了1。本文は含めない |
| 所有と観測 | 空文字は確保せず、通常の所有展開で取得値を解放し、正常終了時にsnapshotを解放。共通IRは変更せず、ILに起動・検査順・上限・所有操作を残す |

[QBE比較テスト](../../tests/program_arguments/qbe.rs)はQBE 1.3のamd64_sysv／amd64_winでILからassemblyを生成します。WindowsとLinuxの実行をそれぞれ4実行器・生成Cと比較し、example、日本語・空文字・CR/LF・結合文字・補助平面文字、短絡・評価順、コピー後の独立性、上限・不正入力・確保失敗・停止前出力、標準入力との併用を検証します。古いLinux ClangにはQBEのCOFFセクション書式を受け付けないものがあるため、Windows向けassemblyのオブジェクト化・実行はWindows CIで必須検証します。

emit-qbeは実行用の区切り以後の引数を受け取りません。内部ヘルパーは安定した埋込みABIではありません。整数読取りは[std:num](integer-parsing.ja.md)に対応しています。

## WATの引数接続

[版付きhost契約](wat-arguments-host.ja.md)を実装しました。起動時に固定入力を検査して非公開memoryへコピーし、実行中の取得でhostを再参照しません。取得文字列の所有・上限・出自・停止前出力を4実行器と比較します。引数を含むinstanceは一度だけ実行し、snapshotはinstance破棄時に回収します。新しい3つのimportが必要で、旧hostでの不足はinstantiate時に診断します。WAT libraryの制限は維持します。

## Nativeの引数接続

共通MIRからLIRのArgumentCount／ArgumentGetへ直接loweringし、通常MIR・SSA経由とも同じASM・自前Objectを生成します。C・LLVM・QBEを内部利用しません。

| 項目 | 実装した契約 |
| --- | --- |
| 起動 | 指定したx86_64-unknown-linux-gnuのmainでUTF-8を検証、x86_64-pc-windows-msvcのwmainでUTF-16を厳密変換。OS文字列の検証後に共通の上限・UTF-8・NUL検証を行い、実行前に全件をコピー |
| 上限・API | emit-lir／emit-asm／emit-objの--argument-count-limit／--argument-byte-limit。既定4096件・1048576バイト。Rustではlower_mir_with_argument_limitsでLIRへ保持。既存APIは既定値を維持 |
| 所有 | 引数表と内容を一つの非公開領域へコピーし、正常終了時に解放。OS変換用一時領域と取得文字列のヒープ上限は別。取得した文字列は通常の所有処理で解放し、空文字は確保しない |
| 失敗 | arguments-v1のcapture／executeを区別。実行時は操作・添字・NodeId/SourceId/Spanと先行出力を保持し、終了1。本文を診断へ含めない |
| 観測 | LIRに命令・上限・MIR対応を残し、ASMの起動・検証・コピーも確認可能。生成時は実引数を取得しない。引数のない生成物のmainは維持 |

[Native比較テスト](../../tests/program_arguments/native.rs)で同じexampleを4実行器、通常MIR／SSAのASM・自前Objectで比較します。Windows/Linux CIで必須実行します。未設定・空・欠落、Unicode・CR/LF、上限、検証順、確保失敗、独立コピー・解放、短絡・評価順、stdin併用を検証します。行入力と引数で厳密UTF-8検証を共有します。

内部ヘルパーは安定した埋込みABIではありません。整数読取りは[std:num](integer-parsing.ja.md)に実装済みです。成功操作の時系列記録は後続です。[Native build](native-build.ja.md)・[release初版](native-release.ja.md)は実装済みです。

PR #182でNativeもmasterへ統合済みです。起動・OS文字列変換・snapshot・取得文字列の確保失敗をASM／Objectで比較し、停止前出力とarguments-v1を確認します。Object生成の[SASHIMI接続](native-encoder.ja.md)は言語の引数契約を変更しません。

## 次の保存・再読込み

[限定ファイルI/Oの設計](file-io.ja.md)で、明示した作業範囲でのUTF-8全文読取りと新規保存を扱います。2026-10-28現在はstd:fs・共通IR・4実行器・Windows/Linux実接続・CLI接続指定を実装済みです。生成C・LLVM・QBEも実装済みです。WATの版付きhost接続も実装済みです。NativeとWAT実OS hostは後続です。上書き・子プロセスは後続です。
