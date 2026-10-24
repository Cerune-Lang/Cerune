//! 複合値の比較・表示とmatch式を、コピー・走査・分岐が見える共通IRへ展開します。
use super::*;
use crate::source::Span;

pub(super) fn lower(program: &mut Program) {
    let mut node = 0;
    let mut binding = 0;
    visit_program(
        program,
        &mut |expr| {
            node = node.max(expr.id.0 + 1);
            if let ExprKind::Variable { id, .. } | ExprKind::Let { binding: id, .. } = expr.kind {
                binding = binding.max(id.0 + 1);
            }
        },
        &mut |_| {},
    );
    visit_program(program, &mut |_| {}, &mut |stmt| {
        node_id_max(stmt, &mut binding);
    });
    // 文だけのNodeIdも含めます。
    visit_program(program, &mut |_| {}, &mut |stmt| {
        node = node.max(stmt.id.0 + 1);
    });
    for function in &program.function_definitions {
        for parameter in &function.parameters {
            binding = binding.max(parameter.id.0 + 1);
        }
    }
    let mut lowerer = Lowerer {
        node,
        binding,
        base: program.function_definitions.len(),
        types: program.type_definitions.clone(),
        functions: Vec::new(),
    };
    visit_program(
        program,
        &mut |expr| {
            if matches!(
                expr.kind,
                ExprKind::Let { .. } | ExprKind::Conditional { .. }
            ) {
                expr.kind = lowerer.expression_function(expr.clone()).kind;
            }
            if let ExprKind::Binary { op, left, right } = &expr.kind
                && matches!(
                    left.ty,
                    Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. }
                )
                && matches!(op, BinaryOp::Equal | BinaryOp::NotEqual)
            {
                let call = lowerer.equal(*left.clone(), *right.clone(), expr.span);
                expr.kind = if *op == BinaryOp::Equal {
                    call.kind
                } else {
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        value: Box::new(call),
                    }
                };
            }
        },
        &mut |_| {},
    );
    visit_program(program, &mut |_| {}, &mut |stmt| {
        if let StatementKind::Print { value } = &stmt.kind
            && matches!(
                value.ty,
                Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. }
            )
        {
            stmt.kind = lowerer.write(value.clone(), stmt.span, true).kind;
        }
    });
    program.function_definitions.extend(lowerer.functions);
}

fn node_id_max(stmt: &Statement, binding: &mut usize) {
    let id = match &stmt.kind {
        StatementKind::Binding { id, .. } => Some(*id),
        StatementKind::Assignment { target, .. } => Some(target.id),
        _ => None,
    };
    if let Some(id) = id {
        *binding = (*binding).max(id.0 + 1);
    }
}

