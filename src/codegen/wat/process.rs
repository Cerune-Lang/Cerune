use super::ir::{Module, Origin};
use crate::ir::{Type, TypeDefinition};
use std::fmt::Write;
mod report;
pub(super) const IMPORT: &str = r#"
  (import "cerune" "process_begin_v1" (func $process_begin_v1 (param i64 i64 i64 i64 i64 i64 i64 i64) (result i32)))
  (import "cerune" "process_field_v1" (func $process_field_v1 (param i32 i64 i64) (result i32)))
  (import "cerune" "process_chunk_v1" (func $process_chunk_v1 (param i64 i32) (result i32)))
  (import "cerune" "process_prepare_v1" (func $process_prepare_v1 (result i32)))
  (import "cerune" "process_execute_v1" (func $process_execute_v1 (param i32) (result i32)))
  (import "cerune" "process_report_v1" (func $process_report_v1 (param i32) (result i64 i32)))
  (import "cerune" "process_copy_v1" (func $process_copy_v1 (param i32 i64 i32) (result i64 i32 i32)))
  (import "cerune" "process_release_v1" (func $process_release_v1 (result i32)))
"#;
pub(super) fn support(m: &Module, s: &mut String) {
    s.push_str(
        &include_str!("process/support.wat.inc")
            .replace("@LIMIT@", &m.array_heap_limit.unwrap().to_string()),
    );
}
fn definition(m: &Module, id: usize) -> &TypeDefinition {
    m.process_types.iter().find(|d| d.id.0 == id).unwrap()
}
fn size(m: &Module, ty: &Type) -> usize {
    match ty {
        Type::Named(id) => definition(m, id.0)
            .fields
            .iter()
            .map(|f| size(m, &f.ty))
            .sum(),
        Type::Array { element, length } => size(m, element) * length,
        _ => 8,
    }
}
fn path(m: &Module, id: usize, names: &[&str]) -> (usize, Type) {
    let mut offset = 0;
    let mut ty = Type::Named(crate::ir::TypeId(id));
    for name in names {
        let Type::Named(id) = ty else { unreachable!() };
        let d = definition(m, id.0);
        let f = d.fields.iter().find(|f| f.name == *name).unwrap();
        offset += d
            .fields
            .iter()
            .take_while(|p| p.name != *name)
            .map(|f| size(m, &f.ty))
            .sum::<usize>();
        ty = f.ty.clone();
    }
    (offset, ty)
}
fn text(value: &str, s: &mut String) {
    for b in value.bytes() {
        writeln!(s, "    (call $write_error_byte (i32.const {b}))").unwrap();
    }
}
fn fail_if(condition: &str, name: &str, s: &mut String) {
    writeln!(
        s,
        "    (if {condition} (then (call ${name}_fail (i32.const 13))))"
    )
    .unwrap();
}
fn checked(expression: &str, name: &str, expected: i32, s: &mut String) {
    writeln!(
        s,
        "    (call ${name}_check {expression} (i32.const {expected}))"
    )
    .unwrap();
}
pub(super) fn located(
    m: &Module,
    request: usize,
    outcome: usize,
    origin: Origin,
    name: &str,
    s: &mut String,
) {
    writeln!(
        s,
        "  (func ${name}_fail (param $status i32)\n    (drop (call $flush_stdout_v1))"
    )
    .unwrap();
    text("cerune-process-abi-v1 status=", s);
    s.push_str("    (call $cerune_process_error_number (i64.extend_i32_u (local.get $status)))\n");
    text(
        &format!(
            " node={}{} bytes={}..{}\n",
            origin.node_id.0,
            origin.span.source_id().record_field(),
            origin.span.start(),
            origin.span.end()
        ),
        s,
    );
    s.push_str("    unreachable)\n");
    writeln!(s,"  (func ${name}_check (param $status i32) (param $expected i32)\n    (if (i32.ne (local.get $status) (local.get $expected)) (then\n      (call ${name}_fail (select (local.get $status) (i32.const 13) (i32.ge_u (local.get $status) (i32.const 10)))))))").unwrap();
    writeln!(s,"  (func ${name} (param $req i32) (param $result i32)\n    (local $args i32) (local $input i32) (local $str i32) (local $index i64)\n    (local $out i32) (local $err i32) (local $status i32) (local $reservation i32)").unwrap();
    for i in 0..32 {
        writeln!(s, "    (local $w{i} i64)").unwrap();
    }
    let load = |field: &str, width: &str| {
        format!(
            "({width}.load offset={} (local.get $req))",
            path(m, request, &[field]).0
        )
    };
    writeln!(
        s,
        "    (local.set $args {})\n    (local.set $input {})",
        load("arguments", "i32"),
        load("stdin", "i32")
    )
    .unwrap();
    checked(
        &format!(
            "(call $process_begin_v1 (call $cerune_array_length (local.get $args)) {} {} {} (i64.const {}) (i64.const {}) (i64.const {}) (i64.const {}))",
            load("stdout_limit", "i64"),
            load("stderr_limit", "i64"),
            load("timeout_ms", "i64"),
            origin.node_id.0,
            origin.span.source_id().index(),
            origin.span.start(),
            origin.span.end()
        ),
        name,
        0,
        s,
    );
    writeln!(s, "    (local.set $str {})", load("target", "i32")).unwrap();
    checked(
        "(call $cerune_process_send (i32.const 0) (i64.const 0) (i64.load (local.get $str)) (i32.add (local.get $str) (i32.const 8)) (i32.const 1))",
        name,
        0,
        s,
    );
    s.push_str("    (block $done (loop $arguments\n      (br_if $done (i64.eq (local.get $index) (call $cerune_array_length (local.get $args))))\n      (local.set $str (i32.load (call $cerune_array_element_address (local.get $args) (local.get $index))))\n");
    checked(
        "(call $cerune_process_send (i32.const 1) (local.get $index) (i64.load (local.get $str)) (i32.add (local.get $str) (i32.const 8)) (i32.const 1))",
        name,
        0,
        s,
    );
    s.push_str(
        "      (local.set $index (i64.add (local.get $index) (i64.const 1))) (br $arguments)))\n",
    );
    checked(
        "(call $cerune_process_send (i32.const 2) (i64.const 0) (call $cerune_array_length (local.get $input)) (i32.add (local.get $input) (i32.const 56)) (i32.const 8))",
        name,
        0,
        s,
    );
    s.push_str("    (local.set $status (call $process_prepare_v1))\n    (if (i32.eq (local.get $status) (i32.const 1)) (then\n");
    writeln!(s,"      (call $cerune_process_reserve {} {})\n      local.set $reservation local.set $err local.set $out",load("stdout_limit","i64"),load("stderr_limit","i64")).unwrap();
    checked(
        "(call $process_execute_v1 (local.get $reservation))",
        name,
        2,
        s,
    );
    s.push_str("    ) (else\n");
    checked("(local.get $status)", name, 2, s);
    s.push_str("    ))\n");
    for i in 0..32 {
        writeln!(
            s,
            "    (call $process_report_v1 (i32.const {i}))\n    local.set $status local.set $w{i}"
        )
        .unwrap();
        checked("(local.get $status)", name, 0, s);
    }
    report::validate(name, s);
    for (word, value) in [
        (2, format!("(i64.const {})", origin.node_id.0)),
        (
            3,
            format!("(i64.const {})", origin.span.source_id().index()),
        ),
        (4, format!("(i64.const {})", origin.span.start())),
        (5, format!("(i64.const {})", origin.span.end())),
        (7, "(call $cerune_array_length (local.get $input))".into()),
        (9, load("stdout_limit", "i64")),
        (10, load("stderr_limit", "i64")),
        (11, load("timeout_ms", "i64")),
    ] {
        fail_if(&format!("(i64.ne (local.get $w{word}) {value})"), name, s);
    }
    for (word, owner) in [(14, "out"), (15, "err")] {
        fail_if(
            &format!(
                "(i64.gt_u (local.get $w{word}) (call $cerune_array_length (local.get ${owner})))"
            ),
            name,
            s,
        );
    }
    checked(
        "(call $cerune_process_copy (local.get $out) (i32.const 1) (local.get $w14))",
        name,
        0,
        s,
    );
    checked(
        "(call $cerune_process_copy (local.get $err) (i32.const 2) (local.get $w15))",
        name,
        0,
        s,
    );
    checked("(call $process_release_v1)", name, 0, s);
    writeln!(
        s,
        "    (memory.fill (local.get $result) (i32.const 0) (i32.const {}))",
        size(m, &Type::Named(crate::ir::TypeId(outcome)))
    )
    .unwrap();
    for (field, owner) in [("stdout", "out"), ("stderr", "err")] {
        writeln!(
            s,
            "    (i32.store offset={} (local.get $result) (local.get ${owner}))",
            path(m, outcome, &[field]).0
        )
        .unwrap();
    }
    report::emit(m, outcome, s);
    s.push_str("  )\n");
}
