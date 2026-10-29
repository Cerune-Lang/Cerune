# Ceruneサンプル

[English](README.en.md)

このディレクトリ直下の`.ceru`が、単独で実行できる正常例です。

| 場所 | 内容 |
| --- | --- |
| このディレクトリ | 一括実行できるプログラム |
| [recursion](recursion/README.md) | 直接・相互再帰、genericの再利用、所有値と深さ上限 |
| [call_frames](call_frames/README.md) | 入れ子呼出し、コピーの独立性、早期return、WATの呼出し領域を観測 |
| [host_calls](host_calls/README.md) | 外部ホスト呼出しの比較にも使う小さな数値関数と、通常のCerune実行例 |
| [dynamic_arrays](dynamic_arrays/README.md) | 動的配列の作成・範囲コピー・入れ子。IR・MIR・VM・C・LLVM・QBE・WAT・ASM・自前Objectで実行 |
| [arguments](arguments/README.md) | プログラム引数・空文字・UTF-8バイト数・seed指定。全既存経路対応 |
| [build_native](build_native/README.md) | Native実行ファイルのbuild・引数付き実行・リンク記録の観測 |
| [formatting](formatting/README.md) | optional let・infer省略・整形・letの表示切替と実行結果の比較 |
| [integer_parsing](integer_parsing/README.md) | 明示的なi64読取り・書式不正・範囲外 |
| [files](files/README.md) | 新規保存・読戻し。Windows/Linuxの明示ディレクトリで4実行器を比較 |
| [stdin](stdin/README.md) | 実stdinの行・空行・EOF、promptのflush、UTF-8バイト数。全経路で比較 |
| [io_protocol](io_protocol/README.md) | 出力と評価順、明示flush、入力値での操作分岐。EOF・空行・失敗を区別（標準入力は未使用） |
| [ir_stages](ir_stages/README.md) | 短絡・loop・関数呼出しを実行し、HIR→MIR→LIR→Nativeの変換とtarget別ABIを観測 |
| [modules](modules/README.md) | import、単一ファイル版との比較、意図した停止 |
| [runtime_failures](runtime_failures/README.md) | 停止前の出力と失敗する式を追う5例 |
| [source_files](source_files/README.md) | 型・関数のファイルを別々に解析し、出自を比較するRust API例。importは使わない |

## IRから直接実行

`cargo run --quiet -- run-ir examples/ir_execution.ceru`で、関数・配列のコピー・反復・文字列連結を試せます。`run`は同じ実行経路で、VMとの比較には`run-vm`を使います。`emit-ir`で共通の処理を確認できます。

| 例 | 出力 |
| --- | --- |
| [ir_execution.ceru](ir_execution.ceru) | `Hello, IR` → `Hello, 世界` → `["Cerune", "世界"]` |

他の正常例も`run-ir`で実行できます。一括スクリプトは`run`によるIR直接実行を使い、`cargo test --test examples --test ir_executor`で両経路と期待値を照合します。

## まとめて実行

リポジトリのルートで実行します。サブディレクトリは一括実行の対象外です。

| 環境 | 全件実行 | 対象を絞る／ビルド済み実行ファイルを使う |
| --- | --- | --- |
| PowerShell | `.\scripts\run-examples.ps1` | `-Pattern "matrix*.ceru"`／`-SkipBuild` |
| WSL / Bash | `bash scripts/run-examples.sh` | `--pattern 'matrix*.ceru'`／`--skip-build` |

名前・出力・終了状態・集計を表示します。WSLにはWSL側のRust環境が必要です。Bashのビルド先は既定で`target/unix`、指定時は`CARGO_TARGET_DIR`です。ビルドを省く場合は、同じ出力先で一度ビルドしてください。

個別実行や各段階の表現を見るときは、次のファイル名を置き換えます。

```sh
cargo run --quiet -- run examples/linear_regression.ceru
cargo run --quiet -- emit-ir examples/linear_regression.ceru
cargo run --quiet -- emit-bytecode examples/linear_regression.ceru
cargo run --quiet -- emit-c examples/linear_regression.ceru
```

## 固定seedで再現する

