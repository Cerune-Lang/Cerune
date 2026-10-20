use super::*;
use crate::ir::{ExprKind as E, StatementKind as S};
use std::collections::HashMap;

/// 完成済みHIRのみを受け取ります。frontendの意味展開は繰り返しません。
pub fn lower(hir: &ir::Program) -> Result<Program, Error> {
    let mut functions = Vec::new();
    for f in &hir.function_definitions {
        functions.push(Builder::function(hir, Some(f))?.finish(&f.body)?);
    }
    let result = Program {
        uses_strings: ir::string_usage::first_string_span(hir).is_some(),
        string_heap_limit: hir.string_heap_limit,
        array_heap_limit: hir.array_heap_limit,
        types: hir
            .type_definitions
            .iter()
            .map(|t| TypeDefinition {
                id: t.id,
                name: t.name.clone(),
                variants: t.variants.clone(),
                fields: t
                    .fields
                    .iter()
                    .map(|f| (f.id, f.name.clone(), f.ty.clone()))
                    .collect(),
            })
            .collect(),
        functions,
        main: Builder::function(hir, None)?.finish(&hir.statements)?,
    };
    validate(&result)?;
    Ok(result)
}

struct PendingBlock {
    origin: Origin,
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
}
struct Builder<'a> {
    hir: &'a ir::Program,
    function: Function,
    bindings: HashMap<ir::BindingId, LocalId>,
    blocks: Vec<PendingBlock>,
    current: BlockId,
    next_instruction: usize,
    loops: Vec<(BlockId, BlockId)>,
}
const ENTRY: Origin = Origin::Synthetic {
    reason: "function-entry",
};
fn source(id: ir::NodeId, span: Span) -> Origin {
    Origin::Source(SourceOrigin { node_id: id, span })
}
fn derived(origin: Origin, reason: &'static str) -> Origin {
    match origin.source() {
        Some(source) => Origin::Derived { source, reason },
        None => Origin::Synthetic { reason },
    }
}
impl<'a> Builder<'a> {
    fn function(hir: &'a ir::Program, f: Option<&ir::FunctionDefinition>) -> Result<Self, Error> {
        let mut b = Self {
            hir,
            function: Function {
                id: f.map(|f| f.id),
                name: f.map_or("main", |f| f.name.as_str()).into(),
                lowering: f.and_then(|f| f.lowering),
                argument_ownership: f
                    .map_or(ir::ArgumentOwnership::Owned, |f| f.argument_ownership()),
                parameters: vec![],
                return_type: f.map_or(ir::ReturnType::Void, |f| f.return_type.clone()),
                locals: vec![],
                entry: BlockId(0),
                blocks: vec![],
            },
            bindings: HashMap::new(),
            blocks: vec![],
            current: BlockId(0),
            next_instruction: 0,
            loops: vec![],
        };
        b.new_block(ENTRY);
        if let Some(f) = f {
            for p in &f.parameters {
                let local = b.binding(
                    p.id,
                    &p.name,
                    &p.ty,
                    false,
                    f.argument_ownership() == ir::ArgumentOwnership::Borrowed,
                    ENTRY,
                )?;
                b.function.parameters.push(local);
            }
        }
        Ok(b)
    }
    fn id(&mut self) -> InstructionId {
        let id = InstructionId(self.next_instruction);
        self.next_instruction += 1;
        id
    }
    fn new_block(&mut self, origin: Origin) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(PendingBlock {
            origin,
            instructions: vec![],
            terminator: None,
        });
        id
    }
    fn emit(&mut self, kind: InstructionKind, origin: Origin) {
        let id = self.id();
        self.blocks[self.current.0]
            .instructions
            .push(Instruction { id, origin, kind });
    }
    fn end(&mut self, kind: TerminatorKind, origin: Origin) {
        let id = self.id();
        self.blocks[self.current.0].terminator = Some(Terminator { id, origin, kind });
    }
    fn jump(&mut self, block: BlockId, origin: Origin) {
        self.end(TerminatorKind::Jump(block), origin);
    }
    fn local(&mut self, ty: ir::Type, kind: LocalKind) -> LocalId {
        let id = LocalId(self.function.locals.len());
        self.function.locals.push(Local { ty, kind });
        id
    }
    fn binding(
        &mut self,
        id: ir::BindingId,
        name: &str,
        ty: &ir::Type,
        mutable: bool,
        borrowed: bool,
        origin: Origin,
    ) -> Result<LocalId, Error> {
        if self.bindings.contains_key(&id) {
            return Err(Error::new("duplicate HIR binding", origin));
        }
        let local = self.local(
            ty.clone(),
            LocalKind::Binding {
                id,
                name: name.into(),
                mutable,
                borrowed,
            },
        );
        self.bindings.insert(id, local);
        Ok(local)
    }
    fn lookup(&self, id: ir::BindingId, origin: Origin) -> Result<LocalId, Error> {
        self.bindings
            .get(&id)
            .copied()
            .ok_or_else(|| Error::new("unknown HIR binding", origin))
    }
    fn ownership(
        &self,
        id: ir::FunctionId,
        origin: Origin,
    ) -> Result<ir::ArgumentOwnership, Error> {
        self.hir
            .function_definitions
            .get(id.0)
            .filter(|f| f.id == id)
            .map(|f| f.argument_ownership())
            .ok_or_else(|| Error::new("unknown HIR function", origin))
    }
    fn assign(&mut self, destination: LocalId, value: Operation, origin: Origin) {
        self.emit(InstructionKind::Assign { destination, value }, origin);
    }
    fn value(&mut self, ty: &ir::Type, value: Operation, origin: Origin) -> LocalId {
        let local = self.local(ty.clone(), LocalKind::Temporary);
        self.assign(local, value, origin);
        local
    }
    fn finish(mut self, body: &[ir::Statement]) -> Result<Function, Error> {
        self.statements(body)?;
        // 入口本体の後に呼ぶmainも、MIR内の明示命令にします。
        if self.function.id.is_none()
            && let Some(main) = self
                .hir
                .function_definitions
                .iter()
                .find(|f| f.name == "main")
        {
            self.emit(
                InstructionKind::Call {
                    function: main.id,
                    arguments: vec![],
                    ownership: main.argument_ownership(),
                },
                Origin::Synthetic {
                    reason: "entry-main-call",
                },
            );
        }
        let kind = match self.function.return_type {
            ir::ReturnType::Void => TerminatorKind::Return(None),
            _ => TerminatorKind::Unreachable,
        };
        self.end(
            kind,
            Origin::Synthetic {
                reason: "function-end",
            },
        );
        self.function.blocks = self
            .blocks
            .into_iter()
            .map(|b| Block {
                origin: b.origin,
                instructions: b.instructions,
                terminator: b.terminator.expect("lowering closes every block"),
            })
            .collect();
        Ok(self.function)
    }
    fn statements(&mut self, body: &[ir::Statement]) -> Result<(), Error> {
        for s in body {
            self.statement(s)?;
        }
        Ok(())
    }
    fn statement(&mut self, s: &ir::Statement) -> Result<(), Error> {
        let o = source(s.id, s.span);
        match &s.kind {
            S::Binding {
                id,
                name,
                ty,
                mutable,
                borrowed,
                value,
            } => {
                let value = self.expr(value)?;
                let destination = self.binding(*id, name, ty, *mutable, *borrowed, o)?;
                self.assign(destination, Operation::Copy(value), o);
            }
            S::Assignment { target, value } => {
                let root = self.lookup(target.id, o)?;
                let mut path = vec![];
                for p in &target.projections {
                    let (index, span) = match p {
                        ir::AssignmentProjection::Index { index, span, .. }
                        | ir::AssignmentProjection::DynamicIndex { index, span, .. } => {
                            (index, *span)
                        }
                    };
                    path.push(self.expr(index)?);
                    self.emit(
                        InstructionKind::CheckIndex {
                            root,
                            path: path.clone(),
                        },
                        source(s.id, span),
                    );
                }
                let value = self.expr(value)?;
                self.emit(InstructionKind::Store { root, path, value }, o);
            }
            S::Write { value, quoted } => {
                let value = self.expr(value)?;
                self.emit(
                    InstructionKind::Output {
                        value,
                        newline: false,
                        quoted: *quoted,
                    },
                    o,
                );
            }
            S::Flush => self.emit(InstructionKind::Flush, o),
            S::Print { value } => {
                let value = self.expr(value)?;
                self.emit(
                    InstructionKind::Output {
                        value,
                        newline: true,
                        quoted: false,
                    },
                    o,
                );
            }
            S::Call {
                function_id,
                arguments,
                ..
            } => {
                let arguments = self.arguments(arguments)?;
                self.emit(
                    InstructionKind::Call {
                        function: *function_id,
                        arguments,
                        ownership: self.ownership(*function_id, o)?,
                    },
                    o,
                );
            }
            S::Return { value } => {
                let value = value.as_ref().map(|e| self.expr(e)).transpose()?;
                self.end(TerminatorKind::Return(value), o);
                self.current = self.new_block(derived(o, "after-return"));
            }
            S::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.expr(condition)?;
                let then_block = self.new_block(derived(o, "if-then"));
                let else_block = self.new_block(derived(o, "if-else"));
                let join = self.new_block(derived(o, "if-join"));
                self.end(
                    TerminatorKind::Branch {
                        condition,
                        then_block,
                        else_block,
                    },
                    o,
                );
                self.current = then_block;
                self.statements(then_body)?;
                self.jump(join, derived(o, "if-join"));
                self.current = else_block;
                self.statements(else_body)?;
                self.jump(join, derived(o, "if-join"));
                self.current = join;
            }
            S::While { condition, body } => self.loop_body(None, condition, None, body, o)?,
            S::For {
                initializer,
                condition,
                update,
                body,
            } => self.loop_body(Some(initializer), condition, Some(update), body, o)?,
            S::Break | S::Continue => {
                let (exit, again) = self
                    .loops
                    .last()
                    .copied()
                    .ok_or_else(|| Error::new("loop control outside loop", o))?;
                self.jump(
                    if matches!(s.kind, S::Break) {
                        exit
                    } else {
                        again
                    },
                    o,
                );
                self.current = self.new_block(derived(o, "after-loop-control"));
            }
            S::ArrayInitialize { array, value } => {
                let array = self.expr(array)?;
                let value = self.expr(value)?;
                self.emit(InstructionKind::ArrayInitialize { array, value }, o);
            }
            S::ArrayRetain { value } => {
                let value = self.expr(value)?;
                self.emit(InstructionKind::ArrayRetain { value }, o);
            }
            S::ArrayFree { value } => {
                let value = self.expr(value)?;
                self.emit(InstructionKind::ArrayFree { value }, o);
            }
            S::StringManage { value, retain } => {
                let value = self.expr(value)?;
                self.emit(
                    InstructionKind::StringManage {
                        value,
                        retain: *retain,
                    },
                    o,
                );
            }
            S::ArrayRangeCheck { length, start, end } => {
                let length = self.expr(length)?;
                let start = self.expr(start)?;
                let end = self.expr(end)?;
                self.emit(InstructionKind::ArrayRangeCheck { length, start, end }, o);
            }
        }
        Ok(())
    }
    fn loop_body(
        &mut self,
        init: Option<&ir::Statement>,
        condition: &ir::Expr,
        update: Option<&ir::Statement>,
        body: &[ir::Statement],
        o: Origin,
    ) -> Result<(), Error> {
        if let Some(init) = init {
            self.statement(init)?;
        }
        let head = self.new_block(derived(o, "loop-condition"));
        let body_block = self.new_block(derived(o, "loop-body"));
        let exit = self.new_block(derived(o, "loop-exit"));
        let again = if update.is_some() {
            self.new_block(derived(o, "for-update"))
        } else {
            head
        };
        self.jump(head, derived(o, "loop-entry"));
        self.current = head;
        let condition = self.expr(condition)?;
        self.end(
            TerminatorKind::Branch {
                condition,
                then_block: body_block,
                else_block: exit,
            },
            o,
        );
        self.loops.push((exit, again));
        self.current = body_block;
        self.statements(body)?;
        self.jump(again, derived(o, "loop-back"));
        if let Some(update) = update {
            self.current = again;
            self.statement(update)?;
            self.jump(head, derived(o, "loop-back"));
        }
        self.loops.pop();
        self.current = exit;
        Ok(())
    }
    fn arguments(&mut self, args: &[ir::Expr]) -> Result<Vec<LocalId>, Error> {
        args.iter().map(|e| self.expr(e)).collect()
    }
    fn expr(&mut self, e: &ir::Expr) -> Result<LocalId, Error> {
        let o = source(e.id, e.span);
        let op = match &e.kind {
            E::Boolean(v) => Operation::Literal(Literal::Boolean(*v)),
            E::String(v) => Operation::Literal(Literal::String(v.clone())),
            E::Integer(v) => Operation::Literal(Literal::Integer(*v)),
            E::Float { text } => Operation::Literal(Literal::Float(text.clone())),
            E::Variable { id, .. } => Operation::Copy(self.lookup(*id, o)?),
            E::Constant { value, .. } => Operation::Copy(self.expr(value)?),
            E::Unary { op, value } => Operation::Unary {
                op: *op,
                value: self.expr(value)?,
            },
            E::Binary { op, left, right } => {
                let left = self.expr(left)?;
                let right = self.expr(right)?;
                Operation::Binary {
                    op: *op,
                    left,
                    right,
                }
            }
            E::Logical { op, left, right } => {
                let left = self.expr(left)?;
                let result = self.local(e.ty.clone(), LocalKind::Temporary);
                self.assign(
                    result,
                    Operation::Copy(left),
                    derived(o, "short-circuit-result"),
                );
                let rhs = self.new_block(derived(o, "short-circuit-rhs"));
                let join = self.new_block(derived(o, "short-circuit-join"));
                let (then_block, else_block) = match op {
                    ir::LogicalOp::And => (rhs, join),
                    ir::LogicalOp::Or => (join, rhs),
                };
                self.end(
                    TerminatorKind::Branch {
                        condition: left,
                        then_block,
                        else_block,
                    },
                    o,
                );
                self.current = rhs;
                let right = self.expr(right)?;
                self.assign(
                    result,
                    Operation::Copy(right),
                    derived(o, "short-circuit-result"),
                );
                self.jump(join, derived(o, "short-circuit-join"));
                self.current = join;
                return Ok(result);
            }
            E::ConvertInteger {
                value,
                from,
                to,
                syntax,
            } => Operation::ConvertInteger {
                value: self.expr(value)?,
                from: *from,
                to: *to,
                syntax: *syntax,
            },
            E::ConvertNumeric {
                value,
                from,
                to,
                mode,
                syntax,
            } => Operation::ConvertNumeric {
                value: self.expr(value)?,
                from: *from,
                to: *to,
                mode: *mode,
                syntax: *syntax,
            },
            E::Array(values) => Operation::Array(self.arguments(values)?),
            E::Construct {
                type_id,
                base,
                fields,
                ..
            } => {
                let base = base.as_ref().map(|e| self.expr(e)).transpose()?;
                let fields = fields
                    .iter()
                    .map(|f| Ok((f.id, self.expr(&f.value)?)))
                    .collect::<Result<_, Error>>()?;
                Operation::Construct {
                    ty: *type_id,
                    base,
                    fields,
                }
            }
            E::FieldAccess {
                type_id,
                field_id,
                base,
                ..
            } => Operation::Field {
                base: self.expr(base)?,
                ty: *type_id,
                field: *field_id,
            },
            E::Index { base, index } => {
                let base = self.expr(base)?;
                let index = self.expr(index)?;
                Operation::Index { base, index }
            }
            E::Call {
                function_id,
                arguments,
                ..
            } => Operation::Call {
                function: *function_id,
                arguments: self.arguments(arguments)?,
                ownership: self.ownership(*function_id, o)?,
            },
            E::ArrayLength { value } => Operation::ArrayLength(self.expr(value)?),
            E::ArgumentCount => Operation::ArgumentCount,
            E::ArgumentGet { index } => Operation::ArgumentGet(self.expr(index)?),
            E::ReadLine { max_bytes } => Operation::ReadLine(self.expr(max_bytes)?),
            E::StringByteLength { value } => Operation::StringByteLength(self.expr(value)?),
            E::StringConcat { left, right } => {
                let left = self.expr(left)?;
                let right = self.expr(right)?;
                Operation::StringConcat { left, right }
            }
            E::ArrayAllocate {
                length,
                element_width,
            } => Operation::ArrayAllocate {
                length: self.expr(length)?,
                element_width: *element_width,
            },
            E::ArrayReleaseOwner { value } => Operation::ArrayReleaseOwner(self.expr(value)?),
            E::ArrayRepeat { .. } | E::ArrayCopy { .. } | E::Let { .. } | E::Conditional { .. } => {
                return Err(Error::new(
                    "incomplete HIR: common expansion is required before MIR lowering",
                    o,
                ));
            }
        };
        Ok(self.value(&e.ty, op, o))
    }
}
