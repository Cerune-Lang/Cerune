#include <stdlib.h>
#include <stddef.h>
#include <math.h>
#include <float.h>
#include <stdint.h>
#include <stdio.h>
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

static double cerune_convert_i16_f64(int64_t value, const char *origin) {
    double result = (double)value;
    double number = (double)result;
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-inexact", origin);
    if ((int64_t)number != value) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static float cerune_convert_u32_f32(int64_t value, const char *origin) {
    float result = (float)value;
    double number = (double)result;
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-inexact", origin);
    if ((int64_t)number != value) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static double cerune_convert_u32_f64(int64_t value, const char *origin) {
    double result = (double)value;
    double number = (double)result;
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-inexact", origin);
    if ((int64_t)number != value) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static double cerune_convert_i64_f64(int64_t value, const char *origin) {
    double result = (double)value;
    double number = (double)result;
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-inexact", origin);
    if ((int64_t)number != value) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static int64_t cerune_convert_f32_i16(float value, const char *origin) {
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    if (number == 0.0 && signbit(number)) cerune_runtime_fail("conversion-negative-zero", origin);
    if (number < -32768.0 || number >= 32768.0) cerune_runtime_fail("conversion-out-of-range", origin);
    int64_t result = (int64_t)number;
    if ((double)result != number) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static double cerune_convert_f32_f64(float value, const char *origin) {
    if (isnan(value)) cerune_runtime_fail("conversion-nan", origin);
    return (double)value;
}

static int64_t cerune_convert_f64_i64(double value, const char *origin) {
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    if (number == 0.0 && signbit(number)) cerune_runtime_fail("conversion-negative-zero", origin);
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-out-of-range", origin);
    int64_t result = (int64_t)number;
    if ((double)result != number) cerune_runtime_fail("conversion-inexact", origin);
    return result;
}

static float cerune_convert_f64_f32(double value, const char *origin) {
    if (isnan(value)) cerune_runtime_fail("conversion-nan", origin);
    if (isinf(value)) return signbit(value) ? -INFINITY : INFINITY;
    if (value > (double)FLT_MAX || value < -(double)FLT_MAX) cerune_runtime_fail("conversion-out-of-range", origin);
    float result = (float)value;
    if ((double)result != value) cerune_runtime_fail("conversion-inexact", origin);
    return result;
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
double cerune_fn_measure_0(int64_t cerune_binding_0_value);

double cerune_fn_measure_0(int64_t cerune_binding_0_value) {
    double _cerune_eval_0;
    double _cerune_eval_1;
    return (_cerune_eval_0 = cerune_convert_i16_f64(cerune_binding_0_value, " node=2 bytes=43..53\n"), _cerune_eval_1 = cerune_convert_i64_f64(2, " node=4 bytes=56..62\n"), (_cerune_eval_0 / _cerune_eval_1));
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    int64_t _cerune_eval_0;
    double _cerune_eval_1;
    int64_t cerune_binding_1_count = 42;
    double cerune_binding_2_wide = cerune_convert_u32_f64(cerune_binding_1_count, " node=9 bytes=95..114\n");
    float cerune_binding_3_narrow = cerune_convert_f64_f32(cerune_binding_2_wide, " node=12 bytes=130..139\n");
    cerune_output_i64(cerune_convert_f32_i16(cerune_binding_3_narrow, " node=15 bytes=147..158\n"), 1, " node=14 bytes=141..160\n");
    cerune_output_i64(cerune_convert_f64_i64(cerune_binding_2_wide, " node=18 bytes=167..176\n"), 1, " node=17 bytes=161..178\n");
    cerune_output_f64(cerune_convert_f32_f64(cerune_binding_3_narrow, " node=21 bytes=185..196\n"), 1, " node=20 bytes=179..198\n");
    cerune_output_f32(cerune_convert_u32_f32(cerune_binding_1_count, " node=24 bytes=205..215\n"), 1, " node=23 bytes=199..217\n");
    cerune_output_f64((_cerune_eval_0 = 3, (cerune_call_enter(0ULL, " node=27 bytes=224..234\n"), _cerune_eval_1 = cerune_fn_measure_0(_cerune_eval_0), cerune_call_leave(), _cerune_eval_1)), 1, " node=26 bytes=218..236\n");
    cerune_output_f32(cerune_convert_f64_f32((-0.0), " node=30 bytes=243..252\n"), 1, " node=29 bytes=237..254\n");
    cerune_call_finish();
    return 0;
}
