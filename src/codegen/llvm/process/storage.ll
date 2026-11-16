; The reservation owns capacity, even when the final result has zero elements.
define internal ptr @cerune.process.result.malloc(i64 %bytes) {
entry:
  %p = call ptr @malloc(i64 %bytes)
  ret ptr %p
}
define internal i64 @cerune.process.reserve.one(i64 %n, ptr %out) {
entry:
  %empty = icmp eq i64 %n, 0
  br i1 %empty, label %done, label %owner
owner:
  %p = call ptr @cerune.process.result.malloc(i64 32)
  %bad.owner = icmp eq ptr %p, null
  br i1 %bad.owner, label %fail, label %data
data:
  %bytes = mul i64 %n, 8
  %buffer = call ptr @cerune.process.result.malloc(i64 %bytes)
  %bad.data = icmp eq ptr %buffer, null
  br i1 %bad.data, label %free.owner, label %initialize
free.owner:
  call void @free(ptr %p)
  br label %fail
initialize:
  %a = insertvalue %cerune.array.owner poison, ptr %buffer, 0
  %b = insertvalue %cerune.array.owner %a, i64 1, 1
  %c = insertvalue %cerune.array.owner %b, i64 %bytes, 2
  %d = insertvalue %cerune.array.owner %c, i64 0, 3
  store %cerune.array.owner %d, ptr %p
  %live = load i64, ptr @cerune.array.live
  %total = add i64 %live, %bytes
  store i64 %total, ptr @cerune.array.live
  %result.p = insertvalue %cerune.array poison, ptr %p, 0
  %result = insertvalue %cerune.array %result.p, i64 %n, 1
  store %cerune.array %result, ptr %out
  br label %done
done:
  ret i64 0
fail:
  ret i64 2
}
define internal void @cerune.process.cancel(ptr %array) {
entry:
  %value = load %cerune.array, ptr %array
  %owner = extractvalue %cerune.array %value, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %done, label %free
free:
  %bytes.p = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 2
  %bytes = load i64, ptr %bytes.p
  %live = load i64, ptr @cerune.array.live
  %remaining = sub i64 %live, %bytes
  store i64 %remaining, ptr @cerune.array.live
  %data = load ptr, ptr %owner
  call void @free(ptr %data)
  call void @free(ptr %owner)
  br label %done
done:
  store %cerune.array zeroinitializer, ptr %array
  ret void
}
define internal i64 @cerune.process.reserve(i64 %a, i64 %b, ptr %out, ptr %err) {
entry:
  %large.a = icmp ugt i64 %a, 1152921504606846975
  %large.b = icmp ugt i64 %b, 1152921504606846975
  %large = or i1 %large.a, %large.b
  br i1 %large, label %capacity, label %budget
budget:
  %sum = add i64 %a, %b
  %bytes = mul i64 %sum, 8
  %live = load i64, ptr @cerune.array.live
  %space = sub i64 @LIMIT@, %live
  %over = icmp ugt i64 %bytes, %space
  br i1 %over, label %capacity, label %stdout
stdout:
  %out.status = call i64 @cerune.process.reserve.one(i64 %a, ptr %out)
  %out.ok = icmp eq i64 %out.status, 0
  br i1 %out.ok, label %stderr, label %cancel
stderr:
  %err.status = call i64 @cerune.process.reserve.one(i64 %b, ptr %err)
  %err.ok = icmp eq i64 %err.status, 0
  br i1 %err.ok, label %done, label %cancel
cancel:
  call void @cerune.process.cancel(ptr %out)
  call void @cerune.process.cancel(ptr %err)
  ret i64 2
capacity:
  ret i64 1
done:
  ret i64 0
}
define internal void @cerune.process.finalize(ptr %array, i64 %n, i64 %channel, ptr %origin) {
entry:
  %bytes = alloca [4096 x i8]
  %copied.slot = alloca i64
  %value = load %cerune.array, ptr %array
  %capacity = extractvalue %cerune.array %value, 1
  %fits = icmp ule i64 %n, %capacity
  %status = select i1 %fits, i32 0, i32 13
  call void @cerune.process.check(i32 %status, i32 0, ptr %origin)
  %owner = extractvalue %cerune.array %value, 0
  %data = call ptr @cerune.process.array.data(%cerune.array %value)
  %context = load ptr, ptr @cerune.process.context
  br label %loop
loop:
  %offset = phi i64 [0, %entry], [%advance, %copied]
  %end = icmp eq i64 %offset, %n
  br i1 %end, label %finish, label %read
read:
  %remaining = sub i64 %n, %offset
  %short = icmp ult i64 %remaining, 4096
  %wanted = select i1 %short, i64 %remaining, i64 4096
  store i64 0, ptr %copied.slot
  %read.status = call i32 @cerune_process_v1_copy(ptr %context, i64 %channel, i64 %offset, ptr %bytes, i64 %wanted, ptr %copied.slot)
  call void @cerune.process.check(i32 %read.status, i32 0, ptr %origin)
  %count = load i64, ptr %copied.slot
  %exact = icmp eq i64 %count, %wanted
  %copy.status = select i1 %exact, i32 0, i32 13
  call void @cerune.process.check(i32 %copy.status, i32 0, ptr %origin)
  br label %elements
elements:
  %i = phi i64 [0, %read], [%next, %element]
  %complete = icmp eq i64 %i, %count
  br i1 %complete, label %copied, label %element
element:
  %source = getelementptr i8, ptr %bytes, i64 %i
  %byte = load i8, ptr %source
  %wide = zext i8 %byte to i64
  %index = add i64 %offset, %i
  %destination = getelementptr i64, ptr %data, i64 %index
  store i64 %wide, ptr %destination
  %next = add i64 %i, 1
  br label %elements
copied:
  %advance = add i64 %offset, %count
  br label %loop
finish:
  %result = insertvalue %cerune.array %value, i64 %n, 1
  store %cerune.array %result, ptr %array
  %owned = icmp ne ptr %owner, null
  br i1 %owned, label %initialized, label %done
initialized:
  %length = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  store i64 %n, ptr %length
  br label %done
done:
  ret void
}
