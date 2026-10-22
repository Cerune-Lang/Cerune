# Native runtimeの型付き構築への移行

[English](native-typed-runtime.en.md)

[SASHIMI接続](native-encoder.ja.md)の次段階です。命令選択・ABI・runtimeの意味はCeruneに残し、機械命令とラベルをSASHIMIの値として構築します。

## 実装状況

| 部分 | 現在の状態 |
| --- | --- |
| UTF-8検証 | `utf8::append` がProgramへ直接追加。標準入力・プログラム引数で共有 |
| 文字列比較 | `string::append` がProgramへ直接追加。Windows/Linuxの引数レジスタはCerune側で明示選択 |
| 補助関数単体のASM/Object | 同じProgramをSASHIMIへ渡して生成。Object生成に文字列解析なし |
| Ceruneプログラム全体 | 移行中。上記補助関数をASMへ表示して既存出力に組み込むため、最終Object生成にはまだASM再解析がある |
| 残る移行 | 定数・データ、起動処理、入出力・heap・array等の補助、LIR命令選択、ソース出自との接続 |

現段階を「ASM再解析の除去完了」とは扱いません。移行用の `machine::render_support` は検証済みASMの表示を既存出力へ合わせるだけで、命令・オペランドを解析しません。SASHIMIの位置コメントと最終stack-noteは外側のemitterに任せます。全体をProgramで共有できた段階でこのadapterを取り除きます。

## 保持する契約

- UTF-8検証は入力バイトを変更せず、空列・NUL・CR/LFを含む有効列を受理します。過長表現、surrogate、範囲外コードポイント、不完全列を拒否します。
- 文字列比較は長さと生バイトで行います。NULで打ち切らず、Unicode正規化・改行変換をしません。
- ABIと文字列レイアウトは従来どおりです。UTF-8内部呼出しはrdx=先頭、rcx=長さ、eax=結果。文字列比較はtargetの引数レジスタ、先頭8バイトの長さと後続バイトを使います。
- runtime内部の命令とラベルはSyntheticとして記録します。架空のNodeId/Spanは作りません。呼出箇所の既存出自ラベル・失敗診断は維持します。
- 命令・ラベルの順序、分岐、整列を保持し、最適化は加えません。対応表は観測用であり実行制御に使いません。

## 検証と既存例

[移行前のASM](../../tests/support/native_runtime_reference/README.md)をテスト専用に固定しています。型付きProgramのObjectを旧エンコーダのObjectと両targetで比較します。135例の接続比較でも、変更した補助関数を固定ASMへ戻した基準とObject全体を照合します。

独立した実行検証では、同じProgramのASM/Objectをclangでリンクし、Windows/Linuxで以下を比較します。LinuxではPIEを有効にします。

| 検証 | 期待値 |
| --- | --- |
| UTF-8全1バイト・全2バイト列、3/4バイト境界・不完全列・日本語 | RustのUTF-8検証 |
| 空文字、日本語、埋込みNUL、CR/LF、結合文字と合成済み文字の組合せ | 生バイト列の等値比較 |
| 補助関数の出自 | Synthetic、命令の非空区間、ラベルの空区間、Section切替の範囲なし |
| 表示・生成の繰返し | 入力ProgramとObjectが不変 |

既存の[string_values](../../examples/string_values.ceru)、[io_protocol/input_flow](../../examples/io_protocol/input_flow.ceru)、[arguments/list](../../examples/arguments/list.ceru)が関連する実行例です。Native引数テストは実例をMIR/SSA、ASM/Object、4実行器で比較し、I/O順・コピー後の独立性・異常停止も検証します。

```sh
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::machine::tests
cargo test --lib codegen::x86_64::object::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --test standard_input program_arguments::native
```

PowerShellでは先に `$env:CERUNE_TEST_ASM_CLANG = 'clang'` を設定します。未設定時の単独実行比較は未実施と表示します。CIのWindows/Linux coreはこの環境変数を設定し、ツールが失敗した場合はテストも失敗します。

## 次の順序

1. 定数・データと残りのruntime補助を同じProgramへ追加できる形に移す。
2. LIRの命令選択、ABIの起動・終了、NodeId/SourceId/Span・MIR出自を接続する。
3. ASM/Object全体を同じProgramから生成し、既存の診断・出力順・実行結果を比較する。
4. 生成ASMの再解析と移行用adapterを除去し、旧比較基準の退役を判断する。

整数読取り等の言語機能追加は、この移行を固めた後に戻ります。
