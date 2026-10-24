# 束縛のinfer省略

[English](infer-omission.en.md)

## 実装した範囲

`value: = expression;`は`value: infer = expression;`の省略形です。`:`は宣言と代入を区別するため必須で、任意の`let`・`mut`と組み合わせられます。明示型と明示的な`infer`も維持します。

| 場所 | 表記と対応 |
| --- | --- |
| 通常の束縛（トップレベル・関数・ブロック） | `let mut value: = 1;`を受理 |
| for初期化 | `for (mut i: = 0; i < 3; i = i + 1) {}`を受理 |
| 配列反復 | 初期値の`=`がないため、従来の`value: infer in values`を維持。`value: in`・`value: = in`は受理しない |
| 定数・引数・戻り値・フィールド | 型を必須とし、省略を広げない |
| for更新節 | 従来どおり代入。宣言は受理しない |

通常の束縛とfor初期化はともに`name: type = expression`なので同じ省略規則を使います。配列反復の推論は反復対象から行われるため、今回の「初期値の前のinfer省略」とは分けています。else if・条件式はこの変更に含めません。

## 意味と観測

Parserは束縛の`:`に続く`=`を見て既存の`TypeSpec::Infer`を返します。`=`は初期値の解析に残し、存在しないinfer tokenやSpanは作りません。型解決・HIR・MIR・SSA・VM・各backendには省略形用の意味論を追加しません。

整数の既定型、数値suffix、複合値、評価順、短絡、コピー、可変性、既存の型エラーは明示的なinferと同じです。解決された型は従来どおり`emit-ir`等で確認できます。元の表記は`emit-sources`で観測でき、各ソースのNodeId/Spanと失敗位置を保持します。

fmtは省略形と明示形をそれぞれ保持します。`--let`はlet接頭辞だけを切り替え、inferを挿入・削除しません。`--infer`等の切替、解決型をソースへ書き戻す操作は未実装です。

## 検証

- 明示inferを同じ幅の空白へ置き換え、位置変化と意味の変化を分けてAST・IR全体の一致を検証。
- [example](../../examples/formatting/infer.ceru)の省略形・明示形と各fmt方針をHIR・MIR・SSA・VM・C・LLVM・QBE・WAT・ASM・Objectで比較。
- 不正な省略箇所、型エラー、失敗前の出力と停止理由、元／整形後の診断位置、コメント保持と整形冪等性を検証。

[#105](https://github.com/Cerune-Lang/Cerune/issues/105)のinfer省略に対応します。同Issueのelse if・条件式等は残るため、Issue全体の完了ではありません。
