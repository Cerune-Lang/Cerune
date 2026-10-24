# Cerune CLIリファレンス

[English](cli.en.md)

この文書では、Cerune v0.1のコマンドラインインターフェースを定義します。

[Native build](../design/native-build.ja.md)と[Native release](../design/native-release.ja.md)は実装済みです。IR/VMの[build・release](../design/owned-routes.ja.md)、[Lean出力](../design/lean-verification.ja.md)は設計・検証段階です。

`emit-lir`はNative生成のLIRを表示します。targetを明示し、`--ssa`でSSA経由も選べます。[LIRの観測・検証](../design/lir-observation.ja.md)を参照してください。`run-lir`は保留で、独立実行はHIR・MIR・SSA・Bytecodeです。

## 観測結果の一括保存

`observe`はsources・HIR・MIR・注釈付きASMとmanifestを[同じコンパイルから保存](../design/observation-bundle.ja.md)します。`--target`と`-o`は必須で、両heap予算を指定できます。既存出力先は拒否します。親ディレクトリは事前に作成してください。入力名・ソース本文も保存しますが、プログラム実行・外部ツール起動・リンクは行いません。Objectや他backendの同梱は未対応です。

## コマンド

現在のCLIは次のコマンドを提供します。

```text
cerune fmt <file> [--let preserve|show|hide] [--check | --write | -o <file>]
cerune build native <file> --target <triple> --linker <clang-path> -o <new-directory> [--ssa] [--include-sources]
cerune release native <build-directory> -o <new-directory>
cerune check <file>
cerune observe <file> [--ssa] [--lir] --target <triple> -o <new-directory>
cerune emit-sources <file> [-o <sources.json>]
cerune emit-ir <file> [-o <output.ceir>]
cerune emit-mir <file> [-o <output.txt>]
cerune emit-lir <file> [--ssa] --target <triple> [-o <output.lir.txt>]
cerune emit-c <file> [-o <output.c>]
cerune emit-llvm <file> [--target <triple>] [-o <output.ll>]
cerune emit-wat <file> [-o <output.wat>]
cerune emit-qbe <file> [--target <triple>] [-o <output.ssa>]
cerune emit-asm <file> [--target <triple>] [--annotate-origins] [-o <output.s>]
cerune emit-obj <file> --target <triple> [--annotate-origins] -o <output.o>
cerune emit-bytecode <file> [-o <output.cebc>]
cerune run <file> [--diagnostic-format runtime-v1]
cerune run-ir <file> [--diagnostic-format runtime-v1]
cerune run-mir <file> [--diagnostic-format runtime-v1]
cerune run-vm <file> [--diagnostic-format runtime-v1]
cerune --version
```

## Native release

`release native <build-directory> -o <new-directory>`は成功済みbuildの実行ファイルと全規定記録を保持し、直接のruntime要求・SHA-256・日英の実行案内を追加します。外部ツールやプログラムは起動しません。target・SSA・上限・本文同梱はbuildの選択を継承し、releaseで変更できません。既存先・入力配下への出力は拒否します。

[配布契約・上限・失敗](../design/native-release.ja.md) · [実行例](../../examples/build_native/README.md)。初版はディレクトリ形式・対象システムruntime要求で、archive・installer・依存物同梱・対象環境の動作保証は含みません。

## Native build

`build native`は自前Objectを明示したClang driverでリンクします。出力ディレクトリには`program`（Linux）または`program.exe`（Windows）、非最適化HIR/MIR、選択したLIR・命令・Object、出自とリンク記録を保存します。結果は実行しません。

経路・`--target`・`--linker`・`-o`は必須です。driverの実ファイルのパスを指定し、PATH探索・暗黙のhost targetは使いません。既存出力を拒否し、親ディレクトリは事前に用意します。`--ssa`でも基準MIRを保持し、`--include-sources`で本文を追加できます。`--link-timeout-seconds`は各tool段階の上限1〜3600秒（既定60秒）です。既存のheap・argument上限4オプションに対応し、`--`以降のプログラム引数は受け付けません。

