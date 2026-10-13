target triple = "x86_64-unknown-linux-gnu"

@cerune.failure.52.70.84.11 = private unnamed_addr constant [71 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\72\72\61\79\2D\69\6E\64\65\78\2D\6F\75\74\2D\6F\66\2D\62\6F\75\6E\64\73\20\6E\6F\64\65\3D\35\32\20\62\79\74\65\73\3D\37\30\2E\2E\38\34\0A"
@cerune.failure.52.70.84 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } { ptr @cerune.failure.52.70.84.11, i64 71 }]
@cerune.failure.55.70.84.11 = private unnamed_addr constant [71 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\72\72\61\79\2D\69\6E\64\65\78\2D\6F\75\74\2D\6F\66\2D\62\6F\75\6E\64\73\20\6E\6F\64\65\3D\35\35\20\62\79\74\65\73\3D\37\30\2E\2E\38\34\0A"
@cerune.failure.55.70.84 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } { ptr @cerune.failure.55.70.84.11, i64 71 }]
@cerune.failure.63.70.84.0 = private unnamed_addr constant [62 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\69\6E\74\65\67\65\72\2D\6F\76\65\72\66\6C\6F\77\20\6E\6F\64\65\3D\36\33\20\62\79\74\65\73\3D\37\30\2E\2E\38\34\0A"
@cerune.failure.63.70.84 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } { ptr @cerune.failure.63.70.84.0, i64 62 }, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer]
@cerune.failure.41.254.256.0 = private unnamed_addr constant [64 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\69\6E\74\65\67\65\72\2D\6F\76\65\72\66\6C\6F\77\20\6E\6F\64\65\3D\34\31\20\62\79\74\65\73\3D\32\35\34\2E\2E\32\35\36\0A"
@cerune.failure.41.254.256 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } { ptr @cerune.failure.41.254.256.0, i64 64 }, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer]
%cerune.string = type { ptr, i64 }
@cerune.string.0 = private unnamed_addr constant [5 x i8] c"\56\61\6C\75\65"
@cerune.string.1 = private unnamed_addr constant [1 x i8] c"\7B"
@cerune.string.2 = private unnamed_addr constant [3 x i8] c"\6E\3A\20"
@cerune.string.3 = private unnamed_addr constant [1 x i8] c"\7D"
@cerune.string.4 = private unnamed_addr constant [5 x i8] c"\45\6D\70\74\79"
@cerune.string.5 = private unnamed_addr constant [1 x i8] c"\7B"
@cerune.string.6 = private unnamed_addr constant [1 x i8] c"\7D"
@cerune.string.7 = private unnamed_addr constant [0 x i8] c""

%cerune.type.Item.0 = type { i64, i64 }

declare void @llvm.trap()
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)

declare i32 @fflush(ptr)
declare i64 @write(i32, ptr, i64)

define internal void @cerune.runtime.fail(ptr %failure, i64 %code) {
entry:
  call i32 @fflush(ptr null)
  %slot = getelementptr inbounds { ptr, i64 }, ptr %failure, i64 %code
  %record = load { ptr, i64 }, ptr %slot
  %data = extractvalue { ptr, i64 } %record, 0
  %length = extractvalue { ptr, i64 } %record, 1
  call i64 @write(i32 2, ptr %data, i64 %length)
  call void @llvm.trap()
  unreachable
}

define internal i64 @cerune_i64_add(i64 %left, i64 %right, ptr %failure) {
entry:
  %checked = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %left, i64 %right)
  %result = extractvalue { i64, i1 } %checked, 0
  %overflow = extractvalue { i64, i1 } %checked, 1
  br i1 %overflow, label %trap, label %ok

trap:
  call void @cerune.runtime.fail(ptr %failure, i64 0)
  unreachable
ok:
  ret i64 %result
}

define internal i64 @cerune_i64_sub(i64 %left, i64 %right, ptr %failure) {
entry:
  %checked = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %left, i64 %right)
  %result = extractvalue { i64, i1 } %checked, 0
  %overflow = extractvalue { i64, i1 } %checked, 1
  br i1 %overflow, label %trap, label %ok

