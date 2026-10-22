# Native runtimeの型付き構築への移行

[English](native-typed-runtime.en.md)

[SASHIMI接続](native-encoder.ja.md)の次段階です。命令選択・ABI・runtimeの意味はCeruneに残し、機械命令とラベルをSASHIMIの値として構築します。

## 実装状況

| 部分 | 現在の状態 |
| --- | --- |
| UTF-8検証 | `utf8::append` がProgramへ直接追加。標準入力・プログラム引数で共有 |
| 文字列比較 | `string::append` がProgramへ直接追加。Windows/Linuxの引数レジスタはCerune側で明示選択 |
| 文字列heap・配列owner | `heap::append` / `array::append` が確保・参照数・解放と書込み可能な管理データをProgramへ直接追加 |
| 通常出力 | `output::append` が数値・真偽値・生文字列・引用表示の整形、書込み、再試行、失敗診断をProgramへ直接追加 |
| 読取り専用の定数 | `data::append` が符号マスク・f32/f64のビット列・長さ付き文字列をProgramへ直接追加 |
| 補助関数・定数単体のASM/Object | 同じProgramをSASHIMIへ渡して生成。Object生成に文字列解析なし |
| Ceruneプログラム全体 | 移行中。上記補助関数・定数をASMへ表示して既存出力に組み込むため、最終Object生成にはまだASM再解析がある |
| 残る移行 | 起動処理、標準入力・引数取得の補助と管理データ、LIR命令選択（配列操作・出力の呼出側とflushを含む）、ソース出自との接続 |

現段階を「ASM再解析の除去完了」とは扱いません。移行用の `machine::render_support` は検証済みASMの表示を既存出力へ合わせるだけで、命令・オペランドを解析しません。SASHIMIの位置コメントと最終stack-noteは外側のemitterに任せます。旧Object入口は外部callのシンボル名からPLT32を生成するため、このadapterでは `.extern` と `@PLT` の表示を省きます。直接Programから生成するASMには両方を残します。全体をProgramで共有できた段階でこのadapterを取り除きます。

## 保持する契約

- UTF-8検証は入力バイトを変更せず、空列・NUL・CR/LFを含む有効列を受理します。過長表現、surrogate、範囲外コードポイント、不完全列を拒否します。
- 文字列比較は長さと生バイトで行います。NULで打ち切らず、Unicode正規化・改行変換をしません。
- ABIと文字列レイアウトは従来どおりです。UTF-8内部呼出しはrdx=先頭、rcx=長さ、eax=結果。文字列比較はtargetの引数レジスタ、先頭8バイトの長さと後続バイトを使います。
- runtime内部の命令・ラベルと定数プールはSyntheticとして記録します。架空のNodeId/Spanは作りません。呼出箇所の既存出自ラベル・失敗診断は維持します。
- 命令・ラベルの順序、分岐、整列を保持し、最適化は加えません。対応表は観測用であり実行制御に使いません。

文字列heapと配列ownerは、容量の加算・参照数・最後の解放・エラー値・Windows/Linuxの異常停止方法を維持します。`malloc` / `free` / `memcpy` は明示的な外部宣言を共有します。管理データと補助命令もSyntheticであり、公開I/O契約や上限値を変更しません。

通常出力は要求・受理バイト数と呼出元の出自を保持します。短い書込みは続きを送り、EINTRは再試行し、0進行・契約外の受理数・書込み失敗ではio-v1診断を出して停止します。LinuxのSIGPIPE処理、数値の書式、引用表示のエスケープ順序も維持します。整形・診断用のC文字列は終端NUL込みの `Data` として構築し、ASMでは `.byte` で表示します（利用者の文字列は従来の長さ付き表現のままです）。

## 定数の表示と出自

SASHIMIの `Data32` / `Data64` はリトルエンディアンの4/8バイトを保持し、ASMでは `.long` / `.quad` の16進表記で幅を表示します。浮動小数点として再計算せず、整列は明示した `Align` のみで行います。文字列は8バイトのバイト長と内容で、終端NULを追加しません。

