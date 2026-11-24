# Kで短絡評価を確かめる

[English](README.en.md)

[分岐例](../lean/branch.ceru)の関数 `choose(value: u8)` を全256入力で実行します。例えば128では `value < 128` が偽になり、右辺の加算を飛ばして130を返します。254も右辺を飛ばしますが、選んだelse側の `value + 2` で桁あふれになります。

| 入力 | 選ぶ側 | 結果 |
| --- | --- | --- |
| 0〜126 | then | 入力 + 1 |
| 127〜253 | else | 入力 + 2 |
| 254・255 | else | `integer-overflow`。else側の加算のNodeIdとSpanを記録 |

## 比較するもの

1. [変換器](main.rs)が完成済みHIRから引数、定数、加算、比較、短絡評価、分岐、復帰をKの項に写します。位置は同じHIRから取得します。対象は `choose` の本体です。
2. [K規則](choose.k)が引数を0から255まで変えて、その項を評価します。二項演算は左から評価し、`andThen` は左辺が真のときだけ右辺へ進みます。桁あふれでは残っている計算を打ち切り、加算の位置を記録します。
3. [比較プログラム](verify.py)が各入力の戻り値または停止理由・位置を、Leanで証明した[仕様関数](../lean/BranchProperties.lean)の実行結果と照合します。HIR・MIR・SSA・VMでは同じ関数を呼び、標準出力と `trace-v1` の分岐・復帰・停止位置をKと照合します。
4. 比較プログラムは短絡規則のコピーを変更し、左辺が偽でも右辺を評価させます。128で右辺の `value + 128` が桁あふれになることを確認します。

## 再現

LinuxでK 7.1.337、Lean 4.34.1、プロジェクトのRustツールチェーン、Python 3.10以降を使用します。リポジトリのルートで実行してください。`CERUNE_TEST_LEAN` にはLeanの実行ファイルを指定できます。

```sh
python3 experiments/k/verify.py
```

実行ごとに `target/k-choose-*` に新しいディレクトリを作り、HIRを写した項、Kの最終状態、Leanの出力表、4実行器のJSON Linesを保存します。通常のコンパイラ利用にはRustでビルドしたCeruneを使用します。[Linuxでの実行手順](../../.github/workflows/k-choose.yml)
