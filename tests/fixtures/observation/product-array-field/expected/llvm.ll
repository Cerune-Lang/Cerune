target triple = "x86_64-unknown-linux-gnu"

@cerune.failure.15.145.161.11 = private unnamed_addr constant [73 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\72\72\61\79\2D\69\6E\64\65\78\2D\6F\75\74\2D\6F\66\2D\62\6F\75\6E\64\73\20\6E\6F\64\65\3D\31\35\20\62\79\74\65\73\3D\31\34\35\2E\2E\31\36\31\0A"
@cerune.failure.15.145.161 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } { ptr @cerune.failure.15.145.161.11, i64 73 }]
@cerune.failure.20.170.185.11 = private unnamed_addr constant [73 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\72\72\61\79\2D\69\6E\64\65\78\2D\6F\75\74\2D\6F\66\2D\62\6F\75\6E\64\73\20\6E\6F\64\65\3D\32\30\20\62\79\74\65\73\3D\31\37\30\2E\2E\31\38\35\0A"
@cerune.failure.20.170.185 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } { ptr @cerune.failure.20.170.185.11, i64 73 }]
%cerune.type.Row.0 = type { [3 x i64] }

declare void @llvm.trap()

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

@cerune.output.origin.14.139.163 = private constant [25 x i8] c" node=14 bytes=139..163\0A\00"
@cerune.output.origin.19.164.187 = private constant [25 x i8] c" node=19 bytes=164..187\0A\00"
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
define internal i64 @cerune.array.get.i64.3([3 x i64] %value, i64 %index, ptr %failure) {
entry:
  %index.low = icmp slt i64 %index, 0
  %index.high = icmp sge i64 %index, 3
  %index.outside = or i1 %index.low, %index.high
  br i1 %index.outside, label %out_of_bounds, label %in_bounds
out_of_bounds:
  call void @cerune.runtime.fail(ptr %failure, i64 11)
  unreachable
in_bounds:
  %array = alloca [3 x i64]
  store [3 x i64] %value, ptr %array
  %element = getelementptr inbounds [3 x i64], ptr %array, i64 0, i64 %index
  %result = load i64, ptr %element
  ret i64 %result
}

define i32 @main() {
entry:
  %cerune_first = alloca %cerune.type.Row.0
  %cerune_second = alloca %cerune.type.Row.0
  %tmp0 = insertvalue [3 x i64] poison, i64 1, 0
  %tmp1 = insertvalue [3 x i64] %tmp0, i64 2, 1
  %tmp2 = insertvalue [3 x i64] %tmp1, i64 3, 2
  %tmp3 = insertvalue %cerune.type.Row.0 poison, [3 x i64] %tmp2, 0
  store %cerune.type.Row.0 %tmp3, ptr %cerune_first
  %tmp4 = load %cerune.type.Row.0, ptr %cerune_first
  store %cerune.type.Row.0 %tmp4, ptr %cerune_second
  %tmp5 = insertvalue [3 x i64] poison, i64 4, 0
  %tmp6 = insertvalue [3 x i64] %tmp5, i64 5, 1
  %tmp7 = insertvalue [3 x i64] %tmp6, i64 6, 2
  %tmp8 = insertvalue %cerune.type.Row.0 poison, [3 x i64] %tmp7, 0
  store %cerune.type.Row.0 %tmp8, ptr %cerune_first
  %tmp9 = load %cerune.type.Row.0, ptr %cerune_second
  %tmp10 = extractvalue %cerune.type.Row.0 %tmp9, 0
  %tmp11 = call i64 @cerune.array.get.i64.3([3 x i64] %tmp10, i64 1, ptr @cerune.failure.15.145.161)
  call void @cerune.output.i64(i64 %tmp11, i1 true, ptr @cerune.output.origin.14.139.163)
  %tmp12 = load %cerune.type.Row.0, ptr %cerune_first
  %tmp13 = extractvalue %cerune.type.Row.0 %tmp12, 0
  %tmp14 = call i64 @cerune.array.get.i64.3([3 x i64] %tmp13, i64 2, ptr @cerune.failure.20.170.185)
  call void @cerune.output.i64(i64 %tmp14, i1 true, ptr @cerune.output.origin.19.164.187)
  ret i32 0
}
