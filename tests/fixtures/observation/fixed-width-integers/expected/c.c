#include <stdlib.h>
#include <stddef.h>
#include <stdbool.h>
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

static int64_t cerune_check_i32(int64_t value, const char *code, const char *origin) {
    if (value < -2147483648LL || value > 2147483647LL) cerune_runtime_fail(code, origin);
    return value;
}

static int64_t cerune_check_u32(int64_t value, const char *code, const char *origin) {
    if (value < 0LL || value > 4294967295LL) cerune_runtime_fail(code, origin);
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
int64_t cerune_fn_add_0(int64_t cerune_binding_0_left, int64_t cerune_binding_1_right);

int64_t cerune_fn_add_0(int64_t cerune_binding_0_left, int64_t cerune_binding_1_right) {
    return cerune_check_i32(cerune_i64_add(cerune_binding_0_left, cerune_binding_1_right, " node=1 bytes=50..62\n"), "integer-overflow", " node=1 bytes=50..62\n");
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    int64_t _cerune_eval_0;
    int64_t _cerune_eval_1;
    int64_t _cerune_eval_2;
    int64_t cerune_binding_2_small = (_cerune_eval_0 = cerune_check_i32(cerune_i64_neg(3, " node=6 bytes=83..85\n"), "integer-overflow", " node=6 bytes=83..85\n"), _cerune_eval_1 = 5, (cerune_call_enter(0ULL, " node=5 bytes=79..89\n"), _cerune_eval_2 = cerune_fn_add_0(_cerune_eval_0, _cerune_eval_1), cerune_call_leave(), _cerune_eval_2));
    int64_t cerune_binding_3_large = 4294967295;
    cerune_output_i64(cerune_binding_2_small, 1, " node=11 bytes=116..129\n");
    cerune_output_i64(cerune_check_u32(cerune_i64_div(cerune_binding_3_large, 2, " node=14 bytes=136..145\n"), "division-overflow", " node=14 bytes=136..145\n"), 1, " node=13 bytes=130..147\n");
    cerune_output_i64(cerune_binding_3_large, 1, " node=17 bytes=148..166\n");
    cerune_output_bool((cerune_binding_3_large > 2147483648), 1, " node=20 bytes=167..193\n");
    cerune_output_i64(cerune_check_u32(cerune_binding_2_small, "integer-conversion-out-of-range", " node=25 bytes=200..219\n"), 1, " node=24 bytes=194..221\n");
    cerune_call_finish();
    return 0;
}
