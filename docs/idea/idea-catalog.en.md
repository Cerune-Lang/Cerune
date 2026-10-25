# Cerune Experiment Catalog

[日本語](idea-catalog.ja.md) | English

A catalog of idea, dogfooding projects, and research topics that could be interesting to try with Cerune.

This document is not a roadmap. Items listed here are not implementation commitments or compatibility promises. They are ideas that may be worth exploring with Cerune now or in the future.

## CPU / ISA

- **RV32I interpreter** — Implement registers, PC, memory, and instruction decoding entirely in Cerune.
- **6502 subset emulator** — Flag behavior makes a good stress test for types and bit operations.
- **CHIP-8 VM** — Small enough to make a good first complete emulator.
- **LC-3 emulator** — An educational ISA that fits Cerune's observability goals well.
- **Custom 8-bit CPU** — Design the ISA itself and compare the result with Cisa.
- **Stack machine** — A minimal JVM-like operand-stack machine.
- **Register machine** — Compare performance and IR shape with a stack VM.
- **Accumulator machine** — Observe how different ISA designs affect execution and lowering.
- **Microcode simulator** — Break each instruction into smaller micro-operations.
- **Five-stage pipeline simulator** — Simulate IF/ID/EX/MEM/WB and hazards.
- **Branch predictor simulator** — Compare always-taken, two-bit predictors, and similar strategies.
- **Cache simulator** — Compare direct-mapped, set-associative, and LRU designs.
- **TLB simulator** — Model virtual-address translation.
- **Page replacement simulator** — Compare FIFO, LRU, and Clock.
- **ALU simulator** — Explicitly model add/subtract/shift/flags.
- **Restoring division** — Build integer division without using `/`.
- **Booth multiplier** — Observe the multiplication algorithm itself.

## VM

- **Brainfuck interpreter** — Run a tiny language through every backend.
- **Forth-like VM** — Experiment with a simplified stack and dictionary.
- **Minimal WebAssembly subset interpreter** — Start with only i32 instructions.
- **JVM-like bytecode subset** — Locals, stack, branches, and calls only.
- **Lua-like register VM** — Useful for comparison with a stack VM.
- **Bytecode verifier** — Statically verify stack height and type consistency.
- **Superinstruction experiment** — Compare VM behavior before and after instruction fusion.
- **Threaded-code-style dispatch comparison** — Compare dispatch approaches within the features Cerune currently supports.

## Compiler / Language Processing

