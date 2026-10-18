# WATライブラリのJS/TS API

[English](wat-js-bindings.en.md)

[WATライブラリ](wat-library.ja.md)の選択した公開宣言から、検査付きJSとTS宣言を生成します。初版はi32/f64の引数・結果です。一般のhost呼出し、複合値、専用CLI、IR/VMのJS公開は今回追加しません。

## 生成と利用

Rustの `Library::javascript()` を `library.mjs`、`Library::typescript()` を `library.d.mts` として保存します。元IR・通常WAT・公開用WAT・公開一覧は従来どおり別に取得できます。生成はファイルI/Oや暗黙の最適化を行いません。

```js
import {createLibrary, metadata, CeruneHostError} from './library.mjs';
const response = await fetch('./library.wasm');
if (!response.ok) throw new Error('Wasm load failed');
const library = await createLibrary(new Uint8Array(await response.arrayBuffer()));
console.log(library.functions.step(10, 2, 0.5)); // 11
console.log(metadata); // 読み取りだけ。プログラムは実行しない。
```

Wasmの読込みは呼出し側が明示します。`createLibrary` はUint8Arrayを受け取り、作成ごとにcompile/instantiateします。同じハンドルへの関数呼出しでは再コンパイルしません。WATは引き続きemit-onlyで、assemblyは外部ツールの責務です。

## 呼出し契約

| 対象 | 検査・動作 |
| --- | --- |
| 引数数 | 公開宣言と完全一致。余分・省略も拒否 |
| i32 | Numberの整数、−2147483648〜2147483647。−0は拒否 |
| f64 | Number。NaN、Infinity、−0も値として保持 |
| 暗黙変換 | string、BigInt、boxed Number、変換可能なobjectを拒否。変換メソッドも呼ばない |
| 戻り値 | 同じ型・範囲を検査。正常なNaNを失敗の印にしない |
| TS宣言 | 選択した関数名・引数数・numberを生成。整数範囲などはJSの実行時検査が担う |
| 実行 | 同期呼出し。ループの終了・時間制限は保証しない |
| 公開ハンドル | 凍結した `functions` のみ。instance/module/memoryや任意のhost callbackを渡さない |

関数一覧はprototypeを持たず、`__proto__`・`constructor`などの名前もそのまま扱います。`functions`で包むため、公開名`then`をPromiseの処理と取り違えません。

## 失敗と再利用

すべてのAPI失敗は `CeruneHostError` で分類します。実行を始めた後の停止ではハンドルを閉じ、次の呼出しは`closed`になります。新しいハンドルは別に作成できます。

| phase | 意味 | 同じハンドルの再利用 |
| --- | --- | --- |
| compile | 入力の形式またはWasm compile失敗 | 未作成 |
| link | export名・種類・数、importが契約と不一致 | 未作成 |
| instantiate | インスタンス作成失敗、作成中の予期しない診断 | 未作成 |
| arguments | 引数数・型・範囲の不一致。Wasmを呼ばない | 可 |
| language | runtime-v1診断を伴うWasmの停止 | 不可 |
| trap | 診断のないWasm trap | 不可 |
| host | host例外、不正な診断、戻り値の契約違反 | 不可 |
| closed | 停止済みハンドルの呼出し | 不可 |

エラーの`functionName`は呼出し対象、`cause`は元例外です。languageでは`record`にruntime-v1、`failure`にcode・NodeId・SourceId・バイト範囲を保持します。匿名SourceIdは0です。元の本文は含めず、同じ生成時のsources.jsonと対応付けます。診断は内部でASCII最大4096バイトを収集し、契約外の値はhost失敗にします。観測用の変更可能なcallbackは設けません。

## 観測と適用範囲

`metadata`はexports.jsonと同じ公開宣言・型変換・出自を持ち、内部まで凍結しています。読取りは実行や状態変更を起こしません。通常のIR/WATや未使用関数も残し、生成JS・型宣言自体も確認できます。

**同じ生成から得たWasmとJSを組にして使います。** WebAssemblyのreflectionで確認するのはexport/importの名前・種類・数です。任意のWasmの関数署名や本文の一致を証明する検査ではなく、改変されたWasmを隔離するsandboxでもありません。生のWasmを直接呼ぶ場合には、このJSの検査と停止後の管理は適用されません。

## 検証

[ブラウザー例](../../experiments/host_calls/browser/README.md)で反復呼出し・公開一覧の読取り・異常引数・依存先での停止・再生成を試せます。NodeのテストではVMとの結果・停止記録比較、全phase、i32範囲、f64特殊値、特殊な公開名、内部memoryの非公開を検査します。生成TSはstrictで検査し、CIでは日本語・英語のHTMLをChromiumで操作して画面を保存します。
