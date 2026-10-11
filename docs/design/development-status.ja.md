# 実行経路と未完了作業の棚卸し

[English](development-status.en.md) · [ロードマップ](language-roadmap.ja.md)

このページは下記時点の監査記録です。その後、[LIRの観測・構造検証](lir-observation.ja.md)を実装し、直接実行器は保留としました。その後の言語機能として`array_repeat`も実装しました。現在の優先順位は[ロードマップ](language-roadmap.ja.md)を参照してください。

## 調査基点と結論

2026-10-11、masterの `763a9fb`（PR #120統合）と、PR #121の `9904461` を比較しました。#121の定数畳み込み設計・基準example・優先順位整理は、監査時点ではPR上の変更です。最適化passそのものはありません。

**LIRはNative生成の内部段階として実装済みです。LIRの直接実行、独立した公開観測形式、専用の構造検証器は未実装です。** 「Nativeが動く」「LIRを表示できる」「LIRを直接評価できる」は別の到達点です。前回の優先順位表はこの区別が不足していました。

## 現在の流れ

以下の実線は実装済みの変換と実行です。各実行器は受け取った表現を実行し、前の実行器の結果を入力にするわけではありません。

```text
Source → 共通frontend → HIR ───────────────→ IR Executor（run / run-ir）
                         ├→ Bytecode ─────→ VM（run-vm）
                         ├→ C / LLVM / QBE / WAT → emit（後続ツールで実行）
                         └→ 非SSA MIR ─────→ MIR Executor（run-mir）
                               ├→ SSA ─────→ SSA Executor（run-mir --ssa）
                               │    └→ copy付きMIR ─┐
                               └────────────────────┴→ x86-64 LIR
                                                        ↓
                                                       ASM
                                                        ├→ 外部アセンブラ → Object
                                                        └→ 自前エンコーダ → Object
                                                                              ↓
                                                                         外部リンク → CPU実行
```

Nativeの外部リンク・起動は現在、利用者またはテスト／観測スクリプトが行います。公開CLIのbuildではありません。SSAはMIRの変換形式、Windows/Linuxはtarget、ASM/Objectは成果物なので、単純に全部を同じ「経路数」へ足しません。

| 段階 | 表現・変換 | 単独の観測 | 直接評価・実行 |
| --- | --- | --- | --- |
| Source／AST | frontendで解析・展開 | `emit-sources`は本文とファイル対応。AST専用出力はない | AST Executorは未採用。調査では当面追加しないことを推奨 |
| HIR | 型・名前・展開・所有の意味が確定 | `emit-ir` | IR Executor |
| 非SSA MIR | 一時値・block・明示制御フロー | `emit-mir` | MIR Executor |
| SSA | scalar SSAと残存slot、MIRとの対応 | `emit-mir --ssa`、対応表、bundle | SSA Executor。Nativeへ進む場合はcopy付きMIRへ戻す |
| Bytecode | stack VM命令と出自 | `emit-bytecode` | VM。Rustからの関数呼出しAPIもあり |
| x86-64 LIR | target・frame・stack・レジスタ操作・疑似命令 | RustのModuleとASM注釈はある。専用のテキスト出力／bundle内snapshotはない | LIR Executor／`run-lir`はない |
| ASM／Object | Windows/Linux ASM、自前COFF/ELF | `emit-asm`／`emit-obj`、注釈・シンボル・逆アセンブル | リンク後にCPUで実行 |
| C／LLVM／QBE／WAT | HIRから各backendの表現へ変換 | 各`emit-*` | 外部toolchain/runtimeで検証。現行CLIは起動を管理しない |
| Lean | 有限例のモデル・定理を実験用exporterで生成 | `experiments/lean`の成果物 | 外部Leanで検査。公開`emit-lean`と全言語の証明はない |

MIR実行・SSA実行があることは、MIR/SSA向けbuild成果物や新しい配布routeの採用を意味しません。現行のbuild/release案はIR・VM・Nativeを対象とします。

## LIRに何があり、何が残るか

根拠は [LIRの型](../../src/codegen/x86_64/ir.rs)、[公開Rust API](../../src/codegen/x86_64.rs)、[lowering](../../src/codegen/x86_64/lower.rs)、[emitter](../../src/codegen/x86_64/emit.rs)、[Native設計](native-mir.ja.md)です。

