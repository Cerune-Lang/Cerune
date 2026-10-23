# 自前のx86-64エンコーダ

[English](native-encoder.en.md)

## 担当する範囲

`emit-obj`は同じプロセス内の固定版SASHIMIでx86-64命令を符号化し、LinuxのELF64またはWindowsのCOFFオブジェクトを返します。外部アセンブラ、JIT、実行ファイルの起動は不要です。リンクは明示した外部リンカの担当です。

```text
HIR → MIR（明示SSA経由も可）→ x86-64 LIR → 型付きProgram
                                         ├→ ASM表示 → emit-asm → 外部アセンブラ
                                         └→ 直接符号化 → emit-obj → ELF/COFF
```

言語の意味と命令選択は既存の経路と共有します。CeruneがOpcode・Operand・Symbol・Section・データ・整列を構築し、SASHIMIへ型付きProgramを渡します。Object生成はASM文字列を経由しません。型推論、演算の失敗条件、評価順、コピーの規則を実装し直さず、任意のASMを受け取るCLIも設けません。

現在の言語機能を通すため、整数8型、f32/f64、真偽値、文字列、配列・構造体、関数、制御構文に必要な命令形式を実装しています。汎用x86-64命令全体、AVX、短い分岐への最適化、自前リンカは対象外です。新しい命令形式を生成する変更は、エンコーダが未対応なら診断で停止します。外部アセンブラへの自動フォールバックはありません。

## SASHIMIへの接続（2026-10-23）

