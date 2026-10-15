# 複合値の比較・表示とmatch式

[English](aggregate-values.en.md)

**状態: 実装済み**

## 値として使う

| 操作 | 規則 | example |
| --- | --- | --- |
| `==`・`!=` | 同じ型の配列・構造体・直和型を内容で比較 | [aggregate_comparison](../../examples/aggregate_comparison.ceru) |
| `print(value);` | 入れ子の値もまとめて表示 | [aggregate_display](../../examples/aggregate_display.ceru) |
| match式 | 選んだ分岐の式を結果にする | [match_values](../../examples/match_values.ceru) |
| ガード | パターンの後の`if`で条件を明示 | [match_guards](../../examples/match_guards.ceru) |

```cerune
enum Lookup { Found { text: string }, Missing }
value: Lookup = Lookup::Found { text: "空" };
label: string = match value {
    Lookup::Found { text: text } if byte_len(text) > 0 => text,
    Lookup::Found { text: _ } => "空欄",
    Lookup::Missing {} => "未登録",
};
print(value); // Found{text: "空"}
print(label); // 空
```

## 等値比較

左右の式を左から一度ずつ評価し、そのコピーを比較します。配列は添字順、構造体はフィールドの宣言順で調べ、最初の不一致で比較を終えます。直和型はタグを先に比較し、一致した選択肢のペイロードだけを調べます。内部の非選択領域は値の意味に含めません。`!=`はこの結果の否定です。

数値・真偽値・文字列の比較規則を入れ子へ適用します。NaNを含む値は自身と比較しても一致せず、正負のゼロは等しいままです。文字列はUTF-8のバイト列を比較し、Unicodeを正規化しません。

配列の要素型・長さや名前付き型が異なる値は比較できません。型のないリテラルへの既存の文脈型付けは有効です。複合値の大小比較は導入しません。定数式や型を指定したジェネリック関数でも使えます。

## 表示

| 値 | 形式 |
| --- | --- |
| 配列 | `[1, 2]` |
| 構造体 | `{x: 1, y: 2}` |
| 直和型 | `Found{text: "空"}`、`Missing{}` |
| 入れ子 | `[{x: 1}, {x: 2}]` |

型名は付けず、構造体のフィールド名と直和型の選択肢名を表示します。並び順は配列の添字順・フィールドの宣言順です。直和型の非選択領域は表示しません。これは値を読むための形式であり、型を復元する直列化形式ではありません。

複合値の中の文字列は引用符で囲みます。`"`・`\`・NUL・LF・CR・TABはそれぞれ`\"`・`\\`・`\0`・`\n`・`\r`・`\t`と表示し、残りのASCII制御バイトは`\u{01}`・`\u{1b}`・`\u{7f}`のような小文字2桁で示します。日本語などのUTF-8バイトはそのままです。値自体は書き換えず、単独の`print(string)`は従来どおり元のバイトを出力します。

引数全体の評価・コピーが終わってから区切り記号を書き、最後にLFを一つ付けます。引数の途中で停止した場合、新しい複合値の一部分は表示しません。それ以前の関数呼び出し等による出力は残ります。数値は既存の表示規則に従い、特殊な浮動小数点値の綴りについても[既存の制約](../reference/language.ja.md)を引き継ぎます。

## match文・式とガード

対象は一度だけ評価してコピーします。分岐はソース順に調べ、タグが合ったときだけ`bool`のガードを評価します。ガードが偽なら次へ進み、真ならその本文または結果式だけを実行します。後続のガード・結果を先に評価しません。

各選択肢にはガードなしの分岐が必要です。`if true`でも網羅したことにはしません。同じ選択肢のガードなし分岐より後にその選択肢を書いた場合は、到達できないため診断します。各フィールドは束縛するか`field: _`で捨てます。束縛はガードとその分岐だけで有効な不変のコピーです。

文の`=>`の後にはブロック、式の`=>`の後には一つの式を書きます。結果の型は全分岐で同じです。代入先等の期待型があれば各分岐へ伝え、なければ最初の分岐で決まる型を使います。暗黙の数値変換はしません。入れ子、関数の戻り値、ジェネリック関数、定数式・配列型の定数長にも使えます。

match文内の`return`・`break`・`continue`は従来どおり周囲の関数・ループへ作用します。ブロックを値にする構文、全体ワイルドカード、入れ子パターン、暗黙のエラー伝播は追加しません。

## 生成過程を追う

全経路の手前で、共通の型付きIRへ展開します。特定のバックエンドだけが複合値の意味を独自に決めない構成です。

| IRの印 | 見える処理 |
| --- | --- |
| `lower aggregate-equality` | 引数のコピー、添字ループ、フィールド・タグ比較、早期return |
| `lower aggregate-display` | 値のコピー、フィールド取り出し、区切り記号、`write`・`write.quoted`、最後の改行 |
| `lower match-binding` | 対象やパターン値の局所的な束縛 |
| `lower match-selection` | 条件と選んだ結果だけを評価する分岐 |

生成した関数には`generated`と元のソース範囲を付けます。match式の外側の値は引数としてコピーし、ガードと結果の計算は該当する関数・分岐の中に残します。定数は元の式と評価結果を保持します。bytecode・LLVM/ASMの出自注釈でも元の演算を追え、失敗時は補助関数内でも元の失敗式の位置・理由・先行出力を保持します。

`write`は改行しない内部操作で、ソース言語の新しい組み込み関数ではありません。C/LLVM/QBE/ASMは数値の既存書式とバイト出力を使います。文字列の引用処理に動的確保を導入しません。WATも[版付き出力契約](wat-output-host.ja.md)で整形済みバイトを渡し、操作全体の要求/受理数・出自・部分失敗を保持します。メモリは公開しません。ターゲットは従来どおり明示し、実行中のOSから黙って選びません。

観測用の情報は実行中の値を外部から変更する窓口を追加しません。コピー後の独立性と短絡評価を保ったまま、生成された手順を確認できます。

## 試す

```sh
cargo run --quiet -- run examples/match_values.ceru
cargo run --quiet -- emit-ir examples/aggregate_comparison.ceru
cargo run --quiet -- emit-bytecode examples/match_guards.ceru
cargo run --quiet -- emit-llvm examples/aggregate_display.ceru --target x86_64-unknown-linux-gnu --annotate-origins
cargo run --quiet -- emit-asm examples/match_values.ceru --target x86_64-unknown-linux-gnu --annotate-origins
```

[モジュール版](../../examples/modules/aggregate_match.ceru)では公開enum・定数と別ファイルのmatch式を組み合わせます。VM・生成C/LLVM・QBE・WAT・Windows/LinuxのASMと自前オブジェクトの出力を照合します。NaN・正負のゼロ・最大u64・NUL/CR/LF・Unicodeの非正規化・コピー・評価順に加え、表示引数やガード・結果式での停止も検証します。

[観測fixture](../../tests/fixtures/observation/aggregate-values/source.ceru)から生成した各形式と注釈付きLLVM/ASMを保存し、型付きIRから生成物までの変化をテストで確認します。
