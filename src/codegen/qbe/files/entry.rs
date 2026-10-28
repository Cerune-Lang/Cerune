use super::super::Target;
pub(in crate::codegen::qbe) fn adapt(entry: &str) -> String {
    let begin = entry.find("  %has_name =w").expect("argument startup");
    let end = entry.find("  %empty =w").expect("argument count");
    (entry[..begin].to_owned() + CONFIGURATION + &entry[end..])
        .replace("  %source_offset =l mul %next, 8", "  %source_index =l add %i, %start\n  %source_offset =l mul %source_index, 8")
        .replace("  %result =w call $cerune_program()", "  call $cerune_files_connect_optional(l %root)\n  %result =w call $cerune_program()\n  call $cerune_files_disconnect()")
}
const CONFIGURATION: &str = r#"  %root_slot =l alloc8 8
  %start =l call $cerune_files_parse(w %argc, l %argv, l %root_slot)
  %root =l loadl %root_slot
  %argc_wide =l extsw %argc
  %has_values =w csgtl %argc_wide, %start
  jnz %has_values, @values_present, @values_absent
@values_present
  %remaining =l sub %argc_wide, %start
  jmp @size
@values_absent
  jmp @size
@size
  %count =l phi @values_present %remaining, @values_absent 0
"#;
pub(super) fn emit(target: Target, out: &mut String) {
    let section = target.read_only_section();
    for word in [
        "configure",
        "open-root",
        "invalid-root",
        "unknown-setting",
        "duplicate-setting",
        "missing-delimiter",
    ] {
        super::data(
            out,
            section,
            &format!("cerune_file_label_{}", word.replace('-', "_")),
            word,
        );
    }
    super::data(
        out,
        section,
        "cerune_files_startup_format",
        "cerune: file-v1 operation=connect phase=%s code=%s origin=host\n",
    );
    let windows = target == Target::X86_64PcWindowsMsvc;
    for (name, value) in [
        ("marker", "--cerune-runtime-v1"),
        ("root_option", "--file-root"),
        ("delimiter", "--"),
    ] {
        let name = format!("cerune_files_{name}");
        if windows {
            super::connection::wide_data(out, section, &name, value);
        } else {
            super::data(out, section, &name, value);
        }
    }
    out.push_str(FAILURE);
    out.push_str(if windows {
        "  call $_write(w 2, l %text, w %count)\n"
    } else {
        "  %wide =l extuw %count\n  call $write(w 2, l %text, l %wide)\n"
    });
    out.push_str("  jmp @exit\n@exit\n  call $_Exit(w 1)\n  hlt\n}\n");
    out.push_str(
        &PARSE
            .replace("@CMP@", if windows { "wcscmp" } else { "strcmp" })
            .replace("@LOAD@", if windows { "loaduh" } else { "loadub" })
            .replace("@STEP@", if windows { "2" } else { "1" }),
    );
}
const FAILURE: &str = r#"
function $cerune_files_startup_failed(l %phase, l %code) {
@start
  %text =l alloc8 256
  %length =w call $snprintf(l %text, l 256, l $cerune_files_startup_format, ..., l %phase, l %code)
  %positive =w csgtw %length, 0
  jnz %positive, @bound, @exit
@bound
  %fits =w cultw %length, 256
  jnz %fits, @fit, @truncate
@fit
  jmp @write
@truncate
  jmp @write
@write
  %count =w phi @fit %length, @truncate 255
"#;
const PARSE: &str = r#"
# runtime-startup-v1: only the leading marker introduces configuration.
function l $cerune_files_parse(w %argc, l %argv, l %selected_root) {
@start
  %index_slot =l alloc8 8
  %root_slot =l alloc8 8
  storel 0, %root_slot
  storel 0, %selected_root
  storel 2, %index_slot
  %argc_wide =l extsw %argc
  %has_argument =w csgtw %argc, 1
  jnz %has_argument, @marker, @unchanged
@marker
  %first_slot =l add %argv, 8
  %first =l loadl %first_slot
  %comparison =w call $@CMP@(l %first, l $cerune_files_marker)
  %is_marker =w ceqw %comparison, 0
  jnz %is_marker, @loop, @unchanged
@unchanged
  ret 1
@loop
  %index =l loadl %index_slot
  %end =w csgel %index, %argc_wide
  jnz %end, @missing_delimiter, @argument
@argument
  %offset =l mul %index, 8
  %slot =l add %argv, %offset
  %arg =l loadl %slot
  %delim_cmp =w call $@CMP@(l %arg, l $cerune_files_delimiter)
  %is_delimiter =w ceqw %delim_cmp, 0
  jnz %is_delimiter, @done, @option
@option
  %option_cmp =w call $@CMP@(l %arg, l $cerune_files_root_option)
  %recognized =w ceqw %option_cmp, 0
  jnz %recognized, @duplicate_check, @unknown
@duplicate_check
  %previous =l loadl %root_slot
  jnz %previous, @duplicate_setting, @value_check
@value_check
  %value_index =l add %index, 1
  %missing =w csgel %value_index, %argc_wide
  jnz %missing, @invalid_root, @value
@value
  %value_offset =l mul %value_index, 8
  %value_slot =l add %argv, %value_offset
  %root =l loadl %value_slot
  %first_char =w @LOAD@ %root
  %empty =w ceqw %first_char, 0
  jnz %empty, @invalid_root, @prefix
@prefix
  %second_ptr =l add %root, @STEP@
  %second_char =w @LOAD@ %second_ptr
  %first_dash =w ceqw %first_char, 45
  %second_dash =w ceqw %second_char, 45
  %looks_option =w and %first_dash, %second_dash
  jnz %looks_option, @invalid_root, @accept
@accept
  storel %root, %root_slot
  %next =l add %value_index, 1
  storel %next, %index_slot
  jmp @loop
@done
  %start =l add %index, 1
  %root_value =l loadl %root_slot
  storel %root_value, %selected_root
  ret %start
@missing_delimiter
  call $cerune_files_startup_failed(l $cerune_file_label_configure, l $cerune_file_label_missing_delimiter)
  hlt
@unknown
  call $cerune_files_startup_failed(l $cerune_file_label_configure, l $cerune_file_label_unknown_setting)
  hlt
@duplicate_setting
  call $cerune_files_startup_failed(l $cerune_file_label_configure, l $cerune_file_label_duplicate_setting)
  hlt
@invalid_root
  call $cerune_files_startup_failed(l $cerune_file_label_configure, l $cerune_file_label_invalid_root)
  hlt
}
"#;
