# Build a Native executable

[日本語](README.md)

[main.ceru](main.ceru) copies a dynamic u8 array, reads an integer argument and prints twice its value. Build does not execute it. Run from the repository root.

```powershell
cargo run --quiet -- build native examples/build_native/main.ceru --target x86_64-pc-windows-msvc --linker "C:\Program Files\LLVM\bin\clang.exe" -o target/native-demo
.\target\native-demo\program.exe 3
```

```sh
cargo run --quiet -- build native examples/build_native/main.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o target/native-demo
./target/native-demo/program 3
```

Both targets produce:

```text
[9, 4]
[2, 4]
6
```

Use the actual Clang path on your machine, with the target linker/SDK/runtime installed. Choose a new output directory for each build; existing output is refused. Without arguments the program asks for an integer; `abc` prints `invalid integer`.

| Comparison / observation | Command or file |
| --- | --- |
| IR | `cargo run --quiet -- run examples/build_native/main.ceru -- 3` |
| VM | `cargo run --quiet -- run-vm examples/build_native/main.ceru -- 3` |
| Baseline HIR/MIR | `program.ceir`, `program.mir.txt` |
| Native generation | `program.lir.txt`, `program.machine.txt`, `program.origins.s`, `program.o` |
| Link execution | `link-plan.json`, `link.stderr.log`, `program.link.map`, `manifest.json` |

Add `--ssa` to compare SSA lowering while retaining baseline MIR, or `--include-sources` to save exact input contents. See the [build contract](../../docs/design/native-build.en.md) for limits, failures and origins. The executable uses the target system runtime; release packaging is described below.

## Distribute the completed build

Package the successful build above. Packaging performs no relinking, optimization or execution.

```powershell
cargo run --quiet -- release native target/native-demo -o target/native-dist
.\target\native-dist\build\program.exe 3
Get-FileHash target/native-dist/build/program.exe -Algorithm SHA256
```

```sh
cargo run --quiet -- release native target/native-demo -o target/native-dist
./target/native-dist/build/program 3
(cd target/native-dist && sha256sum -c SHA256SUMS)
```

Output matches the example above. Copy the entire `native-dist` directory elsewhere to run it, preserving Linux execute permission. The recipient needs no Cerune or Clang, but does need the target OS and runtime recorded in `runtime-requirements.json`.

| Inspection | Distribution path |
| --- | --- |
| Entry, target, per-file SHA-256 | `release.json` |
| ELF loader, libraries and symbol versions, or Windows import DLLs | `runtime-requirements.json` |
| Original generation stages, origins and link records | `build/`, unchanged bytes |
| Recipient usage guides | `README.md`, `README.en.md` |

Existing destinations are not overwritten. Source inclusion follows the build-time choice. Automatic target-environment execution checks and transitive dependency resolution are outside this initial release. See the [distribution contract](../../docs/design/native-release.en.md).