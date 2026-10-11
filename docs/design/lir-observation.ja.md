# LIRの観測・構造検証

[English](lir-observation.en.md)

Native生成で使っているx86-64 LIRを、そのまま表示・保存・構造検証します。新しい意味表現や実行経路は作りません。対応Issueは[#123](https://github.com/Cerune-Lang/Cerune/issues/123)です。

## 実装した範囲

同じModuleをテキスト表示とASM生成に渡します。基準経路とSSA経由の生成物を分けて残します。

| 操作 | 入力と結果 |
| --- | --- |
| `emit-lir <file> --target <triple> [-o <file>]` | HIR→MIR→LIRの観測テキスト |
| `emit-lir <file> --ssa --target <triple>` | HIR→元MIR→SSA→copy付きMIR→LIRの観測テキスト |
| `observe <file> --lir --target <triple> -o <new-directory>` | 既存のbundleに基準LIRを追加。schemaはv4 |
| `observe <file> --ssa --lir --target <triple> -o <new-directory>` | 基準LIRとSSA経由LIRを両方追加。schemaはv4 |
| `x86_64::lir::validate(&module)` | 内部snapshotを構造検証 |
| `x86_64::lir::emit(&module)` | 検証後に観測テキストを生成 |
| `emit_asm_from_lir`／`emit_object_from_lir` | 検証後、同じModuleからASM／自前Objectを生成 |

targetはWindows/Linuxのx86-64 tripleを明示します。ホストOSから補いません。既存のheap予算オプションを使えます。LIR変換自体も末尾で検証します。既存の低水準API `x86_64::emit` は検証済みModule用です。

観測テキストの見出しは`cerune-lir-observation-v1`です。target、heap予算、runtime利用、UTF-8バイト数付き文字列、浮動小数点定数のbit値、関数ID・名前・frameサイズ、命令番号・operand、SourceId/NodeId/Span、MIR block・命令・派生理由を含みます。命令・operand・出典は内部Rust型のDebug表記です。時刻や外部環境には依存せず、同じコンパイラ・同じsnapshotで決定的です。安定したserialization、load形式、配布Imageではなく、内部型の変更に伴い表示は変わり得ます。

命令番号は関数内の順番です。ASMの`cerune-mir`注釈のLIR番号と一致します。一つのLIR命令が複数のASM命令へ展開される場合があります。MIRに対応しないABI準備等は`mir: None`、ソースに直接対応しない操作は`Synthetic`です。幅0の値には、機械バイトを作らない`ObserveOnly`を残します。

## 検証することと限界

検証は破損した内部snapshotを早期に見つけるためのものです。

| 対象 | 検証 |
| --- | --- |
| 出典 | 命令と2種類の出典配列の長さ、MIR出典から得たソース出典との一致 |
| 関数・分岐 | 関数IDとindex、呼出先、block labelの重複と参照、展開用label IDの重複 |
| 定数 | 文字列index、f32/f64の種類別IDと重複 |
| frame | 16byte整列、signed 32-bit変位の範囲、負方向の8byte slot配置 |
| stack operand | 単一slot／集約値／固定配列の範囲、サイズ計算のoverflow、呼出引数領域との重なり |
| ABI | target別の受取register番号、stack引数の整列・shadow space・変位範囲 |
| runtime | 文字列・配列・writeに必要な設定、writeの種類、SysV出力命令のtarget |

値の初期化、pointer先の有効性、所有の寿命、呼出先との型・引数数の完全一致、元MIRに対する出典IDの存在、ASM展開・encoder・実ABIの意味の正しさは保証しません。元MIRやソース本文はLIRに重複保存しないため、出典の実在やUTF-8範囲の照合にはbundleの他段階も必要です。未信頼入力を安全に実行するloaderとしては使いません。

## 比較例

[7引数と構造体戻り値](../../examples/ir_stages/lir_layout.ceru)は`6`、`22`、`LIR → ASM`を出力します。次の操作で引数のregister/stack配置、集約戻り値の保存先、元のソース位置を追えます。

```sh
cargo run --quiet -- run examples/ir_stages/lir_layout.ceru
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-pc-windows-msvc
cargo run --quiet -- emit-lir examples/ir_stages/lir_layout.ceru --target x86_64-unknown-linux-gnu
cargo run --quiet -- observe examples/ir_stages/lir_layout.ceru --ssa --lir --target x86_64-unknown-linux-gnu -o target/lir-observation
```

7個の整数引数のうち、Windowsでは5個目から、SysVでは7個目からstackに置きます。集約戻り値のポインタはCerune内部規約でRAXへ渡します。C向け公開ABIを宣言する例ではありません。

全実行exampleについて両target・基準／SSA経由の構造検証と決定性、同じLIRからのASM一致をテストします。破損例の拒否、既存bundleの内容維持、CLIとの一致も検証します。実行比較は[Native MIRテスト](../../tests/native_mir.rs)と[SSA Nativeテスト](../../tests/ssa_native.rs)で、出力・意図した停止・停止前出力・出典を確認します。Windows/Linuxそれぞれの実行は各OSのテストで扱います。

## LIR直接実行器の採否

**現段階では保留します。観測・検証が揃ったので、言語機能へ戻ります。**

LIRを直接実行するには、RAX/RCX等とXMMの値、stack frame、pointerとメモリ、内部呼出ABI、runtime補助操作を独立にモデル化する必要があります。MIR実行器へ戻して動かすだけではLIRを検証したことになりません。現在のLIRは検査・配列・文字列等の擬似命令も含み、prologueや補助runtimeの詳細はASM emitterで展開します。LIR Executorが一致しても、ASM展開、自前encoder、Object relocation、外部リンカ、OS上の実ABIの正しさまでは確認できません。

独立実行には、実機なしでのtarget別比較やMIR→LIRの差分局所化という利点があります。ただし現在はMIR/SSAの独立実行とNative実行比較があり、追加モデルの保守費用に対する具体的な検証不足がまだありません。今後、LIR特有の不具合を既存比較で局所化できない場合、または機械命令へ近づけた表現を設計する際に再検討します。

型付きmachine IR、汎用emulator、JIT、自前linkerは別件です。LIR Executorや最適化の完成を`array_repeat`・標準入出力・build/releaseの前提にはしません。
