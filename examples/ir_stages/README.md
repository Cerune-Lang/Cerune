# IR段階の観測

[English](README.en.md)

| Example | 確認すること | 実行結果 |
| --- | --- | --- |
| [native_calls.ceru](native_calls.ceru) | MIRからNativeへ変換。型の異なる7引数、continue／break、main呼出し | 見出しとi=0・2の引数、計算結果`4` |
| [owned_values.ceru](owned_values.ceru) | 関数へ渡す動的配列の独立性、不変文字列、continueを通る解放 | 元配列・装飾後の配列・`["反復"]` |
| [ssa_values.ceru](ssa_values.ceru) | 分岐で値を選ぶ・continue後の更新・反復ごとの値の入替え。SSA変換・実行比較用 | `14`、`16`、`20`、`10`の4行 |
| [constant_candidates.ceru](constant_candidates.ceru) | 最適化前の基準。定数計算・短絡・呼出し順・loopの更新 | `42`、u64最大値、`true`、`false`、`true`、`4`、`10`、`7` |
| [control_flow.ceru](control_flow.ceru) | 短絡で呼出しを省く。forのcontinueは更新、breakは出口へ進む | `2`、`4`の2行 |

次のコマンドで、同じソースの実行結果と変換前後を確認できます。

```sh
cargo run --quiet -- run examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-vm examples/ir_stages/ssa_values.ceru
cargo run --quiet -- run-mir examples/ir_stages/ssa_values.ceru --ssa
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru -o target/ssa_values.mir.txt
cargo run --quiet -- emit-mir examples/ir_stages/ssa_values.ceru --ssa -o target/ssa_values.ssa.txt
cargo run --quiet -- observe examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu -o target/ssa-observation
cargo run --quiet -- run examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/owned_values.ceru
cargo run --quiet -- run-vm examples/ir_stages/control_flow.ceru
cargo run --quiet -- run-mir examples/ir_stages/native_calls.ceru
cargo run --quiet -- emit-mir examples/ir_stages/native_calls.ceru -o target/native_calls.mir.txt
cargo run --quiet -- emit-asm examples/ir_stages/native_calls.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/native_calls.s
cargo run --quiet -- emit-obj examples/ir_stages/native_calls.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/native_calls.o
cargo run --quiet -- emit-ir examples/ir_stages/control_flow.ceru
cargo run --quiet -- emit-mir examples/ir_stages/control_flow.ceru -o target/control_flow.mir.txt
```

`native_calls`は見出し`MIR → Native`の後に、`0, 18446744073709551615, 7, 9, 4`、続いて`2, 18446744073709551615, 7, 9, 4`を1値1行で表示します。ASMの`cerune-mir`注釈からMIRのblock・命令番号へ戻れます。Windows向けに生成する場合はtargetを`x86_64-pc-windows-msvc`へ変更します。Objectの生成だけではリンク・実行しません。

`ssa_values`の`choose_and_count`は、10に分岐で2または4を加え、loopで0と2を加えます。`swap_rounds`は10と20を3回入れ替えます。非SSA経路・SSA直接実行・SSA経由Nativeの比較例です。`emit-mir --ssa`で元MIRと自動変換後のSSAを並べて表示できます。[SSA設計](../../docs/design/mir-ssa.ja.md)で合流値と並列受渡しの説明に使います。

`control_flow`の`i=0`では`accept`を呼ばず、`i=1`はcontinueで更新へ進み、`i=2`だけが`accept`を呼んで`2`を表示します。`i=3`は更新前に終了し、合計`4`を表示します。

MIRでは`bbN`がブロック、`%N`が型付き局所値／一時値、`iN`が関数内の命令番号です。`derived=short-circuit-rhs`、`derived=for-update`、`derived=loop-exit`とjump／branchを読むと制御の行き先が分かります。各操作の`hir=#N source=N bytes=A..B`で元のIRとファイル内の範囲へ戻れます。MIRを生成するだけではプログラムは実行されません。

配列コピーと確保失敗も確認する場合は[lowering_order.ceru](../dynamic_arrays/lowering_order.ceru)を使います。`run-mir`は[独立したMIR実行器](../../docs/design/mir-executor.ja.md)です。[NativeもMIR入力へ移行済み](../../docs/design/native-mir.ja.md)です。`--ssa`でSSA変換・実行・観測を選べます。SSA付きbundleは元MIR・SSA・変換後MIR・基準とSSA経由の両ASMを別々に保持します。最適化は行いません。型・検証範囲は[段階設計](../../docs/design/ir-stages.ja.md)に記載しています。

## 定数計算を観測する

`constant_candidates`は `6 * 7`、u64最大値になる加算、比較と短絡を含みます。`reported(4)`が4を表示してから外側の加算結果10を表示し、loopでは1から2ずつ3回加えて7になります。短絡した関数内にはoverflowする計算が残っていますが、実行されません。

```sh
cargo run --quiet -- run examples/ir_stages/constant_candidates.ceru
cargo run --quiet -- run-mir examples/ir_stages/constant_candidates.ceru --ssa
cargo run --quiet -- emit-mir examples/ir_stages/constant_candidates.ceru --ssa -o target/constant_candidates.ssa.txt
```

現在は最適化しません。[定数畳み込みの設計](../../docs/design/ssa-constant-folding.ja.md)で、元の演算・出自を残すことと、どの演算をliteralへ置換できるかを分けて説明します。[未使用結果でもoverflowする例](../runtime_failures/constant_overflow.ceru)は7を出力した後に停止する異常例です。

## SSA経由でNativeを生成する

次はLinux向けObjectを生成し、Clangでリンクして実行する例です。Windowsではtargetを`x86_64-pc-windows-msvc`に変え、実行ファイルを`target/ssa_values.exe`にし、リンク時の`-lm`は外します。出力は同じ4行です。

```sh
cargo run --quiet -- emit-asm examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu --annotate-origins -o target/ssa_values.ssa.s
cargo run --quiet -- emit-obj examples/ir_stages/ssa_values.ceru --ssa --target x86_64-unknown-linux-gnu --annotate-origins -o target/ssa_values.ssa.o
clang target/ssa_values.ssa.o -o target/ssa_values -lm
./target/ssa_values
```

`observe --ssa`の`program.ssa-lowered.mir.txt`では、辺の補助block内に入力のcopyが並び、その後に行き先へのcopyが並びます。`program.ssa-lowered-map.txt`からSSA値・辺とMIR命令の対応を辿れます。`program.origins.s`は基準MIR、`program.ssa.origins.s`は変換後MIRからの生成です。両者を上書きせず比較できます。

## 同じコンパイルを保存する

出力先の親ディレクトリを用意し、まだ存在しないbundle名を指定します。次の例は実行せずに5ファイルを保存します。

```sh
cargo run --quiet -- observe examples/ir_stages/owned_values.ceru --target x86_64-unknown-linux-gnu --string-heap-limit 512 --array-heap-limit 1024 -o target/owned_values.observation
```

`sources.json`、`program.ceir`、`program.mir.txt`、`program.origins.s`、`manifest.json`が生成されます。Windows向けはtargetを`x86_64-pc-windows-msvc`に変更します。再実行には別の新規保存先を指定してください。

`program.mir.txt`の`hir=#N`とASMの`cerune-mir`注釈を辿ると、同じ式がどの命令へ変換されたか確認できます。同じheap予算・targetで個別の`emit-ir`・`emit-mir`・`emit-asm --annotate-origins`を使った出力と一致します。実行結果は別途`run`・`run-mir`・`run-vm`で比較します。[保存形式と範囲](../../docs/design/observation-bundle.ja.md)を参照してください。
