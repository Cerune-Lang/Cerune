# 動的配列の例

[English](README.en.md)

IR／MIR／SSA Executor・VM・生成C・LLVM・QBE・WAT・Windows/Linux ASM・自前Objectで実行できます。

| 例 | 確認すること |
| --- | --- |
| [repeat.ceru](repeat.ceru) | 実行時の行数で表を作り、関数・要素更新・保存したコピー・反復を確認 |
| [repeat_empty.ceru](repeat_empty.ceru) | 0個でも値→個数の順に評価し、一時値を解放。空配列の長さと反復を確認 |
| [lowering_order.ceru](lowering_order.ceru) | 短絡・continue／breakと引数コピー。予算48で成功、47で呼出前に停止（[IR段階設計](../../docs/design/ir-stages.ja.md)） |
| [coordinates.ceru](coordinates.ceru) | 固定長の座標ペアを動的配列に入れ、コピーの平行移動と要素更新を確認 |
| [labels.ceru](labels.ceru) | 文字列配列を関数で加工し、元の配列・保存したコピー・再代入後を比較 |
| [batches.ceru](batches.ceru) | 二つの範囲を固定長配列に入れて返し、保存したコピーと更新を比較 |
| [readings.ceru](readings.ceru) | 構造体配列の範囲を関数から返し、コピーを修正して集計 |
| [window.ceru](window.ceru) | 実行時の長さで範囲を選び、関数から返した配列だけを更新 |
| [copy.ceru](copy.ceru) | 独立コピー、範囲・空配列、要素更新、表示・比較・反復 |
| [nested.ceru](nested.ceru) | 入れ子、文字列のNUL・CR/LF、関数・enum・match、反復中の再代入 |

```sh
cargo run --quiet -- run examples/dynamic_arrays/copy.ceru
cargo run --quiet -- run-vm examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-ir examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-bytecode examples/dynamic_arrays/copy.ceru
cargo run --quiet -- emit-c examples/dynamic_arrays/window.ceru -o window.c
clang -std=c11 window.c -o window
cargo run --quiet -- run examples/dynamic_arrays/copy.ceru --array-heap-limit 24
```

最後のコマンドは、元の3要素が24バイトを使い、`saved`用のコピー領域が足りないため意図的に停止します。既定予算では成功します。コピーのループと確保・解放は生成IRで確認できます。

生成Cは外部のCコンパイラでビルドします。上の`window`をLinuxなら`./window`、Windowsなら`.\window.exe`で実行できます。コピーと解放のループはIRと生成Cの両方で確認できます。

LLVMの生成・実行例（Linux x86-64）：

```sh
cargo run --quiet -- emit-llvm examples/dynamic_arrays/readings.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o readings.ll
clang readings.ll -o readings
./readings
```

Windowsではターゲットを`x86_64-pc-windows-msvc`に変え、`clang readings.ll -o readings.exe`、`.\readings.exe`で実行します。元の範囲は`[{valid: false, value: 0}, {valid: true, value: 20}]`のまま、コピーは15と20になり、合計35を出力します。

QBE 1.3での生成・実行例（Linux x86-64）：

```sh
cargo run --quiet -- emit-qbe examples/dynamic_arrays/batches.ceru --target x86_64-unknown-linux-gnu -o batches.ssa
qbe -t amd64_sysv -o batches.s batches.ssa
clang batches.s -o batches
./batches
```

WindowsではCeruneのターゲットを`x86_64-pc-windows-msvc`、QBEを`-t amd64_win`に変え、`clang --target=x86_64-pc-windows-msvc batches.s -o batches.exe`でリンクします。`.\batches.exe`は更新後`[[99, 20], []]`、保存値`[[10, 20], [30]]`、外側の長さ2、空配列の長さ0を出力します。

WATでの生成・実行例（NodeとWABTを使う開発用ホスト）：

```sh
cargo run --quiet -- emit-wat examples/dynamic_arrays/labels.ceru -o labels.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm labels.wat -o labels.wasm
node tests/support/run_wasm.cjs labels.wasm
```

WABTは`npm install --prefix target/wasm-tools --no-audit --no-fund wabt@1.0.39`で用意できます。出力は順に`["月", "火"]`、`["予定:月", "予定:火"]`、`["休み", "予定:火"]`、`false`です。IR・VM・C・LLVM・QBEでも同じ出力をテストします。コピー・解放・検査は生成WATにも残り、配列を置くmemoryは外部へ公開しません。

Nativeの生成・実行例（Linux x86-64）：

```sh
cargo run --quiet -- emit-asm examples/dynamic_arrays/coordinates.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o coordinates.s
clang coordinates.s -o coordinates-asm
./coordinates-asm
cargo run --quiet -- emit-obj examples/dynamic_arrays/coordinates.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o coordinates.o
clang coordinates.o -o coordinates-native
./coordinates-native
```

Windowsではターゲットを`x86_64-pc-windows-msvc`に変え、`clang --target=x86_64-pc-windows-msvc coordinates.o -o coordinates.exe`、`.\coordinates.exe`で実行します。ASMも同じターゲットでリンクします。元の座標`[[1, 2], [3, 4]]`、平行移動した保存値`[[11, 1], [13, 3]]`、更新後`[[99, 1], [13, 3]]`、`false`を出力します。すべての経路で同じ結果を検証します。
