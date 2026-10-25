//! 完成済みのCerune IRを直接実行します。命令列や別のIRへ変換しません。
use crate::runtime::arguments::{self, Arguments};
use crate::runtime::array_heap::{ArrayError, ArrayHeap};
use crate::runtime::call;
use crate::runtime::input::{self, Source};
use crate::runtime::output::{self, Capture, Sink};
use crate::runtime::value::{self, write_quoted};
use crate::{
    ir::{self, Expr, ExprKind as E, Statement, StatementKind as S},
    runtime::{
        FailureCode, RuntimeFailure,
        numeric::{self, Number},
        string_heap::{HeapError, StringHeap},
    },
    source::Span,
    types::IntegerType,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use value::Value;

/// IR上の要素と、その由来となるソース位置です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub node_id: ir::NodeId,
    pub span: Span,
}
/// 言語の異常停止と、不正なIRによる内部問題を区別します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    CallDepth(Box<call::Failure>),
    Arguments(Box<arguments::Failure>),
    Input(Box<input::Failure>),
    Output(Box<output::Failure>),
    Runtime(FailureCode),
    InvalidIr(&'static str),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionError {
    kind: ErrorKind,
    origin: Option<Origin>,
    output: String,
}
impl ExecutionError {
    pub fn call_failure(&self) -> Option<&call::Failure> {
        match &self.kind {
            ErrorKind::CallDepth(f) => Some(f),
            _ => None,
        }
    }
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    pub const fn origin(&self) -> Option<Origin> {
        self.origin
    }
    pub fn output(&self) -> &str {
        &self.output
    }
    pub fn runtime_failure(&self) -> Option<RuntimeFailure> {
        match (&self.kind, self.origin) {
            (ErrorKind::Runtime(code), Some(origin)) => Some(RuntimeFailure {
                code: *code,
                node_id: origin.node_id,
                span: origin.span,
            }),
            _ => None,
        }
    }
    pub fn diagnostic(&self) -> crate::diagnostic::Diagnostic {
        let message = match &self.kind {
            ErrorKind::CallDepth(failure) => failure.to_string(),
            ErrorKind::Arguments(failure) => failure.to_string(),
            ErrorKind::Input(failure) => failure.to_string(),
            ErrorKind::Output(failure) => failure.to_string(),
            ErrorKind::Runtime(code) => format!("runtime error: {}", code.name()),
            ErrorKind::InvalidIr(reason) => format!("invalid Cerune IR: {reason}"),
        };
        match self.origin {
            Some(origin) => crate::diagnostic::Diagnostic::new(message, origin.span),
            None => crate::diagnostic::Diagnostic::without_span(message),
        }
    }
}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostic().message())
    }
}
impl std::error::Error for ExecutionError {}
#[derive(Debug)]
struct Fault {
    kind: ErrorKind,
    origin: Option<Origin>,
}
type Result<T> = std::result::Result<T, Fault>;
impl From<value::Error> for Fault {
    fn from(error: value::Error) -> Self {
        match error {
            value::Error::Invalid(reason) => Self::invalid(reason),
            value::Error::Runtime(code) => Self::failure(code),
        }
    }
}
impl Fault {
    fn invalid(reason: &'static str) -> Self {
        Self {
            kind: ErrorKind::InvalidIr(reason),
            origin: None,
        }
    }
    fn failure(code: FailureCode) -> Self {
        Self {
            kind: ErrorKind::Runtime(code),
            origin: None,
        }
    }
    fn at(mut self, node_id: ir::NodeId, span: Span) -> Self {
        // 既に失敗した子の式・関数の出自は上書きしません。
        if self.origin.is_none() {
            self.origin = Some(Origin { node_id, span });
        }
        self
    }
}
fn array_error(e: ArrayError) -> Fault {
    match e {
        ArrayError::AllocationSizeOverflow => Fault::failure(FailureCode::AllocationSizeOverflow),
        ArrayError::AllocationLimitExceeded => Fault::failure(FailureCode::AllocationLimitExceeded),
        ArrayError::AllocationFailed => Fault::failure(FailureCode::AllocationFailed),
        ArrayError::IndexOutOfBounds => Fault::failure(FailureCode::ArrayIndexOutOfBounds),
        ArrayError::InvalidOwnership => Fault::invalid("invalid array ownership"),
    }
}
fn heap_error(e: HeapError) -> Fault {
    match e {
        HeapError::AllocationSizeOverflow => Fault::failure(FailureCode::AllocationSizeOverflow),
        HeapError::AllocationLimitExceeded => Fault::failure(FailureCode::AllocationLimitExceeded),
        HeapError::AllocationFailed => Fault::failure(FailureCode::AllocationFailed),
        HeapError::InvalidStringOwnership => Fault::invalid("invalid string ownership"),
    }
}
struct Binding {
    value: Value,
    mutable: bool,
    initializer: Option<ir::NodeId>,
}
type Frame = HashMap<ir::BindingId, Binding>;
mod machine;
struct Executor<'a, 'o> {
    calls: Option<call::Stack>,
    program: &'a ir::Program,
    output: &'o mut dyn Sink,
    input: &'o mut dyn Source,
    program_arguments: &'o Arguments,
    heap: StringHeap,
    arrays: ArrayHeap<Value>,
}

