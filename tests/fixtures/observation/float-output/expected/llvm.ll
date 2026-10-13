target triple = "x86_64-unknown-linux-gnu"



@cerune.output.origin.0.0.16 = private constant [21 x i8] c" node=0 bytes=0..16\0A\00"
@cerune.output.origin.2.17.30 = private constant [22 x i8] c" node=2 bytes=17..30\0A\00"
@cerune.output.origin.4.31.51 = private constant [22 x i8] c" node=4 bytes=31..51\0A\00"
@cerune.output.origin.8.52.68 = private constant [22 x i8] c" node=8 bytes=52..68\0A\00"
@cerune.output.origin.10.69.83 = private constant [23 x i8] c" node=10 bytes=69..83\0A\00"
@cerune.output.origin.12.84.116 = private constant [24 x i8] c" node=12 bytes=84..116\0A\00"
@cerune.output.origin.14.117.147 = private constant [25 x i8] c" node=14 bytes=117..147\0A\00"
@cerune.output.origin.16.148.163 = private constant [25 x i8] c" node=16 bytes=148..163\0A\00"
@cerune.output.origin.19.164.176 = private constant [25 x i8] c" node=19 bytes=164..176\0A\00"
@cerune.output.origin.22.177.191 = private constant [25 x i8] c" node=22 bytes=177..191\0A\00"
@cerune.output.origin.24.192.203 = private constant [25 x i8] c" node=24 bytes=192..203\0A\00"
@cerune.output.origin.26.204.221 = private constant [25 x i8] c" node=26 bytes=204..221\0A\00"
@cerune.output.origin.28.222.236 = private constant [25 x i8] c" node=28 bytes=222..236\0A\00"
@cerune.output.origin.30.237.251 = private constant [25 x i8] c" node=30 bytes=237..251\0A\00"
@cerune.output.origin.32.252.264 = private constant [25 x i8] c" node=32 bytes=252..264\0A\00"
@cerune.output.record = private constant [61 x i8] c"cerune: io-v1 operation=write requested=%llu accepted=%llu%s\00"
@cerune.output.format.error = private constant [35 x i8] c"cerune: output formatting failed%s\00"
@cerune.output.true = private constant [5 x i8] c"true\0A"
@cerune.output.false = private constant [6 x i8] c"false\0A"
@cerune.output.lf = private constant [1 x i8] c"\0A"
@cerune.output.quote = private constant [1 x i8] c"\22"
declare i32 @snprintf(ptr, i64, ptr, ...)
declare void @_Exit(i32) noreturn
declare i64 @write(i32, ptr, i64)
declare ptr @__errno_location()
declare ptr @signal(i32, ptr)
%cerune.output = type { i64, i64, ptr, ptr }
define internal i64 @cerune.output.write(ptr %data, i64 %length) {
entry:
  %result = call i64 @write(i32 1, ptr %data, i64 %length)
  ret i64 %result
}
define internal void @cerune.output.diagnostic(ptr %data, i32 %length) noreturn {
entry:
  %positive = icmp sgt i32 %length, 0
  br i1 %positive, label %write, label %exit
write:
  %fits = icmp ult i32 %length, 512
  %count = select i1 %fits, i32 %length, i32 511
  %wide = zext i32 %count to i64
  call i64 @write(i32 2, ptr %data, i64 %wide)
  br label %exit
exit:
  call void @_Exit(i32 1)
  unreachable
}

define internal void @cerune.output.failed(ptr %op) noreturn {
entry:
  %state = load %cerune.output, ptr %op
  %requested = extractvalue %cerune.output %state, 0
  %accepted = extractvalue %cerune.output %state, 1
  %origin = extractvalue %cerune.output %state, 2
  %text = alloca [512 x i8]
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr @cerune.output.record, i64 %requested, i64 %accepted, ptr %origin)
  call void @cerune.output.diagnostic(ptr %text, i32 %n)
  unreachable
}
define internal void @cerune.output.text(ptr %data, i64 %length, ptr %origin) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %done, label %write
write:
  %op = alloca %cerune.output
  call void @cerune.output.begin(ptr %op, i64 %length, ptr %origin)
  call void @cerune.output.part(ptr %op, ptr %data, i64 %length)
  call void @cerune.output.end(ptr %op)
  br label %done
