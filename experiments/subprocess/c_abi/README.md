# 子プロセスC ABI・静的リンク試作

[English](README.en.md)

**Rust共通runtimeを独立したCプログラムへ静的リンクする試作です。** Ceruneの生成C・LLVM・QBE・Native・WATへの接続、Native build/releaseへの同梱は未実装です。言語exampleは[process_execution](../../../examples/process_execution/README.md)に分けています。

## 構築と実行

Rust 1.98、Node、Clangと対応OSのシステムリンカ・開発用ライブラリを使います。targetとClangは明示指定し、hostと異なるtargetでは試験を拒否します。Linux監督は先に構築します。出力先は未作成ディレクトリにしてください。

~~~text
node experiments/subprocess/c_abi/check.cjs x86_64-pc-windows-msvc clang target/process-c-abi
cargo build --example subprocess_linux_supervisor
node experiments/subprocess/c_abi/check.cjs x86_64-unknown-linux-gnu clang target/process-c-abi /absolute/path/to/subprocess_linux_supervisor
~~~

検証スクリプトはprocess-c-abi featureを指定してstaticlibを作り、[独立したC試験](probe.c)をリンクします。既定ビルドにはC ABIシンボルを追加しません。必要なシステムライブラリはtarget別に明示し、Rustが出力するnative-static-libsも記録します。通常のemitやbuildへ暗黙のライブラリ探索は追加しません。

出力先のcommands.jsonにコマンド・target・リンク入力・終了状態を保存します。config.jsonはローカルの絶対パスを含む試験設定で、配布物やCI artifactには自動収録しません。このarchiveは処理系crateから作る検証用成果物で、runtime配布パッケージの完成を意味しません。

## 呼出し契約

[ヘッダ](cerune_process_v1.h)の関数と[実装](../../../src/runtime/process/abi.rs)を対応させます。v1は現時点では試作契約です。

| 操作 | 状態・責務 |
| --- | --- |
| create | ABI版1とtargetを指定。UTF-8 JSONを共有解析器へ渡す。空設定なら明示的に未接続。失敗時の出力handleはNULL |
| prepare | Idleでのみ呼ぶ。要求をコピーして保持し、共通runtimeの検査・転送領域確保へ進む。1=Pending、2=Ready |
| execute | Pendingで一度だけ呼ぶ。予約結果0=成功、1=容量不足、2=確保失敗を渡す。共通Prepared::executeを使い、結果はReady |
| report | Pending/Readyの固定Report v1を32個のu64へコピー。元のNodeId/SourceId/Spanを保持 |
| copy | Readyのstdout/stderrを位置指定でコピー。内部ポインタは返さず、何度読んでも読取り位置を進めない |
| release | Pendingの未提出領域、またはReadyの結果を解放しIdleへ戻す。接続の停止状態・未回収OS資源は解除しない |
| destroy | contextを破棄して呼出し元handleをNULLにする。未回収資源は既存Backendの所有・隔離規則に従う |

prepare/execute/release/destroyは制御です。report/copyは時計・起動・転送・解放を進めません。準備後に元の要求や設定を変更・破棄しても接続内容は変わりません。context内の要求と接続は非公開にし、Pending中に置換できません。

要求は120バイト、Bytesは16バイトのC構造です。ポインタはこの64ビットtargetのアドレス、長さ・時間・出自はu64です。Reportは[固定Report v1](../../../docs/design/subprocess-generated.ja.md)の数値配置を使います。SourceIdの復元は元のSourceMapへの存在確認ではありません。

C側は有効かつ初期化済みの読取り領域、有効な書込み領域、相互・contextとの非重複を保証します。NULL＋長さ0は許可します。非NULLポインタの有効性、解放済みhandle、同時アクセスは安全に検査できません。同じcontextへの呼出しは直列・再入なしとし、destroyは一度だけ行います。入力中のNULや時間0は、表現として正しければ共通runtimeの失敗値になります。

## 失敗と所有

| ABI状態番号 | 意味 |
| --- | --- |
| 0 / 1 / 2 | 操作成功 / 予約待ち / 結果確定 |
| 10 / 11 / 12 | 未対応版 / 形状不正 / 呼出し状態不正 |
| 13 / 14 / 15 | 幅・範囲不正 / 不正UTF-8 / 要求コピーの確保失敗 |
| 16 / 17 / 18 | 設定不正 / 捕捉した内部panic / target不一致 |

10以上は接続契約または物理資源の失敗で、子プロセスのOutcomeとは別です。呼出し元は元の操作位置と番号を使って停止を診断します。生成経路の診断loweringは後続です。不正状態で起動をやり直さず、エラー出力へ入力本文や環境を転記しません。

捕捉可能なpanicはC側へunwindさせず、そのcontextをPoisonedにしてdestroy以外を拒否します。Rustのpanic hookは変更しません。allocator abort・プロセス強制終了まで回復を保証せず、contextのBox確保もabortの可能性があります。

要求文字列・引数・stdinのコピーと引数view用領域をprepare内で確保します。引数の私有String領域は変更せず、内部参照はOwnedRequestの借用中だけ公開します。移動・元領域の破棄後も参照が維持されることをRust/C両方で検証します。execute中に引数viewを再確保しません。

## 検証範囲

C側の独立したサイズ・offset・Report期待値で、要求コピー、出自、任意バイト、非0終了、位置指定の反復読取り、未知版・不正形状・UTF-8・Span、二重prepare/execute、予約失敗、未登録・起動失敗、出力上限、時間切れ、結果解放後の呼出しを確認します。Windowsの取消しで進行が不明なら、以後のConnectionDisabledも確認します。Rust側ではpanic後の停止と破棄、内部参照の寿命を追加検証します。

観測APIの読取りで実行を変えない検証は、共通runtimeの試験も継続します。全失敗分類の番号は既存codec試験で網羅し、OSで確実に起こせない障害をC試験で再現できたとは扱いません。無効な非NULLポインタやデータ競合を起こす試験は行いません。

次は生成Cの要求変換・配列予約・Report変換・起動設定です。公開配布向けのarchive名・ABI互換性保証・manifest・Linux監督の配置は後続で確定します。
