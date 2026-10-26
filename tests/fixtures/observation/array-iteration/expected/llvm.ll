target triple = "x86_64-unknown-linux-gnu"

@cerune.failure.20.69.85.11 = private unnamed_addr constant [71 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\72\72\61\79\2D\69\6E\64\65\78\2D\6F\75\74\2D\6F\66\2D\62\6F\75\6E\64\73\20\6E\6F\64\65\3D\32\30\20\62\79\74\65\73\3D\36\39\2E\2E\38\35\0A"
@cerune.failure.20.69.85 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } { ptr @cerune.failure.20.69.85.11, i64 71 }]
@cerune.failure.29.141.151.0 = private unnamed_addr constant [64 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\69\6E\74\65\67\65\72\2D\6F\76\65\72\66\6C\6F\77\20\6E\6F\64\65\3D\32\39\20\62\79\74\65\73\3D\31\34\31\2E\2E\31\35\31\0A"
@cerune.failure.29.141.151 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } { ptr @cerune.failure.29.141.151.0, i64 64 }, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer]
@cerune.failure.35.54.98.0 = private unnamed_addr constant [62 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\69\6E\74\65\67\65\72\2D\6F\76\65\72\66\6C\6F\77\20\6E\6F\64\65\3D\33\35\20\62\79\74\65\73\3D\35\34\2E\2E\39\38\0A"
@cerune.failure.35.54.98 = private constant [12 x { ptr, i64 }] [{ ptr, i64 } { ptr @cerune.failure.35.54.98.0, i64 62 }, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer, { ptr, i64 } zeroinitializer]

declare void @llvm.trap()
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)

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

@cerune.output.origin.32.157.170 = private constant [25 x i8] c" node=32 bytes=157..170\0A\00"
@cerune.output.origin.0.26.36 = private constant [22 x i8] c" node=0 bytes=26..36\0A\00"
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

define [2 x i64] @cerune.fn.values.0() {
entry:
  call void @cerune.output.i64(i64 42, i1 true, ptr @cerune.output.origin.0.26.36)
  %tmp0 = insertvalue [2 x i64] poison, i64 1, 0
  %tmp1 = insertvalue [2 x i64] %tmp0, i64 2, 1
  ret [2 x i64] %tmp1
}

define i32 @main() {
entry:
  %cerune_$for_in_snapshot_0_54 = alloca [2 x i64]
  %cerune_$for_in_length_0_54 = alloca i64
  %cerune_$for_in_cursor_0_54 = alloca i64
  %cerune_i = alloca i64
  %cerune_value = alloca i64
  call void @cerune_call_enter(ptr @cerune_call_site_0_7_89_97)
  %tmp0 = call [2 x i64] @cerune.fn.values.0()
  call void @cerune_call_leave()
  store [2 x i64] %tmp0, ptr %cerune_$for_in_snapshot_0_54
  %tmp1 = load [2 x i64], ptr %cerune_$for_in_snapshot_0_54
  store i64 2, ptr %cerune_$for_in_length_0_54
  store i64 0, ptr %cerune_$for_in_cursor_0_54
  br label %block0
block0: ; for_condition
  %tmp2 = load i64, ptr %cerune_$for_in_cursor_0_54
  %tmp3 = load i64, ptr %cerune_$for_in_length_0_54
  %tmp4 = icmp slt i64 %tmp2, %tmp3
  br i1 %tmp4, label %block1, label %block3
block1: ; for_body
  %tmp5 = load i64, ptr %cerune_$for_in_cursor_0_54
  store i64 %tmp5, ptr %cerune_i
  %tmp6 = load [2 x i64], ptr %cerune_$for_in_snapshot_0_54
  %tmp7 = load i64, ptr %cerune_$for_in_cursor_0_54
  %tmp8 = call i64 @cerune.array.get.i64.2([2 x i64] %tmp6, i64 %tmp7, ptr @cerune.failure.20.69.85)
  store i64 %tmp8, ptr %cerune_value
  %tmp9 = load i64, ptr %cerune_i
  %tmp10 = icmp eq i64 %tmp9, 0
  br i1 %tmp10, label %block4, label %block6
block4: ; if_then
  br label %block2
block6: ; if_end
  %tmp11 = load i64, ptr %cerune_value
  %tmp12 = call i64 @cerune_i64_add(i64 %tmp11, i64 10, ptr @cerune.failure.29.141.151)
  store i64 %tmp12, ptr %cerune_value
  %tmp13 = load i64, ptr %cerune_value
  call void @cerune.output.i64(i64 %tmp13, i1 true, ptr @cerune.output.origin.32.157.170)
  br label %block2
block2: ; for_update
  %tmp14 = load i64, ptr %cerune_$for_in_cursor_0_54
  %tmp15 = call i64 @cerune_i64_add(i64 %tmp14, i64 1, ptr @cerune.failure.35.54.98)
  store i64 %tmp15, ptr %cerune_$for_in_cursor_0_54
  br label %block0
block3: ; for_end
  call void @cerune_call_finish()
  ret i32 0
}
; call_depth_limit=128; not a physical stack/memory guarantee
declare ptr @realloc(ptr, i64)
declare void @free(ptr)
@stdout = external global ptr
@cerune_call_site_0_7_89_97_text = private constant [32 x i8] c"\20\66\75\6E\63\74\69\6F\6E\3D\30\20\6E\6F\64\65\3D\37\20\62\79\74\65\73\3D\38\39\2E\2E\39\37\0A"
@cerune_call_site_0_7_89_97_alloc = private constant [62 x i8] c"\63\65\72\75\6E\65\3A\20\72\75\6E\74\69\6D\65\2D\76\31\20\63\6F\64\65\3D\61\6C\6C\6F\63\61\74\69\6F\6E\2D\66\61\69\6C\65\64\20\6E\6F\64\65\3D\37\20\62\79\74\65\73\3D\38\39\2E\2E\39\37\0A"
@cerune_call_site_0_7_89_97 = private constant {ptr,i64,ptr,i64} {ptr @cerune_call_site_0_7_89_97_text,i64 32,ptr @cerune_call_site_0_7_89_97_alloc,i64 62}
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
