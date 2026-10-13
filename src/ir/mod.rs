mod aggregates;
mod ownership;
pub(crate) mod string_usage;

pub const DEFAULT_STRING_HEAP_LIMIT: u64 = 64 * 1024 * 1024;
pub const DEFAULT_ARRAY_HEAP_LIMIT: u64 = 64 * 1024 * 1024;
pub mod builder;
pub mod text;

use crate::{
    source::{ConversionSyntax, Span},
    types::IntegerType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,
    String,
    Integer(IntegerType),
    F32,
    F64,
    Named(TypeId),
    Array { element: Box<Type>, length: usize },
    DynamicArray { element: Box<Type> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// 実行ごとの、生存する動的文字列のバイト数の上限です。
    pub string_heap_limit: u64,
    /// 要素領域の共通計算幅による、生存する動的配列の予算です。
    pub array_heap_limit: u64,
    pub constant_definitions: Vec<ConstantDefinition>,
    pub type_definitions: Vec<TypeDefinition>,
    pub function_definitions: Vec<FunctionDefinition>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDefinition {
    /// この定数を配列型の長さとして参照した位置です。
    pub array_length_uses: Vec<Span>,
    pub id: usize,
    pub name: String,
    pub ty: Type,
    pub initializer: Expr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

/// 一回のコンパイル中で、Cerune IRの文と式を一意に識別します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    Void,
    Value(Type),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub lowering: Option<LoweringKind>,
    pub generic_origin: Option<crate::ast::GenericOrigin>,
    pub id: FunctionId,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: ReturnType,
    pub body: Vec<Statement>,
    pub span: Span,
}

/// 呼び出し境界の論理的な所有です。ABI上の値コピーや参照渡しとは区別します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentOwnership {
    /// 呼び出し側が準備した所有値を受け取り、calleeが解放または返却します。
    Owned,
    /// 呼び出し中だけ読み取り、引数自体の所有は受け取りません。
    Borrowed,
}

impl FunctionDefinition {
    /// 所有展開と表示に使う契約です。実行器は本文の明示操作を実行します。
    pub fn argument_ownership(&self) -> ArgumentOwnership {
        match self.lowering {
            None | Some(LoweringKind::OwnershipRelease | LoweringKind::OwnershipReplace) => {
                ArgumentOwnership::Owned
            }
            Some(
                LoweringKind::AggregateEquality
                | LoweringKind::AggregateDisplay
                | LoweringKind::MatchBinding
                | LoweringKind::MatchSelect
                | LoweringKind::OwnershipExpression
                | LoweringKind::OwnershipRetain,
            ) => ArgumentOwnership::Borrowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoweringKind {
    AggregateEquality,
    AggregateDisplay,
    MatchBinding,
    MatchSelect,
    OwnershipExpression,
    OwnershipRetain,
    OwnershipRelease,
    OwnershipReplace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub id: BindingId,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    /// 宣言順の選択肢名です。添字が内部タグ値に対応します。
    pub variants: Option<Vec<String>>,
    pub id: TypeId,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDefinition {
    pub id: FieldId,
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// ソース上の名前がどの束縛を指すかを一意に識別します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindingId(pub usize);

/// Cerune IRの文と、その文が由来するソース範囲を表します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub id: NodeId,
    pub kind: StatementKind,
    pub span: Span,
}

/// Cerune IRの文の種類を表します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    /// 共通IRの要素コピーの一段。未初期化の末尾へ順に所有値を格納します。
    ArrayInitialize {
        array: Expr,
        value: Expr,
    },
    ArrayRetain {
        value: Expr,
    },
    /// 最後の所有を外した後、要素を解放済みの領域を回収します。
    ArrayFree {
        value: Expr,
    },
    ArrayRangeCheck {
        length: Expr,
        start: Expr,
        end: Expr,
    },
    /// 不変の動的文字列の共有／解放。静的文字列には作用しません。
    StringManage {
        value: Expr,
        retain: bool,
    },
    Binding {
        /// trueは内部の読み取り用束縛。所有元の寿命内だけ使い、独立した保持を要求しません。
        borrowed: bool,
        id: BindingId,
        mutable: bool,
        name: String,
        ty: Type,
        value: Expr,
    },
    Assignment {
        target: AssignmentTarget,
        value: Expr,
    },
    /// 複合値の表示を構成する、改行なしの値出力です。
    Write {
        value: Expr,
        quoted: bool,
    },
    Flush,
    Print {
        value: Expr,
    },
    Call {
        function_id: FunctionId,
        function_name: String,
        arguments: Vec<Expr>,
    },
    Return {
        value: Option<Expr>,
    },
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
    pub id: BindingId,
    pub name: String,
    pub root_ty: Type,
    pub projections: Vec<AssignmentProjection>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentProjection {
    DynamicIndex {
        index: Expr,
        element: Type,
        span: Span,
    },
    Index {
        index: Expr,
        element: Type,
        length: usize,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub id: NodeId,
    pub ty: Type,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    /// 所有展開前の反復生成。値と個数を一度評価してから各要素をコピーします。
    ArrayRepeat {
        value: Box<Expr>,
        count: Box<Expr>,
    },
    /// 所有展開前の明示コピー。展開後は確保・通常のループ・要素格納になります。
    ArrayCopy {
        value: Box<Expr>,
        range: Option<(Box<Expr>, Box<Expr>)>,
    },
    ArrayAllocate {
        length: Box<Expr>,
        element_width: u64,
    },
    /// 一時値からの所有引き継ぎ用。最後の所有なら要素解放へ進みます。
    ArrayReleaseOwner {
        value: Box<Expr>,
    },
    /// 左から右へ評価し、新しい不変のバイト列を確保・コピーします。
    StringConcat {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// 共通lowering前の、matchに由来する局所束縛と値分岐です。
    Let {
        binding: BindingId,
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
    },
    Constant {
        id: usize,
        value: Box<Expr>,
    },
    /// 配列式を一度評価してから、最外側の要素数を返します。
    ArrayLength {
        value: Box<Expr>,
    },
    StringByteLength {
        value: Box<Expr>,
    },
    /// 浮動小数点を含む変換です。値を保つ変換と明示した切り捨てを区別します。
    ConvertNumeric {
        mode: crate::types::ConversionMode,
        from: crate::types::NumericType,
        to: crate::types::NumericType,
        syntax: ConversionSyntax,
        value: Box<Expr>,
    },
    /// 左辺で結果が決まらない場合だけ右辺を評価します。
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// 数値を変えずに整数型を変換します。範囲外なら失敗します。
    ConvertInteger {
        value: Box<Expr>,
        from: IntegerType,
        to: IntegerType,
        syntax: ConversionSyntax,
    },
    Boolean(bool),
    /// 不変のUTF-8文字列値です。元の表記は式のSpanから追跡します。
    String(String),
    Integer(i128),
    Float {
        text: String,
    },
    Variable {
        id: BindingId,
        name: String,
    },
    Construct {
        type_id: TypeId,
        type_name: String,
        /// 先に評価してコピーします。存在する場合、既定値は使いません。
        base: Option<Box<Expr>>,
        fields: Vec<FieldValue>,
    },
    FieldAccess {
        type_id: TypeId,
        field_id: FieldId,
        field_name: String,
        base: Box<Expr>,
    },
    Array(Vec<Expr>),
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        function_id: FunctionId,
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
    pub id: FieldId,
    pub name: String,
    pub value: Expr,
    pub origin: FieldValueOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldValueOrigin {
    Generated { span: Span },
    Explicit { span: Span },
    Default { definition_span: Span },
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

impl Type {
    pub(crate) fn contains_dynamic_array(&self) -> bool {
        match self {
            Self::DynamicArray { .. } => true,
            Self::Array { element, .. } => element.contains_dynamic_array(),
            _ => false,
        }
    }
}
impl Program {
    pub(crate) fn first_dynamic_array_span(&self) -> Option<Span> {
        for d in &self.type_definitions {
            for f in &d.fields {
                if f.ty.contains_dynamic_array() {
                    return Some(f.span);
                }
            }
        }
        for f in &self.function_definitions {
            if f.parameters.iter().any(|p| p.ty.contains_dynamic_array())
                || matches!(&f.return_type, ReturnType::Value(t) if t.contains_dynamic_array())
            {
                return Some(f.span);
            }
        }
        let span = std::cell::Cell::new(None);
        let mut copy = self.clone();
        aggregates::visit_program(
            &mut copy,
            &mut |e| {
                if span.get().is_none() && e.ty.contains_dynamic_array() {
                    span.set(Some(e.span));
                }
            },
            &mut |_| {},
        );
        span.get()
    }
}
