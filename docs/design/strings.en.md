# String design

[日本語](strings.ja.md)

## Value semantics

`string` is an immutable UTF-8 value. Reassignment to a `mut` binding replaces the stored value; it does not modify that string's contents. Previously copied values and values passed to functions remain unchanged.

Comparison uses the entire contents without Unicode normalization or case conversion. Embedded NUL is part of the value. Syntax and operators are specified in the [language reference](../reference/language.en.md#strings).

## Observation and representation

| Stage | Current representation |
| --- | --- |
| Cerune IR | The `string` type and decoded contents; Span identifies the original quoted spelling |
| Bytecode | `push.string`, typed storage, comparison, and output instructions; source-derived instructions retain NodeId and Span |
| VM | Shared immutable content handles; dynamic allocations have separate logical retain/release accounting |
| C | A struct containing a read-only data pointer and a UTF-8 byte count |
| LLVM | `%cerune.string = type { ptr, i64 }` and static/dynamic byte storage |
| QBE | A 64-bit reference to immutable storage containing an eight-byte length and UTF-8 data |
| WAT | A 32-bit reference to length-prefixed data in private linear memory |
| Windows/Linux x86-64 direct assembly | A reference to immutable contents held in registers and stack slots |

Textual IR and bytecode escape line breaks and control characters. `print` writes the contents unchanged. Presentation does not alter the value.

Cerune IR resolves shared semantics; each lowerer selects storage, copies, instructions, and calling conventions. Emitters turn backend IR into artifacts without reinterpreting Cerune types or semantics. The priority is making each transformation and its reasons observable, rather than requiring a single representation. The observation boundaries remain `emit-ir` and generated artifacts.

## C storage lifetime

```c
typedef struct cerune_string {
    const unsigned char *data;
    size_t length;
} cerune_string;
```

Static strings originate from source literals or compile-time evaluation. Their data is emitted as C string literals with static storage duration, retained until process exit. Returning strings or arrays containing strings from functions does not invalidate the data.

Assignment copies the pointer and byte count. Sharing the data preserves immutable value semantics because Cerune exposes no operation to change those contents. Equality compares contents, not sharing or addresses. Static strings need no ownership management; `concat` results use `malloc`, `free`, and reference counting.

Runtime concatenation, budgets, retain, and release are implemented as described in the [dynamic-data design](dynamic-data.en.md). Common IR determines lifetime; each target manages storage.

External input remains unimplemented. The generated C struct is not a stable external-integration ABI.

## Bytes and evaluation order

Each literal byte is emitted as a fixed three-digit octal escape. Japanese text does not depend on the C compiler's execution encoding, and quotes or other contents cannot become C syntax.

Equality checks byte counts before using `memcmp`. Printing passes length-delimited data and LF to the [write operation](standard-io.en.md), retaining requested/accepted counts and origin; it never uses `strlen` or `strcmp`. Windows uses binary output even for numeric-only programs, preserving CR, LF, and NUL.

Where C does not guarantee evaluation order and multiple operands can produce effects or fail, expressions are saved to function-local temporaries in source order. Evaluation stays inside short-circuit operands and loop conditions or updates. Binding names also include Cerune IR IDs to avoid shadowing and runtime-helper collisions.

## LLVM representation and targets

Static LLVM string data is constant and lasts until process exit. Each literal is stored in lowering order as `private unnamed_addr constant [N x i8]`, using fixed two-digit hexadecimal escapes for every UTF-8 byte. No terminator is added; the original byte count is retained as `i64`. Empty strings never dereference their data.

Strings, products containing strings, and nested arrays are copied as LLVM aggregate values, including function parameters and results. Finalized contents are never modified. Dynamic strings preserve this value representation and identify owned allocations through an internal list. Equality checks length and every byte regardless of sharing or constant merging. This internal representation does not guarantee external C ABI compatibility.

Printing passes length-delimited data and LF to the [origin-bearing write operation](standard-io.en.md). Numbers, booleans, and quoted output use the same connection, retaining order, requested/accepted byte counts, and failure locations. Comparisons, short writes, and interruption retries remain readable in generated LLVM.

LLVM programs using strings or writing stdout require an explicit `--target`. Two targets are supported:

| Target | Standard output initialization |
| --- | --- |
| `x86_64-unknown-linux-gnu` | No mode change |
| `x86_64-pc-windows-msvc` | Call `_setmode(1, 32768)` on CRT standard output (descriptor 1) to select binary mode |

Windows initialization runs before any Cerune operation, including a call to an explicit `main`. Failure exits with code 1. Numeric-only and boolean-only output also ends in LF. Checked operations, conversions, and indexing require a target even without output to select the [runtime diagnostic](runtime-diagnostics.en.md) output ABI.

The selection is recorded in the artifact's `target triple`; the compiler never selects it from its host OS or environment variables. Strings in unused definitions also require a target, with a source-located diagnostic when omitted. Pass the same target to downstream Clang. Overriding it with a different target does not translate already generated OS-specific operations.

LLVM lowering emits expressions in source order and retains the required branches inside short-circuit expressions and loops. Target selection is generation data, not authorization to launch external programs or interfere with the compiler.

References: [LLVM constants](https://www.llvm.org/docs/LangRef.html#constants), [target triple](https://www.llvm.org/docs/LangRef.html#target-triple), and [Microsoft CRT _setmode](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/setmode?view=msvc-170).

## QBE, WAT, and direct assembly storage and passing

These routes statically place an eight-byte length followed by the original UTF-8 bytes for each literal, and retain a reference to its beginning as the string value. Function parameters and results, product fields, and array elements use that reference. Neither length nor contents change at runtime; no terminating NUL is added. Cerune does not silently optimize by merging string literals.

Reassignment replaces the reference in a binding or array element. Shared storage is immutable, so previous copies remain unchanged. Product and array copies retain existing `blit`, memory load/store, and stack-slot copying. Common IR generates retain/release for dynamic strings; these operations have no effect on static storage. These internal references do not add a Cerune pointer type or an external ABI.

QBE puts static data in read-only `.rodata` on Linux and `.rdata` on Windows, using `loadl` for length and `loadub` for bytes. Strings and stdout operations require an explicit target. Linux `x86_64-unknown-linux-gnu` maps to `qbe -t amd64_sysv`; Windows `x86_64-pc-windows-msvc` maps to `qbe -t amd64_win`, recorded in the artifact comment. Windows initializes stdout/stderr in binary mode before any Cerune operation and exits with code 1 on initialization failure. Numeric-only output also uses LF with an explicit Windows target. Changing only the downstream QBE target does not translate Cerune's runtime assumptions.

QBE output also uses [origin-bearing writes](standard-io.en.md). Generated IL retains requested/accepted counts, short writes, interruption retries, and failure termination; quoted output is sent through a fixed 256-byte buffer.

Direct assembly supports Windows/Linux x86-64 targets. Linux uses SysV byte output without the Windows stdout initialization described below. Static lengths and bytes reside in read-only storage, and references travel through `RAX` or eight-byte stack slots. Comparisons save the left operand before evaluating the right and passing both to a helper. The output helper follows Windows x64 shadow-space, stack-alignment, and register-preservation rules. `_setmode` runs before the first Cerune operation and exits with code 1 on failure.

WAT puts data in private linear memory and uses 32-bit addresses. Its eight-byte length header is little-endian; current wasm32 operations read the low 32 bits. Lowering selects static regions and the initial page count. Dynamic strings reuse private blocks and grow memory when necessary; growth failure is checked. Equality becomes `i32.load8_u` and branches.

### WAT output and the external boundary

WAT programs using strings import `cerune.write_byte(i32) -> void`. Generated code reads each content byte, passes its value in 0–255, and finally passes LF (10). The host must preserve these bytes in order without character encoding or line-ending translation. Numbers and Booleans retain the existing `print_i64`, `print_u64`, `print_f32`, `print_f64`, and `print_bool` contracts.

Strings nested in aggregates are quoted/escaped; numeric fragments use host functions without a newline. See the [aggregate display contract](aggregate-values.en.md). Standalone string display is unchanged.

Memory is neither exported nor imported for output. Passing byte values instead of string storage references adds no output interface for modifying contents. This is a runtime output contract, not a compiler observation API or a way for observations to control compilation.

## Validation scope

Alongside C snapshots, generated C is compiled with and without optimization and compared byte-for-byte with VM output. Coverage includes Japanese text, empty strings, NUL, line breaks, non-normalizing equality, copies, returned values, nested arrays, evaluation order, and shadowed names. AddressSanitizer and UndefinedBehaviorSanitizer provide additional memory checks.

The LLVM snapshot explicitly selects Linux. On supported Windows/Linux hosts, `cargo test --test llvm_strings` compares generated LLVM, generated C, VM output, and known expectations at `-O0` and `-O2`. It checks raw CR/LF bytes, returns, products, nested arrays, copies, evaluation order, short-circuiting, and out-of-bounds access. Setting `CERUNE_TEST_LLVM_CLANG` and `CERUNE_TEST_CC` makes missing compilers a failure; CI requires comparisons on both operating systems. If unset and a default compiler is missing, execution comparisons print a reason and skip.

The `string-values` observation fixture fixes all eight artifacts. Shared inputs and known expectations in `tests/support/string_cases.rs` are checked against VM, C, LLVM, and, through `cargo test --test string_routes`, QBE, WAT, and direct assembly. Checks distinguish output bytes, evaluation order, short-circuiting, independent copies, and stopping on out-of-bounds access. Even when strings occur only in unused defaults, Windows C, LLVM, and assembly select the same output mode.

QBE and assembly run on Windows x64 and Linux x86-64. The dedicated `qbe_windows` test retains generation stages for Windows comparisons. WAT is validated and converted with WABT, then run in Node's WebAssembly engine; tests also require `main` to be its only export. The test host's floating-point output is limited to the shared fixture's exact values `1.5`, `2`, `2.5`, and `4`, not a general numeric formatter. Unavailable execution routes print a reason and are distinguished from completed comparisons. Together, both CI jobs require execution of every route.

Development tools can be selected through `CERUNE_TEST_QBE`, `CERUNE_TEST_ASM_CLANG`, `CERUNE_TEST_NODE`, and `CERUNE_TEST_WAT2WASM_JS` (WABT's `bin/wat2wasm`). An unavailable configured tool fails the test. Install WABT with `npm install --prefix target/wasm-tools --no-audit --no-fund wabt@1.0.39`. Launching these tools belongs to development tests; Cerune's emission commands do not launch them.

## Reading byte length

`byte_len(value) -> i64` reads the stored UTF-8 byte count. It does not scan or normalize contents or allocate memory. Both static and dynamic values are supported; representable lengths on supported 32/64-bit platforms fit in `i64`.

The frontend resolves arity, input type, and result type into a dedicated Cerune IR `StringByteLength` operation. In ordinary expressions, even literal lengths remain observable operations without implicit constant folding. An explicit const evaluates the operation during compilation and retains its initializer and result in IR. IR renders `byte_len.string(...)`; bytecode uses `byte_len.string`.

| Route | Length access |
| --- | --- |
| VM | Check operand type and read the owned string's byte count |
| C | Read `length`, cast to `int64_t` |
| LLVM | `extractvalue %cerune.string ..., 1` |
| QBE | `loadl` from the length header |
| WAT | `i64.load` from the private memory header |
| Windows/Linux x86-64 ASM | `movq (%rax), %rax` from the length header |

The argument is evaluated once before the read. C sequencing follows observable or fallible child expressions, preserving source order across multiple byte-length expressions. LLVM origin annotations connect the extraction to the source operation.

`examples/string_byte_length.ceru` exercises Japanese text, NUL, CR/LF, composed/decomposed Unicode, returned strings, arrays, defaults, reassignment, and short circuiting. Execution is compared against known output across all routes. `tests/fixtures/observation/string-byte-length/` records a small input and all eight artifacts.
