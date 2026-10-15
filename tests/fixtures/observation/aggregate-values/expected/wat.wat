(module
  (import "cerune" "write_stdout_v1" (func $write_stdout_v1 (param i64 i32) (result i32)))
  (import "cerune" "write_error_byte" (func $write_error_byte (param i32)))

  (func $cerune_i64_add_n63_b70_84 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=63 bytes=70..84
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
      i32.const 54
      call $write_error_byte
      i32.const 51
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
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_sub_n41_b254_256 (param $left i64) (param $right i64) (result i64)
    (local $result i64)
    local.get $left
    local.get $right
    i64.sub
    local.set $result
    local.get $left
    local.get $right
    i64.xor
    local.get $left
    local.get $result
    i64.xor
    i64.and
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=integer-overflow node=41 bytes=254..256
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
      i32.const 52
      call $write_error_byte
      i32.const 49
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
      i32.const 53
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  ;; output bool, newline=true; node=4 bytes=64..86
  (func $cerune_output_bool_lf1_n4_b64_86 (param $value i32)
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
    i32.const 52
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
    i32.const 52
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 56
    call $write_error_byte
    i32.const 54
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=false; node=102 bytes=87..115
  (func $cerune_output_i64_lf0_n102_b87_115 (param $value i64)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 49
    call $write_error_byte
    i32.const 48
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=43 bytes=261..275
  (func $cerune_output_i64_lf1_n43_b261_275 (param $value i64)
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
    i32.const 52
    call $write_error_byte
    i32.const 51
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
    i32.const 49
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=104 bytes=87..115
  (func $cerune_output_string_lf0_n104_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 52
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=111 bytes=87..115
  (func $cerune_output_string_lf0_n111_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 49
    call $write_error_byte
    i32.const 49
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=113 bytes=87..115
  (func $cerune_output_string_lf0_n113_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 49
    call $write_error_byte
    i32.const 51
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=115 bytes=87..115
  (func $cerune_output_string_lf0_n115_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 49
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=95 bytes=87..115
  (func $cerune_output_string_lf0_n95_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 57
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=97 bytes=87..115
  (func $cerune_output_string_lf0_n97_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 57
    call $write_error_byte
    i32.const 55
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=false; node=99 bytes=87..115
  (func $cerune_output_string_lf0_n99_b87_115 (param $value i32)
    (local $requested i64) (local $accepted i64) (local $status i32)
    local.get $value
    i32.const 0
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
    i32.const 57
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output string, newline=true; node=118 bytes=87..115
  (func $cerune_output_string_lf1_n118_b87_115 (param $value i32)
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
    i32.const 49
    call $write_error_byte
    i32.const 56
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
    i32.const 56
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  (memory 1)

  (data (i32.const 344) "\05\00\00\00\00\00\00\00\56\61\6c\75\65")
  (data (i32.const 357) "\01\00\00\00\00\00\00\00\7b")
  (data (i32.const 366) "\03\00\00\00\00\00\00\00\6e\3a\20")
  (data (i32.const 377) "\01\00\00\00\00\00\00\00\7d")
  (data (i32.const 386) "\05\00\00\00\00\00\00\00\45\6d\70\74\79")
  (data (i32.const 399) "\01\00\00\00\00\00\00\00\7b")
  (data (i32.const 408) "\01\00\00\00\00\00\00\00\7d")
  (data (i32.const 417) "\00\00\00\00\00\00\00\00")

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
  (func $cerune_fn__equal0_0 (param $cerune_$left i32) (param $cerune_$right i32) (result i32)
    (local $cerune_$index i64)

    i32.const 128
    local.get $cerune_$left
    i32.store
    i32.const 132
    i32.const 128
    i32.load
    i64.load
    i64.store
    i32.const 168
    i32.const 128
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 140
    i32.const 168
    i32.load
    i64.load
    i64.store
    i32.const 148
    local.get $cerune_$right
    i32.store
    i32.const 152
    i32.const 148
    i32.load
    i64.load
    i64.store
    i32.const 172
    i32.const 148
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 160
    i32.const 172
    i32.load
    i64.load
    i64.store
    i64.const 0
    local.set $cerune_$index
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$index
        i64.const 2
        i64.lt_s
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          i32.const 176
          local.get $cerune_$index
          i64.store
          i32.const 176
          i64.load
          i64.const 0
          i64.lt_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=52 bytes=70..84
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
            i32.const 53
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
            i32.const 55
            call $write_error_byte
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 52
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 176
          i64.load
          i64.const 2
          i64.ge_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=52 bytes=70..84
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
            i32.const 53
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
            i32.const 55
            call $write_error_byte
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 52
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 132
          i32.const 176
          i64.load
          i32.wrap_i64
          i32.const 8
          i32.mul
          i32.add
          i64.load
          i32.const 184
          local.get $cerune_$index
          i64.store
          i32.const 184
          i64.load
          i64.const 0
          i64.lt_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=55 bytes=70..84
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
            i32.const 53
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
            i32.const 55
            call $write_error_byte
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 52
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
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=55 bytes=70..84
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
            i32.const 53
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
            i32.const 55
            call $write_error_byte
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 52
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
          i64.eq
          i32.eqz
          if
            i32.const 0
            return
          end
        end
        local.get $cerune_$index
        i64.const 1
        call $cerune_i64_add_n63_b70_84
        local.set $cerune_$index
        br $for_condition_0
      end
    end
    i32.const 1
    return
  )
  (func $cerune_fn__match_bind1_1 (param $cerune_$capture1 i32) (result i32)
    (local $cerune_n i64)

    i32.const 192
    local.get $cerune_$capture1
    i32.store
    i32.const 196
    i32.const 192
    i32.load
    i64.load
    i64.store
    i32.const 212
    i32.const 192
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 204
    i32.const 212
    i32.load
    i64.load
    i64.store
    i32.const 204
    i64.load
    local.set $cerune_n
    local.get $cerune_n
    i64.const 0
    i64.gt_s
    return
  )
  (func $cerune_fn__match_bind2_2 (param $cerune_$capture1 i32) (result i64)
    (local $cerune_n i64)

    i32.const 216
    local.get $cerune_$capture1
    i32.store
    i32.const 220
    i32.const 216
    i32.load
    i64.load
    i64.store
    i32.const 236
    i32.const 216
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 228
    i32.const 236
    i32.load
    i64.load
    i64.store
    i32.const 228
    i64.load
    local.set $cerune_n
    local.get $cerune_n
    return
  )
  (func $cerune_fn__match_select3_3 (param $cerune_$capture1 i32) (result i64)
    i32.const 240
    local.get $cerune_$capture1
    i32.store
    i32.const 244
    i32.const 240
    i32.load
    i64.load
    i64.store
    i32.const 260
    i32.const 240
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 252
    i32.const 260
    i32.load
    i64.load
    i64.store
    i32.const 244
    i64.load
    i64.const 0
    i64.eq
    if
      i64.const 0
      return
    else
      i64.const 0
      i64.const 1
      call $cerune_i64_sub_n41_b254_256
      return
    end
    unreachable
  )
  (func $cerune_fn__match_select4_4 (param $cerune_$capture1 i32) (result i64)
    i32.const 264
    local.get $cerune_$capture1
    i32.store
    i32.const 268
    i32.const 264
    i32.load
    i64.load
    i64.store
    i32.const 284
    i32.const 264
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 276
    i32.const 284
    i32.load
    i64.load
    i64.store
    i32.const 268
    i64.load
    i64.const 0
    i64.eq
    if (result i32)
      i32.const 268
      call $cerune_fn__match_bind1_1
    else
      i32.const 0
    end
    if
      i32.const 268
      call $cerune_fn__match_bind2_2
      return
    else
      i32.const 268
      call $cerune_fn__match_select3_3
      return
    end
    unreachable
  )
  (func $cerune_fn__match_bind5_5 (result i64)
    i32.const 304
    i64.const 0
    i64.store
    i32.const 312
    i64.const 7
    i64.store
    i32.const 288
    i32.const 304
    i64.load
    i64.store
    i32.const 296
    i32.const 312
    i64.load
    i64.store
    i32.const 288
    call $cerune_fn__match_select4_4
    return
  )
  (func $cerune_fn__display6_6 (param $cerune_$value i32)
    i32.const 320
    local.get $cerune_$value
    i32.store
    i32.const 324
    i32.const 320
    i32.load
    i64.load
    i64.store
    i32.const 340
    i32.const 320
    i32.load
    i32.const 8
    i32.add
    i32.store
    i32.const 332
    i32.const 340
    i32.load
    i64.load
    i64.store
    i32.const 324
    i64.load
    i64.const 0
    i64.eq
    if
      i32.const 344
      call $cerune_output_string_lf0_n95_b87_115
      i32.const 357
      call $cerune_output_string_lf0_n97_b87_115
      i32.const 366
      call $cerune_output_string_lf0_n99_b87_115
      i32.const 332
      i64.load
      call $cerune_output_i64_lf0_n102_b87_115
      i32.const 377
      call $cerune_output_string_lf0_n104_b87_115
    end
    i32.const 324
    i64.load
    i64.const 1
    i64.eq
    if
      i32.const 386
      call $cerune_output_string_lf0_n111_b87_115
      i32.const 399
      call $cerune_output_string_lf0_n113_b87_115
      i32.const 408
      call $cerune_output_string_lf0_n115_b87_115
    end
    i32.const 417
    call $cerune_output_string_lf1_n118_b87_115
    return
  )
  (func $main
    (local $cerune_result i64)

    i32.const 441
    i64.const 1
    i64.store
    i32.const 449
    i64.const 2
    i64.store
    i32.const 425
    i32.const 441
    i64.load
    i64.store
    i32.const 433
    i32.const 449
    i64.load
    i64.store
    i32.const 425
    i32.const 457
    i64.const 1
    i64.store
    i32.const 465
    i64.const 3
    i64.store
    i32.const 457
    call $cerune_fn__equal0_0
    call $cerune_output_bool_lf1_n4_b64_86
    i32.const 473
    i64.const 0
    i64.store
    i32.const 481
    i64.const 7
    i64.store
    i32.const 473
    call $cerune_fn__display6_6
    call $cerune_fn__match_bind5_5
    local.set $cerune_result
    local.get $cerune_result
    call $cerune_output_i64_lf1_n43_b261_275
  )
  (export "main" (func $main))
)
