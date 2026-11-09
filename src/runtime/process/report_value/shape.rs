//! 標準宣言の形状を検査します。タグ番号と型IDはこの表では決めません。
use super::{Error, IntegerType, ir};
#[derive(Clone, Copy)]
pub(super) enum Shape {
    Report,
    Origin,
    Started,
    Exit,
    Failure,
    FailureCode,
    Phase,
    Channel,
    Cleanup,
    CleanupError,
}
#[derive(Clone, Copy)]
pub(super) enum Slot {
    Integer(IntegerType),
    Bool,
    Named(Shape),
}
impl Shape {
    fn variants(self) -> Option<&'static [&'static str]> {
        Some(match self {
            Self::Report | Self::Origin => return None,
            Self::Started => &["No", "Yes", "Unknown"],
            Self::Exit => &["Absent", "Code", "Signal"],
            Self::Failure => &["None", "Reason"],
            Self::FailureCode => &[
                "InvalidRequest",
                "RequestLimit",
                "Unconfigured",
                "Unregistered",
                "ConnectionDisabled",
                "OutputBudget",
                "AllocationFailed",
                "NotFound",
                "Denied",
                "Resources",
                "Unsupported",
                "StartFailed",
                "TransferFailed",
                "CloseFailed",
                "WaitFailed",
                "ZeroProgress",
                "InvalidResponse",
                "OutputLimit",
                "Timeout",
                "CleanupFailed",
                "ProgressUnknown",
            ],
            Self::Phase => &[
                "Validate",
                "Reserve",
                "Start",
                "Transfer",
                "CloseStdin",
                "Wait",
                "Cleanup",
            ],
            Self::Channel => &["Stdin", "Stdout", "Stderr"],
            Self::Cleanup => &["Absent", "Complete", "Incomplete"],
            Self::CleanupError => &["Timeout", "Terminate", "Reap", "Close", "InvalidResponse"],
        })
    }
    fn fields(self) -> &'static [(&'static str, Slot)] {
        use IntegerType::{I64, U32, U64};
        use Slot::{Bool, Integer as I, Named as N};
        match self {
            Self::Report => &[
                ("origin", N(Self::Origin)),
                ("started", N(Self::Started)),
                ("stdin_requested", I(U64)),
                ("stdin_accepted", I(U64)),
                ("stdout_limit", I(U64)),
                ("stderr_limit", I(U64)),
                ("timeout_ms", I(U64)),
                ("stdout_read", I(U64)),
                ("stderr_read", I(U64)),
                ("stdout_retained", I(U64)),
                ("stderr_retained", I(U64)),
                ("progress_known", Bool),
                ("exit", N(Self::Exit)),
                ("failure", N(Self::Failure)),
                ("cleanup", N(Self::Cleanup)),
                ("cleanup_reported", N(Self::Cleanup)),
            ],
            Self::Origin => &[
                ("node_id", I(U64)),
                ("source_id", I(U64)),
                ("start", I(U64)),
                ("end", I(U64)),
            ],
            Self::Exit => &[
                ("$tag", I(I64)),
                ("$Code$value", I(U32)),
                ("$Signal$value", I(U32)),
            ],
            Self::Failure => &[
                ("$tag", I(I64)),
                ("$Reason$code", N(Self::FailureCode)),
                ("$Reason$phase", N(Self::Phase)),
            ],
            Self::Phase => &[("$tag", I(I64)), ("$Transfer$channel", N(Self::Channel))],
            Self::Cleanup => &[
                ("$tag", I(I64)),
                ("$Complete$termination_requested", Bool),
                ("$Incomplete$child_reaped", Bool),
                ("$Incomplete$handles_closed", Bool),
                ("$Incomplete$scope_finished", Bool),
                ("$Incomplete$error", N(Self::CleanupError)),
            ],
            _ => &[("$tag", I(I64))],
        }
    }
}
pub(super) fn validate(
    definitions: &[ir::TypeDefinition],
    id: ir::TypeId,
    shape: Shape,
) -> Result<(), Error> {
    let def = definitions.get(id.0).ok_or(Error::InvalidLayout)?;
    if def.id != id || def.fields.len() != shape.fields().len() {
        return Err(Error::InvalidLayout);
    }
    match (shape.variants(), &def.variants) {
        (None, None) => (),
        (Some(expected), Some(actual))
            if expected.len() == actual.len()
                && expected
                    .iter()
                    .all(|v| actual.iter().filter(|a| a.as_str() == *v).count() == 1) => {}
        _ => return Err(Error::InvalidLayout),
    }
    for (index, (field, (name, slot))) in def.fields.iter().zip(shape.fields()).enumerate() {
        if field.id.0 != index || field.name != *name {
            return Err(Error::InvalidLayout);
        }
        match (slot, &field.ty) {
            (Slot::Integer(a), ir::Type::Integer(b)) if a == b => (),
            (Slot::Bool, ir::Type::Bool) => (),
            (Slot::Named(shape), ir::Type::Named(id)) => validate(definitions, *id, *shape)?,
            _ => return Err(Error::InvalidLayout),
        }
    }
    Ok(())
}
