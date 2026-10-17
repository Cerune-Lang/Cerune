#[path = "support/process.rs"]
mod process;
use cerune_lang::{bytecode, modules, run_bytecode};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self::at_stamp(stamp)
    }
    fn at_stamp(stamp: u128) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        // 同じ時刻の並列テストも分離し、既存ディレクトリを再利用しません。
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cerune-modules-{}-{stamp}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test workspace: {error}"),
            }
        }
    }
    fn put(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }
    fn cli(&self, command: &str, entry: &Path, args: &[&str]) -> Output {
        process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(command)
                .arg(entry)
                .args(args)
                .current_dir(&self.0),
            &self.0,
            command,
            Duration::from_secs(30),
        )
        .unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn diamond_imports_share_nominal_types_and_do_not_execute_imported_main() {
    let w = Workspace::new();
    w.put("lib/value.ceru", "pub type Value { n: u64, } pub fn main() -> u64 { print(999); return 9; } pub fn make(n: u64) -> Value { return Value { n: n }; }");
    w.put(
        "left.ceru",
        "import \"lib/value.ceru\" as v; pub fn make() -> v::Value { return v::make(7); }",
    );
    w.put(
        "nested/right.ceru",
        "import \"../lib/./value.ceru\" as v; pub fn read(x: v::Value) -> u64 { return x.n; }",
    );
    let entry = w.put("main.ceru", "import \"left.ceru\" as l; import \"nested/right.ceru\" as r; fn main() -> void { print(r::read(l::make())); }");
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(compilation.sources.files().count(), 4);
    let ir = compilation.to_ir().unwrap();
    assert_eq!(run_bytecode(&bytecode::lower(&ir).unwrap()).unwrap(), "7\n");
    let output = w.cli("run", &entry, &[]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"7\n");
    assert!(output.stderr.is_empty());
    assert_eq!(
        compilation.source_manifest(),
        modules::load(&entry).unwrap().source_manifest()
    );
}

#[test]
fn same_names_are_local_and_aliases_can_name_the_same_file() {
    let w = Workspace::new();
    w.put(
        "a.ceru",
        "pub fn value() -> i64 { return 1; } fn helper() -> i64 { return 3; }",
    );
    w.put(
        "b.ceru",
        "pub fn value() -> i64 { return helper(); } fn helper() -> i64 { return 2; }",
    );
    let entry = w.put("main.ceru", "import \"a.ceru\" as a; import \"b.ceru\" as b; import \"./a.ceru\" as again; fn value() -> i64 { return 4; } print(a::value()); print(b::value()); print(again::value()); print(value());");
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(compilation.sources.files().count(), 3);
    assert_eq!(
        run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap(),
        "1\n2\n1\n4\n"
    );
}

