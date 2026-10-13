//! 複合値表示の、改行なし出力とUTF-8の引用処理です。
use crate::ir::{Program, Statement, StatementKind, Type};
use std::fmt::Write;
pub(super) fn uses_write(p: &Program) -> bool {
    fn body(ss: &[Statement]) -> bool {
        ss.iter().any(|s| match &s.kind {
            StatementKind::Write { .. } => true,
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => body(then_body) || body(else_body),
            StatementKind::For { body: b, .. } | StatementKind::While { body: b, .. } => body(b),
            _ => false,
        })
    }
    body(&p.statements) || p.function_definitions.iter().any(|f| body(&f.body))
}
pub(super) fn kind(ty: &Type, quoted: bool) -> &'static str {
    match ty {
        Type::String => {
            if quoted {
                "quoted"
            } else {
                "string"
            }
        }
        Type::Bool => "bool",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::Integer(crate::types::IntegerType::U64) => "u64",
        Type::Integer(_) => "i64",
        _ => unreachable!(),
    }
}
fn escapes() -> Vec<(u8, String)> {
    (0..=127u8)
        .filter_map(|c| {
            let text = match c {
                0 => "\\0".into(),
                9 => "\\t".into(),
                10 => "\\n".into(),
                13 => "\\r".into(),
                34 => "\\\"".into(),
                92 => "\\\\".into(),
                1..=31 | 127 => format!("\\u{{{c:02x}}}"),
                _ => return None,
            };
            Some((c, text))
        })
        .collect()
}
const FORMATS: &[(&str, &str)] = &[
    ("i64", "%lld"),
    ("u64", "%llu"),
    ("f32", "%.9g"),
    ("f64", "%.17g"),
    ("true", "true"),
    ("false", "false"),
    ("bool", "%s"),
];
pub(super) fn llvm_type(kind: &str) -> &'static str {
    match kind {
        "string" | "quoted" => "%cerune.string",
        "bool" => "i1",
        "f32" => "float",
        "f64" => "double",
        _ => "i64",
    }
}
pub(super) fn qbe_type(kind: &str) -> &'static str {
    match kind {
        "bool" => "w",
        "f32" => "s",
        "f64" => "d",
        _ => "l",
    }
}
pub(super) fn qbe() -> String {
    let mut o = String::new();
    for (kind, fmt) in FORMATS {
        writeln!(o, "data $write_{kind} = {{ b \"{fmt}\", b 0 }}").unwrap();
    }
    for kind in ["i64", "u64", "f32", "f64", "bool"] {
        writeln!(
            o,
            "function $cerune_write_{kind}({} %value) {{\n@start",
            qbe_type(kind)
        )
        .unwrap();
        let (ty, value) = match kind {
            "f32" => {
                o.push_str("  %wide =d exts %value\n");
                ("d", "%wide")
            }
            "bool" => {
                o.push_str("  jnz %value, @true, @false\n@true\n  call $printf(l $write_bool, ..., l $write_true)\n  ret\n@false\n");
                ("l", "$write_false")
            }
            _ => (qbe_type(kind), "%value"),
        };
        writeln!(
            o,
            "  call $printf(l $write_{kind}, ..., {ty} {value})\n  ret\n}}"
        )
        .unwrap();
    }
    o.push_str("function $cerune_write_escaped_byte(w %value) {\n@start\n");
    for (c, _) in escapes() {
        writeln!(
            o,
            "  %is{c} =w ceqw %value, {c}\n  jnz %is{c}, @escape{c}, @next{c}\n@next{c}"
        )
        .unwrap();
    }
    o.push_str("  call $putchar(w %value)\n  ret\n");
    for (c, text) in escapes() {
        writeln!(o, "@escape{c}").unwrap();
        for b in text.bytes() {
            writeln!(o, "  call $putchar(w {b})").unwrap();
        }
        o.push_str("  ret\n");
    }
    o.push_str("}\n");
    for (kind, quoted) in [("string", false), ("quoted", true)] {
        writeln!(
            o,
            "function $cerune_write_{kind}(l %value) {{\n@start\n  %length =l loadl %value"
        )
        .unwrap();
        if quoted {
            o.push_str("  call $putchar(w 34)\n");
        }
        o.push_str("  jmp @condition\n@condition\n  %index =l phi @start 0, @write %next\n  %done =w ceql %index, %length\n  jnz %done, @end, @write\n@write\n  %offset =l add %index, 8\n  %ptr =l add %value, %offset\n  %byte =w loadub %ptr\n");
        writeln!(
            o,
            "  call ${}(w %byte)",
            if quoted {
                "cerune_write_escaped_byte"
            } else {
                "putchar"
            }
        )
        .unwrap();
        o.push_str("  %next =l add %index, 1\n  jmp @condition\n@end\n");
        if quoted {
            o.push_str("  call $putchar(w 34)\n");
        }
        o.push_str("  ret\n}\n");
    }
    o
}

