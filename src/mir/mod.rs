//! 完成済みCerune IRから作る、最適化なし・非SSAの実行表現です。
//! 読取りや一時保存は論理的なretain/copyではありません。所有操作は独立命令です。
pub mod lower;
mod operands;
pub mod ssa;
pub mod text;
pub mod validate;

use crate::{
    ir,
    source::{ConversionSyntax, Span},
    types::{ConversionMode, IntegerType, NumericType},
};
pub use lower::lower;
pub use validate::validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);
impl std::fmt::Display for LocalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}
/// 関数内で一意。HIR NodeIdとは別の名前空間です。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstructionId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// 定数化で式が消えても、文字列を含む入力の出力契約を保持します。
    pub uses_strings: bool,
    pub string_heap_limit: u64,
    pub array_heap_limit: u64,
    pub types: Vec<TypeDefinition>,
    pub functions: Vec<Function>,
    pub main: Function,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub id: ir::TypeId,
    pub name: String,
    pub fields: Vec<(ir::FieldId, String, ir::Type)>,
    pub variants: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: Option<ir::FunctionId>,
    pub name: String,
    pub lowering: Option<ir::LoweringKind>,
    pub argument_ownership: ir::ArgumentOwnership,
    pub parameters: Vec<LocalId>,
    pub return_type: ir::ReturnType,
    pub locals: Vec<Local>,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub ty: ir::Type,
    pub kind: LocalKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalKind {
    Binding {
        id: ir::BindingId,
        name: String,
        mutable: bool,
        borrowed: bool,
    },
    Temporary,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceOrigin {
    pub node_id: ir::NodeId,
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// checked操作の失敗位置は必ずこの単一の元操作を指します。
    Source(SourceOrigin),
    Derived {
        source: SourceOrigin,
        reason: &'static str,
    },
    Synthetic {
        reason: &'static str,
    },
}
impl Origin {
    pub fn source(self) -> Option<SourceOrigin> {
        match self {
            Self::Source(s) | Self::Derived { source: s, .. } => Some(s),
            Self::Synthetic { .. } => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub origin: Origin,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub id: InstructionId,
    pub origin: Origin,
    pub kind: InstructionKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind<R = LocalId> {
    Assign {
        destination: R,
        value: Operation<R>,
    },
    /// 呼出し先の返り値を捨てる場合も、本文の副作用を実行します。
    Call {
        function: ir::FunctionId,
        arguments: Vec<R>,
        ownership: ir::ArgumentOwnership,
    },
    /// 次の添字や右辺より前に、ここまでの添字列を検査します。
    CheckIndex {
        root: R,
        path: Vec<R>,
    },
    Store {
        root: R,
        path: Vec<R>,
        value: R,
    },
    FileWriteNew {
        name: R,
        text: R,
        max_bytes: R,
    },
    Flush,
    Output {
        value: R,
        newline: bool,
        quoted: bool,
    },
    ArrayInitialize {
        array: R,
        value: R,
    },
    ArrayRetain {
        value: R,
    },
    ArrayFree {
        value: R,
    },
    ArrayRangeCheck {
        length: R,
        start: R,
        end: R,
    },
    StringManage {
        value: R,
        retain: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Boolean(bool),
    String(String),
    Integer(i128),
    Float(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation<R = LocalId> {
    Literal(Literal),
    Copy(R),
    Unary {
        op: ir::UnaryOp,
        value: R,
    },
    Binary {
        op: ir::BinaryOp,
        left: R,
        right: R,
    },
    ConvertInteger {
        value: R,
        from: IntegerType,
        to: IntegerType,
        syntax: ConversionSyntax,
    },
    ConvertNumeric {
        value: R,
        from: NumericType,
        to: NumericType,
        mode: ConversionMode,
        syntax: ConversionSyntax,
    },
    Array(Vec<R>),
    Construct {
        ty: ir::TypeId,
        base: Option<R>,
        fields: Vec<(ir::FieldId, R)>,
    },
    Field {
        base: R,
        ty: ir::TypeId,
        field: ir::FieldId,
    },
    Index {
        base: R,
        index: R,
    },
    Call {
        function: ir::FunctionId,
        arguments: Vec<R>,
        ownership: ir::ArgumentOwnership,
    },
    ArrayLength(R),
    ArgumentCount,
    ArgumentGet(R),
    FileReadText {
        name: R,
        max_bytes: R,
    },
    ReadLine(R),
    StringByteAt {
        value: R,
        index: R,
    },
    StringByteLength(R),
    StringConcat {
        left: R,
        right: R,
    },
    ArrayAllocate {
        length: R,
        element_width: u64,
    },
    ArrayReleaseOwner(R),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator {
    pub id: InstructionId,
    pub origin: Origin,
    pub kind: TerminatorKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminatorKind {
    Jump(BlockId),
    Branch {
        condition: LocalId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return(Option<LocalId>),
    /// 到達不能な合流点。実行時に到達したら内部不整合です。
    Unreachable,
}
impl TerminatorKind {
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Self::Jump(b) => vec![*b],
            Self::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
            _ => vec![],
        }
    }
}
/// 外部Imageの入力検査ではなく、コンパイラ内部の不整合を表します。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub origin: Option<SourceOrigin>,
}
impl Error {
    pub(crate) fn new(message: impl Into<String>, origin: Origin) -> Self {
        Self {
            message: message.into(),
            origin: origin.source(),
        }
    }
    pub fn diagnostic(&self) -> crate::diagnostic::Diagnostic {
        let message = format!("MIR: {}", self.message);
        match self.origin {
            Some(o) => crate::diagnostic::Diagnostic::new(message, o.span),
            None => crate::diagnostic::Diagnostic::without_span(message),
        }
    }
}