現在のLIR定数プールは個別のソース位置を持たないためSyntheticとします。定数を使う命令のNodeId/Spanは従来の出自に残します。全体の出自とProgramの位置対応を接続する作業は後続です。

## 検証と既存例

[移行前のASM](../../tests/support/native_runtime_reference/README.md)をテスト専用に固定しています。型付きProgramのObjectを旧エンコーダのObjectと両targetで比較します。定数データは[旧emitter](../../tests/support/native_data_reference.rs)を0eca28aからテスト専用に固定しています。135例の接続比較では、変更した補助関数・定数・確保管理データを旧出力へ戻した基準とObject全体を照合します。現在のASM表記を確認する観測fixtureは更新しますが、この旧比較基準は更新しません。

独立した実行検証では、同じProgramのASM/Objectをclangでリンクし、Windows/Linuxで以下を比較します。LinuxではPIEを有効にします。

| 検証 | 期待値 |
| --- | --- |
| UTF-8全1バイト・全2バイト列、3/4バイト境界・不完全列・日本語 | RustのUTF-8検証 |
| 空文字、日本語、埋込みNUL、CR/LF、結合文字と合成済み文字の組合せ | 生バイト列の等値比較 |
| f32/f64の±0・非正規化数・±∞・NaN payload、符号マスク | 元のビット列、幅、整列、旧Objectとの一致 |
| 長さ付き文字列と空の内容 | 8バイトの長さと生バイト、8バイト整列、空内容の0バイト区間 |
| 文字列・配列の参照数と容量 | 保持中は容量を再使用せず、最後の解放後に回復。空値・要素幅0・サイズ不正・上限超過を区別 |
| 通常出力の成功・失敗 | ASM/Objectと4実行器で本文・io-v1診断・停止を照合。スカラー整形・日本語/NUL/CRLF・複合値・短い書込み・EINTR・0進行・不正受理数・閉じたstdoutを検証 |
| 補助関数・定数の出自 | Synthetic、命令の非空区間、ラベルの空区間、Section切替の範囲なし |
| 表示・生成の繰返し | 入力ProgramとObjectが不変 |

通常出力の実行例は[io_protocol/output_values](../../examples/io_protocol/output_values.ceru)です。既存の[string_values](../../examples/string_values.ceru)、[io_protocol/input_flow](../../examples/io_protocol/input_flow.ceru)、[arguments/list](../../examples/arguments/list.ceru)が関連する実行例です。Native引数テストは実例をMIR/SSA、ASM/Object、4実行器で比較し、I/O順・コピー後の独立性・異常停止も検証します。

```sh
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::machine::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::data::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::heap::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::output
CERUNE_TEST_ASM_CLANG=clang cargo test --test native_output
cargo test --lib codegen::x86_64::object::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --test standard_input program_arguments::native
```

PowerShellでは先に `$env:CERUNE_TEST_ASM_CLANG = 'clang'` を設定します。未設定時の単独実行比較は未実施と表示します。CIのWindows/Linux coreはこの環境変数を設定し、ツールが失敗した場合はテストも失敗します。

## 次の順序

1. 標準入力・引数取得のruntime補助と管理データを同じProgramへ追加できる形に移す。
2. LIRの命令選択、ABIの起動・終了、NodeId/SourceId/Span・MIR出自を接続する。
3. ASM/Object全体を同じProgramから生成し、既存の診断・出力順・実行結果を比較する。
4. 生成ASMの再解析と移行用adapterを除去し、旧比較基準の退役を判断する。

この作業の完了条件は、Cerune全体のASMとObjectが同じ型付きProgramを使い、生成ASMの再解析を必要とせず、ソース出自・失敗診断・I/O順・既存例の実行結果を保持することです。整数読取り等の言語機能追加は、ここを確認して戻ります。SASHIMIのCerune言語への移植は別の将来作業です。