pub(super) fn wat_imports() -> String {
    let mut o = String::new();
    for (kind, ty) in [
        ("i64", "i64"),
        ("u64", "i64"),
        ("f32", "f32"),
        ("f64", "f64"),
    ] {
        writeln!(
            o,
            "  (import \"cerune\" \"write_{kind}\" (func $cerune_write_{kind} (param {ty})))"
        )
        .unwrap();
    }
    o
}
pub(super) fn wat() -> String {
    let mut o = String::from("  (func $cerune_write_escaped_byte (param $value i32)\n");
    for (c, text) in escapes() {
        writeln!(
            o,
            "    local.get $value\n    i32.const {c}\n    i32.eq\n    if"
        )
        .unwrap();
        for b in text.bytes() {
            writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
        }
        o.push_str("      return\n    end\n");
    }
    o.push_str("    local.get $value\n    call $write_byte\n  )\n  (func $cerune_write_bool (param $value i32)\n    local.get $value\n    if\n");
    for b in b"true" {
        writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
    }
    o.push_str("    else\n");
    for b in b"false" {
        writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
    }
    o.push_str("    end\n  )\n");
    for (kind, quoted) in [("string", false), ("quoted", true)] {
        writeln!(o,"  (func $cerune_write_{kind} (param $value i32)\n    (local $length i32) (local $index i32)\n    local.get $value\n    i32.load\n    local.set $length").unwrap();
        if quoted {
            o.push_str("    i32.const 34\n    call $write_byte\n");
        }
        o.push_str("    block $end\n      loop $loop\n        local.get $index\n        local.get $length\n        i32.eq\n        br_if $end\n        local.get $value\n        local.get $index\n        i32.add\n        i32.load8_u offset=8\n");
        writeln!(
            o,
            "        call ${}",
            if quoted {
                "cerune_write_escaped_byte"
            } else {
                "write_byte"
            }
        )
        .unwrap();
        o.push_str("        local.get $index\n        i32.const 1\n        i32.add\n        local.set $index\n        br $loop\n      end\n    end\n");
        if quoted {
            o.push_str("    i32.const 34\n    call $write_byte\n");
        }
        o.push_str("  )\n");
    }
    o
}
pub(super) fn asm_call(kind: &str, linux: bool) -> String {
    let mut o = String::new();
    if matches!(kind, "string" | "quoted" | "bool") {
        writeln!(
            o,
            "  movq %rax, {}\n  callq cerune_write_{kind}",
            if linux { "%rdi" } else { "%rcx" }
        )
        .unwrap();
    } else {
        if matches!(kind, "f32" | "f64") {
            if kind == "f32" {
                o.push_str("  cvtss2sd %xmm0, %xmm0\n");
            }
            if linux {
                o.push_str("  movl $1, %eax\n");
            } else {
                o.push_str("  movapd %xmm0, %xmm1\n  movq %xmm0, %rdx\n");
            }
        } else if linux {
            o.push_str("  movq %rax, %rsi\n  xorl %eax, %eax\n");
        } else {
            o.push_str("  movq %rax, %rdx\n");
        }
        writeln!(
            o,
            "  leaq .Lwrite_{kind}(%rip), {}\n  callq printf",
            if linux { "%rdi" } else { "%rcx" }
        )
        .unwrap();
    }
    o
}
pub(super) fn asm_support(linux: bool) -> String {
    let mut o = String::from(if linux {
        ".section .rodata\n"
    } else {
        ".section .rdata,\"dr\"\n"
    });
    for (kind, fmt) in FORMATS {
        writeln!(o, ".Lwrite_{kind}:\n  .asciz \"{fmt}\"").unwrap();
    }
    o.push_str(".text\n");
    let arg = if linux { "%edi" } else { "%ecx" };
    let arg64 = if linux { "%rdi" } else { "%rcx" };
    let frame = if linux { 8 } else { 40 };
    writeln!(o, "cerune_write_escaped_byte:\n  subq ${frame}, %rsp").unwrap();
    for (c, _) in escapes() {
        writeln!(o, "  cmpl ${c}, {arg}\n  je .Lescape_{c}").unwrap();
    }
    writeln!(o, "  callq putchar\n  addq ${frame}, %rsp\n  retq").unwrap();
    for (c, text) in escapes() {
        writeln!(o, ".Lescape_{c}:").unwrap();
        for b in text.bytes() {
            writeln!(o, "  movl ${b}, {arg}\n  callq putchar").unwrap();
        }
        writeln!(o, "  addq ${frame}, %rsp\n  retq").unwrap();
    }
    writeln!(
        o,
        "cerune_write_bool:\n  subq ${frame}, %rsp\n  cmpl $0, {arg}\n  je .Lwrite_bool_false"
    )
    .unwrap();
    for (label, text) in [("", "true"), (".Lwrite_bool_false:\n", "false")] {
        o.push_str(label);
        for b in text.bytes() {
            writeln!(o, "  movl ${b}, {arg}\n  callq putchar").unwrap();
        }
        writeln!(o, "  addq ${frame}, %rsp\n  retq").unwrap();
    }
    let (frame, ptr, index) = if linux { (24, 0, 8) } else { (56, 32, 40) };
    for (kind, quoted) in [("string", false), ("quoted", true)] {
        writeln!(o,"cerune_write_{kind}:\n  subq ${frame}, %rsp\n  movq {arg64}, {ptr}(%rsp)\n  movq $0, {index}(%rsp)").unwrap();
        if quoted {
            writeln!(o, "  movl $34, {arg}\n  callq putchar").unwrap();
        }
        writeln!(o,".Lwrite_{kind}_loop:\n  movq {ptr}(%rsp), %rax\n  movq {index}(%rsp), %rdx\n  cmpq (%rax), %rdx\n  je .Lwrite_{kind}_end\n  movzbl 8(%rax,%rdx), {arg}\n  callq {}\n  incq {index}(%rsp)\n  jmp .Lwrite_{kind}_loop\n.Lwrite_{kind}_end:",if quoted{"cerune_write_escaped_byte"}else{"putchar"}).unwrap();
        if quoted {
            writeln!(o, "  movl $34, {arg}\n  callq putchar").unwrap();
        }
        writeln!(o, "  addq ${frame}, %rsp\n  retq").unwrap();
    }
    o
}
