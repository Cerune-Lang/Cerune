use super::ir::{
    Argument, BinaryOp, CompareOp, FloatConstant, Function, Instruction, MirOrigin, Module, Origin,
    Type,
};
use crate::{
    ir as hir,
    mir::{self, InstructionKind as Kind, LocalId, Operation as Op, TerminatorKind as Term},
};
use std::collections::HashMap;

/// 検証済みMIRだけを機械側の表現に変換します。HIRへ復元しません。
pub fn lower(
    program: &mir::Program,
    target: super::Target,
) -> Result<Module, crate::diagnostic::Diagnostic> {
    mir::validate(program).map_err(|e| e.diagnostic())?;
    let mut module = Module {
        target,
        strings: vec![],
        float_constants: vec![],
        functions: vec![],
        origins: vec![],
        mir_origins: vec![],
        instructions: vec![],
        frame_size: 0,
        string_heap_limit: None,
        array_heap_limit: None,
        uses_strings: program.uses_strings,
        uses_write: false,
    };
    for f in program
        .functions
        .iter()
        .chain(std::iter::once(&program.main))
    {
        for l in &f.locals {
            module.uses_strings |= contains_string(&l.ty);
            if contains_array(&l.ty) {
                module.array_heap_limit = Some(program.array_heap_limit);
            }
        }
        for b in &f.blocks {
            for i in &b.instructions {
                match &i.kind {
                    Kind::Output { .. } => module.uses_write = true,
                    Kind::StringManage { .. }
                    | Kind::Assign {
                        value: Op::StringConcat { .. } | Op::ReadLine(_),
                        ..
                    } => module.string_heap_limit = Some(program.string_heap_limit),
                    _ => {}
                }
            }
        }
        let body = lower_body(
            program,
            f,
            target,
            &mut module.strings,
            &mut module.float_constants,
        );
        if let Some(id) = f.id {
            module.functions.push(Function {
                id: id.0,
                name: f.name.clone(),
                frame_size: body.frame_size,
                origins: body.origins,
                mir_origins: body.mir_origins,
                instructions: body.instructions,
            });
        } else {
            module.frame_size = body.frame_size;
            module.origins = body.origins;
            module.mir_origins = body.mir_origins;
            module.instructions = body.instructions;
        }
    }
    for d in &program.types {
        for (_, _, ty) in &d.fields {
            module.uses_strings |= contains_string(ty);
            if contains_array(ty) {
                module.array_heap_limit = Some(program.array_heap_limit);
            }
        }
    }
    super::lir::validate(&module)?;
    Ok(module)
}
fn contains_string(ty: &hir::Type) -> bool {
    match ty {
        hir::Type::String => true,
        hir::Type::Array { element, .. } | hir::Type::DynamicArray { element } => {
            contains_string(element)
        }
        _ => false,
    }
}
fn contains_array(ty: &hir::Type) -> bool {
    match ty {
        hir::Type::DynamicArray { .. } => true,
        hir::Type::Array { element, .. } => contains_array(element),
        _ => false,
    }
}
struct Body {
    frame_size: usize,
    origins: Vec<Origin>,
    mir_origins: Vec<Option<MirOrigin>>,
    instructions: Vec<Instruction>,
}
struct Lowerer<'a> {
    program: &'a mir::Program,
    function: &'a mir::Function,
    slots: Vec<usize>,
    next_slot: usize,
    label: usize,
    return_pointer: Option<isize>,
    checked: HashMap<(LocalId, Vec<LocalId>), isize>,
    strings: &'a mut Vec<String>,
    floats: &'a mut Vec<FloatConstant>,
    origin: Origin,
    mir_origin: Option<MirOrigin>,
    body: Body,
}
fn lower_body(
    program: &mir::Program,
    f: &mir::Function,
    target: super::Target,
    strings: &mut Vec<String>,
    floats: &mut Vec<FloatConstant>,
) -> Body {
    let mut next_slot = 0;
    let slots = f
        .locals
        .iter()
        .map(|l| {
            let slot = next_slot;
            next_slot += slots_for(program, &l.ty).max(1);
            slot
        })
        .collect();
    let mut l = Lowerer {
        program,
        function: f,
        slots,
        next_slot,
        label: f.blocks.len() + 1,
        return_pointer: None,
        checked: HashMap::new(),
        strings,
        floats,
        origin: Origin::Synthetic,
        mir_origin: None,
        body: Body {
            frame_size: 0,
            origins: vec![],
            mir_origins: vec![],
            instructions: vec![],
        },
    };
    let mut args = super::abi::Arguments::new(target);
    for &p in &f.parameters {
        let ty = &f.locals[p.0].ty;
        let location = args.next(matches!(ty, hir::Type::F32 | hir::Type::F64));
        let offset = l.offset(p);
        if aggregate(ty) {
            l.push(Instruction::StoreAggregateParameter {
                location,
                slots: slots_for(program, ty),
                destination_offset: offset,
            });
        } else {
            l.push(Instruction::StoreParameter {
                location,
                ty: scalar_type(ty),
                offset,
            });
        }
    }
    if matches!(&f.return_type,hir::ReturnType::Value(ty) if aggregate(ty)) {
        let offset = l.pointer();
        l.return_pointer = Some(offset);
        l.push(Instruction::StoreAggregateReturnPointer { offset });
    }
    // Storeは先行するCheckIndexが保存したアドレスを使います。右辺評価後に検査し直しません。
    for b in &f.blocks {
        for i in &b.instructions {
            if let Kind::CheckIndex { root, path } = &i.kind {
                let key = (*root, path.clone());
                if !l.checked.contains_key(&key) {
                    let p = l.pointer();
                    l.checked.insert(key, p);
                }
            }
        }
    }
    l.push(Instruction::Jump(f.entry.0));
    for (bid, b) in f.blocks.iter().enumerate() {
        l.set_origin(b.origin, mir::BlockId(bid), None);
        l.push(Instruction::Label {
            id: bid,
            name: "mir_block",
        });
        for i in &b.instructions {
            l.set_origin(i.origin, mir::BlockId(bid), Some(i.id));
            let start = l.body.instructions.len();
            l.instruction(&i.kind);
            if l.body.instructions.len() == start {
                l.push(Instruction::ObserveOnly);
            }
        }
        l.set_origin(
            b.terminator.origin,
            mir::BlockId(bid),
            Some(b.terminator.id),
        );
        match &b.terminator.kind {
            Term::Jump(b) => l.push(Instruction::Jump(b.0)),
            Term::Branch {
                condition,
                then_block,
                else_block,
            } => {
                l.load(*condition);
                l.push(Instruction::JumpIfZero(else_block.0));
                l.push(Instruction::Jump(then_block.0));
            }
            Term::Return(v) => {
                if let Some(v) = v {
                    l.load(*v);
                    if aggregate(l.ty(*v)) {
                        l.push(Instruction::CopyToAggregateReturn {
                            source_offset: l.offset(*v),
                            slots: slots_for(program, l.ty(*v)),
                            pointer_offset: l.return_pointer.expect("aggregate return"),
                        });
                    }
                }
                l.push(if f.id.is_some() {
                    Instruction::Return
                } else {
                    Instruction::Jump(f.blocks.len())
                });
            }
            Term::Unreachable => l.push(Instruction::Unreachable),
        }
    }
    if f.id.is_none() {
        l.origin = Origin::Synthetic;
        l.mir_origin = None;
        l.push(Instruction::Label {
            id: f.blocks.len(),
            name: "main_exit",
        });
    }
    let outgoing = l
        .body
        .instructions
        .iter()
        .filter_map(|i| {
            if let Instruction::Call { arguments, .. } = i {
                let mut a = super::abi::Arguments::new(target);
                for arg in arguments {
                    a.next(matches!(
                        arg,
                        Argument::Scalar {
                            ty: Type::F32 | Type::F64,
                            ..
                        }
                    ));
                }
                Some(a.stack_bytes())
            } else {
                None
            }
        })
        .max()
        .unwrap_or(super::abi::Arguments::new(target).stack_bytes());
    l.body.frame_size = (8 * l.next_slot + outgoing + 15) & !15;
    l.body
}
impl Lowerer<'_> {
    fn set_origin(
        &mut self,
        o: mir::Origin,
        block: mir::BlockId,
        instruction: Option<mir::InstructionId>,
    ) {
        self.origin = match o.source() {
            Some(s) => Origin::Source {
                node_id: s.node_id,
                span: s.span,
            },
            None => Origin::Synthetic,
        };
        self.mir_origin = Some(MirOrigin {
            block,
            instruction,
            origin: o,
        });
    }
    fn push(&mut self, i: Instruction) {
        self.body.instructions.push(i);
        self.body.origins.push(self.origin);
        self.body.mir_origins.push(self.mir_origin);
    }
    fn ty(&self, v: LocalId) -> &hir::Type {
        &self.function.locals[v.0].ty
    }
    fn offset(&self, v: LocalId) -> isize {
        slot_offset(self.slots[v.0])
    }
    fn pointer(&mut self) -> isize {
        let s = self.next_slot;
        self.next_slot += 1;
        slot_offset(s)
    }
    fn next_label(&mut self) -> usize {
        let n = self.label;
        self.label += 1;
        n
    }
    fn load(&mut self, v: LocalId) {
        if !aggregate(self.ty(v)) {
            self.load_scalar(scalar_type(self.ty(v)), self.offset(v));
        }
    }
    fn copy(&mut self, ty: &hir::Type, source: usize, destination: usize) {
        match ty {
            hir::Type::Named(id) => {
                let fields = self.program.types[id.0].fields.clone();
                let mut off = 0;
                for (_, _, ty) in fields {
                    self.copy(&ty, source + off, destination + off);
                    off += slots_for(self.program, &ty);
                }
            }
            hir::Type::Array { element, length } => {
                let stride = slots_for(self.program, element);
                for i in 0..*length {
                    self.copy(element, source + i * stride, destination + i * stride);
                }
            }
            ty => {
                self.load_scalar(scalar_type(ty), slot_offset(source));
                self.store_scalar(scalar_type(ty), slot_offset(destination));
            }
        }
    }
    fn copy_local(&mut self, source: LocalId, destination: usize) {
        self.copy(&self.ty(source).clone(), self.slots[source.0], destination);
    }
    fn store_pointer(&mut self, v: LocalId, pointer: isize) {
        self.load(v);
        if aggregate(self.ty(v)) {
            self.push(Instruction::CopyToPointer {
                source_offset: self.offset(v),
                slots: slots_for(self.program, self.ty(v)),
                pointer_offset: pointer,
            });
        } else {
            self.push(match self.ty(v) {
                hir::Type::F32 => Instruction::StoreF32ToPointer(pointer),
                hir::Type::F64 => Instruction::StoreF64ToPointer(pointer),
                _ => Instruction::StoreI64ToPointer(pointer),
            });
        }
    }
    fn address(&mut self, root: LocalId, path: &[LocalId], destination: isize) {
        let mut ty = self.ty(root).clone();
        let mut base_offset = self.offset(root);
        let mut base_is_pointer = false;
        for (n, &index) in path.iter().enumerate() {
            self.load(index);
            let label = self.next_label();
            let out = if n + 1 == path.len() {
                destination
            } else {
                self.pointer()
            };
            match &ty {
                hir::Type::Array { element, length } => {
                    self.push(Instruction::CheckedArrayAddress {
                        base_offset,
                        base_is_pointer,
                        length: *length,
                        element_slots: slots_for(self.program, element),
                        destination_offset: out,
                        label,
                    });
                    ty = (**element).clone();
                }
                hir::Type::DynamicArray { element } => {
                    self.push(Instruction::DynamicArrayAddress {
                        base_offset,
                        base_is_pointer,
                        destination_offset: out,
                        label,
                    });
                    ty = (**element).clone();
                }
                _ => unreachable!("validated array path"),
            }
            base_offset = out;
            base_is_pointer = true;
        }
    }
    fn instruction(&mut self, k: &Kind) {
        match k {
            Kind::Assign { destination, value } => self.operation(*destination, value),
            Kind::Call {
                function,
                arguments,
                ..
            } => self.call(*function, arguments, None),
            Kind::CheckIndex { root, path } => {
                self.address(*root, path, self.checked[&(*root, path.clone())])
            }
            Kind::Store { root, path, value } => {
                if path.is_empty() {
                    self.copy_local(*value, self.slots[root.0]);
                } else {
                    self.store_pointer(*value, self.checked[&(*root, path.clone())]);
                }
            }
            Kind::Flush => self.push(Instruction::Flush),
            Kind::Output {
                value,
                newline,
                quoted,
            } => {
                self.load(*value);
                self.push(Instruction::Output {
                    kind: crate::codegen::display::kind(self.ty(*value), *quoted),
                    newline: *newline,
                });
            }
            Kind::ArrayInitialize { array, value } => {
                let owner_offset = self.offset(*array);
                let destination_offset = self.pointer();
                self.push(Instruction::ArrayInitAddress {
                    owner_offset,
                    destination_offset,
                });
                self.store_pointer(*value, destination_offset);
                self.push(Instruction::ArrayInitialized { owner_offset });
            }
            Kind::ArrayRetain { value } => {
                self.load(*value);
                self.push(Instruction::ArrayRetain);
            }
            Kind::ArrayFree { value } => {
                self.load(*value);
                self.push(Instruction::ArrayFree);
            }
            Kind::ArrayRangeCheck { length, start, end } => {
                self.load(*end);
                let label = self.next_label();
                self.push(Instruction::ArrayRangeCheck {
                    length_offset: self.offset(*length),
                    start_offset: self.offset(*start),
                    label,
                });
            }
            Kind::StringManage { value, retain } => {
                self.load(*value);
                self.push(Instruction::StringManage { retain: *retain });
            }
        }
    }
    fn call(&mut self, function: hir::FunctionId, args: &[LocalId], destination: Option<LocalId>) {
        let return_type = &self.program.functions[function.0].return_type;
        let result = match return_type {
            hir::ReturnType::Value(ty) if aggregate(ty) => Some(if let Some(d) = destination {
                self.offset(d)
            } else {
                let s = self.next_slot;
                self.next_slot += slots_for(self.program, ty).max(1);
                slot_offset(s)
            }),
            _ => None,
        };
        let arguments = args
            .iter()
            .map(|v| {
                if aggregate(self.ty(*v)) {
                    Argument::Aggregate {
                        offset: self.offset(*v),
                    }
                } else {
                    Argument::Scalar {
                        ty: scalar_type(self.ty(*v)),
                        offset: self.offset(*v),
                    }
                }
            })
            .collect();
        self.push(Instruction::Call {
            function_id: function.0,
            arguments,
            aggregate_result_offset: result,
        });
    }
    fn operation(&mut self, d: LocalId, op: &Op) {
        let ty = self.ty(d).clone();
        let destination = self.slots[d.0];
        match op {
            Op::Copy(v) => {
                self.copy_local(*v, destination);
                return;
            }
            Op::Array(values) => {
                let hir::Type::Array { element, .. } = &ty else {
                    unreachable!()
                };
                let stride = slots_for(self.program, element);
                for (n, v) in values.iter().enumerate() {
                    self.copy_local(*v, destination + n * stride);
                }
                return;
            }
            Op::Construct {
                ty: id,
                base,
                fields,
            } => {
                if let Some(base) = base {
                    self.copy_local(*base, destination);
                }
                for (f, v) in fields {
                    self.copy_local(*v, destination + field_slot(self.program, id.0, f.0));
                }
                return;
            }
            Op::Field {
                base,
                ty: id,
                field,
            } => {
                self.copy(
                    &ty,
                    self.slots[base.0] + field_slot(self.program, id.0, field.0),
                    destination,
                );
                return;
            }
            Op::Index { base, index } => {
                let pointer_offset = self.pointer();
                self.address(*base, &[*index], pointer_offset);
                if aggregate(&ty) {
                    self.push(Instruction::CopyFromPointer {
                        pointer_offset,
                        destination_offset: self.offset(d),
                        slots: slots_for(self.program, &ty),
                    });
                    return;
                } else {
                    self.push(Instruction::LoadFromPointer {
                        ty: scalar_type(&ty),
                        pointer_offset,
                    });
                }
            }
            Op::Call {
                function,
                arguments,
                ..
            } => {
                self.call(*function, arguments, Some(d));
                if aggregate(&ty) {
                    return;
                }
            }
            Op::Literal(v) => match v {
                mir::Literal::Boolean(b) => {
                    self.push(Instruction::MovI64ImmediateToRax(i64::from(*b)))
                }
                mir::Literal::Integer(n) => self.push(Instruction::MovI64ImmediateToRax(*n as i64)),
                mir::Literal::String(s) => {
                    let id = self.strings.len();
                    self.strings.push(s.clone());
                    self.push(Instruction::LoadStringConstant(id));
                }
                mir::Literal::Float(s) => {
                    let id = self.floats.len();
                    self.floats.push(match ty {
                        hir::Type::F32 => FloatConstant::F32 {
                            id,
                            bits: s.parse::<f32>().expect("validated literal").to_bits(),
                        },
                        hir::Type::F64 => FloatConstant::F64 {
                            id,
                            bits: s.parse::<f64>().expect("validated literal").to_bits(),
                        },
                        _ => unreachable!(),
                    });
                    self.push(if ty == hir::Type::F32 {
                        Instruction::LoadF32Constant(id)
                    } else {
                        Instruction::LoadF64Constant(id)
                    });
                }
            },
            Op::Unary { op, value } => {
                self.load(*value);
                match (*op, scalar_type(&ty)) {
                    (hir::UnaryOp::BitNot, Type::I64) => self.push(Instruction::BitNot {
                        mask: crate::codegen::complement_mask(&ty),
                    }),
                    (hir::UnaryOp::Negate, Type::I64) => {
                        self.push(Instruction::NegI64);
                        let label = self.next_label();
                        self.push(Instruction::TrapIfOverflow(label));
                    }
                    (hir::UnaryOp::Negate, Type::F32) => self.push(Instruction::NegF32),
                    (hir::UnaryOp::Negate, Type::F64) => self.push(Instruction::NegF64),
                    (hir::UnaryOp::Not, Type::Bool) => self.push(Instruction::NotBool),
                    _ => unreachable!("validated unary operation"),
                }
            }
            Op::Binary { op, left, right } => self.binary(*op, *left, *right, &ty),
            Op::ConvertInteger {
                value, from, to, ..
            } => {
                self.load(*value);
                let conversion = crate::codegen::NumericConversion {
                    mode: crate::types::ConversionMode::Exact,
                    from: crate::types::NumericType::Integer(*from),
                    to: crate::types::NumericType::Integer(*to),
                };
                if from != to && conversion.uses_u64() {
                    let label = self.next_label();
                    self.push(Instruction::ConvertNumeric { conversion, label });
                }
            }
            Op::ConvertNumeric {
                value,
                from,
                to,
                mode,
                ..
            } => {
                self.load(*value);
                if from == to {
                    self.store_scalar(scalar_type(&ty), self.offset(d));
                    return;
                }
                let label = self.next_label();
                self.push(Instruction::ConvertNumeric {
                    conversion: crate::codegen::NumericConversion {
                        from: *from,
                        to: *to,
                        mode: *mode,
                    },
                    label,
                });
            }
            Op::ArrayLength(v) => match self.ty(*v) {
                hir::Type::Array { length, .. } => {
                    self.push(Instruction::MovI64ImmediateToRax(*length as i64))
                }
                _ => {
                    self.load(*v);
                    self.push(Instruction::ArrayLength);
                }
            },
            Op::ReadLine(v) => {
                self.load(*v);
                self.push(Instruction::ReadLine);
            }
            Op::StringByteLength(v) => {
                self.load(*v);
                self.push(Instruction::LoadStringLength);
            }
            Op::StringConcat { left, right } => {
                self.load(*right);
                let label = self.next_label();
                self.push(Instruction::StringConcat {
                    left_offset: self.offset(*left),
                    label,
                });
            }
            Op::ArrayAllocate {
                length,
                element_width,
            } => {
                self.load(*length);
                let hir::Type::DynamicArray { element } = &ty else {
                    unreachable!()
                };
                let label = self.next_label();
                self.push(Instruction::ArrayAllocate {
                    width: *element_width,
                    stride: 8 * slots_for(self.program, element),
                    label,
                });
            }
            Op::ArrayReleaseOwner(v) => {
                self.load(*v);
                self.push(Instruction::ArrayReleaseOwner);
            }
        }
        if let hir::Type::Integer(t) = ty
            && !matches!(
                t,
                crate::types::IntegerType::I64 | crate::types::IntegerType::U64
            )
            && matches!(
                op,
                Op::Unary { .. } | Op::Binary { .. } | Op::ConvertInteger { .. }
            )
        {
            use crate::runtime::FailureCode as F;
            let failure = match op {
                Op::ConvertInteger { .. } => F::IntegerConversionOutOfRange,
                Op::Binary {
                    op: hir::BinaryOp::Divide,
                    ..
                } => F::DivisionOverflow,
                _ => F::IntegerOverflow,
            };
            let label = self.next_label();
            self.push(Instruction::CheckIntegerRange {
                ty: t,
                label,
                failure,
            });
        }
        self.store_scalar(scalar_type(&ty), self.offset(d));
    }
    fn binary(&mut self, op: hir::BinaryOp, left: LocalId, right: LocalId, ty: &hir::Type) {
        let operand_ty = scalar_type(self.ty(left));
        let scratch = self.offset(left);
        self.load(right);
        match operand_ty {
            Type::DynamicArray => {
                unreachable!("array comparisons are expanded in common IR")
            }
            Type::String => {
                self.push(Instruction::CompareString {
                    left_offset: scratch,
                    equal: op == hir::BinaryOp::Equal,
                });
            }
            Type::Bool | Type::I64 => {
                self.push(Instruction::MoveRaxToRcx);
                self.push(Instruction::LoadI64ScratchToRax(scratch));

                if let Some(op) = crate::codegen::integer_binary_op(op, self.ty(left)) {
                    let label = self.next_label();
                    self.push(Instruction::IntegerBinary {
                        op,
                        ty: crate::codegen::integer_type(ty),
                        label,
                    });
                } else if let Some(op) = compare_op(op) {
                    self.push(if crate::codegen::is_u64(self.ty(left)) {
                        Instruction::CompareU64(op)
                    } else {
                        Instruction::CompareI64(op)
                    });
                } else {
                    let op = (op).into();
                    if op == BinaryOp::Divide {
                        let label = self.next_label();
                        self.push(Instruction::TrapIfInvalidI64Division(label));
                        self.push(Instruction::SignExtendRax);
                        self.push(Instruction::DivideRaxByRcx);
                    } else {
                        self.push(Instruction::I64Binary(op));
                        let label = self.next_label();
                        self.push(Instruction::TrapIfOverflow(label));
                    }
                }
            }
            Type::F32 => {
                self.push(Instruction::CopyXmm0ToXmm1F32);
                self.push(Instruction::LoadF32ScratchToXmm0(scratch));
                if let Some(op) = compare_op(op) {
                    self.push(Instruction::CompareF32(op));
                } else {
                    self.push(Instruction::F32Binary((op).into()));
                }
            }
            Type::F64 => {
                self.push(Instruction::CopyXmm0ToXmm1F64);
                self.push(Instruction::LoadF64ScratchToXmm0(scratch));
                if let Some(op) = compare_op(op) {
                    self.push(Instruction::CompareF64(op));
                } else {
                    self.push(Instruction::F64Binary((op).into()));
                }
            }
        }
    }
    fn load_scalar(&mut self, ty: Type, offset: isize) {
        self.push(match ty {
            Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
                Instruction::LoadI64FromStack(offset)
            }
            Type::F32 => Instruction::LoadF32FromStack(offset),
            Type::F64 => Instruction::LoadF64FromStack(offset),
        });
    }

    fn store_scalar(&mut self, ty: Type, offset: isize) {
        self.push(match ty {
            Type::DynamicArray | Type::String | Type::Bool | Type::I64 => {
                Instruction::StoreI64ToStack(offset)
            }
            Type::F32 => Instruction::StoreF32ToStack(offset),
            Type::F64 => Instruction::StoreF64ToStack(offset),
        });
    }
}
fn aggregate(ty: &hir::Type) -> bool {
    matches!(ty, hir::Type::Named(_) | hir::Type::Array { .. })
}
fn slots_for(p: &mir::Program, ty: &hir::Type) -> usize {
    match ty {
        hir::Type::Named(id) => p.types[id.0]
            .fields
            .iter()
            .map(|(_, _, ty)| slots_for(p, ty))
            .sum(),
        hir::Type::Array { element, length } => slots_for(p, element) * length,
        _ => 1,
    }
}
fn field_slot(p: &mir::Program, id: usize, field: usize) -> usize {
    p.types[id].fields[..field]
        .iter()
        .map(|(_, _, ty)| slots_for(p, ty))
        .sum()
}
fn scalar_type(ty: &hir::Type) -> Type {
    match ty {
        hir::Type::DynamicArray { .. } => Type::DynamicArray,
        hir::Type::String => Type::String,
        hir::Type::Bool => Type::Bool,
        hir::Type::Integer(_) => Type::I64,
        hir::Type::F32 => Type::F32,
        hir::Type::F64 => Type::F64,
        hir::Type::Named(_) | hir::Type::Array { .. } => {
            unreachable!("expected a scalar type")
        }
    }
}

