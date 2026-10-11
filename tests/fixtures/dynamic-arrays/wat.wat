(module
  (import "cerune" "write_i64" (func $cerune_write_i64 (param i64)))
  (import "cerune" "write_u64" (func $cerune_write_u64 (param i64)))
  (import "cerune" "write_f32" (func $cerune_write_f32 (param f32)))
  (import "cerune" "write_f64" (func $cerune_write_f64 (param f64)))
  (import "cerune" "write_error_byte" (func $write_error_byte (param i32)))
  (import "cerune" "write_byte" (func $write_byte (param i32)))
  (import "cerune" "print_i64" (func $print_i64 (param i64)))
  (import "cerune" "print_f32" (func $print_f32 (param f32)))
  (import "cerune" "print_f64" (func $print_f64 (param f64)))

  (func $cerune_array_allocate_w8_s8_n224_b20_38 (param $length i64) (result i32)
    (local $value i32) (local $error i32)
    local.get $length
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-length-out-of-range node=224 bytes=20..38
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
      i32.const 108
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 104
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
      i32.const 114
      call $write_error_byte
      i32.const 97
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
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
      i32.const 50
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $length
    i64.const 8
    i64.const 8
    call $cerune_array_allocate
    local.set $error
    local.set $value
    local.get $error
    i32.const 1
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-size-overflow node=224 bytes=20..38
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
      i32.const 115
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 122
      call $write_error_byte
      i32.const 101
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
      i32.const 50
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $error
    i32.const 2
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-limit-exceeded node=224 bytes=20..38
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
      i32.const 45
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 120
      call $write_error_byte
      i32.const 99
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 100
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
      i32.const 50
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $error
    i32.const 3
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-failed node=224 bytes=20..38
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
      i32.const 50
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $value
  )
  (func $cerune_array_allocate_w8_s8_n280_b55_61 (param $length i64) (result i32)
    (local $value i32) (local $error i32)
    local.get $length
    i64.const 0
    i64.lt_s
    if
      ;; cerune: runtime-v1 code=array-length-out-of-range node=280 bytes=55..61
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
      i32.const 108
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 116
      call $write_error_byte
      i32.const 104
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
      i32.const 114
      call $write_error_byte
      i32.const 97
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
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
      i32.const 56
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $length
    i64.const 8
    i64.const 8
    call $cerune_array_allocate
    local.set $error
    local.set $value
    local.get $error
    i32.const 1
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-size-overflow node=280 bytes=55..61
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
      i32.const 115
      call $write_error_byte
      i32.const 105
      call $write_error_byte
      i32.const 122
      call $write_error_byte
      i32.const 101
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
      i32.const 56
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $error
    i32.const 2
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-limit-exceeded node=280 bytes=55..61
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
      i32.const 45
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 120
      call $write_error_byte
      i32.const 99
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 101
      call $write_error_byte
      i32.const 100
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
      i32.const 56
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $error
    i32.const 3
    i32.eq
    if
      ;; cerune: runtime-v1 code=allocation-failed node=280 bytes=55..61
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
      i32.const 56
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $value
  )
  (func $cerune_array_index_n295_b55_61 (param $owner i32) (param $index i64) (result i32)
    (i32.or (i64.lt_s (local.get $index) (i64.const 0))
      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=295 bytes=55..61
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))
    (call $cerune_array_element_address (local.get $owner) (local.get $index))
  )
  (func $cerune_array_index_n30_b78_92 (param $owner i32) (param $index i64) (result i32)
    (i32.or (i64.lt_s (local.get $index) (i64.const 0))
      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=30 bytes=78..92
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
      i32.const 51
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
      i32.const 56
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))
    (call $cerune_array_element_address (local.get $owner) (local.get $index))
  )
  (func $cerune_array_index_n330_b69_72 (param $owner i32) (param $index i64) (result i32)
    (i32.or (i64.lt_s (local.get $index) (i64.const 0))
      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=330 bytes=69..72
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
      i32.const 51
      call $write_error_byte
      i32.const 51
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
      i32.const 55
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))
    (call $cerune_array_element_address (local.get $owner) (local.get $index))
  )
  (func $cerune_array_index_n59_b93_106 (param $owner i32) (param $index i64) (result i32)
    (i32.or (i64.lt_s (local.get $index) (i64.const 0))
      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=59 bytes=93..106
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
      i32.const 57
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))
    (call $cerune_array_element_address (local.get $owner) (local.get $index))
  )
  (func $cerune_array_index_n7_b69_72 (param $owner i32) (param $index i64) (result i32)
    (i32.or (i64.lt_s (local.get $index) (i64.const 0))
      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))
    if
      ;; cerune: runtime-v1 code=array-index-out-of-bounds node=7 bytes=69..72
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
      i32.const 54
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 55
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))
    (call $cerune_array_element_address (local.get $owner) (local.get $index))
  )
  (func $cerune_array_range_n219_b20_38 (param $length i64) (param $start i64) (param $end i64)
    (i32.or (i64.lt_s (local.get $start) (i64.const 0))
      (i32.or (i64.lt_s (local.get $end) (local.get $start)) (i64.gt_s (local.get $end) (local.get $length))))
    if
      ;; cerune: runtime-v1 code=array-range-out-of-bounds node=219 bytes=20..38
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
      i32.const 114
      call $write_error_byte
      i32.const 97
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
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
      i32.const 49
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
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
  )
  (func $cerune_array_range_n275_b55_61 (param $length i64) (param $start i64) (param $end i64)
    (i32.or (i64.lt_s (local.get $start) (i64.const 0))
      (i32.or (i64.lt_s (local.get $end) (local.get $start)) (i64.gt_s (local.get $end) (local.get $length))))
    if
      ;; cerune: runtime-v1 code=array-range-out-of-bounds node=275 bytes=55..61
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
      i32.const 114
      call $write_error_byte
      i32.const 97
      call $write_error_byte
      i32.const 110
      call $write_error_byte
      i32.const 103
      call $write_error_byte
      i32.const 101
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
      i32.const 55
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
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
  )
  (func $cerune_i64_add_n233_b20_38 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=233 bytes=20..38
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
      i32.const 51
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
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n237_b20_38 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=237 bytes=20..38
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
      i32.const 51
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
      i32.const 50
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n289_b55_61 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=289 bytes=55..61
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
      i32.const 56
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n293_b55_61 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=293 bytes=55..61
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n34_b78_92 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=34 bytes=78..92
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
      i32.const 56
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 57
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_add_n63_b93_106 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=63 bytes=93..106
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
      i32.const 57
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_sub_n221_b20_38 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=221 bytes=20..38
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
      i32.const 50
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
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 51
      call $write_error_byte
      i32.const 56
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_sub_n277_b55_61 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=277 bytes=55..61
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
      i32.const 55
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
      i32.const 53
      call $write_error_byte
      i32.const 53
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 49
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (func $cerune_i64_sub_n354_b40_62 (param $left i64) (param $right i64) (result i64)
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
      ;; cerune: runtime-v1 code=integer-overflow node=354 bytes=40..62
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
      i32.const 52
      call $write_error_byte
      i32.const 48
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 46
      call $write_error_byte
      i32.const 54
      call $write_error_byte
      i32.const 50
      call $write_error_byte
      i32.const 10
      call $write_error_byte
      unreachable
    end
    local.get $result
  )

  (memory 1)

  (data (i32.const 0) "\01\00\00\00\00\00\00\00\5b")
  (data (i32.const 9) "\02\00\00\00\00\00\00\00\2c\20")
  (data (i32.const 23) "\01\00\00\00\00\00\00\00\5d")
  (data (i32.const 32) "\00\00\00\00\00\00\00\00")
  (data (i32.const 40) "\01\00\00\00\00\00\00\00\5b")
  (data (i32.const 49) "\02\00\00\00\00\00\00\00\2c\20")
  (data (i32.const 63) "\01\00\00\00\00\00\00\00\5d")
  (data (i32.const 72) "\00\00\00\00\00\00\00\00")

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

  (func $cerune_print_string (param $value i32)
    (local $length i32) (local $index i32)
    local.get $value
    i32.load
    local.set $length
    block $newline
      loop $write
        local.get $index
        local.get $length
        i32.eq
        br_if $newline
        local.get $value
        local.get $index
        i32.add
        i32.load8_u offset=8
        call $write_byte
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $write
      end
    end
    i32.const 10
    call $write_byte
  )


  (global $cerune_heap_head (mut i32) (i32.const 0))
  (global $cerune_heap_end (mut i64) (i64.const 176))
  (global $cerune_heap_live (mut i64) (i64.const 0))
  (func $cerune_string_retain (param $value i32)
    (local $p i32)
    global.get $cerune_heap_head
    local.set $p
    block $done
      loop $search
        local.get $p
        i32.eqz
        br_if $done
        local.get $p
        i32.const 16
        i32.add
        local.get $value
        i32.eq
        if
          local.get $p
          local.get $p
          i32.load offset=4
          i32.const 1
          i32.add
          i32.store offset=4
          return
        end
        local.get $p
        i32.load
        local.set $p
        br $search
      end
    end
  )
  (func $cerune_string_release (param $value i32)
    (local $p i32) (local $count i32)
    global.get $cerune_heap_head
    local.set $p
    block $done
      loop $search
        local.get $p
        i32.eqz
        br_if $done
        local.get $p
        i32.const 16
        i32.add
        local.get $value
        i32.eq
        if
          local.get $p
          i32.load offset=4
          i32.const 1
          i32.sub
          local.set $count
          local.get $p
          local.get $count
          i32.store offset=4
          local.get $count
          i32.eqz
          if
            global.get $cerune_heap_live
            local.get $p
            i64.load offset=16
            i64.sub
            global.set $cerune_heap_live
          end
          return
        end
        local.get $p
        i32.load
        local.set $p
        br $search
      end
    end
  )
  ;; 戻り値: 長さヘッダーへのaddress、失敗番号（0=成功）。
  (func $cerune_string_allocate (param $length i64) (result i32 i32)
    (local $p i32) (local $size i64) (local $end i64) (local $pages i64)
    local.get $length
    i64.eqz
    if
      i32.const 168
      i32.const 0
      return
    end
    local.get $length
    i64.const 4294967271
    i64.gt_u
    if
      i32.const 0
      i32.const 1
      return
    end
    local.get $length
    i64.const 67108864
    global.get $cerune_heap_live
    i64.sub
    i64.gt_u
    if
      i32.const 0
      i32.const 2
      return
    end
    global.get $cerune_heap_head
    local.set $p
    block $found
      block $new
        loop $search
          local.get $p
          i32.eqz
          br_if $new
          local.get $p
          i32.load offset=4
          i32.eqz
          local.get $p
          i64.load offset=8
          local.get $length
          i64.ge_u
          i32.and
          br_if $found
          local.get $p
          i32.load
          local.set $p
          br $search
        end
      end
      local.get $length
      i64.const 31
      i64.add
      i64.const -8
      i64.and
      local.set $size
      global.get $cerune_heap_end
      local.get $size
      i64.add
      local.tee $end
      i64.const 4294967296
      i64.gt_u
      if
        i32.const 0
        i32.const 3
        return
      end
      local.get $end
      i64.const 65535
      i64.add
      i64.const 65536
      i64.div_u
      local.set $pages
      local.get $pages
      memory.size
      i64.extend_i32_u
      i64.gt_u
      if
        local.get $pages
        memory.size
        i64.extend_i32_u
        i64.sub
        i32.wrap_i64
        memory.grow
        i32.const -1
        i32.eq
        if
          i32.const 0
          i32.const 3
          return
        end
      end
      global.get $cerune_heap_end
      i32.wrap_i64
      local.set $p
      local.get $end
      global.set $cerune_heap_end
      local.get $p
      global.get $cerune_heap_head
      i32.store
      local.get $p
      global.set $cerune_heap_head
      local.get $p
      local.get $size
      i64.const 24
      i64.sub
      i64.store offset=8
    end
    local.get $p
    i32.const 1
    i32.store offset=4
    local.get $p
    local.get $length
    i64.store offset=16
    global.get $cerune_heap_live
    local.get $length
    i64.add
    global.set $cerune_heap_live
    local.get $p
    i32.const 16
    i32.add
    i32.const 0
  )

  (global $cerune_array_head (mut i32) (i32.const 0))
  (global $cerune_array_live (mut i64) (i64.const 0))
  (func $cerune_array_allocate (param $length i64) (param $width i64) (param $stride i64) (result i32 i32)
    (local $bytes i64) (local $physical i64) (local $size i64)
    (local $end i64) (local $pages i64) (local $p i32)
    ;; i32へ狭める前に論理サイズとWasm32の物理サイズを検査します。
    (if (i64.lt_s (local.get $length) (i64.const 0))
      (then (return (i32.const 0) (i32.const 1))))
    (if (i64.eqz (local.get $length))
      (then (return (i32.const 0) (i32.const 0))))
    (if (i32.or
          (i64.gt_u (local.get $width) (i64.div_u (i64.const 9223372036854775807) (local.get $length)))
          (i64.gt_u (local.get $stride) (i64.div_u (i64.const 4294967232) (local.get $length))))
      (then (return (i32.const 0) (i32.const 1))))
    (local.set $bytes (i64.mul (local.get $length) (local.get $width)))
    (local.set $physical (i64.mul (local.get $length) (local.get $stride)))
    (if (i64.gt_u (local.get $bytes) (i64.sub (i64.const 67108864) (global.get $cerune_array_live)))
      (then (return (i32.const 0) (i32.const 2))))
    (local.set $p (global.get $cerune_array_head))
    (block $found
      (loop $search
        (br_if $found (i32.eqz (local.get $p)))
        (br_if $found (i32.and (i32.load offset=4 (local.get $p))
            (i64.ge_u (i64.load offset=48 (local.get $p)) (local.get $physical))))
        (local.set $p (i32.load (local.get $p)))
        (br $search)))
    (if (i32.eqz (local.get $p))
      (then
        (local.set $size (i64.and (i64.add (local.get $physical) (i64.const 63)) (i64.const -8)))
        (local.set $end (i64.add (global.get $cerune_heap_end) (local.get $size)))
        (if (i64.gt_u (local.get $end) (i64.const 4294967296))
          (then (return (i32.const 0) (i32.const 3))))
        (local.set $pages (i64.div_u (i64.add (local.get $end) (i64.const 65535)) (i64.const 65536)))
        (if (i64.gt_u (local.get $pages) (i64.extend_i32_u (memory.size)))
          (then
            (if (i32.eq (memory.grow (i32.wrap_i64 (i64.sub (local.get $pages) (i64.extend_i32_u (memory.size))))) (i32.const -1))
              (then (return (i32.const 0) (i32.const 3))))))
        (local.set $p (i32.wrap_i64 (global.get $cerune_heap_end)))
        (global.set $cerune_heap_end (local.get $end))
        (i32.store (local.get $p) (global.get $cerune_array_head))
        (global.set $cerune_array_head (local.get $p))
        (i64.store offset=48 (local.get $p) (i64.sub (local.get $size) (i64.const 56)))))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i64.store offset=8 (local.get $p) (i64.const 1))
    (i64.store offset=16 (local.get $p) (i64.const 0))
    (i64.store offset=24 (local.get $p) (local.get $length))
    (i64.store offset=32 (local.get $p) (local.get $bytes))
    (i64.store offset=40 (local.get $p) (local.get $stride))
    (global.set $cerune_array_live (i64.add (global.get $cerune_array_live) (local.get $bytes)))
    (local.get $p) (i32.const 0))
  (func $cerune_array_length (param $p i32) (result i64)
    (if (result i64) (i32.eqz (local.get $p))
      (then (i64.const 0)) (else (i64.load offset=24 (local.get $p)))))
  (func $cerune_array_element_address (param $p i32) (param $index i64) (result i32)
    (i32.add (i32.add (local.get $p) (i32.const 56))
      (i32.wrap_i64 (i64.mul (local.get $index) (i64.load offset=40 (local.get $p))))))
  (func $cerune_array_init_address (param $p i32) (result i32)
    (if (i32.eqz (local.get $p)) (then unreachable))
    (if (i64.ge_u (i64.load offset=16 (local.get $p)) (i64.load offset=24 (local.get $p))) (then unreachable))
    (call $cerune_array_element_address (local.get $p) (i64.load offset=16 (local.get $p))))
  (func $cerune_array_initialized (param $p i32)
    (i64.store offset=16 (local.get $p) (i64.add (i64.load offset=16 (local.get $p)) (i64.const 1))))
  (func $cerune_array_retain (param $p i32)
    (local $refs i64)
    (if (i32.eqz (local.get $p)) (then return))
    (local.set $refs (i64.load offset=8 (local.get $p)))
    (if (i32.or (i64.eqz (local.get $refs)) (i64.eq (local.get $refs) (i64.const -1))) (then unreachable))
    (i64.store offset=8 (local.get $p) (i64.add (local.get $refs) (i64.const 1))))
  (func $cerune_array_release_owner (param $p i32) (result i32)
    (local $refs i64)
    (if (i32.eqz (local.get $p)) (then (return (i32.const 1))))
    (local.set $refs (i64.load offset=8 (local.get $p)))
    (if (i64.eqz (local.get $refs)) (then unreachable))
    (local.set $refs (i64.sub (local.get $refs) (i64.const 1)))
    (i64.store offset=8 (local.get $p) (local.get $refs))
    (i64.eqz (local.get $refs)))
  (func $cerune_array_free (param $p i32)
    (if (i32.eqz (local.get $p)) (then return))
    (if (i32.or (i64.ne (i64.load offset=8 (local.get $p)) (i64.const 0))
          (i32.load offset=4 (local.get $p))) (then unreachable))
    (global.set $cerune_array_live (i64.sub (global.get $cerune_array_live) (i64.load offset=32 (local.get $p))))
    (i32.store offset=4 (local.get $p) (i32.const 1)))
  (func $cerune_write_escaped_byte (param $value i32)
    local.get $value
    i32.const 0
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 48
      call $write_byte
      return
    end
    local.get $value
    i32.const 1
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 2
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 50
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 3
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 51
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 4
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 52
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 5
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 53
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 6
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 54
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 7
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 55
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 8
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 56
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 9
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 116
      call $write_byte
      return
    end
    local.get $value
    i32.const 10
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 110
      call $write_byte
      return
    end
    local.get $value
    i32.const 11
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 98
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 12
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 99
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 13
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 114
      call $write_byte
      return
    end
    local.get $value
    i32.const 14
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 101
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 15
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 102
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 16
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 48
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 17
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 18
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 50
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 19
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 51
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 20
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 52
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 21
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 53
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 22
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 54
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 23
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 55
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 24
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 56
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 25
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 57
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 26
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 97
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 27
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 98
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 28
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 99
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 29
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 100
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 30
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 101
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 31
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 49
      call $write_byte
      i32.const 102
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    i32.const 34
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 34
      call $write_byte
      return
    end
    local.get $value
    i32.const 92
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 92
      call $write_byte
      return
    end
    local.get $value
    i32.const 127
    i32.eq
    if
      i32.const 92
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 123
      call $write_byte
      i32.const 55
      call $write_byte
      i32.const 102
      call $write_byte
      i32.const 125
      call $write_byte
      return
    end
    local.get $value
    call $write_byte
  )
  (func $cerune_write_bool (param $value i32)
    local.get $value
    if
      i32.const 116
      call $write_byte
      i32.const 114
      call $write_byte
      i32.const 117
      call $write_byte
      i32.const 101
      call $write_byte
    else
      i32.const 102
      call $write_byte
      i32.const 97
      call $write_byte
      i32.const 108
      call $write_byte
      i32.const 115
      call $write_byte
      i32.const 101
      call $write_byte
    end
  )
  (func $cerune_write_string (param $value i32)
    (local $length i32) (local $index i32)
    local.get $value
    i32.load
    local.set $length
    block $end
      loop $loop
        local.get $index
        local.get $length
        i32.eq
        br_if $end
        local.get $value
        local.get $index
        i32.add
        i32.load8_u offset=8
        call $write_byte
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $loop
      end
    end
  )
  (func $cerune_write_quoted (param $value i32)
    (local $length i32) (local $index i32)
    local.get $value
    i32.load
    local.set $length
    i32.const 34
    call $write_byte
    block $end
      loop $loop
        local.get $index
        local.get $length
        i32.eq
        br_if $end
        local.get $value
        local.get $index
        i32.add
        i32.load8_u offset=8
        call $cerune_write_escaped_byte
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $loop
      end
    end
    i32.const 34
    call $write_byte
  )
  (func $cerune_fn__display0_0 (param $cerune_$value i32)
    (local $cerune_$read6 i32)
    (local $cerune_$index i64)
    (local $cerune_$read22 i32)
    (local $cerune_$read23 i32)
    (local $cerune_$read24 i64)
    (local $cerune_$owned25 i64)
    (local $cerune_$read26 i64)
    (local $cerune_$read27 i32)
    (local $cerune_$read28 i32)

    i32.const 0
    local.set $cerune_$read6
    local.get $cerune_$read6
    call $cerune_write_string
    call $cerune_fn__ownership2_2
    local.set $cerune_$index
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$index
        local.get $cerune_$value
        call $cerune_fn__ownership3_3
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          local.get $cerune_$index
          call $cerune_fn__ownership5_5
          if
            i32.const 9
            local.set $cerune_$read22
            local.get $cerune_$read22
            call $cerune_write_string
          end
          local.get $cerune_$value
          local.set $cerune_$read23
          local.get $cerune_$index
          local.set $cerune_$read24
          local.get $cerune_$read24
          local.set $cerune_$owned25
          i32.const 19
          local.get $cerune_$read23
          local.get $cerune_$owned25
          call $cerune_array_index_n30_b78_92
          i32.store
          i32.const 19
          i32.load
          i64.load
          local.set $cerune_$read26
          local.get $cerune_$read26
          call $cerune_write_i64
        end
        local.get $cerune_$index
        call $cerune_fn__ownership4_4
        local.set $cerune_$index
        br $for_condition_0
      end
    end
    i32.const 23
    local.set $cerune_$read27
    local.get $cerune_$read27
    call $cerune_write_string
    i32.const 32
    local.set $cerune_$read28
    local.get $cerune_$read28
    call $cerune_print_string
    return
  )
  (func $cerune_fn__display1_1 (param $cerune_$value i32)
    (local $cerune_$read29 i32)
    (local $cerune_$index i64)
    (local $cerune_$read45 i32)
    (local $cerune_$read46 i32)
    (local $cerune_$read47 i64)
    (local $cerune_$owned48 i64)
    (local $cerune_$read49 i64)
    (local $cerune_$read50 i32)
    (local $cerune_$read51 i32)

    i32.const 40
    local.set $cerune_$read29
    local.get $cerune_$read29
    call $cerune_write_string
    call $cerune_fn__ownership6_6
    local.set $cerune_$index
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$index
        local.get $cerune_$value
        call $cerune_fn__ownership7_7
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          local.get $cerune_$index
          call $cerune_fn__ownership9_9
          if
            i32.const 49
            local.set $cerune_$read45
            local.get $cerune_$read45
            call $cerune_write_string
          end
          local.get $cerune_$value
          local.set $cerune_$read46
          local.get $cerune_$index
          local.set $cerune_$read47
          local.get $cerune_$read47
          local.set $cerune_$owned48
          i32.const 59
          local.get $cerune_$read46
          local.get $cerune_$owned48
          call $cerune_array_index_n59_b93_106
          i32.store
          i32.const 59
          i32.load
          i64.load
          local.set $cerune_$read49
          local.get $cerune_$read49
          call $cerune_write_i64
        end
        local.get $cerune_$index
        call $cerune_fn__ownership8_8
        local.set $cerune_$index
        br $for_condition_0
      end
    end
    i32.const 63
    local.set $cerune_$read50
    local.get $cerune_$read50
    call $cerune_write_string
    i32.const 72
    local.set $cerune_$read51
    local.get $cerune_$read51
    call $cerune_print_string
    return
  )
  (func $cerune_fn__ownership2_2 (result i64)
    (local $cerune_$owned7 i64)

    i64.const 0
    local.set $cerune_$owned7
    local.get $cerune_$owned7
    return
  )
  (func $cerune_fn__ownership3_3 (param $cerune_$owned8 i64) (param $cerune_$owned9 i32) (result i32)
    (local $cerune_$read10 i64)
    (local $cerune_$read11 i32)
    (local $cerune_$owned12 i64)
    (local $cerune_$owned13 i32)

    local.get $cerune_$owned8
    local.set $cerune_$read10
    local.get $cerune_$owned9
    local.set $cerune_$read11
    local.get $cerune_$read11
    call $cerune_array_length
    local.set $cerune_$owned12
    local.get $cerune_$read10
    local.get $cerune_$owned12
    i64.lt_s
    local.set $cerune_$owned13
    local.get $cerune_$owned13
    return
  )
  (func $cerune_fn__ownership4_4 (param $cerune_$owned14 i64) (result i64)
    (local $cerune_$read15 i64)
    (local $cerune_$read16 i64)
    (local $cerune_$owned17 i64)

    local.get $cerune_$owned14
    local.set $cerune_$read15
    i64.const 1
    local.set $cerune_$read16
    local.get $cerune_$read15
    local.get $cerune_$read16
    call $cerune_i64_add_n34_b78_92
    local.set $cerune_$owned17
    local.get $cerune_$owned17
    return
  )
  (func $cerune_fn__ownership5_5 (param $cerune_$owned18 i64) (result i32)
    (local $cerune_$read19 i64)
    (local $cerune_$read20 i64)
    (local $cerune_$owned21 i32)

    local.get $cerune_$owned18
    local.set $cerune_$read19
    i64.const 0
    local.set $cerune_$read20
    local.get $cerune_$read19
    local.get $cerune_$read20
    i64.ne
    local.set $cerune_$owned21
    local.get $cerune_$owned21
    return
  )
  (func $cerune_fn__ownership6_6 (result i64)
    (local $cerune_$owned30 i64)

    i64.const 0
    local.set $cerune_$owned30
    local.get $cerune_$owned30
    return
  )
  (func $cerune_fn__ownership7_7 (param $cerune_$owned31 i64) (param $cerune_$owned32 i32) (result i32)
    (local $cerune_$read33 i64)
    (local $cerune_$read34 i32)
    (local $cerune_$owned35 i64)
    (local $cerune_$owned36 i32)

    local.get $cerune_$owned31
    local.set $cerune_$read33
    local.get $cerune_$owned32
    local.set $cerune_$read34
    local.get $cerune_$read34
    call $cerune_array_length
    local.set $cerune_$owned35
    local.get $cerune_$read33
    local.get $cerune_$owned35
    i64.lt_s
    local.set $cerune_$owned36
    local.get $cerune_$owned36
    return
  )
  (func $cerune_fn__ownership8_8 (param $cerune_$owned37 i64) (result i64)
    (local $cerune_$read38 i64)
    (local $cerune_$read39 i64)
    (local $cerune_$owned40 i64)

    local.get $cerune_$owned37
    local.set $cerune_$read38
    i64.const 1
    local.set $cerune_$read39
    local.get $cerune_$read38
    local.get $cerune_$read39
    call $cerune_i64_add_n63_b93_106
    local.set $cerune_$owned40
    local.get $cerune_$owned40
    return
  )
  (func $cerune_fn__ownership9_9 (param $cerune_$owned41 i64) (result i32)
    (local $cerune_$read42 i64)
    (local $cerune_$read43 i64)
    (local $cerune_$owned44 i32)

    local.get $cerune_$owned41
    local.set $cerune_$read42
    i64.const 0
    local.set $cerune_$read43
    local.get $cerune_$read42
    local.get $cerune_$read43
    i64.ne
    local.set $cerune_$owned44
    local.get $cerune_$owned44
    return
  )
  (func $cerune_fn__ownership10_10 (result i32)
    (local $cerune_$owned52 i64)
    (local $cerune_$owned53 i64)
    (local $cerune_$owned55 i64)
    (local $cerune_$owned56 i32)
    (local $cerune_$owned57 i64)
    (local $cerune_$read59 i64)
    (local $cerune_$read60 i64)
    (local $cerune_$owned61 i64)
    (local $cerune_$read62 i64)
    (local $cerune_$owned63 i64)

    i64.const 1
    local.set $cerune_$owned52
    i64.const 2
    local.set $cerune_$owned53
    i32.const 112
    local.get $cerune_$owned52
    i64.store
    i32.const 120
    local.get $cerune_$owned53
    i64.store
    i32.const 80
    i32.const 112
    i64.load
    i64.store
    i32.const 88
    i32.const 120
    i64.load
    i64.store
    i64.const 2
    i64.const 0
    i64.const 2
    call $cerune_array_range_n219_b20_38
    i64.const 2
    i64.const 0
    call $cerune_i64_sub_n221_b20_38
    local.set $cerune_$owned55
    local.get $cerune_$owned55
    call $cerune_array_allocate_w8_s8_n224_b20_38
    local.set $cerune_$owned56
    i64.const 0
    local.set $cerune_$owned57
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$owned57
        local.get $cerune_$owned55
        i64.lt_s
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          i32.const 96
          i32.const 80
          i64.load
          i64.store
          i32.const 104
          i32.const 88
          i64.load
          i64.store
          local.get $cerune_$owned57
          local.set $cerune_$read59
          i64.const 0
          local.set $cerune_$read60
          local.get $cerune_$read59
          local.get $cerune_$read60
          call $cerune_i64_add_n237_b20_38
          local.set $cerune_$owned61
          i32.const 128
          local.get $cerune_$owned61
          i64.store
          i32.const 128
          i64.load
          i64.const 0
          i64.lt_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=239 bytes=20..38
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
            i32.const 51
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
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 51
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 128
          i64.load
          i64.const 2
          i64.ge_s
          if
            ;; cerune: runtime-v1 code=array-index-out-of-bounds node=239 bytes=20..38
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
            i32.const 51
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
            i32.const 48
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 46
            call $write_error_byte
            i32.const 51
            call $write_error_byte
            i32.const 56
            call $write_error_byte
            i32.const 10
            call $write_error_byte
            unreachable
          end
          i32.const 96
          i32.const 128
          i64.load
          i32.wrap_i64
          i32.const 8
          i32.mul
          i32.add
          i64.load
          local.set $cerune_$read62
          local.get $cerune_$read62
          local.set $cerune_$owned63
          i32.const 136
          local.get $cerune_$owned56
          i32.store
          i32.const 140
          i32.const 136
          i32.load
          call $cerune_array_init_address
          i32.store
          i32.const 140
          i32.load
          local.get $cerune_$owned63
          i64.store
          i32.const 136
          i32.load
          call $cerune_array_initialized
        end
        local.get $cerune_$owned57
        i64.const 1
        call $cerune_i64_add_n233_b20_38
        local.set $cerune_$owned57
        br $for_condition_0
      end
    end
    local.get $cerune_$owned56
    return
  )
  (func $cerune_fn__ownership11_11 (param $cerune_$owned64 i32) (result i32)
    (local $cerune_$read65 i32)
    (local $cerune_$read66 i32)
    (local $cerune_$owned67 i64)
    (local $cerune_$owned68 i32)
    (local $cerune_$owned69 i64)
    (local $cerune_$read70 i32)
    (local $cerune_$read71 i64)
    (local $cerune_$read72 i64)
    (local $cerune_$owned73 i64)
    (local $cerune_$read74 i64)
    (local $cerune_$owned75 i64)

    local.get $cerune_$owned64
    local.set $cerune_$read65
    local.get $cerune_$read65
    local.set $cerune_$read66
    local.get $cerune_$read66
    call $cerune_array_length
    i64.const 0
    local.get $cerune_$read66
    call $cerune_array_length
    call $cerune_array_range_n275_b55_61
    local.get $cerune_$read66
    call $cerune_array_length
    i64.const 0
    call $cerune_i64_sub_n277_b55_61
    local.set $cerune_$owned67
    local.get $cerune_$owned67
    call $cerune_array_allocate_w8_s8_n280_b55_61
    local.set $cerune_$owned68
    i64.const 0
    local.set $cerune_$owned69
    block $for_end_0
      loop $for_condition_0
        local.get $cerune_$owned69
        local.get $cerune_$owned67
        i64.lt_s
        i32.eqz
        br_if $for_end_0
        block $for_continue_0
          local.get $cerune_$read66
          local.set $cerune_$read70
          local.get $cerune_$owned69
          local.set $cerune_$read71
          i64.const 0
          local.set $cerune_$read72
          local.get $cerune_$read71
          local.get $cerune_$read72
          call $cerune_i64_add_n293_b55_61
          local.set $cerune_$owned73
          i32.const 144
          local.get $cerune_$read70
          local.get $cerune_$owned73
          call $cerune_array_index_n295_b55_61
          i32.store
          i32.const 144
          i32.load
          i64.load
          local.set $cerune_$read74
          local.get $cerune_$read74
          local.set $cerune_$owned75
          i32.const 148
          local.get $cerune_$owned68
          i32.store
          i32.const 152
          i32.const 148
          i32.load
          call $cerune_array_init_address
          i32.store
          i32.const 152
          i32.load
          local.get $cerune_$owned75
          i64.store
          i32.const 148
          i32.load
          call $cerune_array_initialized
        end
        local.get $cerune_$owned69
        i64.const 1
        call $cerune_i64_add_n289_b55_61
        local.set $cerune_$owned69
        br $for_condition_0
      end
    end
    local.get $cerune_$owned68
    return
  )
  (func $cerune_fn__ownership12_12 (result i64)
    (local $cerune_$owned76 i64)

    i64.const 0
    local.set $cerune_$owned76
    local.get $cerune_$owned76
    return
  )
  (func $cerune_fn__ownership13_13 (result i64)
    (local $cerune_$owned79 i64)

    i64.const 9
    local.set $cerune_$owned79
    local.get $cerune_$owned79
    return
  )
  (func $cerune_fn__ownership14_14 (param $cerune_$owned82 i32)
    (local $cerune_$owned83 i64)

    local.get $cerune_$owned82
    call $cerune_array_release_owner
    if
      local.get $cerune_$owned82
      call $cerune_array_length
      local.set $cerune_$owned83
      block $for_end_0
        loop $for_condition_0
          local.get $cerune_$owned83
          i64.const 0
          i64.gt_s
          i32.eqz
          br_if $for_end_0
          block $for_continue_0
          end
          local.get $cerune_$owned83
          i64.const 1
          call $cerune_i64_sub_n354_b40_62
          local.set $cerune_$owned83
          br $for_condition_0
        end
      end
      local.get $cerune_$owned82
      call $cerune_array_free
    end
    return
  )
  (func $main
    (local $cerune_values i32)
    (local $cerune_saved i32)
    (local $cerune_$owned77 i64)
    (local $cerune_$owned78 i64)
    (local $cerune_$read80 i32)
    (local $cerune_$read81 i32)

    call $cerune_fn__ownership10_10
    local.set $cerune_values
    local.get $cerune_values
    call $cerune_fn__ownership11_11
    local.set $cerune_saved
    call $cerune_fn__ownership12_12
    local.set $cerune_$owned77
    i32.const 156
    local.get $cerune_values
    local.get $cerune_$owned77
    call $cerune_array_index_n330_b69_72
    i32.store
    i32.const 156
    i32.load
    i64.load
    local.set $cerune_$owned78
    i32.const 160
    local.get $cerune_values
    i32.store
    i32.const 164
    i32.const 160
    i32.load
    local.get $cerune_$owned77
    call $cerune_array_index_n7_b69_72
    i32.store
    i32.const 164
    i32.load
    call $cerune_fn__ownership13_13
    i64.store
    local.get $cerune_values
    local.set $cerune_$read80
    local.get $cerune_$read80
    call $cerune_fn__display0_0
    local.get $cerune_saved
    local.set $cerune_$read81
    local.get $cerune_$read81
    call $cerune_fn__display1_1
    local.get $cerune_saved
    call $cerune_fn__ownership14_14
    local.get $cerune_values
    call $cerune_fn__ownership14_14
  )
  (export "main" (func $main))
)
