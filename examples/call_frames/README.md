# 呼出しごとの値と戻り先

[English](README.en.md)

[main.ceru](main.ceru)は、文字列と配列を持つ構造体を入れ子の関数へ渡します。値の更新後も元の値は残り、早期returnから受け取った値も使えます。再帰はまだ使いません。

```sh
cargo run --quiet -- run examples/call_frames/main.ceru
cargo run --quiet -- run-vm examples/call_frames/main.ceru
cargo run --quiet -- emit-wat examples/call_frames/main.ceru
```

出力は[main.stdout](main.stdout)のとおりです。

```text
[2, 4]
Cerune
[2, 4]
Cerune!
[12, 4]
```

生成WATの `private-frame-bytes`、`cerune.frame.base`、`cerune_frame_allocate` / `cerune_frame_release`から、関数ごとの領域サイズと呼出し時の確保・return時の返却を追えます。これらは内部の配置であり、Ceruneからmemoryを変更するAPIではありません。[再帰に向けた設計](../../docs/design/recursion.ja.md)を参照してください。

`cargo test --test wat_output frames::call_frames_example`でHIR・MIR・SSA・VM・WATの出力を照合します。WATの実行にはNodeとwabtが必要です。
