//! MIRとSSAで共有する個々の演算・所有操作です。CFGの実行や値の参照方式は含みません。
use super::{Fault, Result};
use crate::runtime::arguments::Arguments;
use crate::runtime::input::{self, Source};
use crate::runtime::output::{self, Sink};
use crate::{
    ir,
    mir::{self, InstructionKind as I, Operation as O},
    runtime::{
        FailureCode,
        array_heap::{ArrayHeap, ArrayValue},
        numeric::{self, Number},
        string_heap::StringHeap,
        value::{self, Value},
    },
    types::IntegerType,
};

pub(crate) struct State<'a> {
    pub output: &'a mut dyn Sink,
    pub input: Option<&'a mut dyn Source>,
    pub arguments: Option<&'a Arguments>,
    pub strings: StringHeap,
    pub arrays: ArrayHeap<Value>,
}
impl<'a> State<'a> {
    pub fn new(string_limit: u64, array_limit: u64, output: &'a mut dyn Sink) -> Self {
        Self {
            output,
            input: None,
            arguments: None,
            strings: StringHeap::new(string_limit),
            arrays: ArrayHeap::new(array_limit),
        }
    }
    pub fn check_released(&self) -> Result<()> {
        if !self.strings.is_empty() || !self.arrays.is_empty() {
            Err(Fault::invalid(
                "live owned storage after successful MIR execution",
            ))
        } else {
            Ok(())
        }
    }
}
pub(crate) trait Frame<R: Copy> {
    fn get(&self, id: R) -> Result<Value>;
    fn set(&mut self, id: R, value: Value) -> Result<()>;
    fn ty(&self, id: R) -> Result<&ir::Type>;
    fn arguments(&self, args: &[R]) -> Result<Vec<Value>> {
        args.iter().map(|id| self.get(*id)).collect()
    }
}
pub(crate) trait Calls<'a> {
    fn state(&mut self) -> &mut State<'a>;
    fn types(&self) -> &[mir::TypeDefinition];
    fn call(&mut self, id: ir::FunctionId, args: Vec<Value>) -> Result<Option<Value>>;
}
pub(crate) fn instruction<'a, R: Copy>(
    executor: &mut impl Calls<'a>,
    i: &I<R>,
    origin: mir::Origin,
    frame: &mut impl Frame<R>,
) -> Result<()> {
    match i {
        I::Assign { destination, value } => {
            let ty = frame.ty(*destination)?.clone();
            let v = operation(executor, value, &ty, frame, origin)?;
            v.check(&ty)?;
            frame.set(*destination, v)?;
        }
        I::Call {
            function,
            arguments,
            ..
        } => {
            executor.call(*function, frame.arguments(arguments)?)?;
        }
        I::CheckIndex { root, path } => {
            // 最後の要素を読まず、長さで検査します。未初期化要素の読取りとは区別します。
            let mut v = frame.get(*root)?;
            for (n, id) in path.iter().enumerate() {
                let index = checked_index(
                    frame.get(*id)?.integer(IntegerType::I64)?,
                    array_length(&v)?,
                )?;
                if n + 1 < path.len() {
                    v = array_get(v, index)?;
                }
            }
        }
        I::Store { root, path, value } => {
            let indices = path
                .iter()
                .map(|id| {
                    usize::try_from(frame.get(*id)?.integer(IntegerType::I64)?)
                        .map_err(|_| Fault::invalid("invalid checked index"))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut destination = frame.get(*root)?;
            set_path(&mut destination, &indices, frame.get(*value)?)?;
            frame.set(*root, destination)?;
        }
        I::Flush => {
            let origin = origin.source().map(|o| output::Origin {
                node_id: o.node_id,
                span: o.span,
            });
            executor
                .state()
                .output
                .flush(origin)
                .map_err(Fault::output)?;
        }
        I::Output {
            value: v,
            newline,
            quoted,
        } => {
            let text = frame.get(*v)?.text()?;
            let mut rendered = String::new();
            if *quoted {
                value::write_quoted(&mut rendered, &text);
            } else {
                rendered.push_str(&text);
            }
            if *newline {
                rendered.push('\n');
            }
            let origin = origin.source().map(|o| output::Origin {
                node_id: o.node_id,
                span: o.span,
            });
            executor
                .state()
                .output
                .write(&rendered, origin)
                .map_err(Fault::output)?;
        }
        I::ArrayInitialize { array, value } => {
            let array = dynamic(frame.get(*array)?)?;
            executor
                .state()
                .arrays
                .initialize(&array, frame.get(*value)?)?;
        }
        I::ArrayRetain { value } => executor
            .state()
            .arrays
            .retain(&dynamic(frame.get(*value)?)?)?,
        I::ArrayFree { value } => executor
            .state()
            .arrays
            .free(&dynamic(frame.get(*value)?)?)?,
        I::ArrayRangeCheck { length, start, end } => {
            let length = frame.get(*length)?.integer(IntegerType::I64)?;
            let start = frame.get(*start)?.integer(IntegerType::I64)?;
            let end = frame.get(*end)?.integer(IntegerType::I64)?;
            if start < 0 || start > end || end > length {
                return Err(Fault::failure(FailureCode::ArrayRangeOutOfBounds));
            }
        }
        I::StringManage { value, retain } => executor
            .state()
            .strings
            .manage(&frame.get(*value)?.string()?, *retain)?,
    }
    Ok(())
}
fn operation<'a, R: Copy>(
    executor: &mut impl Calls<'a>,
    op: &O<R>,
    ty: &ir::Type,
    frame: &impl Frame<R>,
    origin: mir::Origin,
) -> Result<Value> {
    Ok(match op {
        O::ArgumentCount | O::ArgumentGet(_) => {
            let index = match op {
                O::ArgumentGet(value) => Some(frame.get(*value)?.integer(IntegerType::I64)? as i64),
                _ => None,
            };
            let origin = origin
                .source()
                .ok_or_else(|| Fault::invalid("argument operation lacks source origin"))?;
            let site = output::Origin {
                node_id: origin.node_id,
                span: origin.span,
            };
            let state = executor.state();
            let missing = Arguments::unconfigured();
            let arguments = state.arguments.unwrap_or(&missing);
            let result = match index {
                None => arguments
                    .count_at(site)
                    .map(|n| Value::Number(Number::Integer(n as i128, IntegerType::I64))),
                Some(index) => arguments
                    .get_value(&mut state.strings, index, site)
                    .map(Value::String),
            };
            result.map_err(|f| Fault {
                kind: super::ErrorKind::Arguments(Box::new(f)),
                origin: Some(origin),
                location: None,
            })?
        }
        O::ReadLine(value) => {
            let limit = frame.get(*value)?.integer(IntegerType::I64)? as i64;
            let origin = origin
                .source()
                .ok_or_else(|| Fault::invalid("input operation lacks source origin"))?;
            let origin = input::Origin {
                node_id: origin.node_id,
                span: origin.span,
            };
            let state = executor.state();
            let mut missing = input::unconfigured();
            let source = state.input.as_deref_mut().unwrap_or(&mut missing);
            Value::String(
                source
                    .read_value(&mut state.strings, limit, origin)
                    .map_err(|f| Fault {
                        kind: super::ErrorKind::Input(Box::new(f)),
                        origin: Some(mir::SourceOrigin {
                            node_id: origin.node_id,
                            span: origin.span,
                        }),
                        location: None,
                    })?,
            )
        }
        O::Literal(v) => match v {
            mir::Literal::Boolean(v) => Value::Bool(*v),
            mir::Literal::String(v) => Value::String(v.clone().into()),
            mir::Literal::Integer(v) => {
                let ir::Type::Integer(t) = ty else {
                    return Err(Fault::invalid("integer literal type"));
                };
                Value::Number(Number::Integer(*v, *t))
            }
            mir::Literal::Float(v) => Value::Number(match ty {
                ir::Type::F32 => Number::F32(v.parse().map_err(|_| Fault::invalid("f32 literal"))?),
                ir::Type::F64 => Number::F64(v.parse().map_err(|_| Fault::invalid("f64 literal"))?),
                _ => return Err(Fault::invalid("float literal type")),
            }),
        },
        O::Copy(v) => frame.get(*v)?,
        O::Unary { op, value: v } => value::unary(*op, frame.get(*v)?)?,
        O::Binary { op, left, right } => value::binary(*op, frame.get(*left)?, frame.get(*right)?)?,
        O::ConvertInteger {
            value, from, to, ..
        } => {
            let n = frame.get(*value)?.integer(*from)?;
            if !to.contains(n) {
                return Err(Fault::failure(FailureCode::IntegerConversionOutOfRange));
            }
            Value::Number(Number::Integer(n, *to))
        }
        O::ConvertNumeric {
            value: v,
            from,
            to,
            mode,
            ..
        } => Value::Number(
            numeric::convert_with_mode(frame.get(*v)?.number()?, *from, *to, *mode)
                .map_err(value::numeric_error)?,
        ),
        O::Array(values) => {
            let ir::Type::Array { element, .. } = ty else {
                return Err(Fault::invalid("array result type"));
            };
            Value::Array {
                element: *element.clone(),
                values: frame.arguments(values)?,
            }
        }
        O::Construct { ty, base, fields } => {
            let count = executor
                .types()
                .get(ty.0)
                .ok_or_else(|| Fault::invalid("unknown product"))?
                .fields
                .len();
            let mut result = if let Some(base) = base {
                let Value::Aggregate { fields, .. } = frame.get(*base)? else {
                    return Err(Fault::invalid("product base"));
                };
                fields.into_iter().map(Some).collect::<Vec<_>>()
            } else {
                vec![None; count]
            };
            for (id, v) in fields {
                *result
                    .get_mut(id.0)
                    .ok_or_else(|| Fault::invalid("unknown field"))? = Some(frame.get(*v)?);
            }
            Value::Aggregate {
                type_id: *ty,
                fields: result
                    .into_iter()
                    .map(|v| v.ok_or_else(|| Fault::invalid("missing field")))
                    .collect::<Result<_>>()?,
            }
        }
        O::Field { base, field, .. } => {
            let Value::Aggregate { fields, .. } = frame.get(*base)? else {
                return Err(Fault::invalid("field of non-product"));
            };
            fields
                .get(field.0)
                .cloned()
                .ok_or_else(|| Fault::invalid("unknown field"))?
        }
        O::Index { base, index } => {
            let base = frame.get(*base)?;
            let index = checked_index(
                frame.get(*index)?.integer(IntegerType::I64)?,
                array_length(&base)?,
            )?;
            array_get(base, index)?
        }
        O::Call {
            function,
            arguments,
            ..
        } => executor
            .call(*function, frame.arguments(arguments)?)?
            .ok_or_else(|| Fault::invalid("void call used as value"))?,
        O::ArrayLength(v) => Value::Number(Number::Integer(
            array_length(&frame.get(*v)?)? as i128,
            IntegerType::I64,
        )),
        O::StringByteLength(v) => Value::Number(Number::Integer(
            frame.get(*v)?.string()?.len() as i128,
            IntegerType::I64,
        )),
        O::StringConcat { left, right } => Value::String(
            executor
                .state()
                .strings
                .concat(&frame.get(*left)?.string()?, &frame.get(*right)?.string()?)?,
        ),
        O::ArrayAllocate {
            length,
            element_width,
        } => {
            let ir::Type::DynamicArray { element } = ty else {
                return Err(Fault::invalid("array allocate type"));
            };
            let length = u64::try_from(frame.get(*length)?.integer(IntegerType::I64)?)
                .map_err(|_| Fault::failure(FailureCode::ArrayLengthOutOfRange))?;
            Value::DynamicArray {
                element: *element.clone(),
                storage: executor.state().arrays.allocate(length, *element_width)?,
            }
        }
        O::ArrayReleaseOwner(v) => Value::Bool(
            executor
                .state()
                .arrays
                .release_owner(&dynamic(frame.get(*v)?)?)?,
        ),
    })
}
fn dynamic(v: Value) -> Result<ArrayValue<Value>> {
    if let Value::DynamicArray { storage, .. } = v {
        Ok(storage)
    } else {
        Err(Fault::invalid("expected dynamic array"))
    }
}
fn array_length(v: &Value) -> Result<usize> {
    match v {
        Value::Array { values, .. } => Ok(values.len()),
        Value::DynamicArray { storage, .. } => Ok(storage.len()),
        _ => Err(Fault::invalid("expected array")),
    }
}
fn checked_index(index: i128, length: usize) -> Result<usize> {
    usize::try_from(index)
        .ok()
        .filter(|i| *i < length)
        .ok_or_else(|| Fault::failure(FailureCode::ArrayIndexOutOfBounds))
}
fn array_get(v: Value, index: usize) -> Result<Value> {
    match v {
        Value::Array { values, .. } => values
            .get(index)
            .cloned()
            .ok_or_else(|| Fault::invalid("invalid checked index")),
        Value::DynamicArray { storage, .. } => Ok(storage.get(index)?),
        _ => Err(Fault::invalid("expected array")),
    }
}
fn set_path(v: &mut Value, path: &[usize], replacement: Value) -> Result<()> {
    let Some((&index, rest)) = path.split_first() else {
        *v = replacement;
        return Ok(());
    };
    match v {
        Value::Array { values, .. } => set_path(
            values
                .get_mut(index)
                .ok_or_else(|| Fault::invalid("invalid checked index"))?,
            rest,
            replacement,
        ),
        Value::DynamicArray { storage, .. } => {
            let mut item = storage.get(index)?;
            set_path(&mut item, rest, replacement)?;
            storage.set(index, item)?;
            Ok(())
        }
        _ => Err(Fault::invalid("invalid array path")),
    }
}
