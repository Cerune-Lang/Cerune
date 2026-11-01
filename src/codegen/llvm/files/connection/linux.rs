//! x86-64 Linux GNUのABI。statは144バイト、modeは24、dev/inoは0/8です。
pub(super) const DECLARATIONS: &str = r#"
declare i32 @open(ptr, i32, ...)
declare i32 @openat(i32, ptr, i32, ...)
declare i32 @fstat(i32, ptr)
declare i32 @close(i32)
declare i64 @read(i32, ptr, i64)
declare i64 @write(i32, ptr, i64)
declare ptr @__errno_location()
"#;
pub(super) const SOURCE: &str = r#"
define internal i32 @cerune.files.open_error(i32 %error, i1 %create) {
entry:
  switch i32 %error, label %failed [
    i32 2, label %missing
    i32 17, label %exists
    i32 21, label %directory
    i32 20, label %unsupported
    i32 40, label %unsupported
  ]
missing:
  ret i32 1
exists:
  ret i32 2
directory:
  %code = select i1 %create, i32 2, i32 3
  ret i32 %code
unsupported:
  ret i32 3
failed:
  ret i32 4
}
define internal void @cerune.files.connect(ptr %root) {
entry:
  %metadata = alloca [144 x i8], align 8
  %length = call i64 @strlen(ptr %root)
  %size = add i64 %length, 1
  %path = call ptr @malloc(i64 %size)
  %null = icmp eq ptr %path, null
  br i1 %null, label %allocation.failed, label %copy
allocation.failed:
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr @cerune.files.allocation-failed)
  unreachable
copy:
  call ptr @memcpy(ptr %path, ptr %root, i64 %size)
  br label %trim
trim:
  %n = phi i64 [ %length, %copy ], [ %previous, %erase ]
  %single = icmp ule i64 %n, 1
  br i1 %single, label %open, label %last
last:
  %previous = sub i64 %n, 1
  %last.p = getelementptr i8, ptr %path, i64 %previous
  %byte = load i8, ptr %last.p
  %slash = icmp eq i8 %byte, 47
  br i1 %slash, label %erase, label %open
erase:
  store i8 0, ptr %last.p
  br label %trim
open:
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 2752512)
  %ep = call ptr @__errno_location()
  %errno = load i32, ptr %ep
  call void @free(ptr %path)
  %is.failed = icmp slt i32 %fd, 0
  br i1 %is.failed, label %open.failed, label %stat
open.failed:
  %code = call i32 @cerune.files.open_error(i32 %errno, i1 false)
  %text = call ptr @cerune.files.open_code(i32 %code)
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr %text)
  unreachable
stat:
  %status = call i32 @fstat(i32 %fd, ptr %metadata)
  %stat.ok = icmp eq i32 %status, 0
  br i1 %stat.ok, label %kind, label %stat.failed
kind:
  %mode.ptr = getelementptr i8, ptr %metadata, i64 24
  %mode = load i32, ptr %mode.ptr
  %kind.bits = and i32 %mode, 61440
  %directory = icmp eq i32 %kind.bits, 16384
  br i1 %directory, label %connected, label %kind.failed
stat.failed:
  br label %failure
kind.failed:
  br label %failure
failure:
  %reason = phi ptr [ @cerune.files.open-failed, %stat.failed ], [ @cerune.files.unsupported-target, %kind.failed ]
  call i32 @close(i32 %fd)
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr %reason)
  unreachable
