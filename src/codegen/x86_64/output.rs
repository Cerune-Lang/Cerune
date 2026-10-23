//! Native output construction; ABI, formatting and write semantics remain in Cerune.
use super::machine::*;
use Opcode::*;
use Operand::{Immediate as imm, RipRelative as rip, Symbol as sym};
#[cfg(test)]
pub(super) fn support(linux: bool) -> String {
    let target = if linux {
        super::Target::X86_64UnknownLinuxGnu
    } else {
        super::Target::X86_64PcWindowsMsvc
    };
    super::machine::render_support(target, append)
}
fn label(program: &mut Program, name: &str) {
    let id = local(program, name);
    push(program, EntryKind::Label(id));
}
// Internal entry: rax/xmm0=value, r10=newline, r11=origin.
// Frame: 0..48 external arguments, 64..96 requested/accepted/origin/SIGPIPE,
// 96..160 state, 160..416 body, 416..928 diagnostic. Calls are aligned to 16 bytes.
pub(super) fn append(program: &mut Program) {
    let linux = program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    push(program, EntryKind::Section(Section::ReadOnlyData));
    for (name, value) in [
        (
            "record",
            "cerune: io-v1 operation=write requested=%llu accepted=%llu%s",
        ),
        ("format_error", "cerune: output formatting failed%s"),
        ("i64", "%lld"),
        ("u64", "%llu"),
        ("f32", "%.9g"),
        ("f64", "%.17g"),
        ("true", "true\n"),
        ("false", "false\n"),
        ("lf", "\n"),
        ("quote", "\""),
    ] {
        label(program, &format!(".Loutput_{name}"));
        push(program, EntryKind::CString(value.to_owned()));
    }
    push(program, EntryKind::Section(Section::Text));

    // 接続のみ通常ABI。比較テストはこの関数の接続先を置換します。
    label(program, "cerune_output_write");
    instruction(program, Sub64, [imm(40), r64(4)]);

    if linux {
        instruction(program, Mov64, [r64(6), r64(2)]);
        instruction(program, Mov64, [r64(7), r64(6)]);
        instruction(program, Mov32, [imm(1), r32(7)]);
        let symbol = external(program, "write");
        instruction(program, Call, [sym(symbol)]);
    } else {
        instruction(program, Mov32, [r32(2), r32(8)]);
        instruction(program, Mov64, [r64(1), r64(2)]);
        instruction(program, Mov32, [imm(1), r32(1)]);
        let symbol = external(program, "_write");
        instruction(program, Call, [sym(symbol)]);
        instruction(program, ShiftLeft64, [imm(32), r64(0)]);
        instruction(program, ShiftArithmeticRight64, [imm(32), r64(0)]);
    }
    instruction(program, Add64, [imm(40), r64(4)]);
    instruction(program, Ret, []);

    append_part_start(program);
    if linux {
        instruction(program, Mov64, [memory(4, None, 56), r64(7)]);
        instruction(program, Mov64, [memory(4, None, 72), r64(6)]);
    } else {
        instruction(program, Mov64, [memory(4, None, 56), r64(1)]);
        instruction(program, Mov64, [memory(4, None, 72), r64(2)]);
    }
    let symbol = local(program, "cerune_output_write");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    let symbol = local(program, ".Loutput_part_check");
    instruction(program, JumpNs, [sym(symbol)]);

    if linux {
        let symbol = external(program, "__errno_location");
        instruction(program, Call, [sym(symbol)]);
    } else {
        let symbol = external(program, "_errno");
        instruction(program, Call, [sym(symbol)]);
    }
    instruction(program, Cmp32, [imm(4), memory(0, None, 0)]);
    let symbol = local(program, ".Loutput_part_write");
    instruction(program, JumpE, [sym(symbol)]);
    let symbol = local(program, ".Loutput_part_failed");
    instruction(program, Jump, [sym(symbol)]);

    append_part_end(program);
    label(program, "cerune_output_failed");
    instruction(program, Sub64, [imm(584), r64(4)]);

    if linux {
        instruction(program, Lea64, [memory(4, None, 64), r64(7)]);
        instruction(program, Mov32, [imm(512), r32(6)]);
        let symbol = local(program, ".Loutput_record");
        instruction(program, Lea64, [rip(symbol), r64(2)]);
        instruction(program, Mov64, [memory(10, None, 0), r64(1)]);
        instruction(program, Mov64, [memory(10, None, 8), r64(8)]);
        instruction(program, Mov64, [memory(10, None, 16), r64(9)]);
        instruction(program, Xor32, [r32(0), r32(0)]);
    } else {
        instruction(program, Lea64, [memory(4, None, 64), r64(1)]);
        instruction(program, Mov32, [imm(512), r32(2)]);
        let symbol = local(program, ".Loutput_record");
        instruction(program, Lea64, [rip(symbol), r64(8)]);
        instruction(program, Mov64, [memory(10, None, 0), r64(9)]);
        instruction(program, Mov64, [memory(10, None, 8), r64(0)]);
        instruction(program, Mov64, [r64(0), memory(4, None, 32)]);
        instruction(program, Mov64, [memory(10, None, 16), r64(0)]);
        instruction(program, Mov64, [r64(0), memory(4, None, 40)]);
    }
    let symbol = external(program, "snprintf");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_diagnostic");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Undefined, []);

    // 診断配送の失敗でも停止します。stdioの終了時flushは行いません。
    label(program, "cerune_output_diagnostic");
    instruction(program, Sub64, [imm(40), r64(4)]);
    instruction(program, Test32, [r32(0), r32(0)]);
    let symbol = local(program, ".Loutput_exit");
    instruction(program, JumpLe, [sym(symbol)]);
    instruction(program, Cmp32, [imm(512), r32(0)]);
    let symbol = local(program, ".Loutput_diag_write");
    instruction(program, JumpB, [sym(symbol)]);
    instruction(program, Mov32, [imm(511), r32(0)]);
    label(program, ".Loutput_diag_write");

    if linux {
        instruction(program, Mov32, [r32(0), r32(2)]);
        instruction(program, Mov64, [r64(10), r64(6)]);
        instruction(program, Mov32, [imm(2), r32(7)]);
        let symbol = external(program, "write");
        instruction(program, Call, [sym(symbol)]);
        label(program, ".Loutput_exit");
        instruction(program, Mov32, [imm(1), r32(7)]);
    } else {
        instruction(program, Mov32, [r32(0), r32(8)]);
        instruction(program, Mov64, [r64(10), r64(2)]);
        instruction(program, Mov32, [imm(2), r32(1)]);
        let symbol = external(program, "_write");
        instruction(program, Call, [sym(symbol)]);
        label(program, ".Loutput_exit");
        instruction(program, Mov32, [imm(1), r32(1)]);
    }
    let symbol = external(program, "_Exit");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Undefined, []);

    for kind in ["i64", "u64", "f32", "f64", "bool", "string", "quoted"] {
        label(program, &format!("cerune_output_{kind}"));

        if kind == "f32" {
            instruction(
                program,
                Cvtss2sd,
                [
                    Register::xmm(0).expect("fixed output register").into(),
                    Register::xmm(0).expect("fixed output register").into(),
                ],
            );
        }
        if matches!(kind, "f32" | "f64") {
            instruction(
                program,
                Mov64,
                [
                    Register::xmm(0).expect("fixed output register").into(),
                    r64(0),
                ],
            );
        }
        instruction(program, Sub64, [imm(936), r64(4)]);
        instruction(program, Mov64, [r64(0), memory(4, None, 96)]);
        instruction(program, Mov64, [r64(10), memory(4, None, 104)]);
        instruction(program, Mov64, [r64(11), memory(4, None, 80)]);

        let mode = match kind {
            "string" => 1,
            "quoted" => 2,
            _ => 0,
        };
        instruction(program, Mov64, [imm(mode), memory(4, None, 152)]);

        match kind {
            "bool" => append_bool(program),
            "string" => {
                instruction(program, Mov64, [memory(0, None, 0), r64(0)]);
                instruction(program, Add64, [memory(4, None, 104), r64(0)]);
                instruction(program, Mov64, [r64(0), memory(4, None, 64)]);
                let symbol = local(program, ".Loutput_begin");
                instruction(program, Jump, [sym(symbol)]);
            }
            "quoted" => append_count(program),
            _ => {
                if linux {
                    instruction(program, Lea64, [memory(4, None, 160), r64(7)]);
                    instruction(program, Mov32, [imm(128), r32(6)]);
                    let symbol = local(program, &format!(".Loutput_{kind}"));
                    instruction(program, Lea64, [rip(symbol), r64(2)]);

                    if matches!(kind, "f32" | "f64") {
                        instruction(program, Mov32, [imm(1), r32(0)]);
                    } else {
                        instruction(program, Mov64, [r64(0), r64(1)]);
                        instruction(program, Xor32, [r32(0), r32(0)]);
                    }
                } else {
                    instruction(program, Lea64, [memory(4, None, 160), r64(1)]);
                    instruction(program, Mov32, [imm(128), r32(2)]);
                    let symbol = local(program, &format!(".Loutput_{kind}"));
                    instruction(program, Lea64, [rip(symbol), r64(8)]);
                    instruction(program, Mov64, [r64(0), r64(9)]);

                    if matches!(kind, "f32" | "f64") {
                        instruction(
                            program,
                            Movapd,
                            [
                                Register::xmm(0).expect("fixed output register").into(),
                                Register::xmm(3).expect("fixed output register").into(),
                            ],
                        );
                    }
                }
                let symbol = external(program, "snprintf");
                instruction(program, Call, [sym(symbol)]);
                instruction(program, Cmp32, [imm(127), r32(0)]);
                let symbol = local(program, ".Loutput_format_failed");
                instruction(program, JumpAe, [sym(symbol)]);
                instruction(program, Lea64, [memory(4, None, 160), r64(1)]);
                instruction(program, Mov8, [imm(10), memory(1, Some(0), 0)]);
                instruction(program, Add64, [memory(4, None, 104), r64(0)]);
                instruction(program, Mov64, [r64(0), memory(4, None, 64)]);
                instruction(program, Mov64, [r64(1), memory(4, None, 96)]);
                let symbol = local(program, ".Loutput_begin");
                instruction(program, Jump, [sym(symbol)]);
            }
        }
    }
    label(program, ".Loutput_format_failed");

    if linux {
        instruction(program, Lea64, [memory(4, None, 416), r64(7)]);
        instruction(program, Mov32, [imm(512), r32(6)]);
        let symbol = local(program, ".Loutput_format_error");
        instruction(program, Lea64, [rip(symbol), r64(2)]);
        instruction(program, Mov64, [memory(4, None, 80), r64(1)]);
        instruction(program, Xor32, [r32(0), r32(0)]);
    } else {
        instruction(program, Lea64, [memory(4, None, 416), r64(1)]);
        instruction(program, Mov32, [imm(512), r32(2)]);
        let symbol = local(program, ".Loutput_format_error");
        instruction(program, Lea64, [rip(symbol), r64(8)]);
        instruction(program, Mov64, [memory(4, None, 80), r64(9)]);
    }
    let symbol = external(program, "snprintf");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Lea64, [memory(4, None, 416), r64(10)]);
    let symbol = local(program, "cerune_output_diagnostic");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Undefined, []);
    label(program, ".Loutput_begin");
    instruction(program, Mov64, [imm(0), memory(4, None, 72)]);

    if linux {
        instruction(program, Mov32, [imm(13), r32(7)]);
        instruction(program, Mov32, [imm(1), r32(6)]);
        let symbol = external(program, "signal");
        instruction(program, Call, [sym(symbol)]);
        instruction(program, Cmp64, [imm(-1), r64(0)]);
        let symbol = local(program, ".Loutput_failed");
        instruction(program, JumpE, [sym(symbol)]);
        instruction(program, Mov64, [r64(0), memory(4, None, 88)]);
    }
    append_dispatch(program);
    label(program, ".Loutput_end");

    if linux {
        instruction(program, Mov32, [imm(13), r32(7)]);
        instruction(program, Mov64, [memory(4, None, 88), r64(6)]);
        let symbol = external(program, "signal");
        instruction(program, Call, [sym(symbol)]);
        instruction(program, Cmp64, [imm(-1), r64(0)]);
        let symbol = local(program, ".Loutput_failed");
        instruction(program, JumpE, [sym(symbol)]);
    }
    instruction(program, Add64, [imm(936), r64(4)]);
    instruction(program, Ret, []);
    label(program, ".Loutput_failed");
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_failed");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Undefined, []);

    // バイトの変換のみ。OS接続やプログラム状態へアクセスしません。
    label(program, "cerune_output_escape");

    let escapes: Vec<_> = (0..=127u8)
        .filter_map(|b| {
            let text = match b {
                0 => "\\0".into(),
                9 => "\\t".into(),
                10 => "\\n".into(),
                13 => "\\r".into(),
                34 => "\\\"".into(),
                92 => "\\\\".into(),
                1..=31 | 127 => format!("\\u{{{b:02x}}}"),
                _ => return None,
            };
            Some((b, text))
        })
        .collect();
    for (b, _) in &escapes {
        instruction(program, Cmp32, [imm(i128::from(*b)), r32(0)]);
        let symbol = local(program, &format!(".Loutput_escape_{b}"));
        instruction(program, JumpE, [sym(symbol)]);
    }
    instruction(program, Mov8, [r8(0), memory(1, None, 0)]);
    instruction(program, Mov32, [imm(1), r32(0)]);
    instruction(program, Ret, []);

    for (b, text) in escapes {
        label(program, &format!(".Loutput_escape_{b}"));

        for (i, c) in text.bytes().enumerate() {
            instruction(
                program,
                Mov8,
                [imm(i128::from(c)), memory(1, None, i as i32)],
            );
        }
        instruction(program, Mov32, [imm(text.len() as i128), r32(0)]);
        instruction(program, Ret, []);
    }
}
fn append_part_start(program: &mut Program) {
    label(program, "cerune_output_part");
    instruction(program, Sub64, [imm(88), r64(4)]);
    instruction(program, Mov64, [r64(10), memory(4, None, 48)]);
    instruction(program, Mov64, [r64(0), memory(4, None, 56)]);
    instruction(program, Mov64, [r64(2), memory(4, None, 64)]);
    label(program, ".Loutput_part_loop");
    instruction(program, Cmp64, [imm(0), memory(4, None, 64)]);
    let symbol = local(program, ".Loutput_part_done");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, Mov64, [memory(4, None, 64), r64(0)]);
    instruction(program, Cmp64, [imm(65536), r64(0)]);
    let symbol = local(program, ".Loutput_part_bound");
    instruction(program, JumpBe, [sym(symbol)]);
    instruction(program, Mov32, [imm(65536), r32(0)]);
    label(program, ".Loutput_part_bound");
    instruction(program, Mov64, [r64(0), memory(4, None, 72)]);
    label(program, ".Loutput_part_write");
}
fn append_part_end(program: &mut Program) {
    label(program, ".Loutput_part_check");
    instruction(program, Test64, [r64(0), r64(0)]);
    let symbol = local(program, ".Loutput_part_failed");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, Cmp64, [memory(4, None, 72), r64(0)]);
    let symbol = local(program, ".Loutput_part_failed");
    instruction(program, JumpA, [sym(symbol)]);
    instruction(program, Mov64, [memory(4, None, 48), r64(10)]);
    instruction(program, Add64, [r64(0), memory(10, None, 8)]);
    instruction(program, Add64, [r64(0), memory(4, None, 56)]);
    instruction(program, Sub64, [r64(0), memory(4, None, 64)]);
    let symbol = local(program, ".Loutput_part_loop");
    instruction(program, Jump, [sym(symbol)]);
    label(program, ".Loutput_part_done");
    instruction(program, Add64, [imm(88), r64(4)]);
    instruction(program, Ret, []);
    label(program, ".Loutput_part_failed");
    instruction(program, Mov64, [memory(4, None, 48), r64(10)]);
    let symbol = local(program, "cerune_output_failed");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Undefined, []);
}
fn append_bool(program: &mut Program) {
    instruction(program, Test64, [r64(0), r64(0)]);
    let symbol = local(program, ".Loutput_bool_false");
    instruction(program, JumpE, [sym(symbol)]);
    let symbol = local(program, ".Loutput_true");
    instruction(program, Lea64, [rip(symbol), r64(0)]);
    instruction(program, Mov64, [imm(4), memory(4, None, 64)]);
    let symbol = local(program, ".Loutput_bool_ready");
    instruction(program, Jump, [sym(symbol)]);
    label(program, ".Loutput_bool_false");
    let symbol = local(program, ".Loutput_false");
    instruction(program, Lea64, [rip(symbol), r64(0)]);
    instruction(program, Mov64, [imm(5), memory(4, None, 64)]);
    label(program, ".Loutput_bool_ready");
    instruction(program, Mov64, [r64(0), memory(4, None, 96)]);
    instruction(program, Mov64, [memory(4, None, 104), r64(0)]);
    instruction(program, Add64, [r64(0), memory(4, None, 64)]);
    let symbol = local(program, ".Loutput_begin");
    instruction(program, Jump, [sym(symbol)]);
}
fn append_count(program: &mut Program) {
    instruction(program, Mov64, [imm(0), memory(4, None, 112)]);
    instruction(program, Mov64, [memory(4, None, 104), r64(0)]);
    instruction(program, Add64, [imm(2), r64(0)]);
    instruction(program, Mov64, [r64(0), memory(4, None, 64)]);
    label(program, ".Loutput_count");
    instruction(program, Mov64, [memory(4, None, 96), r64(2)]);
    instruction(program, Mov64, [memory(4, None, 112), r64(0)]);
    instruction(program, Cmp64, [memory(2, None, 0), r64(0)]);
    let symbol = local(program, ".Loutput_begin");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, ZeroExtend8To32, [memory(2, Some(0), 8), r32(0)]);
    instruction(program, Lea64, [memory(4, None, 160), r64(1)]);
    let symbol = local(program, "cerune_output_escape");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Add64, [r64(0), memory(4, None, 64)]);
    instruction(program, Inc64, [memory(4, None, 112)]);
    let symbol = local(program, ".Loutput_count");
    instruction(program, Jump, [sym(symbol)]);
}
fn append_dispatch(program: &mut Program) {
    instruction(program, Cmp64, [imm(2), memory(4, None, 152)]);
    let symbol = local(program, ".Loutput_quoted");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, Cmp64, [imm(1), memory(4, None, 152)]);
    let symbol = local(program, ".Loutput_raw");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, Mov64, [memory(4, None, 96), r64(0)]);
    instruction(program, Mov64, [memory(4, None, 64), r64(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    let symbol = local(program, ".Loutput_end");
    instruction(program, Jump, [sym(symbol)]);
    label(program, ".Loutput_raw");
    instruction(program, Mov64, [memory(4, None, 96), r64(0)]);
    instruction(program, Mov64, [memory(0, None, 0), r64(2)]);
    instruction(program, Add64, [imm(8), r64(0)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    let symbol = local(program, ".Loutput_newline");
    instruction(program, Jump, [sym(symbol)]);
    label(program, ".Loutput_quoted");
    let symbol = local(program, ".Loutput_quote");
    instruction(program, Lea64, [rip(symbol), r64(0)]);
    instruction(program, Mov32, [imm(1), r32(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Mov64, [imm(0), memory(4, None, 112)]);
    instruction(program, Mov64, [imm(0), memory(4, None, 120)]);
    label(program, ".Loutput_quote_loop");
    instruction(program, Mov64, [memory(4, None, 96), r64(2)]);
    instruction(program, Mov64, [memory(4, None, 112), r64(0)]);
    instruction(program, Cmp64, [memory(2, None, 0), r64(0)]);
    let symbol = local(program, ".Loutput_quote_tail");
    instruction(program, JumpE, [sym(symbol)]);
    instruction(program, Cmp64, [imm(250), memory(4, None, 120)]);
    let symbol = local(program, ".Loutput_quote_byte");
    instruction(program, JumpBe, [sym(symbol)]);
    instruction(program, Lea64, [memory(4, None, 160), r64(0)]);
    instruction(program, Mov64, [memory(4, None, 120), r64(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Mov64, [imm(0), memory(4, None, 120)]);
    label(program, ".Loutput_quote_byte");
    instruction(program, Mov64, [memory(4, None, 96), r64(2)]);
    instruction(program, Mov64, [memory(4, None, 112), r64(0)]);
    instruction(program, ZeroExtend8To32, [memory(2, Some(0), 8), r32(0)]);
    instruction(program, Lea64, [memory(4, None, 160), r64(1)]);
    instruction(program, Add64, [memory(4, None, 120), r64(1)]);
    let symbol = local(program, "cerune_output_escape");
    instruction(program, Call, [sym(symbol)]);
    instruction(program, Add64, [r64(0), memory(4, None, 120)]);
    instruction(program, Inc64, [memory(4, None, 112)]);
    let symbol = local(program, ".Loutput_quote_loop");
    instruction(program, Jump, [sym(symbol)]);
    label(program, ".Loutput_quote_tail");
    instruction(program, Lea64, [memory(4, None, 160), r64(0)]);
    instruction(program, Mov64, [memory(4, None, 120), r64(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    let symbol = local(program, ".Loutput_quote");
    instruction(program, Lea64, [rip(symbol), r64(0)]);
    instruction(program, Mov32, [imm(1), r32(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    label(program, ".Loutput_newline");
    let symbol = local(program, ".Loutput_lf");
    instruction(program, Lea64, [rip(symbol), r64(0)]);
    instruction(program, Mov64, [memory(4, None, 104), r64(2)]);
    instruction(program, Lea64, [memory(4, None, 64), r64(10)]);
    let symbol = local(program, "cerune_output_part");
    instruction(program, Call, [sym(symbol)]);
    let symbol = local(program, ".Loutput_end");
    instruction(program, Jump, [sym(symbol)]);
}

// Generated native objects are ELF/COFF; Darwin Mach-O support is separate.
#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod native_object_output {
    use super::super::test_process as process;
    use crate as cerune_lang;
    const NATIVE: bool = true;
    const QBE: bool = false;
    const LLVM: bool = false;
    const ENCODER: Option<fn(&str) -> Vec<u8>> = Some(encode);
    fn encode(assembly: &str) -> Vec<u8> {
        super::super::object::assemble(
            assembly,
            if cfg!(windows) {
                super::super::Target::X86_64PcWindowsMsvc
            } else {
                super::super::Target::X86_64UnknownLinuxGnu
            },
        )
        .unwrap()
    }
    include!("../../../tests/support/generated_output.rs");
}

#[cfg(test)]
#[path = "../../../tests/support/typed_native_output.rs"]
pub(super) mod tests;
