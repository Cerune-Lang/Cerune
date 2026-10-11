# Cerune compiler design

[日本語](architecture.ja.md)

Cerune is a statically typed experimental language that keeps type/semantic decisions and transformations into executable representations observable. The [observability contract](observability.en.md) and [output routes/targets](targets.en.md) define observation and artifact conditions.

Current implementation and plans are summarized below.

| Status | Contents |
| --- | --- |
| Implemented | Direct execution of completed Cerune IR (HIR), bytecode VM, C/LLVM/QBE/WAT generation |
| Implemented | Non-SSA MIR types/lowering/validation, `emit-mir`, [independent execution](mir-executor.en.md), and [MIR→Native generation](native-mir.en.md) |
| Implemented | [Observation bundle](observation-bundle.en.md): sources, HIR, MIR, and annotated ASM from one compilation |
| Implemented | [SSA](mir-ssa.en.md) construction, direct execution, CLI/bundles, SSA→MIR/Native, and finite-fixture Lean verification |
| Implemented | x86-64 LIR lowering, display, saving, and structural validation; direct execution deferred ([design](lir-observation.en.md)) |
| Designed, implementation deferred | [Constant-folding pass](ssa-constant-folding.en.md); no implicit optimization |
| Planned, unimplemented | IR/VM/Native [build / release](owned-routes.en.md), with C-like routes classified as emit-only |

## Principles

Cerune aims to combine sophisticated implementation with observability. As transformations become more advanced, their boundaries and results must remain observable.

In particular:

- type decisions should be visible and predictable;
- backend-independent meaning should be resolved before backend lowering;
- backend-specific decisions should happen behind an explicit lowering boundary;
- emitters should format backend IR rather than reinterpret Cerune semantics;
- optimizations should be introduced as explicit, observable passes that can be examined as part of an experiment;
- generated observations should avoid incidental nondeterminism such as timestamps or random identifiers.

Cerune v0.1 does not silently insert numeric conversions or hide transformations that are useful to observe.

## Compiler architecture

The common frontend resolves meaning before passing it to execution and generation routes.

```text
Source → Lexer / Parser → AST → Common frontend → Cerune IR (HIR)
                                                  ├→ emit-ir / .ceir
                                                  ├→ IR Executor: run / run-ir
                                                  ├→ MIR lowering / validation
                                                  │    ├→ emit-mir / run-mir
                                                  │    ├→ SSA → SSA execution / MIR with copies → x86-64 LIR
                                                  │    └→ x86-64 LIR → ASM → self-encoded object
                                                  ├→ Bytecode → emit-bytecode / run-vm
                                                  └→ C / LLVM / QBE / WAT IR → emit-*
```

LIR is a Native generation stage observable through `emit-lir` / `observe --lir`. Reconstructed MIR follows the same LIR→ASM→Object path. Direct `run-lir` execution is [deferred after investigation](lir-observation.en.md).

The frontend resolves names, types, and contextual floats, and expands generics, enum/match, and ownership. The [IR Executor](ir-executor.en.md) directly executes completed HIR statements/expressions; generating routes translate resolved meaning into target representations.

### Architectural invariants

Responsibilities and identifier meanings remain explicit.

1. Backends do not consume AST directly. They use HIR after common semantic/type resolution; Native first lowers HIR into MIR.
2. A lowerer reads its input and output IRs. An emitter reads its backend IR without redetermining semantics from AST or semantic-analysis state.
3. Backend-specific Rust IR is internal. Public observations are HIR/MIR/SSA text and generated artifacts, not stable serialization of internal structures.
4. Optimization is never implicit; future passes expose selection and transformation.
5. Bindings have deterministic compilation-local IDs that distinguish references across shadowing.
6. HIR retains `if`, `while`, `for`, `break`, and `continue`, including separate for initializer/condition/body/update. MIR, bytecode, and backend lowering introduce branches, merges, updates, back edges, and exits.
7. HIR statement/expression `NodeId` values are unique within a compilation. Element identity is distinct from source-location `Span`; MIR block/instruction IDs use separate namespaces.

Rust module layout varies by route. Native uses MIR→LIR→ASM; other generating backends use HIR→backend IR→artifact.

## Cerune IR

Cerune IR is the typed, backend-independent representation produced after parsing, semantic validation, and type resolution.

