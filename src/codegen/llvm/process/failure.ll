define internal void @cerune.process.fail(i32 %status, ptr %origin) noreturn {
entry:
  %text = alloca [512 x i8]
  call i32 @fflush(ptr null)
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr @cerune.process.format, i32 %status, ptr %origin)
  br label %write
write:
  %positive = icmp sgt i32 %length, 0
  %fits = icmp ult i32 %length, 512
  %bounded = select i1 %fits, i32 %length, i32 511
  %count = select i1 %positive, i32 %bounded, i32 0
@WRITE@
  call void @_Exit(i32 1)
  unreachable
}
define internal void @cerune.process.startup_failed(ptr %phase, ptr %code) noreturn {
entry:
  %text = alloca [512 x i8]
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr @cerune.process.startup.format, ptr %phase, ptr %code)
  %positive = icmp sgt i32 %length, 0
  %fits = icmp ult i32 %length, 512
  %bounded = select i1 %fits, i32 %length, i32 511
  %count = select i1 %positive, i32 %bounded, i32 0
@WRITE@
  call void @_Exit(i32 1)
  unreachable
}
