# 子プロセスruntimeの配布

[English](README.en.md)

生成C・LLVM・QBE・Nativeのリンクに使う静的ライブラリと、Linuxで子プロセスを監督する実行ファイルを配布します。対象はx86_64 Windows MSVC／Linux GNUです。macOS・ARM64・muslへの対応は今後の作業です。コンパイラ本体、WAT参照host、利用者の実行ファイルには、それぞれの配布手順を使います。

## ファイルと使い方

```text
静的runtime → リンク → 利用者の実行ファイル
                             ↓ 実行時の設定JSON
                  登録対象・環境変数・上限を指定
                             ↓ Linuxの場合
                  cerune-process-supervisor
```

展開先には次のファイルがあります。

| ファイル | 用途 |
| --- | --- |
| `manifest.json` | 形式版、Cerune版、ソースcommitと未コミット変更の有無、target、ABI版、Rust/Cargo版、ビルドOS、ファイルごとのバイト数・SHA-256 |
| `lib/process-runtime.lib` または `lib/process-runtime.a` | リンク時に指定する静的runtime |
| `include/cerune_process_v1.h` | C ABI v1。Reportは32個の64-bit word |
| `bin/cerune-process-supervisor` | Linuxのみ。設定JSONで絶対パスを指定する監督 |
| `Cargo.lock`、`licenses/` | 固定した依存版、Cerune・依存crate・Rust標準ライブラリのライセンス文書 |
| `README.md`、`README.en.md` | この説明 |

Nativeでは `build native` の `--process-runtime` にライブラリを明示します。

```text
cerune build native program.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang --process-runtime /opt/cerune-runtime/lib/process-runtime.a -o program-build
```

WindowsではtargetとライブラリをWindows用に置き換えます。生成C・LLVM・QBEを外部ツールでリンクする場合は、同じライブラリと `manifest.json` の `systemLinkArgs` が必要です。Native buildはObjectのtargetも確認します。

Linuxでは、実行時に渡す `cerune-process-v1` 設定JSONの `linux_supervisor` に、展開した監督の絶対パスを指定します。`--version` はCerune版と制御プロトコル版を表示します。通常の監督起動にはruntimeが渡す制御用ファイル記述子が必要です。

```text
program --cerune-runtime-v1 --process-config process.json -- program-arguments
```

登録対象・環境変数・設定JSONは利用者が実行環境に合わせて用意します。静的runtimeはリンク先へ組み込まれます。Linux監督は実行時に指定したファイルを使うため、`release native` の利用者も監督の配置と設定を用意してください。

## 作成と検証

Node.js 24、Rust/Cargo、Git、tarを使います。ライセンス収録のため `rustup component add rust-docs` も必要です。targetと実行中のx86_64 OSが一致する環境で、新規ディレクトリを出力先に指定します。

```text
node scripts/process-runtime-bundle.cjs build x86_64-unknown-linux-gnu target/process-runtime-linux
node scripts/process-runtime-bundle.cjs verify x86_64-unknown-linux-gnu target/process-runtime-linux/bundle
```

| 操作 | 結果・確認項目 |
| --- | --- |
| `build` | `--locked --release` でビルドし、`bundle/`、版・target付き `.tar.gz`、その `.sha256` を作成 |
| ビルド記録 | `logs/commands.json` とログにコマンド・成否・標準出力・標準エラーを保存。CIの観測用artifactとして保管 |
| `verify` | ファイルを読み、指定target・ABI・固定のリンク依存・必須ファイル・長さ・SHA-256・Linuxの実行権限を照合 |
| 不一致の検出 | 欠落、変更、重複、想定外ファイル、リンクをエラーにする |

開発ビルドは未コミット変更を `source.dirty: true` と記録します。同じ版番号でもcommitとSHA-256を確認してください。ハッシュは破損検出に使います。配布元の確認には信頼するReleaseを使い、ABIの動作は実行試験で確かめます。ライセンス一覧はCargoが解決した依存を収録するため、最終バイナリにリンクされたcrateより広い範囲を含みます。

作成したアーカイブの実行試験は次のコマンドで行います。展開後に内容を検証するので、自分で作成したものなど、信頼できるアーカイブを入力してください。

```text
node scripts/process-runtime-smoke.cjs x86_64-unknown-linux-gnu /usr/bin/clang target/process-runtime-linux/cerune-process-runtime-0.1.0-x86_64-unknown-linux-gnu.tar.gz /tmp/cerune-runtime-smoke
```

試験はcheckoutの外へ展開し、同梱ヘッダ・ライブラリだけでC ABI試験をリンクします。PATHを空にして実行し、任意バイト、出自、非0終了、失敗、時間切れ、回収を確認します。Release workflowでは、展開したruntimeで既存の `registered.ceru` をNative build/releaseし、移動後の出力と依存記録も検証します。

## Releaseへ渡す手順

```text
同じcommitのソース
  ├─ コンパイラ5対象 → 展開・起動試験
  └─ runtime 2対象 → 展開・C ABI試験 → exampleのbuild/release
                       ↓
             版・commit・ビルドOS・SHA-256を照合
                       ↓
             2対象を再展開してmanifestと照合
                       ↓
             cerune-process-runtime.json を作成
                       ↓
             全試験成功 → タグ実行時にReleaseへ添付
```

| 実行 | 出力 |
| --- | --- |
| PR | アーカイブ・検証記録をActions artifactに保存 |
| 配布タグ | 試験済みファイルをGitHub Releaseへ添付 |

`process-runtime-release.cjs stage` はコンパイラと同じ版・commit、未コミット変更なし、下表のビルドOSを確認します。`catalog` は両対象のアーカイブ・チェックサム・個別JSONの6ファイルを受け取り、再展開したmanifestまで照合します。ログの混入や片方の欠落も検出します。

| target | 作成・実行試験に使うrunner | 記録するシステムruntime |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 22.04 | glibc 2.35 |
| `x86_64-pc-windows-msvc` | Windows Server 2022 | Windows 10.0.20348系 |

Releaseには各targetの `cerune-process-runtime-<version>-<target>.tar.gz`、`.tar.gz.sha256`、`.json` と、一覧の `cerune-process-runtime.json` を添付します。個別JSONにはmanifestと試験runner、一覧には版・commit・ハッシュ・ABI・リンク引数・ビルド時のシステムruntimeを記録します。ビルドログと実行記録はActionsの観測用artifactから確認できます。

実行にはWindowsのシステムDLL、Linuxのglibc等が必要です。上表は配布物を作成して試験するOSです。Server 2022製の配布候補はWindows 11 Home 10.0.26100でも実行試験が成功しました（[試験記録](../release-readiness.ja.md)）。ほかのWindows版や古いOSの対応可否は、対象OSでの実行試験を追加して確定します。タグ作成と公開版の決定は、このworkflowの実装後に行うリリース作業です。