done:
  ret void
}
define internal void @cerune.output.begin(ptr %op, i64 %length, ptr %origin) {
entry:
  %initial = insertvalue %cerune.output zeroinitializer, i64 %length, 0
  %state = insertvalue %cerune.output %initial, ptr %origin, 2
  store %cerune.output %state, ptr %op
  %prior = call ptr @signal(i32 13, ptr inttoptr (i64 1 to ptr))
  %saved = getelementptr %cerune.output, ptr %op, i32 0, i32 3
  store ptr %prior, ptr %saved
  %bad = icmp eq ptr %prior, inttoptr (i64 -1 to ptr)
  br i1 %bad, label %failed, label %done
failed:
  call void @cerune.output.failed(ptr %op)
  unreachable
done:
  ret void
}
define internal void @cerune.output.end(ptr %op) {
entry:
  %saved = getelementptr %cerune.output, ptr %op, i32 0, i32 3
  %prior = load ptr, ptr %saved
  %restored = call ptr @signal(i32 13, ptr %prior)
  %bad = icmp eq ptr %restored, inttoptr (i64 -1 to ptr)
  br i1 %bad, label %failed, label %done
failed:
  call void @cerune.output.failed(ptr %op)
  unreachable
done:
  ret void
}

define internal void @cerune.output.part(ptr %op, ptr %data, i64 %length) {
entry:
  %accepted.ptr = getelementptr %cerune.output, ptr %op, i32 0, i32 1
  br label %condition
condition:
  %cursor = phi ptr [ %data, %entry ], [ %next, %advance ]
  %remaining = phi i64 [ %length, %entry ], [ %rest, %advance ]
  %done = icmp eq i64 %remaining, 0
  br i1 %done, label %end, label %prepare
prepare:
  %small = icmp ult i64 %remaining, 65536
  %request = select i1 %small, i64 %remaining, i64 65536
  br label %write
write:
  %n = call i64 @cerune.output.write(ptr %cursor, i64 %request)
  %negative = icmp slt i64 %n, 0
  br i1 %negative, label %error, label %check
error:
  %errno.ptr = call ptr @__errno_location()
  %errno = load i32, ptr %errno.ptr
  %interrupted = icmp eq i32 %errno, 4
  br i1 %interrupted, label %write, label %failed
check:
  %positive = icmp ugt i64 %n, 0
  %bounded = icmp ule i64 %n, %request
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %advance, label %failed
advance:
  %accepted = load i64, ptr %accepted.ptr
  %total = add i64 %accepted, %n
  store i64 %total, ptr %accepted.ptr
  %next = getelementptr i8, ptr %cursor, i64 %n
  %rest = sub i64 %remaining, %n
  br label %condition
failed:
  call void @cerune.output.failed(ptr %op)
  unreachable
end:
  ret void
}
@cerune.output.format.i64 = private constant [5 x i8] c"%lld\00"
define internal void @cerune.output.i64(i64 %value, i1 %newline, ptr %origin) {
entry:
  %text = alloca [128 x i8]
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 128, ptr @cerune.output.format.i64, i64 %value)
  %valid = icmp ult i32 %n, 127
  br i1 %valid, label %formatted, label %failed
failed:
  %error = alloca [512 x i8]
  %error.length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %error, i64 512, ptr @cerune.output.format.error, ptr %origin)
  call void @cerune.output.diagnostic(ptr %error, i32 %error.length)
  unreachable
formatted:
  %length = zext i32 %n to i64
  %tail = getelementptr i8, ptr %text, i64 %length
  store i8 10, ptr %tail
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)
  ret void
}
@cerune.output.format.u64 = private constant [5 x i8] c"%llu\00"
define internal void @cerune.output.u64(i64 %value, i1 %newline, ptr %origin) {
entry:
  %text = alloca [128 x i8]
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 128, ptr @cerune.output.format.u64, i64 %value)
  %valid = icmp ult i32 %n, 127
  br i1 %valid, label %formatted, label %failed
failed:
  %error = alloca [512 x i8]
  %error.length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %error, i64 512, ptr @cerune.output.format.error, ptr %origin)
  call void @cerune.output.diagnostic(ptr %error, i32 %error.length)
  unreachable
