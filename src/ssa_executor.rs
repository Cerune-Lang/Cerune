//! SSAのblock・辺の引数・値・残存slotを直接実行します。
//! 元MIRへの逆変換や、HIR/VM実行へのfallbackは行いません。
use crate::runtime::output::{self, Capture, Sink};
use crate::{
    ir,
    mir::{
        self,
        ssa::{self, Operand, TerminatorKind as T},
    },
    mir_executor::{
        self, Fault, Result,
        semantics::{self, Calls, Frame as _, State},
    },
    runtime::{RuntimeFailure, value::Value},
};
pub use mir_executor::ErrorKind;

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
        let incoming = edge
            .arguments
            .iter()
            .map(|v| self.get(Operand::Value(*v)))
            .collect::<Result<Vec<_>>>()?;
        for (&arg, value) in target.arguments.iter().zip(incoming) {
            self.set(Operand::Value(arg), value)?;
        }
        Ok(edge.target)
    }
}
struct Executor<'a, 'o> {
    program: &'a ssa::Program,
    state: State<'o>,
    active: Vec<Option<ir::FunctionId>>,
}
/// 検証済みのSSAを新しいheapとframeで直接実行します。
/// 時間制限や外部Imageのsandboxを提供するAPIではありません。
pub fn run(program: &ssa::Program) -> std::result::Result<String, ExecutionError> {
    run_sink(program, &mut Capture::default())
}

/// 接続先へ逐次出力します。自動flushや出力の再収集は行いません。
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
    ssa::validate(program).map_err(|e| error(program, mir_executor::validation_error(e)))?;
    let mut executor = Executor::new(program, output);
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
            active: vec![],
        }
    }
    fn entry(&mut self) -> Result<()> {
        self.function(None, vec![])?;
        self.state.check_released()
    }
    fn function(&mut self, id: Option<ir::FunctionId>, args: Vec<Value>) -> Result<Option<Value>> {
        if self.active.contains(&id) {
            return Err(Fault::invalid("recursive SSA call is not supported"));
        }
        let program = self.program;
        let (function, original) = match id {
            None => (&program.main, &program.original.main),
            Some(id) => (
                program
                    .functions
                    .get(id.0)
                    .ok_or_else(|| Fault::invalid("unknown SSA function"))?,
                program
                    .original
                    .functions
                    .get(id.0)
                    .ok_or_else(|| Fault::invalid("unknown SSA signature"))?,
            ),
        };
        if function.parameters.len() != args.len() {
            return Err(Fault::invalid("SSA argument count"));
        }
        let mut frame = Frame {
            function,
            original,
            values: vec![None; function.values.len()],
            slots: vec![None; original.locals.len()],
        };
        for (&parameter, value) in function.parameters.iter().zip(args) {
            frame.set(parameter, value)?;
        }
        self.active.push(id);
        let result = self.blocks(&mut frame);
        self.active.pop();
        result
    }
    fn blocks(&mut self, frame: &mut Frame<'_>) -> Result<Option<Value>> {
        let f = frame.function;
        let mut current = f.entry;
        loop {
            let b = f
                .blocks
                .get(current.0)
                .ok_or_else(|| Fault::invalid("unknown SSA block"))?;
            for i in &b.instructions {
                semantics::instruction(self, &i.kind, i.origin, frame).map_err(|e| {
                    e.at(
                        mir_executor::Location {
                            function: frame.original.id,
                            block: b.original_block,
                            instruction: i.original_instruction,
                        },
                        i.origin,
                    )
                })?;
            }
            let t = &b.terminator;
            let location = mir_executor::Location {
                function: frame.original.id,
                block: b.original_block,
                instruction: t.original_instruction,
            };
            let fault = |e: Fault| e.at(location, t.origin);
            match &t.kind {
                T::Jump(edge) => current = frame.transfer(edge).map_err(fault)?,
                T::Branch {
                    condition,
                    then_edge,
                    else_edge,
                } => {
                    let yes = frame
                        .get(Operand::Value(*condition))
                        .and_then(|v| v.boolean().map_err(Into::into))
                        .map_err(fault)?;
                    current = frame
                        .transfer(if yes { then_edge } else { else_edge })
                        .map_err(fault)?;
                }
                T::Return(value) => {
                    let result = value.map(|v| frame.get(v)).transpose().map_err(fault)?;
                    match (&frame.original.return_type, &result) {
                        (ir::ReturnType::Void, None) => {}
                        (ir::ReturnType::Value(ty), Some(v)) => {
                            v.check(ty).map_err(Fault::from).map_err(fault)?
                        }
                        _ => return Err(fault(Fault::invalid("SSA return signature"))),
                    }
                    return Ok(result);
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
        &self.program.original.types
    }
    fn call(&mut self, id: ir::FunctionId, args: Vec<Value>) -> Result<Option<Value>> {
        self.function(Some(id), args)
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
}
