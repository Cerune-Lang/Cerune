# 移行前のNative命令選択

[English](README.en.md)

Cerune eef765fのLIR命令選択、数値変換、失敗診断、標準入力・引数処理をテスト専用に凍結しています。新しい型付きProgram生成器と同じLIRを与え、旧エンコーダが作るObject全体を比較します。

定数・文字列・確保管理・通常出力の基準は既存の[旧runtime](../native_runtime_reference/README.md)と[旧定数emitter](../native_data_reference.rs)へ戻して比較します。Programから表示したASMを旧parserで読むだけの比較にはしません。

本番ビルドには含めず、新機能を追加しません。現在のASM表示fixtureとは役割が異なり、移行後の出力に合わせて書き換えません。

比較対象は移行完了時（fe97b30）の135例を `examples.txt` に固定しています。移行後の新機能は各経路の実行テストで検証し、この旧実装へは追加しません。
