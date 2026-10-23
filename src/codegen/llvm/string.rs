use std::fmt::Write;

use super::{Target, ir::Module};

pub(super) fn emit_data(module: &Module, output: &mut String) {
    output.push_str("%cerune.string = type { ptr, i64 }\n");
    for (id, value) in module.strings.iter().enumerate() {
        write!(
            output,
            "@cerune.string.{id} = private unnamed_addr constant [{} x i8] c\"",
            value.len()
        )
        .unwrap();
        // UTF-8の各バイトを固定2桁の16進表記にし、NULもそのまま保持します。
        for byte in value.bytes() {
            write!(output, "\\{byte:02X}").unwrap();
        }
        output.push_str("\"\n");
    }
    output.push('\n');
}

pub(super) fn emit_support(module: &Module, output: &mut String) {
    if module.target == Some(Target::X86_64PcWindowsMsvc) {
        output.push_str("declare i32 @_setmode(i32, i32)\n");
    }
    if module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(&i.instruction, super::ir::Instruction::StringByteAt { .. }))
    {
        output.push_str(BYTE_AT);
    }
    output.push_str(SUPPORT);
}

// 比較をバイト数で制御します。空文字列ではポインタを読みません。
const SUPPORT: &str = r#"define internal i1 @cerune.string.equal(%cerune.string %left, %cerune.string %right) {
entry:
  %left.data = extractvalue %cerune.string %left, 0
  %left.length = extractvalue %cerune.string %left, 1
  %right.data = extractvalue %cerune.string %right, 0
  %right.length = extractvalue %cerune.string %right, 1
  %same.length = icmp eq i64 %left.length, %right.length
  br i1 %same.length, label %condition, label %different
condition:
  %index = phi i64 [ 0, %entry ], [ %next, %advance ]
  %done = icmp eq i64 %index, %left.length
  br i1 %done, label %equal, label %compare
compare:
  %left.ptr = getelementptr inbounds i8, ptr %left.data, i64 %index
  %right.ptr = getelementptr inbounds i8, ptr %right.data, i64 %index
  %left.byte = load i8, ptr %left.ptr
  %right.byte = load i8, ptr %right.ptr
  %same.byte = icmp eq i8 %left.byte, %right.byte
  br i1 %same.byte, label %advance, label %different
advance:
  %next = add i64 %index, 1
  br label %condition
equal:
  ret i1 true
different:
  ret i1 false
}
"#;

const BYTE_AT: &str = r#"
define internal i64 @cerune.string.byte_at(%cerune.string %value, i64 %index) {
entry:
  %length = extractvalue %cerune.string %value, 1
  %negative = icmp slt i64 %index, 0
  %outside = icmp uge i64 %index, %length
  %invalid = or i1 %negative, %outside
  br i1 %invalid, label %missing, label %read
read:
  %data = extractvalue %cerune.string %value, 0
  %address = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %address
  %result = zext i8 %byte to i64
  ret i64 %result
missing:
  ret i64 -1
}
"#;
