use std::{collections::HashMap, fmt::Write};

use crate::{
    diagnostic::Diagnostic,
    ir::{
        self, BinaryOp, BindingId, Expr, ExprKind, NodeId, Program, Statement, StatementKind,
        UnaryOp,
    },
    source::Span,
    types::IntegerType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,
    String,
    Integer(IntegerType),
    F32,
    F64,
    Named(usize),
    Array { element: Box<Type>, length: usize },
    DynamicArray { element: Box<Type> },
}

impl From<crate::types::NumericType> for Type {
    fn from(ty: crate::types::NumericType) -> Self {
        match ty {
            crate::types::NumericType::Integer(ty) => Self::Integer(ty),
            crate::types::NumericType::F32 => Self::F32,
            crate::types::NumericType::F64 => Self::F64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BytecodeProgram {
    pub array_heap_limit: u64,
    pub string_heap_limit: u64,
    pub type_definitions: Vec<TypeDefinition>,
    pub functions: Vec<BytecodeFunction>,
    pub slots: Vec<Slot>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    pub name: String,
    pub parameter_count: usize,
    pub return_type: ReturnType,
    pub slots: Vec<Slot>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    Void,
    Value(Type),
}

#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub origin: InstructionOrigin,
}

impl Instruction {
    /// ソースコードに由来する命令を作ります。
    pub const fn source(kind: InstructionKind, node_id: NodeId, span: Span) -> Self {
        Self {
            kind,
            origin: InstructionOrigin::Source { node_id, span },
        }
    }

    /// コンパイラが補助的に生成した命令を作ります。
    pub const fn synthetic(kind: InstructionKind) -> Self {
        Self {
            kind,
            origin: InstructionOrigin::Synthetic,
        }
    }
}

/// bytecode命令がどこから生成されたかを表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionOrigin {
    /// ソースコード中の構文要素から生成された命令です。
    Source {
        /// 命令を生成したCerune IRの文または式です。
        node_id: NodeId,

        /// 実行時診断で強調するソース範囲です。
        span: Span,
    },

    /// ソースコード上に対応箇所を持たない、コンパイラ生成の命令です。
    Synthetic,
}

#[derive(Debug, Clone)]
pub enum InstructionKind {
    DynamicArrayLength {
        element: Type,
    },
    DynamicIndex {
        element: Type,
    },
    ArrayAllocate {
        element: Type,
        element_width: u64,
    },
    ArrayInitialize,
    ArrayRetain,
    ArrayFree,
    ArrayReleaseOwner,
    ArrayRangeCheck,
    ArrayLength {
        element: Type,
        length: usize,
    },
    StringByteLength,
    StringConcat,
    StringManage {
        retain: bool,
    },
    ConvertNumeric {
        mode: crate::types::ConversionMode,
        from: crate::types::NumericType,
        to: crate::types::NumericType,
    },
    ConvertInteger {
        from: IntegerType,
        to: IntegerType,
    },
    PushBool(bool),
    PushString(String),
    PushInteger(i128, IntegerType),
    PushF32(f32),
    PushF64(f64),

    Load(usize),
    Store(usize),
    Assign(usize),
    ArrayAssign {
        slot: usize,
        path: Vec<ArrayAccess>,
    },
    ArrayCheck {
        slot: usize,
        path: Vec<ArrayAccess>,
    },
    Construct {
        type_id: usize,
        fields: Vec<ConstructField>,
        has_base: bool,
    },
    FieldGet {
        type_id: usize,
        field_id: usize,
    },
    ConstructArray {
        element: Type,
        length: usize,
    },
    Index {
        element: Type,
        length: usize,
    },
    Call {
        function_id: usize,
        argument_count: usize,
    },
    Return {
        has_value: bool,
    },

    Add(Type),
    Subtract(Type),
    Multiply(Type),
    Divide(Type),
    Remainder(IntegerType),
    BitAnd(IntegerType),
    BitOr(IntegerType),
    BitXor(IntegerType),
    ShiftLeft(IntegerType),
    ShiftRight(IntegerType),
    BitNot(IntegerType),

    Equal(Type),
    NotEqual(Type),
    Less(Type),
    LessEqual(Type),
    Greater(Type),
    GreaterEqual(Type),

    Negate(Type),
    Not,

    Write {
        ty: Type,
        quoted: bool,
    },
    Print(Type),

    JumpIfFalse(usize),
    Jump(usize),

