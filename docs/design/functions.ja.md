# 関数の設計

[English](functions.en.md)

Status: Implemented

この文書は、Ceruneの関数、呼び出し、戻り値、entry pointの設計判断と観測上の約束を記録します。現在の構文は[言語リファレンス](../reference/language.ja.md)で定義します。

## 目的

関数は処理を再利用するためだけの機能ではありません。Ceruneでは、一つの処理がソースからCerune IR、backend IR、成果物へ変わる単位でもあります。

守ることは次の三つです。

- 関数名、parameter、呼び出し、戻り値の対応を変換後も追える
- backendのABIをCeruneの意味と混ぜない
- 観測用の識別子からcompilerや実行中の値を変更できない

## 現在の意味

```cerune
fn add(left: i64, right: i64) -> i64 {
    return left + right;
}
```

- `fn`で関数定義を明示する
- parameterと戻り値は具体型を明示する
- 戻り値がなければ`void`を明示する
- 値を返すときは`return expression;`を明示する
- parameterは不変のローカル束縛として扱う
- 名前付きproduct typeと固定長配列もparameterや戻り値に使える
- 関数はトップレベルの実行時束縛を参照しない
- 関数名はファイル全体で先に登録し、forward callを認める

明示的な`main`は`fn main() -> void`だけを認めます。トップレベル実行文があるプログラムではcompilerがentry pointを生成するため、両方は同時に使えません。

## 関数をまたぐ値

scalar、名前付きproduct type、固定長配列は、すべて値として関数をまたぎます。

```cerune
type Point { x: i64, y: i64, }

fn move_x(point: Point, amount: i64) -> Point {
    return Point { x: point.x + amount, y: point.y, };
}

original: Point = Point { x: 2, y: 3, };
moved: Point = move_x(original, 5);
print(original.x); // 2
print(moved.x);    // 7
```

`move_x`が受け取る`point`は、呼び出した側の`original`とは別の値です。関数から返るproduct typeや配列も、呼び出した側の新しい値になります。関数の内外で、同じ変更可能な場所を知らないうちに共有することはありません。

