# コントリビューション

日本語 | [English](translations/CONTRIBUTING.en.md)

Ceruneへの関心と協力をありがとうございます。バグ報告、テスト、ドキュメントの修正、小さな機能改善、検証実験などを歓迎します。Ceruneは個人で開発している実験的な言語で、仕様や実装は変更されることがあります。

参加にあたっては[行動規範](CODE_OF_CONDUCT.md)もご確認ください。すべての実行経路や内部構造を理解してから参加する必要はありません。

## はじめに

- [README](README.md)で使い方を、[サンプル](examples/README.md)で言語機能を確認できます。
- [言語仕様](docs/reference/language.ja.md)・[CLI](docs/reference/cli.ja.md)・[設計文書](docs/README.md)には、現在の仕様や設計判断がまとまっています。
- 作業前に[ロードマップ](docs/design/language-roadmap.ja.md)、[マイルストーン](https://github.com/Cerune-Lang/Cerune/milestones)、[既存のIssue](https://github.com/Cerune-Lang/Cerune/issues)を確認してください。大きな機能追加や設計変更は、実装前にIssueで相談してもらえると助かります。

## 開発方針

Ceruneでは、プログラムの意味と、それが実行される表現へ変わる過程を観測可能に保つことを重視します。変更する範囲に応じて、次の性質を維持してください。

- 言語の意味と型を共通frontendで確定し、バックエンド固有の判断は明示的な変換境界で行う。
- 対応する実行経路間で、評価順・演算の規則・値のコピーと更新・停止条件・外部操作の意味を揃える。対応範囲の違いは明記する。
- ソースとIR・生成コードの対応を維持する。元の出自を失った処理を `Synthetic` として扱わない。
- 観測と制御を分離し、観測の有無でプログラムの意味や実行状態を変えない。
- 最適化は明示的で観測可能な工程として扱う。
- 実装済みの機能、設計案、検証済みの範囲を区別する。

詳しくは[コンパイラ構成](docs/design/architecture.ja.md)・[可観測性の契約](docs/design/observability.ja.md)・[出力経路とターゲット](docs/design/targets.ja.md)を参照してください。

## バグ報告・提案

Issueには、可能な範囲で次の情報を含めてください。

- 何を試したか（実行したコマンドと、最小限の`.ceru`コード、必要な入力）
- 期待した結果と、実際の出力・終了状態・エラーメッセージ
- Ceruneのバージョンまたはコミット、OS、アーキテクチャ
- 関係する実行経路・ターゲット・オプション、使用した外部ツールとそのバージョン（分かる場合）
- 経路間で差がある場合は、その差と関連する診断・IR・トレースなど（取得できる場合）

CeruneにはHIR直接実行（`run` / `run-ir`）、MIR（`run-mir`）、SSA（`run-mir --ssa`）、VM（`run-vm`）の4実行器と、C・LLVM・QBE・WAT・Nativeの生成経路があります。すべての経路を試す必要はありません。[CLIリファレンス](docs/reference/cli.ja.md)には、停止診断・`--trace`・`observe`・各生成経路の使用方法があります。

観測成果物にはソース本文やパスなどが含まれる場合があります。共有前に内容を確認し、秘密情報や公開できないデータを除いてください。セキュリティ上の問題は公開Issueに書かず、[SECURITY.md](SECURITY.md)に従って非公開で報告してください。

## 開発環境

開発にはGitとRust/Cargoを使います。Rustのバージョンは[rust-toolchain.toml](rust-toolchain.toml)で固定しています。

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

基本的な実行には外部のCコンパイラなどは不要です。生成経路や検証目的に応じてClang/GCC、QBE、WABT、Node.js、対象OSのリンカやSDKなどが必要になります。詳細は[READMEの外部ツール一覧](README.md#外部ツール)を参照してください。

LLVMを使うテストにはopaque pointer（`ptr`）対応のClangが必要です。Linux/WSLでは、例えばClang 18を使用する場合、次のように事前検査できます。パスは環境に合わせて変更してください。

```sh
export CERUNE_TEST_LLVM_CLANG=/usr/bin/clang-18
bash scripts/check-llvm-toolchain.sh
cargo test --lib
```

[事前検査](scripts/check-llvm-toolchain.sh)はバージョンを表示し、小さなLLVM IRを実際にコンパイルします。`bash scripts/test.sh`も、Clangがあるか明示指定された場合にこの検査を行います。不適合や明示指定したツールの不在を成功や無条件スキップとして扱いません。環境変数は現在のshellに適用され、新しいshellでは再指定してください。

## テストとCI

コード変更前後の基本的な確認：

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

変更内容に応じて、関連するテストを実行してください。

| 確認内容 | コマンド |
| --- | --- |
| サンプルの期待値 | `cargo test --test examples` |
| 実行トレースなどのCLI出力 | `cargo test --test execution_output_cli` |
| 整形・Clippy・全テストターゲット（Bash/WSL） | `bash scripts/test.sh` |

実行結果やコード生成に影響する変更では、期待値、関係する経路、必要に応じて停止理由・ソース位置・実行トレースも照合してください。バグ修正では、可能な範囲で修正前の問題を検出できる回帰テストを追加してください。外部への副作用を伴う比較は、入力と実行環境を揃え、出力先などを分離してください。

[CI](.github/workflows/ci.yml)ではLinux・Windows・macOSの対応範囲に応じた検証、Lean、ホスト呼出し、子プロセス、runtime配布試験などを行い、`ci-gate` がCI内の必要なジョブ群の結果を集約します。[Dependency Review](.github/workflows/dependency-review.yml)では、新たに導入される既知の高・重大度の脆弱性を確認します。

一部の統合テストには外部ツールや特定のOSが必要です。すべての環境を用意する必要はありません。手元で実行できなかった検証、スキップされた検証、その理由をPRに記載してください。

## 検証・技術実験

言語本体の機能追加に加え、実行経路間の差分検証、意味論モデルとの比較、生成コードの観測、性能測定などの実験も歓迎します。[検証実験](experiments/README.md)は通常の実行サンプルとは分けて管理しています。

実験結果には、対象コミット、OS・アーキテクチャ、使用ツールとそのバージョン、入力・seed（使用する場合）、実行手順、期待値や比較基準、結果および検証範囲を記録してください。仮説と異なる結果や、再現できなかった結果も今後の検討に役立ちます。

有限例の検証結果を処理系全体の保証とせず、何を確認できたかを明確にしてください。詳細な実験手順は各実験のREADMEにまとめます。

## ドキュメント

仕様や設計判断を変更する場合は、対応する日本語・英語の文書、必要に応じてサンプルと期待値を更新してください。公開ガイドの日英ペアやリンクの確認方法は[文書一覧](docs/README.md)にあります。

Node.jsを使用し、リポジトリ直下で次を実行できます。

```sh
node scripts/check-docs.cjs
node --test scripts/check-docs.test.cjs
```

自動検査はローカルリンクと翻訳ペアなどを確認します。日英の内容が一致することや、説明が現在の実装に合っていることも確認してください。

## Pull Request

- 1つのPRでは目的をなるべく絞ってください。
- 変更内容・理由、確認したコマンドや結果、関連Issueがあれば記載してください。
- 実行結果やコード生成に影響する変更では、関係するテスト・サンプル・文書を追加または更新してください。
- 実行できなかった検証や、分かっている制限はそのまま記載してください。
- 観測成果物や実験ログは必要なものだけを共有し、意図しない生成物や秘密情報をコミットしないよう確認してください。

小さな修正や質問からでも歓迎です。
