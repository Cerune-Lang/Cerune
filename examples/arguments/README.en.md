# Program arguments

[日本語](README.md)

[list.ceru](list.ceru) prints the count, each value and its UTF-8 byte length, then checks a missing entry. Array display makes empty strings visible.

```sh
cargo run -- run examples/arguments/list.ceru -- hello --ssa
cargo run -- run-mir examples/arguments/list.ceru --ssa -- hello --ssa
cargo run -- run-vm examples/arguments/list.ceru -- hello --ssa
```

Each prints the following. The `--ssa` after the first `--` is a program value.

```text
2
["hello"]
5
["--ssa"]
5
end
```

No arguments print `0` and `end`. One empty argument prints `1`, `[""]`, `0`, and `end`. Follow your shell's rules for passing empty arguments.

HIR/MIR/SSA/VM, CLI and generated C/LLVM/QBE execution are supported. WAT/Native connections remain unimplemented and explicitly reject generation containing argument operations. Integer parsing is unimplemented. See [contracts, limits and remaining work](../../docs/design/dogfooding-inputs.en.md).

Compare the same output through generated C. On Linux/macOS:

```sh
cargo run -- emit-c examples/arguments/list.ceru -o target/arguments.c
clang -std=c11 target/arguments.c -o target/arguments
./target/arguments hello --ssa
```

For Windows MSVC-target Clang, output to `target/arguments.exe` and run `./target/arguments.exe hello --ssa`. MinGW targets additionally require `-municode` when compiling. The generated executable does not need the Cerune CLI separator `--`.

Run through LLVM as well. For Linux:

```sh
cargo run -- emit-llvm examples/arguments/list.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/arguments.ll
clang target/arguments.ll -o target/arguments-llvm
./target/arguments-llvm hello --ssa
```

On Windows, select `x86_64-pc-windows-msvc` and output to `target/arguments-llvm.exe`. Override limits by passing options such as `--argument-count-limit 2 --argument-byte-limit 10` to `emit-llvm`. The displayed result is the same as above.

QBE 1.3 produces the same output. Generate and link for Linux:

```sh
cargo run -- emit-qbe examples/arguments/list.ceru --target x86_64-unknown-linux-gnu -o target/arguments.ssa
qbe -t amd64_sysv -o target/arguments.s target/arguments.ssa
clang -target x86_64-unknown-linux-gnu target/arguments.s -o target/arguments
./target/arguments hello --ssa
```

On Windows, use `x86_64-pc-windows-msvc` for Cerune and Clang, `-t amd64_win` for QBE, and `target/arguments.exe` for the executable. Set acquisition limits explicitly with options such as `emit-qbe --argument-count-limit 2 --argument-byte-limit 10`.
