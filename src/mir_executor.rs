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
use semantics::{Context, Frame as _, State};

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
/// 入出力・引数・ソース関数の呼出し深さを同じ実行に接続します。
pub fn run_with_io_arguments_and_call_limit<R: std::io::Read, W: std::io::Write>(
    program: &mir::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
    arguments: &Arguments,
    limit: NonZeroU32,
) -> std::result::Result<(), ExecutionError> {
    run_connections_with_limit(program, output, Some(input), Some(arguments), Some(limit))
        .map(|_| ())
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
        }
    }
    fn entry(&mut self) -> Result<()> {
        let mut active = Vec::new();
        semantics::reserve(&mut active, 1)?;
        active.push(self.activation(None, vec![], None)?);
        let result = (|| {
            while let Some(frame) = active.last_mut() {
                let block = frame
                    .function
                    .blocks
                    .get(frame.block.0)
                    .ok_or_else(|| Fault::invalid("unknown MIR block"))?;
                if let Some(instruction) = block.instructions.get(frame.next) {
                    let location = Location {
                        function: frame.function.id,
                        block: frame.block,
                        instruction: instruction.id,
                    };
                    let fault = |e: Fault| e.at(location, instruction.origin);
                    frame.next += 1;
                    if let Some((id, destination, arguments)) =
                        semantics::call_site(&instruction.kind)
                    {
                        let args = frame.data.arguments(arguments).map_err(fault)?;
                        let callee = self
                            .activation(
                                Some(id),
                                args,
                                Some(semantics::ReturnTo {
                                    destination,
                                    location,
                                    origin: instruction.origin,
                                }),
                            )
                            .map_err(fault)?;
                        if let Err(e) = semantics::reserve(&mut active, 1) {
                            self.state.leave(callee.counted);
                            return Err(fault(e));
                        }
                        active.push(callee);
                    } else {
                        semantics::instruction(
                            self,
                            &instruction.kind,
                            instruction.origin,
                            &mut frame.data,
                        )
                        .map_err(fault)?;
                    }
                    continue;
                }
                let terminator = &block.terminator;
                let location = Location {
                    function: frame.function.id,
                    block: frame.block,
                    instruction: terminator.id,
                };
                let fault = |e: Fault| e.at(location, terminator.origin);
                match &terminator.kind {
                    T::Jump(next) => frame.block = *next,
                    T::Branch {
                        condition,
                        then_block,
                        else_block,
                    } => {
                        let yes = frame
                            .data
                            .get(*condition)
                            .and_then(|v| v.boolean().map_err(Into::into))
                            .map_err(fault)?;
                        frame.block = if yes { *then_block } else { *else_block };
                    }
                    T::Unreachable => {
                        return Err(fault(Fault::invalid("reached unreachable MIR block")));
                    }
                    T::Return(value) => {
                        let result = value
                            .map(|v| frame.data.get(v))
                            .transpose()
                            .map_err(fault)?;
                        match (&frame.function.return_type, &result) {
                            (ir::ReturnType::Void, None) => {}
                            (ir::ReturnType::Value(ty), Some(v)) => {
                                v.check(ty).map_err(Fault::from).map_err(fault)?
                            }
                            _ => return Err(fault(Fault::invalid("MIR return signature"))),
                        }
                        let finished = active.pop().unwrap();
                        self.state.leave(finished.counted);
                        if let Some(return_to) = finished.return_to {
                            return_to.resume(&mut active.last_mut().unwrap().data, result)?;
                        }
                        continue;
                    }
                }
                frame.next = 0;
            }
            Ok(())
        })();
        // 停止後はソースを再実行せず、残る内部の深さ計数だけを外します。
        while let Some(frame) = active.pop() {
            self.state.leave(frame.counted);
        }
        result?;
        self.state.check_released()
    }
    fn activation(
        &mut self,
        id: Option<ir::FunctionId>,
        args: Vec<Value>,
        return_to: Option<semantics::ReturnTo<mir::LocalId>>,
    ) -> Result<Activation<'a>> {
        let function = match id {
            None => &self.program.main,
            Some(id) => self
                .program
                .functions
                .get(id.0)
                .ok_or_else(|| Fault::invalid("unknown MIR function"))?,
        };
        let original = function;
        if function.parameters.len() != args.len() {
            return Err(Fault::invalid("MIR argument count"));
        }
        let counted = self
            .state
            .enter(original, return_to.as_ref().and_then(|r| r.origin.source()))?;
        let result = (|| {
            // 暗黙入口を除いたcalleeの確保失敗をテストで注入できます。
            if id.is_some() {
                self.state.prepare_frame()?;
            }
            let mut frame = Frame {
                locals: &function.locals,
                values: semantics::empty_values(function.locals.len())?,
            };
            for (parameter, value) in function.parameters.iter().zip(args) {
                value.check(frame.ty(*parameter)?)?;
                frame.set(*parameter, value)?;
            }
            Ok(Activation {
                function,
                data: frame,
                block: function.entry,
                next: 0,
                return_to,
                counted,
            })
        })();
        if result.is_err() {
            self.state.leave(counted);
        }
        result
    }
}
struct Activation<'a> {
    function: &'a mir::Function,
    data: Frame<'a>,
    block: mir::BlockId,
    next: usize,
    return_to: Option<semantics::ReturnTo<mir::LocalId>>,
    counted: bool,
}
impl<'o> Context<'o> for Executor<'_, 'o> {
    fn state(&mut self) -> &mut State<'o> {
        &mut self.state
    }
    fn types(&self) -> &[mir::TypeDefinition] {
        &self.program.types
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

    #[test]
    fn frame_allocation_failure_keeps_call_site_and_balances_depth() {
        for (source, site, after, limit, prior) in [
            (
                r#"fn leaf()->void {print("body");} print("before"); leaf(); print("after");"#,
                "leaf();",
                0,
                1,
                "before\n",
            ),
            (
                r#"print("before"); values:[string]=array_copy(["a"]); print(values);"#,
                "array_copy([\"a\"])",
                0,
                1,
                "before\n",
            ),
            (
                r#"fn leaf()->void {print("body");} fn outer()->void {print("inside");leaf();} print("before");outer();print("after");"#,
                "leaf();",
                1,
                2,
                "before\ninside\n",
            ),
        ] {
            let p = crate::mir::lower(&crate::compile_to_ir(source).unwrap()).unwrap();
            let mut captured = Capture::default();
            let mut executor = Executor::new(&p, &mut captured);
            executor.state.calls = Some(call::Stack::new(NonZeroU32::new(limit).unwrap()));
            executor.state.fail_frame_after = Some(after);
            let result = executor.entry();
            assert_eq!(executor.state.output.captured(), prior);
            executor.state.fail_frame_after = None;
            executor.entry().unwrap();
            assert!(
                executor
                    .state
                    .output
                    .captured()
                    .starts_with(&format!("{prior}{prior}"))
            );
            let e = finish(result, executor.state).unwrap_err();
            assert_eq!(
                e.kind(),
                &ErrorKind::Runtime(crate::runtime::FailureCode::AllocationFailed)
            );
            let origin = e.origin().unwrap();
            assert_eq!(&source[origin.span.start()..origin.span.end()], site);
            assert!(e.location().is_some());
        }
    }
}
