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

[depth.ceru](depth.ceru)は左右の引数を順に評価してからpairへ入ります。通常実行は `cargo run --quiet -- run examples/call_frames/depth.ceru`、出力は[depth.stdout](depth.stdout)です。

深さ上限はCLIの `--call-depth-limit N` で指定できます（既定128・範囲1〜128、物理stackの安全保証ではありません）。Rust APIでも明示できます。4実行器の`run_with_call_limit`に加え、C・LLVM・QBE・WAT・Nativeの生成APIも対応しています。[生成物を保存する手順](../../experiments/call_depth/README.md)で検査・履歴・出自を確認できます。上限2なら完走し、上限1ならouterから最初のmark(10)へ入る直前に停止します。`cargo run --quiet -- run examples/call_frames/depth.ceru --call-depth-limit 1 --diagnostic-format runtime-v1` で失敗位置とcall stackを確認できます。`cargo test --test ir_executor call_depth` で4実行器の先行出力・停止位置・呼出し履歴を比較します。

[pending_values.ceru](pending_values.ceru)は、先に評価したprice(1)を保持したままsubtotalへ入り、戻ってから加算します。`cargo run --quiet -- run examples/call_frames/pending_values.ceru` の出力は[pending_values.stdout](pending_values.stdout)です。4実行器はそれぞれの表現を保持して明示frameで呼び出します。run-mir、run-mir --ssa、run-vmでも同じ出力になります。emit-ir／emit-mir／emit-bytecodeの表現と出自は変えません。

[return_paths.ceru](return_paths.ceru)は、早期returnと通常returnの両方で浮動小数点の戻り値を使います。[期待出力](return_paths.stdout)は上限2で一致し、上限1では最初のleafに入る前に停止します。
