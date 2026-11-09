# 子プロセスの公開データ型

[English](subprocess-values.en.md)

`import "std:process" as process;`で要求・結果の型を利用できます。実行は別の`std:process/run`から利用し、[共通IRと4実行器](subprocess-execution.ja.md)へ接続済みです。CLI登録と生成経路の実行対応は後続です。

## 型と保持する情報

[同梱ソース](../../src/standard/process.ceru)は通常の構造体・enumを宣言します。専用の値体系や汎用Result型は追加しません。

| 型 | 内容 |
| --- | --- |
| `Request` | `target: string`、`arguments: [string]`、`stdin: [u8]`、`stdout_limit: u64`、`stderr_limit: u64`、`timeout_ms: u64` |
| `Outcome` | `report: Report`と所有する`stdout: [u8]`・`stderr: [u8]`。部分出力も保持 |
| `Report` | 出自、起動状態、指定上限、要求・確認済み・保持バイト数、進行の確実性、終了状態、元の失敗、後始末の判定と生報告 |
| `Origin` | `node_id`・`source_id`・`start`・`end`を`u64`で保持 |
| `Started` | `No`・`Yes`・`Unknown` |
| `Exit` | `Absent`・`Code { value: u32 }`・`Signal { value: u32 }` |
| `Failure` | `None`・`Reason { code: FailureCode, phase: Phase }` |
| `Phase` / `Channel` | 検査・予約・起動・転送・stdin閉鎖・待機・後始末。転送はstdin/stdout/stderrを区別 |
| `FailureCode` | 共通runtimeの21分類と同名。未接続・容量超過・時間切れ・進行不明等を統合しない |
| `Cleanup` / `CleanupError` | 未報告、完了、未完了。終了要求、子の回収・handle閉鎖・管理範囲の終了、後始末の失敗を保持 |

公開値は通常のプログラムでも構築できます。値があることはOS操作を実行した証拠ではありません。現在の[example](../../examples/process_results/README.md)も値を構築して処理します。

## 共通runtimeとの対応

共通runtimeと言語値は次のように対応します。

| Rust側 | 公開値の意味 |
| --- | --- |
| `Report.failure: Option<Failure>` | `None`は操作完了。子の終了コード0とは別。最初の失敗を後始末失敗で上書きしない |
| `Report.exit: Option<Exit>` | 未取得を`Absent`とする。Windowsの32ビット終了値を切り詰めず、signalへ推測変換しない |
| `Report.cleanup` / `cleanup_reported` | 期限等を含む共通runtimeの判定／接続からの生報告を別々に保持。未報告は`Absent` |
| `stdin_requested` / `stdin_accepted` | 要求バイト数／確認済みの受理バイト数 |
| `stdout_read` / `stderr_read` | 接続が読取りを確認したバイト数 |
| `stdout_retained` / `stderr_retained` | 結果の配列に保持したバイト数。読取り数と同一とは限らない |
| `progress_known` | falseなら転送数は確認済みの下限。未確認の進行を0や巻戻しへ置き換えない |
| `Origin` | NodeIdとSpanのSourceId・UTF-8バイト位置`[start, end)`。IDはコンパイル内で有効、SourceId 0は既存の匿名ソース。絶対パスや本文は含めない |
| `Outcome.transfers`・イベント列 | 個別転送の状態はRust側の読取り専用観測に残す。公開Outcomeへ操作handleや制御APIを渡さない |

公開enumの順番をRustの判別値や外部ABIに直結させません。変換は名前と内容の対応を検査します。`FailureCode`の全分類とRust側の対応はテストで確認します。

## 要求・配列・確保

- 時間の公開単位は整数ミリ秒です。言語からの要求は正確に`Duration`へ変換し、任意のRustのサブミリ秒値を黙って丸めて公開する変換は設けません。
- `u64`で表現できることと、実行時に受理する範囲は別です。対象名・NUL・容量・時間・ホスト整数幅の検査は起動前に行います。不正な要求値も構築できます。実行時の受理範囲はRust接続のLimitsで明示し、CLI設定は後続です。
- 本文はバイト配列です。不正UTF-8・NUL・CR/LFを保持し、改行追加・正規化・暗黙の文字列変換を行いません。配列のコピー・関数受渡し・更新は既存の値意味論と所有の展開を使います。
- **[起動前の予約APIと配列の確定・取消し](subprocess-storage.ja.md)を実装しました。4実行器の言語命令へ接続済みです。** 現在の`u8`配列は共通表現で要素あたり64ビット領域を使うため、本文のバイト数だけを言語の配列使用量に数えません。出力保持buffer・転送buffer・言語配列・変換中の一時領域の所有と使用量を分け、起動前に必要領域を予約します。
- 値自体を構築できない処理系の資源不足を、子の終了コードや操作完了へ偽装しません。失敗後の再実行も行いません。

## 観測と検証

標準ソースは識別子`std:process`・契約版1で登録されます。別名・別ファイルからのimportでも同じ型定義へ解決し、`compile(&str)`でも暗黙のファイルI/Oを追加しません。`emit-sources`で宣言と出自、`emit-ir`以降で通常のenum・所有・配列の展開を確認できます。

C出力ではフィールドを`cerune_field_`、enum内部フィールドを別の`cerune_internal_field_`名へ変換し、`stdin`等のヘッダーマクロやC予約語と分離します。ソースのフィールド名とNodeId/Spanは共通IRに保持します。

[比較テスト](../../tests/process_types/mod.rs)は4実行器と生成C・LLVMの出力、型同一性、失敗分類、コピーの独立性、型モジュールにない`run`の拒否を確認します。QBE・WAT・ASM・Objectは生成を検証し、この例の実行比較とは区別します。LLVMの実行比較はWindows/Linuxのx86-64に限定し、生成時とリンク時に同じtargetを明示します。他のhostでも4実行器・Cの比較と生成確認は維持します。OSを起動した結果の言語変換を検証したことにはしません。

```sh
cargo test --test modules process_types
```

Reportの固定領域、Outcome全体と要求変換を含む[共通IRと4実行器](subprocess-execution.ja.md)を実装しました。次はCLI登録・上限・監督パスの接続、その後に生成経路のhost契約と配布同梱を揃えます。観測APIは実行を制御せず、成功する空実装で未対応を隠しません。
