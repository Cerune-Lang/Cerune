//! 型付きMIRの局所値・命令・CFGを直接実行します。
//! HIR実行器やbytecode/VMの実行には委譲しません。
use crate::runtime::arguments::{self, Arguments};
use crate::runtime::call;
use crate::{
    ir,
    mir::{self, TerminatorKind as T},
    runtime::{
        FailureCode, RuntimeFailure,
        array_heap::ArrayError,
        string_heap::HeapError,
        value::{self, Value},
    },
};
use std::num::NonZeroU32;

pub(crate) mod semantics;
use crate::runtime::input;
use crate::runtime::output::{self, Capture, Sink};
use semantics::{Calls, Frame as _, State};

/// emit-mir内の関数・ブロック・命令を識別します。Noneは入口の本体です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub function: Option<ir::FunctionId>,
    pub block: mir::BlockId,
    pub instruction: mir::InstructionId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    CallDepth(Box<call::Failure>),
    Arguments(Box<arguments::Failure>),
    Input(Box<input::Failure>),
    Output(Box<output::Failure>),
    Validation(Box<mir::Error>),
    Runtime(FailureCode),
    InvalidMir(&'static str),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionError {
    kind: ErrorKind,
    origin: Option<mir::SourceOrigin>,
    location: Option<Location>,
    output: String,
}
impl ExecutionError {
    pub fn call_failure(&self) -> Option<&call::Failure> {
        match &self.kind {
            ErrorKind::CallDepth(f) => Some(f),
            _ => None,
        }
    }
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    pub fn origin(&self) -> Option<mir::SourceOrigin> {
        self.origin
    }
    pub fn location(&self) -> Option<Location> {
        self.location
    }
    pub fn output(&self) -> &str {
        &self.output
    }
    pub fn runtime_failure(&self) -> Option<RuntimeFailure> {
        match (&self.kind, self.origin) {
            (ErrorKind::Runtime(code), Some(o)) => Some(RuntimeFailure {
                code: *code,
                node_id: o.node_id,
                span: o.span,
            }),
            _ => None,
        }
    }
    pub fn diagnostic(&self) -> crate::diagnostic::Diagnostic {
        if let ErrorKind::Validation(e) = &self.kind {
            return e.diagnostic();
        }
        let message = match &self.kind {
            ErrorKind::CallDepth(failure) => failure.to_string(),
            ErrorKind::Arguments(failure) => failure.to_string(),
            ErrorKind::Input(failure) => failure.to_string(),
            ErrorKind::Output(failure) => failure.to_string(),
            ErrorKind::Runtime(code) => format!("runtime error: {}", code.name()),
            ErrorKind::InvalidMir(reason) => format!("invalid Cerune MIR: {reason}"),
            ErrorKind::Validation(_) => unreachable!(),
        };
        match self.origin {
            Some(o) => crate::diagnostic::Diagnostic::new(message, o.span),
            None => crate::diagnostic::Diagnostic::without_span(message),
        }
    }
}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostic().message())
    }
}
impl std::error::Error for ExecutionError {}

