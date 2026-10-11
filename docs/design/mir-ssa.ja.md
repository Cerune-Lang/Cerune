# MIRのSSA変換

[English](mir-ssa.en.md) · [IR段階](ir-stages.ja.md) · [Issue #94](https://github.com/Hokutaka/Cerune/issues/94)

## 状態と判断

**SSAの表現・構造検証器・観測テキストと、MIR→SSAの自動変換を実装しました。SSA直接実行、公開CLIの`--ssa`、SSA→MIR／Native、観測bundle連携も実装済みです。** この文書は実装済みの範囲と後続の方針を区別します。基準は非SSAのMIRと、その独立実行器です。[ssa_values.ceru](../../examples/ir_stages/ssa_values.ceru)を含む実行用exampleで、IR・非最適化MIR・SSA・VMの結果を比較します。

SSAは「各値の定義を一つにする」表現です。ソースの`mut`を禁止せず、再代入ごとに別の値番号を付けます。合流点では、通った辺からブロック引数へ値を渡します。変換を明示的に選び、元MIRを上書きしません。

| 決めること | 方針 | 理由 |
| --- | --- | --- |
| 位置づけ | MIRの選択可能な表現変換 | SSA化だけでは最適化しない |
| 合流値 | 型付きブロック引数と辺の実引数 | if・短絡・loopで、どこからどの値が来るかを表示できる |
| 最初の昇格対象 | bool・全整数型・f32/f64の局所値と一時値 | scalarの再代入を扱い、heapの寿命の変更を持ち込まない |
| その他の値 | 型付きslotと現在の操作を保持 | 文字列・固定／動的配列・named型を含むプログラムを、別の意味の言語にしない |
| 実行の基準 | 非SSA MIRを残し、SSAを直接読む評価処理と比較 | HIRや元MIRの実行へ戻して一致したことにしない |
| Native | 検証後にSSA→非SSA MIRを明示して既存LIRへ渡す | Native全体を同時に書き換えず、辺上の値移送も観測できる |

ここでのSSAは**scalar SSAと残存slotの混在**です。heapの状態まで一意な値にするMemory SSAや、配列の要素単位のSSAではありません。残存slotがあることを出力の冒頭と局所値一覧に表示します。言語機能の対応と、SSAへ昇格する値の範囲は別です。

ブロック引数は[MLIRの構造](https://mlir.llvm.org/docs/LangRef/#blocks)にもある表現です。[LLVMのphi](https://llvm.org/docs/LangRef.html#phi-instruction)とは合流値の書き方が異なります。これらの処理系を内部依存に追加する判断ではありません。

## 既存実装との対応

調査したコードと、そのまま保つ契約を整理します。

| 現在の実装 | SSAでの扱い |
| --- | --- |
| [mir/mod.rs](../../src/mir/mod.rs)の`Assign`と空pathの`Store` | 昇格対象への書込みごとに新しいValueIdを定義。再代入も明示copyとして残す |
| `Operation::Copy` | 表現上の読取り。深いコピー・retainではない。SSA化で除去しない |
| `Store`の非空path、`CheckIndex` | 残存slotとSSA化した添字を使い、元の検査・右辺評価・更新順を維持 |
| `Call`、checked演算、確保、retain／release、出力 | 同じblock・同じ順序に残し、型・所有区分・停止出自を維持 |
| [validate.rs](../../src/mir/validate.rs) | 元MIRの型・確実な初期化・先行する添字検査を確認してから変換 |
| [mir_executor.rs](../../src/mir_executor.rs) | 数値・heap等の意味実装を共有できる範囲で整理。制御・値参照はSSA側が評価 |
| [text.rs](../../src/mir/text.rs)、[観測bundle](observation-bundle.ja.md) | 元MIRの形式を維持し、SSA snapshotと変換対応を別に追加 |

`Assign`の右辺を評価してから新しい定義を登録します。`x = x + 1`の右辺は古いxを参照します。関数引数は入口の定義となり、所有区分を引き継ぎます。残存slotの引数準備に隠れた所有コピーを加えません。

SSA値番号は静的な定義の識別子です。loopで同じ命令を再訪すると、その回の動的な値が生成されます。「一度しか実行できない命令」という意味ではありません。

## 表現と変換

新しい責務は`mir::ssa`配下に置く方針です。型・FunctionId・SourceOrigin・演算の意味を再定義しません。ValueId、slot参照、ブロック引数、辺の引数を区別する専用の表現を持ち、元の`mir::Program`に「SSAのつもり」というフラグだけを付けません。既存操作のoperand型を共有する整理は、実装で重複が必要になった箇所に限定します。

入口と状態は次のとおりです。未実装のRust API名は実装時に確定します。

| 入口候補 | 責務 |
| --- | --- |
| `mir::ssa::construct(&mir)`（実装済み） | 入力MIRを検証し、独立した元MIR snapshot・SSA・対応を返す。`scalar-ssa-v1`、オプションなしの変換として記録 |
| `mir::ssa::validate(&ssa)`（実装済み） | 定義・使用・型・辺・残存slot・出自を検査 |
| `mir::ssa::mapping::emit(&ssa)`（実装済み） | 元MIRへの対応だけを観測テキストとして返す。bundleの別ファイルに保存 |
| `mir::ssa::text::emit(&ssa)`（実装済み） | 検証後に`Result<String, mir::Error>`を返す。`Cerune scalar SSA v0.1`の観測テキストでありload形式ではない |
| `ssa_executor::run(&ssa)`（実装済み） | SSAのblock・値・slotを直接評価し、`Result<String, ExecutionError>`を返す。既存の停止・出力契約を使う |
| `mir::ssa::lower(&ssa)`（実装済み） | `Lowered { program, mapping }`。辺の引数を並列copyへ展開した非SSA MIRと観測用の対応テキストを返す |

実装済みの`ssa::Program`は、独立した元MIR snapshotと、SSAの関数・値・blockを保持します。`Operand::Value`と`Operand::Slot`を区別し、演算定義は`mir::Operation<R>`／`InstructionKind<R>`を共有します。既存MIRでは既定の`R = LocalId`を使うため、演算の意味や既存の観測テキストは変えません。

構造検証では元MIRも検証し、到達可能block・命令順・元命令ID・出自の対応と、未到達blockの保持を確認します。SSAの支配関係を検査した後、型・slot初期化・添字検査を既存MIR検証と共有します。そのための内部の参照置換は検査専用であり、辺の並列copyを実装したSSA→MIR loweringや実行経路ではありません。ブロック引数は入口で初期化し、その値を含む前周の添字検査事実を無効化します。

現在の検証は構造と対応記録の整合性の検査です。演算内容・辺の選び方が元MIRと同じ意味かの保証は、自動変換後の実行比較と証明に残ります。初期の対応は一つの元blockと命令順を保つ形に限定し、最適化による命令の削除・統合の記録は未実装です。

実装した自動変換は次の順序で行います。

1. 入力MIRを検証し、関数入口からのCFG到達性を計算する。条件が定数でも両方の辺を辿り、定数畳み込みをしない。
2. 昇格対象と残存slotを決める。各書込みの定義点、各読取りと辺で必要な値を調べる。
3. 支配関係（入口から使用点へのすべての経路が定義点を通ること）と支配境界を計算する。各局所値の反復支配境界のうち入口で生きているblockに引数を置く。loopの戻り辺も含めて不動点まで求める。
4. 支配木に沿って定義を改名し、各辺に実引数を付ける。block内は元の順序で処理する。引数・局所値・blockの走査順を固定し、hashの列挙順に番号を依存させない。
5. 変換後の構造検証を行う。テストでは元MIR・操作・辺・出自・各読取りの最新定義を独立に照合する。SSA直接実行とHIR・非SSA MIR・VMの出力・停止出自・先行出力も比較する。

入口で使わない局所値に引数を作らないことは、未定義の値を捏造しないための表現上の選択です。元の計算やcopyは削除しません。合流する実引数がすべて同じでも、最初の変換では作った引数を簡約しません。

### SSA直接実行

[ssa_executor.rs](../../src/ssa_executor.rs)は、SSAのblock・辺・ValueId・残存slotを直接評価します。元MIRの本体やVMへ実行を委譲しません。元snapshotは型定義・signature・slot型・heap予算と出自の照合に使い、実行前後で変更しません。

個々の数値・文字列・配列・所有操作は[共通の意味処理](../../src/mir_executor/semantics.rs)をMIR実行器と共有します。CFGの進行、値の参照、関数呼出し先の選択はそれぞれの実行器が担当します。辺の全入力を読んでから同時に束縛し、所有copyや確保を追加しません。

診断APIは次の情報を返します。

| 情報 | 意味 |
| --- | --- |
| `ErrorKind` | MIRと共通の構造検証失敗・言語のruntime failure・内部不整合（`InvalidMir`） |
| `origin()` / `runtime_failure()` | 実際に停止した操作のNodeId・SourceId・Span・停止コード。呼出し元で上書きしない |
| `location()` | SSA側のFunctionId・BlockIdと、元MIRのBlockId・InstructionId |
| `output()` | 停止する前までの出力 |

実行前にSSA全体を検証し、検証失敗では出力しません。実行ごとに新しいframeとheapを作り、正常終了時は未解放の所有領域も検査します。SSAの有限例のLean証明は後述の実験に対応し、一般の証明・個別の最適化passは後続です。実行比較は変換一般の形式証明ではありません。

### 未到達block

現行MIRは未到達の文・補助blockも保持し、そこには到達可能部分と同じ初期化保証がありません。これに偽の0・undef・poisonを渡してSSAを成立させません。

入口から到達できる部分をSSA化し、到達できないblockは**元MIRの未変換記録**として同じsnapshot内に別区分で保持します。元の命令・辺・出自と、到達可能部分への参照をすべて残します。対応には「未到達のため保持」と理由を付け、削除とは区別します。したがってsnapshot全体を完全SSAとは呼びません。

SSA評価は到達可能なSSA区分だけを実行します。そこから未変換記録へ進む辺があれば検証エラーです。将来CFGを書き換えて未到達区分を有効化するpassは、初期化を再検査し、その区分も変換する必要があります。SSAから戻す際は未到達記録も復元し、ID参照を対応表で付け直します。

### 辺の実引数

`jump join(a, b)`は、その辺を出る時点のaとbを**両方読んでから**、次のblockの引数へ同時に束縛します。辺の引数は既存の値番号だけで、式・call・slot読取りを埋め込みません。branchは選んだ辺だけを渡ります。then/elseが同じblockを指しても、辺の識別と実引数を区別します。

loopの`jump head(right, left)`を逐次代入すると、入替え前の値を失うことがあります。SSAを非SSAへ戻す場合は一時値を使って並列copyを実現します。SSAの定義とblock引数の保存先にはMIRのTemporaryを割り当てます。複数回書くblock引数をBindingの初期化と偽らず、元の束縛名は対応情報に残します。分岐元に複数の行き先があるときのcopyは、選んだ辺専用の補助blockへ置きます。そこで出力・確保・retain／releaseを追加しません。

### SSA→MIRとNative

[lower.rs](../../src/mir/ssa/lower.rs)はSSAを検証後、値ごとのTemporaryと辺の並列copyへ変換し、生成MIRも通常の検証器で検査します。元MIRの本体を実行用に選び直す処理ではありません。変換前のsnapshotは変更しません。

| 対象 | 変換と観測 |
| --- | --- |
| SSA値・block引数 | 値ごとに専用Temporaryを割り当て、元LocalIdとの対応を記録 |
| 関数引数 | signatureのBindingを保持し、追加の入口blockでSSA Temporaryへcopy。所有操作は増やさない |
| 実引数のある辺 | 辺ごとの補助blockで全入力を一時保存してから全出力を書き、元の行き先へjump |
| 元命令・停止位置 | 元InstructionIdとSourceOriginを維持。追加copyは`ssa-edge-copy`／`ssa-parameter-copy`として区別 |
| 未到達block | 元のblock番号・命令・参照をそのまま復元。実行可能なSSA blockも元block番号の位置へ配置 |
| Native | `Lowered.program`を既存MIR→LIR→ASM／Objectへ渡す |

引数なしの辺には補助blockを作りません。引数付きの辺はjumpでもbranchでも同じ並列copy規則を使い、同じ行き先のthen／elseも別blockです。局所領域の再利用やcopy削除はせず、stack使用量・命令数が基準経路より増える場合があります。文字列・配列の論理heap予算と所有操作は変わりません。

対応テキストは`Cerune SSA lowering mapping v0.1`、変換名は`ssa-lower-v1`です。SSA値からMIR局所値、SSA blockから元番号を保持したMIR block、辺から補助block・read/write命令、引数から入口copyを辿れます。その先は既存のMIR→LIR注釈とObjectシンボルに接続します。

## 比較用の表現

[ssa_values.ceru](../../examples/ir_stages/ssa_values.ceru)は、ifで選んだ値にloopで0と2を加え、別のloopで二つの値を3回入れ替えます。IR・非SSA MIR・SSA・VMでの期待出力は`14\n16\n20\n10\n`です。

次は合流の**説明用の略記であり、現在の生成物や確定したテキスト文法ではありません**。literalやcopy等の命令と出自注釈を省略しています。実際のSSA生成時は元の操作を残します。

```text
非SSA:
  then: total = checked_add(total, 2); jump join
  else: total = checked_add(total, 4); jump join
  join: ... use total ...

SSA案:
  then: v_then = checked_add(v_seed, 2); jump join(v_then)
  else: v_else = checked_add(v_seed, 4); jump join(v_else)
  join(v_total: u8): ... use v_total ...

loopの戻り辺:
  head(v_left: u8, v_right: u8, v_index: u8):
    ...
    jump head(v_right, v_left, v_next_index)
```

[Rust API例](../../experiments/ssa/README.md)は、引数なしなら手作りSSAを表示し、ソースを指定すると実際のMIRから自動変換します。どちらも`original-mir`と`ssa`を別区分で表示します。`--run source.ceru`を明示するとSSAを直接実行します。

```sh
cargo run --quiet --example ssa_model -- examples/ir_stages/ssa_values.ceru
cargo run --quiet --example ssa_model -- examples/ir_stages/owned_values.ceru
cargo run --quiet --example ssa_model -- --run examples/ir_stages/ssa_values.ceru
```

現在の`ssa_values`では、ifの両辺が`bb3(v13)`へ値を渡し、loop条件`bb4(v16, v17)`にtotalとindex、更新`bb7(v25)`にその回のtotalを渡します。`mir-iN`と`original-local`で元MIRへ戻れます。番号はこのソースと変換版での観測例であり、将来の版で同じ番号を保証しません。

Ceruneソースに現在利用できるコマンドは次のとおりです。

```sh
cargo run --quiet -- run examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru --ssa
cargo run --quiet -- run-vm examples/ir_stages/ssa_values.ceru
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru -o target/ssa_values.mir.txt
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru --ssa -o target/ssa_values.ssa.txt
cargo run --quiet -- observe examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu -o target/ssa-observation
```

## 検証器と意味保存

SSAの構造検証では、次を必須にします。

| 検査 | 拒否する例 |
| --- | --- |
| 一意な定義・同一関数内の参照 | 同じValueIdの二重定義、別関数の値、未知の値 |
| 支配とblock内の順序 | 片方のbranch内だけの値を合流後に直接使う、同じblockの後ろの値を先に使う |
| 引数と辺 | 引数の個数・型の不一致、入口引数と関数signatureの不一致、存在しないblock |
| 初期化と残存slot | 未初期化slotの読取り、検査前の添字更新、所有区分の不一致 |
| 出自と対応 | 停止する元操作の欠落、由来の集合から任意の一つを停止位置に選ぶこと |
| 未到達区分 | 実行可能な辺が未変換記録へ入ること |

各block引数はblock入口の定義です。引数へ渡す値の使用点は前任blockのterminatorです。自己ループもこの規則で検査し、引数を無条件に支配済みと扱いません。残存slotの型・初期化・添字検査は現在のMIR検証と同じ義務を持ちます。SSAの構造検証だけでheapのaliasや寿命が証明されたとは扱いません。

意味保存では、戻り値・出力バイト・停止コード・NodeId／SourceId／Span・停止前出力・評価順・短絡・コピーの独立性を比較します。文字列／配列の論理予算、確保と解放の時点も同じです。浮動小数点の結合順、NaN、負のゼロも変更しません。未使用のchecked演算・call・確保・copyもその場に残します。

最初の変換は操作の追加・移動・削除による高速化を行わないため、ブロック引数の受渡しを除いて元の動的な操作列と対応させます。値番号とブロック番号は変わってよく、ソース上の停止出自は変えてはいけません。SSA内部の定義履歴を、外部からruntime状態を書き換えるAPIにはしません。

## 観測・CLI・導入順

変換対応には入力snapshot、出力snapshot、pass名・版・オプション・順序、元FunctionId／BlockId／InstructionId／LocalIdとSSAの値・block・辺を記録します。補助引数は「どの局所値の合流か」、各流入値は「どの辺から来たか」を持ちます。元命令の単一の停止出自と、多対一の由来情報は別に保持します。

`emit-mir --ssa`は元MIRとSSAを表示し、`run-mir --ssa`はSSAを直接実行します。heap予算・既定診断・`runtime-v1`診断は非SSA経路と同じです。指定なしの動作を維持し、対応しない実行・emit経路、重複指定、値付きの`--ssa false`は拒否します。SSAや最適化を暗黙に有効化しません。

`emit-asm --ssa`／`emit-obj --ssa`はSSA→MIRを明示的に通ります。`observe --ssa`は元MIR・SSA・変換後MIR・両段階の対応と、基準／SSA経由のASMを別ファイルに保存します。manifest v3の変換列に`scalar-ssa-v1`→`ssa-lower-v1`と入力・出力・対応を記録します。最適化pass列は空のままです。指定なしのv1は変更しません。形式は[観測bundle](observation-bundle.ja.md)を参照してください。共通pass指定との統合は後続です。

実装を次の単位に分けます。

| 順序 | 作業 | 完了条件 |
| --- | --- | --- |
| 1（実装済み） | 表現・構造検証・決定的なテキスト | 手作りの分岐・loop・並列引数・残存slotを検証。壊れた定義・辺・初期化を拒否 |
| 2（実装済み） | MIR→SSA・変換対応 | 直列・分岐／短絡・loop、残存slotを扱う。実行用example全件で元snapshot不変、操作・辺・出自・最新定義、未到達記録、決定性を検査 |
| 3（実装済み） | SSA直接評価・CLI・bundle | 基準例と既存MIR/runtime testsでHIR・MIR・SSA・VMを比較。heap予算・先行出力・停止出自も比較 |
| 4（実装済み） | SSA→MIR・Native | 並列copy・辺の補助blockを観測し、Windows/LinuxでASM・Objectの結果を照合 |
| 5（有限例を実装済み） | Lean対応 | 具体例のMIR→SSA→MIR対応を独立したSSA制御モデルで検査。詳細は以下 |
| 6（設計済み・実装保留） | [最初の定数畳み込みpass](ssa-constant-folding.ja.md) | ブロック内の成功する整数・bool演算だけを置換。元MIR・SSA前後・置換根拠を保存し、再開時はRust APIと記録から実装。現在の優先順は[ロードマップ](language-roadmap.ja.md)を参照 |

途中の実装で未対応操作が残る間は明示診断し、元MIRの実行へ黙ってfallbackしません。一般公開するSSA実行では現在の言語機能を扱えることを目標にし、残存slotも比較対象にします。

## Leanによる有限例の検証

[証明実験](../../experiments/lean/README.md#ssaの検証)で、実際の元MIR・SSA・変換後MIRをそれぞれLeanの項へ写します。SSAの値・選択した辺・全read後の同時束縛は独立したモデルで評価し、単一演算と命令列の評価は既存MIRモデルと共有します。SSAモデルから元MIRの評価やloweringは呼びません。

現在の範囲を次のように分けます。

| 項目 | 検証内容 |
| --- | --- |
| 具体的な入力プログラム | 既存7例（increment、if・短絡、while、break／continue、入れ子、for、更新overflow）と合流・3回の入替え。さらに入替えのbackedgeをheader引数同士の直接交換にしたSSA API例 |
| 対応と完了 | 各例の全u8入力256通りで元MIR＝SSA＝変換後MIR。戻り値またはoverflowのNodeId／SourceId／Spanを比較。全例について明示上限内の完了も別に要求 |
| 評価上限 | 元MIR／SSAは64、変換後MIRは128のblock訪問。上限探索や一般の停止性の証明ではない。3経路ともfuel不足になる変更を完了扱いしない |
| 独立仕様 | 合流例は0–127で+3、128–253で+2、254–255はreturn加算で停止。入替え例は全入力で元の値。SSAの性質を対応定理で変換後MIRへ移送 |
| 誤変換の拒否 | 合流引数、loop更新、SSAの並列束縛、lowering後の並列copy、停止出自、fuel、利用者仕様を意図的に壊して証明失敗を検査 |
| 実行との照合 | Leanの3モデルの実行をHIR・元MIR・SSA・変換後MIR・VMと全入力で照合。証明と同一の生成定義を使用 |
| 観測資料 | ソース、HIR、元MIR、SSA、変換後MIR、両対応表、Lean定義・定理・実行出力・拒否例を別々に保存 |

証明は固定Lean 4.34.1のkernelで検査し、標準公理`propext`だけへの依存を完全一致で確認します。`sorry`・独自公理・`native_decide`は使いません。RustからLeanへのexporterと共有した演算モデルは監査・実行比較が必要な信頼部分です。

選択した純粋なu8関数だけが証明対象で、呼出側のprint、一般の関数呼出し、残存slot、他の数値型、heap、出力trace、Native、変換アルゴリズム全体は未証明です。未対応操作は省かず拒否します。既定の非SSA経路・観測bundle・最適化pass列は変更しません。実行比較、有限例の証明、一般の意味保存証明を区別します。
