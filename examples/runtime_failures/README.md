# 意図した実行時停止の例

[English](README.en.md)

この6例は正常終了せず、指定した理由で停止することを確認します。ルートの正常終了サンプル一括実行には含めません。

| 例 | 停止前の出力 | 停止理由・観測する箇所 |
| --- | --- | --- |
| [hello_cerune.ceru](hello_cerune.ceru) | `Hello, Cerune!`、`0.300000012`、`0.30000000000000004` | `integer-overflow`、`limit + 1`。型ごとの浮動小数点計算後に停止 |
| [constant_overflow.ceru](constant_overflow.ceru) | `7` | `integer-overflow`、`limit + 1`。結果が未使用でも実行時に停止する |
| [overflow.ceru](overflow.ceru) | `カウンタを更新` | `integer-overflow`、`counter + 1` |
| [array_update.ceru](array_update.ceru) | `4` | `array-index-out-of-bounds`、代入先の`[2]`。右辺を実行しない |
| [function_division.ceru](function_division.ceru) | `false`、`除算を開始` | `division-by-zero`、関数内の`value / divisor`。短絡した最初の呼び出しは実行しない |
| [call_sequence.ceru](call_sequence.ceru) | `除算する値`、`2`、`5`、`false`、`除算する値`、`0` | `division-by-zero`、同じ関数の`10 / value`。正常な呼び出しの後に短絡した呼び出しを飛ばし、最後の呼び出しで停止 |

```sh
cargo run -- run examples/runtime_failures/array_update.ceru --diagnostic-format runtime-v1
```

`run`／`run-ir`／`run-vm`は終了コード1で停止し、stderrに`cerune: runtime-v1 code=... node=... bytes=.....`を1行出します。`bytes`は元ソースのUTF-8バイト範囲で、終端は含みません。NodeIdと合わせて`emit-ir`へ辿れます。CRLF/LFを変更するとバイト位置も変わります。

`constant_overflow`は、[定数畳み込み設計](../../docs/design/ssa-constant-folding.ja.md)の基準例です。`cargo test --test ssa_executor --test ssa_native`でIR・MIR・SSA・SSAから戻したMIR・VMと、利用可能なNative ASM／Objectを比較します。最適化passはまだ実装していません。

既存4例と共通の停止ケースをC・LLVM・QBE・WATでも生成・実行し、このレコードと停止前の出力を比較するテストは`cargo test --test runtime_routes`です。外部ツールの指定は[診断の検証手順](../../docs/design/runtime-diagnostics.ja.md#検証)を参照してください。未設定で利用できない経路は実行済みと数えません。WATホストには`cerune.write_error_byte`が必要です。

Windowsで自前エンコーダと比較する例です。出力ディレクトリには未作成の場所を指定します。

```powershell
node scripts/observe-native.cjs --source examples/runtime_failures/array_update.ceru --target x86_64-pc-windows-msvc --cerune target/debug/cerune.exe --cc clang --objdump llvm-objdump --output-dir target/observe-array-failure --encoder cerune --run --expect-trap
```

Linuxではターゲットを`x86_64-unknown-linux-gnu`、Ceruneを`target/unix/debug/cerune`、外部ツールを`cc`と`objdump`へ明示的に変更します。`--encoder external`なら外部アセンブラで比較できます。成功判定には、停止理由・NodeId・バイト範囲・停止前の出力の一致と、ネイティブの意図したillegal-instruction停止をすべて要求します。詳細は[実行時診断の契約](../../docs/design/runtime-diagnostics.ja.md)を参照してください。
