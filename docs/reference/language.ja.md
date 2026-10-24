# Cerune言語リファレンス

[English](language.en.md)

この文書では、Cerune v0.1の構文と意味を定義します。

## 文法

`let`は通常の束縛・for初期化・配列反復の各束縛で省略できます。`let mut total: infer = 0;`と`mut total: infer = 0;`は同じ意味です。`:`は必須で、定数・引数・フィールド・for更新節にはletを付けません。予約語ではなく、束縛位置で次が識別子または`mut`の場合だけ接頭辞として扱います。既存の`let: i64 = 1;`等の識別子は維持します。整形での保持・表示・非表示は[fmt](cli.ja.md#ソース整形)を参照してください。

```text
program     := item* EOF

item        := enum_definition
             | type_definition
             | function_definition
             | constant_definition
             | statement

type_definition :=
    "type" IDENT "{" field_definition ("," field_definition)* ","? "}"

enum_definition := "enum" IDENT "{" variant ("," variant)* ","? "}"
variant     := IDENT ("{" payload_fields? "}")?
payload_fields := (IDENT ":" type_ref) ("," IDENT ":" type_ref)* ","?
variant_path := IDENT "::" IDENT | IDENT "::" IDENT "::" IDENT
match_statement := "match" expression "{" match_arm ("," match_arm)* ","? "}"
match_arm   := variant_path "{" pattern_fields? "}" ("if" expression)? "=>" block
match_expression := "match" expression "{" match_value_arm ("," match_value_arm)* ","? "}"
match_value_arm := variant_path "{" pattern_fields? "}" ("if" expression)? "=>" expression
pattern_fields := (IDENT ":" IDENT) ("," IDENT ":" IDENT)* ","?

constant_definition := "const" IDENT ":" type_ref "=" expression ";"

field_definition := IDENT ":" type_ref ("=" expression)?

function_definition :=
    "fn" IDENT generic_parameters? "(" parameters? ")" "->" return_type block

generic_parameters := "<" generic_parameter ("," generic_parameter)* ","? ">"
generic_parameter := IDENT | "const" IDENT ":" "i64"
generic_arguments := "::<" generic_argument ("," generic_argument)* ","? ">"
generic_argument := type_ref | INTEGER
function_path := IDENT ("::" IDENT)?
function_call := function_path generic_arguments? "(" arguments? ")"

parameters  := parameter ("," parameter)*

parameter   := IDENT ":" type_ref

return_type := type_ref | "void"

statement   := binding
             | assignment
             | "print" "(" expression ")" ";"
             | "flush" "(" ")" ";"
             | function_call ";"
             | "return" expression? ";"
             | match_statement
             | if_statement
             | while_statement
             | for_statement
             | for_each_statement
             | "break" ";"
             | "continue" ";"

if_statement := "if" expression block ("else" block)?

while_statement := "while" expression block

for_statement :=
    "for" "(" (binding_clause | binding_assignment_clause) ";"
    expression ";" binding_assignment_clause ")" block

for_each_statement :=
    "for" "(" ("let"? IDENT ":" type_spec ",")?
    "let"? "mut"? IDENT ":" type_spec "in" expression ")" block

block       := "{" statement* "}"

binding     := "let"? "mut"? IDENT ":" type_spec? "=" expression ";"

binding_clause := "let"? "mut"? IDENT ":" type_spec? "=" expression

assignment  := assignment_target "=" expression ";"

assignment_target := IDENT ("[" expression "]")*

binding_assignment_clause := IDENT "=" expression

type_spec   := "i8"
             | "u8"
             | "i16"
             | "u16"
             | "i32"
             | "u32"
             | "i64"
             | "u64"
             | "f32"
             | "f64"
             | "bool"
             | "string"
             | fixed_array_type
             | IDENT
             | "infer"

type_ref    := "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "bool" | "string" | fixed_array_type | IDENT

fixed_array_type := "[" type_ref ";" array_length "]"
array_length := INTEGER | IDENT ("::" IDENT)?

expression  := logical_or

logical_or  := logical_and ("||" logical_and)*

logical_and := bit_or ("&&" bit_or)*

bit_or      := bit_xor ("|" bit_xor)*

bit_xor     := bit_and ("^" bit_and)*

bit_and     := equality ("&" equality)*

equality    := comparison (("==" | "!=") comparison)*

comparison  := shift (("<" | "<=" | ">" | ">=") shift)*

shift       := additive (("<<" | ">>") additive)*

additive    := multiply (("+" | "-") multiply)*

multiply    := unary (("*" | "/" | "%") unary)*

unary       := ("-" | "!" | "~") unary
             | postfix

postfix     := primary (("." IDENT) | ("[" expression "]"))*

primary     := match_expression
             | "true"
             | ("i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64") "(" expression ","? ")"
             | "convert" "<" type_ref ">" "(" expression ","? ")"
             | "false"
             | INTEGER
             | FLOAT
             | STRING
             | "[" expression ("," expression)* ","? "]"
             | IDENT
             | function_call
             | variant_path "{" (field_value ("," field_value)* ","?)? "}"
             | IDENT "{" field_value ("," field_value)* ","? "}"
             | IDENT "{" ".." expression ("," field_value)* ","? "}"
             | "(" expression ")"

arguments   := expression ("," expression)*

field_value := IDENT ":" expression
```

変数束縛は標準では不変です。`mut`を付けた束縛だけが再代入できます。式から参照できるのは、その式より前に宣言された変数束縛だけです。

型の指定は常に必要です。

```cerune
count: i64 = 42;
single: f32 = 0.1 + 0.2;
double: f64 = 0.1 + 0.2;
value: infer = count * 2;
```

`infer`は型推論を明示的に要求します。`infer`自体が実行時の型になることはありません。

通常の束縛とfor初期化では`value: = expression;`と省略しても同じ推論を使います。`mut`・任意の`let`とも組み合わせられ、宣言を表す`:`は必須です。配列反復は引き続き`value: infer in values`等の型指定が必要です。定数・引数・戻り値・フィールドへ省略を広げません。fmtは推論表記を保持します。[契約と検証](../design/infer-omission.ja.md)を参照してください。

## 文字列

`string`はUTF-8の文字列です。二重引用符で囲んで書き、空の文字列`""`や日本語も使えます。

```cerune
mut text: string = "こんにちは";
saved: infer = text;
text = "こんばんは";
print(saved);                  // こんにちは
print(text);                   // こんばんは
print(saved == "\u{3053}んにちは"); // true
```

文字列の中身は直接書き換えません。`mut`は、その名前に別の文字列を代入できる指定です。代入しても、別の変数に保存した値は変わりません。文字列は関数の引数・戻り値、構造体のフィールドと既定値、固定長配列の要素にも使えます。`mut`な文字列配列では、要素を別の文字列に置き換えられます。

文字列内で使えるエスケープは次のとおりです。

| 表記 | 値 |
| --- | --- |
| `\"` | 二重引用符 |
| `\\` | バックスラッシュ |
| `\n` / `\r` / `\t` | 改行（LF）/ 復帰（CR）/ タブ |
| `\0` | NUL（文字列の終わりではなく、値の一部） |
| `\u{...}` | 1〜6桁の16進数で指定したUnicodeスカラー値 |

`\u{...}`は`0`〜`10FFFF`の範囲で、サロゲート領域`D800`〜`DFFF`を除きます。不正なエスケープや閉じていない引用符は、元のソース位置付きで診断します。引用符の間にソース上のLFやCRを直接書くことはできません。改行には`\n`などを使います。

`==`と`!=`は文字列全体の内容を比較します。大文字と小文字を区別し、Unicodeの正規化もしません。見た目が同じでも文字の並びが異なれば別の値です。`"a\0b"`を`"a"`として扱うこともありません。

文字列への添字参照、文字数の取得、大小比較、数値との変換は未実装です。文字列と他の型の暗黙変換もありません。

`print`は文字列の内容をそのまま出力し、末尾にLFを加えます。一方、Cerune IRとbytecodeのテキスト表示は、改行や制御文字をエスケープします。値として解釈された内容と、元の引用符付き表記のUTF-8バイト範囲（Span）を区別して保持します。

文字列は全出力経路に対応します。LLVMとQBEでは[CLIのターゲット指定](cli.ja.md#llvmのターゲット指定)が必要で、省略するとlowering前にソース位置付きで診断します。使われない型定義・関数・分岐に含まれる文字列も対象です。直接アセンブリはWindows x64とLinux x86-64に対応し、ネイティブオブジェクトではターゲットの明示が必須です。WATはWebAssemblyのホスト出力契約を使います。

C生成ではデータ参照とバイト数の組で表現し、静的文字列は実行終了まで、動的文字列は最後の解放まで内容を保持します。文字列を使う生成プログラムは、Windowsでも標準出力をバイナリモードにし、LFやCRを自動変換しません。表現と寿命の詳細は[文字列の設計](../design/strings.ja.md)を参照してください。

### 文字列を連結する

`concat(left, right)`は二つの`string`を左から一度ずつ評価し、内容を順につないだ不変の`string`を返します。NUL・CR/LF・Unicodeの表記をバイトのまま保持します。関数返却、配列、構造体、enum、既定値、`const`でも使えます。組み込み名のため同名の関数は定義できません。

```cerune
mut text: string = concat("こんにちは、", "世界");
saved: string = text;
text = concat(text, "！");
print(saved); // こんにちは、世界
print(text);  // こんにちは、世界！
```

結果が空なら静的な空文字列を使い、それ以外は新しく確保します。右辺の評価後に旧値を解放するため、自己代入や旧値を引数に使う再代入も有効です。動的内容の既定予算は64 MiBで、`--string-heap-limit <bytes>`により実行前に変更できます。定数評価は独立した64 MiBです。[所有・寿命と失敗の契約](../design/dynamic-data.ja.md)を参照してください。

### UTF-8のバイト数

`byte_len(text)`は`string`を一つ受け取り、保持しているUTF-8のバイト数を`i64`で返します。引数は一度だけ評価します。

```cerune
print(byte_len(""));          // 0
print(byte_len("日本語"));    // 9
print(byte_len("\0\r\n"));  // 3
print(byte_len("\u{e9}"));    // 2
print(byte_len("e\u{301}"));  // 3
```

NULや改行も数え、Unicodeの正規化はしません。Unicodeスカラー値の数や、画面上の文字数・幅を返す操作ではありません。結果は常に`i64`で、小さい整数型へ代入するときは既存の明示変換を使います。

`byte_len`は組み込み操作名なので、同名の関数は定義できません。引数の数や型が違う場合、または結果を使わずに呼び出し文にした場合はソース位置付きで診断します。変数名と関数名の名前空間は従来どおり別です。関数の返却値、配列要素、フィールド、既定値にも適用できます。

`byte_len(f())`でも`f()`を必ず一度評価します。短絡評価で実行されない右辺は評価しません。引数の評価が範囲外アクセスなどで失敗した場合、長さの取得前に停止します。

## 名前付きproduct type

`type`は、複数のfieldを一つの値へまとめる名前付きproduct typeを定義します。型は名前で区別されるため、同じfieldを持つ二つの型も別の型です。

```cerune
type Point {
    x: f64 = 0.0,
    y: f64,
}

point: Point = Point {
    y: 2.0,
};

print(point.x);
```

fieldの型は定義時に必ず指定し、`infer`は使用できません。既定値のないfieldは値を作るときに必要です。fieldは名前で指定するため、記述順は型定義と異なっても構いません。末尾のカンマも使用できます。

元の値を指定しない構築では、明示したfieldの式はソースに書いた順番で評価します。その後、省略したfieldの既定値を型定義順で評価します。Cerune IRでは、この順番と、値が明示されたか既定値から来たかを観測できます。

field accessには`.`を使い、`segment.start.x`のように入れ子にできます。fieldは直接変更できません。変更するときは新しい値を作り、`mut`な束縛へ全体を再代入します。

```cerune
mut point: Point = Point { x: 1.0, y: 2.0, };
point = Point { x: 3.0, y: point.y, };
```

積値を別の束縛へ入れた後で元の束縛へ再代入しても、先の値は変化しません。これは言語上の値としての規則です。各backendが物理的にどう配置・コピーするかは、生成物で観測できます。

`if`と`while`の条件の直後にある`{`はブロックの開始として読みます。構築式から得たfieldを条件にする場合は、構築式を丸括弧で囲みます。

```cerune
type Flags { enabled: bool, }

if (Flags { enabled: true, }).enabled {
    print(true);
}
```

空のproduct type、空の構築式、無限サイズになる値による再帰型は現在サポートしません。積値全体の等値比較と`print`は[複合値の規則](../design/aggregate-values.ja.md)に従います。

詳細な設計と各backendの表現は[名前付きproduct typeの設計](../design/product-types.ja.md)で説明します。

### 構造体の更新式

`Point { ..original, x: 3 }`は、`original`から`x`だけを変更した新しい`Point`を作ります。

- 元の値は先頭に一つだけ指定し、一度評価してコピーします。その型は指定した名前付き型と同一でなければなりません。
- 続くフィールドの式は記述順に評価します。省略したフィールドは元の値から引き継ぎ、既定値を再評価しません。
- `Point { ..original }`と末尾のカンマも使えます。全フィールドを置き換える場合も、元の値の評価を省略しません。
- 未知・重複したフィールド、型不一致、後置・重複した元の値は診断します。直接のフィールド代入は許可しません。
- 元の値やコピー後の配列は独立です。失敗時は後続の式と代入を実行せず、停止前の出力と失敗式の位置を保持します。

[構文と評価順の設計](../design/product-updates.ja.md)と[実行例](../../examples/product_update.ceru)を参照してください。

## 動的配列

`[T]`は実行時の長さを持つ所有値です。`run`／`run-ir`／`run-vm`で実行でき、`emit-ir`／`emit-bytecode`／`emit-c`／`emit-llvm`／`emit-qbe`／`emit-wat`で処理を確認できます。生成C・LLVM・QBEは外部コンパイラでビルドし、WATはWasmへ変換して出力用ホスト関数のある環境で実行します。LLVM・QBEは`--target`の明示が必要です。`emit-asm`／`emit-obj`でも生成でき、Windows/Linuxの明示ターゲットに対して外部ツールでリンクします。

| 操作 | 意味 |
| --- | --- |
| `array_copy(values)` | 固定長・動的配列から独立した`[T]`を作る |
| `array_copy_range(values, start, end)` | 半開区間をコピー。境界は`i64`、空範囲も可 |
| `array_repeat::<T>(value, count)` | 明示した型`T`の値を`i64`の個数だけ独立コピーして`[T]`を作る |
| `array_len(values)`・添字・表示・等値比較・`for … in` | 固定長配列と同じ規則。比較は同じ型同士、反復は開始時の独立コピー |

`[1, 2]`は固定長のままで、暗黙変換はありません。空配列は`array_copy_range([1], 0, 0)`で作れます。束縛・引数・返却・複合値へのコピーは入れ子も独立し、`mut`では要素更新と異なる長さへの再代入ができます。型引数・モジュール・構造体・enumにも使用できます。

範囲コピーは対象・開始・終了を左から一度ずつ評価してから`0 <= start <= end <= length`を検査し、違反すると`array-range-out-of-bounds`で停止します。要素代入は各添字の検査を次の添字・右辺より先に行います。

`array_repeat`は値、個数の順に一度ずつ評価してから、個数が非負か検査します。負なら`array-length-out-of-range`です。0個でも両式を評価して一時値を解放し、結果の領域は確保しません。各要素内の動的配列も独立します。

`--array-heap-limit <bytes>`は配列要素領域の予算で、既定64 MiB、文字列予算とは独立です。コピーは新旧の領域が同時に必要になります。表示・長さ・比較のためだけにコピーはしません。`const`内の動的配列、成長操作、借用スライスは未対応です。[実行例](../../examples/dynamic_arrays/README.md)と[所有・生成過程の設計](../design/owned-arrays.ja.md)を参照してください。

## 固定長配列

固定長配列は、同じ型の箱を決まった数だけ横に並べた値です。`[i64; 4]`は「`i64`の箱が4個ある配列」という意味です。

```cerune
values: [i64; 4] = [2, 4, 6, 8];
print(values[2]);
```

長さは型の一部です。したがって、`[i64; 3]`と`[i64; 4]`は別の型です。配列リテラルの要素数と宣言した長さが違う場合や、要素の型がそろわない場合はエラーになります。空の配列は、要素型を決められないため現在は使えません。

添字は`i64`で、先頭は`0`です。上の例の`values[2]`は3番目の値`6`を読み取ります。負の添字や長さ以上の添字は、Cerune VMと生成されたプログラムのどちらでも実行時に停止します。境界検査は各backendの生成物にも残るため、どこで安全を確かめているか観測できます。

配列は一つの値としてコピーされます。コピーした後で元の`mut`束縛へ新しい配列を再代入しても、先に作ったコピーは変わりません。

```cerune
mut first: [i64; 2] = [10, 20];
second: [i64; 2] = first;
first = [30, 40];
print(second[0]); // 10
```

要素型には`bool`、`i8`、`u8`、`i16`、`u16`、`i32`、`u32`、`i64`、`u64`、`f32`、`f64`、`string`、名前付きproduct type、直和型または固定長配列を使用できます。固定長配列はproduct typeのfieldにも使用できます。

```cerune
type Point {
    x: i64,
    y: i64,
}

type Path {
    points: [Point; 4],
}

path: Path = Path {
    points: [
        Point { x: 0, y: 0, },
        Point { x: 1, y: 1, },
        Point { x: 2, y: 4, },
        Point { x: 3, y: 9, },
    ],
};

print(path.points[2].y);
```

固定長配列は直接入れ子にできます。`matrix[row][column]`では、二つの添字を順番に検査します。

```cerune
matrix: [[i64; 3]; 2] = [[1, 2, 3], [4, 5, 6]];
print(matrix[1][2]); // 6
```

固定長配列は関数のparameterと戻り値にも使えます。関数をまたぐときも配列は値としてコピーされます。

`mut`な配列では、`values[index] = value;`で一つの要素を更新できます。入れ子の配列も`matrix[row][column] = value;`のように更新できます。すべての添字は左から順に評価して、その都度境界を検査します。すべての検査に成功した後で右辺を評価し、最後に一度だけ書き込みます。途中で検査に失敗した場合、右辺は評価せず、配列も変更しません。

配列をコピーした場合、片方の要素を更新してももう片方は変わりません。要素の型は配列定義と同じでなければなりません。不変な束縛を通した更新は、配列全体への再代入と同様にエラーです。

詳しい設計と各backendでの境界検査は[固定長配列の設計](../design/fixed-arrays.ja.md)で説明します。


### 定数による型の長さ

```cerune
const COUNT: u32 = 2;
values: [i64; COUNT] = [7, 9];
same: [i64; 2] = values;
print(array_len(same)); // 2
```

長さには正の整数リテラル、整数型の定数名、`alias::COUNT`を指定します。全整数型を使えますが、値は正で`i64::MAX`以下が必要です。定数の宣言順は問いません。型の同一性は確定した長さで決まり、要素数不一致は従来どおり診断します。

計算は`const COUNT: i64 = BASE + 1;`のように定数側へ書きます。`[T; COUNT + 1]`、型内の関数呼び出し、実行時変数は未対応です。型を経由する定数の循環も診断します。公開型に同じファイルの非公開定数を使えますが、import先の定数を参照するには`pub`が必要です。

共通の資源上限は一つの複合値で10万scalar分、型計算の深さ128です。リテラル・定数名・`infer`へ同じ制限を適用します。定数の計算と型中の参照位置はIRへ残します。[評価・上限の詳細](../design/constant-array-lengths.ja.md)と[example](../../examples/constant_array_lengths.ceru)を参照してください。

### 配列の要素数

`array_len(values)`は配列を一つ受け取り、最外側の要素数を`i64`で返します。固定長`[T; N]`では`N`、動的`[T]`では実行時の長さです（動的配列はIR・VM・C・LLVM・QBE・WAT・ASM・自前Object対応）。要素型は問いません。文字列のバイト数は`byte_len`で取得します。

```cerune
matrix: [[i64; 3]; 2] = [[1, 2, 3], [4, 5, 6]];
print(array_len(matrix));    // 2
print(array_len(matrix[0])); // 3
```

引数は一度だけ評価します。`array_len(make())`の呼び出しや配列要素の評価を、長さが既知という理由で省略しません。引数の評価で停止した場合、長さを返さず元の失敗式を診断します。短絡評価では実行しない右辺を評価せず、ループ条件では条件を調べるたびに評価します。

定数式でも使用できますが、引数全体に定数式の規則が適用されます。未使用の定数でも引数内の失敗を診断し、通常の関数呼び出しは認めません。整数の結果を定数へ保存し、配列型の長さにも使えます。

同名の関数・定数・import別名は定義できません。変数と関数の名前空間は別です。引数の数・型の誤り、結果を捨てる呼び出し文は診断します。[集計の例](../../examples/array_length.ceru)・[評価順の例](../../examples/array_length_order.ceru)を参照してください。


### 固定長配列の反復

動的配列の反復もIR・VM・C・LLVM・QBE・WAT・ASM・自前Objectで同じ規則を使い、長さ0なら本体を実行しません。

```cerune
values: [i64; 3] = [4, 7, 9];
for (value: infer in values) { print(value); }
for (index: i64, mut value: infer in values) {
    value = value + index;
    print(value);
}
print(values[1]); // 7
```

`for (要素: 型 in 対象)`、または`for (添字: 型, 要素: 型 in 対象)`で固定長配列を先頭から走査します。各束縛に明示型か`infer`が必要です。添字は不変な`i64`で、要素型は対象から決まります。`for (v: u8 in [1])`は型エラーで、対象を`[1u8]`などと指定します。`in`は予約語です。

対象式は開始時に一度、外側のスコープで評価してコピーします。本文内で元の配列を変更しても走査中の値は変わりません。要素も毎回コピーし、`mut`を付けた場合はその回のコピーだけを変更できます。束縛は本文だけで有効で、同じ本文の重複宣言、定数・import別名の隠蔽は診断します。

`continue`は次の要素へ進み、`break`・`return`は既存の意味に従います。最初に`break`しても対象全体の評価は済んでいます。対象や本文で失敗すると元の失敗式を記録して停止します。反復内の`return`だけでは、関数の全経路が値を返すと判定しません。空配列・文字列反復・参照・分解パターンは未対応です。

Cerune IRでは対象のコピー・長さ・カーソルを`$for_in_*`の内部束縛と既存の`for`で表します。対象と本文のソース位置を保ち、生成した制御式はヘッダーへ結び付けます。[設計](../design/array-iteration.ja.md)と[集計・検索の例](../../examples/array_iteration.ceru)を参照してください。

## 直和型とmatch

`enum Lookup { Found { text: string }, Missing }`は、値を持つ選択肢と値を持たない選択肢を定義します。構築は`Lookup::Found { text: "空" }`、`Lookup::Missing {}`です。型は名前で区別し、変数・引数・戻り値・配列・構造体・定数に使えます。

```cerune
enum Lookup { Found { text: string }, Missing }
value: Lookup = Lookup::Found { text: "空" };
match value {
    Lookup::Found { text: text } => { print(text); },
    Lookup::Missing {} => { print("未登録"); },
}
```

`match`は文・式に使えます。対象を一度だけ評価してコピーし、ソース順に分岐を調べます。パターンの後に`if 条件`を付けると、タグが一致した場合だけ`bool`の条件を評価し、偽なら次へ進みます。各選択肢にガードなし分岐が必要で、その後に同じ選択肢の分岐は書けません。選んだ本文・結果だけを評価します。各フィールドは`field: binding`で不変なローカル値として取り出すか`field: _`で捨てます。構築式を対象へ直接書くときは括弧で囲みます。構築のフィールドはソース順に評価します。コピー後の再代入は他の値に影響しません。

`enum`・`match`は予約語です。`pub enum`は全選択肢とフィールドを公開し、import側では`alias::Enum::Variant`を使います。enumのフィールドには既定値を書けません。再帰的な値型、直接フィールド参照、構造体更新式、全体ワイルドカード、入れ子パターンは未対応です。全体の表示・等値比較は[複合値の規則](../design/aggregate-values.ja.md)に従います。`return`・`break`・`continue`は通常の関数・ループへ作用します。実行時停止の捕捉は行いません。[設計と表現](../design/sum-types.ja.md)、[example](../../examples/sum_lookup.ceru)を参照してください。

文の分岐は`=> { 文... }`、式の分岐は`=> 式`です。全結果の型を揃え、期待型があれば各分岐へ伝えます。期待型がなければ最初の結果で型を決めます。暗黙の変換やブロック式は追加しません。束縛はガードとその分岐だけで有効です。

```cerune
label: string = match value {
    Lookup::Found { text: text } if byte_len(text) > 0 => text,
    Lookup::Found { text: _ } => "空欄",
    Lookup::Missing {} => "未登録",
};
```

## コンパイル時定数

`const LIMIT: u64 = 64 * 2;`は、型付きの式をコンパイル時に評価します。ファイル直下で宣言し、関数や既定値からも参照できます。前方参照は可能ですが、循環と128段を超える依存は診断します。

数値・真偽値・文字列・配列・構造体・直和型を扱えます。定数式では実行時変数と通常の関数呼び出しを禁止します。短絡評価を保ちつつ、未使用の定数も型検査・評価します。失敗は実行前のコンパイルエラーです。

定数への代入と同名のローカル束縛はできません。`pub const`で公開し、`alias::LIMIT`で使います。整数定数は配列型の長さにも使えます。ブロック内宣言は未対応です。[評価と観測の契約](../design/constants.ja.md)、[example](../../examples/constants.ceru)を参照してください。

## モジュールと公開範囲

```cerune
import "values.ceru" as values;
item: values::Reading = values::reading(7);
print(item.amount);
```

別ファイルの型・関数・定数を`別名::名前`で使います。importは定義・文より前に置きます。定義は既定でファイル内だけに公開し、外部へ出すものに`pub fn`・`pub type`・`pub enum`・`pub const`を付けます。公開型は全フィールドを公開します。公開関数の引数・戻り値と公開型のフィールドには、非公開型を含めません。

読み込まれるファイルのトップレベルにはimport・型・関数・定数だけを許します。importしただけで初期化処理は走りません。依存側の`main`も自動実行しません。相対`.ceru`パスは宣言元のディレクトリ基準で、`/`を区切りに使います。循環、非公開参照、別名の重複や定義・変数との衝突は診断します。`import`・`as`・`pub`・`const`は予約語です。

再export、ワイルドカード、外部変数、可変なモジュール状態は未対応です。[規則と読み込みの上限](../design/modules.ja.md)、[動作するexample](../../examples/modules/README.md)を参照してください。

### 仮想標準モジュールの行入力

`import "std:io" as io;`で同梱標準モジュールを明示し、`io::read_line(max_bytes)`を呼びます。結果は通常のenum `io::ReadLine::Line { text: string }`／`io::ReadLine::End {}`です。LF・CRLFを保持し、空行とEOFを区別します。上限は改行を含む1〜2147483647バイトです。読取り・UTF-8・上限・予算などの異常は診断して停止します。

現在はHIR・MIR・SSA・VMのRust入力接続APIで利用できます。[CLI・生成C・LLVM・QBE・Nativeの実stdin](../design/stdin-connections.ja.md)も利用できます。[WATの版付きhost接続](../design/wat-input-host.ja.md)も実装済みです。**Windows console入力は未対応です。** 標準モジュールはファイルや環境変数から探さず、本文・型・出自をemit-sources等で確認できます。[解決規則と実行例](../design/standard-modules.ja.md)・[消費数と失敗の契約](../design/line-input.ja.md)を参照してください。

### 明示的な整数読取り

`import "std:num" as num;`で`num::parse_i64(text: string) -> num::ParseI64`を使えます。結果は`Value { value: i64 }`・`InvalidFormat {}`・`OutOfRange {}`です。ASCIIの`-?[0-9]+`だけを全文検査し、その後i64の範囲を検査します。先頭0を許し、`-0`は0です。`+`、空白、改行、NUL、全角数字などは受理せず、入力を変更・正規化しません。

全既存経路に対応します。判定処理は同梱のCeruneソースで、生成過程も観測できます。[詳細契約](../design/integer-parsing.ja.md)・[基本例](../../examples/integer_parsing/README.md)を参照してください。

### プログラム引数

`import "std:env" as env;`で`env::arg_count() -> i64`と`env::arg(index: i64) -> env::Argument`を使えます。結果は`Value { text: string }`／`Missing {}`です。空文字はValue、0以上の範囲外はMissing、負添字は位置付き診断で停止します。文字列の内容を正規化・再分割しません。

HIR・MIR・SSA・VM・CLI・生成C・LLVM・QBE・WAT・Nativeに対応しています。CLIの最初の`--`以後をプログラム引数とし、実行ファイル名やソース名を含めません。既定上限は4096件・UTF-8合計1048576バイトで、区切り前の`--argument-count-limit`／`--argument-byte-limit`で変更できます。不正Unicode・NUL・上限超過は起動準備で停止します。取得文字列には別途文字列ヒープ上限を適用します。

既存の埋込みAPIは暗黙にOS引数を読まず、引数取得時に未設定を診断します。`run_with_arguments`等で明示注入してください。[契約](../design/dogfooding-inputs.ja.md)・[実行例](../../examples/arguments/README.md)を参照してください。

## 関数とentry point

`fn`は、名前を付けた処理を定義します。parameterと戻り値の型は必ず書きます。

```cerune
fn add(left: i64, right: i64) -> i64 {
    return left + right;
}

answer: i64 = add(20, 22);
```

値を返す関数では`return expression;`を明示します。blockの最後に式を書いても、暗黙の戻り値にはなりません。どの経路でも値を返すと確認できない場合はエラーになります。

値を返さない関数は`-> void`と書きます。blockの最後まで進むか、`return;`で途中終了できます。値を返す関数の呼び出し結果は式として使い、`void`関数は文として呼び出します。

```cerune
fn show(value: i64) -> void {
    print(value);
}

show(answer);
```

関数名はファイル全体で解決するため、定義より前から呼び出せます。parameterと関数内の束縛は、その関数の外から見えません。関数からトップレベルの実行時束縛を読むこともできません。

トップレベルに実行文がある場合、compilerがentry pointを生成します。代わりに`fn main() -> void`を明示できますが、明示的な`main`とトップレベル実行文は同時に書けません。`main`はparameterを受け取れません。

関数のparameterと戻り値には`bool`、`i8`、`u8`、`i16`、`u16`、`i32`、`u32`、`i64`、`u64`、`f32`、`f64`、`string`、名前付きproduct type、固定長配列を使えます。product typeと配列は値として渡されるため、関数が受け取った値と呼び出し側の値は同じ変更可能な場所を共有しません。引数の個数に言語としての固定上限は設けません。再帰はまだサポートしません。未対応の書き方は黙って別の意味にせず、診断します。

Cerune IRとbytecodeでは、関数ID、parameterの束縛ID、呼び出し、戻り値を観測できます。各backendの成果物では、これらが関数シンボル、引数、ローカル領域、ABI上のレジスタやメモリへ変わった結果を観測できます。詳細は[関数の設計](../design/functions.ja.md)を参照してください。

### 型・長さを引数にする関数

```cerune
fn first<T, const N: i64>(values: [T; N]) -> T { return values[0]; }
print(first::<string, 2>(["日本語", "予備"]));
```

`T`は型、`const N: i64`は正の配列長です。`::<...>`へ宣言順に全引数を書きます。型は既存の具体型、長さは正の整数リテラル・整数定数名・外側の長さ仮引数を使い、長さは本文でi64値としても参照できます。型仮引数への数値変換は`convert<T>(value)`です。`pub fn`や`alias::first::<string, COUNT>(values)`にも対応します。

同じ型・長さは一つの通常関数へ具体化します。本文は使われた具体型ごとに検査し、未使用テンプレートの本文全体の型検査は保証しません。短絡で実行されない呼び出しも型検査します。値引数の評価順・コピー・停止規則は通常の関数と同じです。型引数の推論・trait・汎用の型定義・再帰は未対応です。詳細な制限と観測形式は[設計](../design/generic-functions.ja.md)、実行例は[generic_functions.ceru](../../examples/generic_functions.ceru)にあります。

## 可変な束縛と再代入

変更が必要な束縛は、宣言の先頭へ`mut`を付けます。

```cerune
mut count: i64 = 40;
count = count + 2;
print(count);
```

`mut`は型ではなく、名前`count`へ再代入できることを指定します。配列の場合は、その束縛を通した要素更新も許可します。再代入では`: type_spec`を書きません。この違いにより、新しい宣言と既存の束縛への代入を区別します。

`mut`のない束縛へ再代入すると型検査エラーになります。

```cerune
count: i64 = 40;
count = 42; // エラー
```

再代入する値の型は、宣言時に解決された型と一致する必要があります。`infer`を使用した場合も、型を推論するのは宣言時だけです。

```cerune
mut value: infer = 1; // i64へ解決
value = 2;            // OK
value = 0.5;          // エラー
```

Cerune IRでは、最初の初期化と再代入を別の文として保持します。bytecodeでも初期化の`store`と再代入の`assign`を区別します。

## 条件分岐、繰り返し、ブロックスコープ

`if`は`bool`の条件に従って文を実行します。`else`は省略できます。

```cerune
if value < 10 {
    print(value);
} else {
    print(10);
}
```

条件が`bool`でなければ型検査エラーになります。`if`は現在、値を作る式ではありません。

`while`は条件が`true`である間、本文を繰り返します。条件は本文を実行する前に毎回評価されるため、最初から`false`なら本文は一度も実行されません。

```cerune
mut count: i64 = 0;

while count < 3 {
    print(count);
    count = count + 1;
}
```

`while`の条件にも`bool`が必要です。`while`も値を作る式ではありません。

`for`は、開始文、`bool`の継続条件、更新文、本文を一つにまとめます。開始文には新しい束縛か、既存の`mut`な束縛への再代入を書けます。

```cerune
mut sum: i64 = 0;

for (mut i: i64 = 0; i < 6; i = i + 1) {
    sum = sum + i;
}
```

開始文は最初に一度だけ実行します。各回の本文を実行する前に継続条件を評価し、本文を最後まで実行した後に更新文を実行してから、もう一度継続条件を評価します。現在の構文では、開始文、継続条件、更新文をすべて省略できません。

開始文で宣言した束縛は、継続条件、更新文、本文から参照できますが、`for`の後からは参照できません。開始文が既存の束縛への再代入なら、その束縛は`for`の後にも残ります。本文は、その内側に別のブロックスコープを作ります。

`break;`は最も内側のループを終了します。`while`内の`continue;`は条件評価へ直接進みます。`for`内では更新を実行してから条件評価へ進みます。どちらもループの外では使用できません。

```cerune
while value < 10 {
    value = value + 1;

    if value < 3 {
        continue;
    }

    if value > 5 {
        break;
    }
}
```

現在は外側のループを名前で指定するラベル付きの`break`や`continue`を持ちません。

波括弧で囲まれた各ブロックは新しいスコープを作ります。ブロック内の束縛は外側から見えません。内側から外側の束縛を参照でき、`mut`なら再代入もできます。

内側のブロックでは、外側と同じ名前を別の束縛として宣言できます。

```cerune
mut value: i64 = 1;

if true {
    value = 2;          // 外側のvalueを更新
    value: bool = true; // 内側だけの別のvalue
    print(value);       // boolのvalue
}

print(value);           // i64のvalue
```

Cerune IRは各束縛へ決定的なIDを付け、同じ名前でもどの宣言を参照したか区別します。構造化された`if`、`while`、`for`、`break`、`continue`はCerune IRに残ります。`for`は初期化、条件、本文、更新を別々に保持します。Bytecodeと各バックエンドIRへのlowering時に、構造化されたループは条件、本文、必要な更新、終了の経路へ変換されます。`break`と`continue`は、対象ループ内の正しい経路へのジャンプへ変換されます。

## 型

Cerune v0.1には、一つの真偽値型、十の数値型、文字列型、固定長配列、ユーザーが定義する名前付きproduct typeがあります。

組み込み型名`bool`、`i8`、`u8`、`i16`、`u16`、`i32`、`u32`、`i64`、`u64`、`f32`、`f64`、`string`と同名の関数や型は定義できません。定義位置でエラーになります。

```text
bool
i8
u8
i16
u16
i32
u32
i64
u64
f32
f64
string
固定長配列
名前付きproduct type
```

バックエンドへのlowering時に、各バックエンドが対応する型を自身の表現へ対応付けます。文字列は全出力経路で対応しています。

たとえば、Cバックエンドでは次のように対応付けます。

```text
Cerune    C
bool      bool
i8        int64_t
u8        int64_t
i16       int64_t
u16       int64_t
i32       int64_t
u32       int64_t
i64       int64_t
u64       uint64_t
f32       float
f64       double
```

整数型の範囲は次のとおりです。

| 型 | 最小値 | 最大値 |
| --- | --- | --- |
| `i8` | -128 | 127 |
| `u8` | 0 | 255 |
| `i16` | -32768 | 32767 |
| `u16` | 0 | 65535 |
| `i32` | -2147483648 | 2147483647 |
| `u32` | 0 | 4294967295 |
| `i64` | -9223372036854775808 | 9223372036854775807 |
| `u64` | 0 | 18446744073709551615 |

値の範囲と格納幅は別です。現在はすべての整数型を生成先の64ビット領域に格納します。配列や構造体も8・16・32ビットへ詰めて格納しないため、`i64`から小さい型へ変更してもメモリ使用量は減りません。外部のバイト列を直接読み書きする機能は未実装です。

## 真偽値と比較

`bool`には`true`と`false`の二つの値があります。`!`は値を反転します。

```cerune
enabled: bool = true;
disabled: bool = !enabled;
```

`==`と`!=`は、同じ型の数値・真偽値・文字列・配列・構造体・直和型を比較できます。左右の式を左から一度ずつ評価し、複合値は添字順・フィールド宣言順で再帰的に調べます。直和型はタグと選択中のペイロードだけを比較し、最初の不一致で比較を終えます。数値型では、さらに`<`、`<=`、`>`、`>=`を使用できます。比較結果の型は常に`bool`です。

```cerune
same: bool = enabled == true;
small: bool = 1 + 2 < 4;
different: bool = 0.1f32 != 0.2f32;
```

`bool`の大小比較と算術演算はできません。また、比較でも暗黙の数値変換は行いません。

## 条件を組み合わせる論理演算

`&&`は両方が`true`の場合に、`||`は少なくとも一方が`true`の場合に`true`を返します。左右の式はどちらも`bool`でなければならず、結果も`bool`です。数値の0と1を真偽値として扱うことはありません。

左辺を一度だけ評価し、次の規則で右辺を評価するか決めます。この仕組みを短絡評価と呼びます。

| 演算 | 右辺を評価する条件 | 省略した場合の結果 |
| --- | --- | --- |
| `left && right` | 左辺が`true` | `false` |
| `left \|\| right` | 左辺が`false` | `true` |

```cerune
count: i64 = 0;
print(count != 0 && 12 / count > 2); // false。右辺の割り算は行わない
values: [i64; 2] = [4, 9];
index: i64 = 2;
print(index == 2 || values[index] == 9); // true。範囲外の要素は読まない
```

右辺が必要な場合も一度だけ評価します。省略された右辺の関数呼び出し、`print`などの作用、実行時検査は実行しません。左辺での失敗や、必要な右辺での失敗は通常どおり停止し、エラーを隠しません。

ただし、名前解決と型検査は左右とも行います。`false && missing`や`true || 1`は、右辺を実行しなくてもコンパイルエラーです。

優先順位は強い順に、単項演算、乗除算・剰余、加減算、シフト、大小比較、等値比較、`&`、`^`、`|`、`&&`、`||`です。したがって、`a < b && c == d || ready`は`((a < b) && (c == d)) || ready`として読みます。同じ演算子が続く場合は左から結び付け、丸括弧でまとまりを変更できます。`a < b < c`を範囲比較として扱うことはありません。

論理演算は条件文だけでなく、束縛、関数の引数・戻り値、配列要素、構造体のフィールドなど、`bool`の式を置ける場所で使えます。[短絡評価のサンプル](../../examples/short_circuit.ceru)に、配列走査の例があります。

Cerune IRには`and.short_circuit.bool`・`or.short_circuit.bool`として残します。bytecodeでは条件付きジャンプ、Cでは`&&`・`||`、LLVM・QBE・Windows x86-64では分岐、WATでは`bool`を返す`if`へ変換します。右辺を先に評価してから結果を選ぶ処理にはしません。

## 数値リテラル

整数リテラルは、接尾辞があればその型を使い、なければ周囲で期待される整数型を使います。手掛かりがなければ`i64`です。

```cerune
x: i64 = 42;
```

整数リテラルは、型が決まるまで10進数字の並びとして保持されます。型が決まった時点でその範囲を検査し、収まらない値はコンパイルエラーになります。符号は単項`-`として解析しますが、`-9223372036854775808`は`i64`の最小値として使用できます。

整数の接尾辞は`i8`、`u8`、`i16`、`u16`、`i32`、`u32`、`i64`、`u64`です。接尾辞のない数字には、宣言・再代入・引数・戻り値・field・配列要素の期待型を伝えます。外側から期待型がなければ、同じ算術式の型が決まった値を手掛かりにします。

```cerune
count: i32 = 4;
first: infer = count + 1;
second: infer = (1 + 2) + count;
explicit: infer = 3000000000u32;
default: infer = 1 + 2; // 手掛かりがないのでi64
```

`first`と`second`は`i32`で、左右の順序によって型を変えません。既に型が決まった変数や接尾辞付きリテラルは読み替えず、異なる型の演算はエラーです。整数を浮動小数点として読み替えることもしません。比較の結果は`bool`ですが、比較対象の整数型は別に解決します。

配列全体が`infer`の場合は、現在の規則どおり最初の要素から要素型を決めます。`[1i32, 2]`は`[i32; 2]`ですが、`[1, 2i32]`は最初が`i64`に決まるためエラーです。明示した`[i32; 2]`には`[1, 2]`を使えます。配列の添字は引き続き`i64`なので、`u32`の位置を使う場合は`values[i64(index)]`と書きます。

接尾辞のない浮動小数点リテラルは、明示的な浮動小数点型を文脈から得られる場合、その型として扱います。

```cerune
a: f32 = 0.1 + 0.2;
b: f64 = 0.1 + 0.2;
```

この違いは、バックエンドへのloweringより前にCerune IRで解決されます。

たとえば、Cバックエンドは次のように出力できます。

```c
float cerune_a = (0.1f + 0.2f);
double cerune_b = (0.1 + 0.2);
```

期待される浮動小数点型がない場合、接尾辞のない浮動小数点リテラルは`f64`になります。

```cerune
x: infer = 0.1 + 0.2;
```

この場合、`x`は`f64`として推論されます。

リテラルの接尾辞で型を明示できます。

```cerune
a: infer = 0.1f32 + 0.2f32;
b: infer = 0.1f64 + 0.2f64;
```

浮動小数点リテラルでは科学的記数法も使用できます。

```cerune
x: f64 = 1.5e-3;
```

## 型検査

四則演算では、左右のオペランドが同じ型である必要があります。剰余・ビット演算は整数型だけで使用できます。

```text
i8 op i8 -> i8
u8 op u8 -> u8
i16 op i16 -> i16
u16 op u16 -> u16
i32 op i32 -> i32
u32 op u32 -> u32
i64 op i64 -> i64
f32 op f32 -> f32
f64 op f64 -> f64
```

浮動小数点の各演算は、その型の精度へ最近接・偶数丸めします。乗算と加算を結果の変わるFMAへまとめません。符号付きゼロを保持し、NaNのpayloadの経路間一致は保証しません。[丸めの契約・外部compilerの条件・検証例](../design/float-rounding.ja.md)を参照してください。

整数の`+`、`-`、`*`と、符号付き整数の単項`-`は、結果がその整数型の範囲を超えた場合に実行を停止します。範囲の端から反対側の値へ暗黙に折り返しません。整数の0による除算と、符号付き整数の最小値を`-1`で割る演算も停止します。符号なし整数`u8`・`u16`・`u32`への単項`-`は禁止し、`-0u8`や`-0u32`もエラーです。整数除算は0方向へ丸めます。

Cerune VMでは、失敗した演算の種類、型、bytecode命令番号、ソース位置を診断します。生成されるC、LLVM IR、QBE IR、WebAssembly Text、Windows x86-64 assemblyにも対応する検査またはtrapが残るため、どこで範囲を守っているか観測できます。

Cerune v0.1は暗黙の数値変換を行いません。

たとえば、次の式は左右のオペランドが`i64`と`f64`であるため型エラーになります。

```cerune
x: infer = 1 + 0.1;
```

変数束縛に明示した型は、解決された式の型と一致するか検査されます。

```cerune
x: f32 = 0.1 + 0.2;
```

この例では、変数束縛に指定した`f32`が、接尾辞のない浮動小数点リテラルに期待される型を与えます。そのため式全体が`f32`として扱われます。

この決定はCerune IRに記録され、各バックエンドで繰り返し計算されることはありません。

比較演算でも左右のオペランドは同じ型である必要があります。比較する値の型と、結果の`bool`はCerune IRで別々に観測できます。

## 剰余とビット演算

ビットは、数を表す0と1の並びです。`u8`なら8個のスイッチのように考えられます。剰余とビット演算は、実装済みの8種類の整数型で使用できます。`bool`や浮動小数点には使えません。

| 演算 | 意味 | 例 |
| --- | --- | --- |
| `a % b` | 割り算の余り | `7 % 3`は`1` |
| `a & b` | 両方が1のビットだけを残す | `6u8 & 3`は`2u8` |
| `a \| b` | どちらかが1なら、そのビットを1にする | `6u8 \| 3`は`7u8` |
| `a ^ b` | 片方だけが1なら、そのビットを1にする | `6u8 ^ 3`は`5u8` |
| `~a` | 元の型の幅だけ0と1を反転する | `~0u8`は`255u8` |
| `a << b` | 左へbビット動かす。範囲外なら失敗 | `3u8 << 2`は`12u8` |
| `a >> b` | 右へbビット動かす | `12u8 >> 2`は`3u8` |

二項演算は左右が同じ整数型で、結果もその型です。シフト量も左辺と同じ型に揃えます。接尾辞のないリテラルには周囲の型を伝えられるので、`value: u8 = 1 << 3;`と書けます。既に型が決まった値を暗黙に変換することはありません。

剰余は、0方向へ丸める整数除算と整合します。0でない余りの符号は左辺と同じです。`-7 % 3`は`-1`、`7 % -3`は`1`です。右辺が0なら停止します。符号付き整数の最小値を`-1`で割った余りは`0`であり、除算そのものとは異なり成功します。

シフト量は、0以上、元の型のビット数未満でなければなりません。`0u8 << 8`も失敗します。`<<`は数学上の「2のb乗を掛ける」結果が元の型に収まる場合だけ成功します。`128u8 << 1`は0へ折り返さず失敗します。ビットを意図的に捨てる左シフトは別の操作として扱う方針で、まだ実装していません。

符号付き整数のビット演算は2の補数表現に従います。`>>`は符号を保つので、`-3i8 >> 1`は`-2i8`です。符号なし整数では左側を0で埋めます。右シフトでは下位ビットが落ちるため、通常の整数除算とは負数の丸め方が異なります。

これらの二項演算は左辺、右辺の順に一度ずつ評価します。左辺で失敗したら右辺は実行しません。`&`・`|`には論理演算のような短絡評価はありません。`0u8 & (1u8 % 0)`も失敗します。ただし、外側の`&&`や`||`が右辺全体を省略する場合は、その中のビット演算も実行しません。

比較はビット演算より優先されるので、ビットの有無を判定するときは`(flags & mask) != 0`と括弧を付けます。[ビットフラグ](../../examples/bit_flags.ceru)、[リングバッファ](../../examples/ring_buffer.ceru)、[部分和](../../examples/subset_sum_bits.ceru)に使用例があります。

Cerune IRとbytecodeには`rem.u8`、`bit_and.u8`、`bit_or.u8`、`bit_xor.u8`、`bit_not.u8`、`shl.checked.u8`、`shr.u8`のように、演算と型を残します。VMの失敗には元の`NodeId`と`Span`を対応付けます。型の範囲、シフト量、剰余の0除算を、演算の種類ごとに区別して診断します。

## 明示的な数値変換

値を正確に保つ数値変換には、同じ意味の二つの書式があります。

```cerune
value: i64 = 42;
compact: infer = i64(value);
explicit: infer = convert<i64>(value);
```

現在は`i8`・`u8`・`i16`・`u16`・`i32`・`u32`・`i64`・`u64`・`f32`・`f64`の全組み合わせで明示変換できます。値を変えずに変換先で表せる場合だけ成功し、表せなければ実行を停止します。切り捨て、丸め、折り返しはしません。`bool`、配列、構造体との変換は使えません。

```cerune
count: u32 = 3000000000;
wide: i64 = i64(count);
back: u32 = convert<u32>(wide);
```

`i32`から`i64`への変換は全値で成功しますが、`u32`から`i32`への変換は同じ32ビットでも失敗し得ます。変換元と変換先の型はVMの失敗理由にも残ります。
`value: i32 = 2147483648;`はリテラルのコンパイルエラー、`i32(2147483648)`は`i64`の値を評価した後の実行時の変換失敗です。

両方の書式で、括弧内の式を一度だけ評価します。入力へ変換先の型を伝えて計算の意味を変えることはありません。入力が数値以外ならコンパイルエラーです。引数は一つ必要で、末尾のカンマは使用できます。変換は値を返す式なので、単独の文としては使いません。

### 浮動小数点を含む変換

| 変換 | 成功する条件 |
| --- | --- |
| 整数から浮動小数点 | 同じ整数を変換先で正確に表せる |
| 浮動小数点から整数 | 有限の値で、小数部分がなく、範囲内で、マイナスのゼロではない |
| `f32`から`f64` | NaN以外。無限大とゼロの符号も保つ |
| `f64`から`f32` | NaNではなく、同じ値を正確に表せる。無限大とゼロの符号も保つ |
| 同じ型 | NaNの内部情報や符号のビットも含め、元の値をそのまま保つ |

```cerune
print(f64(1 / 2));       // 0: 整数の割り算をしてから変換
print(f64(1) / f64(2));  // 0.5: 小数で割り算
print(i32(3.0));         // 3
print(f32(1.5));         // 1.5
// print(i32(3.7));      // 小数部分がなくなるので失敗
// print(f32(16777217)); // 同じ整数を表せないので失敗
// print(f32(0.1));      // 入力のf64の値をf32で正確に表せないので失敗
// print(i32(-0.0));     // ゼロの符号がなくなるので失敗
```

検査するのは、括弧内の計算が終わった時点の値です。通常の浮動小数点の計算やリテラルの読み取りには、引き続き丸めがあります。変換を使っても、数学上の値を無限の精度で扱えるようにはなりません。例えば`f32(0.1f32)`は既に丸められた入力をそのまま保ちます。`f64(0.1f32)`はその値を正確に広げるため、`f64`のリテラル`0.1`とは異なります。

変換の失敗には、変換前後の型、理由、ソース上の位置が残ります。範囲外、精度の喪失、整数にできない無限大・NaN、型を変えるNaN、整数にできないマイナスのゼロを区別します。小数部分の切り捨ては、次の`trunc`で明示します。

### 小数部分を切り捨てる変換

`trunc<T>(value)`は`f32`・`f64`から整数型`T`へ、ゼロに近づく方向で変換します。`T`は`i8`・`u8`・`i16`・`u16`・`i32`・`u32`・`i64`・`u64`です。

| 入力例 | 結果 |
| --- | --- |
| `trunc<i64>(3.7)` / `trunc<i64>(-3.7)` | `3` / `-3` |
| `trunc<u8>(255.9)` / `trunc<i8>(-128.9)` | `255` / `-128` |
| `trunc<u64>(-0.9)` / `trunc<u64>(-0.0)` | どちらも`0` |
| `trunc<u8>(256.0)` / `trunc<u8>(-1.0)` | `conversion-out-of-range`で停止 |
| NaN・正負の無限大 | `conversion-not-finite`で停止 |

入力を一度だけ評価してから、非有限値、切り捨て後の整数範囲の順に検査します。入力の型は変換先から推論し直しません。整数の入力や浮動小数点の変換先はコンパイルエラーです。`convert<T>`・`T(value)`の正確さの検査は変わりません。

`const`の評価と型引数を指定する関数でも使えます。`trunc`はキーワードではなく、通常の関数名や変数名にも使えます。組み込みの`trunc<T>(value)`とユーザー関数の`trunc::<T>(value)`は構文で区別します。

IR・bytecodeには`convert.trunc`、バックエンドIRには`Rounded { rounding: Truncate, overflow: Checked }`を保持し、型・ソース位置とともに観測できます。[設計](../design/truncating-conversions.ja.md)と[実行例](../../examples/truncating_conversions.ceru)を参照してください。

### 丸め方と飽和を選ぶ

`trunc<T>`と同じ入力・変換先の条件で、次の操作も使えます。入力を一度だけ評価し、丸めた後に整数型の範囲を調べます。

| 操作 | 規則 | `2.5` / `-2.5` |
| --- | --- | --- |
| `floor<T>(x)` | 小さい整数へ | 2 / -3 |
| `ceil<T>(x)` | 大きい整数へ | 3 / -2 |
| `round<T>(x)` | 最寄り。同距離ならゼロから遠い方 | 3 / -3 |
| `round_ties_even<T>(x)` | 最寄り。同距離なら偶数の方 | 2 / -2 |

通常はNaN・無限大で`conversion-not-finite`、丸め後の範囲外で`conversion-out-of-range`となります。負のゼロは整数の0になります。型・引数個数の誤りはコンパイルエラーです。

全5操作に`saturating_`付きの形もあります。例えば`saturating_round<u8>(300.0)`は255です。下限未満・負の無限大は最小値、上限超過・正の無限大は最大値、NaNは0へ写します。入力式自身の失敗は捕捉しません。

丸め方と範囲外の方針はAST・IR・bytecode・backend IRで保持し、テキストにも`convert.round` / `convert.saturating_round`のように区別して出力します。定数評価では元の式と結果をIRに残します。[生成手順と観測の設計](../design/rounding-conversions.ja.md)、[丸め方](../../examples/rounding_conversions.ceru)、[飽和](../../examples/saturating_conversions.ceru)の例を参照してください。名前はキーワードではなく、通常の関数呼び出し・比較・ユーザー定義の型引数付き呼び出しでも使えます。

### 変換を観測する

入力側の計算で失敗した場合は、その計算の位置を報告します。例えば`i64(1 / 0)`は変換に到達する前に除算で停止します。

Cerune IRでは変換前後の型、入力、元の書式とソース位置を保持します。二つの書式は同じ種類の処理で、書式の違いは出自情報です。整数同士は従来の`ConvertInteger`と`convert.checked i32 -> u32`、浮動小数点を含む変換は`ConvertNumeric`と`convert.exact i64 -> f64`で表します。どちらも対応するCerune IRの`NodeId`と`Span`を命令の出自として保持します。

C・LLVM・QBE・WAT・Windows x86-64では整数を64ビットで保持し、小さい整数型への変換には範囲検査を生成します。浮動小数点を含む変換はバックエンドIRにも型付きの操作として残し、表現を変えてよいか検査する処理を生成します。同じ型への変換は生成コードに命令を追加せず、Cerune IRとbytecodeには明示変換を残します。整数から`i64`への変換にも追加の実行処理は不要ですが、浮動小数点から`i64`への変換には検査が必要です。

`convert`はキーワードではありません。`convert(value)`という通常の関数呼び出しや、変数を比較する`convert < limit`は引き続き使えます。`convert<型>(式)`の形は組み込みの変換で、同名のユーザー関数の定義によって意味は変わりません。ユーザー定義の型引数付き関数は`name::<型>(式)`の書式で呼び出します。

## 出力

`flush();`は標準出力の保留中の出力を接続先へ渡す組込み文です。引数・戻り値はなく、`flush`は予約語です。失敗した場合は後続処理を実行せず、操作とソース位置を診断します。画面描画や永続保存は保証しません。HIR・MIR・SSA・VMと全生成経路に対応し、操作は`flush.stdout`として観測できます。[契約と対応範囲](../design/standard-io.ja.md)、[実行例](../../examples/io_protocol/explicit_flush.ceru)を参照してください。

マイナスのゼロは`-0`と表示します。符号を隠して`0`に書き換えることはしません。

`print(expression);`は全経路で真偽値・数値・`string`・固定長配列・構造体・直和型を受け付けます。複合値は`[1, 2]`・`{x: 1}`・`Found{text: "空"}`の形で表示し、入れ子の文字列を引用・エスケープします。引数を完全に評価してから表示を開始し、最後にLFを一つ付けます。[表示形式と生成手順](../design/aggregate-values.ja.md)を参照してください。

Ceruneは、観測対象となる浮動小数点数の挙動が見えるだけの精度を保って出力します。

現在の出力形式は次のとおりです。

```text
bool   `true`または`false`
i8 / u8 / i16 / u16 / i32 / u32 / i64 / u64    整数として出力
f32    有効数字9桁
f64    有効数字17桁
```

有効数字は、小数点以下だけでなく整数部分も含めた桁数です。有限の値は、表示を同じ型で読み戻したときに元の値になる精度で出力します。末尾の不要な小数のゼロは省きます。

有効数字で丸めた後の10進指数が`-4`未満、または有効数字の桁数以上なら、`e`を使う指数表記にします。それ以外は通常の小数表記です。指数には符号と最低2桁の数字を付けます。例えば`1e-20`は`9.9999999999999995e-21`と表示され、`0`にはなりません。これは約10のマイナス20乗を意味し、長い数字の部分は`f64`に保存された近似値を表しています。

表示は値を変更しません。`1.0 + 1e-20 == 1.0`が`true`になるのは計算時の丸めであり、表示が小さな値を消しているわけではありません。[小さな数値の例](../../examples/small_values.ceru)で確認できます。

VMはこの規則で整形し、生成C・LLVM・QBE・Nativeは`snprintf`の`%.9g`・`%.17g`を使います。WATは`format_f32_v1`・`format_f64_v1`から同じ規則の整形済みバイトと全長を受け取り、検査後に`write_stdout_v1`で書き込みます。

WATの整数・真偽値は生成コードで整形します。文字列・引用・区切りを含むすべての通常出力に[版付きhost契約](../design/wat-output-host.ja.md)を使い、部分失敗時の要求/受理数と出自を保持します。内部メモリは公開しません。

VMの無限大は`inf`・`-inf`、NaNは`NaN`と表示します。生成コードでの特殊値の綴りは対象の実行環境に依存します。NaNの内部情報は`print`の十進表示では区別できません。

たとえば、次の`f32`の計算では、短く丸められた小数ではなく浮動小数点数による近似値が見える場合があります。

```cerune
x: f32 = 0.1 + 0.2;
print(x);
```

各バックエンドは、自身の対象環境の規約に従って実装しながら、この観測可能な出力の振る舞いを保つ責任を持ちます。


## コンパイル入力の上限

構文・式の深さと、ファイルの個別バイト数・合計・件数を検査します。超過時は診断して停止し、既存成果物を保持します。既定値とRust APIの指定方法は[コンパイル入力の上限](../design/frontend-limits.ja.md)を参照してください。