#[test]
fn rejects_visibility_initialization_cycles_and_namespace_errors_at_their_source() {
    let w = Workspace::new();
    let valid = "pub type Value { n: u64, } type Private { n: u64, } pub fn value() -> u64 { return helper(); } fn helper() -> u64 { return 7; }";
    for (library, entry, reason, file) in [
        (
            valid,
            "import \"lib.ceru\" as lib; print(lib::helper());",
            "private function",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; x: lib::Private = lib::Private { n: 1 };",
            "private type",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; print(helper());",
            "unknown function",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; print(other::value());",
            "unknown import alias",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; print(lib::missing());",
            "unknown function",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; print(lib::value);",
            "unknown constant",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; import \"lib.ceru\" as lib;",
            "duplicate import alias",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; fn lib() -> void {}",
            "conflicts with an import alias",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; fn f(lib: i64) -> void {}",
            "conflicts with an import alias",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as lib; mut lib: i64 = 0;",
            "conflicts with an import alias",
            "main.ceru",
        ),
        (
            valid,
            "import \"lib.ceru\" as i64;",
            "reserved",
            "main.ceru",
        ),
        (
            "print(99);",
            "import \"lib.ceru\" as lib;",
            "only imports, types, functions, and constants",
            "lib.ceru",
        ),
        (
            "import \"main.ceru\" as root;",
            "import \"lib.ceru\" as lib;",
            "cyclic import",
            "lib.ceru",
        ),
        (
            "type Hidden { x: i64, } pub fn leak() -> Hidden { return Hidden { x: 1 }; }",
            "import \"lib.ceru\" as lib;",
            "public signature",
            "lib.ceru",
        ),
        (
            "type Hidden { x: i64, } pub type Leak { x: [Hidden; 1], }",
            "import \"lib.ceru\" as lib;",
            "public signature",
            "lib.ceru",
        ),
        (
            "pub type Same { x: i64, } fn Same() -> i64 { return 1; }",
            "import \"lib.ceru\" as lib; print(lib::Same());",
            "private function",
            "main.ceru",
        ),
        (
            "pub fn i64() -> i64 { return 1; }",
            "import \"lib.ceru\" as lib;",
            "reserved",
            "lib.ceru",
        ),
        (
            "pub fn f() -> void { f(); }",
            "import \"lib.ceru\" as lib;",
            "recursive",
            "lib.ceru",
        ),
        (
            "pub fn f() -> void { print(missing); }",
            "import \"lib.ceru\" as lib;",
            "unknown binding",
            "lib.ceru",
        ),
        (
            "pub fn f() -> void { print(\"\\q\"); }",
            "import \"lib.ceru\" as lib;",
            "escape",
            "lib.ceru",
        ),
    ] {
        w.put("lib.ceru", library);
        let path = w.put("main.ceru", entry);
        let (message, span, name) = match modules::load(&path) {
            Err(error) => {
                let span = error.diagnostic.primary_span().unwrap();
                (
                    error.diagnostic.message().to_owned(),
                    span,
                    error
                        .sources
                        .get(span.source_id())
                        .map(|s| s.name().to_owned()),
                )
            }
            Ok(compilation) => {
                let error = compilation.to_ir().unwrap_err();
                let span = error.primary_span().unwrap();
                (
                    error.message().to_owned(),
                    span,
                    compilation
                        .sources
                        .get(span.source_id())
                        .map(|s| s.name().to_owned()),
                )
            }
        };
        assert!(message.contains(reason), "{entry}: {message}");
        assert!(name.unwrap().ends_with(file), "{entry}: {span:?}");
    }
}

