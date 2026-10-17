# HIR／MIR／LIRと変換過程の観測

[English](ir-stages.en.md) · [Issue #94](https://github.com/Cerune-Lang/Cerune/issues/94)

## 現状と今回の方針

調査基点はNative動的配列対応を統合した`f39f55e`です。**共通MIRの型・検証器・HIRからの変換と`emit-mir`を実装しました。[MIR実行器](mir-executor.ja.md)・`run-mir`も実装済みです。[NativeのMIR入力への移行](native-mir.ja.md)も実装済みです。SSAの表現・構造検証・観測テキスト・自動変換・Rust APIでの直接実行も実装済みです。公開CLI・bundleのSSA対応も実装済みです。SSA→MIR／Nativeも明示指定で利用できます。最適化passは後続です。**

| 段階 | 現在の実装 | 方針 |
| --- | --- | --- |
| AST | ソース構文と位置を保持。名前・型・generic等の共通処理への入力 | 新しいAST実行器は作らない |
| HIR | 完成済み`ir::Program`が相当。型・参照先・所有処理を確定し、構造化制御を保持 | Cerune IRをそのまま使い、同じ意味表現を二重に定義しない |
| MIR | `mir::Program`、HIRからの変換・検証・観測テキスト・独立実行 | 型付き一時値、basic block、明示的な制御フローを持つ、ターゲット非依存の実行表現 |
| LIR | `codegen::x86_64::ir`が相当。レジスタ操作、stack slot、frame、ターゲットを持つ | Nativeへの変換段階として整理。完成した機械命令IRとはまだ呼ばない |
| 成果物 | bytecode、C、LLVM、QBE、WAT、ASM、自前COFF/ELF | 表現と完成成果物の違いは[経路設計](owned-routes.ja.md)で扱う |

`run`／`run-ir`は完成済みCerune IRの直接実行、`run-vm`はbytecode実行のままです。`ir`モジュール、`emit-ir`、`.ceir`をこの整理だけで改名しません。「HIR」は既存IRの役割を示す呼び方です。

ここでのSSAは共通MIRのscalar SSA変換です。生成先のLLVM／QBEが持つSSA形式とは区別します。

LIRの生成・専用テキスト・構造検証・bundle保存は実装済みです。[直接実行器は調査を終えて保留](lir-observation.ja.md)としました。観測と直接評価は別の機能として扱います。

## 既存コードから分かること

| 実装 | 現在担っている処理 | 新段階へ持ち込まないもの |
| --- | --- | --- |
| [IR builder](../../src/ir/builder.rs) | generic展開、配列型の長さ解決、sum/match展開、意味解析、型付きIR構築、aggregate展開、ownership展開 | MIRでの再型推論・再名前解決・所有規則の再定義 |
| [共通IR](../../src/ir/mod.rs) | 型、BindingId／FunctionId、NodeId／Span、if／while／for、配列の確保・初期化・保持・解放 | OSの呼出規約、物理ポインタ幅 |
| [IR Executor](../../src/ir_executor.rs) | 構造化された文・式を直接評価 | MIRの評価をHIR実行へ戻す実装 |
| [bytecode](../../src/bytecode.rs) | operand stack、局所slot、ジャンプ、命令ごとの出自 | bytecodeを共通MIRと呼び替えること |
| [Native lowering](../../src/codegen/x86_64/lower.rs)、[emitter](../../src/codegen/x86_64/emit.rs) | stack／ABIの配置、命令選択。emitterにも検査分岐やprologue等の展開が残る | 全機械命令が現行x86-64 IRで既に確定するという説明 |
| [runtime](../../src/runtime) | 数値規則、文字列／配列領域、表示と停止コードの共有 | 汎用の実行エンジンを新設して比較経路を同一化すること |

ファイル入力のモジュール解決は[modules](../../src/modules.rs)で行い、ファイルの出自を保って共通frontendへ渡します。ソースで要求する定数の評価やgeneric展開は、言語を具体化するための処理です。「最適化なし」でも必要であり、将来の任意の最適化passと区別します。

## 基準経路と移行後の形

現在はHIRからIR実行・bytecode・外部backendへ分岐し、NativeはMIR→LIRを通ります。次の図は現在の実装です。実行器へ渡すのは各表現であり、前の実行器の実行結果ではありません。

```text
Source → frontend → HIR ─────────────→ IR Executor（基準）
                     │
                     └→ MIR（最適化なし） ─→ MIR Interpreter
                              │
                              └→ x86-64 LIR → ASM → 自前Object
                     └→ 既存のbytecode／C／LLVM／QBE／WAT（移行中の比較対象）
```

MIRの独立実行を既存経路と比較した後、Nativeの入力をMIRへ移しました。旧HIR→Nativeと新経路の既知出力・失敗・出自・ABIをWindows/Linuxで検証し、旧変換を取り除きました。非最適化のHIR→MIR→LIRを基準として残します。

未最適化の基準snapshotを上書きせず、選んだ変換の前後を別々に残します。将来の選択経路は次の形です。各passは任意で、SSA化そのものも最適化とは別の変換です。

```text
基準: HIR → MIR → LIR → 成果物
選択: HIR → HIR pass → MIR → SSA変換 → MIR pass → LIR → LIR pass → 成果物
```

`-O2`一つで選択内容や順序が見えなくなる構成にはしません。passの集合と順序、変換前後の対応を記録し、未対応の組合せは拒否します。

VMや外部backendの移行は個別に判断します。C・WATでは構造化制御を出力する方法、LLVM・QBEでは外部IRへの対応を検証してから共通MIRを利用します。これらのbackend IRを一律に機械依存LIRと呼びません。MIRのpassを消費しない経路へ指定された場合は拒否し、黙って無視しません。

## 最初のMIRの契約

最初はSSAにせず、型付きの局所領域と一時値への明示的な代入を許します。現在のRust APIは`mir::lower(&ir)`、`mir::validate(&mir)`、`mir::text::emit(&mir)`です。

| 項目 | 表現すること |
| --- | --- |
| Program／Function | HIRの型・関数との対応、入口、配列／文字列の予算、定数化後も保持する文字列利用（`source-strings`） |
| Block | 決定的なBlockIdと命令列。末尾はjump／branch／return等のterminatorを一つ持つ |
| Operand／place | 型付きの定数・一時値・局所値。物理アドレスやCPUレジスタを含めない |
| 値の計算 | 元の整数幅・符号、f32／f64、明示変換、既存の複合値を維持 |
| call／return | 確定した呼出先と左からの引数準備。所有値の受け渡しと内部読み取りを区別 |
| 副作用 | 出力、確保、初期化、保持、解放、更新を実行順に並べる |
| 検査 | checked算術・変換・添字・範囲・確保を区別し、停止理由と元の出自を保持 |

初期MIRではchecked操作を型付き命令として保持してよく、機械用の比較とtrap分岐にすべて分解する必要はありません。失敗した命令で後続の計算を止め、成功時だけ次へ進む意味を定義します。言語にない例外回復やcatchは追加しません。

局所値の読み取りや一時値への保存だけで隠れた深いコピーを発生させません。独立コピーのループとretain／releaseは、完成済みHIRで明示されたものを変換します。実装上の値の保持と、Ceruneの所有を増減させる操作を混同しません。配列代入は対象と各添字を順番に検査してから右辺を評価し、新しい要素の準備後に旧要素を解放します。

`for`のcontinueは更新ブロック、`while`のcontinueは条件ブロック、breakは対応するループの出口へ進みます。returnも含め、HIRが既に挿入した解放を通る順序を維持します。短絡式の右辺と引数コピーを分岐の手前へ移しません。

現行のMIR検証器は、参照先、型、terminator、読取り前の確実な初期化、言語上の引数型・所有区分、停止出自を検査します。初期化は到達可能な全前任ブロックの積集合を不動点まで求めます。配列代入は、各添字列の検査が全流入経路で先行し、その後に対象や添字が再代入されていないことも確認します。右辺より先に添字を検査する具体的な展開順はテストで固定します。

**heapのalias・部分初期化・retain／releaseを通した寿命の経路検証は未実装です。** 検証成功だけで所有の安全性やHIRとの実行結果一致を保証しません。MIR実行器で既存経路と比較し、正常終了時の未解放領域も検出します。静的な寿命検証は後続作業に残し、将来の外部Imageを安全にloadするための検査とは区別します。

実装の配置と観測情報は次のとおりです。

| 実装 | 内容 |
| --- | --- |
| [mir/mod.rs](../../src/mir/mod.rs) | HIRの型・関数IDを再利用。型付き局所値／一時値、原子的操作、blockとterminator |
| [mir/lower.rs](../../src/mir/lower.rs) | 完成済みHIRの全実行操作を変換。未展開のArrayCopy／Let／Conditionalは明示診断 |
| [mir/validate.rs](../../src/mir/validate.rs) | 構造・型・初期化・添字検査の先行・呼出しの所有区分を検証 |
| [mir/text.rs](../../src/mir/text.rs) | 決定的なv0.1観測テキスト。予算、型、局所値、操作、辺、出自を出力 |

`BlockId`／`LocalId`／`InstructionId`は関数内の識別子です。命令番号はterminatorとも共有し、HIR NodeIdと区別します。元操作の`Source`、元操作に由来する補助辺の`Derived`、関数入口・末尾・明示的なmain呼出しの`Synthetic`を区別します。実行操作の停止位置は単一の元操作です。未到達の文・補助ブロックも残し、暗黙のdead-code eliminationをしません。

型の既定値・定数宣言・genericの展開元等の宣言情報はHIRに残します。MIRは展開済みの実行操作を保持し、元の宣言情報はNodeIdとHIRから観測します。Rust APIの値を変更しても元HIRに干渉しない独立したsnapshotです。

## 意味保存と資源の扱い

比較するのは値だけではありません。同じソースと明示予算に対し、次を維持します。

- 正常終了、戻り値、出力バイト列。日本語・NUL・CR/LFを保持し、Unicodeを正規化しない。
- 言語の異常停止コード、元のNodeId／SourceId／Span、停止前の出力。
- 評価順、短絡、コピー後の独立性、初期化済み要素だけの読取り・解放。
- 配列／文字列の独立したメモリ上限。旧値と新値が共存する時点と、予算の返却時点。
- 成功・言語の停止・内部不整合・未対応・外部ツール失敗・テスト時間切れの区別。

たとえば使わないコピーでも確保時に予算超過する場合があります。dead-code eliminationがそれを消すと停止の意味が変わります。確保・検査・解放・出力は副作用として扱い、単に最終値が等しいだけでは移動・削除しません。浮動小数点の結合順やNaN・負のゼロも保存対象です。

メモリ上限と物理メモリ不足は別です。異なるOS・allocatorの実際のmalloc失敗時点まで同一とは約束しません。固定した確保失敗条件を用い、各段階が適切な失敗・出自・先行出力を返すことを検証します。無限実行も成功に数えず、将来の証明では発散を扱う実行関係か明示したfuelの範囲を定めます。

## 出自と観測

HIRのNodeIdは引き続き意味上の元の文・式を指します。MIRのBlockId／命令ID、LIRの命令IDは別の名前空間とし、snapshot内の識別子です。ソース編集やpass間で同じ番号を保証しません。

各変換は入力要素から出力要素への対応を保存します。一対多・多対一・削除を表し、生成された補助要素と出自欠落を区別します。将来複数の操作を統合するpassでも、実際に失敗する元の操作を診断できなければ変換しません。由来の集合から任意の一つを選んで停止位置にすることは認めません。

| 観測対象 | 残すもの |
| --- | --- |
| HIR→MIR | 構造化制御からblock／辺への変換、評価順を決めた一時値、元の検査・所有操作 |
| MIR→LIR | 型付き操作からtarget／ABI／stack配置への変換 |
| 各pass | 入力・出力snapshot、pass名と版・順序・明示オプション、要素の対応 |
| LIR→成果物 | 既存のASM注釈・Objectシンボルへの対応。命令選択後にemitterで行う展開も説明 |

観測はコンパイラ状態から切り離した読み取り専用の情報とし、可変状態やruntimeのheapを公開しません。新しい公開形式には版を付けます。ソース本文・パスは明示要求時だけ含め、時刻・ランダムID・環境の秘密情報を保存しません。

## CLI・bundle・Leanとの関係

`emit-mir <file> [-o <output.txt>]`を実装しました。`run-mir`で独立実行できます。[`emit-lir`とLIRの構造検証](lir-observation.ja.md)も実装済みです。`emit-hir`とpass指定は未実装です。観測テキストは`Cerune MIR v0.1`で、専用拡張子・load形式・配布用snapshotは未確定です。`emit-ir`の互換性を維持し、MIR実行器の追加だけで新しい配布用routeやbuild形式まで増やしません。

[#60の観測bundle](observation-bundle.ja.md)は、一回のfrontend処理からsources・HIR・MIR・注釈付きASMを保存します。manifestはtarget・heap予算・pass列（現状は空）と成果物を結ぶ索引であり、新しい意味IRではありません。Objectや他backendの同梱は後続です。

[#81](https://github.com/Cerune-Lang/Cerune/issues/81)のshow／emit／build／run／via／releaseと、表現段階は別の分類です。最適化はreleaseの別名にせず、passとその順序を明示します。各backendの外部ツールによる最適化も、Ceruneのpassとは区別して記録します。

[Lean検証](lean-verification.ja.md)では、まず非最適化のHIR→MIR対応、次に選択したpassの前後を検証対象にできます。MIR実行での一致はテストであり形式証明ではありません。実際のu8関数increment・choose・advanceについてHIR→MIR対応と性質を証明します。MIRの型付き局所値・命令列・CFGを独立にモデル化し、if・短絡・while／for・可変局所値・break／continueの最内周への作用・初期化／更新・overflow出自を検査します。ループ例はHIR/MIRそれぞれの明示上限内の完了も証明します。一般のloop停止性・heapや変換アルゴリズム一般、全処理系の証明には拡大解釈しません。

## 基準例と次の実装単位

最初は[control_flow.ceru](../../examples/ir_stages/README.md)で短絡とloopの辺を観測できます。[lowering_order.ceru](../../examples/dynamic_arrays/lowering_order.ceru)は、短絡、forのcontinue／break、動的配列の引数コピーを一緒に観測します。

```sh
cargo run --quiet -- run examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- run-vm examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-ir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48 -o target/lowering_order.mir.txt
cargo run --quiet -- emit-bytecode examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 48
cargo run --quiet -- emit-asm examples/dynamic_arrays/lowering_order.ceru --target x86_64-unknown-linux-gnu --annotate-origins --array-heap-limit 48
```

出力は`begin`、`2`、`40`、`[10, 20, 30]`の4行です。i=0では呼出しを短絡し、i=1は更新へ進み、i=2だけが配列をコピーしてpositiveを呼び、i=3は添字参照の前に終了します。48バイトは元配列24＋引数コピー24です。47に下げると呼出し引数の`values`で`allocation-limit-exceeded`となり、`begin`だけが残ります。

MIRの制御フローをソースに対応させると、次のようになります。**これは構造の説明であり、生成済みMIRのダンプではありません。** 図ではownership補助関数への呼出しを省略しています。MIR導入だけでそれらをinline化しません。実際には各関数の条件計算・コピー・解放にも命令やblockが必要です。

```text
condition → continue判定 → break判定 → 短絡の左辺
                 │              │            ├ true → 加算 → update → condition
                 └→ update      └→ exit      └ false → 引数コピー → call → 結果の分岐
condition false → exit
callの結果: true → 加算 / false → update
```

次の単位で実装と比較を進めます。恒久的に狭い言語サブセットを作る方針ではありません。

| 順序 | 作業 | 完了を判断する条件 |
| --- | --- | --- |
| 1（実装済み） | MIRの型・block・命令・検証器、HIR→MIR、`emit-mir` | 全exampleの変換と決定性、評価順・短絡・loopの辺・出自、破損したMIRの拒否を検証。実行比較は次段階、静的なheap寿命の保証は後続 |
| 2（実装済み） | [独立したMIR Interpreter](mir-executor.ja.md)・`run-mir` | HIR・VMを内部実行せず、現在の言語機能・停止・出自・寿命を比較。動的配列はWindows/Linuxの9経路で照合 |
| 3（実装済み） | [NativeをMIR入力へ移行](native-mir.ja.md) | 現行ASM・COFF/ELFとの既知出力・失敗・出自・ABI比較をWindows/Linuxで通す |
| 4（一部実装済み） | 観測bundle・Lean対応、必要な他backendの移行 | sources/HIR/MIR/ASMのbundleは実装済み。u8の直列・if・短絡・while／for・break／continue・入れ子・更新停止出自と明示上限内の完了を証明。一般のloop・heapは後続。その他の成果物と証明範囲は個別に拡張 |
| 5（一部実装済み） | [SSA変換](mir-ssa.ja.md)、個別の最適化pass | scalar SSAと残存slotの表現・構造検証・観測テキスト・MIR→SSA変換・直接実行を実装。公開CLI・bundleも対応。SSA→MIR／Nativeも対応。有限例のLean対応も実装。[最初の定数畳み込みpass](ssa-constant-folding.ja.md)は設計済み・実装保留 |

最初のMIR PRでSSA、最適化、全backendの付替え、Image loaderを一括導入しません。MIR実行は型・関数・module・generic・配列・文字列・所有・停止診断に対応しています。NativeのMIR移行も実装しました。観測bundleの初期範囲も実装済みです。HIR→MIRのLean検証はu8関数の直列・分岐・短絡・whileの明示上限内の完了まで進みました。while内の分岐・break／continueと入れ子の行き先も検証しました。forの初期化と、本文末尾・continueが更新を通りbreakが飛ばす対応、更新時overflowの出自も検証しました。この非最適化MIRを基準とする[SSA設計](mir-ssa.ja.md)をまとめました。SSAの専用表現・構造検証器・決定的な観測テキストを実装しました。元MIRを保持する自動変換も実装しました。SSA直接実行も実装し、実行用example・停止例・heap予算で基準経路と出力・停止・所有の意味を比較します。公開CLIとbundleでも`--ssa`を明示して利用できます。`emit-asm --ssa`／`emit-obj --ssa`はSSAから明示copy付きMIRへ戻してNativeを生成し、bundleには基準／SSA経由の両ASMを保存します。SSAの有限例のLean対応も実装済みです。[定数畳み込みの設計](ssa-constant-folding.ja.md)と基準exampleを残し、pass実装は保留します。[ロードマップ](language-roadmap.ja.md)に沿って言語機能・入出力・完成成果物を優先します。
