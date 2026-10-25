#[path = "support/process.rs"]
mod process;
const NATIVE: bool = true;
const ENCODER: Option<fn(&str) -> Vec<u8>> = None;
const QBE: bool = false;
const LLVM: bool = false;
include!("support/generated_output.rs");

mod call_depth {
    const ROUTE: &str = "native";
    include!("call_depth/generated.rs");
}
