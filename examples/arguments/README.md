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

HIR・MIR・SSA・VM・CLI・生成Cで実行できます。LLVM・QBE・WAT・Nativeは未接続のため、引数操作を含む生成を明示的に拒否します。整数の読取りは未実装です。[契約・上限・未対応範囲](../../docs/design/dogfooding-inputs.ja.md)を参照してください。

生成Cでも同じ出力を確認できます。Linux/macOSでは次のように実行します。

```sh
cargo run -- emit-c examples/arguments/list.ceru -o target/arguments.c
clang -std=c11 target/arguments.c -o target/arguments
./target/arguments hello --ssa
```

WindowsのMSVC対象Clangでは出力先を`target/arguments.exe`とし、`./target/arguments.exe hello --ssa`で実行します。MinGW対象ならコンパイル時に`-municode`を追加します。生成した実行ファイルにはCerune CLI用の区切り`--`は不要です。
