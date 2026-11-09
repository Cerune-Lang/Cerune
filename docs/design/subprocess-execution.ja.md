# 子プロセスの共通IRと4実行器

[English](subprocess-execution.en.md)

`std:process/run`の`run`をHIR・MIR・SSA・Bytecode VMへ接続しました。[既存の型](subprocess-values.ja.md)と[共通runtime](subprocesses.ja.md)を使います。

## 公開APIと対応範囲

`import "std:process" as types;`で型を、`import "std:process/run" as process;`で`process::run(request)`を利用します。返却型は同じ`types::Outcome`です。

| 対象 | 現在の動作 |
| --- | --- |
| HIR・MIR・SSA・VM | 共通の`process.run`操作を実行 |
| 通常のCLI run系列 | 未接続。妥当な要求へ`Unconfigured`を返し、呼出し元は継続可能 |
| Rustの各実行器 | `run_with_processes(program, &mut Processes<B>, transfer_bytes)`で接続・上限を明示 |
| C・LLVM・QBE・WAT・Native | 実行モジュールを含むIRをソース位置付きで拒否。型だけの既存ソースは引き続き生成可能 |
| CLI登録・生成経路の実行・Linux監督の配布同梱 | 後続作業 |

現在は未使用関数も保持・生成します。既存の型だけを使うソースを未対応扱いにせず、暗黙の未使用削除も追加しないため、型と実行のimportを分けました。実行モジュールをimportして呼ばない場合も、生成経路は未対応操作を検出します。

両モジュールは予約済みの仮想標準ソース・契約版1です。実行モジュールから型モジュールへのimportも通常の名前解決を使い、型を複製しません。別名・import順に依存せず同じ型定義へ解決します。標準モジュールの依存解決はファイル探索をせず、利用者の相対import規則も変えません。

## 実行と失敗

1. 通常の関数呼出しと所有処理で要求を一度評価します。標準ソースの`run`も論理的呼出し深さに数えます。
2. 要求を独立した領域へコピーし、固定Reportと失敗用空配列を準備します。
3. 共通runtimeが要求・接続・容量を検査し、出力配列の予約後にのみ時計取得と起動を行います。
4. 部分出力・非0終了・失敗・後始末をOutcomeに保持し、返却配列を通常の解放処理へ戻します。

起動失敗、転送失敗、期限・容量超過などは値です。後始末未完了で停止した接続への次の呼出しも`ConnectionDisabled`を返し、暗黙に再起動しません。失敗値自体を作れない資源不足は`allocation-failed`等のソース位置付き停止です。子の終了状態へ偽装しません。

MIR/SSAとVMは各経路に保持された型情報からReportの形状を検査します。VMにもenumの選択肢名を保持し、Rust enumの数値や固定の型番号に依存しません。型情報の準備も起動前です。既存Rustのメタデータ複製等でallocatorがabortする場合まで回復を保証しません。

## 観測と検証

`emit-sources`で2つの標準ソース、`emit-ir`・`emit-mir`・`emit-mir --ssa`・`emit-bytecode`で操作と生成過程を確認できます。ReportのNodeId/Spanは標準ラッパー内の内部操作を指します。利用者の呼出し位置へ書き換えません。SourceIdは仮想標準ソースとして識別できます。

観測は読取り専用です。同じ応答を与えたとき、観測容量0／ありで結果とOS操作列が変わらないことを検証します。

- [比較テスト](../../src/runtime/process/tests/execution.rs)：評価順・短絡、非0終了、生バイト、コピー、起動失敗後の明示再呼出し、部分出力、停止接続、起動前容量不足、未接続。
- [モジュールテスト](../../tests/process_types/mod.rs)：型同一性・仮想出自・IR表示、非公開操作の拒否、既存の生成対応と未対応診断。
- [実OS接続](../../experiments/subprocess/execution.rs)：Windows/Linuxで4実行器から明示登録したfixtureへ入力し、非0終了・生バイト・継続を比較。比較用fixtureを4回実行し、任意の利用者プログラムを自動で再実行しません。
- [実行example](../../examples/process_execution/README.md)：未接続を値として受け取り継続。

次はCLIの明示登録・上限・Linux監督パスの接続を整理し、その後に生成経路のABIと配布を揃えます。