connected:
  %wide = sext i32 %fd to i64
  store i64 %wide, ptr @cerune.files.root
  store i1 true, ptr @cerune.files.configured
  ret void
}
define internal i32 @cerune_file_host_open(ptr %name, i64 %length, i32 %writing, ptr %result) {
entry:
  %path = alloca [256 x i8]
  %before = alloca [144 x i8], align 8
  %after = alloca [144 x i8], align 8
  %pinned = alloca [64 x i8]
  call ptr @memcpy(ptr %path, ptr %name, i64 %length)
  %end = getelementptr i8, ptr %path, i64 %length
  store i8 0, ptr %end
  %root = load i64, ptr @cerune.files.root
  %dirfd = trunc i64 %root to i32
  %create = icmp ne i32 %writing, 0
  %flags = select i1 %create, i32 655553, i32 2752512
  %fd = call i32 (i32, ptr, i32, ...) @openat(i32 %dirfd, ptr %path, i32 %flags, i32 438)
  %is.failed = icmp slt i32 %fd, 0
  br i1 %is.failed, label %open.failed, label %opened
open.failed:
  %ep = call ptr @__errno_location()
  %errno = load i32, ptr %ep
  %code = call i32 @cerune.files.open_error(i32 %errno, i1 %create)
  ret i32 %code
opened:
  br i1 %create, label %created, label %stat
created:
  %created.fd = sext i32 %fd to i64
  store i64 %created.fd, ptr %result
  ret i32 0
stat:
  %before.status = call i32 @fstat(i32 %fd, ptr %before)
  %before.ok = icmp eq i32 %before.status, 0
  br i1 %before.ok, label %kind, label %stat.failed
kind:
  %mode.ptr = getelementptr i8, ptr %before, i64 24
  %mode = load i32, ptr %mode.ptr
  %kind.bits = and i32 %mode, 61440
  %regular = icmp eq i32 %kind.bits, 32768
  br i1 %regular, label %reopen, label %kind.failed
stat.failed:
  call i32 @close(i32 %fd)
  ret i32 4
kind.failed:
  call i32 @close(i32 %fd)
  ret i32 3
reopen:
  call i32 (ptr, i64, ptr, ...) @snprintf(ptr %pinned, i64 64, ptr @cerune.files.proc.fd, i32 %fd)
  %readable = call i32 (ptr, i32, ...) @open(ptr %pinned, i32 524288)
  call i32 @close(i32 %fd)
  %reopen.failed = icmp slt i32 %readable, 0
  br i1 %reopen.failed, label %failed, label %restat
failed:
  ret i32 4
restat:
  %after.status = call i32 @fstat(i32 %readable, ptr %after)
  %after.ok = icmp eq i32 %after.status, 0
  br i1 %after.ok, label %identity, label %restat.failed
identity:
  %after.mode.ptr = getelementptr i8, ptr %after, i64 24
  %after.mode = load i32, ptr %after.mode.ptr
  %after.kind = and i32 %after.mode, 61440
  %after.regular = icmp eq i32 %after.kind, 32768
  %before.dev = load i64, ptr %before
  %after.dev = load i64, ptr %after
  %before.ino.ptr = getelementptr i8, ptr %before, i64 8
  %after.ino.ptr = getelementptr i8, ptr %after, i64 8
  %before.ino = load i64, ptr %before.ino.ptr
  %after.ino = load i64, ptr %after.ino.ptr
  %same.dev = icmp eq i64 %before.dev, %after.dev
  %same.ino = icmp eq i64 %before.ino, %after.ino
  %same = and i1 %same.dev, %same.ino
  %valid = and i1 %same, %after.regular
  br i1 %valid, label %ready, label %invalid
restat.failed:
  call i32 @close(i32 %readable)
  ret i32 4
invalid:
  call i32 @close(i32 %readable)
  ret i32 5
ready:
  %wide = sext i32 %readable to i64
  store i64 %wide, ptr %result
  ret i32 0
}
define internal i64 @cerune.files.transfer_result(i64 %count) {
entry:
  %is.failed = icmp slt i64 %count, 0
  br i1 %is.failed, label %error, label %done
error:
  %ep = call ptr @__errno_location()
  %errno = load i32, ptr %ep
  %interrupted = icmp eq i32 %errno, 4
  %failure = select i1 %interrupted, i64 -2, i64 -1
  ret i64 %failure
done:
  ret i64 %count
}
define internal i64 @cerune_file_host_read(i64 %handle, ptr %data, i64 %length) {
entry:
  %fd = trunc i64 %handle to i32
  %count = call i64 @read(i32 %fd, ptr %data, i64 %length)
  %result = call i64 @cerune.files.transfer_result(i64 %count)
  ret i64 %result
}
define internal i64 @cerune_file_host_write(i64 %handle, ptr %data, i64 %length) {
entry:
  %fd = trunc i64 %handle to i32
  %count = call i64 @write(i32 %fd, ptr %data, i64 %length)
  %result = call i64 @cerune.files.transfer_result(i64 %count)
  ret i64 %result
}
define internal i32 @cerune_file_host_close(i64 %handle) {
entry:
  %fd = trunc i64 %handle to i32
  %result = call i32 @close(i32 %fd)
  ret i32 %result
}
"#;
