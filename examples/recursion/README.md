# 再帰

[English](README.en.md)

| 例 | 確認すること | 完走に必要な深さ |
| --- | --- | --- |
| [factorial.ceru](factorial.ceru) | 階乗。再帰から戻って乗算 | 5 |
| [mutual.ceru](mutual.ceru) | evenとoddの相互呼出し | 7 |
| [owned_generic.ceru](owned_generic.ceru) | 文字列・動的配列を持つ値の独立性、genericの同じ具体化への再帰 | 5 |

```sh
cargo run --quiet -- run examples/recursion/factorial.ceru
cargo run --quiet -- run-mir examples/recursion/mutual.ceru --ssa
cargo run --quiet -- run-vm examples/recursion/owned_generic.ceru
cargo run --quiet -- run examples/recursion/factorial.ceru --call-depth-limit 4 --diagnostic-format runtime-v1
cargo run --quiet -- emit-ir examples/recursion/owned_generic.ceru
cargo run --quiet -- emit-c examples/recursion/factorial.ceru --call-depth-limit 5 -o target/factorial.c
```

最後のrunは終了1となり、深さ超過のcall-v1・NodeId/Span・呼出し履歴を出します。上限5なら階乗は成功します。期待出力は各`.stdout`に保存しています。既定128・指定範囲1〜128は意味上の呼出し深さであり、物理stack／memoryの安全保証ではありません。

emit-irではgeneric引数・定義と再帰呼出しの位置、通常のcall／return、所有処理を確認できます。emit-mir・emit-bytecode・各生成経路でも同じ呼出しが残ります。再帰をループへ暗黙変換しません。[契約と検証](../../docs/design/recursion.ja.md)を参照してください。
