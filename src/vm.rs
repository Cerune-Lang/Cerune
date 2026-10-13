use crate::runtime::output::{self, Capture, Sink};
use crate::runtime::{
    array_heap::{ArrayError, ArrayHeap, ArrayValue},
    float_output,
};
mod string_heap;
use string_heap::{StringHeap, StringValue};
mod numeric;
pub mod render;
pub use numeric::NumericConversionFailure;

use crate::bytecode::{ArrayAccess, BytecodeProgram, InstructionKind, ReturnType, Type};
use crate::types::IntegerType;

/// 整数の桁あふれを起こした演算を表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerOperation {
    Add,
    Subtract,
    Multiply,
    Negate,
    ShiftLeft,
}

/// Cerune VMの実行中に発生した問題の種類を表します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmErrorKind {
    Output(Box<output::Failure>),
    ArrayRangeOutOfBounds,
    ArrayLengthOutOfRange,
    InvalidArrayOwnership,
    AllocationSizeOverflow,
    AllocationLimitExceeded,
    AllocationFailed,
    /// bytecodeが既に解放した動的文字列を管理しようとしました。
    InvalidStringOwnership,
    /// bytecodeが対応しない型の組み合わせで切り捨てを要求しました。
    InvalidNumericConversion {
        from: crate::types::NumericType,
        to: crate::types::NumericType,
    },
    /// 変換前後の型と、値を保てなかった理由を保持します。
    NumericConversionFailed {
        from: crate::types::NumericType,
        to: crate::types::NumericType,
        reason: NumericConversionFailure,
    },
    /// シフト量は0以上、左辺の型のビット数未満でなければなりません。
    InvalidShiftCount {
        ty: IntegerType,
        count: i128,
    },
    RemainderByZero,
    /// 明示変換の入力を変換先の範囲で表せませんでした。
    IntegerConversionOutOfRange {
        from: IntegerType,
        to: IntegerType,
    },
    /// bytecodeが整数型の範囲外の値を持っていました。
    InvalidIntegerValue {
        ty: IntegerType,
    },
    InvalidNegationType {
        ty: IntegerType,
    },
    /// 実行位置がbytecodeの範囲外へ到達しました。
    InstructionOutOfBounds,

    /// 存在しないスロットへアクセスしました。
    InvalidSlot {
        slot: usize,
    },

    /// 存在しないproduct typeを使おうとしました。
    InvalidType {
        type_id: usize,
    },

    /// product typeに存在しないfieldを使おうとしました。
    InvalidField {
        type_id: usize,
        field_id: usize,
    },

    /// 存在しない関数を呼び出そうとしました。
    InvalidFunction {
        function_id: usize,
    },

    /// 関数へ渡された引数の数が定義と一致しませんでした。
    InvalidArgumentCount {
        expected: usize,
        actual: usize,
    },

    /// 関数の定義と一致しない方法で値を返そうとしました。
    InvalidReturn,

    /// 初期化前のスロットを読み取りました。
    UninitializedSlot {
        slot: usize,
    },

    /// 別の初期化命令が初期化済みのスロットへ書き込みました。
    SlotAlreadyInitialized {
        slot: usize,
    },

    /// 初期化前のスロットへ再代入しようとしました。
    AssignmentToUninitializedSlot {
        slot: usize,
    },

    /// 不変のスロットへ再代入しようとしました。
    AssignmentToImmutableSlot {
        slot: usize,
    },

    /// 必要な値がスタックにありませんでした。
    StackUnderflow,

    /// 命令が期待した型と実際の値の型が一致しませんでした。
    TypeMismatch {
        expected: Type,
        actual: Type,
    },

    /// その型では利用できない比較命令を実行しようとしました。
    InvalidComparisonType {
        ty: Type,
    },

    /// 整数をゼロで除算しようとしました。
    DivisionByZero,

    /// 整数除算の結果を指定された型で表現できませんでした。
    DivisionOverflow,

    /// 整数演算の結果を指定された型で表現できませんでした。
    IntegerOverflow {
        operation: IntegerOperation,
        ty: Type,
    },

    /// 配列の外側を読み取ろうとしました。
    ArrayIndexOutOfBounds {
        index: i64,
        length: usize,
    },

    /// VM停止時に未使用の値がスタックへ残っていました。
    UnusedStackValues {
        count: usize,
    },
}

/// Cerune VMの実行エラーと発生したbytecode命令位置を表します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmError {
    kind: VmErrorKind,
    instruction_index: usize,
    function_id: Option<usize>,
    output: Box<str>,
}

impl VmError {
    fn new(kind: VmErrorKind, instruction_index: usize) -> Self {
        Self {
            kind,
            instruction_index,
            function_id: None,
            output: Box::from(""),
        }
    }

    const fn in_function(mut self, function_id: usize) -> Self {
        if self.function_id.is_none() {
            self.function_id = Some(function_id);
        }
        self
    }

    /// エラーの種類を返します。
    pub fn kind(&self) -> VmErrorKind {
        self.kind.clone()
    }

    /// `emit-bytecode`の表示と対応する0から始まる命令番号を返します。
    pub const fn instruction_index(&self) -> usize {
        self.instruction_index
    }

    /// エラーが関数内で起きた場合、その関数番号を返します。
    pub const fn function_id(&self) -> Option<usize> {
        self.function_id
    }

