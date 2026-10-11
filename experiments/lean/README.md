# Lean verification experiment / Lean検証実験

公開backendの前段階です。実際の完成済みHIR・MIRと、SSA・SSA経由のMIRのu8関数`increment`・`choose`・`advance`・`swap`を検証します。呼出側のprintや他の関数は証明対象に含めません。[設計](../../docs/design/lean-verification.ja.md)。

This precedes a public backend. It verifies the u8 functions `increment`, `choose`, `advance`, and `swap` from actual completed HIR/MIR and SSA/reconstructed MIR, not printing callers or other functions. [Design](../../docs/design/lean-verification.en.md).

## ファイル / Files

証明・入力・生成ツールはここにまとめます。`examples/`はCeruneの実行サンプル一覧として保ちます。
Proofs, inputs, and generation tools live here; `examples/` remains the Cerune program list.

| File | 内容 / Purpose |
| --- | --- |
| [increment.ceru](increment.ceru) | checked u8の加算 / Checked u8 increment |
| [Model.lean](Model.lean) | HIRの引数・定数・加算・overflowと出自 / HIR expression semantics |
| [MirModel.lean](MirModel.lean) | 独立した型付き局所値・命令列・CFGの評価 / Independent typed locals, instructions, and CFG semantics |
| [branch.ceru](branch.ceru)・[BranchModel.lean](BranchModel.lean) | if・短絡とそのHIR参照モデル / If/short-circuit fixture and HIR reference model |
| [branch.rs](branch.rs)・[BranchProperties.lean](BranchProperties.lean) | HIRの分岐を写す生成器と独立した仕様 / HIR branch exporter and independent properties |
| [emit.rs](emit.rs) | HIRから参照項と直接Lean定義を別々に生成 / HIR reference terms and direct Lean code |
| [mir.rs](mir.rs) | 渡されたMIR snapshotを写す。HIRから再構成しない / Exports the supplied MIR snapshot, without rebuilding it from HIR |
| [Properties.lean](Properties.lean) | 生成器と独立した仕様とHIR/MIRへの移送 / Independent properties and transfer to HIR/MIR |
| [loop.ceru](loop.ceru)・[LoopModel.lean](LoopModel.lean) | 4回の加算と、構造化while・可変局所値のHIR評価 / Four increments and structured HIR while/local semantics |
| [LoopCorrespondence.lean](LoopCorrespondence.lean) | 各ループ例で共通の対応・完了定理 / Shared loop correspondence and completion theorems |
| [loops.rs](loops.rs)・[LoopProperties.lean](LoopProperties.lean) | 評価上限を明示する生成器と単純whileの独立仕様 / Exporter with explicit bounds and plain-while specification |
| [loop_control.ceru](loop_control.ceru)・[LoopControlProperties.lean](LoopControlProperties.lean) | if内のbreak／continue、本文とreturnの停止位置 / Break/continue inside if; body/return failure origins |
| [nested_loop_control.ceru](nested_loop_control.ceru)・[NestedControlProperties.lean](NestedControlProperties.lean) | 最も内側のループへの作用 / Innermost-loop behavior |
| [for_control.ceru](for_control.ceru)・[ForProperties.lean](ForProperties.lean) | 初期化、continueの更新、breakでの更新省略 / Initialization, continue through update, break skipping update |
| [for_update_failure.ceru](for_update_failure.ceru)・[ForFailureProperties.lean](ForFailureProperties.lean) | 更新と本文のoverflowを区別する異常停止例 / Expected failure distinguishing update and body overflow |
| [main.rs](main.rs)・[lean-toolchain](lean-toolchain) | 開発ツールと固定Lean版 / Development tool and pinned checker |

## 実行 / Commands

リポジトリのルートから実行します。実行例の出力は`1 → 42 → 255`です。
Run from the repository root; the example prints `1 → 42 → 255`.

`branch.ceru`は`1 → 127 → 129 → 130 → 255`を出力します。`value < 128 && value + 128 < 255`により、128以上ではoverflowする右辺を省きます。全入力の仕様は次のとおりです。