/// 共通フロントエンドが構築したIRを、新しい実行状態で直接実行します。
/// IRは変更しません。失敗時にも、それまでの出力と元の式の位置を保持します。
pub fn run(program: &ir::Program) -> std::result::Result<String, ExecutionError> {
    run_sink(program, &mut Capture::default())
}

/// ソース関数の同時呼出し数を明示した上限で検査します。既定値は選びません。
/// 物理stack・memory消費を制限するAPIではありません。
pub fn run_with_call_limit(
    program: &ir::Program,
    limit: NonZeroU32,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(
        program,
        &mut Capture::default(),
        &mut input::unconfigured(),
        &Arguments::unconfigured(),
        Some(limit),
    )
}

/// 実行中に接続先へ出力します。失敗時のoutput()は空で、受理済みの出力は接続先に残ります。
/// ソースのflush命令を実行します。終了時の自動flushは行わず、呼出し側がOutput::flushを使います。
pub fn run_with_output<W: std::io::Write>(
    program: &ir::Program,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_sink(program, output).map(|_| ())
}
fn run_sink(
    program: &ir::Program,
    output: &mut dyn Sink,
) -> std::result::Result<String, ExecutionError> {
    run_connections(
        program,
        output,
        &mut crate::runtime::input::unconfigured(),
        &Arguments::unconfigured(),
    )
}
pub fn run_with_io<R: std::io::Read, W: std::io::Write>(
    program: &ir::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, input, &Arguments::unconfigured()).map(|_| ())
}
pub fn run_with_arguments(
    program: &ir::Program,
    arguments: &Arguments,
) -> std::result::Result<String, ExecutionError> {
    run_connections(
        program,
        &mut Capture::default(),
        &mut input::unconfigured(),
        arguments,
    )
}
pub fn run_with_io_and_arguments<R: std::io::Read, W: std::io::Write>(
    program: &ir::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
    arguments: &Arguments,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, input, arguments).map(|_| ())
}
fn run_connections(
    program: &ir::Program,
    output: &mut dyn Sink,
    input: &mut dyn Source,
    program_arguments: &Arguments,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(program, output, input, program_arguments, None)
}
fn run_connections_with_limit(
    program: &ir::Program,
    output: &mut dyn Sink,
    input: &mut dyn Source,
    program_arguments: &Arguments,
    limit: Option<NonZeroU32>,
) -> std::result::Result<String, ExecutionError> {
    let mut executor = Executor {
        calls: limit.map(call::Stack::new),
        program,
        output,
        input,
        program_arguments,
        heap: StringHeap::new(program.string_heap_limit),
        arrays: ArrayHeap::new(program.array_heap_limit),
    };
    match executor.entry() {
        Ok(()) => {
            #[cfg(test)]
            {
                executor.heap.assert_empty();
                executor.arrays.assert_empty();
            }
            Ok(executor.output.captured().to_owned())
        }
        Err(error) => Err(ExecutionError {
            kind: error.kind,
            origin: error.origin,
            output: executor.output.captured().to_owned(),
        }),
    }
}
impl Executor<'_, '_> {
    fn entry(&mut self) -> Result<()> {
        machine::execute(self)
    }
    fn apply_statement(
        &mut self,
        s: &Statement,
        frame: &mut Frame,
        values: &mut impl Iterator<Item = Value>,
    ) -> Result<()> {
        match &s.kind {
            S::Binding {
                id, mutable, ty, ..
            } => {
                let value = operand(values)?;
                value.check(ty)?;
                if frame.get(id).is_some_and(|b| b.initializer != Some(s.id)) {
                    return Err(Fault::invalid("duplicate binding"));
                }
                if !frame.contains_key(id) {
                    frame
                        .try_reserve(1)
                        .map_err(|_| Fault::failure(FailureCode::AllocationFailed))?;
                }
                frame.insert(
                    *id,
                    Binding {
                        value,
                        mutable: *mutable,
                        initializer: Some(s.id),
                    },
                );
            }

            S::ArrayInitialize { .. } => {
                let Value::DynamicArray { element, storage } = operand(values)? else {
                    return Err(Fault::invalid("array initialize"));
                };
                let value = operand(values)?;
                value.check(&element)?;
                self.arrays
                    .initialize(&storage, value)
                    .map_err(array_error)?;
            }
            S::ArrayRetain { .. } | S::ArrayFree { .. } => {
                let Value::DynamicArray { storage, .. } = operand(values)? else {
                    return Err(Fault::invalid("array ownership"));
                };
                if matches!(s.kind, S::ArrayRetain { .. }) {
                    self.arrays.retain(&storage)
                } else {
                    self.arrays.free(&storage)
                }
                .map_err(array_error)?;
            }
            S::ArrayRangeCheck { .. } => {
                let length = operand(values)?.integer(IntegerType::I64)?;
                let start = operand(values)?.integer(IntegerType::I64)?;
                let end = operand(values)?.integer(IntegerType::I64)?;
                if start < 0 || start > end || end > length {
                    return Err(Fault::failure(FailureCode::ArrayRangeOutOfBounds));
                }
            }
            S::StringManage { retain, .. } => {
                let value = operand(values)?.string()?;
                self.heap.manage(&value, *retain).map_err(heap_error)?;
            }
            S::Write { quoted, .. } => {
                let text = operand(values)?.text()?;
                let mut rendered = String::new();
                if *quoted {
                    write_quoted(&mut rendered, &text);
                } else {
                    rendered.push_str(&text);
                }
                self.output
                    .write(
                        &rendered,
                        Some(output::Origin {
                            node_id: s.id,
                            span: s.span,
                        }),
                    )
                    .map_err(|failure| Fault {
                        kind: ErrorKind::Output(Box::new(failure)),
                        origin: None,
                    })?;
            }
            S::Flush => {
                self.output
                    .flush(Some(output::Origin {
                        node_id: s.id,
                        span: s.span,
                    }))
                    .map_err(|failure| Fault {
                        kind: ErrorKind::Output(Box::new(failure)),
                        origin: None,
                    })?;
            }
            S::Print { .. } => {
                let mut text = operand(values)?.text()?;
                text.push('\n');
                self.output
                    .write(
                        &text,
                        Some(output::Origin {
                            node_id: s.id,
                            span: s.span,
                        }),
                    )
                    .map_err(|failure| Fault {
                        kind: ErrorKind::Output(Box::new(failure)),
                        origin: None,
                    })?;
            }
            _ => return Err(Fault::invalid("control statement reached value operation")),
        }
        Ok(())
    }
    fn apply_expr(
        &mut self,
        e: &Expr,
        frame: &Frame,
        values: &mut impl Iterator<Item = Value>,
    ) -> Result<Value> {
        Ok(match &e.kind {
            E::ArrayCopy { .. } | E::ArrayRepeat { .. } => {
                return Err(Fault::invalid("array copy must be lowered"));
            }
            E::ArrayAllocate { element_width, .. } => {
                let ir::Type::DynamicArray { element } = &e.ty else {
                    return Err(Fault::invalid("array allocation type"));
                };
                let count = operand(values)?.integer(IntegerType::I64)?;
                let count = u64::try_from(count)
                    .map_err(|_| Fault::failure(FailureCode::ArrayLengthOutOfRange))?;
                let storage = self
                    .arrays
                    .allocate(count, *element_width)
                    .map_err(array_error)?;
                Value::DynamicArray {
                    element: *element.clone(),
                    storage,
                }
            }
            E::ArrayReleaseOwner { .. } => {
                let Value::DynamicArray { storage, .. } = operand(values)? else {
                    return Err(Fault::invalid("array release"));
                };
                Value::Bool(self.arrays.release_owner(&storage).map_err(array_error)?)
            }
            E::Boolean(v) => Value::Bool(*v),
            E::String(v) => Value::String(v.clone().into()),
            E::Integer(v) => {
                let ir::Type::Integer(t) = e.ty else {
                    return Err(Fault::invalid("integer literal type"));
                };
                if !t.contains(*v) {
                    return Err(Fault::invalid("integer literal out of range"));
                }
                Value::Number(Number::Integer(*v, t))
            }
            E::Float { text } => Value::Number(match e.ty {
                ir::Type::F32 => Number::F32(
                    text.parse()
                        .map_err(|_| Fault::invalid("invalid f32 literal"))?,
                ),
                ir::Type::F64 => Number::F64(
                    text.parse()
                        .map_err(|_| Fault::invalid("invalid f64 literal"))?,
                ),
                _ => return Err(Fault::invalid("float literal type")),
            }),
            E::Variable { id, .. } => frame
                .get(id)
                .ok_or_else(|| Fault::invalid("read of unknown binding"))?
                .value
                .clone(),
            E::Constant { .. } => operand(values)?,
            E::ArgumentCount => Value::Number(Number::Integer(
                self.program_arguments
                    .count_at(output::Origin {
                        node_id: e.id,
                        span: e.span,
                    })
                    .map_err(|failure| Fault {
                        kind: ErrorKind::Arguments(Box::new(failure)),
                        origin: None,
                    })? as i128,
                IntegerType::I64,
            )),
            E::ArgumentGet { .. } => {
                let index = operand(values)?.integer(IntegerType::I64)? as i64;
                Value::String(
                    self.program_arguments
                        .get_value(
                            &mut self.heap,
                            index,
                            output::Origin {
                                node_id: e.id,
                                span: e.span,
                            },
                        )
                        .map_err(|failure| Fault {
                            kind: ErrorKind::Arguments(Box::new(failure)),
                            origin: None,
                        })?,
                )
            }
            E::ReadLine { .. } => {
                let limit = operand(values)?.integer(IntegerType::I64)? as i64;
                Value::String(
                    self.input
                        .read_value(
                            &mut self.heap,
                            limit,
                            input::Origin {
                                node_id: e.id,
                                span: e.span,
                            },
                        )
                        .map_err(|failure| Fault {
                            kind: ErrorKind::Input(Box::new(failure)),
                            origin: None,
                        })?,
                )
            }
            E::StringConcat { .. } => {
                let left = operand(values)?.string()?;
                let right = operand(values)?.string()?;
                Value::String(self.heap.concat(&left, &right).map_err(heap_error)?)
            }
            E::StringByteAt { .. } => {
                let value = operand(values)?.string()?;
                let index = operand(values)?.integer(IntegerType::I64)?;
                let byte = value.byte_at(index);
                Value::Number(Number::Integer(byte, IntegerType::I64))
            }
            E::StringByteLength { .. } => {
                let value = operand(values)?.string()?;
                Value::Number(Number::Integer(value.len() as i128, IntegerType::I64))
            }
            E::ArrayLength { .. } => {
                let len = match operand(values)? {
                    Value::Array { values, .. } => values.len(),
                    Value::DynamicArray { storage, .. } => storage.len(),
                    _ => return Err(Fault::invalid("array_len of non-array")),
                };
                Value::Number(Number::Integer(len as i128, IntegerType::I64))
            }
            E::ConvertInteger { from, to, .. } => {
                let v = operand(values)?.integer(*from)?;
                if !to.contains(v) {
                    return Err(Fault::failure(FailureCode::IntegerConversionOutOfRange));
                }
                Value::Number(Number::Integer(v, *to))
            }
            E::ConvertNumeric {
                value,
                from,
                to,
                mode,
                ..
            } => {
                if value::numeric_type(&value.ty) != Some(*from) {
                    return Err(Fault::invalid("conversion source type"));
                }
                let v = operand(values)?.number()?;
                Value::Number(
                    numeric::convert_with_mode(v, *from, *to, *mode)
                        .map_err(value::numeric_error)?,
                )
            }
            E::Logical { op, .. } => {
                let left = operand(values)?.boolean()?;
                Value::Bool(match op {
                    ir::LogicalOp::And => left && operand(values)?.boolean()?,
                    ir::LogicalOp::Or => left || operand(values)?.boolean()?,
                })
            }
            E::Unary { op, .. } => {
                let value = operand(values)?;
                value::unary(*op, value)?
            }
            E::Binary { op, .. } => {
                let left = operand(values)?;
                let right = operand(values)?;
                value::binary(*op, left, right)?
            }
            E::Array(elements) => {
                let ir::Type::Array { element, length } = &e.ty else {
                    return Err(Fault::invalid("array literal type"));
                };
                if *length != elements.len() {
                    return Err(Fault::invalid("array literal length"));
                }
                let mut items = Vec::new();
                items
                    .try_reserve(elements.len())
                    .map_err(|_| Fault::failure(FailureCode::AllocationFailed))?;
                for _ in elements {
                    let v = operand(values)?;
                    v.check(element)?;
                    items.push(v);
                }
                Value::Array {
                    element: *element.clone(),
                    values: items,
                }
            }
            E::Index { .. } => {
                let base = operand(values)?;
                let index = operand(values)?.integer(IntegerType::I64)?;
                match base {
                    Value::Array { values, .. } => {
                        values[checked_index(index, values.len())?].clone()
                    }
                    Value::DynamicArray { storage, .. } => storage
                        .get(checked_index(index, storage.len())?)
                        .map_err(array_error)?,
                    _ => return Err(Fault::invalid("index of non-array")),
                }
            }
            E::Construct {
                type_id,
                base,
                fields,
                ..
            } => {
                let def = self
                    .program
                    .type_definitions
                    .get(type_id.0)
                    .filter(|d| d.id == *type_id)
                    .ok_or_else(|| Fault::invalid("unknown product type"))?;
                let mut fields_values = if base.is_some() {
                    let base = operand(values)?;
                    base.check(&ir::Type::Named(*type_id))?;
                    let Value::Aggregate { fields, .. } = base else {
                        return Err(Fault::invalid("product base"));
                    };
                    fields.into_iter().map(Some).collect::<Vec<_>>()
                } else {
                    vec![None; def.fields.len()]
                };
                for field in fields {
                    let value = operand(values)?;
                    let expected = def
                        .fields
                        .get(field.id.0)
                        .ok_or_else(|| Fault::invalid("unknown constructor field"))?;
                    value.check(&expected.ty)?;
                    let slot = fields_values
                        .get_mut(field.id.0)
                        .ok_or_else(|| Fault::invalid("invalid product base layout"))?;
                    *slot = Some(value);
                }
                let fields = fields_values
                    .into_iter()
                    .map(|v| v.ok_or_else(|| Fault::invalid("missing constructor field")))
                    .collect::<Result<Vec<_>>>()?;
                Value::Aggregate {
                    type_id: *type_id,
                    fields,
                }
            }
            E::FieldAccess {
                type_id, field_id, ..
            } => {
                let base = operand(values)?;
                base.check(&ir::Type::Named(*type_id))?;
                let Value::Aggregate { fields, .. } = base else {
                    return Err(Fault::invalid("field of non-product"));
                };
                fields
                    .get(field_id.0)
                    .ok_or_else(|| Fault::invalid("unknown product field"))?
                    .clone()
            }
            E::Call { .. } => return Err(Fault::invalid("call reached value operation")),
            E::Let { .. } | E::Conditional { .. } => {
                return Err(Fault::invalid("IR must have completed common expansion"));
            }
        })
    }
}
fn operand(values: &mut impl Iterator<Item = Value>) -> Result<Value> {
    values
        .next()
        .ok_or_else(|| Fault::invalid("missing evaluated operand"))
}
fn checked_index(index: i128, length: usize) -> Result<usize> {
    usize::try_from(index)
        .ok()
        .filter(|i| *i < length)
        .ok_or_else(|| Fault::failure(FailureCode::ArrayIndexOutOfBounds))
}
fn at_path(current: &Value, path: &[usize]) -> Result<Value> {
    let mut current = current.clone();
    for i in path {
        current = match current {
            Value::Array { values, .. } => values
                .get(*i)
                .cloned()
                .ok_or_else(|| Fault::invalid("invalid checked index"))?,
            Value::DynamicArray { storage, .. } => storage.get(*i).map_err(array_error)?,
            _ => return Err(Fault::invalid("invalid array path")),
        };
    }
    Ok(current)
}
fn set_path(current: &mut Value, path: &[usize], replacement: Value) -> Result<()> {
    let Some((&index, rest)) = path.split_first() else {
        *current = replacement;
        return Ok(());
    };
    match current {
        Value::Array { values, .. } => set_path(
            values
                .get_mut(index)
                .ok_or_else(|| Fault::invalid("invalid checked index"))?,
            rest,
            replacement,
        ),
        Value::DynamicArray { storage, .. } => {
            let mut value = storage.get(index).map_err(array_error)?;
            set_path(&mut value, rest, replacement)?;
            storage.set(index, value).map_err(array_error)
        }
        _ => Err(Fault::invalid("invalid array path")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_cleanup_releases_every_successful_example() {
        for entry in std::fs::read_dir("examples").unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "ceru") {
                let source = std::fs::read_to_string(path).unwrap();
                run(&crate::compile_to_ir(&source).unwrap()).unwrap();
            }
        }
    }

    #[test]
    fn allocation_failure_preserves_original_expression_and_prior_output() {
        let source = r#"print("before"); print(concat("a","b")); print("bad");"#;
        let program = crate::compile_to_ir(source).unwrap();
        let mut captured = Capture::default();
        let mut executor = Executor {
            calls: None,
            program: &program,
            output: &mut captured,
            input: &mut crate::runtime::input::unconfigured(),
            program_arguments: &Arguments::unconfigured(),
            heap: StringHeap::new(program.string_heap_limit),
            arrays: ArrayHeap::new(program.array_heap_limit),
        };
        executor.heap.fail_next_allocation();
        let error = executor.entry().unwrap_err();
        assert_eq!(
            error.kind,
            ErrorKind::Runtime(FailureCode::AllocationFailed)
        );
        assert_eq!(executor.output.captured(), "before\n");
        let origin = error.origin.unwrap();
        assert_eq!(
            &source[origin.span.start()..origin.span.end()],
            r#"concat("a","b")"#
        );
        executor.heap.assert_empty();
        assert_eq!(run(&program).unwrap(), "before\nab\nbad\n");
    }

    #[test]
    fn dynamic_array_examples_release_all_array_and_string_storage() {
        for source in [
            include_str!("../examples/dynamic_arrays/copy.ceru"),
            include_str!("../examples/dynamic_arrays/nested.ceru"),
            r#"enum E { A{v:[string]}, B }
                mut e:E=E::A{v:array_copy([concat("a","b")])};copy:E=e;e=E::B{};
                for(x:[string] in array_copy([array_copy([concat("c","d")])])){break;}
                print(copy);"#,
        ] {
            let p = crate::compile_to_ir(source).unwrap();
            run(&p).unwrap();
            crate::vm::run(&crate::bytecode::lower(&p).unwrap()).unwrap();
        }
    }

    #[test]
    fn forced_array_allocation_failure_preserves_origin_and_prior_output() {
        let source = r#"print("before");a:[i64]=array_copy([1]);print(a);"#;
        let p = crate::compile_to_ir(source).unwrap();
        let mut captured = Capture::default();
        let mut executor = Executor {
            calls: None,
            program: &p,
            output: &mut captured,
            input: &mut crate::runtime::input::unconfigured(),
            program_arguments: &Arguments::unconfigured(),
            heap: StringHeap::new(p.string_heap_limit),
            arrays: ArrayHeap::new(p.array_heap_limit),
        };
        executor.arrays.fail_next_allocation();
        let e = executor.entry().unwrap_err();
        assert_eq!(e.kind, ErrorKind::Runtime(FailureCode::AllocationFailed));
        assert_eq!(executor.output.captured(), "before\n");
        let origin = e.origin.unwrap();
        assert_eq!(
            &source[origin.span.start()..origin.span.end()],
            "array_copy([1])"
        );
        executor.arrays.assert_empty();
    }
}
