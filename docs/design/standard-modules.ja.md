# 仮想標準モジュール

[English](standard-modules.en.md)

行入力の公開APIは `import "std:io" as io;` と `io::read_line(4096)` とします。標準モジュールはコンパイラに同梱した仮想ソースであり、実ファイルを探しません。print・flushは変更しません。

## 解決と同一性

- `std:`で始まるimportを標準モジュールとして分類し、現在は完全一致の`std:io`・`std:env`・`std:num`・`std:fs`を認識します。未知の識別子は診断し、ファイル読取りへフォールバックしません。
- 相対`.ceru`パスの解決規則は維持します。標準モジュールのための環境変数・検索パス・ネットワーク取得・一般的なパッケージ管理は追加しません。
- 1コンパイルにつき各標準モジュールを一度だけ登録し、別名や別ファイルからのimportでも同じ型定義へ解決します。公開型は通常のenumであり、backendごとの専用型を作りません。
- `modules::from_source`はメモリ中のソースと同梱標準モジュールだけを解決します。相対ファイルimportは明示的に拒否します。compile(&str)・compile_to_ir(&str)はこの処理を使い、暗黙のファイルI/Oを導入しません。単独のSourceFileを受けるcompile_source系は従来どおりimportを解決せず、標準ソースを含むSourceMapが必要な場合はmodules::from_sourceを使います。

## SourceIdと観測

SourceIdはコンパイル内での登録順の識別子です。入口を先に登録し、依存を宣言順に辿ります。std:io専用の固定数値IDは予約せず、別コンパイルの数値一致を前提にしません。

SourceMapには利用者ソースと標準ソースの種別を保持します。標準ソースの識別子は`std:io`・`std:env`・`std:num`・`std:fs`で、いずれも契約版は1です。emit-sourcesに本文・SourceId・標準種別・識別子・版を出し、ファイルパスらしい偽の名前を使いません。標準モジュールを使わない既存の単一ソースの匿名Spanとmanifestは維持します。

入力命令のSpanは同梱関数内の操作を指します。呼出し側のHIR Callは利用者のSpanを保持し、生成過程から呼出し先と入力操作を辿れます。仮想ソースの診断を利用者ファイルの同じバイト位置へ誤って表示しません。

## 行／EOFと共通IR

公開型`ReadLine`は`Line { text: string }`と`End`の既存enumです。同梱関数の本体は観測できます。読取りそのものだけを共通IRの入力操作とし、通常のenum/matchと所有の展開を再利用します。

内部入力操作は厳密UTF-8の文字列を返します。空文字列は確認済みEOFの内部表現です。LF/CRLFを保持するため空行は必ず非空となり、同梱関数がこの表現をLine/Endへ変換できます。内部操作を利用者の名前として公開せず、標準ソース中の専用呼出しを内部AST名へ解決します。

[行入力契約](line-input.ja.md)の上限・消費数・失敗・予算・出自は変えません。生成経路までの実装状況は同文書で管理し、未対応を成功やEOFへ置き換えません。

## 実行と確認

HIR・MIR・SSAの `run_with_io` とVMの `run_bytecode_with_io` に、InputとOutputを明示的に渡します。従来の収集型APIは入力未設定を維持します。[CLIと生成C・LLVM・QBE・Native](stdin-connections.ja.md)は実stdinへ接続します。[WATの版付きhost接続](wat-input-host.ja.md)も実装済みです。Nativeは通常／SSA経由のMIRからLIR・ASM・自前Objectへ入力を展開します。未使用関数も除去しないため、std:ioのimportだけでも生成物に入力処理が含まれます。

[行入力のCerune例](../../tests/fixtures/line_input.ceru)は、print→flush→read_line→matchを実行します。[比較テスト](../../tests/standard_input.rs)が独立した同じ入力を4実行器へ供給し、[期待出力](../../tests/fixtures/line_input.stdout)・操作順・失敗・所有を確認します。[実stdinのexample](../../examples/stdin/README.md)も全経路で検証します。

```sh
cargo test --test standard_input
cargo run -- emit-sources tests/fixtures/line_input.ceru
cargo run -- emit-ir tests/fixtures/line_input.ceru
```

## プログラム引数 std:env

arg_count／arg／Argumentの宣言を同梱し、同じSourceMap・名前解決・enum展開を使います。HIR・MIR・SSA・VM・CLI・生成C・LLVM・QBE・WAT・Nativeを接続済みです。[契約と残件](dogfooding-inputs.ja.md)・[実行例](../../examples/arguments/README.md)を参照してください。

## 純粋な整数読取り

`std:num`は`parse_i64(string) -> ParseI64`を提供します。判定は同梱のCerune本文で行い、共通IRの内部バイト参照だけを各経路へ追加します。[書式・範囲・出自の契約](integer-parsing.ja.md)を参照してください。

## ファイル操作 std:fs

`read_text`／`write_new`の宣言も同梱仮想ソースです。共通IRを通じ、HIR・MIR・SSA・VMに明示したファイル接続を使います。Windows/LinuxのCLIは`--file-root`で実接続を指定します。生成Cは専用マーカー付き起動で接続を指定します。LLVM／QBE／WAT／Nativeは未実装です。[契約とRust API](file-io.ja.md)・[保存・読戻しのexample](../../examples/files/README.md)を参照してください。