`branch.ceru` prints `1 → 127 → 129 → 130 → 255`. The condition `value < 128 && value + 128 < 255` skips an overflowing RHS for inputs ≥128. The complete input specification follows.

| 入力 / Input | 結果 / Result |
| --- | --- |
| 0–126 | `input + 1` |
| 127–253 | `input + 2` |
| 254–255 | else側`value + 2`でoverflow / Overflow at the else addition |

`loop.ceru`は`4 → 5 → 254 → 255`を出力します。`advance`は値を4回増やし、入力0〜251では`input + 4`、252〜255では`current + 1`の途中でoverflowします。

`loop.ceru` prints `4 → 5 → 254 → 255`. `advance` increments four times: inputs 0–251 return `input + 4`; inputs 252–255 overflow during `current + 1`.

追加した制御例の全入力仕様と、正常サンプルの出力は次のとおりです。
The added control fixtures have the following complete input specifications and sample outputs.

| 入力例 / Fixture | 全入力の仕様 / Full specification | サンプル出力 / Output |
| --- | --- | --- |
| `loop_control.ceru` | 0–249: +6、250–253: returnの加算でoverflow、254–255: 本文の加算でoverflow / +6 through 249; return overflow at 250–253; body overflow at 254–255 | `6 → 7 → 254 → 255` |
| `nested_loop_control.ceru` | 0–253: +2、254–255: 内側の本文の加算でoverflow / +2 through 253; inner-body overflow at 254–255 | `2 → 3 → 254 → 255` |

forの2例は、次の結果を期待します。
The two for fixtures have the following expected results.

| 例 / Fixture | 入力別の仕様 / Specification | 実行例 / Sample |
| --- | --- | --- |
| `for_control.ceru` | 0–250: +5、251–253: returnでoverflow、254–255: 本文でoverflow / +5 through 250; return overflow at 251–253; body overflow at 254–255 | `5 → 6 → 254 → 255` |
| `for_update_failure.ceru` | 0–254: continue後の更新でoverflow、255: 本文でoverflow / Update overflow after continue through 254; body overflow at 255 | 入力1は更新で停止、出力なし、終了コード1 / Input 1 fails in update, no output, exit 1 |

異常停止例はプログラムの正常終了を期待しません。テストは停止コードと出自が仕様に一致した場合に成功します。
The failure example is expected to fail at runtime. Tests pass only when its failure code and origin match the specification.

`Cargo --example lean_verification`は開発用Rustツールの名前です。通常のCerune実行にはLean不要です。`+toolchain`はelan用で、直接Leanを使う場合は固定版を事前に用意し`CERUNE_TEST_LEAN`へパスを渡せます。テストは自動インストールしません。

`Cargo --example lean_verification` names a Rust development tool, not a language backend. Normal Cerune execution needs no Lean. The `+toolchain` form uses elan; a preinstalled matching binary can be selected with `CERUNE_TEST_LEAN`. Tests never auto-install Lean.

```sh
cargo run --quiet -- run experiments/lean/increment.ceru
cargo run --quiet -- run-mir experiments/lean/increment.ceru
cargo run --quiet -- run-vm experiments/lean/increment.ceru
cargo run --quiet -- run experiments/lean/branch.ceru
cargo run --quiet -- run-mir experiments/lean/branch.ceru
cargo run --quiet -- run-vm experiments/lean/branch.ceru
cargo run --quiet -- run experiments/lean/loop.ceru
cargo run --quiet -- run-mir experiments/lean/loop.ceru
cargo run --quiet -- run-vm experiments/lean/loop.ceru
cargo run --quiet -- run experiments/lean/loop_control.ceru
cargo run --quiet -- run-mir experiments/lean/loop_control.ceru
cargo run --quiet -- run-vm experiments/lean/loop_control.ceru
cargo run --quiet -- run experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run-mir experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run-vm experiments/lean/nested_loop_control.ceru
cargo run --quiet -- run experiments/lean/for_control.ceru
cargo run --quiet -- run-mir experiments/lean/for_control.ceru
cargo run --quiet -- run-vm experiments/lean/for_control.ceru
# 異常停止の例。終了コード1を期待する / Expected exit 1.
cargo run --quiet -- run experiments/lean/for_update_failure.ceru
cargo run --quiet --example lean_verification
lean +leanprover/lean4:v4.34.1 target/lean-verification/Verified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/BranchVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/LoopVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/LoopControlVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/NestedControlVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/ForVerified.lean
lean +leanprover/lean4:v4.34.1 target/lean-verification/ForFailureVerified.lean
cargo test --test lean_verification
cargo test --test lean_verification -- --include-ignored
```

