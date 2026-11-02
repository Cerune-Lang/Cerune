use serde_json::{Value, json};
pub struct Pipes {
    pub input: std::fs::File,
    pub output: std::fs::File,
    pub error: std::fs::File,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Code(u32),
    #[cfg(target_os = "linux")]
    Signal(i32),
}
impl Exit {
    pub fn value(self) -> Value {
        match self {
            Self::Code(code) => json!({"kind":"code","value":code}),
            #[cfg(target_os = "linux")]
            Self::Signal(signal) => json!({"kind":"signal","value":signal}),
        }
    }
}
