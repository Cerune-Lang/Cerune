# 標準入出力：実行・制御・観測

[English](standard-io.en.md)

**状態：4実行器の出力接続、CLIの逐次出力、全実行・生成経路の明示flushを実装済み。生成C・LLVM・QBE・WAT・Nativeの通常書込みも対応済み。標準入力と生成経路の成功操作を含む共通記録は未実装です。** [#89](https://github.com/Cerune-Lang/Cerune/issues/89)の最初の用途を、上限付きUTF-8行入力による小さな対話とします。API名・型・失敗の詳細は後述の未決事項です。

## 採用条件

次の3条件を、API・IR・実装・テストの共通基準にします。

| 条件 | 守ること |
| --- | --- |
| 可観測性 | 入力・出力・失敗・変換を、操作順とソース出自で追える |
| 観測と制御の分離 | 観測APIは読み取り専用。入力の供給、応答の置換、状態の変更を行わない |
| 意味の一貫性 | 同じ入力と外部応答に対する振る舞いをHIR・MIR・SSA・VM・各生成経路で揃える |

観測の有無で言語上の結果・外部操作列・停止理由を変えません。観測の処理時間まで同一とはしません。非最適化の表現と明示変換後の表現は引き続き別々に観測できます。

## 現在の実装と不足

調査対象は `src/ir_executor.rs`、`src/mir_executor/`、`src/ssa_executor.rs`、`src/vm.rs`、`src/main.rs`、`src/codegen/`、`tests/support/` です。

| 対象 | 現在 | 必要な変更 |
| --- | --- | --- |
| IR・MIR・SSA・VMとCLI | 出力収集APIを維持し、逐次出力APIを追加。CLIは実行中に出力し、終了時にflush | 標準入力の追加と生成経路の共通記録 |
| C・LLVM・QBE・Native | 操作ごとの要求/受理数・出自、短い書込み・割込み・停止に対応 | 成功操作を含む共通記録は後続 |
| WAT | 版付き書込み・整形import、部分失敗・再試行・停止に対応。memoryは非公開 | 入力のhost契約と成功操作を含む共通記録 |
| 文字列 | 有効なUTF-8を保持する所有値 | 入力バイトからの厳密な検証、予算と所有の規則 |
| プロセステスト | 終了前の出力到着と二重表示防止を検証済み | 質問の出力を受けてから入力を送る対話テスト |

既存の言語停止診断だけでは、OSの読取り・書込み失敗を規定できません。新しい契約と診断範囲を明示して追加します。

## 責務の分離

実行に必要な外部との接続と、その記録を読む機能を分けます。

```text
Ceruneプログラム → 明示的なI/O操作 → 実行用I/O接続 → OSまたは比較用応答
                       ↓ 要求・結果の記録
                    読み取り専用の観測
```

- 実行用の接続は入力を返し、出力を受け取ります。比較用の接続は実行前に明示的に選びます。
- 観測側は記録を読みます。入力の差替えや実行継続の判断を観測コールバックに委ねません。
- 共通化するのは操作契約・値変換・記録形式です。各実行器の制御フローをVM等へ委譲しません。
- 既存の出力収集APIは維持し、プロセスのstdinへ暗黙に接続しません。入力未設定をEOFと同一視しません。
- CLIが出力を逐次転送する場合、終了時や失敗時に収集済み出力を再表示しません。
- compile・定数評価・emit・現在のobserveはプログラムのI/Oを実行しません。

## 実装済みの出力接続

4実行器に`run_with_output(program, &mut Output<W>)`を追加しました。VMのソース出自付き入口は`run_bytecode_with_output`です。従来の`run`は出力を収集し、戻り値・エラーの`output()`に保持します。

| 項目 | 現在の契約 |
| --- | --- |
| 接続 | `runtime::output::Output<W>`が実行前に選んだ`std::io::Write`へ出力する。stdinやプログラム状態へのアクセスはない |
| 逐次実行 | `run_with_output`は成功時に`()`を返す。出力を内部で再収集せず、エラーの`output()`は空。受理済みバイトは接続先に残る |
| 書込み | 短い書込みは残りを送る。`Interrupted`のみ再試行。0バイト進行・契約外の受理数・その他のエラーで停止 |
| 部分失敗 | 操作・`ErrorKind`・要求/受理バイト数・NodeId/Spanを保持。UTF-8の途中で止まってもバイト数を保つ。後続操作・再送・追加flushは行わない |
| flush | 埋め込み側が`Output::flush()`を明示的に呼ぶ。再試行せず失敗を返す。埋め込み操作の出自は`None`。言語側の`flush();`にはNodeId/Spanを付ける |
| CLI | `run`・`run-ir`・`run-mir`（SSA含む）・`run-vm`は逐次出力。接続自体が失敗していなければ、成功・言語停止のどちらでも最後にflushする。停止理由とflush失敗が両方あれば両方を表示 |
| 記録 | `with_trace_limit(writer, n)`で明示的に有効化。完了した論理出力操作とflushの順序・出自・要求/受理数・失敗を、`events()`から読み取り専用で取得 |
| 記録上限 | 0は記録なし。上限・記録用確保の失敗では`trace_truncated()`を立て、実行を続ける。本文やOSエラー文は記録しない |

記録の単位は共通IRの出力操作です。配列の表示等は複数操作へ展開されます。OSへの書込み分割回数は記録単位にしません。保留中の操作のライブ通知はありません。出力本文も比較する場合は、明示的に選んだwriterでバイト列を収集します。

I/O失敗は`ErrorKind::Output`／`VmErrorKind::Output`で返し、既存の言語検査の`runtime-v1`には偽装しません。通常書込みと明示flushは既存経路へ展開済みです。標準入力や成功操作の記録まで全経路対応したという意味ではありません。

埋め込み側では、出力先と記録上限を選んで実行できます。

```rust
use cerune_lang::{compile_to_ir, ir_executor, runtime::output::Output};

let program = compile_to_ir("print(42);").unwrap();
let mut output = Output::with_trace_limit(Vec::<u8>::new(), 32);
ir_executor::run_with_output(&program, &mut output).unwrap();
output.flush().unwrap();
assert_eq!(output.events().len(), 2); // 書込み、埋め込み側のflush
assert_eq!(output.into_inner(), b"42\n");
```

`Output`は内部値やプログラムを所有せず、観測のcallbackも持ちません。writerは実行用の外部接続であり、観測APIではありません。writer自身が隠れた外部作用を行わないことは、その接続を用意する側の責務です。

## 明示flush（実装済み）

`flush();`は引数と戻り値を持たない組込み文です。`flush`は予約語です。標準出力に保留された出力を、その時点で実行用接続へ渡します。画面描画や永続保存は保証しません。

操作と失敗の扱いは次のとおりです。

| 項目 | 契約 |
| --- | --- |
| 順序と出自 | HIR・MIR・SSA・bytecodeに`flush.stdout`を保持。LIRと生成コードにも操作とNodeId/Spanを残し、観測の有無で回数・順序を変えない |
| 成功 | 後続処理へ進む。収集型APIでは出力は既に収集先へ渡っているため成功する |
| 失敗 | 後続処理を実行せず、再送・再flushを行わない。操作とソース位置を診断する。失敗前の出力は取り消せず、flush中にOSへ届いたバイト数は不明 |
| 対応経路 | HIR・MIR・SSA・VM、C・LLVM・QBE・WAT、Windows/LinuxのASM・自前Object（SSA経由を含む） |
| target | LLVM・QBEでは`flush();`だけのプログラムでも明示targetが必要。実行中のOSからは推測しない |
| 観測 | 4実行器の`Output`で成功・失敗を記録。生成経路の成功操作を集める共通記録APIは後続 |

失敗の安定した記録は、言語検査の`runtime-v1`と区別した`io-v1`です。

```text
cerune: io-v1 operation=flush requested=0 accepted=0 node=N[ file=F] bytes=S..E
```

`[ file=F]`は複数ファイルの出自がある場合の任意項目を表し、角括弧は出力しません。要求・受理数の0は「flushが新しい本文を受け取らない」ことを表し、「OSへ0バイト届いた」という意味ではありません。OS固有のエラー分類はこの記録へ含めず、4実行器のRust APIでは`ErrorKind`も保持します。埋め込み側の`Output::flush()`はソース出自を持たず、記録では`origin=host`です。

CLIの既存オプション`--diagnostic-format runtime-v1`は、言語停止を`runtime-v1`、I/O失敗を`io-v1`で表示します。書込み失敗では`operation=write`と要求・受理バイト数を保持します。既定のCLI診断は人向けの位置付き表示を維持します。生成コードは失敗時に`io-v1`をstderrへ送り、終了します。stderr自体の失敗時には診断の配送を保証できません。

### 生成経路の接続

C・LLVM・QBE・Nativeはstdoutのみをflushし、失敗時は終了時のstdio再flushを避けて停止します。Linuxではflush中のSIGPIPEを一時的に無視し、成功時には元の設定を戻します。この設定はプロセス全体に作用するため、現在の単一スレッドの実行成果物を対象とします。外部の多スレッドhostへの埋め込み契約は未定義です。根拠はLinuxの[fflush](https://man7.org/linux/man-pages/man3/fflush.3.html)と[signal](https://man7.org/linux/man-pages/man7/signal.7.html)の規則です。

WATは明示flushを含む成果物だけに`cerune.flush_stdout_v1() -> i32`を追加します。hostは成功で0、失敗で0以外を返します。失敗記録を既存の`write_error_byte`へ送りtrapします。必要なimportがなければinstantiateに失敗し、暗黙に成功とはしません。内部memoryは非公開のままです。テストhostは同期的に出力を収集し、flush成功を返します。

通常書込みの部分失敗・再試行はC・LLVM・QBE・WAT・Nativeで対応済みです。標準入力と、生成経路の成功操作を含む共通記録は未実装です。

## 生成C・LLVM・QBE・Nativeの書込み（実装済み）

共通IRの各出力操作に、要求バイト数・受理済みバイト数・不変のNodeId/Spanを対応させます。C/LLVM/QBE IRとNativeのLIR/ASMにも出自を残し、引数は一度だけ評価してから出力します。複合値の表示は既存の共通IRと同じ操作列へ展開します。

| 項目 | 契約・実装 |
| --- | --- |
| 受理と再試行 | 短い書込みでは残りだけを送る。EINTRのみ再試行。0バイト進行・要求数を超える応答・その他の失敗で停止 |
| 停止 | `io-v1 operation=write`に操作全体の要求/受理数と出自を保持。UTF-8の途中で止まったprefixも保持し、後続処理・再送・追加flushを行わない |
| 整形 | 数値は固定128バイト領域に`snprintf`で整形。文字列は保持済みデータを使い、引用表示は長さを数えてから固定256バイト領域で送る。出力全体をheapへ複製しない |
| 接続 | Cのコンパイル先、LLVM・QBE・Nativeのtargetに応じてLinuxの`write`／Windows CRTの`_write`を選ぶ。1呼出しは最大65536バイト。Ceruneを実行しているOSから生成先を推測しない |
| Windows | 数値だけの出力もバイナリモードとし、LFをCR/LFへ書き換えない。日本語・NUL・CR/LFをバイトで保持 |
| Linux | 書込み中はSIGPIPEを一時的に無視し、成功時に戻す。明示flushと同じ単一スレッド成果物の範囲 |

要求/受理数は共通IRの1出力操作について数え、OS呼出しの分割ごとにはリセットしません。受理は画面描画・永続保存の保証ではありません。stdoutの通常出力にstdioバッファを挟まないため、C・LLVM・QBE・Nativeでは`print`時点で失敗を検出する場合があります。WATは同じ操作契約を版付きhost接続で扱います。`flush();`は引き続き操作として残り、既存のflush失敗契約も維持します。

[C](../../tests/c_output.rs)・[LLVM](../../tests/llvm_output.rs)・[QBE](../../tests/qbe_output.rs)の[共通比較テスト](../../tests/support/generated_output.rs)では、各実行器へ同じ制御応答を渡すwriter、生成C・LLVM・QBE・Nativeへ試験用の実行接続を用意し、prefix・要求/受理数・出自・停止を照合します。観測APIから応答を書き換える窓口は追加しません。割込み、短い書込み、0進行、契約外の応答、引用表示、数値、複数ファイルを含み、実際の切断pipeは別テストで確認します。

LLVM・QBEでは、数値のみの`print(1);`も明示targetが必要です。生成物の`cerune.output.*`（LLVM）／`cerune_output_*`（QBE）に操作状態・整形・接続・再試行・停止を残します。成功操作の共通記録は未実装で、出自注釈の有無は実行を変えません。`snprintf`自体の失敗は、出自付きの整形失敗として停止し、未生成の本文に要求/受理数を捏造しません。

NativeはLIRの`Output { kind, newline }`と出自を保持し、ASM・自前Objectで同じ`cerune_output_*`処理を使います。通常MIRとSSA経由、出自注釈の有無を[ASM比較](../../tests/native_output.rs)と[Object比較](../../src/codegen/x86_64/output.rs)で確認します。Windowsは数値だけの出力もバイナリモードにします。本文バイトの保存は自前エンコーダの`movb`にも対応し、外部アセンブラの命令バイトと照合します。

OS接続の根拠はLinuxの[write](https://man7.org/linux/man-pages/man2/write.2.html)とWindows CRTの[_write](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/write?view=msvc-170)です。

## WATの書込み（実装済み）

[WAT出力host契約 v1](wat-output-host.ja.md)は、通常出力を `write_stdout_v1` にまとめます。hostへ最大8バイトの値を渡し、生成WATで部分受理・割込み・0進行・契約違反・失敗を区別します。要求/受理数とNodeId/Spanを保持し、失敗時は `io-v1` 診断で停止します。

整数・真偽値はWAT内で整形します。floatは版付き整形importから全長とバイトを固定128バイト領域へ受け、検査後に出力するため要求数を正確に数えられます。内部memoryは非公開、`flush_stdout_v1` は従来どおりです。旧hostは必要なimportがなければinstantiateに失敗します。[互換性・整形・テストの詳細](wat-output-host.ja.md)を参照してください。

## 最初の入力と出力順

利用者向けには、バイト数の上限を明示するUTF-8行入力を目指します。以下は実装で満たす意味の方向であり、公開関数名や戻り値型の確定ではありません。

| 項目 | 方針 |
| --- | --- |
| 文字列 | 厳密なUTF-8。無効バイトを置換せず、Unicodeを正規化しない。NULも内容として保持 |
| 行の区切り | LFまでを1行とし、LF・CRLFを含めて保持。単独CRは内容 |
| EOF | 改行なしの末尾があれば一度返し、その後EOF。空行とEOFを区別 |
| 上限 | バイト数で明示。無効な指定は外部読取りより前に検査 |
| 失敗 | 読取り・UTF-8変換・上限・予算の失敗を区別。既に消費した入力を巻き戻したと扱わない |
| 対話 | 質問を書き、flushを完了してから入力を待つ。flush失敗後に入力へ進まない |
| 部分出力 | 要求・受理されたバイト・flush結果を区別。失敗前の外部出力を取り消したと扱わない |

flush成功は接続先へ出力を渡したことを意味し、画面描画や永続保存を保証しません。改行や入力操作による暗黙flushには依存せず、IR上で順序を追えるようにします。

## 各経路への接続

実行器には出力収集・実OS・制御された比較の接続を用意する方向です。構文の展開や所有の処理は既存frontendを利用し、入力機能のために別の意味解決を作りません。

| 経路 | 実装時の条件 |
| --- | --- |
| HIR・MIR・SSA・VM | 操作と出自を保持。SSA変換等でI/Oの順序・回数を変えない |
| C・LLVM・QBE | 同じ契約を生成コードに表す。LLVM・QBEのtargetを実行中のOSから推測しない |
| LIR・ASM・自前Object | 明示されたtargetのABIとI/O補助処理を使い、生成過程も観測できる |
| WAT | 値・状態を返すimportで入力を渡す。hostへ内部heapのポインタや可変memoryを公開しない |

Windows/LinuxでUTF-8・NUL・CR/LFを保つ接続を検証します。consoleとpipeの違いも明記します。WATの出力importの移行範囲は[host契約](wat-output-host.ja.md)に記載しています。入力用importは別途設計します。C等は引き続きemit-onlyであり、新たな暗黙ビルド経路は加えません。

## 観測と比較

[#104](https://github.com/Cerune-Lang/Cerune/issues/104)に従い、比較用の入力・応答と隔離した出力先を各経路に個別に用意します。一つの経路が消費したストリームを次の経路へ渡しません。実OSとの接続テストは別に行います。

操作記録には、順序番号、操作種別、要求・結果、ソースのNodeId/Spanを対応させます。繰返し実行は順序番号で区別します。NodeIdが異なるコンパイル間でも固定とは仮定しません。

比較するのは言語上の操作と結果です。OS呼出しの分割回数そのものを同一とは要求しません。ただし部分読取り・書込み・失敗が結果へ与えた影響は省きません。入力本文の記録は明示的に選び、現在の停止診断へ自動で混ぜません。観測容量に達した場合は記録の欠落を示し、プログラムを停止させたりI/Oを差し替えたりしない方針です。

## 実装順と完了条件

順番は対話に必要な依存関係から決めます。汎用record/replay、effect system、標準ライブラリ全体を前提にはしません。

| 段階 | 作業 | 確認すること |
| --- | --- | --- |
| 1 | 出力収集と実行用I/O接続を分離し、出力・flush・失敗の契約を揃える | 既存出力の互換性、逐次転送、二重表示なし、部分失敗、観測有無の一致 |
| 2 | 上限付き行入力の詳細を確定し、共通IRから各経路へ実装 | EOF・空行・末尾、厳密UTF-8、予算、失敗と出自、所有と解放 |
| 3 | 実際の対話exampleと経路比較を追加 | 質問が届くまで入力を送らないテスト、同じ入力・応答・操作順、Windows/LinuxとWAT host |

段階1の出力接続は実装済みです。明示flushも対応済みです。C・LLVM・QBE・WAT・Nativeの通常書込みも対応済みです。[上限付きUTF-8行入力](line-input.ja.md)の契約と共通Rust接続を追加しました。次は文字列所有・公開結果型・入力命令を接続し、各経路へ展開します。実stdinは未対応です。生成経路の成功操作を含む共通記録は後続として明示し、入力の設計と混同しません。

## 実装前に詰める詳細

残る設計・接続作業を次に示します。確定した行入力契約と未実装の経路を区別します。

- 段階1の残り：生成経路の成功操作を含む共通のI/O記録。通常書込みの停止契約は全経路へ展開済み。埋め込み接続の部分書込み・再試行・記録上限は上記のとおり実装済み。
- 段階2：API名・戻り値型・標準モジュールの供給方法。既存enum/importで足りるか確認し、暗黙のファイル探索を加えない。
- 行入力のEOF確認・最大1バイトの追加消費・異常時停止・分類済み診断は[契約](line-input.ja.md)で確定。回復可能な別APIは後続。
- 上限分の事前予約と所有移行を文字列heapへ接続する。既存の`[u8]`の論理要素幅は1バイトとは限らない。
- console/pipeの対応範囲、OSエラーから共通分類への対応、WAT入力用host ABIの版と互換性。

## 現在動かせる基準例

[explicit_flush.ceru](../../examples/io_protocol/explicit_flush.ceru)は関数・ループ内のflushと日本語・NUL・CR/LFを確認します。[生成経路のテスト](../../tests/explicit_flush.rs)で同じ出力、制御されたflush失敗と出自、終了前の出力到着を比較し、Linuxでは切断したsocketでも停止診断を確認します。

[output_sequence.ceru](../../examples/io_protocol/output_sequence.ceru)は関数評価と表示の順序、日本語・NUL・CR/LFの保持を確認します。[実行APIのテスト](../../tests/execution_output.rs)で観測の有無・容量・部分書込みを比較し、[CLIテスト](../../tests/execution_output_cli.rs)で終了前の出力到着と二重表示防止を確認します。

[入力値による分岐](../../examples/io_protocol/README.md)は、Cerune側に状態と操作の規則を置く既存機能の例です。値はソース内で用意しており、**標準入力・flushは使いません**。空行と終了・失敗の区別、後続操作の抑止、日本語・NUL・改行・非正規化を期待値と比較します。実際の読取り・変換失敗は段階2以降のテストです。