// 定数の初期化式・既定値も、実行される式と同じ展開を受けます。
pub(super) fn visit_program(
    p: &mut Program,
    expr: &mut impl FnMut(&mut Expr),
    stmt: &mut impl FnMut(&mut Statement),
) {
    for c in &mut p.constant_definitions {
        visit_expr(&mut c.initializer, expr);
        visit_expr(&mut c.value, expr);
    }
    for t in &mut p.type_definitions {
        for f in &mut t.fields {
            if let Some(v) = &mut f.default {
                visit_expr(v, expr);
            }
        }
    }
    for f in &mut p.function_definitions {
        visit_body(&mut f.body, expr, stmt);
    }
    visit_body(&mut p.statements, expr, stmt);
}
pub(super) fn visit_body(
    body: &mut [Statement],
    f: &mut impl FnMut(&mut Expr),
    s: &mut impl FnMut(&mut Statement),
) {
    for stmt in body {
        match &mut stmt.kind {
            StatementKind::ArrayInitialize { array, value } => {
                visit_expr(array, f);
                visit_expr(value, f);
            }
            StatementKind::ArrayRangeCheck { length, start, end } => {
                visit_expr(length, f);
                visit_expr(start, f);
                visit_expr(end, f);
            }
            StatementKind::ArrayRetain { value }
            | StatementKind::ArrayFree { value }
            | StatementKind::StringManage { value, .. }
            | StatementKind::Binding { value, .. }
            | StatementKind::Print { value }
            | StatementKind::Write { value, .. } => visit_expr(value, f),
            StatementKind::Assignment { target, value } => {
                for AssignmentProjection::Index { index, .. }
                | AssignmentProjection::DynamicIndex { index, .. } in &mut target.projections
                {
                    visit_expr(index, f);
                }
                visit_expr(value, f);
            }
            StatementKind::Call { arguments, .. } => {
                for e in arguments {
                    visit_expr(e, f);
                }
            }
            StatementKind::Return { value } => {
                if let Some(v) = value {
                    visit_expr(v, f);
                }
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                visit_expr(condition, f);
                visit_body(then_body, f, s);
                visit_body(else_body, f, s);
            }
            StatementKind::While { condition, body } => {
                visit_expr(condition, f);
                visit_body(body, f, s);
            }
            StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                visit_body(std::slice::from_mut(initializer), f, s);
                visit_expr(condition, f);
                visit_body(std::slice::from_mut(update), f, s);
                visit_body(body, f, s);
            }
            StatementKind::Flush | StatementKind::Break | StatementKind::Continue => {}
        }
        s(stmt);
    }
}
pub(super) fn visit_expr(expr: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    match &mut expr.kind {
        ExprKind::Let { value, body, .. } => {
            visit_expr(value, f);
            visit_expr(body, f);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            visit_expr(condition, f);
            visit_expr(then_value, f);
            visit_expr(else_value, f);
        }
        ExprKind::ArrayCopy { value, range } => {
            visit_expr(value, f);
            if let Some((start, end)) = range {
                visit_expr(start, f);
                visit_expr(end, f);
            }
        }
        ExprKind::ArrayAllocate { length: value, .. }
        | ExprKind::ArrayReleaseOwner { value }
        | ExprKind::Constant { value, .. }
        | ExprKind::ArrayLength { value }
        | ExprKind::ArgumentGet { index: value }
        | ExprKind::ReadLine { max_bytes: value }
        | ExprKind::StringByteLength { value }
        | ExprKind::ConvertNumeric { value, .. }
        | ExprKind::ConvertInteger { value, .. }
        | ExprKind::Unary { value, .. }
        | ExprKind::FieldAccess { base: value, .. } => visit_expr(value, f),
        ExprKind::ArrayRepeat {
            value: left,
            count: right,
        }
        | ExprKind::StringByteAt {
            value: left,
            index: right,
        }
        | ExprKind::StringConcat { left, right }
        | ExprKind::Logical { left, right, .. }
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Index {
            base: left,
            index: right,
        } => {
            visit_expr(left, f);
            visit_expr(right, f);
        }
        ExprKind::Construct { base, fields, .. } => {
            if let Some(v) = base {
                visit_expr(v, f);
            }
            for field in fields {
                visit_expr(&mut field.value, f);
            }
        }
        ExprKind::Array(values)
        | ExprKind::Call {
            arguments: values, ..
        } => {
            for v in values {
                visit_expr(v, f);
            }
        }
        _ => {}
    }
    f(expr);
}

