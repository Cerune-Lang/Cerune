/* Measurement-only wrappers; never linked into ordinary generated programs. */
#define PSAPI_VERSION 2
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#include <psapi.h>
#endif

static void *probe_pointers[16];
static size_t probe_sizes[16];
static size_t probe_live, probe_count;
static void *probe_malloc(size_t size) {
    void *p = malloc(size);
    if (!p) return NULL;
    for (size_t i = 0; i < 16; ++i) {
        if (!probe_pointers[i]) {
            probe_pointers[i] = p;
            probe_sizes[i] = size;
            probe_live += size;
            ++probe_count;
            return p;
        }
    }
    abort();
}
static void probe_free(void *p) {
    if (!p) return;
    for (size_t i = 0; i < 16; ++i) {
        if (probe_pointers[i] == p) {
            probe_live -= probe_sizes[i];
            --probe_count;
            probe_pointers[i] = NULL;
            free(p);
            return;
        }
    }
    abort();
}
static void *probe_realloc(void *p, size_t size) {
    for (size_t i = 0; i < 16; ++i) {
        if (probe_pointers[i] == p) {
            void *next = realloc(p, size);
            if (!next) return NULL;
            probe_live -= probe_sizes[i] - size;
            probe_sizes[i] = size;
            probe_pointers[i] = next;
            return next;
        }
    }
    abort();
}
/* GENERATED_RUNTIME */

static void probe_snapshot(const char *phase, int cycle) {
    size_t owners = 0;
    for (cerune_string_owner *p = cerune_string_owners; p; p = p->next) ++owners;
    assert(owners == probe_count);
    printf("cycle=%d phase=%s logical=%llu owners=%zu requested=%zu ",
        cycle, phase, (unsigned long long)cerune_string_live_bytes, owners, probe_live);
#ifdef _WIN32
    PROCESS_MEMORY_COUNTERS_EX counters = {0};
    counters.cb = sizeof(counters);
    if (!GetProcessMemoryInfo(GetCurrentProcess(),
            (PROCESS_MEMORY_COUNTERS *)&counters, sizeof(counters))) abort();
    printf("working_set_bytes=%zu peak_working_set_bytes=%zu private_commit_bytes=%zu",
        counters.WorkingSetSize, counters.PeakWorkingSetSize, counters.PrivateUsage);
#elif defined(__linux__)
    FILE *status = fopen("/proc/self/status", "r");
    if (!status) abort();
    char line[256];
    while (fgets(line, sizeof(line), status)) {
        if (!strncmp(line, "VmSize:", 7) || !strncmp(line, "VmRSS:", 6) ||
            !strncmp(line, "VmHWM:", 6) || !strncmp(line, "VmData:", 7)) {
            line[strcspn(line, "\n")] = 0;
            printf("%s ", line);
        }
    }
    fclose(status);
#else
    printf("os_metrics=unavailable");
#endif
    putchar('\n');
}
int main(void) {
    const int64_t limit = @LIMIT@;
    probe_snapshot("before", 0);
    for (int cycle = 0; cycle < 3; ++cycle) {
        cerune_string values[8];
        for (size_t i = 0; i < 8; ++i) {
            values[i] = cerune_input_read_line(limit, " node=0 bytes=0..0");
            assert(values[i].length == 2);
            assert(memcmp(values[i].data, "x\n", 2) == 0);
        }
        assert(cerune_string_live_bytes == 16);
        assert(probe_live == 8 * (2 + sizeof(cerune_string_owner)));
        probe_snapshot("retained", cycle);
        for (size_t i = 0; i < 8; ++i) cerune_string_release(values[i]);
        assert(cerune_string_live_bytes == 0);
        assert(probe_live == 0 && probe_count == 0 && !cerune_string_owners);
        probe_snapshot("released", cycle);
    }
    return 0;
}