[deterministic_seed.ceru](deterministic_seed.ceru)はi64の整数演算で同じ数列を再現します。seed=1の最初の3値は48271、182605794、1291394886です。0・最大seed・0回も確認します。

`cargo run --quiet -- run examples/deterministic_seed.ceru`で実行できます。seedはソース内の固定値で、CLI引数の取得やOS乱数は使いません。[引数対応の設計](../docs/design/dogfooding-inputs.ja.md)の基準例です。

## 型から探す

数値範囲と用途を型ごとにまとめます。`infer`は値の型ではなく型推論の指定、`void`は値を返さない関数の戻り方です。[floating_point.ceru](floating_point.ceru)と[functions.ceru](functions.ceru)で確認できます。

### 符号付き整数：負数と正数

| 型 | 数値範囲 | サンプル | 確認すること |
| --- | --- | --- | --- |
| `i8` | −128〜127 | [sensor_calibration.ceru](sensor_calibration.ceru) | 小さな負の補正量を、計算前に`i16`へ広げる |
| `i16` | −32768〜32767 | [sensor_calibration.ceru](sensor_calibration.ceru) | 正負の測定値を保持し、集計時には`i32`へ広げる |
| `i32` | −2147483648〜2147483647 | [maximum_subarray.ceru](maximum_subarray.ceru) | 正負の増減を足し、大小を比較する |
| `i64` | −9223372036854775808〜9223372036854775807 | [integer_limits.ceru](integer_limits.ceru) | 最小値・最大値と、桁あふれ前の判定 |

### 符号なし整数：非負の数とビット列

| 型 | 数値範囲 | サンプル | 確認すること |
| --- | --- | --- | --- |
| `u8` | 0〜255 | [color_blending.ceru](color_blending.ceru)、[bit_flags.ceru](bit_flags.ceru) | 色の値と、8個のビットの設定・解除・反転 |
| `u16` | 0〜65535 | [color_blending.ceru](color_blending.ceru) | `u8`の色を加算前に広げ、平均を求めて戻す |
| `u32` | 0〜4294967295 | [population_statistics.ceru](population_statistics.ceru) | 30億前後の値を保持し、集計時には`i64`へ広げる |
| `u64` | 0〜18446744073709551615 | [u64_values.ceru](u64_values.ceru), [packet_counter.ceru](packet_counter.ceru) | 最大値、最上位ビット、符号なし比較・除算、関数・配列・コピー、正確な変換 |

整数の範囲外演算は折り返さず停止します。型を混ぜるときは`i64(value)`や`convert<i64>(value)`で明示変換します（[両表記の例](integer_conversions.ceru)）。型情報のない整数と配列の添字は`i64`です。ビット幅は数値範囲を表し、現在の生成先では小さい整数も64ビット領域に格納します。[u64の表現](../docs/design/u64.ja.md)も参照できます。

### 浮動小数点：小数と精度

| 型 | 表現 | サンプル | 確認すること |
| --- | --- | --- | --- |
| `f32` | 32ビット浮動小数点 | [floating_point.ceru](floating_point.ceru)、[logistic_map.ceru](logistic_map.ceru) | `f64`との丸め・計算結果の違い |
| `f64` | 64ビット浮動小数点 | [floating_point.ceru](floating_point.ceru)、[small_values.ceru](small_values.ceru) | 小さい値の表示と計算時の丸め。型情報のない浮動小数点は`f64` |

[measurement_statistics.ceru](measurement_statistics.ceru)と[normalized_histogram.ceru](normalized_histogram.ceru)では整数と浮動小数点を行き来します。`convert<T>`は値を保てる場合だけ成功します。`trunc<T>`は小数部分をゼロ方向へ切り捨てます。通常の浮動小数点演算で生じる丸めとは別の規則です。

| 変換の例 | 確認すること |
| --- | --- |
| [truncating_conversions.ceru](truncating_conversions.ceru) | `3.7 → 3`、`-3.7 → -3`、`u8`の境界、`u64`、定数と型引数 |
| [truncating_evaluation_order.ceru](truncating_evaluation_order.ceru) | 引数を左から一度だけ評価、短絡で変換を省略、配列コピーの独立性 |
| [rounding_conversions.ceru](rounding_conversions.ceru) | 全5丸め方で、正負の中間値を比較 |
| [rounding_quantities.ceru](rounding_quantities.ceru) | floorで区画、ceilで必要な箱数。定数の元の式と結果も観測 |
| [saturating_conversions.ceru](saturating_conversions.ceru) | 色をu8へ収める、NaN・無限大・型の端 |
| [rounding_evaluation_order.ceru](rounding_evaluation_order.ceru) | 引数の順序・短絡・配列コピーを保つ |

