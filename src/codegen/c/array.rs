//! 要素コピーと逆順の解放は共通IRが行い、ここでは領域と所有数だけを管理します。
pub(super) fn support(limit: u64) -> String {
    SOURCE.replace("@LIMIT@", &limit.to_string())
}
// 配列のメモリ上限とCの物理配置は別々に検査し、切り詰めてから乗算しません。
const SOURCE: &str = r#"
typedef struct cerune_array_owner {
    void *data;
    uint64_t references;
    uint64_t bytes;
    int64_t initialized;
} cerune_array_owner;
typedef struct cerune_dynamic_array {
    cerune_array_owner *owner;
    int64_t length;
} cerune_dynamic_array;

static uint64_t cerune_array_live_bytes;
static uint64_t cerune_array_available_bytes(void) {
    return UINT64_C(@LIMIT@) - cerune_array_live_bytes;
}
static const char *cerune_array_try_allocate(
    int64_t length, uint64_t width, size_t element_size, cerune_dynamic_array *out) {
    *out = (cerune_dynamic_array){NULL, 0};
    if (length < 0) return "array-length-out-of-range";
    if ((width && (uint64_t)length > (uint64_t)INT64_MAX / width) ||
        (element_size && (uint64_t)length > (uint64_t)SIZE_MAX / element_size) ||
        (element_size && (uint64_t)length > (uint64_t)PTRDIFF_MAX / element_size))
        return "allocation-size-overflow";
    uint64_t bytes = (uint64_t)length * width;
    if (bytes > cerune_array_available_bytes())
        return "allocation-limit-exceeded";
    if (!length) return NULL;
    cerune_array_owner *owner = malloc(sizeof(cerune_array_owner));
    if (!owner) return "allocation-failed";
    size_t physical_bytes = (size_t)length * element_size;
    owner->data = malloc(physical_bytes ? physical_bytes : 1);
    if (!owner->data) {
        free(owner);
        return "allocation-failed";
    }
    owner->references = 1;
    owner->bytes = bytes;
    owner->initialized = 0;
    cerune_array_live_bytes += bytes;
    *out = (cerune_dynamic_array){owner, length};
    return NULL;
}
static cerune_dynamic_array cerune_array_allocate_elements(
    int64_t length, uint64_t width, size_t element_size, const char *origin) {
    cerune_dynamic_array array;
    const char *code = cerune_array_try_allocate(length, width, element_size, &array);
    if (code) cerune_runtime_fail(code, origin);
    return array;
}
static void cerune_array_retain_owner(cerune_dynamic_array value) {
    if (!value.owner) return;
    assert(value.owner->references && value.owner->references < UINT64_MAX);
    ++value.owner->references;
}
static bool cerune_array_release_owner_last(cerune_dynamic_array value) {
    if (!value.owner) return true;
    assert(value.owner->references && value.owner->initialized == value.length);
    return --value.owner->references == 0;
}
static void cerune_array_free_elements(cerune_dynamic_array value) {
    if (!value.owner) return;
    assert(!value.owner->references && value.owner->bytes <= cerune_array_live_bytes);
    cerune_array_live_bytes -= value.owner->bytes;
    free(value.owner->data);
    free(value.owner);
}
static void cerune_array_check_range(int64_t length, int64_t start, int64_t end, const char *origin) {
    if (start < 0 || start > end || end > length)
        cerune_runtime_fail("array-range-out-of-bounds", origin);
}
"#;
