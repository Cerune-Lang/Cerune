//! 実行中に通った箇所を順番に保持します。
pub mod file;
use crate::{
    ir::{FunctionId, NodeId},
    runtime::RuntimeFailure,
    source::Span,
};

/// 呼出し式・条件式・return文の位置。暗黙のmain呼出しと末尾復帰では関数宣言を指します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    pub node: Option<NodeId>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Call { function: FunctionId, site: Site },
    Branch { taken: bool, site: Site },
    Return { function: FunctionId, site: Site },
    Stop(RuntimeFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Incomplete {
    EventLimit,
    AllocationFailed,
}

/// events()[0]が最初の出来事。関数名は同じコンパイルのFunctionIdから取得します。
#[derive(Debug)]
pub struct Trace(BoundedEvents<Event>);
impl Trace {
    pub(crate) fn new(limit: usize) -> Self {
        Self(BoundedEvents::new(limit))
    }
    pub(crate) fn record(&mut self, event: Event) {
        self.0.record(event);
    }
    pub fn events(&self) -> &[Event] {
        self.0.events()
    }
    pub fn incomplete(&self) -> Option<Incomplete> {
        self.0.incomplete()
    }
}

/// 実行器はEvent、WATホストは記録箇所の番号を実行順に保持します。
#[derive(Debug)]
pub struct BoundedEvents<T> {
    events: Vec<T>,
    limit: usize,
    incomplete: Option<Incomplete>,
}
impl<T> BoundedEvents<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            events: Vec::new(),
            limit,
            incomplete: None,
        }
    }
    pub fn events(&self) -> &[T] {
        &self.events
    }
    pub fn incomplete(&self) -> Option<Incomplete> {
        self.incomplete
    }
    pub fn record(&mut self, event: T) {
        if self.incomplete.is_some() {
            return;
        }
        if self.events.len() == self.limit {
            self.incomplete = Some(Incomplete::EventLimit);
        } else if self.events.try_reserve(1).is_err() {
            self.incomplete = Some(Incomplete::AllocationFailed);
        } else {
            self.events.push(event);
        }
    }
}
