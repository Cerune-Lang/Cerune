#define CERUNE_PROCESS_TARGET @TARGET@

static cerune_process_context *cerune_process_context_value;
static void cerune_process_contract(uint32_t status, const char *origin) {
    fflush(stdout);
    fprintf(stderr, "cerune-process-abi-v1 status=%u%s\n", (unsigned)status, origin);
    exit(1);
}
static void cerune_process_check(uint32_t status, uint32_t expected, const char *origin) {
    if (status != expected) cerune_process_contract(status, origin);
}
/* Reservation capacity remains charged until the language array is released. */
#ifndef CERUNE_PROCESS_RESULT_MALLOC
#define CERUNE_PROCESS_RESULT_MALLOC malloc
#endif
static int cerune_process_reserve_one(uint64_t n, cerune_dynamic_array *out) {
    if (!n) return 0;
    cerune_array_owner *owner = CERUNE_PROCESS_RESULT_MALLOC(sizeof(*owner));
    if (!owner) return 2;
    owner->data = CERUNE_PROCESS_RESULT_MALLOC((size_t)n * sizeof(int64_t));
    if (!owner->data) { free(owner); return 2; }
    owner->references = 1; owner->bytes = n * 8; owner->initialized = 0;
    cerune_array_live_bytes += owner->bytes;
    *out = (cerune_dynamic_array){owner, (int64_t)n};
    return 0;
}
static void cerune_process_cancel_array(cerune_dynamic_array *out) {
    if (out->owner) {
        cerune_array_live_bytes -= out->owner->bytes;
        free(out->owner->data); free(out->owner);
    }
    *out = (cerune_dynamic_array){NULL,0};
}
static int cerune_process_reserve(uint64_t a, uint64_t b, cerune_dynamic_array *out, cerune_dynamic_array *err) {
    if (a > INT64_MAX/8 || b > INT64_MAX/8 || a > SIZE_MAX/sizeof(int64_t) ||
        b > SIZE_MAX/sizeof(int64_t) || a > PTRDIFF_MAX/sizeof(int64_t) ||
        b > PTRDIFF_MAX/sizeof(int64_t) || a > UINT64_MAX/8-b ||
        (a+b)*8 > UINT64_C(@LIMIT@)-cerune_array_live_bytes) return 1;
    int status = cerune_process_reserve_one(a,out);
    if (!status) status = cerune_process_reserve_one(b,err);
    if (status) { cerune_process_cancel_array(out); cerune_process_cancel_array(err); }
    return status;
}
static void cerune_process_finalize(cerune_dynamic_array *array, uint64_t n, uint64_t channel, const char *origin) {
    if (n > (uint64_t)array->length) cerune_process_contract(13,origin);
    unsigned char bytes[4096];
    for (uint64_t offset=0; offset<n;) {
        uint64_t wanted=n-offset < sizeof(bytes) ? n-offset : sizeof(bytes), copied=0;
        cerune_process_check(cerune_process_v1_copy(cerune_process_context_value,channel,offset,bytes,wanted,&copied),0,origin);
        if (copied!=wanted) cerune_process_contract(13,origin);
        for (uint64_t i=0; i<copied; ++i) ((int64_t *)array->owner->data)[offset+i]=bytes[i];
        offset+=copied;
    }
    array->length=(int64_t)n;
    if (array->owner) array->owner->initialized=(int64_t)n;
}