| 項目 | 現状 |
| --- | --- |
| MIR→LIRの分離 | `x86_64::lower_mir(&mir, target)`がModuleを返す。入力MIRを検証し、明示targetの配置・ABIを決める |
| LIR→ASMの分離 | emitterがModuleを読む。MIRから直接文字列を出すだけの一体処理ではない |
| 出自 | 命令ごとのOriginとMirOrigin。ASMの`cerune-mir`注釈、Objectの対応シンボルへ伝える |
| 命令の粒度 | `MoveRaxToRcx`等に加え、`CheckIntegerRange`・`ArrayAllocate`・`Call`等がある。検査・ABI・runtime処理をemitterがさらに展開する |
| 検証 | MIR検証、emitterの一部整合性assert、生成物・Native実行テストはある。LIR全体の独立した構造検証器はない |
| 公開観測 | `emit-lir`、LIRテキストschema、bundleのLIR snapshotはない。ASM注釈の命令番号だけではLIR本体を一覧できない |
| 独立実行 | LIRを直接解釈する実行器はない。MIR実行やNative起動をLIR Executorとは呼ばない |
| 完全な機械命令IR | 未実装。Objectはemitterが出す内部ASMを読み取って符号化する |

したがってLIRは「未着手」でも「独立した観測・実行まで完成」でもありません。変換段階の分離はできていますが、公開観測と検証の整理が残っています。

### LIR直接実行を追加する価値と費用

LIRを独立評価できれば、MIR→LIRの誤りと、LIR→ASM／Object以降の誤りを切り分けやすくなります。ただしレジスタ、frame、メモリ、分岐、検査、呼出規約、runtime操作のどこまでをモデル化するか決める必要があります。

LIRの疑似命令を正しく評価できても、emitterが展開するx86-64命令、符号化、再配置、実リンカ、OS ABIの正しさは保証されません。Native実行比較は引き続き必要です。MIRやVMへ戻して実行すると、この独立した検証にはなりません。

