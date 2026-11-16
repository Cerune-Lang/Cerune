# 子プロセスruntimeの配布候補

[English](README.en.md)

生成C・LLVM・QBE・Nativeのリンクに使う静的ライブラリと、Linuxで子プロセスを監督する実行ファイルをまとめます。対象は `x86_64-pc-windows-msvc` と `x86_64-unknown-linux-gnu` です。macOS・ARM64・muslには対応しません。WAT参照host、コンパイラ本体、利用者の実行ファイルはこのアーカイブに含みません。

## 内容と使い方

展開先には次のファイルがあります。

| ファイル | 用途 |
| --- | --- |
| `manifest.json` | 形式版、Cerune版、ソースcommitと未コミット変更の有無、target、ABI版、Rust/Cargo版、ファイルごとのバイト数・SHA-256 |
| `lib/process-runtime.lib` または `lib/process-runtime.a` | リンク時に指定する静的runtime |
| `include/cerune_process_v1.h` | C ABI v1。Reportは32個の64-bit word |
| `bin/cerune-process-supervisor` | Linuxのみ。実行時の設定JSONで絶対パスを指定する監督 |
| `Cargo.lock`、`licenses/` | 固定した依存版、Cerune・依存crate・Rust標準ライブラリのライセンス文書 |
| `README.md`、`README.en.md` | この説明 |

Nativeでは `build native` の `--process-runtime` にライブラリを明示します。

```text
cerune build native program.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang --process-runtime /opt/cerune-runtime/lib/process-runtime.a -o program-build
```

WindowsではtargetとライブラリをWindows用に置き換えます。生成C・LLVM・QBEを外部ツールでリンクする場合は、同じライブラリと `manifest.json` の `systemLinkArgs` が必要です。Native buildは既存の検査でObjectのtargetも確認します。

Linuxでは、実行時に渡す `cerune-process-v1` 設定JSONの `linux_supervisor` に、展開した `bin/cerune-process-supervisor` の絶対パスを指定します。自動探索はしません。`--version` はCerune版と制御プロトコル版を表示します。それ以外の起動はruntimeが渡す制御用ファイル記述子を必要とします。

```text
program --cerune-runtime-v1 --process-config process.json -- program-arguments
```

登録対象・環境変数・設定JSONはアーカイブに含めません。静的runtimeはリンク先へ組み込まれますが、Linux監督は別ファイルです。`release native` が監督を自動同梱するように変えたものではありません。

## 作成と検証

リポジトリ内でNode.js 24、Rust/Cargo、Git、tarを使います。ライセンス収録のため `rustup component add rust-docs` も必要です。targetと実行中のx86_64 OSが一致しない場合は作成を拒否します。出力先は新規ディレクトリに限ります。

```text
node scripts/process-runtime-bundle.cjs build x86_64-unknown-linux-gnu target/process-runtime-linux
node scripts/process-runtime-bundle.cjs verify x86_64-unknown-linux-gnu target/process-runtime-linux/bundle
```

`build` は `--locked --release` でビルドし、`bundle/` と `cerune-process-runtime-<version>-<target>.tar.gz`、その `.sha256`、`logs/commands.json` を作成します。ログには実行コマンドと成否・標準出力・標準エラーを保存しますが、アーカイブには含めません。未コミット変更がある開発ビルドは `source.dirty: true` と記録します。同じ版番号でもcommitとSHA-256を確認してください。

`verify` は内容を実行せず、指定target・ABI・固定のリンク依存・必須ファイル・ファイル長・SHA-256・Linuxの実行権限を確認します。欠落、変更、重複、想定外ファイル、リンクを拒否します。ハッシュは破損検出であり、配布者の署名やABIの正しさの証明ではありません。

ローカルで作成したアーカイブには、次の実行試験を使えます。検証前に展開するため、出所不明のアーカイブを入力しないでください。

```text
node scripts/process-runtime-smoke.cjs x86_64-unknown-linux-gnu /usr/bin/clang target/process-runtime-linux/cerune-process-runtime-0.1.0-x86_64-unknown-linux-gnu.tar.gz /tmp/cerune-runtime-smoke
```

Windows/Linux CIはcheckoutの外へ展開し、同梱ヘッダ・ライブラリだけでC ABI試験をリンクします。PATHを空にして実行し、任意バイト、出自、非0終了、失敗、時間切れ、回収を確認します。さらに展開したruntimeで既存の `registered.ceru` をNative build/releaseし、移動後の出力と依存記録を検証します。

## 公開前に残る作業

今回のアーカイブはCIでも作成する配布候補です。GitHub Releaseへの自動添付、公開用のクリーンcommit確認、コンパイラ配布物そのものの試験（Issue #272）は後続です。タグ作成・公開は行いません。

WindowsはシステムDLL、Linuxはglibc等のシステムライブラリが必要です。`systemLinkArgs` とビルド時のOS/glibcを記録しますが、全OS版での実行や古いglibcとの互換性を保証しません。公式配布には対応OSの最低版を確定し、その環境でも試験する必要があります。ライセンス一覧はCargoが解決した依存を収録し、すべてが最終バイナリへリンクされるという意味ではありません。
