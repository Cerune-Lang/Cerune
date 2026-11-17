# 子プロセスの出力と失敗を受け取る

[English](README.en.md)

`std:process`は型、`std:process::run`は実行操作です。失敗も`Outcome`として受け取り、呼出し元が後続処理を続けます。

| サンプル | 動作 | 接続 |
| --- | --- | --- |
| [unconfigured.ceru](unconfigured.ceru) | 未接続を受け取り`continued`まで実行 | 設定不要。[期待出力](unconfigured.stdout) |
| [registered.ceru](registered.ceru) | 子へ引数と生バイトを渡し、失敗・終了状態・stdout/stderr・入力受理数を表示 | [明示したJSON設定](../../docs/design/subprocess-cli.ja.md)で`tool`を登録。[検証用の期待出力](registered.stdout) |

## 未接続の例

```sh
cargo run --quiet -- run examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru
cargo run --quiet -- run-mir examples/process_execution/unconfigured.ceru --ssa
cargo run --quiet -- run-vm examples/process_execution/unconfigured.ceru
```

## 実際の子を起動する例

```sh
cerune run examples/process_execution/registered.ceru --process-config process.json
cerune run-mir examples/process_execution/registered.ceru --ssa --process-config process.json
cerune run-vm examples/process_execution/registered.ceru --process-config process.json
```

`registered.ceru`を設定なしで実行した場合も`Unconfigured`を表示して継続します。`registered.stdout`は検証用の子を接続した場合の出力です。

[CLIテスト](../../tests/process_cli/mod.rs)は[検証用の子](../../tests/support/process_child.rs)を構築し、絶対パス・作業場所・明示した環境変数を設定してこのexampleを実行します。子は空引数・日本語・引用符等を検査し、入力の`[255, 0, 13, 10]`をstdout、`err\0\r\n`をstderrへ返し、終了コード7で終了します。Ceruneは`continued`まで進み、正常終了します。子の検証用ファイルは実行example一覧から分離しています。

Windowsでは次のコマンドで4実行器と`run-ir`別名を比較できます。

```sh
cargo test --test modules process_cli
```

Linuxでは監督を明示して同じテストを実行します（標準のCargo出力先の場合）。

```sh
cargo build --example subprocess_linux_supervisor
CERUNE_TEST_PROCESS_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_supervisor" cargo test --test modules process_cli -- --include-ignored
```

生成CもWindows/Linuxで実行できます。targetを指定して生成し、同じtargetのruntime archiveを明示してリンクします。

~~~sh
cerune emit-c examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.c
~~~

[構築・起動手順](../../docs/design/subprocess-c.ja.md)と[4実行器との比較試験](../../experiments/subprocess/c_abi/generated.cjs)を用意しています。[LLVMでの構築・起動](../../docs/design/subprocess-llvm.ja.md)にも対応しました。[QBEでの構築・起動](../../docs/design/subprocess-qbe.ja.md)にも対応しました。[Nativeでの構築・起動](../../docs/design/subprocess-native.ja.md)にも対応しました。[WAT参照hostでの起動](../../experiments/wat_host/README.md)にも対応しました。[実行設計](../../docs/design/subprocess-execution.ja.md)に対応状況と出自の保持をまとめています。

LLVMへ生成する場合も、同じexampleとprocess.jsonを使います。

~~~sh
cerune emit-llvm examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.ll
~~~

QBEでも同じexampleを使います。明示targetとQBEのtargetを揃え、生成ASMを同じruntime archiveへリンクします。

~~~sh
cerune emit-qbe examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o process.ssa
qbe -t amd64_sysv -o process.s process.ssa
~~~

## Nativeの生成前の表現を確認する

同じ実行例から、要求・結果の保存位置、型のフィールド位置、元のソース位置をLIRで確認できます。

~~~sh
cerune emit-lir examples/process_execution/unconfigured.ceru --target x86_64-pc-windows-msvc
cerune emit-lir examples/process_execution/registered.ceru --ssa --target x86_64-unknown-linux-gnu
~~~

LIRの表示自体は子を起動しません。[NativeのASM・自前Object](../../docs/design/subprocess-native.ja.md)は、共通runtimeを明示してリンクするとこの例を実行できます。`build native --process-runtime <archive>`とreleaseの依存記録検証にも対応しました。[同じexampleのコマンド](../../docs/design/subprocess-native.ja.md#buildreleaseで実行ファイルを作る)を載せています。接続JSONとLinux監督は実行時に別途指定します。
