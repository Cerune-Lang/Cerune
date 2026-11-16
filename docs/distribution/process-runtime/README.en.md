# Subprocess runtime distribution candidates

[日本語](README.md)

This bundle contains the static library used to link generated C, LLVM, QBE and Native programs, plus the executable that supervises Linux subprocesses. Supported targets are `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`. macOS, ARM64 and musl are unsupported. The WAT reference host, compiler and user executables are not included.

## Contents and usage

The extracted directory contains these files.

| File | Purpose |
| --- | --- |
| `manifest.json` | Format version, Cerune version, source commit and dirty state, target, ABI version, Rust/Cargo versions, file lengths and SHA-256 hashes |
| `lib/process-runtime.lib` or `lib/process-runtime.a` | Explicit static library input at link time |
| `include/cerune_process_v1.h` | C ABI v1; Report contains 32 64-bit words |
| `bin/cerune-process-supervisor` | Linux only; specify its absolute path in the execution-time configuration JSON |
| `Cargo.lock`, `licenses/` | Locked dependency versions and license documents for Cerune, dependency crates and the Rust standard library |
| `README.md`, `README.en.md` | These instructions |

Pass the library explicitly through `build native --process-runtime`.

```text
cerune build native program.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang --process-runtime /opt/cerune-runtime/lib/process-runtime.a -o program-build
```

Use the Windows target and library on Windows. External tools linking generated C, LLVM or QBE need the same library and the `systemLinkArgs` from `manifest.json`. Native build also applies its existing Object target validation.

On Linux, set `linux_supervisor` in the execution-time `cerune-process-v1` configuration JSON to the extracted `bin/cerune-process-supervisor` absolute path. No automatic search occurs. Its `--version` prints the Cerune and control protocol versions. Other invocations require the control file descriptor passed by the runtime.

```text
program --cerune-runtime-v1 --process-config process.json -- program-arguments
```

Registered commands, environment values and configuration JSON are not bundled. The static runtime is linked into the program, while the Linux supervisor remains a separate file. This does not change `release native` to include the supervisor automatically.

## Building and checking

Run from the repository using Node.js 24, Rust/Cargo, Git and tar. Install `rustup component add rust-docs` to include the license documents. Builds reject a target that differs from the current x86_64 host OS and require a new output directory.

```text
node scripts/process-runtime-bundle.cjs build x86_64-unknown-linux-gnu target/process-runtime-linux
node scripts/process-runtime-bundle.cjs verify x86_64-unknown-linux-gnu target/process-runtime-linux/bundle
```

`build` uses `--locked --release` and produces `bundle/`, `cerune-process-runtime-<version>-<target>.tar.gz`, its `.sha256`, and `logs/commands.json`. The logs retain commands, status, stdout and stderr, but are not archived. Development builds with uncommitted changes record `source.dirty: true`. Check the commit and hashes even when version numbers match.

`verify` does not execute payloads. It checks the requested target, ABI, fixed link dependencies, required files, lengths, hashes and Linux executable permissions. Missing, modified, duplicate or unexpected files and links are rejected. Hashes detect corruption; they do not authenticate the publisher or prove ABI correctness.

Use this execution test for a locally built archive. It extracts before verification, so do not provide archives from untrusted sources.

```text
node scripts/process-runtime-smoke.cjs x86_64-unknown-linux-gnu /usr/bin/clang target/process-runtime-linux/cerune-process-runtime-0.1.0-x86_64-unknown-linux-gnu.tar.gz /tmp/cerune-runtime-smoke
```

Windows/Linux CI extracts outside the checkout and links the C ABI probe using only the bundled header and library. It runs with an empty PATH and checks arbitrary bytes, origins, nonzero exits, failures, timeout and cleanup. It also builds/releases the existing `registered.ceru` example with the extracted runtime and verifies relocated execution and dependency records.

## Remaining publication work

These archives are distribution candidates also built in CI. Automatic GitHub Release attachment, a clean-commit publication check and compiler archive tests (Issue #272) remain separate work. This change does not create a tag or publish a release.

Windows requires system DLLs; Linux requires system libraries including glibc. `systemLinkArgs` and the build OS/glibc are recorded, but compatibility with every OS version or older glibc is not guaranteed. Official distribution must establish and test minimum supported OS versions. The license inventory covers Cargo-resolved dependencies; it does not claim every package is linked into the final binary.