#[test]
fn paths_and_declarations_are_explicit_and_fail_before_overwriting_output() {
    let w = Workspace::new();
    for source in [
        "import \"absent.ceru\" as missing;",
        "import \"/absolute.ceru\" as missing;",
        "import \"C:/absolute.ceru\" as missing;",
        "import \"dir\\\\file.ceru\" as missing;",
        "import \"bad\\0.ceru\" as missing;",
        "import \"file.txt\" as missing;",
        "import \"absent.ceru\";",
        "print(1); import \"lib.ceru\" as lib;",
        "pub x: i64 = 1;",
        "pub fn f() -> void { import \"lib.ceru\" as lib; }",
    ] {
        let entry = w.put("main.ceru", source);
        w.put("output.c", "keep");
        let output = w.cli("emit-c", &entry, &["-o", "output.c"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read_to_string(w.0.join("output.c")).unwrap(), "keep");
    }
    assert!(
        cerune_lang::compile("import \"lib.ceru\" as lib;")
            .unwrap_err()
            .message()
            .contains("modules::load")
    );
    assert!(
        cerune_lang::compile("pub fn f() -> void {}")
            .unwrap_err()
            .message()
            .contains("modules::load")
    );
    let entry = w.put(
        "main.ceru",
        "type Hidden { n: i64, } pub type Leak { v: Hidden, }",
    );
    assert!(
        modules::load(&entry)
            .unwrap_err()
            .render()
            .contains("public signature")
    );
}

#[test]
fn module_entry_syntax_errors_keep_file_identity_and_deep_imports_are_bounded() {
    let w = Workspace::new();
    let entry = w.put("main.ceru", "import \"lib.ceru\" as lib; print(1 + );");
    let error = modules::load(&entry).unwrap_err();
    assert_eq!(
        error.diagnostic.primary_span().unwrap().source_id().index(),
        1
    );
    assert!(error.render().contains("main.ceru:1:"));
    for index in 0..129 {
        w.put(
            &format!("m{index}.ceru"),
            &format!("import \"m{}.ceru\" as next;", index + 1),
        );
    }
    w.put("m129.ceru", "");
    let error = modules::load(&w.0.join("m0.ceru")).unwrap_err();
    assert!(error.render().contains("import nesting exceeds 128"));
    w.put("m128.ceru", "");
    assert_eq!(
        modules::load(&w.0.join("m0.ceru"))
            .unwrap()
            .sources
            .files()
            .count(),
        129
    );
}

#[test]
#[cfg(unix)]
fn symlink_imports_share_identity_and_resolve_from_the_target_directory() {
    let w = Workspace::new();
    w.put(
        "actual/value.ceru",
        "pub type Value { n: i64, } pub fn make() -> Value { return Value { n: 7 }; }",
    );
    w.put(
        "actual/logic.ceru",
        "import \"value.ceru\" as v; pub fn make() -> v::Value { return v::make(); }",
    );
    std::os::unix::fs::symlink(w.0.join("actual/logic.ceru"), w.0.join("link.ceru")).unwrap();
    let entry = w.put("main.ceru", "import \"link.ceru\" as a; import \"actual/logic.ceru\" as b; print(a::make().n); print(b::make().n);");
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(compilation.sources.files().count(), 3);
    for source in compilation.sources.files() {
        assert_eq!(fs::read_to_string(source.name()).unwrap(), source.text());
    }
    assert_eq!(
        run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap(),
        "7\n7\n"
    );
}

#[test]
fn manifest_preserves_dependency_text_and_cli_failures_identify_the_definition_file() {
    let w = Workspace::new();
    let library = "// 日本語\r\npub fn divide(x: i64) -> i64 { print(\"先行\\0\\r\\n\"); return 10 / x; }\r\n";
    w.put("lib.ceru", library);
    let entry = w.put(
        "main.ceru",
        "import \"lib.ceru\" as lib; print(lib::divide(0));",
    );
    let compilation = modules::load(&entry).unwrap();
    let error = run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap_err();
    let failure = error.runtime_failure().unwrap();
    assert_eq!(compilation.sources.slice(failure.span), Some("10 / x"));
    assert_eq!(failure.span.source_id().index(), 2);
    let output = w.cli("run", &entry, &["--diagnostic-format", "runtime-v1"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, "先行\0\r\n\n".as_bytes());
    assert_eq!(
        String::from_utf8(output.stderr)
            .unwrap()
            .replace("\r\n", "\n"),
        format!("cerune: {}\n", failure.record())
    );
    let human = w.cli("run", &entry, &[]);
    assert!(
        String::from_utf8(human.stderr)
            .unwrap()
            .contains("lib.ceru:2:")
    );
    let output = w.cli("emit-sources", &entry, &[]);
    assert!(output.status.success());
    assert_eq!(output.stdout, compilation.source_manifest().as_bytes());
    let node = std::env::var_os("CERUNE_TEST_NODE").unwrap_or_else(|| "node".into());
    fs::write(w.0.join("sources.json"), output.stdout).unwrap();
    let output = process::bounded_output(Command::new(node).arg("-e").arg("const fs=require('fs');const m=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));if(m.schema!=='cerune-sources-v1'||m.files.length!==2||m.files[1].id!==2)process.exit(1);process.stdout.write(m.files[1].text);").arg(w.0.join("sources.json")), &w.0, "manifest", Duration::from_secs(10)).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, library.as_bytes());
}

#[test]
fn constants_resolve_across_diamond_imports_and_keep_private_values_private() {
    let w = Workspace::new();
    w.put(
        "values.ceru",
        "const HIDDEN: i64 = 3; pub const LIMIT: i64 = HIDDEN * 4;",
    );
    w.put(
        "left.ceru",
        "import \"values.ceru\" as values; pub const LEFT: i64 = values::LIMIT;",
    );
    w.put(
        "right.ceru",
        "import \"values.ceru\" as values; pub const RIGHT: i64 = values::LIMIT + 1;",
    );
    let entry = w.put("main.ceru", "import \"left.ceru\" as a; import \"right.ceru\" as b; const TOTAL: i64 = a::LEFT + b::RIGHT; fn main() -> void { print(TOTAL); }");
    let compilation = modules::load(&entry).unwrap();
    let ir = compilation.to_ir().unwrap();
    assert_eq!(ir.constant_definitions.len(), 5);
    assert_eq!(
        run_bytecode(&bytecode::lower(&ir).unwrap()).unwrap(),
        "25\n"
    );
    for (library, source, reason) in [
        (
            "const PRIVATE: i64 = 1;",
            "import \"values.ceru\" as v; print(v::PRIVATE);",
            "private constant",
        ),
        (
            "type P { x: i64, } pub const VALUE: P = P { x: 1 };",
            "import \"values.ceru\" as v;",
            "public signature",
        ),
        (
            "pub const VALUE: i64 = 1;",
            "import \"values.ceru\" as v; const v: i64 = 1;",
            "import alias",
        ),
        (
            "pub const VALUE: i64 = 1; fn f(VALUE: i64) -> void {}",
            "import \"values.ceru\" as v;",
            "conflicts with a constant",
        ),
    ] {
        w.put("values.ceru", library);
        let entry = w.put("main.ceru", source);
        let error = modules::load(&entry).unwrap_err();
        assert!(error.diagnostic.message().contains(reason), "{error:?}");
    }
}

#[test]
fn invalid_imported_constants_fail_before_output_or_artifact_overwrite() {
    let w = Workspace::new();
    w.put(
        "values.ceru",
        "// 日本語\r\npub const BAD: i64 = 1 / 0;\r\n",
    );
    let entry = w.put(
        "main.ceru",
        "import \"values.ceru\" as v; print(\"not executed\");",
    );
    let compilation = modules::load(&entry).unwrap();
    let error = compilation.to_ir().unwrap_err();
    assert!(error.message().contains("division-by-zero"));
    let span = error.primary_span().unwrap();
    assert_eq!(compilation.sources.slice(span), Some("1 / 0"));
    assert!(
        compilation
            .sources
            .get(span.source_id())
            .unwrap()
            .name()
            .ends_with("values.ceru")
    );
    for command in [
        "run",
        "check",
        "emit-ir",
        "emit-bytecode",
        "emit-c",
        "emit-llvm",
        "emit-qbe",
        "emit-wat",
        "emit-asm",
        "emit-obj",
    ] {
        let artifact = w.put("preserved.txt", "keep");
        let args = if command == "run" || command == "check" {
            Vec::new()
        } else {
            vec![
                "--target",
                "x86_64-unknown-linux-gnu",
                "-o",
                artifact.to_str().unwrap(),
            ]
        };
        // 全コマンドが同じコンパイル時診断を出し、既存成果物を触りません。
        let args: Vec<_> = if matches!(command, "emit-ir" | "emit-bytecode" | "emit-c" | "emit-wat")
        {
            vec!["-o", artifact.to_str().unwrap()]
        } else {
            args
        };
        let output = w.cli(command, &entry, &args);
        assert!(!output.status.success(), "{command}");
        assert!(output.stdout.is_empty(), "{command}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("division-by-zero") && stderr.contains("values.ceru"),
            "{command}: {stderr}"
        );
        assert_eq!(fs::read_to_string(artifact).unwrap(), "keep");
    }
}

#[test]
fn enum_exports_keep_variant_names_nominal_types_and_private_payloads() {
    let w = Workspace::new();
    w.put("lib.ceru", "pub enum E { A { value: string }, B }");
    let entry = w.put(
        "main.ceru",
        r#"
        import "lib.ceru" as lib;
        import "./lib.ceru" as same;
        x: lib::E = same::E::A { value: "shared" };
        match x { same::E::A { value: text } => { print(text); }, lib::E::B {} => {} }
    "#,
    );
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap(),
        "shared\n"
    );
    for (library, source, reason) in [
        (
            "enum Hidden { A }",
            "x: infer = lib::Hidden::A {};",
            "private type",
        ),
        (
            "type Hidden { n: i64 } pub enum E { A { v: Hidden } }",
            "",
            "private type",
        ),
        (
            "pub enum E { A }",
            "x: infer = lib::E::Unknown {};",
            "unknown variant",
        ),
        (
            "pub enum E { A { n: i64 } }",
            "match (lib::E::A { n: 1 }) { lib::E::A { n: lib } => {} }",
            "import alias",
        ),
        (
            "pub enum E { A }",
            "print(lib::E::A());",
            "unknown function",
        ),
    ] {
        w.put("lib.ceru", library);
        let entry = w.put(
            "main.ceru",
            &format!("import \"lib.ceru\" as lib; {source}"),
        );
        let result = w.cli("check", &entry, &[]);
        assert!(!result.status.success());
        let text = String::from_utf8_lossy(&result.stderr);
        assert!(text.contains(reason), "{library}: {source}: {text}");
    }
}

