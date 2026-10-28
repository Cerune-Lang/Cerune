//! Native LIRの観測と構造検証。テキストを入力するloaderや実行器ではありません。
use super::{abi, ir::*};
use crate::diagnostic::Diagnostic;
use std::{collections::BTreeSet, fmt::Write};

/// 内部snapshotの構造を検証します。値の初期化・pointerの生存期間・意味の同値性は対象外です。
pub fn validate(module: &Module) -> Result<(), Diagnostic> {
    if module.source_functions.len() != module.functions.len() {
        return Err(Diagnostic::without_span(
            "LIR: source-function classifications must match functions",
        ));
    }
    if module.argument_limits.max_count > i64::MAX as u64 {
        return Err(Diagnostic::without_span(
            "LIR: argument count limit must not exceed i64::MAX",
        ));
    }
    let mut constants = BTreeSet::new();
    for c in &module.float_constants {
        let key = match c {
            FloatConstant::F32 { id, .. } => (false, *id),
            FloatConstant::F64 { id, .. } => (true, *id),
        };
        if !constants.insert(key) {
            return Err(Diagnostic::without_span("LIR: duplicate float constant"));
        }
    }
    for (index, f) in module.functions.iter().enumerate() {
        if f.id != index {
            return Err(Diagnostic::without_span(
                "LIR: function IDs must match their indices",
            ));
        }
        body(
            module,
            &format!("function {}", f.id),
            f.frame_size,
            &f.instructions,
            &f.origins,
            &f.mir_origins,
        )?;
    }
    body(
        module,
        "main",
        module.frame_size,
        &module.instructions,
        &module.origins,
        &module.mir_origins,
    )
}

