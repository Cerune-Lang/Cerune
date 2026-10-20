use super::{Target, ir::Origin};
use crate::runtime::{FailureCode, RuntimeFailure};

/// 出自は不変データとして出力操作・停止分岐へ渡します。グローバルな可変の「現在位置」は作りません。
pub(super) struct Reporter {
    pub origin: Origin,
    target: Target,
    count: usize,
    pub data: String,
}

impl Reporter {
    pub fn new(target: Target) -> Self {
        Self {
            origin: Origin::Synthetic,
            target,
            count: 0,
            data: String::new(),
        }
    }

    pub fn output(&mut self, kind: &str, newline: bool, output: &mut String) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("output requires a source origin");
        };
        let label = format!(".Lcerune_output_origin_{}", self.count);
        self.count += 1;
        self.data.push_str(&format!(
            "{label}:\n  .asciz \" node={}{} bytes={}..{}\\n\"\n",
            node_id.0,
            span.source_id().record_field(),
            span.start(),
            span.end()
        ));
        output.push_str(&format!(
            "  movl ${}, %r10d\n  leaq {label}(%rip), %r11\n  callq cerune_output_{kind}\n",
            u8::from(newline)
        ));
    }

    pub fn argument(&mut self, get: bool, output: &mut String) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("arguments need source origin")
        };
        let label = format!(".Lcerune_arguments_origin_{}", self.count);
        self.count += 1;
        self.data.push_str(&format!(
            "{label}:\n  .asciz \" node={}{} bytes={}..{}\\n\"\n",
            node_id.0,
            span.source_id().record_field(),
            span.start(),
            span.end()
        ));
        let operation = if get { "get" } else { "count" };
        output.push_str(&format!(
            "  leaq {label}(%rip), %r11\n  callq cerune_arguments_{operation}\n"
        ));
    }

    pub fn input(&mut self, output: &mut String) {
        let Origin::Source { node_id, span } = self.origin else {
            panic!("input needs source origin")
        };
        let label = format!(".Lcerune_input_origin_{}", self.count);
        self.count += 1;
        self.data.push_str(&format!(
            "{label}:\n  .asciz \" node={}{} bytes={}..{}\\n\"\n",
            node_id.0,
            span.source_id().record_field(),
            span.start(),
            span.end()
        ));
        output.push_str(&format!(
            "  leaq {label}(%rip), %r11\n  callq cerune_input_read_line\n"
        ));
    }

    pub fn flush(&mut self, output: &mut String) {
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
        self.data.push_str(&format!(
            "{label}:\n  .asciz \"{}\"\n",
            record.replace('\n', "\\n")
        ));
        if self.target.is_linux() {
            output.push_str(&format!("  subq $16, %rsp\n  movl $13, %edi\n  movl $1, %esi\n  callq signal\n  cmpq $-1, %rax\n  je .Lcerune_flush_failed_{id}\n  movq %rax, (%rsp)\n  movq stdout(%rip), %rdi\n  callq fflush\n  testl %eax, %eax\n  jne .Lcerune_flush_failed_{id}\n  movq (%rsp), %rsi\n  movl $13, %edi\n  callq signal\n  cmpq $-1, %rax\n  je .Lcerune_flush_failed_{id}\n  addq $16, %rsp\n  jmp .Lcerune_flush_done_{id}\n.Lcerune_flush_failed_{id}:\n  addq $16, %rsp\n  movl $2, %edi\n  leaq {label}(%rip), %rsi\n  movl ${}, %edx\n  callq write\n  movl $1, %edi\n  callq _Exit\n  ud2\n.Lcerune_flush_done_{id}:\n", record.len()));
        } else {
            output.push_str(&format!("  movl $1, %ecx\n  callq __acrt_iob_func\n  movq %rax, %rcx\n  callq fflush\n  testl %eax, %eax\n  je .Lcerune_flush_done_{id}\n  movl $2, %ecx\n  leaq {label}(%rip), %rdx\n  movl ${}, %r8d\n  callq _write\n  movl $1, %ecx\n  callq _Exit\n  ud2\n.Lcerune_flush_done_{id}:\n", record.len()));
        }
    }

    pub fn emit(&mut self, code: FailureCode, output: &mut String) {
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
        self.data.push_str(&format!(
            "{label}:\n  .asciz \"{}\"\n",
            record.replace('\n', "\\n")
        ));
        // 既に実行したprintを保持してから、明示したOSのCRTでstderrへ記録します。
        // stderrへの書き込みに失敗しても、元の異常停止を必ず実行します。
        if self.target.is_linux() {
            output.push_str(&format!("  xorl %edi, %edi\n  callq fflush\n  movl $2, %edi\n  leaq {label}(%rip), %rsi\n  movl ${}, %edx\n  callq write\n", record.len()));
        } else {
            output.push_str(&format!("  xorl %ecx, %ecx\n  callq fflush\n  movl $2, %ecx\n  leaq {label}(%rip), %rdx\n  movl ${}, %r8d\n  callq _write\n", record.len()));
        }
        if self.target.is_linux() {
            output.push_str("  ud2\n");
        } else {
            // Windowsの例外巻き戻しに依存せず、FAST_FAIL_FATAL_APP_EXITで停止します。
            // rcx=7はOS側の停止方法。言語上の理由と出自は直前のruntime-v1に残します。
            output.push_str("  movl $7, %ecx\n  int $0x29\n");
        }
    }
}