Every Cerune IR expression has a resolved concrete type:

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
named product types (including expanded sum types)
fixed and dynamic arrays
```

`infer` is resolved before Cerune IR is produced and therefore does not appear as a runtime or backend type.

The AST, semantic model, Cerune IR, and bytecode share `IntegerType`, with `I8`, `U8`, `I16`, `U16`, `I32`, `U32`, `I64`, and `U64` kinds exposing names, signedness, bit widths, and bounds. VM values retain their integer kind; instructions check type agreement and result ranges.

Unsuffixed floating-point literals are also resolved before backend lowering. A backend does not need to repeat contextual type inference.

Integer literals retain their decimal digits in lexer tokens and the AST. Semantic analysis checks the range against the expected integer type, and construction of Cerune IR converts the literal into a resolved value. This also permits `u64` literals above the maximum `i64` value.

The integer conversion spellings `i32(value)` and `convert<i32>(value)` resolve to the same `ConvertInteger`, with original spelling retained as `ConversionSyntax`. The destination does not retype the input, which is evaluated once. All pairs of the eight implemented integer types are supported. Out-of-range conversion is a distinct VM error from arithmetic overflow. Bytecode conversion instructions retain both integer kinds and source origin.

Explicit conversions involving floats resolve to `ConvertNumeric`. Shared `NumericType` distinguishes integer kinds, `f32`, and `f64`; Cerune IR and bytecode retain both source and destination types. Backend IR retains both in `NumericConversion`, so emitters do not infer input types again. Existing integer-only observation forms are unchanged.

Conversion succeeds only if it preserves the already evaluated input exactly. Range checks and round-trip comparisons detect lost fractional parts or precision. Backends such as C and LLVM check bounds before converting a floating-point value to an integer. Since the floating-point representation of `i64` maximum rounds up to 2^63, the upper bound is checked as strictly less than the exactly representable maximum-plus-one, not less than or equal to a rounded maximum.

Float width changes preserve infinity and zero signs and reject NaN. Conversion to integers rejects infinity, NaN, and negative zero. Same-type conversion returns the original value, including NaN payloads, unchanged. Lowering omits the execution operation in this case while retaining explicit conversion in Cerune IR and bytecode. These conversion rules do not prohibit rounding in ordinary floating-point arithmetic.

Current code generation backends store all eight integer kinds in 64-bit values. For `u64`, the high bit is part of the positive value: lowering selects unsigned comparison, division, and right shift. C uses `uint64_t`; other targets use 64-bit storage with unsigned instructions. Arithmetic and integer-to-integer conversion results of 8-, 16-, and 32-bit integers retain explicit `CheckIntegerRange` operations in backend IR. Float-to-integer range checks are part of `ConvertNumeric`. The existing overflow checks also apply to the underlying 64-bit arithmetic.

This implements numerical ranges and failure conditions, not packed 8-, 16-, or 32-bit storage or external ABI support. Arrays and products also use 64-bit locations, so memory use does not yet decrease. Keep the semantic type in Cerune IR distinct from the backend's chosen storage type.

Cerune IR deliberately does not attempt to be a universal machine IR or prematurely impose SSA form. It represents Cerune semantics closely enough to keep the frontend/backend boundary visible.

Cerune IR statements and expressions share one sequence of `NodeId` values. `emit-ir` renders them as `#0`, `#1`, and so on. IDs are allocated deterministically, with a parent before its children and in textual IR order, so the same Cerune version and input produce the same IDs.

A `NodeId` refers to an element within one compilation result. It is not stable across source edits or Cerune versions. Multiple IR elements with the same `Span` can still have different `NodeId` values. This distinction provides a foundation for recording how one expression is later split into multiple backend instructions without relying on source locations as identity.

Type names and suffixes resolve through `IntegerType::ALL`. C, LLVM, QBE, and WAT collect required range checks in an ordered set of integer kinds and emit each helper once. This reduces registration omissions when adding types and keeps artifact ordering deterministic.

## Backend lowering

### Short-circuit logical expressions

The AST and Cerune IR retain `&&` and `||` as `Logical`, separate from eager binary operations. The frontend checks that both operands are `bool` and resolves the condition for executing the right operand as Cerune semantics. A constant left operand that allows skipping does not bypass name resolution, type checking, or IR construction for the right operand.

Bytecode lowering uses a conditional jump consuming the left value and a jump to the merge point. The branch carries the logical expression's `NodeId` and the left operand's `Span`; right-operand instructions retain their own origins. A failure in an evaluated right operand therefore points to the failed operation, not merely the containing logical expression.

C uses short-circuit logical expressions, and WAT uses a Boolean-producing `if`. LLVM and QBE store the left result in compiler-generated storage, update it only in the right-operand branch, and load it after merging. Windows/Linux x86-64 lowers MIR temporaries and conditional blocks into LIR storage and branches. None moves right-operand evaluation outside its branch.

These transformations are observable in HIR, MIR, and artifacts, separately from execution traces.

### Remainder and bit operations

