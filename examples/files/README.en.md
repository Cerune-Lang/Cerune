# New-file creation and readback

[日本語](README.md)

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

Tests separate real directories and in-memory connections, comparing output, saved bytes, records, and failures. Omitting --file-root stops with unconfigured. WAT/Native generation reports unsupported file I/O. No save location is selected implicitly. Linux reads require /proc/self/fd. See the [contract and coverage](../../docs/design/file-io.en.md).


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
