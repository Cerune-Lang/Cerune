; Handwritten ABI calls; not produced by the C frontend.
%cerune.process.bytes = type { ptr, i64 }
%cerune.process.request = type { i64, i64, %cerune.process.bytes, ptr, i64, %cerune.process.bytes, i64, i64, i64, i64, i64, i64, i64 }
declare i32 @cerune_process_v1_prepare(ptr, ptr)
declare i32 @cerune_process_v1_execute(ptr, i64)
declare i32 @cerune_process_v1_report(ptr, ptr, i64)
declare i32 @cerune_process_v1_copy(ptr, i64, i64, ptr, i64, ptr)
declare i32 @cerune_process_v1_release(ptr)
declare i32 @cerune_process_v1_destroy(ptr)

; Runtime prepare owns the pointed-to bytes before this stack allocation expires.
define i32 @cerune_llvm_prepare(ptr %context, ptr %input) {
entry:
  %request = alloca %cerune.process.request, align 8
  %value = load %cerune.process.request, ptr %input, align 8
  store %cerune.process.request %value, ptr %request, align 8
  %status = call i32 @cerune_process_v1_prepare(ptr %context, ptr %request)
  ret i32 %status
}
define i32 @cerune_llvm_execute(ptr %context, i64 %reservation) {
entry:
  %status = call i32 @cerune_process_v1_execute(ptr %context, i64 %reservation)
  ret i32 %status
}
define i32 @cerune_llvm_report(ptr %context, ptr %words, i64 %length) {
entry:
  %status = call i32 @cerune_process_v1_report(ptr %context, ptr %words, i64 %length)
  ret i32 %status
}
; Six arguments exercise register/stack positions on both ABIs.
define i32 @cerune_llvm_copy(ptr %context, i64 %channel, i64 %offset, ptr %output, i64 %capacity, ptr %written) {
entry:
  %status = call i32 @cerune_process_v1_copy(ptr %context, i64 %channel, i64 %offset, ptr %output, i64 %capacity, ptr %written)
  ret i32 %status
}
define i32 @cerune_llvm_release(ptr %context) {
entry:
  %status = call i32 @cerune_process_v1_release(ptr %context)
  ret i32 %status
}
define i32 @cerune_llvm_destroy(ptr %context_address) {
entry:
  %status = call i32 @cerune_process_v1_destroy(ptr %context_address)
  ret i32 %status
}