impl Lowerer {
    fn length(&mut self, p: &Parameter, span: Span) -> Expr {
        if let Type::Array { length, .. } = p.ty {
            return self.int(length, span);
        }
        let value = self.var(p, span);
        self.expr(
            Type::Integer(IntegerType::I64),
            ExprKind::ArrayLength {
                value: Box::new(value),
            },
            span,
        )
    }
    fn fragment(&mut self, text: &str, span: Span) -> Statement {
        let value = self.expr(Type::String, ExprKind::String(text.into()), span);
        self.stmt(
            StatementKind::Write {
                value,
                quoted: false,
            },
            span,
        )
    }
    fn write(&mut self, value: Expr, span: Span, newline: bool) -> Statement {
        if !matches!(
            value.ty,
            Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. }
        ) {
            let quoted = value.ty == Type::String;
            return self.stmt(StatementKind::Write { value, quoted }, span);
        }
        let ty = value.ty.clone();
        let id = FunctionId(self.base + self.functions.len());
        let name = format!("$display{}", id.0);
        let input = self.param("$value", ty.clone(), span);
        self.functions.push(FunctionDefinition {
            lowering: Some(LoweringKind::AggregateDisplay),
            generic_origin: None,
            id,
            name: name.clone(),
            parameters: vec![input.clone()],
            return_type: ReturnType::Void,
            body: vec![],
            span,
        });
        let mut body = Vec::new();
        match ty {
            Type::Array { element, .. } | Type::DynamicArray { element } => {
                body.push(self.fragment("[", span));
                let index = self.param("$index", Type::Integer(IntegerType::I64), span);
                let zero = self.int(0, span);
                let initializer = self.stmt(
                    StatementKind::Binding {
                        borrowed: false,
                        id: index.id,
                        name: index.name.clone(),
                        ty: index.ty.clone(),
                        mutable: true,
                        value: zero,
                    },
                    span,
                );
                let i = self.var(&index, span);
                let limit = self.length(&input, span);
                let condition = self.binary(Type::Bool, BinaryOp::Less, i, limit, span);
                let i = self.var(&index, span);
                let zero = self.int(0, span);
                let nonfirst = self.binary(Type::Bool, BinaryOp::NotEqual, i, zero, span);
                let separator = self.fragment(", ", span);
                let separator = self.stmt(
                    StatementKind::If {
                        condition: nonfirst,
                        then_body: vec![separator],
                        else_body: vec![],
                    },
                    span,
                );
                let base = self.var(&input, span);
                let i = self.var(&index, span);
                let item = self.expr(
                    *element,
                    ExprKind::Index {
                        base: Box::new(base),
                        index: Box::new(i),
                    },
                    span,
                );
                let item = self.write(item, span, false);
                let i = self.var(&index, span);
                let one = self.int(1, span);
                let next = self.binary(index.ty.clone(), BinaryOp::Add, i, one, span);
                let update = self.stmt(
                    StatementKind::Assignment {
                        target: AssignmentTarget {
                            id: index.id,
                            name: index.name,
                            root_ty: index.ty.clone(),
                            ty: index.ty,
                            projections: vec![],
                        },
                        value: next,
                    },
                    span,
                );
                body.push(self.stmt(
                    StatementKind::For {
                        initializer: Box::new(initializer),
                        condition,
                        update: Box::new(update),
                        body: vec![separator, item],
                    },
                    span,
                ));
                body.push(self.fragment("]", span));
            }
            Type::Named(type_id) => {
                let definition = self.types[type_id.0].clone();
                if let Some(variants) = definition.variants {
                    for (tag, variant) in variants.iter().enumerate() {
                        let value = self.var(&input, span);
                        let value = self.field(value, type_id, &definition.fields[0], span);
                        let expected = self.int(tag, span);
                        let condition =
                            self.binary(Type::Bool, BinaryOp::Equal, value, expected, span);
                        let fields: Vec<_> = definition
                            .fields
                            .iter()
                            .filter(|f| f.name.starts_with(&format!("${variant}$")))
                            .cloned()
                            .collect();
                        let mut branch = vec![self.fragment(variant, span)];
                        branch.extend(self.write_fields(type_id, &fields, &input, span));
                        body.push(self.stmt(
                            StatementKind::If {
                                condition,
                                then_body: branch,
                                else_body: vec![],
                            },
                            span,
                        ));
                    }
                } else {
                    body.extend(self.write_fields(type_id, &definition.fields, &input, span));
                }
            }
            _ => unreachable!(),
        }
        if newline {
            let empty = self.expr(Type::String, ExprKind::String(String::new()), span);
            body.push(self.stmt(StatementKind::Print { value: empty }, span));
        }
        body.push(self.stmt(StatementKind::Return { value: None }, span));
        self.functions[id.0 - self.base].body = body;
        self.stmt(
            StatementKind::Call {
                function_id: id,
                function_name: name,
                arguments: vec![value],
            },
            span,
        )
    }
    fn write_fields(
        &mut self,
        type_id: TypeId,
        fields: &[FieldDefinition],
        input: &Parameter,
        span: Span,
    ) -> Vec<Statement> {
        let mut body = vec![self.fragment("{", span)];
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                body.push(self.fragment(", ", span));
            }
            let name = field.name.rsplit('$').next().unwrap();
            body.push(self.fragment(&format!("{name}: "), span));
            let base = self.var(input, span);
            let value = self.field(base, type_id, field, span);
            body.push(self.write(value, span, false));
        }
        body.push(self.fragment("}", span));
        body
    }
}

