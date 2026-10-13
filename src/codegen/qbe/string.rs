use super::ir::Module;
use std::fmt::Write;

pub(super) fn emit(module: &Module, output: &mut String) {
    for (id, value) in module.strings.iter().enumerate() {
        write!(
            output,
            "section \"{}\" data $cerune_string_{id} = align 8 {{ l {}",
            module
                .target
                .unwrap_or(super::Target::X86_64UnknownLinuxGnu)
                .read_only_section(),
            value.len()
        )
        .unwrap();
        for byte in value.bytes() {
            write!(output, ", b {byte}").unwrap();
        }
        output.push_str(" }\n");
    }
    output.push_str(SUPPORT);
}

// 長さを先に確認し、NULを含む全バイトを比較します。アドレス同士は比較しません。
const SUPPORT: &str = r#"
function w $cerune_string_equal(l %left, l %right) {
@start
  %length =l loadl %left
  %right_length =l loadl %right
  %same_length =w ceql %length, %right_length
  jnz %same_length, @condition, @different
@condition
  %index =l phi @start 0, @advance %next
  %done =w ceql %index, %length
  jnz %done, @equal, @compare
@compare
  %offset =l add %index, 8
  %lp =l add %left, %offset
  %rp =l add %right, %offset
  %lb =w loadub %lp
  %rb =w loadub %rp
  %same_byte =w ceqw %lb, %rb
  jnz %same_byte, @advance, @different
@advance
  %next =l add %index, 1
  jmp @condition
@equal
  ret 1
@different
  ret 0
}
"#;
