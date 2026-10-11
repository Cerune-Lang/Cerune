# 実行時停止の共通記録

[English](runtime-diagnostics.en.md)

## 目的と今回の対応範囲

失敗を「非ゼロで終了した」とだけ記録すると、言語が意図した停止と、アクセス違反や起動失敗を区別できません。Ceruneの可観測性として、停止理由と失敗したソース範囲を出力経路間で照合します。内部状態の変更や実行への介入は公開しません。

| 経路 | 現在の実行時診断 |
| --- | --- |
| IR Executor | `run`／`run-ir`でソース位置付き診断、明示的な`runtime-v1`形式。CLI終了コード1 |
| VM | `run-vm`で既定の人向け診断と、明示的な`runtime-v1`形式 |
| Windows/Linux直接ASM | `runtime-v1`をstderrへ出し、Windowsはfast-fail、Linuxは不正命令で停止 |
| 自前エンコーダのCOFF/ELF | ASMと同じlowering・診断処理を符号化 |
| C | `runtime-v1`をC標準のstderrへ出し、`abort`で停止 |
| LLVM | 明示したWindows/Linuxの出力ABIで`runtime-v1`を出し、trapで停止 |
| QBE | 明示したWindows/Linuxの出力ABIで`runtime-v1`を出し、`abort`で停止 |
| WAT | `cerune.write_error_byte`へ`runtime-v1`の各バイトを渡し、`unreachable`で停止 |

IR Executor・VM・C・LLVM・QBE・WAT・直接ASM・自前オブジェクトで、言語の検査失敗の理由とソース位置を照合できます。OSやホストの終了コード自体を統一せず、各経路の意図した停止方法とレコードの両方を検証します。

## レコード

```text
cerune: runtime-v1 code=division-by-zero node=1 bytes=6..11
```

これは`print(1 / 0);`の除算の例です。形式はASCIIの1行で、次を保持します。

- `code`: 下表の停止理由。
- `node`: 対応するCerune IRのNodeId。呼び出し先で失敗した場合は、呼び出し元の式ではなく失敗した式を指す。
- `file`（任意）: `SourceMap`に登録したファイルの正の識別子。`node`と`bytes`の間に置く。省略時は従来の匿名ソース0。
- `bytes`: 元ソースのUTF-8バイト範囲。0起点、終端を含まない。行番号や表示上の文字数ではない。

[ファイル識別のAPI](source-files.ja.md)を使うと、例えば`cerune: runtime-v1 code=division-by-zero node=1 file=2 bytes=6..11`となります。`bytes`は識別されたファイル内の範囲です。既存の単一ソースAPIとimport/pubのないCLI入力のレコードは変わりません。受信側はこの任意フィールドに対応する必要があり、古い厳密なreaderは登録済みファイルのレコードを受理できません。付属の観測スクリプトとWAT検証ホストは両形式を検査します。`file=0`、先頭ゼロ、重複や不正な数値は拒否します。[モジュール](modules.ja.md)を使うCLI入力では入口を含めてfileを記録し、`emit-sources`の一覧と本文で照合します。

パス、ソース本文、実行時の値は含めません。元ソースとコンパイラ版を合わせて解釈します。NodeIdは編集や版の変更をまたぐ永続IDではありません。ネイティブ観測manifestは元ソースのSHA-256と使用ツールも記録します。

| code | 意味 |
| --- | --- |
| `integer-overflow` | 加算・減算・乗算・符号反転・左シフトの結果が型の範囲外 |
| `division-by-zero` | 整数のゼロ除算 |
| `division-overflow` | 整数の除算結果が型の範囲外 |
| `remainder-by-zero` | 整数の剰余演算の除数がゼロ |
| `invalid-shift-count` | シフト量が0未満または型のビット数以上 |
| `integer-conversion-out-of-range` | 整数型間の明示変換で範囲外 |
| `conversion-out-of-range` | 浮動小数点を含む変換で範囲外 |
| `conversion-inexact` | 変換で値が変わる |
| `conversion-not-finite` | NaNまたは無限大を整数へ変換できない |
| `conversion-nan` | floatの型変更でNaNを保持できない |
| `conversion-negative-zero` | 整数への変換で負のゼロの符号を保持できない |
| `array-index-out-of-bounds` | 配列の添字が範囲外 |
| `array-range-out-of-bounds` | 配列の範囲コピーで境界が不正 |
| `array-length-out-of-range` | `array_repeat`の個数が負 |
| `allocation-size-overflow` | 文字列・配列の長さや配置サイズを表現できない |
| `allocation-limit-exceeded` | 生存する文字列内容・配列要素領域がそれぞれの明示予算を超える |
| `allocation-failed` | 検査後の確保が失敗を返す |

