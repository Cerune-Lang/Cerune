# MIRからNativeへの変換

[English](native-mir.en.md) · [段階設計](ir-stages.ja.md)

## 実装した経路

Windows/LinuxのASM・自前COFF/ELFは、完成済みHIR→非SSAのMIR→x86-64 LIR→ASMを通ります。旧HIR→Native変換は取り除きました。名前・型・generic・match・所有規則をNativeで解釈し直しません。

| 段階 | 担当 |
| --- | --- |
| HIR→MIR | 左からの評価、短絡、loopの辺、所有処理の順番。既存の[共通変換](../../src/mir/lower.rs)を使用 |
| MIR→LIR | MIRを検証し、局所値のstack配置、明示したtargetの引数・戻り値ABI、機械操作を決定 |
| LIR→ASM | レジスタ操作、prologue/epilogue、検査・trap・runtime補助処理を展開 |
| ASM→Object | 同じASMを自前で符号化してCOFF/ELF・再配置・シンボルを生成。リンクは別操作 |

LIRの検査命令はemitterで複数の機械命令へ展開されます。LIRの命令番号を最終機械命令番号と同一視しません。ターゲットの既存ルールを維持し、実行中のOSから選びません。

LIRの内部Moduleを経由する生成は実装済みですが、LIR専用のテキスト出力・bundle内snapshot・構造検証器・直接実行器はありません。現在はASM注釈とObjectシンボルから対応を追います。[観測・実行・検証の棚卸し](development-status.ja.md)と[#123](https://github.com/Cerune-Lang/Cerune/issues/123)で残件を分けています。

## 明示的にSSAを経由する

`emit-asm --ssa`／`emit-obj --ssa`はHIR→元MIR→SSA→copy付きMIR→LIRを通ります。指定なしの非最適化経路は維持します。[SSA変換](mir-ssa.ja.md)の`mir::ssa::lower(&ssa)`が返す`program`を、既存のMIR入力APIへ渡します。

`observe --ssa`では元MIR・SSA・変換後MIR・両段階の対応と、基準／SSA経由の両ASMを保存します。追加した一時値・辺上のcopy・引数準備からLIR／Objectまで辿れます。最適化や暗黙の所有copyは追加しません。

## 意味と配置の判断

MIRの全blockを決定的な順で配置し、jump/branchの行き先をそのまま変換します。未到達blockも残します。明示的なmain呼出しはMIRのcallを使い、Native側で関数名から追加しません。

| 操作 | Nativeで維持すること |
| --- | --- |
| 局所値・一時値 | 型に応じたstack領域へ保存。固定配列・productはフィールド配置に従って値をコピー |
| 動的配列・文字列の読取り | 内部参照の保存だけで、暗黙の深いコピーやretainをしない |
| CheckIndex→Store | 各添字の評価後に検査し、要素アドレスを保存。右辺の評価後に検査を移さず、そのアドレスへ書く |
| 確保・初期化・retain・release/free | MIRにある順番を維持。配列と文字列の論理予算も引き継ぐ |
| 数値・表示・停止 | 既存のNative命令とruntimeを使用。幅・符号・丸め・出力バイト・失敗コード・元の出自を維持 |
| 呼出し・戻り値 | Windows/System Vのレジスタ・stack引数配置を再利用。集約戻り値のRAX規約はCerune内部用 |

局所領域の再利用・レジスタ割当・暗黙の最適化は行いません。一時値が明示されたため、旧変換よりstack使用量や命令数が増える場合があります。実行比較で意味を確認しており、同じ命令バイト列や性能を約束する移行ではありません。

## APIと観測

Rust APIは次の形です。既存のHIR入力APIとCLIも内部でこの経路を使います。

| API | 結果 |
| --- | --- |
| `x86_64::lower_mir(&mir, target)` | target、frame、命令列、元の出自、MIR対応を持つLIR |
| `x86_64::emit_asm_from_mir(&mir, target, annotate_origins)` | 同じMIR snapshotからのASM |
| `x86_64::emit_object_from_mir(&mir, target, annotate_origins)` | 同じMIR snapshotからの自前Object |

MIRの`uses_strings`は、定数式から実行時の文字列操作が消えた場合も入力の文字列利用を保持します。`emit-mir`の`source-strings`に出力し、Windowsのバイト出力設定を維持します。検出処理は[IR共通側](../../src/ir/string_usage.rs)に置き、既存backendと共有します。

入力snapshotを変更しません。MIR検証の失敗はDiagnosticとして返します。検証器はheapのalias・寿命を完全に静的保証するものではなく、任意の外部MIRを安全に実行するloaderではありません。

`--annotate-origins`は従来のソース出自に加え、次の対応を残します。

- `# cerune-mir: v1 fn_0 bb2 i17 -> lir 45 (source)`：関数・block・MIR命令から、その関数のLIR命令番号への対応。数字は説明例です。
- `cerune_origin_mir_fn_0_bb2_i17_lir45`：同じ対応を保持するObjectシンボル。
- block先頭は`iN`の代わりに`block`。Derived/Syntheticの理由もコメントへ残す。
- ABIの入口・出口は`synthetic ABI setup/exit`。MIR操作が存在しない補助処理と区別する。

一つのMIR命令は複数のLIR命令へ対応します。MIR API上の幅0の値操作には、バイトを生成しない`ObserveOnly`を残します。これはソース言語へ0長固定配列を追加する変更ではありません。注釈を付けても機械命令バイトは変わりません。出力にはソース本文・パス・時刻を追加しません。

## Exampleと検証

[native_calls.ceru](../../examples/ir_stages/native_calls.ceru)で、7個の混在型引数、loopのcontinue/break、mainの一度だけの呼出しを確認できます。コマンドと出力の説明は[exampleのREADME](../../examples/ir_stages/README.md)にまとめています。

移行前後のNative実行をWindows/Linuxで比較し、既存example、数値境界、異常停止コード・NodeId/SourceId/Span・先行出力、文字列のバイト列、ABIを検証します。[動的配列](../../tests/dynamic_arrays.rs)はHIR・MIR・VM・C・LLVM・QBE・WAT・ASM・自前Objectの9経路で比較します。[Native MIRテスト](../../tests/native_mir.rs)はMIRだけの変更の反映、全命令の出自対応、不正なMIRの拒否も確認します。

生成ASMの期待ファイルは新しい配置に更新しています。既知出力・失敗記録との実行比較と、生成テキストの固定は別の検証です。形式証明・SSA・最適化・観測bundleはこの移行の完成範囲に含めません。後続の[観測bundle](observation-bundle.ja.md)とSSA連携は実装済みで、一回のコンパイルの基準／SSA経由の表現を別々に保存できます。[SSA Nativeテスト](../../tests/ssa_native.rs)で並列受渡し・引数・所有・停止出自をASM／自前Objectの実行と照合します。
