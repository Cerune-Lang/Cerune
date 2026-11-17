# Preparing the next release

[日本語](release-readiness.ja.md)

As of 2026-11-17, the source version for the distribution candidate is **v0.2.0**. The published version remains **v0.1.0**. The 0.1.0 archives below are historical records validating the distribution procedure. New 0.2.0 archives will be built and tested, with the included commit and publication date decided during final review.

See the [v0.2.0 release notes draft](v0.2.0-notes.en.md) for included features and upgrade instructions.

## Completed distribution tests

```text
Choose the commit to include
  → update the version
  → build and test five compiler targets and two runtime targets in a PR
  → review changes, compatibility and tested OSes
  → push the tag
  → rebuild and retest from the tagged source
  → publish to GitHub Release
```

The PR build and execution steps have been verified.

| Item | Verified behavior | Evidence |
| --- | --- | --- |
| Five compiler targets | Extract on matching OS/CPU; help and print(7) succeed with an empty PATH | [#299](https://github.com/Cerune-Lang/Cerune/pull/299), [#300 Release tests](https://github.com/Cerune-Lang/Cerune/actions/runs/44068267393) |
| Two runtime targets | Extracted C ABI tests; child I/O, exit, failure and cleanup; Native build/release and relocated execution | [#300](https://github.com/Cerune-Lang/Cerune/pull/300) |
| Files prepared for Release | Check version, commit, build OS, SHA-256 and extracted manifests; create a two-target catalog | The same #300 run |
| Desktop Windows | Use actual Server 2022 archives on Windows 11 Home 10.0.26100 | Additional tests below |

See the [compiler](compiler/README.en.md) and [runtime](process-runtime/README.en.md) instructions for build steps and file usage.

## Tested operating systems

| Asset | Build and execution tests | Additional execution test |
| --- | --- | --- |
| Compiler Windows x64 | Windows Server 2022 | Windows 11 Home 10.0.26100 |
| Compiler Linux x64 / ARM64 | Ubuntu 22.04 / 22.04 ARM64 | — |
| Compiler macOS Intel / ARM64 | macOS 15 Intel / ARM64 | — |
| Runtime Windows x64 MSVC | Windows Server 2022 (10.0.20348 series) | Windows 11 Home 10.0.26100 |
| Runtime Linux x64 GNU | Ubuntu 22.04 (glibc 2.35) | — |

This table records actual test environments. Add execution tests on Windows 10 or older macOS/glibc versions before announcing support for those environments. Runtime support for macOS, ARM64 and musl remains future work. Linking generated programs requires the specified toolchain, such as Clang; execution requires the target OS system libraries.

## Additional Windows 11 tests

The Windows compiler, runtime and plan artifacts were downloaded from [Release run 44068267393](https://github.com/Cerune-Lang/Cerune/actions/runs/44068267393). The PR head is `bf74aa3a7909eae28951d0e04403302ff3ede87a`. The runtime manifest records the merge commit tested by Actions, `b09797a839f30af370c395ad8b2435e3bdb56d7b`, with `dirty: false`.

| File | SHA-256 |
| --- | --- |
| `cerune-lang-x86_64-pc-windows-msvc.zip` | `7717bda11fc4d2f643c53e82b330725719277f3e37c00960fda9d4fcc48281ef` |
| `cerune-process-runtime-0.1.0-x86_64-pc-windows-msvc.tar.gz` | `7998e3fca3626cc9f2b472984ce54557134ef15147470991dfe71c8dd254d4e7` |

The existing `compiler-archive-smoke.cjs` and `process-runtime-smoke.cjs` scripts extracted outside the checkout. Compiler startup and a short program passed, as did the C ABI probe using the bundled header and library.

The local Cerune after merging #300 was also tested with the downloaded library specified through `CERUNE_TEST_PROCESS_RUNTIME`:

```text
cargo test --locked --test native_program process_runtime_build_release -- --include-ignored
```

The test passed for baseline and SSA Native build/release of `registered.ceru`, dependency content records and relocated execution. Repeat these tests with the new archives when changing the published compiler/runtime version. Actions artifacts expire; this record retains the run number, commit and hashes.

## Remaining publication steps

| Order | Work | Completion condition |
| --- | --- | --- |
| 1 | Merge included PRs | Normal CI, CodeQL and Release tests pass for the selected commit |
| 2 | Update the version | Main package, experimental lockfiles and reference docs updated to 0.2.0; published 0.1.0 instructions retained |
| 3 | Write Japanese and English release notes | Explain changes since v0.1.0, compatibility changes such as import names, tested OSes, required tools and unsupported features |
| 4 | Test the updated version | Five compiler targets, two runtime targets and the catalog agree on version/commit and pass execution tests |
| 5 | Review publication and create the tag | Confirm the commit/version, then rebuild, retest and publish from the tag |

IR/VM Images, optimization, GPU, GUI and other work continue on the [roadmap](../design/language-roadmap.en.md). Making all execution-route CI results required remains [#262](https://github.com/Cerune-Lang/Cerune/issues/262). Before publication, inspect every CI result on the selected commit regardless of which checks are marked required.
