# VMの埋め込み実行

[English](vm-embedding.en.md)

f64引数とf64/void戻り値のRust APIは実装済みです。JS/TS公開と生成WATとの比較は[型付きホスト呼出し](typed-host-calls.ja.md)で扱います。

## 目的

Cerune VMで生成済みbytecode内の関数を、Rustなどのホストアプリケーションから実行できる境界を定義します。

Ceruneにはすでに、ソースからbytecodeを生成する経路と、bytecodeをVMで実行する経路があります。

```text
Cerune Source
      ↓
Cerune IR
      ↓
Bytecode
      ↓
Cerune VM
```

通常のCeruneプログラムでは、入口のトップレベル文または`main`から実行を開始します。

埋め込み利用では、ホストアプリケーションが生成済み`BytecodeProgram`を保持し、その中の関数を選択して、引数を渡し、戻り値を受け取れるようにします。

この機能は新しいCerune言語機能ではありません。既存の関数・bytecode・VM実行を、ホストから利用するための公開境界です。

## 責務

Cerune側は次の処理を担当します。

```text
BytecodeProgram
      +
function
      +
arguments
      ↓
Cerune VM
      ↓
return value
runtime output
runtime error
```

ホストアプリケーション側は次を担当します。

* Ceruneソースの保持
* bytecodeを生成する時期の決定
* 実行対象関数の選択
* ホスト値とアプリケーション固有データの変換
* 実行結果の利用
* 必要に応じた計測やキャッシュ

Cerune VMの埋め込みAPIは、特定のホストアプリケーションのBackend Contractや計測方式を知りません。

## Emitterとの境界

VM埋め込みとEmitter成果物の実行は同じ責務にはしません。

CeruneのEmitterは、既存どおりBackend Artifactを生成します。

```text
Cerune IR
      ↓
Backend Lowering
      ↓
Emitter
      ↓
Backend Artifact
```

現行のemit成果物をどのツールでbuild・link・load・runするかは、受け取るアプリケーションが管理します。将来の[IR・VM・Nativeのbuild / release](owned-routes.ja.md)は別の操作であり、VM埋め込みAPIには組み込みません。C・LLVM・QBE・WAT等は引き続きemit-onlyです。

Ceruneへ以下を追加しません。

* C compilerの探索や実行
* LLVM toolchainの管理
* QBEの探索や実行
* Wasm Runtimeの管理
* assembler / linkerの管理
* 動的ロード
* ベンチマーク機能

これにより、Cerune自体は外部toolchainを必須依存にしません。

## 既存VMとの関係

VM内部では、関数ID・PC・operand stack・slot・初期化位置を呼出しごとの明示frameに保存し、call／returnを反復処理します。元bytecodeの命令と出自は保持します。[深さ上限と確保失敗](recursion.ja.md)は4実行器で共有します。

埋め込みAPIでは、この既存処理を再実装しません。

公開境界から受け取った値をVM内部の値へ変換し、既存の関数実行処理へ渡します。

```text
Host Value
    ↓
VM Value
    ↓
existing function execution
    ↓
VM Value
    ↓
Host Value
```

VM内部表現そのものを安定した公開APIとして公開することは避けます。

内部の`Value`、`Frame`、slot表現などはVM実装の都合で変更できる状態を保ちます。

## コンパイルと実行の分離

埋め込み利用では、コンパイルと関数実行を分離できることを重要な性質とします。

```text
Preparation

Cerune Source
      ↓
compile_to_bytecode
      ↓
BytecodeProgram
      ↓
function resolution


Execution

arguments
      ↓
VM function invocation
      ↓
result
```

一度生成した`BytecodeProgram`を複数回実行するために、関数呼び出しごとにソースを再コンパイルする必要はありません。

これは埋め込みアプリケーションが、コンパイル時間と実行時間を別々に扱えるようにするためでもあります。

## 関数の指定

VM内部では関数を数値IDで扱います。

公開APIでは、内部IDをそのまま長期的な識別子として保証しません。

関数名による解決と、解決済み関数を繰り返し呼び出す方法を分離できる設計とします。

概念上は次の流れです。

```text
BytecodeProgram
      ↓
resolve function
      ↓
resolved function
      ↓
repeated invocation
```

具体的な公開型は実装時に決定します。

generic functionや特殊化によって同名関数が複数存在する場合の扱いは、その情報が必要になった時点で明示的に設計します。単純な名前検索だけを将来の恒久的な関数識別規則とはしません。

## Host Value

公開APIはVM内部の`Value`とは別のHost Value表現を持ちます。

最初の実装では、必要な型だけを公開します。

Whitebaseなどの最初の利用例では`f64`のscalar演算だけでも成立するため、将来の型を先回りして公開APIへ固定しません。

配列、product type、sum type、stringなどを追加する場合は、それぞれの所有権、表現、コピー、エラー境界を決めてからHost Valueへ追加します。

特に整数では、型情報だけでなく値域もCeruneの意味を満たす必要があります。ホストからVMへ値を渡す処理によって、通常のCerune評価では作れない値をVM内部へ注入しないようにします。

## 実行結果

関数実行は戻り値だけでなく、実行中の出力と構造化されたVMエラーを保持できる必要があります。

概念上の結果は次の情報を持ちます。

```text
Function Execution
├── return value
└── output
```

失敗した場合は、既存のVMエラーとbytecode命令の出自を失わないようにします。

通常の`run_bytecode`と埋め込み関数実行で、同じVM停止が異なる意味のエラーへ変換されないことを目標とします。

## 非目標

VM埋め込みAPIでは、次を行いません。

* Cerune言語へ新しい関数構文を追加する
* `export fn`などの外部ABI指定を追加する
* Emitter成果物を実行する
* 外部compilerやruntimeを管理する
* VMと各Emitterを一つのExecution traitへ統一する
* 特定アプリケーション向けのBackend APIを追加する
* VM内部表現をそのまま公開APIとして固定する
* 最初から全Cerune型をHost Valueとして公開する

## 実装順

最初の実装は次の順序で進めます。

1. VM内部の既存関数実行処理を、crate内部から再利用可能な境界として整理する。
2. 最小のHost Valueを定義する。
3. 生成済み`BytecodeProgram`から関数を解決する。
4. 引数を渡して関数を実行し、戻り値と出力を取得する。
5. VMエラーと命令出自を既存の実行経路と同じように保持する。
6. コンパイル済みbytecodeを使った繰り返し実行をテストする。

このAPIはemit-only経路の外部ビルドや実行を管理しません。Cerune-ownedのbuild / release計画とは責務を分けます。
