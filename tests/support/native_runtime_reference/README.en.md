# Reference for typed runtime migration

[日本語](README.md)

Fixed test material extracted from assembly constants in `src/codegen/x86_64/utf8.rs` and `string.rs` at Cerune 8686e7a. It covers UTF-8 validation and Windows/Linux string equality, stored with LF line endings.

Production code does not reference these files. Do not copy new implementation changes or fixes here; retain them while comparing objects with typed Programs. The 135-example comparison also replaces migrated helpers with this assembly to reconstruct the old baseline.

The repository's MIT OR Apache-2.0 licensing is preserved.

`heap.rs` / `array.rs` freeze allocation, reference counting, release and bookkeeping ASM emitters from bf81a78. Target and limit parameters support standalone and 135-example old-object comparisons. These are also test-only references and must not track new implementation changes.
