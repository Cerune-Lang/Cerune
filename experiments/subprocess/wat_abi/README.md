# WAT子プロセスABI試験

[English](README.en.md)

[8個の版付きimport](../../../docs/design/wat-process-host.ja.md)を、Wasmiと既存Rust共通runtimeで検証します。host.rsは要求の転送と状態検査を担当し、prepare/execute・失敗選択・Report・資源の保持は既存C ABIへ委ねます。Cの生成やコンパイルは行いません。

~~~sh
cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host --test process_protocol
~~~

Windows x86-64 MSVC / Linux x86-64 GNUが対象です。既存WAT host crateの試験依存としてprocess-c-abiを有効にし、通常のhost実行やCerune本体に新しい実行経路を追加しません。Windows/Linuxの既存WAT CIでこの試験も実行します。

| 確認すること | 方法 |
| --- | --- |
| import型・旧host拒否 | probe.watを実際にinstantiate。必要なimportがない場合は失敗 |
| 本文・数値 | 全256バイト、UTF-8分割、NUL・CR/LF・結合文字、u64最大値・2^53超の出自を確認 |
| 順序・長さ | 重複・不足・過剰・誤順序・二重prepare/execute・UTF-8不正を拒否 |
| 共通の失敗選択 | InvalidRequest/RequestLimitがUnconfiguredに先行。予約失敗は起動せずOutputBudget/AllocationFailedを返す |
| 観測・解放 | Reportの繰返し・逆順読取り、空出力・範囲外copy、予約取消し、部分要求の破棄、次の操作への移行 |

試験用のexportは生成物に追加しません。memoryは公開せず、本文は最大8バイトの値で渡します。接続ありの試験は登録先なしで行い、子やLinux監督を起動しません。

**未対応：** emit-watへの言語操作・配列予約・診断の接続、生成された.ceruプログラムの比較、参照host CLIの接続JSON指定、実際の子の入出力・時間超過・回収比較。これらは次の段階です。未対応の言語exampleは追加していません。
