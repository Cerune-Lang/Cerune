#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    DynamicArray { element: Box<Type> },
    String,
    Bool,
    I64,
    Float,
    Double,
    Named(usize),
    Array { element: Box<Type>, length: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub array_heap_limit: Option<u64>,
    pub string_heap_limit: Option<u64>,
    pub target: Option<super::Target>,
    pub uses_strings: bool,
    pub strings: Vec<String>,
    pub type_definitions: Vec<TypeDefinition>,
    pub functions: Vec<Function>,
    pub explicit_main: Option<usize>,
    pub slots: Vec<Slot>,
    pub instructions: Vec<LocatedInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: usize,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub slots: Vec<Slot>,
    pub instructions: Vec<LocatedInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub slot: SlotId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub id: usize,
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temp(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    String { id: usize, length: usize },
    Boolean(bool),
    Integer(i64),
    Float32(u32),
    Float64(u64),
    Temp(Temp),
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    CheckedI64Add,
    CheckedI64Sub,
    CheckedI64Mul,
    CheckedI64Div,
    FAdd,
    FSub,
    FMul,
    FDiv,
    Xor,
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
pub enum Instruction {
    ArrayAllocate {
        dest: Temp,
        element: Type,
        length: Operand,
        element_width: u64,
    },
    ArrayInitialize {
        scratch: Temp,
        element: Type,
        array: Operand,
        value: Operand,
    },
    ArrayManage {
        value: Operand,
        retain: bool,
    },
    ArrayReleaseOwner {
        dest: Temp,
        value: Operand,
    },
    ArrayRangeCheck {
        length: Operand,
        start: Operand,
        end: Operand,
    },
    StringConcat {
        dest: Temp,
        left: Operand,
        right: Operand,
    },
    StringManage {
        value: Operand,
        retain: bool,
    },
    Flush,
    Output {
        kind: &'static str,
        value: Operand,
        newline: bool,
    },
    ConvertNumeric {
        conversion: crate::codegen::NumericConversion,
        dest: Temp,
        value: Operand,
    },
    IntegerBinary {
        op: crate::codegen::IntegerBinaryOp,
        ty: crate::types::IntegerType,
        dest: Temp,
        left: Operand,
        right: Operand,
    },
    CheckIntegerRange {
        dest: Temp,
        value: Operand,
        ty: crate::types::IntegerType,
        failure: crate::runtime::FailureCode,
    },
    Label {
        id: Label,
        name: &'static str,
    },
    Branch {
        condition: Operand,
        then_label: Label,
        else_label: Label,
    },
    Jump {
        label: Label,
    },
    Store {
        ty: Type,
        value: Operand,
        slot: SlotId,
    },
    Load {
        dest: Temp,
        ty: Type,
        slot: SlotId,
    },
    InsertValue {
        dest: Temp,
        ty: Type,
        aggregate: Operand,
        value_ty: Type,
        value: Operand,
        field: usize,
    },
    ExtractValue {
        dest: Temp,
        ty: Type,
        aggregate: Operand,
        field: usize,
    },
    ArrayGet {
        dest: Temp,
        element: Type,
        length: Option<usize>,
        array: Operand,
        index: Operand,
    },
    ArraySet {
        dest: Temp,
        element: Type,
        length: Option<usize>,
        array: Operand,
        index: Operand,
        value: Operand,
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
    Binary {
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
    FNeg {
        dest: Temp,
        ty: Type,
        value: Operand,
    },
}

/// 命令の由来は意味を持つ命令本体から分離し、欠落を暗黙の生成扱いにしません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Source {
        node_id: crate::ir::NodeId,
        span: crate::source::Span,
    },
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedInstruction {
    pub instruction: Instruction,
    pub origin: Origin,
}
