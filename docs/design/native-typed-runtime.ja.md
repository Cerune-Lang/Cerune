# Native全体の型付きProgram構築

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
| 標準入力・引数・起動処理 | 共通Programへ命令と管理データを直接追加。OSの選択は明示Targetによる |
| LIR命令選択・関数ABI・失敗診断 | 型付きOpcode・Operand・Symbolで構築。flushと全呼出箇所を含む |
| Ceruneプログラム全体 | 同じNativeProgramからASMとObjectを生成。本番のObject生成にASM再解析なし |
| 出自と生成位置 | LIRごとのNodeId/SourceId/Span・MIR出自をOriginIdで保持し、Object内の区間・シンボル・再配置に対応 |

本番のObject APIはすべて型付きProgramを直接渡します。旧ASM readerと `machine::render_support` は、凍結した比較基準と失敗注入のためのテスト専用です。本番ビルドには含めません。ASMでは明示的な `.extern` とLinux外部呼出しの `@PLT` を保持します。0変位の明示や即値の10進表示など、表記はSASHIMIの表示規則へ統一します。命令・データ・再配置の意味は変えません。

## 保持する契約

- UTF-8検証は入力バイトを変更せず、空列・NUL・CR/LFを含む有効列を受理します。過長表現、surrogate、範囲外コードポイント、不完全列を拒否します。
- 文字列比較は長さと生バイトで行います。NULで打ち切らず、Unicode正規化・改行変換をしません。
- ABIと文字列レイアウトは従来どおりです。UTF-8内部呼出しはrdx=先頭、rcx=長さ、eax=結果。文字列比較はtargetの引数レジスタ、先頭8バイトの長さと後続バイトを使います。
- runtime内部の命令・ラベルと定数プールはSyntheticとして記録します。架空のNodeId/Spanは作りません。呼出箇所の既存出自ラベル・失敗診断は維持します。
- 命令・ラベルの順序、分岐、整列を保持し、最適化は加えません。対応表は観測用であり実行制御に使いません。

文字列heapと配列ownerは、容量の加算・参照数・最後の解放・エラー値・Windows/Linuxの異常停止方法を維持します。`malloc` / `free` / `memcpy` は明示的な外部宣言を共有します。管理データと補助命令もSyntheticであり、公開I/O契約や上限値を変更しません。

通常出力は要求・受理バイト数と呼出元の出自を保持します。短い書込みは続きを送り、EINTRは再試行し、0進行・契約外の受理数・書込み失敗ではio-v1診断を出して停止します。LinuxのSIGPIPE処理、数値の書式、引用表示のエスケープ順序も維持します。整形・診断用のC文字列は `CString` として構築し、ASMでは書式を読める `.asciz` で表示します（利用者の文字列は従来の長さ付き表現のままです）。

## 定数の表示と出自

SASHIMIの `Data32` / `Data64` はリトルエンディアンの4/8バイトを保持し、ASMでは `.long` / `.quad` の16進表記で幅を表示します。浮動小数点として再計算せず、整列は明示した `Align` のみで行います。文字列は8バイトのバイト長と内容で、終端NULを追加しません。

現在のLIR定数プールは個別のソース位置を持たないためSyntheticとします。定数を使う命令のNodeId/Spanは従来の出自に残します。ロード命令の参照フィールドから定数シンボルと位置へ辿れます。

## 検証と既存例

[移行前のASM](../../tests/support/native_runtime_reference/README.md)をテスト専用に固定しています。型付きProgramのObjectを旧エンコーダのObjectと両targetで比較します。定数データは[旧emitter](../../tests/support/native_data_reference.rs)を0eca28aからテスト専用に固定しています。[全体の旧命令選択](../../tests/support/native_emitter_reference/README.md)もeef765fから凍結しています。135例×両target×MIR/SSA×注釈有無の比較では、この旧生成器と固定runtime・定数を基準に、型付きProgramから直接生成したObject全体をバイト単位で照合します。現在のASM表記を確認する観測fixtureは更新しますが、この旧比較基準は更新しません。

独立した実行検証では、同じProgramのASM/Objectをclangでリンクし、Windows/Linuxで以下を比較します。LinuxではPIEを有効にします。

