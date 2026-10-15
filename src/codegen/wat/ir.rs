#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// 不変な長さ付き静的バイト列への参照です。
    String,
    /// 非公開linear memory上の動的配列ownerへの参照です。
    DynamicArray,
    Bool,
    I64,
    F32,
    F64,
    /// 集約値をコピーする間だけ使うlinear memory上のaddressです。
    Pointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub string_heap_start: usize,
    pub string_heap_limit: Option<u64>,
    pub array_heap_limit: Option<u64>,
    pub uses_input: bool,
    pub uses_write: bool,
    pub uses_flush: bool,
    pub uses_strings: bool,
    pub strings: Vec<(usize, String)>,
    pub memory_pages: u32,
    pub functions: Vec<Function>,
    pub explicit_main: Option<usize>,
    pub locals: Vec<Local>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: usize,
    pub name: String,
    pub parameters: Vec<Local>,
    pub return_type: Option<Type>,
    pub locals: Vec<Local>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    While,
    For,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    ArrayAllocate {
        width: u64,
        stride: usize,
    },
    ArrayRangeCheck,
    ArrayAddress,
    ArrayInitAddress,
    ArrayInitialized,
    ArrayLength,
    ArrayRetain,
    ArrayReleaseOwner,
    ArrayFree,
    ReadLine,
    StringConcat,
    StringManage {
        retain: bool,
    },
    Flush {
        origin: Origin,
    },
    Output {
        kind: &'static str,
        newline: bool,
        origin: Origin,
    },
    /// 検査の呼び出し先をソース位置ごとに静的に生成します。
    Located {
        origin: Origin,
        instruction: Box<Instruction>,
    },
    Failure(crate::runtime::RuntimeFailure),
    I64LtU,
    I64LeU,
    I64GtU,
    I64GeU,
    StringEqual,
    StringNotEqual,
    ConvertNumeric {
        conversion: crate::codegen::NumericConversion,
    },
    IntegerBinary {
        op: crate::codegen::IntegerBinaryOp,
        ty: crate::types::IntegerType,
    },
    CheckIntegerRange {
        ty: crate::types::IntegerType,
        failure: crate::runtime::FailureCode,
    },
    I32Const(i32),
    I64Const(i64),
    F32Const(String),
    F64Const(String),

    LocalGet(String),
    LocalSet(String),

    I32Load {
        offset: u32,
    },
    I64Load {
        offset: u32,
    },
    F32Load {
        offset: u32,
    },
    F64Load {
        offset: u32,
    },
    I32Store {
        offset: u32,
    },
    I64Store {
        offset: u32,
    },
    F32Store {
        offset: u32,
    },
    F64Store {
        offset: u32,
    },

    I32WrapI64,
    I32Add,
    I32Mul,
    Unreachable,

    If {
        then_instructions: Vec<Instruction>,
        else_instructions: Vec<Instruction>,
    },
    /// 選んだ分岐からboolを一つスタックへ残します。
    IfBool {
        then_instructions: Vec<Instruction>,
        else_instructions: Vec<Instruction>,
    },
    Loop {
        kind: LoopKind,
        id: usize,
        condition_instructions: Vec<Instruction>,
        body_instructions: Vec<Instruction>,
        update_instructions: Vec<Instruction>,
    },
    Break {
        kind: LoopKind,
        id: usize,
    },
    Continue {
        kind: LoopKind,
        id: usize,
    },
    Call {
        function_id: usize,
    },
    Return,

    CheckedI64Add,
    CheckedI64Sub,
    CheckedI64Mul,
    CheckedI64DivS,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LeS,
    I64GtS,
    I64GeS,

    I32Eq,
    I32Ne,
    I32Eqz,

    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Neg,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Le,
    F32Gt,
    F32Ge,

    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Neg,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Le,
    F64Gt,
    F64Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub node_id: crate::ir::NodeId,
    pub span: crate::source::Span,
}

impl Instruction {
    pub(super) fn at(self, expr: &crate::ir::Expr) -> Self {
        Self::Located {
            origin: Origin {
                node_id: expr.id,
                span: expr.span,
            },
            instruction: Box::new(self),
        }
    }
}
