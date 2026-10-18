# ブラウザーからCerune関数を呼ぶ

[English](README.en.md)

[.ceruの例](../../../examples/host_calls/README.md)を[製品WAT API](../../../docs/design/wat-js-bindings.ja.md)で生成し、同じインスタンスの関数を繰り返し呼びます。HTMLとhost補助コードはこのフォルダに分け、Ceruneの実行例一覧には混ぜません。

## 起動

Rust 1.98.0とNode 20.20.2を使い、リポジトリ直下で各コマンドの成功を確認して進めます。ツールと生成物はtarget以下に置きます。

```sh
npm install --prefix target/host-call-tools --no-audit --no-fund wabt@1.0.39 typescript@5.9.3 playwright@1.64.1
cargo run --locked --example wat_library -- examples/host_calls/library.ceru target/wat-library step sum_to checked
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/wat-library/library.wat -o target/wat-library/library.wasm
node experiments/host_calls/browser/serve.cjs target/wat-library
```

表示されたURLをブラウザーで開きます。サーバーは127.0.0.1だけで待ち受け、Ctrl+Cで終了します。英語版は画面上部から切り替えます。file URLで直接開く方式ではありません。

## 試せること

| 操作 | 結果 |
| --- | --- |
| 位置を1段進める | step(位置, 2, 0.5)。10→11→12と同じ関数を再利用 |
| 1から上限まで合計 | sum_to(4)は10。小数などi32でない入力は実行前に拒否 |
| 文字列を渡す | checked("1.5")を型検査で拒否。その後も利用可能 |
| 小数を厳密変換 | checked(1.5)は停止。runtime-v1と依存先のソース位置を表示 |
| 停止後の呼出し | closed。新しいインスタンスを作れば再開可能 |
| 公開一覧を見る | 読み取りだけ。プログラムの実行・状態変更なし |

一般の同期関数呼出しなので、非常に大きな反復回数はブラウザーを待たせます。実行時間の上限は追加していません。

生成過程のリンクから元IR・bytecode・通常WAT・公開用WAT・JS・型宣言・公開一覧を別々に確認できます。sources.jsonはソース本文も含む、明示的に生成した観測ファイルです。このサーバーで配信されます。

## 自動検証

```sh
node experiments/host_calls/browser/check-types.cjs target/wat-library target/host-call-tools/node_modules/typescript/bin/tsc
node target/host-call-tools/node_modules/playwright/cli.js install chromium
node experiments/host_calls/browser/test.cjs target/wat-library target/host-call-tools/node_modules/playwright
```

Linuxで必要なシステムライブラリも入れるCIでは `install --with-deps chromium` を使います。Windowsで既存のEdgeを使う場合はブラウザーtestコマンド末尾に `msedge` を付け、Chromiumのダウンロードを省略できます。

型検査は生成したlibrary.d.mtsの名前・引数数・値型・読み取り専用性を確認します。ブラウザーテストは日英両画面の反復呼出し、観測、入力拒否、停止理由、停止後の拒否、新規作成を検査し、browser-ja.pngとbrowser-en.pngを生成物フォルダへ保存します。ローカルのEdgeで確認済みです。CIはChromiumで実行します。