The AST and Cerune IR distinguish remainder, AND, OR, XOR, left/right shifts, and unary complement, retaining the original integer kind. Bytecode preserves typed instructions and the expression's `NodeId` and `Span`. The VM calculates left shifts in a wider integer before checking the original range, with separate errors for invalid shift counts and zero remainder divisors.

Code generation passes an integer-only `IntegerBinaryOp` and the original `IntegerType` into each backend IR's `IntegerBinary`. Unary `~` lowers to XOR with `-1` for signed types or the original type's maximum for unsigned types. Thus `~0u8` remains 255 even with 64-bit storage. Emitters do not reinterpret Cerune IR or repeat type inference.

C, LLVM, QBE, and WAT collect required operation/type pairs in an ordered set and emit each helper once. C uses per-expression local temporaries and the comma operator to sequence operands, without moving expressions outside short-circuit branches or loops. Windows/Linux x86-64 lowers to register operations and explicit checking branches.

Shift counts are checked against the original width first. Left shift then checks the permitted input bounds before shifting, avoiding lost bits that a result-only check could miss. C uses checked multiplication rather than shifting negative signed values. Signed right shift is also independent of accidental target behavior. Signed minimum `% -1` returns zero without overflowing a division instruction.

### Target-specific lowering

Native lowers MIR into x86-64 LIR; other code generation backends lower HIR into their own Rust representations.

An expression represented as one integer operation in Cerune IR may become the operation plus an overflow check during backend lowering. The check is not left to accidental behavior in an external tool. Backend IR retains it as a checked integer operation or an explicit trap condition. The generated artifact exposes the target-appropriate result, such as a helper call, an overflow-flag branch, or `unreachable`.

The current output routes and implementation boundaries are:

| Output route | Backend-internal representation | Emitted artifact |
| --- | --- | --- |
| C | C IR | `.c` |
| LLVM | LLVM IR representation | `.ll` |
| QBE | QBE IR representation | `.ssa` |
| WebAssembly | WAT-oriented instruction IR | `.wat` |
| Direct x86-64 Windows/Linux assembly | x86-64 LIR lowered from MIR | `.s` |
| Native object | shared ASM reader, instruction encoding, ELF/COFF writer | `.o` / `.obj` |
| Cerune bytecode | `BytecodeProgram` | `.cebc` |

Backend IR is allowed to encode decisions that do not belong in Cerune IR.

Examples include:

- LLVM/QBE temporary values and backend instructions;
- WAT stack-machine instructions and locals;
- C-specific types and print formats;
- x86-64 stack slots, frame size, constants, registers, and ABI operations;
- bytecode slots and VM instructions.

These representations are not currently public serialization formats and are not part of the stable CLI contract.

### Source locations and bytecode instruction provenance

Cerune IR statements and expressions retain their corresponding UTF-8 byte ranges in the source. A range includes its start and excludes its end. Line and column numbers are derived from this range when displayed.

Each bytecode instruction stores one of the following origins separately from the instruction itself:

- `Source { node_id, span }`: the instruction was lowered from a Cerune IR statement or expression;
- `Synthetic`: the compiler generated the instruction without a directly corresponding source range.

`Synthetic` does not mean that provenance was lost. It explicitly identifies compiler-generated instructions.

The `node_id` identifies the Cerune IR element that produced an instruction. When one IR element lowers into several instructions, those instructions may share the same `node_id`. The `span` is the focused source range used for diagnostics and does not have to cover the whole IR element. For example, bounds checks produced by a nested array-element assignment share the assignment statement's `node_id` while retaining a different index `span` for each check.

The VM reports an execution error using its bytecode instruction index. `run_vm` resolves that instruction's origin and associates the Cerune IR `NodeId` and, when available, a source location with the execution error. This provenance is currently an internal representation and is not included in the `emit-bytecode` text format.

## Observation boundaries

HIR, MIR, and generated artifacts are observable. These are distinct from runtime results and execution traces.

### Observation 1: resolved Cerune meaning

`emit-ir <file> [-o <output.ceir>]` emits backend-independent HIR after parsing, semantic validation, expression typing, and contextual float resolution. It explains how the source was interpreted, without ABI, physical value allocation, stack-machine, or target-instruction decisions.

### MIR: explicit execution steps

`emit-mir <file> [-o <output.txt>]` emits non-SSA MIR with typed locals/temporaries, blocks/branches, ownership operations, and original provenance. `Cerune MIR v0.1` is observation text, not a stable distribution format or loadable Image. `run-mir` independently executes that representation.

### Observation 2: output artifact

Artifacts show how the selected route and target represent resolved meaning.

