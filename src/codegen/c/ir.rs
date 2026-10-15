#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    DynamicArray { element: Box<Type> },
    Bool,
    String,
    I64,
    U64,
    Float,
    Double,
    Named(usize),
    Array { element: Box<Type>, length: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub array_heap_limit: Option<u64>,
    pub string_heap_limit: Option<u64>,
    pub uses_write: bool,
    pub uses_strings: bool,
    pub temporaries: Vec<Type>,
    pub array_types: Vec<Type>,
    pub array_assignment_types: Vec<Type>,
    pub type_definitions: Vec<TypeDefinition>,
    pub functions: Vec<Function>,
    pub explicit_main: Option<usize>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub temporaries: Vec<Type>,
    pub id: usize,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub id: usize,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDefinition {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    ArrayInitialize {
        array: Expr,
        value: Expr,
    },
    ArrayManage {
        value: Expr,
        retain: bool,
    },
    ArrayRangeCheck {
        origin: Origin,
        length: Expr,
        start: Expr,
        end: Expr,
    },
    StringManage {
        value: Expr,
        retain: bool,
    },
    Flush {
        origin: Origin,
    },
    Write {
        origin: Origin,
        kind: &'static str,
        value: Expr,
    },
    Binding {
        name: String,
        ty: Type,
        value: Expr,
    },
    Assignment {
        target: AssignmentTarget,
        value: Expr,
    },
    Print {
        origin: Origin,
        format: PrintFormat,
        value: Expr,
    },
    Call {
        evaluation: Vec<(usize, Expr)>,
        function_id: usize,
        function_name: String,
        arguments: Vec<Expr>,
    },
    Return(Option<Expr>),
    If {
        condition: Expr,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    For {
        initializer: Box<Statement>,
        condition: Expr,
        update: Box<Statement>,
        body: Vec<Statement>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentTarget {
    pub name: String,
    pub projections: Vec<ArrayProjection>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayProjection {
    pub origin: Origin,
    pub index: Expr,
    pub element: Type,
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintFormat {
    U64,
    Bool,
    String,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub origin: Origin,
    pub ty: Type,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub node_id: crate::ir::NodeId,
    pub span: crate::source::Span,
}

impl From<&crate::ir::Expr> for Origin {
    fn from(expr: &crate::ir::Expr) -> Self {
        Self {
            node_id: expr.id,
            span: expr.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    ArrayAllocate {
        length: Box<Expr>,
        element_width: u64,
    },
    ArrayReleaseOwner {
        value: Box<Expr>,
    },
    ReadLine {
        max_bytes: Box<Expr>,
    },
    StringConcat {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    ArrayLength {
        value: Box<Expr>,
        length: Option<usize>,
    },
    StringByteLength {
        value: Box<Expr>,
    },
    /// 複数の作用や失敗を、ソース順に一度ずつ評価します。
    Sequence {
        bindings: Vec<(usize, Expr)>,
        value: Box<Expr>,
    },
    Temporary(usize),
    String(String),
    ConvertNumeric {
        conversion: crate::codegen::NumericConversion,
        value: Box<Expr>,
    },
    IntegerBinary {
        scratch: usize,
        op: crate::codegen::IntegerBinaryOp,
        ty: crate::types::IntegerType,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    CheckIntegerRange {
        value: Box<Expr>,
        ty: crate::types::IntegerType,
        code: crate::runtime::FailureCode,
    },
    Boolean(bool),
    Integer(i128),
    Float {
        text: String,
        suffix_f32: bool,
    },
    Variable(String),
    Construct {
        type_id: usize,
        fields: Vec<FieldValue>,
        base: Option<Box<Expr>>,
        copy_temp: Option<usize>,
    },
    FieldAccess {
        field_name: String,
        base: Box<Expr>,
    },
    Array(Vec<Expr>),
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        function_id: usize,
        function_name: String,
        arguments: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValue {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    CheckedI64Negate,
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    CheckedI64Add,
    CheckedI64Subtract,
    CheckedI64Multiply,
    CheckedI64Divide,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}
