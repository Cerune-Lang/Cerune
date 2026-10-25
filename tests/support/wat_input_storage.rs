//! WATの容量方針を比較する試作。製品emitterへ組み込みません。
use super::*;

const READ_HOOK: &str = "    (i64.store (local.get $value) (i64.extend_i32_u (local.get $length)))";
const RELEASE_HOOK: &str = "            global.set $cerune_heap_live";

fn variant(plain: &str, mode: &str) -> String {
    assert_eq!(plain.matches(READ_HOOK).count(), 1);
    assert_eq!(plain.matches(RELEASE_HOOK).count(), 1);
    let split = if mode == "baseline" {
        ""
    } else {
        "    (call $trial_split (local.get $value) (local.get $limit) (local.get $length))\n"
    };
    let merge = if mode == "coalesce" {
        "            call $trial_coalesce\n"
    } else {
        ""
    };
    let mut text = plain
        .replacen("(module", "(module\n(import \"probe\" \"storage\" (func $trial_report (param i32 i64 i64 i64 i32 i32 i32 i64)))", 1)
        .replace(READ_HOOK, &format!("{READ_HOOK}\n{split}    (call $trial_snapshot (i32.const 1))"))
        .replace(RELEASE_HOOK, &format!("{RELEASE_HOOK}\n{merge}            i32.const 2\n            call $trial_snapshot"));
    let end = text.rfind(')').unwrap();
    let prototype = PROTOTYPE.replace("@CHECK_COALESCED@", if mode == "coalesce" {
        r#"        (if (i32.and (i32.eq (local.get $phase) (i32.const 2))
          (i32.and (i32.eqz (i32.load offset=4 (local.get $p))) (i32.eqz (i32.load offset=4 (local.get $q)))))
          (then (if (i32.or (i64.eq (local.get $end) (i64.extend_i32_u (local.get $q)))
            (i64.eq (local.get $other_end) (local.get $start))) (then unreachable))))"#
    } else { "" });
    text.insert_str(end, &prototype);
    assert!(!text.contains("(export \"memory\""));
    text
}

fn workload(mixed: bool) -> cerune_lang::ir::Program {
    let mut source = String::from("import \"std:io\" as io; mut cycle:i64=0; while cycle<3 {");
    for i in 0..8 {
        source.push_str(&format!("v{i}:io::ReadLine=io::read_line(1048576);"));
        if mixed {
            source.push_str(&format!("a{i}:[i64]=array_copy([{i},42]);"));
        }
    }
    for i in 0..8 {
        source.push_str(&format!("print(v{i});"));
        if mixed {
            source.push_str(&format!("print(a{i});"));
        }
    }
    source.push_str("cycle=cycle+1;}");
    let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
    hir.string_heap_limit = 1_048_576 + 16;
    hir
}

fn rows(path: &Path) -> Vec<Vec<u64>> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| line.split(',').map(|v| v.parse().unwrap()).collect())
        .collect()
}

#[test]
fn wat_storage_policy_repeated_retention_and_array_gaps() {
    let dir = directory("wat-storage-policy");
    let input = b"a\nb\nc\nd\ne\nf\ng\nh\n".repeat(3);
    for mixed in [false, true] {
        let hir = workload(mixed);
        let expected = expected(&hir, &input);
        let plain = generated_wat(&hir);
        let Some(mut plain_cmd) = wat_command(&dir, &format!("plain-{mixed}"), &plain) else {
            return;
        };
        let plain_trace = dir.join(format!("plain-{mixed}-input.json"));
        plain_cmd.env("CERUNE_TEST_WAT_INPUT_TRACE", &plain_trace);
        compare(file_run(&mut plain_cmd, &dir, &input), &expected);
        let original_trace = fs::read_to_string(plain_trace).unwrap();
        for mode in ["baseline", "split", "coalesce"] {
            let stem = format!("{mode}-{mixed}");
            let Some(mut cmd) = wat_command(&dir, &stem, &variant(&plain, mode)) else {
                return;
            };
            let report = dir.join(format!("{stem}-metrics.csv"));
            let trace = dir.join(format!("{stem}-input.json"));
            cmd.env("CERUNE_TEST_WAT_STORAGE_TRACE", &report)
                .env("CERUNE_TEST_WAT_INPUT_TRACE", &trace);
            compare(file_run(&mut cmd, &dir, &input), &expected);
            let trace = fs::read_to_string(trace).unwrap();
            assert_eq!(trace, original_trace);
            let rows = rows(&report);
            let lines: Vec<_> = rows.iter().filter(|r| r[0] == 1).collect();
            let released: Vec<_> = rows.iter().filter(|r| r[0] == 2 && r[4] == 0).collect();
            assert_eq!(lines.len(), 24);
            assert_eq!(released.len(), 3);
            for (i, r) in lines.iter().enumerate() {
                let owners = (i % 8 + 1) as u64;
                assert_eq!(r[1], owners * 2);
                assert_eq!(r[4], owners);
                assert_eq!(
                    r[2],
                    owners * if mode == "baseline" { 1_048_576 } else { 8 }
                );
            }
            for r in &released {
                assert_eq!((r[1], r[2]), (0, 0));
            }
            let pages: Vec<_> = released.iter().map(|r| r[5]).collect();
            if mode == "split" {
                assert!(pages[0] < pages[1] && pages[1] < pages[2]);
            } else {
                assert_eq!(pages, vec![pages[0]; 3]);
            }
            // この固定規模では、再結合も初回の大きな予約8本を減らせません。
            assert_eq!(pages[0], 129);
            // A retained frame block separates the first string from the other seven.
            // Coalescing must not absorb a block owned by the frame allocator.
            if !mixed && mode == "coalesce" {
                assert!(released.iter().all(|r| r[6] == 2
                    && r[3] == 8 * 1_048_576 + 6 * 24
                    && r[7] == 7 * (1_048_576 + 24) - 24));
            }
            if mixed && mode == "coalesce" {
                assert!(
                    released
                        .iter()
                        .all(|r| r[6] == 8 && r[3] == 8 * 1_048_576 && r[7] == 1_048_576)
                );
            }
            println!("{stem}: pages={pages:?}, final={:?}", rows.last().unwrap());
        }
    }
}

