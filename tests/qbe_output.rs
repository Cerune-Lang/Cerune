#[path = "support/process.rs"]
mod process;
const NATIVE: bool = false;
const ENCODER: Option<fn(&str) -> Vec<u8>> = None;
const QBE: bool = true;
const LLVM: bool = false;
include!("support/generated_output.rs");

mod call_depth {
    const ROUTE: &str = "qbe";
    include!("call_depth/generated.rs");
}
