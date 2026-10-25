# Cerune 実験テーマ集

日本語 | [English](idea-catalog.en.md)

Ceruneで試すと面白そうな実験・dogfooding・アイデアのカタログです。

この文書はロードマップではありません。ここにある項目は実装予定や互換性の約束ではなく、現在または将来のCeruneで試せそうな題材を集めたものです。

## CPU / ISA

- **RV32Iインタプリタ** — レジスタ・PC・メモリ・decodeを全部Ceruneで。
- **6502 subsetエミュレータ** — flag処理が型とbit演算の良いストレステスト。
- **CHIP-8 VM** — 小さいので最初の完成系に向く。
- **LC-3エミュレータ** — 教育用ISAなので観測思想との相性がいい。
- **独自8bit CPU** — ISAそのものを設計してCisaと比較。
- **stack machine** — JVM的なoperand stackの最小模型。
- **register machine** — stack VMとの性能・IR形状比較。
- **accumulator machine** — ISA設計の差を観測。
- **microcode simulator** — 1命令をさらに細かいmicro-opへ分解。
- **5段pipeline simulator** — IF/ID/EX/MEM/WBとhazardをシミュレーション。
- **branch predictor simulator** — always-taken、2-bit predictorなどを比較。
- **cache simulator** — direct mapped / set associative / LRU。
- **TLB simulator** — 仮想アドレス変換の模型。
- **page replacement simulator** — FIFO/LRU/Clock比較。
- **ALU simulator** — add/sub/shift/flagsを明示的に構成。
- **restoring division** — `/`を使わず除算器を作る。
- **Booth multiplier** — 乗算アルゴリズム自体を観測する。

## VM

- **Brainfuck interpreter** — 極小言語を全backendで実行。
- **Forth風VM** — stackとdictionaryを簡略化して実験。
- **WebAssembly極小subset interpreter** — i32命令だけから開始。
- **JVM風bytecode subset** — local/stack/branch/callだけ。
- **Lua風register VM** — stack VMとの比較に向く。
- **bytecode verifier** — stack heightや型整合性を静的に検査。
- **superinstruction実験** — 命令融合前後でVM性能比較。
- **threaded-code風dispatch比較** — 現行機能の範囲でdispatch方式を比較。

## Compiler / Language Processing

- **極小言語のlexer** — 入力をu8配列として埋め込めば今でも可能。
- **arithmetic expression parser** — `1 + 2 * 3`程度から。
- **Pratt parser** — precedence設計の教材に。
- **JSON subset parser** — string制約を避けてbyte配列入力でもよい。
- **S式parser** — 最小の構文木実験。
- **tiny BASIC parser** — 行番号＋式＋goto程度。
- **tiny assembly parser** — 独自VMと組み合わせる。
- **tiny language → bytecode compiler** — frontend/backendをCerune自身で再現。
- **tiny language → Cerune IR風IR** — コンパイラ内コンパイラ。
- **Cerune subset parser** — self-hostingへ向かう最初の一歩。
- **Cerune subset formatter** — parserより実用寄りのdogfooding。
- **constant folder** — ASTを畳み込み。
- **dead code elimination** — 小さいIRを対象に。
- **copy propagation** — MIR思想と直結。
- **common subexpression elimination** — value numbering実験。
- **constant propagation** — latticeをenumで表すと綺麗。
- **SCCP** — CFG＋lattice＋reachability。
- **liveness analysis** — register allocationの前段。
- **reaching definitions** — dataflow解析の教材。
- **dominator計算** — iterative algorithmで十分書ける。
- **dominance frontier** — SSA構築の前準備。
- **SSA変換器** — 今後のCerune本体設計の実験台にもなる。
- **φ elimination** — SSA→non-SSA lowering。
- **linear-scan register allocator** — Native backendとの比較材料。
- **graph-coloring register allocator** — 小規模CFGで。
- **instruction selection** — tree pattern matchingの実験。
- **peephole optimizer** — `x + 0 → x`など。
- **superoptimizer** — 短い命令列を総当たりして等価な最短列を探す。
- **peephole rule mining** — 総当たり結果から最適化則を発見。
- **expression equivalence checker** — 小bit幅なら全入力列挙で完全比較。
- **mutation testing engine** — IRを変異させ意味差を検出。
- **reducer / delta debugger** — backend差分を起こす最小プログラム探索。
- **metamorphic testing** — 意味保存変換後に結果一致を見る。
- **random program generator** — deterministic PRNGで小プログラム生成。
- **differential compiler tester** — IR/MIR/VM/C/LLVM/QBE/WAT/native全部をoracle群にする。
- **abstract interpreter** — constant / interval domain。
- **interval analysis** — overflowやbounds解析にもつながる。
- **symbolic executor** — 小さい整数領域限定なら面白い。
- **CFG interpreter** — AST実行との比較。
- **e-graphの極小実装** — equality saturation入門。
- **IR provenance可視化用trace生成** — NodeIdがどこへ流れたかを記録。
- **backend間命令数比較** — 同じCerune programが各生成先でどう膨らむか。
- **code-size研究** — LLVM/QBE/自前Nativeで生成物サイズ比較。
- **optimization-resistant test suite** — 評価順など壊しやすい意味を集める。
- **backend bug zoo** — overflow、shift、NaN、u64境界などを1テーマ1例にする。
- **ABI playground** — SysVとWindows x64の引数配置比較。
- **calling convention visualizer用データ生成** — 引数型ごとの配置を追跡。

