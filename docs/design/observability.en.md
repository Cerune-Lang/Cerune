# Observability contract

[日本語](observability.ja.md)

## Purpose

This document explains what Cerune records and displays to connect source statements and expressions to IR, MIR, and generated code, as well as what observation APIs cannot change.

Cerune provides information for examining compilation results, but it does not provide an observation API for changing type-checking results or generated instructions.

## Separation of observation and control

- Observation APIs return information about compiler types, IR, and instruction-selection results; they provide no operation for changing those results.
- External code cannot directly replace internal IR instructions or type-checking results through an observation API.
- If plugins or hooks that replace compilation steps are introduced later, their operations and permission requirements must be defined separately from APIs that display IR.
- A `NodeId` or another identifier in an observation result cannot be used to delete or replace an internal compiler instruction.

## Internal observability

- Cerune records source locations for tokens, AST nodes, semantic-analysis results, and IR so that statements and expressions can be followed through these stages.
- Every Cerune IR statement and expression has a deterministic `NodeId` unique within one compilation. `NodeId` identity is distinct from source-location `Span` data.
- Syntax elements retain source start and end positions. Each bytecode instruction records its Cerune IR `NodeId` and diagnostic `Span`, or an indication that the compiler generated it.
- `Synthetic` identifies operations intentionally added by the compiler. It must not be substituted for a `NodeId` or `Span` lost during compilation.
- Observation APIs return information extracted for display or inspection, not mutable access to the data being changed inside the compiler.
- Data extracted for observation is not fed back into type checking or instruction selection to change the generated result.

## Public observation surfaces

- `emit-ir` displays Cerune IR (HIR), `emit-mir` displays MIR, and other `emit-*` commands expose generated C, LLVM IR, QBE IR, WAT, ASM, Object files, and related artifacts.
- Cerune IR text exposes statement and expression `NodeId` values as `#N`.
- Not all information retained inside the compiler is accessible through a public API. The internal representation of that information is not guaranteed to stay the same in future versions.
- When introducing a new public observation format, document its version and the meaning of each field.

`emit-mir` displays types, expression evaluation order, ownership operations, branches, and connections to source locations in `Cerune MIR v0.1` text. There is no feature for distributing this text and loading it directly for execution. MIR block/instruction numbers and HIR `NodeId` values refer to different elements even when the numbers happen to match.

## Security

- Source text and file paths are disclosed only when explicitly requested by the user.
- When displaying diagnostics or source snippets in a terminal, handle control characters and escape sequences so that they cannot trigger unintended terminal actions.
- Limit the number of trace entries, the length of diagnostic messages, and the length of displayed source snippets.
- Do not automatically include environment-variable lists, secret keys or tokens, or execution-environment details unnecessary for checking compilation results in observation output.

## Reproducibility

- Adding source-origin comments or symbols with options such as `--annotate-origins` does not change the behavior of generated instructions or program execution results. Annotated and unannotated outputs can differ in comments or object-file symbol tables.
- The same Cerune version, source, output route, target, CPU features, and explicitly supplied options produce the same observation results.
- `NodeId` values are deterministic under those conditions, but they are not persistent IDs across source edits or Cerune versions.
- Observation results do not automatically include the current timestamp or randomly generated identifiers that change between runs.

## LLVM origin annotations

With the [registered-file API](source-files.en.md), both LLVM and assembly add `file=N` between `#N` and `bytes`. Ranges are local to that file. Existing single-source APIs and the CLI omit this field.

`emit-llvm --annotate-origins` adds read-only comments to the existing LLVM artifact. The header identifies the format as `; cerune-origins v1`. A comment such as `; cerune-origin: #7 bytes 208..235` connects the following instruction group to node #7 in `emit-ir`. These offsets identify `echo(left) == "日本語\0"` in the LF version of `examples/string_origins.ceru`. Ranges are UTF-8 byte offsets into the original input, with an exclusive end; CRLF input changes offsets.

The lowerer records `Source { NodeId, Span }` or `Synthetic` for each instruction. For example, after converting the left and right sides of a comparison into instructions, it attaches the comparison expression's own `NodeId` and `Span` to the comparison instruction. This prevents comparison, output, and short-circuit branches from being mistakenly attributed to a child expression. The emitter uses the recorded backend IR. An annotation applies until the next annotation. One source expression may produce several instructions; literals that produce no instruction have no individual annotation. Module constant storage, shared helpers, stack allocation, parameter stores, and startup scaffolding are explicitly synthetic. Synthetic does not mean lost origin information. Follow source origins at use sites; mappings after external optimization and debug information are outside this contract.

Annotations contain no source text or paths. The option affects comments at emission only: removing the annotations reproduces ordinary LLVM exactly. Tests verify this for every example and execute annotated and ordinary LLVM for `string_origins.ceru`. Paired IR/LLVM fixtures are in `tests/fixtures/observation/string-origins/expected/`.

LLVM `cerune-origin` comments let readers find the source `NodeId` for generated instructions. They do not provide an API for changing internal compiler IR.

## ASM and machine-code origins

`emit-asm --annotate-origins` also preserves NodeId and UTF-8 byte ranges. Comments and `cerune_origin_nN_...` labels connect object symbols and disassembly to Cerune IR. Paired IR/ASM fixtures for both targets live in `tests/fixtures/observation/native-values/expected/`. Tests compare annotation-stripped ASM with ordinary output and compare instruction bytes with and without annotations.

Native follows [HIR→MIR→LIR→ASM/Object](native-mir.en.md). Annotated ASM also contains `# cerune-mir: v1` correspondence, and objects retain `cerune_origin_mir_...` symbols. One MIR operation can produce multiple LIR instructions; LIR checks and ABI operations expand further into ASM instructions, so instruction counts are not one-to-one. The records distinguish instructions generated from source expressions, instructions added for required runtime checks, and compiler-added instructions such as stack setup.

Machine code is produced through [Sashimi-backed Object generation](native-encoder.en.md) or by assembling ASM with an external assembler. Linking uses an explicitly selected external tool. Record Object-generation success, expected traps, successful execution, and unexpected failures separately. See [Following ASM into machine code](native-code.en.md) for the mapping contract, address interpretation, and execution commands.