    /// 停止するまでに実行されたprintの出力です。失敗で取り消しません。
    pub fn output(&self) -> &str {
        &self.output
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Value {
    DynamicArray {
        element: Type,
        storage: ArrayValue<Value>,
    },
    Bool(bool),
    String(StringValue),
    Integer(i128, IntegerType),
    F32(f32),
    F64(f64),
    Aggregate {
        type_id: usize,
        fields: Vec<Value>,
    },
    Array {
        element: Type,
        values: Vec<Value>,
    },
}

impl Value {
    fn ty(&self) -> Type {
        match self {
            Self::DynamicArray { element, .. } => Type::DynamicArray {
                element: Box::new(element.clone()),
            },
            Self::Bool(_) => Type::Bool,
            Self::String(_) => Type::String,
            Self::Integer(_, ty) => Type::Integer(*ty),
            Self::F32(_) => Type::F32,
            Self::F64(_) => Type::F64,
            Self::Aggregate { type_id, .. } => Type::Named(*type_id),
            Self::Array { element, values } => Type::Array {
                element: Box::new(element.clone()),
                length: values.len(),
            },
        }
    }
}

type VmResult<T> = Result<T, VmErrorKind>;

fn at_instruction<T>(result: VmResult<T>, instruction_index: usize) -> Result<T, VmError> {
    result.map_err(|kind| VmError::new(kind, instruction_index))
}

#[derive(Clone, Copy)]
enum Frame {
    Entry,
    Function(usize),
}

pub fn run(program: &BytecodeProgram) -> Result<String, VmError> {
    run_sink(program, &mut Capture::default())
}

/// 接続先へ逐次出力し、ソースのflush命令を実行します。終了時の自動flushや再収集は行いません。
pub fn run_with_output<W: std::io::Write>(
    program: &BytecodeProgram,
    output: &mut output::Output<W>,
) -> Result<(), VmError> {
    run_sink(program, output).map(|_| ())
}
fn run_sink(program: &BytecodeProgram, output: &mut dyn Sink) -> Result<String, VmError> {
    let mut heap = StringHeap::new(program.string_heap_limit);
    let mut arrays = ArrayHeap::new(program.array_heap_limit);
    if let Err(mut error) = execute_frame(
        program,
        Frame::Entry,
        Vec::new(),
        output,
        &mut heap,
        &mut arrays,
    ) {
        error.output = output.captured().into();
        return Err(error);
    }
    #[cfg(test)]
    arrays.assert_empty();
    Ok(output.captured().to_owned())
}

/// 定数式専用の引数なしフレーム。呼出し側が副作用を排除したIRだけを渡します。
pub(crate) fn evaluate_constant(program: &BytecodeProgram) -> Result<Value, VmError> {
    execute_function(program, 0, Vec::new())
        .map(|(value, _output)| value.expect("constant evaluator returns a value"))
}

/// コンパイル済みbytecode内の関数を、指定した引数で直接実行します。
///
/// 正常終了時は戻り値と実行中の出力を返し、失敗時はそれまでの出力を
/// `VmError` に保持します。
pub(crate) fn execute_function(
    program: &BytecodeProgram,
    function_id: usize,
    arguments: Vec<Value>,
) -> Result<(Option<Value>, String), VmError> {
    let mut output = Capture::default();

    match execute_frame(
        program,
        Frame::Function(function_id),
        arguments,
        &mut output,
        &mut StringHeap::new(program.string_heap_limit),
        &mut ArrayHeap::new(program.array_heap_limit),
    ) {
        Ok(value) => Ok((value.map(string_heap::freeze), output.0)),
        Err(mut error) => {
            error.output = output.0.into_boxed_str();
            Err(error)
        }
    }
}

fn execute_frame(
    program: &BytecodeProgram,
    frame: Frame,
    arguments: Vec<Value>,
    output: &mut dyn Sink,
    heap: &mut StringHeap,
    arrays: &mut ArrayHeap<Value>,
) -> Result<Option<Value>, VmError> {
    let result = execute_frame_inner(program, frame, arguments, output, heap, arrays);
    match (frame, result) {
        (Frame::Function(function_id), Err(error)) => Err(error.in_function(function_id)),
        (_, result) => result,
    }
}

fn execute_frame_inner(
    program: &BytecodeProgram,
    frame: Frame,
    arguments: Vec<Value>,
    output: &mut dyn Sink,
    heap: &mut StringHeap,
    arrays: &mut ArrayHeap<Value>,
) -> Result<Option<Value>, VmError> {
    let (instructions, slot_info, parameter_count, return_type) = match frame {
        Frame::Entry => (
            program.instructions.as_slice(),
            program.slots.as_slice(),
            0,
            ReturnType::Void,
        ),
        Frame::Function(function_id) => {
            let function = program
                .functions
                .get(function_id)
                .ok_or_else(|| VmError::new(VmErrorKind::InvalidFunction { function_id }, 0))?;
            (
                function.instructions.as_slice(),
                function.slots.as_slice(),
                function.parameter_count,
                function.return_type.clone(),
            )
        }
    };

    if arguments.len() != parameter_count {
        return Err(VmError::new(
            VmErrorKind::InvalidArgumentCount {
                expected: parameter_count,
                actual: arguments.len(),
            },
            0,
        ));
    }

    let mut stack: Vec<Value> = Vec::new();
    let mut slots: Vec<Option<Value>> = vec![None; slot_info.len()];
    let mut initializers: Vec<Option<usize>> = vec![None; slot_info.len()];

    for (index, argument) in arguments.into_iter().enumerate() {
        let expected = slot_info[index].ty.clone();
        if argument.ty() != expected {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch {
                    expected,
                    actual: argument.ty(),
                },
                0,
            ));
        }
        slots[index] = Some(argument);
        initializers[index] = Some(usize::MAX);
    }

    let mut pc = 0;

