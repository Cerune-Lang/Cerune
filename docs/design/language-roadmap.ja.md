# 言語機能の整理と今後の順序

[English](language-roadmap.en.md)

2026-10-20時点の機能・実行経路・次の作業をまとめます。「実装済み」と「計画・未実装」を区別し、候補の構文や採用は各段階の設計で決めます。正確な仕様は[言語リファレンス](../reference/language.ja.md)を参照してください。

[以前の監査記録](development-status.ja.md)は2026-10-11時点の履歴です。現在の状態はこのページで管理します。

## Ceruneが持つべき性質

Ceruneでは、書いた計算の意味と、それが実行される表現へ変わる過程を説明できることを優先します。機能を増やす際も、次を維持します。

- 型、評価順、短絡評価、異常停止の条件を明示する。暗黙の数値変換で値を失わない。
- 値をコピーした後の独立性を保つ。`mut`による更新と、別の値への隠れた変更を混同しない。
- 文字列は不変なUTF-8の値とし、日本語・NUL・CR/LFを保持する。Unicodeを正規化しない。
- ソース、Cerune IR、生成物の対応を観測できるようにする。観測のために実行中の状態を書き換える窓口は作らない。
- ターゲット、生成物を処理する外部ツール、対応機能を明示する。実行中のOSから仕様を黙って選ばない。

## 現在の言語機能

| 項目 | 現在持っているもの | 制限・境界 | 実行できる例 |
| --- | --- | --- | --- |
| 符号付き整数 | `i8`、`i16`、`i32`、`i64`、算術・比較・ビット演算 | 範囲外演算は停止。小さい型も現在の格納領域は64ビット | [sensor_calibration](../../examples/sensor_calibration.ceru)、[integer_limits](../../examples/integer_limits.ceru) |
| 符号なし整数 | `u8`、`u16`、`u32`、`u64` | `u64`は0〜18446744073709551615。暗黙の符号変換や折り返しなし | [u64_values](../../examples/u64_values.ceru)、[packet_counter](../../examples/packet_counter.ceru) |
| 浮動小数点 | `f32`、`f64` | 演算には各精度の丸めがある。`convert`は値を保持、整数化では丸め方と飽和を明示 | [floating_point](../../examples/floating_point.ceru) |
| 真偽値 | `bool`、比較、`!`、短絡評価する`&&`・`||` | 数値との暗黙変換なし | [short_circuit](../../examples/short_circuit.ceru) |
| 文字列 | `string`、表示、`==`・`!=`、`byte_len`、`concat` | 添字参照・文字数・数値変換は未実装 | [string_byte_length](../../examples/string_byte_length.ceru)、[string_lookup](../../examples/string_lookup.ceru) |
| 固定長配列 | `[T; N]`、`array_len`、`for … in`、入れ子、値渡し、`mut`な要素の更新 | 長さは型の一部。添字は`i64`。動的長には`[T]`を使う。借用スライスなし | [array_iteration](../../examples/array_iteration.ceru)、[heat_diffusion](../../examples/heat_diffusion.ceru) |
| 動的配列 | `[T]`、明示コピー・範囲コピー・repeat、独立した入れ子、表示・比較・反復 | IR・MIR・VM・C・LLVM・QBE・WAT・ASM・自前Object対応。`array_repeat`も共通IRへ展開 | [copy](../../examples/dynamic_arrays/copy.ceru)、[nested](../../examples/dynamic_arrays/nested.ceru) |
| 名前付きproduct type | フィールド、既定値、更新式、入れ子、値渡し | フィールドの直接代入なし。新しい値を構築して全体を再代入 | [product-point](../../examples/product-point.ceru)、[packet_counter](../../examples/packet_counter.ceru) |
| 関数と制御構文 | 型付き引数・戻り値、型・長さの明示パラメーター、`void`、`if`・`else`、`while`・`for`、`break`・`continue`・`return` | 引数数の固定上限なし。再帰なし | [function_values](../../examples/function_values.ceru)、[loop_control](../../examples/loop_control.ceru) |
| 束縛と変換 | 既定で不変、`mut`、明示的な`infer`、`T(value)`と`convert<T>(value)` | `infer`は実行時型ではない。浮動小数点から整数への全5丸め方と明示的な飽和に対応 | [integer_conversions](../../examples/integer_conversions.ceru) |
| 直和型 | `enum`の値付き選択肢、網羅的な`match`文・式とガード | 汎用Option/Resultは未実装 | [sum_lookup](../../examples/sum_lookup.ceru) |
| コンパイル時定数 | 型付き`const`、依存式の評価、配列型の長さ、`pub const` | 関数呼び出し・ブロック内宣言なし | [constants](../../examples/constants.ceru) |
| モジュール | 明示的なimport・名前空間・関数/型/定数のpub指定 | 循環・非公開参照を診断。再export・モジュール変数・パッケージ配布なし | [modules](../../examples/modules/README.md) |

