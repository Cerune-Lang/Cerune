use std::collections::HashMap;

use crate::ir as cerune_ir;

use super::ir::{
    BinaryOp, CompareOp, FailureOrigin, Function, Instruction, Module, Operand, Parameter,
    ParameterPassing, Slot, Temp, Type,
};

pub fn lower(program: &cerune_ir::Program) -> Module {
    let mut strings = Vec::new();
    let functions = program
        .function_definitions
        .iter()
        .map(|function| lower_function(program, function, &mut strings))
        .collect();
    let mut slots = Vec::new();
    let mut slot_map = HashMap::new();
    let mut name_counts = HashMap::new();
    collect_slots(
        &program.statements,
        program,
        &mut slots,
        &mut slot_map,
        &mut name_counts,
    );

    let mut lowerer = Lowerer {
        strings: &mut strings,
        program,
        slots,
        instructions: Vec::new(),
        temp: 0,
        label: 0,
        aggregate_temp: 0,
        slot_map,
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

fn lower_function(
    program: &cerune_ir::Program,
    function: &cerune_ir::FunctionDefinition,
    strings: &mut Vec<String>,
) -> Function {
    let mut slots = Vec::new();
    let mut slot_map = HashMap::new();
    let mut name_counts = HashMap::new();
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| {
            let slot = slots.len();
            slots.push(Slot {
                name: parameter.name.clone(),
                size: type_size(program, &parameter.ty),
            });
            slot_map.insert(parameter.id, slot);
            name_counts.insert(parameter.name.clone(), 1);
            Parameter {
                name: parameter.name.clone(),
                passing: match &parameter.ty {
                    cerune_ir::Type::DynamicArray { .. }
                    | cerune_ir::Type::String
                    | cerune_ir::Type::Bool
                    | cerune_ir::Type::Integer(_)
                    | cerune_ir::Type::F32
                    | cerune_ir::Type::F64 => ParameterPassing::Scalar(scalar_type(&parameter.ty)),
                    cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. } => {
                        ParameterPassing::Aggregate {
                            size: type_size(program, &parameter.ty),
                        }
                    }
                },
                slot,
            }
        })
        .collect();
    collect_slots(
        &function.body,
        program,
        &mut slots,
        &mut slot_map,
        &mut name_counts,
    );

    let mut lowerer = Lowerer {
        strings,
        program,
        slots,
        instructions: Vec::new(),
        temp: 0,
        label: 0,
        aggregate_temp: 0,
        slot_map,
        loops: Vec::new(),
    };
    let terminates = lowerer.lower_statements(&function.body);
    if !terminates {
        lowerer
            .instructions
            .push(Instruction::Return { value: None });
    }

    Function {
        id: function.id.0,
        name: function.name.clone(),
        parameters,
        return_type: match &function.return_type {
            cerune_ir::ReturnType::Void => None,
            cerune_ir::ReturnType::Value(
                ty @ (cerune_ir::Type::DynamicArray { .. }
                | cerune_ir::Type::String
                | cerune_ir::Type::Bool
                | cerune_ir::Type::Integer(_)
                | cerune_ir::Type::F32
                | cerune_ir::Type::F64),
            ) => Some(scalar_type(ty)),
            cerune_ir::ReturnType::Value(
                cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. },
            ) => None,
        },
        aggregate_return_size: match &function.return_type {
            cerune_ir::ReturnType::Value(
                ty @ (cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. }),
            ) => Some(type_size(program, ty)),
            cerune_ir::ReturnType::Void
            | cerune_ir::ReturnType::Value(
                cerune_ir::Type::DynamicArray { .. }
                | cerune_ir::Type::String
                | cerune_ir::Type::Bool
                | cerune_ir::Type::Integer(_)
                | cerune_ir::Type::F32
                | cerune_ir::Type::F64,
            ) => None,
        },
        slots: lowerer.slots,
        instructions: lowerer.instructions,
    }
}

struct Lowerer<'a> {
    strings: &'a mut Vec<String>,
    program: &'a cerune_ir::Program,
    slots: Vec<Slot>,
    instructions: Vec<Instruction>,
    temp: usize,
    label: usize,
    aggregate_temp: usize,
    slot_map: HashMap<cerune_ir::BindingId, usize>,
    loops: Vec<LoopContext>,
}

#[derive(Debug, Clone, Copy)]
struct LoopContext {
    continue_label: usize,
    break_label: usize,
}

