//! SSAのblock・辺の引数・値・残存slotを直接実行します。
//! 元MIRへの逆変換や、HIR/VM実行へのfallbackは行いません。
use crate::runtime::arguments::Arguments;
use crate::runtime::call;
use crate::runtime::input;
use crate::runtime::output::{self, Capture, Sink};
use crate::{
    ir,
    mir::{
        self,
        ssa::{self, Operand, TerminatorKind as T},
    },
    mir_executor::{
        self, Fault, Result,
        semantics::{self, Context, Frame as _, State},
    },
    runtime::{RuntimeFailure, value::Value},
};
pub use mir_executor::ErrorKind;
use std::num::NonZeroU32;

/// blockはSSA側の番号です。元MIRの位置は別フィールドに保持します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub function: Option<ir::FunctionId>,
    pub block: mir::BlockId,
    pub original_block: mir::BlockId,
    pub original_instruction: mir::InstructionId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionError {
    inner: Box<mir_executor::ExecutionError>,
    location: Option<Location>,
}
impl ExecutionError {
    pub fn call_failure(&self) -> Option<&call::Failure> {
        self.inner.call_failure()
    }
    pub fn kind(&self) -> &ErrorKind {
        self.inner.kind()
    }
    pub fn origin(&self) -> Option<mir::SourceOrigin> {
        self.inner.origin()
    }
    pub fn location(&self) -> Option<Location> {
        self.location
    }
    pub fn output(&self) -> &str {
        self.inner.output()
    }
    pub fn runtime_failure(&self) -> Option<RuntimeFailure> {
        self.inner.runtime_failure()
    }
    pub fn diagnostic(&self) -> crate::diagnostic::Diagnostic {
        self.inner.diagnostic()
    }
}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
impl std::error::Error for ExecutionError {}
fn error(program: &ssa::Program, inner: mir_executor::ExecutionError) -> ExecutionError {
    let location = inner.location().and_then(|location| {
        let f = match location.function {
            Some(id) => program.functions.get(id.0)?,
            None => &program.main,
        };
        let block = f
            .blocks
            .iter()
            .position(|b| b.original_block == location.block)?;
        Some(Location {
            function: location.function,
            block: mir::BlockId(block),
            original_block: location.block,
            original_instruction: location.instruction,
        })
    });
    ExecutionError {
        inner: Box::new(inner),
        location,
    }
}
struct Frame<'a> {
    function: &'a ssa::Function,
    original: &'a mir::Function,
    values: Vec<Option<Value>>,
    slots: Vec<Option<Value>>,
}
impl semantics::Frame<Operand> for Frame<'_> {
    fn get(&self, id: Operand) -> Result<Value> {
        let value = match id {
            Operand::Value(v) => self.values.get(v.0),
            Operand::Slot(id) => self.slots.get(id.0),
        };
        value
            .and_then(Clone::clone)
            .ok_or_else(|| Fault::invalid("uninitialized SSA value/slot"))
    }
    fn set(&mut self, id: Operand, value: Value) -> Result<()> {
        value.check(self.ty(id)?)?;
        let place = match id {
            Operand::Value(v) => self.values.get_mut(v.0),
            Operand::Slot(id) => self.slots.get_mut(id.0),
        };
        *place.ok_or_else(|| Fault::invalid("unknown SSA value/slot"))? = Some(value);
        Ok(())
    }
    fn ty(&self, id: Operand) -> Result<&ir::Type> {
        match id {
            Operand::Value(v) => self.function.values.get(v.0).map(|v| &v.ty),
            Operand::Slot(id) => self.original.locals.get(id.0).map(|l| &l.ty),
        }
        .ok_or_else(|| Fault::invalid("unknown SSA value/slot type"))
    }
}
impl Frame<'_> {
    fn transfer(&mut self, edge: &ssa::Edge) -> Result<mir::BlockId> {
        let target = self
            .function
            .blocks
            .get(edge.target.0)
            .ok_or_else(|| Fault::invalid("unknown SSA edge target"))?;
        if target.arguments.len() != edge.arguments.len() {
            return Err(Fault::invalid("SSA edge argument count"));
        }
        // 入力をすべて読んでから書きます。自己ループの入替えでも古い値を失いません。
        let mut incoming = Vec::new();
        semantics::reserve(&mut incoming, edge.arguments.len())?;
        for v in &edge.arguments {
            incoming.push(self.get(Operand::Value(*v))?);
        }
        for (&arg, value) in target.arguments.iter().zip(incoming) {
            self.set(Operand::Value(arg), value)?;
        }
        Ok(edge.target)
    }
}
struct Executor<'a, 'o> {
    program: &'a ssa::Program,
    state: State<'o>,
}
/// 検証済みのSSAを新しいheapとframeで直接実行します。
/// 時間制限や外部Imageのsandboxを提供するAPIではありません。
pub fn run(program: &ssa::Program) -> std::result::Result<String, ExecutionError> {
    run_sink(program, &mut Capture::default())
}

