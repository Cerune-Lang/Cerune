#include <stdlib.h>
#include <stddef.h>
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

static int64_t cerune_i64_add(int64_t left, int64_t right, const char *origin) {
    if ((right > 0 && left > INT64_MAX - right) ||
        (right < 0 && left < INT64_MIN - right)) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return left + right;
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
typedef struct cerune_array_i64_2 {
    int64_t items[2];
} cerune_array_i64_2;

static int64_t cerune_array_get_i64_2(cerune_array_i64_2 value, int64_t index, const char *origin) {
    if (index < 0 || index >= 2) {
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    }
    return value.items[index];
}

typedef struct cerune_array_array_i64_2_2 {
    cerune_array_i64_2 items[2];
} cerune_array_array_i64_2_2;

static cerune_array_i64_2 cerune_array_get_array_i64_2_2(cerune_array_array_i64_2_2 value, int64_t index, const char *origin) {
    if (index < 0 || index >= 2) {
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    }
    return value.items[index];
}

typedef struct cerune_type_Point_0 {
    int64_t x;
    int64_t y;
} cerune_type_Point_0;

cerune_type_Point_0 cerune_fn_move_x_0(cerune_type_Point_0 cerune_binding_0_point, int64_t cerune_binding_1_amount);
cerune_type_Point_0 cerune_fn_move_twice_1(cerune_type_Point_0 cerune_binding_2_point, int64_t cerune_binding_3_amount);
cerune_array_i64_2 cerune_fn_first_row_2(cerune_array_array_i64_2_2 cerune_binding_4_matrix);
cerune_array_array_i64_2_2 cerune_fn_duplicate_3(cerune_array_i64_2 cerune_binding_5_row);
cerune_array_array_i64_2_2 cerune_fn_duplicate_first_row_4(cerune_array_array_i64_2_2 cerune_binding_6_matrix);

cerune_type_Point_0 cerune_fn_move_x_0(cerune_type_Point_0 cerune_binding_0_point, int64_t cerune_binding_1_amount) {
    return (cerune_type_Point_0){ .x = cerune_i64_add((cerune_binding_0_point).x, cerune_binding_1_amount, " node=2 bytes=118..134\n"), .y = (cerune_binding_0_point).y };
}

cerune_type_Point_0 cerune_fn_move_twice_1(cerune_type_Point_0 cerune_binding_2_point, int64_t cerune_binding_3_amount) {
    cerune_type_Point_0 _cerune_eval_0;
    int64_t _cerune_eval_1;
    cerune_type_Point_0 _cerune_eval_2;
    cerune_type_Point_0 _cerune_eval_3;
    int64_t _cerune_eval_4;
    cerune_type_Point_0 _cerune_eval_5;
    return (_cerune_eval_3 = (_cerune_eval_0 = cerune_binding_2_point, _cerune_eval_1 = cerune_binding_3_amount, (cerune_call_enter(0ULL, " node=10 bytes=236..257\n"), _cerune_eval_2 = cerune_fn_move_x_0(_cerune_eval_0, _cerune_eval_1), cerune_call_leave(), _cerune_eval_2)), _cerune_eval_4 = cerune_binding_3_amount, (cerune_call_enter(0ULL, " node=9 bytes=229..266\n"), _cerune_eval_5 = cerune_fn_move_x_0(_cerune_eval_3, _cerune_eval_4), cerune_call_leave(), _cerune_eval_5));
}

cerune_array_i64_2 cerune_fn_first_row_2(cerune_array_array_i64_2_2 cerune_binding_4_matrix) {
    return cerune_array_get_array_i64_2_2(cerune_binding_4_matrix, 0, " node=15 bytes=332..341\n");
}

cerune_array_array_i64_2_2 cerune_fn_duplicate_3(cerune_array_i64_2 cerune_binding_5_row) {
    return (cerune_array_array_i64_2_2){ .items = { cerune_binding_5_row, cerune_binding_5_row } };
}

cerune_array_array_i64_2_2 cerune_fn_duplicate_first_row_4(cerune_array_array_i64_2_2 cerune_binding_6_matrix) {
    cerune_array_array_i64_2_2 _cerune_eval_0;
    cerune_array_i64_2 _cerune_eval_1;
    cerune_array_i64_2 _cerune_eval_2;
    cerune_array_array_i64_2_2 _cerune_eval_3;
    return (_cerune_eval_2 = (_cerune_eval_0 = cerune_binding_6_matrix, (cerune_call_enter(2ULL, " node=24 bytes=505..522\n"), _cerune_eval_1 = cerune_fn_first_row_2(_cerune_eval_0), cerune_call_leave(), _cerune_eval_1)), (cerune_call_enter(3ULL, " node=23 bytes=495..523\n"), _cerune_eval_3 = cerune_fn_duplicate_3(_cerune_eval_2), cerune_call_leave(), _cerune_eval_3));
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_type_Point_0 _cerune_eval_0;
    int64_t _cerune_eval_1;
    cerune_type_Point_0 _cerune_eval_2;
    cerune_array_array_i64_2_2 _cerune_eval_3;
    cerune_array_array_i64_2_2 _cerune_eval_4;
    cerune_type_Point_0 cerune_binding_7_original = (cerune_type_Point_0){ .x = 2, .y = 3 };
    cerune_type_Point_0 cerune_binding_8_moved = (_cerune_eval_0 = cerune_binding_7_original, _cerune_eval_1 = 5, (cerune_call_enter(1ULL, " node=31 bytes=584..607\n"), _cerune_eval_2 = cerune_fn_move_twice_1(_cerune_eval_0, _cerune_eval_1), cerune_call_leave(), _cerune_eval_2));
    cerune_array_array_i64_2_2 cerune_binding_9_matrix = (cerune_array_array_i64_2_2){ .items = { (cerune_array_i64_2){ .items = { 1, 2 } }, (cerune_array_i64_2){ .items = { 3, 4 } } } };
    cerune_array_array_i64_2_2 cerune_binding_10_rows = (_cerune_eval_3 = cerune_binding_9_matrix, (cerune_call_enter(4ULL, " node=43 bytes=673..700\n"), _cerune_eval_4 = cerune_fn_duplicate_first_row_4(_cerune_eval_3), cerune_call_leave(), _cerune_eval_4));
    cerune_output_i64((cerune_binding_7_original).x, 1, " node=45 bytes=703..721\n");
    cerune_output_i64((cerune_binding_8_moved).x, 1, " node=48 bytes=722..737\n");
    cerune_output_i64((cerune_binding_8_moved).y, 1, " node=51 bytes=738..753\n");
    cerune_output_i64(cerune_array_get_i64_2(cerune_array_get_array_i64_2_2(cerune_binding_9_matrix, 1, " node=56 bytes=760..769\n"), 0, " node=55 bytes=760..772\n"), 1, " node=54 bytes=754..774\n");
    cerune_output_i64(cerune_array_get_i64_2(cerune_array_get_array_i64_2_2(cerune_binding_10_rows, 0, " node=62 bytes=781..788\n"), 1, " node=61 bytes=781..791\n"), 1, " node=60 bytes=775..793\n");
    cerune_output_i64(cerune_array_get_i64_2(cerune_array_get_array_i64_2_2(cerune_binding_10_rows, 1, " node=68 bytes=800..807\n"), 0, " node=67 bytes=800..810\n"), 1, " node=66 bytes=794..812\n");
    cerune_call_finish();
    return 0;
}