| 検証 | 期待値 |
| --- | --- |
| UTF-8全1バイト・全2バイト列、3/4バイト境界・不完全列・日本語 | RustのUTF-8検証 |
| 空文字、日本語、埋込みNUL、CR/LF、結合文字と合成済み文字の組合せ | 生バイト列の等値比較 |
| f32/f64の±0・非正規化数・±∞・NaN payload、符号マスク | 元のビット列、幅、整列、旧Objectとの一致 |
| 長さ付き文字列と空の内容 | 8バイトの長さと生バイト、8バイト整列、空内容の0バイト区間 |
| 文字列・配列の参照数と容量 | 保持中は容量を再使用せず、最後の解放後に回復。空値・要素幅0・サイズ不正・上限超過を区別 |
| 通常出力の成功・失敗 | ASM/Objectと4実行器で本文・io-v1診断・停止を照合。スカラー整形・日本語/NUL/CRLF・複合値・短い書込み・EINTR・0進行・不正受理数・閉じたstdoutを検証 |
| 引数取得の確保失敗 | テスト所有のProgramコピーでmallocの呼出先を置き換え、同じProgramのASM/Objectで起動時停止・ソース位置付き診断・先行出力を比較 |
| 入力のOS応答 | ASMの外部呼出しを明示的に置き換え、EOF・中断・失敗・不正応答・消費数を比較。注入先が見つからなければテスト失敗 |
| 補助関数・定数の出自 | Synthetic、命令の非空区間、ラベルの空区間、Section切替の範囲なし |
| 表示・生成の繰返し | 入力ProgramとObjectが不変 |

通常出力の実行例は[io_protocol/output_values](../../examples/io_protocol/output_values.ceru)です。既存の[string_values](../../examples/string_values.ceru)、[io_protocol/input_flow](../../examples/io_protocol/input_flow.ceru)、[arguments/list](../../examples/arguments/list.ceru)が関連する実行例です。Native引数テストは実例をMIR/SSA、ASM/Object、4実行器で比較し、I/O順・コピー後の独立性・異常停止も検証します。

```sh
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::machine::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::data::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::heap::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::output
CERUNE_TEST_ASM_CLANG=clang cargo test --test native_output
cargo test --test native_program
cargo test --lib codegen::x86_64::object::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --test standard_input program_arguments::native
```

PowerShellでは先に `$env:CERUNE_TEST_ASM_CLANG = 'clang'` を設定します。未設定時の単独実行比較は未実施と表示します。CIのWindows/Linux coreはこの環境変数を設定し、ツールが失敗した場合はテストも失敗します。

## 出自の観測API

`build_program_from_lir` はLIRを検証して `NativeProgram` を構築します。構築後は読取り専用の `program()`、`origins()`、`render_asm()`、`assemble()` を使います。

- `origins()` の各項目は関数番号（mainはNone）、LIR命令番号、既存のOriginとMirOriginを保持します。OriginIdはこの表の添字で、NodeIdそのものではありません。SourceIdを含むSpanをそのまま保持します。
- 各LIR操作に0バイトのMarkerを置きます。ObserveOnlyにも位置が残り、一つの操作から複数の機械命令・診断データへの対応を追えます。
- `assemble()` のArtifactはObject、各entryのセクション内区間、シンボル位置、参照フィールドを返します。リンク後アドレスや外部アセンブラが選ぶ命令長とは区別します。
- ABIの準備・終了とruntime内部はSyntheticです。元のLIRがSyntheticである場合も元のMIR情報を残し、架空のNodeIdを作りません。
- 注釈の有無にかかわらず対応表を保持します。`--annotate-origins` は従来のコメント・ラベルを加える表示オプションです。符号化エラーに出自がある場合は元のSpanを診断へ渡します。

ASM表示はSASHIMIのentry区切りにCeruneのコメントを配置します。命令文字列を読んでObjectを作る処理ではありません。表示・位置参照・繰返しのObject生成で入力Programを変更しないことを検証します。

## 今回の区切り

全体のProgram共有、出自対応、ASM再解析の本番経路からの除去まで実装しました。凍結した旧生成器は移行同等性の検証に限って保持し、新機能は追加しません。この検証とCIを完了して整数読取り等の言語機能へ戻ります。新しい最適化、公開ABI、SASHIMIのCerune言語への移植は別の作業です。
