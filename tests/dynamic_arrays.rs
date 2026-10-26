//! 動的配列のIR・MIR・VM・生成C・LLVM・QBE・WAT・ASM・自前Objectの結果・停止理由・出自を照合します。
#[path = "support/c_arrays.rs"]
mod c_arrays;
#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/llvm_arrays.rs"]
mod llvm_arrays;
#[path = "support/native_arrays.rs"]
mod native_arrays;
#[path = "support/qbe_arrays.rs"]
mod qbe_arrays;
#[path = "support/wat_arrays.rs"]
mod wat_arrays;
use cerune_lang::{bytecode, compile_to_ir, ir, ir_executor, run_bytecode};

fn compare(source: &str, limit: u64) -> Result<String, ir_executor::ExecutionError> {
    let mut program = compile_to_ir(source).unwrap_or_else(|e| panic!("{source}\n{e:?}"));
    program.array_heap_limit = limit;
    let direct = ir_executor::run(&program);
    let mir_program = cerune_lang::mir::lower(&program).unwrap();
    let mir = cerune_lang::mir_executor::run(&mir_program);
    match (&direct, &mir) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => {
            assert!(a.runtime_failure().is_some(), "{a:?}");
            assert_eq!(a.runtime_failure(), b.runtime_failure(), "{b:?}");
            assert_eq!(a.output(), b.output());
        }
        _ => panic!("HIR: {direct:?}\nMIR: {mir:?}"),
    }
    let ssa = cerune_lang::mir::ssa::construct(&mir_program).unwrap();
    let ssa_result = cerune_lang::ssa_executor::run(&ssa);
    match (&direct, &ssa_result) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => {
            assert_eq!(a.runtime_failure(), b.runtime_failure());
            assert_eq!(a.output(), b.output());
        }
        _ => panic!("IR: {direct:?}\nSSA: {ssa_result:?}"),
    }
    let lowered = cerune_lang::mir::ssa::lower(&ssa).unwrap();
    assert_eq!(cerune_lang::mir_executor::run(&lowered.program), mir);
    let vm = run_bytecode(&bytecode::lower(&program).unwrap());
    match (&direct, &vm) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "{source}"),
        (Err(a), Err(b)) => {
            assert!(a.runtime_failure().is_some(), "{a:?}\n{source}");
            assert_eq!(a.runtime_failure(), b.runtime_failure(), "{source}");
            assert_eq!(a.output(), b.vm_error().output(), "{source}");
        }
        _ => panic!("IR: {direct:?}\nVM: {vm:?}\n{source}"),
    }
    c_arrays::compare(&program, &direct);
    llvm_arrays::compare(&program, &direct);
    qbe_arrays::compare(&program, &direct);
    wat_arrays::compare(&program, &direct);
    native_arrays::compare(&program, &direct);
    direct
}
fn success(source: &str, expected: &str) {
    assert_eq!(
        compare(source, ir::DEFAULT_ARRAY_HEAP_LIMIT).unwrap(),
        expected,
        "{source}"
    );
}
fn failure(source: &str, limit: u64, code: &str, output: &str) {
    let e = compare(source, limit).unwrap_err();
    assert_eq!(e.runtime_failure().unwrap().code.name(), code, "{source}");
    assert_eq!(e.output(), output, "{source}");
}
#[test]
fn copies_nested_arrays_and_fixed_containers_independently() {
    success(
        r#"
        mut a:[[i64]]=array_copy([array_copy([1,2]),array_copy([3])]);
        b:infer=a; a[0][0]=9; a[1]=array_copy([4,5]); a=a;
        print(a); print(b); print(a==b);
        mut c:[[i64];2]=[array_copy([1]),array_copy([2])]; d:infer=c;
        c[0][0]=7; print(c); print(d);
    "#,
        "[[9, 2], [4, 5]]\n[[1, 2], [3]]\nfalse\n[[7], [2]]\n[[1], [2]]\n",
    );
}
#[test]
fn products_enums_generics_and_temporary_projection_preserve_ownership() {
    success(
        r#"
        type Pair { a:[i64], b:[i64] }
        enum E { A { value:Pair }, B }
        fn id<T>(v:T)->T { return v; }
        fn make()->Pair {return Pair{a:array_copy([1,2]),b:array_copy([3])};}
        mut a:[i64]=make().a; a[0]=7; print(a);
        p:Pair=make(); q:Pair=id::<Pair>(p); print(p==q);
        mut e:E=E::A{value:p}; saved:E=e; e=E::B{};
        print(e);
        selected:Pair=match saved { E::A{value:v}=>v, E::B{}=>make() };
        mut copy:[i64]=selected.a; copy[0]=8; print(selected.a); print(copy);
        updated:Pair=Pair{..q,a:array_copy([9])}; print(updated.a); print(updated.b);
    "#,
        "[7, 2]\ntrue\nB{}\n[1, 2]\n[8, 2]\n[9]\n[3]\n",
    );
}
#[test]
fn iteration_uses_independent_snapshot_and_empty_arrays_work() {
    success(
        r#"
        mut a:[i64]=array_copy([1,2,3]);
        for (x:i64 in a) { print(x); a=array_copy([9]); }
        print(a);
        empty:[i64]=array_copy_range(a,1,1);
        for (x:i64 in empty) { print(99); }
        print(empty); print(array_len(empty)); print(empty==array_copy_range(a,0,0));
        print(empty==a); print(empty!=a);
        for (x:[i64] in array_copy([array_copy([1]),array_copy([2])])) {
            if x[0]==1 {continue;} print(x); break;
        }
    "#,
        "1\n2\n3\n[9]\n[]\n0\ntrue\nfalse\ntrue\n[2]\n",
    );
}
#[test]
fn all_scalar_types_and_exact_string_bytes() {
    for (ty, value, output) in [
        ("bool", "true", "true"),
        ("i8", "-128", "-128"),
        ("i16", "-32768", "-32768"),
        ("i32", "-2147483648", "-2147483648"),
        ("i64", "-9223372036854775808", "-9223372036854775808"),
        ("u8", "255", "255"),
        ("u16", "65535", "65535"),
        ("u32", "4294967295", "4294967295"),
        ("u64", "18446744073709551615", "18446744073709551615"),
        ("f32", "1.5", "1.5"),
        ("f64", "2.5", "2.5"),
    ] {
        success(
            &format!(
                "s:[{ty};1]=[{value}]; a:[{ty}]=array_copy(s); b:infer=a; print(b[0]);print(a==b);"
            ),
            &format!("{output}\ntrue\n"),
        );
    }
    success(
        r#"a:[string]=array_copy([concat("日","本\0\r\n"),"e\u{301}","é"]); b:infer=a;
        print(b[0]);print(b[1]==b[2]);print(a==b);"#,
        "日本\0\r\n\nfalse\ntrue\n",
    );
}
#[test]
fn ranges_evaluate_all_arguments_before_validation_and_preserve_output() {
    let source = r#"fn source()->[i64]{print("source");return array_copy([1,2]);}
        fn start()->i64{print("start");return -1;}
        fn end()->i64{print("end");return 2;}
        print(array_copy_range(source(),start(),end()));"#;
    failure(
        source,
        128,
        "array-range-out-of-bounds",
        "source\nstart\nend\n",
    );
    failure(
        r#"fn end()->i64{print("end");return 1/0;} a:[i64]=array_copy([1]);print(array_copy_range(a,-1,end()));"#,
        64,
        "division-by-zero",
        "end\n",
    );
    for (a, b) in [(-1, 0), (2, 1), (0, 4)] {
        failure(
            &format!("print(\"before\");print(array_copy_range([1,2,3],{a},{b}));"),
            0,
            "array-range-out-of-bounds",
            "before\n",
        );
    }
    success(
        "a:[i64]=array_copy_range([1,2,3],1,3);print(a);print(false && array_len(array_copy_range(a,-1,2))==0);",
        "[2, 3]\nfalse\n",
    );
}
#[test]
fn budget_counts_copies_but_not_reads_and_reclaims_scopes() {
    let reads = "a:[i64]=array_copy([1,2,3]);print(a);print(array_len(a));print(a==a);print(a[1]);";
    assert_eq!(compare(reads, 24).unwrap(), "[1, 2, 3]\n3\ntrue\n2\n");
    failure(reads, 23, "allocation-limit-exceeded", "");
    let self_copy = "mut a:[i64]=array_copy([1,2,3]);print(a);a=a;print(a);";
    failure(self_copy, 47, "allocation-limit-exceeded", "[1, 2, 3]\n");
    assert_eq!(compare(self_copy, 48).unwrap(), "[1, 2, 3]\n[1, 2, 3]\n");
    assert_eq!(
        compare(
            "for(mut i:i64=0;i<10;i=i+1){a:[i64]=array_copy([i]);print(a[0]);}",
            8
        )
        .unwrap(),
        "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"
    );
    assert_eq!(
        compare("print(array_copy_range([1],0,0));", 0).unwrap(),
        "[]\n"
    );
    failure(
        "fn later()->i64{print(99);return 0;} fn use_array(a:[i64],b:i64)->void{} a:[i64]=array_copy([1]); use_array(a,later());",
        8,
        "allocation-limit-exceeded",
        "",
    );
}
#[test]
fn bounds_fail_before_later_indices_or_rhs() {
    failure(
        r#"fn later()->i64{print(99);return 0;} mut a:[[i64]]=array_copy([array_copy([1])]);a[1][later()]=later();"#,
        64,
        "array-index-out-of-bounds",
        "",
    );
    failure(
        "a:[i64]=array_copy_range([1],0,0);print(a[0]);",
        0,
        "array-index-out-of-bounds",
        "",
    );
}
#[test]
fn copied_and_returned_parameters_have_one_owner_boundary() {
    let source = "fn size(a:[i64])->i64{return array_len(a);} print(size(array_copy([1,2,3])));";
    assert_eq!(compare(source, 24).unwrap(), "3\n");
    let source =
        "fn id<T>(a:T)->T{return a;} a:[i64]=array_copy([1]);b:[i64]=id::<[i64]>(a);print(b);";
    assert_eq!(compare(source, 24).unwrap(), "[1]\n");
}
#[test]
fn compiled_routes_accept_even_unused_dynamic_types() {
    use cerune_lang::codegen::{
        self,
        x86_64::{self, Target},
    };
    for source in [
        "print(array_copy([1]));",
        "type T{unused:[i64]} print(1);",
        "fn unused(a:[i64])->void{} print(1);",
    ] {
        let p = compile_to_ir(source).unwrap();
        c_arrays::compare(&p, &ir_executor::run(&p));
        llvm_arrays::compare(&p, &ir_executor::run(&p));
        qbe_arrays::compare(&p, &ir_executor::run(&p));
        wat_arrays::compare(&p, &ir_executor::run(&p));
        native_arrays::compare(&p, &ir_executor::run(&p));
        for result in [
            codegen::emit_x86_64_win_asm(&p),
            x86_64::emit_asm(&p, Target::X86_64UnknownLinuxGnu),
            x86_64::emit_asm_with_origins(&p, Target::X86_64PcWindowsMsvc),
        ] {
            assert!(!result.unwrap().is_empty());
        }
        for t in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
            assert!(!x86_64::emit_object(&p, t, true).unwrap().is_empty());
        }
    }
}

