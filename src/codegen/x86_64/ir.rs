#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// 不変な長さ付き文字列データへの内部参照です。
    String,
    /// 動的配列の管理領域への内部参照です。
    DynamicArray,
    Bool,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub string_heap_limit: Option<u64>,
    pub array_heap_limit: Option<u64>,
    pub uses_write: bool,
    pub origins: Vec<Origin>,
    pub mir_origins: Vec<Option<MirOrigin>>,
    pub target: super::Target,
    pub uses_strings: bool,
    pub strings: Vec<String>,
    pub functions: Vec<Function>,
    pub frame_size: usize,
    pub float_constants: Vec<FloatConstant>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub origins: Vec<Origin>,
    pub mir_origins: Vec<Option<MirOrigin>>,
    pub id: usize,
    pub name: String,
    pub frame_size: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirOrigin {
    pub block: crate::mir::BlockId,
    pub instruction: Option<crate::mir::InstructionId>,
    pub origin: crate::mir::Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Source {
        node_id: crate::ir::NodeId,
        span: crate::source::Span,
    },
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentLocation {
    Register(usize),
    /// 呼び出し直前のRSPからのバイト位置。受け取り側ではRBP + 16を基準にします。
    Stack(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argument {
    Scalar {
        ty: Type,
        offset: isize,
    },
    /// 呼び出し先が自身のstackへコピーする値のaddressです。
    Aggregate {
        offset: isize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloatConstant {
    F32 { id: usize, bits: u32 },
    F64 { id: usize, bits: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// 保存幅が0の値の操作もMIR対応に残します。機械バイトは生成しません。
    ObserveOnly,
    /// 到達不能blockの末尾。言語の異常停止とは区別します。
    Unreachable,
    ArrayAllocate {
        width: u64,
        stride: usize,
        label: usize,
    },
    ArrayRangeCheck {
        length_offset: isize,
        start_offset: isize,
        label: usize,
    },
    DynamicArrayAddress {
        base_offset: isize,
        base_is_pointer: bool,
        destination_offset: isize,
        label: usize,
    },
    ArrayInitAddress {
        owner_offset: isize,
        destination_offset: isize,
    },
    ArrayInitialized {
        owner_offset: isize,
    },
    ArrayLength,
    ArrayRetain,
    ArrayReleaseOwner,
    ArrayFree,
    LoadFromPointer {
        ty: Type,
        pointer_offset: isize,
    },
    CopyFromPointer {
        pointer_offset: isize,
        destination_offset: isize,
        slots: usize,
    },
    StringConcat {
        left_offset: isize,
        label: usize,
    },
    StringManage {
        retain: bool,
    },
    Flush,
    Write {
        kind: &'static str,
    },
    CallPrintSysV(Type),
    CallPrintU64,
    CompareU64(CompareOp),
    LoadStringLength,
    LoadStringConstant(usize),
    CompareString {
        left_offset: isize,
        equal: bool,
    },
    PrintString,
    ConvertNumeric {
        conversion: crate::codegen::NumericConversion,
        label: usize,
    },
    BitNot {
        mask: i64,
    },
    IntegerBinary {
        op: crate::codegen::IntegerBinaryOp,
        ty: crate::types::IntegerType,
        label: usize,
    },
    CheckIntegerRange {
        ty: crate::types::IntegerType,
        label: usize,
        failure: crate::runtime::FailureCode,
    },
    Label {
        id: usize,
        name: &'static str,
    },
    JumpIfZero(usize),
    Jump(usize),

    MovI64ImmediateToRax(i64),

    LoadI64FromStack(isize),
    StoreI64ToStack(isize),

    LoadF32FromStack(isize),
    StoreF32ToStack(isize),

    LoadF64FromStack(isize),
    StoreF64ToStack(isize),

    CheckedArrayLoad {
        ty: Type,
        base_offset: isize,
        length: usize,
        label: usize,
    },
    CheckedArrayCopy {
        base_offset: isize,
        length: usize,
        element_slots: usize,
        destination_offset: isize,
        label: usize,
    },
    CheckedArrayAddress {
        base_offset: isize,
        base_is_pointer: bool,
        length: usize,
        element_slots: usize,
        destination_offset: isize,
        label: usize,
    },
    StoreI64ToPointer(isize),
    StoreF32ToPointer(isize),
    StoreF64ToPointer(isize),
    CopyToPointer {
        source_offset: isize,
        slots: usize,
        pointer_offset: isize,
    },

    StoreParameter {
        location: ArgumentLocation,
        ty: Type,
        offset: isize,
    },
    StoreAggregateParameter {
        location: ArgumentLocation,
        slots: usize,
        destination_offset: isize,
    },
    /// 内部呼び出し規約で`RAX`に渡された集約戻り値の保存先を退避します。
    StoreAggregateReturnPointer {
        offset: isize,
    },
    CopyToAggregateReturn {
        source_offset: isize,
        slots: usize,
        pointer_offset: isize,
    },
    Call {
        function_id: usize,
        arguments: Vec<Argument>,
        aggregate_result_offset: Option<isize>,
    },
    Return,

    LoadF32Constant(usize),
    LoadF64Constant(usize),

    NegI64,
    TrapIfOverflow(usize),
    NotBool,
    NegF32,
    NegF64,

    MoveRaxToRcx,
    LoadI64ScratchToRax(isize),
    I64Binary(BinaryOp),
    CompareI64(CompareOp),
    SignExtendRax,
    TrapIfInvalidI64Division(usize),
    DivideRaxByRcx,

    CopyXmm0ToXmm1F32,
    CopyXmm0ToXmm1F64,
    LoadF32ScratchToXmm0(isize),
    LoadF64ScratchToXmm0(isize),
    F32Binary(BinaryOp),
    F64Binary(BinaryOp),
    CompareF32(CompareOp),
    CompareF64(CompareOp),

    MoveRaxToRdx,
    LoadFormatI64ToRcx,

    ConvertF32ToF64Argument,
    MoveXmm1ToRdx,
    LoadFormatF32ToRcx,

    CopyXmm0ToXmm1F64Scalar,
    LoadFormatF64ToRcx,

    CallPrintf,
    CallPrintBool,
}