/// ソース関数の同時呼出し数を明示した上限で検査します。既定値は選びません。
/// 物理stack・memory消費を制限するAPIではありません。
pub fn run_with_call_limit(
    program: &ssa::Program,
    limit: NonZeroU32,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(program, &mut Capture::default(), None, None, Some(limit))
}

/// 接続先へ逐次出力し、ソースのflush命令を実行します。終了時の自動flushや再収集は行いません。
pub fn run_with_output<W: std::io::Write>(
    program: &ssa::Program,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_sink(program, output).map(|_| ())
}
fn run_sink(
    program: &ssa::Program,
    output: &mut dyn Sink,
) -> std::result::Result<String, ExecutionError> {
    run_connections(program, output, None, None)
}
pub fn run_with_io<R: std::io::Read, W: std::io::Write>(
    program: &ssa::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, Some(input), None).map(|_| ())
}
pub fn run_with_arguments(
    program: &ssa::Program,
    arguments: &Arguments,
) -> std::result::Result<String, ExecutionError> {
    run_connections(program, &mut Capture::default(), None, Some(arguments))
}
pub fn run_with_io_and_arguments<R: std::io::Read, W: std::io::Write>(
    program: &ssa::Program,
    input: &mut input::Input<R>,
    output: &mut output::Output<W>,
    arguments: &Arguments,
) -> std::result::Result<(), ExecutionError> {
    run_connections(program, output, Some(input), Some(arguments)).map(|_| ())
}
fn run_connections<'o>(
    program: &ssa::Program,
    output: &'o mut dyn Sink,
    input: Option<&'o mut dyn input::Source>,
    arguments: Option<&'o Arguments>,
) -> std::result::Result<String, ExecutionError> {
    run_connections_with_limit(program, output, input, arguments, None)
}
fn run_connections_with_limit<'o>(
    program: &ssa::Program,
    output: &'o mut dyn Sink,
    input: Option<&'o mut dyn input::Source>,
    arguments: Option<&'o Arguments>,
    limit: Option<NonZeroU32>,
) -> std::result::Result<String, ExecutionError> {
    ssa::validate(program).map_err(|e| error(program, mir_executor::validation_error(e)))?;
    let mut executor = Executor::new(program, output);
    executor.state.input = input;
    executor.state.arguments = arguments;
    executor.state.calls = limit.map(call::Stack::new);
    let result = executor.entry();
    mir_executor::finish(result, executor.state).map_err(|e| error(program, e))
}
impl<'a, 'o> Executor<'a, 'o> {
    fn new(program: &'a ssa::Program, output: &'o mut dyn Sink) -> Self {
        Self {
            program,
            state: State::new(
                program.original.string_heap_limit,
                program.original.array_heap_limit,
                output,
            ),
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
                    .ok_or_else(|| Fault::invalid("unknown SSA block"))?;
                if let Some(instruction) = block.instructions.get(frame.next) {
                    let location = mir_executor::Location {
                        function: frame.data.original.id,
                        block: block.original_block,
                        instruction: instruction.original_instruction,
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
                let location = mir_executor::Location {
                    function: frame.data.original.id,
                    block: block.original_block,
                    instruction: terminator.original_instruction,
                };
                let fault = |e: Fault| e.at(location, terminator.origin);
                match &terminator.kind {
                    T::Jump(edge) => frame.block = frame.data.transfer(edge).map_err(fault)?,
                    T::Branch {
                        condition,
                        then_edge,
                        else_edge,
                    } => {
                        let yes = frame
                            .data
                            .get(Operand::Value(*condition))
                            .and_then(|v| v.boolean().map_err(Into::into))
                            .map_err(fault)?;
                        frame.block = frame
                            .data
                            .transfer(if yes { then_edge } else { else_edge })
                            .map_err(fault)?;
                    }
                    T::Return(value) => {
                        let result = value
                            .map(|v| frame.data.get(v))
                            .transpose()
                            .map_err(fault)?;
                        match (&frame.data.original.return_type, &result) {
                            (ir::ReturnType::Void, None) => {}
                            (ir::ReturnType::Value(ty), Some(v)) => {
                                v.check(ty).map_err(Fault::from).map_err(fault)?
                            }
                            _ => return Err(fault(Fault::invalid("SSA return signature"))),
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
        return_to: Option<semantics::ReturnTo<Operand>>,
    ) -> Result<Activation<'a>> {
        let function = match id {
            None => &self.program.main,
            Some(id) => self
                .program
                .functions
                .get(id.0)
                .ok_or_else(|| Fault::invalid("unknown SSA function"))?,
        };
        let original = match id {
            None => &self.program.original.main,
            Some(id) => self
                .program
                .original
                .functions
                .get(id.0)
                .ok_or_else(|| Fault::invalid("unknown SSA signature"))?,
        };
        if function.parameters.len() != args.len() {
            return Err(Fault::invalid("SSA argument count"));
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
                function,
                original,
                values: semantics::empty_values(function.values.len())?,
                slots: semantics::empty_values(original.locals.len())?,
            };
            for (parameter, value) in function.parameters.iter().zip(args) {
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
    function: &'a ssa::Function,
    data: Frame<'a>,
    block: mir::BlockId,
    next: usize,
    return_to: Option<semantics::ReturnTo<Operand>>,
    counted: bool,
}
impl<'o> Context<'o> for Executor<'_, 'o> {
    fn state(&mut self) -> &mut State<'o> {
        &mut self.state
    }
    fn types(&self) -> &[mir::TypeDefinition] {
        &self.program.original.types
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn program(source: &str) -> ssa::Program {
        ssa::construct(&mir::lower(&crate::compile_to_ir(source).unwrap()).unwrap()).unwrap()
    }
    #[test]
    fn self_edge_reads_all_arguments_before_binding() {
        use crate::{runtime::numeric::Number, types::IntegerType};
        let p =
            program("mut a:u8=10; mut b:u8=20; while a<b {c:u8=a; a=b; b=c;} print(a); print(b);");
        let (index, block) = p
            .main
            .blocks
            .iter()
            .enumerate()
            .find(|(_, b)| b.arguments.len() == 2)
            .unwrap();
        let [a, b] = block.arguments[..] else {
            panic!()
        };
        let mut frame = Frame {
            function: &p.main,
            original: &p.original.main,
            values: vec![None; p.main.values.len()],
            slots: vec![None; p.original.main.locals.len()],
        };
        frame
            .set(
                Operand::Value(a),
                Value::Number(Number::Integer(10, IntegerType::U8)),
            )
            .unwrap();
        frame
            .set(
                Operand::Value(b),
                Value::Number(Number::Integer(20, IntegerType::U8)),
            )
            .unwrap();
        let edge = ssa::Edge {
            target: mir::BlockId(index),
            arguments: vec![b, a],
        };
        for expected in [(20, 10), (10, 20), (20, 10)] {
            assert_eq!(frame.transfer(&edge).unwrap(), edge.target);
            assert_eq!(
                frame
                    .get(Operand::Value(a))
                    .unwrap()
                    .integer(IntegerType::U8)
                    .unwrap(),
                expected.0
            );
            assert_eq!(
                frame
                    .get(Operand::Value(b))
                    .unwrap()
                    .integer(IntegerType::U8)
                    .unwrap(),
                expected.1
            );
        }
    }
    #[test]
    fn injected_allocation_failures_keep_site_and_prior_output() {
        for (source, array, expected) in [
            (
                r#"print("before"); print(concat("a","b"));"#,
                false,
                r#"concat("a","b")"#,
            ),
            (
                r#"print("before"); a:[i64]=array_copy([1]); print(a);"#,
                true,
                "array_copy([1])",
            ),
        ] {
            let p = program(source);
            let mut captured = Capture::default();
            let mut executor = Executor::new(&p, &mut captured);
            if array {
                executor.state.arrays.fail_next_allocation();
            } else {
                executor.state.strings.fail_next_allocation();
            }
            let result = executor.entry();
            assert!(executor.state.arrays.is_empty());
            assert!(executor.state.strings.is_empty());
            let e = error(
                &p,
                mir_executor::finish(result, executor.state).unwrap_err(),
            );
            assert_eq!(
                e.runtime_failure().unwrap().code,
                crate::runtime::FailureCode::AllocationFailed
            );
            assert_eq!(e.output(), "before\n");
            let origin = e.origin().unwrap();
            assert_eq!(&source[origin.span.start()..origin.span.end()], expected);
            assert!(e.location().is_some());
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
            let p = program(source);
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
            let e = mir_executor::finish(result, executor.state).unwrap_err();
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
