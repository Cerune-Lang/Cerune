# WAT子プロセスABI試験

[English](README.en.md)

[8個の版付きimport](../../../docs/design/wat-process-host.ja.md)を、Wasmiと既存Rust共通runtimeで検証します。[参照hostのprocess.rs](../../wat_host/src/process.rs)は要求の転送と状態検査を担当し、prepare/execute・失敗選択・Report・資源の保持は既存C ABIへ委ねます。Cの生成やコンパイルは行いません。

~~~sh
cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host --test process_protocol
~~~

Windows x86-64 MSVC / Linux x86-64 GNUが対象です。この独立hostの対応targetでprocess-c-abiを有効にします。ABI試験・生成WAT試験・通常の参照host CLIが同じ転送実装を使用します。Cerune本体の標準依存や実行経路は増やしません。Windows/Linuxの既存WAT CIでこの試験も実行します。

| 確認すること | 方法 |
| --- | --- |
| import型・旧host拒否 | probe.watを実際にinstantiate。必要なimportがない場合は失敗 |
| 本文・数値 | 全256バイト、UTF-8分割、NUL・CR/LF・結合文字、u64最大値・2^53超の出自を確認 |
| 順序・長さ | 重複・不足・過剰・誤順序・二重prepare/execute・UTF-8不正を拒否 |
| 共通の失敗選択 | InvalidRequest/RequestLimitがUnconfiguredに先行。予約失敗は起動せずOutputBudget/AllocationFailedを返す |
| 観測・解放 | Reportの繰返し・逆順読取り、空出力・範囲外copy、予約取消し、部分要求の破棄、次の操作への移行 |

試験用のexportは生成物に追加しません。memoryは公開せず、本文は最大8バイトの値で渡します。接続ありの試験は登録先なしで行い、子やLinux監督を起動しません。

[生成WATの比較](../../wat_host/tests/process_generated.rs)では同じprocess.rsを使い、既存.ceru example、配列予約・解放・位置付き診断、Windows/Linuxの実子入出力を検証します。上の独立したABI試験とは別のテストです。[CLI比較](../../wat_host/tests/process_cli.rs)で明示JSON・target、時間超過、回収失敗、標準入出力・ファイル・引数との併用も検証します。回収失敗は試験用Linux監督で再現します。runtime・監督の公式配布は後続です。