| Command | Artifact | Options |
| --- | --- | --- |
| `emit-c` | `.c` | `-o` to save |
| `emit-llvm` | `.ll` | `--target`, `--annotate-origins`, `-o` |
| `emit-qbe` | `.ssa` | `--target`, `-o` |
| `emit-wat` | `.wat` | `-o` to save |
| `emit-asm` | `.s` | `--target`, `--annotate-origins`, `-o` |
| `emit-obj` | `.o` / `.obj` | Required `--target`/`-o`; optional origins |
| `emit-bytecode` | `.cebc` | `-o` to save |

Each command accepts `<file>`. The [CLI reference](../reference/cli.en.md) defines features requiring targets and omission behavior. Individual observations use `emit-*`. [`observe`](observation-bundle.en.md) collects existing observations from one compilation into a bundle; it is not another semantic representation or execution route.

### Internal backend IR is not an observation contract

Internal Rust structures are not part of a stable v0.1 serialization contract. `emit-lir` provides [versioned observation text](lir-observation.en.md), not a loadable distribution format.

## Code generation

Cerune avoids unobservable, implicit source-level optimization rather than optimization itself.

Backend lowering may perform advanced transformations as well as the mechanical transformations required by its target representation. If a transformation removes structure that is useful to observe, it must be treated as an explicit, observable pass.

Examples of legitimate backend lowering include:

- selecting typed LLVM or QBE instructions;
- converting a Cerune expression into WebAssembly stack instructions;
- mapping Cerune values to C types and expressions;
- allocating Direct ASM stack slots and materializing constants;
- lowering Cerune operations into bytecode instructions.

If optimization is introduced later, it should appear as a named pass with an explicit boundary rather than being hidden inside emission.

## Tool responsibilities

Cerune, Tint*, and Whitebase have different responsibilities.

### Cerune

Cerune owns compiler transformation and emission:

```text
Parse
  ↓
Resolve
  ↓
Cerune IR
  ↓
Lower
  ↓
Backend IR
  ↓
Emit
```

Cerune defines and produces the observable compiler artifacts.

### Tint*

Tint* is a visual development and inspection environment for Cerune.

It should consume Cerune's public CLI observations rather than duplicate compiler semantics. Its role is to make source and generated representations easy to inspect and compare interactively.

### Whitebase

Whitebase currently runs, measures, and compares built-in Rust, C++, and Assembly operations. It does not yet accept Cerune source or artifacts and build and run them through external compilers.

The intended integration lets Whitebase consume emitted artifacts as experiment inputs, select build routes, build, run, measure, and compare them while recording the external choices that affect the experiment. The following diagram describes that responsibility boundary, not an existing implementation.

```text
Cerune source
      ↓
Cerune
      ↓
Observation artifact
      ↓
Whitebase
  - select build route
  - invoke external tools
  - run
  - measure
  - compare
```

Whitebase should treat Cerune as an external tool boundary rather than depending on Cerune's internal Rust IR.

Cerune's regression tests verify that types, diagnostics, artifacts, and execution results follow language rules. Development tests using external compilers to check emitted code can belong to Cerune for that purpose. User-facing selection of toolchains and conditions, experiment storage, and comparison belong to the consumer. This does not prescribe a single test storage format or comparison method.

Generation support, build readiness, execution readiness, and permission to execute are separate. See [Artifact consumer boundary](targets.en.md#artifact-consumer-boundary). Clarifying this boundary does not require a new public API or make language features such as strings wait for integration to be completed.

## Reproducibility

Observation artifacts are most useful when they can be compared directly.

For the same Cerune version, source input, output route, target, target features, and explicit options, Cerune should produce deterministic textual observations whenever practical.

Cerune-generated observations should therefore avoid incidental values such as timestamps, random identifiers, or environment-dependent metadata unless such data is itself the subject of an experiment.

External toolchain output is outside this guarantee and should be recorded by the consumer, such as Whitebase.

## Non-goals and future work

The current design intentionally does not require:

- a public serialization format for backend-specific Rust IR;
- another observation API redefining semantics alongside individual emits and bundles;
- mandatory SSA for every backend (MIR SSA conversion is a future explicit option);
- implicit optimization;
- orchestration of external compilers and runtimes for emit-only routes (distinct from planned Cerune-owned builds);
- benchmarking or performance measurement inside Cerune.

Possible future work includes:

- an explicit optimization pipeline with additional observation boundaries;
- optional backend-IR inspection when a concrete use case requires it;
- extending observation bundles with objects, other backends, and before/after comparisons;
- verified, loadable IR/VM Images as completed artifacts, with formats and verifiers described in the [build / release plan](owned-routes.en.md).

Those features should be added only when they preserve the central rule:

> Cerune should make transformations easier to observe and easier to explain.