    Halt,
}

#[derive(Debug, Clone)]
pub struct ArrayAccess {
    pub element: Type,
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstructField {
    pub field_id: usize,
    pub origin: FieldOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOrigin {
    Generated,
    Explicit,
    Default,
}

pub fn lower(program: &Program) -> Result<BytecodeProgram, Diagnostic> {
    let type_definitions = program
        .type_definitions
        .iter()
        .map(|definition| TypeDefinition {
            name: definition.name.clone(),
            fields: definition
                .fields
                .iter()
                .map(|field| FieldDefinition {
                    name: field.name.clone(),
                    ty: field.ty.clone().into(),
                })
                .collect(),
        })
        .collect();
    let functions = program
        .function_definitions
        .iter()
        .map(|function| {
            let mut slots = Vec::new();
            let mut slot_map = HashMap::new();
            for parameter in &function.parameters {
                let slot = slots.len();
                slots.push(Slot {
                    name: parameter.name.clone(),
                    ty: parameter.ty.clone().into(),
                    mutable: false,
                });
                slot_map.insert(parameter.id, slot);
            }
            collect_slots(&function.body, &mut slots, &mut slot_map);
            let mut compiler = Compiler {
                slot_map,
                instructions: Vec::new(),
                loops: Vec::new(),
            };
            let terminates = compiler.emit_statements(&function.body);
            if !terminates {
                compiler
                    .instructions
                    .push(Instruction::synthetic(InstructionKind::Return {
                        has_value: false,
                    }));
            }
            BytecodeFunction {
                name: function.name.clone(),
                parameter_count: function.parameters.len(),
                return_type: match &function.return_type {
                    ir::ReturnType::Void => ReturnType::Void,
                    ir::ReturnType::Value(ty) => ReturnType::Value(ty.clone().into()),
                },
                slots,
                instructions: compiler.instructions,
            }
        })
        .collect();

    let mut slots = Vec::new();
    let mut slot_map = HashMap::new();

    collect_slots(&program.statements, &mut slots, &mut slot_map);

    let mut compiler = Compiler {
        slot_map,
        instructions: Vec::new(),
        loops: Vec::new(),
    };

    compiler.emit_statements(&program.statements);

    // 明示mainの起動は、ソースに直接書かれた呼び出しではありません。
    if let Some(main) = program
        .function_definitions
        .iter()
        .find(|function| function.name == "main")
    {
        compiler
            .instructions
            .push(Instruction::synthetic(InstructionKind::Call {
                function_id: main.id.0,
                argument_count: 0,
            }));
    }

    compiler
        .instructions
        .push(Instruction::synthetic(InstructionKind::Halt));

    Ok(BytecodeProgram {
        array_heap_limit: program.array_heap_limit,
        string_heap_limit: program.string_heap_limit,
        type_definitions,
        functions,
        slots,
        instructions: compiler.instructions,
    })
}

pub fn format_program(program: &BytecodeProgram) -> String {
    let mut output = String::new();

    writeln!(output, "; Cerune bytecode v0.1").unwrap();
    if program
        .instructions
        .iter()
        .chain(program.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i.kind, InstructionKind::ArrayAllocate { .. }))
    {
        writeln!(
            output,
            "; array-heap-limit={} bytes (live element storage)",
            program.array_heap_limit
        )
        .unwrap();
    }
    if program
        .instructions
        .iter()
        .chain(program.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i.kind, InstructionKind::StringConcat))
    {
        writeln!(
            output,
            "; string-heap-limit={} bytes (live dynamic payload)",
            program.string_heap_limit
        )
        .unwrap();
    }

    for (type_id, definition) in program.type_definitions.iter().enumerate() {
        writeln!(output, "\n.type {type_id} {}", definition.name).unwrap();
        for (field_id, field) in definition.fields.iter().enumerate() {
            writeln!(
                output,
                ".field {type_id}.{field_id} {} {}",
                field.name,
                type_name(&field.ty, program)
            )
            .unwrap();
        }
    }

    for (function_id, function) in program.functions.iter().enumerate() {
        writeln!(output).unwrap();
        write!(output, ".function {function_id} {}(", function.name).unwrap();
        for parameter_index in 0..function.parameter_count {
            if parameter_index > 0 {
                output.push_str(", ");
            }
            let parameter = &function.slots[parameter_index];
            write!(
                output,
                "{parameter_index}:{}:{}",
                parameter.name,
                type_name(&parameter.ty, program)
            )
            .unwrap();
        }
        match &function.return_type {
            ReturnType::Void => writeln!(output, ") -> void").unwrap(),
            ReturnType::Value(ty) => {
                writeln!(output, ") -> {}", type_name(ty, program)).unwrap();
            }
        }

        for (index, slot) in function.slots.iter().enumerate() {
            write!(output, ".slot {index} ").unwrap();
            if slot.mutable {
                output.push_str("mut ");
            }
            writeln!(output, "{} {}", slot.name, type_name(&slot.ty, program)).unwrap();
        }

        for (pc, instruction) in function.instructions.iter().enumerate() {
            write!(output, "{pc:04}  ").unwrap();
            format_instruction(&instruction.kind, program, &function.slots, &mut output);
        }
        writeln!(output, ".end").unwrap();
    }

    if !program.slots.is_empty() {
        writeln!(output).unwrap();

        for (index, slot) in program.slots.iter().enumerate() {
            write!(output, ".slot {index} ").unwrap();

            if slot.mutable {
                output.push_str("mut ");
            }

            writeln!(output, "{} {}", slot.name, type_name(&slot.ty, program)).unwrap();
        }
    }

    writeln!(output).unwrap();

    for (pc, instruction) in program.instructions.iter().enumerate() {
        write!(output, "{pc:04}  ").unwrap();

        format_instruction(&instruction.kind, program, &program.slots, &mut output);
    }

    output
}

struct Compiler {
    slot_map: HashMap<BindingId, usize>,
    instructions: Vec<Instruction>,
    loops: Vec<LoopContext>,
}

struct LoopContext {
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
}

impl Compiler {
    fn emit_statements(&mut self, statements: &[Statement]) -> bool {
        for statement in statements {
            if self.emit_statement(statement) {
                return true;
            }
        }

        false
    }

