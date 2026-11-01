//! 転送・EOF確認・close・UTF-8検査と文字列所有をQBE ILで実行します。
pub(super) const SOURCE: &str = r#"
function l $cerune_files_read_text(l %name, l %limit, l %origin) {
@start
  %handle_slot =l alloc8 8
  %probe =l alloc4 4
  %code_slot =l alloc8 8
  %phase_slot =l alloc8 8
  %known_slot =l alloc4 4
  %eof_slot =l alloc4 4
  %length_slot =l alloc8 8
  %transferred_slot =l alloc8 8
  storel 0, %code_slot
  storel $cerune_file_label_read, %phase_slot
  storew 1, %known_slot
  storew 0, %eof_slot
  storel 0, %length_slot
  storel 0, %transferred_slot
  %validation =l call $cerune_files_check(l %name, l %limit)
  jnz %validation, @validation_failed, @budget
@validation_failed
  call $cerune_files_fail(l $cerune_file_label_read_text, l $cerune_file_label_validate, l %validation, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@budget
  %live =l loadl $cerune_string_live
  %available =l sub @LIMIT@, %live
  %over =w cugtl %limit, %available
  jnz %over, @budget_failed, @allocate
@budget_failed
  call $cerune_files_fail(l $cerune_file_label_read_text, l $cerune_file_label_reserve, l $cerune_file_label_budget_exceeded, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@allocate
  %size =l add %limit, 24
  %owner =l call $malloc(l %size)
  jnz %owner, @open, @allocation_failed
@allocation_failed
  call $cerune_files_fail(l $cerune_file_label_read_text, l $cerune_file_label_reserve, l $cerune_file_label_allocation_failed, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@open
  %reserved =l add %live, %limit
  storel %reserved, $cerune_string_live
  %open_code =l call $cerune_files_open_file(l %name, w 0, l %handle_slot)
  jnz %open_code, @open_failure, @ready
@open_failure
  call $free(l %owner)
  storel %live, $cerune_string_live
  call $cerune_files_fail(l $cerune_file_label_read_text, l $cerune_file_label_open, l %open_code, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@ready
  %handle =l loadl %handle_slot
  %data =l add %owner, 24
  jmp @read
@read
  %length =l loadl %length_slot
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
  storel %more, %transferred_slot
  jnz %full, @limit_failed, @advance
@advance
  storel %more, %length_slot
  jmp @read
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
  %primary =l loadl %code_slot
  jnz %primary, @finish, @check_close
@check_close
  jnz %closed, @decode, @close_failed
@close_failed
  storel $cerune_file_label_close_failed, %code_slot
  storel $cerune_file_label_close, %phase_slot
  jmp @finish
@decode
  %total =l loadl %length_slot
  %utf8 =w call $cerune_utf8(l %data, l %total)
  jnz %utf8, @finish, @decode_failed
@decode_failed
  storel $cerune_file_label_invalid_utf8, %code_slot
  storel $cerune_file_label_decode, %phase_slot
  jmp @finish
@finish
  %final_code =l loadl %code_slot
  jnz %final_code, @failure, @success
@failure
  %phase =l loadl %phase_slot
  %requested_known =w loadw %eof_slot
  %requested =l loadl %length_slot
  %transferred =l loadl %transferred_slot
  %known =w loadw %known_slot
  call $free(l %owner)
  storel %live, $cerune_string_live
  call $cerune_files_fail(l $cerune_file_label_read_text, l %phase, l %final_code, l %limit, w %requested_known, l %requested, l %transferred, w %known, l $cerune_file_label_no, l %cleanup, l %origin)
  hlt
@success
  %final_length =l loadl %length_slot
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
function $cerune_files_write_new(l %name, l %text, l %limit, l %origin) {
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
  %data =l add %text, 8
  %length =l loadl %text
  %validation =l call $cerune_files_check(l %name, l %limit)
  jnz %validation, @validation_failed, @size
@size
  %over =w cugtl %length, %limit
  jnz %over, @size_failed, @open
@size_failed
  jmp @validation_failed
@validation_failed
  %validation_code =l phi @start %validation, @size_failed $cerune_file_label_limit_exceeded
  call $cerune_files_fail(l $cerune_file_label_write_new, l $cerune_file_label_validate, l %validation_code, l %limit, w 1, l %length, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@open
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
  call $cerune_files_fail(l $cerune_file_label_write_new, l $cerune_file_label_open, l %open_code, l %limit, w 1, l %length, l 0, w 1, l %created, l $cerune_file_label_not_attempted, l %origin)
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
  call $cerune_files_fail(l $cerune_file_label_write_new, l %phase, l %final_code, l %limit, w 1, l %length, l %accepted, w %known, l $cerune_file_label_yes, l %cleanup, l %origin)
  hlt
@success
  ret
}
"#;