#[derive(Debug, Clone)]
enum Value {
    Scalar {
        ty: Type,
        operand: Operand,
    },
    Aggregate {
        type_id: usize,
        address: Operand,
    },
    Array {
        element: ArrayElement,
        length: usize,
        address: Operand,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArrayElement {
    Scalar(Type),
    Named(usize),
    Array {
        element: Box<ArrayElement>,
        length: usize,
    },
}

impl Lowerer<'_> {
    fn lower_statements(&mut self, statements: &[cerune_ir::Statement]) -> bool {
        for statement in statements {
            if self.lower_statement(statement) {
                return true;
            }
        }

        false
    }

    fn lower_statement(&mut self, statement: &cerune_ir::Statement) -> bool {
        match &statement.kind {
            cerune_ir::StatementKind::ArrayInitialize { array, value } => {
                let (_, array) = self.lower_scalar_expr(array);
                let lowered = self.lower_expr(value);
                let dest = self.next_temp();
                self.instructions.push(Instruction::ArrayInitAddress {
                    dest,
                    value: array.clone(),
                });
                self.store_value(&value.ty, lowered, Operand::Temp(dest));
                self.instructions
                    .push(Instruction::ArrayInitialized { value: array });
                false
            }
            cerune_ir::StatementKind::ArrayRetain { value }
            | cerune_ir::StatementKind::ArrayFree { value } => {
                let (_, value) = self.lower_scalar_expr(value);
                self.instructions.push(Instruction::ArrayManage {
                    value,
                    retain: matches!(statement.kind, cerune_ir::StatementKind::ArrayRetain { .. }),
                });
                false
            }
            cerune_ir::StatementKind::ArrayRangeCheck { length, start, end } => {
                let (_, length) = self.lower_scalar_expr(length);
                let (_, start) = self.lower_scalar_expr(start);
                let (_, end) = self.lower_scalar_expr(end);
                self.instructions.push(Instruction::ArrayRangeCheck {
                    length,
                    start,
                    end,
                    origin: FailureOrigin {
                        node: statement.id,
                        span: statement.span,
                    },
                });
                false
            }
            cerune_ir::StatementKind::Binding { id, ty, value, .. } => {
                let value = self.lower_expr(value);
                let destination = Operand::Slot(self.slot(*id));
                self.store_value(ty, value, destination);
                false
            }

            cerune_ir::StatementKind::Assignment { target, value } => {
                let mut destination = Operand::Slot(self.slot(target.id));
                for projection in &target.projections {
                    destination = match projection {
                        cerune_ir::AssignmentProjection::Index {
                            index,
                            element,
                            length,
                            span,
                        } => self.lower_checked_array_address(
                            destination,
                            &array_element_type(element),
                            *length,
                            index,
                            FailureOrigin {
                                node: statement.id,
                                span: *span,
                            },
                        ),
                        cerune_ir::AssignmentProjection::DynamicIndex { index, span, .. } => {
                            let owner = self.next_temp();
                            self.instructions.push(Instruction::Load {
                                dest: owner,
                                address: destination,
                                ty: Type::DynamicArray,
                            });
                            let (_, index) = self.lower_scalar_expr(index);
                            let dest = self.next_temp();
                            self.instructions.push(Instruction::ArrayAddress {
                                dest,
                                value: Operand::Temp(owner),
                                index,
                                origin: FailureOrigin {
                                    node: statement.id,
                                    span: *span,
                                },
                            });
                            Operand::Temp(dest)
                        }
                    };
                }
                let value = self.lower_expr(value);
                self.store_value(&target.ty, value, destination);
                false
            }

            cerune_ir::StatementKind::StringManage { value, retain } => {
                let Value::Scalar { operand, .. } = self.lower_expr(value) else {
                    unreachable!()
                };
                self.instructions.push(Instruction::StringManage {
                    value: operand,
                    retain: *retain,
                });
                false
            }
            cerune_ir::StatementKind::Write { value, quoted } => {
                let kind = crate::codegen::display::kind(&value.ty, *quoted);
                let Value::Scalar { operand, .. } = self.lower_expr(value) else {
                    unreachable!()
                };
                self.instructions.push(Instruction::Output {
                    kind,
                    value: operand,
                    newline: false,
                    origin: FailureOrigin {
                        node: statement.id,
                        span: statement.span,
                    },
                });
                false
            }
            cerune_ir::StatementKind::Flush => {
                self.instructions.push(Instruction::Flush {
                    origin: FailureOrigin {
                        node: statement.id,
                        span: statement.span,
                    },
                });
                false
            }
            cerune_ir::StatementKind::Print { value } => {
                let kind = crate::codegen::display::kind(&value.ty, false);
                let Value::Scalar { operand, .. } = self.lower_expr(value) else {
                    unreachable!()
                };
                self.instructions.push(Instruction::Output {
                    kind,
                    value: operand,
                    newline: true,
                    origin: FailureOrigin {
                        node: statement.id,
                        span: statement.span,
                    },
                });
                false
            }

            cerune_ir::StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.lower_scalar_expr(condition);
                let then_label = self.next_label();
                let else_label = self.next_label();
                let end_label = self.next_label();
                self.instructions.push(Instruction::Branch {
                    condition: condition.1,
                    then_label,
                    else_label,
                });
                self.instructions.push(Instruction::Label {
                    id: then_label,
                    name: "if_then",
                });
                let then_terminates = self.lower_statements(then_body);
                if !then_terminates {
                    self.instructions.push(Instruction::Jump(end_label));
                }
                self.instructions.push(Instruction::Label {
                    id: else_label,
                    name: "if_else",
                });
                let else_terminates = self.lower_statements(else_body);
                if !else_terminates {
                    self.instructions.push(Instruction::Jump(end_label));
                }

                if then_terminates && else_terminates {
                    true
                } else {
                    self.instructions.push(Instruction::Label {
                        id: end_label,
                        name: "if_end",
                    });
                    false
                }
            }

            cerune_ir::StatementKind::While { condition, body } => {
                let condition_label = self.next_label();
                let body_label = self.next_label();
                let end_label = self.next_label();

                self.instructions.push(Instruction::Jump(condition_label));
                self.instructions.push(Instruction::Label {
                    id: condition_label,
                    name: "while_condition",
                });
                let condition = self.lower_scalar_expr(condition);
                self.instructions.push(Instruction::Branch {
                    condition: condition.1,
                    then_label: body_label,
                    else_label: end_label,
                });
                self.instructions.push(Instruction::Label {
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
                    self.instructions.push(Instruction::Jump(condition_label));
                }
                self.instructions.push(Instruction::Label {
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

                self.instructions.push(Instruction::Jump(condition_label));
                self.instructions.push(Instruction::Label {
                    id: condition_label,
                    name: "for_condition",
                });
                let condition = self.lower_scalar_expr(condition);
                self.instructions.push(Instruction::Branch {
                    condition: condition.1,
                    then_label: body_label,
                    else_label: end_label,
                });
                self.instructions.push(Instruction::Label {
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
                    self.instructions.push(Instruction::Jump(update_label));
                }
                self.instructions.push(Instruction::Label {
                    id: update_label,
                    name: "for_update",
                });
                self.lower_statement(update);
                self.instructions.push(Instruction::Jump(condition_label));
                self.instructions.push(Instruction::Label {
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
                self.instructions.push(Instruction::Jump(target));
                true
            }

            cerune_ir::StatementKind::Continue => {
                let target = self
                    .loops
                    .last()
                    .expect("semantic analysis rejects continue outside a loop")
                    .continue_label;
                self.instructions.push(Instruction::Jump(target));
                true
            }
            cerune_ir::StatementKind::Call {
                function_id,
                arguments,
                ..
            } => {
                self.lower_call(function_id.0, arguments, None);
                false
            }
            cerune_ir::StatementKind::Return { value } => {
                match value.as_ref().map(|value| (value, self.lower_expr(value))) {
                    Some((_, Value::Scalar { ty, operand })) => {
                        self.instructions.push(Instruction::Return {
                            value: Some((ty, operand)),
                        });
                    }
                    Some((
                        value,
                        Value::Aggregate { address, .. } | Value::Array { address, .. },
                    )) => {
                        self.instructions.push(Instruction::Blit {
                            source: address,
                            destination: Operand::ReturnPointer,
                            size: type_size(self.program, &value.ty),
                        });
                        self.instructions.push(Instruction::Return { value: None });
                    }
                    None => self.instructions.push(Instruction::Return { value: None }),
                }
                true
            }
        }
    }

    fn lower_expr(&mut self, expr: &cerune_ir::Expr) -> Value {
        let value = self.lower_expr_unchecked(expr);
        if let Some(ty) = super::super::integer_range_check(expr) {
            let Value::Scalar { operand, .. } = value else {
                unreachable!()
            };
            let dest = self.next_temp();
            self.instructions.push(Instruction::CheckIntegerRange {
                origin: expr.into(),
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
                dest,
                value: operand,
                ty,
            });
            return Value::Scalar {
                ty: Type::I64,
                operand: Operand::Temp(dest),
            };
        }
        value
    }

    fn lower_expr_unchecked(&mut self, expr: &cerune_ir::Expr) -> Value {
        if let Some((value, conversion)) = crate::codegen::u64_integer_conversion(expr) {
            let (_, value) = self.lower_scalar_expr(value);
            let dest = self.next_temp();
            self.instructions.push(Instruction::ConvertNumeric {
                origin: expr.into(),
                dest,
                value,
                conversion,
            });
            return Value::Scalar {
                ty: Type::I64,
                operand: Operand::Temp(dest),
            };
        }

        match &expr.kind {
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
                let (_, length) = self.lower_scalar_expr(length);
                let dest = self.next_temp();
                self.instructions.push(Instruction::ArrayAllocate {
                    dest,
                    length,
                    width: *element_width,
                    stride: type_size(self.program, element),
                    origin: expr.into(),
                });
                Value::Scalar {
                    ty: Type::DynamicArray,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ArrayReleaseOwner { value } => {
                let (_, value) = self.lower_scalar_expr(value);
                let dest = self.next_temp();
                self.instructions
                    .push(Instruction::ArrayReleaseOwner { dest, value });
                Value::Scalar {
                    ty: Type::Bool,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Let { .. } | cerune_ir::ExprKind::Conditional { .. } => {
                unreachable!("match expressions are lowered before code generation")
            }
            cerune_ir::ExprKind::Constant { value, .. } => self.lower_expr(value),
            cerune_ir::ExprKind::ArrayLength { value } => {
                let lowered = self.lower_expr(value);
                let operand = match &value.ty {
                    cerune_ir::Type::Array { length, .. } => Operand::Integer(*length as i64),
                    cerune_ir::Type::DynamicArray { .. } => {
                        let Value::Scalar { operand, .. } = lowered else {
                            unreachable!()
                        };
                        let dest = self.next_temp();
                        self.instructions.push(Instruction::ArrayLength {
                            dest,
                            value: operand,
                        });
                        Operand::Temp(dest)
                    }
                    _ => unreachable!(),
                };
                Value::Scalar {
                    ty: Type::I64,
                    operand,
                }
            }
            cerune_ir::ExprKind::StringConcat { left, right } => {
                let Value::Scalar { operand: left, .. } = self.lower_expr(left) else {
                    unreachable!()
                };
                let Value::Scalar { operand: right, .. } = self.lower_expr(right) else {
                    unreachable!()
                };
                let dest = self.next_temp();
                self.instructions.push(Instruction::StringConcat {
                    dest,
                    left,
                    right,
                    origin: FailureOrigin {
                        node: expr.id,
                        span: expr.span,
                    },
                });
                Value::Scalar {
                    ty: Type::String,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ReadLine { .. } => {
                unreachable!("line input rejected before generated lowering")
            }
            cerune_ir::ExprKind::StringByteLength { value } => {
                let value = self.lower_expr(value);
                let Value::Scalar {
                    ty,
                    operand: address,
                } = value
                else {
                    unreachable!("byte_len operand is a string")
                };
                debug_assert_eq!(ty, Type::String);
                let dest = self.next_temp();
                self.instructions.push(Instruction::Load {
                    dest,
                    address,
                    ty: Type::I64,
                });
                Value::Scalar {
                    ty: Type::I64,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::String(value) => {
                let id = self.strings.len();
                self.strings.push(value.clone());
                Value::Scalar {
                    ty: Type::String,
                    operand: Operand::String(id),
                }
            }
            cerune_ir::ExprKind::ConvertNumeric {
                value,
                from,
                to,
                mode,
                ..
            } => {
                let (ty, value) = self.lower_scalar_expr(value);
                if from == to {
                    return Value::Scalar { ty, operand: value };
                }
                let dest = self.next_temp();
                self.instructions.push(Instruction::ConvertNumeric {
                    origin: expr.into(),
                    dest,
                    value,
                    conversion: crate::codegen::NumericConversion {
                        mode: *mode,
                        from: *from,
                        to: *to,
                    },
                });
                Value::Scalar {
                    ty: scalar_type(&expr.ty),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::ConvertInteger { value, .. } => self.lower_expr(value),
            cerune_ir::ExprKind::Boolean(value) => Value::Scalar {
                ty: Type::Bool,
                operand: Operand::Boolean(*value),
            },
            cerune_ir::ExprKind::Integer(value) => Value::Scalar {
                ty: Type::I64,
                operand: Operand::Integer(*value as i64),
            },
            cerune_ir::ExprKind::Float { text } => Value::Scalar {
                ty: scalar_type(&expr.ty),
                operand: match &expr.ty {
                    cerune_ir::Type::F32 => Operand::Float32(text.clone()),
                    cerune_ir::Type::F64 => Operand::Float64(text.clone()),
                    _ => unreachable!("a float literal has a float type"),
                },
            },
            cerune_ir::ExprKind::Variable { id, .. } => match &expr.ty {
                cerune_ir::Type::Named(type_id) => Value::Aggregate {
                    type_id: type_id.0,
                    address: Operand::Slot(self.slot(*id)),
                },
                cerune_ir::Type::Array { element, length } => Value::Array {
                    element: array_element_type(element),
                    length: *length,
                    address: Operand::Slot(self.slot(*id)),
                },
                scalar => {
                    let ty = scalar_type(scalar);
                    let dest = self.next_temp();
                    self.instructions.push(Instruction::Load {
                        dest,
                        address: Operand::Slot(self.slot(*id)),
                        ty,
                    });
                    Value::Scalar {
                        ty,
                        operand: Operand::Temp(dest),
                    }
                }
            },
            cerune_ir::ExprKind::Unary { op, value } => {
                let (ty, operand) = self.lower_scalar_expr(value);
                let dest = self.next_temp();
                match (op, ty) {
                    (cerune_ir::UnaryOp::BitNot, _) => {
                        self.instructions.push(Instruction::IntegerBinary {
                            origin: expr.into(),
                            dest,
                            op: crate::codegen::IntegerBinaryOp::BitXor,
                            ty: crate::codegen::integer_type(&expr.ty),
                            left: operand,
                            right: Operand::Integer(crate::codegen::complement_mask(&expr.ty)),
                        })
                    }
                    (cerune_ir::UnaryOp::Negate, Type::I64) => {
                        self.instructions.push(Instruction::CheckedI64Negate {
                            origin: expr.into(),
                            dest,
                            value: operand,
                        })
                    }
                    (cerune_ir::UnaryOp::Negate, _) => {
                        self.instructions.push(Instruction::Negate {
                            dest,
                            ty,
                            value: operand,
                        })
                    }
                    (cerune_ir::UnaryOp::Not, _) => self.instructions.push(Instruction::Not {
                        dest,
                        value: operand,
                    }),
                }
                Value::Scalar {
                    ty,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Logical { op, left, right } => {
                let (_, left) = self.lower_scalar_expr(left);
                let rhs_label = self.next_label();
                let end_label = self.next_label();
                let slot = self.slots.len();
                let mut name = format!("logical_result{slot}");
                while self.slots.iter().any(|slot| slot.name == name) {
                    name.push('_');
                }
                self.slots.push(Slot { name, size: 8 });
                self.instructions.push(Instruction::Store {
                    ty: Type::Bool,
                    value: left.clone(),
                    address: Operand::Slot(slot),
                });
                let (then_label, else_label) = match op {
                    cerune_ir::LogicalOp::And => (rhs_label, end_label),
                    cerune_ir::LogicalOp::Or => (end_label, rhs_label),
                };
                self.instructions.push(Instruction::Branch {
                    condition: left,
                    then_label,
                    else_label,
                });
                self.instructions.push(Instruction::Label {
                    id: rhs_label,
                    name: "logical_rhs",
                });
                let (_, right) = self.lower_scalar_expr(right);
                self.instructions.push(Instruction::Store {
                    ty: Type::Bool,
                    value: right,
                    address: Operand::Slot(slot),
                });
                self.instructions.push(Instruction::Jump(end_label));
                self.instructions.push(Instruction::Label {
                    id: end_label,
                    name: "logical_end",
                });
                let dest = self.next_temp();
                self.instructions.push(Instruction::Load {
                    dest,
                    ty: Type::Bool,
                    address: Operand::Slot(slot),
                });
                Value::Scalar {
                    ty: Type::Bool,
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Binary { op, left, right } => {
                let source_operand_ty = left.ty.clone();
                let (left_ty, left) = self.lower_scalar_expr(left);
                let (right_ty, right) = self.lower_scalar_expr(right);
                debug_assert_eq!(left_ty, right_ty);
                let dest = self.next_temp();

                if let Some(op) = crate::codegen::integer_binary_op(*op, &source_operand_ty) {
                    self.instructions.push(Instruction::IntegerBinary {
                        origin: expr.into(),
                        dest,
                        op,
                        ty: crate::codegen::integer_type(&expr.ty),
                        left,
                        right,
                    });
                } else if let Some(op) = compare_op(*op) {
                    self.instructions.push(Instruction::Compare {
                        unsigned: crate::codegen::is_u64(&source_operand_ty),
                        dest,
                        op,
                        operand_ty: left_ty,
                        left,
                        right,
                    });
                } else {
                    let op = if left_ty == Type::I64 {
                        match op {
                            cerune_ir::BinaryOp::Add => BinaryOp::CheckedI64Add,
                            cerune_ir::BinaryOp::Subtract => BinaryOp::CheckedI64Subtract,
                            cerune_ir::BinaryOp::Multiply => BinaryOp::CheckedI64Multiply,
                            cerune_ir::BinaryOp::Divide => BinaryOp::CheckedI64Divide,
                            _ => unreachable!("comparisons use dedicated QBE instructions"),
                        }
                    } else {
                        (*op).into()
                    };
                    self.instructions.push(Instruction::Binary {
                        origin: expr.into(),
                        dest,
                        op,
                        ty: left_ty,
                        left,
                        right,
                    });
                }

                Value::Scalar {
                    ty: scalar_type(&expr.ty),
                    operand: Operand::Temp(dest),
                }
            }
            cerune_ir::ExprKind::Construct {
                type_id,
                base,
                fields,
                ..
            } => {
                let slot = self.allocate_aggregate(type_size(self.program, &expr.ty));
                if let Some(base) = base {
                    let Value::Aggregate { address, .. } = self.lower_expr(base) else {
                        unreachable!("update base has the same aggregate type")
                    };
                    self.instructions.push(Instruction::Blit {
                        source: address,
                        destination: Operand::Slot(slot),
                        size: type_size(self.program, &expr.ty),
                    });
                }
                for field in fields {
                    let definition = &self.program.type_definitions[type_id.0].fields[field.id.0];
                    let value = self.lower_expr(&field.value);
                    let offset = field_offset(self.program, type_id.0, field.id.0);
                    let destination = self.address(Operand::Slot(slot), offset);
                    match (&definition.ty, value) {
                        (
                            cerune_ir::Type::Named(nested),
                            Value::Aggregate {
                                type_id: actual,
                                address,
                            },
                        ) => {
                            debug_assert_eq!(nested.0, actual);
                            self.instructions.push(Instruction::Blit {
                                source: address,
                                destination,
                                size: type_size(self.program, &definition.ty),
                            });
                        }
                        (
                            cerune_ir::Type::Array { element, length },
                            Value::Array {
                                element: actual_element,
                                length: actual_length,
                                address,
                            },
                        ) => {
                            debug_assert_eq!(array_element_type(element), actual_element);
                            debug_assert_eq!(*length, actual_length);
                            self.instructions.push(Instruction::Blit {
                                source: address,
                                destination,
                                size: type_size(self.program, &definition.ty),
                            });
                        }
                        (scalar, Value::Scalar { ty, operand }) => {
                            debug_assert_eq!(scalar_type(scalar), ty);
                            self.instructions.push(Instruction::Store {
                                address: destination,
                                ty,
                                value: operand,
                            });
                        }
                        _ => unreachable!("semantic analysis keeps field types equal"),
                    }
                }
                Value::Aggregate {
                    type_id: type_id.0,
                    address: Operand::Slot(slot),
                }
            }
            cerune_ir::ExprKind::FieldAccess {
                type_id,
                field_id,
                base,
                ..
            } => {
                let Value::Aggregate { address, .. } = self.lower_expr(base) else {
                    unreachable!("semantic analysis requires an aggregate field base")
                };
                let address =
                    self.address(address, field_offset(self.program, type_id.0, field_id.0));
                match &expr.ty {
                    cerune_ir::Type::Named(nested) => Value::Aggregate {
                        type_id: nested.0,
                        address,
                    },
                    cerune_ir::Type::Array { element, length } => Value::Array {
                        element: array_element_type(element),
                        length: *length,
                        address,
                    },
                    scalar => {
                        let ty = scalar_type(scalar);
                        let dest = self.next_temp();
                        self.instructions
                            .push(Instruction::Load { dest, address, ty });
                        Value::Scalar {
                            ty,
                            operand: Operand::Temp(dest),
                        }
                    }
                }
            }
            cerune_ir::ExprKind::Array(values) => {
                let cerune_ir::Type::Array { element, length } = &expr.ty else {
                    unreachable!("array expression must have an array type")
                };
                let slot = self.allocate_aggregate(type_size(self.program, &expr.ty));
                let expected = array_element_type(element);
                let stride = type_size(self.program, element);
                for (index, value) in values.iter().enumerate() {
                    let value = self.lower_expr(value);
                    let destination = self.address(Operand::Slot(slot), index * stride);
                    match (expected.clone(), value) {
                        (ArrayElement::Scalar(expected), Value::Scalar { ty, operand }) => {
                            debug_assert_eq!(expected, ty);
                            self.instructions.push(Instruction::Store {
                                address: destination,
                                ty,
                                value: operand,
                            });
                        }
                        (
                            ArrayElement::Named(expected),
                            Value::Aggregate {
                                type_id,
                                address: source,
                            },
                        ) => {
                            debug_assert_eq!(expected, type_id);
                            self.instructions.push(Instruction::Blit {
                                source,
                                destination,
                                size: stride,
                            });
                        }
                        (
                            ArrayElement::Array {
                                element: expected_element,
                                length: expected_length,
                            },
                            Value::Array {
                                element,
                                length,
                                address: source,
                            },
                        ) => {
                            debug_assert_eq!(*expected_element, element);
                            debug_assert_eq!(expected_length, length);
                            self.instructions.push(Instruction::Blit {
                                source,
                                destination,
                                size: stride,
                            });
                        }
                        _ => unreachable!("semantic analysis keeps array element types equal"),
                    }
                }
                Value::Array {
                    element: expected,
                    length: *length,
                    address: Operand::Slot(slot),
                }
            }
            cerune_ir::ExprKind::Index { base, index } => {
                let (element, address) = match self.lower_expr(base) {
                    Value::Array {
                        element,
                        length,
                        address,
                    } => {
                        let address = self.lower_checked_array_address(
                            address,
                            &element,
                            length,
                            index,
                            expr.into(),
                        );
                        (element, address)
                    }
                    Value::Scalar {
                        ty: Type::DynamicArray,
                        operand,
                    } => {
                        let cerune_ir::Type::DynamicArray { element } = &base.ty else {
                            unreachable!()
                        };
                        let (_, index) = self.lower_scalar_expr(index);
                        let dest = self.next_temp();
                        self.instructions.push(Instruction::ArrayAddress {
                            dest,
                            value: operand,
                            index,
                            origin: expr.into(),
                        });
                        (array_element_type(element), Operand::Temp(dest))
                    }
                    _ => unreachable!("indexed expression must have an array base"),
                };
                match element {
                    ArrayElement::Scalar(ty) => {
                        let dest = self.next_temp();
                        self.instructions.push(Instruction::Load {
                            dest,
                            address: address.clone(),
                            ty,
                        });
                        Value::Scalar {
                            ty,
                            operand: Operand::Temp(dest),
                        }
                    }
                    ArrayElement::Named(type_id) => Value::Aggregate {
                        type_id,
                        address: address.clone(),
                    },
                    ArrayElement::Array { element, length } => Value::Array {
                        element: *element,
                        length,
                        address,
                    },
                }
            }
            cerune_ir::ExprKind::Call {
                function_id,
                arguments,
                ..
            } => self
                .lower_call(function_id.0, arguments, Some(&expr.ty))
                .expect("call expressions produce a value"),
        }
    }

    fn store_value(&mut self, ty: &cerune_ir::Type, value: Value, destination: Operand) {
        match (ty, value) {
            (
                cerune_ir::Type::Named(type_id),
                Value::Aggregate {
                    type_id: actual,
                    address,
                },
            ) => {
                debug_assert_eq!(type_id.0, actual);
                self.instructions.push(Instruction::Blit {
                    source: address,
                    destination,
                    size: type_size(self.program, ty),
                });
            }
            (
                cerune_ir::Type::Array { element, length },
                Value::Array {
                    element: actual_element,
                    length: actual_length,
                    address,
                },
            ) => {
                debug_assert_eq!(array_element_type(element), actual_element);
                debug_assert_eq!(*length, actual_length);
                self.instructions.push(Instruction::Blit {
                    source: address,
                    destination,
                    size: type_size(self.program, ty),
                });
            }
            (
                scalar,
                Value::Scalar {
                    ty: actual,
                    operand,
                },
            ) => {
                debug_assert_eq!(scalar_type(scalar), actual);
                self.instructions.push(Instruction::Store {
                    address: destination,
                    ty: actual,
                    value: operand,
                });
            }
            _ => unreachable!("semantic analysis keeps assignment types equal"),
        }
    }

    fn lower_checked_array_address(
        &mut self,
        base: Operand,
        element: &ArrayElement,
        length: usize,
        index: &cerune_ir::Expr,
        origin: FailureOrigin,
    ) -> Operand {
        let (index_ty, index) = self.lower_scalar_expr(index);
        debug_assert_eq!(index_ty, Type::I64);
        let non_negative = self.next_label();
        let in_bounds = self.next_label();
        let out_of_bounds = self.next_label();
        let is_negative = self.next_temp();
        self.instructions.push(Instruction::Compare {
            unsigned: false,
            dest: is_negative,
            op: CompareOp::Less,
            operand_ty: Type::I64,
            left: index.clone(),
            right: Operand::Integer(0),
        });
        self.instructions.push(Instruction::Branch {
            condition: Operand::Temp(is_negative),
            then_label: out_of_bounds,
            else_label: non_negative,
        });
        self.instructions.push(Instruction::Label {
            id: non_negative,
            name: "array_index_non_negative",
        });
        let is_too_large = self.next_temp();
        self.instructions.push(Instruction::Compare {
            unsigned: false,
            dest: is_too_large,
            op: CompareOp::GreaterEqual,
            operand_ty: Type::I64,
            left: index.clone(),
            right: Operand::Integer(length as i64),
        });
        self.instructions.push(Instruction::Branch {
            condition: Operand::Temp(is_too_large),
            then_label: out_of_bounds,
            else_label: in_bounds,
        });
        self.instructions.push(Instruction::Label {
            id: out_of_bounds,
            name: "array_index_out_of_bounds",
        });
        self.instructions.push(Instruction::Abort { origin });
        self.instructions.push(Instruction::Label {
            id: in_bounds,
            name: "array_index_in_bounds",
        });
        let scaled = self.next_temp();
        self.instructions.push(Instruction::Binary {
            origin,
            dest: scaled,
            op: BinaryOp::Multiply,
            ty: Type::I64,
            left: index,
            right: Operand::Integer(array_element_size(self.program, element) as i64),
        });
        let address = self.next_temp();
        self.instructions.push(Instruction::Binary {
            origin,
            dest: address,
            op: BinaryOp::Add,
            ty: Type::I64,
            left: base,
            right: Operand::Temp(scaled),
        });
        Operand::Temp(address)
    }

    fn lower_call(
        &mut self,
        function_id: usize,
        arguments: &[cerune_ir::Expr],
        result_type: Option<&cerune_ir::Type>,
    ) -> Option<Value> {
        let mut lowered_arguments = Vec::new();
        let aggregate_result = result_type.and_then(|ty| match ty {
            cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. } => {
                let slot = self.allocate_aggregate(type_size(self.program, ty));
                lowered_arguments.push((Type::Pointer, Operand::Slot(slot)));
                Some((ty, slot))
            }
            cerune_ir::Type::DynamicArray { .. }
            | cerune_ir::Type::String
            | cerune_ir::Type::Bool
            | cerune_ir::Type::Integer(_)
            | cerune_ir::Type::F32
            | cerune_ir::Type::F64 => None,
        });

        for argument in arguments {
            match self.lower_expr(argument) {
                Value::Scalar { ty, operand } => lowered_arguments.push((ty, operand)),
                Value::Aggregate { address, .. } | Value::Array { address, .. } => {
                    lowered_arguments.push((Type::Pointer, address));
                }
            }
        }

        let (dest, return_type, scalar_result) = match result_type {
            Some(
                ty @ (cerune_ir::Type::DynamicArray { .. }
                | cerune_ir::Type::String
                | cerune_ir::Type::Bool
                | cerune_ir::Type::Integer(_)
                | cerune_ir::Type::F32
                | cerune_ir::Type::F64),
            ) => {
                let ty = scalar_type(ty);
                let dest = self.next_temp();
                (Some(dest), Some(ty), Some((ty, dest)))
            }
            Some(cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. }) | None => {
                (None, None, None)
            }
        };

        self.instructions.push(Instruction::Call {
            dest,
            function_id,
            return_type,
            arguments: lowered_arguments,
        });

        if let Some((ty, dest)) = scalar_result {
            return Some(Value::Scalar {
                ty,
                operand: Operand::Temp(dest),
            });
        }

        aggregate_result.map(|(ty, slot)| match ty {
            cerune_ir::Type::Named(id) => Value::Aggregate {
                type_id: id.0,
                address: Operand::Slot(slot),
            },
            cerune_ir::Type::Array { element, length } => Value::Array {
                element: array_element_type(element),
                length: *length,
                address: Operand::Slot(slot),
            },
            cerune_ir::Type::DynamicArray { .. }
            | cerune_ir::Type::String
            | cerune_ir::Type::Bool
            | cerune_ir::Type::Integer(_)
            | cerune_ir::Type::F32
            | cerune_ir::Type::F64 => unreachable!("aggregate result type is checked above"),
        })
    }

    fn lower_scalar_expr(&mut self, expr: &cerune_ir::Expr) -> (Type, Operand) {
        let Value::Scalar { ty, operand } = self.lower_expr(expr) else {
            unreachable!("semantic analysis requires a scalar value here")
        };
        (ty, operand)
    }

    fn address(&mut self, base: Operand, offset: usize) -> Operand {
        if offset == 0 {
            base
        } else {
            let dest = self.next_temp();
            self.instructions
                .push(Instruction::Address { dest, base, offset });
            Operand::Temp(dest)
        }
    }

    fn allocate_aggregate(&mut self, size: usize) -> usize {
        let id = self.aggregate_temp;
        self.aggregate_temp += 1;
        let slot = self.slots.len();
        self.slots.push(Slot {
            name: format!("aggregate_tmp{id}"),
            size,
        });
        slot
    }

    fn next_temp(&mut self) -> Temp {
        let temp = Temp(self.temp);
        self.temp += 1;
        temp
    }

    fn next_label(&mut self) -> usize {
        let label = self.label;
        self.label += 1;
        label
    }

    fn slot(&self, id: cerune_ir::BindingId) -> usize {
        self.slot_map[&id]
    }
}

fn collect_slots(
    statements: &[cerune_ir::Statement],
    program: &cerune_ir::Program,
    slots: &mut Vec<Slot>,
    slot_map: &mut HashMap<cerune_ir::BindingId, usize>,
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
                let slot = slots.len();
                slots.push(Slot {
                    name: lowered_name,
                    size: type_size(program, ty),
                });
                slot_map.insert(*id, slot);
            }
            cerune_ir::StatementKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_slots(then_body, program, slots, slot_map, name_counts);
                collect_slots(else_body, program, slots, slot_map, name_counts);
            }
            cerune_ir::StatementKind::While { body, .. } => {
                collect_slots(body, program, slots, slot_map, name_counts);
            }
            cerune_ir::StatementKind::For {
                initializer,
                update,
                body,
                ..
            } => {
                collect_slots(
                    std::slice::from_ref(initializer),
                    program,
                    slots,
                    slot_map,
                    name_counts,
                );
                collect_slots(
                    std::slice::from_ref(update),
                    program,
                    slots,
                    slot_map,
                    name_counts,
                );
                collect_slots(body, program, slots, slot_map, name_counts);
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

fn type_size(program: &cerune_ir::Program, ty: &cerune_ir::Type) -> usize {
    match ty {
        cerune_ir::Type::DynamicArray { .. }
        | cerune_ir::Type::String
        | cerune_ir::Type::Bool
        | cerune_ir::Type::Integer(_)
        | cerune_ir::Type::F32
        | cerune_ir::Type::F64 => 8,
        cerune_ir::Type::Named(id) => program.type_definitions[id.0]
            .fields
            .iter()
            .map(|field| type_size(program, &field.ty))
            .sum(),
        cerune_ir::Type::Array { element, length } => type_size(program, element) * length,
    }
}

fn field_offset(program: &cerune_ir::Program, type_id: usize, field_id: usize) -> usize {
    program.type_definitions[type_id].fields[..field_id]
        .iter()
        .map(|field| type_size(program, &field.ty))
        .sum()
}

fn scalar_type(ty: &cerune_ir::Type) -> Type {
    match ty {
        cerune_ir::Type::DynamicArray { .. } => Type::DynamicArray,
        cerune_ir::Type::String => Type::String,
        cerune_ir::Type::Bool => Type::Bool,
        cerune_ir::Type::Integer(_) => Type::I64,
        cerune_ir::Type::F32 => Type::Single,
        cerune_ir::Type::F64 => Type::Double,
        cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. } => {
            unreachable!("expected a scalar type")
        }
    }
}

fn array_element_type(element: &cerune_ir::Type) -> ArrayElement {
    match element {
        cerune_ir::Type::DynamicArray { .. } => ArrayElement::Scalar(Type::DynamicArray),
        cerune_ir::Type::String => ArrayElement::Scalar(Type::String),
        cerune_ir::Type::Bool => ArrayElement::Scalar(Type::Bool),
        cerune_ir::Type::Integer(_) => ArrayElement::Scalar(Type::I64),
        cerune_ir::Type::F32 => ArrayElement::Scalar(Type::Single),
        cerune_ir::Type::F64 => ArrayElement::Scalar(Type::Double),
        cerune_ir::Type::Named(id) => ArrayElement::Named(id.0),
        cerune_ir::Type::Array { element, length } => ArrayElement::Array {
            element: Box::new(array_element_type(element)),
            length: *length,
        },
    }
}

fn array_element_size(program: &cerune_ir::Program, element: &ArrayElement) -> usize {
    match element {
        ArrayElement::Scalar(_) => 8,
        ArrayElement::Named(id) => {
            type_size(program, &cerune_ir::Type::Named(cerune_ir::TypeId(*id)))
        }
        ArrayElement::Array { element, length } => array_element_size(program, element) * length,
    }
}

impl From<cerune_ir::BinaryOp> for BinaryOp {
    fn from(value: cerune_ir::BinaryOp) -> Self {
        match value {
            cerune_ir::BinaryOp::Add => Self::Add,
            cerune_ir::BinaryOp::Subtract => Self::Subtract,
            cerune_ir::BinaryOp::Multiply => Self::Multiply,
            cerune_ir::BinaryOp::Divide => Self::Divide,
            cerune_ir::BinaryOp::Remainder
            | cerune_ir::BinaryOp::BitAnd
            | cerune_ir::BinaryOp::BitOr
            | cerune_ir::BinaryOp::BitXor
            | cerune_ir::BinaryOp::ShiftLeft
            | cerune_ir::BinaryOp::ShiftRight => {
                unreachable!("integer operation uses separate lowering")
            }
            cerune_ir::BinaryOp::Equal
            | cerune_ir::BinaryOp::NotEqual
            | cerune_ir::BinaryOp::Less
            | cerune_ir::BinaryOp::LessEqual
            | cerune_ir::BinaryOp::Greater
            | cerune_ir::BinaryOp::GreaterEqual => {
                unreachable!("comparisons use a dedicated QBE instruction")
            }
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
