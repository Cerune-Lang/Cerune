pub(super) fn support(limit: u64) -> String {
    SOURCE.replace("@LIMIT@", &limit.to_string())
}
const SOURCE: &str = r#"
/* Explicit compile-time limits, independent of the returned-string heap. */
#ifndef CERUNE_ARGUMENT_COUNT_LIMIT
#define CERUNE_ARGUMENT_COUNT_LIMIT UINT64_C(4096)
#endif
#ifndef CERUNE_ARGUMENT_BYTE_LIMIT
#define CERUNE_ARGUMENT_BYTE_LIMIT UINT64_C(1048576)
#endif
static cerune_string *cerune_arguments_values;
static size_t cerune_arguments_length;
static int cerune_arguments_configured;

static void cerune_arguments_capture_failed(const char *code, int64_t index) {
    fprintf(stderr, "cerune: arguments-v1 phase=capture code=%s", code);
    if (index >= 0) fprintf(stderr, " index=%lld", (long long)index);
    fputc('\n', stderr);
    fflush(stderr);
    _Exit(1);
}
static void cerune_arguments_failed(const char *code, int get, int64_t index, const char *origin) {
    fflush(stdout);
    fprintf(stderr, "cerune: arguments-v1 phase=execute operation=%s code=%s", get ? "get" : "count", code);
    if (get) fprintf(stderr, " index=%lld", (long long)index);
    fputs(origin, stderr);
    fflush(stderr);
    _Exit(1);
}
static void cerune_arguments_dispose(void) {
    for (size_t i = 0; i < cerune_arguments_length; ++i)
        if (cerune_arguments_values[i].length) free((void *)cerune_arguments_values[i].data);
    free(cerune_arguments_values);
    cerune_arguments_values = NULL;
    cerune_arguments_length = 0;
    cerune_arguments_configured = 0;
}
/* All input is validated before copying; no partial snapshot is published. */
static void cerune_arguments_capture(size_t count, const cerune_string *values,
                                     uint64_t max_count, uint64_t max_bytes) {
    if (max_count > INT64_MAX) cerune_arguments_capture_failed("invalid-limit", -1);
    if ((uint64_t)count > max_count) cerune_arguments_capture_failed("count-limit-exceeded", -1);
    uint64_t bytes = 0;
    for (size_t i = 0; i < count; ++i) {
        size_t length = values[i].length;
        if ((uint64_t)length > max_bytes - bytes)
            cerune_arguments_capture_failed("byte-limit-exceeded", (int64_t)i);
        bytes += length;
        if (!cerune_utf8(values[i].data, length))
            cerune_arguments_capture_failed("invalid-utf8", (int64_t)i);
        if (length && memchr(values[i].data, 0, length))
            cerune_arguments_capture_failed("embedded-nul", (int64_t)i);
    }
    cerune_string *copy = NULL;
    if (count) {
        if (count > SIZE_MAX / sizeof(cerune_string))
            cerune_arguments_capture_failed("allocation-failed", -1);
        copy = malloc(count * sizeof(cerune_string));
        if (!copy) cerune_arguments_capture_failed("allocation-failed", -1);
    }
    for (size_t i = 0; i < count; ++i) {
        size_t length = values[i].length;
        unsigned char *data = NULL;
        if (length) {
            data = malloc(length);
            if (!data) {
                for (size_t j = 0; j < i; ++j)
                    if (copy[j].length) free((void *)copy[j].data);
                free(copy);
                cerune_arguments_capture_failed("allocation-failed", (int64_t)i);
            }
            memcpy(data, values[i].data, length);
        }
        copy[i] = (cerune_string){length ? data : (const unsigned char *)"", length};
    }
    cerune_arguments_dispose();
    cerune_arguments_values = copy;
    cerune_arguments_length = count;
    cerune_arguments_configured = 1;
}
static int64_t cerune_arguments_count(const char *origin) {
    if (!cerune_arguments_configured) cerune_arguments_failed("unconfigured", 0, 0, origin);
    return (int64_t)cerune_arguments_length;
}
static cerune_string cerune_arguments_get(int64_t index, const char *origin) {
    if (!cerune_arguments_configured) cerune_arguments_failed("unconfigured", 1, index, origin);
    if (index < 0) cerune_arguments_failed("invalid-index", 1, index, origin);
    if ((uint64_t)index >= cerune_arguments_length)
        cerune_arguments_failed("index-out-of-bounds", 1, index, origin);
    cerune_string value = cerune_arguments_values[index];
    if (!value.length) return value;
    if ((uint64_t)value.length > UINT64_C(@LIMIT@) - cerune_string_live_bytes)
        cerune_arguments_failed("budget-exceeded", 1, index, origin);
    if (value.length > SIZE_MAX - sizeof(cerune_string_owner))
        cerune_arguments_failed("allocation-failed", 1, index, origin);
    cerune_string_owner *p = malloc(sizeof(cerune_string_owner) + value.length);
    if (!p) cerune_arguments_failed("allocation-failed", 1, index, origin);
    p->next = cerune_string_owners;
    p->references = 1;
    p->length = value.length;
    memcpy(p->data, value.data, value.length);
    cerune_string_owners = p;
    cerune_string_live_bytes += value.length;
    return (cerune_string){p->data, value.length};
}
"#;

pub(super) const ENTRY: &str = r#"
/* Selected by the C compilation target, never by the Cerune emitter's OS. */
#ifdef _WIN32
#include <windows.h>
int wmain(int argc, wchar_t **argv) {
#else
int main(int argc, char **argv) {
#endif
    size_t count = argc > 0 ? (size_t)(argc - 1) : 0;
    cerune_string *values = NULL;
    if (count) {
        if (count > SIZE_MAX / sizeof(cerune_string))
            cerune_arguments_capture_failed("allocation-failed", -1);
        values = malloc(count * sizeof(cerune_string));
        if (!values) cerune_arguments_capture_failed("allocation-failed", -1);
    }
    for (size_t i = 0; i < count; ++i) {
#ifdef _WIN32
        /* Strict UTF-16 -> UTF-8, without locale conversion or quote reparsing. */
        int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[i + 1], -1, NULL, 0, NULL, NULL);
        if (!size) cerune_arguments_capture_failed("invalid-unicode", (int64_t)i);
        unsigned char *data = malloc((size_t)size);
        if (!data) cerune_arguments_capture_failed("allocation-failed", (int64_t)i);
        if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[i + 1], -1, (char *)data, size, NULL, NULL) != size)
            cerune_arguments_capture_failed("invalid-unicode", (int64_t)i);
        values[i] = (cerune_string){data, (size_t)size - 1};
#else
        size_t length = strlen(argv[i + 1]);
        if (!cerune_utf8((const unsigned char *)argv[i + 1], length))
            cerune_arguments_capture_failed("invalid-unicode", (int64_t)i);
        values[i] = (cerune_string){(const unsigned char *)argv[i + 1], length};
#endif
    }
    cerune_arguments_capture(count, values, (uint64_t)CERUNE_ARGUMENT_COUNT_LIMIT, (uint64_t)CERUNE_ARGUMENT_BYTE_LIMIT);
#ifdef _WIN32
    for (size_t i = 0; i < count; ++i) free((void *)values[i].data);
#endif
    free(values);
    int result = cerune_program();
    cerune_arguments_dispose();
    return result;
}
"#;
