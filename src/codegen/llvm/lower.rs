use std::collections::HashMap;

use crate::ir as cerune_ir;

use super::ir::{
    BinaryOp, CompareOp, Function, Instruction, Label, LocatedInstruction, Module, Operand, Origin,
    Parameter, Slot, SlotId, Temp, Type, TypeDefinition,
};

pub fn lower(program: &cerune_ir::Program) -> Module {
    let mut strings = Vec::new();
    let functions = program
        .function_definitions
        .iter()
        .map(|function| lower_function(function, &mut strings))
        .collect();
    let mut slots = Vec::new();
    let mut slot_map = HashMap::new();
    let mut name_counts = HashMap::new();
    collect_slots(
        &program.statements,
        &mut slots,
        &mut slot_map,
        &mut name_counts,
    );

    let mut lowerer = Lowerer {
        strings: &mut strings,
        slots,
        slot_map,
        instructions: Vec::new(),
        origin: Origin::Synthetic,
        temp: 0,
        label: 0,
        loops: Vec::new(),
    };

    lowerer.lower_statements(&program.statements);

    Module {
        array_heap_limit: program
            .first_dynamic_array_span()
            .map(|_| program.array_heap_limit),
        string_heap_limit: crate::codegen::support::string_heap_limit(program),
        target: None,
        uses_strings: crate::codegen::support::first_string_span(program).is_some(),
        type_definitions: program
            .type_definitions
            .iter()
            .map(|definition| TypeDefinition {
                id: definition.id.0,
                name: definition.name.clone(),
                fields: definition
                    .fields
                    .iter()
                    .map(|field| field.ty.clone().into())
                    .collect(),
            })
            .collect(),
        functions,
        explicit_main: program
            .function_definitions
            .iter()
            .find(|function| function.name == "main")
            .map(|function| function.id.0),
        slots: lowerer.slots,
        instructions: lowerer.instructions,
        strings,
    }
}

fn lower_function(function: &cerune_ir::FunctionDefinition, strings: &mut Vec<String>) -> Function {
    let mut slots = Vec::new();
    let mut slot_map = HashMap::new();
    let mut name_counts = HashMap::new();
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| {
            let slot = SlotId(slots.len());
            slots.push(Slot {
                name: parameter.name.clone(),
                ty: parameter.ty.clone().into(),
            });
            slot_map.insert(parameter.id, slot);
            name_counts.insert(parameter.name.clone(), 1);
            Parameter {
                name: parameter.name.clone(),
                ty: parameter.ty.clone().into(),
                slot,
            }
        })
        .collect();
    collect_slots(&function.body, &mut slots, &mut slot_map, &mut name_counts);

    let mut lowerer = Lowerer {
        strings,
        slots,
        slot_map,
        instructions: Vec::new(),
        origin: Origin::Synthetic,
        temp: 0,
        label: 0,
        loops: Vec::new(),
    };
    let terminates = lowerer.lower_statements(&function.body);
    let return_type = match &function.return_type {
        cerune_ir::ReturnType::Void => None,
        cerune_ir::ReturnType::Value(ty) => Some(ty.clone().into()),
    };
    if !terminates {
        lowerer.push(Instruction::Return { value: None });
    }

    Function {
        id: function.id.0,
        name: function.name.clone(),
        parameters,
        return_type,
        slots: lowerer.slots,
        instructions: lowerer.instructions,
    }
}

struct Lowerer<'a> {
    origin: Origin,
    strings: &'a mut Vec<String>,
    slots: Vec<Slot>,
    slot_map: HashMap<cerune_ir::BindingId, SlotId>,
    instructions: Vec<LocatedInstruction>,
    temp: usize,
    label: usize,
    loops: Vec<LoopContext>,
}

#[derive(Debug, Clone, Copy)]
struct LoopContext {
    continue_label: Label,
    break_label: Label,
}

#[derive(Debug, Clone)]
struct Value {
    ty: Type,
    operand: Operand,
}

