pub(super) fn emit() -> String {
    super::super::arguments::ENTRY
        .replace("    size_t count = argc > 0 ? (size_t)(argc - 1) : 0;", PARSE)
        .replace("argv[i + 1]", "argv[i + start]")
        .replace("    int result = cerune_program();", "    if (root) cerune_file_connect(root);\n    int result = cerune_program();\n    cerune_file_disconnect();")
}
const PARSE: &str = r#"
    /* runtime-startup-v1: only the leading marker introduces configuration. */
#ifdef _WIN32
#define CERUNE_ARG_TEXT(s) L##s
#define CERUNE_ARG_CMP wcscmp
    wchar_t *root = NULL;
#else
#define CERUNE_ARG_TEXT(s) s
#define CERUNE_ARG_CMP strcmp
    char *root = NULL;
#endif
    int start = 1;
    if (argc > 1 && !CERUNE_ARG_CMP(argv[1], CERUNE_ARG_TEXT("--cerune-runtime-v1"))) {
        start = 2;
        while (start < argc && CERUNE_ARG_CMP(argv[start], CERUNE_ARG_TEXT("--"))) {
            if (CERUNE_ARG_CMP(argv[start], CERUNE_ARG_TEXT("--file-root")))
                cerune_file_startup_failed("configure", "unknown-setting");
            if (root) cerune_file_startup_failed("configure", "duplicate-setting");
            if (++start >= argc || !argv[start][0] ||
                (argv[start][0] == '-' && argv[start][1] == '-'))
                cerune_file_startup_failed("configure", "invalid-root");
            root = argv[start++];
        }
        if (start >= argc) cerune_file_startup_failed("configure", "missing-delimiter");
        ++start;
    }
    size_t count = argc > start ? (size_t)(argc - start) : 0;
#undef CERUNE_ARG_TEXT
#undef CERUNE_ARG_CMP
"#;
