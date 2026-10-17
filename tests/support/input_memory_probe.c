/* Allocator probe linked only into instrumented test objects. */
#define PSAPI_VERSION 2
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#include <psapi.h>

#endif
static void *pointers[32];
static size_t sizes[32], live, peak, count, peak_count, allocations, releases;
static void report(void) {
    fprintf(stderr, "probe allocations=%zu releases=%zu live=%zu peak=%zu owners=%zu peak_owners=%zu\n",
        allocations, releases, live, peak, count, peak_count);
}
void *probe_malloc(size_t size) {
    if (!allocations && atexit(report)) abort();
    void *p = malloc(size);
    if (!p) return NULL;
    for (size_t i = 0; i < 32; ++i) {
        if (!pointers[i]) {
            pointers[i] = p; sizes[i] = size;
            live += size; ++count; ++allocations;
            if (count > peak_count) peak_count = count;
            if (live > peak) {
                peak = live;
#ifdef _WIN32
                PROCESS_MEMORY_COUNTERS_EX c = {0};
                c.cb = sizeof(c);
                if (!GetProcessMemoryInfo(GetCurrentProcess(), (PROCESS_MEMORY_COUNTERS *)&c, sizeof(c))) abort();
                fprintf(stderr, "os requested=%zu working_set_bytes=%zu private_commit_bytes=%zu\n",
                    live, c.WorkingSetSize, c.PrivateUsage);
#elif defined(__linux__)
                FILE *f = fopen("/proc/self/status", "r");
                if (!f) abort();
                char line[256];
                fprintf(stderr, "os requested=%zu ", live);
                while (fgets(line, sizeof(line), f)) {
                    if (!strncmp(line,"VmSize:",7) || !strncmp(line,"VmRSS:",6)) {
                        line[strcspn(line,"\n")] = 0;
                        fprintf(stderr,"%s ",line);
                    }
                }
                fclose(f); fputc('\n',stderr);
#endif
            }
            return p;
        }
    }
    abort();
}
void probe_free(void *p) {
    if (!p) return;
    for (size_t i = 0; i < 32; ++i) {
        if (pointers[i] == p) {
            live -= sizes[i]; --count; ++releases;
            pointers[i] = NULL; free(p); return;
        }
    }
    abort();
}

static size_t shrink_calls, shrink_successes, moved_copy_peak;
static void shrink_report(void) {
    fprintf(stderr, "shrink calls=%zu successes=%zu moved_copy_peak=%zu\n",
        shrink_calls, shrink_successes, moved_copy_peak);
}
void *probe_realloc(void *p, size_t size) {
    if (!shrink_calls && atexit(shrink_report)) abort();
    ++shrink_calls;
    const char *mode = getenv("CERUNE_TEST_SHRINK");
    assert(mode);
    if (!strcmp(mode,"fail")) return NULL;
    for (size_t i = 0; i < 32; ++i) {
        if (pointers[i] == p) {
            assert(size > 0 && size < sizes[i]);
            void *next;
            if (!strcmp(mode,"move")) {
                next = malloc(size);
                if (!next) abort();
                if (live + size > moved_copy_peak) moved_copy_peak = live + size;
                memcpy(next,p,size); free(p);
            } else {
                assert(!strcmp(mode,"normal"));
                next = realloc(p,size);
                if (!next) return NULL;
            }
            live -= sizes[i] - size;
            sizes[i] = size; pointers[i] = next; ++shrink_successes;
            return next;
        }
    }
    abort();
}