生成先`target/lean-verification`に`Generated.lean`（モデル・参照項・生成関数・対応定理）、`Verified.lean`（独立した性質を追加）、`increment.ceir`、`increment.mir.txt`、`lean-toolchain`を保存します。元の表現、命令順・局所値、定理の前提を照合できます。生成関数とMIRモデルはHIRの`eval`を呼びません。

Outputs under `target/lean-verification` are `Generated.lean` (models, reference terms, generated function, correspondence), `Verified.lean` (independent properties appended), `increment.ceir`, `increment.mir.txt`, and `lean-toolchain`. Inspect source representations, instruction order/locals, and theorem assumptions together. Neither the generated function nor the MIR evaluator calls HIR `eval`.

分岐例も同じディレクトリに`BranchGenerated.lean`、`BranchVerified.lean`、`branch.ceir`、`branch.mir.txt`を保存します。HIRの構造化された条件と、MIRでのblock・jump・branchへの展開を比較できます。

The same directory also contains `BranchGenerated.lean`, `BranchVerified.lean`, `branch.ceir`, and `branch.mir.txt`, exposing structured HIR conditions alongside their MIR block/jump/branch lowering.

ループ例は`LoopGenerated.lean`、`LoopVerified.lean`、`loop.ceir`、`loop.mir.txt`を保存します。生成定義には`hirFuel = 16`、`mirFuel = 11`も残します。

The loop fixture adds `LoopGenerated.lean`, `LoopVerified.lean`, `loop.ceir`, and `loop.mir.txt`; generated definitions retain `hirFuel = 16` and `mirFuel = 11`.

制御例も`LoopControl{Generated,Verified}.lean`・`loop_control.{ceir,mir.txt}`、`NestedControl{Generated,Verified}.lean`・`nested_loop_control.{ceir,mir.txt}`として保存します。各定義の上限は次のとおりです。

Control fixtures also save `LoopControl{Generated,Verified}.lean` / `loop_control.{ceir,mir.txt}` and `NestedControl{Generated,Verified}.lean` / `nested_loop_control.{ceir,mir.txt}`. Their definitions record the following bounds.

| 例 / Fixture | HIRの文評価 / Statement visits | MIRのblock評価 / Block visits |
| --- | --- | --- |
| 単純while / Plain while | 16 | 11 |
| break／continue | 22 | 22 |
| 入れ子 / Nested loops | 28 | 23 |

forの生成物は`For{Generated,Verified}.lean`・`for_control.{ceir,mir.txt}`と`ForFailure{Generated,Verified}.lean`・`for_update_failure.{ceir,mir.txt}`です。上限は通常例がHIR 23文／MIR 25block、更新停止例が8文／5blockです。

For artifacts are `For{Generated,Verified}.lean` / `for_control.{ceir,mir.txt}` and `ForFailure{Generated,Verified}.lean` / `for_update_failure.{ceir,mir.txt}`. Their bounds are 23 HIR statements/25 MIR blocks and 8/5 respectively.

対応・完了証明は`LoopCorrespondence.lean`を各定義に付けて検査します。実行比較と上限直前の検査では、証明済みファイルから同じ定義をそのまま取り出し、重複した対応証明の検査を省きます。証明対象の定義を別に作り直すことはありません。

