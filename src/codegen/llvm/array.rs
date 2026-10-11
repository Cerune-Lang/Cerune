//! LLVMの領域管理プリミティブ。深いコピー・逆順の要素解放は共通IRが担当します。
pub(super) fn support(limit: u64) -> String {
    SOURCE.replace("@LIMIT@", &limit.to_string())
}
const SOURCE: &str = r#"
@cerune.array.live = internal global i64 0
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64)
define internal %cerune.array @cerune.array.allocate.elements(i64 %length, i64 %width, i64 %stride, ptr %failure) {
entry:
  %negative = icmp slt i64 %length, 0
  br i1 %negative, label %length.fail, label %sizes
sizes:
  %logical = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %length, i64 %width)
  %bytes = extractvalue { i64, i1 } %logical, 0
  %logical.overflow = extractvalue { i64, i1 } %logical, 1
  %logical.large = icmp ugt i64 %bytes, 9223372036854775807
  %physical = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %length, i64 %stride)
  %physical.bytes = extractvalue { i64, i1 } %physical, 0
  %physical.overflow = extractvalue { i64, i1 } %physical, 1
  %physical.large = icmp ugt i64 %physical.bytes, 9223372036854775807
  %bad.logical = or i1 %logical.overflow, %logical.large
  %bad.physical = or i1 %physical.overflow, %physical.large
  %bad.sizes = or i1 %bad.logical, %bad.physical
  br i1 %bad.sizes, label %size.fail, label %budget
budget:
  %live = load i64, ptr @cerune.array.live
  %available = sub i64 @LIMIT@, %live
  %over = icmp ugt i64 %bytes, %available
  br i1 %over, label %limit.fail, label %empty.check
empty.check:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %zero, label %allocate.owner
zero:
  ret %cerune.array zeroinitializer
allocate.owner:
  %owner = call ptr @malloc(i64 ptrtoint (ptr getelementptr (%cerune.array.owner, ptr null, i64 1) to i64))
  %owner.failed = icmp eq ptr %owner, null
  br i1 %owner.failed, label %allocation.fail, label %allocate.data
allocate.data:
  %zero.size = icmp eq i64 %physical.bytes, 0
  %size = select i1 %zero.size, i64 1, i64 %physical.bytes
  %data = call ptr @malloc(i64 %size)
  %data.failed = icmp eq ptr %data, null
  br i1 %data.failed, label %data.fail, label %initialize
data.fail:
  call void @free(ptr %owner)
  br label %allocation.fail
initialize:
  store ptr %data, ptr %owner
  %refs.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 1
  store i64 1, ptr %refs.ptr
  %bytes.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 2
  store i64 %bytes, ptr %bytes.ptr
  %count.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  store i64 0, ptr %count.ptr
  %total = add i64 %live, %bytes
  store i64 %total, ptr @cerune.array.live
  %result.owner = insertvalue %cerune.array poison, ptr %owner, 0
  %result = insertvalue %cerune.array %result.owner, i64 %length, 1
  ret %cerune.array %result
length.fail:
  call void @cerune.runtime.fail(ptr %failure, i64 16)
  unreachable
size.fail:
  call void @cerune.runtime.fail(ptr %failure, i64 12)
  unreachable
limit.fail:
  call void @cerune.runtime.fail(ptr %failure, i64 13)
  unreachable
allocation.fail:
  call void @cerune.runtime.fail(ptr %failure, i64 14)
  unreachable
}
define internal void @cerune.array.retain.owner(%cerune.array %value) {
entry:
  %owner = extractvalue %cerune.array %value, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %done, label %check
check:
  %refs.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 1
  %refs = load i64, ptr %refs.ptr
  %zero = icmp eq i64 %refs, 0
  %max = icmp eq i64 %refs, -1
  %bad = or i1 %zero, %max
  br i1 %bad, label %invalid, label %retain
retain:
  %more = add i64 %refs, 1
  store i64 %more, ptr %refs.ptr
  br label %done
done:
  ret void
invalid:
  call void @llvm.trap()
  unreachable
}
define internal i1 @cerune.array.release.owner.last(%cerune.array %value) {
entry:
  %owner = extractvalue %cerune.array %value, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %zero, label %check
zero:
  ret i1 true
check:
  %refs.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 1
  %refs = load i64, ptr %refs.ptr
  %no.owner = icmp eq i64 %refs, 0
  %count.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  %count = load i64, ptr %count.ptr
  %length = extractvalue %cerune.array %value, 1
  %partial = icmp ne i64 %count, %length
  %bad = or i1 %no.owner, %partial
  br i1 %bad, label %invalid, label %release
release:
  %less = sub i64 %refs, 1
  store i64 %less, ptr %refs.ptr
  %last = icmp eq i64 %less, 0
  ret i1 %last
invalid:
  call void @llvm.trap()
  unreachable
}
define internal void @cerune.array.free.elements(%cerune.array %value) {
entry:
  %owner = extractvalue %cerune.array %value, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %done, label %check
check:
  %refs.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 1
  %refs = load i64, ptr %refs.ptr
  %owned = icmp ne i64 %refs, 0
  br i1 %owned, label %invalid, label %destroy
destroy:
  %data = load ptr, ptr %owner
  %bytes.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 2
  %bytes = load i64, ptr %bytes.ptr
  %live = load i64, ptr @cerune.array.live
  %remaining = sub i64 %live, %bytes
  store i64 %remaining, ptr @cerune.array.live
  call void @free(ptr %data)
  call void @free(ptr %owner)
  br label %done
done:
  ret void
invalid:
  call void @llvm.trap()
  unreachable
}
define internal void @cerune.array.check.range(i64 %length, i64 %start, i64 %end, ptr %failure) {
entry:
  %low = icmp slt i64 %start, 0
  %reversed = icmp sgt i64 %start, %end
  %high = icmp sgt i64 %end, %length
  %bad.start = or i1 %low, %reversed
  %bad = or i1 %bad.start, %high
  br i1 %bad, label %fail, label %done
fail:
  call void @cerune.runtime.fail(ptr %failure, i64 15)
  unreachable
done:
  ret void
}
define internal ptr @cerune.array.checked.data(%cerune.array %value, i64 %index, ptr %failure) {
entry:
  %length = extractvalue %cerune.array %value, 1
  %low = icmp slt i64 %index, 0
  %high = icmp sge i64 %index, %length
  %bad = or i1 %low, %high
  br i1 %bad, label %fail, label %check.owner
fail:
  call void @cerune.runtime.fail(ptr %failure, i64 11)
  unreachable
check.owner:
  %owner = extractvalue %cerune.array %value, 0
  %empty = icmp eq ptr %owner, null
  br i1 %empty, label %invalid, label %check.initialized
check.initialized:
  %count.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  %count = load i64, ptr %count.ptr
  %uninitialized = icmp sge i64 %index, %count
  br i1 %uninitialized, label %invalid, label %done
done:
  %data = load ptr, ptr %owner
  ret ptr %data
invalid:
  call void @llvm.trap()
  unreachable
}
"#;
