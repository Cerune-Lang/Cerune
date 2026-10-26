//! 所有の処理を通常の束縛・関数・分岐に展開し、生成先ごとの差を防ぎます。
use super::aggregates::{visit_body, visit_expr, visit_program};
use super::*;
mod arrays;

pub(super) fn lower(program: &mut Program) {
    let mut needed = program.first_dynamic_array_span().is_some();
    for f in &mut program.function_definitions {
        visit_body(
            &mut f.body,
            &mut |e| {
                needed |= matches!(
                    e.kind,
                    ExprKind::StringConcat { .. }
                        | ExprKind::ArgumentGet { .. }
                        | ExprKind::ReadLine { .. }
                        | ExprKind::FileReadText { .. }
                );
            },
            &mut |_| {},
        );
    }
    visit_body(
        &mut program.statements,
        &mut |e| {
            needed |= matches!(
                e.kind,
                ExprKind::StringConcat { .. }
                    | ExprKind::ArgumentGet { .. }
                    | ExprKind::ReadLine { .. }
                    | ExprKind::FileReadText { .. }
            );
        },
        &mut |_| {},
    );
    let mut node = 0;
    let mut binding = 0;
    visit_program(
        program,
        &mut |e| {
            node = node.max(e.id.0 + 1);
            if let ExprKind::Variable { id, .. } = e.kind {
                binding = binding.max(id.0 + 1);
            }
        },
        &mut |_| {},
    );
    if !needed {
        return;
    }
    visit_program(program, &mut |_| {}, &mut |s| {
        node = node.max(s.id.0 + 1);
        if let StatementKind::Binding { id, .. } = s.kind {
            binding = binding.max(id.0 + 1);
        }
    });
    for f in &program.function_definitions {
        for p in &f.parameters {
            binding = binding.max(p.id.0 + 1);
        }
    }
    let mut l = Lowerer {
        node,
        binding,
        base: program.function_definitions.len(),
        arguments: program
            .function_definitions
            .iter()
            .map(FunctionDefinition::argument_ownership)
            .collect(),
        types: program.type_definitions.clone(),
        functions: vec![],
        managers: vec![],
        scopes: vec![],
        loops: vec![],
    };
    for f in &mut program.function_definitions {
        // 利用者の関数は準備済みの所有を受け取ります。入口で再度保持しません。
        // 比較・表示・matchの生成関数は、呼び出し中だけ引数を借ります。
        l.scopes
            .push(if f.argument_ownership() == ArgumentOwnership::Owned {
                f.parameters.clone()
            } else {
                vec![]
            });
        let mut body = l.body(std::mem::take(&mut f.body));
        l.cleanup(0, &mut body);
        l.scopes.pop();
        f.body = body;
    }
    program.statements = l.body(std::mem::take(&mut program.statements));
    program.function_definitions.extend(l.functions);
}
// 借用した値か、読み取り後に解放する一時的な所有値かを区別します。
struct ReadValue {
    value: Expr,
    owned: bool,
}
struct Lowerer {
    node: usize,
    binding: usize,
    base: usize,
    arguments: Vec<ArgumentOwnership>,
    types: Vec<TypeDefinition>,
    functions: Vec<FunctionDefinition>,
    managers: Vec<(Type, bool, FunctionId, String)>,
    scopes: Vec<Vec<Parameter>>,
    loops: Vec<usize>,
}
impl Lowerer {
    fn expr(&mut self, ty: Type, kind: ExprKind, span: Span) -> Expr {
        let id = NodeId(self.node);
        self.node += 1;
        Expr { id, ty, kind, span }
    }
    fn stmt(&mut self, kind: StatementKind, span: Span) -> Statement {
        let id = NodeId(self.node);
        self.node += 1;
        Statement { id, kind, span }
    }
    fn param(&mut self, ty: Type, span: Span) -> Parameter {
        let id = BindingId(self.binding);
        self.binding += 1;
        Parameter {
            id,
            name: format!("$owned{}", id.0),
            ty,
            span,
        }
    }
    fn var(&mut self, p: &Parameter) -> Expr {
        self.expr(
            p.ty.clone(),
            ExprKind::Variable {
                id: p.id,
                name: p.name.clone(),
            },
            p.span,
        )
    }
    fn copy(&mut self, value: &Expr) -> Expr {
        let mut value = value.clone();
        visit_expr(&mut value, &mut |e| {
            e.id = NodeId(self.node);
            self.node += 1;
        });
        value
    }
    fn int(&mut self, value: usize, span: Span) -> Expr {
        self.expr(
            Type::Integer(IntegerType::I64),
            ExprKind::Integer(value as i128),
            span,
        )
    }
    fn binary(&mut self, ty: Type, op: BinaryOp, left: Expr, right: Expr, span: Span) -> Expr {
        self.expr(
            ty,
            ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )
    }
    fn field(&mut self, base: Expr, id: TypeId, f: &FieldDefinition) -> Expr {
        let span = base.span;
        self.expr(
            f.ty.clone(),
            ExprKind::FieldAccess {
                type_id: id,
                field_id: f.id,
                field_name: f.name.clone(),
                base: Box::new(base),
            },
            span,
        )
    }
    fn bind(&mut self, value: Expr, mutable: bool, body: &mut Vec<Statement>) -> Parameter {
        self.bind_value(value, mutable, false, body)
    }
    fn bind_read(&mut self, value: Expr, body: &mut Vec<Statement>) -> Parameter {
        self.bind_value(value, false, true, body)
    }
    fn bind_value(
        &mut self,
        value: Expr,
        mutable: bool,
        borrowed: bool,
        body: &mut Vec<Statement>,
    ) -> Parameter {
        let mut p = self.param(value.ty.clone(), value.span);
        if borrowed {
            p.name = format!("$read{}", p.id.0);
        }
        body.push(self.stmt(
            StatementKind::Binding {
                borrowed,
                id: p.id,
                mutable,
                name: p.name.clone(),
                ty: p.ty.clone(),
                value,
            },
            p.span,
        ));
        p
    }
    fn target(p: &Parameter) -> AssignmentTarget {
        AssignmentTarget {
            id: p.id,
            name: p.name.clone(),
            root_ty: p.ty.clone(),
            projections: vec![],
            ty: p.ty.clone(),
        }
    }
    fn owns(&self, ty: &Type) -> bool {
        match ty {
            Type::String | Type::DynamicArray { .. } => true,
            Type::Array { element, length } => *length != 0 && self.owns(element),
            Type::Named(id) => self.types[id.0].fields.iter().any(|f| self.owns(&f.ty)),
            _ => false,
        }
    }
    fn function(
        &mut self,
        kind: LoweringKind,
        parameters: Vec<Parameter>,
        ret: ReturnType,
        span: Span,
    ) -> (FunctionId, String) {
        let id = FunctionId(self.base + self.functions.len());
        let name = format!("$ownership{}", id.0);
        self.functions.push(FunctionDefinition {
            lowering: Some(kind),
            generic_origin: None,
            id,
            name: name.clone(),
            parameters,
            return_type: ret,
            body: vec![],
            span,
        });
        (id, name)
    }
    // 管理関数は保持／解放を本文に直接生成し、入口・出口の管理を重ねません。
    fn manage(&mut self, value: Expr, retain: bool, body: &mut Vec<Statement>) {
        if !self.owns(&value.ty) {
            return;
        }
        let span = value.span;
        if value.ty == Type::String {
            body.push(self.stmt(StatementKind::StringManage { value, retain }, span));
            return;
        }
        let ty = value.ty.clone();
        let found = self
            .managers
            .iter()
            .find(|(t, r, _, _)| *t == ty && *r == retain)
            .cloned();
        let (id, name) = if let Some((_, _, id, name)) = found {
            (id, name)
        } else {
            let p = self.param(ty.clone(), span);
            let (id, name) = self.function(
                if retain {
                    LoweringKind::OwnershipRetain
                } else {
                    LoweringKind::OwnershipRelease
                },
                vec![p.clone()],
                ReturnType::Void,
                span,
            );
            self.managers.push((ty.clone(), retain, id, name.clone()));
            let mut b = vec![];
            match ty {
                Type::DynamicArray { element } => {
                    let value = self.var(&p);
                    if retain {
                        b.push(self.stmt(StatementKind::ArrayRetain { value }, span));
                    } else {
                        let condition = self.expr(
                            Type::Bool,
                            ExprKind::ArrayReleaseOwner {
                                value: Box::new(value),
                            },
                            span,
                        );
                        let value = self.var(&p);
                        let length = self.length(value);
                        let mut cleanup = vec![];
                        self.array_loop(
                            length,
                            true,
                            span,
                            |this, index| {
                                let base = this.var(&p);
                                let value = this.expr(
                                    *element,
                                    ExprKind::Index {
                                        base: Box::new(base),
                                        index: Box::new(index),
                                    },
                                    span,
                                );
                                let mut statements = vec![];
                                this.manage(value, false, &mut statements);
                                statements
                            },
                            &mut cleanup,
                        );
                        let value = self.var(&p);
                        cleanup.push(self.stmt(StatementKind::ArrayFree { value }, span));
                        b.push(self.stmt(
                            StatementKind::If {
                                condition,
                                then_body: cleanup,
                                else_body: vec![],
                            },
                            span,
                        ));
                    }
                }
                Type::Array { element, length } => {
                    let start = self.int(if retain { 0 } else { length }, span);
                    let index = self.param(Type::Integer(IntegerType::I64), span);
                    let initializer = self.stmt(
                        StatementKind::Binding {
                            borrowed: false,
                            id: index.id,
                            mutable: true,
                            name: index.name.clone(),
                            ty: index.ty.clone(),
                            value: start,
                        },
                        span,
                    );
                    let i = self.var(&index);
                    let limit = self.int(if retain { length } else { 0 }, span);
                    let condition = self.binary(
                        Type::Bool,
                        if retain {
                            BinaryOp::Less
                        } else {
                            BinaryOp::Greater
                        },
                        i,
                        limit,
                        span,
                    );
                    let i = self.var(&index);
                    let one = self.int(1, span);
                    let next = self.binary(
                        index.ty.clone(),
                        if retain {
                            BinaryOp::Add
                        } else {
                            BinaryOp::Subtract
                        },
                        i,
                        one,
                        span,
                    );
                    let update = self.stmt(
                        StatementKind::Assignment {
                            target: Self::target(&index),
                            value: next,
                        },
                        span,
                    );
                    let base = self.var(&p);
                    let mut i = self.var(&index);
                    if !retain {
                        let one = self.int(1, span);
                        i = self.binary(index.ty.clone(), BinaryOp::Subtract, i, one, span);
                    }
                    let item = self.expr(
                        *element,
                        ExprKind::Index {
                            base: Box::new(base),
                            index: Box::new(i),
                        },
                        span,
                    );
                    let mut loop_body = vec![];
                    self.manage(item, retain, &mut loop_body);
                    b.push(self.stmt(
                        StatementKind::For {
                            initializer: Box::new(initializer),
                            condition,
                            update: Box::new(update),
                            body: loop_body,
                        },
                        span,
                    ));
                }
                Type::Named(type_id) => {
                    let def = self.types[type_id.0].clone();
                    if let Some(variants) = def.variants {
                        for (tag, variant) in variants.iter().enumerate() {
                            let base = self.var(&p);
                            let tag_value = self.field(base, type_id, &def.fields[0]);
                            let expected = self.int(tag, span);
                            let condition =
                                self.binary(Type::Bool, BinaryOp::Equal, tag_value, expected, span);
                            let fields: Vec<_> = def
                                .fields
                                .iter()
                                .filter(|f| f.name.starts_with(&format!("${variant}$")))
                                .cloned()
                                .collect();
                            let mut branch = vec![];
                            self.manage_fields(&p, type_id, fields, retain, &mut branch);
                            b.push(self.stmt(
                                StatementKind::If {
                                    condition,
                                    then_body: branch,
                                    else_body: vec![],
                                },
                                span,
                            ));
                        }
                    } else {
                        self.manage_fields(&p, type_id, def.fields, retain, &mut b);
                    }
                }
                _ => unreachable!(),
            }
            b.push(self.stmt(StatementKind::Return { value: None }, span));
            self.functions[id.0 - self.base].body = b;
            (id, name)
        };
        body.push(self.stmt(
            StatementKind::Call {
                function_id: id,
                function_name: name,
                arguments: vec![value],
            },
            span,
        ));
    }
    fn manage_fields(
        &mut self,
        p: &Parameter,
        id: TypeId,
        mut fields: Vec<FieldDefinition>,
        retain: bool,
        body: &mut Vec<Statement>,
    ) {
        if !retain {
            fields.reverse();
        }
        for f in fields {
            let base = self.var(p);
            let value = self.field(base, id, &f);
            self.manage(value, retain, body);
        }
    }
    fn cleanup(&mut self, from: usize, body: &mut Vec<Statement>) {
        let values: Vec<_> = self.scopes[from..]
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev().cloned())
            .collect();
        for p in values {
            let value = self.var(&p);
            self.manage(value, false, body);
        }
    }
    // 式を値を返す関数にすることで、while/for条件の評価位置を保ちます。
    fn wrap(&mut self, mut value: Expr) -> Expr {
        let span = value.span;
        let ty = value.ty.clone();
        let mut captures = Vec::<Parameter>::new();
        visit_expr(&mut value, &mut |e| {
            if let ExprKind::Variable { id, name } = &e.kind
                && !captures.iter().any(|p| p.id == *id)
            {
                captures.push(Parameter {
                    id: *id,
                    name: name.clone(),
                    ty: e.ty.clone(),
                    span: e.span,
                });
            }
        });
        let parameters: Vec<_> = captures
            .iter()
            .map(|p| self.param(p.ty.clone(), p.span))
            .collect();
        visit_expr(&mut value, &mut |e| {
            if let ExprKind::Variable { id, name } = &mut e.kind {
                let i = captures.iter().position(|p| p.id == *id).unwrap();
                *id = parameters[i].id;
                *name = parameters[i].name.clone();
            }
        });
        let (id, name) = self.function(
            LoweringKind::OwnershipExpression,
            parameters,
            ReturnType::Value(ty.clone()),
            span,
        );
        let mut body = vec![];
        let result = self.owned(value, &mut body);
        body.push(self.stmt(
            StatementKind::Return {
                value: Some(result),
            },
            span,
        ));
        self.functions[id.0 - self.base].body = body;
        let arguments = captures.iter().map(|p| self.var(p)).collect();
        self.expr(
            ty,
            ExprKind::Call {
                function_id: id,
                function_name: name,
                arguments,
            },
            span,
        )
    }
    // 束縛の読み取りでは所有を増やしません。一時値はconsumer後に解放します。
    fn read(&mut self, mut value: Expr, body: &mut Vec<Statement>) -> ReadValue {
        let mut parent = None;
        match &mut value.kind {
            ExprKind::Variable { .. }
            | ExprKind::Constant { .. }
            | ExprKind::Boolean(_)
            | ExprKind::String(_)
            | ExprKind::Integer(_)
            | ExprKind::Float { .. } => {}
            ExprKind::FieldAccess { base, .. } => {
                let read = self.read(*base.clone(), body);
                **base = self.copy(&read.value);
                parent = Some(read);
            }
            ExprKind::Index { base, index } => {
                let read = self.read(*base.clone(), body);
                **base = self.copy(&read.value);
                **index = self.owned(*index.clone(), body);
                parent = Some(read);
            }
            _ => {
                return ReadValue {
                    value: self.owned(value, body),
                    owned: true,
                };
            }
        }
        let owned = parent.as_ref().is_some_and(|p| p.owned);
        let p = if owned {
            self.bind(value, false, body)
        } else {
            self.bind_read(value, body)
        };
        if let Some(parent) = parent.filter(|p| p.owned) {
            // 一時的な複合値からの抽出は、選んだ値を保持してから元を解放します。
            // 無関係なフィールドの寿命は延ばさず、次のオペランドの予算を保ちます。
            let selected = self.var(&p);
            self.manage(selected, true, body);
            self.manage(parent.value, false, body);
        }
        ReadValue {
            value: self.var(&p),
            owned,
        }
    }
    fn owned(&mut self, mut value: Expr, body: &mut Vec<Statement>) -> Expr {
        let span = value.span;
        if matches!(
            value.kind,
            ExprKind::Variable { .. }
                | ExprKind::Constant { .. }
                | ExprKind::FieldAccess { .. }
                | ExprKind::Index { .. }
        ) {
            let read = self.read(value, body);
            if read.owned {
                return read.value;
            }
            return self.copy_owned(read.value, body);
        }
        if let ExprKind::ArrayRepeat { value: seed, count } = value.kind {
            let seed = self.read(*seed, body);
            let count = self.owned(*count, body);
            let input = self.copy(&seed.value);
            let result = self.repeat(input, count, span, body);
            if seed.owned {
                self.manage(seed.value, false, body);
            }
            return result;
        }
        if let ExprKind::ArrayCopy {
            value: source,
            range,
        } = value.kind
        {
            let source = self.read(*source, body);
            let range =
                range.map(|(start, end)| (self.owned(*start, body), self.owned(*end, body)));
            let input = self.copy(&source.value);
            let result = self.copy_range(input, range, span, body);
            if source.owned {
                self.manage(source.value, false, body);
            }
            return result;
        }
        if let ExprKind::Logical { op, left, right } = value.kind {
            let left = self.owned(*left, body);
            let result = self.bind(left, true, body);
            let mut condition = self.var(&result);
            if op == LogicalOp::Or {
                condition = self.expr(
                    Type::Bool,
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        value: Box::new(condition),
                    },
                    span,
                );
            }
            let mut branch = vec![];
            let right = self.owned(*right, &mut branch);
            branch.push(self.stmt(
                StatementKind::Assignment {
                    target: Self::target(&result),
                    value: right,
                },
                span,
            ));
            body.push(self.stmt(
                StatementKind::If {
                    condition,
                    then_body: branch,
                    else_body: vec![],
                },
                span,
            ));
            return self.var(&result);
        }
        let fresh = matches!(
            value.kind,
            ExprKind::Call { .. }
                | ExprKind::StringConcat { .. }
                | ExprKind::ArgumentGet { .. }
                | ExprKind::ReadLine { .. }
                | ExprKind::FileReadText { .. }
                | ExprKind::Array(_)
                | ExprKind::Construct { base: None, .. }
        );
        let transfer = match &value.kind {
            ExprKind::Call { function_id, .. } => {
                self.arguments[function_id.0] == ArgumentOwnership::Owned
            }
            ExprKind::Array(_) | ExprKind::Construct { base: None, .. } => true,
            _ => false,
        };
        let mut children = vec![];
        let mut child = |e: &mut Expr, own: bool, this: &mut Self, body: &mut Vec<Statement>| {
            *e = if own {
                let owned = this.owned(e.clone(), body);
                children.push(this.copy(&owned));
                owned
            } else {
                let read = this.read(e.clone(), body);
                if read.owned {
                    children.push(this.copy(&read.value));
                }
                read.value
            };
        };
        match &mut value.kind {
            ExprKind::ArrayLength { value }
            | ExprKind::ArgumentGet { index: value }
            | ExprKind::ReadLine { max_bytes: value }
            | ExprKind::StringByteLength { value }
            | ExprKind::ConvertNumeric { value, .. }
            | ExprKind::ConvertInteger { value, .. }
            | ExprKind::Unary { value, .. } => child(value, false, self, body),
            ExprKind::StringByteAt {
                value: left,
                index: right,
            }
            | ExprKind::FileReadText {
                name: left,
                max_bytes: right,
            }
            | ExprKind::StringConcat { left, right }
            | ExprKind::Binary { left, right, .. } => {
                child(left, false, self, body);
                child(right, false, self, body);
            }
            ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    child(base, true, self, body);
                }
                for f in fields {
                    child(&mut f.value, true, self, body);
                }
            }
            ExprKind::Array(values) => {
                for v in values {
                    child(v, true, self, body);
                }
            }
            ExprKind::Call { arguments, .. } => {
                for v in arguments {
                    child(v, transfer, self, body);
                }
            }
            ExprKind::Let { .. } | ExprKind::Conditional { .. } => {
                unreachable!("aggregate lowering precedes ownership")
            }
            _ => {}
        }
        let p = self.bind(value, false, body);
        if !fresh {
            let v = self.var(&p);
            self.manage(v, true, body);
        }
        // 引数や新しい複合値へ渡した所有は、ここで重ねて解放しません。
        if !transfer {
            for v in children.into_iter().rev() {
                self.manage(v, false, body);
            }
        }
        self.var(&p)
    }
    fn replace(&mut self, old: Expr, new: Expr) -> Expr {
        if !self.owns(&new.ty) {
            return new;
        }
        let span = new.span;
        let ty = new.ty.clone();
        let a = self.param(ty.clone(), span);
        let b = self.param(ty.clone(), span);
        let (id, name) = self.function(
            LoweringKind::OwnershipReplace,
            vec![a.clone(), b.clone()],
            ReturnType::Value(ty.clone()),
            span,
        );
        let mut body = vec![];
        let old_value = self.var(&a);
        self.manage(old_value, false, &mut body);
        let result = self.var(&b);
        body.push(self.stmt(
            StatementKind::Return {
                value: Some(result),
            },
            span,
        ));
        self.functions[id.0 - self.base].body = body;
        self.expr(
            ty,
            ExprKind::Call {
                function_id: id,
                function_name: name,
                arguments: vec![old, new],
            },
            span,
        )
    }
    fn body(&mut self, source: Vec<Statement>) -> Vec<Statement> {
        self.scopes.push(vec![]);
        let mut out = vec![];
        for s in source {
            self.statement(s, &mut out);
        }
        self.cleanup(self.scopes.len() - 1, &mut out);
        self.scopes.pop();
        out
    }
    fn statement(&mut self, mut s: Statement, out: &mut Vec<Statement>) {
        let span = s.span;
        match &mut s.kind {
            StatementKind::ArrayInitialize { .. }
            | StatementKind::ArrayRetain { .. }
            | StatementKind::ArrayFree { .. }
            | StatementKind::ArrayRangeCheck { .. } => {
                unreachable!("array storage statements are generated after ownership expansion")
            }
            StatementKind::Binding {
                id,
                name,
                ty,
                value,
                ..
            } => {
                *value = self.wrap(value.clone());
                self.scopes.last_mut().unwrap().push(Parameter {
                    id: *id,
                    name: name.clone(),
                    ty: ty.clone(),
                    span,
                });
            }
            StatementKind::Assignment { target, value } => {
                let mut old = self.expr(
                    target.root_ty.clone(),
                    ExprKind::Variable {
                        id: target.id,
                        name: target.name.clone(),
                    },
                    span,
                );
                for AssignmentProjection::Index {
                    index,
                    element,
                    span,
                    ..
                }
                | AssignmentProjection::DynamicIndex {
                    index,
                    element,
                    span,
                } in &mut target.projections
                {
                    let evaluated = self.wrap(index.clone());
                    let p = self.bind(evaluated, false, out);
                    *index = self.var(&p);
                    let i = self.var(&p);
                    old = self.expr(
                        element.clone(),
                        ExprKind::Index {
                            base: Box::new(old),
                            index: Box::new(i),
                        },
                        *span,
                    );
                    // 各段階の範囲検査を、次の添字や右辺より先に実行します。借用一時値です。
                    let checked = self.copy(&old);
                    self.bind(checked, false, out);
                }
                let new = self.wrap(value.clone());
                *value = self.replace(old, new);
            }
            StatementKind::FileWriteNew {
                name,
                text,
                max_bytes,
            } => {
                let mut owners = vec![];
                for operand in [name, text, max_bytes] {
                    let read = self.read(operand.clone(), out);
                    if read.owned {
                        owners.push(self.copy(&read.value));
                    }
                    *operand = read.value;
                }
                out.push(s);
                for owner in owners.into_iter().rev() {
                    self.manage(owner, false, out);
                }
                return;
            }
            StatementKind::Print { value } | StatementKind::Write { value, .. } => {
                let read = self.read(value.clone(), out);
                *value = self.copy(&read.value);
                out.push(s);
                if read.owned {
                    self.manage(read.value, false, out);
                }
                return;
            }
            StatementKind::Call {
                function_id,
                arguments,
                ..
            } => {
                let transfer = self.arguments[function_id.0] == ArgumentOwnership::Owned;
                let mut owners = vec![];
                for a in arguments {
                    *a = if transfer {
                        let evaluated = self.wrap(a.clone());
                        let p = self.bind(evaluated, false, out);
                        self.var(&p)
                    } else {
                        let read = self.read(a.clone(), out);
                        if read.owned {
                            owners.push(self.copy(&read.value));
                        }
                        read.value
                    };
                }
                out.push(s);
                for value in owners.into_iter().rev() {
                    self.manage(value, false, out);
                }
                return;
            }
            StatementKind::Return { value } => {
                if let Some(v) = value {
                    let evaluated = self.wrap(v.clone());
                    let p = self.bind(evaluated, false, out);
                    *v = self.var(&p);
                }
                self.cleanup(0, out);
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                *condition = self.wrap(condition.clone());
                *then_body = self.body(std::mem::take(then_body));
                *else_body = self.body(std::mem::take(else_body));
            }
            StatementKind::While { condition, body } => {
                *condition = self.wrap(condition.clone());
                self.loops.push(self.scopes.len());
                *body = self.body(std::mem::take(body));
                self.loops.pop();
            }
            StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.scopes.push(vec![]);
                let mut init = vec![];
                self.statement(*initializer.clone(), &mut init);
                assert_eq!(
                    init.len(),
                    1,
                    "for initializer is a binding or scalar assignment"
                );
                **initializer = init.pop().unwrap();
                *condition = self.wrap(condition.clone());
                let mut updates = vec![];
                self.statement(*update.clone(), &mut updates);
                assert_eq!(updates.len(), 1, "for update is a scalar assignment");
                **update = updates.pop().unwrap();
                self.loops.push(self.scopes.len());
                *body = self.body(std::mem::take(body));
                self.loops.pop();
                out.push(s);
                self.cleanup(self.scopes.len() - 1, out);
                self.scopes.pop();
                return;
            }
            StatementKind::Flush => {}
            StatementKind::Break | StatementKind::Continue => {
                self.cleanup(*self.loops.last().unwrap(), out)
            }
            StatementKind::StringManage { .. } => unreachable!("ownership is lowered once"),
        }
        out.push(s);
    }
}
