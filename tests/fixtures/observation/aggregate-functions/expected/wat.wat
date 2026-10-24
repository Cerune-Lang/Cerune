(module
  (import "cerune" "write_stdout_v1" (func $write_stdout_v1 (param i64 i32) (result i32)))
  (import "cerune" "write_error_byte" (func $write_error_byte (param i32)))

  (func $cerune_i64_add_n2_b118_134 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=2 bytes=118..134
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
      i32.const 56
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  ;; output i64, newline=true; node=45 bytes=703..721
  (func $cerune_output_i64_lf1_n45_b703_721 (param $value i64)
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
    i32.const 51
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 49
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=48 bytes=722..737
  (func $cerune_output_i64_lf1_n48_b722_737 (param $value i64)
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
    i32.const 55
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 50
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=51 bytes=738..753
  (func $cerune_output_i64_lf1_n51_b738_753 (param $value i64)
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
    i32.const 53
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
    i32.const 55
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 56
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=54 bytes=754..774
  (func $cerune_output_i64_lf1_n54_b754_774 (param $value i64)
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
    i32.const 53
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
    i32.const 55
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 52
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 52
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=60 bytes=775..793
  (func $cerune_output_i64_lf1_n60_b775_793 (param $value i64)
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
    i32.const 54
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
    i32.const 55
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 53
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 55
    call $write_error_byte
    i32.const 57
    call $write_error_byte
    i32.const 51
    call $write_error_byte
    i32.const 10
    call $write_error_byte
      unreachable
    end
  )
  ;; output i64, newline=true; node=66 bytes=794..812
  (func $cerune_output_i64_lf1_n66_b794_812 (param $value i64)
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
    i32.const 54
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
    i32.const 57
    call $write_error_byte
    i32.const 52
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 46
    call $write_error_byte
    i32.const 56
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
  (memory 1)

  (global $cerune_heap_end (mut i64) (i64.const 136))

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
  (func $cerune_fn_move_x_0 (param $cerune_abi.result i32) (param $cerune_point i32) (param $cerune_amount i64) (param $cerune.frame.base i32)
    (local $cerune.frame.next i32)
    ;; function=0 private-frame-bytes=48
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune_abi.result
    i32.store
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    local.get $cerune_point
    i32.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    i64.load
    local.get $cerune_amount
    call $cerune_i64_add_n2_b118_134
    i64.store
    local.get $cerune.frame.base
    i32.const 36
    i32.add
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 36
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    call $cerune_frame_release
    return
  )
  (func $cerune_fn_move_twice_1 (param $cerune_abi.result i32) (param $cerune_point i32) (param $cerune_amount i64) (param $cerune.frame.base i32)
    (local $cerune.frame.next i32)
    ;; function=1 private-frame-bytes=64
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune_abi.result
    i32.store
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    local.get $cerune_point
    i32.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune_amount
    ;; invocation frame: 48 bytes
    i64.const 48
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=10 bytes=236..257
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 50
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_move_x_0
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune_amount
    ;; invocation frame: 48 bytes
    i64.const 48
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=9 bytes=229..266
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 50
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_move_x_0
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 36
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    call $cerune_frame_release
    return
  )
  (func $cerune_fn_first_row_2 (param $cerune_abi.result i32) (param $cerune_matrix i32) (param $cerune.frame.base i32)
    (local $cerune.frame.next i32)
    ;; function=2 private-frame-bytes=72
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune_abi.result
    i32.store
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    local.get $cerune_matrix
    i32.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 40
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 40
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 16
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 48
    i32.add
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 32
    i32.add
    local.get $cerune.frame.base
    i32.const 48
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.const 0
    i64.store
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=15 bytes=332..341
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
      i32.const 51
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=15 bytes=332..341
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
      i32.const 51
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 16
    i32.mul
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 68
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 68
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    call $cerune_frame_release
    return
  )
  (func $cerune_fn_duplicate_3 (param $cerune_abi.result i32) (param $cerune_row i32) (param $cerune.frame.base i32)
    (local $cerune.frame.next i32)
    ;; function=3 private-frame-bytes=72
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune_abi.result
    i32.store
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    local.get $cerune_row
    i32.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 36
    i32.add
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 28
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 36
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 16
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 68
    i32.add
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 68
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    call $cerune_frame_release
    return
  )
  (func $cerune_fn_duplicate_first_row_4 (param $cerune_abi.result i32) (param $cerune_matrix i32) (param $cerune.frame.base i32)
    (local $cerune.frame.next i32)
    ;; function=4 private-frame-bytes=112
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune_abi.result
    i32.store
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    local.get $cerune_matrix
    i32.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 40
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 40
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    local.get $cerune.frame.base
    i32.const 4
    i32.add
    i32.load
    i32.const 16
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 48
    i32.add
    local.get $cerune.frame.base
    i32.const 44
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 32
    i32.add
    local.get $cerune.frame.base
    i32.const 48
    i32.add
    i32.load
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    local.get $cerune.frame.base
    i32.const 84
    i32.add
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    ;; invocation frame: 72 bytes
    i64.const 72
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=24 bytes=505..522
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 53
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_first_row_2
    local.get $cerune.frame.base
    i32.const 84
    i32.add
    ;; invocation frame: 72 bytes
    i64.const 72
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=23 bytes=495..523
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 52
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_duplicate_3
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 52
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 100
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 100
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 60
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 104
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i32.load
    i32.const 16
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 104
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 68
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 108
    i32.add
    local.get $cerune.frame.base
    i32.const 104
    i32.add
    i32.load
    i32.const 8
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 108
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 76
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    call $cerune_frame_release
    return
  )
  (func $main
    (local $cerune.frame.base i32) (local $cerune.frame.next i32)
    ;; invocation frame: 284 bytes
    i64.const 284
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=26 bytes=528..568
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 53
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    local.set $cerune.frame.base
    local.get $cerune.frame.base
    i32.const 96
    i32.add
    i64.const 2
    i64.store
    local.get $cerune.frame.base
    i32.const 104
    i32.add
    i64.const 3
    i64.store
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    local.get $cerune.frame.base
    i32.const 96
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 8
    i32.add
    local.get $cerune.frame.base
    i32.const 104
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 112
    i32.add
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i64.const 5
    ;; invocation frame: 64 bytes
    i64.const 64
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=31 bytes=584..607
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 53
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 52
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_move_twice_1
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    local.get $cerune.frame.base
    i32.const 112
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    local.get $cerune.frame.base
    i32.const 120
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 160
    i32.add
    i64.const 1
    i64.store
    local.get $cerune.frame.base
    i32.const 168
    i32.add
    i64.const 2
    i64.store
    local.get $cerune.frame.base
    i32.const 128
    i32.add
    local.get $cerune.frame.base
    i32.const 160
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 136
    i32.add
    local.get $cerune.frame.base
    i32.const 168
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 176
    i32.add
    i64.const 3
    i64.store
    local.get $cerune.frame.base
    i32.const 184
    i32.add
    i64.const 4
    i64.store
    local.get $cerune.frame.base
    i32.const 144
    i32.add
    local.get $cerune.frame.base
    i32.const 176
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 152
    i32.add
    local.get $cerune.frame.base
    i32.const 184
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 32
    i32.add
    local.get $cerune.frame.base
    i32.const 128
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 40
    i32.add
    local.get $cerune.frame.base
    i32.const 136
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 48
    i32.add
    local.get $cerune.frame.base
    i32.const 144
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 56
    i32.add
    local.get $cerune.frame.base
    i32.const 152
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 192
    i32.add
    local.get $cerune.frame.base
    i32.const 32
    i32.add
    ;; invocation frame: 112 bytes
    i64.const 112
    call $cerune_frame_allocate
    local.tee $cerune.frame.next
    i32.eqz
    if
      ;; cerune: runtime-v1 code=allocation-failed node=43 bytes=673..700
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
      i32.const 108
      call $write_error_byte
      i32.const 108
      call $write_error_byte
      i32.const 111
      call $write_error_byte
      i32.const 99
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
      i32.const 45
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
      i32.const 54
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.next
    call $cerune_fn_duplicate_first_row_4
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    local.get $cerune.frame.base
    i32.const 192
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 72
    i32.add
    local.get $cerune.frame.base
    i32.const 200
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 80
    i32.add
    local.get $cerune.frame.base
    i32.const 208
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 88
    i32.add
    local.get $cerune.frame.base
    i32.const 216
    i32.add
    i64.load
    i64.store
    local.get $cerune.frame.base
    i32.const 0
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n45_b703_721
    local.get $cerune.frame.base
    i32.const 16
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n48_b722_737
    local.get $cerune.frame.base
    i32.const 24
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n51_b738_753
    local.get $cerune.frame.base
    i32.const 224
    i32.add
    i64.const 1
    i64.store
    local.get $cerune.frame.base
    i32.const 224
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=56 bytes=760..769
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
      i32.const 54
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 224
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=56 bytes=760..769
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
      i32.const 54
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 232
    i32.add
    local.get $cerune.frame.base
    i32.const 32
    i32.add
    local.get $cerune.frame.base
    i32.const 224
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 16
    i32.mul
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 236
    i32.add
    i64.const 0
    i64.store
    local.get $cerune.frame.base
    i32.const 236
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=55 bytes=760..772
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
      i32.const 54
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 236
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=55 bytes=760..772
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
      i32.const 54
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 232
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 236
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 8
    i32.mul
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n54_b754_774
    local.get $cerune.frame.base
    i32.const 244
    i32.add
    i64.const 0
    i64.store
    local.get $cerune.frame.base
    i32.const 244
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=62 bytes=781..788
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
      i32.const 54
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
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 244
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=62 bytes=781..788
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
      i32.const 54
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
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 252
    i32.add
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    local.get $cerune.frame.base
    i32.const 244
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 16
    i32.mul
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 256
    i32.add
    i64.const 1
    i64.store
    local.get $cerune.frame.base
    i32.const 256
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=61 bytes=781..791
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
      i32.const 54
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
      i32.const 55
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 256
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=61 bytes=781..791
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
      i32.const 54
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
      i32.const 55
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 252
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 256
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 8
    i32.mul
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n60_b775_793
    local.get $cerune.frame.base
    i32.const 264
    i32.add
    i64.const 1
    i64.store
    local.get $cerune.frame.base
    i32.const 264
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=68 bytes=800..807
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
      i32.const 54
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
      i32.const 48
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 264
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=68 bytes=800..807
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
      i32.const 54
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
      i32.const 48
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 272
    i32.add
    local.get $cerune.frame.base
    i32.const 64
    i32.add
    local.get $cerune.frame.base
    i32.const 264
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 16
    i32.mul
    i32.add
    i32.store
    local.get $cerune.frame.base
    i32.const 276
    i32.add
    i64.const 0
    i64.store
    local.get $cerune.frame.base
    i32.const 276
    i32.add
    i64.load
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=67 bytes=800..810
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
      i32.const 54
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
      i32.const 48
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 276
    i32.add
    i64.load
    i64.const 2
    i64.ge_s
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=67 bytes=800..810
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
      i32.const 54
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
      i32.const 48
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $cerune.frame.base
    i32.const 272
    i32.add
    i32.load
    local.get $cerune.frame.base
    i32.const 276
    i32.add
    i64.load
    i32.wrap_i64
    i32.const 8
    i32.mul
    i32.add
    i64.load
    call $cerune_output_i64_lf1_n66_b794_812
    local.get $cerune.frame.base
    call $cerune_frame_release
  )
  (export "main" (func $main))
)