impl Lowerer<'_> {
    fn push(&mut self, instruction: Instruction) {
        self.instructions.push(LocatedInstruction {
            instruction,
            origin: self.origin,
        });
    }

    fn lower_statement(&mut self, statement: &cerune_ir::Statement) -> bool {
        let previous = self.origin;
        self.origin = Origin::Source {
            node_id: statement.id,
            span: statement.span,
        };
        let terminates = self.lower_statement_body(statement);
        self.origin = previous;
        terminates
    }

    fn lower_expr(&mut self, expr: &cerune_ir::Expr) -> Value {
        let previous = self.origin;
        self.origin = Origin::Source {
            node_id: expr.id,
            span: expr.span,
        };
        let value = self.lower_expr_value(expr);
        self.origin = previous;
        value
    }

    fn lower_statements(&mut self, statements: &[cerune_ir::Statement]) -> bool {
        for statement in statements {
            if self.lower_statement(statement) {
                return true;
            }
        }

        false
    }

    fn lower_statement_body(&mut self, statement: &cerune_ir::Statement) -> bool {
        match &statement.kind {
            cerune_ir::StatementKind::ArrayInitialize { array, value } => {
                let array = self.lower_expr(array);
                let value = self.lower_expr(value);
                let scratch = self.next_temp();
                self.push(Instruction::ArrayInitialize {
                    scratch,
                    element: value.ty,
                    array: array.operand,
                    value: value.operand,
                });
                false
            }
            cerune_ir::StatementKind::ArrayRetain { value }
            | cerune_ir::StatementKind::ArrayFree { value } => {
                let value = self.lower_expr(value).operand;
                self.push(Instruction::ArrayManage {
                    value,
                    retain: matches!(statement.kind, cerune_ir::StatementKind::ArrayRetain { .. }),
                });
                false
            }
            cerune_ir::StatementKind::ArrayRangeCheck { length, start, end } => {
                let length = self.lower_expr(length).operand;
                let start = self.lower_expr(start).operand;
                let end = self.lower_expr(end).operand;
                self.push(Instruction::ArrayRangeCheck { length, start, end });
                false
            }
            cerune_ir::StatementKind::Binding { id, ty, value, .. } => {
                let slot = self.slot(*id);
                let ty = ty.clone().into();

                let value = self.lower_expr(value);

                self.push(Instruction::Store {
                    ty,
                    value: value.operand,
                    slot,
                });
                false
            }

            cerune_ir::StatementKind::Assignment { target, value } => {
                let slot = self.slot(target.id);
                let root_ty: Type = target.root_ty.clone().into();
                if target.projections.is_empty() {
                    let value = self.lower_expr(value);
                    self.push(Instruction::Store {
                        ty: root_ty,
                        value: value.operand,
                        slot,
                    });
                    return false;
                }

                let root = self.next_temp();
                self.push(Instruction::Load {
                    dest: root,
                    ty: root_ty.clone(),
                    slot,
                });
                let mut current = Value {
                    ty: root_ty.clone(),
                    operand: Operand::Temp(root),
                };
                let mut parents = Vec::with_capacity(target.projections.len());
                for projection in &target.projections {
                    let (index, element, length, span) = match projection {
                        cerune_ir::AssignmentProjection::Index {
                            index,
                            element,
                            length,
                            span,
                        } => (index, element, Some(*length), span),
                        cerune_ir::AssignmentProjection::DynamicIndex {
                            index,
                            element,
                            span,
                        } => (index, element, None, span),
                    };
                    let index = self.lower_expr(index);
                    let element: Type = element.clone().into();
                    let dest = self.next_temp();
                    let previous = self.origin;
                    self.origin = Origin::Source {
                        node_id: statement.id,
                        span: *span,
                    };
                    self.push(Instruction::ArrayGet {
                        dest,
                        element: element.clone(),
                        length,
                        array: current.operand,
                        index: index.operand,
                    });
                    self.origin = previous;
                    parents.push((
                        current.operand,
                        index.operand,
                        element.clone(),
                        length,
                        *span,
                    ));
                    current = Value {
                        ty: element,
                        operand: Operand::Temp(dest),
                    };
                }

                let mut updated = self.lower_expr(value).operand;
                for (array, index, element, length, span) in parents.into_iter().rev() {
                    let dest = self.next_temp();
                    let previous = self.origin;
                    self.origin = Origin::Source {
                        node_id: statement.id,
                        span,
                    };
                    self.push(Instruction::ArraySet {
                        dest,
                        element: element.clone(),
                        length,
                        array,
                        index,
                        value: updated,
                    });
                    self.origin = previous;
                    updated = Operand::Temp(dest);
                }
                self.push(Instruction::Store {
                    ty: root_ty,
                    value: updated,
                    slot,
                });
                false
            }

            cerune_ir::StatementKind::StringManage { value, retain } => {
                let value = self.lower_expr(value);
                self.push(Instruction::StringManage {
                    value: value.operand,
                    retain: *retain,
                });
                false
            }
            cerune_ir::StatementKind::Write { value, quoted } => {
                let kind = crate::codegen::display::kind(&value.ty, *quoted);
                let value = self.lower_expr(value);
                self.push(Instruction::Output {
                    kind,
                    value: value.operand,
                    newline: false,
                });
                false
            }
            cerune_ir::StatementKind::Flush => {
                self.push(Instruction::Flush);
                false
            }
            cerune_ir::StatementKind::Print { value } => {
                let kind = crate::codegen::display::kind(&value.ty, false);
                let value = self.lower_expr(value);
                self.push(Instruction::Output {
                    kind,
                    value: value.operand,
                    newline: true,
                });
                false
            }

            cerune_ir::StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.lower_expr(condition);
                let then_label = self.next_label();
                let else_label = self.next_label();
                let end_label = self.next_label();

                self.push(Instruction::Branch {
                    condition: condition.operand,
                    then_label,
                    else_label: if else_body.is_empty() {
                        end_label
                    } else {
                        else_label
                    },
                });
                self.push(Instruction::Label {
                    id: then_label,
                    name: "if_then",
                });
                let then_terminates = self.lower_statements(then_body);
                if !then_terminates {
                    self.push(Instruction::Jump { label: end_label });
                }

                if else_body.is_empty() {
                    self.push(Instruction::Label {
                        id: end_label,
                        name: "if_end",
                    });
                    false
                } else {
                    self.push(Instruction::Label {
                        id: else_label,
                        name: "if_else",
                    });
                    let else_terminates = self.lower_statements(else_body);
                    if !else_terminates {
                        self.push(Instruction::Jump { label: end_label });
                    }

                    if then_terminates && else_terminates {
                        true
                    } else {
                        self.push(Instruction::Label {
                            id: end_label,
                            name: "if_end",
                        });
                        false
                    }
                }
            }

            cerune_ir::StatementKind::While { condition, body } => {
                let condition_label = self.next_label();
                let body_label = self.next_label();
                let end_label = self.next_label();

                self.push(Instruction::Jump {
                    label: condition_label,
                });
                self.push(Instruction::Label {
                    id: condition_label,
                    name: "while_condition",
                });

                let condition = self.lower_expr(condition);
                self.push(Instruction::Branch {
                    condition: condition.operand,
                    then_label: body_label,
                    else_label: end_label,
                });
                self.push(Instruction::Label {
                    id: body_label,
                    name: "while_body",
                });

                self.loops.push(LoopContext {
                    continue_label: condition_label,
                    break_label: end_label,
                });
                let body_terminates = self.lower_statements(body);
                self.loops.pop().expect("while loop context must exist");

                if !body_terminates {
                    self.push(Instruction::Jump {
                        label: condition_label,
                    });
                }
                self.push(Instruction::Label {
                    id: end_label,
                    name: "while_end",
                });
                false
            }

            cerune_ir::StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.lower_statement(initializer);

                let condition_label = self.next_label();
                let body_label = self.next_label();
                let update_label = self.next_label();
                let end_label = self.next_label();

                self.push(Instruction::Jump {
                    label: condition_label,
                });
                self.push(Instruction::Label {
                    id: condition_label,
                    name: "for_condition",
                });
                let condition = self.lower_expr(condition);
                self.push(Instruction::Branch {
                    condition: condition.operand,
                    then_label: body_label,
                    else_label: end_label,
                });
                self.push(Instruction::Label {
                    id: body_label,
                    name: "for_body",
                });

                self.loops.push(LoopContext {
                    continue_label: update_label,
                    break_label: end_label,
                });
                let body_terminates = self.lower_statements(body);
                self.loops.pop().expect("for loop context must exist");

                if !body_terminates {
                    self.push(Instruction::Jump {
                        label: update_label,
                    });
                }
                self.push(Instruction::Label {
                    id: update_label,
                    name: "for_update",
                });
                self.lower_statement(update);
                self.push(Instruction::Jump {
                    label: condition_label,
                });
                self.push(Instruction::Label {
                    id: end_label,
                    name: "for_end",
                });
                false
            }

            cerune_ir::StatementKind::Break => {
                let target = self
                    .loops
                    .last()
                    .expect("semantic analysis rejects break outside a loop")
                    .break_label;
                self.push(Instruction::Jump { label: target });
                true
            }

            cerune_ir::StatementKind::Continue => {
                let target = self
                    .loops
                    .last()
                    .expect("semantic analysis rejects continue outside a loop")
                    .continue_label;
                self.push(Instruction::Jump { label: target });
                true
            }
            cerune_ir::StatementKind::Call {
                function_id,
                arguments,
                ..
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        let value = self.lower_expr(argument);
                        (value.ty, value.operand)
                    })
                    .collect();
                self.push(Instruction::Call {
                    dest: None,
                    function_id: function_id.0,
                    return_type: None,
                    arguments,
                });
                false
            }
            cerune_ir::StatementKind::Return { value } => {
                let value = value.as_ref().map(|value| {
                    let value = self.lower_expr(value);
                    (value.ty, value.operand)
                });
                self.push(Instruction::Return { value });
                true
            }
        }
    }

    fn lower_expr_value(&mut self, expr: &cerune_ir::Expr) -> Value {
        let value = self.lower_expr_unchecked(expr);
        if let Some(ty) = super::super::integer_range_check(expr) {
            let dest = self.next_temp();
            self.push(Instruction::CheckIntegerRange {
                dest,
                value: value.operand,
                ty,
                failure: match expr.kind {
                    cerune_ir::ExprKind::ConvertInteger { .. } => {
                        crate::runtime::FailureCode::IntegerConversionOutOfRange
                    }
                    cerune_ir::ExprKind::Binary {
                        op: cerune_ir::BinaryOp::Divide,
                        ..
                    } => crate::runtime::FailureCode::DivisionOverflow,
                    _ => crate::runtime::FailureCode::IntegerOverflow,
                },
            });
            return Value {
                ty: Type::I64,
                operand: Operand::Temp(dest),
            };
        }
        value
    }

    fn lower_expr_unchecked(&mut self, expr: &cerune_ir::Expr) -> Value {
        if let Some((value, conversion)) = crate::codegen::u64_integer_conversion(expr) {
            let value = self.lower_expr(value);
            let dest = self.next_temp();
            self.push(Instruction::ConvertNumeric {
                dest,
                value: value.operand,
                conversion,
            });
            return Value {
                ty: Type::I64,
                operand: Operand::Temp(dest),
            };
        }

        match &expr.kind {
            cerune_ir::ExprKind::ArgumentCount | cerune_ir::ExprKind::ArgumentGet { .. } => {
                unreachable!("argument operations rejected before lowering")
            }
            cerune_ir::ExprKind::ArrayCopy { .. } | cerune_ir::ExprKind::ArrayRepeat { .. } => {
                unreachable!("array copies are expanded in common IR")
            }
            cerune_ir::ExprKind::ArrayAllocate {
                length,
                element_width,
            } => {
                let cerune_ir::Type::DynamicArray { element } = &expr.ty else {
                    unreachable!()
                };
                let length = self.lower_expr(length).operand;
                let dest = self.next_temp();
                self.push(Instruction::ArrayAllocate {
                    dest,
                    length,
                    element: (**element).clone().into(),
                    element_width: *element_width,
                });
                Value {
                    ty: expr.ty.clone().into(),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ArrayReleaseOwner { value } => {
                let value = self.lower_expr(value).operand;
                let dest = self.next_temp();
                self.push(Instruction::ArrayReleaseOwner { dest, value });
                Value {
                    ty: Type::Bool,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Let { .. } | cerune_ir::ExprKind::Conditional { .. } => {
                unreachable!("match expressions are lowered before code generation")
            }
            cerune_ir::ExprKind::Constant { value, .. } => self.lower_expr(value),
            cerune_ir::ExprKind::ArrayLength { value } => {
                let value = self.lower_expr(value);
                let operand = match &value.ty {
                    Type::Array { length, .. } => Operand::Integer(*length as i64),
                    Type::DynamicArray { .. } => {
                        let dest = self.next_temp();
                        self.push(Instruction::ExtractValue {
                            dest,
                            ty: value.ty,
                            aggregate: value.operand,
                            field: 1,
                        });
                        Operand::Temp(dest)
                    }
                    _ => unreachable!(),
                };
                Value {
                    ty: Type::I64,
                    operand,
                }
            }
            cerune_ir::ExprKind::StringConcat { left, right } => {
                let left = self.lower_expr(left).operand;
                let right = self.lower_expr(right).operand;
                let dest = self.next_temp();
                self.push(Instruction::StringConcat { dest, left, right });
                Value {
                    ty: Type::String,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ReadLine { max_bytes } => {
                let max_bytes = self.lower_expr(max_bytes).operand;
                let dest = self.next_temp();
                self.push(Instruction::ReadLine { dest, max_bytes });
                Value {
                    ty: Type::String,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::StringByteLength { value } => {
                let value = self.lower_expr(value);
                let dest = self.next_temp();
                self.push(Instruction::ExtractValue {
                    dest,
                    ty: Type::String,
                    aggregate: value.operand,
                    field: 1,
                });
                Value {
                    ty: Type::I64,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::String(value) => {
                // 出現順のIDを使い、保存先や共有の有無に意味を持たせません。
                let id = self.strings.len();
                self.strings.push(value.clone());
                Value {
                    ty: Type::String,
                    operand: Operand::String {
                        id,
                        length: value.len(),
                    },
                }
            }
            cerune_ir::ExprKind::ConvertNumeric {
                value,
                from,
                to,
                mode,
                ..
            } => {
                let value = self.lower_expr(value);
                if from == to {
                    return value;
                }
                let dest = self.next_temp();
                self.push(Instruction::ConvertNumeric {
                    dest,
                    value: value.operand,
                    conversion: crate::codegen::NumericConversion {
                        mode: *mode,
                        from: *from,
                        to: *to,
                    },
                });
                Value {
                    ty: expr.ty.clone().into(),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ConvertInteger { value, .. } => self.lower_expr(value),
            cerune_ir::ExprKind::Boolean(value) => Value {
                ty: Type::Bool,
                operand: Operand::Boolean(*value),
            },

            cerune_ir::ExprKind::Integer(value) => Value {
                ty: Type::I64,
                operand: Operand::Integer(*value as i64),
            },

            cerune_ir::ExprKind::Float { text } => match expr.ty {
                cerune_ir::Type::DynamicArray { .. } => {
                    unreachable!("dynamic arrays are rejected before backend lowering")
                }
                cerune_ir::Type::String => {
                    unreachable!("a float literal cannot have string type")
                }
                cerune_ir::Type::F32 => {
                    let value = text
                        .parse::<f32>()
                        .expect("validated floating-point literal");

                    Value {
                        ty: Type::Float,
                        operand: Operand::Float32(value.to_bits()),
                    }
                }

                cerune_ir::Type::F64 => {
                    let value = text
                        .parse::<f64>()
                        .expect("validated floating-point literal");

                    Value {
                        ty: Type::Double,
                        operand: Operand::Float64(value.to_bits()),
                    }
                }

                cerune_ir::Type::Integer(_) => {
                    unreachable!("integer cannot be lowered as float")
                }

                cerune_ir::Type::Bool => {
                    unreachable!("boolean cannot be lowered as float")
                }
                cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. } => {
                    unreachable!("a float literal cannot have an aggregate type")
                }
            },

            cerune_ir::ExprKind::Variable { id, .. } => {
                let ty: Type = expr.ty.clone().into();
                let dest = self.next_temp();
                let slot = self.slot(*id);

                self.push(Instruction::Load {
                    dest,
                    ty: ty.clone(),
                    slot,
                });

                Value {
                    ty,
                    operand: Operand::Temp(dest),
                }
            }

            cerune_ir::ExprKind::Unary { op, value } => {
                let value = self.lower_expr(value);
                let dest = self.next_temp();

                match (*op, &value.ty) {
                    (cerune_ir::UnaryOp::BitNot, _) => {
                        self.push(Instruction::IntegerBinary {
                            dest,
                            op: crate::codegen::IntegerBinaryOp::BitXor,
                            ty: crate::codegen::integer_type(&expr.ty),
                            left: value.operand,
                            right: Operand::Integer(crate::codegen::complement_mask(&expr.ty)),
                        });
                    }
                    (cerune_ir::UnaryOp::Negate, Type::I64) => {
                        self.push(Instruction::Binary {
                            dest,
                            op: BinaryOp::CheckedI64Sub,
                            ty: Type::I64,
                            left: Operand::Integer(0),
                            right: value.operand,
                        });
                    }

                    (cerune_ir::UnaryOp::Negate, Type::Float | Type::Double) => {
                        self.push(Instruction::FNeg {
                            dest,
                            ty: value.ty.clone(),
                            value: value.operand,
                        });
                    }
                    (cerune_ir::UnaryOp::Not, Type::Bool) => {
                        self.push(Instruction::Binary {
                            dest,
                            op: BinaryOp::Xor,
                            ty: Type::Bool,
                            left: value.operand,
                            right: Operand::Boolean(true),
                        });
                    }
                    (cerune_ir::UnaryOp::Negate, Type::Bool)
                    | (cerune_ir::UnaryOp::Not, Type::I64 | Type::Float | Type::Double) => {
                        unreachable!("semantic analysis rejects invalid unary operands");
                    }
                    (
                        _,
                        Type::String
                        | Type::DynamicArray { .. }
                        | Type::Named(_)
                        | Type::Array { .. },
                    ) => {
                        unreachable!(
                            "semantic analysis rejects string and aggregate unary operands"
                        );
                    }
                }

                Value {
                    ty: value.ty,
                    operand: Operand::Temp(dest),
                }
            }

            cerune_ir::ExprKind::Logical { op, left, right } => {
                let left = self.lower_expr(left);
                let rhs_label = self.next_label();
                let end_label = self.next_label();
                let slot = SlotId(self.slots.len());
                let mut name = format!("logical_result{}", slot.0);
                while self.slots.iter().any(|slot| slot.name == name) {
                    name.push('_');
                }
                self.slots.push(Slot {
                    name,
                    ty: Type::Bool,
                });
                // 分岐前の結果を保存し、右辺を評価した場合だけ置き換えます。
                self.push(Instruction::Store {
                    ty: Type::Bool,
                    value: left.operand,
                    slot,
                });
                let (then_label, else_label) = match op {
                    cerune_ir::LogicalOp::And => (rhs_label, end_label),
                    cerune_ir::LogicalOp::Or => (end_label, rhs_label),
                };
                self.push(Instruction::Branch {
                    condition: left.operand,
                    then_label,
                    else_label,
                });
                self.push(Instruction::Label {
                    id: rhs_label,
                    name: "logical_rhs",
                });
                let right = self.lower_expr(right);
                self.push(Instruction::Store {
                    ty: Type::Bool,
                    value: right.operand,
                    slot,
                });
                self.push(Instruction::Jump { label: end_label });
                self.push(Instruction::Label {
                    id: end_label,
                    name: "logical_end",
                });
                let dest = self.next_temp();
                self.push(Instruction::Load {
                    dest,
                    ty: Type::Bool,
                    slot,
                });
                Value {
                    ty: Type::Bool,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Binary { op, left, right } => {
                let source_operand_ty = left.ty.clone();
                let left = self.lower_expr(left);
                let right = self.lower_expr(right);
                let dest = self.next_temp();

                if let Some(op) = crate::codegen::integer_binary_op(*op, &source_operand_ty) {
                    self.push(Instruction::IntegerBinary {
                        dest,
                        op,
                        ty: crate::codegen::integer_type(&expr.ty),
                        left: left.operand,
                        right: right.operand,
                    });
                } else if let Some(op) = compare_op(*op) {
                    self.push(Instruction::Compare {
                        unsigned: crate::codegen::is_u64(&source_operand_ty),
                        dest,
                        op,
                        operand_ty: left.ty.clone(),
                        left: left.operand,
                        right: right.operand,
                    });
                } else {
                    self.push(Instruction::Binary {
                        dest,
                        op: binary_op(*op, &left.ty),
                        ty: left.ty.clone(),
                        left: left.operand,
                        right: right.operand,
                    });
                }

                Value {
                    ty: expr.ty.clone().into(),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Construct {
                type_id,
                base,
                fields,
                ..
            } => {
                let ty = Type::Named(type_id.0);
                let mut aggregate = base
                    .as_ref()
                    .map_or(Operand::Poison, |base| self.lower_expr(base).operand);
                for field in fields {
                    let value = self.lower_expr(&field.value);
                    let dest = self.next_temp();
                    self.push(Instruction::InsertValue {
                        dest,
                        ty: ty.clone(),
                        aggregate,
                        value_ty: value.ty,
                        value: value.operand,
                        field: field.id.0,
                    });
                    aggregate = Operand::Temp(dest);
                }
                Value {
                    ty,
                    operand: aggregate,
                }
            }
            cerune_ir::ExprKind::FieldAccess {
                type_id,
                field_id,
                base,
                ..
            } => {
                let aggregate = self.lower_expr(base);
                let dest = self.next_temp();
                self.push(Instruction::ExtractValue {
                    dest,
                    ty: Type::Named(type_id.0),
                    aggregate: aggregate.operand,
                    field: field_id.0,
                });
                Value {
                    ty: expr.ty.clone().into(),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Array(values) => {
                let ty: Type = expr.ty.clone().into();
                let mut aggregate = Operand::Poison;
                for (index, value) in values.iter().enumerate() {
                    let value = self.lower_expr(value);
                    let dest = self.next_temp();
                    self.push(Instruction::InsertValue {
                        dest,
                        ty: ty.clone(),
                        aggregate,
                        value_ty: value.ty,
                        value: value.operand,
                        field: index,
                    });
                    aggregate = Operand::Temp(dest);
                }
                Value {
                    ty,
                    operand: aggregate,
                }
            }
            cerune_ir::ExprKind::Index { base, index } => {
                let array = self.lower_expr(base);
                let index = self.lower_expr(index);
                let (element, length) = match &array.ty {
                    Type::Array { element, length } => (element, Some(*length)),
                    Type::DynamicArray { element } => (element, None),
                    _ => unreachable!("indexed expression must have an array base"),
                };
                let dest = self.next_temp();
                self.push(Instruction::ArrayGet {
                    dest,
                    element: (**element).clone(),
                    length,
                    array: array.operand,
                    index: index.operand,
                });
                Value {
                    ty: expr.ty.clone().into(),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Call {
                function_id,
                arguments,
                ..
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        let value = self.lower_expr(argument);
                        (value.ty, value.operand)
                    })
                    .collect();
                let ty: Type = expr.ty.clone().into();
                let dest = self.next_temp();
                self.push(Instruction::Call {
                    dest: Some(dest),
                    function_id: function_id.0,
                    return_type: Some(ty.clone()),
                    arguments,
                });
                Value {
                    ty,
                    operand: Operand::Temp(dest),
                }
            }
        }
    }

    fn slot(&self, id: cerune_ir::BindingId) -> SlotId {
        self.slot_map
            .get(&id)
            .copied()
            .expect("binding must have an LLVM slot")
    }

    fn next_temp(&mut self) -> Temp {
        let temp = Temp(self.temp);
        self.temp += 1;
        temp
    }

    fn next_label(&mut self) -> Label {
        let label = Label(self.label);
        self.label += 1;
        label
    }
}

fn collect_slots(
    statements: &[cerune_ir::Statement],
    slots: &mut Vec<Slot>,
    slot_map: &mut HashMap<cerune_ir::BindingId, SlotId>,
    name_counts: &mut HashMap<String, usize>,
) {
    for statement in statements {
        match &statement.kind {
            cerune_ir::StatementKind::ArrayInitialize { .. }
            | cerune_ir::StatementKind::ArrayRetain { .. }
            | cerune_ir::StatementKind::ArrayFree { .. }
            | cerune_ir::StatementKind::ArrayRangeCheck { .. } => {}
            cerune_ir::StatementKind::Binding { id, name, ty, .. } => {
                let count = name_counts.entry(name.clone()).or_default();
                let lowered_name = if *count == 0 {
                    name.clone()
                } else {
                    format!("{name}_{}", id.0)
                };
                *count += 1;

                let slot = SlotId(slots.len());
                slots.push(Slot {
                    name: lowered_name,
                    ty: ty.clone().into(),
                });
                slot_map.insert(*id, slot);
            }
            cerune_ir::StatementKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_slots(then_body, slots, slot_map, name_counts);
                collect_slots(else_body, slots, slot_map, name_counts);
            }
            cerune_ir::StatementKind::While { body, .. } => {
                collect_slots(body, slots, slot_map, name_counts);
            }
            cerune_ir::StatementKind::For {
                initializer,
                update,
                body,
                ..
            } => {
                collect_slots(
                    std::slice::from_ref(initializer),
                    slots,
                    slot_map,
                    name_counts,
                );
                collect_slots(std::slice::from_ref(update), slots, slot_map, name_counts);
                collect_slots(body, slots, slot_map, name_counts);
            }
            cerune_ir::StatementKind::Assignment { .. }
            | cerune_ir::StatementKind::StringManage { .. }
            | cerune_ir::StatementKind::Write { .. }
            | cerune_ir::StatementKind::Flush
            | cerune_ir::StatementKind::Print { .. }
            | cerune_ir::StatementKind::Call { .. }
            | cerune_ir::StatementKind::Return { .. }
            | cerune_ir::StatementKind::Break
            | cerune_ir::StatementKind::Continue => {}
        }
    }
}

impl From<cerune_ir::Type> for Type {
    fn from(value: cerune_ir::Type) -> Self {
        match value {
            cerune_ir::Type::DynamicArray { element } => Self::DynamicArray {
                element: Box::new((*element).into()),
            },
            cerune_ir::Type::String => Self::String,
            cerune_ir::Type::Bool => Self::Bool,
            cerune_ir::Type::Integer(_) => Self::I64,
            cerune_ir::Type::F32 => Self::Float,
            cerune_ir::Type::F64 => Self::Double,
            cerune_ir::Type::Named(id) => Self::Named(id.0),
            cerune_ir::Type::Array { element, length } => Self::Array {
                element: Box::new((*element).into()),
                length,
            },
        }
    }
}

fn binary_op(op: cerune_ir::BinaryOp, ty: &Type) -> BinaryOp {
    match (op, ty) {
        (
            cerune_ir::BinaryOp::Remainder
            | cerune_ir::BinaryOp::BitAnd
            | cerune_ir::BinaryOp::BitOr
            | cerune_ir::BinaryOp::BitXor
            | cerune_ir::BinaryOp::ShiftLeft
            | cerune_ir::BinaryOp::ShiftRight,
            _,
        ) => unreachable!("integer operation uses separate lowering"),
        (cerune_ir::BinaryOp::Add, Type::I64) => BinaryOp::CheckedI64Add,
        (cerune_ir::BinaryOp::Subtract, Type::I64) => BinaryOp::CheckedI64Sub,
        (cerune_ir::BinaryOp::Multiply, Type::I64) => BinaryOp::CheckedI64Mul,
        (cerune_ir::BinaryOp::Divide, Type::I64) => BinaryOp::CheckedI64Div,

        (cerune_ir::BinaryOp::Add, Type::Float | Type::Double) => BinaryOp::FAdd,
        (cerune_ir::BinaryOp::Subtract, Type::Float | Type::Double) => BinaryOp::FSub,
        (cerune_ir::BinaryOp::Multiply, Type::Float | Type::Double) => BinaryOp::FMul,
        (cerune_ir::BinaryOp::Divide, Type::Float | Type::Double) => BinaryOp::FDiv,

        (cerune_ir::BinaryOp::Add, Type::Bool)
        | (cerune_ir::BinaryOp::Subtract, Type::Bool)
        | (cerune_ir::BinaryOp::Multiply, Type::Bool)
        | (cerune_ir::BinaryOp::Divide, Type::Bool)
        | (
            cerune_ir::BinaryOp::Add
            | cerune_ir::BinaryOp::Subtract
            | cerune_ir::BinaryOp::Multiply
            | cerune_ir::BinaryOp::Divide,
            Type::Named(_) | Type::String,
        )
        | (
            cerune_ir::BinaryOp::Add
            | cerune_ir::BinaryOp::Subtract
            | cerune_ir::BinaryOp::Multiply
            | cerune_ir::BinaryOp::Divide,
            Type::DynamicArray { .. } | Type::Array { .. },
        )
        | (cerune_ir::BinaryOp::Equal, _)
        | (cerune_ir::BinaryOp::NotEqual, _)
        | (cerune_ir::BinaryOp::Less, _)
        | (cerune_ir::BinaryOp::LessEqual, _)
        | (cerune_ir::BinaryOp::Greater, _)
        | (cerune_ir::BinaryOp::GreaterEqual, _) => {
            unreachable!("comparison and invalid arithmetic use separate lowering")
        }
    }
}

const fn compare_op(op: cerune_ir::BinaryOp) -> Option<CompareOp> {
    match op {
        cerune_ir::BinaryOp::Add
        | cerune_ir::BinaryOp::Subtract
        | cerune_ir::BinaryOp::Multiply
        | cerune_ir::BinaryOp::Divide
        | cerune_ir::BinaryOp::Remainder
        | cerune_ir::BinaryOp::BitAnd
        | cerune_ir::BinaryOp::BitOr
        | cerune_ir::BinaryOp::BitXor
        | cerune_ir::BinaryOp::ShiftLeft
        | cerune_ir::BinaryOp::ShiftRight => None,
        cerune_ir::BinaryOp::Equal => Some(CompareOp::Equal),
        cerune_ir::BinaryOp::NotEqual => Some(CompareOp::NotEqual),
        cerune_ir::BinaryOp::Less => Some(CompareOp::Less),
        cerune_ir::BinaryOp::LessEqual => Some(CompareOp::LessEqual),
        cerune_ir::BinaryOp::Greater => Some(CompareOp::Greater),
        cerune_ir::BinaryOp::GreaterEqual => Some(CompareOp::GreaterEqual),
    }
}
