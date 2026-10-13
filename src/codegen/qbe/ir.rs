#[derive(Debug, Clone)]
pub struct Module {
    pub array_heap_limit: Option<u64>,
    pub string_heap_limit: Option<u64>,
    pub uses_write: bool,
    pub target: Option<super::Target>,
    pub uses_strings: bool,
    pub strings: Vec<String>,
    pub functions: Vec<Function>,
    pub explicit_main: Option<usize>,
    pub slots: Vec<Slot>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: usize,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub aggregate_return_size: Option<usize>,
    pub slots: Vec<Slot>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub passing: ParameterPassing,
    pub slot: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum ParameterPassing {
    Scalar(Type),
    /// 呼び出し側の値のaddress。関数開始時に自身のslotへコピーします。
    Aggregate {
        size: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub name: String,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// 長さと所有領域を保持する内部記述子へのポインタです。
    DynamicArray,
    /// 不変な長さ付き静的バイト列への参照です。
    String,
    Bool,
    I64,
    Single,
    Double,
    /// 集約値をコピーする間だけ使うbackend内部のaddressです。
    Pointer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temp(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    String(usize),
    Boolean(bool),
    Integer(i64),
    Float32(String),
    Float64(String),
    Temp(Temp),
    Slot(usize),
    /// 呼び出し側が用意した集約戻り値の保存先です。
    ReturnPointer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    CheckedI64Add,
    CheckedI64Subtract,
    CheckedI64Multiply,
    CheckedI64Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintFormat {
    U64,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    ArrayAllocate {
        dest: Temp,
        length: Operand,
        width: u64,
        stride: usize,
        origin: FailureOrigin,
    },
    ArrayRangeCheck {
        length: Operand,
        start: Operand,
        end: Operand,
        origin: FailureOrigin,
    },
    ArrayAddress {
        dest: Temp,
        value: Operand,
        index: Operand,
        origin: FailureOrigin,
    },
    ArrayInitAddress {
        dest: Temp,
        value: Operand,
    },
    ArrayInitialized {
        value: Operand,
    },
    ArrayLength {
        dest: Temp,
        value: Operand,
    },
    ArrayReleaseOwner {
        dest: Temp,
        value: Operand,
    },
    ArrayManage {
        value: Operand,
        retain: bool,
    },
    StringConcat {
        dest: Temp,
        left: Operand,
        right: Operand,
        origin: FailureOrigin,
    },
    StringManage {
        value: Operand,
        retain: bool,
    },
    Flush {
        origin: FailureOrigin,
    },
    Write {
        kind: &'static str,
        value: Operand,
    },
    PrintString {
        value: Operand,
    },
    ConvertNumeric {
        origin: FailureOrigin,
        conversion: crate::codegen::NumericConversion,
        dest: Temp,
        value: Operand,
    },
    IntegerBinary {
        origin: FailureOrigin,
        op: crate::codegen::IntegerBinaryOp,
        ty: crate::types::IntegerType,
        dest: Temp,
        left: Operand,
        right: Operand,
    },
    CheckIntegerRange {
        origin: FailureOrigin,
        failure: crate::runtime::FailureCode,
        dest: Temp,
        value: Operand,
        ty: crate::types::IntegerType,
    },
    Label {
        id: usize,
        name: &'static str,
    },
    Branch {
        condition: Operand,
        then_label: usize,
        else_label: usize,
    },
    Jump(usize),
    Store {
        address: Operand,
        ty: Type,
        value: Operand,
    },
    Load {
        dest: Temp,
        address: Operand,
        ty: Type,
    },
    Address {
        dest: Temp,
        base: Operand,
        offset: usize,
    },
    Blit {
        source: Operand,
        destination: Operand,
        size: usize,
    },
    Abort {
        origin: FailureOrigin,
    },
    Call {
        dest: Option<Temp>,
        function_id: usize,
        return_type: Option<Type>,
        arguments: Vec<(Type, Operand)>,
    },
    Return {
        value: Option<(Type, Operand)>,
    },
    Negate {
        dest: Temp,
        ty: Type,
        value: Operand,
    },
    CheckedI64Negate {
        origin: FailureOrigin,
        dest: Temp,
        value: Operand,
    },
    Not {
        dest: Temp,
        value: Operand,
    },
    Binary {
        origin: FailureOrigin,
        dest: Temp,
        op: BinaryOp,
        ty: Type,
        left: Operand,
        right: Operand,
    },
    Compare {
        unsigned: bool,
        dest: Temp,
        op: CompareOp,
        operand_ty: Type,
        left: Operand,
        right: Operand,
    },
    ExtendSingleToDouble {
        dest: Temp,
        value: Operand,
    },
    CallPrintf {
        dest: Temp,
        format: PrintFormat,
        arg_ty: Type,
        value: Operand,
    },
    CallPrintBool {
        offset: Temp,
        scaled_offset: Temp,
        address: Temp,
        text: Temp,
        result: Temp,
        value: Operand,
    },
}

/// 実行中に変更する現在位置ではなく、検査箇所へ埋め込む不変の由来です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureOrigin {
    pub node: crate::ir::NodeId,
    pub span: crate::source::Span,
}

impl From<&crate::ir::Expr> for FailureOrigin {
    fn from(expr: &crate::ir::Expr) -> Self {
        Self {
            node: expr.id,
            span: expr.span,
        }
    }
}