リンク失敗時はfailed状態と記録を残します。生成失敗時は出力先を作りません。build成功はホスト上の実行可否や別OS/runtime版への可搬性を保証しません。[契約](../design/native-build.ja.md) · [実行例](../../examples/build_native/README.md)。

## ソース整形

`fmt`は指定した単一ファイルの構文だけを確認して整形します。import先の読込み・型検査・実行はしません。既定は標準出力で、ファイルを変更しません。

| 選択 | 動作 |
| --- | --- |
| `--let preserve`（既定） | 元のlet有無を保持 |
| `--let show` / `hide` | 束縛の接頭辞を追加／除去。識別子のletは保持 |
| `--check` | 整形済みなら終了0、差分があれば終了1。stdout・ファイルは変更しない |
| `--write` / `-o <file>` | 入力を置換／指定先へ保存。checkを含め相互に併用不可 |

4空白、LF改行、コメント・リテラルの保持を基本とします。初期値を持つ束縛の`infer`有無は元のまま保持し、`--let`では切り替えません。読込み上限は8 MiBで、構文・UTF-8・保存エラーも終了1です。保存は一時ファイルを経由し、リンク・ディレクトリ・読取り専用ファイルへの置換を拒否します。設定ファイル・import整列・再帰的整形は未対応です。

[実行例](../../examples/formatting/README.md)と[設計](../design/formatting.ja.md)に意味と出自の検証を記載しています。


## 実行中の標準出力

`run`・`run-ir`・`run-mir`（`--ssa`含む）・`run-vm`は、実行中に標準出力へ渡します。出力先のバッファリングは残り、プログラム終了前に全バイトの配送を保証するものではありません。接続が失敗していなければ、正常終了・言語停止の最後にflushします。先行出力を終了時に再表示しません。

書込み・flush失敗は診断して終了コード1を返します。言語停止と終了時flushの両方が失敗した場合は、両方を表示します。ソースの`flush();`はその位置でflushし、失敗時は後続処理を止めます。標準入力はstd:ioのread_lineで読みます。Windows/Linuxの同期pipe・fileを対象とし、Windows consoleはunsupported-connectionで停止します。[入力契約・実行例](../design/stdin-connections.ja.md)を参照してください。[出力接続の契約](../design/standard-io.ja.md)にAPIと対応範囲を記載しています。

`--diagnostic-format runtime-v1`では、言語停止は従来の`runtime-v1`、書込み・flush失敗は`io-v1`で表示します。I/O記録は操作・要求/受理バイト数・NodeId/Spanを持ち、終了時のhost flushでは`origin=host`です。flushの要求/受理数0はOSへの配送バイト数を表しません。

## 動的文字列の予算

`observe`・`run`・`run-ir`・`run-mir`・`run-vm`・`emit-ir`・`emit-mir`・`emit-bytecode`・`emit-c`・`emit-llvm`・`emit-qbe`・`emit-wat`・`emit-asm`・`emit-obj`・`build native`では`--string-heap-limit <bytes>`を指定できます。既定は67108864（64 MiB）。値は0〜9223372036854775807の十進整数で、省略・重複・負数を拒否します。`fmt`・`check`・`emit-sources`では受け付けません。

生存する動的文字列の内容を数え、共有領域は一度だけ、静的文字列は数えません。再代入中に共存する旧値・新値は両方数えます。コンパイル時評価には独立した64 MiBがあり、この設定では変更しません。物理メモリ全体の上限ではありません。値は生成物に固定され、実行中に変更するAPIはありません。

```sh
cerune run examples/string_concat.ceru --string-heap-limit 1024
cerune emit-llvm examples/string_concat.ceru --target x86_64-unknown-linux-gnu --string-heap-limit 1024 -o concat.ll
```

## 動的配列の予算

同じコマンドで`--array-heap-limit <bytes>`も指定できます。既定値・数値形式は文字列予算と同じで、二つの予算は独立です。要素の計算幅に基づく生存領域を数え、物理メモリ全体の上限ではありません。`fmt`・`check`・`emit-sources`では受け付けません。

動的配列は`run`／`run-ir`／`run-mir`／`run-vm`とIR・MIR・bytecode・C・LLVM・QBE・WAT・ASM・自前Object出力に対応しています。各成果物には使用した配列予算を記録します。LLVM・QBEは未使用の配列型も含め`--target`が必要です。ASM・Objectもターゲットを明示して生成し、外部ツールでリンクします。