**提案は、まずLIRの観測・構造検証を設計し、直接実行の採否を別に判断することです。** [#123](https://github.com/Cerune-Lang/Cerune/issues/123)で追跡します。LIR Executor、型付き機械命令IR、自前リンカ、JITを一括で実装する計画にはしません。

## 優先順位の組み直し

作業を目的ごとに分けます。一列に並べて、前の大きな研究課題が終わらないと機能追加できない形を避けます。

| 優先 | 作業 | 先に必要なこと／完了の目安 |
| --- | --- | --- |
| 今回の整理 | 現状表・資料の食い違いを修正 | SSA実装済み、LIR観測／実行未実装、build未実装を区別 |
| 次の設計確認 | LIR観測・構造検証と直接実行の採否（#123） | 同じModuleから命令本体・target・frame・出自を保存する範囲を決める。直接実行の完成は他作業の条件にしない |
| 次の言語機能 | `array_repeat`（#122） | 既存案を確認し、0個・不正個数・予算・入れ子コピー・生成過程を全既存経路で照合 |
| 実用上の優先 | 標準入出力＋副作用比較（#89／#104）、必要な標準ライブラリ | 対話出力・flush、EOF・失敗・資源上限、同じ入力と操作列。小さな対話例から始める |
| 独立して優先 | Native build（#81） | 完成成果物、明示target・リンカ・runtime依存・失敗・manifest。現行プログラムで開始できる |
| 上の作業に併せる | 観測成果物の扱い（#118）、必要箇所のコード整理（#103）、QBE対応の維持（#92） | 出力・停止・出自を変えず、観測資料の共有と使用revisionを明示 |
| 用途から選ぶ追加候補 | 明示wrapping等（#88）、束縛・条件構文（#90／#105） | 実用途はIssueに記録済み。通常のchecked算術を維持。表記改善と新しい意味を区別する |
| 依存が大きい後続 | IR/VM Image・release、再帰、配列成長・借用、文字列操作、モジュール配布 | 検証／所有／資源／互換性をそれぞれ設計。Native releaseはIR/VMの完成を一律に待たない |
| 当面保留・研究 | 最適化pass、一般の証明、AST実行、LIR直接実行の実装、GPU／FPGA | 採否・対象を個別判断。機能追加の必須前提にしない |

LIR観測はCeruneの可観測性に直接関係するため、前回の表へ明示的に戻します。一方、LIR実行器の大きな実装を次の必須課題と断定する根拠はまだありません。#122は引き続き次の言語機能で、LIRの設計確認と別に進められます。

## 言語側に残る主な不足

実行経路が同じ意味を扱えることと、言語として必要な機能が揃ったことは別です。[現行リファレンス](../reference/language.ja.md)と個別設計から、未対応を用途別にまとめます。以下をすべて直ちに採用する決定ではありません。

| 分野 | 主な未対応 |
| --- | --- |
| 入出力・ライブラリ | stdin・CLI引数・一般のstderr書込み・ファイル／外部連携、標準moduleの配置・探索・互換方針 |
| データ操作 | array_repeat、動的配列の成長・借用、文字列の添字・文字数・数値読取り、0長固定配列・空リテラル |
| 関数・型 | 再帰、関数値、汎用の型定義・trait・型引数推論、再帰的な値型 |
| 失敗の処理 | 汎用Option/Result、暗黙のエラー伝播、実行時停止の捕捉。既存enumで成功／失敗を値として返すことは可能 |
| 長いソースの記述 | optional let、infer省略、else if、条件式、formatter。全部が新しい言語意味を必要とするわけではない |
| 配布・連携 | 再export・パッケージ管理、IR/VM Image、build/release、show／via、optional backend |

これらの不足を、AST／LIR実行器や最適化の追加だけで解消することはできません。用途を広げる仕事と、変換を観測・比較する仕事を分けて選びます。

## Issueを全部どこへ置くか

監査開始時のopen Issueは14件、closed Issueは#2の1件です。PRとは分けて確認しました。

| 分類 | Issue | 今回の位置づけ |
| --- | --- | --- |
| 観測・変換 | [#60](https://github.com/Cerune-Lang/Cerune/issues/60)、[#94](https://github.com/Cerune-Lang/Cerune/issues/94) | 部分完了。SSA bundle／有限例証明を反映し、LIR残件を明示。最適化は保留 |
| 完成成果物 | [#81](https://github.com/Cerune-Lang/Cerune/issues/81) | build／release、show／via、Image、optional backendはそれぞれ未実装。全部を一括前提にしない |
| 入力と比較 | [#89](https://github.com/Cerune-Lang/Cerune/issues/89)、[#104](https://github.com/Cerune-Lang/Cerune/issues/104) | stdinを最初の用途にし、通常実行と比較の外部接続を一緒に設計 |
| 言語機能 | [#122](https://github.com/Cerune-Lang/Cerune/issues/122)、[#88](https://github.com/Cerune-Lang/Cerune/issues/88) | array_repeatが次。wrapping等は実用途に基づく別の候補 |
| 構文 | [#90](https://github.com/Cerune-Lang/Cerune/issues/90)、[#105](https://github.com/Cerune-Lang/Cerune/issues/105) | let、infer省略、else if、条件式を区別。formatterも未実装 |
| 保守 | [#92](https://github.com/Cerune-Lang/Cerune/issues/92)、[#103](https://github.com/Cerune-Lang/Cerune/issues/103)、[#118](https://github.com/Cerune-Lang/Cerune/issues/118) | QBE、必要箇所の分離、ローカル観測と公開fixtureの扱い |
| 配布方針 | [#119](https://github.com/Cerune-Lang/Cerune/issues/119) | ライセンス変更の検討。現行MITを変更する決定ではない |
| 実験 | [#93](https://github.com/Cerune-Lang/Cerune/issues/93) | FPGA／Cisa ISAの構想。実装経路に数えない |
| 完了した過去の計画 | [#2](https://github.com/Cerune-Lang/Cerune/issues/2) | 関数・制御・集約型の初期IR計画。現在の未実装一覧として読まない |

本監査で#123を追加しました。Issue #92の上流patchは既存コメントと保留中の研究ブランチの記録として確認しました。今回、最新QBEへの更新や回避処理の削除、研究ブランチの統合は行っていません。

## 確認した資料と修正点

監査開始時に、追跡対象のdocs全83文書（日英）、README／SECURITY、exampleの説明12文書、実験資料4文書、スクリプト4件、CLI・各IRの公開API・Native lowering／emitter／encoder・bundle・対応テストとCIを棚卸ししました。実装判断に関係する記述をコードと照合し、全テストや全ソース行の再レビューとは区別しています。

| 資料 | 確認結果 |
| --- | --- |
| architecture | SSAを「計画・未実装」とする古い行と、SSAが抜けた経路図を修正 |
| 段階設計・Native設計・targets・CLI・bundle | LIR内部段階は記載済みだが、独立観測／実行がないことを明記する必要があった |
| native-encoder | 旧図がHIR→MIRを省略していたため現在の流れへ修正 |
| roadmap | LIRの残件、wrapping・構文等の別系統の候補を含む本資料へ接続 |
| docs／experimentsの索引 | 状況整理・定数畳み込み・source-files・SSA実験への入口を補う。日英の文章を別ファイルへ分ける |
| Lean／SSA実験 | 有限例の検証と公開backendを区別。証明範囲をLIR／機械語まで拡大しない |
| GitHub | 全Issue本文・コメント、open PR #121、統合済みPRの履歴を確認。Release一覧は空。Wiki用remoteは取得できず、Wiki本文は確認していない |
| local-notes | 未合意の非公開Draftで、旧名・過去の実装状態を含む。現行仕様の根拠にせず、公開資料へ転記しない |

実物の確認では `native_calls.ceru` の `observe --ssa` をLinux target指定で生成し、10ファイルのbundleとASM中のMIR→LIR対応を確認しました。LIR専用ファイルはありません。これは生成・観測の確認であり、この監査でLinux実行を再検証したという意味ではありません。

今後の現状は[ロードマップ](language-roadmap.ja.md)、実装済みCLIは[リファレンス](../reference/cli.ja.md)、各作業の採否と完了条件はIssueで更新します。本資料は監査時点の根拠と判断を残す記録です。
