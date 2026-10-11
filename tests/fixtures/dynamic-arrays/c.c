#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <assert.h>
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

static int64_t cerune_i64_sub(int64_t left, int64_t right, const char *origin) {
    if ((right < 0 && left > INT64_MAX + right) ||
        (right > 0 && left < INT64_MIN + right)) {
        cerune_runtime_fail("integer-overflow", origin);
    }
    return left - right;
}

typedef struct cerune_string {
    const unsigned char *data;
    size_t length;
} cerune_string;

static inline bool cerune_string_equal(cerune_string left, cerune_string right) {
    return left.length == right.length &&
        (left.length == 0 || memcmp(left.data, right.data, left.length) == 0);
}

static inline void cerune_print_string(cerune_string value) {
    fwrite(value.data, 1, value.length, stdout);
    fputc('\n', stdout);
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

typedef struct cerune_array_owner {
    void *data;
    uint64_t references;
    uint64_t bytes;
    int64_t initialized;
} cerune_array_owner;
typedef struct cerune_dynamic_array {
    cerune_array_owner *owner;
    int64_t length;
} cerune_dynamic_array;

static uint64_t cerune_array_live_bytes;
static cerune_dynamic_array cerune_array_allocate_elements(
    int64_t length, uint64_t width, size_t element_size, const char *origin) {
    if (length < 0) cerune_runtime_fail("array-length-out-of-range", origin);
    if ((width && (uint64_t)length > (uint64_t)INT64_MAX / width) ||
        (element_size && (uint64_t)length > (uint64_t)SIZE_MAX / element_size) ||
        (element_size && (uint64_t)length > (uint64_t)PTRDIFF_MAX / element_size))
        cerune_runtime_fail("allocation-size-overflow", origin);
    uint64_t bytes = (uint64_t)length * width;
    if (bytes > UINT64_C(67108864) - cerune_array_live_bytes)
        cerune_runtime_fail("allocation-limit-exceeded", origin);
    if (!length) return (cerune_dynamic_array){NULL, 0};
    cerune_array_owner *owner = malloc(sizeof(cerune_array_owner));
    if (!owner) cerune_runtime_fail("allocation-failed", origin);
    size_t physical_bytes = (size_t)length * element_size;
    owner->data = malloc(physical_bytes ? physical_bytes : 1);
    if (!owner->data) {
        free(owner);
        cerune_runtime_fail("allocation-failed", origin);
    }
    owner->references = 1;
    owner->bytes = bytes;
    owner->initialized = 0;
    cerune_array_live_bytes += bytes;
    return (cerune_dynamic_array){owner, length};
}
static void cerune_array_retain_owner(cerune_dynamic_array value) {
    if (!value.owner) return;
    assert(value.owner->references && value.owner->references < UINT64_MAX);
    ++value.owner->references;
}
static bool cerune_array_release_owner_last(cerune_dynamic_array value) {
    if (!value.owner) return true;
    assert(value.owner->references && value.owner->initialized == value.length);
    return --value.owner->references == 0;
}
static void cerune_array_free_elements(cerune_dynamic_array value) {
    if (!value.owner) return;
    assert(!value.owner->references && value.owner->bytes <= cerune_array_live_bytes);
    cerune_array_live_bytes -= value.owner->bytes;
    free(value.owner->data);
    free(value.owner);
}
static void cerune_array_check_range(int64_t length, int64_t start, int64_t end, const char *origin) {
    if (start < 0 || start > end || end > length)
        cerune_runtime_fail("array-range-out-of-bounds", origin);
}
static void cerune_write_escaped_byte(unsigned char value) {
    switch (value) {
    case 0:
        putchar(92);
        putchar(48);
        return;
    case 1:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(49);
        putchar(125);
        return;
    case 2:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(50);
        putchar(125);
        return;
    case 3:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(51);
        putchar(125);
        return;
    case 4:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(52);
        putchar(125);
        return;
    case 5:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(53);
        putchar(125);
        return;
    case 6:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(54);
        putchar(125);
        return;
    case 7:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(55);
        putchar(125);
        return;
    case 8:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(56);
        putchar(125);
        return;
    case 9:
        putchar(92);
        putchar(116);
        return;
    case 10:
        putchar(92);
        putchar(110);
        return;
    case 11:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(98);
        putchar(125);
        return;
    case 12:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(99);
        putchar(125);
        return;
    case 13:
        putchar(92);
        putchar(114);
        return;
    case 14:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(101);
        putchar(125);
        return;
    case 15:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(48);
        putchar(102);
        putchar(125);
        return;
    case 16:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(48);
        putchar(125);
        return;
    case 17:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(49);
        putchar(125);
        return;
    case 18:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(50);
        putchar(125);
        return;
    case 19:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(51);
        putchar(125);
        return;
    case 20:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(52);
        putchar(125);
        return;
    case 21:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(53);
        putchar(125);
        return;
    case 22:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(54);
        putchar(125);
        return;
    case 23:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(55);
        putchar(125);
        return;
    case 24:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(56);
        putchar(125);
        return;
    case 25:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(57);
        putchar(125);
        return;
    case 26:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(97);
        putchar(125);
        return;
    case 27:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(98);
        putchar(125);
        return;
    case 28:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(99);
        putchar(125);
        return;
    case 29:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(100);
        putchar(125);
        return;
    case 30:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(101);
        putchar(125);
        return;
    case 31:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(49);
        putchar(102);
        putchar(125);
        return;
    case 34:
        putchar(92);
        putchar(34);
        return;
    case 92:
        putchar(92);
        putchar(92);
        return;
    case 127:
        putchar(92);
        putchar(117);
        putchar(123);
        putchar(55);
        putchar(102);
        putchar(125);
        return;
    default: putchar(value); return;
    }
}
static void cerune_write_string(cerune_string value) {
    for (size_t i = 0; i < value.length; ++i) putchar(value.data[i]);
}
static void cerune_write_quoted(cerune_string value) {
    putchar(34);
    for (size_t i = 0; i < value.length; ++i) cerune_write_escaped_byte(value.data[i]);
    putchar(34);
}
static void cerune_write_i64(int64_t value) { printf("%lld", (long long)value); }
static void cerune_write_u64(uint64_t value) { printf("%llu", (unsigned long long)value); }
static void cerune_write_f32(float value) { printf("%.9g", (double)value); }
static void cerune_write_f64(double value) { printf("%.17g", (double)value); }
static void cerune_write_bool(bool value) { fputs(value ? "true" : "false", stdout); }

typedef struct cerune_array_i64_2 {
    int64_t items[2];
} cerune_array_i64_2;

static int64_t cerune_array_get_i64_2(cerune_array_i64_2 value, int64_t index, const char *origin) {
    if (index < 0 || index >= 2) {
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    }
    return value.items[index];
}

static int64_t cerune_dynamic_get_i64(cerune_dynamic_array value, int64_t index, const char *origin) {
    if (index < 0 || index >= value.length)
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    assert(value.owner && index < value.owner->initialized);
    return ((int64_t *)value.owner->data)[index];
}
static int64_t *cerune_dynamic_at_i64(cerune_dynamic_array *value, int64_t index, const char *origin) {
    if (index < 0 || index >= value->length)
        cerune_runtime_fail("array-index-out-of-bounds", origin);
    assert(value->owner && index < value->owner->initialized);
    return &((int64_t *)value->owner->data)[index];
}

void cerune_fn__display0_0(cerune_dynamic_array cerune_binding_2__value);
void cerune_fn__display1_1(cerune_dynamic_array cerune_binding_4__value);
int64_t cerune_fn__ownership2_2(void);
bool cerune_fn__ownership3_3(int64_t cerune_binding_8__owned8, cerune_dynamic_array cerune_binding_9__owned9);
int64_t cerune_fn__ownership4_4(int64_t cerune_binding_14__owned14);
bool cerune_fn__ownership5_5(int64_t cerune_binding_18__owned18);
int64_t cerune_fn__ownership6_6(void);
bool cerune_fn__ownership7_7(int64_t cerune_binding_31__owned31, cerune_dynamic_array cerune_binding_32__owned32);
int64_t cerune_fn__ownership8_8(int64_t cerune_binding_37__owned37);
bool cerune_fn__ownership9_9(int64_t cerune_binding_41__owned41);
cerune_dynamic_array cerune_fn__ownership10_10(void);
cerune_dynamic_array cerune_fn__ownership11_11(cerune_dynamic_array cerune_binding_64__owned64);
int64_t cerune_fn__ownership12_12(void);
int64_t cerune_fn__ownership13_13(void);
void cerune_fn__ownership14_14(cerune_dynamic_array cerune_binding_82__owned82);

void cerune_fn__display0_0(cerune_dynamic_array cerune_binding_2__value) {
    cerune_string cerune_binding_6__read6 = (cerune_string){ (const unsigned char *)"\133", 1 };
    cerune_write_string(cerune_binding_6__read6);
    int64_t cerune_binding_3__index = cerune_fn__ownership2_2();
    for (; cerune_fn__ownership3_3(cerune_binding_3__index, cerune_binding_2__value); cerune_binding_3__index = cerune_fn__ownership4_4(cerune_binding_3__index)) {
        if (cerune_fn__ownership5_5(cerune_binding_3__index)) {
            cerune_string cerune_binding_22__read22 = (cerune_string){ (const unsigned char *)"\054\040", 2 };
            cerune_write_string(cerune_binding_22__read22);
        }
        cerune_dynamic_array cerune_binding_23__read23 = cerune_binding_2__value;
        int64_t cerune_binding_24__read24 = cerune_binding_3__index;
        int64_t cerune_binding_25__owned25 = cerune_binding_24__read24;
        int64_t cerune_binding_26__read26 = cerune_dynamic_get_i64(cerune_binding_23__read23, cerune_binding_25__owned25, " node=30 bytes=78..92\n");
        cerune_write_i64(cerune_binding_26__read26);
    }
    cerune_string cerune_binding_27__read27 = (cerune_string){ (const unsigned char *)"\135", 1 };
    cerune_write_string(cerune_binding_27__read27);
    cerune_string cerune_binding_28__read28 = (cerune_string){ (const unsigned char *)"", 0 };
    cerune_print_string(cerune_binding_28__read28);
    return;
}

void cerune_fn__display1_1(cerune_dynamic_array cerune_binding_4__value) {
    cerune_string cerune_binding_29__read29 = (cerune_string){ (const unsigned char *)"\133", 1 };
    cerune_write_string(cerune_binding_29__read29);
    int64_t cerune_binding_5__index = cerune_fn__ownership6_6();
    for (; cerune_fn__ownership7_7(cerune_binding_5__index, cerune_binding_4__value); cerune_binding_5__index = cerune_fn__ownership8_8(cerune_binding_5__index)) {
        if (cerune_fn__ownership9_9(cerune_binding_5__index)) {
            cerune_string cerune_binding_45__read45 = (cerune_string){ (const unsigned char *)"\054\040", 2 };
            cerune_write_string(cerune_binding_45__read45);
        }
        cerune_dynamic_array cerune_binding_46__read46 = cerune_binding_4__value;
        int64_t cerune_binding_47__read47 = cerune_binding_5__index;
        int64_t cerune_binding_48__owned48 = cerune_binding_47__read47;
        int64_t cerune_binding_49__read49 = cerune_dynamic_get_i64(cerune_binding_46__read46, cerune_binding_48__owned48, " node=59 bytes=93..106\n");
        cerune_write_i64(cerune_binding_49__read49);
    }
    cerune_string cerune_binding_50__read50 = (cerune_string){ (const unsigned char *)"\135", 1 };
    cerune_write_string(cerune_binding_50__read50);
    cerune_string cerune_binding_51__read51 = (cerune_string){ (const unsigned char *)"", 0 };
    cerune_print_string(cerune_binding_51__read51);
    return;
}

int64_t cerune_fn__ownership2_2(void) {
    int64_t cerune_binding_7__owned7 = 0;
    return cerune_binding_7__owned7;
}

bool cerune_fn__ownership3_3(int64_t cerune_binding_8__owned8, cerune_dynamic_array cerune_binding_9__owned9) {
    int64_t cerune_binding_10__read10 = cerune_binding_8__owned8;
    cerune_dynamic_array cerune_binding_11__read11 = cerune_binding_9__owned9;
    int64_t cerune_binding_12__owned12 = (cerune_binding_11__read11).length;
    bool cerune_binding_13__owned13 = (cerune_binding_10__read10 < cerune_binding_12__owned12);
    return cerune_binding_13__owned13;
}

int64_t cerune_fn__ownership4_4(int64_t cerune_binding_14__owned14) {
    int64_t cerune_binding_15__read15 = cerune_binding_14__owned14;
    int64_t cerune_binding_16__read16 = 1;
    int64_t cerune_binding_17__owned17 = cerune_i64_add(cerune_binding_15__read15, cerune_binding_16__read16, " node=34 bytes=78..92\n");
    return cerune_binding_17__owned17;
}

bool cerune_fn__ownership5_5(int64_t cerune_binding_18__owned18) {
    int64_t cerune_binding_19__read19 = cerune_binding_18__owned18;
    int64_t cerune_binding_20__read20 = 0;
    bool cerune_binding_21__owned21 = (cerune_binding_19__read19 != cerune_binding_20__read20);
    return cerune_binding_21__owned21;
}

int64_t cerune_fn__ownership6_6(void) {
    int64_t cerune_binding_30__owned30 = 0;
    return cerune_binding_30__owned30;
}

bool cerune_fn__ownership7_7(int64_t cerune_binding_31__owned31, cerune_dynamic_array cerune_binding_32__owned32) {
    int64_t cerune_binding_33__read33 = cerune_binding_31__owned31;
    cerune_dynamic_array cerune_binding_34__read34 = cerune_binding_32__owned32;
    int64_t cerune_binding_35__owned35 = (cerune_binding_34__read34).length;
    bool cerune_binding_36__owned36 = (cerune_binding_33__read33 < cerune_binding_35__owned35);
    return cerune_binding_36__owned36;
}

int64_t cerune_fn__ownership8_8(int64_t cerune_binding_37__owned37) {
    int64_t cerune_binding_38__read38 = cerune_binding_37__owned37;
    int64_t cerune_binding_39__read39 = 1;
    int64_t cerune_binding_40__owned40 = cerune_i64_add(cerune_binding_38__read38, cerune_binding_39__read39, " node=63 bytes=93..106\n");
    return cerune_binding_40__owned40;
}

bool cerune_fn__ownership9_9(int64_t cerune_binding_41__owned41) {
    int64_t cerune_binding_42__read42 = cerune_binding_41__owned41;
    int64_t cerune_binding_43__read43 = 0;
    bool cerune_binding_44__owned44 = (cerune_binding_42__read42 != cerune_binding_43__read43);
    return cerune_binding_44__owned44;
}

cerune_dynamic_array cerune_fn__ownership10_10(void) {
    int64_t cerune_binding_52__owned52 = 1;
    int64_t cerune_binding_53__owned53 = 2;
    cerune_array_i64_2 cerune_binding_54__owned54 = (cerune_array_i64_2){ .items = { cerune_binding_52__owned52, cerune_binding_53__owned53 } };
    {
        int64_t length = 2;
        int64_t start = 0;
        int64_t end = 2;
        cerune_array_check_range(length, start, end, " node=219 bytes=20..38\n");
    }
    int64_t cerune_binding_55__owned55 = cerune_i64_sub(2, 0, " node=221 bytes=20..38\n");
    cerune_dynamic_array cerune_binding_56__owned56 = cerune_array_allocate_elements(cerune_binding_55__owned55, UINT64_C(8), sizeof(int64_t), " node=224 bytes=20..38\n");
    int64_t cerune_binding_57__owned57 = 0;
    for (; (cerune_binding_57__owned57 < cerune_binding_55__owned55); cerune_binding_57__owned57 = cerune_i64_add(cerune_binding_57__owned57, 1, " node=233 bytes=20..38\n")) {
        cerune_array_i64_2 cerune_binding_58__read58 = cerune_binding_54__owned54;
        int64_t cerune_binding_59__read59 = cerune_binding_57__owned57;
        int64_t cerune_binding_60__read60 = 0;
        int64_t cerune_binding_61__owned61 = cerune_i64_add(cerune_binding_59__read59, cerune_binding_60__read60, " node=237 bytes=20..38\n");
        int64_t cerune_binding_62__read62 = cerune_array_get_i64_2(cerune_binding_58__read58, cerune_binding_61__owned61, " node=239 bytes=20..38\n");
        int64_t cerune_binding_63__owned63 = cerune_binding_62__read62;
        {
            cerune_dynamic_array array = cerune_binding_56__owned56;
            int64_t value = cerune_binding_63__owned63;
            assert(array.owner && array.owner->references && array.owner->initialized < array.length);
            ((int64_t *)array.owner->data)[array.owner->initialized++] = value;
        }
    }
    return cerune_binding_56__owned56;
}

cerune_dynamic_array cerune_fn__ownership11_11(cerune_dynamic_array cerune_binding_64__owned64) {
    cerune_dynamic_array cerune_binding_65__read65 = cerune_binding_64__owned64;
    cerune_dynamic_array cerune_binding_66__read66 = cerune_binding_65__read65;
    {
        int64_t length = (cerune_binding_66__read66).length;
        int64_t start = 0;
        int64_t end = (cerune_binding_66__read66).length;
        cerune_array_check_range(length, start, end, " node=275 bytes=55..61\n");
    }
    int64_t cerune_binding_67__owned67 = cerune_i64_sub((cerune_binding_66__read66).length, 0, " node=277 bytes=55..61\n");
    cerune_dynamic_array cerune_binding_68__owned68 = cerune_array_allocate_elements(cerune_binding_67__owned67, UINT64_C(8), sizeof(int64_t), " node=280 bytes=55..61\n");
    int64_t cerune_binding_69__owned69 = 0;
    for (; (cerune_binding_69__owned69 < cerune_binding_67__owned67); cerune_binding_69__owned69 = cerune_i64_add(cerune_binding_69__owned69, 1, " node=289 bytes=55..61\n")) {
        cerune_dynamic_array cerune_binding_70__read70 = cerune_binding_66__read66;
        int64_t cerune_binding_71__read71 = cerune_binding_69__owned69;
        int64_t cerune_binding_72__read72 = 0;
        int64_t cerune_binding_73__owned73 = cerune_i64_add(cerune_binding_71__read71, cerune_binding_72__read72, " node=293 bytes=55..61\n");
        int64_t cerune_binding_74__read74 = cerune_dynamic_get_i64(cerune_binding_70__read70, cerune_binding_73__owned73, " node=295 bytes=55..61\n");
        int64_t cerune_binding_75__owned75 = cerune_binding_74__read74;
        {
            cerune_dynamic_array array = cerune_binding_68__owned68;
            int64_t value = cerune_binding_75__owned75;
            assert(array.owner && array.owner->references && array.owner->initialized < array.length);
            ((int64_t *)array.owner->data)[array.owner->initialized++] = value;
        }
    }
    return cerune_binding_68__owned68;
}

int64_t cerune_fn__ownership12_12(void) {
    int64_t cerune_binding_76__owned76 = 0;
    return cerune_binding_76__owned76;
}

int64_t cerune_fn__ownership13_13(void) {
    int64_t cerune_binding_79__owned79 = 9;
    return cerune_binding_79__owned79;
}

void cerune_fn__ownership14_14(cerune_dynamic_array cerune_binding_82__owned82) {
    if (cerune_array_release_owner_last(cerune_binding_82__owned82)) {
        int64_t cerune_binding_83__owned83 = (cerune_binding_82__owned82).length;
        for (; (cerune_binding_83__owned83 > 0); cerune_binding_83__owned83 = cerune_i64_sub(cerune_binding_83__owned83, 1, " node=354 bytes=40..62\n")) {
        }
        cerune_array_free_elements(cerune_binding_82__owned82);
    }
    return;
}

int main(void) {
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        fputs("cerune: cannot set stdout to binary mode\n", stderr);
        return 1;
    }
#endif
    cerune_dynamic_array cerune_binding_0_values = cerune_fn__ownership10_10();
    cerune_dynamic_array cerune_binding_1_saved = cerune_fn__ownership11_11(cerune_binding_0_values);
    int64_t cerune_binding_77__owned77 = cerune_fn__ownership12_12();
    int64_t cerune_binding_78__owned78 = cerune_dynamic_get_i64(cerune_binding_0_values, cerune_binding_77__owned77, " node=330 bytes=69..72\n");
    {
        int64_t *cerune_assignment_target_0 = cerune_dynamic_at_i64(&cerune_binding_0_values, cerune_binding_77__owned77, " node=7 bytes=69..72\n");
        int64_t cerune_assignment_value = cerune_fn__ownership13_13();
        *cerune_assignment_target_0 = cerune_assignment_value;
    }
    cerune_dynamic_array cerune_binding_80__read80 = cerune_binding_0_values;
    cerune_fn__display0_0(cerune_binding_80__read80);
    cerune_dynamic_array cerune_binding_81__read81 = cerune_binding_1_saved;
    cerune_fn__display1_1(cerune_binding_81__read81);
    cerune_fn__ownership14_14(cerune_binding_1_saved);
    cerune_fn__ownership14_14(cerune_binding_0_values);
    return 0;
}
