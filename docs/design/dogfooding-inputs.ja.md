# 利用例から進める引数と数値の読取り

[English](dogfooding-inputs.en.md)

2026-10-20。[#173](https://github.com/Cerune-Lang/Cerune/issues/173)と[#89](https://github.com/Cerune-Lang/Cerune/issues/89)の次の設計案です。**引数取得はHIR・MIR・SSA・VMとCLIに対応済み。生成経路と整数読取りは未実装**です。実装済みの標準入力・WAT関数公開を作り直しません。

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

`std:env`という予約済み仮想モジュールに、引数数と指定位置の引数を読む操作を置く案です。環境変数やファイル探索の追加は含めません。公開名はarg_count／arg／Argumentに確定しました。生成経路への接続は後続です。

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

最初は文字列からi64を読む操作を提案します。seedだけの特別処理や、既存の数値型間の`convert<T>`への暗黙追加にはしません。

| 入力 | 提案する結果 |
| --- | --- |
| ASCIIの`-?[0-9]+` | i64の値。先頭ゼロを許可、`-0`は整数0 |
| 空、符号だけ、`+1`、空白、改行、全角数字、小数、NUL、末尾の文字 | 不正な書式 |
| 文法に合うがi64の上下限を超える | 範囲外 |
| 結果型 | 既存enumの成功／不正書式／範囲外。汎用Resultを前提にしない |
| 判定順 | 全体の書式を確認してから範囲を判定。`巨大な桁列+x`を単なる範囲外にしない |

整数への読取りと、seedの用途上の範囲検査は分けます。i64として正しくても負値や2147483647以上のseedは例側が拒否します。書式の誤りを値で返すことと、確保失敗などの実行時停止も区別します。入力文字列は消費・変更しません。

現在は一般の文字列添字APIがありません。同梱Ceruneソースだけでこの処理を無理に実装せず、必要な純粋IR操作の範囲と各backendの実装を比較して決めます。

## 実装単位と検証

1. 実装済み：公開宣言・snapshot・共通IRをHIR/MIR/SSA/VMとCLIへ接続。引数を表示するexampleと比較テストを追加。
2. C/LLVM/QBE/WAT/Nativeへ接続し、同じ引数列で比較する。未接続の経路を空列や成功として扱わない。
3. 純粋な整数読取りを全経路へ追加し、seedを引数から受け取る例を完成する。
4. 引数・整数読取りの完了後は、[本体ロードマップ](language-roadmap.ja.md)に従ってoptional let＋fmt、Native build・配布初版、再帰へ進む。その後、明示した作業範囲でのファイルI/Oとshellを経由しない子プロセス操作を個別設計する。

各段階で元HIR・非SSA MIR・SSA・生成物・ソース対応を維持します。最適化・Native build・パッケージ管理は前提にしません。

生成経路の実装前に確定する残件は、Windowsの起動引数取得とUTF-16変換、WATの版付きhost ABI、各経路での上限設定です。WATの内部memoryを引数転送のために公開しません。C/LLVM/QBE/Nativeでもターゲット固有の入力方法を明示し、生成実行中のOSから黙って選びません。

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

C・LLVM・QBE・WAT・Nativeは未接続で、引数操作を含む生成を位置付きで拒否します。未使用関数も保持するため、std:envのimportだけでも対象になります。emit-sources／emit-ir／emit-mir／emit-bytecodeは観測できますが、Native生成を含むobserve bundleは未対応です。成功した各取得の時系列イベント記録は未実装です。次は生成経路の接続を進め、全経路が揃ってから整数読取りへ進みます。
