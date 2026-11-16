#define _CRT_SECURE_NO_WARNINGS
#include "cerune_process_v1.h"
/* Clang compiles these ordinary C calls. Runtime calls in the reused assertions
 * go through typed Native instructions; the C frontend does not lower those calls. */
uint32_t cerune_native_create(uint64_t, uint64_t, const uint8_t *, uint64_t, cerune_process_context **);
uint32_t cerune_native_prepare(cerune_process_context *, const cerune_process_request_v1 *);
uint32_t cerune_native_execute(cerune_process_context *, uint64_t);
uint32_t cerune_native_report(cerune_process_context *, uint64_t *, uint64_t);
uint32_t cerune_native_copy(cerune_process_context *, uint64_t, uint64_t, uint8_t *, uint64_t, uint64_t *);
uint32_t cerune_native_release(cerune_process_context *);
uint32_t cerune_native_destroy(cerune_process_context **);
_Static_assert(sizeof(cerune_process_bytes)==16, "Native Bytes size");
_Static_assert(sizeof(cerune_process_request_v1)==120, "Native Request copy size");
static uint32_t create(uint64_t version, uint64_t target, cerune_process_bytes config, cerune_process_context **out) {
    return cerune_native_create(version, target, config.data, config.len, out);
}
#define cerune_process_v1_create create
#define cerune_process_v1_prepare cerune_native_prepare
#define cerune_process_v1_execute cerune_native_execute
#define cerune_process_v1_report cerune_native_report
#define cerune_process_v1_copy cerune_native_copy
#define cerune_process_v1_release cerune_native_release
#define cerune_process_v1_destroy cerune_native_destroy
#define main c_abi_assertions
#include "../c_abi/probe.c"
#undef main

static int wide_values(void) {
    cerune_process_context *context=NULL;
    CHECK(create(1,TARGET,bytes(NULL,0),&context)==0);
    cerune_process_request_v1 request = {1,sizeof request,bytes("fixture",7),NULL,0,
        bytes(NULL,0),UINT64_MAX,UINT64_C(0x8000000000000001),UINT64_MAX,
        UINT64_C(0x100000001),UINT64_C(0x100000002),UINT64_C(0x100000003),UINT64_C(0x100000004)};
    uint64_t report[32];
    CHECK(cerune_native_prepare(context,&request)==CERUNE_PROCESS_READY);
    CHECK(cerune_native_report(context,report,32)==0);
    CHECK(report[2]==request.node && report[3]==request.source);
    CHECK(report[4]==request.start && report[5]==request.end);
    CHECK(report[9]==request.stdout_limit && report[10]==request.stderr_limit && report[11]==request.timeout_ms);
    CHECK(report[6]==0 && report[19]==2 && report[20]==1); /* RequestLimit, Validate. */
    CHECK(cerune_native_destroy(&context)==0 && !context);
    return 0;
}
int main(int argc, char **argv) {
    if (argc==3 && !strcmp(argv[1],"--child")) return c_abi_assertions(argc,argv);
    CHECK(wide_values()==0);
    CHECK(c_abi_assertions(argc,argv)==0);
    puts("Native ABI: aggregate and scalar calls, request copy, u64 high bits, C runtime assertions: ok");
    return 0;
}
