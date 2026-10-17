# 浮動小数点演算の丸めと経路比較

[English](float-rounding.en.md)

## 契約

`f32` / `f64`の通常の算術は、式の木と型に従い、各演算の結果をそれぞれbinary32 / binary64へ丸めます。既定の環境は最近接・偶数丸めです。`a * b + c`は乗算後に一度丸め、加算後にもう一度丸めます。結果が変わるFMAへの収縮や式の再結合は認めません。現在、明示FMAの言語操作はありません。

| 比較対象 | 契約 |
| --- | --- |
| 有限値・無限大 | 同じ入力・型・演算順で同じ値。誤差許容で経路差を隠さない |
| 正負のゼロ | 符号を保持し、ビット列または符号を保持した出力で区別する |
| NaN | NaNとしての比較・分類を保つ。payloadや符号の全経路ビット一致は保証しない |
| 外部の浮動小数点環境 | 埋込み側が丸め方向やsubnormal処理を変更した環境は検証対象外。観測のために環境や値を変更しない |

`convert`の厳密変換と、通常の算術で生じる丸めは別です。変換前の数学的な実数を復元する意味ではありません。

## 生成Cの条件

生成物にClangの`#pragma clang fp contract(off)`とGCCの`#pragma GCC optimize ("fp-contract=off")`を出力します。Clangを先に判定します。収縮禁止をテスト起動時の引数だけに隠さず、生成過程を読み取れるようにします。

外部compilerではNaN・無限大・符号付きゼロ・各演算の丸めを保持する設定を使用します。`-ffast-math`、`-Ofast`、Clangのpragmaを上書きする`-ffp-contract=fast`は本契約の検証対象外です。他のC compilerや拡張精度を使うtargetも、同じ条件の検証なしに対応済みとはしません。

根拠は[ClangのFP契約設定](https://clang.llvm.org/docs/UsersManual.html#controlling-floating-point-behavior)と[GCCの浮動小数点実装](https://gcc.gnu.org/onlinedocs/gcc/Floating-point-implementation.html)です。Clangの収縮設定はpragmaとコマンドラインの関係を持つため、`-O0`だけでは収縮禁止を保証しません。

## #156の切り分け（2026-10-18）

[Issue #156](https://github.com/Cerune-Lang/Cerune/issues/156)はApple Siliconの生成CとVMで線形回帰の出力差を観測した記録です。今回の調査はWindows x64でClang 22.1.8、Linux x64でGCC 9.4.0を使用しました。ARM64はClangのcross生成ASMを確認し、実機での実行はCIに分けます。

最小の演算は次の2関数です。

```c
float step32(float a, float b, float c) { return a * b + c; }
double step64(double a, double b, double c) { return a * b + c; }
```

| 型 | a / b / c（正確に表せる値） | 別々に丸めた結果のbits | FMAの結果のbits |
| --- | --- | --- | --- |
| f32 | 1+2^-23 / 1-2^-23 / -1 | 00000000 | a8800000（-2^-46） |
| f64 | 1+2^-52 / 1-2^-52 / -1 | 0000000000000000 | b970000000000000（-2^-104） |

乗算の正確な値は1より僅かに小さく、各型の精度では1に丸められます。その後の加算は+0です。FMAは途中の丸めを省くため、負の微小値が残ります。

| 条件 | 観測 |
| --- | --- |
| Clang x64、`-mfma`、O0/O2、対策なし | 上記FMAのbits。生成Cの収縮禁止ありでは両方+0 |
| Clang `--target=arm64-apple-darwin`、O0/O2 | 対策なしは`fmadd`。収縮禁止で`fmul`＋`fadd` |
| GCC x64、`-mfma -O2`、標準指定なし | 対策なしはFMAのbits。GCCのpragma追加で両方+0 |
| GCC `-std=c11`、O0/O2 | 対策前から両方+0。この条件だけではGCCの差分を検出できない |
| 線形回帰の生成C、Clang x64 `-mfma -O0` | 収縮の有無だけで最初の損失が0.011821707721273521 / 0.011821707721273523に変化し、元の報告と一致 |

最小例と元のexampleの両方で収縮による差分を確認しました。すべての将来の浮動小数点差分を同じ原因と推定するものではありません。

## 検証と観測

- [実行example](../../examples/float_rounding.ceru)：f32/f64の乗算・加算、桁境界、負のゼロ、無限大、NaN。出力は`0, 0, 0, 0, -0, -0, true, true`を各行に表示。
- `tests/c_strings.rs`：生成関数へvolatile入力を渡し、memcpyで戻り値のbitsを取り出す。定数畳込みや表示整形で差分を隠さない。C11／compiler既定、O0／O2を検証。x86でFMAが使用可能なら明示的に有効化。
- `tests/float_output.rs`：HIR・MIR・SSA・SSAから戻したMIR・VMを既知の結果と比較。有限値・負のゼロの出力をbitsへ戻して確認。
- `tests/dynamic_arrays.rs`：exampleの配列保存も含め、C・LLVM・QBE・WAT・ASM・自前Objectへ比較を広げる。Windows/Linuxで実行確認済み。
- 観測fixtureのC出力を更新。非最適化のIRや既存の演算出自を削除しない。

```sh
cargo run -- run examples/float_rounding.ceru
cargo run -- emit-c examples/float_rounding.ceru -o target/float-rounding.c
cargo test --test c_strings floating_point_contraction -- --nocapture
cargo test --test float_output
cargo test --test dynamic_arrays floating_point -- --nocapture
```

外部ツールがない場合のskipは成功した実行比較とは数えません。検証を必須にする場合は既存の`CERUNE_TEST_*`環境変数でツールを指定します。DarwinのLLVM/QBE/Native対応をこの調査によって追加したことにはしません。
