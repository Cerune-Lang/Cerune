//! 元HIRへの参照と継続位置を保持する実行機械。IRや命令列を新しく構築しません。
use super::*;

fn push<T>(values: &mut Vec<T>, value: T) -> Result<()> {
    values
        .try_reserve(1)
        .map_err(|_| Fault::failure(FailureCode::AllocationFailed))?;
    values.push(value);
    Ok(())
}
fn origin(s: &Statement) -> Origin {
    Origin {
        node_id: s.id,
        span: s.span,
    }
}
fn expr_origin(e: &Expr) -> Origin {
    Origin {
        node_id: e.id,
        span: e.span,
    }
}

// 各frameは自分の式の途中値と制御位置だけを保持します。
struct Activation<'a> {
    function: Option<&'a ir::FunctionDefinition>,
    destination: Destination<'a>,
    counted: bool,
    bindings: Frame,
    values: Vec<Value>,
    pending: Vec<Task<'a>>,
}
#[derive(Clone, Copy)]
enum Destination<'a> {
    Entry,
    Void(Option<Origin>),
    Value(&'a Expr),
}
impl Destination<'_> {
    fn origin(self) -> Option<Origin> {
        match self {
            Self::Entry => None,
            Self::Void(o) => o,
            Self::Value(e) => Some(expr_origin(e)),
        }
    }
}
// Operandsのusizeは、次の子式の番号と評価済み値の開始位置です。
enum Task<'a> {
    Main,
    Block(&'a [Statement]),
    Statement(&'a Statement),
    StatementOperands(&'a Statement, usize, usize),
    Expr(&'a Expr),
    ExprOperands(&'a Expr, usize, usize),
    Branch(&'a Statement),
    LoopTest(&'a Statement),
    LoopDecision(&'a Statement),
    LoopEnd(&'a Statement),
    ForClause(&'a Statement),
    Return(&'a Statement, bool),
    AssignNext(&'a Statement, usize, Vec<usize>),
    AssignIndex(&'a Statement, usize, Vec<usize>),
    AssignStore(&'a Statement, Vec<usize>),
}
impl Task<'_> {
    fn origin(&self) -> Option<Origin> {
        match self {
            Self::Main => None,
            Self::Block(body) => body.first().map(origin),
            Self::Expr(e) | Self::ExprOperands(e, ..) => Some(expr_origin(e)),
            Self::Statement(s)
            | Self::StatementOperands(s, ..)
            | Self::Branch(s)
            | Self::LoopTest(s)
            | Self::LoopDecision(s)
            | Self::LoopEnd(s)
            | Self::ForClause(s)
            | Self::Return(s, _)
            | Self::AssignNext(s, ..)
            | Self::AssignIndex(s, ..)
            | Self::AssignStore(s, ..) => Some(origin(s)),
        }
    }
}
struct Machine<'a> {
    active: Vec<Activation<'a>>,
    #[cfg(test)]
    fail_frame_after: Option<usize>,
}
pub(super) fn execute(executor: &mut Executor<'_, '_>) -> Result<()> {
    Machine::new().run(executor)
}
impl<'a> Machine<'a> {
    fn new() -> Self {
        Self {
            active: vec![],
            #[cfg(test)]
            fail_frame_after: None,
        }
    }
    fn run(&mut self, executor: &mut Executor<'a, '_>) -> Result<()> {
        let program = executor.program;
        let setup = (|| {
            let mut pending = vec![];
            push(&mut pending, Task::Main)?;
            push(&mut pending, Task::Block(&program.statements))?;
            push(
                &mut self.active,
                Activation {
                    function: None,
                    destination: Destination::Entry,
                    counted: false,
                    bindings: Frame::new(),
                    values: vec![],
                    pending,
                },
            )
        })();
        setup.map_err(|e: Fault| match program.statements.first() {
            Some(s) => e.at(s.id, s.span),
            None => e,
        })?;
        let result = self.drive(executor);
        // 失敗後にユーザーの処理を再開せず、実行器内部の計数だけを外します。
        while let Some(frame) = self.active.pop() {
            if frame.counted {
                executor.calls.as_mut().unwrap().leave();
            }
        }
        result
    }
    fn drive(&mut self, executor: &mut Executor<'a, '_>) -> Result<()> {
        while let Some(frame) = self.active.last_mut() {
            let fallback = frame.destination.origin();
            let result = if let Some(task) = frame.pending.pop() {
                let site = task.origin().or(fallback);
                self.step(executor, task).map_err(|e| match site {
                    Some(o) => e.at(o.node_id, o.span),
                    None => e,
                })
            } else {
                self.finish(executor, None).map_err(|e| match fallback {
                    Some(o) => e.at(o.node_id, o.span),
                    None => e,
                })
            };
            result?;
        }
        Ok(())
    }
    fn frame(&mut self) -> &mut Activation<'a> {
        self.active.last_mut().expect("active execution frame")
    }
    fn task(&mut self, task: Task<'a>) -> Result<()> {
        push(&mut self.frame().pending, task)
    }
    fn value(&mut self) -> Result<Value> {
        self.frame()
            .values
            .pop()
            .ok_or_else(|| Fault::invalid("missing expression value"))
    }
    fn put(&mut self, value: Value) -> Result<()> {
        push(&mut self.frame().values, value)
    }
    fn enter(
        &mut self,
        executor: &mut Executor<'a, '_>,
        id: ir::FunctionId,
        base: usize,
        destination: Destination<'a>,
    ) -> Result<()> {
        let function = executor
            .program
            .function_definitions
            .get(id.0)
            .filter(|f| f.id == id)
            .ok_or_else(|| Fault::invalid("unknown function"))?;
        if self.frame().values.len().checked_sub(base) != Some(function.parameters.len()) {
            return Err(Fault::invalid("incorrect argument count"));
        }
        let counted = function.lowering.is_none() && executor.calls.is_some();
        if counted {
            executor
                .calls
                .as_mut()
                .unwrap()
                .enter(call::Frame {
                    function: id,
                    origin: destination.origin().map(|o| call::Origin {
                        node_id: o.node_id,
                        span: o.span,
                    }),
                })
                .map_err(|failure| Fault {
                    kind: match failure {
                        call::EnterError::Depth(f) => ErrorKind::CallDepth(f),
                        call::EnterError::Allocation => {
                            ErrorKind::Runtime(FailureCode::AllocationFailed)
                        }
                    },
                    origin: destination.origin(),
                })?;
        }
        // 確保失敗でも、入った計数を必ず戻します。実際のframe追加まで本体は実行しません。
        let result = (|| {
            #[cfg(test)]
            if let Some(left) = &mut self.fail_frame_after {
                if *left == 0 {
                    return Err(Fault::failure(FailureCode::AllocationFailed));
                }
                *left -= 1;
            }
            let mut bindings = Frame::new();
            bindings
                .try_reserve(function.parameters.len())
                .map_err(|_| Fault::failure(FailureCode::AllocationFailed))?;
            for (p, value) in function
                .parameters
                .iter()
                .zip(self.frame().values.drain(base..))
            {
                value.check(&p.ty)?;
                if bindings
                    .insert(
                        p.id,
                        Binding {
                            value,
                            mutable: false,
                            initializer: None,
                        },
                    )
                    .is_some()
                {
                    return Err(Fault::invalid("duplicate parameter binding"));
                }
            }
            let mut pending = vec![];
            push(&mut pending, Task::Block(&function.body))?;
            push(
                &mut self.active,
                Activation {
                    function: Some(function),
                    destination,
                    counted,
                    bindings,
                    values: vec![],
                    pending,
                },
            )
        })();
        if result.is_err() && counted {
            executor.calls.as_mut().unwrap().leave();
        }
        result
    }
    fn finish(&mut self, executor: &mut Executor<'a, '_>, value: Option<Value>) -> Result<()> {
        let frame = self.frame();
        if let Some(f) = frame.function {
            match (&f.return_type, &value) {
                (ir::ReturnType::Void, None) => {}
                (ir::ReturnType::Value(ty), Some(value)) => value.check(ty)?,
                _ => return Err(Fault::invalid("return value does not match the function")),
            }
        }
        let frame = self.active.pop().unwrap();
        if frame.counted {
            executor.calls.as_mut().unwrap().leave();
        }
        match (frame.destination, value) {
            (Destination::Entry | Destination::Void(_), None) => Ok(()),
            (Destination::Value(e), Some(value)) => {
                value
                    .check(&e.ty)
                    .map_err(Fault::from)
                    .map_err(|e2| e2.at(e.id, e.span))?;
                self.put(value).map_err(|e2| e2.at(e.id, e.span))
            }
            (destination, _) => Err(Fault {
                kind: ErrorKind::InvalidIr("call result does not match use"),
                origin: destination.origin(),
            }),
        }
    }
    fn jump_loop(&mut self, continuing: bool) -> Result<()> {
        let pending = &mut self.frame().pending;
        for index in (0..pending.len()).rev() {
            match pending[index] {
                Task::LoopEnd(_) => {
                    pending.truncate(index + usize::from(continuing));
                    return Ok(());
                }
                Task::ForClause(_) => {
                    return Err(Fault::invalid("control flow escaped for clause"));
                }
                _ => {}
            }
        }
        Err(Fault::invalid("loop control escaped a function"))
    }
    fn step(&mut self, executor: &mut Executor<'a, '_>, task: Task<'a>) -> Result<()> {
        match task {
            Task::Main => {
                if let Some(main) = executor
                    .program
                    .function_definitions
                    .iter()
                    .find(|f| f.name == "main")
                {
                    if !main.parameters.is_empty() || main.return_type != ir::ReturnType::Void {
                        return Err(Fault::invalid(
                            "main must have no parameters and return void",
                        ));
                    }
                    self.enter(executor, main.id, 0, Destination::Void(None))?;
                }
            }
            Task::Block(body) => {
                if let Some((first, rest)) = body.split_first() {
                    self.task(Task::Block(rest))?;
                    self.task(Task::Statement(first))?;
                }
            }
            Task::Statement(s) => match &s.kind {
                S::If { condition, .. } => {
                    self.task(Task::Branch(s))?;
                    self.task(Task::Expr(condition))?;
                }
                S::While { .. } => self.task(Task::LoopTest(s))?,
                S::For { initializer, .. } => {
                    self.task(Task::LoopTest(s))?;
                    self.task(Task::ForClause(s))?;
                    self.task(Task::Statement(initializer))?;
                }
                S::Break => self.jump_loop(false)?,
                S::Continue => self.jump_loop(true)?,
                S::Return { value } => {
                    if self.frame().function.is_none()
                        || self
                            .frame()
                            .pending
                            .iter()
                            .any(|t| matches!(t, Task::ForClause(_)))
                    {
                        return Err(Fault::invalid("return escaped entry or for clause"));
                    }
                    self.task(Task::Return(s, value.is_some()))?;
                    if let Some(value) = value {
                        self.task(Task::Expr(value))?;
                    }
                }
                S::Assignment { target, .. } => {
                    let binding = self
                        .frame()
                        .bindings
                        .get(&target.id)
                        .ok_or_else(|| Fault::invalid("assignment to unknown binding"))?;
                    if !binding.mutable {
                        return Err(Fault::invalid("assignment to immutable binding"));
                    }
                    binding.value.check(&target.root_ty)?;
                    self.task(Task::AssignNext(s, 0, vec![]))?;
                }
                _ => {
                    let base = self.frame().values.len();
                    self.task(Task::StatementOperands(s, 0, base))?;
                }
            },
            Task::StatementOperands(s, next, base) => {
                if let Some(e) = statement_operand(s, next) {
                    self.task(Task::StatementOperands(s, next + 1, base))?;
                    self.task(Task::Expr(e))?;
                } else if let S::Call { function_id, .. } = &s.kind {
                    self.enter(
                        executor,
                        *function_id,
                        base,
                        Destination::Void(Some(origin(s))),
                    )?;
                } else {
                    let frame = self.frame();
                    executor.apply_statement(
                        s,
                        &mut frame.bindings,
                        &mut frame.values.drain(base..),
                    )?;
                }
            }
            Task::Expr(e) => {
                let base = self.frame().values.len();
                self.task(Task::ExprOperands(e, 0, base))?;
            }
            Task::ExprOperands(e, mut next, base) => {
                if let E::Logical { op, .. } = &e.kind
                    && next == 1
                {
                    let left = self
                        .frame()
                        .values
                        .last()
                        .ok_or_else(|| Fault::invalid("missing left operand"))?
                        .clone()
                        .boolean()?;
                    if matches!(op, ir::LogicalOp::And) && !left
                        || matches!(op, ir::LogicalOp::Or) && left
                    {
                        next = 2;
                    }
                }
                if let Some(child) = expression_operand(e, next) {
                    self.task(Task::ExprOperands(e, next + 1, base))?;
                    self.task(Task::Expr(child))?;
                } else if let E::Call { function_id, .. } = &e.kind {
                    self.enter(executor, *function_id, base, Destination::Value(e))?;
                } else {
                    let frame = self.frame();
                    let value =
                        executor.apply_expr(e, &frame.bindings, &mut frame.values.drain(base..))?;
                    value.check(&e.ty)?;
                    self.put(value)?;
                }
            }
            Task::Branch(s) => {
                let S::If {
                    then_body,
                    else_body,
                    ..
                } = &s.kind
                else {
                    unreachable!()
                };
                let yes = self.value()?.boolean()?;
                self.task(Task::Block(if yes { then_body } else { else_body }))?;
            }
            Task::LoopTest(s) => {
                let condition = match &s.kind {
                    S::While { condition, .. } | S::For { condition, .. } => condition,
                    _ => unreachable!(),
                };
                self.task(Task::LoopDecision(s))?;
                self.task(Task::Expr(condition))?;
            }
            Task::LoopDecision(s) => {
                if self.value()?.boolean()? {
                    let body = match &s.kind {
                        S::While { body, .. } | S::For { body, .. } => body,
                        _ => unreachable!(),
                    };
                    self.task(Task::LoopEnd(s))?;
                    self.task(Task::Block(body))?;
                }
            }
            Task::LoopEnd(s) => {
                self.task(Task::LoopTest(s))?;
                if let S::For { update, .. } = &s.kind {
                    self.task(Task::ForClause(s))?;
                    self.task(Task::Statement(update))?;
                }
            }
            Task::ForClause(_) => {}
            Task::Return(_, has_value) => {
                let value = if has_value { Some(self.value()?) } else { None };
                self.finish(executor, value)?;
            }
            Task::AssignNext(s, next, path) => {
                let S::Assignment { target, value } = &s.kind else {
                    unreachable!()
                };
                if let Some(projection) = target.projections.get(next) {
                    let index = match projection {
                        ir::AssignmentProjection::Index { index, .. }
                        | ir::AssignmentProjection::DynamicIndex { index, .. } => index,
                    };
                    self.task(Task::AssignIndex(s, next, path))?;
                    self.task(Task::Expr(index))?;
                } else {
                    self.task(Task::AssignStore(s, path))?;
                    self.task(Task::Expr(value))?;
                }
            }
            Task::AssignIndex(s, next, mut path) => {
                let S::Assignment { target, .. } = &s.kind else {
                    unreachable!()
                };
                let index = self.value()?.integer(IntegerType::I64)?;
                let (element, fixed, span) = match &target.projections[next] {
                    ir::AssignmentProjection::Index {
                        element,
                        length,
                        span,
                        ..
                    } => (element, Some(*length), span),
                    ir::AssignmentProjection::DynamicIndex { element, span, .. } => {
                        (element, None, span)
                    }
                };
                let root = &self
                    .frame()
                    .bindings
                    .get(&target.id)
                    .ok_or_else(|| Fault::invalid("unknown assignment root"))?
                    .value;
                let current = at_path(root, &path)?;
                current.check(&match fixed {
                    Some(length) => ir::Type::Array {
                        element: Box::new(element.clone()),
                        length,
                    },
                    None => ir::Type::DynamicArray {
                        element: Box::new(element.clone()),
                    },
                })?;
                let length = match current {
                    Value::Array { values, .. } => values.len(),
                    Value::DynamicArray { storage, .. } => storage.len(),
                    _ => return Err(Fault::invalid("array assignment type")),
                };
                push(
                    &mut path,
                    checked_index(index, length).map_err(|e| e.at(s.id, *span))?,
                )?;
                self.task(Task::AssignNext(s, next + 1, path))?;
            }
            Task::AssignStore(s, path) => {
                let S::Assignment { target, .. } = &s.kind else {
                    unreachable!()
                };
                let replacement = self.value()?;
                replacement.check(&target.ty)?;
                let root = &mut self
                    .frame()
                    .bindings
                    .get_mut(&target.id)
                    .ok_or_else(|| Fault::invalid("unknown assignment root"))?
                    .value;
                let destination = at_path(root, &path)?;
                replacement.check(&destination.ty())?;
                set_path(root, &path, replacement)?;
            }
        }
        Ok(())
    }
}
fn statement_operand(s: &Statement, index: usize) -> Option<&Expr> {
    match &s.kind {
        S::Binding { value, .. }
        | S::StringManage { value, .. }
        | S::Write { value, .. }
        | S::Print { value }
        | S::ArrayRetain { value }
        | S::ArrayFree { value } => (index == 0).then_some(value),
        S::ArrayInitialize { array, value } => [array, value].get(index).copied(),
        S::FileWriteNew {
            name: length,
            text: start,
            max_bytes: end,
        }
        | S::ArrayRangeCheck { length, start, end } => [length, start, end].get(index).copied(),
        S::Call { arguments, .. } => arguments.get(index),
        S::Flush => None,
        _ => unreachable!("structured statement operands"),
    }
}
fn expression_operand(e: &Expr, index: usize) -> Option<&Expr> {
    match &e.kind {
        E::ArrayAllocate { length, .. } => (index == 0).then_some(length),
        E::ArgumentGet { index: value }
        | E::ReadLine { max_bytes: value }
        | E::ArrayReleaseOwner { value }
        | E::Constant { value, .. }
        | E::ArrayLength { value }
        | E::StringByteLength { value }
        | E::ConvertNumeric { value, .. }
        | E::ConvertInteger { value, .. }
        | E::FieldAccess { base: value, .. }
        | E::Unary { value, .. } => (index == 0).then_some(value),
        E::FileReadText {
            name: left,
            max_bytes: right,
        }
        | E::StringConcat { left, right }
        | E::Logical { left, right, .. }
        | E::Binary { left, right, .. }
        | E::StringByteAt {
            value: left,
            index: right,
        }
        | E::Index {
            base: left,
            index: right,
        } => [left.as_ref(), right.as_ref()].get(index).copied(),
        E::Array(elements)
        | E::Call {
            arguments: elements,
            ..
        } => elements.get(index),
        E::Construct { base, fields, .. } => {
            if let Some(base) = base {
                if index == 0 {
                    Some(base)
                } else {
                    fields.get(index - 1).map(|f| &f.value)
                }
            } else {
                fields.get(index).map(|f| &f.value)
            }
        }
        E::Boolean(_)
        | E::String(_)
        | E::Integer(_)
        | E::Float { .. }
        | E::Variable { .. }
        | E::ArgumentCount
        | E::ArrayRepeat { .. }
        | E::ArrayCopy { .. }
        | E::Let { .. }
        | E::Conditional { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_allocation_failure_keeps_origin_output_and_balances_depth() {
        for (source, site) in [
            (
                r#"fn leaf()->void {print("body");} print("before"); leaf(); print("after");"#,
                "leaf();",
            ),
            (
                r#"print("before"); values:[string]=array_copy(["a"]); print(values);"#,
                "array_copy([\"a\"])",
            ),
        ] {
            let program = crate::compile_to_ir(source).unwrap();
            let mut output = Capture::default();
            let mut input = input::unconfigured();
            let arguments = Arguments::unconfigured();
            let mut executor = Executor {
                calls: Some(call::Stack::new(NonZeroU32::new(1).unwrap())),
                program: &program,
                output: &mut output,
                input: &mut input,
                files: &mut files::unconfigured(),
                program_arguments: &arguments,
                heap: StringHeap::new(program.string_heap_limit),
                arrays: ArrayHeap::new(program.array_heap_limit),
            };
            let mut machine = Machine::new();
            machine.fail_frame_after = Some(0);
            let failure = machine.run(&mut executor).unwrap_err();
            assert_eq!(
                failure.kind,
                ErrorKind::Runtime(FailureCode::AllocationFailed)
            );
            assert_eq!(executor.output.captured(), "before\n");
            let origin = failure.origin.unwrap();
            assert_eq!(&source[origin.span.start()..origin.span.end()], site);
            assert!(machine.active.is_empty());
            // 同じ計数器を再利用でき、次の実行を誤って深さ超過にしません。
            machine.fail_frame_after = None;
            machine.run(&mut executor).unwrap();
            assert!(executor.output.captured().starts_with("before\nbefore\n"));
            executor.heap.assert_empty();
            executor.arrays.assert_empty();
        }
    }
}
