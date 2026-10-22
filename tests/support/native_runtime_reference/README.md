# 型付きruntime移行の比較基準

[English](README.en.md)

Cerune 8686e7aの `src/codegen/x86_64/utf8.rs` と `string.rs` にあったASM定数を抽出した固定テスト資料です。UTF-8検証とWindows/Linuxの文字列比較を含み、改行はLFで保存します。

本番コードからは参照しません。新しい実装・修正をここへ複製せず、型付きProgramとのObject一致を検証する間だけ保持します。135例の比較でも変更箇所をこのASMへ置換した旧基準を使います。

ライセンスはリポジトリのMIT OR Apache-2.0を維持します。

`heap.rs` / `array.rs` はbf81a78時点の確保・参照数・解放と管理データのASM emitterを固定しています。targetと上限値を引数で変え、単体および135例の旧Object比較に使います。これらも本番から参照せず、新実装へ追従して更新しません。

`output.rs` は0691c66時点の通常出力emitterです。整形・再試行・失敗診断の型付き移行を、元の定数・命令・再配置と比較するために固定しています。
