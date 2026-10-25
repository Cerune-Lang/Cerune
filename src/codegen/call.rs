//! 各生成経路へ渡す不変の呼出し位置。実行状態や別のIRは保持しません。
use crate::{ir::NodeId, source::Span};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Site {
    pub function: usize,
    pub origin: Option<(NodeId, Span)>,
}
impl Site {
    pub fn name(self) -> String {
        match self.origin {
            Some((node, span)) => format!(
                "cerune_call_site_{}_{}_{}_{}",
                self.function,
                node.0,
                span.start(),
                span.end()
            ),
            None => format!("cerune_call_site_{}_host", self.function),
        }
    }
    fn origin(self) -> String {
        match self.origin {
            Some((node, span)) => format!(
                " node={}{} bytes={}..{}\n",
                node.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
            None => " origin=host\n".into(),
        }
    }
    pub fn suffix(self) -> String {
        format!(" function={}{}", self.function, self.origin())
    }
    pub fn allocation(self) -> String {
        format!("cerune: runtime-v1 code=allocation-failed{}", self.origin())
    }
}