## Formal / Verification

- **small-step semantics interpreter** — 1ステップずつ状態遷移。
- **big-step vs small-step比較** — 同じ言語を2意味論で実装。
- **type preservation checker実験** — tiny language限定。
- **progress property探索** — stuck状態を総当たり。
- **bounded model checker** — 状態数を限定して全探索。
- **finite-state protocol model checker** — deadlockやunsafe stateを探索。
- **SAT solver** — iterative DPLL。
- **bit-vector SAT toy** — u8程度の式を総当たりでも面白い。
- **BDD** — boolean functionの共有表現。
- **tautology checker** — 命題論理の教材。
- **CNF変換器** — parserと組み合わせても良い。

## Math / Numerical Computing

- **行列積** — backend比較の定番。
- **matrix-vector product** — SIMD前段の基準kernel。
- **Gaussian elimination** — 浮動小数点挙動が見える。
- **LU decomposition** — 配列と数値演算を酷使。
- **Jacobi iteration** — 反復計算に向く。
- **Gauss-Seidel** — 更新順序の意味が出る。
- **Conjugate Gradient簡易版** — linear algebra stress test。
- **determinant** — 小サイズ固定配列なら綺麗。
- **polynomial evaluation** — Horner法比較。
- **Newton法** — sqrtや方程式解法を自前実装。
- **bisection method** — deterministicでbackend比較しやすい。
- **Monte Carlo π** — PRNGの題材にも。
- **numerical integration** — 台形則、Simpson則。
- **finite difference** — PDE実験への入口。
- **heat diffusion** — 既存方向性とも合う。
- **wave equation** — 時間発展型simulation。
- **cellular automata** — 整数だけでも豊富。
- **Conway's Game of Life** — 配列境界テストにもなる。
- **elementary cellular automata** — Rule 30/110など。
- **Langton's Ant** — state machine＋grid。
- **Mandelbrot** — f32/f64差が見て分かる。
- **Julia set** — 同上。
- **logistic map** — floating-point divergenceが非常に面白い。
- **chaotic pendulumの簡易数値積分** — 誤差増幅の教材。
- **N-body** — O(N²) benchmark。
- **boids** — deterministic群集simulation。
- **reaction-diffusion** — 配列計算のstress test。
- **random walk** — PRNG＋統計。
- **percolation simulation** — grid＋union-find。

## DSP

- **DFT** — 複素数をstructで表す。
- **iterative FFT** — 再帰なしでも書ける。
- **FIR filter** — signal processing入門。
- **moving average** — 最小DSP。
- **convolution** — 画像にも音にもつながる。
- **fixed-point DSP** — overflow semanticsを活かせる。

