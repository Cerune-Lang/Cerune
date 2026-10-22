use super::machine::{Linkage, Opcode, Operand, r32, r64};
use super::program::{NativeProgram, address};
use super::{Target, ir::Origin};
use crate::runtime::{FailureCode, RuntimeFailure};

/// 出自は不変データとして出力操作・停止分岐へ渡します。グローバルな可変の「現在位置」は作りません。
pub(super) struct Reporter {
    pub origin: Origin,
    target: Target,
    count: usize,
    pub data: Vec<(String, String, super::machine::Provenance)>,
}

impl Reporter {
    pub fn new(target: Target) -> Self {
        Self {
            origin: Origin::Synthetic,
            target,
            count: 0,
            data: Vec::new(),
        }
    }

    pub fn output(&mut self, kind: &str, newline: bool, output: &mut NativeProgram) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("output requires a source origin");
        };
        let label = format!(".Lcerune_output_origin_{}", self.count);
        self.count += 1;
        self.data.push((
            label.clone(),
            format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
            output.provenance(),
        ));

        output.emit(
            Opcode::Mov32,
            [
                Operand::Immediate((u8::from(newline)).to_owned() as i128),
                r32(10),
            ],
        );

        let symbol = output.symbol(&label, Linkage::Local);
        output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(11)]);

        let symbol = output.symbol(&format!("cerune_output_{kind}"), Linkage::Local);
        output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
    }

    pub fn argument(&mut self, get: bool, output: &mut NativeProgram) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("arguments need source origin")
        };
        let label = format!(".Lcerune_arguments_origin_{}", self.count);
        self.count += 1;
        self.data.push((
            label.clone(),
            format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
            output.provenance(),
        ));
        let operation = if get { "get" } else { "count" };

        let symbol = output.symbol(&label, Linkage::Local);
        output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(11)]);

        let symbol = output.symbol(&format!("cerune_arguments_{operation}"), Linkage::Local);
        output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
    }

    pub fn input(&mut self, output: &mut NativeProgram) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("input needs source origin")
        };
        let label = format!(".Lcerune_input_origin_{}", self.count);
        self.count += 1;
        self.data.push((
            label.clone(),
            format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
            output.provenance(),
        ));

        let symbol = output.symbol(&label, Linkage::Local);
        output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(11)]);

        let symbol = output.symbol("cerune_input_read_line", Linkage::Local);
        output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
    }

    pub fn flush(&mut self, output: &mut NativeProgram) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("flush needs source origin")
        };
        let record = format!(
            "cerune: {}\n",
            crate::runtime::output::flush_failure_record(crate::runtime::output::Origin {
                node_id,
                span
            })
        );
        let id = self.count;
        self.count += 1;
        let label = format!(".Lcerune_flush_failure_{id}");
        self.data
            .push((label.clone(), record.clone(), output.provenance()));
        if self.target.is_linux() {
            output.emit(Opcode::Sub64, [Operand::Immediate(16), r64(4)]);
            output.emit(Opcode::Mov32, [Operand::Immediate(13), r32(7)]);
            output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(6)]);

            let symbol = output.symbol("signal", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cmp64, [Operand::Immediate(-1), r64(0)]);

            let symbol = output.symbol(&format!(".Lcerune_flush_failed_{id}"), Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov64, [r64(0), address(4, None, 0)]);

            let symbol = output.symbol("stdout", Linkage::External);
            output.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(7)]);

            let symbol = output.symbol("fflush", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Test32, [r32(0), r32(0)]);

            let symbol = output.symbol(&format!(".Lcerune_flush_failed_{id}"), Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov64, [address(4, None, 0), r64(6)]);
            output.emit(Opcode::Mov32, [Operand::Immediate(13), r32(7)]);

            let symbol = output.symbol("signal", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cmp64, [Operand::Immediate(-1), r64(0)]);

            let symbol = output.symbol(&format!(".Lcerune_flush_failed_{id}"), Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Add64, [Operand::Immediate(16), r64(4)]);

            let symbol = output.symbol(&format!(".Lcerune_flush_done_{id}"), Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&format!(".Lcerune_flush_failed_{id}"));
            output.emit(Opcode::Add64, [Operand::Immediate(16), r64(4)]);
            output.emit(Opcode::Mov32, [Operand::Immediate(2), r32(7)]);

            let symbol = output.symbol(&label, Linkage::Local);
            output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(6)]);

            output.emit(
                Opcode::Mov32,
                [
                    Operand::Immediate((record.len()).to_owned() as i128),
                    r32(2),
                ],
            );

            let symbol = output.symbol("write", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(7)]);

            let symbol = output.symbol("_Exit", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Undefined, []);
            output.label(&format!(".Lcerune_flush_done_{id}"));
        } else {
            output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(1)]);

            let symbol = output.symbol("__acrt_iob_func", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov64, [r64(0), r64(1)]);

            let symbol = output.symbol("fflush", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Test32, [r32(0), r32(0)]);

            let symbol = output.symbol(&format!(".Lcerune_flush_done_{id}"), Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov32, [Operand::Immediate(2), r32(1)]);

            let symbol = output.symbol(&label, Linkage::Local);
            output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

            output.emit(
                Opcode::Mov32,
                [
                    Operand::Immediate((record.len()).to_owned() as i128),
                    r32(8),
                ],
            );

            let symbol = output.symbol("_write", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov32, [Operand::Immediate(1), r32(1)]);

            let symbol = output.symbol("_Exit", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Undefined, []);
            output.label(&format!(".Lcerune_flush_done_{id}"));
        }
    }

    pub fn emit(&mut self, code: FailureCode, output: &mut NativeProgram) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("language failure requires a source origin");
        };
        let record = format!(
            "cerune: {}\n",
            RuntimeFailure {
                code,
                node_id,
                span
            }
            .record()
        );
        let label = format!(".Lcerune_failure_{}", self.count);
        self.count += 1;
        self.data
            .push((label.clone(), record.clone(), output.provenance()));
        // 既に実行したprintを保持してから、明示したOSのCRTでstderrへ記録します。
        // stderrへの書き込みに失敗しても、元の異常停止を必ず実行します。
        if self.target.is_linux() {
            output.emit(Opcode::Xor32, [r32(7), r32(7)]);

            let symbol = output.symbol("fflush", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov32, [Operand::Immediate(2), r32(7)]);

            let symbol = output.symbol(&label, Linkage::Local);
            output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(6)]);

            output.emit(
                Opcode::Mov32,
                [
                    Operand::Immediate((record.len()).to_owned() as i128),
                    r32(2),
                ],
            );

            let symbol = output.symbol("write", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
        } else {
            output.emit(Opcode::Xor32, [r32(1), r32(1)]);

            let symbol = output.symbol("fflush", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov32, [Operand::Immediate(2), r32(1)]);

            let symbol = output.symbol(&label, Linkage::Local);
            output.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

            output.emit(
                Opcode::Mov32,
                [
                    Operand::Immediate((record.len()).to_owned() as i128),
                    r32(8),
                ],
            );

            let symbol = output.symbol("_write", Linkage::External);
            output.emit(Opcode::Call, [Operand::Symbol(symbol)]);
        }
        if self.target.is_linux() {
            output.emit(Opcode::Undefined, []);
        } else {
            // Windowsの例外巻き戻しに依存せず、FAST_FAIL_FATAL_APP_EXITで停止します。
            // rcx=7はOS側の停止方法。言語上の理由と出自は直前のruntime-v1に残します。

            output.emit(Opcode::Mov32, [Operand::Immediate(7), r32(1)]);
            output.emit(Opcode::Interrupt, [Operand::Immediate(0x29)]);
        }
    }
}
