# Subprocess connections in generated C

[日本語](subprocess-c.ja.md)

**Implemented and verified on Windows x86-64 MSVC and Linux x86-64 GNU.** Common-IR process.run connects to the [existing runtime C ABI](../../experiments/subprocess/c_abi/README.en.md). LLVM/QBE/Native/WAT operations and runtime/supervisor distribution integration follow later.

## Emit, link, and start

emit-c requires an explicit target when subprocess operations are present. The emitter never chooses from its host OS, and generated C checks the compilation target. Existing C generation without process operations gains no link dependency. The Rust API is emit_c_with_target(program, target, call_limit). Existing APIs without targets reject subprocess operations with source diagnostics.

On Linux, build as follows. process.json uses the [existing explicit configuration](subprocess-cli.en.md); its linux_supervisor field names the built supervisor by absolute path.

~~~sh
cargo build --example subprocess_linux_supervisor
cargo rustc --lib --features process-c-abi --target x86_64-unknown-linux-gnu --crate-type staticlib -- --print native-static-libs
cargo run --quiet -- emit-c examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.c
clang --target=x86_64-unknown-linux-gnu -std=c11 target/process.c target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

On Windows, build the same source with an MSVC/Windows SDK development environment.

~~~powershell
cargo rustc --lib --features process-c-abi --target x86_64-pc-windows-msvc --crate-type staticlib -- --print native-static-libs
cargo run --quiet -- emit-c examples/process_execution/registered.ceru --target x86_64-pc-windows-msvc -o target/process.c
clang --target=x86_64-pc-windows-msvc -std=c11 -D_CRT_SECURE_NO_WARNINGS target/process.c target/x86_64-pc-windows-msvc/debug/cerune_lang.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -ladvapi32 -lbcrypt -o target/process.exe
target/process.exe --cerune-runtime-v1 --process-config process.json --
~~~

Users link explicitly. Check native-static-libs output when changing Rust versions or targets. ABI declarations are embedded in generated C, so no external header search is needed. The archive is currently a prototype distribution form built from the compiler crate, not a completed Native build/release artifact.

Only a leading runtime marker introduces configuration. Everything after -- remains program arguments. Without the marker, preserve all arguments and leave the process connection unconfigured. Programs also using file I/O can combine --file-root. Duplicate, missing, unknown, unreadable, or invalid settings stop before source execution. JSON is capped at 1 MiB and uses the same parser as the CLI. An empty file is invalid configuration, distinct from an absent connection setting.

## Visible generated steps

| Order | Operation and responsibility |
| --- | --- |
| 1 | Retain ProcessRun in C IR. Evaluate the request once; preserve short-circuit and left-to-right evaluation through existing sequencing |
| 2 | Convert request strings/arrays into ABI inputs. Explicitly pack u8 values stored as int64_t into raw bytes |
| 3 | Prepare fixed Outcome and empty arrays; shared prepare validates and allocates transfer storage |
| 4 | When Pending, reserve both result arrays. Roll back added storage on capacity/allocation failure and pass failure to execute; no child launches |
| 5 | Only successful reservation advances clock, launch, transfers, and cleanup. Read Report and retained output at explicit positions |
| 6 | Convert u64 words into public types. Resolve tags by IR variant names; initialize inactive payloads to zero |
| 7 | Release results while preserving disabled connections and unreclaimed OS resources. Common-IR ownership handles returned arrays |

Charge both output limits times eight bytes against the existing array limit. Reserved capacity remains charged until array destruction, even for short or empty results. Distinguish semantic limits, physical size representation, and malloc failure. Ordinary array-allocation failure still stops execution. Check both retained lengths before finalizing initialized counts and lengths without further allocation.

Report preserves the common operation's NodeId/SourceId/Span. Resource failures outside the ABI use existing runtime diagnostics; ABI status failures stop with cerune-process-abi-v1 and the operation origin. Launch/transfer/cleanup failures remain Outcomes. Observational report/copy operations advance neither execution nor release.

## Verification and remaining work

The [comparison script](../../experiments/subprocess/c_abi/generated.cjs) runs [existing examples](../../examples/process_execution/README.en.md) through generated C and HIR/MIR/SSA/VM. Each OS checks 64 comparisons plus four injected allocation failures, invalid startup settings, and wrong-target compilation rejection. It also checks zero live bytes after all arrays are released.

Successful results, origins, unconfigured/validation/reservation failures, copies, and evaluation order are compared. For output limits/timeouts, separate OS runs can receive different exit notifications or partial output on other channels. Original records remain unchanged; output-limit cases check confirmed stdout and failure, while timeout cases check Timeout and caller continuation. Shared-runtime scripted tests retain exact notification-sequence comparisons.

ABI compatibility guarantees, additional-runtime manifests, and supervisor layout/bundling remain distribution work. Generated C currently requires an explicitly linked matching archive.