#[test]
fn wat_storage_policy_examples_failures_and_fixed_page_limit() {
    let dir = directory("wat-storage-examples");
    let source = include_str!("../../examples/stdin/retained_lines.ceru")
        .replace("read_line(64)", "read_line(1048576)");
    let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
    hir.string_heap_limit = 1_048_576 + 16;
    let plain = generated_wat(&hir);
    for (case, input) in [
        &b"a\nb\n"[..],
        "日本\0\r\ne\u{301}".as_bytes(),
        b"x\n",
        b"",
        b"x\n\xff\n",
    ]
    .iter()
    .enumerate()
    {
        let mut original = None;
        for mode in ["baseline", "split", "coalesce"] {
            let stem = format!("{mode}-{case}");
            let Some(mut cmd) = wat_command(&dir, &stem, &variant(&plain, mode)) else {
                return;
            };
            let trace = dir.join(format!("{stem}.json"));
            cmd.env(
                "CERUNE_TEST_WAT_STORAGE_TRACE",
                dir.join(format!("{stem}-metrics.csv")),
            )
            .env("CERUNE_TEST_WAT_INPUT_TRACE", &trace);
            compare(file_run(&mut cmd, &dir, input), &expected(&hir, input));
            let current = fs::read_to_string(trace).unwrap();
            if let Some(previous) = &original {
                assert_eq!(&current, previous);
            } else {
                original = Some(current);
            }
        }
    }
    // ページ上限で2回目の予約が失敗する位置・出力・消費数も変わりません。
    let mut original = None;
    for mode in ["baseline", "split", "coalesce"] {
        let text = variant(&plain, mode).replace("(memory 1)", "(memory 1 17)");
        assert!(text.contains("(memory 1 17)"));
        let Some(mut cmd) = wat_command(&dir, &format!("limited-{mode}"), &text) else {
            return;
        };
        let trace = dir.join(format!("limited-{mode}.json"));
        cmd.env(
            "CERUNE_TEST_WAT_STORAGE_TRACE",
            dir.join(format!("limited-{mode}-metrics.csv")),
        )
        .env("CERUNE_TEST_WAT_INPUT_TRACE", &trace);
        let out = file_run(&mut cmd, &dir, b"a\nb\n");
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("code=allocation-failed"));
        let trace = fs::read_to_string(trace).unwrap();
        assert!(trace.contains("\"consumed\":2") && trace.contains("\"reads\":2"));
        let result = (out.stdout, out.stderr, trace);
        if let Some(previous) = &original {
            assert_eq!(&result, previous);
        } else {
            original = Some(result);
        }
    }
}

