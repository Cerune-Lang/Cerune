#include <errno.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <wchar.h>
typedef wchar_t cerune_trace_char;
#define CERUNE_TRACE_TEXT(s) L##s
#define cerune_trace_cmp wcscmp
#else
typedef char cerune_trace_char;
#define CERUNE_TRACE_TEXT(s) s
#define cerune_trace_cmp strcmp
#endif

#define CERUNE_TRACE_END(status) "{\"schema\":\"trace-v1\",\"event\":\"end\",\"status\":\"" status "\"}\n"
#define CERUNE_TRACE_ROW "{\"schema\":\"trace-v1\",\"seq\":"
static const cerune_trace_char *cerune_trace_path;
static size_t cerune_trace_events = 4096, cerune_trace_bytes = 1048576;
static const char **cerune_trace_rows;
static size_t cerune_trace_count, cerune_trace_capacity;
static const char *cerune_trace_end = CERUNE_TRACE_END("complete");
static int cerune_trace_incomplete;

static void cerune_trace_record(const char *row) {
    if (!cerune_trace_path || cerune_trace_incomplete) return;
    if (cerune_trace_count == cerune_trace_events) {
        cerune_trace_end = CERUNE_TRACE_END("event-limit");
        cerune_trace_incomplete = 1;
        return;
    }
    if (cerune_trace_count == cerune_trace_capacity) {
        size_t next = cerune_trace_capacity > SIZE_MAX / 2 ? SIZE_MAX :
            (cerune_trace_capacity ? 2 * cerune_trace_capacity : 16);
        if (next > cerune_trace_events) next = cerune_trace_events;
        int saved_errno = errno;
        const char **rows = next <= SIZE_MAX / sizeof(*rows) ?
            realloc(cerune_trace_rows, next * sizeof(*rows)) : NULL;
        errno = saved_errno;
        if (!rows) {
            cerune_trace_end = CERUNE_TRACE_END("allocation-failed");
            cerune_trace_incomplete = 1;
            return;
        }
        cerune_trace_rows = rows;
        cerune_trace_capacity = next;
    }
    cerune_trace_rows[cerune_trace_count++] = row;
}

static int cerune_trace_branch(int taken, const char *yes, const char *no) {
    cerune_trace_record(taken ? yes : no);
    return taken;
}

static void cerune_trace_save(void) {
    if (!cerune_trace_path) return;
#ifdef _WIN32
    FILE *f = _wfopen(cerune_trace_path, L"wbx");
#else
    FILE *f = fopen(cerune_trace_path, "wbx");
#endif
    cerune_trace_path = NULL;
    int failed = !f;
    if (f) {
        size_t remaining = cerune_trace_bytes;
        const char *end = cerune_trace_end;
        for (size_t i = 0; i < cerune_trace_count; ++i) {
            char prefix[64];
            int n = snprintf(prefix, sizeof(prefix), CERUNE_TRACE_ROW "%zu", i + 1);
            if (n < 0 || (size_t)n >= sizeof(prefix)) { failed = 1; break; }
            size_t suffix = strlen(cerune_trace_rows[i]);
            size_t reserve = strlen(i + 1 == cerune_trace_count ? end : CERUNE_TRACE_END("byte-limit"));
            if (remaining < reserve || (size_t)n > remaining - reserve ||
                suffix > remaining - reserve - (size_t)n) {
                end = CERUNE_TRACE_END("byte-limit");
                break;
            }
            if (fputs(prefix, f) == EOF || fputs(cerune_trace_rows[i], f) == EOF) { failed = 1; break; }
            remaining -= (size_t)n + suffix;
        }
        if (!failed && fputs(end, f) == EOF) failed = 1;
        if (fclose(f) != 0) failed = 1;
    }
    free(cerune_trace_rows);
    if (failed) fputs("cerune: trace-v1: could not save trace file\n", stderr);
}

/* runtime-v1 passes static code and origin strings, including file= for imported sources. */
static void cerune_trace_stop(const char *code, const char *origin) {
    if (cerune_trace_path) {
        size_t node, file = 0, start, end;
        if (sscanf(origin, " node=%zu file=%zu bytes=%zu..%zu", &node, &file, &start, &end) == 4 ||
            sscanf(origin, " node=%zu bytes=%zu..%zu", &node, &start, &end) == 3) {
            static char row[256];
            int n = snprintf(row, sizeof(row), ",\"event\":\"stop\",\"code\":\"%s\",\"node\":%zu,\"file\":%zu,\"start\":%zu,\"end\":%zu}\n",
                code, node, file, start, end);
            if (n > 0 && (size_t)n < sizeof(row)) cerune_trace_record(row);
        }
        cerune_trace_save();
    }
}
static void cerune_trace_exit(void) {
    cerune_trace_save();
    _Exit(1);
}
