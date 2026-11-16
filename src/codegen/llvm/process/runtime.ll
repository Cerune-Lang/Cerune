%cerune.process.bytes = type { ptr, i64 }
%cerune.process.request = type { i64, i64, %cerune.process.bytes, ptr, i64, %cerune.process.bytes, i64, i64, i64, i64, i64, i64, i64 }
@cerune.process.context = internal global ptr null
declare i32 @cerune_process_v1_prepare(ptr, ptr)
declare i32 @cerune_process_v1_execute(ptr, i64)
declare i32 @cerune_process_v1_report(ptr, ptr, i64)
declare i32 @cerune_process_v1_copy(ptr, i64, i64, ptr, i64, ptr)
declare i32 @cerune_process_v1_release(ptr)
declare i32 @cerune_process_v1_destroy(ptr)

define internal void @cerune.process.check(i32 %status, i32 %expected, ptr %origin) {
entry:
  %ok = icmp eq i32 %status, %expected
  br i1 %ok, label %done, label %fail
fail:
  call void @cerune.process.fail(i32 %status, ptr %origin)
  unreachable
done:
  ret void
}
define internal ptr @cerune.process.array.data(%cerune.array %array) {
entry:
  %owner = extractvalue %cerune.array %array, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %zero, label %read
zero:
  ret ptr null
read:
  %data = load ptr, ptr %owner
  ret ptr %data
}
; An explicit byte conversion; Cerune u8 elements occupy i64 slots.
define internal ptr @cerune.process.pack(%cerune.array %input, ptr %failure) {
entry:
  %n = extractvalue %cerune.array %input, 1
  %empty = icmp eq i64 %n, 0
  br i1 %empty, label %zero, label %allocate
zero:
  ret ptr null
allocate:
  %data = call ptr @malloc(i64 %n)
  %failed = icmp eq ptr %data, null
  br i1 %failed, label %fail, label %source
fail:
  call void @cerune.runtime.fail(ptr %failure, i64 14)
  unreachable
source:
  %input.data = call ptr @cerune.process.array.data(%cerune.array %input)
  br label %loop
loop:
  %i = phi i64 [0, %source], [%next, %copy]
  %end = icmp eq i64 %i, %n
  br i1 %end, label %done, label %copy
copy:
  %from = getelementptr i64, ptr %input.data, i64 %i
  %wide = load i64, ptr %from
  %byte = trunc i64 %wide to i8
  %to = getelementptr i8, ptr %data, i64 %i
  store i8 %byte, ptr %to
  %next = add i64 %i, 1
  br label %loop
done:
  ret ptr %data
}
