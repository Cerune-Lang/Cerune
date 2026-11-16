; Windows x64 C passes the 16-byte Bytes value through a pointer to a caller copy.
declare i32 @cerune_process_v1_create(i64, i64, ptr, ptr)
define internal i32 @cerune.process.create(i64 %version, i64 %target, ptr %data, i64 %length, ptr %out) {
entry:
  %config = alloca %cerune.process.bytes, align 8
  %value.data = insertvalue %cerune.process.bytes poison, ptr %data, 0
  %value = insertvalue %cerune.process.bytes %value.data, i64 %length, 1
  store %cerune.process.bytes %value, ptr %config, align 8
  %status = call i32 @cerune_process_v1_create(i64 %version, i64 %target, ptr %config, ptr %out)
  ret i32 %status
}
