use super::*;

// 測定用mainから生成Cの入力・所有管理を呼び、コンパイル時のメモリを除外します。
#[test]
fn generated_c_retained_input_capacity_is_distinct_from_logical_budget() {
    let dir = directory("input-memory");
    for limit in [2, 1_048_576] {
        let source = format!("import \"std:io\" as io; value:io::ReadLine=io::read_line({limit});");
        let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
        hir.string_heap_limit = limit as u64 + 16;
        let generated = c::emit_c(&hir).unwrap();
        assert_eq!(generated.matches("int main(void)").count(), 1);
        let generated = generated.replace("int main(void)", "int cerune_unused_main(void)");
        // 標準ヘッダーの宣言は書き換えず、生成runtimeの呼出しだけを追跡します。
        let generated = generated
            .replace("malloc(", "probe_malloc(")
            .replace("free(", "probe_free(");
        let probe = include_str!("input_memory.c").replace("@LIMIT@", &limit.to_string());
        let (prefix, suffix) = probe.split_once("/* GENERATED_RUNTIME */").unwrap();
        for opt in ["-O0", "-O2"] {
            let text = format!("{prefix}\n{generated}\n{suffix}");
            let Some(exe) = compile_c(&dir, &format!("capacity-{limit}-{opt}"), &text, opt) else {
                return;
            };
            let out = file_run(&mut Command::new(exe), &dir, b"x\n".repeat(24).as_slice());
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                out.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let report = String::from_utf8(out.stdout).unwrap();
            // OS統計は参考値です。allocatorへの要求、所有数、論理課金を検証します。
            assert_eq!(report.matches("phase=retained").count(), 3);
            assert_eq!(report.matches("phase=released").count(), 3);
            assert_eq!(report.matches("logical=16 owners=8").count(), 3);
            assert_eq!(report.matches("logical=0 owners=0 requested=0").count(), 4);
            for line in report
                .lines()
                .filter(|line| line.contains("phase=retained"))
            {
                assert!(line.contains(&format!("requested={} ", 8 * (limit + 24))));
            }
            println!("C {opt} limit={limit}\n{report}");
        }
    }
}

#[test]
fn retained_lines_example_matches_routes_with_small_and_large_limits() {
    let dir = directory("retained-lines");
    for limit in [64, 1_048_576] {
        let source = include_str!("../../examples/stdin/retained_lines.ceru")
            .replace("read_line(64)", &format!("read_line({limit})"));
        let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
        // 1行目の実長だけが次の読取りの予算から差し引かれることも確認します。
        hir.string_heap_limit = limit as u64 + 2;
        let input = b"x\ny\n";
        let expected = expected(&hir, input);
        assert_eq!(expected.0, b"[\"x\\n\"]\n[\"y\\n\"]\n");
        let source_file = dir.join("retained.ceru");
        fs::write(&source_file, &source).unwrap();
        for route in ROUTES {
            compare(
                file_run(
                    &mut cli(route, &source_file, hir.string_heap_limit),
                    &dir,
                    input,
                ),
                &expected,
            );
        }
        let stem = format!("retained-{limit}");
        if let Some(exe) = compile_c(&dir, &stem, &c::emit_c(&hir).unwrap(), "-O2") {
            compare(file_run(&mut Command::new(exe), &dir, input), &expected);
        }
        if let Some(exe) = compile_llvm(&dir, &stem, &generated_llvm(&hir, true), "-O2") {
            compare(file_run(&mut Command::new(exe), &dir, input), &expected);
        }
        if let Some(exe) = compile_qbe(&dir, &stem, &generated_qbe(&hir)) {
            compare(file_run(&mut Command::new(exe), &dir, input), &expected);
        }
        if let Some(mut cmd) = wat_command(&dir, &stem, &generated_wat(&hir)) {
            compare(file_run(&mut cmd, &dir, input), &expected);
        }
        for exe in native_commands(&dir, &stem, &hir) {
            compare(file_run(&mut Command::new(exe), &dir, input), &expected);
        }
    }
}
