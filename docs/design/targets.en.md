# Output routes and targets

[日本語](targets.ja.md)

This document distinguishes output routes, targets, artifacts, and backends in Cerune's generated output.

This distinction is necessary to identify observations from the same Cerune program correctly and to preserve room for additional instruction sets and execution environments.

## Terminology

### Output route

An output route describes which kind of representation is produced from Cerune IR.

The current output routes are C, LLVM IR, QBE IR, WebAssembly Text, native assembly, native objects, and Cerune bytecode.

Native uses [HIR→MIR→x86-64 LIR](native-mir.en.md). MIR is a shared execution representation, not a new OS target.

### Direct execution

The [IR Executor](ir-executor.en.md), selected by `run` or `run-ir`, directly executes completed Cerune IR. It produces no new artifact and selects no target ABI, so it is separate from the output-route table. `run-vm` selects Bytecode → VM. `run-mir` executes HIR→MIR using the [independent MIR executor](mir-executor.en.md); `emit-mir` observes that representation.

`run-mir --ssa` evaluates SSA directly. LIR is an internal Native generation stage, with no `run-lir` or dedicated LIR observation command. See the [stage inventory](development-status.en.md).

### Target

A target describes the execution environment assumed by an artifact. It has the following elements when they are relevant:

- instruction set architecture, such as `x86_64`, `riscv64`, or `wasm32`;
- execution environment, such as Windows, Linux, or bare metal;
- ABI, such as Windows x64, System V, or RISC-V LP64D;
- features, such as the RISC-V M, A, F, and D extensions.

Some output routes do not have a target selected by Cerune. For example, the caller of an external C compiler may decide which environment a C source artifact is compiled for.

### Artifact

An artifact is the actual observable output, such as C source, LLVM IR, assembly, or Cerune bytecode.

An artifact format may have attributes such as assembly syntax or file format. Even when two artifacts are both assembly, GNU AT&T syntax and Intel syntax are distinct formats.

### Backend

A backend is an internal implementation component that lowers Cerune IR for a particular output route and target and then produces an artifact.

Backend is an implementation term. It is not used as a collective name for an output route, target, and artifact.

Dynamic arrays `[T]` currently execute in IR Executor, MIR Executor, Bytecode VM, generated C, LLVM, QBE, WAT, Windows/Linux ASM, and self-encoded COFF/ELF. See [support status](owned-arrays.en.md).

## Execution-route responsibilities

IR, VM, and Native are routes for which Cerune defines completed artifacts and execution contracts. C, LLVM, QBE, WAT, and future Lean-like outputs are emit-only: consumers manage external build, execution, and proof checking. Cerune remains responsible for preserving semantics in generated output.

The [route diagram and build / release plan](owned-routes.en.md) include future operations. The following table describes current output.

## Current configuration

The current outputs can be described as follows:

| Output route | Target | Artifact |
| --- | --- | --- |
| C | not selected by Cerune | C source `.c` |
| LLVM IR | unspecified, or explicit Windows x64 / Linux x86-64 (required for strings and runtime checks) | LLVM IR `.ll` |
| QBE IR | unspecified, or explicit Windows x64 / Linux x86-64 (required for strings) | QBE IR `.ssa` |
| WebAssembly Text | WebAssembly | WAT `.wat` |
| Native assembly | x86-64, Windows / Linux, respective calling conventions | GNU-style assembly `.s` |
| Native object | explicit Windows x64 / Linux x86-64 | COFF `.obj` / ELF64 `.o` |
| Cerune bytecode | Cerune VM | Cerune bytecode `.cebc` |

"Not selected by Cerune" does not mean inferred implicitly from the host environment. It means that Cerune does not include target-specific decisions in that observation and that the caller of a downstream tool selects the target.

### QBE validation version