struct Lowerer {
    node: usize,
    binding: usize,
    base: usize,
    types: Vec<TypeDefinition>,
    functions: Vec<FunctionDefinition>,
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
    fn var(&mut self, p: &Parameter, span: Span) -> Expr {
        self.expr(
            p.ty.clone(),
            ExprKind::Variable {
                id: p.id,
                name: p.name.clone(),
            },
            span,
        )
    }
    fn param(&mut self, name: &str, ty: Type, span: Span) -> Parameter {
        let id = BindingId(self.binding);
        self.binding += 1;
        Parameter {
            id,
            name: name.into(),
            ty,
            span,
        }
    }
    fn bool(&mut self, value: bool, span: Span) -> Expr {
        self.expr(Type::Bool, ExprKind::Boolean(value), span)
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
    fn ret(&mut self, value: Expr, span: Span) -> Statement {
        self.stmt(StatementKind::Return { value: Some(value) }, span)
    }
    fn reject_unless(&mut self, equal: Expr, span: Span) -> Statement {
        let condition = self.expr(
            Type::Bool,
            ExprKind::Unary {
                op: UnaryOp::Not,
                value: Box::new(equal),
            },
            span,
        );
        let no = self.bool(false, span);
        let ret = self.ret(no, span);
        self.stmt(
            StatementKind::If {
                condition,
                then_body: vec![ret],
                else_body: vec![],
            },
            span,
        )
    }
    fn field(&mut self, base: Expr, type_id: TypeId, field: &FieldDefinition, span: Span) -> Expr {
        self.expr(
            field.ty.clone(),
            ExprKind::FieldAccess {
                type_id,
                field_id: field.id,
                field_name: field.name.clone(),
                base: Box::new(base),
            },
            span,
        )
    }
    fn equal(&mut self, left: Expr, right: Expr, span: Span) -> Expr {
        if !matches!(
            left.ty,
            Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. }
        ) {
            return self.binary(Type::Bool, BinaryOp::Equal, left, right, span);
        }
        let ty = left.ty.clone();
        let id = FunctionId(self.base + self.functions.len());
        let name = format!("$equal{}", id.0);
        let a = self.param("$left", ty.clone(), span);
        let b = self.param("$right", ty.clone(), span);
        self.functions.push(FunctionDefinition {
            lowering: Some(LoweringKind::AggregateEquality),
            generic_origin: None,
            id,
            name: name.clone(),
            parameters: vec![a.clone(), b.clone()],
            return_type: ReturnType::Value(Type::Bool),
            body: vec![],
            span,
        });
        let mut body = Vec::new();
        match ty {
            Type::Array { element, .. } | Type::DynamicArray { element } => {
                if matches!(a.ty, Type::DynamicArray { .. }) {
                    let al = self.length(&a, span);
                    let bl = self.length(&b, span);
                    let equal = self.binary(Type::Bool, BinaryOp::Equal, al, bl, span);
                    body.push(self.reject_unless(equal, span));
                }
                let i = self.param("$index", Type::Integer(IntegerType::I64), span);
                let zero = self.int(0, span);
                let initializer = self.stmt(
                    StatementKind::Binding {
                        borrowed: false,
                        id: i.id,
                        name: i.name.clone(),
                        ty: i.ty.clone(),
                        mutable: true,
                        value: zero,
                    },
                    span,
                );
                let index = self.var(&i, span);
                let limit = self.length(&a, span);
                let condition = self.binary(Type::Bool, BinaryOp::Less, index, limit, span);
                let av = self.var(&a, span);
                let index = self.var(&i, span);
                let av = self.expr(
                    *element.clone(),
                    ExprKind::Index {
                        base: Box::new(av),
                        index: Box::new(index),
                    },
                    span,
                );
                let bv = self.var(&b, span);
                let index = self.var(&i, span);
                let bv = self.expr(
                    *element,
                    ExprKind::Index {
                        base: Box::new(bv),
                        index: Box::new(index),
                    },
                    span,
                );
                let equal = self.equal(av, bv, span);
                let check = self.reject_unless(equal, span);
                let index = self.var(&i, span);
                let one = self.int(1, span);
                let next = self.binary(i.ty.clone(), BinaryOp::Add, index, one, span);
                let update = self.stmt(
                    StatementKind::Assignment {
                        target: AssignmentTarget {
                            id: i.id,
                            name: i.name,
                            root_ty: i.ty.clone(),
                            ty: i.ty,
                            projections: vec![],
                        },
                        value: next,
                    },
                    span,
                );
                body.push(self.stmt(
                    StatementKind::For {
                        initializer: Box::new(initializer),
                        condition,
                        update: Box::new(update),
                        body: vec![check],
                    },
                    span,
                ));
            }
            Type::Named(type_id) => {
                let definition = self.types[type_id.0].clone();
                if let Some(variants) = definition.variants {
                    let tag = &definition.fields[0];
                    let av = self.var(&a, span);
                    let av = self.field(av, type_id, tag, span);
                    let bv = self.var(&b, span);
                    let bv = self.field(bv, type_id, tag, span);
                    let equal = self.equal(av, bv, span);
                    body.push(self.reject_unless(equal, span));
                    for (index, variant) in variants.iter().enumerate() {
                        let av = self.var(&a, span);
                        let av = self.field(av, type_id, tag, span);
                        let expected = self.int(index, span);
                        let condition =
                            self.binary(Type::Bool, BinaryOp::Equal, av, expected, span);
                        let fields: Vec<_> = definition
                            .fields
                            .iter()
                            .filter(|f| f.name.starts_with(&format!("${variant}$")))
                            .cloned()
                            .collect();
                        let mut branch = self.fields_equal(type_id, &fields, &a, &b, span);
                        let yes = self.bool(true, span);
                        branch.push(self.ret(yes, span));
                        body.push(self.stmt(
                            StatementKind::If {
                                condition,
                                then_body: branch,
                                else_body: vec![],
                            },
                            span,
                        ));
                    }
                    let no = self.bool(false, span);
                    body.push(self.ret(no, span));
                } else {
                    body.extend(self.fields_equal(type_id, &definition.fields, &a, &b, span));
                }
            }
            _ => unreachable!(),
        }
        let yes = self.bool(true, span);
        body.push(self.ret(yes, span));
        self.functions[id.0 - self.base].body = body;
        self.expr(
            Type::Bool,
            ExprKind::Call {
                function_id: id,
                function_name: name,
                arguments: vec![left, right],
            },
            span,
        )
    }
    fn fields_equal(
        &mut self,
        type_id: TypeId,
        fields: &[FieldDefinition],
        a: &Parameter,
        b: &Parameter,
        span: Span,
    ) -> Vec<Statement> {
        let mut body = Vec::new();
        for field in fields {
            let av = self.var(a, span);
            let av = self.field(av, type_id, field, span);
            let bv = self.var(b, span);
            let bv = self.field(bv, type_id, field, span);
            let equal = self.equal(av, bv, span);
            body.push(self.reject_unless(equal, span));
        }
        body
    }
}