```sh
cerune run examples/dynamic_arrays/copy.ceru --array-heap-limit 1024
cerune run-vm examples/dynamic_arrays/copy.ceru --array-heap-limit 1024
```

## 検証

`fmt`以外の各コマンドの`<file>`は入口ファイルです。[モジュール](../design/modules.ja.md)のimportは宣言元のディレクトリから解決し、全依存を検査します。CLIを別ディレクトリから起動してもimport先は変わりません。対象の生成経路に必要な`--target`指定は従来通りです。

### 依存ファイルの観測

```sh
cerune emit-sources examples/modules/main.ceru -o sources.json
```

ファイル名と本文を明示的に出力します。JSONの`schema`は`cerune-sources-v1`、`files`は登録順の`id`・`name`・`text`です。`text`は正規化前の正確なUTF-8本文をJSONエスケープで保持します。import/pubのない入力のIDは既存の匿名ソース0、モジュール入力は入口1からです。実行時記録の`file`とそのファイル内の`bytes`を、この本文で照合します。生成プログラムへ本文を埋め込む操作ではありません。`-o`がなければstdoutへ出します。

`run`・`run-ir`・`run-vm`の既定診断は依存ファイルの名前・行・列を表示します。`--diagnostic-format runtime-v1`では数値の`file`を含む記録を出します。コンパイル失敗時は出力成果物を書き換えません。

### 構文・型の検証

```text
cerune check <file>
```

`cerune check`は、入力されたソースファイルの構文解析、意味検証、型検査を行います。

`check`の成功は、すべての出力経路がそのプログラムに対応することを保証しません。文字列は全経路で使用できます。LLVMとQBEのターゲットが未指定の場合は、文字列を含む型定義や式をソース位置付きで診断し、成果物を生成しません。この診断では`-o`で指定した既存ファイルも変更しません。

## Cerune IRの直接実行

```sh
cerune run examples/ir_execution.ceru
cerune run-ir examples/modules/failure.ceru --diagnostic-format runtime-v1
```

`run`はソース（`.ceru`）から共通IRを構築し、その文・式を直接実行します。`run-ir`は同じ処理・オプションを持つ別名です。現在の言語機能に対応し、Bytecodeへの変換・VMへの委譲はしません。`.ceir`テキストの読み込みコマンドではありません。`--target`や`-o`は受け付けません。

正常時は出力をstdoutへ返します。失敗時も先行出力を保持し、stderrへ診断して終了コード1を返します。既定診断は停止理由とソース位置、`runtime-v1`は他経路と同じFailureCode・NodeId・Spanです。命令番号はありません。

従来の`run`によるVM実行を使うスクリプトや性能測定は、`run-vm`へ変更してください。言語の出力・停止記録は共通ですが、既定診断のbytecode命令番号はVM経路だけにあります。

Rust APIは`run_ir(source)`、完成済みIRには`ir_executor::run(&program)`を使います。`IrRunError`は構築失敗と実行失敗を区別し、実行エラーから`output()`・`origin()`・`runtime_failure()`を取得できます。importには`modules::load(path)?.to_ir()`を使います。[責務・制限](../design/ir-executor.ja.md)も参照してください。

## Cerune IRの出力

```text
cerune emit-ir <file> [-o <output.ceir>]
```

`cerune emit-ir`は、意味と型が解決されたバックエンド非依存のCerune IRを出力します。

## MIRの独立実行

```sh
cerune run-mir examples/ir_stages/owned_values.ceru
cerune run-mir examples/dynamic_arrays/lowering_order.ceru --array-heap-limit 47 --diagnostic-format runtime-v1
```

`run-mir`はソースからHIR→MIRへ変換・検証し、MIRの命令・ブロックを直接実行します。現行言語機能に対応し、HIR実行器・VMを内部利用しません。`run`／`run-ir`の既定の意味は変わりません。

