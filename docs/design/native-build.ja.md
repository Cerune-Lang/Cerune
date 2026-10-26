# Native build初版

[English](native-build.en.md) · [Issue #81](https://github.com/Cerune-Lang/Cerune/issues/81)

## 完成成果物と範囲

`build native`は、Ceruneの自前ObjectをClang driverでリンクし、実行ファイルと生成・リンク記録を新規ディレクトリへ保存します。プログラムは実行しません。`emit-obj`は引き続きObjectだけを生成します。IR/VM Image、release、自作リンカは今回の対象外です。

```sh
cerune build native app.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o app-build
```

| 選択 | 契約 |
| --- | --- |
| `native`・`--target`・`--linker`・`-o` | 必須。OSからtargetを補わず、PATHからClangを探索しない。linkerは実ファイルへの絶対／相対パス |
| target | `x86_64-unknown-linux-gnu` → `program`（ELF64 PIE）、`x86_64-pc-windows-msvc` → `program.exe`（PE32+） |
| `--ssa` | 明示的なSSA変換。基準MIRも残す。暗黙の最適化passはない |
| `--include-sources` | 正確なソース本文を`sources.json`へ追加。既定でも出自のファイル名・生成コード中のリテラルは記録される |
| `--link-timeout-seconds` | version照会とリンクそれぞれの上限。既定60秒、1〜3600 |
| heap・argument上限 | 既存emit-objと同じ4オプション。生成物とmanifestへ固定 |

出力先の親ディレクトリは事前に用意します。既存の出力先（ファイル・ディレクトリ・リンク）は拒否し、上書きしません。任意のlinker引数、sysroot指定、追加Object／ライブラリ入力は初版では提供しません。targetのSDK・CRT・リンカが使えるClang環境を指定してください。

## 責務と観測

`native_build::generate`は共通frontend→HIR→MIR→LIR→型付きNativeProgram→Objectと観測資料を構築し、外部ツールを起動しません。ASM表示とObjectは同じNativeProgramから生成します。`native_build::link::Request`は入力Object・target・driver・引数・作業ディレクトリ・時間上限を保持し、Clang呼出しと結果確認を担当します。将来の別adapterや自作リンカでObject生成を置き換える必要はありません。汎用plugin ABIは設けません。

| 保存物 | 内容 |
| --- | --- |
| `manifest.json` | schema `cerune-native-build-v1`、building/success/failed、Cerune版、target、予算、SSA選択、成果物一覧。成功は最終確認後だけ記録 |
| `program.ceir`・`program.mir.txt` | HIRと基準の非SSA MIR |
| `program.ssa*`（選択時） | SSA、その出自map、SSAから戻したMIRとmap |
| `program.lir.txt`・`program.machine.txt` | 実際にObjectへ渡すLIR・型付き命令 |
| `program.origins.s`・`program.o` | 同じ命令から生成した注釈付きASMと自前Object。外部アセンブラは呼ばない |
| `program.origins.json`・`program.object-map.txt` | OriginId→NodeId/SourceId/Span・MIR/LIR、および命令・symbol・relocationのObject内位置 |
| `source-index.json`・任意の`sources.json` | ファイルID・名前・ユーザー／標準モジュールの出自・UTF-8サイズ、選択時は本文も保持 |
| `runtime.json` | 外部参照symbol、targetのruntime条件。Cerune runtimeはObject内に生成 |
| `link-plan.json` | driverの解決済みパス、明示target、入力、引数配列、cwd、stdinと環境の方針 |
| `version.*`・`link.*` | stdout/stderrの生ログと終了コード／終了理由・経過時間。Clangの`-v`で実際の下位リンカ選択・引数も記録 |
| `program.link.map`・`link-result.json` | 下位リンカのmapとリンク全体の結果 |

Objectの位置はセクション内offsetであり、リンク後アドレスとは区別します。Object側の出自symbolと下位リンカのmapを残しますが、統一されたリンク後アドレス照会APIやデバッガ情報を提供するものではありません。機械命令・Object mapのテキストは観測形式であり、安定した再入力形式ではありません。

## リンクと実行条件

Linuxは明示target・`-pie`・`--no-undefined`・map・`-lm`を指定します。Windowsは明示MSVC targetとmapを指定します。Clangのversionを照会し、Clang driverであることを確認します。shellコマンド文字列は組み立てず、引数配列で起動します。環境はtoolchainへ引き継ぎます。SDK・下位リンカ・ライブラリの選択はClangに委ね、verboseログに残します。ログは環境全体の記録ではなく、再現可能ビルドや別環境での動作保証ではありません。

Clangが終了0でも、選択targetのx86-64実行形式ヘッダと空でないmapが確認できなければ失敗です。これは成果物の最低限の形式確認で、任意の実行ファイルの安全性・意味の証明ではありません。build成功は、このホスト上で実行できることを意味しません。

Windowsは選択toolchainのMSVC/UCRT・Win32、Linuxは選択toolchainのglibc・libm・ELF loaderに依存します。OSライブラリや再頒布runtimeを同梱せず、最低OS／glibc版も初版では確約しません。[Native release初版](native-release.ja.md)で直接のruntime要求と配布構成を別途記録します。

## 失敗と検証

オプション・コンパイル・Object生成の失敗は、外部ツールを起動せず出力先も作りません。出力作成後のI/O失敗は不完全なディレクトリを残す場合があります。driver起動失敗、version不一致、リンク非0終了、timeout、成果物不正を区別し、リンク段階の失敗はObject・ログ・failed manifestを残します。途中中断や記録の書込み失敗ではbuildingのまま残る場合があり、success以外は完成品として扱いません。再実行には新しい出力先を指定します。

stdinはnullです。timeout時は専用プロセスグループ（Unix）／driverの子孫（Windows）を終了させ、driverを回収します。toolchainを隔離するsandboxではなく、明示したローカルツールを信頼します。

Windows/Linuxで、通常MIR／SSAのbuild→実行、既存emitとの一致、引数・予算・失敗理由と出自を比較します。異常系はdriverの失敗・timeout・成功を装う未生成出力、構文／型エラーと既存出力保護を検証します。[実行example](../../examples/build_native/README.md)を参照してください。

buildは `--call-depth-limit N`（既定128・範囲1〜128）を生成LIR・検査・manifestのcall_depth_limitへ保持します。物理stack／memoryの安全保証ではありません。