## Image Processing

- **grayscale convolution** — 画像を数値配列として扱う。
- **box blur** — 最小フィルタ。
- **Gaussian blur近似** — kernel演算比較。
- **Sobel edge detector** — integer演算でもいける。
- **thresholding** — SIMD/GPU候補にもつながる。
- **ordered dithering** — 出力をASCIIでも確認可能。
- **Floyd–Steinberg dithering** — 評価順が重要。
- **ASCII renderer** — 外部I/Oなしでも成果が見える。

## Graphics

- **triangle rasterizer** — barycentric coordinatesとz-buffer。
- **wireframe renderer** — 3D変換の最小系。
- **fixed-point rasterizer** — 整数意味論が生きる。
- **software z-buffer** — array-heavy。
- **voxel renderer** — gridとray stepping。
- **tiny ray tracer** — sqrt等を自前近似しても面白い。
- **signed-distance ray marcher簡易版** — 数値近似の実験。

## Crypto / Hash

- **CRC32** — bitwise stress testとして優秀。
- **Adler-32** — 小さいchecksum。
- **FNV-1a** — hash tableにも使える。
- **MurmurHash系toy** — overflow/rotateの試験。
- **SHA-256実装** — u32 bit演算を徹底的に使う。
- **ChaCha quarter-round** — rotate/add/xorの美しいstress test。

## Encoding / Compression / ECC

- **Base64 encoder/decoder** — byte配列操作。
- **hex codec** — 最小binary codec。
- **varint codec** — integer boundaryのテスト。
- **UTF-8 validator** — string indexingがなくてもbyte列なら実験可能。
- **RLE** — 最小圧縮。
- **delta encoding** — 数列向き。
- **LZ77 toy** — sliding windowの教材。
- **Huffman coding** — priority queueとtreeを配列indexで。
- **parity / Hamming code** — bit-level処理。
- **Reed–Solomon toy** — finite field演算。

## Algorithms

- **quicksort** — 再帰なしなら明示stack版。
- **mergesort** — iterative版。
- **heapsort** — heap構造の確認にも。
- **radix sort** — 整数型が豊富なので相性良い。
- **binary search** — 境界条件教材。
- **union-find** — graph/grid系の基盤。
- **BFS** — queue実装込み。
- **DFS iterative** — recursionなしを逆に活かす。
- **Dijkstra** — priority queue込み。
- **A*** — heuristic＋state。
- **Floyd–Warshall** — 配列だけで書きやすい。
- **topological sort** — DAG処理。
- **strongly connected components iterative版** — 少し難しい良題材。
- **max-flow** — residual graph実験。
- **bipartite matching** — graph algorithm教材。
- **bloom filter** — bitset＋hash。
- **bitset library** — compiler解析にも使える。

## Data Structures / Memory

- **binary heap** — schedulerにも使える。
- **ring buffer** — VMやnetwork simulation向き。
- **hash table** — language/runtimeの基礎。
- **open addressing比較** — linear/quadratic probing。
- **trie** — indexを使ったarena表現。
- **arena allocator模型** — pointerなしでもindex arenaとして。
- **bump allocator simulator** — allocator思想の最小形。
- **buddy allocator simulator** — OS教材。
- **slab allocator simulator** — サイズクラス。
- **mark-and-sweep GC simulator** — heapをindex graphとして。
- **reference counting simulator** — cycle問題を再現。
- **copying GC simulator** — semispaceを配列で。

## OS

- **round-robin scheduler** — deterministic simulation。
- **priority scheduler** — starvationも観測可能。
- **MLFQ simulator** — scheduler比較。
- **Banker's algorithm** — deadlock avoidance。
- **deadlock detector** — wait-for graph。
- **virtual memory simulator** — page table＋fault。
- **inode/block filesystem simulator** — diskを配列としてモデル化。
- **journaling crash simulator** — crash pointを全列挙。