#[derive(Debug)]
pub(crate) struct Fault {
    kind: ErrorKind,
    origin: Option<mir::SourceOrigin>,
    location: Option<Location>,
}
pub(crate) type Result<T> = std::result::Result<T, Fault>;
impl Fault {
    pub(crate) fn call_failure(failure: call::EnterError) -> Self {
        Self {
            kind: match failure {
                call::EnterError::Depth(f) => ErrorKind::CallDepth(f),
                call::EnterError::Allocation => ErrorKind::Runtime(FailureCode::AllocationFailed),
            },
            origin: None,
            location: None,
        }
    }
    pub(crate) fn output(failure: output::Failure) -> Self {
        Self {
            kind: ErrorKind::Output(Box::new(failure)),
            origin: None,
            location: None,
        }
    }
    pub(crate) fn invalid(reason: &'static str) -> Self {
        Self {
            kind: ErrorKind::InvalidMir(reason),
            origin: None,
            location: None,
        }
    }
    fn failure(code: FailureCode) -> Self {
        Self {
            kind: ErrorKind::Runtime(code),
            origin: None,
            location: None,
        }
    }
    pub(crate) fn at(mut self, location: Location, origin: mir::Origin) -> Self {
        // 呼出し先で決まった命令と出自を、呼出し元の位置で上書きしません。
        if self.location.is_none() {
            self.location = Some(location);
            self.origin = origin.source();
        }
        self
    }
}
impl From<value::Error> for Fault {
    fn from(e: value::Error) -> Self {
        match e {
            value::Error::Invalid(s) => Self::invalid(s),
            value::Error::Runtime(c) => Self::failure(c),
        }
    }
}
impl From<ArrayError> for Fault {
    fn from(e: ArrayError) -> Self {
        match e {
            ArrayError::AllocationSizeOverflow => {
                Self::failure(FailureCode::AllocationSizeOverflow)
            }
            ArrayError::AllocationLimitExceeded => {
                Self::failure(FailureCode::AllocationLimitExceeded)
            }
            ArrayError::AllocationFailed => Self::failure(FailureCode::AllocationFailed),
            ArrayError::IndexOutOfBounds => Self::failure(FailureCode::ArrayIndexOutOfBounds),
            ArrayError::InvalidOwnership => Self::invalid("invalid array ownership"),
        }
    }
}
impl From<HeapError> for Fault {
    fn from(e: HeapError) -> Self {
        match e {
            HeapError::AllocationSizeOverflow => Self::failure(FailureCode::AllocationSizeOverflow),
            HeapError::AllocationLimitExceeded => {
                Self::failure(FailureCode::AllocationLimitExceeded)
            }
            HeapError::AllocationFailed => Self::failure(FailureCode::AllocationFailed),
            HeapError::InvalidStringOwnership => Self::invalid("invalid string ownership"),
        }
    }
}
struct Frame<'a> {
    locals: &'a [mir::Local],
    values: Vec<Option<Value>>,
}
impl semantics::Frame<mir::LocalId> for Frame<'_> {
    fn get(&self, id: mir::LocalId) -> Result<Value> {
        self.values
            .get(id.0)
            .and_then(|v| v.clone())
            .ok_or_else(|| Fault::invalid("uninitialized MIR local"))
    }
    fn set(&mut self, id: mir::LocalId, v: Value) -> Result<()> {
        *self
            .values
            .get_mut(id.0)
            .ok_or_else(|| Fault::invalid("unknown MIR local"))? = Some(v);
        Ok(())
    }
    fn ty(&self, id: mir::LocalId) -> Result<&ir::Type> {
        self.locals
            .get(id.0)
            .map(|l| &l.ty)
            .ok_or_else(|| Fault::invalid("unknown MIR local"))
    }
}
struct Executor<'a, 'o> {
    program: &'a mir::Program,
    state: State<'o>,
    active: Vec<Option<ir::FunctionId>>,
}
/// コンパイラ内で構築したMIRを検証後、新しい実行状態で実行します。
/// 成功時にも論理的な領域が残っていたら内部不整合として返します。
/// 外部の不正入力に対するsandboxや、実行時間の制限を提供するAPIではありません。
pub fn run(program: &mir::Program) -> std::result::Result<String, ExecutionError> {
    run_sink(program, &mut Capture::default())
}

/// ソース関数の同時呼出し数を明示した上限で検査します。既定値は選びません。
/// 物理stack・memory消費を制限するAPIではありません。
pub fn run_with_call_limit(
    program: &mir::Program,
    limit: NonZeroU32,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(program, &mut Capture::default(), None, None, Some(limit))
}

