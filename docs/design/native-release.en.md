# Initial Native release

[日本語](native-release.ja.md) · [Issue #81](https://github.com/Cerune-Lang/Cerune/issues/81)

## Operation and scope

`cerune release native <build-directory> -o <new-directory>` packages a successful [Native build](native-build.en.md) as a distribution directory. It does not regenerate source or invoke Clang, the executable, or external inspection tools. Build and release are separate operations.

| Item | Initial contract |
| --- | --- |
| Input | Build directory with schema `cerune-native-build-v1` and status `success` |
| Output | New directory with an existing parent. Reject existing destinations and output inside the input |
| Target, limits, SSA | Preserve build choices; release has no options to change them |
| Runtime | Require the target system runtime; no libraries, SDKs or compilers bundled |
| Source contents | Preserve the build-time `--include-sources` choice; do not add or remove contents |
| Optimization | No compilation, relinking, stripping or optimization |
| Out of scope | IR/VM Images, compressed archives, installers, signatures, automatic publication, library bundling/discovery |

The distribution format is a directory. External archive tools may package it, preserving contents and Linux execute permissions after extraction. This command is separate from GitHub releases of the Cerune compiler.

## Contents

| Path | Contents |
| --- | --- |
| `build/` | Every prescribed build file, byte-for-byte: executable, baseline HIR/MIR, selected SSA/LIR, typed instructions/Object, origins and link records |
| `release.json` | Schema `cerune-native-release-v1`, status, target, relative entry path, source inclusion, per-file sizes and SHA-256; written last |
| `runtime-requirements.json` | Direct runtime requirements inspected from the executable without execution |
| `SHA256SUMS` | SHA-256 of all payload files and release.json, excluding itself |
| `README.md`, `README.en.md` | Target, invocation, dependency conditions, observation records and checksum meaning |
| `CERUNE-LICENSE-MIT`, `CERUNE-LICENSE-APACHE` | Cerune license texts; this does not assign a license to the user's program |

The entry is `build/program` on Linux or `build/program.exe` on Windows. Absolute build paths and linker choices remain historical records, not instructions to relink at the new location. Omitting source contents does not remove literals, filenames or toolchain paths.

## Runtime inspection limits

| Target | Metadata inspected | Explicitly rejected |
| --- | --- | --- |
| Linux x86-64 | ELF64 PIE, entry, interpreter, DT_NEEDED and GNU symbol-version requirements (library, version, flags) | Wrong architecture, non-PIE, missing entry, missing/incomplete version requirements, RPATH/RUNPATH |
| Windows x64 | PE32+, entry, direct import DLLs and names with hints / ordinals, OS/subsystem header versions | Wrong architecture, DLL, missing entry, delay imports |

Goblin reads ELF/PE ([ELF API](https://docs.rs/goblin/0.10.7/goblin/elf/struct.Elf.html), [PE API](https://docs.rs/goblin/0.10.7/goblin/pe/struct.PE.html)). Never infer the target from the host; artifacts for another OS are inspected as bytes. The parser's internal dependency on 32-bit ELF types does not add Cerune 32-bit support.

The list does not check installed libraries. Transitive dependencies, dynamic loading and target-environment search results are not resolved. GNU version requirements and PE header versions are not certified minimum OS versions. Records contain `deployment_verified: false` and `transitive_dependencies_resolved: false`. Deployment and execution checks are separate work for the distributor.

## Input checks and failures

Check build schema, successful status, target, selected options, prescribed inventory, link results/exit codes/target and runtime policy. Reject unknown artifact names, missing/extra files, symlinks/reparse points, malformed JSON and invalid executable formats. Manifest paths are not arbitrary copy instructions.

Input limits are 128 MiB per file, 512 MiB total and 16 MiB per JSON record. Snapshot and validate inputs before creating output. Invalid input creates no output directory; later I/O failures may leave partial output. Packaging is complete only with a valid successful release.json, every file and matching hashes. Retry into a new directory.

Do not concurrently modify inputs. This does not isolate hostile concurrent writers. Hashes describe distribution-time contents, not signatures or proof of the original build's authenticity or semantic correspondence. No loadable IR format or general arbitrary-executable verifier is introduced.

## Verification and next stage

On Windows/Linux, build, package, relocate and execute the example with known results already compared with IR/MIR/VM. Cover baseline/SSA, included/omitted sources, byte-for-byte preservation, hashes, direct dependencies, inconsistent/missing records and existing-output preservation. Commands are in the [example](../../examples/build_native/README.en.md).

This change completes the initial distribution mode requiring a target system runtime. IR/VM Images, broader portability and library bundling remain separate work. The next language feature in the [roadmap](language-roadmap.en.md) is recursion.
Release preserves build call_depth_limit unchanged. Missing legacy records are not filled with a default; release.json records null (unknown). Present values must be integers in 1–128.
