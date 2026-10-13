# 観測bundle

[English](observation-bundle.en.md)

## 目的と範囲

[#60](https://github.com/Cerune-Lang/Cerune/issues/60)の最初の実装として、一回のfrontend処理からHIR、非SSAのMIR、注釈付きNative ASMを保存します。manifestは索引であり、新しいIRではありません。

`cerune observe <file> [--ssa] [--lir] --target <triple> -o <new-directory>`で保存します。Windows/Linuxのx86-64 targetは必須です。既存のheap予算オプションも使えます。

| ファイル | 内容 |
| --- | --- |
| `manifest.json` | `cerune-observation-v1`、Cerune版、target、両heap予算、最適化pass一覧（現状は空）、実行していないこと、成果物の相対パス |
| `sources.json` | ファイル名・正確な本文・SourceId。既存の`emit-sources`と同じ形式 |
| `program.ceir` | 完成済みHIR |
| `program.mir.txt` | 同じHIRから一度だけ変換・検証したMIR |
| `program.origins.s` | 同じMIRから生成したASM。HIR出自とMIR→LIR対応を注釈として保持 |

## SSAを選んだ場合

`--ssa`を付けると同じ非最適化MIRからSSAを構築し、次の5ファイルを追加します。元の4成果物は指定なしとバイト単位で同じです。

| 追加ファイル | 内容 |
| --- | --- |
| `program.ssa.txt` | `emit-mir --ssa`と同じ観測テキスト。元MIRの全文とSSAを別区分で保持 |
| `program.ssa-map.txt` | `Cerune SSA mapping v0.1`。関数・引数・値／slot・block・命令・terminator・辺の受渡し・未到達保持の対応 |
| `program.ssa-lowered.mir.txt` | SSAから専用Temporary・並列copyへ戻したMIR |
| `program.ssa-lowered-map.txt` | SSA→MIRの値・block・辺上のcopy・関数引数・未到達保持の対応 |
| `program.ssa.origins.s` | 変換後MIRを入力とする、注釈付きNative ASM |

対応の値番号は関数内、命令indexはblock内で解釈します。then／elseが同じ行き先でも辺を別に記録し、合流先の値と元局所値を示します。元命令IDと出自は元MIRのsnapshotへ戻る参照です。対応表は意味保存の証明やload形式ではありません。

SSA付きmanifestは`cerune-observation-v3`です。v2のSSA観測にSSA→MIR／Nativeを追加しました。指定なしは既存のv1を維持します。

| v3の追加項目 | 内容 |
| --- | --- |
| `transformations` | 0からの順序、`kind=representation`、版付きpass名、`options={}`。`scalar-ssa-v1`: `mir`→`ssa`／`ssa_mapping`、`ssa-lower-v1`: `ssa`→`ssa_lowered_mir`／`ssa_lowering_mapping`のartifact key |
| `artifact_inputs` | 各生成物の入力。基準の`assembly`は`mir`、追加の`ssa_assembly`は`ssa_lowered_mir`から生成 |
| `artifacts`の追加key | `ssa`・`ssa_mapping`・`ssa_lowered_mir`・`ssa_lowering_mapping`・`ssa_assembly`と上記ファイルの相対パス |

`optimization_passes`は空、`executed`はfalseのままです。基準の非最適化NativeとSSA経由Nativeを別々に観測できます。Objectはbundleに同梱せず、`emit-obj --ssa`で個別に出力します。

## 同じコンパイルであること

入力・importを一度読み込み、共通frontendを一度実行します。そのHIRとMIRのsnapshotを後続生成へ渡し、CLIを繰り返し起動して収集しません。型・名前・所有規則の再解釈や暗黙の最適化は行いません。

同じ版・入力名と本文・target・heap予算では同じ内容を生成します。日時、出力先ディレクトリ、環境変数は記録しません。bundleには入力名・本文も含まれます。入力パス表記が異なる場合のバイト一致は保証しません。

## 保存と失敗

親ディレクトリは存在する必要があります。出力先は新規ディレクトリに限定し、既存のファイル・ディレクトリ・symlinkは上書きしません。全成果物をメモリ上で生成してから保存を始め、manifestを最後に書きます。

保存失敗は失敗として返します。manifestがないディレクトリは完成bundleではありません。通常の書き込み失敗では、この操作で作成したファイルだけを片付けます。強制終了や削除失敗では不完全なディレクトリが残る可能性があります。transactionやクラッシュ時の永続性は保証しません。

## 実行・配布との区別

プログラムの実行、外部ツールの起動、リンクは行いません。[#104](https://github.com/Cerune-Lang/Cerune/issues/104)の副作用再実行やrecord/replayは追加しません。`build`／`release`の完成成果物ではなく、IR/MIRのloaderも追加しません。

`--lir`を明示するとv4形式で`program.lir.txt`を追加します。`--ssa --lir`では`program.ssa.lir.txt`も追加し、合計12ファイルです（SSAなしは6ファイル）。それぞれのLIRとASMは同じModuleから生成します。manifestの`mir-lir-v1`と`artifact_inputs`にMIR→LIR→ASMを記録します。指定なしのv1と`--ssa`のみのv3は維持します。[形式と検証範囲](lir-observation.ja.md)を参照してください。

Object・逆アセンブル・bytecode・C/LLVM/QBE/WAT・Leanの同梱、実行結果、最適化比較は後続です。未収録を生成失敗や対応済みと混同しません。個別`emit-*`は引き続き使えます。

## 確認

[IR段階のexample](../../examples/ir_stages/README.md)を使い、各ファイルを同じオプションの個別emitとバイト単位で比較します。両target、複数ファイル、文字列バイト、heap予算、決定性、入力エラー、出力先の衝突を確認します。実行すると停止するプログラムも、観測だけでは実行しないことを確認します。