## Database

- **row-store engine** — array of structs。
- **column-store engine** — struct of arraysと比較。
- **selection/filter executor** — query engine最小形。
- **nested-loop join** — 最初のjoin。
- **hash join** — hash tableもdogfood。
- **sort-merge join** — sort＋merge。
- **bitmap index** — bitset活用。
- **B-tree simulator** — 永続化なしでも構造は実験可能。
- **B+tree simulator** — page layout教材。
- **MVCC simulator** — transaction visibility。
- **WAL state machine** — recovery設計。
- **query planner toy** — join orderのcost比較。

## Distributed Systems

- **Lamport clock simulator** — message queueを配列で。
- **vector clock** — causal order。
- **two-phase commit** — failure stateの模型。
- **Raft state machine** — network自体をdeterministic simulation。
- **Paxos toy model** — 実装より状態探索が面白い。
- **gossip protocol simulator** — convergenceを見る。
- **CRDT G-Counter** — mergeの意味を確認。
- **PN-Counter** — 状態merge。
- **OR-Set toy** — CRDTをもう一段進める。
- **consistent hashing** — node追加時の再配置を可視化。
- **leader election simulator** — bully/ring系。
- **network partition simulator** — partition前後のstate比較。

## Protocol

- **TCP handshake state machine** — packet自体は構造体で。
- **retransmission protocol toy** — timeoutをlogical clockで。
- **stop-and-wait ARQ** — 最小信頼通信。
- **sliding-window protocol** — ring bufferと相性良い。
- **DNS packet parser toy** — byte-level parsing。
- **HTTP request parser subset** — 外部通信なしでもparserは書ける。
- **binary packet decoder** — embedded test vectorで十分。
- **packet checksum verifier** — network＋CRC。

## Games

- **Tic-tac-toe engine** — 最小game state。
- **Connect Four** — bitboardにも発展できる。
- **Othello/Reversi** — bit演算版が特に面白い。
- **Checkers engine** — move generation。
- **Chess move generator** — search抜きでも十分大作。
- **bitboard chess experiments** — u64の実践題材。
- **Sudoku solver** — iterative backtracking。
- **maze generator** — PRNG＋grid。
- **maze solver** — BFS/A*。
- **roguelike combat simulator** — enum/matchが映える。
- **Tetris rules engine** — renderingなしでも成立。
- **Snake simulation** — state更新の教材。
- **ECS toy** — entity/componentをindexベースで。
- **deterministic replay system** — 同入力→同stateを全backend比較。

## Automata / Matching

- **DFA engine** — parser/regex基盤。
- **NFA engine** — regexへ。
- **regex subset matcher** — `* + ? |`程度。
- **glob matcher** — 実用寄り。
- **Aho–Corasick toy** — multi-pattern search。
- **KMP** — string/byte matching。
- **Boyer–Moore toy** — byte配列で。

## Simulation

- **traffic light controller** — state machine最小例。
- **elevator controller** — event/stateの教材。
- **vending machine** — exhaustive matchが生きる。
- **railway signaling toy** — safety invariant付き。
- **packet router** — queue＋routing。
- **load balancer** — round robin/least-load。
- **job scheduler** — queueing theory風。
- **warehouse robot grid** — pathfinding＋state。
- **predator-prey grid** — deterministic cellular model。

## Testing

- **xorshift PRNG** — 他のランダム試験の土台。
- **PCG系PRNG** — unsigned arithmeticの題材。
- **property-based test toy** — 値生成→property検査。
- **exhaustive small-domain tester** — u8なら全入力が現実的。
- **fault injection framework** — N回目の操作を失敗させる模型。
- **crash consistency explorer** — DB/FSと相性が良い。
- **state-machine fuzzing** — protocolやVMをランダム遷移。
- **backend一致signature suite** — 各経路の出力hashだけ比較。

## Cerune自身を題材にする