    fn emit_statement(&mut self, statement: &Statement) -> bool {
        match &statement.kind {
            StatementKind::ArrayInitialize { array, value } => {
                self.emit_expr(array);
                self.emit_expr(value);
                self.emit_source(
                    InstructionKind::ArrayInitialize,
                    statement.id,
                    statement.span,
                );
                false
            }
            StatementKind::ArrayRetain { value } | StatementKind::ArrayFree { value } => {
                self.emit_expr(value);
                let op = if matches!(statement.kind, StatementKind::ArrayRetain { .. }) {
                    InstructionKind::ArrayRetain
                } else {
                    InstructionKind::ArrayFree
                };
                self.emit_source(op, statement.id, statement.span);
                false
            }
            StatementKind::ArrayRangeCheck { length, start, end } => {
                self.emit_expr(length);
                self.emit_expr(start);
                self.emit_expr(end);
                self.emit_source(
                    InstructionKind::ArrayRangeCheck,
                    statement.id,
                    statement.span,
                );
                false
            }
            StatementKind::Binding { id, value, .. } => {
                self.emit_expr(value);

                let slot = self
                    .slot_map
                    .get(id)
                    .copied()
                    .expect("binding must have a bytecode slot");

                self.emit_source(InstructionKind::Store(slot), statement.id, statement.span);
                false
            }

            StatementKind::Assignment { target, value } => {
                let slot = self
                    .slot_map
                    .get(&target.id)
                    .copied()
                    .expect("assignment target must have a bytecode slot");

                if target.projections.is_empty() {
                    self.emit_expr(value);
                    self.emit_source(InstructionKind::Assign(slot), statement.id, statement.span);
                } else {
                    for (projection_index, projection) in target.projections.iter().enumerate() {
                        let (ir::AssignmentProjection::Index { index, span, .. }
                        | ir::AssignmentProjection::DynamicIndex { index, span, .. }) = projection;
                        self.emit_expr(index);
                        self.emit_source(
                            InstructionKind::ArrayCheck {
                                slot,
                                path: target.projections[..=projection_index]
                                    .iter()
                                    .map(array_access)
                                    .collect(),
                            },
                            statement.id,
                            *span,
                        );
                    }
                    self.emit_expr(value);
                    self.emit_source(
                        InstructionKind::ArrayAssign {
                            slot,
                            path: target.projections.iter().map(array_access).collect(),
                        },
                        statement.id,
                        statement.span,
                    );
                }
                false
            }

            StatementKind::StringManage { value, retain } => {
                self.emit_expr(value);
                self.emit_source(
                    InstructionKind::StringManage { retain: *retain },
                    statement.id,
                    statement.span,
                );
                false
            }
            StatementKind::Write { value, quoted } => {
                self.emit_expr(value);
                self.emit_source(
                    InstructionKind::Write {
                        ty: value.ty.clone().into(),
                        quoted: *quoted,
                    },
                    statement.id,
                    statement.span,
                );
                false
            }
            StatementKind::Print { value } => {
                self.emit_expr(value);

                self.emit_source(
                    InstructionKind::Print(value.ty.clone().into()),
                    statement.id,
                    statement.span,
                );
                false
            }

            StatementKind::Call {
                function_id,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.emit_expr(argument);
                }
                self.emit_source(
                    InstructionKind::Call {
                        function_id: function_id.0,
                        argument_count: arguments.len(),
                    },
                    statement.id,
                    statement.span,
                );
                false
            }

            StatementKind::Return { value } => {
                if let Some(value) = value {
                    self.emit_expr(value);
                }
                self.emit_source(
                    InstructionKind::Return {
                        has_value: value.is_some(),
                    },
                    statement.id,
                    statement.span,
                );
                true
            }

            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.emit_expr(condition);
                let false_jump = self.instructions.len();
                self.emit_source(
                    InstructionKind::JumpIfFalse(usize::MAX),
                    statement.id,
                    condition.span,
                );

                let then_terminates = self.emit_statements(then_body);

                if else_body.is_empty() {
                    let end = self.instructions.len();
                    self.patch_jump(false_jump, end);
                    false
                } else {
                    let end_jump = if then_terminates {
                        None
                    } else {
                        let index = self.instructions.len();
                        self.emit_source(
                            InstructionKind::Jump(usize::MAX),
                            statement.id,
                            statement.span,
                        );
                        Some(index)
                    };
                    let else_start = self.instructions.len();
                    self.patch_jump(false_jump, else_start);

                    let else_terminates = self.emit_statements(else_body);

                    let end = self.instructions.len();
                    if let Some(end_jump) = end_jump {
                        self.patch_jump(end_jump, end);
                    }

                    then_terminates && else_terminates
                }
            }

            StatementKind::While { condition, body } => {
                let condition_start = self.instructions.len();
                self.emit_expr(condition);
                let end_jump = self.instructions.len();
                self.emit_source(
                    InstructionKind::JumpIfFalse(usize::MAX),
                    statement.id,
                    condition.span,
                );

                self.loops.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                let body_terminates = self.emit_statements(body);
                let loop_context = self.loops.pop().expect("while loop context must exist");

                if !body_terminates {
                    self.emit_source(
                        InstructionKind::Jump(condition_start),
                        statement.id,
                        statement.span,
                    );
                }
                for continue_jump in loop_context.continue_jumps {
                    self.patch_jump(continue_jump, condition_start);
                }
                let end = self.instructions.len();
                self.patch_jump(end_jump, end);
                for break_jump in loop_context.break_jumps {
                    self.patch_jump(break_jump, end);
                }

                false
            }

            StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.emit_statement(initializer);

                let condition_start = self.instructions.len();
                self.emit_expr(condition);
                let end_jump = self.instructions.len();
                self.emit_source(
                    InstructionKind::JumpIfFalse(usize::MAX),
                    statement.id,
                    condition.span,
                );

