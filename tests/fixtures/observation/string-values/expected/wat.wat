(module
  (import "cerune" "write_stdout_v1" (func $write_stdout_v1 (param i64 i32) (result i32)))
  (import "cerune" "write_error_byte" (func $write_error_byte (param i32)))

  ;; output bool, newline=true; node=6 bytes=75..112
  (func $cerune_output_bool_lf1_n6_b75_112 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 1
    call $cerune_output_bool
    local.set $status
    local.set $accepted
    local.set $requested
    local.get $status
    if
      local.get $status
      i32.const 4
      i32.eq
      if
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 58
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 102
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 109
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 103
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 102
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
      else
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 58
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 45
    call $write_error_byte
    i32.const 118
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 119
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 113
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 115
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    local.get $requested
    call $cerune_output_error_u64
    i32.const 32
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 99
    call $write_error_byte
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    local.get $accepted
    call $cerune_output_error_u64
      end
    i32.const 32
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 54
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 98
    call $write_error_byte
    i32.const 121
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 115
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=true; node=10 bytes=113..125
  (func $cerune_output_string_lf1_n10_b113_125 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 1
    call $cerune_output_string
    local.set $status
    local.set $accepted
    local.set $requested
    local.get $status
    if
      local.get $status
      i32.const 4
      i32.eq
      if
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 58
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 102
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 109
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 103
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 102
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
      else
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 58
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 45
    call $write_error_byte
    i32.const 118
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 119
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 113
    call $write_error_byte
    i32.const 117
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 115
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    local.get $requested
    call $cerune_output_error_u64
    i32.const 32
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 99
    call $write_error_byte
    i32.const 99
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    local.get $accepted
    call $cerune_output_error_u64
      end
    i32.const 32
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 48
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 98
    call $write_error_byte
    i32.const 121
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 115
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  (memory 1)

  (data (i32.const 128) "\0b\00\00\00\00\00\00\00\e6\97\a5\e6\9c\ac\e8\aa\9e\0a\00")
  (data (i32.const 147) "\07\00\00\00\00\00\00\00\63\68\61\6e\67\65\64")
  (data (i32.const 162) "\0b\00\00\00\00\00\00\00\e6\97\a5\e6\9c\ac\e8\aa\9e\0a\00")

  (func $cerune_string_equal (param $left i32) (param $right i32) (result i32)
    (local $length i32) (local $index i32)
    local.get $left
    i32.load
    local.tee $length
    local.get $right
    i32.load
    i32.ne
    if
      i32.const 0
      return
    end
    block $equal
      loop $compare
        local.get $index
        local.get $length
        i32.eq
        br_if $equal
        local.get $left
        local.get $index
        i32.add
        i32.load8_u offset=8
        local.get $right
        local.get $index
        i32.add
        i32.load8_u offset=8
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $compare
      end
    end
    i32.const 1
  )

  (global $cerune_heap_end (mut i64) (i64.const 184))

  ;; frame block: next:u32, padding:u32, capacity:u64, payload (8-byte aligned).
  (global $cerune_frame_free (mut i32) (i32.const 0))
  (func $cerune_frame_allocate (param $size i64) (result i32)
    (local $p i32) (local $prev i32) (local $next i32)
    (local $bytes i64) (local $end i64) (local $pages i64)
    (if (i64.gt_u (local.get $size) (i64.const 4294967272))
      (then (return (i32.const 0))))
    (local.set $p (global.get $cerune_frame_free))
    (block $new
      (loop $search
        (br_if $new (i32.eqz (local.get $p)))
        (if (i64.ge_u (i64.load offset=8 (local.get $p)) (local.get $size))
          (then
            (local.set $next (i32.load (local.get $p)))
            (if (i32.eqz (local.get $prev))
              (then (global.set $cerune_frame_free (local.get $next)))
              (else (i32.store (local.get $prev) (local.get $next))))
            (return (i32.add (local.get $p) (i32.const 16)))))
        (local.set $prev (local.get $p))
        (local.set $p (i32.load (local.get $p)))
        (br $search)))
    (local.set $bytes (i64.and (i64.add (local.get $size) (i64.const 23)) (i64.const -8)))
    (local.set $end (i64.add (global.get $cerune_heap_end) (local.get $bytes)))
    (if (i64.gt_u (local.get $end) (i64.const 4294967296))
      (then (return (i32.const 0))))
    (local.set $pages (i64.div_u (i64.add (local.get $end) (i64.const 65535)) (i64.const 65536)))
    (if (i64.gt_u (local.get $pages) (i64.extend_i32_u (memory.size)))
      (then
        (if (i32.eq (memory.grow (i32.wrap_i64 (i64.sub (local.get $pages) (i64.extend_i32_u (memory.size))))) (i32.const -1))
          (then (return (i32.const 0))))))
    (local.set $p (i32.wrap_i64 (global.get $cerune_heap_end)))
    (global.set $cerune_heap_end (local.get $end))
    (i64.store offset=8 (local.get $p) (i64.sub (local.get $bytes) (i64.const 16)))
    (i32.add (local.get $p) (i32.const 16)))
  (func $cerune_frame_release (param $base i32) (local $p i32)
    (if (i32.eqz (local.get $base)) (then (return)))
    (local.set $p (i32.sub (local.get $base) (i32.const 16)))
    (i32.store (local.get $p) (global.get $cerune_frame_free))
    (global.set $cerune_frame_free (local.get $p)))
;; call_depth_limit=128; not a physical stack/memory guarantee
  (func $cerune_call_site_write (param $site i32) (param $allocation i32)
    unreachable
  )
  (func $cerune_call_header
    ;; call-v1 reason=depth-limit limit=128
    i32.const 99
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 45
    call $write_error_byte
    i32.const 118
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 115
    call $write_error_byte
    i32.const 111
    call $write_error_byte
    i32.const 110
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 100
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 112
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 104
    call $write_error_byte
    i32.const 45
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 109
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 109
    call $write_error_byte
    i32.const 105
    call $write_error_byte
    i32.const 116
    call $write_error_byte
    i32.const 61
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 56
    call $write_error_byte
  )
  (func $cerune_call_prefix
    ;; call-v1 frame=
    i32.const 99
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 108
    call $write_error_byte
    i32.const 45
    call $write_error_byte
    i32.const 118
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 32
    call $write_error_byte
    i32.const 102
    call $write_error_byte
    i32.const 114
    call $write_error_byte
    i32.const 97
    call $write_error_byte
    i32.const 109
    call $write_error_byte
    i32.const 101
    call $write_error_byte
    i32.const 61
    call $write_error_byte
  )

  (global $cerune_call_depth (mut i64) (i64.const 0))
  (global $cerune_call_head (mut i32) (i32.const 0))
  (global $cerune_call_tail (mut i32) (i32.const 0))
  ;; Private linked history: next:u32, previous:u32, immutable-site-id:u32.
  (func $cerune_call_number (param $n i64) (local $divisor i64) (local $started i32) (local $digit i64)
    (local.set $divisor (i64.const 1000000000))
    (loop $digits
      (local.set $digit (i64.div_u (local.get $n) (local.get $divisor)))
      (if (i32.or (local.get $started) (i32.or (i64.ne (local.get $digit) (i64.const 0)) (i64.eq (local.get $divisor) (i64.const 1))))
        (then
          (call $write_error_byte (i32.add (i32.wrap_i64 (local.get $digit)) (i32.const 48)))
          (local.set $started (i32.const 1))))
      (local.set $n (i64.rem_u (local.get $n) (local.get $divisor)))
      (local.set $divisor (i64.div_u (local.get $divisor) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $divisor) (i64.const 0)))))
  (func $cerune_call_enter (param $site i32) (local $p i32) (local $i i64)
    (if (i64.ge_u (global.get $cerune_call_depth) (i64.const 128))
      (then
        (call $cerune_call_header)
        (call $cerune_call_site_write (local.get $site) (i32.const 0))
        (local.set $p (global.get $cerune_call_head))
        (loop $history
          (call $cerune_call_prefix)
          (call $cerune_call_number (local.get $i))
          (call $cerune_call_site_write (i32.load offset=8 (local.get $p)) (i32.const 0))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (local.set $p (i32.load (local.get $p)))
          (br_if $history (local.get $p)))
        unreachable))
    (local.set $p (call $cerune_frame_allocate (i64.const 16)))
    (if (i32.eqz (local.get $p))
      (then (call $cerune_call_site_write (local.get $site) (i32.const 1)) unreachable))
    (i32.store (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (global.get $cerune_call_tail))
    (i32.store offset=8 (local.get $p) (local.get $site))
    (if (global.get $cerune_call_tail)
      (then (i32.store (global.get $cerune_call_tail) (local.get $p)))
      (else (global.set $cerune_call_head (local.get $p))))
    (global.set $cerune_call_tail (local.get $p))
    (global.set $cerune_call_depth (i64.add (global.get $cerune_call_depth) (i64.const 1))))
  (func $cerune_call_leave (local $p i32)
    (local.set $p (global.get $cerune_call_tail))
    (global.set $cerune_call_tail (i32.load offset=4 (local.get $p)))
    (if (global.get $cerune_call_tail)
      (then (i32.store (global.get $cerune_call_tail) (i32.const 0)))
      (else (global.set $cerune_call_head (i32.const 0))))
    (call $cerune_frame_release (local.get $p))
    (global.set $cerune_call_depth (i64.sub (global.get $cerune_call_depth) (i64.const 1))))

  (func $cerune_output_send (param $bits i64) (param $count i32) (result i32 i32)
    (local $accepted i32) (local $n i32)
    (loop $retry
      (local.set $n (call $write_stdout_v1 (local.get $bits) (local.get $count)))
      (br_if $retry (i32.eq (local.get $n) (i32.const -2)))
      (if (i32.eqz (local.get $n)) (then (return (local.get $accepted) (i32.const 1))))
      (if (i32.eq (local.get $n) (i32.const -1)) (then (return (local.get $accepted) (i32.const 3))))
      (if (i32.gt_u (local.get $n) (local.get $count)) (then (return (local.get $accepted) (i32.const 2))))
      (local.set $accepted (i32.add (local.get $accepted) (local.get $n)))
      (local.set $count (i32.sub (local.get $count) (local.get $n)))
      (if (i32.eqz (local.get $count)) (then (return (local.get $accepted) (i32.const 0))))
      (local.set $bits (i64.shr_u (local.get $bits) (i64.extend_i32_u (i32.mul (local.get $n) (i32.const 8)))))
      (br $retry))
    unreachable)
  (func $cerune_output_pack (param $ptr i32) (param $count i32) (result i64)
    (local $bits i64) (local $i i32)
    (loop $pack
      (local.set $bits (i64.or (local.get $bits) (i64.shl
        (i64.load8_u (i32.add (local.get $ptr) (local.get $i)))
        (i64.extend_i32_u (i32.mul (local.get $i) (i32.const 8))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $pack (i32.lt_u (local.get $i) (local.get $count))))
    local.get $bits)
  (func $cerune_output_bytes (param $ptr i32) (param $length i32) (param $newline i32) (result i64 i64 i32)
    (local $requested i64) (local $accepted i64) (local $count i32) (local $n i32) (local $status i32)
    (local.set $requested (i64.add (i64.extend_i32_u (local.get $length)) (i64.extend_i32_u (local.get $newline))))
    (block $end (loop $write
      (br_if $end (i32.eqz (local.get $length)))
      (local.set $count (select (i32.const 8) (local.get $length) (i32.gt_u (local.get $length) (i32.const 8))))
      (call $cerune_output_send (call $cerune_output_pack (local.get $ptr) (local.get $count)) (local.get $count))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
      (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
      (local.set $ptr (i32.add (local.get $ptr) (local.get $count)))
      (local.set $length (i32.sub (local.get $length) (local.get $count)))
      (br $write)))
    (if (local.get $newline) (then
      (call $cerune_output_send (i64.const 10) (i32.const 1))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))))
    local.get $requested local.get $accepted local.get $status)
  (func $cerune_output_integer (param $value i64) (param $negative i32) (param $newline i32) (result i64 i64 i32)
    (local $ptr i32)
    (local.set $ptr (i32.const 128))
    (loop $digits
      (local.set $ptr (i32.sub (local.get $ptr) (i32.const 1)))
      (i32.store8 (local.get $ptr) (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48)))
      (local.set $value (i64.div_u (local.get $value) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $value) (i64.const 0))))
    (if (local.get $negative) (then
      (local.set $ptr (i32.sub (local.get $ptr) (i32.const 1)))
      (i32.store8 (local.get $ptr) (i32.const 45))))
    (call $cerune_output_bytes (local.get $ptr) (i32.sub (i32.const 128) (local.get $ptr)) (local.get $newline)))
  (func $cerune_output_i64 (param $value i64) (param $newline i32) (result i64 i64 i32)
    (local $negative i32)
    (local.set $negative (i64.lt_s (local.get $value) (i64.const 0)))
    (if (local.get $negative) (then (local.set $value (i64.sub (i64.const 0) (local.get $value)))))
    (call $cerune_output_integer (local.get $value) (local.get $negative) (local.get $newline)))
  (func $cerune_output_u64 (param $value i64) (param $newline i32) (result i64 i64 i32)
    (call $cerune_output_integer (local.get $value) (i32.const 0) (local.get $newline)))
  (func $cerune_output_bool (param $value i32) (param $newline i32) (result i64 i64 i32)
    (i64.store (i32.const 0) (select (i64.const 1702195828) (i64.const 435728179558) (local.get $value)))
    (call $cerune_output_bytes (i32.const 0) (select (i32.const 4) (i32.const 5) (local.get $value)) (local.get $newline)))
  (func $cerune_output_error_u64 (param $value i64)
    (if (i64.ge_u (local.get $value) (i64.const 10))
      (then (call $cerune_output_error_u64 (i64.div_u (local.get $value) (i64.const 10)))))
    (call $write_error_byte (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48))))

  (func $cerune_output_string (param $value i32) (param $newline i32) (result i64 i64 i32)
    (call $cerune_output_bytes (i32.add (local.get $value) (i32.const 8)) (i32.load (local.get $value)) (local.get $newline)))
  (func $cerune_output_quoted (param $value i32) (param $newline i32) (result i64 i64 i32)
    (local $length i32) (local $index i32) (local $bits i64) (local $count i32)
    (local $requested i64) (local $accepted i64) (local $status i32) (local $n i32)
    (local.set $length (i32.load (local.get $value)))
    (local.set $requested (i64.add (i64.const 2) (i64.extend_i32_u (local.get $newline))))
    (block $counted (loop $counting
      (br_if $counted (i32.eq (local.get $index) (local.get $length)))
      (call $cerune_output_escape (i32.load8_u offset=8 (i32.add (local.get $value) (local.get $index))))
      local.set $count drop
      (local.set $requested (i64.add (local.get $requested) (i64.extend_i32_u (local.get $count))))
      (local.set $index (i32.add (local.get $index) (i32.const 1)))
      (br $counting)))
    (call $cerune_output_send (i64.const 34) (i32.const 1))
    local.set $status local.set $n
    (local.set $accepted (i64.extend_i32_u (local.get $n)))
    (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
    (local.set $index (i32.const 0))
    (block $written (loop $writing
      (br_if $written (i32.eq (local.get $index) (local.get $length)))
      (call $cerune_output_escape (i32.load8_u offset=8 (i32.add (local.get $value) (local.get $index))))
      local.set $count local.set $bits
      (call $cerune_output_send (local.get $bits) (local.get $count))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
      (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
      (local.set $index (i32.add (local.get $index) (i32.const 1)))
      (br $writing)))
    (call $cerune_output_send (select (i64.const 2594) (i64.const 34) (local.get $newline))
      (i32.add (i32.const 1) (local.get $newline)))
    local.set $status local.set $n
    (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
    local.get $requested local.get $accepted local.get $status)
  (func $cerune_output_escape (param $byte i32) (result i64 i32)
    (if (i32.eq (local.get $byte) (i32.const 0))
      (then (return (i64.const 12380) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 1))
      (then (return (i64.const 137650220266844) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 2))
      (then (return (i64.const 137654515234140) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 3))
      (then (return (i64.const 137658810201436) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 4))
      (then (return (i64.const 137663105168732) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 5))
      (then (return (i64.const 137667400136028) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 6))
      (then (return (i64.const 137671695103324) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 7))
      (then (return (i64.const 137675990070620) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 8))
      (then (return (i64.const 137680285037916) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 9))
      (then (return (i64.const 29788) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 10))
      (then (return (i64.const 28252) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 11))
      (then (return (i64.const 137860673664348) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 12))
      (then (return (i64.const 137864968631644) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 13))
      (then (return (i64.const 29276) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 14))
      (then (return (i64.const 137873558566236) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 15))
      (then (return (i64.const 137877853533532) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 16))
      (then (return (i64.const 137645942076764) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 17))
      (then (return (i64.const 137650237044060) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 18))
      (then (return (i64.const 137654532011356) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 19))
      (then (return (i64.const 137658826978652) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 20))
      (then (return (i64.const 137663121945948) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 21))
      (then (return (i64.const 137667416913244) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 22))
      (then (return (i64.const 137671711880540) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 23))
      (then (return (i64.const 137676006847836) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 24))
      (then (return (i64.const 137680301815132) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 25))
      (then (return (i64.const 137684596782428) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 26))
      (then (return (i64.const 137856395474268) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 27))
      (then (return (i64.const 137860690441564) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 28))
      (then (return (i64.const 137864985408860) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 29))
      (then (return (i64.const 137869280376156) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 30))
      (then (return (i64.const 137873575343452) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 31))
      (then (return (i64.const 137877870310748) (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 34))
      (then (return (i64.const 8796) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 92))
      (then (return (i64.const 23644) (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 127))
      (then (return (i64.const 137877970974044) (i32.const 6))))
    (i64.extend_i32_u (local.get $byte)) (i32.const 1)
  )
  (func $main
    (local $cerune.frame.base i32) (local $cerune.frame.next i32)
    (local $cerune_text i32)
    (local $cerune_saved i32)

    i32.const 128
    local.set $cerune_text
    local.get $cerune_text
    local.set $cerune_saved
    i32.const 147
    local.set $cerune_text
    local.get $cerune_saved
    i32.const 162
    call $cerune_string_equal
    call $cerune_output_bool_lf1_n6_b75_112
    local.get $cerune_text
    call $cerune_output_string_lf1_n10_b113_125
  )
  (export "main" (func $main))
)
