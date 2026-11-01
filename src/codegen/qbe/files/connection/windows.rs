//! Win64 ABI。UNICODE_STRING=16、OBJECT_ATTRIBUTES=48、IO_STATUS_BLOCK=16バイトです。
pub(super) const SOURCE: &str = r#"
function w $cerune_files_nt_open(l %root, l %name, l %length, w %directory, w %create, l %handle) {
@entry
  %text =l alloc8 16
  %attributes =l alloc8 48
  %status =l alloc8 16
  %long =w cugtl %length, 32767
  jnz %long, @failed, @prepare
@prepare
  call $memset(l %text, w 0, l 16)
  call $memset(l %attributes, w 0, l 48)
  call $memset(l %status, w 0, l 16)
  %bytes =l mul %length, 2
  %short =w copy %bytes
  storeh %short, %text
  %maximum =l add %text, 2
  storeh %short, %maximum
  %buffer =l add %text, 8
  storel %name, %buffer
  storew 48, %attributes
  %root_p =l add %attributes, 8
  storel %root, %root_p
  %name_p =l add %attributes, 16
  storel %text, %name_p
  %flags_p =l add %attributes, 24
  storew 64, %flags_p
  %module =l call $GetModuleHandleW(l $cerune_files_ntdll)
  %open_file =l call $GetProcAddress(l %module, l $cerune_files_nt_create)
  %absent =w ceql %open_file, 0
  jnz %absent, @failed, @open
@open
  jnz %create, @select_data_access_yes, @select_data_access_no
@select_data_access_yes
  jmp @select_data_access_done
@select_data_access_no
  jmp @select_data_access_done
@select_data_access_done
  %data_access =w phi @select_data_access_yes 1048706, @select_data_access_no 1048705
  jnz %directory, @select_access_yes, @select_access_no
@select_access_yes
  jmp @select_access_done
@select_access_no
  jmp @select_access_done
@select_access_done
  %access =w phi @select_access_yes 1048736, @select_access_no %data_access
  jnz %directory, @select_options_yes, @select_options_no
@select_options_yes
  jmp @select_options_done
@select_options_no
  jmp @select_options_done
@select_options_done
  %options =w phi @select_options_yes 2097185, @select_options_no 2097248
  jnz %create, @select_disposition_yes, @select_disposition_no
@select_disposition_yes
  jmp @select_disposition_done
@select_disposition_no
  jmp @select_disposition_done
@select_disposition_done
  %disposition =w phi @select_disposition_yes 2, @select_disposition_no 1
  %result =w call %open_file(l %handle, w %access, l %attributes, l %status, l 0, w 0, w 7, w %disposition, w %options, l 0, w 0)
  %negative =w csltw %result, 0
  jnz %negative, @error, @check
@error
  %switch_1 =w ceqw %result, 3221225524
  jnz %switch_1, @missing, @switch_1
@switch_1
  %switch_2 =w ceqw %result, 3221225530
  jnz %switch_2, @missing, @switch_2
@switch_2
  %switch_3 =w ceqw %result, 3221225525
  jnz %switch_3, @exists, @switch_3
@switch_3
  %switch_4 =w ceqw %result, 3221225658
  jnz %switch_4, @is_directory, @switch_4
@switch_4
  %switch_5 =w ceqw %result, 3221225731
  jnz %switch_5, @unsupported, @switch_5
@switch_5
  %switch_6 =w ceqw %result, 3221226105
  jnz %switch_6, @unsupported, @switch_6
@switch_6
  %switch_7 =w ceqw %result, 3221226763
  jnz %switch_7, @unsupported, @failed
@is_directory
  jnz %create, @select_kind_code_yes, @select_kind_code_no
@select_kind_code_yes
  jmp @select_kind_code_done
@select_kind_code_no
  jmp @select_kind_code_done
@select_kind_code_done
  %kind_code =w phi @select_kind_code_yes 2, @select_kind_code_no 3
  ret %kind_code
@check
  %h =l loadl %handle
  %null =w ceql %h, 0
  %invalid =w ceql %h, -1
  %bad =w or %null, %invalid
  jnz %bad, @select_code_yes, @select_code_no
@select_code_yes
  jmp @select_code_done
@select_code_no
  jmp @select_code_done
@select_code_done
  %code =w phi @select_code_yes 5, @select_code_no 0
  ret %code
@missing
  ret 1
@exists
  ret 2
@unsupported
  ret 3
@failed
  ret 4
}
function w $cerune_files_kind(l %handle, w %directory) {
@entry
  %info =l alloc8 8
  %kind =w call $GetFileType(l %handle)
  %disk =w ceqw %kind, 1
  jnz %disk, @query, @unsupported
@query
  %result =w call $GetFileInformationByHandleEx(l %handle, w 9, l %info, w 8)
  %ok =w cnew %result, 0
  jnz %ok, @check, @failed
@check
  %attributes =w loadw %info
  %reparse_bits =w and %attributes, 1024
  %reparse =w cnew %reparse_bits, 0
  %directory_bits =w and %attributes, 16
  %is_directory =w cnew %directory_bits, 0
  %wrong_kind =w cnew %directory, %is_directory
  %unsupported_kind =w or %reparse, %wrong_kind
  jnz %unsupported_kind, @select_code_yes, @select_code_no
@select_code_yes
  jmp @select_code_done
@select_code_no
  jmp @select_code_done
@select_code_done
  %code =w phi @select_code_yes 3, @select_code_no 0
  ret %code
@unsupported
  ret 3
@failed
  ret 4
}
function $cerune_files_connect(l %root) {
@entry
  %offset_slot =l alloc8 8
  %prefix_slot =l alloc8 8
  %handle_slot =l alloc8 8
  storel 0, %handle_slot
  storel 0, %offset_slot
  storel 4, %prefix_slot
  %length =w call $GetFullPathNameW(l %root, w 0, l 0, l 0)
  %zero =w ceqw %length, 0
  %long =w cugtw %length, 32760
  %bad =w or %zero, %long
  jnz %bad, @invalid_root, @allocate
@allocate
  %capacity =w add %length, 1
  %wide =l extuw %length
  %absolute_units =l add %wide, 1
  %name_units =l add %wide, 9
  %absolute_size =l mul %absolute_units, 2
  %name_size =l mul %name_units, 2
  %absolute =l call $malloc(l %absolute_size)
  %name =l call $malloc(l %name_size)
  %no_absolute =w ceql %absolute, 0
  %no_name =w ceql %name, 0
  %allocation_failed =w or %no_absolute, %no_name
  jnz %allocation_failed, @allocation_failure, @resolve
@resolve
  %actual =w call $GetFullPathNameW(l %root, w %capacity, l %absolute, l 0)
  %empty =w ceqw %actual, 0
  %too_long =w cugtw %actual, %length
  %invalid =w or %empty, %too_long
  jnz %invalid, @invalid_root, @prefix
@prefix
  call $memcpy(l %name, l $cerune_files_nt_prefix, l 8)
  %actual_wide =l extuw %actual
  %four =w cugew %actual, 4
  jnz %four, @extended, @unc_check
@extended
  %extended_cmp =w call $wcsncmp(l %absolute, l $cerune_files_extended, l 4)
  %is_extended =w ceqw %extended_cmp, 0
  jnz %is_extended, @strip_extended, @device
@strip_extended
  storel 4, %offset_slot
  jmp @validate
@device
  %device_cmp =w call $wcsncmp(l %absolute, l $cerune_files_device, l 4)
  %is_device =w ceqw %device_cmp, 0
  jnz %is_device, @invalid_root, @unc_check
@unc_check
  %two =w cugew %actual, 2
  jnz %two, @unc_prefix, @validate
@unc_prefix
  %first =w loaduh %absolute
  %second_p =l add %absolute, 2
  %second =w loaduh %second_p
  %first_slash =w ceqw %first, 92
  %second_slash =w ceqw %second, 92
  %unc =w and %first_slash, %second_slash
  jnz %unc, @add_unc, @validate
@add_unc
  %after_prefix =l add %name, 8
  call $memcpy(l %after_prefix, l $cerune_files_unc_prefix, l 8)
  storel 8, %prefix_slot
  storel 2, %offset_slot
  jmp @validate
@validate
  %offset =l loadl %offset_slot
  %prefix_length =l loadl %prefix_slot
  %remaining =l sub %actual_wide, %offset
  %offset_8 =l mul %offset, 2
  %suffix =l add %absolute, %offset_8
  %prefix_unc =w ceql %prefix_length, 8
  jnz %prefix_unc, @copy, @drive_check
@drive_check
  %three =w cugel %remaining, 3
  jnz %three, @drive, @invalid_root
@drive
  %colon_p =l add %suffix, 2
  %slash_p =l add %suffix, 4
  %colon =w loaduh %colon_p
  %slash =w loaduh %slash_p
  %is_colon =w ceqw %colon, 58
  %is_slash =w ceqw %slash, 92
  %is_drive =w and %is_colon, %is_slash
  jnz %is_drive, @copy, @extended_unc_check
@extended_unc_check
  %unc_long =w cugel %remaining, 4
  jnz %unc_long, @extended_unc, @invalid_root
@extended_unc
  %unc_cmp =w call $wcsncmp(l %suffix, l $cerune_files_unc_prefix, l 4)
  %unc_ok =w ceqw %unc_cmp, 0
  jnz %unc_ok, @copy, @invalid_root
@copy
  %offset_9 =l mul %prefix_length, 2
  %destination =l add %name, %offset_9
  %copy_bytes =l mul %remaining, 2
  call $memcpy(l %destination, l %suffix, l %copy_bytes)
  %name_length =l add %prefix_length, %remaining
  jmp @trim
@trim
  %n =l phi @copy %name_length, @trim_more %previous
  %minimum =w culel %n, 7
  jnz %minimum, @open, @last
@last
  %previous =l sub %n, 1
  %offset_10 =l mul %previous, 2
  %last_p =l add %name, %offset_10
  %last_char =w loaduh %last_p
  %trailing =w ceqw %last_char, 92
  jnz %trailing, @trim_more, @open
@trim_more
  jmp @trim
@open
  %code =w call $cerune_files_nt_open(l 0, l %name, l %n, w 1, w 0, l %handle_slot)
  call $free(l %name)
  call $free(l %absolute)
  %opened =w ceqw %code, 0
  jnz %opened, @kind, @open_failed
@kind
  %handle =l loadl %handle_slot
  %kind_code =w call $cerune_files_kind(l %handle, w 1)
  %valid_kind =w ceqw %kind_code, 0
  jnz %valid_kind, @connected, @kind_failed
@kind_failed
  call $CloseHandle(l %handle)
  jmp @failure
@open_failed
  jmp @failure
@failure
  %error =w phi @open_failed %code, @kind_failed %kind_code
  %error_text =l call $cerune_files_open_code(w %error)
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l %error_text)
  hlt