- **MIR stress corpus** — branch/loop/ownershipを意図的に極端化。
- **source provenance torture test** — 1式を大量loweringさせてNodeId追跡。
- **checked arithmetic torture test** — 型ごとの境界値を総当たり。
- **numeric conversion matrix** — 全数値型×全数値型の境界試験。
- **f32/f64 divergence gallery** — 同じ式の差を集める。
- **negative zero / NaN gallery** — backend差が出やすい値を収集。
- **short-circuit semantics suite** — RHSがtrapする式を大量生成。
- **aggregate-copy independence suite** — array/structのコピー意味を検証。
- **module graph stress test** — import/pub/cycle診断。
- **compiler pipeline benchmark corpus** — frontend/MIR/backend別に重い例を作る。
- **「同じ意味を何通りで表せるか」展** — source/HIR/MIR/bytecode/C/LLVM/QBE/WAT/ASMを並べる。
- **semantic checksum** — 各段階から意味的fingerprintを計算して比較。
- **optimization witness** — 最適化前後で対応するNodeIdを記録。
- **source→machine provenance explorer** — 最終命令がどのsource expression由来か辿る。
- **self-hosting countdown** — lexer→parser→typechecker→IR emitterを順番にCerune化。

## AI × Compiler

- **AI生成programのdifferential検証** — AIにCeruneを書く側を任せ、全backend一致を自動oracleにする。
- **AI生成optimization ruleの検証** — AIが出したrewriteを小bitwidthで全数検証。
- **AIにbug-inducing programを探索させる** — compiler fuzzingと相性がいい。
- **AIによるIR説明と実際のtraceの比較** — 「AIはこのloweringを正しく説明できるか」。
- **自然言語→Cerune→多backend比較** — AI codingの正しさをcompiler側で挟む。
- **同じ仕様から複数実装をAI生成して相互比較** — N-version programmingっぽい実験。
- **AI生成compiler passのmutation testing** — “それっぽく動く”最適化を壊して検出力を見る。

## Research

- **HIR/MIR semantic distance測定** — 構造がどれだけ変わったか定量化。
- **backend semantic drift分類** — どの種類の意味差がどこで発生するか。
- **provenance保持コスト測定** — NodeId/span追跡がコード量・速度に与える影響。
- **observability vs optimization tradeoff** — 観測可能性を残すとどこまで最適化できるか。
- **checked semanticsのコード生成コスト** — overflow/bounds checkのbackend別コスト。
- **IR granularity比較** — 粗いHIRと細かいMIRで解析容易性がどう変わるか。
- **SSA化前後の可読性・観測性比較** — Ceruneのテーマど真ん中。
- **VM vs MIR executor vs Nativeのバグ局在性** — エラーがどの層で見つけやすいか。
- **differential testing oracle independence** — 共通loweringを共有するとoracle独立性がどこまで落ちるか。
- **deterministic compiler artifacts** — 同じ入力→完全同一出力をどこまで保証できるか。
- **reproducible build toy** — source/hash/targetから成果物同一性を検証。
- **「最適化しても説明できるコンパイラ」** — optimization provenanceを第一級の出力にする。

## 将来: I/Oが入ったら

- **grep風ツール** — byte/string処理の実用品。
- **wc/cat/head風CLI** — runtime境界の最初の実用品。
- **CSV processor** — parser＋集計。
- **JSON pretty-printer** — parser/formatter dogfooding。
- **binary inspector** — byte processingの本命。
- **assembler/disassembler CLI** — Cisa/Ceruneとの相性抜群。
- **compiler artifact explorer** — `.ceir/.mir/.ll/.ssa/.s`を横並びに。
- **tiny database CLI** — in-memory engineから自然に伸ばせる。
- **image filter CLI** — PPM/PGMから始めると実装が軽い。
- **network packet analyzer** — byte parsing＋protocol state。
- **REPL** — 言語処理系として一気に“使ってる感”が出る。
- **CeruneでCerune compiler frontend** — self-hostingの本格化。

## 配布用Image・実行環境・アプリケーション（現在と将来）

