use super::*;
#[cfg(all(target_arch = "x86_64", any(windows, target_os = "linux")))]
use cerune_lang::codegen::x86_64 as native;

fn workload(limit: usize) -> cerune_lang::ir::Program {
    let mut source = String::from("import \"std:io\" as io; mut cycle:i64=0; while cycle<3 {");
    for i in 0..8 {
        source.push_str(&format!("v{i}:io::ReadLine=io::read_line({limit});"));
    }
    for i in 0..8 {
        source.push_str(&format!("print(v{i});"));
    }
    source.push_str("cycle=cycle+1;}");
    let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
    hir.string_heap_limit = limit as u64 + 16;
    hir
}
#[cfg(all(target_arch = "x86_64", any(windows, target_os = "linux")))]
fn checked(cmd: &mut Command, dir: &Path, label: &str) {
    let out = process::bounded_output(cmd, dir, label, Duration::from_secs(30)).unwrap();
    assert!(
        out.status.success(),
        "{label}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[cfg(all(target_arch = "x86_64", any(windows, target_os = "linux")))]
fn instrument(
    dir: &Path,
    stem: &str,
    artifact: &Path,
    cc: &std::ffi::OsStr,
    objcopy: &std::ffi::OsStr,
    probe: &Path,
) -> PathBuf {
    let object = if matches!(
        artifact.extension().and_then(|x| x.to_str()),
        Some("o" | "obj")
    ) {
        artifact.to_owned()
    } else {
        let object = dir.join(format!("{stem}.o"));
        checked(
            Command::new(cc)
                .arg("-c")
                .arg("-O0")
                .arg(artifact)
                .arg("-o")
                .arg(&object),
            dir,
            "probe compile",
        );
        object
    };
    let measured = dir.join(format!("{stem}-measured.o"));
    // 再配置情報を保ったまま、生成物側の参照だけを付け替えます。
    checked(
        Command::new(objcopy)
            .args([
                "--redefine-sym",
                "malloc=probe_malloc",
                "--redefine-sym",
                "free=probe_free",
                "--redefine-sym",
                "realloc=probe_realloc",
            ])
            .arg(&object)
            .arg(&measured),
        dir,
        "probe symbols",
    );
    let exe = dir.join(format!("{stem}-measured{}", std::env::consts::EXE_SUFFIX));
    let mut link = Command::new(compiler(false).expect("C linker already checked"));
    link.arg(measured).arg(probe).arg("-o").arg(&exe);
    if cfg!(unix) {
        link.arg("-lm");
    }
    checked(&mut link, dir, "probe link");
    exe
}

#[test]
#[cfg(all(target_arch = "x86_64", any(windows, target_os = "linux")))]
fn compiled_routes_retained_input_capacity() {
    let configured = std::env::var_os("CERUNE_TEST_OBJCOPY");
    let objcopy = configured.clone().unwrap_or_else(|| {
        if cfg!(windows) {
            "llvm-objcopy"
        } else {
            "objcopy"
        }
        .into()
    });
    if !Command::new(&objcopy)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(configured.is_none(), "configured objcopy unavailable");
        eprintln!("input capacity checks skipped: objcopy unavailable");
        return;
    }
    let Some(cc) = compiler(false) else { return };
    let dir = directory("input-memory-routes");
    let probe = dir.join("probe.c");
    fs::write(&probe, include_str!("input_memory_probe.c")).unwrap();
    for (limit, length) in [
        (2, 2),
        (4097, 2),
        (4098, 2),
        (8192, 4096),
        (8193, 4097),
        (1_048_576, 2),
    ] {
        let mut line = vec![b'x'; length];
        if length > 16 {
            line[..8].copy_from_slice("日本\0\r".as_bytes());
        }
        line[length - 1] = b'\n';
        let input: Vec<u8> = (0..24)
            .flat_map(|index| {
                let mut distinct = line.clone();
                distinct[length - 2] = b'a' + (index % 8) as u8;
                distinct
            })
            .collect();
        let mut hir = workload(limit);
        hir.string_heap_limit = limit as u64 + (length * 8) as u64;
        let expected = expected(&hir, &input);
        let mut artifacts = Vec::new();
        let stem = format!("c-{limit}");
        if let Some(exe) = compile_c(&dir, &stem, &c::emit_c(&hir).unwrap(), "-O0") {
            artifacts.push((stem.clone(), exe, dir.join(format!("{stem}.c")), cc.clone()));
        }
        let stem = format!("llvm-{limit}");
        if let Some(exe) = compile_llvm(&dir, &stem, &generated_llvm(&hir, true), "-O0") {
            artifacts.push((
                stem.clone(),
                exe,
                dir.join(format!("{stem}-llvm.ll")),
                compiler(true).unwrap(),
            ));
        }
        let stem = format!("qbe-{limit}");
        if let Some(exe) = compile_qbe(&dir, &stem, &generated_qbe(&hir)) {
            artifacts.push((
                stem.clone(),
                exe,
                dir.join(format!("{stem}-qbe.s")),
                cc.clone(),
            ));
        }
        let mir = cerune_lang::mir::lower(&hir).unwrap();
        let ssa = cerune_lang::mir::ssa::lower(&cerune_lang::mir::ssa::construct(&mir).unwrap())
            .unwrap()
            .program;
        for (stage, program) in [("mir", &mir), ("ssa", &ssa)] {
            let lir = native::lower_mir(program, native_target()).unwrap();
            for (kind, bytes, ext) in [
                (
                    "asm",
                    native::emit_asm_from_lir(&lir, true).unwrap().into_bytes(),
                    "s",
                ),
                (
                    "obj",
                    native::emit_object_from_lir(&lir, true).unwrap(),
                    if cfg!(windows) { "obj" } else { "o" },
                ),
            ] {
                let stem = format!("{stage}-{kind}-{limit}");
                if let Some(exe) = native_link(&dir, &stem, &bytes, ext) {
                    artifacts.push((
                        stem.clone(),
                        exe,
                        dir.join(format!("{stem}.{ext}")),
                        cc.clone(),
                    ));
                }
            }
        }
        for (stem, plain, artifact, tool) in artifacts {
            compare(file_run(&mut Command::new(plain), &dir, &input), &expected);
            let exe = instrument(&dir, &stem, &artifact, &tool, &objcopy, &probe);
            for mode in ["normal", "fail", "move"] {
                let out = file_run(
                    Command::new(&exe).env("CERUNE_TEST_SHRINK", mode),
                    &dir,
                    &input,
                );
                assert!(
                    out.status.success(),
                    "{stem}/{mode}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                assert_eq!(out.stdout, expected.0, "{stem}/{mode}");
                let report = String::from_utf8(out.stderr).unwrap();
                let eligible = length <= limit / 2 && limit - length >= 4096;
                let compact = eligible && mode != "fail";
                let peak = if compact {
                    limit + 24 + 7 * (length + 24)
                } else {
                    8 * (limit + 24)
                };
                assert_eq!(
                    report.lines().last().unwrap(),
                    format!(
                        "probe allocations=24 releases=24 live=0 peak={peak} owners=0 peak_owners=8"
                    ),
                    "{stem}/{mode}: {report}"
                );
                if eligible {
                    let successes = if compact { 24 } else { 0 };
                    assert!(
                        report.contains(&format!("shrink calls=24 successes={successes} ")),
                        "{stem}/{mode}: {report}"
                    );
                    if mode == "move" {
                        assert!(
                            report.contains(&format!("moved_copy_peak={}", peak + length + 24))
                        );
                    }
                } else {
                    assert!(!report.contains("shrink calls="));
                }
                fs::write(dir.join(format!("{stem}-{mode}-metrics.txt")), &report).unwrap();
                println!("{stem}/{mode}\n{report}");
            }
        }
    }
}

const WAT_PROBE: &str = r#"
  (func $probe_memory (param $phase i32)
    (local $p i32) (local $owners i32) (local $capacity i64) (local $free i64)
    (local.set $p (global.get $cerune_heap_head))
    (block $done (loop $next
      (br_if $done (i32.eqz (local.get $p)))
      (if (i32.load offset=4 (local.get $p))
        (then
          (local.set $owners (i32.add (local.get $owners) (i32.const 1)))
          (local.set $capacity (i64.add (local.get $capacity) (i64.load offset=8 (local.get $p)))))
        (else (local.set $free (i64.add (local.get $free) (i64.load offset=8 (local.get $p))))))
      (local.set $p (i32.load (local.get $p)))
      (br $next)))
    (call $probe_memory_report (local.get $phase) (global.get $cerune_heap_live)
      (local.get $capacity) (local.get $free) (local.get $owners) (memory.size)))
"#;
#[test]
fn wat_retained_input_capacity_and_free_storage() {
    let dir = directory("input-memory-wat");
    let input = b"x\n".repeat(24);
    for limit in [2, 1_048_576] {
        let hir = workload(limit);
        let expected = expected(&hir, &input);
        let plain = generated_wat(&hir);
        assert!(!plain.contains("(export \"memory\""));
        let Some(mut plain_cmd) = wat_command(&dir, &format!("plain-{limit}"), &plain) else {
            return;
        };
        compare(file_run(&mut plain_cmd, &dir, &input), &expected);
        let mut text = plain.replacen("(module","(module\n(import \"probe\" \"memory\" (func $probe_memory_report (param i32 i64 i64 i64 i32 i32)))",1);
        let end = text.rfind(')').unwrap();
        text.insert_str(end, WAT_PROBE);
        let read_hook = "    (i64.store (local.get $value) (i64.extend_i32_u (local.get $length)))";
        assert_eq!(text.matches(read_hook).count(), 1);
        text = text.replace(
            read_hook,
            &format!("{read_hook}\n    (call $probe_memory (i32.const 1))"),
        );
        let release_hook = "            global.set $cerune_heap_live";
        assert_eq!(text.matches(release_hook).count(), 1);
        text = text.replace(
            release_hook,
            &format!("{release_hook}\n            i32.const 2\n            call $probe_memory"),
        );
        let Some(mut cmd) = wat_command(&dir, &format!("measured-{limit}"), &text) else {
            panic!()
        };
        let report = dir.join(format!("wat-{limit}-metrics.json"));
        cmd.env("CERUNE_TEST_WAT_MEMORY_TRACE", &report);
        compare(file_run(&mut cmd, &dir, &input), &expected);
        // host側で全記録・最後の解放・memory再利用を検証し、結果だけを保存します。
        assert!(report.is_file());
        println!("WAT limit={limit} {}", fs::read_to_string(report).unwrap());
    }
}
