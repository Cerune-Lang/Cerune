use cerune_lang::{bytecode, compile_to_ir, ir_executor, mir, mir_executor, ssa_executor};

#[test]
fn scripted_input_preserves_state_line_bytes_and_stop_order() {
    let hir = compile_to_ir(include_str!("../examples/io_protocol/input_flow.ceru")).unwrap();
    let expected = include_bytes!("../examples/io_protocol/input_flow.stdout");
    let mir = mir::lower(&hir).unwrap();
    let ssa = mir::ssa::construct(&mir).unwrap();
    let bytecode = bytecode::lower(&hir).unwrap();
    for (route, output) in [
        ("HIR", ir_executor::run(&hir).unwrap()),
        ("MIR", mir_executor::run(&mir).unwrap()),
        ("SSA", ssa_executor::run(&ssa).unwrap()),
        ("VM", cerune_lang::run_bytecode(&bytecode).unwrap()),
    ] {
        assert_eq!(output.as_bytes(), expected, "{route}");
    }
}
