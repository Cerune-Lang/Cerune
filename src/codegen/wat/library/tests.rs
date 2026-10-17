use super::*;
use crate::modules;

fn error(source: &str, names: &[&str]) -> Diagnostic {
    emit_library(&modules::from_source(source).unwrap(), names).unwrap_err()
}

#[test]
fn selected_exports_preserve_ir_types_and_declaration_origins() {
    let source = "pub fn add(x:i32)->i32{return x+1;} pub fn other(x:f64)->f64{return x;} fn hidden()->f64{return 3.0;}";
    let c = modules::from_source(source).unwrap();
    let before = c.to_ir().unwrap();
    let ordinary = super::super::emit_wat(&before).unwrap();
    let library = emit_library(&c, &["other", "add"]).unwrap();
    assert_eq!(library.program(), &before);
    assert_eq!(super::super::emit_wat(library.program()).unwrap(), ordinary);
    assert!(ordinary.contains("(export \"main\""));
    assert!(!library.wat().contains("(func $main"));
    assert!(!library.wat().contains("(start "));
    assert!(!library.wat().contains("(export \"memory\""));
    assert_eq!(
        library
            .exports()
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>(),
        ["other", "add"]
    );
    assert_eq!(library.exports()[1].parameters(), [ScalarType::I32]);
    assert_eq!(library.exports()[1].result(), ScalarType::I32);
    let span = library.exports()[1].span();
    assert_eq!(&source[span.start()..span.end()], "add");
    assert_eq!(
        span.source_id(),
        c.function_declarations()[0].name_span().source_id()
    );
    assert!(library.wat().contains("i64.extend_i32_s"));
    assert!(library.wat().contains("i32.wrap_i64"));
    assert_eq!(library.program().function_definitions.len(), 3);
    assert_eq!(
        library.export_manifest(),
        emit_library(&c, &["other", "add"])
            .unwrap()
            .export_manifest()
    );
}

#[test]
fn rejects_unknown_private_duplicate_and_empty_selections() {
    let s = "pub fn add(x:i32)->i32{return x;} fn hidden()->f64{return 1.0;}";
    assert!(error(s, &[]).message().contains("at least one"));
    assert!(
        error(s, &["missing"])
            .message()
            .contains("unknown entry-module")
    );
    let private = error(s, &["hidden"]);
    assert!(private.message().contains("not pub"));
    let span = private.primary_span().unwrap();
    assert_eq!(&s[span.start()..span.end()], "hidden");
    assert!(
        error(s, &["add", "add"])
            .message()
            .contains("duplicate export")
    );
}

#[test]
fn rejects_initialization_and_unsupported_export_signatures() {
    for (s, needle) in [
        ("pub fn f()->f64{return 1.0;} print(1);", "top-level"),
        (
            "pub fn f()->f64{return 1.0;} fn main()->void{}",
            "declare main",
        ),
        ("pub fn f(x:u64)->f64{return f64(x);}", "i32 or f64"),
        ("pub fn f()->i64{return 1;}", "i32 or f64"),
        ("pub fn f()->void{}", "scalar result"),
        ("pub fn f<T>(x:T)->T{return x;}", "generic"),
    ] {
        assert!(error(s, &["f"]).message().contains(needle), "{s}");
    }
}

#[test]
fn validates_unused_helpers_and_unexecuted_branches() {
    let entry = "pub fn f(x:f64)->f64{return x;}";
    for helper in [
        "fn unused()->void{print(1);}",
        "fn unused()->void{flush();}",
        "fn unused()->void{if false {print(1);}}",
        "fn unused()->string{return \"hidden\";}",
        "fn unused()->i64{return byte_len(\"hidden\");}",
        "fn unused()->void{v:[i32]=array_repeat::<i32>(1,2);}",
        "type Unused { x:i32 }",
        "fn generic<T>(x:T)->T{return x;}",
        "const TEXT:string=\"hidden\";",
    ] {
        let diagnostic = error(&format!("{entry} {helper}"), &["f"]);
        assert!(diagnostic.primary_span().is_some(), "{helper}");
    }
    let c = modules::from_source("import \"std:io\" as io; pub fn f()->f64{return 1.0;}").unwrap();
    assert!(emit_library(&c, &["f"]).is_err());
}

#[test]
fn rejects_stale_metadata_after_external_ast_changes() {
    let mut c = modules::from_source("pub fn f(x:f64)->f64{return x;}").unwrap();
    let ast::Item::FunctionDefinition(f) = &mut c.program.items[0] else {
        panic!()
    };
    f.name = "renamed".into();
    assert!(
        emit_library(&c, &["f"])
            .unwrap_err()
            .message()
            .contains("no longer match")
    );
}