/// 接続先へ逐次出力し、ソースのflush命令を実行します。終了時の自動flushや再収集は行いません。
pub fn run_with_output<W: std::io::Write>(
    program: &mir::Program,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_sink(program, output).map(|_| ())
}
fn run_sink(
    program: &mir::Program,
    output: &mut dyn Sink,
) -> std::result::Result<String, ExecutionError> {
    run_connections(program, output, None, None)
}
pub fn run_with_io<R: std::io::Read, W: std::io::Write>(
    program: &mir::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, Some(input), None).map(|_| ())
}
pub fn run_with_arguments(
    program: &mir::Program,
    arguments: &Arguments,
) -> std::result::Result<String, ExecutionError> {
    run_connections(program, &mut Capture::default(), None, Some(arguments))
}
pub fn run_with_io_and_arguments<R: std::io::Read, W: std::io::Write>(
    program: &mir::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
    arguments: &Arguments,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, Some(input), Some(arguments)).map(|_| ())
}
fn run_connections<'o>(
    program: &mir::Program,
    output: &'o mut dyn Sink,
    input: Option<&'o mut dyn input::Source>,
    arguments: Option<&'o Arguments>,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(program, output, input, arguments, None)
}
fn run_connections_with_limit<'o>(
    program: &mir::Program,
    output: &'o mut dyn Sink,
    input: Option<&'o mut dyn input::Source>,
    arguments: Option<&'o Arguments>,
    limit: Option<NonZeroU32>,
) -> std::result::Result<String, ExecutionError> {
    mir::validate(program).map_err(validation_error)?;
    let mut executor = Executor::new(program, output);
    executor.state.input = input;
    executor.state.arguments = arguments;
    executor.state.calls = limit.map(call::Stack::new);
    let result = executor.entry();
    finish(result, executor.state)
}
pub(crate) fn validation_error(e: mir::Error) -> ExecutionError {
    ExecutionError {
        origin: e.origin,
        kind: ErrorKind::Validation(Box::new(e)),
        location: None,
        output: String::new(),
    }
}
pub(crate) fn finish(
    result: Result<()>,
    state: State<'_>,
) -> std::result::Result<String, ExecutionError> {
    match result {
        Ok(()) => Ok(state.output.captured().to_owned()),
        Err(e) => Err(ExecutionError {
            kind: e.kind,
            origin: e.origin,
            location: e.location,
            output: state.output.captured().to_owned(),
        }),
    }
}
impl<'a, 'o> Executor<'a, 'o> {
    fn new(program: &'a mir::Program, output: &'o mut dyn Sink) -> Self {
        Self {
            program,
            state: State::new(program.string_heap_limit, program.array_heap_limit, output),
            active: vec![],
        }
    }
    fn entry(&mut self) -> Result<()> {
        let program = self.program;
        self.function(&program.main, vec![])?;
        self.state.check_released()
    }
    fn call(
        &mut self,
        id: ir::FunctionId,
        args: Vec<Value>,
        origin: Option<mir::SourceOrigin>,
    ) -> Result<Option<Value>> {
        let program = self.program;
        let f = program
            .functions
            .get(id.0)
            .filter(|f| f.id == Some(id))
            .ok_or_else(|| Fault::invalid("unknown MIR function"))?;
        let counted = f.lowering.is_none() && self.state.calls.is_some();
        if counted {
            self.state
                .calls
                .as_mut()
                .unwrap()
                .enter(call::Frame {
                    function: id,
                    origin: origin.map(|o| call::Origin {
                        node_id: o.node_id,
                        span: o.span,
                    }),
                })
                .map_err(Fault::call_failure)?;
        }
        let result = self.function(f, args);
        if counted {
            self.state.calls.as_mut().unwrap().leave();
        }
        result
    }
    fn function(&mut self, f: &mir::Function, args: Vec<Value>) -> Result<Option<Value>> {
        if self.active.contains(&f.id) {
            return Err(Fault::invalid("recursive MIR call is not supported"));
        }
        if f.parameters.len() != args.len() {
            return Err(Fault::invalid("MIR argument count"));
        }
        let mut frame = Frame {
            locals: &f.locals,
            values: vec![None; f.locals.len()],
        };
        for (id, v) in f.parameters.iter().zip(args) {
            v.check(&f.locals[id.0].ty)?;
            frame.set(*id, v)?;
        }
        self.active.push(f.id);
        let result = self.blocks(f, &mut frame);
        self.active.pop();
        result
    }
    fn blocks(&mut self, f: &mir::Function, frame: &mut Frame<'_>) -> Result<Option<Value>> {
        let mut current = f.entry;
        loop {
            let b = f
                .blocks
                .get(current.0)
                .ok_or_else(|| Fault::invalid("unknown MIR block"))?;
            for i in &b.instructions {
                semantics::instruction(self, &i.kind, i.origin, frame).map_err(|e| {
                    e.at(
                        Location {
                            function: f.id,
                            block: current,
                            instruction: i.id,
                        },
                        i.origin,
                    )
                })?;
            }
            let location = Location {
                function: f.id,
                block: current,
                instruction: b.terminator.id,
            };
            let fault = |e: Fault| e.at(location, b.terminator.origin);
            match &b.terminator.kind {
                T::Jump(next) => current = *next,
                T::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let yes = frame
                        .get(*condition)
                        .and_then(|v| v.boolean().map_err(Into::into))
                        .map_err(fault)?;
                    current = if yes { *then_block } else { *else_block };
                }
                T::Return(v) => {
                    let result = v.map(|v| frame.get(v)).transpose().map_err(fault)?;
                    match (&f.return_type, &result) {
                        (ir::ReturnType::Void, None) => {}
                        (ir::ReturnType::Value(t), Some(v)) => {
                            v.check(t).map_err(Fault::from).map_err(fault)?
                        }
                        _ => return Err(fault(Fault::invalid("MIR return signature"))),
                    }
                    return Ok(result);
                }
                T::Unreachable => {
                    return Err(fault(Fault::invalid("reached unreachable MIR block")));
                }
            }
        }
    }
}
impl<'o> Calls<'o> for Executor<'_, 'o> {
    fn state(&mut self) -> &mut State<'o> {
        &mut self.state
    }
    fn types(&self) -> &[mir::TypeDefinition] {
        &self.program.types
    }
    fn call(
        &mut self,
        id: ir::FunctionId,
        args: Vec<Value>,
        origin: Option<mir::SourceOrigin>,
    ) -> Result<Option<Value>> {
        Executor::call(self, id, args, origin)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injected_allocation_failures_keep_site_and_prior_output() {
        for (src, array, expected) in [
            (
                r#"print("before"); print(concat("a","b"));"#,
                false,
                r#"concat("a","b")"#,
            ),
            (
                r#"print("before"); a:[i64]=array_copy([1]);print(a);"#,
                true,
                "array_copy([1])",
            ),
        ] {
            let p = crate::mir::lower(&crate::compile_to_ir(src).unwrap()).unwrap();
            let mut captured = Capture::default();
            let mut executor = Executor::new(&p, &mut captured);
            if array {
                executor.state.arrays.fail_next_allocation();
            } else {
                executor.state.strings.fail_next_allocation();
            }
            let e = executor.entry().unwrap_err();
            assert_eq!(e.kind, ErrorKind::Runtime(FailureCode::AllocationFailed));
            assert_eq!(executor.state.output.captured(), "before\n");
            let o = e.origin.unwrap();
            assert_eq!(&src[o.span.start()..o.span.end()], expected);
            assert!(e.location.is_some());
            assert!(executor.state.arrays.is_empty());
            assert!(executor.state.strings.is_empty());
            assert!(run(&p).is_ok());
        }
    }
}
