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

static void cerune_runtime_fail(const char *code, const char *origin) {
    fflush(stdout);
    fputs("cerune: runtime-v1 code=", stderr);
    fputs(code, stderr);
    fputs(origin, stderr);
    fflush(stderr);
    abort();
}

static int64_t cerune_i64_add(int64_t left, int64_t right, const char *origin) {
    if ((right > 0 && left > INT64_MAX - right) ||
        (right < 0 && left < INT64_MIN - right)) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return left + right;
}

static int64_t cerune_i64_neg(int64_t value, const char *origin) {
    if (value == INT64_MIN) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return -value;
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
typedef struct cerune_array_i64_2 {
    int64_t items[2];
} cerune_array_i64_2;

static int64_t cerune_array_get_i64_2(cerune_array_i64_2 value, int64_t index, const char *origin) {
    if (index < 0 || index >= 2) {
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    }
    return value.items[index];
}

typedef struct cerune_type_Item_0 {
    int64_t _dtag;
    int64_t _dValue_dn;
} cerune_type_Item_0;

bool cerune_fn__equal0_0(cerune_array_i64_2 cerune_binding_5__left, cerune_array_i64_2 cerune_binding_6__right);
bool cerune_fn__match_bind1_1(cerune_type_Item_0 cerune_binding_8__capture1);
int64_t cerune_fn__match_bind2_2(cerune_type_Item_0 cerune_binding_9__capture1);
int64_t cerune_fn__match_select3_3(cerune_type_Item_0 cerune_binding_10__capture1);
int64_t cerune_fn__match_select4_4(cerune_type_Item_0 cerune_binding_11__capture1);
int64_t cerune_fn__match_bind5_5(void);
void cerune_fn__display6_6(cerune_type_Item_0 cerune_binding_12__value);

bool cerune_fn__equal0_0(cerune_array_i64_2 cerune_binding_5__left, cerune_array_i64_2 cerune_binding_6__right) {
    int64_t _cerune_eval_0;
    int64_t _cerune_eval_1;
    for (int64_t cerune_binding_7__index = 0; (cerune_binding_7__index < 2); cerune_binding_7__index = cerune_i64_add(cerune_binding_7__index, 1, " node=63 bytes=70..84\n")) {
        if (!(_cerune_eval_0 = cerune_array_get_i64_2(cerune_binding_5__left, cerune_binding_7__index, " node=52 bytes=70..84\n"), _cerune_eval_1 = cerune_array_get_i64_2(cerune_binding_6__right, cerune_binding_7__index, " node=55 bytes=70..84\n"), (_cerune_eval_0 == _cerune_eval_1))) {
            return false;
        }
    }
    return true;
}

bool cerune_fn__match_bind1_1(cerune_type_Item_0 cerune_binding_8__capture1) {
    int64_t cerune_binding_2_n = (cerune_binding_8__capture1)._dValue_dn;
    return (cerune_binding_2_n > 0);
}

int64_t cerune_fn__match_bind2_2(cerune_type_Item_0 cerune_binding_9__capture1) {
    int64_t cerune_binding_3_n = (cerune_binding_9__capture1)._dValue_dn;
    return cerune_binding_3_n;
}

int64_t cerune_fn__match_select3_3(cerune_type_Item_0 cerune_binding_10__capture1) {
    if ((cerune_binding_10__capture1)._dtag == 0) {
        return 0;
    } else {
        return cerune_i64_neg(1, " node=41 bytes=254..256\n");
    }
}

int64_t cerune_fn__match_select4_4(cerune_type_Item_0 cerune_binding_11__capture1) {
    if ((((cerune_binding_11__capture1)._dtag == 0) && cerune_fn__match_bind1_1(cerune_binding_11__capture1))) {
        return cerune_fn__match_bind2_2(cerune_binding_11__capture1);
    } else {
        return cerune_fn__match_select3_3(cerune_binding_11__capture1);
    }
}

int64_t cerune_fn__match_bind5_5(void) {
    cerune_type_Item_0 cerune_binding_1__match0 = (cerune_type_Item_0){ ._dtag = 0, ._dValue_dn = 7 };
    return cerune_fn__match_select4_4(cerune_binding_1__match0);
}

void cerune_fn__display6_6(cerune_type_Item_0 cerune_binding_12__value) {
    if ((cerune_binding_12__value)._dtag == 0) {
        cerune_output_string((cerune_string){ (const unsigned char *)"\126\141\154\165\145", 5 }, 0, " node=95 bytes=87..115\n");
        cerune_output_string((cerune_string){ (const unsigned char *)"\173", 1 }, 0, " node=97 bytes=87..115\n");
        cerune_output_string((cerune_string){ (const unsigned char *)"\156\072\040", 3 }, 0, " node=99 bytes=87..115\n");
        cerune_output_i64((cerune_binding_12__value)._dValue_dn, 0, " node=102 bytes=87..115\n");
        cerune_output_string((cerune_string){ (const unsigned char *)"\175", 1 }, 0, " node=104 bytes=87..115\n");
    }
    if ((cerune_binding_12__value)._dtag == 1) {
        cerune_output_string((cerune_string){ (const unsigned char *)"\105\155\160\164\171", 5 }, 0, " node=111 bytes=87..115\n");
        cerune_output_string((cerune_string){ (const unsigned char *)"\173", 1 }, 0, " node=113 bytes=87..115\n");
        cerune_output_string((cerune_string){ (const unsigned char *)"\175", 1 }, 0, " node=115 bytes=87..115\n");
    }
    cerune_output_string((cerune_string){ (const unsigned char *)"", 0 }, 1, " node=118 bytes=87..115\n");
    return;
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_array_i64_2 cerune_binding_0_left = (cerune_array_i64_2){ .items = { 1, 2 } };
    cerune_output_bool(cerune_fn__equal0_0(cerune_binding_0_left, (cerune_array_i64_2){ .items = { 1, 3 } }), 1, " node=4 bytes=64..86\n");
    cerune_fn__display6_6((cerune_type_Item_0){ ._dtag = 0, ._dValue_dn = 7 });
    int64_t cerune_binding_4_result = cerune_fn__match_bind5_5();
    cerune_output_i64(cerune_binding_4_result, 1, " node=43 bytes=261..275\n");
    return 0;
}