#[test]
fn examples_and_generated_steps_match_checked_artifacts() {
    success(
        include_str!("../examples/dynamic_arrays/coordinates.ceru"),
        "[[1, 2], [3, 4]]\n[[11, 1], [13, 3]]\n[[99, 1], [13, 3]]\nfalse\n",
    );
    success(
        include_str!("../examples/dynamic_arrays/labels.ceru"),
        "[\"月\", \"火\"]\n[\"予定:月\", \"予定:火\"]\n[\"休み\", \"予定:火\"]\nfalse\n",
    );

    success(
        include_str!("../examples/dynamic_arrays/batches.ceru"),
        "[[99, 20], []]\n[[10, 20], [30]]\n2\n0\n",
    );
    success(
        include_str!("../examples/dynamic_arrays/readings.ceru"),
        "[{valid: false, value: 0}, {valid: true, value: 20}]\n[{valid: true, value: 15}, {valid: true, value: 20}]\n35\n",
    );
    success(
        include_str!("../examples/dynamic_arrays/window.ceru"),
        "[20, 30]\n[10, 20, 30]\n[99, 30]\n[]\n",
    );
    for (source, expected) in [
        (
            include_str!("../examples/dynamic_arrays/copy.ceru"),
            include_str!("fixtures/dynamic-arrays/copy.stdout"),
        ),
        (
            include_str!("../examples/dynamic_arrays/nested.ceru"),
            include_str!("fixtures/dynamic-arrays/nested.stdout"),
        ),
    ] {
        success(source, expected);
    }
    let p = compile_to_ir(include_str!("fixtures/dynamic-arrays/source.ceru")).unwrap();
    assert_eq!(
        cerune_lang::codegen::emit_wat(&p).unwrap(),
        include_str!("fixtures/dynamic-arrays/wat.wat")
    );
    assert_eq!(
        cerune_lang::codegen::llvm::emit_llvm_with_target(
            &p,
            Some(cerune_lang::codegen::llvm::Target::X86_64UnknownLinuxGnu)
        )
        .unwrap(),
        include_str!("fixtures/dynamic-arrays/llvm.ll")
    );
    assert_eq!(
        cerune_lang::codegen::qbe::emit_qbe_with_target(
            &p,
            Some(cerune_lang::codegen::qbe::Target::X86_64UnknownLinuxGnu)
        )
        .unwrap(),
        include_str!("fixtures/dynamic-arrays/qbe.ssa")
    );
    assert_eq!(
        cerune_lang::codegen::emit_c(&p).unwrap(),
        include_str!("fixtures/dynamic-arrays/c.c")
    );
    assert_eq!(
        ir::text::emit(&p),
        include_str!("fixtures/dynamic-arrays/ir.ceir")
    );
    assert_eq!(
        bytecode::format_program(&bytecode::lower(&p).unwrap()),
        include_str!("fixtures/dynamic-arrays/bytecode.cebc")
    );
    success(
        include_str!("fixtures/dynamic-arrays/source.ceru"),
        "[9, 2]\n[1, 2]\n",
    );
}
#[test]
fn temporary_projection_releases_unused_storage_before_next_argument() {
    let source = r#"type P {a:[i64],b:[i64]}
        fn make()->P{return P{a:array_copy([1]),b:array_copy([2,3])};}
        fn take(a:[i64],b:[i64])->void{print(a);print(b);}
        take(make().a,array_copy([4,5]));"#;
    assert_eq!(compare(source, 24).unwrap(), "[1]\n[4, 5]\n");
}
#[test]
fn array_and_string_budgets_are_independent() {
    let mut p = compile_to_ir(r#"a:[string]=array_copy([concat("a","b")]);print(a);"#).unwrap();
    p.array_heap_limit = 16;
    p.string_heap_limit = 2;
    c_arrays::compare(&p, &ir_executor::run(&p));
    llvm_arrays::compare(&p, &ir_executor::run(&p));
    qbe_arrays::compare(&p, &ir_executor::run(&p));
    wat_arrays::compare(&p, &ir_executor::run(&p));
    native_arrays::compare(&p, &ir_executor::run(&p));
    assert_eq!(ir_executor::run(&p).unwrap(), "[\"ab\"]\n");
    assert_eq!(
        run_bytecode(&bytecode::lower(&p).unwrap()).unwrap(),
        "[\"ab\"]\n"
    );
    p.string_heap_limit = 1;
    c_arrays::compare(&p, &ir_executor::run(&p));
    llvm_arrays::compare(&p, &ir_executor::run(&p));
    qbe_arrays::compare(&p, &ir_executor::run(&p));
    wat_arrays::compare(&p, &ir_executor::run(&p));
    native_arrays::compare(&p, &ir_executor::run(&p));
    let e = ir_executor::run(&p).unwrap_err();
    assert_eq!(
        e.runtime_failure().unwrap().code.name(),
        "allocation-limit-exceeded"
    );
    assert_eq!(
        e.runtime_failure(),
        run_bytecode(&bytecode::lower(&p).unwrap())
            .unwrap_err()
            .runtime_failure()
    );
    assert_eq!(
        &r#"a:[string]=array_copy([concat("a","b")]);print(a);"#
            [e.origin().unwrap().span.start()..e.origin().unwrap().span.end()],
        r#"concat("a","b")"#
    );
}
#[test]
fn rejects_invalid_types_calls_recursion_and_constant_allocation() {
    for source in [
        "a:[i64]=[1,2];",
        "a:[i64;2]=array_copy([1,2]);",
        "print(array_copy(1));",
        "print(array_copy());",
        "print(array_copy_range([1],0));",
        "print(array_copy_range([1],0.0,1));",
        "a:[i64]=array_copy([1]);print(a==[1]);",
        "type Recursive {values:[Recursive]}",
        "const A:[i64]=array_copy([1]);",
        "fn array_copy(a:i64)->i64{return a;}",
        "fn array_copy_range(a:i64)->i64{return a;}",
    ] {
        assert!(compile_to_ir(source).is_err(), "{source}");
    }
}

#[path = "support/process.rs"]
mod process;

#[test]
fn cli_examples_limits_modules_and_target_requirements_are_explicit() {
    use std::{
        fs,
        process::Command,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    struct Workspace(std::path::PathBuf);
    impl Drop for Workspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let w = Workspace(std::env::temp_dir().join(format!(
            "cerune-dynamic-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir(&w.0).unwrap();
    let cli = |args: &[&str]| {
        process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune")).args(args),
            &w.0,
            "dynamic-cli",
            Duration::from_secs(30),
        )
        .unwrap()
    };
    for (name, expected) in [
        ("copy", include_str!("fixtures/dynamic-arrays/copy.stdout")),
        (
            "nested",
            include_str!("fixtures/dynamic-arrays/nested.stdout"),
        ),
    ] {
        let file = format!("examples/dynamic_arrays/{name}.ceru");
        for command in ["run", "run-ir", "run-vm"] {
            let result = cli(&[command, &file]);
            assert!(
                result.status.success(),
                "{:?}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, expected.as_bytes());
        }
    }
    let file = "tests/fixtures/dynamic-arrays/source.ceru";
    for command in ["run", "run-vm"] {
        let result = cli(&[
            command,
            file,
            "--array-heap-limit",
            "32",
            "--string-heap-limit",
            "0",
        ]);
        assert!(
            result.status.success(),
            "{:?}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, b"[9, 2]\n[1, 2]\n");
        let result = cli(&[command, file, "--array-heap-limit", "31"]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("allocation-limit-exceeded"));
    }
    for limit in ["32", "31"] {
        let wat = cli(&[
            "emit-wat",
            file,
            "--array-heap-limit",
            limit,
            "--string-heap-limit",
            "0",
        ]);
        assert!(wat.status.success(), "{wat:?}");
        let text = String::from_utf8(wat.stdout).unwrap();
        if let Some(actual) = wat_arrays::execute(&text) {
            let mut p = compile_to_ir(include_str!("fixtures/dynamic-arrays/source.ceru")).unwrap();
            p.array_heap_limit = limit.parse().unwrap();
            match ir_executor::run(&p) {
                Ok(expected) => {
                    assert!(actual.status.success(), "{actual:?}");
                    assert_eq!(actual.stdout, expected.as_bytes());
                    assert!(actual.stderr.is_empty());
                }
                Err(expected) => {
                    assert!(!actual.status.success());
                    assert_eq!(actual.stdout, expected.output().as_bytes());
                    assert_eq!(
                        String::from_utf8(actual.stderr).unwrap(),
                        format!("cerune: {}\n", expected.runtime_failure().unwrap().record())
                    );
                }
            }
        }
    }
    let preserved = w.0.join("missing-llvm-target.ll");
    fs::write(&preserved, b"keep").unwrap();
    let result = cli(&["emit-llvm", file, "-o", preserved.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("call depth checks require an explicit --target")
    );
    assert_eq!(fs::read(&preserved).unwrap(), b"keep");
    let result = cli(&["emit-qbe", file, "-o", preserved.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("call depth checks require an explicit --target")
    );
    assert_eq!(fs::read(&preserved).unwrap(), b"keep");
    for command in ["emit-asm", "emit-obj"] {
        for target in ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"] {
            let out = w.0.join("native-output");
            let result = cli(&[
                command,
                file,
                "-o",
                out.to_str().unwrap(),
                "--target",
                target,
                "--annotate-origins",
            ]);
            assert!(result.status.success(), "{command}: {result:?}");
            assert!(!fs::read(&out).unwrap().is_empty());
        }
    }
    for options in [
        vec!["--array-heap-limit"],
        vec!["--array-heap-limit", "-1"],
        vec!["--array-heap-limit", "--string-heap-limit", "0", "32"],
        vec!["--array-heap-limit", "1.5"],
        vec!["--array-heap-limit", "9223372036854775808"],
        vec!["--array-heap-limit", "1", "--array-heap-limit", "2"],
    ] {
        let mut args = vec!["run", file];
        args.extend(options);
        let result = cli(&args);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("--array-heap-limit"));
    }
    for command in ["check", "emit-sources"] {
        assert!(
            !cli(&[command, file, "--array-heap-limit", "0"])
                .status
                .success()
        );
    }
    fs::write(
        w.0.join("values.ceru"),
        "pub fn make()->[i64]{return array_copy([1,2]);}",
    )
    .unwrap();
    let main = w.0.join("main.ceru");
    fs::write(
        &main,
        "import \"values.ceru\" as values; a:[i64]=values::make();print(a);",
    )
    .unwrap();
    let result = cli(&["run", main.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[1, 2]\n");
    for module_source in [
        "pub fn make()->[i64]{return array_copy([1,2]);}",
        "pub fn make()->[i64]{print(7);return array_copy_range([1],-1,1);}",
    ] {
        fs::write(w.0.join("values.ceru"), module_source).unwrap();
        let p = cerune_lang::modules::load(&main).unwrap().to_ir().unwrap();
        let direct = ir_executor::run(&p);
        let vm = run_bytecode(&bytecode::lower(&p).unwrap());
        match (&direct, &vm) {
            (Ok(a), Ok(b)) => assert_eq!(a, b),
            (Err(a), Err(b)) => {
                assert_eq!(a.runtime_failure(), b.runtime_failure());
                assert_eq!(a.output(), b.vm_error().output());
            }
            _ => panic!("module IR/VM mismatch"),
        }
        c_arrays::compare(&p, &direct);
        llvm_arrays::compare(&p, &direct);
        qbe_arrays::compare(&p, &direct);
        wat_arrays::compare(&p, &direct);
        native_arrays::compare(&p, &direct);
    }
    fs::write(
        w.0.join("values.ceru"),
        "pub fn make()->[i64]{return array_copy([1,2]);}",
    )
    .unwrap();
    let result = cli(&["run-vm", main.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[1, 2]\n");
}
#[test]
fn inactive_payloads_need_no_array_storage() {
    assert_eq!(
        compare("enum E{A{a:[string]},B} a:E=E::B{};b:E=a;print(a==b);", 0).unwrap(),
        "true\n"
    );
}

#[test]
fn partial_nested_copy_failure_and_element_replacement_keep_previous_output() {
    let source =
        r#"a:[[i64]]=array_copy([array_copy([1]),array_copy([2])]);print(a);b:infer=a;print(b);"#;
    failure(source, 88, "allocation-limit-exceeded", "[[1], [2]]\n");
    assert_eq!(compare(source, 96).unwrap(), "[[1], [2]]\n[[1], [2]]\n");
    let source = r#"mut a:[[i64]]=array_copy([array_copy([1])]);keep:[i64]=array_copy([7]);print(a);a[0]=a[0];print(a);"#;
    failure(source, 39, "allocation-limit-exceeded", "[[1]]\n");
    assert_eq!(compare(source, 40).unwrap(), "[[1]]\n[[1]]\n");
}

#[test]
fn c_allocation_boundaries_are_distinct_from_budget_and_empty_storage() {
    let p = compile_to_ir("print(array_copy([1]));").unwrap();
    c_arrays::runtime_boundaries(&p);
}

#[test]
fn llvm_allocation_failures_keep_the_original_operation_and_prior_output() {
    let p = compile_to_ir("print(7); print(array_copy([1]));").unwrap();
    llvm_arrays::runtime_boundaries(&p);
}

#[test]
fn array_memory_limit_is_independent_of_llvm_element_layout() {
    for (ty, value) in [("bool", "true"), ("f32", "1.5")] {
        let source = format!("source:[{ty};1]=[{value}]; a:[{ty}]=array_copy(source);print(a[0]);");
        assert_eq!(compare(&source, 8).unwrap(), format!("{value}\n"));
        failure(&source, 7, "allocation-limit-exceeded", "");
    }
}

#[test]
fn llvm_requires_explicit_array_target_and_preserves_origins() {
    use cerune_lang::codegen::llvm::{self, Options, Target};
    for source in [
        "print(array_copy([1]));",
        "type T{unused:[i64]} print(1);",
        "fn unused(a:[i64])->void{} print(1);",
    ] {
        let p = compile_to_ir(source).unwrap();
        assert!(
            llvm::emit_llvm(&p)
                .unwrap_err()
                .message()
                .contains("dynamic arrays require an explicit --target")
        );
        for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
            let plain = llvm::emit_llvm_with_target(&p, Some(target)).unwrap();
            let annotated = llvm::emit_llvm_with_options(
                &p,
                Options {
                    target: Some(target),
                    annotate_origins: true,
                },
            )
            .unwrap();
            let stripped = annotated
                .lines()
                .filter(|line| !line.trim_start().starts_with("; cerune-origin"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(plain, stripped);
        }
    }
}

#[test]
fn qbe_allocation_failures_keep_the_original_operation_and_prior_output() {
    let p = compile_to_ir("print(7); print(array_copy([1]));").unwrap();
    qbe_arrays::runtime_boundaries(&p);
}

#[test]
fn dynamic_arrays_and_stack_float_parameters_keep_their_types() {
    success(
        r#"
        fn choose(a:i64,b:i64,c:i64,d:i64,value:f32,source:[i64])->[f32] {
            print(source); return array_copy([value]);
        }
        fn pair(a:i64,b:i64,c:i64,value:f64,source:[i64])->[[f64];1] {
            print(source); return [array_copy([value])];
        }
        print(choose(1,2,3,4,1.5,array_copy([7])));
        print(pair(1,2,3,2.5,array_copy([8])));
    "#,
        "[7]\n[1.5]\n[8]\n[[2.5]]\n",
    );
}
#[test]
fn qbe_requires_an_explicit_array_target_even_for_unused_types() {
    use cerune_lang::codegen::qbe::{self, Target};
    for source in [
        "print(array_copy([1]));",
        "type T{unused:[i64]} print(1);",
        "fn unused(a:[i64])->void{} print(1);",
    ] {
        let p = compile_to_ir(source).unwrap();
        assert!(
            qbe::emit_qbe(&p)
                .unwrap_err()
                .message()
                .contains("dynamic arrays require an explicit --target")
        );
        for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
            let before = ir::text::emit(&p);
            assert!(
                qbe::emit_qbe_with_target(&p, Some(target))
                    .unwrap()
                    .contains(target.qbe_name())
            );
            assert_eq!(ir::text::emit(&p), before);
        }
    }
}

#[test]
fn wat_allocation_and_memory_boundaries_keep_original_origins() {
    let p = compile_to_ir("print(7); print(array_copy([1]));").unwrap();
    wat_arrays::runtime_boundaries(&p);
}

#[test]
fn wat_reuses_array_storage_and_string_storage_without_overlap() {
    wat_arrays::reuse_and_growth();
}

#[test]
fn dynamic_index_keeps_all_i64_bits_before_wasm32_addressing() {
    for index in ["-1", "4294967296", "9223372036854775807"] {
        failure(
            &format!("a:[i64]=array_copy([42]);print(7);print(a[{index}]);"),
            ir::DEFAULT_ARRAY_HEAP_LIMIT,
            "array-index-out-of-bounds",
            "7\n",
        );
        failure(
            &format!("mut a:[i64]=array_copy([42]);print(7);a[{index}]=9;"),
            ir::DEFAULT_ARRAY_HEAP_LIMIT,
            "array-index-out-of-bounds",
            "7\n",
        );
    }
}

#[test]
fn mixed_array_assignment_preserves_nested_index_call_temporaries() {
    success(
        r#"
        fn add(a:i64,b:i64)->i64 {print(a);return a+b;}
        mut a:[[i64;2]]=array_copy([[1,2],[3,4]]);
        keep:infer=a;
        a[add(0,add(0,1))][add(0,add(0,1))]=8;
        print(a);print(keep);
        mut b:[[i64];2]=[array_copy([1,2]),array_copy([3,4])];
        b[add(0,add(0,1))][add(0,add(0,1))]=9;
        print(b);
        "#,
        "0\n0\n0\n0\n[[1, 2], [3, 8]]\n[[1, 2], [3, 4]]\n0\n0\n0\n0\n[[1, 2], [3, 9]]\n",
    );
}
#[test]
fn native_origins_and_saved_assembly_match() {
    use cerune_lang::codegen::x86_64::{self, Target};
    let p = compile_to_ir(include_str!("fixtures/dynamic-arrays/source.ceru")).unwrap();
    for (target, expected) in [
        (
            Target::X86_64UnknownLinuxGnu,
            include_str!("fixtures/dynamic-arrays/linux.s"),
        ),
        (
            Target::X86_64PcWindowsMsvc,
            include_str!("fixtures/dynamic-arrays/windows.s"),
        ),
    ] {
        let plain = x86_64::emit_asm(&p, target).unwrap();
        assert_eq!(plain, expected);
        let annotated = x86_64::emit_asm_with_origins(&p, target).unwrap();
        let stripped = annotated
            .lines()
            .filter(|line| !line.starts_with("# cerune-") && !line.starts_with("cerune_origin_"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(plain, stripped);
        assert!(!x86_64::emit_object(&p, target, true).unwrap().is_empty());
    }
}

#[test]
fn lowering_order_example_preserves_short_circuit_loop_edges_and_copy_failure() {
    let source = include_str!("../examples/dynamic_arrays/lowering_order.ceru");
    assert_eq!(compare(source, 48).unwrap(), "begin\n2\n40\n[10, 20, 30]\n");
    let error = compare(source, 47).unwrap_err();
    assert_eq!(error.output(), "begin\n");
    assert_eq!(
        error.runtime_failure().unwrap().code.name(),
        "allocation-limit-exceeded"
    );
    let start = source.find("positive(values, i)").unwrap() + "positive(".len();
    let span = error.origin().unwrap().span;
    assert_eq!((span.start(), span.end()), (start, start + "values".len()));
}

#[test]
fn mir_owned_values_example_matches_all_routes() {
    success(
        include_str!("../examples/ir_stages/owned_values.ceru"),
        "[\"こんにちは\", \"Cerune\"]\n[\"こんにちは！\", \"Cerune\"]\n[\"反復\"]\n",
    );
}

#[test]
fn repeat_evaluates_once_in_order_including_zero() {
    success(
        r#"
        fn seed()->i64 {print("seed");return 7;}
        fn count()->i64 {print("count");return 3;}
        print(array_repeat::<i64>(seed(),count()));
        print(array_repeat::<i64>(seed(),0));
    "#,
        "seed\ncount\n[7, 7, 7]\nseed\n[]\n",
    );
    assert_eq!(
        compare("print(array_repeat::<i64>(1,0));", 0).unwrap(),
        "[]\n"
    );
    failure(
        r#"fn seed()->i64{print("seed");return 1;}
        fn count()->i64{print("count");return -1;}
        print(array_repeat::<i64>(seed(),count()));"#,
        0,
        "array-length-out-of-range",
        "seed\ncount\n",
    );
    failure(
        r#"fn seed()->i64{print("seed");return 1/0;}
        fn count()->i64{print("count");return 0;}
        print(array_repeat::<i64>(seed(),count()));"#,
        0,
        "division-by-zero",
        "seed\n",
    );
    failure(
        r#"print(array_repeat::<i64>(1,1/0));"#,
        0,
        "division-by-zero",
        "",
    );
}
#[test]
fn repeat_nested_values_are_independent() {
    success(
        r#"
        fn copies<T>(seed:T,n:i64)->[T]{return array_repeat::<T>(seed,n);}
        mut seed:[i64]=array_copy([1,2]);
        mut repeated:[[i64]]=array_repeat::<[i64]>(seed,2);
        seed[0]=8; repeated[0][1]=9;
        print(seed);print(repeated);
        saved:infer=repeated; repeated[1][0]=7; print(saved);
        print(copies::<[i64;2]>([3,4],2));
        type P{values:[i64]}
        mut ps:[P]=array_repeat::<P>(P{values:array_copy([5])},2);
        ps[0]=P{values:array_copy([6])};print(ps);
        enum E{A{values:[i64]},B}
        es:[E]=array_repeat::<E>(E::A{values:array_copy([7])},2);
        print(es);print(es==es);
        e:E=E::B{};
        print(array_repeat::<i64>(match e {E::A{values:v}=>v[0],E::B{}=>4},1));
    "#,
        "[8, 2]\n[[1, 9], [1, 2]]\n[[1, 9], [1, 2]]\n[[3, 4], [3, 4]]\n[{values: [6]}, {values: [5]}]\n[A{values: [7]}, A{values: [7]}]\ntrue\n[4]\n",
    );
}
#[test]
fn repeat_budget_origin_and_temporary_release() {
    let source = "print(array_repeat::<i64>(3,-1));";
    let e = compare(source, 0).unwrap_err();
    let span = e.runtime_failure().unwrap().span;
    assert_eq!(
        &source[span.start()..span.end()],
        "array_repeat::<i64>(3,-1)"
    );
    failure(
        "print(array_repeat::<i64>(0,9223372036854775807));",
        0,
        "allocation-size-overflow",
        "",
    );
    failure(
        "print(array_repeat::<i64>(1,2));",
        15,
        "allocation-limit-exceeded",
        "",
    );
    assert_eq!(
        compare("print(array_repeat::<i64>(1,2));", 16).unwrap(),
        "[1, 1]\n"
    );
    let nested = "seed:[i64]=array_copy([1]);print(array_repeat::<[i64]>(seed,2));";
    assert_eq!(compare(nested, 56).unwrap(), "[[1], [1]]\n");
    let e = compare(nested, 55).unwrap_err();
    assert_eq!(
        e.runtime_failure().unwrap().code.name(),
        "allocation-limit-exceeded"
    );
    let span = e.runtime_failure().unwrap().span;
    assert_eq!(
        &nested[span.start()..span.end()],
        "array_repeat::<[i64]>(seed,2)"
    );
    assert_eq!(
        compare(
            "for(mut i:i64=0;i<3;i=i+1){print(array_repeat::<[i64]>(array_copy([1]),0));}",
            8
        )
        .unwrap(),
        "[]\n[]\n[]\n"
    );
    assert_eq!(
        compare(
            "seed:[i64]=array_copy([1]);print(array_repeat::<[i64]>(seed,0));print(seed);",
            8
        )
        .unwrap(),
        "[]\n[1]\n"
    );
}
#[test]
fn repeat_scalar_types_and_exact_strings() {
    for (ty, value) in [
        ("bool", "true"),
        ("i8", "-128"),
        ("i16", "-32768"),
        ("i32", "-2147483648"),
        ("i64", "-9223372036854775808"),
        ("u8", "255"),
        ("u16", "65535"),
        ("u32", "4294967295"),
        ("u64", "18446744073709551615"),
        ("f32", "1.5"),
        ("f64", "2.5"),
    ] {
        success(
            &format!("a:[{ty}]=array_repeat::<{ty}>({value},2);print(a[1]);print(a[0]==a[1]);"),
            &format!("{value}\ntrue\n"),
        );
    }
    success(
        r#"a:[string]=array_repeat::<string>(concat("日","本\0\r\n"),2);
        print(a[0]);print(a[0]==a[1]);print(array_repeat::<string>("e\u{301}",1)[0]=="é");"#,
        "日本\0\r\n\ntrue\nfalse\n",
    );
}
#[test]
fn repeat_requires_explicit_types_and_rejects_invalid_calls() {
    for source in [
        "array_repeat::<i64>(1,2);",
        "print(array_repeat(1,2));",
        "print(array_repeat::<i64>(1));",
        "print(array_repeat::<i64>(1,2,3));",
        "print(array_repeat::<i64,u64>(1,2));",
        "print(array_repeat::<2>(1,2));",
        "print(array_repeat::<bool>(1,2));",
        "n:u64=2;print(array_repeat::<i64>(1,n));",
        "const X:[i64]=array_repeat::<i64>(1,0);",
        "fn array_repeat()->i64{return 1;}",
    ] {
        assert!(compile_to_ir(source).is_err(), "{source}");
    }
}

#[test]
fn repeat_examples_match_all_routes() {
    success(
        include_str!("../examples/dynamic_arrays/repeat.ceru"),
        include_str!("fixtures/dynamic-arrays/repeat.stdout"),
    );
    success(
        include_str!("../examples/dynamic_arrays/repeat_empty.ceru"),
        include_str!("fixtures/dynamic-arrays/repeat-empty.stdout"),
    );
}
#[test]
fn repeat_defaults_const_lengths_short_circuit_and_recursion() {
    success(
        r#"
        const N:i64=2;
        fn repeat_fixed<T,const L:i64>(v:[T;L])->[[T;L]]{
            return array_repeat::<[T;L]>(v,2);
        }
        type P{tag:bool,items:[i64]=array_repeat::<i64>(9,2)}
        p:P=P{tag:true};print(p.items);
        print(repeat_fixed::<i64,N>([1,2]));
        print(false && array_len(array_repeat::<i64>(1,-1))==0);
    "#,
        "[9, 9]\n[[1, 2], [1, 2]]\nfalse\n",
    );
    for source in [
        "fn f()->i64{return array_repeat::<i64>(f(),1)[0];}print(f());",
        "fn f()->i64{return array_repeat::<i64>(1,f())[0];}print(f());",
    ] {
        assert!(compile_to_ir(source).is_err(), "{source}");
    }
}

#[test]
fn repeat_generation_exposes_allocation_copy_loop_and_release() {
    let p = compile_to_ir(include_str!("fixtures/dynamic-arrays/repeat.ceru")).unwrap();
    assert_eq!(
        ir::text::emit(&p),
        include_str!("fixtures/dynamic-arrays/repeat.ceir")
    );
    assert_eq!(
        bytecode::format_program(&bytecode::lower(&p).unwrap()),
        include_str!("fixtures/dynamic-arrays/repeat.cebc")
    );
    let mir = cerune_lang::mir::lower(&p).unwrap();
    cerune_lang::mir::validate(&mir).unwrap();
    for target in [
        cerune_lang::codegen::x86_64::Target::X86_64UnknownLinuxGnu,
        cerune_lang::codegen::x86_64::Target::X86_64PcWindowsMsvc,
    ] {
        let lir = cerune_lang::codegen::x86_64::lower_mir(&mir, target).unwrap();
        cerune_lang::codegen::x86_64::lir::validate(&lir).unwrap();
    }
}

#[test]
fn floating_point_rounding_survives_owned_array_storage_across_routes() {
    success(
        include_str!("../examples/float_rounding.ceru"),
        "0\n0\n0\n0\n-0\n-0\ntrue\ntrue\n",
    );
}