fn slot_offset(slot: usize) -> isize {
    -8 * (slot as isize + 1)
}

impl From<hir::BinaryOp> for BinaryOp {
    fn from(value: hir::BinaryOp) -> Self {
        match value {
            hir::BinaryOp::Add => Self::Add,
            hir::BinaryOp::Subtract => Self::Subtract,
            hir::BinaryOp::Multiply => Self::Multiply,
            hir::BinaryOp::Divide => Self::Divide,
            hir::BinaryOp::Remainder
            | hir::BinaryOp::BitAnd
            | hir::BinaryOp::BitOr
            | hir::BinaryOp::BitXor
            | hir::BinaryOp::ShiftLeft
            | hir::BinaryOp::ShiftRight => {
                unreachable!("integer operation uses separate lowering")
            }
            hir::BinaryOp::Equal
            | hir::BinaryOp::NotEqual
            | hir::BinaryOp::Less
            | hir::BinaryOp::LessEqual
            | hir::BinaryOp::Greater
            | hir::BinaryOp::GreaterEqual => {
                unreachable!("comparisons use dedicated x86-64 instructions")
            }
        }
    }
}

const fn compare_op(op: hir::BinaryOp) -> Option<CompareOp> {
    match op {
        hir::BinaryOp::Add
        | hir::BinaryOp::Subtract
        | hir::BinaryOp::Multiply
        | hir::BinaryOp::Divide
        | hir::BinaryOp::Remainder
        | hir::BinaryOp::BitAnd
        | hir::BinaryOp::BitOr
        | hir::BinaryOp::BitXor
        | hir::BinaryOp::ShiftLeft
        | hir::BinaryOp::ShiftRight => None,
        hir::BinaryOp::Equal => Some(CompareOp::Equal),
        hir::BinaryOp::NotEqual => Some(CompareOp::NotEqual),
        hir::BinaryOp::Less => Some(CompareOp::Less),
        hir::BinaryOp::LessEqual => Some(CompareOp::LessEqual),
        hir::BinaryOp::Greater => Some(CompareOp::Greater),
        hir::BinaryOp::GreaterEqual => Some(CompareOp::GreaterEqual),
    }
}
