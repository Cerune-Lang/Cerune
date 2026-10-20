//! Cの行入力を明示的な接続・UTF-8検査・文字列所有へ展開します。
pub(super) fn support(limit: u64) -> String {
    SOURCE.replace("@LIMIT@", &limit.to_string()) + CONNECTION
}
const SOURCE: &str = r#"
#include <errno.h>

/* The target connection is supplied below; it never reads ahead. */
static const char *cerune_input_connection(void);
static int64_t cerune_input_read_byte(unsigned char *byte);
static int cerune_input_eof;

static void cerune_input_failed(const char *code, int64_t limit, size_t consumed,
                               int known, const char *origin) {
    fprintf(stderr, "cerune: io-v1 operation=read-line code=%s limit=%lld consumed=%llu consumption=%s%s",
        code, (long long)limit, (unsigned long long)consumed, known ? "known" : "unknown", origin);
    fflush(stderr);
    _Exit(1);
}
static cerune_string cerune_input_read_line(int64_t limit, const char *origin) {
    if (limit < 1 || limit > INT32_MAX)
        cerune_input_failed("invalid-limit", limit, 0, 1, origin);
    const char *code = cerune_input_connection();
    if (code) cerune_input_failed(code, limit, 0, 1, origin);
    if (cerune_input_eof) return (cerune_string){(const unsigned char *)"", 0};
    if ((uint64_t)limit > UINT64_C(@LIMIT@) - cerune_string_live_bytes)
        cerune_input_failed("budget-exceeded", limit, 0, 1, origin);
    if ((uint64_t)limit > SIZE_MAX - sizeof(cerune_string_owner))
        cerune_input_failed("allocation-failed", limit, 0, 1, origin);
    cerune_string_owner *p = malloc(sizeof(cerune_string_owner) + (size_t)limit);
    if (!p) cerune_input_failed("allocation-failed", limit, 0, 1, origin);
    cerune_string_live_bytes += (uint64_t)limit;
    size_t length = 0, consumed = 0;
    int known = 1;
    for (;;) {
        unsigned char byte;
        int64_t count = cerune_input_read_byte(&byte);
        if (count < 0) {
            if (errno == EINTR) continue;
            code = "read-failed"; break;
        }
        if (count > 1) { known = 0; code = "invalid-response"; break; }
        if (count == 0) { cerune_input_eof = 1; break; }
        ++consumed;
        if (length == (size_t)limit) { code = "limit-exceeded"; break; }
        p->data[length++] = byte;
        if (byte == 10) break;
    }
    if (!code && !cerune_utf8(p->data, length)) code = "invalid-utf8";
    if (code || length == 0) {
        cerune_string_live_bytes -= (uint64_t)limit;
        free(p);
        if (code) cerune_input_failed(code, limit, consumed, known, origin);
        return (cerune_string){(const unsigned char *)"", 0};
    }
    /* Compact only before publishing the owner; failure keeps the input. */
    if (length <= (size_t)limit / 2 && (size_t)limit - length >= 4096) {
        void *cerune_compacted = realloc(p, sizeof(cerune_string_owner) + length);
        if (cerune_compacted) p = cerune_compacted;
    }
    p->next = cerune_string_owners;
    p->references = 1;
    p->length = length;
    cerune_string_owners = p;
    cerune_string_live_bytes -= (uint64_t)limit - length;
    return (cerune_string){p->data, length};
}
"#;

const CONNECTION: &str = r#"
#ifdef _WIN32
#include <windows.h>
static HANDLE cerune_input_handle;
static int cerune_input_pipe;
static const char *cerune_input_connection(void) {
    static int initialized;
    static const char *failure;
    if (!initialized) {
        DWORD mode;
        initialized = 1;
        cerune_input_handle = GetStdHandle(STD_INPUT_HANDLE);
        if (!cerune_input_handle || cerune_input_handle == INVALID_HANDLE_VALUE) failure = "read-failed";
        else if (GetConsoleMode(cerune_input_handle, &mode)) failure = "unsupported-connection";
        else cerune_input_pipe = GetFileType(cerune_input_handle) == FILE_TYPE_PIPE;
    }
    return failure;
}
static int64_t cerune_input_read_byte(unsigned char *byte) {
    for (;;) {
        DWORD count = 0;
        if (ReadFile(cerune_input_handle, byte, 1, &count, NULL)) {
            if (count == 0 && cerune_input_pipe) continue;
            return count;
        }
        DWORD error = GetLastError();
        if (error == ERROR_MORE_DATA && count > 0) return count;
        if (error == ERROR_BROKEN_PIPE || error == ERROR_HANDLE_EOF) return 0;
        errno = EIO;
        return -1;
    }
}
#elif defined(__linux__)
#include <unistd.h>
static const char *cerune_input_connection(void) { return NULL; }
static int64_t cerune_input_read_byte(unsigned char *byte) { return read(0, byte, 1); }
#else
static const char *cerune_input_connection(void) { return "unsupported-connection"; }
static int64_t cerune_input_read_byte(unsigned char *byte) { (void)byte; errno = EIO; return -1; }
#endif
"#;