                self.loops.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                self.emit_statements(body);
                let loop_context = self.loops.pop().expect("for loop context must exist");

                let update_start = self.instructions.len();
                for continue_jump in loop_context.continue_jumps {
                    self.patch_jump(continue_jump, update_start);
                }
                self.emit_statement(update);
                self.emit_source(
                    InstructionKind::Jump(condition_start),
                    statement.id,
                    statement.span,
                );

                let end = self.instructions.len();
                self.patch_jump(end_jump, end);
                for break_jump in loop_context.break_jumps {
                    self.patch_jump(break_jump, end);
                }

                false
            }

            StatementKind::Break => {
                let jump = self.instructions.len();
                self.emit_source(
                    InstructionKind::Jump(usize::MAX),
                    statement.id,
                    statement.span,
                );
                self.loops
                    .last_mut()
                    .expect("semantic analysis rejects break outside a loop")
                    .break_jumps
                    .push(jump);
                true
            }

            StatementKind::Continue => {
                let jump = self.instructions.len();
                self.emit_source(
                    InstructionKind::Jump(usize::MAX),
                    statement.id,
                    statement.span,
                );
                self.loops
                    .last_mut()
                    .expect("semantic analysis rejects continue outside a loop")
                    .continue_jumps
                    .push(jump);
                true
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::ArrayCopy { .. } | ExprKind::ArrayRepeat { .. } => {
                unreachable!("copies are expanded in common IR")
            }
            ExprKind::ArrayAllocate {
                length,
                element_width,
            } => {
                let ir::Type::DynamicArray { element } = &expr.ty else {
                    unreachable!()
                };
                self.emit_expr(length);
                self.emit_source(
                    InstructionKind::ArrayAllocate {
                        element: element.as_ref().clone().into(),
                        element_width: *element_width,
                    },
                    expr.id,
                    expr.span,
                );
            }
            ExprKind::ArrayReleaseOwner { value } => {
                self.emit_expr(value);
                self.emit_source(InstructionKind::ArrayReleaseOwner, expr.id, expr.span);
            }
            ExprKind::Let { .. } | ExprKind::Conditional { .. } => {
                unreachable!("match expressions are lowered before code generation")
            }
            ExprKind::Constant { value, .. } => self.emit_expr(value),
            ExprKind::ArrayLength { value } => {
                self.emit_expr(value);
                let op = match &value.ty {
                    ir::Type::Array { element, length } => InstructionKind::ArrayLength {
                        element: element.as_ref().clone().into(),
                        length: *length,
                    },
                    ir::Type::DynamicArray { element } => InstructionKind::DynamicArrayLength {
                        element: element.as_ref().clone().into(),
                    },
                    _ => unreachable!(),
                };
                self.emit_source(op, expr.id, expr.span);
            }
            ExprKind::StringConcat { left, right } => {
                self.emit_expr(left);
                self.emit_expr(right);
                self.emit_source(InstructionKind::StringConcat, expr.id, expr.span);
            }
            ExprKind::StringByteLength { value } => {
                self.emit_expr(value);
                self.emit_source(InstructionKind::StringByteLength, expr.id, expr.span);
            }
            ExprKind::ConvertNumeric {
                value,
                from,
                to,
                mode,
                ..
            } => {
                self.emit_expr(value);
                self.emit_source(
                    InstructionKind::ConvertNumeric {
                        mode: *mode,
                        from: *from,
                        to: *to,
                    },
                    expr.id,
                    expr.span,
                );
            }
            ExprKind::ConvertInteger {
                value, from, to, ..
            } => {
                self.emit_expr(value);
                self.emit_source(
                    InstructionKind::ConvertInteger {
                        from: *from,
                        to: *to,
                    },
                    expr.id,
                    expr.span,
                );
            }
            ExprKind::Boolean(value) => {
                self.emit_source(InstructionKind::PushBool(*value), expr.id, expr.span);
            }

            ExprKind::Integer(value) => {
                self.emit_source(
                    InstructionKind::PushInteger(
                        *value,
                        match expr.ty {
                            ir::Type::Integer(ty) => ty,
                            _ => unreachable!(),
                        },
                    ),
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::Float { text } => match expr.ty {
                ir::Type::F32 => {
                    let value = text
                        .parse::<f32>()
                        .expect("validated floating-point literal");

                    self.emit_source(InstructionKind::PushF32(value), expr.id, expr.span);
                }

                ir::Type::F64 => {
                    let value = text
                        .parse::<f64>()
                        .expect("validated floating-point literal");

                    self.emit_source(InstructionKind::PushF64(value), expr.id, expr.span);
                }

                ir::Type::Integer(_) => {
                    unreachable!("integer cannot be emitted as float");
                }

                ir::Type::Bool => {
                    unreachable!("boolean cannot be emitted as float");
                }
                ir::Type::String
                | ir::Type::Named(_)
                | ir::Type::DynamicArray { .. }
                | ir::Type::Array { .. } => {
                    unreachable!("a float literal must have a numeric type");
                }
            },

            ExprKind::Variable { id, .. } => {
                let slot = self
                    .slot_map
                    .get(id)
                    .copied()
                    .expect("variable must have a bytecode slot");

                self.emit_source(InstructionKind::Load(slot), expr.id, expr.span);
            }

            ExprKind::String(value) => {
                self.emit_source(
                    InstructionKind::PushString(value.clone()),
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::Construct {
                type_id,
                base,
                fields,
                ..
            } => {
                if let Some(base) = base {
                    self.emit_expr(base);
                }
                for field in fields {
                    self.emit_expr(&field.value);
                }
                self.emit_source(
                    InstructionKind::Construct {
                        type_id: type_id.0,
                        has_base: base.is_some(),
                        fields: fields
                            .iter()
                            .map(|field| ConstructField {
                                field_id: field.id.0,
                                origin: match field.origin {
                                    ir::FieldValueOrigin::Generated { .. } => {
                                        FieldOrigin::Generated
                                    }
                                    ir::FieldValueOrigin::Explicit { .. } => FieldOrigin::Explicit,
                                    ir::FieldValueOrigin::Default { .. } => FieldOrigin::Default,
                                },
                            })
                            .collect(),
                    },
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::FieldAccess {
                type_id,
                field_id,
                base,
                ..
            } => {
                self.emit_expr(base);
                self.emit_source(
                    InstructionKind::FieldGet {
                        type_id: type_id.0,
                        field_id: field_id.0,
                    },
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::Array(values) => {
                for value in values {
                    self.emit_expr(value);
                }
                let ir::Type::Array { element, length } = &expr.ty else {
                    unreachable!("array expressions must have an array type")
                };
                self.emit_source(
                    InstructionKind::ConstructArray {
                        element: (**element).clone().into(),
                        length: *length,
                    },
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::Index { base, index } => {
                self.emit_expr(base);
                self.emit_expr(index);
                let op = match &base.ty {
                    ir::Type::Array { element, length } => InstructionKind::Index {
                        element: element.as_ref().clone().into(),
                        length: *length,
                    },
                    ir::Type::DynamicArray { element } => InstructionKind::DynamicIndex {
                        element: element.as_ref().clone().into(),
                    },
                    _ => unreachable!(),
                };
                self.emit_source(op, expr.id, expr.span);
            }

            ExprKind::Call {
                function_id,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.emit_expr(argument);
                }
                self.emit_source(
                    InstructionKind::Call {
                        function_id: function_id.0,
                        argument_count: arguments.len(),
                    },
                    expr.id,
                    expr.span,
                );
            }

            ExprKind::Unary { op, value } => {
                self.emit_expr(value);

                match *op {
                    UnaryOp::Negate => {
                        self.emit_source(
                            InstructionKind::Negate(expr.ty.clone().into()),
                            expr.id,
                            expr.span,
                        );
                    }
                    UnaryOp::BitNot => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        self.emit_source(InstructionKind::BitNot(ty), expr.id, expr.span);
                    }
                    UnaryOp::Not => {
                        self.emit_source(InstructionKind::Not, expr.id, expr.span);
                    }
                }
            }

            ExprKind::Logical { op, left, right } => {
                self.emit_expr(left);
                let false_jump = self.instructions.len();
                self.emit_source(InstructionKind::JumpIfFalse(usize::MAX), expr.id, left.span);
                match op {
                    crate::ir::LogicalOp::And => self.emit_expr(right),
                    crate::ir::LogicalOp::Or => {
                        self.emit_source(InstructionKind::PushBool(true), expr.id, expr.span)
                    }
                }
                let end_jump = self.instructions.len();
                self.emit_source(InstructionKind::Jump(usize::MAX), expr.id, expr.span);
                self.patch_jump(false_jump, self.instructions.len());
                match op {
                    crate::ir::LogicalOp::And => {
                        self.emit_source(InstructionKind::PushBool(false), expr.id, expr.span)
                    }
                    crate::ir::LogicalOp::Or => self.emit_expr(right),
                }
                self.patch_jump(end_jump, self.instructions.len());
            }
            ExprKind::Binary { op, left, right } => {
                self.emit_expr(left);
                self.emit_expr(right);

                let instruction = match *op {
                    BinaryOp::Add => InstructionKind::Add(expr.ty.clone().into()),
                    BinaryOp::Subtract => InstructionKind::Subtract(expr.ty.clone().into()),
                    BinaryOp::Multiply => InstructionKind::Multiply(expr.ty.clone().into()),
                    BinaryOp::Divide => InstructionKind::Divide(expr.ty.clone().into()),
                    BinaryOp::Remainder => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::Remainder(ty)
                    }
                    BinaryOp::BitAnd => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::BitAnd(ty)
                    }
                    BinaryOp::BitOr => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::BitOr(ty)
                    }
                    BinaryOp::BitXor => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::BitXor(ty)
                    }
                    BinaryOp::ShiftLeft => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::ShiftLeft(ty)
                    }
                    BinaryOp::ShiftRight => {
                        let ir::Type::Integer(ty) = expr.ty else {
                            unreachable!()
                        };
                        InstructionKind::ShiftRight(ty)
                    }
                    BinaryOp::Equal => InstructionKind::Equal(left.ty.clone().into()),
                    BinaryOp::NotEqual => InstructionKind::NotEqual(left.ty.clone().into()),
                    BinaryOp::Less => InstructionKind::Less(left.ty.clone().into()),
                    BinaryOp::LessEqual => InstructionKind::LessEqual(left.ty.clone().into()),
                    BinaryOp::Greater => InstructionKind::Greater(left.ty.clone().into()),
                    BinaryOp::GreaterEqual => InstructionKind::GreaterEqual(left.ty.clone().into()),
                };

                self.emit_source(instruction, expr.id, expr.span);
            }
        }
    }

    fn emit_source(&mut self, kind: InstructionKind, node_id: NodeId, span: Span) {
        self.instructions
            .push(Instruction::source(kind, node_id, span));
    }

    fn patch_jump(&mut self, index: usize, target: usize) {
        match &mut self.instructions[index].kind {
            InstructionKind::JumpIfFalse(current) | InstructionKind::Jump(current) => {
                *current = target;
            }
            _ => unreachable!("only jump instructions are patched"),
        }
    }
}

fn collect_slots(
    statements: &[Statement],
    slots: &mut Vec<Slot>,
    slot_map: &mut HashMap<BindingId, usize>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding {
                id,
                mutable,
                name,
                ty,
                ..
            } => {
                let index = slots.len();
                slots.push(Slot {
                    name: name.clone(),
                    ty: ty.clone().into(),
                    mutable: *mutable,
                });
                slot_map.insert(*id, index);
            }
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_slots(then_body, slots, slot_map);
                collect_slots(else_body, slots, slot_map);
            }
            StatementKind::While { body, .. } => {
                collect_slots(body, slots, slot_map);
            }
            StatementKind::For {
                initializer, body, ..
            } => {
                collect_slots(std::slice::from_ref(initializer), slots, slot_map);
                collect_slots(body, slots, slot_map);
            }
            StatementKind::ArrayInitialize { .. }
            | StatementKind::ArrayRetain { .. }
            | StatementKind::ArrayFree { .. }
            | StatementKind::ArrayRangeCheck { .. }
            | StatementKind::Assignment { .. }
            | StatementKind::StringManage { .. }
            | StatementKind::Write { .. }
            | StatementKind::Print { .. }
            | StatementKind::Call { .. }
            | StatementKind::Return { .. }
            | StatementKind::Break
            | StatementKind::Continue => {}
        }
    }
}