Each definition is checked with `LoopCorrespondence.lean` appended. Execution and below-bound tests use the exact definition prefix from that checked file, omitting redundant correspondence checks; they never regenerate a different proof subject.

## 証明とテスト / Proofs and tests

対象と依存は次のとおりです。Proof subjects and dependencies are explicit.

| 対象 / Subject | 保証 / Claim | 公理 / Axioms |
| --- | --- | --- |
| HIR→直接Lean / direct Lean | 全有効入力での対応、255未満の正確な加算とHIRへの移送、255のoverflow / Correspondence, exact addition below 255 and transfer to HIR, overflow at 255 | 既存4定理は空 / Existing four: none |
| HIR→MIR | `∀ x : Fin 256, evalMir mirReference x.val = .completed (eval reference x.val)` | `propext` |
| MIRの性質 / properties | 同じ正確な加算とoverflowを対応証明から移す / Transfer the same exact addition and overflow | `propext` |
| 分岐HIR→MIR / Branch HIR→MIR | 全256入力で`evalMir = .completed (evalBranch …)` | `propext` |
| 分岐の仕様 / Branch specification | 上の入力表をHIRで証明 / Prove the input table above for HIR | 空 / None |
| 分岐MIRの性質 / Branch MIR property | 対応定理から入力表の仕様を移す / Transfer the input-table specification through correspondence | `propext` |
| ループ対応・完了・MIRの性質 / Loop correspondence, completion, MIR property | 全入力で同じ結果になり、両経路とも明示上限内で完了 / Same result and completion within each explicit bound for every input | `propext` |
| ループの仕様 / Loop specifications | while・break／continue・入れ子・forの入力別仕様 / Input-specific while, break/continue, nesting, and for specifications | `propext` |

frameを加えたHIRモデルでは、以前は依存が空だった単純whileの仕様定理も`propext`を使います。検査を緩めず、現在の出力と公理一覧を一致させます。
With frames in the HIR model, the plain-while specification theorem also uses `propext`, whereas it previously had no dependencies. Checks still require the exact current axiom list.

MIRの定理は有限入力について`decide`で証明し、Lean kernelで検査します。`propext`は有限量化・等値性の決定手続きで使うLean標準の命題外延性公理です。依存を完全一致で検査し、`sorryAx`・独自公理・native評価への追加信頼は許容しません。実行結果の比較だけを証明として出力するものではありません。

The MIR theorem uses `decide` over the finite domain and is checked by Lean's kernel. `propext` is Lean's standard propositional-extensionality axiom used by finite quantification/equality decision procedures. Tests require the exact declared dependencies; `sorryAx`, custom axioms, and additional native-evaluation trust are not accepted. Runtime output comparison alone does not produce the proof.

テストは全256入力をHIR・MIR・VM・生成Lean・Lean MIRモデルと既知値で照合し、overflowのFailureCode／NodeId／SourceId／Span・空の先行出力を確認します。HIRを固定し、MIRの定数・NodeId・SourceId・Span・戻り先を改変すると対応証明が失敗します。命令順の破損や未対応操作は生成前に拒否します。既存の生成値・出自・利用者仕様の改変検査も残します。

Tests compare all 256 inputs across HIR, MIR, VM, direct Lean, the Lean MIR model, and known expectations, including overflow FailureCode/NodeId/SourceId/Span and empty prior output. With HIR fixed, changing MIR literals, NodeIds, SourceIds, spans, or returns breaks the proof. Invalid instruction order and unsupported operations are rejected before export. Existing generated-value/origin/user-specification mutation tests remain.

分岐例でも全256入力をHIR・MIR・VM・LeanのHIR/MIRモデルと既知値で照合します。分岐先の逆転、短絡右辺の無条件実行、比較の左右逆転、停止NodeIdの改変、独立仕様の127境界の改変を拒否します。`||`への置換例の対応も検査します。

The branch fixture also compares all 256 inputs across HIR, MIR, VM, Lean HIR/MIR models, and known expectations. Checks reject swapped branch targets/comparison operands, eager RHS execution, altered failure NodeIds, and changing the independent specification's 127 boundary. A variant using `||` also checks correspondence.

