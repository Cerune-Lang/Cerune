/* Trace settings belong to the leading runtime marker, never to program arguments. */
static int cerune_trace_limit(const cerune_trace_char *s, size_t *value) {
    size_t n = 0;
    if (!*s) return 0;
    for (; *s; ++s) {
        if (*s < '0' || *s > '9') return 0;
        size_t digit = (size_t)(*s - '0');
        if (n > (SIZE_MAX - digit) / 10) return 0;
        n = 10 * n + digit;
    }
    *value = n;
    return 1;
}
static int cerune_trace_option(const cerune_trace_char *s) {
    if (!cerune_trace_cmp(s, CERUNE_TRACE_TEXT("--trace"))) return 1;
    if (!cerune_trace_cmp(s, CERUNE_TRACE_TEXT("--trace-event-limit"))) return 2;
    if (!cerune_trace_cmp(s, CERUNE_TRACE_TEXT("--trace-byte-limit"))) return 4;
    return 0;
}
static int cerune_trace_configure(int *argc, cerune_trace_char **argv) {
    if (*argc < 2 || cerune_trace_cmp(argv[1], CERUNE_TRACE_TEXT("--cerune-runtime-v1"))) return 1;
    int i = 2, seen = 0, kept = 2;
    for (; i < *argc && cerune_trace_cmp(argv[i], CERUNE_TRACE_TEXT("--")); i += 2) {
        int option = cerune_trace_option(argv[i]);
        if (i + 1 >= *argc || !argv[i+1][0] || (argv[i+1][0] == '-' && argv[i+1][1] == '-')) return 0;
        if (option == 0) {
            if (!(CERUNE_HAS_FILE_ROOT && !cerune_trace_cmp(argv[i], CERUNE_TRACE_TEXT("--file-root"))) &&
                !(CERUNE_HAS_PROCESS_CONFIG && !cerune_trace_cmp(argv[i], CERUNE_TRACE_TEXT("--process-config")))) return 0;
            argv[kept++] = argv[i];
            argv[kept++] = argv[i+1];
        } else {
            if (seen & option) return 0;
            seen |= option;
            if (option == 1) cerune_trace_path = argv[i+1];
            if (option == 2 && !cerune_trace_limit(argv[i+1], &cerune_trace_events)) return 0;
            if (option == 4 && (!cerune_trace_limit(argv[i+1], &cerune_trace_bytes) ||
                cerune_trace_bytes < sizeof(CERUNE_TRACE_END("allocation-failed")) - 1)) return 0;
        }
    }
    if (i == *argc || (seen && !(seen & 1))) return 0;
    if (!CERUNE_HAS_FILE_ROOT && !CERUNE_HAS_PROCESS_CONFIG) kept = 1;
    else argv[kept++] = argv[i];
    ++i;
    while (i < *argc) argv[kept++] = argv[i++];
    argv[kept] = NULL;
    *argc = kept;
    return 1;
}
#ifdef _WIN32
int wmain(int argc, wchar_t **argv) {
#else
int main(int argc, char **argv) {
#endif
    if (!cerune_trace_configure(&argc, argv)) {
        fputs("cerune: trace-v1: invalid runtime configuration\n", stderr);
        return 1;
    }
    if (cerune_trace_path && atexit(cerune_trace_save) != 0) {
        fputs("cerune: trace-v1: could not register trace writer\n", stderr);
        cerune_trace_path = NULL;
    }
    return CERUNE_PROGRAM_ENTRY;
}
