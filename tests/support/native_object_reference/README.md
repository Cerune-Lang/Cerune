# 移行比較用の旧エンコーダ

[English](README.en.md)

Cerune f48a72cのエンコーダと既存低レベルテストを移動した固定基準です。src/codegen/x86_64.rsのcfg(test)からだけコンパイルし、本番Object生成はSASHIMIを使います。

符号化・解析・形式の新機能や修正をここへ複製しません。切替前後のObject全体・診断の一致を確認する間だけ保持します。SASHIMI側で型付きAPIと独立した回帰検証が整った段階で、この比較基準の廃止を判断します。

出典：[旧実装](https://github.com/Cerune-Lang/Cerune/tree/f48a72c01f33ff32f483a8c125d0b69b4f7d2444/src/codegen/x86_64)。ライセンスはリポジトリのMIT OR Apache-2.0を維持します。
