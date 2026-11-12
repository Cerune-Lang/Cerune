<div align="center">

<h1><img src="docs/images/assets/banner.png" alt="Cerune" width="960"></h1>

日本語 | [English](README.en.md)

Ceruneは、静的型付け・IR直接実行・VM実行・複数形式へのコード生成に対応する実験用プログラミング言語です。

[![CI](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml/badge.svg)](https://github.com/Cerune-Lang/Cerune/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Cerune-Lang/Cerune?label=release)](https://github.com/Cerune-Lang/Cerune/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#ライセンス)
[![Prebuilt: Windows, Linux, macOS](https://img.shields.io/badge/prebuilt-Windows%20%7C%20Linux%20%7C%20macOS-555)](https://github.com/Cerune-Lang/Cerune/releases/latest)

[型・用途別サンプル](examples/README.md) · [言語仕様](docs/reference/language.ja.md) · [CLI](docs/reference/cli.ja.md)  
[設計文書](docs/README.md) · [今後の計画](docs/design/language-roadmap.ja.md) · [Primerからの移行](docs/design/naming.ja.md)

</div>

## インストール

Cerune v0.1.0は、Windows・Linux・macOS向けのビルド済みバイナリを[GitHub Releases](https://github.com/Cerune-Lang/Cerune/releases/tag/v0.1.0)からインストールできます。

### リリース版

**Windows（PowerShell）**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/Cerune-Lang/Cerune/releases/download/v0.1.0/cerune-lang-installer.ps1 | iex"
```

**Linux / macOS（Shell）**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Cerune-Lang/Cerune/releases/download/v0.1.0/cerune-lang-installer.sh | sh
```

### 開発版をソースからビルド

Rust/Cargoがあれば、`master`ブランチの最新の実装をソースからビルドして試せます。

```sh
git clone https://github.com/Cerune-Lang/Cerune.git
cd Cerune
cargo run --quiet -- run examples/floating_point.ceru
```

ソースからインストールする場合：

```sh
cargo install --path .
```

## クイックスタート

インストールしたら、作業ディレクトリに `hello.ceru` というファイルを作成します。

```cerune
print("Hello, Cerune!");
```

```sh
cerune run hello.ceru
```

`Hello, Cerune!` と表示されれば成功です。基本的なIR・MIR・VM実行には、Cコンパイラなどの外部ツールは必要ありません。

### 実行経路を比較する

以下はリポジトリ内のサンプルを使います。`examples/` があるディレクトリで実行してください（必要なら上記の手順でリポジトリをクローンします）。

```cerune
print("Hello, Cerune!");
a: f32 = 0.1 + 0.2;
b: f64 = 0.1 + 0.2;
print(a);
print(b);

limit: u8 = 255;
unused: u8 = limit + 1;
print(99);
```

```sh
cerune run examples/runtime_failures/hello_cerune.ceru
cerune run-mir examples/runtime_failures/hello_cerune.ceru
cerune run-vm examples/runtime_failures/hello_cerune.ceru
cerune emit-ir examples/runtime_failures/hello_cerune.ceru
```

最初に`Hello, Cerune!`と型ごとの計算結果を表示します。どの実行経路でも、使われない`unused`の加算で`integer-overflow`となり、`99`は表示されません。

開発中は`cerune`を`cargo run --quiet --`に置き換えられます。

## 対応・推奨環境

| 用途 | 対応・推奨環境 |
| --- | --- |
| 配布版（v0.1.0） | Windows x64、Linux x64 / ARM64、macOS Intel（x64）/ Apple Silicon（ARM64）向けのバイナリを提供 |
| 基本的な実行（`run` / `run-mir` / `run-vm`） | 対応する配布版のCerune本体。Clang・Node.js・WABTは不要 |
| ソースからのビルド・開発 | Git、Rust / Cargo。リポジトリは `rust-toolchain.toml` で Rust 1.98.0 を指定 |
| コード生成結果のコンパイル・実行 | 経路に応じたClang / GCC、QBE 1.3、WABT、OSのリンカ・SDKなど（[外部ツール](#外部ツール)参照） |

※ これは配布対象と用途別の推奨構成です。最低OSバージョンやCPU・メモリ容量の動作保証ではありません。生成バックエンドの対応ターゲットは経路により異なります。

## 実行と出力

| コマンド | 結果・成果物 | 用途・対象 |
| --- | --- | --- |
| `build native` | 実行ファイル・生成過程・リンク記録 | Windows/Linux targetとClangパスを明示 |
| `release native` | 配布ディレクトリ・依存条件・SHA-256 | 成功済みbuildを再生成せず梱包 |
| `fmt` | ソース整形 | コメント保持、letの保持・表示・非表示、`--check`・明示保存 |
| `check` | 構文・型検査 | Ceruneソース（`.ceru`）を検証 |
| `run` / `run-ir` | IR直接実行 | Ceruneソース（`.ceru`）を共通IRから実行 |
| `run-mir` | MIR実行 | 制御フローへ変換して比較。`--ssa`でSSAを直接実行 |
| `run-vm` | VM実行 | Ceruneソース（`.ceru`）を実行 |
| `emit-sources` | ソース一覧（JSON） | 読み込んだファイル名・本文を出力 |
| `emit-ir` | Cerune IR（`.ceir`） | 型・演算を確認 |
| `observe` | sources・HIR・MIR・注釈付きASMとmanifest | 同一コンパイルを保存（target必須）。`--ssa`でSSA経由、`--lir`でLIRも保存 |
| `emit-mir` | MIRの観測テキスト | 一時値・分岐・所有操作・出自。`--ssa`で元MIRとSSAを表示 |
| `emit-lir` | Native LIRの観測テキスト | target必須。命令・frame・ABI・出自。`--ssa`でSSA経由 |
| `emit-bytecode` | bytecodeテキスト（`.cebc`） | 命令列を確認 |
| `emit-c` | C（`.c`） | GCC / Clangなど |
| `emit-llvm` | LLVM IR（`.ll`） | LLVM / Clang、Windows / Linux x86-64 |
| `emit-qbe` | QBE IR（`.ssa`） | QBE 1.3、Windows x64 / Linux x86-64（明示ターゲット） |
| `emit-wat` | WebAssembly Text（`.wat`） | WebAssembly用ツールとホスト |
| `emit-asm` | アセンブリ（`.s`） | Windows / Linux x86-64。`--ssa`でSSA経由を選択 |
| `emit-obj` | ELF / COFFオブジェクト（`.o` / `.obj`） | 外部リンカ。`--target`・`-o`必須、`--ssa`対応 |

テキストは標準出力へ、`-o`でファイルへ保存します。LLVM・QBEは`--target`が必須です。WATは出力・診断用のホスト関数を使います。

## 機能

### 型

| 分類 | 型・操作 |
| --- | --- |
| 符号付き整数 | `i8`・`i16`・`i32`・`i64` |
| 符号なし整数 | `u8`・`u16`・`u32`・`u64` |
| 浮動小数点 | `f32`・`f64` |
| 真偽値 | `bool`（`true` / `false`） |
| 文字列 | 不変なUTF-8の`string`。表示、等値比較、`byte_len`、`concat`による連結 |

### 構文・操作

| 機能 | 対応内容 |
| --- | --- |
| 変数 | 任意の`let`、型指定・`infer`または`: =`による型推論。既定は再代入不可、`mut`で再代入可 |
| 演算 | 算術・剰余・比較・ビット演算、論理否定、短絡評価。複合値の`==`・`!=` |
| 数値変換 | `convert<T>(x)` / `T(x)`で値を保持。整数化は丸め方と飽和を明示して選択 |
| 構造体 | 名前付きの型、フィールド参照・既定値、部分変更で新しい値を作る更新式、入れ子、値コピー |
| 直和型 | `enum`の選択肢と値、網羅的な`match`文・式、`if`ガード |
| 配列 | 固定長配列（長さに定数も指定可）、`array_len`で要素数取得、入れ子、要素の参照・更新、値コピー |
| 動的配列 | `[T]`、`array_copy`・`array_copy_range`・`array_repeat`、独立コピー、表示・比較・反復 |
| 関数 | 型付き引数・戻り値、`void`、`return`、型・配列長のパラメーター、直接・相互再帰。複合値も受け渡し可能 |
| 実行開始 | トップレベル実行文、または`fn main() -> void`（併用不可） |
| 制御構文 | `if` / `else`、`while`・`for`・配列の`for … in`、`break` / `continue` |
| 定数 | 型付き`const`、コンパイル時評価、`pub const`で共有 |
| モジュール | 明示的なimport、名前空間、`pub`による公開範囲 |
| 標準操作 | `std:io`の行入力、`std:env`の引数取得、`std:num`のi64読取り、`std:fs`の読取り・新規保存（Windows/Linux、明示した作業ディレクトリ） |
| 表示・診断 | `print(expr);`（複合値も可）、エラーの理由・ソース位置・停止前の出力 |

**計算の規則：** 暗黙の数値変換はありません。整数の桁あふれ、不正な整数除算、範囲外参照、正確な変換で値を保てない場合や、丸め後に範囲を超える場合は停止します。飽和を明示すると型の端へ収めます。浮動小数点計算には丸めがあります。

**主な未実装・制限：**

- **言語機能：** 文字列の公開添字参照・文字数取得・i64以外の数値読取り、動的配列の伸長・借用、実行時停止の捕捉、汎用`Option`/`Result`。
- **入出力・モジュール：** ネットワークI/O、モジュールの再エクスポート、パッケージ管理。
- **ビルド・配布：** IR/VMの`build`・`release`（Nativeの実行ファイル構築・配布ディレクトリ作成には対応）。

## サンプルと開発

| 操作 | コマンド |
| --- | --- |
| サンプルを一括実行（PowerShell） | `.\scripts\run-examples.ps1` |
| サンプルを一括実行（WSL / Bash） | `bash scripts/run-examples.sh` |
| サンプルを絞り込む | PowerShell：`-Pattern "matrix*.ceru"`、Bash：`--pattern 'matrix*.ceru'` |
| サンプルの期待値を検証 | `cargo test --test examples` |
| fmt・Clippy・全テスト（WSL / Bash） | `bash scripts/test.sh` |

WSL側にもRustが必要です。`.sh`のビルド先は既定で`target/unix`です。

## 関連リポジトリ

[Organization一覧](https://github.com/Cerune-Lang) · [各プロジェクトの役割](docs/design/ecosystem.ja.md)

## 外部ツール

生成したコードをコンパイル・リンク・実行する場合に使うツールです。Ceruneの通常のIR・MIR・VM実行には必要ありません。

| 外部ツール | 使用する経路 |
| --- | --- |
| Rust / Cargo | Cerune本体をソースからビルド・テストする場合 |
| GCC / Clang | `emit-c` |
| Clang（LLVM IR対応） | `emit-llvm` |
| [QBE 1.3](https://c9x.me/compile/releases.html) | `emit-qbe` |
| [WABT](https://github.com/WebAssembly/wabt)（`wat2wasm`） | `emit-wat` |
| Wasm実行環境（必要なホスト関数を実装） | `emit-wat` の生成物の実行 |
| Clang・対象OSのリンカと開発ライブラリ | `emit-asm` / `emit-obj` のリンク、`build native` |
| Node.js | WasmのJavaScriptホストを使うテスト |

Windows向けリンクにはMSVCツールチェーンとWindows SDK、Linux向けリンクにはCランタイムの開発ライブラリなどが必要です。`emit-obj`によるObject生成自体には外部アセンブラは不要です。

## このプロジェクトについて

Ceruneは、プログラミング言語やコンパイラの仕組みを学び、試すために個人で開発している実験的なプロジェクトです。リポジトリで使用する画像には、生成AIを利用したものが含まれる可能性があります。

参加時は[行動規範](CODE_OF_CONDUCT.md)をご確認ください。セキュリティ上の問題は[セキュリティポリシー](SECURITY.md)に沿って非公開で報告してください。

## ライセンス

Ceruneは、以下のいずれかのライセンスを選択して利用できます。

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)