trap:
  call void @cerune.runtime.fail(ptr %failure, i64 0)
  unreachable
ok:
  ret i64 %result
}

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
@cerune.output.origin.4.64.86 = private constant [22 x i8] c" node=4 bytes=64..86\0A\00"
@cerune.output.origin.43.261.275 = private constant [25 x i8] c" node=43 bytes=261..275\0A\00"
@cerune.output.origin.95.87.115 = private constant [24 x i8] c" node=95 bytes=87..115\0A\00"
@cerune.output.origin.97.87.115 = private constant [24 x i8] c" node=97 bytes=87..115\0A\00"
@cerune.output.origin.99.87.115 = private constant [24 x i8] c" node=99 bytes=87..115\0A\00"
@cerune.output.origin.102.87.115 = private constant [25 x i8] c" node=102 bytes=87..115\0A\00"
@cerune.output.origin.104.87.115 = private constant [25 x i8] c" node=104 bytes=87..115\0A\00"
@cerune.output.origin.111.87.115 = private constant [25 x i8] c" node=111 bytes=87..115\0A\00"
@cerune.output.origin.113.87.115 = private constant [25 x i8] c" node=113 bytes=87..115\0A\00"
@cerune.output.origin.115.87.115 = private constant [25 x i8] c" node=115 bytes=87..115\0A\00"
@cerune.output.origin.118.87.115 = private constant [25 x i8] c" node=118 bytes=87..115\0A\00"
@cerune.output.record = private constant [61 x i8] c"cerune: io-v1 operation=write requested=%llu accepted=%llu%s\00"
@cerune.output.format.error = private constant [35 x i8] c"cerune: output formatting failed%s\00"
@cerune.output.true = private constant [5 x i8] c"true\0A"
@cerune.output.false = private constant [6 x i8] c"false\0A"
@cerune.output.lf = private constant [1 x i8] c"\0A"
@cerune.output.quote = private constant [1 x i8] c"\22"
declare i32 @snprintf(ptr, i64, ptr, ...)
declare void @_Exit(i32) noreturn
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
define internal i64 @cerune.array.get.i64.2([2 x i64] %value, i64 %index, ptr %failure) {
entry:
  %index.low = icmp slt i64 %index, 0
  %index.high = icmp sge i64 %index, 2
  %index.outside = or i1 %index.low, %index.high
  br i1 %index.outside, label %out_of_bounds, label %in_bounds
out_of_bounds:
  call void @cerune.runtime.fail(ptr %failure, i64 11)
  unreachable
in_bounds:
  %array = alloca [2 x i64]
  store [2 x i64] %value, ptr %array
  %element = getelementptr inbounds [2 x i64], ptr %array, i64 0, i64 %index
  %result = load i64, ptr %element
  ret i64 %result
}

define i1 @cerune.fn._equal0.0([2 x i64] %arg0, [2 x i64] %arg1) {
entry:
  %cerune_$left = alloca [2 x i64]
  %cerune_$right = alloca [2 x i64]
  %cerune_$index = alloca i64
  store [2 x i64] %arg0, ptr %cerune_$left
  store [2 x i64] %arg1, ptr %cerune_$right
  store i64 0, ptr %cerune_$index
  br label %block0
block0: ; for_condition
  %tmp0 = load i64, ptr %cerune_$index
  %tmp1 = icmp slt i64 %tmp0, 2
  br i1 %tmp1, label %block1, label %block3
block1: ; for_body
  %tmp2 = load [2 x i64], ptr %cerune_$left
  %tmp3 = load i64, ptr %cerune_$index
  %tmp4 = call i64 @cerune.array.get.i64.2([2 x i64] %tmp2, i64 %tmp3, ptr @cerune.failure.52.70.84)
  %tmp5 = load [2 x i64], ptr %cerune_$right
  %tmp6 = load i64, ptr %cerune_$index
  %tmp7 = call i64 @cerune.array.get.i64.2([2 x i64] %tmp5, i64 %tmp6, ptr @cerune.failure.55.70.84)
  %tmp8 = icmp eq i64 %tmp4, %tmp7
  %tmp9 = xor i1 %tmp8, 1
  br i1 %tmp9, label %block4, label %block6
block4: ; if_then
  ret i1 0
block6: ; if_end
  br label %block2
block2: ; for_update
  %tmp10 = load i64, ptr %cerune_$index
  %tmp11 = call i64 @cerune_i64_add(i64 %tmp10, i64 1, ptr @cerune.failure.63.70.84)
  store i64 %tmp11, ptr %cerune_$index
  br label %block0
block3: ; for_end
  ret i1 1
}

