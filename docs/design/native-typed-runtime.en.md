# Migrating Native runtime to typed construction

[日本語](native-typed-runtime.ja.md)

The next stage of the [SASHIMI connection](native-encoder.en.md). Cerune retains instruction selection, ABI and runtime semantics while constructing machine instructions and labels as SASHIMI values.

## Implementation status

| Part | Current state |
| --- | --- |
| UTF-8 validation | `utf8::append` adds directly to Program; shared by stdin and program arguments |
| String equality | `string::append` adds directly to Program; Cerune explicitly selects Windows/Linux argument registers |
| Standalone helper ASM/Object | Generate both from the same Program; object generation does not parse text |
| Whole Cerune programs | Migration in progress. Helpers are rendered into existing assembly output, so final object generation still reparses assembly |
| Remaining migration | Constants/data, startup, I/O/heap/array helpers, LIR instruction selection and source provenance |

This does not complete removal of assembly reparsing. The temporary `machine::render_support` adapter formats validated assembly for the existing output without parsing instructions or operands. SASHIMI location comments and the final stack note are left to the enclosing emitter. Remove this adapter when the entire output shares one Program.

## Preserved contracts

- UTF-8 validation does not mutate input bytes. It accepts valid sequences including empty data, NUL and CR/LF, and rejects overlong encodings, surrogates, out-of-range code points and incomplete sequences.
- String equality compares lengths and raw bytes, without NUL termination, Unicode normalization or newline conversion.
- ABI and string layout are unchanged. The internal UTF-8 convention is rdx=pointer, rcx=length, eax=result. String equality uses target argument registers and an eight-byte length followed by contents.
- Runtime instructions and labels carry Synthetic provenance. No NodeId/Span is invented. Existing call-site origin labels and failure diagnostics remain.
- Preserve instruction/label order, branches and alignment; introduce no optimization. Mappings serve observation, not execution control.

## Validation and existing examples

The [pre-migration assembly](../../tests/support/native_runtime_reference/README.en.md) is a fixed test-only reference. Compare typed Program objects with the old encoder on both targets. The 135-example connection comparison also restores the migrated helpers from fixed assembly and compares complete objects.

Independent execution tests link ASM/Object from the same Program with clang and compare both on Windows/Linux. Linux explicitly enables PIE.

| Check | Expectation |
| --- | --- |
| Every one-byte and two-byte sequence; three/four-byte boundaries, incomplete sequences and Japanese | Rust UTF-8 validation |
| Empty strings, Japanese, embedded NUL, CR/LF, combining and precomposed characters | Raw byte equality |
| Helper provenance | Synthetic, nonempty instruction ranges, empty label ranges, no range for section selection |
| Repeated rendering/generation | Unchanged input Program and Object |

Relevant runnable examples are [string_values](../../examples/string_values.ceru), [io_protocol/input_flow](../../examples/io_protocol/input_flow.ceru) and [arguments/list](../../examples/arguments/list.ceru). Native argument tests compare examples through MIR/SSA, ASM/Object and four executors, including I/O ordering, copy independence and failures.

```sh
CERUNE_TEST_ASM_CLANG=clang cargo test --lib codegen::x86_64::machine::tests
cargo test --lib codegen::x86_64::object::tests
CERUNE_TEST_ASM_CLANG=clang cargo test --test standard_input program_arguments::native
```

In PowerShell first set `$env:CERUNE_TEST_ASM_CLANG = 'clang'`. Without it, standalone execution comparisons report they were not run. Windows/Linux core CI sets this variable and treats tool failures as test failures.

## Next steps

1. Migrate constants/data and remaining runtime helpers so they can append to the same Program.
2. Connect LIR instruction selection, ABI startup/exit, NodeId/SourceId/Span and MIR provenance.
3. Generate whole-program ASM/Object from the same Program, comparing existing diagnostics, output ordering and execution.
4. Remove generated-assembly reparsing and the transitional adapter, then decide when to retire migration references.

Return to language additions such as integer input after this migration is established.
