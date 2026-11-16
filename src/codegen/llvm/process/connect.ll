define internal void @cerune.process.connect(ptr %path) {
entry:
  %absent = icmp eq ptr %path, null
  br i1 %absent, label %unconfigured, label %open
unconfigured:
  %empty = call i32 @cerune.process.create(i64 1, i64 @TARGET@, ptr null, i64 0, ptr @cerune.process.context)
  call void @cerune.process.check(i32 %empty, i32 0, ptr @cerune.process.host)
  ret void
open:
  %file = call ptr @@OPEN@(ptr %path, ptr @cerune.process.mode)
  %missing = icmp eq ptr %file, null
  br i1 %missing, label %failed, label %allocate
allocate:
  %data = call ptr @malloc(i64 1048577)
  %null = icmp eq ptr %data, null
  br i1 %null, label %close.failed, label %read
read:
  %n = call i64 @fread(ptr %data, i64 1, i64 1048577, ptr %file)
  %error = call i32 @ferror(ptr %file)
  %closed = call i32 @fclose(ptr %file)
  %empty.data = icmp eq i64 %n, 0
  %excess = icmp ugt i64 %n, 1048576
  %size.bad = or i1 %empty.data, %excess
  %read.bad = icmp ne i32 %error, 0
  %close.bad = icmp ne i32 %closed, 0
  %io.bad = or i1 %read.bad, %close.bad
  %bad = or i1 %size.bad, %io.bad
  br i1 %bad, label %free.failed, label %create
create:
  %status = call i32 @cerune.process.create(i64 1, i64 @TARGET@, ptr %data, i64 %n, ptr @cerune.process.context)
  call void @free(ptr %data)
  call void @cerune.process.check(i32 %status, i32 0, ptr @cerune.process.host)
  ret void
close.failed:
  call i32 @fclose(ptr %file)
  call void @cerune.process.fail(i32 15, ptr @cerune.process.host)
  unreachable
free.failed:
  call void @free(ptr %data)
  br label %failed
failed:
  call void @cerune.process.fail(i32 16, ptr @cerune.process.host)
  unreachable
}
