# Subprocess runtime distribution

[日本語](README.md)

The bundle provides the static library for generated C, LLVM, QBE and Native programs, plus the Linux subprocess supervisor. Targets are x86_64 Windows MSVC and Linux GNU. Support for macOS, ARM64 and musl remains future work. The compiler, WAT reference host and user executables follow their own distribution procedures.

## Files and usage

```text
Static runtime → link → user executable
                              ↓ execution-time configuration JSON
                   registered commands, environment, limits
                              ↓ on Linux
                   cerune-process-supervisor
```

The extracted directory contains these files.

| File | Purpose |
| --- | --- |
| `manifest.json` | Format and Cerune versions, source commit and dirty state, target, ABI, Rust/Cargo versions, build OS, file lengths and SHA-256 hashes |
| `lib/process-runtime.lib` or `lib/process-runtime.a` | Static runtime specified at link time |
| `include/cerune_process_v1.h` | C ABI v1; Report contains 32 64-bit words |
| `bin/cerune-process-supervisor` | Linux only; specify its absolute path in the configuration JSON |
| `Cargo.lock`, `licenses/` | Locked dependency versions and license documents for Cerune, dependency crates and the Rust standard library |
| `README.md`, `README.en.md` | These instructions |

Pass the library explicitly through `build native --process-runtime`.

```text
cerune build native program.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang --process-runtime /opt/cerune-runtime/lib/process-runtime.a -o program-build
```

Use the Windows target and library on Windows. External tools linking generated C, LLVM or QBE need the same library and the `systemLinkArgs` from `manifest.json`. Native build also checks the Object target.

On Linux, set `linux_supervisor` in the execution-time `cerune-process-v1` configuration JSON to the extracted supervisor's absolute path. Its `--version` prints the Cerune and control protocol versions. Normal supervisor startup requires the control file descriptor passed by the runtime.

```text
program --cerune-runtime-v1 --process-config process.json -- program-arguments
```

Users supply registered commands, environment values and configuration JSON for their execution environment. The static runtime is linked into the program. The Linux supervisor uses the file specified at execution time, so users of `release native` must also arrange its installation and configuration.

## Building and checking

Use Node.js 24, Rust/Cargo, Git and tar. Install `rustup component add rust-docs` for license documents. Build on the matching x86_64 host OS and specify a new output directory.

```text
node scripts/process-runtime-bundle.cjs build x86_64-unknown-linux-gnu target/process-runtime-linux
node scripts/process-runtime-bundle.cjs verify x86_64-unknown-linux-gnu target/process-runtime-linux/bundle
```

| Operation | Output or checks |
| --- | --- |
| `build` | Builds with `--locked --release`, producing `bundle/`, a versioned target `.tar.gz`, and its `.sha256` |
| Build records | `logs/commands.json` and logs retain commands, status, stdout and stderr; CI stores them as observation artifacts |
| `verify` | Reads files and checks target, ABI, fixed link dependencies, required files, lengths, hashes and Linux executable permissions |
| Mismatches | Missing, modified, duplicate or unexpected files and links produce errors |

Development builds record uncommitted changes as `source.dirty: true`. Check the commit and hashes even when version numbers match. Hashes detect corruption. Use a trusted Release to establish provenance and execution tests to check ABI behavior. The license inventory covers Cargo-resolved dependencies, which can exceed the crates linked into the final binary.

Run the following execution test on an archive you built. Extraction precedes content verification, so supply a trusted archive.

```text
node scripts/process-runtime-smoke.cjs x86_64-unknown-linux-gnu /usr/bin/clang target/process-runtime-linux/cerune-process-runtime-0.1.0-x86_64-unknown-linux-gnu.tar.gz /tmp/cerune-runtime-smoke
```

The test extracts outside the checkout and links the C ABI probe using only the bundled header and library. It runs with an empty PATH and checks arbitrary bytes, origins, nonzero exits, failures, timeout and cleanup. The Release workflow also builds/releases the existing `registered.ceru` example with the extracted runtime, checking relocated execution and dependency records.

## Preparing Release assets

```text
Source from the same commit
  ├─ Compiler, five targets → extraction and execution tests
  └─ Runtime, two targets → extraction and C ABI tests → example build/release
                              ↓
                  check version, commit, build OS and SHA-256
                              ↓
                  extract both targets again and compare manifests
                              ↓
                  write cerune-process-runtime.json
                              ↓
                  all tests pass → attach on a release tag run
```

| Run | Output |
| --- | --- |
| PR | Archives and validation records saved as Actions artifacts |
| Release tag | Tested files attached to GitHub Release |

`process-runtime-release.cjs stage` checks the compiler version and commit, clean source, and the build OS listed below. `catalog` accepts exactly six files: an archive, checksum and metadata JSON for each target. It extracts the archives and compares their manifests with the metadata. Extra logs or a missing target produce an error.

| Target | Build and execution test runner | Recorded system runtime |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 22.04 | glibc 2.35 |
| `x86_64-pc-windows-msvc` | Windows Server 2022 | Windows 10.0.20348 series |

Release assets include `cerune-process-runtime-<version>-<target>.tar.gz`, `.tar.gz.sha256`, and `.json` for each target, plus the catalog `cerune-process-runtime.json`. Each metadata JSON records the manifest and test runner. The catalog records version, commit, hashes, ABI, link arguments and the build system runtime. Build and execution logs remain available in the Actions observation artifacts.

Execution requires Windows system DLLs or Linux system libraries including glibc. The table identifies the OS used to build and test the assets. Windows desktop support and compatibility with older OS versions will be established through execution tests on those systems. Selecting a release version and creating its tag are release operations performed after implementing this workflow.
