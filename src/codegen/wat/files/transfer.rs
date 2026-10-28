//! 転送と停止は生成WAT側で決定します。状態は同期実行中の一操作だけが使います。
pub(super) const SOURCE: &str = r#"
  (global $cerune_files_started (mut i32) (i32.const 0))
  (global $cerune_file_code (mut i32) (i32.const 0))
  (global $cerune_file_phase (mut i32) (i32.const 0))
  (global $cerune_file_known (mut i32) (i32.const 1))
  (global $cerune_file_created (mut i32) (i32.const 0))
  (global $cerune_file_cleanup (mut i32) (i32.const 0))
  (global $cerune_file_requested (mut i64) (i64.const -1))
  (global $cerune_file_transferred (mut i64) (i64.const 0))
  (func $cerune_files_start
    (if (global.get $cerune_files_started) (then unreachable))
    (global.set $cerune_files_started (i32.const 1)))
  (func $cerune_file_error_u64 (param $value i64)
    (if (i64.ge_u (local.get $value) (i64.const 10))
      (then (call $cerune_file_error_u64 (i64.div_u (local.get $value) (i64.const 10)))))
    (call $write_error_byte (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48))))
  (func $cerune_file_error_i64 (param $value i64)
    (if (i64.lt_s (local.get $value) (i64.const 0)) (then
      (call $write_error_byte (i32.const 45))
      (local.set $value (i64.sub (i64.const 0) (local.get $value)))))
    (call $cerune_file_error_u64 (local.get $value)))
  (func $cerune_file_reset (param $requested i64)
    (global.set $cerune_file_code (i32.const 0))
    (global.set $cerune_file_phase (i32.const 0))
    (global.set $cerune_file_known (i32.const 1))
    (global.set $cerune_file_created (i32.const 0))
    (global.set $cerune_file_cleanup (i32.const 0))
    (global.set $cerune_file_transferred (i64.const 0))
    (global.set $cerune_file_requested (local.get $requested)))
  (func $cerune_file_validate (param $name i32) (param $limit i64) (result i32)
    (local $status i32)
    (if (i64.gt_u (local.get $limit) (i64.const 2147483647)) (then
      (global.set $cerune_file_code (i32.const 1)) (return (i32.const 0))))
    (if (i32.eqz (call $cerune_file_name_valid (local.get $name))) (then
      (global.set $cerune_file_code (i32.const 2)) (return (i32.const 0))))
    (local.set $status (call $files_status_v1))
    (if (local.get $status) (then
      (global.set $cerune_file_code (select (i32.const 3) (i32.const 14) (i32.eq (local.get $status) (i32.const 1))))
      (return (i32.const 0))))
    (i32.const 1))
  (func $cerune_file_pack (param $data i32) (param $count i32) (result i64)
    (local $bits i64) (local $i i32)
    (loop $pack
      (local.set $bits (i64.or (local.get $bits)
        (i64.shl (i64.extend_i32_u (i32.load8_u (i32.add (local.get $data) (local.get $i))))
          (i64.extend_i32_u (i32.mul (local.get $i) (i32.const 8))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $pack (i32.lt_u (local.get $i) (local.get $count))))
    (local.get $bits))
  (func $cerune_file_open (param $name i32) (param $mode i32) (result i32)
    (local $length i32) (local $offset i32) (local $count i32) (local $status i32) (local $handle i32)
    (global.set $cerune_file_phase (i32.const 2))
    (local.set $length (i32.wrap_i64 (i64.load (local.get $name))))
    (local.set $status (call $file_name_begin_v1 (local.get $length)))
    (block $prepared
      (br_if $prepared (local.get $status))
      (loop $name
        (local.set $count (i32.sub (local.get $length) (local.get $offset)))
        (local.set $count (select (i32.const 8) (local.get $count) (i32.gt_u (local.get $count) (i32.const 8))))
        (local.set $status (call $file_name_chunk_v1
          (call $cerune_file_pack (i32.add (i32.add (local.get $name) (i32.const 8)) (local.get $offset)) (local.get $count))
          (local.get $count)))
        (br_if $prepared (local.get $status))
        (local.set $offset (i32.add (local.get $offset) (local.get $count)))
        (br_if $name (i32.lt_u (local.get $offset) (local.get $length)))))
    (if (local.get $status) (then
      (global.set $cerune_file_code (select (i32.const 9) (i32.const 14) (i32.eq (local.get $status) (i32.const 1))))
      (return (i32.const 0))))
    (call $file_open_v1 (local.get $mode))
    local.set $status
    local.set $handle
    (if (i32.eqz (local.get $status)) (then
      (if (i32.gt_s (local.get $handle) (i32.const 0)) (then
        (global.set $cerune_file_created (local.get $mode))
        (return (local.get $handle))))))
    (if (i32.and (i32.eqz (local.get $handle)) (i32.lt_u (i32.sub (local.get $status) (i32.const 1)) (i32.const 4))) (then
      (global.set $cerune_file_code (i32.add (local.get $status) (i32.const 5)))
      (return (i32.const 0))))
    (global.set $cerune_file_created (i32.mul (local.get $mode) (i32.const 2)))
    (global.set $cerune_file_code (i32.const 14))
    (i32.const 0))
  (func $cerune_file_close (param $handle i32)
    (local $status i32)
    (local.set $status (call $file_close_v1 (local.get $handle)))
    (global.set $cerune_file_cleanup (select (i32.const 2) (i32.const 1) (local.get $status)))
    (if (i32.eqz (global.get $cerune_file_code)) (then
      (global.set $cerune_file_phase (i32.const 5))
      (if (local.get $status) (then (global.set $cerune_file_code (i32.const 12)))))))
  (func $cerune_file_read (param $name i32) (param $limit i64) (result i32)
    (local $value i32) (local $allocation i32) (local $handle i32)
    (local $length i32) (local $request i32) (local $count i32) (local $bits i64) (local $i i32)
    (call $cerune_file_reset (i64.const -1))
    (if (i32.eqz (call $cerune_file_validate (local.get $name) (local.get $limit)))
      (then (return (i32.const @EMPTY@))))
    (global.set $cerune_file_phase (i32.const 1))
    (call $cerune_string_allocate (local.get $limit))
    local.set $allocation
    local.set $value
    (if (local.get $allocation) (then
      (global.set $cerune_file_code (select (i32.const 4) (i32.const 5) (i32.eq (local.get $allocation) (i32.const 2))))
      (return (i32.const @EMPTY@))))
    (local.set $handle (call $cerune_file_open (local.get $name) (i32.const 0)))
    (if (local.get $handle) (then
      (global.set $cerune_file_phase (i32.const 3))
      (block $done (loop $read
        (local.set $request (i32.sub (i32.wrap_i64 (local.get $limit)) (local.get $length)))
        (local.set $request (select (i32.const 8) (local.get $request) (i32.gt_u (local.get $request) (i32.const 8))))
        (local.set $request (select (local.get $request) (i32.const 1) (local.get $request)))
        (call $file_read_v1 (local.get $handle) (local.get $request))
        local.set $count
        local.set $bits
        (br_if $read (i32.eq (local.get $count) (i32.const -2)))
        (if (i32.eq (local.get $count) (i32.const -1)) (then
          (global.set $cerune_file_code (i32.const 10)) (br $done)))
        (if (i32.gt_u (local.get $count) (local.get $request)) (then
          (global.set $cerune_file_code (i32.const 14)) (global.set $cerune_file_known (i32.const 0)) (br $done)))
        (if (i32.eqz (local.get $count)) (then
          (global.set $cerune_file_requested (global.get $cerune_file_transferred)) (br $done)))
        (global.set $cerune_file_transferred (i64.add (global.get $cerune_file_transferred) (i64.extend_i32_u (local.get $count))))
        (if (i64.gt_u (global.get $cerune_file_transferred) (local.get $limit)) (then
          (global.set $cerune_file_code (i32.const 16)) (br $done)))
        (local.set $i (i32.const 0))
        (loop $copy
          (i32.store8 (i32.add (i32.add (local.get $value) (i32.const 8)) (local.get $length)) (i32.wrap_i64 (local.get $bits)))
          (local.set $bits (i64.shr_u (local.get $bits) (i64.const 8)))
          (local.set $length (i32.add (local.get $length) (i32.const 1)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br_if $copy (i32.lt_u (local.get $i) (local.get $count))))
        (br $read)))
      (call $cerune_file_close (local.get $handle))))
    (if (i32.eqz (global.get $cerune_file_code)) (then
      (global.set $cerune_file_phase (i32.const 6))
      (if (i32.eqz (call $cerune_utf8 (i32.add (local.get $value) (i32.const 8)) (local.get $length))) (then
        (global.set $cerune_file_code (i32.const 15))))))
    (if (i32.or (global.get $cerune_file_code) (i32.eqz (local.get $length))) (then
      (call $cerune_string_release (local.get $value))
      (return (i32.const @EMPTY@))))
    (global.set $cerune_heap_live (i64.add (i64.sub (global.get $cerune_heap_live) (local.get $limit)) (i64.extend_i32_u (local.get $length))))
    (i64.store (local.get $value) (i64.extend_i32_u (local.get $length)))
    (local.get $value))
  (func $cerune_file_write (param $name i32) (param $text i32) (param $limit i64)
    (local $length i64) (local $handle i32) (local $count i32) (local $accepted i32) (local $offset i32)
    (local.set $length (i64.load (local.get $text)))
    (call $cerune_file_reset (local.get $length))
    (if (i32.eqz (call $cerune_file_validate (local.get $name) (local.get $limit))) (then (return)))
    (if (i64.gt_u (local.get $length) (local.get $limit)) (then
      (global.set $cerune_file_code (i32.const 16)) (return)))
    (local.set $handle (call $cerune_file_open (local.get $name) (i32.const 1)))
    (if (i32.eqz (local.get $handle)) (then (return)))
    (global.set $cerune_file_phase (i32.const 4))
    (block $done (loop $write
      (br_if $done (i64.eq (global.get $cerune_file_transferred) (local.get $length)))
      (local.set $offset (i32.wrap_i64 (global.get $cerune_file_transferred)))
      (local.set $count (i32.sub (i32.wrap_i64 (local.get $length)) (local.get $offset)))
      (local.set $count (select (i32.const 8) (local.get $count) (i32.gt_u (local.get $count) (i32.const 8))))
      (local.set $accepted (call $file_write_v1 (local.get $handle)
        (call $cerune_file_pack (i32.add (i32.add (local.get $text) (i32.const 8)) (local.get $offset)) (local.get $count))
        (local.get $count)))
      (br_if $write (i32.eq (local.get $accepted) (i32.const -2)))
      (if (i32.eq (local.get $accepted) (i32.const -1)) (then
        (global.set $cerune_file_code (i32.const 11)) (br $done)))
      (if (i32.eqz (local.get $accepted)) (then
        (global.set $cerune_file_code (i32.const 13)) (br $done)))
      (if (i32.gt_u (local.get $accepted) (local.get $count)) (then
        (global.set $cerune_file_code (i32.const 14)) (global.set $cerune_file_known (i32.const 0)) (br $done)))
      (global.set $cerune_file_transferred (i64.add (global.get $cerune_file_transferred) (i64.extend_i32_u (local.get $accepted))))
      (br $write)))
    (call $cerune_file_close (local.get $handle)))
"#;