- **Tiny-language lexer** — Embed input as a `u8` array and make it work even before general external I/O exists.
- **Arithmetic expression parser** — Start with expressions such as `1 + 2 * 3`.
- **Pratt parser** — A good experiment in precedence handling.
- **JSON subset parser** — Use byte-array input where current string limitations get in the way.
- **S-expression parser** — A minimal syntax-tree experiment.
- **Tiny BASIC parser** — Line numbers, expressions, and `goto` are enough to start.
- **Tiny assembly parser** — Combine it with a custom VM.
- **Tiny language → bytecode compiler** — Recreate a frontend/backend pipeline inside Cerune.
- **Tiny language → Cerune-IR-like IR** — A compiler inside the compiler.
- **Cerune subset parser** — A first step toward self-hosting.
- **Cerune subset formatter** — Practical dogfooding built on top of parsing.
- **Constant folder** — Fold expressions in an AST.
- **Dead code elimination** — Apply it to a small IR.
- **Copy propagation** — Closely related to Cerune's MIR work.
- **Common subexpression elimination** — Experiment with value numbering.
- **Constant propagation** — An enum can represent the analysis lattice cleanly.
- **SCCP** — Combine CFGs, a lattice, and reachability.
- **Liveness analysis** — A prerequisite for register allocation.
- **Reaching definitions** — A standard dataflow-analysis exercise.
- **Dominator computation** — An iterative algorithm is enough.
- **Dominance frontier** — Preparation for SSA construction.
- **SSA converter** — Also useful as an experimental prototype for Cerune's own future design.
- **φ elimination** — Lower SSA back to non-SSA form.
- **Linear-scan register allocator** — Compare it with the Native backend.
- **Graph-coloring register allocator** — Try it on small CFGs.
- **Instruction selection** — Experiment with tree-pattern matching.
- **Peephole optimizer** — Rules such as `x + 0 → x`.
- **Superoptimizer** — Exhaustively search short instruction sequences for the shortest equivalent sequence.
- **Peephole rule mining** — Derive optimization rules from exhaustive-search results.
- **Expression equivalence checker** — Exhaustively compare all inputs for small bit widths.
- **Mutation testing engine** — Mutate IR and check whether semantic differences are detected.
- **Reducer / delta debugger** — Automatically minimize programs that expose backend differences.
- **Metamorphic testing** — Apply meaning-preserving transformations and check that results remain equal.
- **Random program generator** — Generate small programs using a deterministic PRNG.
- **Differential compiler tester** — Treat IR/MIR/VM/C/LLVM/QBE/WAT/Native as a family of oracles.
- **Abstract interpreter** — Start with constant and interval domains.
- **Interval analysis** — Useful for overflow and bounds reasoning.
- **Symbolic executor** — Restrict the domain to small integers to keep it tractable.
- **CFG interpreter** — Compare execution at CFG level with AST execution.
- **Minimal e-graph implementation** — An introduction to equality saturation.
- **IR provenance trace generator** — Record where each `NodeId` flows.
- **Backend instruction-count comparison** — Measure how the same Cerune program expands in each generated representation.
- **Code-size study** — Compare LLVM, QBE, and Cerune's own Native output.
- **Optimization-resistant test suite** — Collect cases whose semantics are easy to break, such as evaluation order.
- **Backend bug zoo** — One focused example each for overflow, shifts, NaN, `u64` boundaries, and related cases.
- **ABI playground** — Compare SysV and Windows x64 argument placement.
- **Calling-convention visualizer data generator** — Track placement for different argument types.

## Formal / Verification

- **Small-step semantics interpreter** — Advance program state one transition at a time.
- **Big-step vs. small-step comparison** — Implement two semantics for the same tiny language.
- **Type preservation checker experiment** — Limit the experiment to a small language.
- **Progress-property exploration** — Exhaustively search for stuck states.
- **Bounded model checker** — Exhaustively search a bounded state space.
- **Finite-state protocol model checker** — Explore deadlocks and unsafe states.
- **SAT solver** — Implement iterative DPLL.
- **Bit-vector SAT toy** — Exhaustive search over `u8`-sized expressions can already be interesting.
- **BDD** — Experiment with shared representations of Boolean functions.
- **Tautology checker** — A compact propositional-logic exercise.
- **CNF converter** — Combine it with a parser if useful.

## Math / Numerical Computing

- **Matrix multiplication** — A classic backend-comparison workload.
- **Matrix-vector product** — A good baseline kernel before SIMD experiments.
- **Gaussian elimination** — Makes floating-point behavior visible.
- **LU decomposition** — Exercises arrays and numeric operations heavily.
- **Jacobi iteration** — Well suited to iterative computation.
- **Gauss-Seidel** — Update order matters semantically.
- **Simplified Conjugate Gradient** — A linear-algebra stress test.
- **Determinant** — Clean to express for small fixed-size matrices.
- **Polynomial evaluation** — Compare with Horner's method.
- **Newton's method** — Implement square roots or equation solving.
- **Bisection method** — Deterministic and easy to compare across backends.
- **Monte Carlo π** — Also a useful PRNG exercise.
- **Numerical integration** — Trapezoidal and Simpson rules.
- **Finite differences** — An entry point to PDE experiments.
- **Heat diffusion** — Fits Cerune's existing numerical examples well.
- **Wave equation** — A time-evolving simulation.
- **Cellular automata** — Rich experiments using integers only.
- **Conway's Game of Life** — Also a good array-bounds test.
- **Elementary cellular automata** — Rule 30, Rule 110, and others.
- **Langton's Ant** — State machine plus grid.
- **Mandelbrot set** — Makes `f32`/`f64` differences easy to observe.
- **Julia set** — Similar benefits to Mandelbrot experiments.
- **Logistic map** — Excellent for observing floating-point divergence.
- **Simplified chaotic-pendulum integration** — Demonstrates error amplification.
- **N-body simulation** — An O(N²) benchmark.
- **Boids** — A deterministic flocking simulation.
- **Reaction-diffusion** — A strong array-computation stress test.
- **Random walk** — PRNG plus simple statistics.
- **Percolation simulation** — Grid processing plus union-find.

