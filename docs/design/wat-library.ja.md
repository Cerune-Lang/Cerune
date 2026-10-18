# WATライブラリの明示的な関数公開

[English](wat-library.en.md)

[#161](https://github.com/Cerune-Lang/Cerune/issues/161)の[比較設計](typed-host-calls.ja.md)に基づく、Rust APIの初版です。公開関数の選択・検査・WAT生成を実装しました。[検査付きJS・TS宣言の生成](wat-js-bindings.ja.md)と[HTML例](../../experiments/host_calls/browser/README.md)も利用できます。専用CLIは未実装です。

## APIと生成過程

```rust
use cerune_lang::{modules, codegen::wat};

let compilation = modules::from_source(
    "pub fn increment(x:i32)->i32{return x+1;}"
).unwrap();
let library = wat::emit_library(&compilation, &["increment"]).unwrap();
println!("{}", library.wat());
println!("{}", library.export_manifest());
```

`modules::load`で相対importも利用できます。`from_source`は従来どおりファイルI/Oを行いません。公開指定は入口モジュールの元の関数名を使います。

| 段階・API | 保持・検査する情報 |
| --- | --- |
| モジュール解決 | 元の関数名、解決後の名前、宣言と名前のSpan、pub、入口所属、genericの有無を記録 |
| `Compilation::function_declarations()` | 宣言情報の読み取り専用一覧。非モジュール形式は従来の匿名SourceId=0とprivateを保持 |
| 共通IR構築 | 既存の名前・型解決を再利用。ホスト向けに別の意味論を構築しない |
| `wat::emit_library` | 宣言とASTの対応、ライブラリ全体、選択した公開関数の型を検査 |
| `Library::program()` | 通常と同じ完成IR。未使用関数も保持し、観測や他経路での比較に使える |
| `Library::exports()` | 選択順の公開名・FunctionId・確定した引数型／結果型・名前のSpan |
| `Library::wat()` | 通常と同じloweringで関数本体を生成し、型変換を明示したwrapperを追加 |
| `Library::export_manifest()` | `cerune-wat-library-v1`。公開名、型、内部型、wrapper、関数対応、SourceIdとバイト範囲 |

内部名・FunctionId・SourceIdはコンパイル単位の観測情報で、永続的な外部識別子ではありません。外部から使う名前は明示選択した元の名前です。本文を対応付けるときは、同じCompilationの`source_manifest()`を別に保存します。公開一覧には本文を含めません。

宣言情報を変える公開setterはありません。既存の公開ASTを利用者が変更し、元の関数名・位置・generic情報との対応が崩れた場合は診断します。生成物が保持するIRと公開一覧も読み取り専用です。

## 初版の受入範囲

| 項目 | 契約 |
| --- | --- |
| 公開関数 | 入口モジュールのpub関数を1個以上、重複せず明示指定。private、存在しない名前、依存先の直接指定は拒否 |
| 公開型 | 引数・結果はi32/f64。引数ゼロも可。void結果、他の公開型は未対応 |
| i32 ABI | 外側はWasm i32、内部は既存のi64。wrapperで符号拡張し、結果をi32へ戻す |
| f64 ABI | Wasm f64。NaN、Infinity、負のゼロを既存の算術と同様に扱う |
| 本文 | bool・各整数・f32/f64、局所更新、条件、反復、通常の関数呼出し、数値変換、スカラー定数 |
| ライブラリ全体 | 未使用の関数と依存先も検査。型宣言、generic宣言、複合値、I/O、所有・確保操作は初版で拒否 |
| 初期化 | トップレベル実行文と入口のmainを拒否。start節や合成mainを生成しない |
| 観測 | 元IR、通常WAT、wrapper付きWATを分けて取得。未使用関数の削除や暗黙の最適化は行わない |

依存先のpub関数を使う場合も、入口に通常の関数を書いて明示的に呼び出します。[library.ceru](../../examples/host_calls/library.ceru)がその例です。pubだけで自動的にWasm exportにはなりません。

## 実行時の責務

`Library::javascript()`と`Library::typescript()`は[検査付きJSと型宣言](wat-js-bindings.ja.md)を生成します。JSは引数の数・型・範囲を実行前に検査し、実行時停止後はハンドルを閉じます。生のWasmを直接呼ぶ場合は呼出し側が正しい入力を用意します。

必要な場合だけ`cerune.write_error_byte`をimportし、言語上の停止は既存のruntime-v1とNodeId/SourceId/Spanを出力してtrapします。内部memoryは公開せず、DOM・ネットワーク・一般のhost呼出しも追加しません。停止後のインスタンスは呼出し側が破棄します。生のexport自体には「閉じたハンドル」の検査はありません。ループの終了も保証しません。

WATは引き続きemit-onlyです。Wasmへのassembly・実行は外部ツールが担当し、build/releaseの責務を増やしません。

## 実行・検証

[通常のCerune例](../../examples/host_calls/README.md)と[Rust APIの生成例](../../experiments/host_calls/README.md)を分離しています。

```sh
cargo test --lib codegen::wat::library
cargo test --test modules --test wat_output
cargo run --quiet -- run examples/host_calls/library_demo.ceru
```

通常例の出力は11、10、42です。テストは同名の依存先関数と公開範囲、未使用関数のI/O、未対応型、古くなった宣言情報、通常IR/WATの保持を確認します。Nodeとwabtが利用可能なら、i32/f64の反復呼出しと既知の期待値、VMとのf64結果・依存先での停止記録を比較します。CIのWindows/Linux/macOSのWAT検証ではツールを必須にしています。
