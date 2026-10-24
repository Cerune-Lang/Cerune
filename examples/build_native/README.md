# Native実行ファイルを作る

[English](README.en.md)

[main.ceru](main.ceru)は動的u8配列をコピーし、整数引数を読み取って2倍を表示します。build自体は実行しません。リポジトリのルートから試します。

```powershell
cargo run --quiet -- build native examples/build_native/main.ceru --target x86_64-pc-windows-msvc --linker "C:\Program Files\LLVM\bin\clang.exe" -o target/native-demo
.\target\native-demo\program.exe 3
```

```sh
cargo run --quiet -- build native examples/build_native/main.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o target/native-demo
./target/native-demo/program 3
```

両ターゲットの実行結果：

```text
[9, 4]
[2, 4]
6
```

Clangのパスは実際の配置に合わせ、targetのリンカ・SDK・runtimeを用意します。毎回新しい出力先を使い、既存先は拒否します。引数なしでは整数引数を促し、`abc`では`invalid integer`を表示します。

| 比較・観測 | コマンド・ファイル |
| --- | --- |
| IR | `cargo run --quiet -- run examples/build_native/main.ceru -- 3` |
| VM | `cargo run --quiet -- run-vm examples/build_native/main.ceru -- 3` |
| 基準HIR/MIR | `program.ceir`・`program.mir.txt` |
| Native生成 | `program.lir.txt`・`program.machine.txt`・`program.origins.s`・`program.o` |
| リンク実行 | `link-plan.json`・`link.stderr.log`・`program.link.map`・`manifest.json` |

`--ssa`で基準MIRを残してSSA経由を比較でき、`--include-sources`で入力本文も保存できます。上限・失敗・出自は[build契約](../../docs/design/native-build.ja.md)を参照してください。実行ファイルはtargetのシステムruntimeを使用し、releaseによる配布構成は次の別変更です。