    loop {
        let instruction = instructions
            .get(pc)
            .ok_or_else(|| VmError::new(VmErrorKind::InstructionOutOfBounds, pc))?;

        match &instruction.kind {
            InstructionKind::ArrayAllocate {
                element,
                element_width,
            } => {
                let n = at_instruction(pop_i64(&mut stack), pc)?;
                let n = u64::try_from(n)
                    .map_err(|_| VmError::new(VmErrorKind::ArrayLengthOutOfRange, pc))?;
                let storage =
                    at_instruction(arrays.allocate(n, *element_width).map_err(array_error), pc)?;
                stack.push(Value::DynamicArray {
                    element: element.clone(),
                    storage,
                });
            }
            InstructionKind::ArrayInitialize => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let (element, storage) = at_instruction(pop_dynamic_array(&mut stack), pc)?;
                if value.ty() != element {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected: element,
                            actual: value.ty(),
                        },
                        pc,
                    ));
                }
                at_instruction(arrays.initialize(&storage, value).map_err(array_error), pc)?;
            }
            InstructionKind::ArrayRetain
            | InstructionKind::ArrayFree
            | InstructionKind::ArrayReleaseOwner => {
                let (_, storage) = at_instruction(pop_dynamic_array(&mut stack), pc)?;
                match instruction.kind {
                    InstructionKind::ArrayRetain => {
                        at_instruction(arrays.retain(&storage).map_err(array_error), pc)?
                    }
                    InstructionKind::ArrayFree => {
                        at_instruction(arrays.free(&storage).map_err(array_error), pc)?
                    }
                    _ => {
                        let last = at_instruction(
                            arrays.release_owner(&storage).map_err(array_error),
                            pc,
                        )?;
                        stack.push(Value::Bool(last));
                    }
                }
            }
            InstructionKind::ArrayRangeCheck => {
                let end = at_instruction(pop_i64(&mut stack), pc)?;
                let start = at_instruction(pop_i64(&mut stack), pc)?;
                let length = at_instruction(pop_i64(&mut stack), pc)?;
                if start < 0 || start > end || end > length {
                    return Err(VmError::new(VmErrorKind::ArrayRangeOutOfBounds, pc));
                }
            }
            InstructionKind::DynamicArrayLength { element }
            | InstructionKind::DynamicIndex { element } => {
                let index = if matches!(instruction.kind, InstructionKind::DynamicIndex { .. }) {
                    Some(at_instruction(pop_i64(&mut stack), pc)?)
                } else {
                    None
                };
                let (actual, storage) = at_instruction(pop_dynamic_array(&mut stack), pc)?;
                if actual != *element {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected: Type::DynamicArray {
                                element: Box::new(element.clone()),
                            },
                            actual: Type::DynamicArray {
                                element: Box::new(actual),
                            },
                        },
                        pc,
                    ));
                }
                if let Some(index) = index {
                    let index = at_instruction(array_index(index, storage.len()), pc)?;
                    stack.push(at_instruction(storage.get(index).map_err(array_error), pc)?);
                } else {
                    stack.push(Value::Integer(storage.len() as i128, IntegerType::I64));
                }
            }
            InstructionKind::ArrayLength { element, length } => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let expected = Type::Array {
                    element: Box::new(element.clone()),
                    length: *length,
                };
                let actual = value.ty();
                if actual != expected {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch { expected, actual },
                        pc,
                    ));
                }
                stack.push(Value::Integer(*length as i128, IntegerType::I64));
            }
            InstructionKind::StringConcat => {
                let right = at_instruction(string_heap::pop(&mut stack), pc)?;
                let left = at_instruction(string_heap::pop(&mut stack), pc)?;
                let value =
                    at_instruction(heap.concat(&left, &right).map_err(string_heap::error), pc)?;
                stack.push(Value::String(value));
            }
            InstructionKind::StringManage { retain } => {
                let value = at_instruction(string_heap::pop(&mut stack), pc)?;
                at_instruction(heap.manage(&value, *retain).map_err(string_heap::error), pc)?;
            }
            InstructionKind::StringByteLength => {
                let value = at_instruction(pop_string(&mut stack), pc)?;
                // 対応する32/64ビット環境のRust文字列はisize::MAXを超えません。
                stack.push(Value::Integer(value.len() as i128, IntegerType::I64));
            }
            InstructionKind::ConvertNumeric { from, to, mode } => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                stack.push(at_instruction(
                    numeric::convert_with_mode(value, *from, *to, *mode),
                    pc,
                )?);
            }
            InstructionKind::ConvertInteger { from, to } => {
                let value = at_instruction(pop_integer(&mut stack, *from), pc)?;
                if !to.contains(value) {
                    return Err(VmError::new(
                        VmErrorKind::IntegerConversionOutOfRange {
                            from: *from,
                            to: *to,
                        },
                        pc,
                    ));
                }
                stack.push(Value::Integer(value, *to));
            }
            InstructionKind::PushBool(value) => {
                stack.push(Value::Bool(*value));
            }
            InstructionKind::PushString(value) => {
                stack.push(Value::String(value.clone().into()));
            }

            InstructionKind::PushInteger(value, ty) => {
                if !ty.contains(*value) {
                    return Err(VmError::new(
                        VmErrorKind::InvalidIntegerValue { ty: *ty },
                        pc,
                    ));
                }
                stack.push(Value::Integer(*value, *ty));
            }

            InstructionKind::PushF32(value) => {
                stack.push(Value::F32(*value));
            }

            InstructionKind::PushF64(value) => {
                stack.push(Value::F64(*value));
            }

            InstructionKind::Load(slot) => {
                let value = slots
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?
                    .clone()
                    .ok_or_else(|| {
                        VmError::new(VmErrorKind::UninitializedSlot { slot: *slot }, pc)
                    })?;

                stack.push(value);
            }

            InstructionKind::Store(slot) => {
                let value = at_instruction(pop_value(&mut stack), pc)?;

                let expected = slot_info
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?
                    .ty
                    .clone();

                if value.ty() != expected {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected,
                            actual: value.ty(),
                        },
                        pc,
                    ));
                }

                let destination = slots
                    .get_mut(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;

                let initializer = initializers
                    .get_mut(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;

                if initializer.is_some_and(|initializer| initializer != pc) {
                    return Err(VmError::new(
                        VmErrorKind::SlotAlreadyInitialized { slot: *slot },
                        pc,
                    ));
                }

                *initializer = Some(pc);
                *destination = Some(value);
            }

            InstructionKind::Assign(slot) => {
                let value = at_instruction(pop_value(&mut stack), pc)?;

                let slot_definition = slot_info
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;

                if !slot_definition.mutable {
                    return Err(VmError::new(
                        VmErrorKind::AssignmentToImmutableSlot { slot: *slot },
                        pc,
                    ));
                }

                let expected = slot_definition.ty.clone();

                if value.ty() != expected {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected,
                            actual: value.ty(),
                        },
                        pc,
                    ));
                }

                let destination = slots
                    .get_mut(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;

                if destination.is_none() {
                    return Err(VmError::new(
                        VmErrorKind::AssignmentToUninitializedSlot { slot: *slot },
                        pc,
                    ));
                }

                *destination = Some(value);
            }

            InstructionKind::ArrayAssign { slot, path } => {
                let replacement = at_instruction(pop_value(&mut stack), pc)?;
                let mut indices = Vec::with_capacity(path.len());
                for _ in 0..path.len() {
                    indices.push(at_instruction(pop_i64(&mut stack), pc)?);
                }
                indices.reverse();

                let slot_definition = slot_info
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;
                if !slot_definition.mutable {
                    return Err(VmError::new(
                        VmErrorKind::AssignmentToImmutableSlot { slot: *slot },
                        pc,
                    ));
                }

                let destination = slots
                    .get_mut(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;
                let Some(current) = destination else {
                    return Err(VmError::new(
                        VmErrorKind::AssignmentToUninitializedSlot { slot: *slot },
                        pc,
                    ));
                };

                let mut updated = current.clone();
                at_instruction(
                    assign_array_path(&mut updated, path, &indices, replacement),
                    pc,
                )?;
                *current = updated;
            }

            InstructionKind::ArrayCheck { slot, path } => {
                let slot_definition = slot_info
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?;
                if !slot_definition.mutable {
                    return Err(VmError::new(
                        VmErrorKind::AssignmentToImmutableSlot { slot: *slot },
                        pc,
                    ));
                }
                let current = slots
                    .get(*slot)
                    .ok_or_else(|| VmError::new(VmErrorKind::InvalidSlot { slot: *slot }, pc))?
                    .as_ref()
                    .ok_or_else(|| {
                        VmError::new(
                            VmErrorKind::AssignmentToUninitializedSlot { slot: *slot },
                            pc,
                        )
                    })?;
                if stack.len() < path.len() {
                    return Err(VmError::new(VmErrorKind::StackUnderflow, pc));
                }
                let indices = &stack[stack.len() - path.len()..];
                at_instruction(check_array_path(current, path, indices), pc)?;
            }

            InstructionKind::Construct {
                type_id,
                fields,
                has_base,
            } => {
                let definition = program.type_definitions.get(*type_id).ok_or_else(|| {
                    VmError::new(VmErrorKind::InvalidType { type_id: *type_id }, pc)
                })?;
                let mut values = vec![None; definition.fields.len()];

                for field in fields.iter().rev() {
                    let value = at_instruction(pop_value(&mut stack), pc)?;
                    let expected = definition
                        .fields
                        .get(field.field_id)
                        .ok_or_else(|| {
                            VmError::new(
                                VmErrorKind::InvalidField {
                                    type_id: *type_id,
                                    field_id: field.field_id,
                                },
                                pc,
                            )
                        })?
                        .ty
                        .clone();
                    if value.ty() != expected {
                        return Err(VmError::new(
                            VmErrorKind::TypeMismatch {
                                expected,
                                actual: value.ty(),
                            },
                            pc,
                        ));
                    }
                    values[field.field_id] = Some(value);
                }

                if *has_base {
                    let base = at_instruction(pop_value(&mut stack), pc)?;
                    let actual = base.ty();
                    if actual != Type::Named(*type_id) {
                        return Err(VmError::new(
                            VmErrorKind::TypeMismatch {
                                expected: Type::Named(*type_id),
                                actual,
                            },
                            pc,
                        ));
                    }
                    let Value::Aggregate {
                        fields: inherited, ..
                    } = base
                    else {
                        unreachable!("named values are aggregates")
                    };
                    // Load/引数受け渡しで複製した値を使い、元のスロットには触れません。
                    for (slot, value) in values.iter_mut().zip(inherited) {
                        if slot.is_none() {
                            *slot = Some(value);
                        }
                    }
                }
                let values = values
                    .into_iter()
                    .enumerate()
                    .map(|(field_id, value)| {
                        value.ok_or_else(|| {
                            VmError::new(
                                VmErrorKind::InvalidField {
                                    type_id: *type_id,
                                    field_id,
                                },
                                pc,
                            )
                        })
                    })
                    .collect::<Result<_, _>>()?;
                stack.push(Value::Aggregate {
                    type_id: *type_id,
                    fields: values,
                });
            }

            InstructionKind::FieldGet { type_id, field_id } => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let actual_type = value.ty();
                let Value::Aggregate {
                    type_id: aggregate_type,
                    fields,
                } = value
                else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected: Type::Named(*type_id),
                            actual: actual_type,
                        },
                        pc,
                    ));
                };
                if aggregate_type != *type_id {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch {
                            expected: Type::Named(*type_id),
                            actual: Type::Named(aggregate_type),
                        },
                        pc,
                    ));
                }
                let field = fields.get(*field_id).cloned().ok_or_else(|| {
                    VmError::new(
                        VmErrorKind::InvalidField {
                            type_id: *type_id,
                            field_id: *field_id,
                        },
                        pc,
                    )
                })?;
                stack.push(field);
            }

            InstructionKind::ConstructArray { element, length } => {
                let mut values = Vec::with_capacity(*length);
                for _ in 0..*length {
                    let value = at_instruction(pop_value(&mut stack), pc)?;
                    if value.ty() != *element {
                        return Err(VmError::new(
                            VmErrorKind::TypeMismatch {
                                expected: element.clone(),
                                actual: value.ty(),
                            },
                            pc,
                        ));
                    }
                    values.push(value);
                }
                values.reverse();
                stack.push(Value::Array {
                    element: element.clone(),
                    values,
                });
            }

            InstructionKind::Index { element, length } => {
                let index = at_instruction(pop_i64(&mut stack), pc)?;
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let expected = Type::Array {
                    element: Box::new(element.clone()),
                    length: *length,
                };
                let actual = value.ty();
                let Value::Array {
                    element: actual_element,
                    values,
                } = value
                else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch { expected, actual },
                        pc,
                    ));
                };
                if actual_element != *element || values.len() != *length {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch { expected, actual },
                        pc,
                    ));
                }
                let value = usize::try_from(index)
                    .ok()
                    .and_then(|index| values.get(index).cloned())
                    .ok_or_else(|| {
                        VmError::new(
                            VmErrorKind::ArrayIndexOutOfBounds {
                                index,
                                length: *length,
                            },
                            pc,
                        )
                    })?;
                stack.push(value);
            }

            InstructionKind::Call {
                function_id,
                argument_count,
            } => {
                let function = program.functions.get(*function_id).ok_or_else(|| {
                    VmError::new(
                        VmErrorKind::InvalidFunction {
                            function_id: *function_id,
                        },
                        pc,
                    )
                })?;
                if *argument_count != function.parameter_count {
                    return Err(VmError::new(
                        VmErrorKind::InvalidArgumentCount {
                            expected: function.parameter_count,
                            actual: *argument_count,
                        },
                        pc,
                    ));
                }

                let mut arguments = Vec::with_capacity(*argument_count);
                for _ in 0..*argument_count {
                    arguments.push(at_instruction(pop_value(&mut stack), pc)?);
                }
                arguments.reverse();

                if let Some(value) = execute_frame(
                    program,
                    Frame::Function(*function_id),
                    arguments,
                    output,
                    heap,
                    arrays,
                )? {
                    stack.push(value);
                }
            }

            InstructionKind::Return { has_value } => {
                if matches!(frame, Frame::Entry) {
                    return Err(VmError::new(VmErrorKind::InvalidReturn, pc));
                }

                let value = match (&return_type, *has_value) {
                    (ReturnType::Void, false) => None,
                    (ReturnType::Value(expected), true) => {
                        let value = at_instruction(pop_value(&mut stack), pc)?;
                        if value.ty() != *expected {
                            return Err(VmError::new(
                                VmErrorKind::TypeMismatch {
                                    expected: expected.clone(),
                                    actual: value.ty(),
                                },
                                pc,
                            ));
                        }
                        Some(value)
                    }
                    _ => return Err(VmError::new(VmErrorKind::InvalidReturn, pc)),
                };

                if !stack.is_empty() {
                    return Err(VmError::new(
                        VmErrorKind::UnusedStackValues { count: stack.len() },
                        pc,
                    ));
                }
                return Ok(value);
            }

            InstructionKind::Add(ty) => {
                at_instruction(binary(ty.clone(), &mut stack, BinaryOperation::Add), pc)?;
            }

            InstructionKind::Subtract(ty) => {
                at_instruction(
                    binary(ty.clone(), &mut stack, BinaryOperation::Subtract),
                    pc,
                )?;
            }

            InstructionKind::Multiply(ty) => {
                at_instruction(
                    binary(ty.clone(), &mut stack, BinaryOperation::Multiply),
                    pc,
                )?;
            }

            InstructionKind::Divide(ty) => {
                at_instruction(binary(ty.clone(), &mut stack, BinaryOperation::Divide), pc)?;
            }

            InstructionKind::Equal(ty) => {
                at_instruction(compare(ty.clone(), &mut stack, Comparison::Equal), pc)?;
            }

            InstructionKind::NotEqual(ty) => {
                at_instruction(compare(ty.clone(), &mut stack, Comparison::NotEqual), pc)?;
            }

            InstructionKind::Less(ty) => {
                at_instruction(compare(ty.clone(), &mut stack, Comparison::Less), pc)?;
            }

            InstructionKind::LessEqual(ty) => {
                at_instruction(compare(ty.clone(), &mut stack, Comparison::LessEqual), pc)?;
            }

            InstructionKind::Greater(ty) => {
                at_instruction(compare(ty.clone(), &mut stack, Comparison::Greater), pc)?;
            }

            InstructionKind::GreaterEqual(ty) => {
                at_instruction(
                    compare(ty.clone(), &mut stack, Comparison::GreaterEqual),
                    pc,
                )?;
            }

            InstructionKind::Negate(ty) => {
                at_instruction(negate(ty.clone(), &mut stack), pc)?;
            }

            InstructionKind::BitNot(ty) => {
                let value = at_instruction(pop_integer(&mut stack, *ty), pc)?;
                let mask = if ty.is_signed() { -1 } else { ty.maximum() };
                stack.push(Value::Integer(value ^ mask, *ty));
            }
            InstructionKind::Remainder(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::Remainder),
                    pc,
                )?;
            }
            InstructionKind::BitAnd(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::BitAnd),
                    pc,
                )?;
            }
            InstructionKind::BitOr(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::BitOr),
                    pc,
                )?;
            }
            InstructionKind::BitXor(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::BitXor),
                    pc,
                )?;
            }
            InstructionKind::ShiftLeft(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::ShiftLeft),
                    pc,
                )?;
            }
            InstructionKind::ShiftRight(ty) => {
                at_instruction(
                    integer_binary(*ty, &mut stack, IntegerBinaryOperation::ShiftRight),
                    pc,
                )?;
            }
            InstructionKind::Not => {
                let value = at_instruction(pop_bool(&mut stack), pc)?;
                stack.push(Value::Bool(!value));
            }

            InstructionKind::Write { ty, quoted } => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let text = at_instruction(format_value(value, ty.clone()), pc)?;
                let mut rendered = String::new();
                if *quoted {
                    crate::runtime::value::write_quoted(&mut rendered, &text);
                } else {
                    rendered.push_str(&text);
                }
                at_instruction(
                    output
                        .write(
                            &rendered,
                            match instruction.origin {
                                crate::bytecode::InstructionOrigin::Source { node_id, span } => {
                                    Some(output::Origin { node_id, span })
                                }
                                crate::bytecode::InstructionOrigin::Synthetic => None,
                            },
                        )
                        .map_err(|f| VmErrorKind::Output(Box::new(f))),
                    pc,
                )?;
            }
            InstructionKind::Flush => {
                at_instruction(
                    output
                        .flush(match instruction.origin {
                            crate::bytecode::InstructionOrigin::Source { node_id, span } => {
                                Some(output::Origin { node_id, span })
                            }
                            crate::bytecode::InstructionOrigin::Synthetic => None,
                        })
                        .map_err(|f| VmErrorKind::Output(Box::new(f))),
                    pc,
                )?;
            }
            InstructionKind::Print(ty) => {
                let value = at_instruction(pop_value(&mut stack), pc)?;
                let mut text = at_instruction(format_value(value, ty.clone()), pc)?;
                text.push('\n');
                at_instruction(
                    output
                        .write(
                            &text,
                            match instruction.origin {
                                crate::bytecode::InstructionOrigin::Source { node_id, span } => {
                                    Some(output::Origin { node_id, span })
                                }
                                crate::bytecode::InstructionOrigin::Synthetic => None,
                            },
                        )
                        .map_err(|f| VmErrorKind::Output(Box::new(f))),
                    pc,
                )?;
            }
            InstructionKind::JumpIfFalse(target) => {
                let condition = at_instruction(pop_bool(&mut stack), pc)?;

                if !condition {
                    pc = *target;
                    continue;
                }
            }

            InstructionKind::Jump(target) => {
                pc = *target;
                continue;
            }

            InstructionKind::Halt => {
                if matches!(frame, Frame::Function(_)) {
                    return Err(VmError::new(VmErrorKind::InvalidReturn, pc));
                }
                break;
            }
        }

        pc += 1;
    }

    if !stack.is_empty() {
        return Err(VmError::new(
            VmErrorKind::UnusedStackValues { count: stack.len() },
            pc,
        ));
    }

    Ok(None)
}