ループ例も全256入力を比較します。反復回数・更新store・戻り辺・停止NodeIdを壊したMIR、条件blockで循環するMIR、独立仕様の252境界の改変を拒否します。HIR/MIRの上限を1減らす場合と両方0にする場合も拒否し、fuel不足同士の一致を成功にしません。誤変換は上限付きモデルで検査し、停止しない実行ファイルを起動しません。

The loop fixture also compares all 256 inputs. Checks reject changed iteration counts, update stores, backedges, failure NodeIds, a self-looping condition block, and the independent specification's 252 boundary. Reducing either bound by one or setting both to zero is rejected; matching exhaustion is not success. Mutations run only in bounded models, never as potentially nonterminating executables.

制御例・入れ子例でも全256入力を比較します。continueを出口へ、breakを条件へ、内側の制御を外側へ飛ばすMIR、ifの分岐先逆転、独立仕様の入力境界の変更を拒否します。各上限の1つ手前では未完了であること、ループ外のbreak／continueが不正状態になることも検査します。

Both control fixtures compare all 256 inputs. Checks reject continue-to-exit, break-to-condition, inner-to-outer transfers, swapped if targets, and changed specification boundaries. They also check exhaustion immediately below each bound and invalid break/continue outside a loop.

forの全256入力でも値と停止出自を比較します。初期化の省略、continue／本文末尾から更新を飛ばすjump、breakから更新へのjump、更新の停止NodeId改変、独立仕様の境界変更を拒否します。有限で終了する誤変換にはMIRの上限を増やしても対応が成立しないことを検査します。

The for fixtures also compare values/failure origins for all 256 inputs. Checks reject skipped initialization, continue/body completion bypassing update, break executing update, altered update failure NodeIds, and changed specification boundaries. Finishing mutants are checked with an increased MIR bound.

Lean起動テストは通常実行ではignored、専用CIでは必須です。証明・改変・MIRの観測結果は`target/lean-verification-test`に残します。
Lean execution is explicitly ignored in ordinary runs and mandatory in the dedicated CI job. Proofs, rejected mutations, and MIR observations remain in `target/lean-verification-test`.

## 未対応と信頼する部分 / Limits and trust

`Nat`は数学的な計算領域で、入力は`Fin 256`、定数は0〜255、各加算は範囲検査します。`u8`を無制限整数には変えません。MIRはu8/boolのliteral・copy・scalar store・add・less、jump・branch・return・unreachableを写します。直列・分岐のexporterは未到達blockも含めて循環CFGを拒否します。ループ実験だけは循環を許し、別々の明示上限と完了証明を要求します。

`Nat` is a mathematical domain: inputs are `Fin 256`, constants are 0–255, and each addition checks its range. MIR exports u8/bool literals, copy/scalar store/add/less, jump/branch/return/unreachable. Straight-line/branch exporters reject cyclic CFGs, including unreachable blocks. Only the loop experiment allows cycles, with separate explicit bounds and a completion proof.

非循環例のMIRモデルはblock数をfuelとします。循環がなければ経路上のblock数の上界になり、対応定理は全入力で`.completed`へ到達することも検査します。`.completed (.error …)`はCeruneの停止、`.invalid`は不正状態、`.exhausted`はfuel不足です。後二者を言語上の停止や証明成功として扱いません。

HIRのframeは最も内側のループを先頭に、条件への再開先とループ後の文を保持します。ifはframeを増やしません。whileの本文末尾とcontinueは条件へ、forでは更新を一度通って条件へ進みます。breakは更新を飛ばしてそのループ後へ進みます。frameの復帰は管理処理なので文のfuelを消費しません。MIRの飛び先からHIRのframeを作ることはありません。

HIR frames store the innermost loop first, with its condition restart and statements after the loop. An if adds no frame. While body completion and continue resume the condition. For a for-loop they execute the update once before retesting. Break skips the update and resumes after that loop. Frame restoration is administrative and consumes no statement fuel. HIR frames are not derived from MIR targets.

