# プログラム引数

[English](README.en.md)

[list.ceru](list.ceru)は引数の件数、各値、UTF-8バイト数を表示し、最後に欠落を確認します。空文字を見分けられるよう値を配列で表示します。

```sh
cargo run -- run examples/arguments/list.ceru -- hello --ssa
cargo run -- run-mir examples/arguments/list.ceru --ssa -- hello --ssa
cargo run -- run-vm examples/arguments/list.ceru -- hello --ssa
```

いずれも次を出力します。最初の `--` 以後の `--ssa` はプログラムの値です。

```text
2
["hello"]
5
["--ssa"]
5
end
```

引数なしでは `0` と `end`、空文字を1個渡すと `1`・`[""]`・`0`・`end` です。空文字の渡し方は利用するshellの規則に従います。

HIR・MIR・SSA・VM・CLI・生成C・LLVM・QBE・WAT・Nativeで実行できます。整数の読取りは未実装です。[契約・上限・未対応範囲](../../docs/design/dogfooding-inputs.ja.md)を参照してください。

生成Cでも同じ出力を確認できます。Linux/macOSでは次のように実行します。

```sh
cargo run -- emit-c examples/arguments/list.ceru -o target/arguments.c
clang -std=c11 target/arguments.c -o target/arguments
./target/arguments hello --ssa
```

WindowsのMSVC対象Clangでは出力先を`target/arguments.exe`とし、`./target/arguments.exe hello --ssa`で実行します。MinGW対象ならコンパイル時に`-municode`を追加します。生成した実行ファイルにはCerune CLI用の区切り`--`は不要です。

LLVMでも実行できます。以下はLinux向けです。

```sh
cargo run -- emit-llvm examples/arguments/list.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/arguments.ll
clang target/arguments.ll -o target/arguments-llvm
./target/arguments-llvm hello --ssa
```

Windowsではターゲットを`x86_64-pc-windows-msvc`、出力先を`target/arguments-llvm.exe`に変更します。上限を変える場合は`emit-llvm`に`--argument-count-limit 2 --argument-byte-limit 10`等を指定します。表示される結果は上記と同じです。

QBE 1.3でも同じ出力を確認できます。Linux向けの生成・リンク手順です。

```sh
cargo run -- emit-qbe examples/arguments/list.ceru --target x86_64-unknown-linux-gnu -o target/arguments.ssa
qbe -t amd64_sysv -o target/arguments.s target/arguments.ssa
clang -target x86_64-unknown-linux-gnu target/arguments.s -o target/arguments
./target/arguments hello --ssa
```

WindowsではCeruneとClangのtargetを`x86_64-pc-windows-msvc`、QBEの`-t`を`amd64_win`、実行ファイルを`target/arguments.exe`にします。引数取得の上限は`emit-qbe --argument-count-limit 2 --argument-byte-limit 10`のように明示します。

WATはWABT 1.0.39・Node.js 20と[テストhost契約](../../docs/design/wat-arguments-host.ja.md)で同じ引数を明示注入します。

```sh
cargo run -- emit-wat examples/arguments/list.ceru -o target/arguments.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm target/arguments.wat -o target/arguments.wasm
CERUNE_TEST_WAT_ARGUMENTS='{"values":["hello","--ssa"]}' node tests/support/run_wasm_input.cjs target/arguments.wasm
```

PowerShellでは最後の行を次に置き換えます。

```powershell
$env:CERUNE_TEST_WAT_ARGUMENTS = '{"values":["hello","--ssa"]}'
node tests/support/run_wasm_input.cjs target/arguments.wasm
```

Nativeでも同じ出力になります。Linux向けに、自前Objectを生成してリンクします。

```sh
cargo run -- emit-obj examples/arguments/list.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/arguments-native.o
clang target/arguments-native.o -o target/arguments-native
./target/arguments-native hello --ssa
```

Windowsではtargetを`x86_64-pc-windows-msvc`、Objectを`.obj`、実行ファイルを`.exe`にします。`emit-asm`ならAssembly、`emit-lir`なら命令・上限・出自を確認できます。`--ssa`でSSA経由にしても同じ結果です。上限は`--argument-count-limit 2 --argument-byte-limit 10`のように生成時に指定します。