define i1 @cerune.fn._match_bind1.1(%cerune.type.Item.0 %arg0) {
entry:
  %cerune_$capture1 = alloca %cerune.type.Item.0
  %cerune_n = alloca i64
  store %cerune.type.Item.0 %arg0, ptr %cerune_$capture1
  %tmp0 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp1 = extractvalue %cerune.type.Item.0 %tmp0, 1
  store i64 %tmp1, ptr %cerune_n
  %tmp2 = load i64, ptr %cerune_n
  %tmp3 = icmp sgt i64 %tmp2, 0
  ret i1 %tmp3
}

define i64 @cerune.fn._match_bind2.2(%cerune.type.Item.0 %arg0) {
entry:
  %cerune_$capture1 = alloca %cerune.type.Item.0
  %cerune_n = alloca i64
  store %cerune.type.Item.0 %arg0, ptr %cerune_$capture1
  %tmp0 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp1 = extractvalue %cerune.type.Item.0 %tmp0, 1
  store i64 %tmp1, ptr %cerune_n
  %tmp2 = load i64, ptr %cerune_n
  ret i64 %tmp2
}

define i64 @cerune.fn._match_select3.3(%cerune.type.Item.0 %arg0) {
entry:
  %cerune_$capture1 = alloca %cerune.type.Item.0
  store %cerune.type.Item.0 %arg0, ptr %cerune_$capture1
  %tmp0 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp1 = extractvalue %cerune.type.Item.0 %tmp0, 0
  %tmp2 = icmp eq i64 %tmp1, 0
  br i1 %tmp2, label %block0, label %block1
block0: ; if_then
  ret i64 0
block1: ; if_else
  %tmp3 = call i64 @cerune_i64_sub(i64 0, i64 1, ptr @cerune.failure.41.254.256)
  ret i64 %tmp3
}

define i64 @cerune.fn._match_select4.4(%cerune.type.Item.0 %arg0) {
entry:
  %cerune_$capture1 = alloca %cerune.type.Item.0
  %cerune_logical_result1 = alloca i1
  store %cerune.type.Item.0 %arg0, ptr %cerune_$capture1
  %tmp0 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp1 = extractvalue %cerune.type.Item.0 %tmp0, 0
  %tmp2 = icmp eq i64 %tmp1, 0
  store i1 %tmp2, ptr %cerune_logical_result1
  br i1 %tmp2, label %block0, label %block1
block0: ; logical_rhs
  %tmp3 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp4 = call i1 @cerune.fn._match_bind1.1(%cerune.type.Item.0 %tmp3)
  store i1 %tmp4, ptr %cerune_logical_result1
  br label %block1
block1: ; logical_end
  %tmp5 = load i1, ptr %cerune_logical_result1
  br i1 %tmp5, label %block2, label %block3
block2: ; if_then
  %tmp6 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp7 = call i64 @cerune.fn._match_bind2.2(%cerune.type.Item.0 %tmp6)
  ret i64 %tmp7
block3: ; if_else
  %tmp8 = load %cerune.type.Item.0, ptr %cerune_$capture1
  %tmp9 = call i64 @cerune.fn._match_select3.3(%cerune.type.Item.0 %tmp8)
  ret i64 %tmp9
}