CI uses [QBE 1.3](https://c9x.me/compile/releases.html), verified against its official SHA-256. Execution comparisons use Linux `amd64_sysv` and Windows `amd64_win`. On Windows, MinGW/UCRT builds QBE itself; Clang/MSVC links its generated assembly. The tool's build environment and the generated program's target are separate.

See [QBE Windows support](qbe-windows.en.md) for output, diagnostics, argument passing, and validation of each stage.

## Artifact consumer boundary

An existing output route does not imply support for every language feature. Strings now lower through every route. LLVM and QBE diagnose a missing target before lowering, including unused definitions. Successful semantic validation is distinct from successful generation through each route.

LLVM `--target` selects `x86_64-unknown-linux-gnu` or `x86_64-pc-windows-msvc` and records a `target triple` in the artifact. Programs using strings use this selection to initialize Windows standard output in binary mode. Omitting the target for strings produces a source-located diagnostic; the host never supplies a default. Numeric-only programs with runtime checks also require a target to select the diagnostic output ABI. See [string design](strings.en.md#llvm-representation-and-targets) for representation and initialization details.

Artifact comparison separates the following questions. Support information is data, not permission to launch external programs.

QBE pairs explicit `x86_64-unknown-linux-gnu` / `x86_64-pc-windows-msvc` selection with downstream `amd64_sysv` / `amd64_win`, respectively. Direct assembly accepts explicit Windows/Linux x86-64 selection, and WAT remains fixed to WebAssembly; WAT string output has a host contract accepting byte values. Route-specific storage and output choices remain documented in [string design](strings.en.md).

This table describes consumers processing current emitted artifacts. Planned Cerune-owned builds make Cerune responsible for managing the tools and dependencies needed to construct the completed artifact.

| Question | What is checked | Owner |
| --- | --- | --- |
| Can it be generated? | Whether the Cerune version supports the output route and target; source-specific errors are diagnosed during generation | Cerune |
| Can it be built? | Whether suitable compilers, assemblers, linkers, and required libraries are available | Consumer |
| Can it be executed? | Whether CPU, OS, runtime libraries, and host functions meet the requirements | Consumer |
| May it be executed? | Whether user authorization and execution-environment restrictions permit it | Consumer execution policy |

Finding an external tool does not establish that a build or execution will succeed. Keep preflight checks separate from actual stage results, distinguishing unsupported routes, missing tools, policy denial, generation failure, build failure, execution failure, and timeout. Do not silently drop unexecutable routes or count them as successes.

The environment running Cerune is separate from the artifact target. Calling `emit-asm` in WSL still produces GNU AT&T assembly for Windows x86-64. Select Linux assembly explicitly with `--target x86_64-unknown-linux-gnu`. Even on Windows, Cerune's internal aggregate passing convention does not guarantee external C ABI compatibility.

### What comparison establishes

The current CLI directly executes IR through `run` or `run-ir`, runs bytecode in the VM through `run-vm` and returns artifacts through `emit-*`. Comparing exit status and standard output through these commands does not require a new public API. There is no CLI for loading and executing `.cebc` files in the VM.

Matching standard output establishes matching displayed results. It does not prove equality of unprinted information such as floating-point NaN payload bits. If bitwise comparison becomes necessary, design a separate typed observation format. Numeric tolerances and newline normalization must also be explicit comparison conditions when used.

Do not treat the VM alone as an oracle. Retain checks against known expected results and language rules. Two nonzero exits do not establish the same cause of failure. Preserve the failing stage (generation, build, or execution) and each stage's diagnostics.

### Recording and safety

The consumer records input and artifact identity, Cerune build identity, output route, actual target, external-tool versions and explicit options, stage results, and comparison conditions. `cerune --version` reports the package version. To distinguish development builds sharing a version, also record the verified commit or executable hash. A machine-readable supported-route listing is not implemented. Define a versioned public schema from actual consumer needs when one becomes necessary.

Do not interpret strings in artifacts or observation data as executable commands or authorization. Executing generated code is a separate operation from reading observations. The consumer selects trusted tools and arguments and runs untrusted code in appropriate isolation. Timeouts and separate processes alone are not a security sandbox; also bound captured output. Do not unconditionally record source text, paths, or entire environments.

## Observation identity

When a target affects an artifact, an observation is identified by at least the following conditions:

- Cerune version;
- source input;
- output route;
- target;
- target features;
- explicit options.

When these conditions are the same, Cerune produces a deterministic observation.

Target information does not have to be embedded in the artifact itself. However, the output route and target that produced an artifact must be identifiable from the CLI invocation or associated metadata.

Selecting a target implicitly from the host OS, host CPU, or environment variables would allow the same explicit input to produce different results. Cerune does not make implicit target choices that affect observations.

## Lowering boundary

Cerune IR preserves meaning that is independent of output routes and targets.

Decisions such as the following are made after backend lowering begins:

- instruction selection;
- register use;
- stack-frame layout;
- value size, layout, and alignment;
- calling conventions;
- instructions selected for target features;
- assembly syntax and artifact format.

Target-specific IR may remain an internal boundary of each backend. Cerune does not require a universal machine IR shared by every target.

## Adding a target

Adding a target preserves the following properties:

- the target has a stable identity;
- supported combinations of output routes and targets are explicit;
- target-specific decisions do not enter Cerune IR;
- unsupported combinations produce diagnostics;
- target features have deterministic ordering and representation;
- the same explicit conditions produce the same artifact.

Adding a target does not grant new authority to an observation API. A target identifier is data that selects lowering conditions, not authority to execute external commands or mutate compiler state.

Selection and execution of external assemblers, linkers, and compilers remain the responsibility of the consumer of Cerune artifacts.

See [native code](native-code.en.md) for machine-code object and executable generation and observation.
