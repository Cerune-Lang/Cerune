# Initial Native build

[日本語](native-build.ja.md) · [Issue #81](https://github.com/Cerune-Lang/Cerune/issues/81)

## Completed artifact and scope

`build native` links Cerune’s own Object through the Clang driver and saves an executable plus generation/link records in a new directory. It does not execute the program. `emit-obj` continues to emit just an Object. IR/VM Images, release and a custom linker are outside this change.

```sh
cerune build native app.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o app-build
```

| Selection | Contract |
| --- | --- |
| `native`, `--target`, `--linker`, `-o` | Required. No host-inferred target or PATH search for Clang. Linker is an absolute/relative path to an actual file |
| Target | `x86_64-unknown-linux-gnu` → `program` (ELF64 PIE); `x86_64-pc-windows-msvc` → `program.exe` (PE32+) |
| `--ssa` | Explicit SSA transformation; retain baseline MIR. No implicit optimization passes |
| `--include-sources` | Add exact source contents in `sources.json`. File names and literals in generated code are recorded even without this option |
| `--link-timeout-seconds` | Limit for each of version query and link; default 60 seconds, range 1–3600 |
| Heap/argument limits | The same four options as emit-obj, fixed in generated code and manifest |

Create the parent directory beforehand. Reject existing output files, directories and links without overwriting. Arbitrary linker arguments, sysroots, extra Objects and library inputs are not provided initially. Select a Clang environment with the target SDK, CRT and linker available.

## Responsibilities and observation

`native_build::generate` constructs common frontend→HIR→MIR→LIR→typed NativeProgram→Object and observations, without launching tools. ASM and Object come from the same NativeProgram. `native_build::link::Request` holds the Object input, target, driver, arguments, working directory and timeout; it invokes Clang and checks the result. Future adapters or a custom linker need not replace Object generation. No general plugin ABI is introduced.

| Files | Contents |
| --- | --- |
| `manifest.json` | `cerune-native-build-v1`, building/success/failed, Cerune version, target, limits, SSA choice and artifact list. Success is written only after final checks |
| `program.ceir`, `program.mir.txt` | HIR and baseline non-SSA MIR |
| `program.ssa*` (when selected) | SSA, its origin map, lowered MIR and map |
| `program.lir.txt`, `program.machine.txt` | LIR and typed instructions actually used for Object generation |
| `program.origins.s`, `program.o` | Annotated ASM and Object from the same instructions; no external assembler |
| `program.origins.json`, `program.object-map.txt` | OriginId→NodeId/SourceId/Span, MIR/LIR; instruction, symbol and relocation positions within the Object |
| `source-index.json`, optional `sources.json` | File IDs, names, user/standard-module origins and UTF-8 sizes; exact contents when requested |
| `runtime.json` | External symbols and target runtime conditions; Cerune runtime is generated inside the Object |
| `link-plan.json` | Resolved driver path, explicit target, input, argument array, cwd, stdin and environment policy |
| `version.*`, `link.*` | Raw stdout/stderr and exit code/termination reason/duration. Clang `-v` also records downstream linker selection and arguments |
| `program.link.map`, `link-result.json` | Downstream linker map and overall link result |

Object positions are section offsets, not linked addresses. Retain Object origin symbols and the downstream linker map, without claiming a unified linked-address lookup API or debugger information. Machine/Object-map text is for observation, not stable re-input.

## Linking and execution requirements

Linux explicitly selects the target, `-pie`, `--no-undefined`, map and `-lm`. Windows selects the MSVC target and map. Query the version and verify a Clang driver. Invoke an argument array without constructing a shell command. Inherit the toolchain environment; Clang selects SDKs, downstream linkers and libraries and records them in verbose output. These logs do not capture the complete environment or guarantee reproducible builds or portability.

Exit 0 is insufficient: require an x86-64 executable header for the chosen target and a nonempty map. This is a minimal format check, not proof of arbitrary executable safety or semantics. Successful building does not imply executability on the current host.

Windows requires the selected MSVC/UCRT and Win32 environment; Linux requires the selected glibc, libm and ELF loader. No OS libraries or redistributable runtimes are bundled, and no minimum OS/glibc version is promised initially. [Initial Native release](native-release.en.md) separately records direct runtime requirements and distribution contents.

## Failures and verification

Option, compilation and Object-generation failures launch no tools and create no output directory. Later filesystem errors may leave an incomplete directory. Distinguish driver startup, version mismatch, nonzero link exit, timeout and invalid artifacts; link failures retain Object, logs and a failed manifest. Interruption or record-writing failure can leave building status. Anything other than success is incomplete; retries use a new output directory.

Tool stdin is null. On timeout terminate the dedicated process group (Unix) or driver descendants (Windows) and reap the driver. This is not a toolchain sandbox; the explicitly selected local tool is trusted.

On Windows/Linux compare baseline/SSA build→execution, exact existing emit output, arguments, limits, failure reasons and origins. Test failing/timed-out drivers, successful exit without output, syntax/type errors and existing-output preservation. See the [runnable example](../../examples/build_native/README.en.md).
