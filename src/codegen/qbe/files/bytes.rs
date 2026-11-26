//! ファイルのバイトと動的配列の8バイト要素を相互に変換します。
pub(super) const SOURCE: &str = r#"
function l $cerune_files_read_bytes(l %name, l %limit, l %origin) {
@start
  %result =l alloc8 32
  %live =l loadl $cerune_array_live
  %available =l sub @ARRAY_LIMIT@, %live
  %elements =l udiv %available, 8
  call $cerune_files_validate_read(l $cerune_file_label_read_bytes, l %name, l %limit, l %elements, l %origin)
  %zero =w ceql %limit, 0
  jnz %zero, @read, @allocate
@allocate
  %size =l mul %limit, 8
  %storage =l call $cerune_array_try_storage(l %limit, l 8, l %size, l %size, l %live)
  %failed =w ceql %storage, 0
  jnz %failed, @allocation_failed, @ready
@allocation_failed
  call $cerune_files_fail(l $cerune_file_label_read_bytes, l $cerune_file_label_reserve, l $cerune_file_label_allocation_failed, l %limit, w 0, l 0, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@ready
  %buffer =l loadl %storage
  %length_slot =l add %storage, 8
  storel 0, %length_slot
  jmp @read
@read
  %owner =l phi @start 0, @ready %storage
  %data =l phi @start 0, @ready %buffer
  %code =l call $cerune_files_read_into(l %name, l %data, l %limit, l %result)
  %error =w cnel %code, 0
  jnz %error, @failure, @success
@failure
  call $cerune_array_release_owner_last(l %owner)
  call $cerune_array_free_elements(l %owner)
  call $cerune_files_read_failed(l $cerune_file_label_read_bytes, l %limit, l %code, l %result, l %origin)
  hlt
@success
  %length =l loadl %result
  jnz %zero, @done, @widen
@widen
  # Widen backwards to preserve packed bytes still waiting to be read.
  %end =l phi @success %length, @element %index
  %complete =w ceql %end, 0
  jnz %complete, @adopt, @element
@element
  %index =l sub %end, 1
  %at =l add %data, %index
  %byte =l loadub %at
  %offset =l mul %index, 8
  %slot =l add %data, %offset
  storel %byte, %slot
  jmp @widen
@adopt
  %actual_length =l add %owner, 8
  %initialized =l add %owner, 32
  storel %length, %actual_length
  storel %length, %initialized
@done
  ret %owner
}
function $cerune_files_write_bytes_new(l %name, l %owner, l %limit, l %origin) {
@start
  %length =l call $cerune_array_length(l %owner)
  call $cerune_files_validate_write(l $cerune_file_label_write_bytes_new, l %name, l %length, l %limit, l %origin)
  %empty =w ceql %length, 0
  jnz %empty, @write, @allocate
@allocate
  %buffer =l call $malloc(l %length)
  %failed =w ceql %buffer, 0
  jnz %failed, @allocation_failed, @ready
@allocation_failed
  call $cerune_files_fail(l $cerune_file_label_write_bytes_new, l $cerune_file_label_reserve, l $cerune_file_label_allocation_failed, l %limit, w 1, l %length, l 0, w 1, l $cerune_file_label_no, l $cerune_file_label_not_attempted, l %origin)
  hlt
@ready
  %elements =l loadl %owner
  jmp @pack
@pack
  %index =l phi @ready 0, @pack %next
  %offset =l mul %index, 8
  %slot =l add %elements, %offset
  %value =l loadl %slot
  %at =l add %buffer, %index
  storeb %value, %at
  %next =l add %index, 1
  %complete =w ceql %next, %length
  jnz %complete, @write, @pack
@write
  %data =l phi @start 0, @pack %buffer
  call $cerune_files_write_from(l $cerune_file_label_write_bytes_new, l %name, l %data, l %length, l %limit, l %origin)
  call $free(l %data)
  ret
}
"#;
