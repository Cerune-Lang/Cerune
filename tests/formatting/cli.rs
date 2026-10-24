use super::*;
use cerune_lang::format::{LetStyle, format};

#[test]
fn formatter_cli_defaults_to_stdout_and_requires_explicit_file_writes() {
    let w = Workspace::new();
    let source = "// keep\r\nmut x:i64=1;print(x);\r\n";
    let input = w.put("format.ceru", source);
    let text = format(source, LetStyle::Show).unwrap();
    let out = w.cli("fmt", &input, &["--let", "show"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, text.as_bytes());
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let check = w.cli("fmt", &input, &["--check", "--let", "show"]);
    assert_eq!(check.status.code(), Some(1));
    assert!(check.stdout.is_empty());
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let output = w.put("other.ceru", "previous");
    assert!(
        w.cli(
            "fmt",
            &input,
            &["--let", "show", "-o", output.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert_eq!(fs::read_to_string(output).unwrap(), text);
    assert!(
        w.cli("fmt", &input, &["--let", "show", "--write"])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), text);
    assert!(
        w.cli("fmt", &input, &["--let", "show", "--check"])
            .status
            .success()
    );
    assert!(w.cli("run", &input, &[]).status.success());
    assert!(
        !fs::read_dir(&w.0).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp"))
    );
}

#[test]
fn formatter_errors_preserve_input_and_output_and_do_not_load_imports() {
    let w = Workspace::new();
    let output = w.put("saved.ceru", "keep");
    let input = w.put("source.ceru", "let missing=1;");
    for args in [
        vec!["--write"],
        vec!["-o", output.to_str().unwrap()],
        vec!["--let", "bad"],
        vec!["--write", "--check"],
        vec!["--let", "show", "--let", "hide"],
    ] {
        let out = w.cli("fmt", &input, &args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert_eq!(fs::read_to_string(&input).unwrap(), "let missing=1;");
        assert_eq!(fs::read_to_string(&output).unwrap(), "keep");
    }
    fs::write(&input, [255, 0]).unwrap();
    assert!(
        !w.cli("fmt", &input, &["-o", output.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(fs::read(&input).unwrap(), [255, 0]);
    assert_eq!(fs::read_to_string(&output).unwrap(), "keep");
    fs::write(
        &input,
        " ".repeat(cerune_lang::modules::LoadLimits::default().max_file_bytes + 1),
    )
    .unwrap();
    assert!(!w.cli("fmt", &input, &["--write"]).status.success());
    let source = "import \"not-downloaded.ceru\" as other; pub fn f()->i64{return other::value();}";
    fs::write(&input, source).unwrap();
    let out = w.cli("fmt", &input, &[]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
}

#[test]
fn formatted_failures_refer_to_their_own_source_positions() {
    let w = Workspace::new();
    let before = "// original\nlet x: i64=0; print(7);print(1/x);print(99);";
    let after = format(before, LetStyle::Hide).unwrap();
    let mut spans = vec![];
    for source in [before, &after] {
        let file = w.put("origin.ceru", source);
        let c = modules::load(&file).unwrap();
        let p = c.to_ir().unwrap();
        let error = cerune_lang::ir_executor::run(&p).unwrap_err();
        assert_eq!(error.output(), "7\n");
        let failure = error.runtime_failure().unwrap();
        assert_eq!(
            failure.code,
            cerune_lang::runtime::FailureCode::DivisionByZero
        );
        assert_eq!(
            source[failure.span.start()..failure.span.end()].replace(' ', ""),
            "1/x"
        );
        let m = cerune_lang::mir::lower(&p).unwrap();
        let mir_error = cerune_lang::mir_executor::run(&m).unwrap_err();
        let ssa_error =
            cerune_lang::ssa_executor::run(&cerune_lang::mir::ssa::construct(&m).unwrap())
                .unwrap_err();
        let vm_error = cerune_lang::run_bytecode(&bytecode::lower(&p).unwrap()).unwrap_err();
        assert_eq!(mir_error.runtime_failure(), Some(failure));
        assert_eq!(ssa_error.runtime_failure(), Some(failure));
        assert_eq!(vm_error.runtime_failure(), Some(failure));
        assert_eq!(mir_error.output(), "7\n");
        assert_eq!(ssa_error.output(), "7\n");
        assert_eq!(vm_error.vm_error().output(), "7\n");
        spans.push(failure.span);
        let out = w.cli("run", &file, &[]);
        assert!(!out.status.success());
        assert_eq!(out.stdout, b"7\n");
    }
    assert_ne!(spans[0].start(), spans[1].start());
}

#[test]
fn formatter_refuses_read_only_files_and_directories_without_leaving_temps() {
    let w = Workspace::new();
    let input = w.put("input.ceru", "print(1);");
    let output = w.put("output.ceru", "keep");
    let original_permissions = fs::metadata(&output).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&output, readonly).unwrap();
    let result = w.cli("fmt", &input, &["-o", output.to_str().unwrap()]);
    fs::set_permissions(&output, original_permissions).unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "keep");
    assert!(
        !w.cli("fmt", &input, &["-o", w.0.to_str().unwrap()])
            .status
            .success()
    );
    assert!(!fs::read_dir(&w.0).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".cerune-fmt-")
    }));
}

#[cfg(unix)]
#[test]
fn formatter_refuses_replacing_symlinks_and_retains_file_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let w = Workspace::new();
    let input = w.put("input.ceru", "print(1);");
    let link = w.0.join("link.ceru");
    symlink(&input, &link).unwrap();
    assert!(!w.cli("fmt", &link, &["--write"]).status.success());
    assert!(
        !w.cli("fmt", &input, &["-o", link.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), "print(1);");
    fs::set_permissions(&input, fs::Permissions::from_mode(0o640)).unwrap();
    let mode = fs::metadata(&input).unwrap().permissions().mode() & 0o777;
    assert!(w.cli("fmt", &input, &["--write"]).status.success());
    assert_eq!(
        fs::metadata(&input).unwrap().permissions().mode() & 0o777,
        mode
    );
}
