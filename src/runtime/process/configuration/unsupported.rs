//! 未対応OSでは接続を構築せず、型だけを共有します。
use crate::runtime::process::{
    Backend, Batch, CallContext, Launch, Request,
    transfers::{Ticket, Transfer},
};
use std::time::Duration;
pub enum OsBackend {}
impl Backend for OsBackend {
    fn now(&mut self) -> Duration {
        match *self {}
    }
    fn start(&mut self, _: &Request<'_>, _: CallContext, _: Duration) -> Launch {
        match *self {}
    }
    fn submit(&mut self, _: Transfer, _: Duration) {
        match *self {}
    }
    fn poll(&mut self, _: Duration) -> Batch {
        match *self {}
    }
    fn cancel(&mut self, _: Ticket, _: Duration) {
        match *self {}
    }
    fn begin_cleanup(&mut self, _: bool, _: Duration) {
        match *self {}
    }
}
