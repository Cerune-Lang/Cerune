# ホスト呼出しに使う関数

[English](README.en.md)

通常のCerune例です。JS/TS・Rustの比較コードは[別の実験フォルダ](../../experiments/host_calls/README.md)に置きます。

| ファイル | 内容 |
| --- | --- |
| [math.ceru](math.ceru) | 位置の更新、厳密な数値変換、i32/u64/f32の関数 |
| [demo.ceru](demo.ceru) | mathをimportして、Ceruneから同じ関数を呼ぶ実行例 |

リポジトリ直下で実行します。

```sh
cargo run --quiet -- run examples/host_calls/demo.ceru
cargo run --quiet -- run-vm examples/host_calls/demo.ceru
cargo run --quiet -- emit-ir examples/host_calls/demo.ceru
```

両実行経路の出力は次のとおりです。

```text
11
0
42
42
18446744073709551615
1.25
```

`checked(1.5)`は小数部分を黙って捨てず停止します。実験ではその停止記録もVMとWATで比較します。`pub`はCerune内の可視性であり、現在の通常のemit-watがこれらをJS向けに自動公開するわけではありません。