`--string-heap-limit`／`--array-heap-limit`と`--diagnostic-format runtime-v1`を受け付けます。成功は終了コード0、失敗は1で、言語の停止では元の位置と先行出力を保持します。検証失敗・内部不整合を言語の停止と混同しません。`-o`・`--target`・SSA／pass指定は拒否します。MIRテキストのloaderや新しいbuild形式ではありません。[API・検証範囲](../design/mir-executor.ja.md)も参照してください。

## MIRの観測

```sh
cerune emit-mir examples/ir_stages/control_flow.ceru -o target/control_flow.mir.txt
```

完成済みIRを最適化なし・非SSAのMIRへ変換し、検証後に観測テキストを出力します。`-o`省略時はstdoutへ出します。ターゲットは不要で、`--target`・`--ssa`・pass指定は受け付けません。二つのheap予算は出力に記録します。

`Cerune MIR v0.1`には型付き局所値、明示的な評価順、basic block・辺、所有操作、HIR NodeId／SourceId／Spanを含めます。本文・パスは自動で含めません。ソースの文字列リテラルと識別子はプログラムの一部として表示します。専用の拡張子・loaderはなく、実行可能な配布成果物ではありません。`run`は引き続きHIR直接実行です。[変換と検証範囲](../design/ir-stages.ja.md)も参照してください。

## 出力成果物の生成

```text
cerune emit-c <file> [-o <output.c>]
cerune emit-llvm <file> [--target <triple>] [-o <output.ll>]
cerune emit-qbe <file> [--target <triple>] [-o <output.ssa>]
cerune emit-wat <file> [-o <output.wat>]
cerune emit-asm <file> [--target <triple>] [--annotate-origins] [-o <output.s>]
cerune emit-obj <file> --target <triple> [--annotate-origins] -o <output.o>
cerune emit-bytecode <file> [-o <output.cebc>]
```

各コマンドが出力する成果物は次のとおりです。

| コマンド | 出力経路 | 現在のターゲット | 成果物 |
| --- | --- | --- | --- |
| `emit-c` | C | Ceruneでは指定しない | `.c` |
| `emit-llvm` | LLVM IR | 未指定、または明示的なWindows x64 / Linux x86-64 | `.ll` |
| `emit-qbe` | QBE IR | 未指定、または明示的なWindows x64 / Linux x86-64 | `.ssa` |
| `emit-wat` | WebAssembly Text | WebAssembly | `.wat` |
| `emit-asm` | ネイティブアセンブリ | x86-64、Windows / Linux、各OSの呼出規約 | `.s` |
| `emit-obj` | 自前のネイティブオブジェクト | 明示的なWindows x64 / Linux x86-64 | `.obj` / `.o` |
| `emit-bytecode` | Cerune bytecode | Cerune VM | `.cebc` |

テキストを出力する`emit-*`コマンドは、`-o`を指定しない場合、観測結果を標準出力へ書き出します。`-o`を指定した場合は、利用者が出力先のパスを決定します。バイナリの`emit-obj`では`-o`が必須です。

`emit-asm --target x86_64-unknown-linux-gnu`でLinux、`--target x86_64-pc-windows-msvc`でWindows向けのアセンブリを生成します。省略時は互換性のためWindows固定で、ホストOSから推測しません。`--annotate-origins`で式と命令の対応を注釈とラベルとして残せます。[機械語までの観測](../design/native-code.ja.md)も参照してください。

### LLVMのターゲット指定

`--target`は`x86_64-unknown-linux-gnu`または`x86_64-pc-windows-msvc`を受け付けます。文字列・動的配列を使うプログラムでは、未使用の型・関数も含めて指定が必須です。数値だけでも、検査付き演算・変換・配列添字など実行時診断を持つ場合は指定が必要です。書込み失敗の接続を選ぶため、数値のみの`print(1);`も指定が必要です。`flush();`と`std:io`の行入力も明示targetを必須とします。ホストOSからは推測しません。`--target`と`-o`（`--output`も可）の順序は自由ですが、同じオプションの重複、値の省略、未対応ターゲットはエラーです。

Linux x86-64上での例:

```sh
cerune emit-llvm examples/string_lookup.ceru --target x86_64-unknown-linux-gnu -o target/string_lookup.ll
clang --target=x86_64-unknown-linux-gnu target/string_lookup.ll -o target/string_lookup
./target/string_lookup
```