// 24-byte headers: next, references, capacity, length. Addresses remain private.
// Coalescing deliberately scans the whole string list: it is an experiment,
// not a production allocator or a claim about allocation-time complexity.
const PROTOTYPE: &str = r#"
  (func $trial_split (param $value i32) (param $limit i64) (param $length i32)
    (local $p i32) (local $tail i32) (local $capacity i64) (local $keep i64)
    (if (i32.or
      (i64.gt_u (i64.extend_i32_u (local.get $length)) (i64.div_u (local.get $limit) (i64.const 2)))
      (i64.lt_u (i64.sub (local.get $limit) (i64.extend_i32_u (local.get $length))) (i64.const 4096)))
      (then return))
    (local.set $p (i32.sub (local.get $value) (i32.const 16)))
    (local.set $capacity (i64.load offset=8 (local.get $p)))
    (local.set $keep (i64.and (i64.add (i64.extend_i32_u (local.get $length)) (i64.const 7)) (i64.const -8)))
    (if (i64.lt_u (i64.sub (local.get $capacity) (local.get $keep)) (i64.const 32)) (then return))
    (local.set $tail (i32.add (local.get $p) (i32.wrap_i64 (i64.add (local.get $keep) (i64.const 24)))))
    (i32.store (local.get $tail) (global.get $cerune_heap_head))
    (i32.store offset=4 (local.get $tail) (i32.const 0))
    (i64.store offset=8 (local.get $tail) (i64.sub (i64.sub (local.get $capacity) (local.get $keep)) (i64.const 24)))
    (i64.store offset=16 (local.get $tail) (i64.const 0))
    (i64.store offset=8 (local.get $p) (local.get $keep))
    (global.set $cerune_heap_head (local.get $tail)))
  (func $trial_coalesce
    (local $a i32) (local $b i32) (local $previous i32) (local $end i64)
    (loop $restart
      (local.set $a (global.get $cerune_heap_head))
      (block $done (loop $outer
        (br_if $done (i32.eqz (local.get $a)))
        (if (i32.eqz (i32.load offset=4 (local.get $a))) (then
          (local.set $end (i64.add (i64.extend_i32_u (local.get $a)) (i64.add (i64.const 24) (i64.load offset=8 (local.get $a)))))
          (local.set $b (global.get $cerune_heap_head))
          (local.set $previous (i32.const 0))
          (block $searched (loop $inner
            (br_if $searched (i32.eqz (local.get $b)))
            (if (i32.and (i32.eqz (i32.load offset=4 (local.get $b)))
                  (i64.eq (local.get $end) (i64.extend_i32_u (local.get $b)))) (then
              (i64.store offset=8 (local.get $a)
                (i64.add (i64.load offset=8 (local.get $a)) (i64.add (i64.const 24) (i64.load offset=8 (local.get $b)))))
              (if (local.get $previous)
                (then (i32.store (local.get $previous) (i32.load (local.get $b))))
                (else (global.set $cerune_heap_head (i32.load (local.get $b)))))
              (br $restart)))
            (local.set $previous (local.get $b))
            (local.set $b (i32.load (local.get $b)))
            (br $inner)))))
        (local.set $a (i32.load (local.get $a)))
        (br $outer)))))
  (func $trial_snapshot (param $phase i32)
    (local $p i32) (local $q i32) (local $steps i32) (local $other_steps i32)
    (local $owners i32) (local $capacity i64) (local $free i64) (local $maximum i64) (local $live i64)
    (local $start i64) (local $end i64) (local $other_end i64)
    (local.set $p (global.get $cerune_heap_head))
    (block $done (loop $next
      (br_if $done (i32.eqz (local.get $p)))
      (local.set $steps (i32.add (local.get $steps) (i32.const 1)))
      (if (i32.gt_u (local.get $steps) (i32.const 1024)) (then unreachable))
      (local.set $start (i64.extend_i32_u (local.get $p)))
      (local.set $end (i64.add (local.get $start) (i64.add (i64.const 24) (i64.load offset=8 (local.get $p)))))
      (if (i32.or (i64.gt_u (local.get $end) (global.get $cerune_heap_end))
        (i64.ne (i64.and (local.get $end) (i64.const 7)) (i64.const 0))) (then unreachable))
      (if (i32.and (local.get $p) (i32.const 7)) (then unreachable))
      (local.set $q (i32.load (local.get $p)))
      (local.set $other_steps (i32.const 0))
      (block $compared (loop $pair
        (br_if $compared (i32.eqz (local.get $q)))
        (local.set $other_steps (i32.add (local.get $other_steps) (i32.const 1)))
        (if (i32.gt_u (local.get $other_steps) (i32.const 1024)) (then unreachable))
        (local.set $other_end (i64.add (i64.extend_i32_u (local.get $q)) (i64.add (i64.const 24) (i64.load offset=8 (local.get $q)))))
        (if (i32.and (i64.lt_u (local.get $start) (local.get $other_end))
          (i64.lt_u (i64.extend_i32_u (local.get $q)) (local.get $end))) (then unreachable))
@CHECK_COALESCED@
        (local.set $q (i32.load (local.get $q)))
        (br $pair)))
      (if (i32.load offset=4 (local.get $p))
        (then
          (if (i64.gt_u (i64.load offset=16 (local.get $p)) (i64.load offset=8 (local.get $p))) (then unreachable))
          (local.set $live (i64.add (local.get $live) (i64.load offset=16 (local.get $p))))
          (local.set $owners (i32.add (local.get $owners) (i32.const 1)))
          (local.set $capacity (i64.add (local.get $capacity) (i64.load offset=8 (local.get $p)))))
        (else
          (local.set $free (i64.add (local.get $free) (i64.load offset=8 (local.get $p))))
          (local.set $maximum (select (i64.load offset=8 (local.get $p)) (local.get $maximum)
            (i64.gt_u (i64.load offset=8 (local.get $p)) (local.get $maximum))))))
      (local.set $p (i32.load (local.get $p)))
      (br $next)))
    (if (i64.ne (local.get $live) (global.get $cerune_heap_live)) (then unreachable))
    (call $trial_report (local.get $phase) (local.get $live) (local.get $capacity) (local.get $free)
      (local.get $owners) (memory.size) (local.get $steps) (local.get $maximum)))
"#;
