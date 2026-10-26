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

Tests separate real directories and in-memory connections, comparing output, saved bytes, records, and failures. Omitting --file-root stops with unconfigured. LLVM/QBE/WAT/Native generation reports unsupported file I/O. No save location is selected implicitly. Linux reads require /proc/self/fd. See the [contract and coverage](../../docs/design/file-io.en.md).


The same example also runs through generated C. Select Windows/Linux through the C compilation target.

~~~sh
cargo run --quiet -- emit-c examples/files/roundtrip.ceru -o roundtrip.c
clang -std=c11 roundtrip.c -o roundtrip
mkdir c-output
./roundtrip --cerune-runtime-v1 --file-root c-output --
~~~

In Windows PowerShell use .\roundtrip.exe for the last command. MinGW-targeted Clang also needs -municode for wmain. The terminating -- is required. An omitted connection never defaults to cwd. Subsequent arguments are passed to the program.