#[test]
fn array_lengths_resolve_inside_imported_functions_and_constants() {
    let w = Workspace::new();
    w.put("values.ceru", "pub const ITEMS: [i64; 2] = [7, 9]; pub const COUNT: i64 = array_len(ITEMS); pub fn size() -> i64 { return array_len(ITEMS); }");
    let entry = w.put("main.ceru", "import \"values.ceru\" as values; print(values::COUNT); print(values::size()); print(array_len(values::ITEMS));");
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap(),
        "2\n2\n2\n"
    );
    for (library, source) in [
        (
            "pub fn array_len() -> i64 { return 1; }",
            "import \"values.ceru\" as values;",
        ),
        (
            "pub const array_len: i64 = 1;",
            "import \"values.ceru\" as values;",
        ),
        ("", "import \"values.ceru\" as array_len;"),
    ] {
        w.put("values.ceru", library);
        let entry = w.put("main.ceru", source);
        if let Ok(compilation) = modules::load(&entry) {
            assert!(compilation.to_ir().is_err());
        }
    }
}

#[test]
fn array_iteration_resolves_imported_types_constants_and_functions() {
    let w = Workspace::new();
    w.put(
        "values.ceru",
        r#"
        pub type Row { value: i64 }
        pub const ROWS: [Row; 2] = [Row { value: 7 }, Row { value: 9 }];
        pub fn total() -> i64 {
            mut result: i64 = 0;
            for (row: Row in ROWS) { result = result + row.value; }
            return result;
        }
    "#,
    );
    let entry = w.put(
        "main.ceru",
        r#"
        import "values.ceru" as values;
        for (i: i64, row: values::Row in values::ROWS) { print(i); print(row.value); }
        print(values::total());
    "#,
    );
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&compilation.to_ir().unwrap()).unwrap()).unwrap(),
        "0\n7\n1\n9\n16\n"
    );
    for header in ["values: infer", "values: i64, row: infer"] {
        let entry = w.put(
            "main.ceru",
            &format!("import \"values.ceru\" as values; for ({header} in [1]) {{}}"),
        );
        let result = w.cli("check", &entry, &[]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("import alias"));
    }
}

