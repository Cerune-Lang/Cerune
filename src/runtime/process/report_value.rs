//! std:process::Reportの固定領域を準備し、OS処理後は即値だけを書き込みます。
use super::{Channel, Cleanup, CleanupError, Code, Exit, Phase, Report, Started};
use crate::{ir, types::IntegerType};
mod shape;
use shape::Shape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidLayout,
    Allocation,
    UnrepresentableReport,
}

/// 制御フロー・所有処理を共有せず、既存の2種類の値表現だけを橋渡しします。
pub(crate) trait Value: Sized {
    fn integer(value: i128, ty: IntegerType) -> Self;
    fn boolean(value: bool) -> Self;
    fn aggregate(id: ir::TypeId, fields: Vec<Self>) -> Self;
    fn parts(&mut self) -> (ir::TypeId, &mut [Self]);
}
impl Value for crate::runtime::value::Value {
    fn integer(value: i128, ty: IntegerType) -> Self {
        Self::Number(crate::runtime::numeric::Number::Integer(value, ty))
    }
    fn boolean(value: bool) -> Self {
        Self::Bool(value)
    }
    fn aggregate(type_id: ir::TypeId, fields: Vec<Self>) -> Self {
        Self::Aggregate { type_id, fields }
    }
    fn parts(&mut self) -> (ir::TypeId, &mut [Self]) {
        match self {
            Self::Aggregate { type_id, fields } => (*type_id, fields),
            _ => unreachable!("prepared process report aggregate"),
        }
    }
}
impl Value for crate::vm::Value {
    fn integer(value: i128, ty: IntegerType) -> Self {
        Self::Integer(value, ty)
    }
    fn boolean(value: bool) -> Self {
        Self::Bool(value)
    }
    fn aggregate(type_id: ir::TypeId, fields: Vec<Self>) -> Self {
        Self::Aggregate {
            type_id: type_id.0,
            fields,
        }
    }
    fn parts(&mut self) -> (ir::TypeId, &mut [Self]) {
        match self {
            Self::Aggregate { type_id, fields } => (ir::TypeId(*type_id), fields),
            _ => unreachable!("prepared process report aggregate"),
        }
    }
}

/// 公開する前の固定値です。未使用なら通常のDropで全Vecを解放します。
pub(crate) struct PreparedReport<'a, V> {
    definitions: &'a [ir::TypeDefinition],
    value: V,
}
impl<'a, V: Value> PreparedReport<'a, V> {
    /// report_idはコンパイル済み標準モジュールの型IDを呼出し元が渡します。
    /// 型名やRust enumの番号から推測せず、共通IRの形状を起動前に検査します。
    pub(crate) fn new(
        definitions: &'a [ir::TypeDefinition],
        report_id: ir::TypeId,
    ) -> Result<Self, Error> {
        Self::prepare(definitions, report_id, &mut |_| Ok(()))
    }
    fn prepare(
        definitions: &'a [ir::TypeDefinition],
        report_id: ir::TypeId,
        before_allocation: &mut impl FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        shape::validate(definitions, report_id, Shape::Report)?;
        Ok(Self {
            definitions,
            value: allocate(definitions, report_id, before_allocation)?,
        })
    }
    /// 全scalarの表現可能性を先に検査し、既存の領域へ書き込みます。
    /// Durationを丸めたり、cleanupの生報告を判定結果で置き換えたりしません。
    pub(crate) fn finish(mut self, report: &Report) -> Result<V, Error> {
        let timeout =
            u64::try_from(report.timeout.as_millis()).map_err(|_| Error::UnrepresentableReport)?;
        if !report.timeout.subsec_nanos().is_multiple_of(1_000_000) {
            return Err(Error::UnrepresentableReport);
        }
        let counts = [
            report.stdin_requested,
            report.stdin_accepted,
            report.stdout_limit,
            report.stderr_limit,
            report.stdout_read,
            report.stderr_read,
            report.stdout_retained,
            report.stderr_retained,
        ]
        .map(u64::try_from);
        if counts.iter().any(Result::is_err) {
            return Err(Error::UnrepresentableReport);
        }
        let origin = [
            report.origin.node_id.0,
            report.origin.span.source_id().index(),
            report.origin.span.start(),
            report.origin.span.end(),
        ]
        .map(u64::try_from);
        if origin.iter().any(Result::is_err) {
            return Err(Error::UnrepresentableReport);
        }
        let d = self.definitions;
        let (_, fields) = self.value.parts();
        for (field, number) in fields[0].parts().1.iter_mut().zip(origin) {
            *field = V::integer(i128::from(number.unwrap()), IntegerType::U64);
        }
        tag(
            &mut fields[1],
            d,
            match report.started {
                Started::No => "No",
                Started::Yes => "Yes",
                Started::Unknown => "Unknown",
            },
        );
        for (index, number) in [2, 3, 4, 5, 7, 8, 9, 10].into_iter().zip(counts) {
            fields[index] = V::integer(i128::from(number.unwrap()), IntegerType::U64);
        }
        fields[6] = V::integer(i128::from(timeout), IntegerType::U64);
        fields[11] = V::boolean(report.progress_known);
        match report.exit {
            None => tag(&mut fields[12], d, "Absent"),
            Some(Exit::Code(value)) => {
                tag(&mut fields[12], d, "Code");
                fields[12].parts().1[1] = V::integer(i128::from(value), IntegerType::U32);
            }
            Some(Exit::Signal(value)) => {
                tag(&mut fields[12], d, "Signal");
                fields[12].parts().1[2] = V::integer(i128::from(value), IntegerType::U32);
            }
        }
        match report.failure {
            None => tag(&mut fields[13], d, "None"),
            Some(failure) => {
                tag(&mut fields[13], d, "Reason");
                let f = fields[13].parts().1;
                tag(&mut f[1], d, code_name(failure.code));
                phase(&mut f[2], d, failure.phase);
            }
        }
        cleanup(&mut fields[14], d, report.cleanup);
        cleanup(&mut fields[15], d, report.cleanup_reported);
        Ok(self.value)
    }
}

