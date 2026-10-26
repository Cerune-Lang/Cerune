# Native release初版

[English](native-release.en.md) · [Issue #81](https://github.com/Cerune-Lang/Cerune/issues/81)

## 操作と範囲

`cerune release native <build-directory> -o <new-directory>`は、成功済みの[Native build](native-build.ja.md)を配布ディレクトリにまとめます。ソースから再生成せず、Clang・実行ファイル・外部検査ツールを起動しません。buildとreleaseは独立した操作です。

| 項目 | 初版の契約 |
| --- | --- |
| 入力 | schema `cerune-native-build-v1`、status `success`のbuildディレクトリ |
| 出力 | 新規ディレクトリ。親は事前に作成。既存先・入力配下への出力は拒否 |
| target・上限・SSA | buildの選択を保持。releaseで変更するオプションはない |
| runtime | 対象OSのシステムruntimeを要求。ライブラリ・SDK・コンパイラは同梱しない |
| 本文 | buildの`--include-sources`選択を保持。releaseで追加・削除しない |
| 最適化 | 再コンパイル・再リンク・strip・最適化を行わない |
| 対象外 | IR/VM Image、圧縮archive、installer、署名、自動公開、依存ライブラリの同梱・探索 |

配布物の形式はディレクトリです。zip等への梱包は別ツールで行えますが、展開後の内容とLinux実行権限を保持してください。処理系自体のGitHub Releaseとは別の機能です。

## 保存内容

| パス | 内容 |
| --- | --- |
| `build/` | 元のbuildの全規定ファイル。実行ファイル、非最適化HIR/MIR、選択したSSA/LIR、型付き命令・Object、出自・リンク記録を同じバイト列で保持 |
| `release.json` | schema `cerune-native-release-v1`、status、target、入口の相対パス、本文同梱の有無、ファイル別サイズ・SHA-256。全書込みの最後に作成 |
| `runtime-requirements.json` | 実行ファイルを非実行で解析した直接のruntime要求 |
| `SHA256SUMS` | payloadとrelease.jsonのSHA-256。自分自身は含まない |
| `README.md`・`README.en.md` | target、実行方法、依存条件、観測記録・ハッシュの意味 |
| `CERUNE-LICENSE-MIT`・`CERUNE-LICENSE-APACHE` | Ceruneのライセンス本文。利用者のプログラムのライセンスを設定する操作ではない |

実行入口は`build/program`（Linux）または`build/program.exe`（Windows）です。buildの絶対パス・下位リンカの選択は履歴としてそのまま残し、新しい場所でリンクを再実行する手順とは扱いません。本文省略時もリテラル・ファイル名・toolchainパスは含まれ得ます。

## runtime要求の確認範囲

| target | 実行ファイルから読む内容 | 明示的に拒否するもの |
| --- | --- | --- |
| Linux x86-64 | ELF64 PIE、入口、interpreter、DT_NEEDED、GNU symbol version要求（ライブラリ・版・flags） | 別architecture・非PIE・入口なし・欠落／不完全な版要求、RPATH/RUNPATH |
| Windows x64 | PE32+、入口、直接import DLLと名前＋hint／ordinal、headerのOS・subsystem版 | 別architecture・DLL・入口なし・delay import |

ELF/PEの読取りはGoblinを使用します（[ELF API](https://docs.rs/goblin/0.10.7/goblin/elf/struct.Elf.html)、[PE API](https://docs.rs/goblin/0.10.7/goblin/pe/struct.PE.html)）。OSからtargetを推定せず、別OSの成果物もバイト列として扱います。ELF解析ライブラリ内部の32-bit型依存は、Ceruneの32-bit対応を意味しません。

要求一覧はライブラリのインストール確認ではありません。推移的な依存先、動的ロード、対象環境での探索結果は解決しません。GNU版要求やPE headerの版を、そのまま「動作保証する最低OS」とはしません。`deployment_verified: false`と`transitive_dependencies_resolved: false`を保存します。対象環境への配置・実行確認は配布者が別途行います。

## 入力確認と失敗

build manifestの形式・成功状態・target・選択オプションと規定ファイル一覧、リンク結果・終了コード・target、runtime方針を照合します。未知の成果物名、欠落・余分なファイル、symlink/reparse point、JSON不正、実行形式不正を拒否します。自由なパスをmanifestから読み出してコピーしません。

読取り上限は1ファイル128 MiB、全入力512 MiB、JSONごと16 MiBです。入力を読み取って確認してから出力先を作ります。入力確認失敗では出力を作りません。後段の書込み失敗は部分的なディレクトリを残し得ます。有効なsuccessのrelease.jsonと全ファイル・ハッシュが揃わないものは配布完了としません。再実行には新規出力先を指定します。

入力は処理中に他の操作で変更しないでください。これは敵対的な同時書換えを隔離する仕組みではありません。ハッシュは配布時点の内容確認であり、署名や元buildの真正性・意味対応の証明ではありません。再入力可能なIR形式や一般的な任意実行ファイル検証器も追加しません。

## 検証と次段階

Windows/Linuxでbuild→release→別ディレクトリへ移動→実行を試し、IR/MIR/VMと比較済みのexampleの期待値を確認します。通常MIR/SSA・本文あり／なし、全ファイルのバイト一致・SHA-256、直接依存物、記録の不整合・欠落・既存先保護を検証します。[example](../../examples/build_native/README.md)にコマンドを載せています。

対象システムruntimeを要求する配布初版までを本変更の区切りとします。IR/VM Image、より広い配布互換性・依存物同梱は別作業です。次の言語機能は[ロードマップ](language-roadmap.ja.md)の再帰です。
releaseはbuildのcall_depth_limitを変更せず引き継ぎます。旧buildで未記録なら既定値を補わず、release.jsonではnull（不明）とします。記録値が1〜128の整数でなければ拒否します。