impl Lowerer {
    // 式の位置を変えず、既存の関数・分岐へ展開します。自由変数は不変のコピーです。
    fn expression_function(&mut self, mut expr: Expr) -> Expr {
        let span = expr.span;
        let bound = if let ExprKind::Let { binding, .. } = expr.kind {
            Some(binding)
        } else {
            None
        };
        let mut captures = Vec::<Parameter>::new();
        visit_expr(&mut expr, &mut |e| {
            if let ExprKind::Variable { id, name } = &e.kind
                && Some(*id) != bound
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
            .map(|p| self.param(&format!("$capture{}", p.id.0), p.ty.clone(), p.span))
            .collect();
        visit_expr(&mut expr, &mut |e| {
            if let ExprKind::Variable { id, name } = &mut e.kind
                && let Some(index) = captures.iter().position(|p| p.id == *id)
            {
                *id = parameters[index].id;
                *name = parameters[index].name.clone();
            }
        });
        let id = FunctionId(self.base + self.functions.len());
        let name = format!(
            "$match_{}{}",
            if bound.is_some() { "bind" } else { "select" },
            id.0
        );
        let body = match expr.kind {
            ExprKind::Let {
                binding,
                name,
                value,
                body,
            } => vec![
                self.stmt(
                    StatementKind::Binding {
                        borrowed: false,
                        id: binding,
                        name,
                        mutable: false,
                        ty: value.ty.clone(),
                        value: *value,
                    },
                    span,
                ),
                self.ret(*body, span),
            ],
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let yes = self.ret(*then_value, span);
                let no = self.ret(*else_value, span);
                vec![self.stmt(
                    StatementKind::If {
                        condition: *condition,
                        then_body: vec![yes],
                        else_body: vec![no],
                    },
                    span,
                )]
            }
            _ => unreachable!(),
        };
        let arguments = captures.iter().map(|p| self.var(p, p.span)).collect();
        self.functions.push(FunctionDefinition {
            lowering: Some(if bound.is_some() {
                LoweringKind::MatchBinding
            } else {
                LoweringKind::MatchSelect
            }),
            generic_origin: None,
            id,
            name: name.clone(),
            parameters,
            return_type: ReturnType::Value(expr.ty.clone()),
            body,
            span,
        });
        self.expr(
            expr.ty,
            ExprKind::Call {
                function_id: id,
                function_name: name,
                arguments,
            },
            span,
        )
    }
}