Object生成は[SASHIMI](https://github.com/Cerune-Lang/SASHIMI/tree/ee8f6b132aea322af3d4ff1909035c7903836cd5)のRustライブラリへ接続しました。Cargo.tomlの完全なコミットIDとCargo.lockで版を固定します。隣のSASHIMI cloneは調査・開発用で、Ceruneのビルドにローカルpath依存はありません。初回ビルドには依存ソースの取得が必要ですが、生成した実行ファイルにSASHIMIの別プロセスや動的ライブラリは不要です。

| 担当 | 責務 |
| --- | --- |
| Cerune | MIR→LIR、命令選択、ABI、起動・runtime、出自ラベル、CLI・観測bundle、リンク後の言語動作の検証 |
| SASHIMI | 型付きProgramの検証、ASM表示、命令符号化、ラベル解決、再配置、ELF64/COFFと位置対応の出力 |
| 接続コード | 明示Target、OriginIdとCeruneの出自、符号化診断のSpanを対応付ける |

[SASHIMI PR #5](https://github.com/Cerune-Lang/SASHIMI/pull/5)で、同じ型付きProgramからASM・Object・OriginIdの出自対応表を生成するAPIを実装しました。[SASHIMI PR #6](https://github.com/Cerune-Lang/SASHIMI/pull/6)でData32/Data64を追加し、[SASHIMI PR #7](https://github.com/Cerune-Lang/SASHIMI/pull/7)の読める終端付き文字列対応へ固定して既存経路の互換性を検証しています。本番接続は[プログラム全体の型付き構築](native-typed-runtime.ja.md)へ移行しました。起動・関数ABI・入力・引数・出力・flush・確保管理・全LIR命令と失敗診断が対象です。内部／外部シンボルは明示宣言し、同じProgramからASMとObjectを生成します。安定した公開ABIは別の将来作業です。

[旧エンコーダ](../../tests/support/native_object_reference/README.md)は移行比較用にテスト専用へ移し、本番ビルドには含めません。新機能の追加先にはしません。[接続テスト](../../tests/support/sashimi_connection.rs)は135個の実行用exampleについて、両ターゲット・通常MIR/SSA・出自注釈の有無を組み合わせ、Object全体と失敗診断を旧実装に照合します。この一致は切替前後の同等性であり、双方に共通する不具合を否定する証明ではありません。外部アセンブラとの比較・既知の期待値・リンク後実行も維持します。

SASHIMIはOriginIdに複数範囲・0バイトの操作・合成命令を対応させます。位置はObjectのセクション内オフセットであり、リンク後アドレスではありません。Ceruneの `NativeProgram::origins()` がNodeId／SourceId／Span・MIR・LIRの対応を保持します。`assemble()` はObjectと位置対応を返し、`render_asm()` は同じProgramを表示します。[Cerune言語へのSASHIMI移植](https://github.com/Cerune-Lang/SASHIMI/issues/2)は別段階です。

契約と実行例は[SASHIMIのProgram API](https://github.com/Cerune-Lang/SASHIMI/blob/ee8f6b132aea322af3d4ff1909035c7903836cd5/docs/design/typed-program.ja.md)を参照してください。新APIはELFの外部call・jmp・jccにPLT32を使用し、ASM表示にも@PLTを付け、Windows/LinuxのASM/Object実行比較を行います。旧テキスト入口の外部jmpは互換性のためPC32のままです。現在のCerune生成と失敗注入用ObjectはABIに従うcallを使います。

## 符号化と再配置

- レガシープレフィックス、REX、ModR/M、SIB、変位、即値を命令形式に従って書きます。レジスタの種別・幅、アドレスの倍率、即値の範囲を検査します。
- ローカル分岐は32ビット相対変位で固定します。`.text`内のラベル参照は解決し、範囲外の変位は拒否します。
- 定数・文字列は読み取り専用セクションへ置きます。RIP相対参照はELFのPC32、外部呼び出しはPLT32、COFFではREL32として残します。変位の後ろに即値がある形式も、命令末尾を基準にaddendを計算します。
- 動的文字列の管理情報は非実行・書き込み可能な`.data`へ配置します。静的文字列の読み取り専用属性は維持します。
- ELFには空の`.note.GNU-stack`を設け、実行スタックを要求しません。COFFのタイムスタンプは0固定です。シンボルとセクションの順序も固定します。
- COFFの再配置は65,535件までです。セクション位置、数値、変位を狭いフィールドへ黙って切り捨てません。

自前版は即値や分岐の長い形式と単一バイトNOPによる整列を使う場合があります。外部アセンブラと命令バイトやアドレスが同じとは限りません。同じ言語の意味と実行結果を比較します。自前版同士では同じ入力・ターゲット・オプションから同じオブジェクトを得られます。

`--annotate-origins`は既存の出自ラベルをシンボルとして残します。有無によって`.text`、定数・文字列、再配置の意味は変わりません。オブジェクトのシンボル表自体は変わります。生成物は観測用データであり、外部からコンパイラ内部を変更する口にはしません。

## 使用例

バイナリを端末へ暗黙に出さないため、`--target`と`-o`は必須です。出力はリンク前のオブジェクトです。

```powershell
cargo build
target/debug/cerune.exe emit-obj examples/packet_counter.ceru --target x86_64-pc-windows-msvc --annotate-origins -o target/packet.obj
clang target/packet.obj -o target/packet.exe
target/packet.exe
```

```sh
cargo build
target/debug/cerune emit-obj examples/packet_counter.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/packet.o
cc target/packet.o -o target/packet
target/packet
```

WSLで`CARGO_TARGET_DIR=target/unix`を使う場合は`target/unix/debug/cerune`を指定します。エンコードだけなら、実行中のOSと生成先は一致しなくても構いません。実行する環境とリンク時のライブラリは生成先に合わせます。

観測スクリプトでは`--encoder cerune`を明示します。既定の`external`は比較対象として残ります。

```powershell
node scripts/observe-native.cjs --source examples/packet_counter.ceru --target x86_64-pc-windows-msvc --cerune target/debug/cerune.exe --cc clang --objdump llvm-objdump --output-dir target/packet-own-windows --encoder cerune --run
```

`manifest.json`は`encoder: cerune`と`encode-object`工程を記録します。Cドライバはリンクだけに使います。IR、対応するASM、自前オブジェクト、逆アセンブル、VMとの出力比較は[ネイティブコードの観測](native-code.ja.md)と同じ手順で保存します。

## 検証

全サンプル、u64・文字列の境界値、大きな配列の引数と戻り値をWindows/Linuxでリンク・実行し、VMと外部アセンブラ版に照合します。期待する不正命令停止も両エンコーダで確認します。低レベルではREX/SIB、変位の境界、SSE2、ローカル分岐、RIP相対addendを検証し、命令バイトは独立したLLVMアセンブラで照合した期待値を保持します。PATHを空にしたCLIテストにより、オブジェクト生成が外部ツールを要求しないことも確認します。

形式の参照先は[Intelの命令セット仕様](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)、[MicrosoftのPE/COFF仕様](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format)、[ELF64形式](https://uclibc.org/docs/elf-64-gen.pdf)、[x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI)です。
