//! 既存の言語値と共通子プロセスruntimeの接続です。命令の制御フローは含めません。
use super::{Backend, Origin, Outcome, Processes, Request, ReservationError, report_value};
use crate::{
    ir,
    runtime::{
        array_heap::{ArrayError, ArrayHeap, byte_outputs::ReservedByteOutputs},
        string_heap::StringValue,
    },
    types::IntegerType,
};
use report_value::PreparedReport;
mod request;
pub(crate) use request::PreparedRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidLayout,
    InvalidValue,
    Allocation,
    Size,
    Array(ArrayError),
    Report(report_value::Error),
}
impl From<ArrayError> for Error {
    fn from(value: ArrayError) -> Self {
        Self::Array(value)
    }
}
impl From<report_value::Error> for Error {
    fn from(value: report_value::Error) -> Self {
        Self::Report(value)
    }
}
use crate::runtime::array_heap::io_values::ArrayIoValue;
pub(crate) use crate::runtime::array_heap::io_values::Element;

pub(crate) trait Value: report_value::Value + ArrayIoValue {
    fn fields(&self, id: ir::TypeId) -> Option<&[Self]>;
    fn text(&self) -> Option<&StringValue>;
}
impl Value for crate::runtime::value::Value {
    fn fields(&self, id: ir::TypeId) -> Option<&[Self]> {
        match self {
            Self::Aggregate { type_id, fields } if *type_id == id => Some(fields),
            _ => None,
        }
    }
    fn text(&self) -> Option<&StringValue> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}
impl Value for crate::vm::Value {
    fn fields(&self, id: ir::TypeId) -> Option<&[Self]> {
        match self {
            Self::Aggregate { type_id, fields } if *type_id == id.0 => Some(fields),
            _ => None,
        }
    }
    fn text(&self) -> Option<&StringValue> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

/// 返却の外側・Report・失敗時の空配列を、runtime呼出し前から所有します。
pub(crate) struct PreparedOutcome<'d, 'h, V: Value> {
    type_id: ir::TypeId,
    report: PreparedReport<'d, V>,
    fields: Vec<V>,
    outputs: ReservedByteOutputs<'h, V>,
}
impl<'d, 'h, V: Value> PreparedOutcome<'d, 'h, V> {
    pub(crate) fn new(
        definitions: &'d [ir::TypeDefinition],
        type_id: ir::TypeId,
        heap: &'h mut ArrayHeap<V>,
    ) -> Result<Self, Error> {
        let def = definition(definitions, type_id, &["report", "stdout", "stderr"])?;
        let ir::Type::Named(report_id) = def.fields[0].ty else {
            return Err(Error::InvalidLayout);
        };
        for field in &def.fields[1..] {
            if !matches!(&field.ty,ir::Type::DynamicArray{element} if **element==ir::Type::Integer(IntegerType::U8))
            {
                return Err(Error::InvalidLayout);
            }
        }
        let report = PreparedReport::new(definitions, report_id)?;
        let mut fields = Vec::new();
        fields.try_reserve_exact(3).map_err(|_| Error::Allocation)?;
        let outputs = heap.reserve_byte_outputs(0, 0)?;
        Ok(Self {
            type_id,
            report,
            fields,
            outputs,
        })
    }
    pub(crate) fn run_prepared<B: Backend>(
        self,
        processes: &mut Processes<B>,
        request: &PreparedRequest,
        available_bytes: usize,
        origin: Origin,
    ) -> Result<V, Error> {
        request.with_request(|r| self.run(processes, r, available_bytes, origin))?
    }
    pub(crate) fn run<B: Backend>(
        mut self,
        processes: &mut Processes<B>,
        request: Request<'_>,
        available_bytes: usize,
        origin: Origin,
    ) -> Result<V, Error> {
        let stdout = u64::try_from(request.stdout_limit).map_err(|_| Error::Size)?;
        let stderr = u64::try_from(request.stderr_limit).map_err(|_| Error::Size)?;
        let (outcome, _) = processes.run_with_reservation(request, available_bytes, origin, || {
            self.outputs
                .reserve_limits(stdout, stderr)
                .map_err(|e| match e {
                    ArrayError::AllocationSizeOverflow | ArrayError::AllocationLimitExceeded => {
                        ReservationError::Capacity
                    }
                    ArrayError::AllocationFailed => ReservationError::Allocation,
                    _ => unreachable!("private output reservation"),
                })
        });
        self.finish(&outcome)
    }
    fn finish(mut self, outcome: &Outcome) -> Result<V, Error> {
        if outcome.report.stdout_retained != outcome.stdout.len()
            || outcome.report.stderr_retained != outcome.stderr.len()
        {
            return Err(Error::InvalidValue);
        }
        // 失敗し得るReport変換を先に済ませ、配列を公開した後は再確保しません。
        let report = self.report.finish(&outcome.report)?;
        let [stdout, stderr] = self
            .outputs
            .finish([&outcome.stdout, &outcome.stderr], V::byte)?;
        self.fields.push(report);
        self.fields.push(V::byte_array(stdout));
        self.fields.push(V::byte_array(stderr));
        Ok(V::aggregate(self.type_id, self.fields))
    }
}
fn definition<'a>(
    definitions: &'a [ir::TypeDefinition],
    id: ir::TypeId,
    names: &[&str],
) -> Result<&'a ir::TypeDefinition, Error> {
    let def = definitions.get(id.0).ok_or(Error::InvalidLayout)?;
    if def.id != id
        || def.variants.is_some()
        || def.fields.len() != names.len()
        || def
            .fields
            .iter()
            .zip(names)
            .enumerate()
            .any(|(i, (f, n))| f.id.0 != i || f.name != *n)
    {
        return Err(Error::InvalidLayout);
    }
    Ok(def)
}
#[cfg(test)]
mod tests;