#[test]
fn constant_array_lengths_resolve_public_values_and_private_implementation_sizes() {
    let entry = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/modules/constant_array_lengths.ceru");
    let compilation = modules::load(&entry).unwrap();
    let ir = compilation.to_ir().unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&ir).unwrap()).unwrap(),
        "共有サイズ\n6\n15\n保持\0\r\n\n"
    );
    let uses: Vec<_> = ir
        .constant_definitions
        .iter()
        .flat_map(|d| &d.array_length_uses)
        .collect();
    assert!(
        uses.iter()
            .any(|span| compilation.sources.slice(**span) == Some("dimensions::WIDTH"))
    );
    assert!(
        uses.iter()
            .any(|span| compilation.sources.slice(**span) == Some("INTERNAL_WIDTH"))
    );
}

#[test]
fn imported_invalid_lengths_preserve_files_and_existing_artifacts() {
    let w = Workspace::new();
    for (library, main, reason, slice) in [
        (
            "pub const N: i64 = 1 / 0;",
            "type T { values: [i64; dims::N] }",
            "division-by-zero",
            "1 / 0",
        ),
        (
            "pub const N: i64 = 0;",
            "type T { values: [i64; dims::N] }",
            "greater than zero",
            "dims::N",
        ),
        (
            "const N: i64 = 2;",
            "type T { values: [i64; dims::N] }",
            "private constant",
            "dims::N",
        ),
        (
            "pub const N: i64 = array_len(DATA); const DATA: [i64; N] = [1];",
            "type T { values: [i64; dims::N] }",
            "cyclic",
            "N",
        ),
    ] {
        w.put("dims.ceru", library);
        let entry = w.put(
            "main.ceru",
            &format!("import \"dims.ceru\" as dims; print(\"never\"); {main}"),
        );
        let diagnostic = match modules::load(&entry) {
            Ok(compilation) => {
                let error = compilation.to_ir().unwrap_err();
                assert_eq!(
                    compilation.sources.slice(error.primary_span().unwrap()),
                    Some(slice)
                );
                compilation.render(&error)
            }
            Err(error) => {
                assert_eq!(
                    error
                        .sources
                        .slice(error.diagnostic.primary_span().unwrap()),
                    Some(slice)
                );
                error.render()
            }
        };
        assert!(diagnostic.contains(reason), "{diagnostic}");
        for command in [
            "run",
            "emit-ir",
            "emit-bytecode",
            "emit-c",
            "emit-llvm",
            "emit-qbe",
            "emit-wat",
            "emit-asm",
        ] {
            let artifact = w.put("artifact.txt", "keep");
            let args = if command == "run" {
                vec![]
            } else {
                vec!["-o", artifact.to_str().unwrap()]
            };
            let result = w.cli(command, &entry, &args);
            assert!(!result.status.success(), "{command}");
            assert!(result.stdout.is_empty(), "{command}");
            assert!(
                String::from_utf8_lossy(&result.stderr).contains(reason),
                "{command}: {:?}",
                result.stderr
            );
            assert_eq!(fs::read_to_string(artifact).unwrap(), "keep");
        }
    }
}