define i64 @cerune.fn._match_bind5.5() {
entry:
  %cerune_$match0 = alloca %cerune.type.Item.0
  %tmp0 = insertvalue %cerune.type.Item.0 poison, i64 0, 0
  %tmp1 = insertvalue %cerune.type.Item.0 %tmp0, i64 7, 1
  store %cerune.type.Item.0 %tmp1, ptr %cerune_$match0
  %tmp2 = load %cerune.type.Item.0, ptr %cerune_$match0
  %tmp3 = call i64 @cerune.fn._match_select4.4(%cerune.type.Item.0 %tmp2)
  ret i64 %tmp3
}

define void @cerune.fn._display6.6(%cerune.type.Item.0 %arg0) {
entry:
  %cerune_$value = alloca %cerune.type.Item.0
  store %cerune.type.Item.0 %arg0, ptr %cerune_$value
  %tmp0 = load %cerune.type.Item.0, ptr %cerune_$value
  %tmp1 = extractvalue %cerune.type.Item.0 %tmp0, 0
  %tmp2 = icmp eq i64 %tmp1, 0
  br i1 %tmp2, label %block0, label %block2
block0: ; if_then
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.0, i64 5 }, i1 false, ptr @cerune.output.origin.95.87.115)
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.1, i64 1 }, i1 false, ptr @cerune.output.origin.97.87.115)
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.2, i64 3 }, i1 false, ptr @cerune.output.origin.99.87.115)
  %tmp3 = load %cerune.type.Item.0, ptr %cerune_$value
  %tmp4 = extractvalue %cerune.type.Item.0 %tmp3, 1
  call void @cerune.output.i64(i64 %tmp4, i1 false, ptr @cerune.output.origin.102.87.115)
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.3, i64 1 }, i1 false, ptr @cerune.output.origin.104.87.115)
  br label %block2
block2: ; if_end
  %tmp5 = load %cerune.type.Item.0, ptr %cerune_$value
  %tmp6 = extractvalue %cerune.type.Item.0 %tmp5, 0
  %tmp7 = icmp eq i64 %tmp6, 1
  br i1 %tmp7, label %block3, label %block5
block3: ; if_then
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.4, i64 5 }, i1 false, ptr @cerune.output.origin.111.87.115)
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.5, i64 1 }, i1 false, ptr @cerune.output.origin.113.87.115)
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.6, i64 1 }, i1 false, ptr @cerune.output.origin.115.87.115)
  br label %block5
block5: ; if_end
  call void @cerune.output.string(%cerune.string { ptr @cerune.string.7, i64 0 }, i1 true, ptr @cerune.output.origin.118.87.115)
  ret void
}

define i32 @main() {
entry:
  %cerune_left = alloca [2 x i64]
  %cerune_result = alloca i64
  %tmp0 = insertvalue [2 x i64] poison, i64 1, 0
  %tmp1 = insertvalue [2 x i64] %tmp0, i64 2, 1
  store [2 x i64] %tmp1, ptr %cerune_left
  %tmp2 = load [2 x i64], ptr %cerune_left
  %tmp3 = insertvalue [2 x i64] poison, i64 1, 0
  %tmp4 = insertvalue [2 x i64] %tmp3, i64 3, 1
  %tmp5 = call i1 @cerune.fn._equal0.0([2 x i64] %tmp2, [2 x i64] %tmp4)
  call void @cerune.output.bool(i1 %tmp5, i1 true, ptr @cerune.output.origin.4.64.86)
  %tmp6 = insertvalue %cerune.type.Item.0 poison, i64 0, 0
  %tmp7 = insertvalue %cerune.type.Item.0 %tmp6, i64 7, 1
  call void @cerune.fn._display6.6(%cerune.type.Item.0 %tmp7)
  %tmp8 = call i64 @cerune.fn._match_bind5.5()
  store i64 %tmp8, ptr %cerune_result
  %tmp9 = load i64, ptr %cerune_result
  call void @cerune.output.i64(i64 %tmp9, i1 true, ptr @cerune.output.origin.43.261.275)
  ret i32 0
}
