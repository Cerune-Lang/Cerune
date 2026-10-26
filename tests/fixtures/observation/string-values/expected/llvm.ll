target triple = "x86_64-unknown-linux-gnu"

%cerune.string = type { ptr, i64 }
@cerune.string.0 = private unnamed_addr constant [11 x i8] c"\E6\97\A5\E6\9C\AC\E8\AA\9E\0A\00"
@cerune.string.1 = private unnamed_addr constant [7 x i8] c"\63\68\61\6E\67\65\64"
@cerune.string.2 = private unnamed_addr constant [11 x i8] c"\E6\97\A5\E6\9C\AC\E8\AA\9E\0A\00"



define internal i1 @cerune.string.equal(%cerune.string %left, %cerune.string %right) {
entry:
  %left.data = extractvalue %cerune.string %left, 0
  %left.length = extractvalue %cerune.string %left, 1
  %right.data = extractvalue %cerune.string %right, 0
  %right.length = extractvalue %cerune.string %right, 1
  %same.length = icmp eq i64 %left.length, %right.length
  br i1 %same.length, label %condition, label %different
condition:
  %index = phi i64 [ 0, %entry ], [ %next, %advance ]
  %done = icmp eq i64 %index, %left.length
  br i1 %done, label %equal, label %compare
compare:
  %left.ptr = getelementptr inbounds i8, ptr %left.data, i64 %index
  %right.ptr = getelementptr inbounds i8, ptr %right.data, i64 %index
  %left.byte = load i8, ptr %left.ptr
  %right.byte = load i8, ptr %right.ptr
  %same.byte = icmp eq i8 %left.byte, %right.byte
  br i1 %same.byte, label %advance, label %different
advance:
  %next = add i64 %index, 1
  br label %condition
equal:
  ret i1 true
different:
  ret i1 false
}
@cerune.output.origin.6.75.112 = private constant [23 x i8] c" node=6 bytes=75..112\0A\00"
@cerune.output.origin.10.113.125 = private constant [25 x i8] c" node=10 bytes=113..125\0A\00"
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
define internal i64 @cerune.output.escape(i8 %byte, ptr %out) {
entry:
  switch i8 %byte, label %plain [
    i8 0, label %escape0
    i8 1, label %escape1
    i8 2, label %escape2
    i8 3, label %escape3
    i8 4, label %escape4
    i8 5, label %escape5
    i8 6, label %escape6
    i8 7, label %escape7
    i8 8, label %escape8
    i8 9, label %escape9
    i8 10, label %escape10
    i8 11, label %escape11
    i8 12, label %escape12
    i8 13, label %escape13
    i8 14, label %escape14
    i8 15, label %escape15
    i8 16, label %escape16
    i8 17, label %escape17
    i8 18, label %escape18
    i8 19, label %escape19
    i8 20, label %escape20
    i8 21, label %escape21
    i8 22, label %escape22
    i8 23, label %escape23
    i8 24, label %escape24
    i8 25, label %escape25
    i8 26, label %escape26
    i8 27, label %escape27
    i8 28, label %escape28
    i8 29, label %escape29
    i8 30, label %escape30
    i8 31, label %escape31
    i8 34, label %escape34
    i8 92, label %escape92
    i8 127, label %escape127
  ]
plain:
  store i8 %byte, ptr %out
  ret i64 1
escape0:
  %p0.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p0.0
  %p0.1 = getelementptr i8, ptr %out, i64 1
  store i8 48, ptr %p0.1
  ret i64 2

escape1:
  %p1.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p1.0
  %p1.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p1.1
  %p1.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p1.2
  %p1.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p1.3
  %p1.4 = getelementptr i8, ptr %out, i64 4
  store i8 49, ptr %p1.4
  %p1.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p1.5
  ret i64 6

escape2:
  %p2.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p2.0
  %p2.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p2.1
  %p2.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p2.2
  %p2.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p2.3
  %p2.4 = getelementptr i8, ptr %out, i64 4
  store i8 50, ptr %p2.4
  %p2.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p2.5
  ret i64 6

escape3:
  %p3.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p3.0
  %p3.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p3.1
  %p3.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p3.2
  %p3.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p3.3
  %p3.4 = getelementptr i8, ptr %out, i64 4
  store i8 51, ptr %p3.4
  %p3.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p3.5
  ret i64 6

escape4:
  %p4.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p4.0
  %p4.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p4.1
  %p4.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p4.2
  %p4.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p4.3
  %p4.4 = getelementptr i8, ptr %out, i64 4
  store i8 52, ptr %p4.4
  %p4.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p4.5
  ret i64 6

escape5:
  %p5.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p5.0
  %p5.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p5.1
  %p5.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p5.2
  %p5.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p5.3
  %p5.4 = getelementptr i8, ptr %out, i64 4
  store i8 53, ptr %p5.4
  %p5.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p5.5
  ret i64 6

escape6:
  %p6.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p6.0
  %p6.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p6.1
  %p6.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p6.2
  %p6.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p6.3
  %p6.4 = getelementptr i8, ptr %out, i64 4
  store i8 54, ptr %p6.4
  %p6.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p6.5
  ret i64 6

escape7:
  %p7.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p7.0
  %p7.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p7.1
  %p7.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p7.2
  %p7.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p7.3
  %p7.4 = getelementptr i8, ptr %out, i64 4
  store i8 55, ptr %p7.4
  %p7.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p7.5
  ret i64 6

escape8:
  %p8.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p8.0
  %p8.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p8.1
  %p8.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p8.2
  %p8.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p8.3
  %p8.4 = getelementptr i8, ptr %out, i64 4
  store i8 56, ptr %p8.4
  %p8.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p8.5
  ret i64 6

escape9:
  %p9.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p9.0
  %p9.1 = getelementptr i8, ptr %out, i64 1
  store i8 116, ptr %p9.1
  ret i64 2

escape10:
  %p10.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p10.0
  %p10.1 = getelementptr i8, ptr %out, i64 1
  store i8 110, ptr %p10.1
  ret i64 2

escape11:
  %p11.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p11.0
  %p11.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p11.1
  %p11.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p11.2
  %p11.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p11.3
  %p11.4 = getelementptr i8, ptr %out, i64 4
  store i8 98, ptr %p11.4
  %p11.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p11.5
  ret i64 6

escape12:
  %p12.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p12.0
  %p12.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p12.1
  %p12.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p12.2
  %p12.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p12.3
  %p12.4 = getelementptr i8, ptr %out, i64 4
  store i8 99, ptr %p12.4
  %p12.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p12.5
  ret i64 6

escape13:
  %p13.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p13.0
  %p13.1 = getelementptr i8, ptr %out, i64 1
  store i8 114, ptr %p13.1
  ret i64 2

escape14:
  %p14.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p14.0
  %p14.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p14.1
  %p14.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p14.2
  %p14.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p14.3
  %p14.4 = getelementptr i8, ptr %out, i64 4
  store i8 101, ptr %p14.4
  %p14.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p14.5
  ret i64 6

escape15:
  %p15.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p15.0
  %p15.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p15.1
  %p15.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p15.2
  %p15.3 = getelementptr i8, ptr %out, i64 3
  store i8 48, ptr %p15.3
  %p15.4 = getelementptr i8, ptr %out, i64 4
  store i8 102, ptr %p15.4
  %p15.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p15.5
  ret i64 6

escape16:
  %p16.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p16.0
  %p16.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p16.1
  %p16.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p16.2
  %p16.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p16.3
  %p16.4 = getelementptr i8, ptr %out, i64 4
  store i8 48, ptr %p16.4
  %p16.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p16.5
  ret i64 6

escape17:
  %p17.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p17.0
  %p17.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p17.1
  %p17.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p17.2
  %p17.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p17.3
  %p17.4 = getelementptr i8, ptr %out, i64 4
  store i8 49, ptr %p17.4
  %p17.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p17.5
  ret i64 6

escape18:
  %p18.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p18.0
  %p18.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p18.1
  %p18.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p18.2
  %p18.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p18.3
  %p18.4 = getelementptr i8, ptr %out, i64 4
  store i8 50, ptr %p18.4
  %p18.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p18.5
  ret i64 6

escape19:
  %p19.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p19.0
  %p19.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p19.1
  %p19.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p19.2
  %p19.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p19.3
  %p19.4 = getelementptr i8, ptr %out, i64 4
  store i8 51, ptr %p19.4
  %p19.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p19.5
  ret i64 6

escape20:
  %p20.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p20.0
  %p20.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p20.1
  %p20.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p20.2
  %p20.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p20.3
  %p20.4 = getelementptr i8, ptr %out, i64 4
  store i8 52, ptr %p20.4
  %p20.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p20.5
  ret i64 6

escape21:
  %p21.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p21.0
  %p21.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p21.1
  %p21.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p21.2
  %p21.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p21.3
  %p21.4 = getelementptr i8, ptr %out, i64 4
  store i8 53, ptr %p21.4
  %p21.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p21.5
  ret i64 6

escape22:
  %p22.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p22.0
  %p22.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p22.1
  %p22.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p22.2
  %p22.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p22.3
  %p22.4 = getelementptr i8, ptr %out, i64 4
  store i8 54, ptr %p22.4
  %p22.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p22.5
  ret i64 6

escape23:
  %p23.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p23.0
  %p23.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p23.1
  %p23.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p23.2
  %p23.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p23.3
  %p23.4 = getelementptr i8, ptr %out, i64 4
  store i8 55, ptr %p23.4
  %p23.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p23.5
  ret i64 6

escape24:
  %p24.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p24.0
  %p24.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p24.1
  %p24.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p24.2
  %p24.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p24.3
  %p24.4 = getelementptr i8, ptr %out, i64 4
  store i8 56, ptr %p24.4
  %p24.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p24.5
  ret i64 6

escape25:
  %p25.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p25.0
  %p25.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p25.1
  %p25.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p25.2
  %p25.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p25.3
  %p25.4 = getelementptr i8, ptr %out, i64 4
  store i8 57, ptr %p25.4
  %p25.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p25.5
  ret i64 6

escape26:
  %p26.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p26.0
  %p26.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p26.1
  %p26.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p26.2
  %p26.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p26.3
  %p26.4 = getelementptr i8, ptr %out, i64 4
  store i8 97, ptr %p26.4
  %p26.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p26.5
  ret i64 6

escape27:
  %p27.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p27.0
  %p27.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p27.1
  %p27.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p27.2
  %p27.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p27.3
  %p27.4 = getelementptr i8, ptr %out, i64 4
  store i8 98, ptr %p27.4
  %p27.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p27.5
  ret i64 6

escape28:
  %p28.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p28.0
  %p28.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p28.1
  %p28.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p28.2
  %p28.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p28.3
  %p28.4 = getelementptr i8, ptr %out, i64 4
  store i8 99, ptr %p28.4
  %p28.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p28.5
  ret i64 6

escape29:
  %p29.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p29.0
  %p29.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p29.1
  %p29.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p29.2
  %p29.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p29.3
  %p29.4 = getelementptr i8, ptr %out, i64 4
  store i8 100, ptr %p29.4
  %p29.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p29.5
  ret i64 6

escape30:
  %p30.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p30.0
  %p30.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p30.1
  %p30.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p30.2
  %p30.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p30.3
  %p30.4 = getelementptr i8, ptr %out, i64 4
  store i8 101, ptr %p30.4
  %p30.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p30.5
  ret i64 6

escape31:
  %p31.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p31.0
  %p31.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p31.1
  %p31.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p31.2
  %p31.3 = getelementptr i8, ptr %out, i64 3
  store i8 49, ptr %p31.3
  %p31.4 = getelementptr i8, ptr %out, i64 4
  store i8 102, ptr %p31.4
  %p31.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p31.5
  ret i64 6

escape34:
  %p34.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p34.0
  %p34.1 = getelementptr i8, ptr %out, i64 1
  store i8 34, ptr %p34.1
  ret i64 2

escape92:
  %p92.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p92.0
  %p92.1 = getelementptr i8, ptr %out, i64 1
  store i8 92, ptr %p92.1
  ret i64 2

escape127:
  %p127.0 = getelementptr i8, ptr %out, i64 0
  store i8 92, ptr %p127.0
  %p127.1 = getelementptr i8, ptr %out, i64 1
  store i8 117, ptr %p127.1
  %p127.2 = getelementptr i8, ptr %out, i64 2
  store i8 123, ptr %p127.2
  %p127.3 = getelementptr i8, ptr %out, i64 3
  store i8 55, ptr %p127.3
  %p127.4 = getelementptr i8, ptr %out, i64 4
  store i8 102, ptr %p127.4
  %p127.5 = getelementptr i8, ptr %out, i64 5
  store i8 125, ptr %p127.5
  ret i64 6

}

