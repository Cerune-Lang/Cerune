//! 共通IRの一出力操作を、本文・受理数・不変の出自を持つC呼出しへ写します。
use super::ir::{Module, Statement};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    fn body(s: &Statement) -> bool {
        match s {
            Statement::Print { .. } | Statement::Write { .. } => true,
            Statement::If {
                then_body,
                else_body,
                ..
            } => then_body.iter().chain(else_body).any(body),
            Statement::While { body: b, .. } | Statement::For { body: b, .. } => b.iter().any(body),
            _ => false,
        }
    }
    module
        .statements
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.body))
        .any(body)
}

pub(super) fn support(strings: bool) -> String {
    let mut out = String::from(COMMON);
    for (kind, ty, fmt, cast) in [
        ("i64", "int64_t", "%lld", "long long"),
        ("u64", "uint64_t", "%llu", "unsigned long long"),
        ("f32", "float", "%.9g", "double"),
        ("f64", "double", "%.17g", "double"),
    ] {
        writeln!(out, "static void cerune_output_{kind}({ty} value, int newline, const char *origin) {{\n    char text[128];\n    int n = snprintf(text, sizeof(text), \"{fmt}\", ({cast})value);\n    if (n < 0 || (size_t)n >= sizeof(text) - 1) {{ fputs(\"cerune: output formatting failed\\n\", stderr); _Exit(1); }}\n    if (newline) text[n++] = '\\n';\n    cerune_output_text((const unsigned char *)text, (size_t)n, origin);\n}}").unwrap();
    }
    out.push_str(
        r#"static void cerune_output_bool(int value, int newline, const char *origin) {
    const char *text = value ? "true\n" : "false\n";
    cerune_output_text((const unsigned char *)text, (value ? 4 : 5) + (size_t)newline, origin);
}
"#,
    );
    if strings {
        out.push_str(STRINGS);
    }
    out
}

const COMMON: &str = r#"
#include <stddef.h>
#include <stdlib.h>
#include <errno.h>
#include <signal.h>
#ifdef _WIN32
#include <io.h>
#else
#include <unistd.h>
#endif

typedef struct cerune_output_operation {
    size_t requested;
    size_t accepted;
    const char *origin;
#ifdef SIGPIPE
    void (*prior_sigpipe)(int);
#endif
} cerune_output_operation;

static void cerune_output_failed(cerune_output_operation *op) {
    fprintf(stderr, "cerune: io-v1 operation=write requested=%llu accepted=%llu%s",
        (unsigned long long)op->requested, (unsigned long long)op->accepted, op->origin);
    fflush(stderr);
    _Exit(1);
}

/* Execution connection; no observation callbacks or mutable current origin. */
static int64_t cerune_output_write(const unsigned char *data, size_t length) {
#ifdef _WIN32
    return _write(1, data, (unsigned int)length);
#else
    return write(1, data, length);
#endif
}

static cerune_output_operation cerune_output_begin(size_t length, const char *origin) {
    cerune_output_operation op;
    op.requested = length;
    op.accepted = 0;
    op.origin = origin;
#ifdef SIGPIPE
    op.prior_sigpipe = signal(SIGPIPE, SIG_IGN);
    if (op.prior_sigpipe == SIG_ERR) cerune_output_failed(&op);
#endif
    return op;
}

static void cerune_output_part(cerune_output_operation *op, const unsigned char *data, size_t length) {
    while (length != 0) {
        size_t request = length < 65536 ? length : 65536;
        int64_t n = cerune_output_write(data, request);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0 || (uint64_t)n > request) cerune_output_failed(op);
        op->accepted += (size_t)n;
        data += (size_t)n;
        length -= (size_t)n;
    }
}
static void cerune_output_end(cerune_output_operation *op) {
#ifdef SIGPIPE
    if (signal(SIGPIPE, op->prior_sigpipe) == SIG_ERR) cerune_output_failed(op);
#else
    (void)op;
#endif
}
static void cerune_output_text(const unsigned char *data, size_t length, const char *origin) {
    if (length == 0) return;
    cerune_output_operation op = cerune_output_begin(length, origin);
    cerune_output_part(&op, data, length);
    cerune_output_end(&op);
}
"#;

const STRINGS: &str = r#"
static void cerune_output_string(cerune_string value, int newline, const char *origin) {
    cerune_output_operation op = cerune_output_begin(value.length + (size_t)newline, origin);
    cerune_output_part(&op, value.data, value.length);
    if (newline) cerune_output_part(&op, (const unsigned char *)"\n", 1);
    cerune_output_end(&op);
}
static size_t cerune_output_escape(unsigned char b, unsigned char *out) {
    out[0] = '\\';
    switch (b) {
    case 0: out[1] = '0'; return 2;
    case 9: out[1] = 't'; return 2;
    case 10: out[1] = 'n'; return 2;
    case 13: out[1] = 'r'; return 2;
    case 34: out[1] = '"'; return 2;
    case 92: out[1] = '\\'; return 2;
    default:
        if (b < 32 || b == 127) {
            const unsigned char *hex = (const unsigned char *)"0123456789abcdef";
            out[1] = 'u'; out[2] = '{'; out[3] = hex[b >> 4]; out[4] = hex[b & 15]; out[5] = '}';
            return 6;
        }
        out[0] = b; return 1;
    }
}
static void cerune_output_quoted(cerune_string value, int newline, const char *origin) {
    unsigned char escaped[6], buffer[256];
    size_t requested = 2 + (size_t)newline;
    for (size_t i = 0; i < value.length; ++i) requested += cerune_output_escape(value.data[i], escaped);
    cerune_output_operation op = cerune_output_begin(requested, origin);
    size_t used = 0;
    buffer[used++] = '"';
    for (size_t i = 0; i < value.length; ++i) {
        size_t n = cerune_output_escape(value.data[i], escaped);
        for (size_t j = 0; j < n; ++j) {
            if (used == sizeof(buffer)) {
                cerune_output_part(&op, buffer, used);
                used = 0;
            }
            buffer[used++] = escaped[j];
        }
    }
    cerune_output_part(&op, buffer, used);
    cerune_output_part(&op, (const unsigned char *)"\"", 1);
    if (newline) cerune_output_part(&op, (const unsigned char *)"\n", 1);
    cerune_output_end(&op);
}
"#;
