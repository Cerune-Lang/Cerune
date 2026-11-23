# Cerune本体と周辺プロジェクトの役割

[English](ecosystem.en.md)

2026-11-24に[Organization](https://github.com/Cerune-Lang)のリポジトリ一覧を確認しました。表は各プロジェクトの担当を示します。後半の実装・文書の調査記録は2026年10月時点のものです。

| 担当 | プロジェクト | 現在地 |
| --- | --- | --- |
| 言語・意味・実行 | [Cerune](https://github.com/Cerune-Lang/Cerune) | 共通IR、実行器、生成経路、所有・停止・観測の契約 |
| 命令符号化・Object生成 | [SASHIMI](https://github.com/Cerune-Lang/SASHIMI) | Ceruneは固定版に型付き命令を渡し、出自に対応するASMとObjectを取得する。[Nativeの設計](native-encoder.ja.md) |
| 観測UI | [Tint-St](https://github.com/Cerune-Lang/Tint-St) | CLI経由の表示・実行とブラウザー用Wasm実験。言語意味論の複製はしない |
| 編集支援 | [Cerune-vscode](https://github.com/Cerune-Lang/Cerune-vscode) | 構文色分け・snippet。処理系やLSPの完成とは区別 |
| 差分検証 | [Parallax](https://github.com/Cerune-Lang/Parallax) | IR/MIR/VM、固定seed、有限の式・分岐・反復・配列の生成、期待値と停止の比較 |
| 安全性研究 | [Fracta](https://github.com/Cerune-Lang/Fracta) | overflow・ゼロ除算・添字・変換の安全な停止を確認。不具合と断定しない |
| 汎用数値計算 | [Quantelia](https://github.com/Cerune-Lang/Quantelia) | f32 Tensorの初期表現。shape整合性はまだ保証しない |
| 機械学習 | [Nerune](https://github.com/Cerune-Lang/Nerune) | Tensorの初期実装。学習機能は将来 |
| リンカ・OS | Symnexa / LiseOS | Objectのリンク／OS上の実行。本体で検討するのは[バイナリファイル操作](https://github.com/Cerune-Lang/Cerune/issues/243)と[ハードウェアI/O](https://github.com/Cerune-Lang/Cerune/issues/244) |
| Web・Nativeアプリ | Tarot | Web/WasmとWindows Nativeで占いを実行し、[公開関数・テストの要望](https://github.com/Cerune-Lang/Cerune/issues/258)を記録 |
| 利用例 | [Cisa](https://github.com/Cerune-Lang/Cisa) / [stelle](https://github.com/Cerune-Lang/stelle) | 仮想CPU／CLI RPGで不足機能を具体化 |
| 説明と入口 | [website](https://github.com/Cerune-Lang/website) / [.github](https://github.com/Cerune-Lang/.github) | 読者向け説明／Organization案内。仕様の正本はCeruneのdocs |

WhitebaseはこのOrganizationの一覧には含まれません。外部の計測・比較環境として扱い、Ceruneや数値ライブラリの必須依存にはしません。

## 実装から確認した範囲

- [Parallax 356f357](https://github.com/Cerune-Lang/Parallax/tree/356f357cb2bf347ef8ec7462e6bdd77410a6e7d7)は正常系だけでなく、終了状態・stdout・runtime-v1のcodeを比較します。停止前の出力や生成テストもあります。一般のNodeId/Span一致、SSA、compiled routeまでの網羅は未完了です。
- [Fracta 4124812](https://github.com/Cerune-Lang/Fracta/tree/41248121ca0cdcaf759e61b8d0972bbe4130f7da)は4種の安全な停止を期待値で分類します。セキュリティ問題の未公開報告は各SECURITY.mdに従います。
- 両プロジェクトは自分のCIでCeruneのmasterをビルドします。Cerune本体のPR CIへの組込みや、版を固定した互換性検証の完了とは異なります。再現には実際のCeruneコミットを記録する必要があります。
- Parallaxの独立したPython期待値計算を残します。Ceruneで生成する範囲を増やしても、同じ処理系の不具合を共有する検証だけにはしません。

本体の[#131](https://github.com/Cerune-Lang/Cerune/issues/131)は生成範囲・経路比較、[#153](https://github.com/Cerune-Lang/Cerune/issues/153)は堅牢化、[#173](https://github.com/Cerune-Lang/Cerune/issues/173)は実例から分かった不足機能を扱います。本体の回帰テストと外部の探索テストは併存させます。

## 説明と実装のずれ

| 場所 | 整理候補 |
| --- | --- |
| Parallax README | 正常系のみ・生成未対応という初期説明を実装に合わせる |
| Fracta README | 最初の1ケースだけでなく現在の4ケースを案内 |
| .github/profile | website・Parallax・Fractaを入口へ追加 |
| stelle README | 標準入力未実装という説明を更新。実装済み行入力とWindows console未対応を区別 |
| Quantelia / Nerune | Tensor実装の重複を整理。設計上の依存方向はNerune→Quantelia→Ceruneで、現行の統合完了とは扱わない |
| 関連URL・日英文書 | 旧所有者URLや混在記事を、それぞれのリポジトリで確認 |
| website | 二次説明の根拠となるCerune版を追跡。公開済みCIと、本文の正確さは別に検査 |

## 本体の次の作業

Ceruneで受け持つ候補と各Issueの完了条件は[本体ロードマップ](language-roadmap.ja.md)にまとめています。関連プロジェクトで結果を報告するときは、使用したCeruneのタグまたはコミットを記録してください。
