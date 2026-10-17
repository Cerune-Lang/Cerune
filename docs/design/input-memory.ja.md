# 行入力の保持容量と論理予算

[English](input-memory.en.md)

2026-10-18、#154 / #133の初回測定です。基準はPR #164統合時の実装（`12ddad3`）です。入力処理・予算・失敗の仕様は変更していません。

## 確認したこと

**短い行でも予約容量が残り、論理課金との差が出ます。** Rust共通runtimeと生成CをWindows/Linuxで測定しました。8本の2バイト行を保持する場合の結果です。

| 行ごとの上限 | 合計文字列長・論理課金 | Rustの合計capacity | 生成Cの生存malloc要求量 |
| --- | ---: | ---: | ---: |
| 2バイト | 16 | 16 | 208 |
| 1 MiB | 16 | 8,388,608 | 8,388,800 |

Cの値には1本24バイトの所有管理領域を含みます。mallocへの要求量であり、allocatorの丸め・内部管理量は含みません。Rustのcapacityにも所有管理やHashMapの容量は含みません。

保持→全解放を3回繰り返すと、毎回、所有数・論理課金・追跡対象の生存確保量はゼロに戻りました。このケースは解放漏れを示しません。一般的なリーク不在やプロセス全体のメモリ上限を証明するものでもありません。

## OS指標は別に測る

生成Cの測定用プロセスで、コンパイル後の開始時・保持中・解放後を記録しました。以下は1 MiB上限、`-O2`、最初の周期の実測値です。測定用のprintfやOS照会も含む参考値で、CIの合否には使いません。

| 環境・指標 | 開始時 | 8本保持 | 解放後 |
| --- | ---: | ---: | ---: |
| Windows working set（bytes） | 4,001,792 | 4,038,656 | 4,005,888 |
| Windows private commit（bytes） | 561,152 | 8,998,912 | 561,152 |
| Linux/WSL VmSize（kB） | 10,548 | 18,772 | 10,548 |
| Linux/WSL VmRSS（kB） | 580 | 640 | 612 |

測定対象はx86-64です。Windowsは10.0.26100、Clang 22.1.8。LinuxはWSLの`4.4.0-26100-Microsoft`、GCC 9.4.0。Rustは1.98.0です。Cは`-O0`でも確保量・課金・解放を照合しました。

Windowsのprivate commitは[PROCESS_MEMORY_COUNTERS_EX](https://learn.microsoft.com/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex)のPrivateUsageです。[Linuxの/proc指標](https://www.kernel.org/doc/html/v6.1/filesystems/proc.html)とは同じ指標ではありません。このWSL環境ではVmHWM/VmDataが0を返したため、それらをピークRSS・コミットの測定結果として扱いません。実Linuxでの追加測定は残ります。容量8 MiBを「物理RAMも8 MiB増える」とは解釈できません。

## 経路ごとの確認範囲

| 経路 | 今回の確認 |
| --- | --- |
| HIR・MIR・SSA・VM | 共用StringHeapの長さ・capacity・課金・解放を単体測定。各実行器では同じ入力例を実行 |
| C | 実際のemitterが生成した入力・所有管理関数を測定用mainから呼出し。malloc/free要求・生存所有者・論理課金・OS指標を記録 |
| LLVM・QBE・ASM・自前Object | 例の出力と論理予算の挙動を比較。ソース確認ではlimit + 24の領域を引き継ぐ。容量・OS指標の個別測定は未実施 |
| WAT | 同じ例で出力・予算を比較。入力上限分の割当て後、論理課金だけを実長へ変更する実装を確認。保持領域・free list・linear memory pagesの測定は未実施 |

[retained_lines.ceru](../../examples/stdin/retained_lines.ceru)は、2行を読んでから両方を表示します。テストでは上限64バイトと1 MiBを比較し、ヒープ予算を「上限＋最初の行の2バイト」にして2回目の読取りを通します。これはcapacity課金へ黙って変わっていないことも検証します。

Cの測定用mainは標準モジュールの名前解決やenum loweringの測定ではありません。それらは上記のソースからの経路比較で確認します。測定用コードはtests内に置き、製品の観測APIや内部memoryの公開には使いません。

## 改善候補と判断

| 候補 | 利点 | 契約への影響・制約 |
| --- | --- | --- |
| 読取り中の逐次拡張 | 短い行の確保量を抑える | 入力消費後に確保失敗し得る。現在の「物理確保失敗も読取り前」の契約を変える |
| 完成時の必須縮小 | 成功文字列の余剰容量を抑える | 縮小のための再確保失敗が、読取り後の新しい失敗になる可能性 |
| 完成時の任意縮小 | 失敗時に元の領域を保てば入力結果・課金を維持できる | メモリ上限にはならない。コピー・一時ピーク・所有者移管、WATの領域再利用を別途検証する |
| capacityを論理予算に課金 | 保持容量を予算に反映 | 同じソースでも予算不足になる位置が変わる。allocator依存値では経路間の意味が揃わない |
| 別の実行ホスト上限 | metadata・出力・コンパイラ等を含めて制限できる | OS・ホスト固有。io-v1の通常失敗と同一視できず、設定・停止・診断の別設計が必要 |

**当面は論理予算の意味と入力前の予約を維持します。次に検証する候補は完成時の任意縮小です。** まだ採用・実装済みとは扱いません。成功時の追加確保を行わない現契約の文言も含めて整理し、縮小失敗の注入でも通常結果・消費数・後続出力が変わらないことを確認してから判断します。

#154は継続します。残りはLLVM/QBE/Native/WATの容量測定、実LinuxでのOS指標、任意縮小の試作と失敗・所有管理の比較です。#133の配列・出力コピー・コンパイラピーク、1バイトずつ読む呼出しコストはこの測定では扱っていません。

## 再実行

~~~sh
cargo test --lib retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_input_capacity -- --nocapture
cargo test --test stdin_routes retained_lines_example -- --nocapture
~~~

生成Cの測定では`CERUNE_TEST_CC`を明示します（Windows: clang、Linux: cc）。経路比較には`CERUNE_TEST_LLVM_CLANG`、`CERUNE_TEST_ASM_CLANG`、`CERUNE_TEST_QBE`、`CERUNE_TEST_NODE`、`CERUNE_TEST_WAT2WASM_JS`も有効なツールへ設定してください。未指定でツールがなければ既存テストは該当経路をskipします。指定済みツールの欠落は失敗です。

測定用Cと実行物は`target/stdin-input-memory-<pid>`へ生成します。各実行は3周期・8本・最大1 MiB/本、読取りは48バイトで有限です。生存量の回帰検査は現在の実装の基準値であり、将来の縮小実装時には意図的に更新します。[入力契約](line-input.ja.md)は別に維持します。
