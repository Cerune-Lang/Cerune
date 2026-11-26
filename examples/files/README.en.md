# New-file creation and readback

[日本語](README.md)

## Read an ELF64 header

[elf_header.ceru](elf_header.ceru) uses `std:fs::bytes` to read `input.o`. It checks the minimum 64-byte header size, ELF magic, class and byte order, decodes 16-, 32- and 64-bit little-endian fields, and saves every byte to `copy.o`.

```sh
mkdir elf-sample
cargo run --quiet -- emit-obj examples/recursion/factorial.ceru --target x86_64-unknown-linux-gnu -o elf-sample/input.o
cargo run --quiet -- run examples/files/elf_header.ceru --file-root elf-sample
```

The output gives the object type (1 = ET_REL), version (1) and entry address (0). An input shorter than 64 bytes prints a message and returns. To compare routes, place the same input in a fresh directory for each run.

To try generated C on Linux, use the same input and compare the complete files with `cmp`.

```sh
mkdir elf-c
cp elf-sample/input.o elf-c/input.o
cargo run --quiet -- emit-c examples/files/elf_header.ceru -o elf-c/reader.c
clang -std=c11 -O2 elf-c/reader.c -lm -o elf-c/reader
./elf-c/reader --cerune-runtime-v1 --file-root elf-c --
cmp elf-c/input.o elf-c/copy.o
```

For generated LLVM, use the same input and specify the target explicitly. On Linux x86-64:

```sh
mkdir elf-llvm
cp elf-sample/input.o elf-llvm/input.o
cargo run --quiet -- emit-llvm examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-llvm/reader.ll
clang --target=x86_64-unknown-linux-gnu -O2 elf-llvm/reader.ll -lm -o elf-llvm/reader
./elf-llvm/reader --cerune-runtime-v1 --file-root elf-llvm --
cmp elf-llvm/input.o elf-llvm/copy.o
```

QBE 1.3 translates the `.ssa` file to assembly. Select `amd64_sysv` on Linux x86-64 or `amd64_win` on Windows x86-64. On Linux, run the following commands.

```sh
mkdir elf-qbe
cp elf-sample/input.o elf-qbe/input.o
cargo run --quiet -- emit-qbe examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-qbe/reader.ssa
qbe -t amd64_sysv -o elf-qbe/reader.s elf-qbe/reader.ssa
clang --target=x86_64-unknown-linux-gnu elf-qbe/reader.s -lm -o elf-qbe/reader
./elf-qbe/reader --cerune-runtime-v1 --file-root elf-qbe --
cmp elf-qbe/input.o elf-qbe/copy.o
```

To run generated WAT with the [reference host](../../experiments/wat_host/README.en.md), point `--file-root` at the directory containing `input.o`. On Linux, compare every byte of the copy with these commands.

```sh
mkdir elf-wat
cp elf-sample/input.o elf-wat/input.o
cargo run --quiet -- emit-wat examples/files/elf_header.ceru -o elf-wat/reader.wat
cargo run --quiet --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- elf-wat/reader.wat --file-root elf-wat --
cmp elf-wat/input.o elf-wat/copy.o
```

On Linux x86-64, compile and run the same example through Native. An exit status of zero from `cmp` confirms that every byte matches.

```sh
mkdir elf-native
cp elf-sample/input.o elf-native/input.o
cargo run --quiet -- emit-obj examples/files/elf_header.ceru --target x86_64-unknown-linux-gnu -o elf-native/reader.o
clang elf-native/reader.o -o elf-native/reader
./elf-native/reader --cerune-runtime-v1 --file-root elf-native --
cmp elf-native/input.o elf-native/copy.o
```

## Save and read back text

[roundtrip.ceru](roundtrip.ceru) saves Japanese text, NUL, CR/LF, and combining characters, then prints readback equality and byte length. Expected output is true followed by 16.

Prepare an empty directory on Windows/Linux and run the example.

~~~sh
mkdir sample-output
cargo run --quiet -- run examples/files/roundtrip.ceru --file-root sample-output
~~~

This creates sample.txt. Reusing the directory stops with already-exists without changing its contents. run-ir, run-mir, run-mir --ssa, and run-vm also work. Use independent empty directories when comparing routes.

~~~sh
cargo test --test standard_input file_io
cargo run --quiet -- emit-ir examples/files/roundtrip.ceru
cargo run --quiet -- emit-mir examples/files/roundtrip.ceru
cargo run --quiet -- emit-bytecode examples/files/roundtrip.ceru
~~~

Tests separate real directories and in-memory connections, comparing output, saved bytes, records, and failures. Omitting --file-root stops with unconfigured. Native also supports explicit Windows/Linux targets. WAT runs through the versioned test host below. No save location is selected implicitly. Linux reads require /proc/self/fd. See the [contract and coverage](../../docs/design/file-io.en.md).


The same example also runs through generated C. Select Windows/Linux through the C compilation target.

~~~sh
cargo run --quiet -- emit-c examples/files/roundtrip.ceru -o roundtrip.c
clang -std=c11 roundtrip.c -o roundtrip
mkdir c-output
./roundtrip --cerune-runtime-v1 --file-root c-output --
~~~

In Windows PowerShell use .\roundtrip.exe for the last command. MinGW-targeted Clang also needs -municode for wmain. The terminating -- is required. An omitted connection never defaults to cwd. Subsequent arguments are passed to the program.

The same example also runs through generated LLVM. Select an explicit target, then use Clang to link. On Linux:

