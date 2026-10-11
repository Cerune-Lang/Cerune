//! QBEの領域管理。値コピー・比較・表示・逆順解放は共通IRのループで実行します。
//! ownerの配置: data/length/references/logical_bytes/initialized/stride（各8バイト）。
pub(super) fn support(limit: u64) -> String {
    SOURCE.replace("@LIMIT@", &limit.to_string())
}
const SOURCE: &str = r#"
data $cerune_array_live = align 8 { l 0 }
function l $cerune_array_size(l %length, l %width, l %origin, l %origin_len) {
@start
  %empty =w ceql %length, 0
  jnz %empty, @zero, @check
@zero
  ret 0
@check
  %max =l udiv 9223372036854775807, %length
  %bad =w cugtl %width, %max
  jnz %bad, @fail, @done
@fail
  call $cerune_fail_allocation_size_overflow(l %origin, l %origin_len)
  hlt
@done
  %bytes =l mul %length, %width
  ret %bytes
}
function l $cerune_array_allocate(l %length, l %width, l %stride, l %origin, l %origin_len) {
@start
  %negative =w csltl %length, 0
  jnz %negative, @length_fail, @sizes
@sizes
  %bytes =l call $cerune_array_size(l %length, l %width, l %origin, l %origin_len)
  %physical =l call $cerune_array_size(l %length, l %stride, l %origin, l %origin_len)
  %live =l loadl $cerune_array_live
  %available =l sub @LIMIT@, %live
  %over =w cugtl %bytes, %available
  jnz %over, @limit_fail, @empty
@empty
  %zero =w ceql %length, 0
  jnz %zero, @return_zero, @owner
@return_zero
  ret 0
@owner
  %p =l call $malloc(l 48)
  %owner_failed =w ceql %p, 0
  jnz %owner_failed, @allocation_fail, @physical_size
@physical_size
  %zero_size =w ceql %physical, 0
  jnz %zero_size, @minimum, @ordinary
@minimum
  jmp @data
@ordinary
  jmp @data
@data
  %size =l phi @minimum 1, @ordinary %physical
  %data =l call $malloc(l %size)
  %data_failed =w ceql %data, 0
  jnz %data_failed, @data_fail, @initialize
@data_fail
  call $free(l %p)
  jmp @allocation_fail
@initialize
  storel %data, %p
  %length_p =l add %p, 8
  storel %length, %length_p
  %refs_p =l add %p, 16
  storel 1, %refs_p
  %bytes_p =l add %p, 24
  storel %bytes, %bytes_p
  %count_p =l add %p, 32
  storel 0, %count_p
  %stride_p =l add %p, 40
  storel %stride, %stride_p
  %total =l add %live, %bytes
  storel %total, $cerune_array_live
  ret %p
@length_fail
  call $cerune_fail_array_length_out_of_range(l %origin, l %origin_len)
  hlt
@limit_fail
  call $cerune_fail_allocation_limit_exceeded(l %origin, l %origin_len)
  hlt
@allocation_fail
  call $cerune_fail_allocation_failed(l %origin, l %origin_len)
  hlt
}
function l $cerune_array_length(l %p) {
@start
  %empty =w ceql %p, 0
  jnz %empty, @zero, @load
@zero
  ret 0
@load
  %length_p =l add %p, 8
  %length =l loadl %length_p
  ret %length
}
function l $cerune_array_init_address(l %p) {
@start
  %empty =w ceql %p, 0
  jnz %empty, @invalid, @check
@check
  %length_p =l add %p, 8
  %length =l loadl %length_p
  %count_p =l add %p, 32
  %count =l loadl %count_p
  %full =w cugel %count, %length
  jnz %full, @invalid, @address
@address
  %data =l loadl %p
  %stride_p =l add %p, 40
  %stride =l loadl %stride_p
  %offset =l mul %count, %stride
  %element =l add %data, %offset
  ret %element
@invalid
  hlt
}
function $cerune_array_initialized(l %p) {
@start
  %count_p =l add %p, 32
  %count =l loadl %count_p
  %next =l add %count, 1
  storel %next, %count_p
  ret
}
function $cerune_array_retain_owner(l %p) {
@start
  %empty =w ceql %p, 0
  jnz %empty, @done, @check
@check
  %refs_p =l add %p, 16
  %refs =l loadl %refs_p
  %zero =w ceql %refs, 0
  %max =w ceql %refs, -1
  %bad =w or %zero, %max
  jnz %bad, @invalid, @retain
@retain
  %more =l add %refs, 1
  storel %more, %refs_p
@done
  ret
@invalid
  hlt
}
function w $cerune_array_release_owner_last(l %p) {
@start
  %empty =w ceql %p, 0
  jnz %empty, @zero, @check
@zero
  ret 1
@check
  %refs_p =l add %p, 16
  %refs =l loadl %refs_p
  %zero_refs =w ceql %refs, 0
  %length_p =l add %p, 8
  %length =l loadl %length_p
  %count_p =l add %p, 32
  %count =l loadl %count_p
  %partial =w cnel %count, %length
  %bad =w or %zero_refs, %partial
  jnz %bad, @invalid, @release
@release
  %less =l sub %refs, 1
  storel %less, %refs_p
  %last =w ceql %less, 0
  ret %last
@invalid
  hlt
}
function $cerune_array_free_elements(l %p) {
@start
  %empty =w ceql %p, 0
  jnz %empty, @done, @check
@check
  %refs_p =l add %p, 16
  %refs =l loadl %refs_p
  %owned =w cnel %refs, 0
  jnz %owned, @invalid, @destroy
@destroy
  %data =l loadl %p
  %bytes_p =l add %p, 24
  %bytes =l loadl %bytes_p
  %live =l loadl $cerune_array_live
  %remaining =l sub %live, %bytes
  storel %remaining, $cerune_array_live
  call $free(l %data)
  call $free(l %p)
@done
  ret
@invalid
  hlt
}
function $cerune_array_check_range(l %length, l %start, l %end, l %origin, l %origin_len) {
@start
  %low =w csltl %start, 0
  %reversed =w csgtl %start, %end
  %high =w csgtl %end, %length
  %bad_start =w or %low, %reversed
  %bad =w or %bad_start, %high
  jnz %bad, @fail, @done
@fail
  call $cerune_fail_array_range_out_of_bounds(l %origin, l %origin_len)
  hlt
@done
  ret
}
function l $cerune_array_checked_address(l %p, l %index, l %origin, l %origin_len) {
@start
  %length =l call $cerune_array_length(l %p)
  %low =w csltl %index, 0
  %high =w csgel %index, %length
  %bad =w or %low, %high
  jnz %bad, @fail, @check
@fail
  call $cerune_fail_array_index_out_of_bounds(l %origin, l %origin_len)
  hlt
@check
  %count_p =l add %p, 32
  %count =l loadl %count_p
  %uninitialized =w csgel %index, %count
  jnz %uninitialized, @invalid, @address
@address
  %data =l loadl %p
  %stride_p =l add %p, 40
  %stride =l loadl %stride_p
  %offset =l mul %index, %stride
  %element =l add %data, %offset
  ret %element
@invalid
  hlt
}
"#;
