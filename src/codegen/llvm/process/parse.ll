
; runtime-startup-v1: only the leading marker introduces configuration.
define internal { i64, ptr, ptr } @cerune.process.parse(i32 %argc, ptr %argv) {
entry:
  %index.slot = alloca i64
  %root.slot = alloca ptr
  %config.slot = alloca ptr
  store ptr null, ptr %config.slot
  store ptr null, ptr %root.slot
  store i64 2, ptr %index.slot
  %argc.wide = sext i32 %argc to i64
  %has.argument = icmp sgt i32 %argc, 1
  br i1 %has.argument, label %marker, label %unchanged
marker:
  %first.slot = getelementptr ptr, ptr %argv, i64 1
  %first = load ptr, ptr %first.slot
  %comparison = call i32 @@CMP@(ptr %first, ptr @cerune.process.marker)
  %is.marker = icmp eq i32 %comparison, 0
  br i1 %is.marker, label %loop, label %unchanged
unchanged:
  ret { i64, ptr, ptr } { i64 1, ptr null, ptr null }
loop:
  %index = load i64, ptr %index.slot
  %end = icmp sge i64 %index, %argc.wide
  br i1 %end, label %missing.delimiter, label %argument
argument:
  %slot = getelementptr ptr, ptr %argv, i64 %index
  %arg = load ptr, ptr %slot
  %delim.cmp = call i32 @@CMP@(ptr %arg, ptr @cerune.process.delimiter)
  %is.delimiter = icmp eq i32 %delim.cmp, 0
  br i1 %is.delimiter, label %done, label %option
option:
  %option.cmp = call i32 @@CMP@(ptr %arg, ptr @cerune.process.root.option)
  %recognized = icmp eq i32 %option.cmp, 0
  %file.allowed = and i1 %recognized, @FILES@
  %config.cmp = call i32 @@CMP@(ptr %arg, ptr @cerune.process.config.option)
  %is.config = icmp eq i32 %config.cmp, 0
  %allowed = or i1 %file.allowed, %is.config
  %setting.slot = select i1 %is.config, ptr %config.slot, ptr %root.slot
  br i1 %allowed, label %duplicate.check, label %unknown
duplicate.check:
  %previous = load ptr, ptr %setting.slot
  %duplicate = icmp ne ptr %previous, null
  br i1 %duplicate, label %duplicate.setting, label %value.check
value.check:
  %value.index = add i64 %index, 1
  %missing = icmp sge i64 %value.index, %argc.wide
  br i1 %missing, label %invalid.root, label %value
value:
  %value.slot = getelementptr ptr, ptr %argv, i64 %value.index
  %root = load ptr, ptr %value.slot
  %first.char = load @CHAR@, ptr %root
  %empty = icmp eq @CHAR@ %first.char, 0
  br i1 %empty, label %invalid.root, label %prefix
prefix:
  %second.ptr = getelementptr @CHAR@, ptr %root, i64 1
  %second.char = load @CHAR@, ptr %second.ptr
  %first.dash = icmp eq @CHAR@ %first.char, 45
  %second.dash = icmp eq @CHAR@ %second.char, 45
  %looks.option = and i1 %first.dash, %second.dash
  br i1 %looks.option, label %invalid.root, label %accept
accept:
  store ptr %root, ptr %setting.slot
  %next = add i64 %value.index, 1
  store i64 %next, ptr %index.slot
  br label %loop
done:
  %start = add i64 %index, 1
  %selected.root = load ptr, ptr %root.slot
  %with.start = insertvalue { i64, ptr, ptr } poison, i64 %start, 0
  %result = insertvalue { i64, ptr, ptr } %with.start, ptr %selected.root, 1
  %config = load ptr, ptr %config.slot
  %complete = insertvalue { i64, ptr, ptr } %result, ptr %config, 2
  ret { i64, ptr, ptr } %complete
missing.delimiter:
  call void @cerune.process.startup_failed(ptr @cerune.process.configure, ptr @cerune.process.missing-delimiter)
  unreachable
unknown:
  call void @cerune.process.startup_failed(ptr @cerune.process.configure, ptr @cerune.process.unknown-setting)
  unreachable
duplicate.setting:
  call void @cerune.process.startup_failed(ptr @cerune.process.configure, ptr @cerune.process.duplicate-setting)
  unreachable
invalid.root:
  call void @cerune.process.startup_failed(ptr @cerune.process.configure, ptr @cerune.process.invalid-root)
  unreachable
}
