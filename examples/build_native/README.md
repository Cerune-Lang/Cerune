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

`--ssa`で基準MIRを残してSSA経由を比較でき、`--include-sources`で入力本文も保存できます。上限・失敗・出自は[build契約](../../docs/design/native-build.ja.md)を参照してください。実行ファイルはtargetのシステムruntimeを使用します。

## buildしたものを配布する

成功した上記のbuildから配布ディレクトリを作ります。再リンク・最適化・実行は行いません。

```powershell
cargo run --quiet -- release native target/native-demo -o target/native-dist
.\target\native-dist\build\program.exe 3
Get-FileHash target/native-dist/build/program.exe -Algorithm SHA256
```

```sh
cargo run --quiet -- release native target/native-demo -o target/native-dist
./target/native-dist/build/program 3
(cd target/native-dist && sha256sum -c SHA256SUMS)
```

実行結果は上記と同じです。`native-dist`全体を別の場所へコピーして実行できます。Linuxの実行権限を保持してください。配布先にCerune・Clangは不要ですが、対象OSと`runtime-requirements.json`に記録されたruntimeが必要です。

| 確認内容 | 配布物の場所 |
| --- | --- |
| 起動パス・target・全ファイルのSHA-256 | `release.json` |
| ELF loader・共有ライブラリ・symbol版、またはWindows import DLL | `runtime-requirements.json` |
| 元の生成過程・出自・リンク記録 | `build/`（元のバイト列のまま） |
| 配布先での実行案内 | `README.md`・`README.en.md` |

既存の配布先は上書きしません。本文の同梱はbuild時の選択を引き継ぎます。配布先での実行可能性や推移的依存物の自動検査は初版に含みません。[配布契約](../../docs/design/native-release.ja.md)も参照してください。