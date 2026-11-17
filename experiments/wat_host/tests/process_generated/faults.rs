use super::*;
use wasmi::Caller;
fn fault(config: &[u8], install: impl FnOnce(&mut Linker<host::Process>)) -> String {
    let engine = Engine::default();
    let module = Module::new(&engine, wat::emit_wat(&hir(EXAMPLE)).unwrap()).unwrap();
    let mut linker = Linker::new(&engine);
    host::imports(&mut linker).unwrap();
    linker.allow_shadowing(true);
    let out = Arc::new(Mutex::new(vec![]));
    let err = Arc::new(Mutex::new(vec![]));
    output_imports(&mut linker, out.clone(), err.clone());
    install(&mut linker);
    let mut store = Store::new(&engine, host::Process::new(target(), config).unwrap());
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    assert!(
        instance
            .get_typed_func::<(), ()>(&store, "main")
            .unwrap()
            .call(&mut store, ())
            .is_err()
    );
    assert_eq!(*out.lock().unwrap(), b"before\n");
    let text = String::from_utf8(err.lock().unwrap().clone()).unwrap();
    assert!(text.starts_with("cerune-process-abi-v1 status="), "{text}");
    assert!(
        text.contains(" node=") && text.contains(" bytes="),
        "{text}"
    );
    assert!(!text.contains("tool") && !text.contains("echo"), "{text}");
    text
}
#[test]
fn malformed_report_stops_before_following_output() {
    // Version, word count, origins, enums, counts, reserved words and invalid payload combinations.
    for (word, value) in [
        (0, 2),
        (1, 31),
        (2, -1),
        (3, -1),
        (4, 999999),
        (5, 0),
        (6, 3),
        (7, 0),
        (8, 5),
        (9, 33),
        (10, 33),
        (11, 0),
        (14, 1),
        (15, 1),
        (16, 2),
        (17, 3),
        (18, 1),
        (19, 22),
        (20, 0),
        (21, 1),
        (22, 3),
        (23, 1),
        (24, 1),
        (25, 3),
        (26, 1),
        (27, 1),
        (28, 1),
        (29, 1),
        (30, 1),
        (31, 1),
    ] {
        let error = fault(&[], |l| {
            l.func_wrap(
                "cerune",
                "process_report_v1",
                move |c: Caller<'_, host::Process>, i: i32| {
                    let (v, status) = c.data().report(i);
                    (if i == word { value } else { v }, status)
                },
            )
            .unwrap();
        });
        assert!(error.contains("status=13 "), "word {word}: {error}");
    }
    for fields in [
        [(19, 0), (20, 0), (21, 1)],
        [(22, 1), (23, 2), (24, 0)],
        [(22, 2), (23, 1), (24, 1)],
        [(25, 2), (26, 14), (27, 6)],
    ] {
        fault(&[], |l| {
            l.func_wrap(
                "cerune",
                "process_report_v1",
                move |c: Caller<'_, host::Process>, i: i32| {
                    let (v, status) = c.data().report(i);
                    (
                        fields.iter().find(|(w, _)| *w == i).map_or(v, |(_, v)| *v),
                        status,
                    )
                },
            )
            .unwrap();
        });
    }
}
#[test]
fn transport_failures_and_copy_responses_are_not_silently_accepted() {
    for status in [1, 2, 12, 17] {
        fault(&[], |l| {
            l.func_wrap("cerune", "process_chunk_v1", move |_: i64, _: i32| status)
                .unwrap();
        });
    }
    fault(&[], |l| {
        l.func_wrap("cerune", "process_prepare_v1", || 0i32)
            .unwrap();
    });
    fault(&[], |l| {
        l.func_wrap("cerune", "process_report_v1", |_: i32| (0i64, 17i32))
            .unwrap();
    });
    fault(&[], |l| {
        l.func_wrap("cerune", "process_release_v1", || 12i32)
            .unwrap();
    });
    let config = serde_json::to_vec(&configuration()).unwrap();
    for (bits, copied, status) in [(0i64, 0, 0), (0, 2, 0), (256, 1, 0), (0, 1, 17)] {
        fault(&config, |l| {
            // An injected one-byte result fits the pre-reserved array, but copying violates v1.
            l.func_wrap(
                "cerune",
                "process_report_v1",
                |c: Caller<'_, host::Process>, i: i32| {
                    let (v, s) = c.data().report(i);
                    (if i == 12 || i == 14 { 1 } else { v }, s)
                },
            )
            .unwrap();
            l.func_wrap(
                "cerune",
                "process_copy_v1",
                move |_: i32, _: i64, _: i32| (bits, copied, status),
            )
            .unwrap();
        });
    }
}
#[test]
fn no_process_imports_for_ordinary_programs_and_old_host_rejection_for_unused_calls() {
    let text = wat::emit_wat(&hir("print(1);")).unwrap();
    assert!(!text.contains("process_"));
    let source = r#"import "std:process" as p; import "std:process::run" as r;
        fn unused(request:p::Request)->p::Outcome { return r::run(request); } print(1);"#;
    let text = wat::emit_wat(&hir(source)).unwrap();
    let engine = Engine::default();
    let module = Module::new(&engine, text).unwrap();
    let mut linker = Linker::new(&engine);
    output_imports(
        &mut linker,
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut store = Store::new(&engine, host::Process::new(target(), &[]).unwrap());
    let error = linker
        .instantiate_and_start(&mut store, &module)
        .unwrap_err()
        .to_string();
    assert!(error.contains("process_begin_v1"), "{error}");
}
#[test]
fn injected_cleanup_payloads_keep_flags_and_reported_verdict_separate() {
    // This checks generated enum conversion, not an OS cleanup failure.
    let source =
        format!("{EXAMPLE}\nprint(result.report.cleanup);print(result.report.cleanup_reported);");
    let engine = Engine::default();
    let module = Module::new(&engine, wat::emit_wat(&hir(&source)).unwrap()).unwrap();
    let mut linker = Linker::new(&engine);
    host::imports(&mut linker).unwrap();
    linker.allow_shadowing(true);
    linker
        .func_wrap(
            "cerune",
            "process_report_v1",
            |c: Caller<'_, host::Process>, i: i32| {
                let (word, status) = c.data().report(i);
                let replacement = match i {
                    19 => 20,
                    20 => 7,
                    22 => 2,
                    23 => 14,
                    24 => 5,
                    25 => 1,
                    26 => 1,
                    _ => word,
                };
                (replacement, status)
            },
        )
        .unwrap();
    let out = Arc::new(Mutex::new(vec![]));
    let err = Arc::new(Mutex::new(vec![]));
    output_imports(&mut linker, out.clone(), err.clone());
    let mut store = Store::new(&engine, host::Process::new(target(), &[]).unwrap());
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    instance
        .get_typed_func::<(), ()>(&store, "main")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("Reason{code: CleanupFailed{}, phase: Cleanup{}}"),
        "{text}"
    );
    assert!(text.ends_with("Incomplete{child_reaped: true, handles_closed: true, scope_finished: true, error: InvalidResponse{}}\nComplete{termination_requested: true}\n"),"{text}");
    assert!(err.lock().unwrap().is_empty());
}
