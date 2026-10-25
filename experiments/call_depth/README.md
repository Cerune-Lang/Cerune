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

## 生成Cの上限と資源条件

Rust API `codegen::c::emit_c_with_call_limit` を試す入口として、`call_depth_emit_c`を用意しています。既存のCLIに上限設定を加えるものではありません。出力先は新規ファイルだけを受け付けます。

```powershell
cargo run --example call_depth_emit_c -- examples/call_frames/depth.ceru 2 target/depth-limit-2.c
clang --target=x86_64-pc-windows-msvc -std=c11 -O0 target/depth-limit-2.c -o target/depth-limit-2.exe
.\target\depth-limit-2.exe
```

上限2なら[depth.stdout](../../examples/call_frames/depth.stdout)と一致します。上限1で別のCを生成すると、outerからmark(10)へ入る直前にcall-v1をstderrへ記録し、終了コード1で停止します。本文の出力はありません。生成Cには引数用一時値、enter／leave、履歴確保、上限が残ります。

物理stackの条件を揃える実験では、ターゲット・Cコンパイラ・上限を明示します。Windowsはリンカの`/STACK`で予約上限、Linuxは`prlimit --stack`でOS上限を指定します。これらは実験用の制御で、Ceruneの観測APIではありません。

```powershell
cargo build --example call_depth_emit_c
node experiments/call_depth/probe_c.cjs target/debug/examples/call_depth_emit_c.exe clang windows-msvc target/c-depth-windows 32,128 2097152
```

```sh
cargo build --example call_depth_emit_c
node experiments/call_depth/probe_c.cjs target/debug/examples/call_depth_emit_c clang linux-gnu target/c-depth-linux 32,128 2097152
```

`probe_c.cjs`は実行環境からターゲットを選ばず、指定ターゲットと実行環境が違えば拒否します。Linuxではprlimitが必要です。新規ディレクトリへソース・C・実行ファイル、正確なコマンド、コンパイラ版、生成物のSHA-256、生のstdout/stderr、終了状態、期待値との一致を保存します。子プロセスは生成30秒・コンパイル120秒・実行30秒で打ち切ります。中断や異常終了を深さ超過の成功に数えず、失敗時も記録を残します。

2026-10-25の[測定結果](observations-c.json)は次のとおりです。両OSの16ケースずつで、上限N−1はcall-v1と先行出力が一致し、上限Nは期待した値を返しました。

| 実行条件 | 値と深さ | 結果 |
| --- | --- | --- |
| Windows x86_64 MSVC、Clang 22.1.8、予約stack 2 MiB、O0/O2 | 整数／所有文字列と配列入り構造体、深さ32/128 | 16ケース一致 |
| Linux x86_64 GNU、Clang 10.0.0、stack上限2 MiB、O0/O2 | 同上 | 16ケース一致 |

これは物理stackの最大消費量や、任意のプログラムの安全範囲の証明ではありません。大きな固定配列・別のcompiler/ABI・生成補助関数による消費は別途検証が必要です。Cの深さ上限と物理stack不足を別の事象として扱い、共通の既定値はまだ選びません。

`cargo test --test c_output call_depth`は生成Cを4実行器と比較します。`node --test experiments/call_depth/probe_c.test.cjs`は、異常終了・余分な出力・欠落した履歴を誤成功にしないことを検証します。
