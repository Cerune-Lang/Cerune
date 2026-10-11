use super::*;
impl Lowerer {
    pub(super) fn dynamic(&self, ty: &Type) -> bool {
        match ty {
            Type::DynamicArray { .. } => true,
            Type::Array { element, .. } => self.dynamic(element),
            Type::Named(id) => self.types[id.0].fields.iter().any(|f| self.dynamic(&f.ty)),
            _ => false,
        }
    }
    fn element_width(&self, ty: &Type) -> u64 {
        match ty {
            Type::DynamicArray { .. } | Type::String => 16,
            Type::Array { element, length } => self.element_width(element) * *length as u64,
            Type::Named(id) => self.types[id.0]
                .fields
                .iter()
                .map(|f| self.element_width(&f.ty))
                .sum(),
            _ => 8,
        }
    }
    pub(super) fn length(&mut self, array: Expr) -> Expr {
        let span = array.span;
        if let Type::Array { length, .. } = array.ty {
            return self.int(length, span);
        }
        self.expr(
            Type::Integer(IntegerType::I64),
            ExprKind::ArrayLength {
                value: Box::new(array),
            },
            span,
        )
    }
    pub(super) fn array_loop(
        &mut self,
        length: Expr,
        reverse: bool,
        span: Span,
        make: impl FnOnce(&mut Self, Expr) -> Vec<Statement>,
        out: &mut Vec<Statement>,
    ) {
        let cursor = self.param(Type::Integer(IntegerType::I64), span);
        let start = if reverse {
            self.copy(&length)
        } else {
            self.int(0, span)
        };
        let initializer = self.stmt(
            StatementKind::Binding {
                borrowed: false,
                id: cursor.id,
                name: cursor.name.clone(),
                ty: cursor.ty.clone(),
                mutable: true,
                value: start,
            },
            span,
        );
        let current = self.var(&cursor);
        let limit = if reverse { self.int(0, span) } else { length };
        let condition = self.binary(
            Type::Bool,
            if reverse {
                BinaryOp::Greater
            } else {
                BinaryOp::Less
            },
            current,
            limit,
            span,
        );
        let current = self.var(&cursor);
        let one = self.int(1, span);
        let next = self.binary(
            cursor.ty.clone(),
            if reverse {
                BinaryOp::Subtract
            } else {
                BinaryOp::Add
            },
            current,
            one,
            span,
        );
        let index = if reverse {
            self.copy(&next)
        } else {
            self.var(&cursor)
        };
        let update = self.stmt(
            StatementKind::Assignment {
                target: Self::target(&cursor),
                value: next,
            },
            span,
        );
        let body = make(self, index);
        out.push(self.stmt(
            StatementKind::For {
                initializer: Box::new(initializer),
                condition,
                update: Box::new(update),
                body,
            },
            span,
        ));
    }
    pub(super) fn repeat(
        &mut self,
        seed: Expr,
        count: Expr,
        span: Span,
        out: &mut Vec<Statement>,
    ) -> Expr {
        let count = self.bind(count, false, out);
        let length = self.var(&count);
        let allocate = self.expr(
            Type::DynamicArray {
                element: Box::new(seed.ty.clone()),
            },
            ExprKind::ArrayAllocate {
                length: Box::new(length),
                element_width: self.element_width(&seed.ty),
            },
            span,
        );
        let result = self.bind(allocate, false, out);
        let length = self.var(&count);
        self.array_loop(
            length,
            false,
            span,
            |this, _| {
                let mut body = vec![];
                let mut input = this.copy(&seed);
                // コピーの失敗は、seedの読み取りではなく反復生成式を指します。
                input.span = span;
                let value = this.copy_owned(input, &mut body);
                let array = this.var(&result);
                body.push(this.stmt(StatementKind::ArrayInitialize { array, value }, span));
                body
            },
            out,
        );
        self.var(&result)
    }
    pub(super) fn copy_range(
        &mut self,
        source: Expr,
        range: Option<(Expr, Expr)>,
        span: Span,
        out: &mut Vec<Statement>,
    ) -> Expr {
        let element = match &source.ty {
            Type::Array { element, .. } | Type::DynamicArray { element } => *element.clone(),
            _ => unreachable!(),
        };
        let source_value = self.copy(&source);
        let length = self.length(source_value);
        let (start, end) = range.unwrap_or_else(|| (self.int(0, span), self.copy(&length)));
        let checked_length = self.copy(&length);
        let checked_start = self.copy(&start);
        let checked_end = self.copy(&end);
        out.push(self.stmt(
            StatementKind::ArrayRangeCheck {
                length: checked_length,
                start: checked_start,
                end: checked_end,
            },
            span,
        ));
        let start_copy = self.copy(&start);
        let count = self.binary(
            Type::Integer(IntegerType::I64),
            BinaryOp::Subtract,
            end,
            start_copy,
            span,
        );
        let count = self.bind(count, false, out);
        let length = self.var(&count);
        let result_ty = Type::DynamicArray {
            element: Box::new(element.clone()),
        };
        let allocate = self.expr(
            result_ty,
            ExprKind::ArrayAllocate {
                length: Box::new(length),
                element_width: self.element_width(&element),
            },
            span,
        );
        let result = self.bind(allocate, false, out);
        let length = self.var(&count);
        self.array_loop(
            length,
            false,
            span,
            |this, index| {
                let mut body = vec![];
                let start = this.copy(&start);
                let index = this.binary(
                    Type::Integer(IntegerType::I64),
                    BinaryOp::Add,
                    index,
                    start,
                    span,
                );
                let source = this.copy(&source);
                let item = this.expr(
                    element,
                    ExprKind::Index {
                        base: Box::new(source),
                        index: Box::new(index),
                    },
                    span,
                );
                let value = this.owned(item, &mut body);
                let array = this.var(&result);
                body.push(this.stmt(StatementKind::ArrayInitialize { array, value }, span));
                body
            },
            out,
        );
        self.var(&result)
    }
    pub(super) fn copy_owned(&mut self, value: Expr, out: &mut Vec<Statement>) -> Expr {
        if !self.dynamic(&value.ty) {
            let p = self.bind(value, false, out);
            let value = self.var(&p);
            self.manage(value, true, out);
            return self.var(&p);
        }
        let span = value.span;
        match value.ty.clone() {
            Type::DynamicArray { .. } => {
                let source = self.bind_read(value, out);
                let value = self.var(&source);
                self.copy_range(value, None, span, out)
            }
            Type::Array { element, length } => {
                // 固定長の外側を値として複製し、所有する各要素を順に置き換えます。
                let result = self.bind(value, true, out);
                let count = self.int(length, span);
                self.array_loop(
                    count,
                    false,
                    span,
                    |this, index| {
                        let mut body = vec![];
                        let base = this.var(&result);
                        let read_index = this.copy(&index);
                        let item = this.expr(
                            *element.clone(),
                            ExprKind::Index {
                                base: Box::new(base),
                                index: Box::new(read_index),
                            },
                            span,
                        );
                        let copied = this.copy_owned(item, &mut body);
                        let mut target = Self::target(&result);
                        target.projections.push(AssignmentProjection::Index {
                            index,
                            element: *element.clone(),
                            length,
                            span,
                        });
                        target.ty = *element.clone();
                        body.push(this.stmt(
                            StatementKind::Assignment {
                                target,
                                value: copied,
                            },
                            span,
                        ));
                        body
                    },
                    out,
                );
                self.var(&result)
            }
            Type::Named(id) => {
                let definition = self.types[id.0].clone();
                let input = self.bind_read(value, out);
                let result = self.param(Type::Named(id), span);
                // enumは選択中のペイロードだけをコピーします。非選択の空値はそのままです。
                let variants = definition
                    .variants
                    .clone()
                    .unwrap_or_else(|| vec![String::new()]);
                let mut branches = vec![];
                for (tag, variant) in variants.iter().enumerate() {
                    let mut body = vec![];
                    let mut fields = vec![];
                    for f in &definition.fields {
                        let base = self.var(&input);
                        let value = self.field(base, id, f);
                        let active = definition.variants.is_none()
                            || f.name.starts_with(&format!("{}{}{}", "$", variant, "$"));
                        let value = if active {
                            self.copy_owned(value, &mut body)
                        } else {
                            value
                        };
                        fields.push(FieldValue {
                            id: f.id,
                            name: f.name.clone(),
                            value,
                            origin: FieldValueOrigin::Generated { span },
                        });
                    }
                    let value = self.expr(
                        Type::Named(id),
                        ExprKind::Construct {
                            type_id: id,
                            type_name: definition.name.clone(),
                            base: None,
                            fields,
                        },
                        span,
                    );
                    body.push(self.stmt(
                        StatementKind::Assignment {
                            target: Self::target(&result),
                            value,
                        },
                        span,
                    ));
                    if definition.variants.is_some() {
                        let base = self.var(&input);
                        let tag_value = self.field(base, id, &definition.fields[0]);
                        let expected = self.int(tag, span);
                        let condition =
                            self.binary(Type::Bool, BinaryOp::Equal, tag_value, expected, span);
                        branches.push(self.stmt(
                            StatementKind::If {
                                condition,
                                then_body: body,
                                else_body: vec![],
                            },
                            span,
                        ));
                    } else {
                        branches.extend(body);
                    }
                }
                let initial = self.var(&input);
                out.push(self.stmt(
                    StatementKind::Binding {
                        borrowed: false,
                        id: result.id,
                        name: result.name.clone(),
                        ty: result.ty.clone(),
                        mutable: true,
                        value: initial,
                    },
                    span,
                ));
                out.extend(branches);
                self.var(&result)
            }
            _ => unreachable!(),
        }
    }
}
