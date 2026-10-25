# 検証実験

[English](README.en.md)

言語本体・通常のCerune実行サンプルとは分けて、モデル・変換・外部ツールによる検証を置きます。各フォルダに対象範囲、必要なツール、実行手順を記録します。

| 実験 | 内容 |
| --- | --- |
| [lean](lean/README.md) | 有限例のHIR／MIR／SSAとLeanモデルの対応、プログラムの性質、実行比較 |
| [host_calls](host_calls/README.md) | 同じ関数をVM埋め込みと生成WATから呼び、値・停止・型変換・サイズを比較 |
| [call_depth](call_depth/README.md) | 4実行器と生成Cの深さ上限・物理スタックを分離。OS・値の形・ビルド条件による違いを測定 |
| [ssa](ssa/README.md) | SSA構築・観測・直接実行のRust API例。公開CLIの代わりに別の言語仕様を作るものではない |

LIR直接実行の実験はありません。[全体の現状](../docs/design/development-status.ja.md)で実装・観測・証明範囲を区別します。

GitHubのLanguagesはソースの言語集計で、必要な実行環境の一覧ではありません。検証用のLean、CJS補助ツール、生成物の比較用fixtureは[.gitattributes](../.gitattributes)で集計対象から外します。証明や観測結果をレビューできるよう、diffを隠す設定は使いません。

将来のLean emitter本体はRust実装として集計し、比較用のLeanファイルは検証側へ置きます。
