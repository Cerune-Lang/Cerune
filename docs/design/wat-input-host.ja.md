# WATの行入力host契約 v1

[English](wat-input-host.en.md)

[共通の行入力契約](line-input.ja.md)をWATへ接続します。hostは入力を供給する実行接続であり、観測APIではありません。内部memory・文字列address・所有情報は渡しません。

## importと互換性

入力命令を含む生成物だけが次のimportを要求します。未使用の関数も除去しないため、std:ioのimportだけでも必要です。

| cerune import | 型 | 意味 |
| --- | --- | --- |
| stdin_status_v1 | () → i32 | 消費なしの接続確認。0=利用可能、1=未設定、2=未対応接続、3=接続失敗 |
| read_stdin_byte_v1 | () → (i32 byte, i32 status) | 1バイト要求。statusは0=1バイト、1=EOF、2=割込み、3=読取り失敗 |

成功byteは0〜255、その他のstatusではbyte=0です。範囲外のbyte・未知statusはinvalid-responseとし、直前までの消費数とconsumption=unknownを記録します。接続確認の未知状態もinvalid-responseです。読取り失敗・割込みでは消費しません。割込みだけ再試行し、先読み・巻戻し・暗黙flushは行いません。接続はinstanceの実行中に差し替えず、hostから実行中のmainへ再入しません。

入力を含まない生成物は既存hostで引き続き動きます。入力を含む生成物は両importが必要で、不足時はinstantiateに失敗します。未設定入力をEOFと見なす互換動作はありません。新しいhostが未設定を表す場合はstdin_status_v1から1を返します。既存のwrite_stdout_v1、数値整形、flush_stdout_v1、write_error_byteは変更しません。

## 生成WATの責務

上限→接続→既知EOF→予算・確保→読取り→UTF-8の順に処理します。入力領域は既存の文字列allocatorで上限分を予約し、成功時は同じ領域の長さと上限に対する使用バイト数を実長に縮めます。失敗・空EOFでは解放します。memory.grow等の物理確保失敗はallocation-failedです。

失敗時はwrite_error_byteでio-v1の理由・上限・消費数・NodeId/Spanを出し、unreachableで停止します。本文を診断へ含めません。成功操作の共通観測記録は今回追加しません。mainへ戻らないhost例外や診断配送の失敗はhost自身の失敗であり、言語の入力結果として成功扱いしません。

EOFはinstance内で保持します。失敗後のinstanceの再開は公開契約に含めず、入力失敗を保持して後続の読取りを防ぎます。記録取得で接続応答を差し替える機構は追加しません。

## 検証

同じ入力を各実行器・C・LLVM・QBE・WATへ供給し、行／EOF・出力・停止理由・消費数・出自を比較します。実file/pipeの試験と、読取り失敗・割込み・契約違反・確保失敗の注入を分けます。旧hostの拒否、入力なし生成物の互換性、memory非公開、失敗後の再読取り防止も確認します。テストhostは汎用のWASI runtimeや標準ライブラリではありません。

## 実行例（テストhost）

[行入力のソース](../../tests/fixtures/line_input.ceru)と[入力ファイル](../../tests/fixtures/line_input.stdin)を使い、[期待出力](../../tests/fixtures/line_input.stdout)と比較できます。WABT 1.0.39とNode.js 20を使います。テストhostは同期file/pipeを読み、Windows consoleはunsupported-connectionで拒否します。

~~~sh
cargo run -- emit-wat tests/fixtures/line_input.ceru -o target/line-input.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm target/line-input.wat -o target/line-input.wasm
node tests/support/run_wasm_input.cjs target/line-input.wasm < tests/fixtures/line_input.stdin
~~~

WindowsのPowerShellでは最後の行を `cmd /c "node tests/support/run_wasm_input.cjs target/line-input.wasm < tests/fixtures/line_input.stdin"` とし、テキストpipeの変換を避けます。[経路比較](../../tests/stdin_routes.rs)と[host契約試験](../../tests/support/wat_input.rs)に同じ例・異常系を含めています。
