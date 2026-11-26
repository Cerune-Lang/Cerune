//! 転送・close・UTF-8検査と所有管理。接続関数は意味上の再試行を行いません。
pub(super) const SOURCE: &str = r#"
; code, phase, cleanup, transferred count, EOF observed, progress known
%cerune.files.read.result = type { ptr, ptr, ptr, i64, i1, i1 }

define internal void @cerune.files.validate_read(ptr %operation, %cerune.string %name, i64 %limit, i64 %available, ptr %origin) {
entry:
  %validation = call ptr @cerune.files.check(%cerune.string %name, i64 %limit)
  %invalid = icmp ne ptr %validation, null
  br i1 %invalid, label %validation.failed, label %budget
validation.failed:
  call void @cerune.files.fail(ptr %operation, ptr @cerune.files.validate, ptr %validation, i64 %limit, i1 false, i64 0, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
budget:
  %over = icmp ugt i64 %limit, %available
  br i1 %over, label %budget.failed, label %done
budget.failed:
  call void @cerune.files.fail(ptr %operation, ptr @cerune.files.reserve, ptr @cerune.files.budget-exceeded, i64 %limit, i1 false, i64 0, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
done:
  ret void
}
define internal %cerune.files.read.result @cerune.files.read_into(%cerune.string %name, ptr %data, i64 %limit) {
entry:
  %handle.slot = alloca i64
  %probe = alloca i8
  %code.slot = alloca ptr
  %phase.slot = alloca ptr
  %known.slot = alloca i1
  %eof.slot = alloca i1
  %length.slot = alloca i64
  %transferred.slot = alloca i64
  store ptr null, ptr %code.slot
  store ptr @cerune.files.read, ptr %phase.slot
  store i1 true, ptr %known.slot
  store i1 false, ptr %eof.slot
  store i64 0, ptr %length.slot
  store i64 0, ptr %transferred.slot
  %open.code = call ptr @cerune.files.open_file(%cerune.string %name, i32 0, ptr %handle.slot)
  %open.failed = icmp ne ptr %open.code, null
  br i1 %open.failed, label %open.failure, label %ready
open.failure:
  %open.result = insertvalue %cerune.files.read.result { ptr null, ptr @cerune.files.open, ptr @cerune.files.not-attempted, i64 0, i1 false, i1 true }, ptr %open.code, 0
  ret %cerune.files.read.result %open.result
ready:
  %handle = load i64, ptr %handle.slot
  br label %read
read:
  %length = load i64, ptr %length.slot
  %full = icmp eq i64 %length, %limit
  %remaining = sub i64 %limit, %length
  %small = icmp ult i64 %remaining, 4096
  %chunk = select i1 %small, i64 %remaining, i64 4096
  %request = select i1 %full, i64 1, i64 %chunk
  %at = getelementptr i8, ptr %data, i64 %length
  %into = select i1 %full, ptr %probe, ptr %at
  %count = call i64 @cerune_file_host_read(i64 %handle, ptr %into, i64 %request)
  %interrupted = icmp eq i64 %count, -2
  br i1 %interrupted, label %read, label %check.error
check.error:
  %failed = icmp slt i64 %count, 0
  br i1 %failed, label %read.failed, label %check.response
check.response:
  %bad = icmp ugt i64 %count, %request
  br i1 %bad, label %response.failed, label %check.eof
check.eof:
  %eof = icmp eq i64 %count, 0
  br i1 %eof, label %eof.reached, label %received
eof.reached:
  store i1 true, ptr %eof.slot
  br label %close
received:
  %more = add i64 %length, %count
  store i64 %more, ptr %transferred.slot
  br i1 %full, label %limit.failed, label %advance
advance:
  store i64 %more, ptr %length.slot
  br label %read
read.failed:
  store ptr @cerune.files.read-failed, ptr %code.slot
  br label %close
response.failed:
  store ptr @cerune.files.invalid-response, ptr %code.slot
  store i1 false, ptr %known.slot
  br label %close
limit.failed:
  store ptr @cerune.files.limit-exceeded, ptr %code.slot
  br label %close
close:
  %status = call i32 @cerune_file_host_close(i64 %handle)
  %closed = icmp eq i32 %status, 0
  %cleanup = select i1 %closed, ptr @cerune.files.closed, ptr @cerune.files.failed
  %primary = load ptr, ptr %code.slot
  %has.primary = icmp ne ptr %primary, null
  br i1 %has.primary, label %finish, label %check.close
check.close:
  br i1 %closed, label %finish, label %close.failed
close.failed:
  store ptr @cerune.files.close-failed, ptr %code.slot
  store ptr @cerune.files.close, ptr %phase.slot
  br label %finish
finish:
  %code = load ptr, ptr %code.slot
  %phase = load ptr, ptr %phase.slot
  %transferred = load i64, ptr %transferred.slot
  %requested.known = load i1, ptr %eof.slot
  %known = load i1, ptr %known.slot
  %r0 = insertvalue %cerune.files.read.result poison, ptr %code, 0
  %r1 = insertvalue %cerune.files.read.result %r0, ptr %phase, 1
  %r2 = insertvalue %cerune.files.read.result %r1, ptr %cleanup, 2
  %r3 = insertvalue %cerune.files.read.result %r2, i64 %transferred, 3
  %r4 = insertvalue %cerune.files.read.result %r3, i1 %requested.known, 4
  %result = insertvalue %cerune.files.read.result %r4, i1 %known, 5
  ret %cerune.files.read.result %result
}
define internal void @cerune.files.read_failed(ptr %operation, i64 %limit, %cerune.files.read.result %result, ptr %origin) noreturn {
entry:
  %code = extractvalue %cerune.files.read.result %result, 0
  %phase = extractvalue %cerune.files.read.result %result, 1
  %cleanup = extractvalue %cerune.files.read.result %result, 2
  %count = extractvalue %cerune.files.read.result %result, 3
  %eof = extractvalue %cerune.files.read.result %result, 4
  %known = extractvalue %cerune.files.read.result %result, 5
  call void @cerune.files.fail(ptr %operation, ptr %phase, ptr %code, i64 %limit, i1 %eof, i64 %count, i64 %count, i1 %known, ptr @cerune.files.no, ptr %cleanup, ptr %origin)
  unreachable
}
define internal %cerune.string @cerune.files.read_text(%cerune.string %name, i64 %limit, ptr %origin) {
entry:
  %live = load i64, ptr @cerune.string.live
  %available = sub i64 @LIMIT@, %live
  call void @cerune.files.validate_read(ptr @cerune.files.read-text, %cerune.string %name, i64 %limit, i64 %available, ptr %origin)
  %size = add i64 %limit, 24
  %owner = call ptr @malloc(i64 %size)
  %null = icmp eq ptr %owner, null
  br i1 %null, label %allocation.failed, label %open
allocation.failed:
  call void @cerune.files.fail(ptr @cerune.files.read-text, ptr @cerune.files.reserve, ptr @cerune.files.allocation-failed, i64 %limit, i1 false, i64 0, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
open:
  %reserved = add i64 %live, %limit
  store i64 %reserved, ptr @cerune.string.live
  %data = getelementptr i8, ptr %owner, i64 24
  %result.read = call %cerune.files.read.result @cerune.files.read_into(%cerune.string %name, ptr %data, i64 %limit)
  %code = extractvalue %cerune.files.read.result %result.read, 0
  %failed = icmp ne ptr %code, null
  br i1 %failed, label %failure, label %decode
decode:
  %final.length = extractvalue %cerune.files.read.result %result.read, 3
  %utf8 = call i1 @cerune.utf8(ptr %data, i64 %final.length)
  br i1 %utf8, label %success, label %decode.failed
decode.failed:
  %bad = insertvalue %cerune.files.read.result %result.read, ptr @cerune.files.invalid-utf8, 0
  %decode.result = insertvalue %cerune.files.read.result %bad, ptr @cerune.files.decode, 1
  br label %failure
failure:
  %failure.result = phi %cerune.files.read.result [ %result.read, %open ], [ %decode.result, %decode.failed ]
  call void @free(ptr %owner)
  store i64 %live, ptr @cerune.string.live
  call void @cerune.files.read_failed(ptr @cerune.files.read-text, i64 %limit, %cerune.files.read.result %failure.result, ptr %origin)
  unreachable
success:
  %empty = icmp eq i64 %final.length, 0
  br i1 %empty, label %release.empty, label %adopt
release.empty:
  call void @free(ptr %owner)
  store i64 %live, ptr @cerune.string.live
  ret %cerune.string { ptr @cerune.string.empty, i64 0 }
adopt:
  %head = load ptr, ptr @cerune.string.owners
  store ptr %head, ptr %owner
  %references = getelementptr i8, ptr %owner, i64 8
  store i64 1, ptr %references
  %owner.length = getelementptr i8, ptr %owner, i64 16
  store i64 %final.length, ptr %owner.length
  store ptr %owner, ptr @cerune.string.owners
  %charged = add i64 %live, %final.length
  store i64 %charged, ptr @cerune.string.live
  %result.data = insertvalue %cerune.string poison, ptr %data, 0
  %result = insertvalue %cerune.string %result.data, i64 %final.length, 1
  ret %cerune.string %result
}
define internal void @cerune.files.validate_write(ptr %operation, %cerune.string %name, i64 %length, i64 %limit, ptr %origin) {
entry:
  %validation = call ptr @cerune.files.check(%cerune.string %name, i64 %limit)
  %invalid = icmp ne ptr %validation, null
  br i1 %invalid, label %validation.failed, label %size
size:
  %over = icmp ugt i64 %length, %limit
  br i1 %over, label %size.failed, label %done
size.failed:
  br label %validation.failed
validation.failed:
  %validation.code = phi ptr [ %validation, %entry ], [ @cerune.files.limit-exceeded, %size.failed ]
  call void @cerune.files.fail(ptr %operation, ptr @cerune.files.validate, ptr %validation.code, i64 %limit, i1 true, i64 %length, i64 0, i1 true, ptr @cerune.files.no, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
done:
  ret void
}
define internal void @cerune.files.write_from(ptr %operation, %cerune.string %name, ptr %data, i64 %length, i64 %limit, ptr %origin) {
entry:
  %handle.slot = alloca i64
  %code.slot = alloca ptr
  %phase.slot = alloca ptr
  %known.slot = alloca i1
  %transferred.slot = alloca i64
  store ptr null, ptr %code.slot
  store ptr @cerune.files.write, ptr %phase.slot
  store i1 true, ptr %known.slot
  store i64 0, ptr %transferred.slot
  %open.code = call ptr @cerune.files.open_file(%cerune.string %name, i32 1, ptr %handle.slot)
  %open.failed = icmp ne ptr %open.code, null
  br i1 %open.failed, label %open.failure, label %ready
open.failure:
  %open.invalid = icmp eq ptr %open.code, @cerune.files.invalid-response
  %created = select i1 %open.invalid, ptr @cerune.files.unknown, ptr @cerune.files.no
  call void @cerune.files.fail(ptr %operation, ptr @cerune.files.open, ptr %open.code, i64 %limit, i1 true, i64 %length, i64 0, i1 true, ptr %created, ptr @cerune.files.not-attempted, ptr %origin)
  unreachable
ready:
  %handle = load i64, ptr %handle.slot
  br label %write
write:
  %transferred = load i64, ptr %transferred.slot
  %done = icmp eq i64 %transferred, %length
  br i1 %done, label %close, label %transfer
transfer:
  %request = sub i64 %length, %transferred
  %at = getelementptr i8, ptr %data, i64 %transferred
  %count = call i64 @cerune_file_host_write(i64 %handle, ptr %at, i64 %request)
  %interrupted = icmp eq i64 %count, -2
  br i1 %interrupted, label %write, label %check.error
check.error:
  %failed = icmp slt i64 %count, 0
  br i1 %failed, label %write.failed, label %check.response
check.response:
  %invalid.response = icmp ugt i64 %count, %request
  br i1 %invalid.response, label %response.failed, label %check.progress
check.progress:
  %zero = icmp eq i64 %count, 0
  br i1 %zero, label %zero.failed, label %advance
advance:
  %more = add i64 %transferred, %count
  store i64 %more, ptr %transferred.slot
  br label %write
write.failed:
  store ptr @cerune.files.write-failed, ptr %code.slot
  br label %close
response.failed:
  store ptr @cerune.files.invalid-response, ptr %code.slot
  store i1 false, ptr %known.slot
  br label %close
zero.failed:
  store ptr @cerune.files.zero-progress, ptr %code.slot
  br label %close
close:
  %status = call i32 @cerune_file_host_close(i64 %handle)
  %closed = icmp eq i32 %status, 0
  %cleanup = select i1 %closed, ptr @cerune.files.closed, ptr @cerune.files.failed
  %primary = load ptr, ptr %code.slot
  %has.primary = icmp ne ptr %primary, null
  %needs.close.error = xor i1 %closed, true
  %no.primary = xor i1 %has.primary, true
  %close.failed = and i1 %needs.close.error, %no.primary
  br i1 %close.failed, label %close.failure, label %finish
close.failure:
  store ptr @cerune.files.close-failed, ptr %code.slot
  store ptr @cerune.files.close, ptr %phase.slot
  br label %finish
finish:
  %final.code = load ptr, ptr %code.slot
  %has.error = icmp ne ptr %final.code, null
  br i1 %has.error, label %failure, label %success
failure:
  %phase = load ptr, ptr %phase.slot
  %accepted = load i64, ptr %transferred.slot
  %known = load i1, ptr %known.slot
  call void @cerune.files.fail(ptr %operation, ptr %phase, ptr %final.code, i64 %limit, i1 true, i64 %length, i64 %accepted, i1 %known, ptr @cerune.files.yes, ptr %cleanup, ptr %origin)
  unreachable
success:
  ret void
}
define internal void @cerune.files.write_new(%cerune.string %name, %cerune.string %text, i64 %limit, ptr %origin) {
entry:
  %data = extractvalue %cerune.string %text, 0
  %length = extractvalue %cerune.string %text, 1
  call void @cerune.files.validate_write(ptr @cerune.files.write-new, %cerune.string %name, i64 %length, i64 %limit, ptr %origin)
  call void @cerune.files.write_from(ptr @cerune.files.write-new, %cerune.string %name, ptr %data, i64 %length, i64 %limit, ptr %origin)
  ret void
}
"#;
