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

Add `--ssa` to compare SSA lowering while retaining baseline MIR, or `--include-sources` to save exact input contents. See the [build contract](../../docs/design/native-build.en.md) for limits, failures and origins. The executable uses the target system runtime; release packaging is a separate follow-up.