ここでは「Ceruneで何を書くか」ではなく、「Ceruneで作ったプログラムをどう保存・配布・再実行・組み込みできるか」を題材にする。既存のcompiler差分検証や再現可能ビルドの実験とは、**配布された完成成果物を起点にする**点が異なる。

現時点で[Native build / release](../design/owned-routes.ja.md)の初版は実装済み。IR/VM Imageの形式・検証器・loader・build/releaseは未実装。以下は実験アイデアであり、機能の実装や互換性を約束するものではない。

### 現行Native build / releaseから着手可能

- **Native配布物のスモークテスト** — release成果物を別ディレクトリへ移し、対象OS・CPU・runtime要件を満たす環境で起動・終了状態を確認。
- **配布manifest監査** — target・直接runtime要件・SHA-256・buildの出自が配布物と一致するか確認。
- **Native配布物とIR/VM実行の照合** — 元ソースを各実行器で評価し、対象環境で起動した配布実行ファイルと出力・停止理由を比較。

### IR/VM Imageとloaderの完成後

- **IR Image runner** — 保存したIR Imageを別プロセスでload・検証・実行し、コンパイラ実行から独立させる。
- **VM Bytecode Image runner** — Bytecode Imageと互換VM runtimeだけでプログラムを実行する配布模型。
- **Image round-trip試験** — build→保存→再読込→実行を繰り返し、保存前のIR/VM実行と結果を照合。
- **Image互換性マトリクス** — 形式版・意味版・runtime版を組み合わせ、受理・拒否・診断の境界を調べる。
- **不正Imageの拒否テスト** — 壊れた参照・型・制御フロー・所有情報・資源上限を持つImageを安全に拒否できるか検査。
- **IR/VM/Nativeの配布経路比較** — 同じ入力から各経路でreleaseし、依存runtime・成果物サイズ・起動・停止の観測可能性を比較。
- **配布後provenance explorer** — 配布したImageや実行ファイルの診断を元のソース位置・build記録へ結び直す。

### 追加のI/O・ホストAPI・隔離設計が必要

- **組み込みCerune VM** — 外部アプリからImageを読み込んで関数を呼ぶ。VMの埋め込みAPIとホスト側の値受け渡しが前提。
- **差し替え可能なアプリロジック** — ゲーム・シミュレータ・拡張機能のロジックをVM Imageとして配る。ホスト連携と更新時の互換条件を検討。
- **制限付き実行ホスト** — 実行予算・公開するI/O・権限・隔離を設計してImageを実行する。Imageの形式・型検証だけではサンドボックスにはならない。
- **入出力付きCLIやサービスの配布** — ファイルI/O・ネットワークI/Oなどが追加された後、実用品のbuild/release・依存関係・障害診断を試す。

## 将来: GPU

- **matrix kernel CPU/GPU比較** — 既存ロードマップと直結。
- **heat diffusion CPU/GPU比較** — 誤差・同期・境界検査まで観測。
- **element-wise checked arithmetic** — GPUでtrap情報をどう回収するかという珍しい研究題材。

## 将来: FPGA

- **Cerune IR→簡易回路** — 算術kernelをhardware化。
- **ALU生成** — 型付き演算から回路生成。
- **finite-state machine synthesis** — enum/matchからFSMへ。

## 特にCeruneらしい候補

Ceruneの現在の特徴を最も活かしやすい候補を抜き出すと、次あたりが特に面白い。

- **RV32I / 独自CPU**
- **tiny language + 複数executor**
- **SSA / 最適化passの観測**
- **differential compiler testing**
- **delta debugging**
- **AI生成最適化則の全数検証**
- **source→machine provenance**
- **checked arithmeticのbackend別コスト**
- **f32/f64 divergence gallery**
- **bounded model checking**
- **distributed protocol simulation**
- **self-hosting countdown**

普通の言語では「作れる」で終わる題材も、Ceruneでは同じプログラムが

`source → HIR → MIR → VM / C / LLVM / QBE / WAT / Native`

へどう変換され、それでも意味が一致するかを観測できること自体が実験テーマになる。