配列代入ではNodeIdは代入文、バイト範囲は失敗した添字の`[...]`を指します。入れ子の添字は左から検査し、失敗した場合は右辺を評価しません。整数からfloatへの変換で丸めにより範囲外へ出ても、元の整数値を保存できない理由は`conversion-inexact`です。

この形式はコンパイル診断、壊れたbytecodeの内部エラー、OSの資源不足、出力のI/O失敗を言語の検査失敗として分類しません。VMではそれらに既存の診断を使い、生成物に予期しないクラッシュが起きた場合も共通レコードを捏造しません。

## 使い方と出力の保持

```sh
cerune run examples/runtime_failures/function_division.ceru --diagnostic-format runtime-v1
```

`run`／`run-ir`は停止理由とソース位置、`run-vm`は従来のソース位置・bytecode位置付き診断を使います。`--diagnostic-format runtime-v1`で言語の検査失敗を共通形式にします。VMは終了コード1です。`ExecutionError::runtime_failure()`で構造化された理由と範囲を取得できます。

失敗より前に実行した`print`は取り消しません。収集型のVM APIは`VmError::output()`に出力を保持します。逐次出力APIでは接続先へ既に渡しているため`output()`は空です。CLIは逐次出力し、接続が失敗していなければ最後にflushします。終了時に再表示しません。[出力接続](standard-io.ja.md)の失敗は言語の検査失敗とは区別します。直接ASM・自前objectでは、失敗時に既存のstdoutバッファをflushしてからstderrへレコードを書きます。Linuxは`ud2`、Windowsは`FAST_FAIL_FATAL_APP_EXIT`（rcx=7、`int 0x29`、終了状態`0xc0000409`）で停止します。stdoutとstderrを混ぜた表示順までは保証しません。Windowsで文字列を含むプログラムのstdoutは従来通りバイナリ出力です。

書き込みは明示したターゲットのLinux `write` / Windows `_write`を使います。stderrの改行は比較時にCRLF/LFを揃えます。標準エラーが閉じられている、書き込みが途中で失敗する、等の場合にもターゲットに応じた停止を試みますが、完全な診断の配送までは保証しません。観測ツールは欠けたレコードを合格にしません。

