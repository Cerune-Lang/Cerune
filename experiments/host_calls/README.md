# VM埋め込みとWAT関数公開の比較

[English](README.en.md)

[#161の設計](../../docs/design/typed-host-calls.ja.md)に使う固定関数の実験です。**固定関数の比較実験そのものは製品の公開APIではありません。** 末尾には、別途実装した製品Rust APIの利用例も置いています。 Ceruneのプログラムは[examples/host_calls](../../examples/host_calls/README.md)、補助コードはこちらに分けています。

## 再現

Rust 1.98.0とwasm32-unknown-unknown target、Node 20.20.2、wabt 1.0.39、TypeScript 5.9.3を使用します。リポジトリ直下で、各コマンドの成功を確認して順に実行します。依存ツールと生成物はtarget以下へ置きます。

```sh
rustup target add wasm32-unknown-unknown
npm install --prefix target/host-call-tools --no-audit --no-fund wabt@1.0.39 typescript@5.9.3
cargo build --manifest-path experiments/host_calls/vm/Cargo.toml --locked --target wasm32-unknown-unknown --release --target-dir target/host-calls-vm
cargo run --locked --example host_calls -- target/host-calls
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/host-calls/exports.wat -o target/host-calls/exports.wasm
node experiments/host_calls/compare.cjs target/host-calls target/host-calls-vm/wasm32-unknown-unknown/release/cerune_host_call_probe.wasm
node target/host-call-tools/node_modules/typescript/bin/tsc --strict --noEmit --target ES2022 --module NodeNext experiments/host_calls/types.test.mts
```

WindowsとLinux/WSLのNodeで比較済みです。CIの「Typed host call comparison」はLinuxで両Wasmをビルドして実行し、観測ファイルをartifactへ保存します。固定VM比較のブラウザー検証は行っていません。製品WAT APIの[ブラウザー例](browser/README.md)は別途検証します。

## コードと生成物

| 場所 | 役割 |
| --- | --- |
| [main.rs](main.rs) | 元IR・bytecode・通常WAT・wrapper付きWATを保存。通常のdemoをIR/VMで実行し、既知の期待値を検査 |
| [vm](vm/Cargo.toml) | 同じ固定ソースを一度コンパイルし、既存のVM埋め込みAPIを呼ぶWasm。独自の実行器は作らない |
| [host.mjs](host.mjs) | 引数検査、型付きの結果、停止時のハンドル閉鎖。生のinstance/memoryを返さない |
| [host.d.mts](host.d.mts) / [types.test.mts](types.test.mts) | 手書きの実験用TS宣言と、型・引数数・経路別関数の検査 |
| [compare.cjs](compare.cjs) | 同じNode内で両経路を反復呼出し。結果、異常引数、停止記録、再生成したinstance、raw Wasmの変換を検査 |
| target/host-calls | source.ceir、source.cebc、plain.wat、exports.wat/wasm、failure.txt、demo.stdout、measurements.json |

サイズ、Wasmのcompile、instantiate、VMのソース準備、1,000回の準備呼出し後の10,000回の呼出しを分けます。値の合計も検査し、時間の大小を合否にしません。オフラインのWAT生成・assembly時間は呼出し測定に含めません。

## 読み方と限界

`create(bytes, 'vm' | 'wat')`が返す `api.step(10, 2, 0.5)`は11を返します。コンパイル済みコードを保持し、呼出しごとのソースコンパイルは行いません。

- 共通f64関数は既知の期待値と比較し、`checked(1.5)`のruntime-v1・出自も一致させます。正常なNaNを失敗と取り違えません。
- 固定関数の公開wrapperは内部frame引数を隠し、従来の引数数・型を維持します。この実験の関数がframe領域を必要としないことも生成時に検査し、必要になった場合は停止します。
- WAT側ではi32 wrapperとu64の範囲検査も試します。f32は生のWasm exportで丸めを確認するだけで、共通APIの対応型ではありません。
- JSの実行時検査とTSの静的検査は別です。raw Wasmが受け付ける暗黙変換を、そのまま公開契約にしません。
- 呼出し時の失敗はarguments/language/trap/host/closedを区別します。compile/instantiate失敗の共通エラー化は未実装です。
- VMのRust Wasmにはmemory exportがありますが、実験アダプターからは隠します。生成WATのmemoryはexportしません。悪意のあるhostからの隔離を証明する実験ではありません。
- 固定の内部名検索・WATへの文字列追記・手書きTS宣言は実験専用です。一般の公開対象選択、pub情報の保存、ライブラリ検査、生成JS/TS、HTML例は末尾の製品APIで実装しています。
- 言語上の停止後にJSハンドルを閉じる方針は、既存のRust VM APIの独立した再呼出しを変更しません。

計測結果と最初にWAT公開を進める理由は[設計書](../../docs/design/typed-host-calls.ja.md)に記録します。JS/TS補助コードはGitHub Languagesの集計対象から外し、diffは隠しません。

## 製品のWATライブラリAPIを使う

[library.rs](library.rs)は[製品のRust API](../../docs/design/wat-library.ja.md)を呼びます。上記の固定wrapper実験とは別の生成例です。専用のCerune CLIコマンドではありません。

```sh
cargo run --quiet --example wat_library -- examples/host_calls/library.ceru target/wat-library step sum_to checked
node target/host-call-tools/node_modules/wabt/bin/wat2wasm target/wat-library/library.wat -o target/wat-library/library.wasm
cargo test --test wat_output library_exports
```

library.watとexports.jsonに、選択した公開名・型変換・出自を保存します。source.ceir、source.cebc、plain.wat、sources.jsonも保存し、通常の表現と比較できます。sources.jsonは本文を含む明示的な保存です。検査に失敗した場合はファイルを作り始めません。

library.mjsとlibrary.d.mtsも生成し、引数検査・停止後のハンドル管理・型付き呼出しに使えます。[ブラウザー例と実行手順](browser/README.md)で反復呼出しと生成過程を確認できます。