#[derive(Clone, Copy)]
enum IntegerBinaryOperation {
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

fn integer_binary(
    ty: IntegerType,
    stack: &mut Vec<Value>,
    op: IntegerBinaryOperation,
) -> VmResult<()> {
    let right = pop_integer(stack, ty)?;
    let left = pop_integer(stack, ty)?;
    if matches!(
        op,
        IntegerBinaryOperation::ShiftLeft | IntegerBinaryOperation::ShiftRight
    ) && (right < 0 || right >= i128::from(ty.bit_width()))
    {
        return Err(VmErrorKind::InvalidShiftCount { ty, count: right });
    }
    // 広い型で計算し、最小値 % -1と左シフトをホスト側の桁あふれから守ります。
    let value = match op {
        IntegerBinaryOperation::Remainder => {
            if right == 0 {
                return Err(VmErrorKind::RemainderByZero);
            }
            left % right
        }
        IntegerBinaryOperation::BitAnd => left & right,
        IntegerBinaryOperation::BitOr => left | right,
        IntegerBinaryOperation::BitXor => left ^ right,
        IntegerBinaryOperation::ShiftLeft => left << right,
        IntegerBinaryOperation::ShiftRight => left >> right,
    };
    if value < ty.minimum() || value > ty.maximum() {
        return Err(VmErrorKind::IntegerOverflow {
            operation: IntegerOperation::ShiftLeft,
            ty: Type::Integer(ty),
        });
    }
    stack.push(Value::Integer(value, ty));
    Ok(())
}

#[derive(Clone, Copy)]
enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

fn binary(ty: Type, stack: &mut Vec<Value>, operation: BinaryOperation) -> VmResult<()> {
    match ty {
        Type::Bool => {
            return Err(VmErrorKind::TypeMismatch {
                expected: Type::Integer(IntegerType::I64),
                actual: Type::Bool,
            });
        }

        Type::Integer(integer) => {
            let right = pop_integer(stack, integer)? as i128;
            let left = pop_integer(stack, integer)? as i128;
            let value = match operation {
                BinaryOperation::Add => left + right,
                BinaryOperation::Subtract => left - right,
                BinaryOperation::Multiply => {
                    left.checked_mul(right)
                        .ok_or_else(|| VmErrorKind::IntegerOverflow {
                            operation: IntegerOperation::Multiply,
                            ty: ty.clone(),
                        })?
                }
                BinaryOperation::Divide => {
                    if right == 0 {
                        return Err(VmErrorKind::DivisionByZero);
                    }
                    left / right
                }
            };
            if value < integer.minimum() || value > integer.maximum() {
                let operation = match operation {
                    BinaryOperation::Add => IntegerOperation::Add,
                    BinaryOperation::Subtract => IntegerOperation::Subtract,
                    BinaryOperation::Multiply => IntegerOperation::Multiply,
                    BinaryOperation::Divide => return Err(VmErrorKind::DivisionOverflow),
                };
                return Err(VmErrorKind::IntegerOverflow { operation, ty });
            }
            stack.push(Value::Integer(value, integer));
        }

        Type::F32 => {
            let right = pop_f32(stack)?;

            let left = pop_f32(stack)?;

            let value = match operation {
                BinaryOperation::Add => left + right,

                BinaryOperation::Subtract => left - right,

                BinaryOperation::Multiply => left * right,

                BinaryOperation::Divide => left / right,
            };

            stack.push(Value::F32(value));
        }

        Type::F64 => {
            let right = pop_f64(stack)?;

            let left = pop_f64(stack)?;

            let value = match operation {
                BinaryOperation::Add => left + right,

                BinaryOperation::Subtract => left - right,

                BinaryOperation::Multiply => left * right,

                BinaryOperation::Divide => left / right,
            };

            stack.push(Value::F64(value));
        }

        Type::String | Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. } => {
            return Err(VmErrorKind::TypeMismatch {
                expected: Type::Integer(IntegerType::I64),
                actual: ty,
            });
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

fn compare(ty: Type, stack: &mut Vec<Value>, comparison: Comparison) -> VmResult<()> {
    let result = match ty {
        Type::String => {
            let right = pop_string(stack)?;
            let left = pop_string(stack)?;
            match comparison {
                Comparison::Equal => left == right,
                Comparison::NotEqual => left != right,
                _ => return Err(VmErrorKind::InvalidComparisonType { ty }),
            }
        }
        Type::Bool => {
            let right = pop_bool(stack)?;
            let left = pop_bool(stack)?;

            match comparison {
                Comparison::Equal => left == right,
                Comparison::NotEqual => left != right,
                Comparison::Less
                | Comparison::LessEqual
                | Comparison::Greater
                | Comparison::GreaterEqual => {
                    return Err(VmErrorKind::InvalidComparisonType { ty });
                }
            }
        }
        Type::Integer(integer) => {
            let right = pop_integer(stack, integer)?;
            let left = pop_integer(stack, integer)?;
            compare_values(left, right, comparison)
        }
        Type::F32 => {
            let right = pop_f32(stack)?;
            let left = pop_f32(stack)?;
            compare_values(left, right, comparison)
        }
        Type::F64 => {
            let right = pop_f64(stack)?;
            let left = pop_f64(stack)?;
            compare_values(left, right, comparison)
        }
        Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. } => {
            return Err(VmErrorKind::InvalidComparisonType { ty });
        }
    };

    stack.push(Value::Bool(result));
    Ok(())
}

fn compare_values<T: PartialEq + PartialOrd>(left: T, right: T, comparison: Comparison) -> bool {
    match comparison {
        Comparison::Equal => left == right,
        Comparison::NotEqual => left != right,
        Comparison::Less => left < right,
        Comparison::LessEqual => left <= right,
        Comparison::Greater => left > right,
        Comparison::GreaterEqual => left >= right,
    }
}

fn negate(ty: Type, stack: &mut Vec<Value>) -> VmResult<()> {
    match ty {
        Type::Bool => {
            return Err(VmErrorKind::TypeMismatch {
                expected: Type::Integer(IntegerType::I64),
                actual: Type::Bool,
            });
        }

        Type::Integer(integer) => {
            if !integer.is_signed() {
                return Err(VmErrorKind::InvalidNegationType { ty: integer });
            }
            let value = pop_integer(stack, integer)?;
            let value = value
                .checked_neg()
                .filter(|value| integer.contains(*value))
                .ok_or(VmErrorKind::IntegerOverflow {
                    operation: IntegerOperation::Negate,
                    ty,
                })?;
            stack.push(Value::Integer(value, integer));
        }

        Type::F32 => {
            let value = pop_f32(stack)?;

            stack.push(Value::F32(-value));
        }

        Type::F64 => {
            let value = pop_f64(stack)?;

            stack.push(Value::F64(-value));
        }

        Type::String | Type::Named(_) | Type::Array { .. } | Type::DynamicArray { .. } => {
            return Err(VmErrorKind::TypeMismatch {
                expected: Type::Integer(IntegerType::I64),
                actual: ty,
            });
        }
    }

    Ok(())
}

fn pop_value(stack: &mut Vec<Value>) -> VmResult<Value> {
    stack.pop().ok_or(VmErrorKind::StackUnderflow)
}

fn pop_i64(stack: &mut Vec<Value>) -> VmResult<i64> {
    pop_integer(stack, IntegerType::I64).map(|value| value as i64)
}

fn pop_integer(stack: &mut Vec<Value>, ty: IntegerType) -> VmResult<i128> {
    match pop_value(stack)? {
        Value::Integer(value, actual) if actual == ty => Ok(value),

        other => Err(VmErrorKind::TypeMismatch {
            expected: Type::Integer(ty),
            actual: other.ty(),
        }),
    }
}

fn array_error(error: ArrayError) -> VmErrorKind {
    match error {
        ArrayError::AllocationSizeOverflow => VmErrorKind::AllocationSizeOverflow,
        ArrayError::AllocationLimitExceeded => VmErrorKind::AllocationLimitExceeded,
        ArrayError::AllocationFailed => VmErrorKind::AllocationFailed,
        ArrayError::InvalidOwnership | ArrayError::IndexOutOfBounds => {
            VmErrorKind::InvalidArrayOwnership
        }
    }
}
fn pop_dynamic_array(stack: &mut Vec<Value>) -> VmResult<(Type, ArrayValue<Value>)> {
    match pop_value(stack)? {
        Value::DynamicArray { element, storage } => Ok((element, storage)),
        _ => Err(VmErrorKind::InvalidArrayOwnership),
    }
}
fn array_index(index: i64, length: usize) -> VmResult<usize> {
    usize::try_from(index)
        .ok()
        .filter(|i| *i < length)
        .ok_or(VmErrorKind::ArrayIndexOutOfBounds { index, length })
}
fn array_child(current: &Value, access: &ArrayAccess, index: i64) -> VmResult<(usize, Value)> {
    let expected = match access.length {
        Some(length) => Type::Array {
            element: Box::new(access.element.clone()),
            length,
        },
        None => Type::DynamicArray {
            element: Box::new(access.element.clone()),
        },
    };
    if current.ty() != expected {
        return Err(VmErrorKind::TypeMismatch {
            expected,
            actual: current.ty(),
        });
    }
    match current {
        Value::Array { values, .. } => {
            let i = array_index(index, values.len())?;
            Ok((i, values[i].clone()))
        }
        Value::DynamicArray { storage, .. } => {
            let i = array_index(index, storage.len())?;
            Ok((i, storage.get(i).map_err(array_error)?))
        }
        _ => unreachable!(),
    }
}
fn assign_array_path(
    current: &mut Value,
    path: &[ArrayAccess],
    indices: &[i64],
    replacement: Value,
) -> VmResult<()> {
    let Some((access, rest)) = path.split_first() else {
        return Err(VmErrorKind::InvalidArrayOwnership);
    };
    let Some((&index, remaining)) = indices.split_first() else {
        return Err(VmErrorKind::StackUnderflow);
    };
    let (i, mut child) = array_child(current, access, index)?;
    if rest.is_empty() {
        if replacement.ty() != access.element {
            return Err(VmErrorKind::TypeMismatch {
                expected: access.element.clone(),
                actual: replacement.ty(),
            });
        }
        child = replacement;
    } else {
        assign_array_path(&mut child, rest, remaining, replacement)?;
    }
    match current {
        Value::Array { values, .. } => {
            values[i] = child;
            Ok(())
        }
        Value::DynamicArray { storage, .. } => storage.set(i, child).map_err(array_error),
        _ => unreachable!(),
    }
}
fn check_array_path(current: &Value, path: &[ArrayAccess], indices: &[Value]) -> VmResult<()> {
    let mut current = current.clone();
    for (access, index) in path.iter().zip(indices) {
        let Value::Integer(index, IntegerType::I64) = index else {
            return Err(VmErrorKind::TypeMismatch {
                expected: Type::Integer(IntegerType::I64),
                actual: index.ty(),
            });
        };
        current = array_child(&current, access, *index as i64)?.1;
    }
    Ok(())
}

fn pop_bool(stack: &mut Vec<Value>) -> VmResult<bool> {
    match pop_value(stack)? {
        Value::Bool(value) => Ok(value),

        other => Err(VmErrorKind::TypeMismatch {
            expected: Type::Bool,
            actual: other.ty(),
        }),
    }
}

fn pop_string(stack: &mut Vec<Value>) -> VmResult<StringValue> {
    match pop_value(stack)? {
        Value::String(value) => Ok(value),
        other => Err(VmErrorKind::TypeMismatch {
            expected: Type::String,
            actual: other.ty(),
        }),
    }
}

fn pop_f32(stack: &mut Vec<Value>) -> VmResult<f32> {
    match pop_value(stack)? {
        Value::F32(value) => Ok(value),

        other => Err(VmErrorKind::TypeMismatch {
            expected: Type::F32,
            actual: other.ty(),
        }),
    }
}

fn pop_f64(stack: &mut Vec<Value>) -> VmResult<f64> {
    match pop_value(stack)? {
        Value::F64(value) => Ok(value),

        other => Err(VmErrorKind::TypeMismatch {
            expected: Type::F64,
            actual: other.ty(),
        }),
    }
}

fn format_value(value: Value, expected: Type) -> VmResult<String> {
    match (value, expected) {
        (Value::Bool(value), Type::Bool) => Ok(value.to_string()),
        (Value::String(value), Type::String) => Ok(value.text()),

        (Value::Integer(value, actual), Type::Integer(expected)) if actual == expected => {
            Ok(value.to_string())
        }

        (Value::F32(value), Type::F32) => Ok(float_output::f32(value)),

        (Value::F64(value), Type::F64) => Ok(float_output::f64(value)),

        (value, expected) => Err(VmErrorKind::TypeMismatch {
            expected,
            actual: value.ty(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, execute_function};
    use crate::types::IntegerType;
    use crate::{
        bytecode::{self, BytecodeProgram, Instruction, InstructionKind, Slot, Type},
        compile_to_bytecode, compile_to_ir,
    };

    use super::{IntegerOperation, VmErrorKind, run};

    #[test]
    fn executes_floating_point_program() {
        let program =
            compile_to_ir("a: f32 = 0.1 + 0.2; b: f64 = 0.1 + 0.2; print(a); print(b);").unwrap();

        let bytecode = bytecode::lower(&program).unwrap();

        let output = run(&bytecode).unwrap();

        assert_eq!(output, "0.300000012\n0.30000000000000004\n");
    }

    #[test]
    fn reports_integer_division_by_zero_at_instruction() {
        let program = compile_to_bytecode("print(1 / 0);").unwrap();

        let error = run(&program).unwrap_err();

        assert_eq!(error.kind(), VmErrorKind::DivisionByZero);
        assert_eq!(error.instruction_index(), 2);
    }

    #[test]
    fn distinguishes_integer_division_overflow() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: Vec::new(),
            instructions: vec![
                Instruction::synthetic(InstructionKind::PushInteger(
                    i64::MIN as i128,
                    IntegerType::I64,
                )),
                Instruction::synthetic(InstructionKind::PushInteger(-1, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Divide(Type::Integer(IntegerType::I64))),
                Instruction::synthetic(InstructionKind::Halt),
            ],
        };

        let error = run(&program).unwrap_err();

        assert_eq!(error.kind(), VmErrorKind::DivisionOverflow);
        assert_eq!(error.instruction_index(), 2);
    }

    #[test]
    fn reports_integer_addition_overflow_at_instruction() {
        let program = compile_to_bytecode("print(9223372036854775807 + 1);").unwrap();

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::IntegerOverflow {
                operation: IntegerOperation::Add,
                ty: Type::Integer(IntegerType::I64),
            }
        );
        assert_eq!(error.instruction_index(), 2);
    }

    #[test]
    fn reports_integer_subtraction_overflow_at_instruction() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: Vec::new(),
            instructions: vec![
                Instruction::synthetic(InstructionKind::PushInteger(
                    i64::MIN as i128,
                    IntegerType::I64,
                )),
                Instruction::synthetic(InstructionKind::PushInteger(1, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Subtract(Type::Integer(IntegerType::I64))),
                Instruction::synthetic(InstructionKind::Halt),
            ],
        };

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::IntegerOverflow {
                operation: IntegerOperation::Subtract,
                ty: Type::Integer(IntegerType::I64),
            }
        );
        assert_eq!(error.instruction_index(), 2);
    }

    #[test]
    fn reports_integer_multiplication_overflow_at_instruction() {
        let program = compile_to_bytecode("print(9223372036854775807 * 2);").unwrap();

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::IntegerOverflow {
                operation: IntegerOperation::Multiply,
                ty: Type::Integer(IntegerType::I64),
            }
        );
        assert_eq!(error.instruction_index(), 2);
    }

