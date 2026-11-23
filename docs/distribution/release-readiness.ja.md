# v0.2.0の配布試験記録

[English](release-readiness.en.md)

[Cerune v0.2.0](https://github.com/Cerune-Lang/Cerune/releases/tag/v0.2.0) を2026-11-17に公開しました。公開commitは `481d325` です。 [タグの配布試験](https://github.com/Cerune-Lang/Cerune/actions/runs/44079560770)でコンパイラ5対象とruntime 2対象を検証しました。

収録内容と移行方法は [v0.2.0リリースノート](v0.2.0-notes.ja.md)で確認できます。

## 完了した配布試験

```text
収録するcommitを決定
  → 版を更新
  → PRでコンパイラ5対象・runtime2対象を作成して試験
  → 変更点・互換性・対応OSを確認
  → タグをpush
  → タグのソースから再ビルド・再試験
  → GitHub Releaseへ公開
```

11月24日にReleaseからWindows x64用のコンパイラとruntimeをダウンロードしてSHA-256を照合しました。展開したコンパイラで `print(7);` を実行し、同梱ヘッダと静的ライブラリでリンクしたC ABI試験も成功しました。以下は公開前の試験記録です。

| 対象 | 確認したこと | 根拠 |
| --- | --- | --- |
| コンパイラ5対象 | 同じOS・CPUで展開し、PATHが空でもhelpと `print(7);` が成功 | [#299](https://github.com/Cerune-Lang/Cerune/pull/299)、[#300のRelease試験](https://github.com/Cerune-Lang/Cerune/actions/runs/44068267393) |
| runtime2対象 | 展開後のC ABI試験、実子の入出力・終了・失敗・回収、Native build/releaseと移動後実行 | [#300](https://github.com/Cerune-Lang/Cerune/pull/300) |
| Releaseへ渡すファイル | 版・commit・ビルドOS・SHA-256・展開したmanifestを照合し、両対象の一覧を作成 | #300の同じ試験 |
| Windowsデスクトップ | Server 2022製の実アーカイブをWindows 11 Home 10.0.26100で使用 | 下記の追加試験 |

作成手順とファイルの用途は、[コンパイラ](compiler/README.md)と[runtime](process-runtime/README.md)の説明を参照してください。

## 試験したOS

| 配布物 | 作成・実行試験 | 追加実行試験 |
| --- | --- | --- |
| コンパイラ Windows x64 | Windows Server 2022 | Windows 11 Home 10.0.26100 |
| コンパイラ Linux x64 / ARM64 | Ubuntu 22.04 / 22.04 ARM64 | — |
| コンパイラ macOS Intel / ARM64 | macOS 15 Intel / ARM64 | — |
| runtime Windows x64 MSVC | Windows Server 2022（10.0.20348系） | Windows 11 Home 10.0.26100 |
| runtime Linux x64 GNU | Ubuntu 22.04（glibc 2.35） | — |

この表は実際に試験したOSの一覧です。Windows 10や古いmacOS・glibcについて対応を公表する場合は、その環境で配布物を実行する試験を追加してください。runtimeのmacOS・ARM64・musl対応は今後の作業です。生成物のリンクにはClangなどの指定したツール、実行には対応OSのシステムライブラリが必要です。

## Windows 11での追加試験

[Release試験44068267393](https://github.com/Cerune-Lang/Cerune/actions/runs/44068267393)から、Windows用コンパイラ・runtime・planのartifactをダウンロードしました。PRのheadは `bf74aa3a7909eae28951d0e04403302ff3ede87a` です。runtimeのmanifestには、Actionsが試験したマージcommit `b09797a839f30af370c395ad8b2435e3bdb56d7b` と `dirty: false` が記録されています。

| ファイル | SHA-256 |
| --- | --- |
| `cerune-lang-x86_64-pc-windows-msvc.zip` | `7717bda11fc4d2f643c53e82b330725719277f3e37c00960fda9d4fcc48281ef` |
| `cerune-process-runtime-0.1.0-x86_64-pc-windows-msvc.tar.gz` | `7998e3fca3626cc9f2b472984ce54557134ef15147470991dfe71c8dd254d4e7` |

checkout外へ展開し、既存の `compiler-archive-smoke.cjs` と `process-runtime-smoke.cjs` を実行しました。コンパイラの起動・短いプログラムの実行、同梱ヘッダとライブラリを使ったC ABI試験が成功しています。

さらに、#300統合後のローカルCeruneを使い、ダウンロードしたruntimeを `CERUNE_TEST_PROCESS_RUNTIME` に指定して次のテストを実行しました。

```text
cargo test --locked --test native_program process_runtime_build_release -- --include-ignored
```

`registered.ceru` の通常／SSA経由のNative build/release、依存内容の記録、移動後の実行が成功しました。コンパイラ・runtimeの公開版を変更した際は、その版のアーカイブで再試験します。Actions artifactには保存期限があるため、この記録にはrun番号・commit・ハッシュも残しています。

## 公開に使った手順

| 順番 | 作業 | 完了条件 |
| --- | --- | --- |
| 1 | 収録PRを統合 | 対象commitの通常CI・CodeQL・Release試験が成功 |
| 2 | 版を更新 | 本体・実験用lockfile・仕様文書を0.2.0へ更新済み。公開済み0.1.0への案内は維持 |
| 3 | 日英のリリースノートを作成 | v0.1.0からの変更点、import名などの互換性変更、対応OS、必要ツール、未対応項目を説明 |
| 4 | 更新した版で配布試験 | コンパイラ5対象・runtime2対象・一覧の版とcommitが一致し、実行試験が成功 |
| 5 | 公開内容を確認してタグ作成 | 収録commitと版を確認し、タグから再ビルド・再試験して公開 |

IR/VM Image、最適化、GPU、GUI等は[ロードマップ](../design/language-roadmap.ja.md)で継続します。CIの必須チェックを全経路へ揃える作業は [#262](https://github.com/Cerune-Lang/Cerune/issues/262) に残っています。公開前には必須指定の有無にかかわらず、対象commitの全CI結果を確認してください。
