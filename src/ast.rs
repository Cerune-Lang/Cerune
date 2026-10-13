use crate::{
    source::{ConversionSyntax, Span},
    types::IntegerType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Bool,
    String,
    Integer(IntegerType),
    F32,
    F64,
}

impl Type {
    /// 組み込み型の名前を解決します。関数やユーザー定義型で上書きできません。
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(Self::Bool),
            "string" => Some(Self::String),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            name => IntegerType::from_name(name).map(Self::Integer),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSpec {
    Explicit(TypeRef),
    Infer,
}

/// ソースに書かれた、まだ意味解析で解決していない型です。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub kind: TypeRefKind,
    pub span: Span,
}

impl TypeRef {
    pub fn is_named(&self, expected: &str) -> bool {
        matches!(&self.kind, TypeRefKind::Named(name) if name == expected)
    }
}

/// ソースに書かれた型の形を、入れ子を失わずに保持します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefKind {
    Named(String),
    DynamicArray {
        element: Box<TypeRef>,
    },
    ArrayConstant {
        element: Box<TypeRef>,
        constant: Box<ArrayLengthRef>,
    },
    Array {
        element: Box<TypeRef>,
        length: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayLengthRef {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn statement(&self, index: usize) -> &Stmt {
        self.items
            .iter()
            .filter_map(|item| match item {
                Item::TypeDefinition(_)
                | Item::FunctionDefinition(_)
                | Item::ConstantDefinition(_) => None,
                Item::Statement(statement) => Some(statement),
            })
            .nth(index)
            .expect("statement index must exist")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    ConstantDefinition(ConstantDefinition),
    TypeDefinition(TypeDefinition),
    FunctionDefinition(FunctionDefinition),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDefinition {
    pub name: String,
    pub name_span: Span,
    pub type_ref: TypeRef,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub generic_parameters: Vec<GenericParameter>,
    pub name: String,
    pub name_span: Span,
    pub parameters: Vec<Parameter>,
    pub return_type: ReturnTypeRef,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParameter {
    pub name: String,
    pub span: Span,
    /// trueは正のi64配列長、falseは型パラメーターです。
    pub length: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArgument {
    /// 名前の意味は対応する仮引数（型または長さ）で決まります。
    Type(TypeRef),
    Length {
        value: usize,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericOrigin {
    pub template: String,
    pub definition: Span,
    pub arguments: Vec<String>,
    pub calls: Vec<GenericUse>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericUse {
    pub span: Span,
    pub argument_spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericCall {
    pub name: String,
    pub name_span: Span,
    pub generic_arguments: Vec<GenericArgument>,
    pub arguments: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub name_span: Span,
    pub type_ref: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnTypeRef {
    Void(Span),
    Value(TypeRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    /// enumの選択肢です。通常の構造体ではNoneです。
    pub variants: Option<Vec<VariantDefinition>>,
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<FieldDefinition>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantDefinition {
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<FieldDefinition>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub guard: Option<Expr>,
    pub result: Option<Box<Expr>>,
    pub variant: String,
    pub variant_span: Span,
    pub fields: Vec<PatternField>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternField {
    pub name: String,
    pub name_span: Span,
    pub binding: String,
    pub binding_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDefinition {
    pub name: String,
    pub name_span: Span,
    pub type_ref: TypeRef,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    ForEach {
        index: Option<IterationBinding>,
        element: IterationBinding,
        value: Expr,
        body: Vec<Stmt>,
        header_span: Span,
    },
    Match {
        value: Expr,
        arms: Vec<MatchArm>,
    },
    /// 共通フロントエンドが作る、束縛の範囲を保つブロックです。
    Block(Vec<Stmt>),
    Binding {
        mutable: bool,
        name: String,
        type_spec: TypeSpec,
        value: Expr,
    },
    Assignment {
        target: AssignmentTarget,
        value: Expr,
    },
    Flush,
    Print {
        value: Expr,
    },
    Call {
        value: Expr,
    },
    Return {
        value: Option<Expr>,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        initializer: Box<Stmt>,
        condition: Expr,
        update: Box<Stmt>,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationBinding {
    pub mutable: bool,
    pub name: String,
    pub type_spec: TypeSpec,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentTarget {
    pub name: String,
    pub name_span: Span,
    pub projections: Vec<AssignmentProjection>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentProjection {
    Index { index: Expr, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// 型が決まる前の10進整数リテラルを保持します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerLiteral {
    digits: String,
    explicit_type: Option<IntegerType>,
}

impl IntegerLiteral {
    /// 符号を含まない10進数字から整数リテラルを作ります。
    pub fn decimal(digits: impl Into<String>) -> Self {
        Self {
            digits: digits.into(),
            explicit_type: None,
        }
    }

    pub fn with_type(digits: impl Into<String>, ty: IntegerType) -> Self {
        Self {
            digits: digits.into(),
            explicit_type: Some(ty),
        }
    }

    pub const fn explicit_type(&self) -> Option<IntegerType> {
        self.explicit_type
    }

    /// 符号や型接尾辞を含まない10進数字を返します。
    pub fn digits(&self) -> &str {
        &self.digits
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// matchのコピー束縛と分岐を表す、共通フロントエンド内の式です。
    Let {
        name: String,
        type_ref: TypeRef,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
    },
    GenericCall(Box<GenericCall>),
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Convert {
        mode: crate::types::ConversionMode,
        target: TypeRef,
        value: Box<Expr>,
        syntax: ConversionSyntax,
    },
    Boolean(bool),
    String(String),
    Integer(IntegerLiteral),
    Float {
        text: String,
        explicit_type: Option<Type>,
    },
    Variable(String),
    Construct {
        type_name: String,
        type_name_span: Span,
        /// 更新式の先頭で一度だけ評価する、同じ型の元の値です。
        base: Option<Box<Expr>>,
        fields: Vec<FieldValue>,
    },
    FieldAccess {
        base: Box<Expr>,
        field_name: String,
        field_name_span: Span,
    },
    Array(Vec<Expr>),
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        name: String,
        name_span: Span,
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
    /// タグ・非選択領域など、共通フロントエンドが合成した値です。
    pub generated: bool,
    pub name: String,
    pub name_span: Span,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
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
