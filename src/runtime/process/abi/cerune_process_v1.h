#ifndef CERUNE_PROCESS_V1_H
#define CERUNE_PROCESS_V1_H
#include <stdint.h>
#include <stddef.h>
/* Prototype ABI. All calls on a context are serial and non-reentrant.
 * Valid, initialized input ranges and exclusive output ranges must not overlap
 * each other or the context. Non-null pointers are not validated for liveness.
 * Caller retains inputs only for the duration of prepare/create.
 * A returned context must be destroyed exactly once. */
typedef struct cerune_process_context cerune_process_context;
typedef struct { const uint8_t *data; uint64_t len; } cerune_process_bytes;
typedef struct {
    uint64_t version, size;
    cerune_process_bytes target;
    const cerune_process_bytes *arguments;
    uint64_t argument_count;
    cerune_process_bytes stdin_bytes;
    uint64_t stdout_limit, stderr_limit, timeout_ms;
    uint64_t node, source, start, end;
} cerune_process_request_v1;
enum {
    CERUNE_PROCESS_OK=0, CERUNE_PROCESS_PENDING=1, CERUNE_PROCESS_READY=2,
    CERUNE_PROCESS_VERSION=10, CERUNE_PROCESS_SHAPE=11, CERUNE_PROCESS_STATE=12,
    CERUNE_PROCESS_RANGE=13, CERUNE_PROCESS_UTF8=14, CERUNE_PROCESS_ALLOCATION=15,
    CERUNE_PROCESS_CONFIG=16, CERUNE_PROCESS_PANIC=17, CERUNE_PROCESS_TARGET=18
};
/* target: 1=x86_64-pc-windows-msvc, 2=x86_64-unknown-linux-gnu.
 * Empty configuration explicitly creates an unconfigured connection.
 * Other configuration bytes use cerune-process-v1 JSON, not a pathname.
 * create writes NULL to out on failure (out must itself be valid). */
uint32_t cerune_process_v1_create(uint64_t version, uint64_t target,
    cerune_process_bytes config, cerune_process_context **out);
uint32_t cerune_process_v1_prepare(cerune_process_context *, const cerune_process_request_v1 *);
/* reservation: 0=success, 1=capacity exceeded, 2=allocation failed. */
uint32_t cerune_process_v1_execute(cerune_process_context *, uint64_t reservation);
/* Exact 32-u64 Report v1; pending or ready. No state or clock advances. */
uint32_t cerune_process_v1_report(cerune_process_context *, uint64_t *out, uint64_t words);
/* Ready only. channel: 1=stdout, 2=stderr. Copy min(capacity, remaining).
 * offset==length is allowed; offset>length is rejected. No read cursor.
 * out may be NULL when no bytes are copied. written must always be valid.
 * Output arguments remain unchanged on errors except create/destroy. */
uint32_t cerune_process_v1_copy(cerune_process_context *, uint64_t channel,
    uint64_t offset, uint8_t *out, uint64_t capacity, uint64_t *written);
/* Discard pending or ready data; preserves disabled connection and OS ownership. */
uint32_t cerune_process_v1_release(cerune_process_context *);
/* Frees pending/ready/poisoned context, clears the handle before dropping. */
uint32_t cerune_process_v1_destroy(cerune_process_context **);
_Static_assert(sizeof(cerune_process_bytes)==16, "bytes ABI");
_Static_assert(sizeof(cerune_process_request_v1)==120, "request ABI");
_Static_assert(offsetof(cerune_process_request_v1,target)==16, "target offset");
_Static_assert(offsetof(cerune_process_request_v1,stdin_bytes)==48, "stdin offset");
_Static_assert(offsetof(cerune_process_request_v1,node)==88, "origin offset");
#endif
