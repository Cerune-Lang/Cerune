# MIRの独立実行

[English](mir-executor.en.md) · [IR段階](ir-stages.ja.md)

## 役割と現在の対応

`run-mir`は、完成済みCerune IRを非SSAのMIRへ変換し、MIRの局所値・命令・ブロックを直接実行します。現在の型、関数、module、generic、match、固定／動的配列、文字列、所有操作、停止診断に対応します。frontendによる名前・型・所有の確定は共通です。

| コマンド | 実行する表現 | 位置づけ |
| --- | --- | --- |
| `run`／`run-ir` | 完成済みCerune IR（HIR） | 既定の直接実行 |
| `run-mir` | 非SSAのMIR | HIR→MIRの意味保存を比較する独立経路 |
| `run-mir --ssa` | scalar SSAと残存slot | 明示的なSSA変換後の直接実行 |
| `run-vm` | bytecode | VMの命令実行 |
| `emit-mir` | 実行しない | MIRの操作・辺・出自を観測 |

MIRからHIRやbytecodeを再構築して実行することはありません。MIR実行の追加は、新しいbuild／release成果物の追加でもありません。[Nativeも同じMIRを入力に使います](native-mir.ja.md)。[SSA変換と直接実行](mir-ssa.ja.md)はRust APIと明示的な`--ssa`で利用できます。最適化と外部MIRのloaderは未実装です。

逐次出力には`run_with_output`を使います。既存の`run`は出力収集を維持します。CLIは逐次出力APIを使い、出力済みの内容を再表示しません。接続・部分書込み・I/O失敗・読み取り専用記録は[標準入出力](standard-io.ja.md)を参照してください。

## 実装と共有範囲

制御フローと原子的な値操作を分けています。

| 実装 | 責務 |
| --- | --- |
| [mir_executor.rs](../../src/mir_executor.rs) | 局所値のフレーム、命令実行、branch／jump／return、関数呼出し、MIR上の停止位置 |
| [mir_executor/semantics.rs](../../src/mir_executor/semantics.rs) | MIR・SSAで共有する個々の命令・演算・所有操作。CFGと値参照は各実行器が担当 |
| [runtime/value.rs](../../src/runtime/value.rs) | HIR・MIR・SSAで共有する値表現、数値・比較・ビット演算、表示。制御フローは含めない |
| [runtime/numeric.rs](../../src/runtime/numeric.rs) | 数値変換・丸めの意味 |
| [runtime/string_heap.rs](../../src/runtime/string_heap.rs)、[array_heap.rs](../../src/runtime/array_heap.rs) | 値の所有数、文字列・配列のメモリ上限、領域の回収 |
| [mir/validate.rs](../../src/mir/validate.rs) | 実行前の構造・型・初期化・添字検査の先行を検証 |

読取り・一時保存のRust上のcloneは、動的領域の論理的なretainや独立コピーではありません。MIRに記録された確保・コピーのループ・retain／release／freeだけが言語上の所有を変えます。ABIや実行中のOSから処理を切り替えません。

入口本体の後に呼ぶ`main`は、loweringが`synthetic=entry-main-call`のcall命令として追加します。MIR実行器は関数名から暗黙の呼出しを補いません。呼出し先で止まった場合は、その関数内の命令とソース位置を保持します。

関数呼出しでは、元MIRへの参照、block、次の命令、局所値、戻り先を明示frameに保存します。ホストの再帰呼出しへは積みません。[深さ・確保失敗の契約](recursion.ja.md)は4実行器で共有します。

## APIと停止

ソースには`run_mir(source)`、importを持つ入力には`modules::load(path)?.to_ir()`→`mir::lower(&hir)`→`mir_executor::run(&mir)`を使います。各実行は独立した状態を持ち、入力のHIR／MIRは変更しません。

`MirRunError`は構築失敗と実行失敗を区別します。実行エラーから取得できる情報は次のとおりです。

| API | 内容 |
| --- | --- |
| `kind()` | 検証失敗、言語の停止コード、実行中の内部不整合を区別 |
| `origin()` | 元のHIR NodeIdとSourceIdを含むSpan |
| `location()` | MIRのFunctionId（入口はNone）・BlockId・InstructionId。呼出し元で上書きしない |
| `output()` | 収集型APIの先行出力。逐次出力APIでは空 |
| `runtime_failure()` | 言語の停止だけを既存経路と同じ共通記録に変換 |

検証失敗は実行前に返すため出力は空です。CLIの`--diagnostic-format runtime-v1`は他経路と同じ停止コード・元の位置・先行出力を返します。既定診断は元ファイルの位置を示し、MIR命令番号はRust APIから`emit-mir`と照合できます。

## 所有の検証と限界

正常終了時には文字列・配列heapの生存領域が空であることを確認します。残っていれば成功にせず内部不整合を返します。途中の失敗では、未初期化領域を含む実行状態を破棄するときに領域を回収します。

これは全経路の寿命を静的に証明するものではありません。alias・部分初期化・retain／releaseの静的な経路検証は引き続き未実装です。MIRはコンパイラ内で構築した値を対象とし、不正な外部入力のsandboxや実行時間制限を提供しません。言語で未対応の再帰はfrontendで拒否します。MIR実行器は同じ関数への再入も独立した明示frameで扱い、テスト用IRではrun_with_call_limitで上限を検証します。上限未指定の不正IRの無限実行は防ぎません。

## 比較とexample

[control_flow.ceru](../../examples/ir_stages/control_flow.ceru)は短絡とloopの辺、[owned_values.ceru](../../examples/ir_stages/owned_values.ceru)は関数・動的配列・文字列・continueを通る解放を確認します。

```sh
cargo run --quiet -- run-mir examples/ir_stages/owned_values.ceru
cargo run --quiet -- emit-mir examples/ir_stages/owned_values.ceru -o target/owned_values.mir.txt
cargo run --quiet -- run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 47 --diagnostic-format runtime-v1
```

最後のコマンドは意図した予算不足です。`begin`だけを出力し、引数コピーの元の位置で`allocation-limit-exceeded`となります。成功したテストでは、異常停止そのものも期待値と照合しています。

[IR比較テスト](../../tests/ir_executor.rs)と[動的配列比較](../../tests/dynamic_arrays.rs)へMIRを加え、既知出力・停止コード・位置・先行出力を確認します。[MIR専用テスト](../../tests/mir_executor.rs)では、MIRだけを変更するとその結果が実行に反映されること、呼出し先の出自、未解放領域の検出も確認します。Windows/Linuxの動的配列比較は、HIR・MIR・VM・C・LLVM・QBE・WAT・ASM・自前Objectの9経路を対象にします。

共有する値演算の誤りはHIR/MIRの比較だけでは検出できないため、VM・生成経路と既知の期待値も使います。実行比較はテストであり、形式証明や性能同等の保証ではありません。
