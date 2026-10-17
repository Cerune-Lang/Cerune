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
#endif

static void cerune_runtime_fail(const char *code, const char *origin) {
    fflush(stdout);
    fputs("cerune: runtime-v1 code=", stderr);
    fputs(code, stderr);
    fputs(origin, stderr);
    fflush(stderr);
    abort();
}

typedef struct cerune_string {
    const unsigned char *data;
    size_t length;
} cerune_string;

static inline bool cerune_string_equal(cerune_string left, cerune_string right) {
    return left.length == right.length &&
        (left.length == 0 || memcmp(left.data, right.data, left.length) == 0);
}


typedef struct cerune_string_owner {
    struct cerune_string_owner *next;
    uint64_t references;
    size_t length;
    unsigned char data[];
} cerune_string_owner;
static cerune_string_owner *cerune_string_owners;
static uint64_t cerune_string_live_bytes;
static void cerune_string_retain(cerune_string value) {
    for (cerune_string_owner *p = cerune_string_owners; p; p = p->next)
        if (p->data == value.data) { ++p->references; return; }
}
static void cerune_string_release(cerune_string value) {
    cerune_string_owner **link = &cerune_string_owners;
    while (*link) {
        cerune_string_owner *p = *link;
        if (p->data == value.data) {
            if (--p->references == 0) {
                *link = p->next;
                cerune_string_live_bytes -= p->length;
                free(p);
            }
            return;
        }
        link = &p->next;
    }
}
static cerune_string cerune_string_concat(cerune_string left, cerune_string right, const char *origin) {
    if (right.length > (uint64_t)INT64_MAX - left.length ||
        right.length > SIZE_MAX - left.length ||
        left.length + right.length > SIZE_MAX - sizeof(cerune_string_owner))
        cerune_runtime_fail("allocation-size-overflow", origin);
    size_t length = left.length + right.length;
    if (!length) return (cerune_string){(const unsigned char *)"", 0};
    if ((uint64_t)length > UINT64_C(67108864) - cerune_string_live_bytes)
        cerune_runtime_fail("allocation-limit-exceeded", origin);
    cerune_string_owner *p = malloc(sizeof(cerune_string_owner) + length);
    if (!p) cerune_runtime_fail("allocation-failed", origin);
    p->next = cerune_string_owners;
    p->references = 1;
    p->length = length;
    memcpy(p->data, left.data, left.length);
    memcpy(p->data + left.length, right.data, right.length);
    cerune_string_owners = p;
    cerune_string_live_bytes += length;
    return (cerune_string){p->data, length};
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
cerune_string cerune_fn__ownership0_0(void);
cerune_string cerune_fn__ownership1_1(cerune_string cerune_binding_5__owned5);
cerune_string cerune_fn__ownership2_2(cerune_string cerune_binding_8__owned8);
cerune_string cerune_fn__ownership3_3(cerune_string cerune_binding_12__owned12, cerune_string cerune_binding_13__owned13);

cerune_string cerune_fn__ownership0_0(void) {
    cerune_string cerune_binding_2__read2 = (cerune_string){ (const unsigned char *)"\346\227\245", 3 };
    cerune_string cerune_binding_3__read3 = (cerune_string){ (const unsigned char *)"\346\234\254", 3 };
    cerune_string cerune_binding_4__owned4 = cerune_string_concat(cerune_binding_2__read2, cerune_binding_3__read3, " node=1 bytes=19..39\n");
    return cerune_binding_4__owned4;
}

cerune_string cerune_fn__ownership1_1(cerune_string cerune_binding_5__owned5) {
    cerune_string cerune_binding_6__read6 = cerune_binding_5__owned5;
    cerune_string cerune_binding_7__owned7 = cerune_binding_6__read6;
    cerune_string_retain(cerune_binding_7__owned7);
    return cerune_binding_7__owned7;
}

cerune_string cerune_fn__ownership2_2(cerune_string cerune_binding_8__owned8) {
    cerune_string cerune_binding_9__read9 = cerune_binding_8__owned8;
    cerune_string cerune_binding_10__read10 = (cerune_string){ (const unsigned char *)"\041", 1 };
    cerune_string cerune_binding_11__owned11 = cerune_string_concat(cerune_binding_9__read9, cerune_binding_10__read10, " node=7 bytes=70..87\n");
    return cerune_binding_11__owned11;
}

cerune_string cerune_fn__ownership3_3(cerune_string cerune_binding_12__owned12, cerune_string cerune_binding_13__owned13) {
    cerune_string_release(cerune_binding_12__owned12);
    return cerune_binding_13__owned13;
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_string cerune_binding_0_text = cerune_fn__ownership0_0();
    cerune_string cerune_binding_1_saved = cerune_fn__ownership1_1(cerune_binding_0_text);
    cerune_binding_0_text = cerune_fn__ownership3_3(cerune_binding_0_text, cerune_fn__ownership2_2(cerune_binding_0_text));
    cerune_string cerune_binding_14__read14 = cerune_binding_1_saved;
    cerune_output_string(cerune_binding_14__read14, 1, " node=10 bytes=89..102\n");
    cerune_string cerune_binding_15__read15 = cerune_binding_0_text;
    cerune_output_string(cerune_binding_15__read15, 1, " node=12 bytes=103..115\n");
    cerune_string_release(cerune_binding_1_saved);
    cerune_string_release(cerune_binding_0_text);
    return 0;
}