@connected
  %raw =l copy %handle
  storel %raw, $cerune_files_root
  storew 1, $cerune_files_configured
  ret
@invalid_root
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l $cerune_file_label_invalid_root)
  hlt
@allocation_failure
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l $cerune_file_label_allocation_failed)
  hlt
}
function w $cerune_file_host_open(l %name, l %length, w %writing, l %result) {
@entry
  %wide =l alloc8 512
  %handle_slot =l alloc8 8
  storel 0, %handle_slot
  %narrow =w copy %length
  %converted =w call $MultiByteToWideChar(w 65001, w 8, l %name, w %narrow, l %wide, w 256)
  %failed =w ceqw %converted, 0
  jnz %failed, @conversion_failed, @open
@conversion_failed
  ret 4
@open
  %root_raw =l loadl $cerune_files_root
  %root =l copy %root_raw
  %count =l extuw %converted
  %create =w cnew %writing, 0
  %code =w call $cerune_files_nt_open(l %root, l %wide, l %count, w 0, w %create, l %handle_slot)
  %opened =w ceqw %code, 0
  jnz %opened, @opened_handle, @open_failed
@open_failed
  ret %code
@opened_handle
  %handle =l loadl %handle_slot
  jnz %create, @ready, @kind
@kind
  %kind_code =w call $cerune_files_kind(l %handle, w 0)
  %valid =w ceqw %kind_code, 0
  jnz %valid, @ready, @kind_failed
@kind_failed
  call $CloseHandle(l %handle)
  ret %kind_code
@ready
  %raw =l copy %handle
  storel %raw, %result
  ret 0
}
function l $cerune_file_host_read(l %raw, l %data, l %length) {
@entry
  %count_slot =l alloc8 8
  storew 0, %count_slot
  %handle =l copy %raw
  %request =w copy %length
  %status =w call $ReadFile(l %handle, l %data, w %request, l %count_slot, l 0)
  %count =w loadw %count_slot
  %ok =w cnew %status, 0
  jnz %ok, @done, @check_progress
@done
  %wide =l extuw %count
  ret %wide
@check_progress
  %some =w cnew %count, 0
  jnz %some, @invalid, @error
@invalid
  %unknown =l add %length, 1
  ret %unknown
@error
  %code =w call $GetLastError()
  %eof =w ceqw %code, 38
  jnz %eof, @select_result_yes, @select_result_no
@select_result_yes
  jmp @select_result_done
@select_result_no
  jmp @select_result_done
@select_result_done
  %result =l phi @select_result_yes 0, @select_result_no -1
  ret %result
}
function l $cerune_file_host_write(l %raw, l %data, l %length) {
@entry
  %count_slot =l alloc8 8
  storew 0, %count_slot
  %handle =l copy %raw
  %request =w copy %length
  %status =w call $WriteFile(l %handle, l %data, w %request, l %count_slot, l 0)
  %count =w loadw %count_slot
  %ok =w cnew %status, 0
  jnz %ok, @done, @error
@done
  %wide =l extuw %count
  ret %wide
@error
  %some =w cnew %count, 0
  %unknown =l add %length, 1
  jnz %some, @select_result_yes, @select_result_no
@select_result_yes
  jmp @select_result_done
@select_result_no
  jmp @select_result_done
@select_result_done
  %result =l phi @select_result_yes %unknown, @select_result_no -1
  ret %result
}
function w $cerune_file_host_close(l %raw) {
@entry
  %handle =l copy %raw
  %status =w call $CloseHandle(l %handle)
  %ok =w cnew %status, 0
  jnz %ok, @select_result_yes, @select_result_no
@select_result_yes
  jmp @select_result_done
@select_result_no
  jmp @select_result_done
@select_result_done
  %result =w phi @select_result_yes 0, @select_result_no -1
  ret %result
}
"#;
