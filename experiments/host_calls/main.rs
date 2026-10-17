//! 同じCerune関数のVM埋め込み・生成WATを比較するための成果物を作ります。
use cerune_lang::{
    bytecode,
    embedding::{self, HostValue},
    modules,
};
use std::{fs, path::PathBuf, sync::Arc};
fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    fs::create_dir_all(&dir).unwrap();
    let source = include_str!("../../examples/host_calls/math.ceru");
    let hir = modules::from_source(source).unwrap().to_ir().unwrap();
    fs::write(dir.join("source.ceir"), cerune_lang::ir::text::emit(&hir)).unwrap();
    let program = Arc::new(bytecode::lower(&hir).unwrap());
    fs::write(dir.join("source.cebc"), bytecode::format_program(&program)).unwrap();
    let find = |name: &str| {
        hir.function_definitions
            .iter()
            .find(|f| f.name.ends_with(&format!("_{name}")))
            .unwrap()
    };
    let step = embedding::resolve_function(Arc::clone(&program), &find("step").name).unwrap();
    for (a, b, c, result) in [
        (10.0, 2.0, 0.5, 11.0),
        (-1.0, 4.0, 0.25, 0.0),
        (1.0, 0.0, 3.0, 1.0),
    ] {
        let value = embedding::invoke_function(
            &step,
            &[HostValue::F64(a), HostValue::F64(b), HostValue::F64(c)],
        )
        .unwrap();
        assert_eq!(value.return_value(), Some(&HostValue::F64(result)));
    }
    let checked = embedding::resolve_function(Arc::clone(&program), &find("checked").name).unwrap();
    let embedding::FunctionInvocationError::Execution(error) =
        embedding::invoke_function(&checked, &[HostValue::F64(1.5)]).unwrap_err()
    else {
        panic!()
    };
    fs::write(
        dir.join("failure.txt"),
        error.runtime_failure().unwrap().record(),
    )
    .unwrap();
    let mut wat = cerune_lang::codegen::wat::emit_wat(&hir).unwrap();
    fs::write(dir.join("plain.wat"), &wat).unwrap();
    assert_eq!(wat.matches("  (export \"main\" (func $main))").count(), 1);
    wat = wat.replace("  (export \"main\" (func $main))", "");
    let mut wrappers = String::new();
    let name = |n: &str| {
        let f = find(n);
        format!("cerune_fn_{}_{}", f.name.replace('$', "_"), f.id.0)
    };
    for n in ["step", "checked", "echo_u64", "echo_f32"] {
        wrappers.push_str(&format!("  (export \"{n}\" (func ${}))\n", name(n)));
    }
    wrappers.push_str(&format!("  (func (export \"increment\") (param i32) (result i32) local.get 0 i64.extend_i32_s call ${} i32.wrap_i64)\n",name("increment")));
    let end = wat.rfind(')').unwrap();
    wat.insert_str(end, &wrappers);
    fs::write(dir.join("exports.wat"), wat).unwrap();
    let demo = modules::load(std::path::Path::new("examples/host_calls/demo.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    let expected = "11\n0\n42\n42\n18446744073709551615\n1.25\n";
    assert_eq!(cerune_lang::ir_executor::run(&demo).unwrap(), expected);
    assert_eq!(
        cerune_lang::vm::run(&bytecode::lower(&demo).unwrap()).unwrap(),
        expected
    );
    fs::write(dir.join("demo.stdout"), expected).unwrap();
}
