//! ファイルから受け取る各バイトを、非公開の配列に格納します。
pub(super) const SOURCE: &str = r#"
  (func $cerune_file_release_buffer (param $owner i32)
    (if (call $cerune_array_release_owner (local.get $owner))
      (then (call $cerune_array_free (local.get $owner)))))
  (func $cerune_file_read_bytes (param $name i32) (param $limit i64) (result i32)
    (local $owner i32) (local $allocation i32) (local $data i32)
    (local $length i32) (local $i i32)
    (call $cerune_file_reset (i64.const -1))
    (if (i32.eqz (call $cerune_file_validate (local.get $name) (local.get $limit)))
      (then (return (i32.const 0))))
    (global.set $cerune_file_phase (i32.const 1))
    (if (i64.gt_u (local.get $limit) (i64.div_u
        (i64.sub (i64.const @LIMIT@) (global.get $cerune_array_live)) (i64.const 8))) (then
      (global.set $cerune_file_code (i32.const 4)) (return (i32.const 0))))
    (call $cerune_array_allocate (local.get $limit) (i64.const 8) (i64.const 8))
    local.set $allocation
    local.set $owner
    (if (local.get $allocation) (then
      (global.set $cerune_file_code (i32.const 5)) (return (i32.const 0))))
    (if (local.get $owner) (then
      (local.set $data (call $cerune_array_element_address (local.get $owner) (i64.const 0)))))
    (local.set $length (call $cerune_file_read_into (local.get $name) (local.get $data) (local.get $limit)))
    (if (global.get $cerune_file_code) (then
      (call $cerune_file_release_buffer (local.get $owner)) (return (i32.const 0))))
    (if (local.get $owner) (then
      ;; Widen backwards to preserve the packed bytes still waiting to be read.
      (local.set $i (local.get $length))
      (block $done (loop $widen
        (br_if $done (i32.eqz (local.get $i)))
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (i64.store (call $cerune_array_element_address (local.get $owner) (i64.extend_i32_u (local.get $i)))
          (i64.load8_u (i32.add (local.get $data) (local.get $i))))
        (br $widen)))
      (i64.store offset=16 (local.get $owner) (i64.extend_i32_u (local.get $length)))
      (i64.store offset=24 (local.get $owner) (i64.extend_i32_u (local.get $length)))))
    (local.get $owner))
  (func $cerune_file_write_bytes_new (param $name i32) (param $owner i32) (param $limit i64)
    (local $length i64) (local $scratch i32) (local $allocation i32) (local $data i32) (local $i i32)
    (local.set $length (call $cerune_array_length (local.get $owner)))
    (if (i32.eqz (call $cerune_file_validate_write (local.get $name) (local.get $length) (local.get $limit)))
      (then return))
    (global.set $cerune_file_phase (i32.const 1))
    ;; Transport storage costs physical memory but contains no additional Cerune array elements.
    (call $cerune_array_allocate (local.get $length) (i64.const 0) (i64.const 1))
    local.set $allocation
    local.set $scratch
    (if (local.get $allocation) (then (global.set $cerune_file_code (i32.const 5)) (return)))
    (if (local.get $scratch) (then
      (local.set $data (call $cerune_array_element_address (local.get $scratch) (i64.const 0)))
      (loop $pack
        (i64.store8 (i32.add (local.get $data) (local.get $i))
          (i64.load (call $cerune_array_element_address (local.get $owner) (i64.extend_i32_u (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br_if $pack (i32.lt_u (local.get $i) (i32.wrap_i64 (local.get $length)))))))
    (call $cerune_file_write_from (local.get $name) (local.get $data) (local.get $length))
    (call $cerune_file_release_buffer (local.get $scratch)))
"#;