#[test]
fn generic_modules_preserve_nominal_types_private_lengths_and_source_files() {
    let entry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/modules/generic_functions.ceru");
    let compilation = modules::load(&entry).unwrap();
    let ir = compilation.to_ir().unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&ir).unwrap()).unwrap(),
        "共有関数\n日本語\0\r\n\n18446744073709551615\n"
    );
    let origins: Vec<_> = ir
        .function_definitions
        .iter()
        .filter_map(|f| f.generic_origin.as_ref())
        .collect();
    assert_eq!(origins.len(), 3);
    assert!(origins.iter().any(|o| {
        o.calls
            .iter()
            .any(|c| c.span.source_id() != o.definition.source_id())
    }));
    assert!(
        origins
            .iter()
            .flat_map(|o| &o.calls)
            .flat_map(|c| &c.argument_spans)
            .any(|s| compilation.sources.slice(*s) == Some("arrays::COUNT"))
    );
    assert!(origins.iter().any(|o| {
        o.calls
            .iter()
            .any(|c| compilation.sources.slice(c.span) == Some("first::<T, SIZE>(values)"))
    }));
}

#[test]
fn invalid_imported_generic_calls_preserve_diagnostics_and_artifacts() {
    let w = Workspace::new();
    for (library, main, reason) in [
        (
            "fn echo<T>(v:T)->T{return v;}",
            "print(lib::echo::<i64>(1));",
            "private function",
        ),
        (
            "type Secret { n:i64 } pub fn echo<T>(v:T)->T{return v;}",
            "print(lib::echo::<lib::Secret>(1));",
            "private type",
        ),
        (
            "const N:i64=2; pub fn size<const M:i64>()->i64{return M;}",
            "print(lib::size::<lib::N>());",
            "private constant",
        ),
        (
            "pub fn add<T>(a:T,b:T)->T{return a+b;}",
            "print(lib::add::<string>(\"a\",\"b\"));",
            "generic instantiations",
        ),
        (
            "pub fn echo<T>(v:T)->T{return v;}",
            "print(lib::echo::<u8>(256));",
            "does not fit",
        ),
        (
            "pub fn echo<T>(v:T)->T{return v;}",
            "print(lib::echo::<i64,2>(1));",
            "generic arguments",
        ),
        (
            "type Secret { n:i64 } pub fn bad<T>(v:Secret)->T{return 1;}",
            "print(1);",
            "private type",
        ),
        (
            "pub fn bad<T>(v:T)->T{mut T:i64=1;return v;}",
            "print(1);",
            "conflicts with generic",
        ),
    ] {
        w.put("lib.ceru", library);
        let entry = w.put("main.ceru", &format!("import \"lib.ceru\" as lib; {main}"));
        let artifact = w.put("result.ceir", "keep");
        let output = w.cli("emit-ir", &entry, &["-o", artifact.to_str().unwrap()]);
        assert!(!output.status.success(), "{library}: {main}");
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains(reason), "{diagnostic}");
        assert!(diagnostic.contains(".ceru:"), "{diagnostic}");
        assert_eq!(fs::read_to_string(artifact).unwrap(), "keep");
    }
}