forは初期化・条件・更新・本文を`forLess`に保持します。初期化後はモデル内部の`forNext`で反復し、初期化を繰り返しません。HIRのfuelではforの開始状態、初期化、各条件判定、更新を別々に数えます。

`forLess` retains initialization, condition, update, and body. Internal `forNext` repeats after initialization without rerunning it. HIR fuel counts for entry, initialization, each condition test, and update separately.

ループ例ではHIRは文（while／ifの条件判定、break／continueを含む）の訪問数、MIRはblockの訪問数を数えます。式・block内の有限命令列はそれぞれ構造的に評価します。16と11は単位が異なり、同じfuel値や実行時間の比較ではありません。全256入力で両モデルの完了を証明しますが、任意のloopの停止性・上限を求めるアルゴリズムの証明ではありません。

一般のloop停止性・call・出力trace・heap・所有・他整数・float等は未対応です。ここまでの有限例を非最適化MIRの基準とし、SSAの有限例の検証は下記の実験で行います。heap等の証明拡張は別途追跡します。小さな実験を最終的な言語サブセットには固定しません。

For acyclic fixtures, the MIR model uses block count as fuel, which bounds every path. Correspondence checks that every input reaches `.completed`. `.completed (.error …)` is a Cerune failure, `.invalid` an invalid state, and `.exhausted` insufficient fuel. The last two are neither language failures nor proof success.

The loop fixtures count HIR statement visits (including while/if tests and break/continue) and MIR block visits. Expressions and finite block instruction lists are evaluated structurally. Bounds 16 and 11 have different units, not a shared fuel scale or time measurement. Completion is proved for all 256 inputs, not for arbitrary loops or an algorithm that discovers bounds.

General loop termination, calls, output traces, heap/ownership, other integers, floats, and other features are not covered. Use these finite fixtures as an unoptimized MIR baseline; the experiment below checks finite SSA fixtures. Track heap and other proof extensions separately, without fixing a permanent language subset.

RustのHIR/MIRとモデルの対応、exporter、Lean kernelと明記した公理は信頼する部分に残ります。個々の実際の変換結果に対するtranslation validationであり、全loweringアルゴリズム・全処理系・Nativeの証明ではありません。

The Rust HIR/MIR-to-model correspondence, exporters, Lean kernel, and declared axioms remain trusted. This is translation validation of individual actual lowering results, not proof of the general lowering algorithm, whole compiler, or Native output.

## SSAの検証 / SSA verification

元MIR・SSA・変換後MIRをそれぞれ実際のsnapshotからLeanへ写します。SSAのCFG評価はMIRへの変換を呼ばず、辺の値をすべて読んでからblock引数へ束縛します。演算と命令列の評価はMIRモデルを共有します。

Actual baseline MIR, SSA, and reconstructed MIR snapshots are exported separately. Independent SSA control reads all incoming values before binding block arguments; operations and instruction evaluation share the MIR model.

追加ファイルの役割は次のとおりです。The added files have these roles.

| File | 内容 / Purpose |
| --- | --- |
| [ssa.rs](ssa.rs)・[SsaModel.lean](SsaModel.lean) | SSAの実際の値・命令・辺を写すexporterと独立した制御評価 / Actual SSA exporter and independent control semantics |
| [SsaCorrespondence.lean](SsaCorrespondence.lean) | 元MIR＝SSA＝変換後MIRと全入力での完了 / Correspondence and completion for every input |
| [ssa_join.ceru](ssa_join.ceru)・[SsaJoinProperties.lean](SsaJoinProperties.lean) | 更新値の合流と独立仕様。出力は3 → 130 → 130 → 255 / Join and independent specification; prints 3 → 130 → 130 → 255 |
| [ssa_swap.ceru](ssa_swap.ceru)・[SsaSwapProperties.lean](SsaSwapProperties.lean) | 3回の入替えと独立仕様。出力は0 → 127 → 255 / Three swaps and independent specification; prints 0 → 127 → 255 |
| [lean_ssa.rs](../../tests/lean_ssa.rs) | 全入力の証明・実行比較と誤変換拒否 / All-input proofs, execution comparisons, and mutation rejection |

