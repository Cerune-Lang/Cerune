# コントリビューション

日本語 | [English](CONTRIBUTING.en.md)

Ceruneへの関心と協力をありがとうございます。バグ報告、テスト、ドキュメントの修正、小さな機能改善などを歓迎します。Ceruneは開発中の実験的な言語で、仕様や実装は変更されることがあります。

参加にあたっては[行動規範](CODE_OF_CONDUCT.md)もご確認ください。

## はじめに

- [README](README.md)で使い方を、[サンプル](examples/README.md)で言語機能を確認できます。
- [言語仕様](docs/reference/language.ja.md)と[設計文書](docs/README.md)には、現在の仕様や設計判断がまとまっています。
- 作業前に[既存のIssue](https://github.com/Cerune-Lang/Cerune/issues)を確認してください。大きな機能追加や設計変更は、実装前にIssueで相談してもらえると助かります。

## バグ報告・提案

Issueには、可能な範囲で次の情報を含めてください。

- 何を試したか（実行したコマンドと、最小限の`.ceru`コード）
- 期待した結果と、実際の出力・エラーメッセージ
- Ceruneのバージョンまたはコミット、OS、アーキテクチャ
- 関係する実行経路やターゲット（分かる場合）

Ceruneには`run` / `run-ir`、`run-mir`、`run-vm`などの実行経路があります。結果が異なる場合は、どの経路で何が起きたかを書いてもらえると調査しやすくなります。すべての経路を試す必要はありません。

セキュリティ上の問題は公開Issueに書かず、[SECURITY.md](SECURITY.md)に従って非公開で報告してください。

## 開発・確認

開発にはRust/Cargoを使います。Rustのバージョンは[rust-toolchain.toml](rust-toolchain.toml)で固定しています。

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

変更前後の基本的な確認：

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

変更に関連するテストも実行してください。一部の統合テストには外部ツールが必要なため、手元で実行できない場合はPRにその旨を書いてください。各環境・実行経路の広い検証はCIでも行います。

LLVMを使うテストには、opaque pointer（`ptr`）対応のClangが必要です。Clang 18.1.8で検証しています。Linux/WSLでは使用する実行ファイルを明示し、次の順で確認できます。

```sh
export CERUNE_TEST_LLVM_CLANG=/usr/bin/clang-18
bash scripts/check-llvm-toolchain.sh
cargo test --lib
```

[事前検査](scripts/check-llvm-toolchain.sh)はバージョンを表示し、小さなLLVM IRを実際にコンパイルします。`bash scripts/test.sh`も、Clangがあるか明示指定された場合にこの検査を行います。コンパイラの不適合をテスト成功や無条件スキップにはしません。環境変数は現在のshellに適用され、新しいshellでは再指定してください。

ドキュメントを変更する場合は、必要に応じて日本語・英語の両方を更新し、Node.jsがあれば`node scripts/check-docs.cjs`でリンクと翻訳ペアを確認できます。

## Pull Request

- 1つのPRでは目的をなるべく絞ってください。
- 変更内容・理由、確認したコマンドや結果、関連Issueがあれば記載してください。
- 実行結果やコード生成に影響する変更では、関係する実行経路のテスト・サンプルを追加または更新してください。
- 実行できなかった検証や、分かっている制限はそのまま記載してください。

小さな修正や質問からでも歓迎です。
