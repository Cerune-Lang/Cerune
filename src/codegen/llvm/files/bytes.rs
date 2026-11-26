//! ファイルの各バイトと動的配列の i64 要素を相互に変換します。
pub(super) const SOURCE: &str = r#"
define internal %cerune.array @cerune.files.read_bytes(%cerune.string %name, i64 %limit, ptr %origin) {
entry:
  %live = load i64, ptr @cerune.array.live
  %available = sub i64 @ARRAY_LIMIT@, %live
  %elements = udiv i64 %available, 8
  call void @cerune.files.validate_read(ptr @cerune.files.read-bytes, %cerune.string %name, i64 %limit, i64 %elements, ptr %origin)
  %bytes = mul i64 %limit, 8
  %array = call %cerune.array @cerune.array.try.storage(i64 %limit, i64 %bytes, i64 %bytes, i64 %live)
  %owner = extractvalue %cerune.array %array, 0
  %zero = icmp eq i64 %limit, 0
  br i1 %zero, label %read, label %check.owner
check.owner:
  %failed = icmp eq ptr %owner, null
  br i1 %failed, label %allocation.failed, label %storage
allocation.failed:
  call void @cerune.files.fail(ptr @cerune.files.read-bytes, ptr @cerune.files.reserve, ptr @cerune.files.allocation-failed, i64 %limit, i1 false, i64 0, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
storage:
  %buffer = load ptr, ptr %owner
  br label %read
read:
  %data = phi ptr [ null, %entry ], [ %buffer, %storage ]
  %read.result = call %cerune.files.read.result @cerune.files.read_into(%cerune.string %name, ptr %data, i64 %limit)
  %code = extractvalue %cerune.files.read.result %read.result, 0
  %error = icmp ne ptr %code, null
  br i1 %error, label %failure, label %success
failure:
  %unused = insertvalue %cerune.array %array, i64 0, 1
  call i1 @cerune.array.release.owner.last(%cerune.array %unused)
  call void @cerune.array.free.elements(%cerune.array %unused)
  call void @cerune.files.read_failed(ptr @cerune.files.read-bytes, i64 %limit, %cerune.files.read.result %read.result, ptr %origin)
  unreachable
success:
  %length = extractvalue %cerune.files.read.result %read.result, 3
  br i1 %zero, label %done, label %widen
widen:
  ; Walk backwards to preserve the remaining packed input bytes.
  %end = phi i64 [ %length, %success ], [ %index, %element ]
  %complete = icmp eq i64 %end, 0
  br i1 %complete, label %adopt, label %element
element:
  %index = sub i64 %end, 1
  %at = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %at
  %value = zext i8 %byte to i64
  %slot = getelementptr i64, ptr %data, i64 %index
  store i64 %value, ptr %slot
  br label %widen
adopt:
  %initialized = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  store i64 %length, ptr %initialized
  br label %done
done:
  %result = insertvalue %cerune.array %array, i64 %length, 1
  ret %cerune.array %result
}
define internal void @cerune.files.write_bytes_new(%cerune.string %name, %cerune.array %array, i64 %limit, ptr %origin) {
entry:
  %length = extractvalue %cerune.array %array, 1
  call void @cerune.files.validate_write(ptr @cerune.files.write-bytes-new, %cerune.string %name, i64 %length, i64 %limit, ptr %origin)
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %write, label %allocate
allocate:
  %buffer = call ptr @malloc(i64 %length)
  %failed = icmp eq ptr %buffer, null
  br i1 %failed, label %allocation.failed, label %storage
allocation.failed:
  call void @cerune.files.fail(ptr @cerune.files.write-bytes-new, ptr @cerune.files.reserve, ptr @cerune.files.allocation-failed, i64 %limit, i1 true, i64 %length, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
storage:
  %owner = extractvalue %cerune.array %array, 0
  %slots = load ptr, ptr %owner
  br label %pack
pack:
  %index = phi i64 [ 0, %storage ], [ %next, %pack ]
  %slot = getelementptr i64, ptr %slots, i64 %index
  %value = load i64, ptr %slot
  %byte = trunc i64 %value to i8
  %at = getelementptr i8, ptr %buffer, i64 %index
  store i8 %byte, ptr %at
  %next = add i64 %index, 1
  %complete = icmp eq i64 %next, %length
  br i1 %complete, label %write, label %pack
write:
  %data = phi ptr [ null, %entry ], [ %buffer, %pack ]
  call void @cerune.files.write_from(ptr @cerune.files.write-bytes-new, %cerune.string %name, ptr %data, i64 %length, i64 %limit, ptr %origin)
  call void @free(ptr %data)
  ret void
}
"#;