## DSP

- **DFT** — Represent complex numbers with structs.
- **Iterative FFT** — Works without recursion.
- **FIR filter** — A compact signal-processing exercise.
- **Moving average** — A minimal DSP workload.
- **Convolution** — Useful for both image and signal processing.
- **Fixed-point DSP** — Makes Cerune's overflow semantics relevant.

## Image Processing

- **Grayscale convolution** — Treat images as numeric arrays.
- **Box blur** — A minimal image filter.
- **Gaussian blur approximation** — Compare kernel computations.
- **Sobel edge detector** — Works well with integer arithmetic.
- **Thresholding** — Also a future SIMD/GPU candidate.
- **Ordered dithering** — Results can even be inspected as ASCII.
- **Floyd-Steinberg dithering** — Evaluation order matters.
- **ASCII renderer** — Produces visible output even without file I/O.

## Graphics

- **Triangle rasterizer** — Barycentric coordinates and a z-buffer.
- **Wireframe renderer** — A minimal 3D transform pipeline.
- **Fixed-point rasterizer** — Makes integer semantics central.
- **Software z-buffer** — Array-heavy and easy to stress.
- **Voxel renderer** — Grid traversal and ray stepping.
- **Tiny ray tracer** — Approximate or implement required math operations explicitly.
- **Simplified signed-distance ray marcher** — A numerical-approximation experiment.

## Crypto / Hash

- **CRC32** — An excellent bitwise stress test.
- **Adler-32** — A small checksum algorithm.
- **FNV-1a** — Also useful as a building block for hash tables.
- **MurmurHash-style toy** — Exercises overflow and rotation behavior.
- **SHA-256 implementation** — Heavy use of `u32` bit operations.
- **ChaCha quarter-round** — A compact add/xor/rotate stress test.

## Encoding / Compression / ECC

- **Base64 encoder/decoder** — Byte-array manipulation.
- **Hex codec** — A minimal binary/text codec.
- **Varint codec** — Tests integer boundaries.
- **UTF-8 validator** — Can operate on byte arrays even before string indexing exists.
- **RLE** — Minimal compression.
- **Delta encoding** — Natural for numeric sequences.
- **LZ77 toy** — A sliding-window exercise.
- **Huffman coding** — Implement a priority queue and represent trees by indices.
- **Parity / Hamming code** — Bit-level error detection and correction.
- **Reed-Solomon toy** — Finite-field arithmetic.

## Algorithms

- **Quicksort** — Use an explicit stack instead of recursion.
- **Mergesort** — Implement the iterative form.
- **Heapsort** — Also exercises heap data structures.
- **Radix sort** — A good fit for Cerune's integer types.
- **Binary search** — A compact boundary-condition exercise.
- **Union-find** — A foundation for graph and grid experiments.
- **BFS** — Includes building a queue.
- **Iterative DFS** — Turn the lack of recursion into an explicit-state exercise.
- **Dijkstra** — Includes a priority queue.
- **A*** — Heuristics plus state management.
- **Floyd-Warshall** — Straightforward with arrays.
- **Topological sort** — DAG processing.
- **Iterative strongly connected components** — A slightly harder graph exercise.
- **Max flow** — Residual-graph processing.
- **Bipartite matching** — A useful graph-algorithm exercise.
- **Bloom filter** — Bitsets plus hashing.
- **Bitset library** — Also reusable for compiler analyses.

## Data Structures / Memory

- **Binary heap** — Reusable by schedulers and graph algorithms.
- **Ring buffer** — Useful for VMs and network simulations.
- **Hash table** — A foundation for language runtimes and other experiments.
- **Open-addressing comparison** — Linear vs. quadratic probing.
- **Trie** — Represent nodes through indices into an arena.
- **Arena allocator model** — Model allocation through indices even without pointers.
- **Bump allocator simulator** — A minimal allocator model.
- **Buddy allocator simulator** — A classic OS exercise.
- **Slab allocator simulator** — Explore size classes.
- **Mark-and-sweep GC simulator** — Represent the heap as an indexed graph.
- **Reference-counting simulator** — Reproduce cycle problems.
- **Copying GC simulator** — Model semispaces with arrays.

