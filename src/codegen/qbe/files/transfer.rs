//! 転送・EOF確認・close・UTF-8検査と文字列所有をQBE ILで実行します。
pub(super) const SOURCE: &str = r#"
function $cerune_files_validate_read(l %operation, l %name, l %limit, l %available, l %origin) {
@start
  %validation =l call $cerune_files_check(l %name, l %limit)
  jnz %validation, @validation_failed, @budget
@validation_failed
  call $cerune_files_fail(l %operation, l $cerune_file_label_validate, l %validation, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@budget
  %over =w cugtl %limit, %available
  jnz %over, @budget_failed, @done
@budget_failed
  call $cerune_files_fail(l %operation, l $cerune_file_label_reserve, l $cerune_file_label_budget_exceeded, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@done
  ret
}
# Result slots: bytes transferred, phase, cleanup, progress known, EOF observed.
function l $cerune_files_read_into(l %name, l %data, l %limit, l %result) {
@start
  %handle_slot =l alloc8 8
  %probe =l alloc4 4
  %code_slot =l alloc8 8
  %phase_slot =l add %result, 8
  %cleanup_slot =l add %result, 16
  %known_slot =l add %result, 24
  %eof_slot =l add %result, 28
  storel 0, %result
  storel $cerune_file_label_open, %phase_slot
  storel $cerune_file_label_not_attempted, %cleanup_slot
  storew 1, %known_slot
  storew 0, %eof_slot
  storel 0, %code_slot
  %open_code =l call $cerune_files_open_file(l %name, w 0, l %handle_slot)
  %open_failed =w cnel %open_code, 0
  jnz %open_failed, @open_failure, @ready
@open_failure
  ret %open_code
@ready
  %handle =l loadl %handle_slot
  storel $cerune_file_label_read, %phase_slot
  jmp @read
@read
  %length =l loadl %result
  %full =w ceql %length, %limit
  jnz %full, @probe, @chunk
@probe
  jmp @transfer
@chunk
  %remaining =l sub %limit, %length
  %small =w cultl %remaining, 4096
  jnz %small, @small, @large
@small
  jmp @chunk_ready
@large
  jmp @chunk_ready
@chunk_ready
  %chunk =l phi @small %remaining, @large 4096
  %at =l add %data, %length
  jmp @transfer
@transfer
  %request =l phi @probe 1, @chunk_ready %chunk
  %into =l phi @probe %probe, @chunk_ready %at
  %count =l call $cerune_file_host_read(l %handle, l %into, l %request)
  %interrupted =w ceql %count, -2
  jnz %interrupted, @read, @check_error
@check_error
  %failed =w csltl %count, 0
  jnz %failed, @read_failed, @check_response
@check_response
  %bad =w cugtl %count, %request
  jnz %bad, @response_failed, @check_eof
@check_eof
  %eof =w ceql %count, 0
  jnz %eof, @eof_reached, @received
@eof_reached
  storew 1, %eof_slot
  jmp @close
@received
  %more =l add %length, %count
  storel %more, %result
  jnz %full, @limit_failed, @read
@read_failed
  storel $cerune_file_label_read_failed, %code_slot
  jmp @close
@response_failed
  storel $cerune_file_label_invalid_response, %code_slot
  storew 0, %known_slot
  jmp @close
@limit_failed
  storel $cerune_file_label_limit_exceeded, %code_slot
  jmp @close
@close
  %status =w call $cerune_file_host_close(l %handle)
  %closed =w ceqw %status, 0
  jnz %closed, @closed, @close_error
@closed
  jmp @primary
@close_error
  jmp @primary
@primary
  %cleanup =l phi @closed $cerune_file_label_closed, @close_error $cerune_file_label_failed
  storel %cleanup, %cleanup_slot
  %primary =l loadl %code_slot
  jnz %primary, @finish, @check_close
@check_close
  jnz %closed, @finish, @close_failed
@close_failed
  storel $cerune_file_label_close_failed, %code_slot
  storel $cerune_file_label_close, %phase_slot
  jmp @finish
@finish
  %code =l loadl %code_slot
  ret %code
}
function $cerune_files_read_failed(l %operation, l %limit, l %code, l %result, l %origin) {
@start
  %transferred =l loadl %result
  %phase_slot =l add %result, 8
  %phase =l loadl %phase_slot
  %cleanup_slot =l add %result, 16
  %cleanup =l loadl %cleanup_slot
  %known_slot =l add %result, 24
  %known =w loadw %known_slot
  %eof_slot =l add %result, 28
  %eof =w loadw %eof_slot
  call $cerune_files_fail(l %operation, l %phase, l %code, l %limit, w %eof, l %transferred, l %transferred, w %known, l $cerune_file_label_no, l %cleanup, l %origin)
  hlt
}
function l $cerune_files_read_text(l %name, l %limit, l %origin) {
@start
  %result =l alloc8 32
  %live =l loadl $cerune_string_live
  %available =l sub @LIMIT@, %live
  call $cerune_files_validate_read(l $cerune_file_label_read_text, l %name, l %limit, l %available, l %origin)
  %size =l add %limit, 24
  %owner =l call $malloc(l %size)
  jnz %owner, @open, @allocation_failed
@allocation_failed
  call $cerune_files_fail(l $cerune_file_label_read_text, l $cerune_file_label_reserve, l $cerune_file_label_allocation_failed, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@open
  %reserved =l add %live, %limit
  storel %reserved, $cerune_string_live
  %data =l add %owner, 24
  %code =l call $cerune_files_read_into(l %name, l %data, l %limit, l %result)
  %failed =w cnel %code, 0
  jnz %failed, @failure, @decode
@decode
  %final_length =l loadl %result
  %utf8 =w call $cerune_utf8(l %data, l %final_length)
  jnz %utf8, @success, @decode_failed
@decode_failed
  %phase_slot =l add %result, 8
  storel $cerune_file_label_decode, %phase_slot
  jmp @failure
@failure
  %final_code =l phi @open %code, @decode_failed $cerune_file_label_invalid_utf8
  call $free(l %owner)
  storel %live, $cerune_string_live
  call $cerune_files_read_failed(l $cerune_file_label_read_text, l %limit, l %final_code, l %result, l %origin)
  hlt
@success
  %empty =w ceql %final_length, 0
  jnz %empty, @release_empty, @adopt
@release_empty
  call $free(l %owner)
  storel %live, $cerune_string_live
  ret $cerune_string_empty
@adopt
  %head =l loadl $cerune_string_owners
  storel %head, %owner
  %references =l add %owner, 8
  storel 1, %references
  %string =l add %owner, 16
  storel %final_length, %string
  storel %owner, $cerune_string_owners
  %charged =l add %live, %final_length
  storel %charged, $cerune_string_live
  ret %string
}
function $cerune_files_validate_write(l %operation, l %name, l %length, l %limit, l %origin) {
@start
  %validation =l call $cerune_files_check(l %name, l %limit)
  jnz %validation, @validation_failed, @size
@size
  %over =w cugtl %length, %limit
  jnz %over, @size_failed, @done
@size_failed
  jmp @validation_failed
@validation_failed
  %validation_code =l phi @start %validation, @size_failed $cerune_file_label_limit_exceeded
  call $cerune_files_fail(l %operation, l $cerune_file_label_validate, l %validation_code, l %limit, w 1, l %length, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@done
  ret
}
function $cerune_files_write_from(l %operation, l %name, l %data, l %length, l %limit, l %origin) {
@start
  %handle_slot =l alloc8 8
  %code_slot =l alloc8 8
  %phase_slot =l alloc8 8
  %known_slot =l alloc4 4
  %transferred_slot =l alloc8 8
  storel 0, %code_slot
  storel $cerune_file_label_write, %phase_slot
  storew 1, %known_slot
  storel 0, %transferred_slot
  %open_code =l call $cerune_files_open_file(l %name, w 1, l %handle_slot)
  jnz %open_code, @open_failure, @ready
@open_failure
  %open_invalid =w ceql %open_code, $cerune_file_label_invalid_response
  jnz %open_invalid, @unknown, @not_created
@unknown
  jmp @open_failed
@not_created
  jmp @open_failed
@open_failed
  %created =l phi @unknown $cerune_file_label_unknown, @not_created $cerune_file_label_no
  call $cerune_files_fail(l %operation, l $cerune_file_label_open, l %open_code, l %limit, w 1, l %length, l 0, w 1, l %created, l $cerune_file_label_not_attempted, l %origin)
  hlt
@ready
  %handle =l loadl %handle_slot
  jmp @write
@write
  %transferred =l loadl %transferred_slot
  %done =w ceql %transferred, %length
  jnz %done, @close, @transfer
@transfer
  %request =l sub %length, %transferred
  %at =l add %data, %transferred
  %count =l call $cerune_file_host_write(l %handle, l %at, l %request)
  %interrupted =w ceql %count, -2
  jnz %interrupted, @write, @check_error
@check_error
  %failed =w csltl %count, 0
  jnz %failed, @write_failed, @check_response
@check_response
  %invalid_response =w cugtl %count, %request
  jnz %invalid_response, @response_failed, @check_progress
@check_progress
  %zero =w ceql %count, 0
  jnz %zero, @zero_failed, @advance
@advance
  %more =l add %transferred, %count
  storel %more, %transferred_slot
  jmp @write
@write_failed
  storel $cerune_file_label_write_failed, %code_slot
  jmp @close
@response_failed
  storel $cerune_file_label_invalid_response, %code_slot
  storew 0, %known_slot
  jmp @close
@zero_failed
  storel $cerune_file_label_zero_progress, %code_slot
  jmp @close
@close
  %status =w call $cerune_file_host_close(l %handle)
  %closed =w ceqw %status, 0
  jnz %closed, @closed, @close_error
@closed
  jmp @primary
@close_error
  jmp @primary
@primary
  %cleanup =l phi @closed $cerune_file_label_closed, @close_error $cerune_file_label_failed
  %primary =l loadl %code_slot
  jnz %primary, @finish, @check_close
@check_close
  jnz %closed, @finish, @close_failed
@close_failed
  storel $cerune_file_label_close_failed, %code_slot
  storel $cerune_file_label_close, %phase_slot
  jmp @finish
@finish
  %final_code =l loadl %code_slot
  jnz %final_code, @failure, @success
@failure
  %phase =l loadl %phase_slot
  %accepted =l loadl %transferred_slot
  %known =w loadw %known_slot
  call $cerune_files_fail(l %operation, l %phase, l %final_code, l %limit, w 1, l %length, l %accepted, w %known, l $cerune_file_label_yes, l %cleanup, l %origin)
  hlt
@success
  ret
}
function $cerune_files_write_new(l %name, l %text, l %limit, l %origin) {
@start
  %length =l loadl %text
  %data =l add %text, 8
  call $cerune_files_validate_write(l $cerune_file_label_write_new, l %name, l %length, l %limit, l %origin)
  call $cerune_files_write_from(l $cerune_file_label_write_new, l %name, l %data, l %length, l %limit, l %origin)
  ret
}
"#;
