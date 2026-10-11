//! 実際のSSAと変換後MIRを独立したCFGモデルへ写す有限入力の検証実験です。
use cerune_lang::{
    ir::{ReturnType, Type},
    mir::{self, InstructionKind, ssa},
    types::IntegerType,
};
#[path = "mir.rs"]
#[allow(dead_code)]
mod mir_experiment;
pub const MODEL: &str = include_str!("SsaModel.lean");
pub const TOOLCHAIN: &str = include_str!("lean-toolchain");
pub const JOIN: &str = include_str!("ssa_join.ceru");
pub const SWAP: &str = include_str!("ssa_swap.ceru");

fn index(operand: &ssa::Operand) -> Result<usize, String> {
    match operand {
        ssa::Operand::Value(value) => Ok(value.0),
        ssa::Operand::Slot(_) => Err("SSA proof does not support residual slots".into()),
    }
}
fn numbers(values: &[ssa::ValueId]) -> String {
    values
        .iter()
        .map(|v| v.0.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
fn edge(edge: &ssa::Edge) -> String {
    format!("⟨{}, [{}]⟩", edge.target.0, numbers(&edge.arguments))
}
pub fn definition(program: &ssa::Program, selected: usize) -> Result<String, String> {
    ssa::validate(program).map_err(|e| e.message)?;
    let original = program
        .original
        .functions
        .get(selected)
        .ok_or("missing original function")?;
    let function = program
        .functions
        .get(selected)
        .ok_or("missing SSA function")?;
    if original.lowering.is_some()
        || original.return_type != ReturnType::Value(Type::Integer(IntegerType::U8))
        || function
            .values
            .iter()
            .any(|v| !matches!(v.ty, Type::Integer(IntegerType::U8) | Type::Bool))
    {
        return Err("SSA experiment requires an ordinary u8 function".into());
    }
    let [parameter] = function.parameters.as_slice() else {
        return Err("SSA experiment requires one parameter".into());
    };
    let parameter = index(parameter)?;
    if function.values[parameter].ty != Type::Integer(IntegerType::U8) {
        return Err("SSA parameter must be u8".into());
    }
    let mut blocks = Vec::new();
    for block in &function.blocks {
        let mut instructions = Vec::new();
        for instruction in &block.instructions {
            let InstructionKind::Assign { destination, value } = &instruction.kind else {
                return Err("unsupported SSA instruction in selected function".into());
            };
            let operation = mir_experiment::operation(value, &instruction.origin, index)?;
            instructions.push(format!(
                "⟨{}, {}, ({operation})⟩",
                instruction.original_instruction.0,
                index(destination)?
            ));
        }
        let terminator = match &block.terminator.kind {
            ssa::TerminatorKind::Return(Some(value)) => format!(".ret {}", index(value)?),
            ssa::TerminatorKind::Jump(next) => format!(".jump {}", edge(next)),
            ssa::TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => format!(
                ".branch {} {} {}",
                condition.0,
                edge(then_edge),
                edge(else_edge)
            ),
            _ => return Err("SSA experiment requires a u8 return".into()),
        };
        blocks.push(format!(
            "⟨[{}], [{}], ({terminator})⟩",
            numbers(&block.arguments),
            instructions.join(", ")
        ));
    }
    Ok(format!(
        "namespace CeruneProof\ndef ssaReference : SsaFunction := ⟨{parameter}, {}, [{}]⟩\nend CeruneProof\n",
        function.entry.0,
        blocks.join(", ")
    ))
}

/// 上限と完了は定理側で指定します。exporterがfuelや期待結果を推測しません。
pub fn emit(
    baseline: &mir::Program,
    program: &ssa::Program,
    lowered: &mir::Program,
    selected: usize,
) -> Result<String, String> {
    if baseline != &program.original {
        return Err("baseline differs from the retained original MIR".into());
    }
    Ok(format!(
        "{}\n{}\n{MODEL}\n{}\n{}\n{}",
        include_str!("Model.lean"),
        mir_experiment::MODEL,
        mir_experiment::snapshot_definition(baseline, selected, "mirReference", true)?,
        definition(program, selected)?,
        mir_experiment::snapshot_definition(lowered, selected, "loweredReference", true)?
    ))
}
