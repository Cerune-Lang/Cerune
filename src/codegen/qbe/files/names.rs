//! 単一成分の名前を変換せずに検査します。
use std::fmt::Write;
pub(super) fn emit(out: &mut String) {
    out.push_str(SOURCE);
    for (i, b) in [127, 60, 62, 58, 34, 47, 92, 124, 63, 42]
        .iter()
        .enumerate()
    {
        writeln!(out, "@punctuation_{i}\n  %punctuation_{i} =w ceqw %b, {b}\n  jnz %punctuation_{i}, @invalid, @punctuation_{}", i+1).unwrap();
    }
    out.push_str(STEM);
    for (i, word) in ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .enumerate()
    {
        writeln!(out, "@reserved_{i}\n  %matches_{i} =w call $cerune_files_ascii_equal(l %data, l %stem, l $cerune_file_label_{}, l {})\n  jnz %matches_{i}, @invalid, @reserved_{}", word.replace('$', "_"), word.len(), i+1).unwrap();
    }
    out.push_str(TAIL);
}
const SOURCE: &str = r#"
function w $cerune_files_ascii_equal(l %data, l %length, l %word, l %size) {
@start
  %same_length =w ceql %length, %size
  jnz %same_length, @loop, @no
@loop
  %i =l phi @start 0, @compare %next
  %end =w ceql %i, %length
  jnz %end, @yes, @byte
@byte
  %p =l add %data, %i
  %b =w loadub %p
  %from_a =w sub %b, 97
  %lower =w cultw %from_a, 26
  jnz %lower, @upper, @unchanged
@upper
  %uppercase =w sub %b, 32
  jmp @expected
@unchanged
  jmp @expected
@expected
  %c =w phi @upper %uppercase, @unchanged %b
  %q =l add %word, %i
  %expected =w loadub %q
  %match =w ceqw %c, %expected
  jnz %match, @compare, @no
@compare
  %next =l add %i, 1
  jmp @loop
@yes
  ret 1
@no
  ret 0
}
function w $cerune_files_name_valid(l %name) {
@start
  %data =l add %name, 8
  %length =l loadl %name
  %zero =w ceql %length, 0
  %long =w cugtl %length, 255
  %bad_length =w or %zero, %long
  jnz %bad_length, @invalid, @last
@last
  %last_index =l sub %length, 1
  %last_p =l add %data, %last_index
  %last_b =w loadub %last_p
  %last_space =w ceqw %last_b, 32
  %last_dot =w ceqw %last_b, 46
  %bad_last =w or %last_space, %last_dot
  jnz %bad_last, @invalid, @scan
@scan
  %i =l phi @last 0, @advance %next
  %end =w ceql %i, %length
  jnz %end, @stem_loop, @byte
@byte
  %p =l add %data, %i
  %b =w loadub %p
  %control =w cultw %b, 32
  jnz %control, @invalid, @punctuation_0
"#;
const STEM: &str = r#"
@punctuation_10
  jmp @advance
@advance
  %next =l add %i, 1
  jmp @scan
@stem_loop
  %s =l phi @scan 0, @stem_advance %s_next
  %s_end =w ceql %s, %length
  jnz %s_end, @trim, @stem_byte
@stem_byte
  %s_p =l add %data, %s
  %s_b =w loadub %s_p
  %dot =w ceqw %s_b, 46
  jnz %dot, @trim, @stem_advance
@stem_advance
  %s_next =l add %s, 1
  jmp @stem_loop
@trim
  %stem =l phi @stem_loop %s, @stem_byte %s, @trim_byte %previous
  %empty =w ceql %stem, 0
  jnz %empty, @reserved_0, @trim_byte
@trim_byte
  %previous =l sub %stem, 1
  %t_p =l add %data, %previous
  %t_b =w loadub %t_p
  %space =w ceqw %t_b, 32
  jnz %space, @trim, @reserved_0
"#;
const TAIL: &str = r#"
@reserved_6
  %enough =w cugel %stem, 4
  jnz %enough, @device, @valid
@device
  %com =w call $cerune_files_ascii_equal(l %data, l 3, l $cerune_file_label_COM, l 3)
  %lpt =w call $cerune_files_ascii_equal(l %data, l 3, l $cerune_file_label_LPT, l 3)
  %prefix =w or %com, %lpt
  jnz %prefix, @number, @valid
@number
  %digit_p =l add %data, 3
  %digit =w loadub %digit_p
  %four =w ceql %stem, 4
  jnz %four, @ascii, @superscript_check
@ascii
  %offset =w sub %digit, 49
  %reserved =w cultw %offset, 9
  jnz %reserved, @invalid, @valid
@superscript_check
  %five =w ceql %stem, 5
  %lead =w ceqw %digit, 194
  %superscript =w and %five, %lead
  jnz %superscript, @superscript_tail, @valid
@superscript_tail
  %tail_p =l add %data, 4
  %tail =w loadub %tail_p
  %one =w ceqw %tail, 185
  %two =w ceqw %tail, 178
  %three =w ceqw %tail, 179
  %two_three =w or %two, %three
  %sup_reserved =w or %one, %two_three
  jnz %sup_reserved, @invalid, @valid
@valid
  ret 1
@invalid
  ret 0
}
function l $cerune_files_check(l %name, l %limit) {
@start
  %bad_limit =w cugtl %limit, 2147483647
  jnz %bad_limit, @invalid_limit, @check_name
@check_name
  %valid =w call $cerune_files_name_valid(l %name)
  jnz %valid, @connection, @invalid_name
@connection
  %configured =w call $cerune_file_host_configured()
  jnz %configured, @yes, @no
@yes
  ret 0
@no
  ret $cerune_file_label_unconfigured
@invalid_limit
  ret $cerune_file_label_invalid_limit
@invalid_name
  ret $cerune_file_label_invalid_name
}
function l $cerune_files_open_file(l %name, w %writing, l %handle) {
@start
  %data =l add %name, 8
  %length =l loadl %name
  %status =w call $cerune_file_host_open(l %data, l %length, w %writing, l %handle)
  %ok =w ceqw %status, 0
  jnz %ok, @ok, @check_missing
@check_missing
  %missing =w ceqw %status, 1
  jnz %missing, @missing, @check_exists
@check_exists
  %exists =w ceqw %status, 2
  jnz %exists, @exists, @check_unsupported
@check_unsupported
  %unsupported =w ceqw %status, 3
  jnz %unsupported, @unsupported, @check_failed
@check_failed
  %failed =w ceqw %status, 4
  jnz %failed, @failed, @invalid
@ok
  ret 0
@missing
  ret $cerune_file_label_not_found
@exists
  ret $cerune_file_label_already_exists
@unsupported
  ret $cerune_file_label_unsupported_target
@failed
  ret $cerune_file_label_open_failed
@invalid
  ret $cerune_file_label_invalid_response
}
"#;