Windows x64（MSVC CRTとリンカが利用可能な環境）での例:

```powershell
cerune emit-llvm examples/string_lookup.ceru --target x86_64-pc-windows-msvc -o target/string_lookup.ll
clang --target=x86_64-pc-windows-msvc target/string_lookup.ll -o target/string_lookup.exe
.\target\string_lookup.exe
```

CeruneはLLVMの生成だけを行い、Clangや実行ファイルを起動しません。指定は`target triple`に記録されます。下流ツールにも同じターゲットを渡します。Windowsでは数値のみの出力も含めて標準出力をバイナリモードにし、NUL・CR・LFを保持します。詳しくは[文字列の設計](../design/strings.ja.md#llvmでの表現とターゲット)を参照してください。

ライブラリでは`compile_to_llvm_with_target(source, Some(codegen::llvm::Target::X86_64UnknownLinuxGnu))`、または`X86_64PcWindowsMsvc`を使います。既存の`compile_to_llvm(source)`はターゲット未指定のAPIとして残ります。

### QBEのターゲット指定

QBEの`--target`は`x86_64-unknown-linux-gnu`または`x86_64-pc-windows-msvc`です。文字列・動的配列・標準出力（`print(1);`や`flush();`も含む）・`std:io`の行入力では必須です。省略・未対応のターゲット・オプションの重複は診断し、既存の出力ファイルを変更しません。これらを使わないプログラムでは省略できます。実行中のOSから生成先を推測しません。

```sh
cerune emit-qbe examples/string_lookup.ceru --target x86_64-unknown-linux-gnu -o target/string_lookup.ssa
qbe -t amd64_sysv -o target/string_lookup.s target/string_lookup.ssa
cc target/string_lookup.s -o target/string_lookup
./target/string_lookup
```

Windows x64では、QBE 1.3とClang、MSVCのCRT・リンカを用意します。

```powershell
cerune emit-qbe examples/function_arguments.ceru --target x86_64-pc-windows-msvc -o target/function_arguments.ssa
qbe -t amd64_win -o target/function_arguments.s target/function_arguments.ssa
clang --target=x86_64-pc-windows-msvc target/function_arguments.s -o target/function_arguments.exe
./target/function_arguments.exe
```

生成物のコメントにターゲットとQBEの`-t`を残します。QBEとリンカの起動は利用側の操作です。ライブラリでは`compile_to_qbe_with_target(source, Some(codegen::qbe::Target::X86_64UnknownLinuxGnu))`、または`X86_64PcWindowsMsvc`を使います。[WindowsのABIと検証](../design/qbe-windows.ja.md)も参照してください。

### WATと直接アセンブリの文字列

`emit-wat`はWebAssembly固定です。`emit-asm`は明示的にWindows/Linuxを選べ、省略時は従来のWindows固定です。

通常出力を持つWATは`cerune.write_stdout_v1(i64, i32) -> i32`をimportし、最大8バイトの値と受理数・失敗をやり取りします。メモリは公開しません。hostは[版付き出力契約](../design/wat-output-host.ja.md)を実装します。`emit-wat`自体はhostを起動しません。

引数操作を含むWATには[版付き引数host契約](../design/wat-arguments-host.ja.md)が必要です。`emit-wat --argument-count-limit <count> --argument-byte-limit <bytes>`で起動時の上限を生成物へ明示し、実行用の引数列はhostへ別途注入します。

float表示には`format_f32_v1`・`format_f64_v1`も必要です。全長とバイトを整形用の非公開領域へ受け取ってから、通常出力と同じ接続で書き込みます。旧hostは必要なimportがなければinstantiateに失敗します。

実行時検査を持つWATは`cerune.write_error_byte(i32) -> void`もimportします。ホストはこのASCII診断をstderrとして出力し、`unreachable`で停止しても先行stdoutを保持します。[実行時診断の契約](../design/runtime-diagnostics.ja.md)を参照してください。

Windows x64の直接アセンブリは`cerune emit-asm examples/string_lookup.ceru -o target/string_lookup.s`で生成し、`clang --target=x86_64-pc-windows-msvc target/string_lookup.s -o target/string_lookup.exe`でビルドできます。文字列の出力前に標準出力をバイナリモードへ切り替えます。

## Nativeの入力と出自

`emit-asm`／`emit-obj`はHIR→MIR→x86-64 LIRを経て生成します。`--annotate-origins`を付けると、元のNodeId・ファイル内のバイト範囲に加え、MIRのblock・命令番号からLIRへの対応を記録します。Objectでは対応シンボルも保持します。[NativeのMIR変換](../design/native-mir.ja.md)を参照してください。既存のターゲット指定と出力形式は維持します。

## VM実行

```text
cerune run-vm <file>
```

`cerune run-vm`はCerune bytecodeへloweringし、生成された`BytecodeProgram`をCerune VMで実行します。

実行結果は検証や実験に利用できますが、[コンパイラ設計](../design/architecture.ja.md)で定めるHIR・MIR・生成物の観測とは区別します。

実行時エラーがソースコードに由来するbytecode命令で発生した場合、診断にはソース位置とbytecode命令番号の両方を表示します。

```text
cerune: cannot divide an integer by zero at 1:7 (bytecode instruction 0002)
```

対応するソース位置がない場合も、bytecode命令番号は表示します。簡潔な診断には、ソース本文や入力ファイルのパスを含めません。

`run-vm --diagnostic-format runtime-v1`では、言語の検査失敗を停止理由・NodeId・UTF-8バイト範囲を持つ1行の記録で出力します。コンパイル診断やVM内部エラーは従来の形式です。停止前に実行した`print`はstdoutに残します。C・LLVM・QBE・WAT・Windows/LinuxのASMと自前オブジェクトも同じ停止記録を使います。[共通診断の契約](../design/runtime-diagnostics.ja.md)と[意図した停止の例](../../examples/runtime_failures/README.md)を参照してください。

## バージョン表示

```text
cerune --version
```

`cerune --version`はCeruneのバージョンを表示します。

## Ceruneが扱わない外部設定

Ceruneは、次のような外部実験の方針を決定しません。

- GCCとClangのどちらを使用するか
- 外部コンパイラの最適化レベル
- 外部ツールチェーンの対象CPU
- ベンチマーク設定
- 測定方法
- 比較方法

これらはCeruneを呼び出す側が決定し、必要に応じて記録します。

## LLVMの出自を辿る

```sh
cargo run -- emit-ir examples/string_origins.ceru
cargo run -- emit-llvm examples/string_origins.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o string-origins.ll
cargo run -- run examples/string_origins.ceru
```

Windows向けには`--target x86_64-pc-windows-msvc`を指定します。`--annotate-origins`はLLVMだけの任意指定です。通常の出力は従来どおりです。APIでは`compile_to_llvm_with_options(source, llvm::Options { target, annotate_origins: true })`を使います。コメントの意味と対応範囲は[可観測性の契約](../design/observability.ja.md#llvmの出自注釈)を参照してください。

## WATのu64出力

`u64`を表示する成果物は`cerune.print_u64(i64) -> void`をimportします。ホストは符号なし64ビットの十進数とLFを出力します。JavaScriptでは`BigInt.asUintN(64, value).toString()`を使います。[u64の設計](../design/u64.ja.md)を参照してください。

## 自前オブジェクトの生成

`emit-obj`は`--target x86_64-pc-windows-msvc`または`x86_64-unknown-linux-gnu`と`-o`を必須とします。外部ツールなしでCOFF/ELFを生成し、標準出力へバイナリは書きません。`--annotate-origins`で出自ラベルを残せます。誤ったオプションやコンパイル診断では既存出力を変更しません。リンク・実行は別の明示操作です。[自前エンコーダ](../design/native-encoder.ja.md)を参照してください。ライブラリでは`compile_to_native_object(source, target, annotate_origins)`が`Vec<u8>`を返します。

## WATの明示flush

`flush();`を含む成果物は`cerune.flush_stdout_v1() -> i32`をimportします。hostは成功で0、失敗で0以外を返します。失敗すると`write_error_byte`へ`io-v1`記録を送りtrapします。importがないhostはinstantiateに失敗します。memoryは公開しません。