#[test]
fn distinct_module_types_do_not_share_generic_instances() {
    let w = Workspace::new();
    w.put("one.ceru", "pub type Item { value:i64 }");
    w.put("two.ceru", "pub type Item { value:i64 }");
    let entry = w.put(
        "main.ceru",
        r#"
        import "one.ceru" as one; import "two.ceru" as two;
        fn echo<T>(v:T)->T{return v;}
        print(echo::<one::Item>(one::Item{value:1}).value);
        print(echo::<two::Item>(two::Item{value:2}).value);
    "#,
    );
    let compilation = modules::load(&entry).unwrap();
    let ir = compilation.to_ir().unwrap();
    assert_eq!(
        run_bytecode(&bytecode::lower(&ir).unwrap()).unwrap(),
        "1\n2\n"
    );
    assert_eq!(ir.function_definitions.len(), 2);
    assert_ne!(
        ir.function_definitions[0]
            .generic_origin
            .as_ref()
            .unwrap()
            .arguments,
        ir.function_definitions[1]
            .generic_origin
            .as_ref()
            .unwrap()
            .arguments
    );
}

#[test]
fn array_repeat_resolves_imported_types_and_generic_element_lengths() {
    let w = Workspace::new();
    w.put("lib.ceru", "pub type P{n:i64} pub const N:i64=2; pub fn copies<T>(v:T,n:i64)->[T]{return array_repeat::<T>(v,n);}");
    let entry = w.put(
        "main.ceru",
        r#"import "lib.ceru" as lib;
        a:[lib::P]=array_repeat::<lib::P>(lib::P{n:4},2);
        print(a);print(lib::copies::<[i64;lib::N]>([1,2],2));"#,
    );
    for command in ["run", "run-vm", "run-mir"] {
        let result = w.cli(command, &entry, &[]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, b"[{n: 4}, {n: 4}]\n[[1, 2], [1, 2]]\n");
    }
}

#[path = "frontend_limits/mod.rs"]
mod frontend_limits;

#[test]
fn function_declarations_keep_visibility_source_names_and_entry_identity() {
    let w = Workspace::new();
    w.put(
        "a.ceru",
        "pub fn same(x:i32)->i32{return x+1;} fn hidden()->i32{return 0;}",
    );
    w.put("b.ceru", "pub fn same(x:i32)->i32{return x+2;}");
    let source = "import \"a.ceru\" as a; import \"b.ceru\" as b; pub fn same(x:i32)->i32{return a::same(b::same(x));}";
    let entry = w.put("entry.ceru", source);
    let c = modules::load(&entry).unwrap();
    let d = c.function_declarations();
    assert_eq!(d.len(), 4);
    let names = d.iter().filter(|f| f.name() == "same").collect::<Vec<_>>();
    assert_eq!(names.len(), 3);
    assert_eq!(names.iter().filter(|f| f.is_entry()).count(), 1);
    assert!(names.iter().all(|f| f.is_public()));
    assert_ne!(names[0].resolved_name(), names[1].resolved_name());
    assert_ne!(
        names[1].name_span().source_id(),
        names[2].name_span().source_id()
    );
    let l = cerune_lang::codegen::wat::emit_library(&c, &["same"]).unwrap();
    assert_eq!(l.exports().len(), 1);
    let private = d.iter().find(|f| f.name() == "hidden").unwrap();
    assert!(!private.is_public());
    assert!(!private.is_entry());
    assert!(cerune_lang::codegen::wat::emit_library(&c, &["a::same"]).is_err());
    assert!(cerune_lang::codegen::wat::emit_library(&c, &["hidden"]).is_err());
    for decl in d {
        let file = c.sources.get(decl.name_span().source_id()).unwrap();
        let span = decl.name_span();
        assert_eq!(&file.text()[span.start()..span.end()], decl.name());
    }
}

#[test]
fn nonmodular_declarations_keep_existing_anonymous_ir_and_private_visibility() {
    let c = modules::from_source("fn f(x:f64)->f64{return x;}").unwrap();
    let decl = &c.function_declarations()[0];
    assert_eq!(decl.name(), "f");
    assert_eq!(decl.resolved_name(), "f");
    assert!(decl.is_entry());
    assert!(!decl.is_public());
    assert_eq!(
        decl.name_span().source_id(),
        cerune_lang::source::SourceId::ANONYMOUS
    );
    assert_eq!(
        c.to_ir().unwrap(),
        cerune_lang::compile_to_ir("fn f(x:f64)->f64{return x;}").unwrap()
    );
}