[生成手順の確認](../docs/design/rounding-conversions.ja.md#試す)では、IR・bytecode・注釈付きLLVM/ASMを同じソースから出力します。

### 真偽値と文字列

| 型 | 値 | サンプル | 確認すること |
| --- | --- | --- | --- |
| `bool` | `true` / `false` | [boolean_comparisons.ceru](boolean_comparisons.ceru)、[short_circuit.ceru](short_circuit.ceru) | 比較・否定と、評価を省く短絡評価 |
| `string` | 内容が不変のUTF-8文字列 | [string_values.ceru](string_values.ceru)、[string_byte_length.ceru](string_byte_length.ceru)、[string_concat.ceru](string_concat.ceru) | 日本語・比較・バイト数・連結・再代入とコピーの独立性 |

[string_origins.ceru](string_origins.ceru)は文字列の処理をCerune IRと出自注釈付きLLVMで辿る例です。文字列の内容は不変ですが、mutな束縛への再代入はできます。

### 配列と構造体：型を組み合わせる

| 型の形 | サンプル | 確認すること |
| --- | --- | --- |
| 動的配列 `[T]` | [copy.ceru](dynamic_arrays/copy.ceru)、[nested.ceru](dynamic_arrays/nested.ceru) | 空範囲、独立コピー、入れ子・関数・enum・反復。[repeat](dynamic_arrays/repeat.ceru)は実行時の個数から作成 |
| 固定長配列 `[T; N]` | [fixed_arrays.ceru](fixed_arrays.ceru)、[bubble_sort.ceru](bubble_sort.ceru) | 添字、要素の更新、コピー後の独立性 |
| 構造体 `type Point { ... }` | [product-point.ceru](product-point.ceru)、[product_arrays.ceru](product_arrays.ceru) | フィールド・既定値と、構造体を要素にする配列 |
| 入れ子の配列・構造体 | [function_values.ceru](function_values.ceru)、[u64_values.ceru](u64_values.ceru)、[string_lookup.ceru](string_lookup.ceru) | 数値や文字列を組み合わせ、関数へ値として渡す |
| 固定容量と使用数 | [bounded_sequence.ceru](bounded_sequence.ceru) | 空・満杯・不正な使用数を扱う。動的配列ではなく容量4の値 |
| 関数へ渡す所有と寿命 | [owned_arguments.ceru](owned_arguments.ceru) | 固定長配列の独立性、引数の評価順、未使用引数と返却値の寿命。文字列予算22バイトで反復 |
| 読み取りと値コピー | [borrowed_reads.ceru](borrowed_reads.ceru) | 配列の表示・比較、一時値の抽出、コピー後の更新。文字列予算30バイトで実行 |
| 入れ子のコピーと寿命 | [array_copy_lifetimes.ceru](array_copy_lifetimes.ceru) | 固定長配列に動的文字列を格納し、返却後の生存・範囲コピー・更新の独立性を確認 |
| 固定容量への範囲コピー | [array_window.ceru](array_window.ceru) | 空・不正範囲、評価順・短絡、元の更新後も独立。借用スライスではない |
| 複合値の等値比較 | [aggregate_comparison.ceru](aggregate_comparison.ceru) | 配列・構造体・直和型、左右の評価順、定数とジェネリック関数 |
| 複合値の表示 | [aggregate_display.ceru](aggregate_display.ceru) | 入れ子の値、最大u64、文字列の引用、NUL・CR/LFを保持 |

### 直和型：選択肢に応じた値

| サンプル | 型・確認すること |
| --- | --- |
| [sum_lookup.ceru](sum_lookup.ceru) | `enum Lookup`：文字列とu64を持つ成功、値を持たない未登録 |
| [sum_divide.ceru](sum_divide.ceru) | `enum Division`：成功値・ゼロ除算・範囲外を値で返して処理を続ける |
| [sum_values.ceru](sum_values.ceru) | 配列・構造体を持つ選択肢、コピー、構築順と分岐内の再代入 |
| [modules/sum_lookup.ceru](modules/sum_lookup.ceru) | 公開enumのimport・構築・網羅的な分岐 |
| [match_values.ceru](match_values.ceru) | match式の戻り値、ガード、型引数、定数で配列長を決める |
| [match_guards.ceru](match_guards.ceru) | 対象を一度評価、ガードの順と省略、コピーの独立性、continue・break |
| [modules/aggregate_match.ceru](modules/aggregate_match.ceru) | 公開enum・定数の比較と表示、別ファイルのmatch式 |

[比較・表示・分岐の規則と生成手順](../docs/design/aggregate-values.ja.md)。

### コンパイル時定数

| サンプル | 確認すること |
| --- | --- |
| [constants.ceru](constants.ceru) | u64の上限、文字列、f32・構造体・配列の定数、コピーと短絡評価 |
| [modules/constants.ceru](modules/constants.ceru) | 非公開の補正定数から計算した公開設定をimportして利用 |

### 構造体の更新式

| サンプル | 確認すること |
| --- | --- |
| [product_update.ceru](product_update.ceru) | u64・文字列・配列・入れ子を更新し、元の値が変わらないこと |
| [product_update_order.ceru](product_update_order.ceru) | 元の値を一度だけ評価、フィールドの記述順、既定値を再評価しないこと、短絡・ループ |
| [modules/product_update.ceru](modules/product_update.ceru) | importした型・関数を使った更新 |

[更新式の規則](../docs/design/product-updates.ja.md)。

### 多い引数を組み合わせる

[function_arguments.ceru](function_arguments.ceru)は7引数の入れ子呼び出しと、11引数の値の受け渡しを比較します。

| 型 | 確認すること |
| --- | --- |
| `i64` | `observed(1)`〜`observed(7)`を順に一度だけ実行し、入れ子の呼び出しの後も合計は`28` |
| `u64` | 最大値と最上位ビットを、5個目以降の引数でも保持 |
| `f32`・`f64`・`bool` | 整数と混ぜても`1.5`・`2.5`・`true`を保持し、比較結果は`true` |
| `string` | 日本語・NUL・CR/LFをそのまま渡す |
| 配列・構造体 | 関数内の配列コピーを更新しても元は不変。構造体の戻り値も独立 |

引数の個数と型は宣言に一致させます。個数の固定上限はありません。[関数の配置と検証](../docs/design/functions.ja.md)も参照できます。

## 基本と制御

| サンプル | 内容 |
| --- | --- |
| [hello.ceru](hello.ceru) | 整数に名前を付け、足し算の結果を`print`で表示する最初の例 |
| [greeting.ceru](greeting.ceru) | `string`の名前を関数へ渡し、`concat`で挨拶を作る |
| [shopping_total.ceru](shopping_total.ceru) | `u64`の価格配列を`for … in`で足し、合計とお釣りを表示 |
| [countdown.ceru](countdown.ceru) | `mut i64`と`while`で3・2・1を表示して終了 |
| [short_circuit.ceru](short_circuit.ceru) | `&&`・`\|\|`で条件を組み合わせ、不要な割り算・配列参照・関数呼び出しを省略する |
| [conditional.ceru](conditional.ceru) | `if` / `else`とscope |
| [else_if.ceru](else_if.ceru) | `else if`の連鎖、上から順の条件判定、最後の`else`省略 |
| [loop_control.ceru](loop_control.ceru) | `while`、`break`、`continue` |
| [for_sum.ceru](for_sum.ceru) | `for`と開始文の再代入 |
| [generic_functions.ceru](generic_functions.ceru) | 型・長さを指定して集計・コピーを共用。文字列・u64・構造体・直和型 |
| [generic_evaluation_order.ceru](generic_evaluation_order.ceru) | ジェネリック関数の転送、引数の評価順、短絡 |
| [functions.ceru](functions.ceru) | 型付き関数、parameter、戻り値、`void`関数 |

## データ構造

| サンプル | 内容 |
| --- | --- |
| [constant_array_lengths.ceru](constant_array_lengths.ceru) | 定数で固定サイズを共有。前方参照、型の同一性、関数、文字列・u64・enumとコピー |
| [modules/constant_array_lengths.ceru](modules/constant_array_lengths.ceru) | importした寸法と、公開型の内部で使う非公開定数 |
| [array_iteration.ceru](array_iteration.ceru) | `for … in`による集計・検索、添字、途中終了、走査中のコピー |
| [array_iteration_values.ceru](array_iteration_values.ceru) | 文字列・u64・入れ子配列・構造体・直和型の反復と`mut`な要素コピー |
| [array_length.ceru](array_length.ceru) | 要素数を使った集計。定数、入れ子、文字列・u64・構造体・直和型の配列とコピー |
| [array_length_order.ceru](array_length_order.ceru) | 長さが既知でも関数・要素を一度ずつ評価。短絡評価とループ条件 |
| [ring_buffer.ceru](ring_buffer.ceru) | `%`で保存位置を循環させ、直近4件の値と平均を保つ |
| [string_lookup.ceru](string_lookup.ceru) | 文字列をキーに構造体の配列を線形探索し、対応する表示や既定値を返す |
| [product-point.ceru](product-point.ceru) | 点の座標を構造体にまとめる。フィールドの既定値と読み取り |
| [fixed_arrays.ceru](fixed_arrays.ceru) | 固定長配列の要素を読み、合計と線形探索を行う。コピーした配列が独立した値であることも確認する |
| [product_arrays.ceru](product_arrays.ceru) | 構造体を配列に並べ、最も近い点を探す。配列のコピーも確認する |
| [function_values.ceru](function_values.ceru) | 構造体と入れ子の固定長配列を、関数へ値として渡して受け取る |
| [packet_counter.ceru](packet_counter.ceru) | u64の最上位ビット、u8のフラグ、不変の文字列を構造体で渡し、自前エンコーダで実行する |
| [native_values.ceru](native_values.ceru) | u64・整数・小数・文字列・構造体を混在する4引数で渡し、Windows/Linuxの実行と機械語までを辿る |

## 数値計算

| サンプル | 内容 |
| --- | --- |
| [float_rounding.ceru](float_rounding.ceru) | f32/f64の演算ごとの丸め、FMAとの差、負のゼロと非有限値 |
| [measurement_statistics.ceru](measurement_statistics.ceru) | 整数の測定値から小数の平均・分散を求め、値を変えずに`f32`へ保存する |
| [normalized_histogram.ceru](normalized_histogram.ceru) | 整数の回数を小数の割合へ変換し、保存した割合から元の回数を復元する |
| [square_root.ceru](square_root.ceru) | 手順を展開した平方根の近似 |
| [while_square_root.ceru](while_square_root.ceru) | `while`で繰り返す平方根の近似 |
| [logistic_map.ceru](logistic_map.ceru) | `f32`と`f64`で生まれる計算結果の違い |
| [matrix_vector_product.ceru](matrix_vector_product.ceru) | 入れ子の固定長配列を使った3×3行列と3要素ベクトルの積 |
| [matrix_composition.ceru](matrix_composition.ceru) | 構造体と入れ子配列を関数で受け渡し、2×2行列の合成とベクトル変換を行う |
| [population_statistics.ceru](population_statistics.ceru) | `u32`の大きな値を`i64`へ広げて集計し、平均と最大値を構造体で返す |
| [heat_diffusion.ceru](heat_diffusion.ceru) | 棒の熱が広がる4段階の計算。更新前の配列から次の温度を求める |
| [linear_regression.ceru](linear_regression.ceru) | 5点から直線を学習し、傾き・切片・誤差の変化を追う |

## アルゴリズム

| サンプル | 内容 |
| --- | --- |
| [color_blending.ceru](color_blending.ceru) | `u8`の色を足す前に`u16`へ広げ、平均を求めて混ぜる |
| [sensor_calibration.ceru](sensor_calibration.ceru) | `i16`の測定値を`i8`の補正量で調整し、`i32`へ広げて集計する |
| [maximum_subarray.ceru](maximum_subarray.ceru) | `i32`の増減から連続区間の最大合計を求める。`u32`の位置を添字へ変換する |
| [subset_sum_bits.ceru](subset_sum_bits.ceru) | シフトとビットORで、選んだ重さから作れる合計を一度に求める |
| [euclidean_gcd.ceru](euclidean_gcd.ceru) | ユークリッドの互除法による最大公約数 |
| [fibonacci.ceru](fibonacci.ceru) | Fibonacci数列と複数の値の更新順 |
| [factorial.ceru](factorial.ceru) | `for`による階乗 |
| [collatz.ceru](collatz.ceru) | Collatz予想と条件ごとの状態遷移 |
| [prime_check.ceru](prime_check.ceru) | 試し割りによる素数判定と早期終了 |
| [integer_square_root.ceru](integer_square_root.ceru) | 二分探索による整数平方根 |
| [exponentiation_by_squaring.ceru](exponentiation_by_squaring.ceru) | 繰り返し二乗法による累乗 |
| [pythagorean_triples.ceru](pythagorean_triples.ceru) | 入れ子の`for`によるピタゴラス数の探索 |
| [bubble_sort.ceru](bubble_sort.ceru) | `mut`な固定長配列の要素をその場で入れ替えるバブルソート |
| [xor_neural_network.ceru](xor_neural_network.ceru) | 固定長配列の重みを使う小さなニューラルネットのXOR推論 |
| [coin_change.ceru](coin_change.ceru) | 少ない金額の答えを使い回して最少枚数を求め、使った硬貨も復元する動的計画法 |
| [shortest_paths.ceru](shortest_paths.ceru) | 途中で寄れる町を増やして、全組み合わせの最短距離を求める |

## 出力と計算途中の読み方

`\0`・`\r`・`\n`はNUL・CR・LF、矢印とカンマは出力行の区切りです。各ソースの日本語コメントでも順序を説明しています。

| サンプル | 出力と確認内容 |
| --- | --- |
| `u64_values.ceru` | 冒頭は`u64 → 18446744073709551615 → 9223372036854775808 → 0`。再代入後もコピーを保持し、最上位ビットも正の数として扱う |
| `sum_lookup.ceru` | `空\0\r\n\n6\n未登録\n`。IRでは選択肢のタグ・対象コピー・分岐を確認 |
| `constants.ceru` | 冒頭は`128 → 18446744073709551615 → 9`（文字列のバイト数）。配列のコピーを`99`に変えても、二つの構造体定数は`10`。IRでは定義式と評価済みの値を確認 |
| `product_update.ceru` | 元／更新後の番号は`9223372036854775808 / 9223372036854775809`、配列要素は`10 / 99` |
| `product_update_order.ceru` | 冒頭は`base → default → replacement → 2`。既定値は元の構築時だけ実行し、更新・コピーでは再実行しない |
| `function_arguments.ceru` | 冒頭は`引数の評価順`、`1`〜`7`、`28` |
| `string_origins.ceru` | `日本語\0\ntrue\nfalse\n`。`skipped`は出ない。IRと注釈付きLLVMで比較`#7`・短絡評価`#14`から呼び出し・分岐へ辿る（[手順](../docs/reference/cli.ja.md#llvmの出自を辿る)） |
| `string_byte_length.ceru` | `0, 9, 3, 2, 3, 4, 7, 3, 9, left, right, 9, false, false, 6, 10`に各LF。`left`・`right`は一回ずつ、`skipped`は出ない。`byte_len`でUTF-8長・コピー・関数・配列・既定値を確認 |
| `generic_functions.ceru` | 合計`6 → 9223372036854775809 → 4`、元の文字列と更新後のコピー、構造体・直和型の値 |
| `generic_evaluation_order.ceru` | `評価順 → 配列 → 添字 → 2 → 末尾 → false → true`。「呼ばれない」は出ない |
| `constant_array_lengths.ceru` | 寸法`2 → 3`、行の合計`6 → 15`。コピー変更後も元は`6`、コピーは`99`。続いてu64の境界値、文字列のバイト数と内容 |
| `array_iteration.ceru` | 正数の合計`15`、検索結果の添字`2`と`未登録`。元を変更しても走査は`1 → 2 → 3`、元の二番目は`99` |
| `array_iteration_values.ceru` | 文字列の添字・バイト数・内容、u64の境界値。コピー変更後も元の行は`1 → 3`、元の構造体は`10`。直和型は`空 → 海` |
| `array_length.ceru` | 集計は`4 → 20 → 4 → 118 → 20`。コピーを変更しても元は不変。型別の長さは`2 → 3 → 2 → 9 → 2 → 2 → 2`（`9`だけ文字列のバイト数） |
| `array_length_order.ceru` | `make → 10 → 20 → 2`。二度目の呼び出し後に`later → 5`。短絡した`skipped`は出ず、ループ条件の`2`は終了判定も含め3回 |
| `coin_change.ceru` | 1〜6円の最少枚数、その後に使う硬貨の`3 → 3` |
| `shortest_paths.ceru` | 町0→3の距離の変化、その後に4×4の距離表を行順で表示。`-1`は到達不能 |
| `heat_diffusion.ceru` | 5区間の温度を4段階分、その後に保存した初期の中央温度 |
| `linear_regression.ceru` | 学習前の誤差、10回ごとの回数・傾き・切片・誤差、最後に入力3の予測値 |
| `integer_limits.ceru` | 通常は成功。末尾の式のコメントを外すと桁あふれと診断位置を確認できる |

`linear_regression.ceru`の`rate`（一回の更新幅）や繰り返し回数を変えると、学習の進み方を比較できます。勾配降下法で傾き・切片を学ぶ例です。`xor_neural_network.ceru`は固定した重みによる推論だけで、XORの学習は含みません。

## 生成と検証

VM・C・LLVM・QBE・WAT・Windows/Linux ASM・自前オブジェクトで、動的配列を含む上記の型と言語機能を扱います。可変配列によるその場でのソートや動的計画法も可能です。[再帰](recursion/README.md)では親の値の独立性と深さ上限も確認できます。

| 経路 | 実行条件・確認内容 |
| --- | --- |
| C | 外部Cコンパイラでコンパイルして実行（下記） |
| LLVM | Windows/Linuxのターゲットを明示（[コマンド例](../docs/reference/cli.ja.md#llvmのターゲット指定)） |
| QBE | Linux x86-64のターゲットを明示 |
| WAT | 出力用ホスト関数を備えたWebAssembly環境 |
| ASM／自前オブジェクト | Windows x64／Linux x86-64のターゲット・ツールを明示。`native_values.ceru`で混在引数と値渡しを確認（[外部ツールの手順](../docs/design/native-code.ja.md)、[自前エンコーダ](../docs/design/native-encoder.ja.md)） |

```sh
cargo run --quiet -- emit-c examples/string_lookup.ceru -o target/string_lookup.c
clang -std=c11 target/string_lookup.c -o target/string_lookup
```

Bashでは`./target/string_lookup`、Windowsでは`.\target\string_lookup.exe`で実行します。注釈付きLLVMの生成例：

```sh
cargo run -- emit-llvm examples/string_byte_length.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/string-byte-length.ll
```

| 検証 | コマンド・内容 |
| --- | --- |
| 一括実行 | 冒頭のスクリプト。終了状態を確認 |
| 配列反復 | `cargo test --test array_iteration`。exampleの期待出力、コピー・制御・診断位置 |
| 期待出力 | `cargo test --test examples` |
| Cの文字列 | `cargo test --test c_strings`。最適化あり・なしでVMと比較 |
| LLVMの文字列 | `cargo test --test llvm_strings`。VM・C・LLVMのバイト列を比較 |
| QBE・WAT・ASMの文字列 | `cargo test --test string_routes`（[ツールと検証範囲](../docs/design/strings.ja.md#検証範囲)） |
| 全体 | `bash scripts/test.sh`。fmt・clippy・全テスト |

`u64_values.ceru`は全経路で既知の出力と比較します。文字列のバイト数も期待バイト列と照合し、小さい入力と各経路の表現は[観測fixture](../tests/fixtures/observation/string-byte-length/)で確認できます。

既定コンパイラがなければ実行比較を省略する場合があります。`CERUNE_TEST_CC`・`CERUNE_TEST_LLVM_CLANG`で指定すると必須になります。CIではClangを必須とし、AddressSanitizer・UndefinedBehaviorSanitizerでも検査します。
