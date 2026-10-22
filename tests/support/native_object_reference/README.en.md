# Frozen encoder migration reference

[日本語](README.md)

Moved from Cerune f48a72c together with its existing low-level tests. Compiled only under cfg(test) from src/codegen/x86_64.rs; production object generation uses SASHIMI.

Do not duplicate new encoding, parsing or format features/fixes here. Retain this reference while checking complete objects and diagnostics across the switch. Reconsider its removal once SASHIMI has a typed API and independent regression coverage.

Source: [old implementation](https://github.com/Cerune-Lang/Cerune/tree/f48a72c01f33ff32f483a8c125d0b69b4f7d2444/src/codegen/x86_64). The repository's MIT OR Apache-2.0 licensing remains unchanged.
