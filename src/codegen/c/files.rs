//! ファイル操作を生成Cへ展開します。OS接続と意味上の検査を分けて残します。
mod connection;
mod entry;
pub(super) fn entry() -> String {
    entry::emit()
}
pub(super) fn support(limit: u64) -> String {
    connection::SOURCE.to_owned() + &SOURCE.replace("@LIMIT@", &limit.to_string())
}
const SOURCE: &str = r#"
static void cerune_file_failed(const char *operation, const char *phase, const char *code,
    int64_t limit, int requested_known, uint64_t requested, uint64_t transferred,
    int known, const char *created, const char *cleanup, const char *origin) {
    fflush(stdout);
    fprintf(stderr, "cerune: file-v1 operation=%s phase=%s code=%s limit=%lld requested=",
        operation, phase, code, (long long)limit);
    if (requested_known) fprintf(stderr, "%llu", (unsigned long long)requested);
    else fputs("unknown", stderr);
    fprintf(stderr, " transferred=%llu progress=%s created=%s cleanup=%s%s",
        (unsigned long long)transferred, known ? "known" : "unknown", created, cleanup, origin);
    fflush(stderr); _Exit(1);
}
static int cerune_file_ascii_equal(const unsigned char *s, size_t n, const char *word) {
    if (n != strlen(word)) return 0;
    for (size_t i = 0; i < n; ++i) {
        unsigned char c = s[i]; if (c >= 'a' && c <= 'z') c -= 'a' - 'A';
        if (c != (unsigned char)word[i]) return 0;
    }
    return 1;
}
static int cerune_file_name_valid(cerune_string name) {
    const unsigned char *s = name.data; size_t n = name.length;
    if (!n || n > 255 || s[n-1] == ' ' || s[n-1] == '.') return 0;
    for (size_t i = 0; i < n; ++i)
        if (s[i] <= 31 || s[i] == 127 || strchr("<>:\"/\\|?*", s[i])) return 0;
    size_t stem = 0; while (stem < n && s[stem] != '.') ++stem;
    while (stem && s[stem-1] == ' ') --stem;
    const char *reserved[] = {"CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"};
    for (size_t i = 0; i < sizeof(reserved)/sizeof(*reserved); ++i)
        if (cerune_file_ascii_equal(s, stem, reserved[i])) return 0;
    if (stem >= 4 && (cerune_file_ascii_equal(s, 3, "COM") || cerune_file_ascii_equal(s, 3, "LPT"))) {
        if (stem == 4 && s[3] >= '1' && s[3] <= '9') return 0;
        if (stem == 5 && s[3] == 0xc2 && (s[4] == 0xb9 || s[4] == 0xb2 || s[4] == 0xb3)) return 0;
    }
    return 1;
}
static const char *cerune_file_validate(cerune_string name, int64_t limit) {
    if (limit < 0 || limit > INT32_MAX) return "invalid-limit";
    if (!cerune_file_name_valid(name)) return "invalid-name";
    if (!cerune_file_configured) return "unconfigured";
    return NULL;
}
static cerune_string cerune_file_read_text(cerune_string name, int64_t limit, const char *origin) {
    const char *code = cerune_file_validate(name, limit);
    if (code) cerune_file_failed("read-text", "validate", code, limit, 0, 0, 0, 1, "no", "not-attempted", origin);
    if ((uint64_t)limit > UINT64_C(@LIMIT@) - cerune_string_live_bytes)
        cerune_file_failed("read-text", "reserve", "budget-exceeded", limit, 0, 0, 0, 1, "no", "not-attempted", origin);
    if ((uint64_t)limit > SIZE_MAX - sizeof(cerune_string_owner))
        cerune_file_failed("read-text", "reserve", "allocation-failed", limit, 0, 0, 0, 1, "no", "not-attempted", origin);
    cerune_string_owner *p = malloc(sizeof(cerune_string_owner) + (size_t)limit);
    if (!p) cerune_file_failed("read-text", "reserve", "allocation-failed", limit, 0, 0, 0, 1, "no", "not-attempted", origin);
    cerune_string_live_bytes += (uint64_t)limit;
    intptr_t handle = 0;
    code = cerune_file_open(name, 0, &handle);
    if (code) {
        free(p); cerune_string_live_bytes -= (uint64_t)limit;
        cerune_file_failed("read-text", "open", code, limit, 0, 0, 0, 1,
            "no", "not-attempted", origin);
    }
    size_t length = 0; uint64_t transferred = 0; int known = 1, eof = 0;
    for (;;) {
        unsigned char probe;
        size_t request = length == (size_t)limit ? 1 : (size_t)limit - length;
        if (request > 4096) request = 4096;
        int64_t n = cerune_file_read(handle, length == (size_t)limit ? &probe : p->data + length, request);
        if (n < 0) { if (errno == EINTR) continue; code = "read-failed"; break; }
        if ((uint64_t)n > request) { code = "invalid-response"; known = 0; break; }
        if (!n) { eof = 1; break; }
        transferred += (uint64_t)n;
        if (length == (size_t)limit) { code = "limit-exceeded"; break; }
        length += (size_t)n;
    }
    const char *phase = "read";
    int closed = cerune_file_close(handle) == 0;
    if (!code && !closed) { phase = "close"; code = "close-failed"; }
    if (!code && !cerune_utf8(p->data, length)) { phase = "decode"; code = "invalid-utf8"; }
    if (code || !length) {
        free(p); cerune_string_live_bytes -= (uint64_t)limit;
        if (code) cerune_file_failed("read-text", phase, code, limit, eof, length, transferred,
            known, "no", closed ? "closed" : "failed", origin);
        return (cerune_string){(const unsigned char *)"", 0};
    }
    p->next = cerune_string_owners; p->references = 1; p->length = length;
    cerune_string_owners = p; cerune_string_live_bytes -= (uint64_t)limit - length;
    return (cerune_string){p->data, length};
}
static void cerune_file_write_new(cerune_string name, cerune_string text, int64_t limit, const char *origin) {
    const char *code = cerune_file_validate(name, limit);
    if (!code && text.length > (uint64_t)limit) code = "limit-exceeded";
    if (code) cerune_file_failed("write-new", "validate", code, limit, 1, text.length, 0, 1, "no", "not-attempted", origin);
    intptr_t handle = 0; code = cerune_file_open(name, 1, &handle);
    if (code) cerune_file_failed("write-new", "open", code, limit, 1, text.length, 0, 1,
        !strcmp(code, "invalid-response") ? "unknown" : "no", "not-attempted", origin);
    size_t transferred = 0; int known = 1;
    while (transferred < text.length) {
        size_t request = text.length - transferred;
        int64_t n = cerune_file_write(handle, text.data + transferred, request);
        if (n < 0) { if (errno == EINTR) continue; code = "write-failed"; break; }
        if ((uint64_t)n > request) { code = "invalid-response"; known = 0; break; }
        if (!n) { code = "zero-progress"; break; }
        transferred += (size_t)n;
    }
    const char *phase = "write"; int closed = cerune_file_close(handle) == 0;
    if (!code && !closed) { phase = "close"; code = "close-failed"; }
    if (code) cerune_file_failed("write-new", phase, code, limit, 1, text.length, transferred,
        known, "yes", closed ? "closed" : "failed", origin);
}
"#;
