# Pre-migration Native instruction selection

[日本語](README.md)

[Instruction selection, numeric conversion, failure reporting, stdin and arguments](emit.rs) are frozen from Cerune eef765f for tests. Feed the same LIR to this emitter and the typed Program builder, then compare complete objects produced by the old encoder.

Constants, strings, allocation bookkeeping and normal output use the existing [frozen runtime](../native_runtime_reference/README.en.md) and [data emitter](../native_data_reference.rs). This is not merely a comparison obtained by parsing the new Program ASM.

Excluded from production builds; do not add features or update this baseline to match migrated output. Current ASM display fixtures serve a different purpose.

`examples.txt` freezes the 135 examples at migration completion (fe97b30). New features are checked through execution tests across routes and are not added to this old implementation.
