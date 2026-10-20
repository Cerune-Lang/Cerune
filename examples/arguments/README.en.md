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

HIR/MIR/SSA/VM, CLI and generated C execution are supported. LLVM/QBE/WAT/Native connections remain unimplemented and explicitly reject generation containing argument operations. Integer parsing is unimplemented. See [contracts, limits and remaining work](../../docs/design/dogfooding-inputs.en.md).

Compare the same output through generated C. On Linux/macOS:

```sh
cargo run -- emit-c examples/arguments/list.ceru -o target/arguments.c
clang -std=c11 target/arguments.c -o target/arguments
./target/arguments hello --ssa
```

For Windows MSVC-target Clang, output to `target/arguments.exe` and run `./target/arguments.exe hello --ssa`. MinGW targets additionally require `-municode` when compiling. The generated executable does not need the Cerune CLI separator `--`.