impl From<ir::Type> for Type {
    fn from(value: ir::Type) -> Self {
        match value {
            ir::Type::DynamicArray { element } => Self::DynamicArray {
                element: Box::new((*element).into()),
            },
            ir::Type::Bool => Self::Bool,
            ir::Type::String => Self::String,
            ir::Type::Integer(ty) => Self::Integer(ty),
            ir::Type::F32 => Self::F32,
            ir::Type::F64 => Self::F64,
            ir::Type::Named(id) => Self::Named(id.0),
            ir::Type::Array { element, length } => Self::Array {
                element: Box::new((*element).into()),
                length,
            },
        }
    }
}

fn format_instruction(
    instruction: &InstructionKind,
    program: &BytecodeProgram,
    slots: &[Slot],
    output: &mut String,
) {
    match instruction {
        InstructionKind::DynamicArrayLength { element } => {
            writeln!(output, "array.len.dynamic {}", type_name(element, program)).unwrap();
        }
        InstructionKind::DynamicIndex { element } => {
            writeln!(
                output,
                "array.index.dynamic {}",
                type_name(element, program)
            )
            .unwrap();
        }
        InstructionKind::ArrayAllocate {
            element,
            element_width,
        } => {
            writeln!(
                output,
                "array.allocate-elements {} width={element_width}",
                type_name(element, program)
            )
            .unwrap();
        }
        InstructionKind::ArrayInitialize => {
            writeln!(output, "array.initialize-next").unwrap();
        }
        InstructionKind::ArrayRetain => {
            writeln!(output, "array.retain-owner").unwrap();
        }
        InstructionKind::ArrayFree => {
            writeln!(output, "array.free-elements").unwrap();
        }
        InstructionKind::ArrayReleaseOwner => {
            writeln!(output, "array.release-owner-last").unwrap();
        }
        InstructionKind::ArrayRangeCheck => {
            writeln!(output, "array.check-range").unwrap();
        }
        InstructionKind::PushBool(value) => {
            writeln!(output, "push.bool {value}").unwrap();
        }
        InstructionKind::ArrayLength { element, length } => {
            writeln!(
                output,
                "array_len [{}; {length}]",
                type_name(element, program)
            )
            .unwrap();
        }
        InstructionKind::StringConcat => {
            writeln!(output, "string.concat.allocate-copy").unwrap();
        }
        InstructionKind::StringManage { retain } => {
            writeln!(
                output,
                "string.{}",
                if *retain { "retain" } else { "release" }
            )
            .unwrap();
        }
        InstructionKind::StringByteLength => {
            writeln!(output, "byte_len.string").unwrap();
        }
        InstructionKind::PushString(value) => {
            writeln!(output, "push.string {value:?}").unwrap();
        }

        InstructionKind::PushInteger(value, integer) => {
            writeln!(output, "push.{} {value}", integer.name()).unwrap();
        }

        InstructionKind::PushF32(value) => {
            writeln!(output, "push.f32 {value}").unwrap();
        }

        InstructionKind::PushF64(value) => {
            writeln!(output, "push.f64 {value}").unwrap();
        }

        InstructionKind::Load(slot) => {
            writeln!(output, "load {slot}        ; {}", slots[*slot].name).unwrap();
        }

        InstructionKind::Store(slot) => {
            writeln!(output, "store {slot}       ; {}", slots[*slot].name).unwrap();
        }

        InstructionKind::Assign(slot) => {
            writeln!(output, "assign {slot}      ; {}", slots[*slot].name).unwrap();
        }

        InstructionKind::ArrayAssign { slot, path } => {
            write!(output, "array.assign {slot} [").unwrap();
            for (index, access) in path.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write!(
                    output,
                    "{}; {}",
                    type_name(&access.element, program),
                    access
                        .length
                        .map_or_else(|| "dynamic".into(), |n| n.to_string())
                )
                .unwrap();
            }
            writeln!(output, "] ; {}", slots[*slot].name).unwrap();
        }

        InstructionKind::ArrayCheck { slot, path } => {
            write!(output, "array.check {slot} [").unwrap();
            for (index, access) in path.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write!(
                    output,
                    "{}; {}",
                    type_name(&access.element, program),
                    access
                        .length
                        .map_or_else(|| "dynamic".into(), |n| n.to_string())
                )
                .unwrap();
            }
            writeln!(output, "] ; {}", slots[*slot].name).unwrap();
        }

        InstructionKind::Construct {
            type_id,
            fields,
            has_base,
        } => {
            write!(
                output,
                "{} {} [",
                if *has_base {
                    "construct.from_base"
                } else {
                    "construct"
                },
                type_name(&Type::Named(*type_id), program)
            )
            .unwrap();
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                let definition = &program.type_definitions[*type_id];
                let origin = match field.origin {
                    FieldOrigin::Generated => "generated",
                    FieldOrigin::Explicit => "explicit",
                    FieldOrigin::Default => "default",
                };
                write!(
                    output,
                    "%{}@{}:{origin}",
                    definition.fields[field.field_id].name, field.field_id
                )
                .unwrap();
            }
            writeln!(output, "]").unwrap();
        }

        InstructionKind::FieldGet { type_id, field_id } => {
            let definition = &program.type_definitions[*type_id];
            writeln!(
                output,
                "field.get %{}@{}.%{}@{}",
                definition.name, type_id, definition.fields[*field_id].name, field_id
            )
            .unwrap();
        }

        InstructionKind::ConstructArray { element, length } => {
            writeln!(output, "array.new {} {length}", type_name(element, program)).unwrap();
        }

        InstructionKind::Index { element, length } => {
            writeln!(output, "array.get {} {length}", type_name(element, program)).unwrap();
        }

        InstructionKind::Call {
            function_id,
            argument_count,
        } => {
            writeln!(
                output,
                "call {function_id} {argument_count}  ; {}",
                program.functions[*function_id].name
            )
            .unwrap();
        }

        InstructionKind::Return { has_value: true } => {
            writeln!(output, "return.value").unwrap();
        }

        InstructionKind::Return { has_value: false } => {
            writeln!(output, "return").unwrap();
        }

        InstructionKind::Add(ty) => {
            writeln!(output, "add.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Subtract(ty) => {
            writeln!(output, "sub.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Multiply(ty) => {
            writeln!(output, "mul.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Divide(ty) => {
            writeln!(output, "div.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Equal(ty) => {
            writeln!(output, "eq.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::NotEqual(ty) => {
            writeln!(output, "ne.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Less(ty) => {
            writeln!(output, "lt.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::LessEqual(ty) => {
            writeln!(output, "le.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Greater(ty) => {
            writeln!(output, "gt.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::GreaterEqual(ty) => {
            writeln!(output, "ge.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::ConvertNumeric { from, to, mode } => {
            writeln!(
                output,
                "convert.{} {} -> {}",
                mode.name(),
                from.name(),
                to.name()
            )
            .unwrap();
        }
        InstructionKind::ConvertInteger { from, to } => {
            writeln!(output, "convert.checked {} -> {}", from.name(), to.name()).unwrap();
        }
        InstructionKind::Negate(ty) => {
            writeln!(output, "neg.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::Remainder(ty) => {
            writeln!(output, "rem.{}", ty.name()).unwrap();
        }
        InstructionKind::BitAnd(ty) => {
            writeln!(output, "bit_and.{}", ty.name()).unwrap();
        }
        InstructionKind::BitOr(ty) => {
            writeln!(output, "bit_or.{}", ty.name()).unwrap();
        }
        InstructionKind::BitXor(ty) => {
            writeln!(output, "bit_xor.{}", ty.name()).unwrap();
        }
        InstructionKind::ShiftLeft(ty) => {
            writeln!(output, "shl.checked.{}", ty.name()).unwrap();
        }
        InstructionKind::ShiftRight(ty) => {
            writeln!(output, "shr.{}", ty.name()).unwrap();
        }
        InstructionKind::BitNot(ty) => {
            writeln!(output, "bit_not.{}", ty.name()).unwrap();
        }
        InstructionKind::Not => {
            writeln!(output, "not.bool").unwrap();
        }

        InstructionKind::Write { ty, quoted } => {
            writeln!(
                output,
                "write{} {}",
                if *quoted { ".quoted" } else { "" },
                type_name(ty, program)
            )
            .unwrap();
        }
        InstructionKind::Print(ty) => {
            writeln!(output, "print.{}", type_name(ty, program),).unwrap();
        }

        InstructionKind::JumpIfFalse(target) => {
            writeln!(output, "jump-if-false {target:04}").unwrap();
        }

        InstructionKind::Jump(target) => {
            writeln!(output, "jump {target:04}").unwrap();
        }

        InstructionKind::Halt => {
            writeln!(output, "halt").unwrap();
        }
    }
}

fn array_access(projection: &ir::AssignmentProjection) -> ArrayAccess {
    match projection {
        ir::AssignmentProjection::Index {
            element, length, ..
        } => ArrayAccess {
            element: element.clone().into(),
            length: Some(*length),
        },
        ir::AssignmentProjection::DynamicIndex { element, .. } => ArrayAccess {
            element: element.clone().into(),
            length: None,
        },
    }
}

fn type_name(ty: &Type, program: &BytecodeProgram) -> String {
    match ty {
        Type::DynamicArray { element } => format!("[{}]", type_name(element, program)),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Integer(ty) => ty.name().into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Named(id) => format!("%{}@{id}", program.type_definitions[*id].name),
        Type::Array { element, length } => {
            format!("[{}; {length}]", type_name(element, program))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::IntegerType;
    use crate::{
        compile_to_bytecode, compile_to_ir,
        ir::{NodeId, StatementKind},
        source::Span,
    };

    use super::{BytecodeProgram, InstructionKind, InstructionOrigin, Type, format_program, lower};

    #[test]
    fn emits_typed_bytecode() {
        let program = compile_to_ir("x: f32 = 0.1 + 0.2; print(x);").unwrap();

        let bytecode = lower(&program).unwrap();

        let text = format_program(&bytecode);

        assert!(text.contains("push.f32"));
        assert!(text.contains("add.f32"));
        assert!(text.contains("store 0"));
        assert!(text.contains("print.f32"));
        assert!(text.contains("halt"));
    }

    #[test]
    fn records_source_and_synthetic_instruction_origins() {
        let bytecode = compile_to_bytecode("print(1 / 0);").unwrap();

        assert!(matches!(
            bytecode.instructions[0].kind,
            InstructionKind::PushInteger(1, IntegerType::I64)
        ));
        assert!(matches!(
            bytecode.instructions[1].kind,
            InstructionKind::PushInteger(0, IntegerType::I64)
        ));
        assert!(matches!(
            bytecode.instructions[2].kind,
            InstructionKind::Divide(Type::Integer(IntegerType::I64))
        ));
        assert!(matches!(
            bytecode.instructions[3].kind,
            InstructionKind::Print(Type::Integer(IntegerType::I64))
        ));
        assert!(matches!(
            bytecode.instructions[4].kind,
            InstructionKind::Halt
        ));

        let origins: Vec<_> = bytecode
            .instructions
            .iter()
            .map(|instruction| instruction.origin)
            .collect();

        assert_eq!(
            origins,
            vec![
                InstructionOrigin::Source {
                    node_id: NodeId(2),
                    span: Span::new(6, 7),
                },
                InstructionOrigin::Source {
                    node_id: NodeId(3),
                    span: Span::new(10, 11),
                },
                InstructionOrigin::Source {
                    node_id: NodeId(1),
                    span: Span::new(6, 11),
                },
                InstructionOrigin::Source {
                    node_id: NodeId(0),
                    span: Span::new(0, 13),
                },
                InstructionOrigin::Synthetic,
            ]
        );
    }

    #[test]
    fn instruction_origins_are_deterministic() {
        let first = compile_to_bytecode("value: i64 = 1; print(value);").unwrap();
        let second = compile_to_bytecode("value: i64 = 1; print(value);").unwrap();

        let origins = |program: &BytecodeProgram| {
            program
                .instructions
                .iter()
                .map(|instruction| instruction.origin)
                .collect::<Vec<_>>()
        };

        assert_eq!(origins(&first), origins(&second));
    }

    #[test]
    fn one_array_assignment_node_maps_to_each_check_and_the_write() {
        let source = "mut matrix: [[i64; 2]; 2] = [[1, 2], [3, 4]]; matrix[0][1] = 9;";
        let ir = compile_to_ir(source).unwrap();
        let statement = &ir.statements[1];
        let StatementKind::Assignment { target, .. } = &statement.kind else {
            panic!("expected assignment");
        };
        let projection_spans = target
            .projections
            .iter()
            .map(|projection| match projection {
                crate::ir::AssignmentProjection::Index { span, .. }
                | crate::ir::AssignmentProjection::DynamicIndex { span, .. } => *span,
            })
            .collect::<Vec<_>>();

        let bytecode = lower(&ir).unwrap();
        let origins = bytecode
            .instructions
            .iter()
            .filter_map(|instruction| match &instruction.kind {
                InstructionKind::ArrayCheck { .. } | InstructionKind::ArrayAssign { .. } => {
                    Some(instruction.origin)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            origins,
            vec![
                InstructionOrigin::Source {
                    node_id: statement.id,
                    span: projection_spans[0],
                },
                InstructionOrigin::Source {
                    node_id: statement.id,
                    span: projection_spans[1],
                },
                InstructionOrigin::Source {
                    node_id: statement.id,
                    span: statement.span,
                },
            ]
        );
    }

    #[test]
    fn emits_all_typed_comparisons() {
        let program = compile_to_bytecode(
            "a: bool = 1 == 1; b: bool = 1 != 2; c: bool = 1 < 2;
             d: bool = 1 <= 2; e: bool = 2 > 1; f: bool = 2 >= 1;",
        )
        .unwrap();
        let text = format_program(&program);

        for instruction in ["eq.i64", "ne.i64", "lt.i64", "le.i64", "gt.i64", "ge.i64"] {
            assert!(text.contains(instruction));
        }
    }

    #[test]
    fn exposes_function_frames_calls_and_returns() {
        let program = compile_to_bytecode(
            "fn add(left: i64, right: i64) -> i64 { return left + right; }
             answer: i64 = add(20, 22);",
        )
        .unwrap();
        let text = format_program(&program);

        assert!(text.contains(".function 0 add(0:left:i64, 1:right:i64) -> i64"));
        assert!(text.contains("0003  return.value"));
        assert!(text.contains("call 0 2  ; add"));
    }
}