~~~sh
cargo run --quiet -- emit-llvm examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.ll
clang --target=x86_64-unknown-linux-gnu roundtrip.ll -o roundtrip
mkdir llvm-output
./roundtrip --cerune-runtime-v1 --file-root llvm-output --
~~~

On Windows, use x86_64-pc-windows-msvc for both targets, roundtrip.exe as output, and .\roundtrip.exe to run. Clang and an MSVC-compatible linking environment are required. Startup settings match C. In addition to real-connection comparisons, development tests inject short transfers and close failures.

~~~sh
cargo test --lib codegen::llvm::files::tests
~~~

The same example runs with QBE 1.3. On Linux, explicitly select amd64_sysv:

~~~sh
cargo run --quiet -- emit-qbe examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.ssa
qbe -t amd64_sysv -o roundtrip.s roundtrip.ssa
clang --target=x86_64-unknown-linux-gnu roundtrip.s -o roundtrip
mkdir qbe-output
./roundtrip --cerune-runtime-v1 --file-root qbe-output --
~~~

On Windows, use x86_64-pc-windows-msvc for Cerune and Clang, amd64_win for QBE -t, roundtrip.exe as output, and .\roundtrip.exe to run. Clang and an MSVC-compatible link environment are required. Startup settings match C/LLVM. Set CERUNE_TEST_QBE to the QBE path to require real-connection and injected-response comparisons.

~~~sh
cargo test --test standard_input file_io::generated::qbe_tests
cargo test --lib codegen::qbe::files::tests
~~~

The same Cerune source also runs as WAT. The following test host uses in-memory files, not the OS filesystem. Its configuration selects initial state and inspected names; optional test records receive copies of saved contents.

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files-roundtrip.wat
node target/wasm-tools/node_modules/wabt/bin/wat2wasm target/files-roundtrip.wat -o target/files-roundtrip.wasm
CERUNE_TEST_WAT_FILES_FILE=tests/fixtures/file_io/wat_host.json CERUNE_TEST_WAT_FILES_TRACE=target/files-trace.json node tests/support/run_wasm_input.cjs target/files-roundtrip.wasm
~~~

Use Node and WABT 1.0.39. In Windows PowerShell, set $env:CERUNE_TEST_WAT_FILES_FILE='tests/fixtures/file_io/wat_host.json' and $env:CERUNE_TEST_WAT_FILES_TRACE='target/files-trace.json', then run the final command starting at node. Expected output is true, 16. The contents.sample.txt field of files-trace.json contains the 16-byte array. Each run creates an independent in-memory connection; this does not test saving sample.txt into an OS directory.

A host implementing the [WAT file contract](../../docs/design/wat-file-host.en.md) is required. Old hosts fail on missing imports. Set configured=false to represent an unconfigured connection; the operation stops as unconfigured. A real-OS file host is separate; --file-root or cwd is not used automatically.

~~~sh
cargo test --test standard_input file_io::wat
~~~

## Connect WAT to real files

On Windows/Linux, use the independent [reference host](../../experiments/wat_host/README.en.md). Run from the repository root.

~~~sh
cargo run --quiet -- emit-wat examples/files/roundtrip.ceru -o target/files.wat
mkdir target/wat-files-data
cargo run --manifest-path experiments/wat_host/Cargo.toml --locked --target-dir target/wat-host -- target/files.wat --file-root target/wat-files-data --
~~~

Expected output is true and 16. The program saves and reloads 16 bytes in sample.txt under the selected directory. Reusing that directory stops with already-exists. The final -- ends host settings; subsequent values are program arguments. Omitting root leaves files unconfigured. Internal memory stays private. WAT remains emit-only; the reference host is not part of Cerune build/release.

## Run through Native

On Linux, generate an Object and link using an explicitly selected Clang. Use a fresh output directory.

```sh
cargo run --quiet -- emit-obj examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu -o roundtrip.o
clang --target=x86_64-unknown-linux-gnu roundtrip.o -o roundtrip
mkdir native-output
./roundtrip --cerune-runtime-v1 --file-root native-output --
```

On Windows, use target `x86_64-pc-windows-msvc`, Object `roundtrip.obj`, executable `roundtrip.exe`, and launch `./roundtrip.exe`. An MSVC-compatible linking environment is required. Linking ASM generated by `emit-asm` also works. Select `--ssa` to generate through SSA.

Output is `true` followed by `16`. The saved Japanese text, NUL, CR/LF, and combining character are read back unchanged. Rerunning in the same directory rejects overwriting the existing file, so use separate directories for comparisons.

## Build and distribute

The same roundtrip can be saved and distributed as a complete Native artifact. On Linux:

```sh
cargo run --quiet -- build native examples/files/roundtrip.ceru --target x86_64-unknown-linux-gnu --linker /usr/bin/clang -o target/files-build
cargo run --quiet -- release native target/files-build -o target/files-release
mkdir target/files-data
./target/files-release/build/program --cerune-runtime-v1 --file-root target/files-data --
```

On Windows, select `x86_64-pc-windows-msvc` and the actual Clang path, then launch `./target/files-release/build/program.exe`. Use fresh output directories. Add `--ssa` to build to generate through SSA.

Output and saved contents match the earlier example. Moving the distribution does not change the explicit root selected at startup. Build/release do not create sample.txt, and release neither relinks nor optimizes the executable.

The `file_connection` records in `manifest.json` and `runtime.json` describe startup and OS requirements, inherited by release.json and runtime-requirements.json. No actual save directory is embedded. See the [build](../../docs/design/native-build.en.md) and [release](../../docs/design/native-release.en.md) contracts.
