#include <stdlib.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stddef.h>
#include <string.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#endif
#include <stdlib.h>
#if defined(__clang__)
#pragma clang fp contract(off)
#elif defined(__GNUC__)
#pragma GCC optimize ("fp-contract=off")
#endif

static void cerune_runtime_fail(const char *code, const char *origin) {
    fflush(stdout);
    fputs("cerune: runtime-v1 code=", stderr);
    fputs(code, stderr);
    fputs(origin, stderr);
    fflush(stderr);
    abort();
}

/* Explicit source-call depth limit: 128; not a physical stack guarantee. */
#define CERUNE_CALL_LIMIT 128U

typedef struct { uint64_t function; const char *origin; } cerune_call_frame;
static cerune_call_frame *cerune_call_frames = NULL;
static size_t cerune_call_depth = 0, cerune_call_capacity = 0;
static void cerune_call_enter(uint64_t function, const char *origin) {
    if (cerune_call_depth >= CERUNE_CALL_LIMIT) {
        fflush(stdout);
        fprintf(stderr, "call-v1 reason=depth-limit limit=%u function=%llu%s",
            (unsigned)CERUNE_CALL_LIMIT, (unsigned long long)function, origin);
        for (size_t i = 0; i < cerune_call_depth; ++i) {
            fprintf(stderr, "call-v1 frame=%llu function=%llu%s", (unsigned long long)i,
                (unsigned long long)cerune_call_frames[i].function, cerune_call_frames[i].origin);
        }
        fflush(stderr);
        _Exit(1);
    }
    if (cerune_call_depth == cerune_call_capacity) {
        uint64_t next = cerune_call_capacity ? (uint64_t)cerune_call_capacity * 2 : 8;
        if (next > CERUNE_CALL_LIMIT) next = CERUNE_CALL_LIMIT;
        if (next > SIZE_MAX / sizeof(cerune_call_frame)) cerune_runtime_fail("allocation-failed", origin);
        cerune_call_frame *frames = (cerune_call_frame *)realloc(cerune_call_frames, (size_t)next * sizeof(cerune_call_frame));
        if (!frames) cerune_runtime_fail("allocation-failed", origin);
        cerune_call_frames = frames;
        cerune_call_capacity = (size_t)next;
    }
    cerune_call_frames[cerune_call_depth++] = (cerune_call_frame){function, origin};
}
static void cerune_call_leave(void) { --cerune_call_depth; }
static void cerune_call_finish(void) {
    free(cerune_call_frames);
    cerune_call_frames = NULL;
    cerune_call_capacity = 0;
}

typedef struct cerune_string {
    const unsigned char *data;
    size_t length;
} cerune_string;

static inline bool cerune_string_equal(cerune_string left, cerune_string right) {
    return left.length == right.length &&
        (left.length == 0 || memcmp(left.data, right.data, left.length) == 0);
}


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
static void cerune_output_i64(int64_t value, int newline, const char *origin) {
    char text[128];
    int n = snprintf(text, sizeof(text), "%lld", (long long)value);
    if (n < 0 || (size_t)n >= sizeof(text) - 1) { fputs("cerune: output formatting failed\n", stderr); _Exit(1); }
    if (newline) text[n++] = '\n';
    cerune_output_text((const unsigned char *)text, (size_t)n, origin);
}
static void cerune_output_u64(uint64_t value, int newline, const char *origin) {
    char text[128];
    int n = snprintf(text, sizeof(text), "%llu", (unsigned long long)value);
    if (n < 0 || (size_t)n >= sizeof(text) - 1) { fputs("cerune: output formatting failed\n", stderr); _Exit(1); }
    if (newline) text[n++] = '\n';
    cerune_output_text((const unsigned char *)text, (size_t)n, origin);
}
static void cerune_output_f32(float value, int newline, const char *origin) {
    char text[128];
    int n = snprintf(text, sizeof(text), "%.9g", (double)value);
    if (n < 0 || (size_t)n >= sizeof(text) - 1) { fputs("cerune: output formatting failed\n", stderr); _Exit(1); }
    if (newline) text[n++] = '\n';
    cerune_output_text((const unsigned char *)text, (size_t)n, origin);
}
static void cerune_output_f64(double value, int newline, const char *origin) {
    char text[128];
    int n = snprintf(text, sizeof(text), "%.17g", (double)value);
    if (n < 0 || (size_t)n >= sizeof(text) - 1) { fputs("cerune: output formatting failed\n", stderr); _Exit(1); }
    if (newline) text[n++] = '\n';
    cerune_output_text((const unsigned char *)text, (size_t)n, origin);
}
static void cerune_output_bool(int value, int newline, const char *origin) {
    const char *text = value ? "true\n" : "false\n";
    cerune_output_text((const unsigned char *)text, (value ? 4 : 5) + (size_t)newline, origin);
}

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
int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_string cerune_binding_0_text = (cerune_string){ (const unsigned char *)"\346\227\245\346\234\254\350\252\236\012\000", 11 };
    cerune_string cerune_binding_1_saved = cerune_binding_0_text;
    cerune_binding_0_text = (cerune_string){ (const unsigned char *)"\143\150\141\156\147\145\144", 7 };
    cerune_output_bool((cerune_string_equal(cerune_binding_1_saved, (cerune_string){ (const unsigned char *)"\346\227\245\346\234\254\350\252\236\012\000", 11 })), 1, " node=6 bytes=75..112\n");
    cerune_output_string(cerune_binding_0_text, 1, " node=10 bytes=113..125\n");
    cerune_call_finish();
    return 0;
}