define internal void @cerune.output.string(%cerune.string %value, i1 %newline, ptr %origin) {
entry:
  %data = extractvalue %cerune.string %value, 0
  %length = extractvalue %cerune.string %value, 1
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  %op = alloca %cerune.output
  call void @cerune.output.begin(ptr %op, i64 %requested, ptr %origin)
  call void @cerune.output.part(ptr %op, ptr %data, i64 %length)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.lf, i64 %extra)
  call void @cerune.output.end(ptr %op)
  ret void
}
define internal void @cerune.output.quoted(%cerune.string %value, i1 %newline, ptr %origin) {
entry:
  %data = extractvalue %cerune.string %value, 0
  %length = extractvalue %cerune.string %value, 1
  %extra = zext i1 %newline to i64
  %initial = add i64 2, %extra
  %scratch = alloca [6 x i8]
  %buffer = alloca [256 x i8]
  %op = alloca %cerune.output
  br label %count
count:
  %index = phi i64 [ 0, %entry ], [ %next.index, %count.byte ]
  %requested = phi i64 [ %initial, %entry ], [ %next.count, %count.byte ]
  %counted = icmp eq i64 %index, %length
  br i1 %counted, label %begin, label %count.byte
count.byte:
  %source = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %source
  %n = call i64 @cerune.output.escape(i8 %byte, ptr %scratch)
  %next.count = add i64 %requested, %n
  %next.index = add i64 %index, 1
  br label %count
begin:
  call void @cerune.output.begin(ptr %op, i64 %requested, ptr %origin)
  store i8 34, ptr %buffer
  br label %condition
condition:
  %i = phi i64 [ 0, %begin ], [ %next, %append ]
  %used = phi i64 [ 1, %begin ], [ %filled, %append ]
  %done = icmp eq i64 %i, %length
  br i1 %done, label %tail, label %space
space:
  %full = icmp ugt i64 %used, 250
  br i1 %full, label %drain, label %append
drain:
  call void @cerune.output.part(ptr %op, ptr %buffer, i64 %used)
  br label %append
append:
  %offset = phi i64 [ %used, %space ], [ 0, %drain ]
  %p = getelementptr i8, ptr %data, i64 %i
  %b = load i8, ptr %p
  %destination = getelementptr i8, ptr %buffer, i64 %offset
  %size = call i64 @cerune.output.escape(i8 %b, ptr %destination)
  %filled = add i64 %offset, %size
  %next = add i64 %i, 1
  br label %condition
tail:
  call void @cerune.output.part(ptr %op, ptr %buffer, i64 %used)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.quote, i64 1)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.lf, i64 %extra)
  call void @cerune.output.end(ptr %op)
  ret void
}
define i32 @main() {
entry:
  %cerune_text = alloca %cerune.string
  %cerune_saved = alloca %cerune.string
  store %cerune.string { ptr @cerune.string.0, i64 11 }, ptr %cerune_text
  %tmp0 = load %cerune.string, ptr %cerune_text
  store %cerune.string %tmp0, ptr %cerune_saved
  store %cerune.string { ptr @cerune.string.1, i64 7 }, ptr %cerune_text
  %tmp1 = load %cerune.string, ptr %cerune_saved
  %tmp2 = call i1 @cerune.string.equal(%cerune.string %tmp1, %cerune.string { ptr @cerune.string.2, i64 11 })
  call void @cerune.output.bool(i1 %tmp2, i1 true, ptr @cerune.output.origin.6.75.112)
  %tmp3 = load %cerune.string, ptr %cerune_text
  call void @cerune.output.string(%cerune.string %tmp3, i1 true, ptr @cerune.output.origin.10.113.125)
  call void @cerune_call_finish()
  ret i32 0
}
; call_depth_limit=128; not a physical stack/memory guarantee
declare ptr @realloc(ptr, i64)
declare void @free(ptr)
declare i32 @fflush(ptr)
@stdout = external global ptr
@cerune_call_header = private constant [36 x i8] c"\63\61\6C\6C\2D\76\31\20\72\65\61\73\6F\6E\3D\64\65\70\74\68\2D\6C\69\6D\69\74\20\6C\69\6D\69\74\3D\31\32\38"
@cerune_call_frame_prefix = private constant [14 x i8] c"\63\61\6C\6C\2D\76\31\20\66\72\61\6D\65\3D"
define internal void @cerune_call_write(ptr %p, i64 %n) {
entry:
  call i64 @write(i32 2, ptr %p, i64 %n)
  ret void
}
define internal void @cerune_call_flush() {
entry:
  %stream = load ptr, ptr @stdout
  call i32 @fflush(ptr %stream)
  ret void
}

