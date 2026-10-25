# 呼出し深さと物理スタックの実験

[English](README.en.md)

**既定値を決める前の検証用です。通常のCerune CLIや再帰のexampleではありません。** 同じソース関数の深さでも、実行器・値の形・Rustビルド条件によって、共通上限の検査より先に物理スタックが枯渇し得ることを確認します。

## 測定方法

`probe.cjs`が再帰を使わない直線状の関数呼出しを生成します。深さNに対して上限N−1とNをそれぞれ実行し、前者は深さ超過、後者は完走することを期待します。整数だけの例と、所有文字列・固定長配列を持つ構造体の例を使います。生成補助関数は論理的な深さに加算しません。

| 項目 | 扱い |
| --- | --- |
| 分離 | 経路・値の形・深さ・上限ごとに別プロセス。1件30秒で打切り |
| 準備 | ソース解析、HIR・MIR・SSA・bytecodeの構築は測定threadの外。実行API内の検証と実行状態の解放は測定に含む |
| スタック | `thread::Builder::stack_size`への要求量を明示。OSが実際に確保した量や使用量を測るものではない |
| 記録 | 準備開始／完了、実行開始／完了、深さ診断、期待出力、終了code・signal、stdout・stderrを残す |
| 判定 | 正常終了codeと完全な応答、期待出力が揃った場合だけ一致。深さ超過は上限・履歴長・call-v1も照合 |
| 資源不足 | 実行開始後の異常終了とRustのstack overflowメッセージが揃った場合だけ `host-stack-exhausted`。それ以外は原因未確定の `process-terminated`、起動失敗、timeout等に分離 |
| 成功扱い | 期待値と不一致のケースが1件でもあればdriverは終了code 1。スタック枯渇を正常とは扱わない |

物理的なabortはCeruneの共通診断へ変換できません。実行器が保持していた先行出力も回収できない場合があります。これは外側の実験driverによる観測であり、言語本体の資源不足対応を実装したものではありません。

## 実行

RustとNode.js 20以降を使います。出力先は親ディレクトリが存在する**新規ディレクトリ**にしてください。以前の観測は上書きしません。

```sh
cargo build --example call_depth_probe
# Windowsでは実行ファイル名に.exeを付けます。
node experiments/call_depth/probe.cjs target/debug/examples/call_depth_probe target/depth-debug 8,32,64 2097152
cargo build --release --example call_depth_probe
node experiments/call_depth/probe.cjs target/release/examples/call_depth_probe target/depth-release 8,32,64 2097152
```

Linux等では先に同じshellで `ulimit -c 0` を実行し、意図的な異常終了によるcore dumpを抑えてください。Windowsの子プロセスは既存のテスト用処理でクラッシュダイアログを抑えます。大きな試行は通常CIでは実行しません。

成果物は次のとおりです。

| ファイル | 内容 |
| --- | --- |
| `environment.json` | OS・CPUアーキテクチャ・Node・現在のrustc・実行ファイルSHA-256・要求stack量。rustcはビルド時証明ではなく測定環境の記録 |
| `scalar-N.ceru` / `aggregate-N.ceru` | 全経路へ渡す同一ソース。通常CLIのemit-ir等でも変換を観測できる |
| `results.jsonl` | 正確な実行引数、期待値、分類、終了状態 |
| 各ケースの `.stdout` / `.stderr` | 生の観測。stdoutにはOS・debug_assertions・生成補助関数数・call-v1診断も含む |

`cargo run --example call_depth_probe -- hir <file.ceru> <上限> <stack要求bytes>`で単独実行もできますが、プロセス自体がabortする場合があります。通常はdriverを使ってください。

## 2026-10-25の結果

処理系本体はcommit `143aae0`（PR #200）、Rust 1.98.0、Windows x86_64 MSVC／Linux x86_64 GNUで測定しました。深さごとに上限N−1とNの両方を試しました。詳細は[測定結果](observations.json)に保持しています。

| 条件 | 整数だけ | 文字列・配列を含む構造体 |
| --- | --- | --- |
| Windows / Linux、debug、要求2 MiB、深さ8 | 4経路とも期待どおり | 4経路とも期待どおり |
| 同、深さ32 | HIRはstack枯渇。MIR・SSA・VMは期待どおり | 4経路ともstack枯渇 |
| 同、深さ64 | 4経路ともstack枯渇 | 4経路ともstack枯渇 |
| Windows、debug、要求16 MiB、深さ8/32/64 | 4経路とも期待どおり | 4経路とも期待どおり |
| Windows / Linux、release、要求2 MiB、深さ8/32/64 | 4経路とも期待どおり | 4経路とも期待どおり |

これは最小安全stack量や最大安全深さの探索ではありません。OS・コンパイラ・最適化・式の形・生成補助処理によって変わります。releaseで通る値を共通の安全な上限とみなしたり、debugだけ上限を変えたりはしません。C・LLVM・QBE・WAT・Nativeの物理資源測定はまだ含みません。

## 実装順と現在の進捗

1. 4実行器の関数呼出しを明示frameと継続位置へ移す（実装済み）。ソース関数だけの深さ規則を維持し、生成補助関数も物理frameとして管理する。
2. frameの確保失敗を深さ超過と区別し、引数の評価順、所有の解放、NodeId/Span、早期returnを既存テストと比較する。MIR・SSA・VMにも適用済み。
3. 生成経路の資源条件を検証し、共通の設定範囲・CLI・診断を整えてから再帰を解禁する。

上の表とobservations.jsonは変更前のcommit 143aae0の測定記録です。後続の実装で4実行器は明示frameへ移行し、要求2 MiB・深さ128の成功／深さ上限と、frame確保失敗の診断をテストに追加しました。[現在の状態](../../docs/design/recursion.ja.md)を参照してください。次の対象は生成経路の資源条件・深さ診断です。IRの二重実装、観測APIによる実行制御、暗黙の末尾呼出し最適化は導入しません。

CIでは `node --test experiments/call_depth/probe.test.cjs` で判定器の誤成功を防ぎ、深さ4・要求16 MiBの浅い16ケースでfixture・4実行器・期待出力を照合します。