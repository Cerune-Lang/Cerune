# CI用QBE 1.3の予備ファイル

[English](README.en.md)

QBE公式サイトへ接続できないときも、同じ版でCIを実行するための配布archiveです。実装・圧縮内容は変更していません。Ceruneの配布版や改変版QBEではありません。

| 項目 | 値 |
| --- | --- |
| 元の配布先 | https://c9x.me/compile/release/qbe-1.3.tar.xz |
| 公式リリース情報 | https://c9x.me/compile/releases.html |
| 上流commit | `c0818978acec60ebb6167fade60fb7012cbf20ca` |
| SHA-256 | `d587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320` |
| ライセンス | 同梱archive内のMIT許諾文を[LICENSE](LICENSE)にも変更せず保存 |

[取得スクリプト](../../../scripts/fetch-qbe.sh)をLinuxとWindows/MSYS2の4か所で共有します。公式HTTPSへ接続し、接続10秒・取得全体30秒の上限を設けます。取得失敗時はこの予備を使い、使用した取得先と照合値をログへ残します。公式から取得できてもhashが異なる場合は停止し、予備で成功扱いにしません。既存出力や予備の不一致も拒否します。検証前の一時ファイルは展開せず、失敗時に削除します。

~~~sh
bash scripts/fetch-qbe.sh target/qbe-1.3.tar.xz
# ネットワークを使わず、照合済みの予備を使う確認
bash scripts/fetch-qbe.sh --offline target/qbe-offline.tar.xz
~~~

QBE更新時は、公式の版・commit・SHA-256を確認してarchive、許諾文、この表、スクリプトの固定値を同じ変更で更新します。tarを作り直したものや別版は代用しません。このディレクトリはGitHubの言語集計から除外します。
