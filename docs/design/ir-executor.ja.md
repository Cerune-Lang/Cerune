# Cerune IR Executor

[English](ir-executor.en.md)

## 位置づけと入力

IR Executorは、既存のCerune IRの意味を直接実行する公式の基準経路です。IRの定義・構築・型検査・名前解決・共通展開を共用します。Bytecode、VM、backend固有IRへの変換やフォールバックは行いません。Whitebase固有の仕様は設けません。

```text
既存のフロントエンドと共通展開
                  ↓
            ir::Program
             ├─ IR Executor
             ├─ Bytecode → VM
             ├─ C / LLVM / QBE / WAT
             └─ Assembly / Native Object
```

入力は`compile_to_ir`または`modules::Compilation::to_ir`が返す完成済みの`ir::Program`です。ジェネリック・enum/match・複合値の比較と表示・文字列所有の共通展開は既存処理を利用します。定数評価を含むIR構築工程では現在のBytecode/VM利用を維持します。独立するのは、IRを受け取った後の実行です。`.ceir`テキストの読み込みは別機能であり追加しません。

逐次出力には`run_with_output`を使います。既存の`run`は出力収集を維持します。CLIは逐次出力APIを使い、出力済みの内容を再表示しません。接続・部分書込み・I/O失敗・読み取り専用記録は[標準入出力](standard-io.ja.md)を参照してください。

## APIと実行の責務

`ir_executor::run(&ir::Program) -> Result<String, ExecutionError>`を入口とします。再度コンパイルせず同じIRを繰り返し実行でき、実行状態は呼び出しごとに独立します。IRを書き換えたり、隠れた最適化・実行用IRの構築をしたりしません。

CLIは`run <file>`または同じ意味の`run-ir <file>`で選びます。VM実行は`run-vm <file>`で明示します。import・診断・文字列予算は既存の共通フロントエンドと設定を使い、`--diagnostic-format runtime-v1`に対応します。ソースからのAPIは`run_ir(source) -> Result<String, IrRunError>`です。`IrRunError`はコンパイル失敗と実行失敗を区別します。実行先を黙って変更しません。

- 文・式を直接辿り、関数ごとの束縛をBindingIdで管理する。
- if/while/forを構造のまま実行し、return/break/continueを区別する。forのcontinueでも更新部を実行する。
- トップレベル文と明示mainの起動規則は既存と一致させる。
- 引数・演算子・配列要素・構築フィールドを左から一度ずつ評価し、短絡する右辺は評価しない。
- 添字代入では各段の検査を次の添字や右辺より先に行う。

内部値は型付き数値、bool、不変文字列ハンドル、値としての配列と構造体です。VMのValueやbytecodeのTypeを利用しません。配列・構造体は入れ子も独立した値として扱い、不変の文字列内容は共有します。動的配列の要素コピーは、ホストの暗黙コピーではなく共通IRのループが担います。enumは共通IRのタグとフィールドとして実行し、独自の言語解釈を加えません。

## 共有する意味論と所有

数値変換、浮動小数点の出力、文字列領域の管理をruntime配下の小さい部品へ整理しました。VMは互換アダプターを通じて利用し、既存の公開エラーとAPIを維持します。命令ディスパッチ・スタック・スロット・関数実行は共有しません。

整数の範囲、u64、丸めと飽和、NaN・符号付きゼロ、UTF-8/NUL/CR/LF、引用表示を既存仕様に合わせます。IR内のstring.retain/releaseを実行し、Rust側の一時コピーとは論理的な参照数を分離します。string-heap-limitの生存バイト数、旧新の共存、失敗時の回収も一致させます。

共有した処理の誤りを相互比較だけで見逃さないよう、既知の期待値・境界値・生成C等の独立した実装も検証に用います。

## 失敗と可観測性

Executorのエラーは、言語の停止理由と不正IR等の内部問題を分けます。言語の失敗は共通のFailureCode・NodeId・Spanで表し、停止までの出力を保持します。子の式や呼び出した関数で発生した出自を外側の式で上書きしません。不正IRを正常な言語の停止へ偽装しません。

生成されたループ・条件・保持・解放も通常のIRとして実行します。観測の入口は既存のemit-irと出自付き診断です。外部から実行中の束縛・所有状態を書き換えるAPIは導入しません。構築済みIRを受け取るAPIにより、将来の性能比較ではコンパイル時間と実行時間を分けられます。速度の優位性を保証するものではありません。

## 実装と検証の順序

1. 共有部品の整理と、基本値・演算・構造化制御。
2. 関数・配列・構造体・共通展開後のenum/match。
3. 動的文字列と所有・予算・エラー出自、CLI/API。
4. 現行exampleと正常／失敗fixtureを既知の期待値・VM・各生成先と比較し、日英の機能表を更新。

途中の段階は完成扱いせず、対応状況をテストで確認してから現行機能との同等性を確定します。新しい言語機能や動的配列本体の追加は、この照合が済んでから戻ります。

## 現在の対応と制限

上記の段階を実装し、現行の完成済みIRを実行対象としています。

| 対応 | 内容 |
| --- | --- |
| 値と演算 | 全整数型・f32/f64・bool・文字列、範囲検査、明示変換・丸め・飽和 |
| 制御と関数 | if/while/for、短絡評価、break/continue/return、トップレベル／main |
| 複合値 | 固定長・動的配列・構造体・enum、入れ子・値コピー・添字更新・比較・表示 |
| 共通展開 | import、定数、型と長さのジェネリック、配列反復、match式・ガード |
| 文字列 | concat・byte_len、不変内容、明示retain/release、予算・失敗時回収 |
| 観測 | emit-ir、NodeId・ファイル内Span・先行出力、runtime-v1 |

`tests/ir_executor.rs`は完成済みIRをVM用にlowerした結果と照合し、元のIRが不変であることも検査します。`tests/examples.rs`・共有fixtureは既知の期待値を確認します。`runtime_routes`・`string_heap_routes`は生成先とも停止・出自・予算を照合します。`ir_execution.ceru`はC・LLVM・QBE・WAT・ASM・オブジェクトの比較対象にも含めます。

未対応は、テキストIRの読み込み、任意関数を外部から呼ぶ埋め込みAPI、実行の一時停止・再開です。実行途中の介入APIは設けません。再帰は未対応です。[動的配列](owned-arrays.ja.md)は後続の言語機能としてIR・VMで実装し、確保・コピー・解放を共通IRから実行します。公開Rust APIはフロントエンドの完成済みIRを前提とし、手作りIRの完全な検証器ではありません。検出した構造・型・所有の不整合は`InvalidIr`とし、未展開の`Let`/`Conditional`を黙って展開・VMへ委譲しません。

実装は`src/ir_executor.rs`（制御・フレーム・診断）、`src/runtime/value.rs`（MIRと共有する内部値・原子的演算）、`src/runtime/{numeric,float_output,string_heap,array_heap}.rs`（共有部品）に分けています。VM用の変換は`src/vm`に残し、直接実行からVMの値・命令列へ依存しません。