## OS

- **Round-robin scheduler** — A deterministic simulation.
- **Priority scheduler** — Starvation can also be observed.
- **MLFQ simulator** — Compare scheduling policies.
- **Banker's algorithm** — Deadlock avoidance.
- **Deadlock detector** — Build and inspect a wait-for graph.
- **Virtual-memory simulator** — Page tables and faults.
- **Inode/block filesystem simulator** — Model a disk as arrays.
- **Journaling crash simulator** — Enumerate crash points.

## Database

- **Row-store engine** — Array of structs.
- **Column-store engine** — Compare with struct-of-arrays organization.
- **Selection/filter executor** — A minimal query engine.
- **Nested-loop join** — The simplest join strategy.
- **Hash join** — Dogfood the hash-table implementation.
- **Sort-merge join** — Sorting plus merging.
- **Bitmap index** — Reuse bitsets.
- **B-tree simulator** — Explore the structure even without persistence.
- **B+ tree simulator** — A page-layout exercise.
- **MVCC simulator** — Model transaction visibility.
- **WAL state machine** — Explore recovery behavior.
- **Query planner toy** — Compare join-order costs.

## Distributed Systems

- **Lamport clock simulator** — Model message queues with arrays.
- **Vector clocks** — Explore causal ordering.
- **Two-phase commit** — Model failure states.
- **Raft state machine** — Simulate the network deterministically.
- **Paxos toy model** — State exploration may be more interesting than a production-style implementation.
- **Gossip protocol simulator** — Observe convergence.
- **CRDT G-Counter** — Verify merge behavior.
- **PN-Counter** — Extend state merging.
- **OR-Set toy** — A more advanced CRDT experiment.
- **Consistent hashing** — Observe remapping when nodes are added.
- **Leader-election simulator** — Bully, ring, or similar algorithms.
- **Network-partition simulator** — Compare state before and after partitions.

## Protocol

- **TCP handshake state machine** — Represent packets as structs.
- **Retransmission protocol toy** — Model timeouts with a logical clock.
- **Stop-and-wait ARQ** — A minimal reliable protocol.
- **Sliding-window protocol** — Pairs naturally with a ring buffer.
- **DNS packet parser toy** — Byte-level parsing.
- **HTTP request parser subset** — Parsing can be explored even without real networking.
- **Binary packet decoder** — Embedded test vectors are enough.
- **Packet checksum verifier** — Networking plus CRC.

## Games

- **Tic-tac-toe engine** — A minimal game-state model.
- **Connect Four** — Can later be expressed as a bitboard.
- **Othello/Reversi** — Especially interesting with bit operations.
- **Checkers engine** — Focus on move generation.
- **Chess move generator** — Already substantial even without search.
- **Bitboard chess experiments** — A practical `u64` workload.
- **Sudoku solver** — Iterative backtracking.
- **Maze generator** — PRNG plus grid.
- **Maze solver** — BFS or A*.
- **Roguelike combat simulator** — Enums and `match` fit naturally.
- **Tetris rules engine** — Useful even without rendering.
- **Snake simulation** — A compact state-update exercise.
- **ECS toy** — Index-based entities and components.
- **Deterministic replay system** — Feed the same input and compare final state across all backends.

## Automata / Matching

- **DFA engine** — A foundation for parsers and regex engines.
- **NFA engine** — A step toward regular expressions.
- **Regex subset matcher** — Start with operators such as `*`, `+`, `?`, and `|`.
- **Glob matcher** — A more practical pattern matcher.
- **Aho-Corasick toy** — Multi-pattern search.
- **KMP** — String/byte matching.
- **Boyer-Moore toy** — Byte-array matching.

## Simulation