    #[test]
    fn reports_integer_negation_overflow_at_instruction() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: Vec::new(),
            instructions: vec![
                Instruction::synthetic(InstructionKind::PushInteger(
                    i64::MIN as i128,
                    IntegerType::I64,
                )),
                Instruction::synthetic(InstructionKind::Negate(Type::Integer(IntegerType::I64))),
                Instruction::synthetic(InstructionKind::Halt),
            ],
        };

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::IntegerOverflow {
                operation: IntegerOperation::Negate,
                ty: Type::Integer(IntegerType::I64),
            }
        );
        assert_eq!(error.instruction_index(), 1);
    }

    #[test]
    fn reports_overflow_when_negating_the_minimum_i64_literal() {
        let program = compile_to_bytecode("print(--9223372036854775808);").unwrap();

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::IntegerOverflow {
                operation: IntegerOperation::Negate,
                ty: Type::Integer(IntegerType::I64),
            }
        );
        assert_eq!(error.instruction_index(), 1);
    }

    #[test]
    fn rejects_bytecode_assignment_to_immutable_slot() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: vec![Slot {
                name: "value".into(),
                ty: Type::Integer(IntegerType::I64),
                mutable: false,
            }],
            instructions: vec![
                Instruction::synthetic(InstructionKind::PushInteger(1, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Store(0)),
                Instruction::synthetic(InstructionKind::PushInteger(2, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Assign(0)),
                Instruction::synthetic(InstructionKind::Halt),
            ],
        };

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::AssignmentToImmutableSlot { slot: 0 }
        );
        assert_eq!(error.instruction_index(), 3);
    }

    #[test]
    fn rejects_distinct_initializers_for_the_same_slot() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: vec![Slot {
                name: "value".into(),
                ty: Type::Integer(IntegerType::I64),
                mutable: false,
            }],
            instructions: vec![
                Instruction::synthetic(InstructionKind::PushInteger(1, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Store(0)),
                Instruction::synthetic(InstructionKind::PushInteger(2, IntegerType::I64)),
                Instruction::synthetic(InstructionKind::Store(0)),
                Instruction::synthetic(InstructionKind::Halt),
            ],
        };

        let error = run(&program).unwrap_err();

        assert_eq!(
            error.kind(),
            VmErrorKind::SlotAlreadyInitialized { slot: 0 }
        );
        assert_eq!(error.instruction_index(), 3);
    }

    #[test]
    fn reports_missing_instruction_without_panicking() {
        let program = BytecodeProgram {
            array_heap_limit: crate::ir::DEFAULT_ARRAY_HEAP_LIMIT,
            string_heap_limit: crate::ir::DEFAULT_STRING_HEAP_LIMIT,
            type_definitions: Vec::new(),
            functions: Vec::new(),
            slots: Vec::new(),
            instructions: Vec::new(),
        };

        let error = run(&program).unwrap_err();

        assert_eq!(error.kind(), VmErrorKind::InstructionOutOfBounds);
        assert_eq!(error.instruction_index(), 0);
    }

    #[test]
    fn executes_else_and_skips_if_without_else() {
        let program = compile_to_bytecode(
            "if false { print(1); } else { print(2); }
             if false { print(3); }
             print(4);",
        )
        .unwrap();

        assert_eq!(run(&program).unwrap(), "2\n4\n");
    }

    #[test]
    fn executes_while_and_rechecks_its_condition() {
        let program = compile_to_bytecode(
            "mut count: i64 = 0;
             while count < 3 {
                 print(count);
                 count = count + 1;
             }
             print(count);",
        )
        .unwrap();

        assert_eq!(run(&program).unwrap(), "0\n1\n2\n3\n");
    }

    #[test]
    fn executes_break_and_continue() {
        let program = compile_to_bytecode(
            "mut value: i64 = 0;
             mut sum: i64 = 0;
             while value < 10 {
                 value = value + 1;
                 if value < 3 { continue; }
                 if value > 5 { break; }
                 sum = sum + value;
             }
             print(sum);
             print(value);",
        )
        .unwrap();

        assert_eq!(run(&program).unwrap(), "12\n6\n");
    }

    #[test]
    fn break_targets_only_the_innermost_loop() {
        let program = compile_to_bytecode(
            "mut outer: i64 = 0;
             mut hits: i64 = 0;
             while outer < 2 {
                 mut inner: i64 = 0;
                 while inner < 3 {
                     inner = inner + 1;
                     if inner == 2 { break; }
                     hits = hits + 1;
                 }
                 outer = outer + 1;
             }
             print(hits);",
        )
        .unwrap();

        assert_eq!(run(&program).unwrap(), "2\n");
    }

    #[test]
    fn executes_function_directly() {
        let program = crate::compile_to_bytecode(
            r#"
            fn add(lhs: f64, rhs: f64) -> f64 {
                return lhs + rhs;
            }
            "#,
        )
        .unwrap();

        let function_id = program
            .functions
            .iter()
            .position(|function| function.name == "add")
            .unwrap();

        let (value, output) = execute_function(
            &program,
            function_id,
            vec![Value::F64(1.25), Value::F64(2.5)],
        )
        .unwrap();

        match value {
            Some(Value::F64(value)) => assert_eq!(value, 3.75),
            other => panic!("expected f64 return value, found {other:?}"),
        }

        assert_eq!(output, "");
    }

    #[test]
    fn forced_array_allocation_failure_has_source_origin_and_prior_output() {
        let source = r#"print("before");a:[i64]=array_copy([1]);"#;
        let p = crate::compile_to_bytecode(source).unwrap();
        let mut heap = super::StringHeap::new(p.string_heap_limit);
        let mut arrays = super::ArrayHeap::new(p.array_heap_limit);
        arrays.fail_next_allocation();
        let mut output = crate::runtime::output::Capture::default();
        let e = super::execute_frame(
            &p,
            super::Frame::Entry,
            vec![],
            &mut output,
            &mut heap,
            &mut arrays,
        )
        .unwrap_err();
        assert_eq!(e.kind(), VmErrorKind::AllocationFailed);
        assert_eq!(output.0, "before\n");
        let instruction =
            &p.functions[e.function_id().unwrap()].instructions[e.instruction_index()];
        let crate::bytecode::InstructionOrigin::Source { span, .. } = instruction.origin else {
            panic!()
        };
        assert_eq!(&source[span.start()..span.end()], "array_copy([1])");
        arrays.assert_empty();
    }
}