Windowsの自前ASM/COFFは現時点で巻き戻し情報を出していません。深い生成スタックからの`ud2`では、元の不正命令例外の後にOS側で別の失敗へ変わる例を確認しました。言語の検査失敗は、例外ハンドラーを呼ばず直接停止する[fast-fail](https://learn.microsoft.com/en-us/cpp/intrinsics/fastfail?view=msvc-170)へ明示的に出力します（Windows 8以降）。コード7はOS側の停止理由で、Ceruneの具体的な理由・位置は`runtime-v1`が持ちます。終了コードだけで合格にせず記録と先行出力を照合します。デバッガでのスタック復元に必要な[巻き戻し情報](https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64?view=msvc-170)の生成は別途必要です。

## 実装上の判断

Cでは検査関数へ不変の位置文字列を渡し、失敗時にC標準の`stderr`へコードと位置を出します。QBEとLLVMでも、読み取り専用のデータと検査関数の引数で位置を保持します。正常経路でグローバルな位置を書き換えたり、呼び出し元の位置で関数内の失敗を上書きしたりしません。

LLVMでは文字列に加え、失敗し得る演算・変換・添字検査を持つプログラムにも`--target`が必要です。たとえば`print(1 + 2);`も検査付き加算を生成します。未指定時は診断し、Windows/Linuxをホストから推測しません。QBEも明示ターゲットで診断ABIを選びます。文字列ではターゲットが必須で、数値だけの未指定呼び出しは従来のLinux/SysV前提を保ちます。Windowsは`_write`で記録後、CRTの追加メッセージとクラッシュ収集を無効にして`abort`で終了コード3を返します。詳細は[QBE Windows対応](qbe-windows.ja.md)を参照してください。

WATは失敗箇所ごとの静的な検査関数から`cerune.write_error_byte(i32) -> void`へASCIIバイトを渡します。このimportは診断を持つモジュールに必要です。ホストはこれをstderrとして保持・出力し、trapが起きても先行したstdoutを捨てません。メモリや可変の診断状態は公開しません。ホストが出力を捨てる、importから例外を投げる、またはOSのI/Oが失敗する場合の完全な配送は保証しません。

ソース由来の停止箇所ごとに、読み取り専用のレコードを生成します。生成物は大きくなりますが、失敗分岐だけが診断処理を呼び、正常経路に「現在位置」を書き換える可変状態を持ち込みません。レコードのための動的確保や、外部から書き換える診断変数はありません。数値変換で複数の失敗理由を区別するために必要な検査は、演算の意味を保つ検査として常に生成します。

`--annotate-origins`は引き続きコメント・シンボルによる観測だけを切り替えます。実行時診断を無効にはしません。注釈の有無で命令・データの内容と実行結果が一致することをテストします。検査のABIと書き込み処理は実行中のOSから推測せず、生成時のターゲットで決めます。

## 検証

`cargo test --test runtime_routes`は以下の外部ツールを使います。設定したツールが利用できない場合は失敗とします。このテストのQBE比較はLinux x86-64で行います。Windowsは`cargo test --test qbe_windows -- --include-ignored`で検証し、数値だけの場合もstdout/stderrの全バイトを一致させます。`runtime_routes`の既存C・LLVM経路では、Windowsの数値専用stdoutのCRLFを比較時にLFへ揃え、文字列を含む出力はNUL・CR/LFを含めて完全一致を要求します。

| 環境変数 | ツール |
| --- | --- |
| `CERUNE_TEST_CC` | CコンパイラとQBE生成ASMのリンカ（例: `clang`） |
| `CERUNE_TEST_LLVM_CLANG` | 生成LLVMを処理するClang |
| `CERUNE_TEST_QBE` | QBE 1.3の実行ファイル |
| `CERUNE_TEST_NODE` | Node.js |
| `CERUNE_TEST_WAT2WASM_JS` | WABTの`bin/wat2wasm`スクリプトへのパス |

CIは利用するツールをすべて指定します。ローカルで未設定のツールが見つからない場合やWATの変換ツールが未設定の場合は、その比較をスキップと明記します。未実行の経路を成功した実行として数えません。

[意図した停止の4例](../../examples/runtime_failures/README.md)で、桁あふれ、入れ子の配列代入、関数内のゼロ除算、同じ関数の正常・短絡・失敗の順序を辿れます。正常終了のサンプルと混ぜず、期待する停止理由を表で示しています。

`observe-native.cjs --run --expect-trap`はVMを共通形式で実行し、理由・NodeId・バイト範囲・停止前の出力をネイティブと比較します。さらにSIGILL / Windows fast-fail終了を確認します。診断だけ一致する通常終了、レコードなしのクラッシュ、追加エラー、タイムアウトは合格にしません。manifestの`runtimeFailure`には照合できたレコードを保持します。

49種類の失敗を既知の理由と比較し、Windows/Linux双方のASMと自前オブジェクトをVMと照合します。Unicode・CRLF、短絡評価、関数の呼び出し、既定フィールドの式、変換の境界値、出力の保持も含みます。通常の50サンプルと全既存テストも検証対象です。

C・LLVM・QBE・WATはこの49例に実行時停止のexampleと評価順のケースを追加して比較します。C・LLVMは`-O0`と`-O2`、QBEは生成ASMのリンクを両フラグで検証します。

この失敗テストはリンクを30秒、生成プログラムを10秒に制限し、時間切れは成功ではなくケース名と取得済み出力を伴う失敗にします。stdout/stderrは一時ファイルに取得し、子プロセスの終了とパイプのEOFを混同しません。`CERUNE_TEST_TRACE=1`でケースごとの開始・終了を記録します。Linux CIのテスト全体にも8分の上限（終了猶予10秒）、ジョブには15分の上限を設定し、テストログをartifactへ保存します。

Linux CIでは使い捨てrunnerの`kernel.core_pattern`をファイル型に変更し、テストのシェルで`ulimit -c 0`を設定します。pipe型のクラッシュ収集はcoreサイズ制限だけでは止まらないためです。これはCIの観測環境の設定であり、生成プログラムの`ud2`や診断の検証は変えません。coreファイルはこのテストの判定に使いません。トレースには照合成功と一時ディレクトリの後片付けも記録し、8分の制限はログ取得の`tee`を含むパイプライン全体に適用します。
