# QBE Windows support

[日本語](qbe-windows.ja.md)

## Targets and responsibilities

| Explicit Cerune target | QBE 1.3 option | Validated link environment |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `-t amd64_sysv` | Linux x86-64, Clang / cc |
| `x86_64-pc-windows-msvc` | `-t amd64_win` | Windows x64, Clang, MSVC CRT and linker |

`emit-qbe` only generates SSA. Consumers explicitly invoke external QBE and the linker. The host OS never chooses the target. Strings, dynamic arrays, and standard I/O require a target; existing numeric-only output without one retains Linux/SysV assumptions.

CI verifies [official QBE 1.3](https://c9x.me/compile/release/qbe-1.3.html) against its SHA-256. The Windows job builds QBE itself using MinGW/UCRT and links generated programs using Clang/MSVC. QBE is not patched.

## Output and termination

Windows calls `_setmode(1, 32768)` and `_setmode(2, 32768)` before any Cerune operation. Initialization failure exits with code 1. Binary stdout/stderr preserve Japanese text, NUL, CR, and LF; numeric and Boolean output also uses LF.

Static string and diagnostic data reside in read-only `.rdata`. Language errors flush prior stdout and pass immutable diagnostic fragments to `_write`. Its Windows count argument is 32-bit. These short, fixed-format fragments fit that width, which the generated SSA explicitly selects.

After reporting a language error, the runtime calls `abort`. Immediately beforehand, `_set_abort_behavior(0, 3)` disables additional CRT messages and crash collection, producing exit code 3 under the [Microsoft contract](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/set-abort-behavior?view=msvc-170). Tests require matching reason, NodeId, SourceId, byte range, and prior output as well as code 3. Complete delivery is not guaranteed if OS writes fail.

## Argument limitation reproduced with QBE 1.3

`examples/function_arguments.ceru` reproduced invalid Windows assembly for `f32` and `f64` arguments in the fifth or later positions. QBE 1.3's `amd64/winabi.c` loads these stack arguments with an integer class, potentially feeding an integer register to a floating-point comparison.

For internal Cerune calls, these `f32` arguments travel as `w` bits and `f64` arguments as `l` bits, restored by a same-width `cast` at entry. The hidden aggregate-return pointer counts toward argument positions. QBE [cast](https://c9x.me/compile/doc/il.html#Cast-and-Copy) preserves bits without rounding or numeric conversion. Only passing already-evaluated arguments changes; evaluation order and Cerune IR types remain unchanged.

This applies only to Windows internal calls. Function parameters, calls, and casts remain directly visible in generated SSA. It does not promise general FFI compatibility with external functions.

Non-finite floating-point constants use raw bit constants, avoiding dependence on whether the CRT used to build QBE accepts spellings such as `inf` through `scanf`. Existing finite literal spelling is preserved.

## Execution comparisons and observation

The [CLI instructions](../reference/cli.en.md#qbe-target-selection) show each step from an example to SSA, assembly, and an executable. Run automated Windows comparisons with:

```powershell
$env:CERUNE_TEST_QBE = 'C:\tools\qbe.exe'
$env:CERUNE_TEST_QBE_CLANG = 'C:\Program Files\LLVM\bin\clang.exe'
cargo test --test qbe_windows -- --include-ignored
```

Both tool paths are required. Ordinary tests report the external execution test as `ignored`; the dedicated `windows-qbe` CI job requires it.

| Inputs | Checks |
| --- | --- |
| `examples/*.ceru` | Run the same completed IR through IR Executor, VM, and QBE |
| Shared value fixtures | Known output, strings, u64, aggregates, ownership, evaluation order |
| Runtime failure fixtures | Reason, origin, prior output, intended exit code |
| Small string budgets | Ownership, borrowing, copies, release, limit failures |
| Module examples | Execution after name resolution and failures originating in another file |
| CLI | Explicit targets and preservation of existing output on invalid options |

Comparisons require exact stdout/stderr bytes without newline or floating-point display normalization. `target/qbe-windows-observations/<case>/` retains `.ceir`, `.cebc`, `.ssa`, `.s`, executable, expected output, and each process's output. CI uploads the same directory as an artifact.

[Dynamic arrays](owned-arrays.en.md) also compare results, failure reasons, source origins, and prior output with IR/VM/C/LLVM. Run `cargo test --test dynamic_arrays` to check allocation, independent copies, ranges, ownership release, and Windows argument boundaries; CI sets the QBE variables above and runs these tests too.

[Line input](stdin-connections.en.md) is also implemented for both QBE 1.3 targets. Windows reads raw bytes from synchronous files/pipes with ReadFile and rejects consoles with zero-consumption unsupported-connection. Input failure emits io-v1 and stops with _Exit(1), distinct from language-error abort. Input never implicitly flushes.

CI also runs `cargo test --test stdin_routes`, comparing actual files/pipes, EINTR, partial responses, EOF, input limits, budgets, UTF-8, consumption, and origins. Generated IL, assembly, and process records remain in `target/stdin-*/` and are included in the Windows QBE job artifact.
