# New-file creation and readback

[日本語](README.md)

[roundtrip.ceru](roundtrip.ceru) saves Japanese text, NUL, CR/LF, and combining characters, then prints readback equality and byte length. Expected output is `true` followed by `16`.

**Currently runs through an explicit in-memory connection supplied to the Rust API. CLI filesystem connections are not implemented.** The test below supplies separate empty connections to HIR/MIR/SSA/VM, comparing output, saved bytes, and operation records. Reusing the same connection stops with `already-exists` without changing the existing file.

```sh
cargo test --test standard_input file_io::roundtrip_example
cargo run --quiet -- emit-ir examples/files/roundtrip.ceru
cargo run --quiet -- emit-mir examples/files/roundtrip.ceru
cargo run --quiet -- emit-bytecode examples/files/roundtrip.ceru
```

Ordinary `run` stops with an `unconfigured` diagnostic. C/LLVM/QBE/WAT/Native generation reports unsupported file I/O. No connection or source-directory save location is selected implicitly. See the [design and Rust APIs](../../docs/design/file-io.en.md) for current coverage.
