//! x86-64 GNU ABI。statは144バイト、modeは24、dev/inoは0/8です。
pub(super) const SOURCE: &str = r#"
function w $cerune_files_open_error(w %error, w %create) {
@entry
  %switch_1 =w ceqw %error, 2
  jnz %switch_1, @missing, @switch_1
@switch_1
  %switch_2 =w ceqw %error, 17
  jnz %switch_2, @exists, @switch_2
@switch_2
  %switch_3 =w ceqw %error, 21
  jnz %switch_3, @directory, @switch_3
@switch_3
  %switch_4 =w ceqw %error, 20
  jnz %switch_4, @unsupported, @switch_4
@switch_4
  %switch_5 =w ceqw %error, 40
  jnz %switch_5, @unsupported, @failed
@missing
  ret 1
@exists
  ret 2
@directory
  jnz %create, @select_code_yes, @select_code_no
@select_code_yes
  jmp @select_code_done
@select_code_no
  jmp @select_code_done
@select_code_done
  %code =w phi @select_code_yes 2, @select_code_no 3
  ret %code
@unsupported
  ret 3
@failed
  ret 4
}
function $cerune_files_connect(l %root) {
@entry
  %metadata =l alloc8 144
  %length =l call $strlen(l %root)
  %size =l add %length, 1
  %path =l call $malloc(l %size)
  %null =w ceql %path, 0
  jnz %null, @allocation_failed, @copy
@allocation_failed
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l $cerune_file_label_allocation_failed)
  hlt
@copy
  call $memcpy(l %path, l %root, l %size)
  jmp @trim
@trim
  %n =l phi @copy %length, @erase %previous
  %single =w culel %n, 1
  jnz %single, @open, @last
@last
  %previous =l sub %n, 1
  %last_p =l add %path, %previous
  %byte =w loadub %last_p
  %slash =w ceqw %byte, 47
  jnz %slash, @erase, @open
@erase
  storeb 0, %last_p
  jmp @trim
@open
  %fd =w call $open(l %path, w 2752512, ...)
  %ep =l call $__errno_location()
  %errno =w loadw %ep
  call $free(l %path)
  %is_failed =w csltw %fd, 0
  jnz %is_failed, @open_failed, @stat
@open_failed
  %code =w call $cerune_files_open_error(w %errno, w 0)
  %text =l call $cerune_files_open_code(w %code)
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l %text)
  hlt
@stat
  %status =w call $fstat(w %fd, l %metadata)
  %stat_ok =w ceqw %status, 0
  jnz %stat_ok, @kind, @stat_failed
@kind
  %mode_ptr =l add %metadata, 24
  %mode =w loadw %mode_ptr
  %kind_bits =w and %mode, 61440
  %directory =w ceqw %kind_bits, 16384
  jnz %directory, @connected, @kind_failed
@stat_failed
  jmp @failure
@kind_failed
  jmp @failure
@failure
  %reason =l phi @stat_failed $cerune_file_label_open_failed, @kind_failed $cerune_file_label_unsupported_target
  call $close(w %fd)
  call $cerune_files_startup_failed(l $cerune_file_label_open_root, l %reason)
  hlt
@connected
  %wide =l extsw %fd
  storel %wide, $cerune_files_root
  storew 1, $cerune_files_configured
  ret
}
function w $cerune_file_host_open(l %name, l %length, w %writing, l %result) {
@entry
  %path =l alloc8 256
  %before =l alloc8 144
  %after =l alloc8 144
  %pinned =l alloc8 64
  call $memcpy(l %path, l %name, l %length)
  %end =l add %path, %length
  storeb 0, %end
  %root =l loadl $cerune_files_root
  %dirfd =w copy %root
  %create =w cnew %writing, 0
  jnz %create, @select_flags_yes, @select_flags_no
@select_flags_yes
  jmp @select_flags_done
@select_flags_no
  jmp @select_flags_done
@select_flags_done
  %flags =w phi @select_flags_yes 655553, @select_flags_no 2752512
  %fd =w call $openat(w %dirfd, l %path, w %flags, ..., w 438)
  %is_failed =w csltw %fd, 0
  jnz %is_failed, @open_failed, @opened
@open_failed
  %ep =l call $__errno_location()
  %errno =w loadw %ep
  %code =w call $cerune_files_open_error(w %errno, w %create)
  ret %code
@opened
  jnz %create, @created, @stat
@created
  %created_fd =l extsw %fd
  storel %created_fd, %result
  ret 0
@stat
  %before_status =w call $fstat(w %fd, l %before)
  %before_ok =w ceqw %before_status, 0
  jnz %before_ok, @kind, @stat_failed
@kind
  %mode_ptr =l add %before, 24
  %mode =w loadw %mode_ptr
  %kind_bits =w and %mode, 61440
  %regular =w ceqw %kind_bits, 32768
  jnz %regular, @reopen, @kind_failed
@stat_failed
  call $close(w %fd)
  ret 4
@kind_failed
  call $close(w %fd)
  ret 3
@reopen
  call $snprintf(l %pinned, l 64, l $cerune_files_proc_fd, ..., w %fd)
  %readable =w call $open(l %pinned, w 524288, ...)
  call $close(w %fd)
  %reopen_failed =w csltw %readable, 0
  jnz %reopen_failed, @failed, @restat
@failed
  ret 4
@restat
  %after_status =w call $fstat(w %readable, l %after)
  %after_ok =w ceqw %after_status, 0
  jnz %after_ok, @identity, @restat_failed
@identity
  %after_mode_ptr =l add %after, 24
  %after_mode =w loadw %after_mode_ptr
  %after_kind =w and %after_mode, 61440
  %after_regular =w ceqw %after_kind, 32768
  %before_dev =l loadl %before
  %after_dev =l loadl %after
  %before_ino_ptr =l add %before, 8
  %after_ino_ptr =l add %after, 8
  %before_ino =l loadl %before_ino_ptr
  %after_ino =l loadl %after_ino_ptr
  %same_dev =w ceql %before_dev, %after_dev
  %same_ino =w ceql %before_ino, %after_ino
  %same =w and %same_dev, %same_ino
  %valid =w and %same, %after_regular
  jnz %valid, @ready, @invalid
@restat_failed
  call $close(w %readable)
  ret 4
@invalid
  call $close(w %readable)
  ret 5
@ready
  %wide =l extsw %readable
  storel %wide, %result
  ret 0
}
function l $cerune_files_transfer_result(l %count) {
@entry
  %is_failed =w csltl %count, 0
  jnz %is_failed, @error, @done
@error
  %ep =l call $__errno_location()
  %errno =w loadw %ep
  %interrupted =w ceqw %errno, 4
  jnz %interrupted, @select_failure_yes, @select_failure_no
@select_failure_yes
  jmp @select_failure_done
@select_failure_no
  jmp @select_failure_done
@select_failure_done
  %failure =l phi @select_failure_yes -2, @select_failure_no -1
  ret %failure
@done
  ret %count
}
function l $cerune_file_host_read(l %handle, l %data, l %length) {
@entry
  %fd =w copy %handle
  %count =l call $read(w %fd, l %data, l %length)
  %result =l call $cerune_files_transfer_result(l %count)
  ret %result
}
function l $cerune_file_host_write(l %handle, l %data, l %length) {
@entry
  %fd =w copy %handle
  %count =l call $write(w %fd, l %data, l %length)
  %result =l call $cerune_files_transfer_result(l %count)
  ret %result
}
function w $cerune_file_host_close(l %handle) {
@entry
  %fd =w copy %handle
  %result =w call $close(w %fd)
  ret %result
}
"#;