既存7例＋合流＋入替え＋header引数を直接交換するSSA API例の10ケース、各256入力を検査します。元MIRとSSAは64、変換後MIRは128のblock訪問を上限とし、正常終了か言語上のoverflowへの到達を別定理で確認します。元MIRの完了を証明し、対応定理からSSAと変換後MIRの完了を導きます。全ケースで停止理由とNodeId／SourceId／Spanを保持します。呼出側のprint・heap・一般の停止性・変換器全体の証明は含みません。

Ten cases (seven existing fixtures, join, swap, and direct header-argument swapping via the SSA API) each cover all 256 inputs. Bounds are 64 block visits for baseline MIR/SSA and 128 for reconstructed MIR; a separate theorem requires success or language-level overflow by proving baseline completion and transferring it through the checked correspondences, retaining failure code and NodeId/SourceId/Span. Printing callers, heap, general termination, and the entire transformer are outside the proof scope.

実行方法は次のとおりです。Run from the repository root.

```sh
cargo run --quiet -- run experiments/lean/ssa_join.ceru
cargo run --quiet -- run-mir experiments/lean/ssa_join.ceru --ssa
cargo run --quiet -- run experiments/lean/ssa_swap.ceru
cargo run --quiet -- run-mir experiments/lean/ssa_swap.ceru --ssa
cargo test --test lean_ssa
# 固定Leanが必要 / Requires the pinned Lean toolchain.
cargo test --test lean_ssa -- --include-ignored
```

通常テストはLeanを起動せず、生成時の対応範囲と5実行経路のサンプルを確認します。ignoredテストはLean 4.34.1で証明と誤変換拒否を検査し、3つのLeanモデルの実行をHIR・元MIR・SSA・変換後MIR・VMと全入力で照合します。CIでも必ずignoredテストを実行します。各Leanプロセスは120秒で打ち切り、タイムアウトは検証成功に数えません。

Normal tests check export scope and sample execution across five routes without Lean. The ignored tests use Lean 4.34.1 to check proofs and rejected mutations, then compares execution of all three Lean models against HIR, baseline MIR, SSA, reconstructed MIR, and VM for every input. CI explicitly runs them. Every Lean process has a 120-second limit; timeouts fail validation.

観測資料は`target/lean-verification-test/ssa`に保存します。各ケースの`.ceru`、`.ceir`、元MIR、SSA、変換後MIR、両対応表、`Generated.lean`、`Verified.lean`、`Execute.lean`、拒否例、プロセス出力、toolchain版を比較できます。`Parallel`だけは自動生成SSAのbackedgeを等価な直接交換に変更したAPI例で、その変更をSSAと対応表で確認できます。

Observations live under `target/lean-verification-test/ssa`: source, HIR, baseline MIR, SSA, reconstructed MIR, both mappings, generated/proved/executed Lean definitions, rejected cases, process output, and toolchain version. Only `Parallel` changes the generated SSA backedge to an equivalent direct exchange through the API; the SSA snapshot and mapping expose that change.

対応・完了・追加仕様の定理は`decide`で証明し、kernel検査と`#print axioms`の完全一致（標準`propext`のみ）を要求します。`native_decide`や独自公理は使いません。Rust exporterと共有演算モデルは信頼部分として明記し、有限例の証明を一般の意味保存証明と区別します。詳しくは[SSA設計](../../docs/design/mir-ssa.ja.md#leanによる有限例の検証)を参照してください。

Correspondence, completion, and added property theorems use `decide`, kernel checking, and exact axiom-list checks (standard `propext` only). No `native_decide` or custom axioms. Rust exporters and shared operation semantics remain trusted; these finite-fixture proofs are distinct from general semantic preservation. See the [SSA design](../../docs/design/mir-ssa.en.md#finite-fixture-lean-verification).
