(module
  (import "cerune" "write_stdout_v1" (func $write_stdout_v1 (param i64 i32) (result i32)))
  (import "cerune" "write_error_byte" (func $write_error_byte (param i32)))

  (func $cerune_i64_add_n29_b141_151 (param $left i64) (param $right i64) (result i64)
    (local $result i64)
    local.get $left
    local.get $right
    i64.add
    local.set $result
    local.get $result
    local.get $left
    i64.xor
    local.get $result
    local.get $right
    i64.xor
    i64.and
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=integer-overflow node=29 bytes=141..151
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
      i32.const 114
      call $write_error_byte
      i32.const 117
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 109
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 45
      call $write_error_byte
      i32.const 118
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 32
      call $write_error_byte
      i32.const 99
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 100
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 61
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 114
      call $write_error_byte
      i32.const 45
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 118
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 114
      call $write_error_byte
      i32.const 102
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 119
      call $write_error_byte
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
      i32.const 50
      call $write_error_byte
      i32.const 57
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
      i32.const 52
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n35_b54_98 (param $left i64) (param $right i64) (result i64)
    (local $result i64)
    local.get $left
    local.get $right
    i64.add
    local.set $result
    local.get $result
    local.get $left
    i64.xor
    local.get $result
    local.get $right
    i64.xor
    i64.and
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=integer-overflow node=35 bytes=54..98
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
      i32.const 114
      call $write_error_byte
      i32.const 117
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 109
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 45
      call $write_error_byte
      i32.const 118
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 32
      call $write_error_byte
      i32.const 99
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 100
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 61
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 114
      call $write_error_byte
      i32.const 45
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 118
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 114
      call $write_error_byte
      i32.const 102
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 119
      call $write_error_byte
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
      i32.const 51
      call $write_error_byte
      i32.const 53
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
      i32.const 53
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  ;; output i64, newline=true; node=0 bytes=26..36
  (func $cerune_output_i64_lf1_n0_b26_36 (param $value i64)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 1
    call $cerune_output_i64
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
    i32.const 50
    call $write_error_byte
    i32.const 54
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 54
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=32 bytes=157..170
  (func $cerune_output_i64_lf1_n32_b157_170 (param $value i64)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 1
    call $cerune_output_i64
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
    i32.const 51
    call $write_error_byte
    i32.const 50
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
    i32.const 53
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 48
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  (memory 1)


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
  (func $cerune_fn_values_0 (param $cerune_abi.result i32)
    i32.const 128
    local.get $cerune_abi.result
    i32.store
    i64.const 42
    call $cerune_output_i64_lf1_n0_b26_36
    i32.const 132
    i64.const 1
    i64.store
    i32.const 140
    i64.const 2
    i64.store
    i32.const 128
    i32.load
    i32.const 132
    i64.load
    i64.store
    i32.const 148
    i32.const 128
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 148
    i32.load
    i32.const 140
    i64.load
    i64.store
    return
  )
  (func $main
    (local $cerune_$for_in_length_0_54 i64)
    (local $cerune_$for_in_cursor_0_54 i64)
    (local $cerune_i i64)
    (local $cerune_value i64)

    i32.const 168
    call $cerune_fn_values_0
    i32.const 152
    i32.const 168
    i64.load
    i64.store
    i32.const 160
    i32.const 176
    i64.load
    i64.store
    i64.const 2
    local.set $cerune_$for_in_length_0_54
    i64.const 0
    local.set $cerune_$for_in_cursor_0_54
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$for_in_cursor_0_54
        local.get $cerune_$for_in_length_0_54
        i64.lt_s
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          local.get $cerune_$for_in_cursor_0_54
          local.set $cerune_i
          i32.const 184
          local.get $cerune_$for_in_cursor_0_54
          i64.store
          i32.const 184
          i64.load
          i64.const 0
          i64.lt_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=20 bytes=69..85
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
            i32.const 114
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 116
            call $write_error_byte
            i32.const 105
            call $write_error_byte
            i32.const 109
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 118
            call $write_error_byte
            i32.const 49
            call $write_error_byte
            i32.const 32
            call $write_error_byte
            i32.const 99
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 61
            call $write_error_byte
            i32.const 97
            call $write_error_byte
            i32.const 114
            call $write_error_byte
            i32.const 114
            call $write_error_byte
            i32.const 97
            call $write_error_byte
            i32.const 121
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 105
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 120
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 116
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 102
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 98
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 115
            call $write_error_byte
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
            i32.const 50
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
            i32.const 54
            call $write_error_byte
            i32.const 57
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 53
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 184
          i64.load
          i64.const 2
          i64.ge_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=20 bytes=69..85
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
            i32.const 114
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 116
            call $write_error_byte
            i32.const 105
            call $write_error_byte
            i32.const 109
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 118
            call $write_error_byte
            i32.const 49
            call $write_error_byte
            i32.const 32
            call $write_error_byte
            i32.const 99
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 61
            call $write_error_byte
            i32.const 97
            call $write_error_byte
            i32.const 114
            call $write_error_byte
            i32.const 114
            call $write_error_byte
            i32.const 97
            call $write_error_byte
            i32.const 121
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 105
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 101
            call $write_error_byte
            i32.const 120
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 116
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 102
            call $write_error_byte
            i32.const 45
            call $write_error_byte
            i32.const 98
            call $write_error_byte
            i32.const 111
            call $write_error_byte
            i32.const 117
            call $write_error_byte
            i32.const 110
            call $write_error_byte
            i32.const 100
            call $write_error_byte
            i32.const 115
            call $write_error_byte
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
            i32.const 50
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
            i32.const 54
            call $write_error_byte
            i32.const 57
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 53
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 152
          i32.const 184
          i64.load
          i32.wrap_i64
          i32.const 8
          i32.mul
          i32.add
          i64.load
          local.set $cerune_value
          local.get $cerune_i
          i64.const 0
          i64.eq
          if
            br $for_continue_0
          end
          local.get $cerune_value
          i64.const 10
          call $cerune_i64_add_n29_b141_151
          local.set $cerune_value
          local.get $cerune_value
          call $cerune_output_i64_lf1_n32_b157_170
        end
        local.get $cerune_$for_in_cursor_0_54
        i64.const 1
        call $cerune_i64_add_n35_b54_98
        local.set $cerune_$for_in_cursor_0_54
        br $for_condition_0
      end
    end
  )
  (export "main" (func $main))
)
