#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#endif
#include <stdlib.h>

static void cerune_runtime_fail(const char *code, const char *origin) {
    fflush(stdout);
    fputs("cerune: runtime-v1 code=", stderr);
    fputs(code, stderr);
    fputs(origin, stderr);
    fflush(stderr);
    abort();
}

static int64_t cerune_check_i8(int64_t value, const char *code, const char *origin) {
    if (value < -128LL || value > 127LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_check_u8(int64_t value, const char *code, const char *origin) {
    if (value < 0LL || value > 255LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_check_i16(int64_t value, const char *code, const char *origin) {
    if (value < -32768LL || value > 32767LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_check_u16(int64_t value, const char *code, const char *origin) {
    if (value < 0LL || value > 65535LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_i64_add(int64_t left, int64_t right, const char *origin) {
    if ((right > 0 && left > INT64_MAX - right) ||
        (right < 0 && left < INT64_MIN - right)) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return left + right;
}

static int64_t cerune_i64_div(int64_t left, int64_t right, const char *origin) {
    if (right == 0) {
        cerune_runtime_fail("division-by-zero", origin);
    }
    if (left == INT64_MIN && right == -1) {
        cerune_runtime_fail("division-overflow", origin);
    }
    return left / right;
}

static int64_t cerune_i64_neg(int64_t value, const char *origin) {
    if (value == INT64_MIN) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return -value;
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
int64_t cerune_fn_average_0(int64_t cerune_binding_0_left, int64_t cerune_binding_1_right);

int64_t cerune_fn_average_0(int64_t cerune_binding_0_left, int64_t cerune_binding_1_right) {
    int64_t _cerune_eval_0;
    int64_t _cerune_eval_1;
    return cerune_check_u8(cerune_check_u16(cerune_i64_div(cerune_check_u16((_cerune_eval_0 = cerune_check_u16(cerune_binding_0_left, "integer-conversion-out-of-range", " node=4 bytes=55..64\n"), _cerune_eval_1 = cerune_check_u16(cerune_binding_1_right, "integer-conversion-out-of-range", " node=6 bytes=67..77\n"), cerune_i64_add(_cerune_eval_0, _cerune_eval_1, " node=3 bytes=54..78\n")), "integer-overflow", " node=3 bytes=54..78\n"), 2, " node=2 bytes=54..82\n"), "division-overflow", " node=2 bytes=54..82\n"), "integer-conversion-out-of-range", " node=1 bytes=51..83\n");
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    int64_t cerune_binding_2_offset = cerune_check_i8(cerune_i64_neg(3, " node=10 bytes=101..103\n"), "integer-overflow", " node=10 bytes=101..103\n");
    int64_t cerune_binding_3_reading = cerune_check_i16(cerune_i64_neg(32000, " node=13 bytes=120..126\n"), "integer-overflow", " node=13 bytes=120..126\n");
    cerune_output_i64(cerune_check_i16(cerune_i64_add(cerune_binding_3_reading, cerune_check_i16(cerune_binding_2_offset, "integer-conversion-out-of-range", " node=18 bytes=144..155\n"), " node=16 bytes=134..155\n"), "integer-overflow", " node=16 bytes=134..155\n"), 1, " node=15 bytes=128..157\n");
    cerune_output_i64(cerune_fn_average_0(240, 80), 1, " node=20 bytes=158..182\n");
    cerune_output_bool((127 > -128), 1, " node=24 bytes=183..205\n");
    cerune_output_i64(cerune_check_u16(255, "integer-conversion-out-of-range", " node=29 bytes=212..231\n"), 1, " node=28 bytes=206..233\n");
    return 0;
}
