#[path = "support/process.rs"]
mod process;
const NATIVE: bool = false;
const ENCODER: Option<fn(&str) -> Vec<u8>> = None;
const QBE: bool = false;
const LLVM: bool = false;
include!("support/generated_output.rs");

#[path = "call_depth/c.rs"]
mod call_depth;
