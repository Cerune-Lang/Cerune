# Ceruneの文書

[English](README.en.md)

現在の機能・Issue・優先順位は[ロードマップ](design/language-roadmap.ja.md)、以前の判断と根拠は[2026-10-11の監査記録](design/development-status.ja.md)を参照してください。designは設計判断、referenceは実装済み仕様、ideaは未採用の題材を記録します。

## 設計

各文書から対応する英語版へ切り替えられます。

- [実行経路と未完了作業の棚卸し](design/development-status.ja.md)
- [命名と移行](design/naming.ja.md)
- [コンパイラ構成](design/architecture.ja.md)
- [IR段階と変換の観測](design/ir-stages.ja.md)
- [LIRの観測・構造検証と直接実行の採否](design/lir-observation.ja.md)
- [MIRのSSA変換（段階実装）](design/mir-ssa.ja.md)
- [観測結果の一括保存](design/observation-bundle.ja.md)
- [可観測性の契約](design/observability.ja.md)
- [実行時停止の共通記録](design/runtime-diagnostics.ja.md)
- [AST直接実行の調査](design/ast-executor-study.ja.md)
- [MIRの独立実行](design/mir-executor.ja.md)
- [IRの直接実行](design/ir-executor.ja.md)
- [VMの埋め込み実行](design/vm-embedding.ja.md)
- [標準入出力：実行・制御・観測（設計）](design/standard-io.ja.md)
- [CLI・生成C・LLVMの実stdin接続](design/stdin-connections.ja.md)
- [仮想標準モジュールと出自](design/standard-modules.ja.md)
- [上限付きUTF-8行入力](design/line-input.ja.md)
- [実行・配布の責務](design/owned-routes.ja.md)
- [意味の対応とプログラムの証明](design/lean-verification.ja.md)
- [出力経路とターゲット](design/targets.ja.md)
- [QBEのWindows対応](design/qbe-windows.ja.md)
- [所有する動的配列](design/owned-arrays.ja.md)
- [動的文字列とデータ](design/dynamic-data.ja.md)
- [言語機能と今後の順序](design/language-roadmap.ja.md)
- [丸め方と飽和](design/rounding-conversions.ja.md)
- [小数部分の切り捨て](design/truncating-conversions.ja.md)
- [型・長さを引数にする関数](design/generic-functions.ja.md)
- [関数](design/functions.ja.md)
- [名前付きproduct type](design/product-types.ja.md)
- [複合値とmatch式](design/aggregate-values.ja.md)
- [直和型と分岐](design/sum-types.ja.md)
- [コンパイル時定数](design/constants.ja.md)
- [構造体の更新式](design/product-updates.ja.md)
- [配列の反復](design/array-iteration.ja.md)
- [配列型の定数長](design/constant-array-lengths.ja.md)
- [固定長配列](design/fixed-arrays.ja.md)
- [文字列](design/strings.ja.md)
- [u64](design/u64.ja.md)
- [MIRからNativeへの変換](design/native-mir.ja.md)
- [ネイティブコードの観測](design/native-code.ja.md)
- [自前オブジェクトエンコーダ](design/native-encoder.ja.md)
- [定数畳み込みの設計（実装保留）](design/ssa-constant-folding.ja.md)
- [複数ソースと出自](design/source-files.ja.md)

## リファレンス

各文書から対応する英語版へ切り替えられます。

- [言語仕様](reference/language.ja.md)
- [CLI](reference/cli.ja.md)

## アイデア

各文書から対応する英語版へ切り替えられます。

- [アイデアカタログ](idea/idea-catalog.ja.md)

[検証実験](../experiments/README.md)は通常のCerune実行exampleと分けて管理します。

## 文書の確認

リポジトリ直下で次を実行します。CIでも同じ検査を行います。

```sh
node scripts/check-docs.cjs
node --test scripts/check-docs.test.cjs
```

Git管理対象と未追跡・非除外ファイルのMarkdownについて、リンク先のファイル／ディレクトリ、旧Cerune URL、日英ペアを確認します。公開ガイドは `.ja.md`／`.en.md`、READMEは `README.md`／`README.en.md` に揃えます。GitHub用の英語方針文書 `SECURITY.md` はペア検査の対象外です。

検査は本文のインラインリンク・画像・参照リンク定義を対象とし、コード例は除外します。外部URLの到達性、見出しアンカー、日英の内容一致、実装状況の正しさは人が確認します。構文全体を解析するMarkdown検証器ではありません。
