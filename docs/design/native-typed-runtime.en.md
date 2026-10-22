# Migrating Native runtime to typed construction

[日本語](native-typed-runtime.ja.md)

The next stage of the [SASHIMI connection](native-encoder.en.md). Cerune retains instruction selection, ABI and runtime semantics while constructing machine instructions and labels as SASHIMI values.

## Implementation status

| Part | Current state |
| --- | --- |
| UTF-8 validation | `utf8::append` adds directly to Program; shared by stdin and program arguments |
| String equality | `string::append` adds directly to Program; Cerune explicitly selects Windows/Linux argument registers |
| String heap / array owners | `heap::append` / `array::append` add allocation, reference counting, release and writable bookkeeping directly to Program |
| Normal output | `output::append` directly builds numeric/boolean/raw/quoted formatting, writes, retries and failure diagnostics in Program |
| Read-only constants | `data::append` adds sign masks, f32/f64 bits and length-prefixed strings directly to Program |
| Standalone helper/constant ASM/Object | Generate both from the same Program; object generation does not parse text |
| Whole Cerune programs | Migration in progress. Helpers and constants are rendered into existing assembly output, so final object generation still reparses assembly |
| Remaining migration | Startup, stdin/argument helpers and bookkeeping, LIR instruction selection (including array/output callers and flush) and source provenance |

This does not complete removal of assembly reparsing. The temporary `machine::render_support` adapter formats validated assembly for the existing output without parsing instructions or operands. SASHIMI location comments and the final stack note are left to the enclosing emitter. The legacy Object reader derives PLT32 from external call symbol names, so this adapter omits `.extern` and `@PLT`; direct Program assembly retains both. Remove this adapter when the entire output shares one Program.

## Preserved contracts

- UTF-8 validation does not mutate input bytes. It accepts valid sequences including empty data, NUL and CR/LF, and rejects overlong encodings, surrogates, out-of-range code points and incomplete sequences.
- String equality compares lengths and raw bytes, without NUL termination, Unicode normalization or newline conversion.
- ABI and string layout are unchanged. The internal UTF-8 convention is rdx=pointer, rcx=length, eax=result. String equality uses target argument registers and an eight-byte length followed by contents.
- Runtime instructions, labels and constant pools carry Synthetic provenance. No NodeId/Span is invented. Existing call-site origin labels and failure diagnostics remain.
- Preserve instruction/label order, branches and alignment; introduce no optimization. Mappings serve observation, not execution control.

String heap and array owners preserve capacity accounting, reference counts, final release, error values and Windows/Linux trap behavior. `malloc` / `free` / `memcpy` share explicit external declarations. Bookkeeping data and helper instructions remain Synthetic; public I/O contracts and limits do not change.

Normal output retains requested/accepted byte counts and call-site origins. Short writes continue, EINTR retries, and zero progress, invalid accepted counts or write failures emit io-v1 diagnostics and stop. Linux SIGPIPE handling, numeric formats and escape ordering are preserved. Formatting/diagnostic C strings are built as `Data` including the terminating NUL and rendered as `.byte` in ASM; user strings retain their existing length-prefixed representation.

## Constant representation and provenance

SASHIMI `Data32` / `Data64` preserve four/eight little-endian bytes and render as hexadecimal `.long` / `.quad` so widths remain visible. No floating-point recalculation occurs; only explicit `Align` entries add alignment. Strings retain an eight-byte byte length and contents, with no added NUL terminator.

Current LIR constant pools have no per-constant source position, so they remain Synthetic. Instructions using constants retain their existing NodeId/Span origins. Connecting whole-program provenance to Program positions remains a later step.

## Validation and existing examples

The [pre-migration assembly](../../tests/support/native_runtime_reference/README.en.md) is a fixed test-only reference. Compare typed Program objects with the old encoder on both targets. The [old data emitter](../../tests/support/native_data_reference.rs) is frozen from 0eca28a for tests. The 135-example connection comparison restores migrated helpers, constants and allocation bookkeeping from these old outputs and compares complete objects. Current-output ASM observation fixtures are updated; these frozen migration references are not.

Independent execution tests link ASM/Object from the same Program with clang and compare both on Windows/Linux. Linux explicitly enables PIE.

| Check | Expectation |
| --- | --- |
| Every one-byte and two-byte sequence; three/four-byte boundaries, incomplete sequences and Japanese | Rust UTF-8 validation |
| Empty strings, Japanese, embedded NUL, CR/LF, combining and precomposed characters | Raw byte equality |
| f32/f64 ±0, subnormals, ±infinity, NaN payloads and sign masks | Original bits, width, alignment and identical old objects |
| Length-prefixed strings and empty contents | Eight-byte byte length plus raw contents, eight-byte alignment and zero-byte empty-content range |
| String/array reference counts and capacity | Capacity stays occupied while retained and recovers after final release; distinguish empty values, zero-width elements, invalid sizes and limit failures |
| Normal output success/failure | Compare bytes, io-v1 diagnostics and stopping across ASM/Object and four executors: scalar formats, Japanese/NUL/CRLF, aggregates, short writes, EINTR, zero progress, invalid counts and closed stdout |
| Helper/constant provenance | Synthetic, nonempty instruction ranges, empty label ranges, no range for section selection |
| Repeated rendering/generation | Unchanged input Program and Object |

Normal output uses [io_protocol/output_values](../../examples/io_protocol/output_values.ceru). Other relevant runnable examples are [string_values](../../examples/string_values.ceru), [io_protocol/input_flow](../../examples/io_protocol/input_flow.ceru) and [arguments/list](../../examples/arguments/list.ceru). Native argument tests compare examples through MIR/SSA, ASM/Object and four executors, including I/O ordering, copy independence and failures.

```sh
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::machine::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::data::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::heap::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::output
CERUNE_TEST_ASM_CLANG=clang cargo test --test native_output
cargo test --lib codegen::x86_64::object::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --test standard_input program_arguments::native
```

In PowerShell first set `$env:CERUNE_TEST_ASM_CLANG = 'clang'`. Without it, standalone execution comparisons report they were not run. Windows/Linux core CI sets this variable and treats tool failures as test failures.

## Next steps

1. Migrate stdin and argument runtime helpers and their bookkeeping to the same Program.
2. Connect LIR instruction selection, ABI startup/exit, NodeId/SourceId/Span and MIR provenance.
3. Generate whole-program ASM/Object from the same Program, comparing existing diagnostics, output ordering and execution.
4. Remove generated-assembly reparsing and the transitional adapter, then decide when to retire migration references.

Completion means whole-program ASM and Object share one typed Program, object generation no longer reparses generated assembly, and source origins, failure diagnostics, I/O order and existing example behavior are preserved. Return to language additions such as integer input after verifying these conditions. Porting SASHIMI to Cerune is separate future work.
