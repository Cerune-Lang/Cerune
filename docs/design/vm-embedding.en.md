# VM Embedding

[日本語](vm-embedding.ja.md)

The Rust API for f64 arguments and f64/void results is implemented. [Typed host calls](typed-host-calls.en.md) covers JS/TS exposure and comparisons with generated WAT.

## Purpose

Define the boundary for invoking functions in already generated Cerune bytecode from a host application such as a Rust program.

Cerune already provides a path from source to bytecode and a VM that executes that bytecode.

```text
Cerune Source
      ↓
Cerune IR
      ↓
Bytecode
      ↓
Cerune VM
```

Normal Cerune execution starts from top-level statements or `main`.

Embedding adds a host-facing boundary that allows an application to keep a compiled `BytecodeProgram`, select a function from it, pass arguments, and receive its return value.

This is not a new Cerune language feature. It exposes existing function, bytecode, and VM execution capabilities to a host application.

## Responsibilities

Cerune is responsible for:

```text
BytecodeProgram
      +
function
      +
arguments
      ↓
Cerune VM
      ↓
return value
runtime output
runtime error
```

The host application is responsible for:

* storing Cerune source,
* deciding when compilation occurs,
* selecting the function to invoke,
* converting application-specific data to and from host values,
* consuming execution results,
* and applying measurement or caching when needed.

The VM embedding API does not know about a specific application's backend contract or measurement model.

## Boundary with Emitters

VM embedding and execution of emitted artifacts remain separate responsibilities.

Cerune emitters continue to produce backend artifacts.

```text
Cerune IR
      ↓
Backend Lowering
      ↓
Emitter
      ↓
Backend Artifact
```

Consumers choose tools to build, link, load, and execute current emitted artifacts. Planned [IR/VM/Native build / release](owned-routes.en.md) operations are separate from the VM embedding API. C, LLVM, QBE, WAT, and similar routes remain emit-only.

Cerune does not add:

* C compiler discovery or invocation,
* LLVM toolchain management,
* QBE discovery or invocation,
* Wasm runtime management,
* assembler or linker management,
* dynamic loading,
* or benchmarking.

Cerune therefore does not require external toolchains merely to build and use the language implementation itself.

## Relationship to the Existing VM

The VM stores function identifiers, PCs, operand stacks, slots, and initializer positions in explicit per-call frames, processing call/return iteratively. Original bytecode instructions and origins are preserved. [Depth limits and allocation failures](recursion.en.md) are shared with the other executors.

The embedding API must reuse this existing execution path rather than implementing a second function evaluator.

```text
Host Value
    ↓
VM Value
    ↓
existing function execution
    ↓
VM Value
    ↓
Host Value
```

Internal VM representations are not made part of the stable public API.

Types such as the internal `Value`, `Frame`, and slot representation remain implementation details that may evolve independently.

## Separating Compilation from Execution

Embedding must allow compilation and function execution to happen at different times.

```text
Preparation

Cerune Source
      ↓
compile_to_bytecode
      ↓
BytecodeProgram
      ↓
function resolution


Execution

arguments
      ↓
VM function invocation
      ↓
result
```

A host can execute the same compiled `BytecodeProgram` repeatedly without recompiling the original source for every call.

This also allows applications to measure compilation and execution separately.

## Function Identification

The VM internally identifies functions numerically.

The public API does not treat those internal identifiers as permanent external identities.

Function resolution and repeated invocation of an already resolved function should be separable operations.

Conceptually:

```text
BytecodeProgram
      ↓
resolve function
      ↓
resolved function
      ↓
repeated invocation
```

Concrete public types are decided during implementation.

If generic specialization or another feature introduces multiple functions that cannot be identified safely by a simple source name, the resolution contract will be designed explicitly at that point. A simple name lookup is not automatically established as Cerune's permanent function identity model.

## Host Values

The public API uses a host value representation distinct from the VM's internal `Value`.

The initial API exposes only the value kinds required by actual embedding use cases.

The first Whitebase integration can be established with scalar `f64` alone, so unrelated value kinds do not need to be frozen into the public API in advance.

Arrays, product types, sum types, strings, and other values are added only after their representation, ownership, copy behavior, and error boundary are defined.

Integer host values must also preserve Cerune's range rules. Host conversion must not allow values that ordinary Cerune evaluation could not construct to enter the VM.

## Execution Result

Function invocation must preserve more than only the return value. Runtime output and structured VM failures also belong to the execution boundary.

Conceptually:

```text
Function Execution
├── return value
└── output
```

Failures should preserve the existing VM error and bytecode instruction origin information.

The same VM failure should not acquire different semantics merely because it was reached through the embedding API instead of `run_bytecode`.

## Non-goals

The VM embedding API does not:

* add new Cerune function syntax,
* add an `export fn` or external ABI declaration,
* execute emitted artifacts,
* manage external compilers or runtimes,
* unify the VM and all emitters behind a speculative execution trait,
* add APIs for a specific embedding application,
* expose the VM's internal value representation as the stable public API,
* or expose every Cerune value type from the first version.

## Implementation Order

The first implementation proceeds in this order:

1. Make the VM's existing function execution path reusable through a crate-internal boundary.
2. Define the minimal host value representation.
3. Resolve a function from an already generated `BytecodeProgram`.
4. Invoke it with host arguments and return its value and output.
5. Preserve VM errors and instruction origin information consistently with existing execution.
6. Test repeated execution of compiled bytecode without recompilation.

This API does not manage external build or execution for emit-only routes. Planned Cerune-owned build / release operations have a separate responsibility.