formatted:
  %length = zext i32 %n to i64
  %tail = getelementptr i8, ptr %text, i64 %length
  store i8 10, ptr %tail
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)
  ret void
}
@cerune.output.format.f32 = private constant [5 x i8] c"%.9g\00"
define internal void @cerune.output.f32(float %value, i1 %newline, ptr %origin) {
entry:
  %text = alloca [128 x i8]
  %wide = fpext float %value to double
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 128, ptr @cerune.output.format.f32, double %wide)
  %valid = icmp ult i32 %n, 127
  br i1 %valid, label %formatted, label %failed
failed:
  %error = alloca [512 x i8]
  %error.length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %error, i64 512, ptr @cerune.output.format.error, ptr %origin)
  call void @cerune.output.diagnostic(ptr %error, i32 %error.length)
  unreachable
formatted:
  %length = zext i32 %n to i64
  %tail = getelementptr i8, ptr %text, i64 %length
  store i8 10, ptr %tail
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)
  ret void
}
@cerune.output.format.f64 = private constant [6 x i8] c"%.17g\00"
define internal void @cerune.output.f64(double %value, i1 %newline, ptr %origin) {
entry:
  %text = alloca [128 x i8]
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 128, ptr @cerune.output.format.f64, double %value)
  %valid = icmp ult i32 %n, 127
  br i1 %valid, label %formatted, label %failed
failed:
  %error = alloca [512 x i8]
  %error.length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %error, i64 512, ptr @cerune.output.format.error, ptr %origin)
  call void @cerune.output.diagnostic(ptr %error, i32 %error.length)
  unreachable
formatted:
  %length = zext i32 %n to i64
  %tail = getelementptr i8, ptr %text, i64 %length
  store i8 10, ptr %tail
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)
  ret void
}
define internal void @cerune.output.bool(i1 %value, i1 %newline, ptr %origin) {
entry:
  %text = select i1 %value, ptr @cerune.output.true, ptr @cerune.output.false
  %length = select i1 %value, i64 4, i64 5
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)
  ret void
}
define i32 @main() {
entry:
  call void @cerune.output.f32(float 0x3BC79CA100000000, i1 true, ptr @cerune.output.origin.0.0.16)
  call void @cerune.output.f64(double 0x3BC79CA10C924223, i1 true, ptr @cerune.output.origin.2.17.30)
  %tmp0 = fcmp une double 0x3BC79CA10C924223, 0x0000000000000000
  call void @cerune.output.bool(i1 %tmp0, i1 true, ptr @cerune.output.origin.4.31.51)
  call void @cerune.output.f32(float 0x36A0000000000000, i1 true, ptr @cerune.output.origin.8.52.68)
  call void @cerune.output.f64(double 0x0000000000000001, i1 true, ptr @cerune.output.origin.10.69.83)
  call void @cerune.output.f32(float 0x47EFFFFFE0000000, i1 true, ptr @cerune.output.origin.12.84.116)
  call void @cerune.output.f64(double 0x7FEFFFFFFFFFFFFF, i1 true, ptr @cerune.output.origin.14.117.147)
  %tmp1 = fneg float 0x0000000000000000
  call void @cerune.output.f32(float %tmp1, i1 true, ptr @cerune.output.origin.16.148.163)
  %tmp2 = fneg double 0x0000000000000000
  call void @cerune.output.f64(double %tmp2, i1 true, ptr @cerune.output.origin.19.164.176)
  call void @cerune.output.f32(float 0x0000000000000000, i1 true, ptr @cerune.output.origin.22.177.191)
  call void @cerune.output.f64(double 0x0000000000000000, i1 true, ptr @cerune.output.origin.24.192.203)
  call void @cerune.output.f32(float 0x3F1A36E2E0000000, i1 true, ptr @cerune.output.origin.26.204.221)
  call void @cerune.output.f64(double 0x3F1A36E2EB1C432D, i1 true, ptr @cerune.output.origin.28.222.236)
  call void @cerune.output.f32(float 0x41CDCD6500000000, i1 true, ptr @cerune.output.origin.30.237.251)
  call void @cerune.output.f64(double 0x4376345785D8A000, i1 true, ptr @cerune.output.origin.32.252.264)
  ret i32 0
}
