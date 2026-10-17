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

static int64_t cerune_saturating_trunc_f64_u8(double value, const char *origin) {
    /* policy: saturating_trunc */
    double number = (double)value;
    (void)origin;
    if (isnan(number)) return 0;
    if (isinf(number)) return number > 0.0 ? INT64_C(255) : INT64_C(0);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < 0.0) return INT64_C(0);
    if (number >= 256.0) return INT64_C(255);
    return (int64_t)number;
}

static int64_t cerune_floor_f64_i64(double value, const char *origin) {
    /* policy: floor */
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((0) ? 1 : 0) - ((fraction < 0.0) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-out-of-range", origin);
    return (int64_t)number;
}

static int64_t cerune_saturating_floor_f64_u8(double value, const char *origin) {
    /* policy: saturating_floor */
    double number = (double)value;
    (void)origin;
    if (isnan(number)) return 0;
    if (isinf(number)) return number > 0.0 ? INT64_C(255) : INT64_C(0);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((0) ? 1 : 0) - ((fraction < 0.0) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < 0.0) return INT64_C(0);
    if (number >= 256.0) return INT64_C(255);
    return (int64_t)number;
}

static int64_t cerune_ceil_f64_i64(double value, const char *origin) {
    /* policy: ceil */
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction > 0.0) ? 1 : 0) - ((0) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-out-of-range", origin);
    return (int64_t)number;
}

static int64_t cerune_saturating_ceil_f64_u8(double value, const char *origin) {
    /* policy: saturating_ceil */
    double number = (double)value;
    (void)origin;
    if (isnan(number)) return 0;
    if (isinf(number)) return number > 0.0 ? INT64_C(255) : INT64_C(0);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction > 0.0) ? 1 : 0) - ((0) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < 0.0) return INT64_C(0);
    if (number >= 256.0) return INT64_C(255);
    return (int64_t)number;
}

static int64_t cerune_round_f64_i64(double value, const char *origin) {
    /* policy: round */
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction >= 0.5) ? 1 : 0) - ((fraction <= -0.5) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-out-of-range", origin);
    return (int64_t)number;
}

static int64_t cerune_saturating_round_f64_u8(double value, const char *origin) {
    /* policy: saturating_round */
    double number = (double)value;
    (void)origin;
    if (isnan(number)) return 0;
    if (isinf(number)) return number > 0.0 ? INT64_C(255) : INT64_C(0);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction >= 0.5) ? 1 : 0) - ((fraction <= -0.5) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < 0.0) return INT64_C(0);
    if (number >= 256.0) return INT64_C(255);
    return (int64_t)number;
}

static int64_t cerune_round_ties_even_f64_i64(double value, const char *origin) {
    /* policy: round_ties_even */
    double number = (double)value;
    if (!isfinite(number)) cerune_runtime_fail("conversion-not-finite", origin);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction > 0.5 || (fraction == 0.5 && whole % 2 != 0)) ? 1 : 0) - ((fraction < -0.5 || (fraction == -0.5 && whole % 2 != 0)) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < -9223372036854775808.0 || number >= 9223372036854775808.0) cerune_runtime_fail("conversion-out-of-range", origin);
    return (int64_t)number;
}

static int64_t cerune_saturating_round_ties_even_f64_u8(double value, const char *origin) {
    /* policy: saturating_round_ties_even */
    double number = (double)value;
    (void)origin;
    if (isnan(number)) return 0;
    if (isinf(number)) return number > 0.0 ? INT64_C(255) : INT64_C(0);
    /* round finite input; values at or beyond 2^52 are integral */
    if (number > -4503599627370496.0 && number < 4503599627370496.0) {
        int64_t whole = (int64_t)number;
        double fraction = number - (double)whole;
        int64_t delta = ((fraction > 0.5 || (fraction == 0.5 && whole % 2 != 0)) ? 1 : 0) - ((fraction < -0.5 || (fraction == -0.5 && whole % 2 != 0)) ? 1 : 0);
        whole += delta;
        number = (double)whole;
    }
    /* apply range policy after rounding, then convert */
    if (number < 0.0) return INT64_C(0);
    if (number >= 256.0) return INT64_C(255);
    return (int64_t)number;
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
double cerune_fn_value_0(double cerune_binding_0_x);

double cerune_fn_value_0(double cerune_binding_0_x) {
    return cerune_binding_0_x;
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_output_i64(cerune_floor_f64_i64(cerune_fn_value_0((-2.5)), " node=7 bytes=79..102\n"), 1, " node=6 bytes=73..104\n");
    cerune_output_i64(cerune_ceil_f64_i64(cerune_fn_value_0((-2.5)), " node=12 bytes=111..133\n"), 1, " node=11 bytes=105..135\n");
    cerune_output_i64(cerune_round_f64_i64(cerune_fn_value_0(2.5), " node=17 bytes=142..164\n"), 1, " node=16 bytes=136..166\n");
    cerune_output_i64(cerune_round_ties_even_f64_i64(cerune_fn_value_0(2.5), " node=21 bytes=173..205\n"), 1, " node=20 bytes=167..207\n");
    cerune_output_i64(cerune_saturating_trunc_f64_u8(cerune_fn_value_0(300.0), " node=25 bytes=214..248\n"), 1, " node=24 bytes=208..250\n");
    cerune_output_i64(cerune_saturating_floor_f64_u8(cerune_fn_value_0((-0.1)), " node=29 bytes=257..290\n"), 1, " node=28 bytes=251..292\n");
    cerune_output_i64(cerune_saturating_ceil_f64_u8(cerune_fn_value_0(255.1), " node=34 bytes=299..332\n"), 1, " node=33 bytes=293..334\n");
    cerune_output_i64(cerune_saturating_round_f64_u8(cerune_fn_value_0(255.5), " node=38 bytes=341..375\n"), 1, " node=37 bytes=335..377\n");
    cerune_output_i64(cerune_saturating_round_ties_even_f64_u8(cerune_fn_value_0(254.5), " node=42 bytes=384..428\n"), 1, " node=41 bytes=378..430\n");
    cerune_output_i64(3, 1, " node=45 bytes=431..444\n");
    return 0;
}