fn body(
    m: &Module,
    name: &str,
    frame: usize,
    code: &[Instruction],
    origins: &[Origin],
    mir: &[Option<MirOrigin>],
) -> Result<(), Diagnostic> {
    let error = |message: &str| Diagnostic::without_span(format!("LIR {name}: {message}"));
    if code.len() != origins.len() || code.len() != mir.len() {
        return Err(error("instruction/origin lengths differ"));
    }
    if !frame.is_multiple_of(16) || frame > i32::MAX as usize {
        return Err(error(
            "frame size must be aligned to 16 and fit a signed 32-bit displacement",
        ));
    }
    let mut outgoing = abi::Arguments::new(m.target).stack_bytes();
    for i in code {
        if let Instruction::Call { arguments, .. } = i {
            let mut locations = abi::Arguments::new(m.target);
            for a in arguments {
                locations.next(matches!(
                    a,
                    Argument::Scalar {
                        ty: Type::F32 | Type::F64,
                        ..
                    }
                ));
            }
            outgoing = outgoing.max(locations.stack_bytes());
        }
    }
    let available = frame
        .checked_sub(outgoing)
        .ok_or_else(|| error("frame lacks outgoing argument space"))?;
    let mut labels = BTreeSet::new();
    let mut helpers = BTreeSet::new();
    for i in code {
        if let Instruction::Label { id, .. } = i
            && !labels.insert(*id)
        {
            return Err(error("duplicate block label"));
        }
    }
    for (index, i) in code.iter().enumerate() {
        let fail = |message: &str| {
            let message = format!("LIR {name} i{index}: {message}");
            match origins[index] {
                Origin::Source { span, .. } => Diagnostic::new(message, span),
                Origin::Synthetic => Diagnostic::without_span(message),
            }
        };
        if let Some(o) = mir[index] {
            let expected = match o.origin.source() {
                Some(s) => Origin::Source {
                    node_id: s.node_id,
                    span: s.span,
                },
                None => Origin::Synthetic,
            };
            if origins[index] != expected {
                return Err(fail("source and MIR origins disagree"));
            }
        }
        if matches!(
            i,
            Instruction::ArgumentCount
                | Instruction::ArgumentGet
                | Instruction::ReadLine
                | Instruction::FileReadText { .. }
                | Instruction::FileWriteNew { .. }
                | Instruction::Flush
                | Instruction::Output { .. }
        ) && !matches!(origins[index], Origin::Source { .. })
        {
            return Err(fail("I/O needs a source origin"));
        }
        // 集約値は基準slotから負方向へ8byteずつ配置します。
        let stack = |offset: isize, slots: usize| -> Result<(), Diagnostic> {
            if slots == 0 {
                return Ok(());
            }
            let depth = offset.checked_neg().and_then(|v| usize::try_from(v).ok());
            let end = depth.and_then(|v| slots.checked_sub(1)?.checked_mul(8)?.checked_add(v));
            if offset >= 0 || offset % 8 != 0 || end.is_none_or(|v| v > available) {
                return Err(fail(
                    "stack operand is outside the local frame or overlaps outgoing arguments",
                ));
            }
            Ok(())
        };
        let parameter = |location: ArgumentLocation, ty: Type| -> Result<(), Diagnostic> {
            let valid = match location {
                ArgumentLocation::Register(n) => {
                    n < if !m.target.is_linux() {
                        4
                    } else if matches!(ty, Type::F32 | Type::F64) {
                        8
                    } else {
                        6
                    }
                }
                ArgumentLocation::Stack(n) => {
                    n.is_multiple_of(8)
                        && n >= if m.target.is_linux() { 0 } else { 32 }
                        && n <= i32::MAX as usize - 16
                }
            };
            if valid {
                Ok(())
            } else {
                Err(fail("invalid ABI parameter location"))
            }
        };
        let product = |a: usize, b: usize| {
            a.checked_mul(b)
                .ok_or_else(|| fail("stack extent overflow"))
        };
        use Instruction::*;
        let helper = match i {
            ArrayAllocate { label, .. }
            | ArrayRangeCheck { label, .. }
            | DynamicArrayAddress { label, .. }
            | LoadStringByte { label, .. }
            | StringConcat { label, .. }
            | ConvertNumeric { label, .. }
            | IntegerBinary { label, .. }
            | CheckIntegerRange { label, .. }
            | CheckedArrayLoad { label, .. }
            | CheckedArrayCopy { label, .. }
            | CheckedArrayAddress { label, .. }
            | TrapIfOverflow(label)
            | TrapIfInvalidI64Division(label) => Some(*label),
            _ => None,
        };
        if let Some(id) = helper
            && !helpers.insert(id)
        {
            return Err(fail("duplicate helper label ID"));
        }
        match i {
            Jump(id) | JumpIfZero(id) if !labels.contains(id) => {
                return Err(fail("missing branch label"));
            }
            LoadStringConstant(id) if *id >= m.strings.len() => {
                return Err(fail("missing string constant"));
            }
            LoadF32Constant(id)
                if !m
                    .float_constants
                    .iter()
                    .any(|c| matches!(c, FloatConstant::F32 { id: n, .. } if n == id)) =>
            {
                return Err(fail("missing f32 constant"));
            }
            LoadF64Constant(id)
                if !m
                    .float_constants
                    .iter()
                    .any(|c| matches!(c, FloatConstant::F64 { id: n, .. } if n == id)) =>
            {
                return Err(fail("missing f64 constant"));
            }
            Call {
                function_id,
                arguments,
                aggregate_result_offset,
            } => {
                if *function_id >= m.functions.len() {
                    return Err(fail("missing callee"));
                }
                for a in arguments {
                    match a {
                        Argument::Scalar { offset, .. } | Argument::Aggregate { offset } => {
                            stack(*offset, 1)?
                        }
                    }
                }
                if let Some(offset) = aggregate_result_offset {
                    stack(*offset, 1)?;
                }
            }
            LoadI64FromStack(o)
            | StoreI64ToStack(o)
            | LoadF32FromStack(o)
            | StoreF32ToStack(o)
            | LoadF64FromStack(o)
            | StoreF64ToStack(o)
            | StoreI64ToPointer(o)
            | StoreF32ToPointer(o)
            | StoreF64ToPointer(o)
            | LoadI64ScratchToRax(o)
            | LoadF32ScratchToXmm0(o)
            | LoadF64ScratchToXmm0(o)
            | StoreAggregateReturnPointer { offset: o }
            | LoadFromPointer {
                pointer_offset: o, ..
            }
            | ArrayInitialized { owner_offset: o }
            | LoadStringByte {
                string_offset: o, ..
            }
            | StringConcat { left_offset: o, .. }
            | CompareString { left_offset: o, .. } => stack(*o, 1)?,
            FileReadText { name_offset } => stack(*name_offset, 1)?,
            FileWriteNew {
                name_offset,
                text_offset,
            } => {
                stack(*name_offset, 1)?;
                stack(*text_offset, 1)?;
            }
            ArrayRangeCheck {
                length_offset,
                start_offset,
                ..
            } => {
                stack(*length_offset, 1)?;
                stack(*start_offset, 1)?;
            }
            DynamicArrayAddress {
                base_offset,
                destination_offset,
                ..
            } => {
                stack(*base_offset, 1)?;
                stack(*destination_offset, 1)?;
            }
            ArrayInitAddress {
                owner_offset,
                destination_offset,
            } => {
                stack(*owner_offset, 1)?;
                stack(*destination_offset, 1)?;
            }
            CopyFromPointer {
                pointer_offset,
                destination_offset,
                slots,
            } => {
                stack(*pointer_offset, 1)?;
                stack(*destination_offset, *slots)?;
            }
            CopyToPointer {
                pointer_offset,
                source_offset,
                slots,
            }
            | CopyToAggregateReturn {
                pointer_offset,
                source_offset,
                slots,
            } => {
                stack(*pointer_offset, 1)?;
                stack(*source_offset, *slots)?;
            }
            StoreParameter {
                location,
                ty,
                offset,
            } => {
                parameter(*location, *ty)?;
                stack(*offset, 1)?;
            }
            StoreAggregateParameter {
                location,
                slots,
                destination_offset,
            } => {
                parameter(*location, Type::I64)?;
                stack(*destination_offset, *slots)?;
            }
            CheckedArrayLoad {
                base_offset,
                length,
                ..
            } => stack(*base_offset, *length)?,
            CheckedArrayCopy {
                base_offset,
                length,
                element_slots,
                destination_offset,
                ..
            } => {
                stack(*base_offset, product(*length, *element_slots)?)?;
                stack(*destination_offset, *element_slots)?;
            }
            CheckedArrayAddress {
                base_offset,
                base_is_pointer,
                length,
                element_slots,
                destination_offset,
                ..
            } => {
                stack(
                    *base_offset,
                    if *base_is_pointer {
                        1
                    } else {
                        product(*length, *element_slots)?
                    },
                )?;
                stack(*destination_offset, 1)?;
            }
            // offsetを持たない命令。追加した命令の検証漏れをコンパイル時に検出します。
            ObserveOnly
            | Unreachable
            | ArrayAllocate { .. }
            | ArrayLength
            | ArrayRetain
            | ArrayReleaseOwner
            | ArrayFree
            | StringManage { .. }
            | ArgumentCount
            | ArgumentGet
            | ReadLine
            | Flush
            | Output { .. }
            | CompareU64(_)
            | LoadStringLength
            | LoadStringConstant(_)
            | ConvertNumeric { .. }
            | BitNot { .. }
            | IntegerBinary { .. }
            | CheckIntegerRange { .. }
            | Label { .. }
            | JumpIfZero(_)
            | Jump(_)
            | MovI64ImmediateToRax(_)
            | Return
            | LoadF32Constant(_)
            | LoadF64Constant(_)
            | NegI64
            | TrapIfOverflow(_)
            | NotBool
            | NegF32
            | NegF64
            | MoveRaxToRcx
            | I64Binary(_)
            | CompareI64(_)
            | SignExtendRax
            | TrapIfInvalidI64Division(_)
            | DivideRaxByRcx
            | CopyXmm0ToXmm1F32
            | CopyXmm0ToXmm1F64
            | F32Binary(_)
            | F64Binary(_)
            | CompareF32(_)
            | CompareF64(_) => {}
        }
        if let Output { kind, .. } = i
            && !matches!(
                *kind,
                "string" | "quoted" | "bool" | "i64" | "u64" | "f32" | "f64"
            )
        {
            return Err(fail("unknown write kind"));
        }
        if matches!(i, Output { .. }) && !m.uses_write {
            return Err(fail("write runtime is absent"));
        }
        if matches!(
            i,
            LoadStringByte { .. } | LoadStringConstant(_) | LoadStringLength | CompareString { .. }
        ) && !m.uses_strings
        {
            return Err(fail("string runtime is absent"));
        }
        if matches!(
            i,
            ArgumentCount
                | ArgumentGet
                | ReadLine
                | FileReadText { .. }
                | FileWriteNew { .. }
                | StringConcat { .. }
                | StringManage { .. }
        ) && m.string_heap_limit.is_none()
        {
            return Err(fail("string heap runtime is absent"));
        }
        if matches!(
            i,
            ArrayAllocate { .. }
                | ArrayRangeCheck { .. }
                | DynamicArrayAddress { .. }
                | ArrayInitAddress { .. }
                | ArrayInitialized { .. }
                | ArrayLength
                | ArrayRetain
                | ArrayReleaseOwner
                | ArrayFree
        ) && m.array_heap_limit.is_none()
        {
            return Err(fail("array runtime is absent"));
        }
    }
    Ok(())
}

