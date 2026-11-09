# 子プロセスのCLI接続

[English](subprocess-cli.en.md)

`run`・`run-ir`・`run-mir`・`run-mir --ssa`・`run-vm`に`--process-config <JSONファイル>`を指定し、[共通runtime](subprocesses.ja.md)へWindows/Linux接続を渡します。指定がなければ妥当な要求は`Unconfigured`を返します。環境変数・探索パス・設定ファイルの自動探索は使いません。

## 設定と責務

設定ファイルはUTF-8のJSON、最大1 MiBです。版・OS・全上限・登録先を実行前に検査します。未知の項目、項目の重複、必須項目の欠落、負数・小数・整数幅超過は拒否します。設定本文や環境変数の値を診断に転記しません。

次はLinux向けの形です。絶対パスを利用者の環境に合わせて指定してください。

```json
{
  "schema": "cerune-process-v1",
  "platform": "linux",
  "linux_supervisor": "/absolute/path/subprocess_linux_supervisor",
  "limits": {
    "arguments": 32,
    "argument_bytes": 16384,
    "stdin_bytes": 1024,
    "output_bytes": 1024,
    "timeout_ms": 10000,
    "cleanup_timeout_ms": 5000
  },
  "transfer_bytes": 32768,
  "targets": [{
    "name": "tool",
    "executable": "/absolute/path/child",
    "directory": "/absolute/working/directory",
    "environment": [["CERUNE_CHILD_VALUE", "日本語\r\n"]]
  }]
}
```

| 項目 | 意味・検査 |
| --- | --- |
| `schema` | `cerune-process-v1`固定。別版を黙って解釈しない |
| `platform` | `windows`または`linux`。実行するCeruneのOSと一致必須。他OSの接続は未対応 |
| `linux_supervisor` | Linuxは専用監督の絶対パス必須。Windowsは省略またはnullのみ |
| `targets` | 最大64登録。名前は一意、空・NUL不可、UTF-8で256バイト以内。空リストは許可 |
| `executable` / `directory` | 実行ファイル・子の作業場所の絶対パス。設定ファイル位置から補完しない |
| `environment` | 名前と値の対のリスト。最大128対、親の環境は継承しない。空リストも有効。重複名は拒否（Windowsは大文字小文字を区別しない） |
| `limits.arguments` | 1要求の引数件数上限。0〜4096 |
| `limits.argument_bytes` | 対象名と全引数のUTF-8合計バイト上限。0〜16384。WindowsのUTF-16表現上限等は別途OS接続が検査 |
| `limits.stdin_bytes` | 1要求の入力バイト上限 |
| `limits.output_bytes` | 1要求で指定するstdout/stderr上限の合計上限 |
| `limits.timeout_ms` | 要求が指定できる実行時間の上限。正のu64ミリ秒 |
| `limits.cleanup_timeout_ms` | 1操作の後始末の時間上限。正のu64ミリ秒 |
| `transfer_bytes` | 両出力の予約領域と3個の4096バイト転送bufferの合計上限。言語の配列領域上限とは別 |

件数・バイト数はホストの`usize`以内です。0の容量も明示可能で、受理できない要求は起動前の失敗値になります。数値の既定値や黙った切詰めは設けません。OS固有のパス・環境・登録総量の上限は[Windows](subprocess-windows.ja.md)・[Linux](subprocess-linux.ja.md)を参照してください。

Windows設定は`platform`を`windows`にし、`linux_supervisor`を削除して、`C:/absolute/path/child.exe`等を明示します。shellを介さず、対象名を登録先へ解決します。Windows引数は既存のMicrosoft CRT規則で符号化します。任意の独自引数解析器との互換性を保証するものではありません。

## 実行と失敗

```sh
cerune run app.ceru --process-config process.json -- argument
cerune run-mir app.ceru --ssa --process-config process.json
cerune run-vm app.ceru --process-config process.json
```

- 設定の読取り・検証失敗はプログラムの実行前にCLIを停止させます。子やプログラムの最初のprintも実行しません。
- 実行ファイルの存在は設定検査で先読みしません。登録先が存在しない場合は操作の`NotFound`、未登録名は`Unregistered`です。起動・転送・時間・容量の失敗は`Outcome`で返し、後続処理を継続できます。結果自体を作れない資源不足は既存の言語停止です。
- 非0終了はCLI自身の失敗ではありません。子の終了コードと生バイトは`Outcome`に保持します。Cerune側の後続処理が正常完了すればCLIも正常終了します。
- 同じ接続を実行中の呼出しで共有します。共通runtimeが接続を停止した後は、次の要求に`ConnectionDisabled`を返します。
- `--`以降は従来どおりプログラム引数です。そこにある`--process-config`を設定指定として解釈しません。既存の標準入出力・`--file-root`・呼出し深さ・領域上限と併用できます。
- 型・操作・NodeId/Spanは[共通IRの契約](subprocess-execution.ja.md)を維持します。CLI初版はイベント履歴を保存しません。返却ReportとIRの出自は観測でき、Rust APIの読取り専用記録も維持します。

## 検証と残る作業

[登録先を呼ぶexample](../../examples/process_execution/README.md)を、Windows/Linuxで同じ4実行器（`run-ir`別名も含む）から実行します。[CLIテスト](../../tests/process_cli/mod.rs)は引用符・空引数・日本語、環境・cwd、NUL・不正UTF-8を含む生バイト、非0終了、未登録・不存在、容量・時間超過と継続、既存入出力との併用、設定異常を検査します。任意の利用者プログラムを比較のため自動再実行しません。

Linuxの実OSテストは`CERUNE_TEST_PROCESS_SUPERVISOR`で明示した検証用監督を必要とします。通常のテストではignoreされ、OS専用CIで`--include-ignored`を付けて必ず実行します。これはテスト設定であり、Cerune CLIはこの環境変数を読みません。

Linux監督は現在Cargo exampleとして構築します。**監督の配布同梱、生成C・LLVM・QBE・WAT・Nativeの実行ABI、Native build/releaseの設定記録は後続です。** emit/build/releaseへ今回のCLIオプションを適用しません。生成経路では未対応操作をソース位置付きで拒否します。
