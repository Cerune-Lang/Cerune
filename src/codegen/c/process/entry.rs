//! 専用マーカー内だけで接続設定を解釈し、残りはそのままargvへ渡します。
pub(super) fn emit(files: bool) -> String {
    let parse = PARSE.replace(
        "@FILE_PARSE@",
        if files {
            r#"if (!CERUNE_ARG_CMP(argv[start], CERUNE_ARG_TEXT("--file-root"))) slot=&root; else"#
        } else {
            ""
        },
    );
    let mut s = CONFIG.to_owned();
    s.push_str(&super::super::arguments::ENTRY
        .replace("    size_t count = argc > 0 ? (size_t)(argc - 1) : 0;", &parse)
        .replace("argv[i + 1]","argv[i + start]")
        .replace("    int result = cerune_program();",&format!("    cerune_process_connect(config);\n{}    int result = cerune_program();\n    cerune_process_check(cerune_process_v1_destroy(&cerune_process_context_value),0,\" startup\");\n{}",
            if files {"    if (root) cerune_file_connect(root);\n"}else{""},
            if files {"    cerune_file_disconnect();\n"}else{""})));
    if !files {
        s = s.replace("    CERUNE_PATH root = NULL;\n", "");
    }
    s
}
const CONFIG: &str = r#"
static void cerune_process_startup_failed(const char *code) {
    fprintf(stderr,"cerune-process-startup-v1 code=%s\n",code);
    exit(1);
}
#ifdef _WIN32
#define CERUNE_PATH wchar_t *
#else
#define CERUNE_PATH char *
#endif
static void cerune_process_connect(CERUNE_PATH path) {
    unsigned char *bytes=NULL; size_t length=0;
    if (path) {
#ifdef _WIN32
        FILE *file=_wfopen(path,L"rb");
#else
        FILE *file=fopen(path,"rb");
#endif
        if (!file) cerune_process_startup_failed("open-config");
        bytes=malloc(1048577);
        if (!bytes) { fclose(file); cerune_process_startup_failed("allocation-failed"); }
        length=fread(bytes,1,1048577,file);
        int failed=ferror(file);
        if (fclose(file)) failed=1;
        if (failed) { free(bytes); cerune_process_startup_failed("read-config"); }
        if (length>1048576) { free(bytes); cerune_process_startup_failed("config-too-large"); }
        /* Empty file is invalid JSON, distinct from no connection setting. */
        if (!length) { free(bytes); cerune_process_startup_failed("invalid-config"); }
    }
    uint32_t status=cerune_process_v1_create(1,CERUNE_PROCESS_TARGET,
        (cerune_process_bytes){bytes,length},&cerune_process_context_value);
    free(bytes);
    cerune_process_check(status,0," startup");
}
"#;
const PARSE: &str = r#"
#ifdef _WIN32
#define CERUNE_ARG_TEXT(s) L##s
#define CERUNE_ARG_CMP wcscmp
#else
#define CERUNE_ARG_TEXT(s) s
#define CERUNE_ARG_CMP strcmp
#endif
    CERUNE_PATH root = NULL;
    CERUNE_PATH config = NULL;
    int start=1;
    if (argc>1 && !CERUNE_ARG_CMP(argv[1],CERUNE_ARG_TEXT("--cerune-runtime-v1"))) {
        start=2;
        while (start<argc && CERUNE_ARG_CMP(argv[start],CERUNE_ARG_TEXT("--"))) {
            CERUNE_PATH *slot=NULL;
            @FILE_PARSE@
            if (!CERUNE_ARG_CMP(argv[start],CERUNE_ARG_TEXT("--process-config"))) slot=&config;
            else cerune_process_startup_failed("unknown-setting");
            if (*slot) cerune_process_startup_failed("duplicate-setting");
            if (++start>=argc || !argv[start][0] || (argv[start][0]=='-' && argv[start][1]=='-'))
                cerune_process_startup_failed("missing-value");
            *slot=argv[start++];
        }
        if (start>=argc) cerune_process_startup_failed("missing-delimiter");
        ++start;
    }
    size_t count=argc>start ? (size_t)(argc-start) : 0;
#undef CERUNE_ARG_TEXT
#undef CERUNE_ARG_CMP
"#;