- **Traffic-light controller** — A minimal state-machine example.
- **Elevator controller** — Events plus state.
- **Vending machine** — Exhaustive `match` fits well.
- **Railway-signaling toy** — Add safety invariants.
- **Packet router** — Queues plus routing.
- **Load balancer** — Round robin vs. least-load strategies.
- **Job scheduler** — A queueing-style simulation.
- **Warehouse robot grid** — Pathfinding plus state.
- **Predator-prey grid** — A deterministic cellular model.

## Testing

- **xorshift PRNG** — A building block for randomized tests.
- **PCG-style PRNG** — A useful unsigned-arithmetic workload.
- **Property-based testing toy** — Generate values and check properties.
- **Exhaustive small-domain tester** — Exhaustive `u8` input spaces are practical.
- **Fault-injection framework** — Fail the Nth operation in a model.
- **Crash-consistency explorer** — Useful with database and filesystem simulations.
- **State-machine fuzzing** — Randomly drive protocols and VMs.
- **Backend-consistency signature suite** — Compare hashes or signatures of results across routes.

## Cerune Itself as the Subject

- **MIR stress corpus** — Deliberately extreme branches, loops, and ownership patterns.
- **Source-provenance torture test** — Lower one source expression into many operations and track `NodeId`.
- **Checked-arithmetic torture test** — Exhaustively exercise boundaries for every integer type.
- **Numeric conversion matrix** — Test all numeric-type pairs at important boundaries.
- **f32/f64 divergence gallery** — Collect examples where the same expression diverges.
- **Negative-zero / NaN gallery** — Collect values likely to expose backend differences.
- **Short-circuit semantics suite** — Generate many expressions whose RHS traps if incorrectly evaluated.
- **Aggregate-copy independence suite** — Verify copy semantics for arrays and structs.
- **Module-graph stress test** — Exercise imports, `pub`, cycles, and diagnostics.
- **Compiler-pipeline benchmark corpus** — Separate workloads that stress the frontend, MIR, and backends.
- **"One meaning, many representations" gallery** — Show source/HIR/MIR/bytecode/C/LLVM/QBE/WAT/ASM side by side.
- **Semantic checksum** — Compute a semantic fingerprint at each stage and compare them.
- **Optimization witness** — Record how `NodeId` mappings change across optimization.
- **Source→machine provenance explorer** — Trace final machine operations back to source expressions.
- **Self-hosting countdown** — Move lexer → parser → type checker → IR emitter into Cerune one stage at a time.

## AI × Compiler

- **Differential verification of AI-generated programs** — Let AI generate Cerune programs and use all backends as an automated consistency oracle.
- **Verification of AI-generated optimization rules** — Exhaustively verify proposed rewrites over small bit widths.
- **AI search for bug-inducing programs** — Combine AI generation with compiler fuzzing.
- **Compare AI explanations of IR with actual traces** — Ask whether AI correctly explains a lowering step.
- **Natural language → Cerune → multi-backend comparison** — Put compiler checks around AI-generated code.
- **Generate multiple implementations from one specification and cross-check them** — An N-version-programming-style experiment.
- **Mutation testing for AI-generated compiler passes** — Deliberately break plausible-looking optimizations and measure whether the tests detect it.

## Research

- **HIR/MIR semantic-distance measurement** — Quantify how much structure changes across lowering.
- **Backend semantic-drift classification** — Classify what kinds of semantic differences arise and where.
- **Provenance-retention cost measurement** — Measure the code-size and performance cost of carrying `NodeId`/span information.
- **Observability vs. optimization tradeoff** — Study how much optimization can be done while retaining useful observability.
- **Code-generation cost of checked semantics** — Compare the cost of overflow and bounds checks across backends.
- **IR granularity comparison** — Compare analysis convenience between coarse HIR and more explicit MIR.
- **Readability and observability before/after SSA conversion** — Directly aligned with Cerune's goals.
- **Bug-localization quality: VM vs. MIR Executor vs. Native** — Compare which execution layer makes failures easiest to localize.
- **Differential-testing oracle independence** — Study how shared lowerings reduce the independence of supposedly separate oracles.
- **Deterministic compiler artifacts** — Explore how far identical input can guarantee byte-for-byte identical output.
- **Reproducible-build toy** — Verify artifact identity from source/hash/target inputs.
- **"A compiler that stays explainable after optimization"** — Treat optimization provenance as a first-class output.

