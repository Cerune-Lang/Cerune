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

static int64_t cerune_i64_rem(int64_t left, int64_t right, const char *origin) {
    if (right == 0) cerune_runtime_fail("remainder-by-zero", origin);
    if (right == -1) return 0;
    return left % right;
}

static int64_t cerune_u8_bit_and(int64_t left, int64_t right, const char *origin) {
    return left & right;
}

static int64_t cerune_u8_bit_or(int64_t left, int64_t right, const char *origin) {
    return left | right;
}

static int64_t cerune_u8_bit_xor(int64_t left, int64_t right, const char *origin) {
    return left ^ right;
}

static int64_t cerune_u8_shl(int64_t left, int64_t right, const char *origin) {
    if (right < 0 || right >= 8) cerune_runtime_fail("invalid-shift-count", origin);
    if (left < (0) || left > (255LL >> right)) cerune_runtime_fail("integer-overflow", origin);
    return left * (INT64_C(1) << right);
}

static int64_t cerune_i8_shr(int64_t left, int64_t right, const char *origin) {
    if (right < 0 || right >= 8) cerune_runtime_fail("invalid-shift-count", origin);
    return left >= 0 ? (int64_t)((uint64_t)left >> right)
        : -1 - (int64_t)((uint64_t)(-1 - left) >> right);
}

static int64_t cerune_u8_shr(int64_t left, int64_t right, const char *origin) {
    if (right < 0 || right >= 8) cerune_runtime_fail("invalid-shift-count", origin);
    return left >= 0 ? (int64_t)((uint64_t)left >> right)
        : -1 - (int64_t)((uint64_t)(-1 - left) >> right);
}

static int64_t cerune_check_i8(int64_t value, const char *code, const char *origin) {
    if (value < -128LL || value > 127LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_check_u8(int64_t value, const char *code, const char *origin) {
    if (value < 0LL || value > 255LL) cerune_runtime_fail(code, origin);
    return value;
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
int64_t cerune_fn_mark_0(int64_t cerune_binding_0_value);

int64_t cerune_fn_mark_0(int64_t cerune_binding_0_value) {
    cerune_output_i64(cerune_binding_0_value, 1, " node=0 bytes=27..40\n");
    return cerune_binding_0_value;
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    int64_t _cerune_bit_left_5, _cerune_bit_right_5;
    int64_t _cerune_bit_left_11, _cerune_bit_right_11;
    int64_t _cerune_bit_left_15, _cerune_bit_right_15;
    int64_t _cerune_bit_left_18, _cerune_bit_right_18;
    int64_t _cerune_bit_left_21, _cerune_bit_right_21;
    int64_t _cerune_bit_left_27, _cerune_bit_right_27;
    int64_t _cerune_bit_left_31, _cerune_bit_right_31;
    int64_t _cerune_bit_left_36, _cerune_bit_right_36;
    int64_t _cerune_bit_left_41, _cerune_bit_right_41;
    int64_t _cerune_bit_left_49, _cerune_bit_right_49;
    int64_t cerune_binding_1_bits = cerune_check_u8((_cerune_bit_left_5 = 1, _cerune_bit_right_5 = 7, cerune_u8_shl(_cerune_bit_left_5, _cerune_bit_right_5, " node=5 bytes=68..74\n")), "integer-overflow", " node=5 bytes=68..74\n");
    cerune_output_i64(cerune_binding_1_bits, 1, " node=8 bytes=76..88\n");
    cerune_output_i64(cerune_check_u8((_cerune_bit_left_11 = cerune_binding_1_bits, _cerune_bit_right_11 = 7, cerune_u8_shr(_cerune_bit_left_11, _cerune_bit_right_11, " node=11 bytes=95..104\n")), "integer-overflow", " node=11 bytes=95..104\n"), 1, " node=10 bytes=89..106\n");
    cerune_output_i64(cerune_check_u8((_cerune_bit_left_15 = 0, _cerune_bit_right_15 = 255, cerune_u8_bit_xor(_cerune_bit_left_15, _cerune_bit_right_15, " node=15 bytes=113..117\n")), "integer-overflow", " node=15 bytes=113..117\n"), 1, " node=14 bytes=107..119\n");
    cerune_output_i64(cerune_check_u8((_cerune_bit_left_18 = cerune_fn_mark_0(1), _cerune_bit_right_18 = cerune_check_u8((_cerune_bit_left_21 = cerune_fn_mark_0(2), _cerune_bit_right_21 = cerune_fn_mark_0(3), cerune_u8_bit_xor(_cerune_bit_left_21, _cerune_bit_right_21, " node=21 bytes=136..155\n")), "integer-overflow", " node=21 bytes=136..155\n"), cerune_u8_bit_or(_cerune_bit_left_18, _cerune_bit_right_18, " node=18 bytes=126..155\n")), "integer-overflow", " node=18 bytes=126..155\n"), 1, " node=17 bytes=120..157\n");
    cerune_output_i64(cerune_check_u8((_cerune_bit_left_27 = cerune_binding_1_bits, _cerune_bit_right_27 = 127, cerune_u8_bit_and(_cerune_bit_left_27, _cerune_bit_right_27, " node=27 bytes=164..174\n")), "integer-overflow", " node=27 bytes=164..174\n"), 1, " node=26 bytes=158..176\n");
    cerune_output_i64((_cerune_bit_left_31 = cerune_i64_neg(7, " node=32 bytes=183..185\n"), _cerune_bit_right_31 = 3, cerune_i64_rem(_cerune_bit_left_31, _cerune_bit_right_31, " node=31 bytes=183..189\n")), 1, " node=30 bytes=177..191\n");
    cerune_output_i64((_cerune_bit_left_36 = INT64_MIN, _cerune_bit_right_36 = cerune_i64_neg(1, " node=38 bytes=221..223\n"), cerune_i64_rem(_cerune_bit_left_36, _cerune_bit_right_36, " node=36 bytes=198..223\n")), 1, " node=35 bytes=192..225\n");
    cerune_output_i64(cerune_check_i8((_cerune_bit_left_41 = cerune_check_i8(cerune_i64_neg(3, " node=42 bytes=232..236\n"), "integer-overflow", " node=42 bytes=232..236\n"), _cerune_bit_right_41 = 1, cerune_i8_shr(_cerune_bit_left_41, _cerune_bit_right_41, " node=41 bytes=232..241\n")), "integer-overflow", " node=41 bytes=232..241\n"), 1, " node=40 bytes=226..243\n");
    cerune_output_bool((false && (cerune_check_u8((_cerune_bit_left_49 = cerune_binding_1_bits, _cerune_bit_right_49 = 1, cerune_u8_shl(_cerune_bit_left_49, _cerune_bit_right_49, " node=49 bytes=259..270\n")), "integer-overflow", " node=49 bytes=259..270\n") == 0)), 1, " node=45 bytes=244..277\n");
    return 0;
}