/// 検証したsnapshotを表示します。命令名/operandは内部enumのDebug表記で、永続入力形式ではありません。
pub fn emit(module: &Module) -> Result<String, Diagnostic> {
    validate(module)?;
    let mut out = format!(
        "cerune-lir-observation-v1\ntarget {}\nstring-heap {:?}\narray-heap {:?}\nuses-strings {}\nuses-write {}\n",
        module.target.triple(),
        module.string_heap_limit,
        module.array_heap_limit,
        module.uses_strings,
        module.uses_write
    );
    if let Some(limit) = module.call_limit {
        writeln!(
            out,
            "call-depth-limit {limit}\nsource-functions {:?}",
            module.source_functions
        )
        .unwrap();
    }
    if super::arguments::used(module) {
        writeln!(
            out,
            "argument-count-limit {}\nargument-byte-limit {}",
            module.argument_limits.max_count, module.argument_limits.max_bytes
        )
        .unwrap();
    }
    for (id, s) in module.strings.iter().enumerate() {
        writeln!(out, "string {id} bytes={} {s:?}", s.len()).unwrap();
    }
    for c in &module.float_constants {
        writeln!(out, "constant {c:?}").unwrap();
    }
    for f in &module.functions {
        emit_body(
            &mut out,
            &format!("function {} {:?}", f.id, f.name),
            f.frame_size,
            &f.instructions,
            &f.origins,
            &f.mir_origins,
        );
    }
    emit_body(
        &mut out,
        "main",
        module.frame_size,
        &module.instructions,
        &module.origins,
        &module.mir_origins,
    );
    Ok(out)
}
fn emit_body(
    out: &mut String,
    name: &str,
    frame: usize,
    code: &[Instruction],
    origins: &[Origin],
    mir: &[Option<MirOrigin>],
) {
    writeln!(out, "{name} frame-bytes={frame} {{").unwrap();
    for (id, ((i, o), m)) in code.iter().zip(origins).zip(mir).enumerate() {
        writeln!(out, "  i{id}: {i:?}\n    source: {o:?}\n    mir: {m:?}").unwrap();
    }
    out.push_str("}\n");
}