fn allocate<V: Value>(
    definitions: &[ir::TypeDefinition],
    id: ir::TypeId,
    before_allocation: &mut impl FnMut(usize) -> Result<(), Error>,
) -> Result<V, Error> {
    let def = &definitions[id.0];
    before_allocation(def.fields.len())?;
    let mut fields = Vec::new();
    fields
        .try_reserve_exact(def.fields.len())
        .map_err(|_| Error::Allocation)?;
    for field in &def.fields {
        fields.push(match field.ty {
            ir::Type::Integer(ty) => V::integer(0, ty),
            ir::Type::Bool => V::boolean(false),
            ir::Type::Named(id) => allocate(definitions, id, before_allocation)?,
            _ => unreachable!("validated process report field"),
        });
    }
    Ok(V::aggregate(id, fields))
}
fn tag<V: Value>(value: &mut V, definitions: &[ir::TypeDefinition], name: &str) {
    let (id, fields) = value.parts();
    let tag = definitions[id.0]
        .variants
        .as_ref()
        .unwrap()
        .iter()
        .position(|v| v == name)
        .expect("validated process report variant");
    fields[0] = V::integer(tag as i128, IntegerType::I64);
}
fn phase<V: Value>(value: &mut V, d: &[ir::TypeDefinition], p: Phase) {
    let name = match p {
        Phase::Validate => "Validate",
        Phase::Reserve => "Reserve",
        Phase::Start => "Start",
        Phase::Transfer(_) => "Transfer",
        Phase::CloseStdin => "CloseStdin",
        Phase::Wait => "Wait",
        Phase::Cleanup => "Cleanup",
    };
    tag(value, d, name);
    if let Phase::Transfer(channel) = p {
        tag(
            &mut value.parts().1[1],
            d,
            match channel {
                Channel::Stdin => "Stdin",
                Channel::Stdout => "Stdout",
                Channel::Stderr => "Stderr",
            },
        );
    }
}
fn cleanup<V: Value>(value: &mut V, d: &[ir::TypeDefinition], c: Option<Cleanup>) {
    match c {
        None => tag(value, d, "Absent"),
        Some(Cleanup::Complete {
            termination_requested,
        }) => {
            tag(value, d, "Complete");
            value.parts().1[1] = V::boolean(termination_requested);
        }
        Some(Cleanup::Incomplete {
            child_reaped,
            handles_closed,
            scope_finished,
            error,
        }) => {
            tag(value, d, "Incomplete");
            let f = value.parts().1;
            f[2] = V::boolean(child_reaped);
            f[3] = V::boolean(handles_closed);
            f[4] = V::boolean(scope_finished);
            tag(
                &mut f[5],
                d,
                match error {
                    CleanupError::Timeout => "Timeout",
                    CleanupError::Terminate => "Terminate",
                    CleanupError::Reap => "Reap",
                    CleanupError::Close => "Close",
                    CleanupError::InvalidResponse => "InvalidResponse",
                },
            );
        }
    }
}
fn code_name(code: Code) -> &'static str {
    match code {
        Code::InvalidRequest => "InvalidRequest",
        Code::RequestLimit => "RequestLimit",
        Code::Unconfigured => "Unconfigured",
        Code::Unregistered => "Unregistered",
        Code::ConnectionDisabled => "ConnectionDisabled",
        Code::OutputBudget => "OutputBudget",
        Code::AllocationFailed => "AllocationFailed",
        Code::NotFound => "NotFound",
        Code::Denied => "Denied",
        Code::Resources => "Resources",
        Code::Unsupported => "Unsupported",
        Code::StartFailed => "StartFailed",
        Code::TransferFailed => "TransferFailed",
        Code::ProgressUnknown => "ProgressUnknown",
        Code::CloseFailed => "CloseFailed",
        Code::WaitFailed => "WaitFailed",
        Code::ZeroProgress => "ZeroProgress",
        Code::InvalidResponse => "InvalidResponse",
        Code::OutputLimit => "OutputLimit",
        Code::Timeout => "Timeout",
        Code::CleanupFailed => "CleanupFailed",
    }
}
#[cfg(test)]
mod tests;
