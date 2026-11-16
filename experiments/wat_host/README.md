# WATの実OS接続を試す参照host

[English](README.en.md)

Ceruneが生成したWAT／WasmをWindows/Linuxのファイル・標準入出力・引数・子プロセスへ接続する、独立した参照実装です。WATは引き続きemit-onlyです。Cerune CLIのrun／build／releaseへ新しい経路は追加しません。

## 実行

リポジトリ直下から実行します。保存先は毎回新規ディレクトリにしてください。

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files.wat
mkdir target/wat-files-data
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/files.wat --file-root target/wat-files-data --
~~~

出力はtrueと16です。sample.txtへ日本語・NUL・CR/LF・結合文字を含む16バイトを保存し、そのまま読み戻します。再実行はalready-existsで停止し、上書きしません。生成済み.wasmも同じ引数位置へ指定できます。

| 指定 | 意味 |
| --- | --- |
| 最初の引数 | 読み込む.watまたは.wasm |
| --file-root directory | 実行前に開く作業ディレクトリ。省略時は未設定 |
| --process-target triple | 子プロセスimportを使う場合は必須。Windows MSVC / Linux GNUのx86-64を明示 |
| --process-config json | 既存のcerune-process-v1接続JSON。target指定が必要。省略時はUnconfigured |
| -- | host設定の終端。引数がなくても必須 |
| --以降 | std:envへ渡すプログラム引数。専用マーカーも通常の引数として保持 |

設定の重複・未知の設定・値や区切りの欠落は起動失敗です。cwd・環境変数からrootを補いません。明示した相対パスは起動時のcwdから解決します。Native生成物の--cerune-runtime-v1形式とは別のhost CLIです。設定・root接続・モジュール読込みは実行開始前に検証し、プログラムの引数上限は生成WATが検査します。

## 子プロセス

次はLinux x86-64で未接続のexampleを実行するコマンドです。Windowsではtargetを x86_64-pc-windows-msvc に置き換えます。実行中のOSからtargetや接続先を補いません。

~~~sh
cargo run --quiet -- emit-wat examples/process_execution/unconfigured.ceru -o target/process.wat
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/process.wat --process-target x86_64-unknown-linux-gnu --
~~~

[registered.ceru](../../examples/process_execution/README.md)を同様に生成し、--process-config process.json を追加すると登録した子を起動できます。JSONは[既存の契約](../../docs/design/subprocess-cli.ja.md)と同じです。Linux監督の絶対パスもJSONで明示します。空・不正・1 MiB超のJSONや異なるtargetは実行前に拒否し、本文・環境値を診断に載せません。

各Wasmi Storeが1個の子プロセス接続を所有します。要求・結果のreleaseでは接続の無効化を解除せず、正常終了・trap・instantiate失敗時には既存runtimeの破棄処理へ一度渡します。時間超過後に転送量や取消通知を確認できなければ、回収済みでも次の起動がConnectionDisabledになる場合があります。成功へ置き換えたり、暗黙に接続し直したりしません。

対応はx86-64 Windows MSVC / Linux GNUです。他のhostビルドでは子プロセス指定を明示的に拒否します。Wasmのメモリは公開しません。runtime・Linux監督の公式配布は別の作業です。

## 構成と対象

| 部分 | 担当 |
| --- | --- |
| Wasmi 2.0.0 | Wasmの検証・実行 |
| src/main.rs | 版付きimport、引数snapshot、標準入出力、mainを一度だけ呼ぶ起動処理 |
| src/process.rs | 版付きの子プロセス転送、要求・結果の保持。実行・失敗選択・回収は共通runtimeへ委任 |
| src/files.rs | 最大8バイトの転送とinstance内handleの管理 |
| Ceruneのruntime::files::Directory | OSの作業範囲、通常ファイル検査、排他的作成、close |
| 生成WAT | 共通IRに由来する検査・所有・再試行・UTF-8・転送数・file-v1診断 |

[Wasmi](https://docs.rs/wasmi/2.0.0/wasmi/)はこのcrateのCargo.lockで固定し、Cerune本体の依存には追加しません。cerune-langの相対依存は同じリポジトリ内です。標準出力の浮動小数点整形は実行器と同じ純粋関数を使います。ファイル転送では本文全体を複製しません。子プロセス転送ではprepare前に要求本文を所有します。どちらも内部memoryへアクセスしません。公開exportはmainだけを受け付けます。

Windowsは既存のディレクトリhandle相対接続、Linuxは既存のopenat／procfs接続を使います。rootの移動後も同じディレクトリを保持し、symlink／reparse pointや通常ファイル以外を拒否します。実OS接続の条件は[共通契約](../../docs/design/file-io.ja.md)を参照してください。

標準入力は既存の生バイト接続です。Windowsの直接コンソール入力は未対応で、pipe／fileを使います。出力は複製したOS handleへ直接書き、受理数と失敗を返します。入力本文の正規化はしません。

## 停止・観測

ファイルの意味上の失敗は生成WATがNodeId／SourceId／Span付きfile-v1を配送し、unreachableで停止します。host・エンジンの失敗は別のエラーとして停止し、EOF・成功・depth-limitへ置き換えません。再入・mainの再実行・停止後の再開は行いません。

closeは失敗してもhandleを消費し、再試行しません。エンジン停止などで残ったhandleは終了時に一度だけ回収します。途中の保存内容・先行出力を取り消しません。この回収はソースのclose診断を作り直す操作ではありません。

生成WATと既存の出自付き診断を観測できます。応答注入はtests側の実行制御です。この参照hostには新たな観測API・内部memory公開・安定した埋込みAPIを設けません。任意の未信頼Wasmを実行するためのサンドボックスや汎用WASIランナーではありません。WAT libraryの公開関数呼出しは別の実験です。

## 検証

~~~sh
cargo fmt --manifest-path experiments/wat_host/Cargo.toml --check
cargo clippy --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host --all-targets -- -D warnings
cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host
~~~

Windows/Linux CIで必須実行します。4実行器との出力・file-v1全文・残存ファイル比較、空・上限ちょうど・超過・不正UTF-8・使用量上限、評価順・短絡・再帰・コピーを確認します。標準入力・引数・数値／複合値出力の併用、実行中のroot移動、リンク拒否も検証します。

短い転送・割込み・不正応答・close消費・全256バイト値は、転送adapterへの明示した試験接続で確認します。実OSでそれらの失敗を自然発生させたという意味ではありません。生成WAT側の応答処理は既存の[メモリ接続テスト](../../tests/file_io/wat.rs)と[host契約](../../docs/design/wat-file-host.ja.md)でも比較します。

[子プロセスABI試験](../subprocess/wat_abi/README.md)、[生成WAT](tests/process_generated.rs)、[CLI比較](tests/process_cli.rs)も実行します。既存example・標準入出力・ファイル保存・引数の併用、時間超過、後続処理と接続の無効化を確認します。Linuxの回収失敗は試験用監督で通知を遅らせて再現します。両OSのReport注入試験はenum変換の検証であり、OSで自然発生した回収失敗とは扱いません。

Linuxでは次の指定で、通常はignoreされる実子・監督試験も含めます。Windows/Linux CIではこれらを必須にしています。

~~~sh
cargo build --locked --example subprocess_linux_supervisor --example subprocess_linux_backend
CERUNE_TEST_PROCESS_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_supervisor" CERUNE_TEST_PROCESS_FAULT_SUPERVISOR="$PWD/target/debug/examples/subprocess_linux_backend" cargo test --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- --include-ignored
~~~
