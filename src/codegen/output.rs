//! 明示flushの生成補助。stdoutだけを対象にし、失敗後に再flushしません。
use crate::runtime::output::{Origin, flush_failure_record};
use std::fmt::Write;

pub(super) fn llvm(module: &super::llvm::ir::Module, out: &mut String) {
    use super::llvm::ir::{Instruction, Origin as O};
    let origins: Vec<_> = module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .filter_map(|i| match (&i.instruction, i.origin) {
            (Instruction::Flush, O::Source { node_id, span }) => Some(Origin { node_id, span }),
            _ => None,
        })
        .collect();
    if origins.is_empty() {
        return;
    }
    let windows = module.target.expect("flush target") == super::llvm::Target::X86_64PcWindowsMsvc;
    for declaration in [
        "declare i32 @fflush(ptr)\n",
        "declare void @_Exit(i32) noreturn\n",
        if windows {
            "declare ptr @__acrt_iob_func(i32)\n"
        } else {
            "@stdout = external global ptr\n"
        },
        if windows {
            "declare i32 @_write(i32, ptr, i32)\n"
        } else {
            "declare i64 @write(i32, ptr, i64)\n"
        },
    ] {
        if !out.contains(declaration) {
            out.push_str(declaration);
        }
    }
    if !windows {
        out.push_str("declare ptr @signal(i32, ptr)\n");
    }
    for origin in origins {
        let id = origin.node_id.0;
        let record = format!("cerune: {}\n", flush_failure_record(origin));
        let bytes: String = record.bytes().map(|b| format!("\\{b:02X}")).collect();
        let n = record.len();
        writeln!(
            out,
            "@cerune.flush.error.{id} = private constant [{n} x i8] c\"{bytes}\""
        )
        .unwrap();
        writeln!(out, "define internal void @cerune.flush.{id}() {{\nentry:").unwrap();
        out.push_str(if windows {
            "  %stream = call ptr @__acrt_iob_func(i32 1)\n"
        } else {
            "  %stream = load ptr, ptr @stdout\n"
        });
        if !windows {
            out.push_str("  %prior = call ptr @signal(i32 13, ptr inttoptr (i64 1 to ptr))\n  %setup.failed = icmp eq ptr %prior, inttoptr (i64 -1 to ptr)\n  br i1 %setup.failed, label %failed, label %flush\nflush:\n");
        }
        out.push_str("  %status = call i32 @fflush(ptr %stream)\n  %ok = icmp eq i32 %status, 0\n  br i1 %ok, label %done, label %failed\nfailed:\n");
        if windows {
            writeln!(
                out,
                "  call i32 @_write(i32 2, ptr @cerune.flush.error.{id}, i32 {n})"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "  call i64 @write(i32 2, ptr @cerune.flush.error.{id}, i64 {n})"
            )
            .unwrap();
        }
        out.push_str("  call void @_Exit(i32 1)\n  unreachable\ndone:\n");
        if !windows {
            out.push_str("  %restored = call ptr @signal(i32 13, ptr %prior)\n  %restore.failed = icmp eq ptr %restored, inttoptr (i64 -1 to ptr)\n  br i1 %restore.failed, label %failed, label %return\nreturn:\n");
        }
        out.push_str("  ret void\n}\n");
    }
}

pub(super) fn qbe(module: &super::qbe::ir::Module, out: &mut String) {
    use super::qbe::ir::Instruction;
    let origins: Vec<_> = module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .filter_map(|i| match i {
            Instruction::Flush { origin } => Some(*origin),
            _ => None,
        })
        .collect();
    if origins.is_empty() {
        return;
    }
    let target = module.target.expect("flush target");
    let windows = target == super::qbe::Target::X86_64PcWindowsMsvc;
    for origin in origins {
        let id = origin.node.0;
        let record = format!(
            "cerune: {}\n",
            flush_failure_record(Origin {
                node_id: origin.node,
                span: origin.span
            })
        );
        let bytes = record
            .bytes()
            .map(|b| format!("b {b}"))
            .collect::<Vec<_>>()
            .join(", ");
        let section = if windows { ".rdata" } else { ".rodata" };
        writeln!(
            out,
            "section \"{section}\" data $cerune_flush_error_{id} = {{ {bytes} }}"
        )
        .unwrap();
        writeln!(out, "function $cerune_flush_{id}() {{\n@start").unwrap();
        out.push_str(if windows {
            "  %stream =l call $__acrt_iob_func(w 1)\n"
        } else {
            "  %stream =l loadl $stdout\n"
        });
        if !windows {
            out.push_str("  %prior =l call $signal(w 13, l 1)\n  %setup_bad =w ceql %prior, -1\n  jnz %setup_bad, @failed, @flush\n@flush\n");
        }
        out.push_str(
            "  %status =w call $fflush(l %stream)\n  jnz %status, @failed, @done\n@failed\n",
        );
        if windows {
            writeln!(
                out,
                "  call $_write(w 2, l $cerune_flush_error_{id}, w {})",
                record.len()
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "  call $write(w 2, l $cerune_flush_error_{id}, l {})",
                record.len()
            )
            .unwrap();
        }
        out.push_str("  call $_Exit(w 1)\n  hlt\n@done\n");
        if !windows {
            out.push_str("  %restored =l call $signal(w 13, l %prior)\n  %restore_bad =w ceql %restored, -1\n  jnz %restore_bad, @failed, @return\n@return\n");
        }
        out.push_str("  ret\n}\n");
    }
}