## Experiments with Distributed Artifacts

Focus on the boundary where **a completed artifact leaves its build environment, then is validated, reloaded, and executed**. This is different from testing backends against each other, verifying bytecode in memory, checking reproducible builds, or tracing source-to-machine provenance alone.

The first Native build / release implementation is available; IR/VM Image formats, validators, loaders, and build / release are not. See [execution routes and build / release](../design/owned-routes.en.md). These are experiment ideas, not promises of implementation dates, compatibility, or security.

### With the Current Native Release

- **Release relocation test** — Move the release directory away from the build environment and check launch behavior under the target and system-runtime conditions recorded in its manifest.
- **Distribution provenance checker** — Audit the released executable's SHA-256, direct runtime requirements, and build provenance against its manifest.

### Once IR/VM Images and Loaders Exist

- **Image round-trip** — Save IR/VM as an Image, validate/load it in another process, execute it, and compare against execution before serialization.
- **Compiler-free minimal package** — Deliver only an Image and compatible runtime; measure the smallest working distribution layout and its dependencies.
- **Image compatibility lab** — Cross format, semantic, and runtime versions to tabulate acceptance, rejection, and diagnostics.
- **Malformed-Image acceptance boundary** — Construct saved Images with invalid types, references, ownership metadata, and resource limits; ensure rejection before execution. Format validation does not replace sandboxing.
- **Post-distribution fault provenance** — Map a failure in a released Image back to its source location and build record using saved provenance.

### Once Embedding APIs and Host Integration Exist

- **Replaceable VM logic** — Distribute game or simulation computations as Images for a host to load, call, and swap; test typed host APIs, version compatibility, and resource limits.

## Future: Once External I/O Exists

- **grep-like tool** — A practical byte/string-processing utility.
- **wc/cat/head-like CLI tools** — Small real programs for exercising runtime boundaries.
- **CSV processor** — Parsing plus aggregation.
- **JSON pretty-printer** — Parser/formatter dogfooding.
- **Binary inspector** — A natural target for byte-level processing.
- **Assembler/disassembler CLI** — A strong fit with Cisa and Cerune.
- **Compiler artifact explorer** — Compare `.ceir`, MIR text, `.ll`, `.ssa`, and `.s` side by side.
- **Tiny database CLI** — Extend the in-memory database experiments.
- **Image filter CLI** — Start with PPM/PGM for a simple file format.
- **Network packet analyzer** — Byte parsing plus protocol state.
- **REPL** — Makes Cerune immediately feel more like an interactive language.
- **Cerune compiler frontend in Cerune** — A major step toward self-hosting.

## Future: GPU

- **Matrix-kernel CPU/GPU comparison** — Directly connected to the existing roadmap.
- **Heat-diffusion CPU/GPU comparison** — Observe numerical error, synchronization, and bounds checks.
- **Element-wise checked arithmetic** — An unusual experiment in collecting trap information from GPU execution.

## Future: FPGA

- **Cerune IR → simple hardware** — Turn arithmetic kernels into circuits.
- **ALU generation** — Generate circuits from typed operations.
- **Finite-state-machine synthesis** — Lower enums and `match`-driven logic into FSMs.

## Particularly Cerune-like Candidates

The following ideas make especially good use of Cerune's current direction:

- **RV32I / custom CPU**
- **Tiny language + multiple executors**
- **Observable SSA / optimization passes**
- **Differential compiler testing**
- **Delta debugging**
- **Exhaustive verification of AI-generated optimization rules**
- **Source→machine provenance**
- **Backend cost of checked arithmetic**
- **f32/f64 divergence gallery**
- **Bounded model checking**
- **Distributed-protocol simulation**
- **Self-hosting countdown**

In many languages, these projects are simply things that can be built. In Cerune, the transformation itself can become part of the experiment:

`source → HIR → MIR → VM / C / LLVM / QBE / WAT / Native`

The interesting question is not only whether the program runs, but how its representation changes across those routes and whether its meaning remains the same.
