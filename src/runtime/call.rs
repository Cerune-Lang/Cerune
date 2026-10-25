//! Explicit call-depth control and immutable failure snapshots shared by executors.
use crate::{
    ir::{FunctionId, NodeId},
    source::Span,
};
use std::{fmt, num::NonZeroU32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub node_id: NodeId,
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub function: FunctionId,
    pub origin: Option<Origin>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    limit: NonZeroU32,
    attempted: Frame,
    active: Vec<Frame>,
}
impl Failure {
    pub fn limit(&self) -> NonZeroU32 {
        self.limit
    }
    pub fn attempted(&self) -> Frame {
        self.attempted
    }
    /// Outermost first. The rejected call is separate and was never entered.
    pub fn active(&self) -> &[Frame] {
        &self.active
    }
    /// ASCII only: no paths, source text, arguments, or runtime values.
    pub fn record(&self) -> String {
        use fmt::Write;
        let mut out = format!(
            "call-v1 reason=depth-limit limit={}{}",
            self.limit,
            fields(self.attempted)
        );
        for (index, frame) in self.active.iter().enumerate() {
            write!(out, "\ncall-v1 frame={index}{}", fields(*frame)).unwrap();
        }
        out
    }
}
fn fields(frame: Frame) -> String {
    let position = match frame.origin {
        Some(o) => format!(
            " node={}{} bytes={}..{}",
            o.node_id.0,
            o.span.source_id().record_field(),
            o.span.start(),
            o.span.end()
        ),
        None => " origin=host".into(),
    };
    format!(" function={}{}", frame.function.0, position)
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "call depth limit exceeded (limit {})", self.limit)
    }
}
impl std::error::Error for Failure {}

pub(crate) struct Stack {
    limit: NonZeroU32,
    active: Vec<Frame>,
}
impl Stack {
    pub fn new(limit: NonZeroU32) -> Self {
        Self {
            limit,
            active: vec![],
        }
    }
    pub fn enter(&mut self, frame: Frame) -> Result<(), Box<Failure>> {
        if self.active.len() >= self.limit.get() as usize {
            return Err(Box::new(Failure {
                limit: self.limit,
                attempted: frame,
                active: self.active.clone(),
            }));
        }
        self.active.push(frame);
        Ok(())
    }
    pub fn leave(&mut self) {
        self.active.pop().expect("balanced source function calls");
    }
}