@cerune_call_depth = internal global i64 0
@cerune_call_capacity = internal global i64 0
@cerune_call_frames = internal global ptr null
; History stores immutable call-site pointers, separate from language heap budgets.
define internal void @cerune_call_site_write(ptr %site, i64 %field) {
entry:
  %p = getelementptr {ptr,i64,ptr,i64}, ptr %site, i64 0, i32 0
  %pair = getelementptr {ptr,i64}, ptr %p, i64 %field
  %record = load {ptr,i64}, ptr %pair
  %data = extractvalue {ptr,i64} %record, 0
  %len = extractvalue {ptr,i64} %record, 1
  call void @cerune_call_write(ptr %data, i64 %len)
  ret void
}
define internal void @cerune_call_number(i64 %number) {
entry:
  %buffer = alloca [20 x i8]
  br label %digits
digits:
  %v = phi i64 [%number,%entry],[%q,%digits]
  %end = phi i64 [20,%entry],[%index,%digits]
  %q = udiv i64 %v, 10
  %r = urem i64 %v, 10
  %ascii = add i64 %r, 48
  %byte = trunc i64 %ascii to i8
  %index = sub i64 %end, 1
  %p = getelementptr i8, ptr %buffer, i64 %index
  store i8 %byte, ptr %p
  %more = icmp ne i64 %q, 0
  br i1 %more, label %digits, label %done
done:
  %length = sub i64 20, %index
  call void @cerune_call_write(ptr %p, i64 %length)
  ret void
}
define internal void @cerune_call_enter(ptr %site) {
entry:
  %depth = load i64, ptr @cerune_call_depth
  %full = icmp uge i64 %depth, 128
  br i1 %full, label %overflow, label %capacity
overflow:
  call void @cerune_call_flush()
  call void @cerune_call_write(ptr @cerune_call_header, i64 36)
  call void @cerune_call_site_write(ptr %site, i64 0)
  %history = load ptr, ptr @cerune_call_frames
  br label %history_loop
history_loop:
  %i = phi i64 [0,%overflow],[%next_i,%history_loop]
  call void @cerune_call_write(ptr @cerune_call_frame_prefix, i64 14)
  call void @cerune_call_number(i64 %i)
  %slot = getelementptr ptr, ptr %history, i64 %i
  %saved = load ptr, ptr %slot
  call void @cerune_call_site_write(ptr %saved, i64 0)
  %next_i = add i64 %i, 1
  %more = icmp ult i64 %next_i, %depth
  br i1 %more, label %history_loop, label %stop
stop:
  call void @_Exit(i32 1)
  unreachable
capacity:
  %cap = load i64, ptr @cerune_call_capacity
  %grow = icmp eq i64 %depth, %cap
  br i1 %grow, label %allocate, label %push
allocate:
  %zero = icmp eq i64 %cap, 0
  %double = mul i64 %cap, 2
  %want = select i1 %zero, i64 8, i64 %double
  %exceeds = icmp ugt i64 %want, 128
  %next_cap = select i1 %exceeds, i64 128, i64 %want
  %bytes = mul i64 %next_cap, 8
  %old = load ptr, ptr @cerune_call_frames
  %new = call ptr @realloc(ptr %old, i64 %bytes)
  %failed = icmp eq ptr %new, null
  br i1 %failed, label %allocation_failed, label %allocated
allocation_failed:
  call void @cerune_call_flush()
  call void @cerune_call_site_write(ptr %site, i64 1)
  call void @_Exit(i32 1)
  unreachable
allocated:
  store ptr %new, ptr @cerune_call_frames
  store i64 %next_cap, ptr @cerune_call_capacity
  br label %push
push:
  %frames = load ptr, ptr @cerune_call_frames
  %destination = getelementptr ptr, ptr %frames, i64 %depth
  store ptr %site, ptr %destination
  %entered = add i64 %depth, 1
  store i64 %entered, ptr @cerune_call_depth
  ret void
}
define internal void @cerune_call_leave() {
entry:
  %depth = load i64, ptr @cerune_call_depth
  %left = sub i64 %depth, 1
  store i64 %left, ptr @cerune_call_depth
  ret void
}
define internal void @cerune_call_finish() {
entry:
  %frames = load ptr, ptr @cerune_call_frames
  call void @free(ptr %frames)
  store ptr null, ptr @cerune_call_frames
  store i64 0, ptr @cerune_call_capacity
  ret void
}
