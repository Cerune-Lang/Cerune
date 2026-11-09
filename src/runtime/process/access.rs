//! 実行器から共通runtimeへ渡す実行用接続です。観測APIではありません。
use super::{
    language_values::{self, PreparedOutcome, PreparedRequest, Value},
    *,
};
use crate::{ir, runtime::array_heap::ArrayHeap};
pub(crate) struct Call<'a, V: Value> {
    pub definitions: &'a [ir::TypeDefinition],
    pub request_type: ir::TypeId,
    pub outcome_type: ir::TypeId,
    pub request: &'a V,
    pub heap: &'a mut ArrayHeap<V>,
    pub available: usize,
    pub origin: Origin,
}
pub(crate) trait Access<V: Value> {
    fn run_value(&mut self, call: Call<'_, V>) -> Result<V, language_values::Error>;
}
impl<V: Value, B: Backend> Access<V> for Processes<B> {
    fn run_value(&mut self, c: Call<'_, V>) -> Result<V, language_values::Error> {
        let request = PreparedRequest::new(c.definitions, c.request_type, c.request)?;
        PreparedOutcome::new(c.definitions, c.outcome_type, c.heap)?.run_prepared(
            self,
            &request,
            c.available,
            c.origin,
        )
    }
}
pub(crate) struct Connection<'a, V: Value> {
    pub access: &'a mut dyn Access<V>,
    pub available: usize,
}
pub(crate) fn execute<V: Value>(
    connection: &mut Option<Connection<'_, V>>,
    mut call: Call<'_, V>,
) -> Result<V, language_values::Error> {
    match connection {
        Some(c) => {
            call.available = c.available;
            c.access.run_value(call)
        }
        None => unconfigured().run_value(call),
    }
}
struct Unconfigured;
impl Backend for Unconfigured {
    fn now(&mut self) -> Duration {
        unreachable!("unconfigured process")
    }
    fn start(&mut self, _: &Request<'_>, _: CallContext, _: Duration) -> Launch {
        unreachable!("unconfigured process")
    }
    fn submit(&mut self, _: transfers::Transfer, _: Duration) {
        unreachable!("unconfigured process")
    }
    fn poll(&mut self, _: Duration) -> Batch {
        unreachable!("unconfigured process")
    }
    fn cancel(&mut self, _: transfers::Ticket, _: Duration) {
        unreachable!("unconfigured process")
    }
    fn begin_cleanup(&mut self, _: bool, _: Duration) {
        unreachable!("unconfigured process")
    }
}
fn unconfigured() -> Processes<Unconfigured> {
    Processes::unconfigured(Limits {
        arguments: usize::MAX,
        argument_bytes: usize::MAX,
        stdin_bytes: usize::MAX,
        output_bytes: usize::MAX,
        timeout: Duration::MAX,
        cleanup_timeout: Duration::MAX,
    })
}
/// 資源不足だけを言語の停止理由にします。壊れた型・値は内部エラーです。
pub(crate) fn resource_error(error: language_values::Error) -> Option<crate::runtime::FailureCode> {
    use crate::runtime::{FailureCode as F, array_heap::ArrayError as A};
    use language_values::Error as E;
    match error {
        E::Allocation
        | E::Report(report_value::Error::Allocation)
        | E::Array(A::AllocationFailed) => Some(F::AllocationFailed),
        E::Size | E::Array(A::AllocationSizeOverflow) => Some(F::AllocationSizeOverflow),
        E::Array(A::AllocationLimitExceeded) => Some(F::AllocationLimitExceeded),
        _ => None,
    }
}
pub(crate) fn mir_types(types: &[crate::mir::TypeDefinition]) -> Vec<ir::TypeDefinition> {
    types
        .iter()
        .map(|t| ir::TypeDefinition {
            id: t.id,
            name: t.name.clone(),
            variants: t.variants.clone(),
            span: crate::source::Span::empty(0),
            fields: t
                .fields
                .iter()
                .map(|(id, name, ty)| ir::FieldDefinition {
                    id: *id,
                    name: name.clone(),
                    ty: ty.clone(),
                    default: None,
                    span: crate::source::Span::empty(0),
                })
                .collect(),
        })
        .collect()
}
pub(crate) fn bytecode_types(types: &[crate::bytecode::TypeDefinition]) -> Vec<ir::TypeDefinition> {
    fn ty(t: &crate::bytecode::Type) -> ir::Type {
        use crate::bytecode::Type as B;
        match t {
            B::Bool => ir::Type::Bool,
            B::String => ir::Type::String,
            B::Integer(t) => ir::Type::Integer(*t),
            B::F32 => ir::Type::F32,
            B::F64 => ir::Type::F64,
            B::Named(id) => ir::Type::Named(ir::TypeId(*id)),
            B::Array { element, length } => ir::Type::Array {
                element: Box::new(ty(element)),
                length: *length,
            },
            B::DynamicArray { element } => ir::Type::DynamicArray {
                element: Box::new(ty(element)),
            },
        }
    }
    types
        .iter()
        .enumerate()
        .map(|(id, t)| ir::TypeDefinition {
            id: ir::TypeId(id),
            name: t.name.clone(),
            variants: t.variants.clone(),
            span: crate::source::Span::empty(0),
            fields: t
                .fields
                .iter()
                .enumerate()
                .map(|(id, f)| ir::FieldDefinition {
                    id: ir::FieldId(id),
                    name: f.name.clone(),
                    ty: ty(&f.ty),
                    default: None,
                    span: crate::source::Span::empty(0),
                })
                .collect(),
        })
        .collect()
}