型の数値範囲と用途別サンプルは[examplesの型別表](../../examples/README.md#型から探す)にまとめています。配列・構造体・直和型全体の[等値比較と表示](aggregate-values.ja.md)にも対応します。

## 言語機能と出力経路を分ける

動的配列を含む上記機能を[IR Executor](ir-executor.ja.md)、[MIR Executor](mir-executor.ja.md)、VM、生成C、LLVM、QBE、WAT、Windows/Linuxの直接ASM、および自前エンコーダのオブジェクトで扱えます。Windows/Linuxはターゲットの違い、ASM/オブジェクトは成果物の違いです。[経路とターゲットの表](targets.ja.md#現在の構成)を基準に数えます。

自前エンコーダはx86-64命令とCOFF/ELFを生成します。ASMと共通のloweringを使い、現在は内部ASM表現を読み取って符号化します。リンクは外部ツールです。将来の型付き機械命令IRや自前リンカはコンパイラ実装の候補であり、新しい言語機能ではありません。[自前エンコーダの設計](native-encoder.ja.md)を参照してください。

言語機能を扱えることと、観測の詳しさが等しいことは別です。言語の検査失敗は全経路で共通の停止理由・ソース位置・先行出力を照合できます。ただし、LLVM/ASMの出自注釈は全経路・全最適化段階のデバッグ情報を保証するものではありません。

## 実行・配布・証明の基盤

言語機能と、コンパイラの段階・配布・証明を分けて管理します。新しい分類や証明実験だけで、言語機能や生成経路の未対応を実装済みとはしません。

| 状態 | 基盤 | 到達点・残る作業 |
| --- | --- | --- |
| 実装済み | [MIR](ir-stages.ja.md)・[独立実行器](mir-executor.ja.md) | 非SSAの型・変換・検証・`emit-mir`・`run-mir`。既存経路と出力・停止理由・出自・所有の寿命を比較 |
| 実装済み | [NativeのMIR入力](native-mir.ja.md) | 旧経路との実行比較、MIR→LIR→ASM/Objectの出自対応を検証 |
| 初期範囲を実装 | [観測bundle](observation-bundle.ja.md) | 同一コンパイルのsources・HIR・MIR・注釈付きASM・manifestを保存。`--ssa`でSSA・変換後MIR・対応・SSA経由ASMも追加。その他の成果物は後続（[#60](https://github.com/Cerune-Lang/Cerune/issues/60)） |
| 一部実験済み | [Lean検証](lean-verification.ja.md) | u8のincrement・choose・advanceでHIR→Lean／HIR→MIRの対応と性質を検証。if・短絡・while／for・break／continue・入れ子・更新停止出自と明示上限内の完了まで対象。全言語・全lowering・処理系全体の証明や公開`emit-lean`は未実装 |
| 一部実装済み | [SSA変換](mir-ssa.ja.md)・後続の最適化pass | scalar SSAと残存slotの表現・構造検証・観測テキスト・自動変換・Rust APIでの直接実行を実装。公開CLI・bundleもSSAに対応。SSA→MIR／Nativeも対応。MIR→SSA→MIRの有限例のLean検証も対応。定数畳み込みは設計済み・未実装で当面保留。一般の証明も後続 |
| 実装済み | [LIRの観測・構造検証](lir-observation.ja.md) | `emit-lir`・`observe --lir`・検証器・両targetのexample比較。直接実行は調査を終えて保留 |
| 配布済み | コンパイラのバイナリ v0.1.0 | Windows x64、Linux x64/arm64、macOS x64/arm64向けに公開。Ceruneプログラムのbuild/releaseやDarwin Native backendの実装とは別 |
| 計画・未実装 | [IR・VM・Nativeのbuild / release](owned-routes.ja.md) | [#81](https://github.com/Cerune-Lang/Cerune/issues/81)の責務整理を基準に、完成成果物・配布を最適化と分ける |

## 次に持つべきもの：提案する順序

SSAは構築・直接実行・Nativeへの変換・有限例のLean検証までで一区切りにします。[定数畳み込み](ssa-constant-folding.ja.md)は設計と基準exampleを保存し、passの実装は当面の優先作業から外します。非最適化MIR、明示的なSSA選択、既存の観測と比較テストは維持します。

利用者が書ける処理を増やすことと、完成成果物を使えることを優先します。以下は作業の優先順であり、全項目が直前の完了を必要とするわけではありません。

| 優先 | 作業・現在の状態 | 最初の完了条件 |
| --- | --- | --- |
| 完了した区切り | [LIR観測・構造検証と直接実行の採否](lir-observation.ja.md)（[#123](https://github.com/Cerune-Lang/Cerune/issues/123)） | 基準／SSA経由のLIR表示・保存・構造検証とexample比較を実装。直接実行器は保留し、次の言語機能へ戻る |
| 実装した機能 | [動的配列の `array_repeat`](owned-arrays.ja.md)（[#122](https://github.com/Cerune-Lang/Cerune/issues/122)） | 実行時の個数、0個・負の個数、評価順、予算超過、入れ子の独立コピーを実装。example・生成過程・既存経路の比較テストを追加 |
| 完了 | 文書とIssueの整合性（[#130](https://github.com/Cerune-Lang/Cerune/issues/130)） | 現行状態と監査履歴を分離し、リンク・日英ペアを自動確認する |
| 完了 | 文字列ヒープの所有者検証（[#134](https://github.com/Cerune-Lang/Cerune/issues/134)） | 同じIDでも異なるヒープの値を拒否し、retain/release失敗で所有数・liveバイト数・内容を変えない。通常のソースからの再現は未確認 |
| 完了した初版 | [標準入出力](standard-io.ja.md)（[#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#104](https://github.com/Cerune-Lang/Cerune/issues/104)） | 4実行器の出力・flush・行入力と、CLI/C/LLVM/QBE/WAT/Nativeの接続・失敗・対話試験を実装。Windows consoleや生成経路の成功操作の共通記録は後続 |
| 1：完了 | [浮動小数点の丸め](float-rounding.ja.md)（[#156](https://github.com/Cerune-Lang/Cerune/issues/156)） | PR #163で統合。最小例・ビット列・生成ASMでFMA差分を確認し、Clang/GCCの生成Cへ収縮禁止と回帰テストを追加 |
| 2：初版実装 | [コンパイル入力の上限](frontend-limits.ja.md)（[#153](https://github.com/Cerune-Lang/Cerune/issues/153) / [#135](https://github.com/Cerune-Lang/Cerune/issues/135)） | 構文・式の深さ、個別／合計バイト数、ファイル数を診断。import探索は順序を保って反復化。#153の隔離・C・CI候補は残件 |
| 3：初版の判断・実装済み | [入力容量の測定](input-memory.ja.md)。行入力の確保容量とメモリ上限（[#154](https://github.com/Cerune-Lang/Cerune/issues/154) / [#133](https://github.com/Cerune-Lang/Cerune/issues/133)） | 全経路で容量・使用バイト数・解放を測定し、同じexampleの出力を比較。[Rust/C/LLVM/QBE/Nativeの縮小](input-compaction.ja.md)を実装。[WATの比較](wat-input-storage.ja.md)で分割・再結合は導入せず、既存の再利用を維持と判断。広いメモリ調査は#133へ |
| 4：i32/f64公開実装済み | [JS/TSから型付き関数を呼ぶ](typed-host-calls.ja.md)（[#161](https://github.com/Cerune-Lang/Cerune/issues/161)） | [WATライブラリAPI](wat-library.ja.md)で宣言保持・明示選択・全体検査・i32/f64 wrapperを実装。元IRと型変換を観測可能。[検査付きJS/TS生成](wat-js-bindings.ja.md)とHTML例も実装済み |
| 5：次の設計 | [引数と整数の読取り](dogfooding-inputs.ja.md)（[#173](https://github.com/Cerune-Lang/Cerune/issues/173) / #89） | Parallaxの固定seedを引数で指定する。固定seedの基準exampleは実装済み。引数取得・純粋な整数読取り・全経路比較は未実装。小さな標準モジュールとして設計し、汎用Option/Resultやパッケージ管理を前提にしない |
| 独立して着手可能 | [Nativeのbuild](owned-routes.ja.md)。責務は整理済み、CLI・リンク管理は未実装 | 最初の成果物候補は実行ファイル。明示target・外部リンカ・依存物・失敗・観測記録を決め、既存exampleを生成・リンクしてIR/VMと照合する |
| build安定後 | IR/VMのImageと3経路のrelease。未実装 | 版・型・参照・所有・予算を検証してloadし、保存前後で比較。各経路の完成成果物が安定した段階で配布構成・互換条件・停止出自を検証する |
| 後続 | 再帰、配列の成長・借用、文字列操作の拡張、モジュール配布 | 再帰の呼出し領域・上限・終了処理、借用の寿命・更新規則、再export・依存の版・再現性を用途ごとに設計する |
| 保留・実験 | 最適化pass、証明範囲の拡張、AST／LIR直接実行の追加、GPU／FPGA | 具体的な必要性から再開。言語機能・入出力・buildの着手条件にはしない |

Native buildは新しい入力や標準ライブラリがなくても、現在のプログラムを対象に進められます。`array_repeat`は実装済みです。新しい調査課題をすべて入出力の前提にはしません。releaseは最適化の別名ではありません。IR/VMのImage検証を省略して、観測テキストを実行可能な完成品と扱うこともしません。

### 標準入出力と標準ライブラリの現状

現在できることと、次の設計で決めることを分けます。

| 項目 | 現状 | 次に決めること |
| --- | --- | --- |
| 標準出力 | `print`で値を表示できる。`run`／`run-mir`／`run-vm`は実行中に接続先へ出力し、最後にflush。収集型のRust APIも維持 | `flush();`は実装済み。C・LLVM・QBE・WAT・Nativeの書込み失敗も実装済み。質問を表示してから入力を待つ対話テストも実装済み |
| 標準エラー | 処理系の診断に使用。プログラム用の一般的な書込みAPIはない | プログラムの出力と処理系の停止診断を区別し、書込み失敗も定義する |
| 標準入力・引数 | io::read_lineと行／EOFのenumを4実行器・CLI・生成C・LLVM・QBE・Native、WATの版付きhostで利用可能 | Windows console対応が残る。CLI引数・ファイル・FFI・ネットワークは後続 |
| 標準ライブラリ | std:ioは同梱仮想ソース。型・本文・出自を観測でき、ファイル探索しない | 言語の組込み、runtimeの外部接続、Ceruneで書くライブラリを区別する |

[標準入出力の設計](standard-io.ja.md)に責務と実装順を整理しました。[入力値の分岐例](../../examples/io_protocol/README.md)は実行できますが、標準入力は使いません。[行入力の契約と共通Rust接続](line-input.ja.md)を追加しました。初版は行／EOFを返し、異常時停止とします。既存enumで表現でき、汎用Resultは不要です。仮想std:ioのread_line／ReadLineを4実行器へ接続済みです。CLI/C/LLVM/QBE/Nativeの実stdin接続は[初版の範囲](stdin-connections.ja.md)で実装済み、WATの版付き入力hostは実装済み。Native入力も実装済みです。比較テストには同じ入力と制御できる出力先を使い、実環境での接続確認を分けます。record/replayや汎用effect systemの一括実装は前提にしません。

[固定容量の使用数](../../examples/bounded_sequence.ceru)と[範囲コピー](../../examples/array_window.ceru)の例は現行機能で実行できます。動的配列や借用スライスの実装済みサンプルではありません。

既存の基盤は[共通の停止記録](runtime-diagnostics.ja.md)、[ファイルの出自](source-files.ja.md)、[モジュール](modules.ja.md)、[定数](constants.ja.md)、[関数](functions.ja.md)、[構造体更新](product-updates.ja.md)、[直和型](sum-types.ja.md)、[固定長配列](fixed-arrays.ja.md)です。[混在引数](../../examples/function_arguments.ceru)・[配列長](../../examples/array_length.ceru)の例で値渡しと評価順を確認できます。

この表は一括実装の約束ではありません。GPU実験は上記すべての完了を前提にしません。継承、暗黙の共有可変参照、自動GPU振り分け、汎用非同期処理は、現在の例から必要性が確認できていないため先行させません。

### 現在のIssue一覧

2026-10-20にopenの17件と直近の統合、Organizationの進捗を確認し、引数取得#174・整数読取り#175を分割しました（分割後19件）。以下は優先度と目的の整理です。調査Issueを確認済みの不具合として扱いません。

| Issue | 目的 | 進め方・残件 |
| --- | --- | --- |
| [#156](https://github.com/Cerune-Lang/Cerune/issues/156) | 浮動小数点の意味の一致 | PR #163で完了。Clang/GCCの収縮対策、example・回帰テスト・日英契約を追加。macOSを含むCI成功 |
| [#161](https://github.com/Cerune-Lang/Cerune/issues/161) | JS/TSの型付き関数呼出し | 優先4。[WATライブラリのRust API](wat-library.ja.md)を実装。[検査付きJS/TS生成](wat-js-bindings.ja.md)・HTML例も実装済み。追加型・IR/VMのJS公開は実用途を見て判断 |
| [#130](https://github.com/Cerune-Lang/Cerune/issues/130) | 文書整合性 | PR #136で統合済み。旧URL・ライセンス・監査履歴・日英ペアを点検 |
| [#134](https://github.com/Cerune-Lang/Cerune/issues/134) | 文字列所有の正しさ | PR #137で統合済み。内部APIの所属・所有数を変更前に検証。性能改善とは分ける |
| [#89](https://github.com/Cerune-Lang/Cerune/issues/89) / [#104](https://github.com/Cerune-Lang/Cerune/issues/104) | 標準入出力と副作用比較 | 出力接続は統合済み。明示flushを実装。C・LLVM・QBE・WAT・Nativeの書込み失敗も実装。仮想std:io・行入力・所有を4実行器へ接続。CLI/C/LLVM/QBE/Nativeの実stdin・対話テストを追加。WATの版付き入力hostも実装済み。Native入力も実装済み。次は実例で必要になった小さな標準ライブラリを選ぶ。生成経路の成功操作の共通記録は後続 |
| [#153](https://github.com/Cerune-Lang/Cerune/issues/153) / [#135](https://github.com/Cerune-Lang/Cerune/issues/135) | Parserとソース読込みの上限 | 初版を実装。共通frontendで超過を診断し、子プロセスとCLIの境界を検証。import隔離・生成C・CIの他候補は別段階 |
| [#131](https://github.com/Cerune-Lang/Cerune/issues/131) | 型付きプログラムの差分検証 | [Parallax](ecosystem.ja.md)で固定seed・式・分岐・反復・配列、IR/MIR/VMと独立期待値の比較を実装。SSA・生成経路・一般の出自比較は残件 |
| [#173](https://github.com/Cerune-Lang/Cerune/issues/173) / [#174](https://github.com/Cerune-Lang/Cerune/issues/174) / [#175](https://github.com/Cerune-Lang/Cerune/issues/175) | 利用例から不足機能を選ぶ | [担当処理の棚卸しと引数設計](dogfooding-inputs.ja.md)。引数→数値読取り→限定ファイルI/O→子プロセスの順。Pythonの独立期待値を維持 |
| [#154](https://github.com/Cerune-Lang/Cerune/issues/154) / [#133](https://github.com/Cerune-Lang/Cerune/issues/133) | 入力容量とメモリ測定 | 優先3。メモリ上限と確保容量・RSS・WATページを区別。大きな上限と短い入力を測定し、未確認の漏えい・退行を断定しない |
| [#132](https://github.com/Cerune-Lang/Cerune/issues/132) | ActionsのNode.js警告 | PR #157で完了。Actionの実行用Node 24とWATテスト用Node 20.20.2を区別 |
| [#81](https://github.com/Cerune-Lang/Cerune/issues/81) | build / release・経路の責務 | Native buildは独立着手可。Image検証・配布へ進む。show／via／optional backendは用途と詳細設計待ち |
| [#60](https://github.com/Cerune-Lang/Cerune/issues/60) / [#94](https://github.com/Cerune-Lang/Cerune/issues/94) | 観測・変換・証明 | SSA・LIR観測まで部分完了。未収録の成果物・一般の証明・最適化は残件。非最適化表現を維持 |
| [#88](https://github.com/Cerune-Lang/Cerune/issues/88) | 整数演算の追加方針 | checkedを維持し、実用途のwrappingを先に検討。飽和・overflow報告・拡幅・ビット破棄・除算は別々に意味を決める |
| [#90](https://github.com/Cerune-Lang/Cerune/issues/90) / [#105](https://github.com/Cerune-Lang/Cerune/issues/105) | 長いコードの構文 | optional letとinfer省略→else ifを整合。条件式の意味、formatter、for適用は別途設計 |
| [#92](https://github.com/Cerune-Lang/Cerune/issues/92) | QBE Windows | 現行回避処理を維持。正式な上流変更を基準に再検証。調査ブランチは未統合で保管 |
| [#103](https://github.com/Cerune-Lang/Cerune/issues/103) / [#118](https://github.com/Cerune-Lang/Cerune/issues/118) | コード構成・観測成果物 | 変更する責務から整理。ローカル記録と公開fixtureを区別し、共有する本文・パス・環境情報を確認 |
| [#93](https://github.com/Cerune-Lang/Cerune/issues/93) | FPGA | 同じIR意味の別の実現形態を調査。機能追加の必須前提にしない |

ライセンスの[#119](https://github.com/Cerune-Lang/Cerune/issues/119)は完了です。PR #129で **MIT OR Apache-2.0** を採用しました（[MIT](../../LICENSE-MIT)／[Apache-2.0](../../LICENSE-APACHE)）。LIRの#123・array_repeatの#122も完了済みで、open作業に戻しません。

実験題材は[アイデアカタログ](../idea/idea-catalog.ja.md)に分けています。題材の掲載は採用・実装順の決定ではありません。Issueは詳細と議論、ロードマップは到達点と依存順、referenceは実装済み仕様を管理します。

## GPUをどう扱うか

GPUは将来の対象に含める価値があります。同じ計算がCPUの逐次実行から多数の要素の並列実行へどう変わるかを観測することは、Ceruneの目的に合います。ただし、現時点でGPU生成・実行は未実装です。ここでは計算用途を検討し、描画APIは対象に含めません。

### 型と実行契約を先に決める

LLVMのGPU出力先を選ぶだけでは、現在のプログラムはそのまま動きません。NVPTXではホストから起動するkernelとdevice関数を区別し、GPU固有のアドレス空間を扱います。CPUの出力補助関数もそのまま前提にできません。[LLVM NVPTX guide](https://llvm.org/docs/NVPTXUsage.html)

候補の比較は次の通りです。採用するAPIや機器は未決定です。

| 候補 | 判断に必要な点 |
| --- | --- |
| WebGPU / WGSL | まず32ビット数値の限定実験を考えやすい。ただしWGSLの実行時scalar型には`u64`・`f64`がない。現在のCerune全型に対応する経路とは扱えない |
| Vulkan / SPIR-V | `shaderInt64`・`shaderFloat64`などの機能を機器に問い合わせて選ぶ。64ビット対応を全機器へ仮定しない |
| LLVM NVPTX / CUDA | NVIDIA向けの候補。kernelの呼出規約、メモリ、ホスト側の起動・回収処理を別に設計する |

型の制約は[WGSLのscalar型](https://www.w3.org/TR/WGSL/#scalar-types)、機器の機能確認は[Vulkanのfeatures](https://docs.vulkan.org/spec/latest/chapters/features.html)を根拠にしています。未対応の`u64`を`f32`などへ黙って変換してはいけません。対応範囲を明示して診断するか、同じ意味を保つ代替実装を別途検証します。

GPU対応では少なくとも次を観測可能にします。

- 入出力の要素型・長さ・配置、ホストとdeviceの所有境界、転送と結果回収。
- 要素番号と処理の対応、起動する仕事の範囲、同期の位置。実行中の外部書き換えを観測APIとして公開しない。
- 整数の範囲検査と添字検査。失敗時は理由・ソース位置・論理的な要素番号を回収する契約を定める。
- 浮動小数点の丸め、演算のまとめ方、比較基準。許容誤差で比較する場合は先に明示し、既存CPU経路の意味を黙って緩めない。
- コンパイル・転送・kernel実行・回収の時間を分けた記録。小さい例で速くなるとは仮定しない。

GPU内の実行順全体をCPUの`print`順と同一視しません。最初はkernel内の出力を禁止し、回収後にホストが要素順に表示する境界を明示します。同期の必要性は[Vulkan compute tutorial](https://docs.vulkan.org/tutorial/latest/11_Compute_Shader.html)も参照してください。

### 最初の実験と完了条件

1. 読み取り専用の入力配列と、各要素が別々の場所へ書く出力配列を使う要素ごとの加算・変換を選ぶ。最初は範囲と添字の安全性を確認できる限定形にする。
2. 明示した型・機器・ツールで生成し、CPUの既知の答えと結果を比較する。未対応型や範囲不明の計算は明示的に拒否する。
3. 一般の計算へ広げる前に、失敗情報の回収を実装する。複数要素の失敗は、例えば最小の論理要素番号を採用するなど、実行順に依存しない規則を設計する。
4. 要素数が仕事のまとまりで割り切れない場合、境界外、桁あふれ、対応機能不足、転送失敗も区別して検証する。
5. その後、行列計算や熱拡散へ広げる。[matrix_vector_product](../../examples/matrix_vector_product.ceru)と[heat_diffusion](../../examples/heat_diffusion.ceru)は現在のCPU比較例であり、GPU対応済みの例ではない。後者の`f64`を限定経路の`f32`へ変更するなら、別の型付き実験として扱う。

最初の実験では共有領域への競合書き込み、atomic、総和の並列集約、描画、自前GPU機械語エンコーダは扱いません。

## 機能を追加するときの完了条件

日本語の設計理由、対応する英語文書、型別の小さなexample、既知の期待値、正常・失敗の実行比較を一緒に更新します。正常終了・想定した診断/停止・想定外の失敗・未実行を区別し、単なる非ゼロ終了を成功判定に使いません。

共通言語機能を追加したときは既存経路で意味を揃え、評価順・短絡評価・コピーの独立性・文字列バイト列・出自を検証します。GPUのような限定実験は対応範囲を明示し、未対応の機能を含むまま「全経路対応」と記載しません。
