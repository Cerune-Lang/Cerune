# WATのプログラム引数host契約 v1

[English](wat-arguments-host.en.md)

[共通の引数契約](dogfooding-inputs.ja.md)をWATへ接続します。hostは明示的な実行時入力を供給し、生成WATが検査・コピー・取得・所有を担当します。内部memory・文字列address・所有情報は公開しません。

## importと互換性

引数操作を含む生成物が次のimportを要求します。未使用関数を除去しないため、std:envのimportだけでも必要です。

| cerune import | 型 | 成功時の意味 |
| --- | --- | --- |
| arguments_count_v1 | () → (i64 count, i32 status) | 実行ファイル名等を除いた引数件数 |
| argument_length_v1 | (i64 index) → (i64 length, i32 status) | 指定引数のUTF-8バイト数。空文字は0 |
| argument_byte_v1 | (i64 index, i64 offset) → (i32 byte, i32 status) | 指定位置の1バイト。0〜255 |

status=0が成功、2が接続失敗です。countだけはstatus=1・count=0で未設定を表せます。その他のstatus、負の件数・長さ、範囲外byteはinvalid-responseです。status=2の値部分は使用しません。indexとoffsetは取得済み件数・長さの範囲内で要求します。EOFや割込み再試行はありません。

hostは開始前に入力を固定し、起動時の繰返し参照へ同じ応答を返します。正規化・再分割はせず、JavaScript文字列を扱うhostは不正なUTF-16を置換して成功にしません。元配列やBufferを変更してもsnapshotへ反映させません。契約を守らず取得中に値を変えるhostの意味は保証しません。host例外はhost自身の失敗であり、空文字・欠落へ変換しません。

引数を含まない生成物のimportは変わりません。引数を含む生成物では上記3つと既存のflush_stdout_v1・write_error_byteが必要で、不足時はinstantiateで失敗します。未設定は空列として扱わず、実際のcount/get命令で位置付き診断を出します。既存の出力・行入力のhost契約は変更しません。

## 生成WATの処理

1. mainの開始時に接続状態・件数を確認する。
2. 上限の妥当性、件数、各引数の累積バイト数・UTF-8・NULを共通順序で検査する。UTF-8検査の遷移規則は行入力と共有する。
3. 全件が妥当なら、非公開memoryに表と内容を予約・コピーする。検査とコピー中はhostの固定入力を複数回読む。部分的なsnapshotを公開しない。
4. Ceruneの実行を始める。count/getは非公開snapshotだけを参照し、hostへ再読取りしない。取得値は既存文字列ヒープへコピーし、通常の所有展開で解放する。空文字は確保しない。

`emit-wat --argument-count-limit`／`--argument-byte-limit`はWATに見える定数へ上限を設定します。既定4096件・UTF-8合計1048576バイトで、CR/LFを含み、終端や管理領域を含みません。Rustではemit_wat_with_argument_limitsを使い、既存emit_watは既定値を維持します。emit時にhost入力を読んだり、実行用の区切り以後の引数を受け取ったりしません。

snapshotと返却文字列は別の上限です。同じ非公開linear memoryを物理的に使いますが、snapshotを返却文字列の使用バイト数へ加算しません。wasm32のアドレス容量・memory.grow等で確保できない場合はallocation-failedです。引数を含むinstanceのmainは一度だけ実行でき、失敗後も再開しません。再実行には新しいinstanceを用意します。snapshot領域はinstance破棄時に回収されます。

## 診断と残件

起動中はarguments-v1 phase=captureで理由と該当添字を出し、本文や架空のソース位置を含めません。接続失敗はconnection-failed、契約外応答はinvalid-responseです。実行時はphase=executeで操作・理由・添字・NodeId/SourceId/Spanを保持し、既存のflush接続へ先行出力を渡してからunreachableで停止します。診断配送・host自体の例外は別のhost失敗です。

元IRを変更せず、引数命令・出自・起動時検査・上限・コピー・所有操作を生成物で確認できます。成功した参照の時系列記録は未実装です。初版はmain実行のWATが対象で、純粋な公開関数を対象とするWAT libraryの制限は維持します。WASI・OS引数接続・安定した埋込みABIは追加しません。Nativeの引数接続と整数読取りは後続です。

## 検証と実行例

[引数example](../../examples/arguments/README.md)と[比較テスト](../../tests/program_arguments/wat.rs)で、4実行器との出力・停止理由・出自・先行出力、所有・解放、未設定／空／欠落、不正入力、上限、確保失敗、旧host拒否、memory非公開、stdin併用を検証します。テストhostは入力固定後の元データ変更と不正UTF-16を検査し、通常出力が始まった後の引数再読取りを拒否します。Windows・Linux・macOSのWAT CIで必須実行します。

[テストhost](../../tests/support/run_wasm_input.cjs)はCERUNE_TEST_WAT_ARGUMENTSのJSONで明示注入します。valuesの要素には文字列またはUTF-8のバイト配列を指定できます。この環境変数はテストhostの設定であり、Cerune言語が環境変数を探索する機能ではありません。指定を省くと引数importを追加せず、旧hostの互換性を検査できます。

大きなテスト入力は同じJSONをファイルへ保存し、CERUNE_TEST_WAT_ARGUMENTS_FILEでパスを指定できます。両方の指定は拒否します。Windowsの環境変数長に依存せず、ページ拡張・物理確保失敗・取得文字列の上限分離を検証できます。