動的文字列を含むプログラムでは、呼び出し側が準備した引数の所有を関数へ一度だけ渡します。未使用の引数もcalleeが解放し、戻り値の所有は解放前に確保します。[呼び出し境界の所有](dynamic-data.ja.md#呼び出し境界の所有)と[実行例](../../examples/owned_arguments.ceru)で確認できます。

## 観測できるもの

| 段階 | 保持する情報 |
| --- | --- |
| AST | ソース上の名前、型名、block、Span |
| Cerune IR | `FunctionId`、`BindingId`、解決済み型、構造化されたcallとreturn |
| Bytecode | 関数番号、parameter slot、関数内命令番号、callの引数数、returnの有無 |
| Backend IR | backend固有のlocal、temporary、control flow、呼び出し表現 |
| 成果物 | 関数symbol、引数表現、stackやmemory、実際のcallとreturn |

VMエラーは、関数番号とその関数内の命令番号を保持します。命令の出自からソースのSpanへ戻れるため、関数内の実行時エラーもentry pointの命令と混同しません。

## Backendの境界

以下では、複数の値をまとめたproduct typeと固定長配列を「集約値」と呼びます。

- Cは型付き関数とprototypeを生成し、Cの値として受け渡す
- LLVM IRは型付きparameterと戻り値を生成し、受け取った値を観測しやすいlocal slotへ保存する
- QBE IRは集約引数の場所を受け取り、関数の開始時に自身のstack領域へコピーする。集約戻り値には、呼び出し側が用意した戻り先を隠れた先頭引数として使う
- WebAssembly TextはQBEと同じ形をlinear memory上のaddressで表す。呼出しごとの非公開領域を確保し、内部の末尾引数で基点を渡す。戻り値をコピーしてから領域を再利用可能にする
- Windows/Linux x86-64はscalarと集約値の場所を、明示したターゲットの引数registerまたはstackへloweringする。集約引数は受け取り側でコピーし、集約戻り値の戻り先は内部規約として`RAX`で渡す
- Cerune bytecodeとVMは呼び出しごとに独立したframeを作り、集約値を複製して渡す

Cerune IRはABI register、stack offset、隠れた戻り先を決めません。これらはbackend loweringの判断であり、成果物で観測します。QBE、WebAssembly、Windows/Linux x86-64が内部でaddressを渡しても、それは値をコピーするための実装です。Ceruneの参照型や外部ABIを定義するものではありません。

## 多い引数と可観測性

[function_arguments.ceru](../../examples/function_arguments.ceru)では、7引数の入れ子呼び出しと、数値・文字列・配列・構造体を混ぜた11引数を実行します。引数は左から一度ずつ評価し、途中で失敗したら後の引数と関数本体は実行しません。短絡評価で呼び出しを省く場合は、引数も評価しません。

直接ASMと自前エンコーダは同じ引数配置を使います。Windowsは先頭4個を位置別のregister、5個目以降を32バイトのshadow spaceの後へ置きます。Linux/SysVは整数・アドレス用の6個と浮動小数点用の8個のregisterを別々に数え、溢れた引数を元の引数順でstackへ置きます。一方が埋まっても他方の空きregisterは使えます。ターゲットは明示指定し、実行中のOSから選びません。

評価済み引数の一時領域と、次の呼び出しに渡すstack領域は分離します。呼び出し時のstackは16バイト境界に揃え、f32/f64は変換せずビットを渡します。集約値のコピーと`RAX`の戻り先はCerune内部の規約であり、外部関数とのABI互換を約束しません。配置は生成ASMと逆アセンブルから観測できますが、Ceruneのプログラムにそのaddressや変更権限を公開しません。

`cargo test --test source_files many_argument`はVMの既知の出力を基準にC・LLVM・QBE・WAT・直接ASM・自前オブジェクトを照合します。混在する22引数、整数境界値、負のゼロ、600要素の配列、コピーの独立性、失敗した式の位置と先行出力を確認します。必要な外部ツールを指定したLinux/Windows CIでも実行します。

registerとstackの基本規則は[Microsoft x64 calling convention](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention)と[System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI)を参照してください。

型・長さをまたぐ再利用には[ジェネリック関数](generic-functions.ja.md)を使えます。具体化後はこの文書の値渡し・ABI・停止の規則を適用します。

## 現在の制約

引数の個数に言語としての固定上限は設けません。scalar、名前付きproduct type、固定長配列をparameterと戻り値に使えます。

有限のstack・memoryと外部ツールの制約は引き続き適用されます。可変長引数やデフォルト引数を導入する変更ではなく、宣言と呼び出しの引数数・型は完全に一致させます。

再帰は直接呼び出しと間接呼び出しの両方を引き続き診断します。WATの集約値・一時領域は呼出しごとに分離しましたが、全経路共通の呼出し深さ上限とcall stack診断は未実装です。[再帰の設計と実装順](recursion.ja.md)で解禁条件を管理します。

再帰を追加するときは、少なくとも次を同時に満たします。

- すべてのbackendで呼び出しframeが独立する
- product型のlocalとtemporaryも呼び出しごとに独立する
- 観測結果からcall stackと出自を区別できる
- stackまたはmemory消費へ明示的な上限を設けられる

## セキュリティ境界

`FunctionId`、`BindingId`、命令番号は、一回のcompile結果の関係を読むための識別子です。関数を差し替えるhandle、実行中のframeを変更する参照、compiler内部へ書き戻す権限ではありません。

生成物に現れる集約値のaddressも同様です。Ceruneのプログラムはそのaddressを値として取り出したり、後で使うために保持したりできません。受け取った関数は最初に自身の領域へ値をコピーするため、呼び出し側の値を書き換える経路にもなりません。

将来、外部関数やpluginを追加する場合も、観測APIと実行・変更権限は別に設計します。観測機能を有効にしただけで呼び出し先や生成結果が変わってはいけません。
