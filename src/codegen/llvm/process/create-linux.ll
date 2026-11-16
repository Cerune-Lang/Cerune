; System V AMD64 C splits Bytes into pointer and i64 integer-class arguments.
declare i32 @cerune_process_v1_create(i64, i64, ptr, i64, ptr)
define internal i32 @cerune.process.create(i64 %version, i64 %target, ptr %data, i64 %length, ptr %out) {
entry:
  %status = call i32 @cerune_process_v1_create(i64 %version, i64 %target, ptr %data, i64 %length, ptr %out)
  ret i32 %status
}
