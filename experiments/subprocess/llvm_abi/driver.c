#define _CRT_SECURE_NO_WARNINGS
#include "cerune_process_v1.h"
/* Clang compiles these ordinary C calls. Runtime calls in the reused assertions
 * go through handwritten LLVM; the C frontend does not lower those calls. */
uint32_t cerune_llvm_create(uint64_t, uint64_t, const uint8_t *, uint64_t, cerune_process_context **);
uint32_t cerune_llvm_prepare(cerune_process_context *, const cerune_process_request_v1 *);
uint32_t cerune_llvm_execute(cerune_process_context *, uint64_t);
uint32_t cerune_llvm_report(cerune_process_context *, uint64_t *, uint64_t);
uint32_t cerune_llvm_copy(cerune_process_context *, uint64_t, uint64_t, uint8_t *, uint64_t, uint64_t *);
uint32_t cerune_llvm_release(cerune_process_context *);
uint32_t cerune_llvm_destroy(cerune_process_context **);
void cerune_llvm_layout(uint64_t *);
static uint32_t create(uint64_t version, uint64_t target, cerune_process_bytes config, cerune_process_context **out) {
    return cerune_llvm_create(version, target, config.data, config.len, out);
}
#define cerune_process_v1_create create
#define cerune_process_v1_prepare cerune_llvm_prepare
#define cerune_process_v1_execute cerune_llvm_execute
#define cerune_process_v1_report cerune_llvm_report
#define cerune_process_v1_copy cerune_llvm_copy
#define cerune_process_v1_release cerune_llvm_release
#define cerune_process_v1_destroy cerune_llvm_destroy
#define main c_abi_assertions
#include "../c_abi/probe.c"
#undef main

static int layout_and_wide_values(void) {
    uint64_t actual[15];
    cerune_llvm_layout(actual);
    const uint64_t expected[] = {
        sizeof(cerune_process_bytes), sizeof(cerune_process_request_v1),
        offsetof(cerune_process_request_v1,version), offsetof(cerune_process_request_v1,size),
        offsetof(cerune_process_request_v1,target), offsetof(cerune_process_request_v1,arguments),
        offsetof(cerune_process_request_v1,argument_count), offsetof(cerune_process_request_v1,stdin_bytes),
        offsetof(cerune_process_request_v1,stdout_limit), offsetof(cerune_process_request_v1,stderr_limit),
        offsetof(cerune_process_request_v1,timeout_ms), offsetof(cerune_process_request_v1,node),
        offsetof(cerune_process_request_v1,source), offsetof(cerune_process_request_v1,start),
        offsetof(cerune_process_request_v1,end)
    };
    CHECK(!memcmp(actual,expected,sizeof expected));
    cerune_process_context *context=NULL;
    CHECK(create(1,TARGET,bytes(NULL,0),&context)==0);
    cerune_process_request_v1 request = {1,sizeof request,bytes("fixture",7),NULL,0,
        bytes(NULL,0),UINT64_MAX,UINT64_C(0x8000000000000001),UINT64_MAX,
        UINT64_C(0x100000001),UINT64_C(0x100000002),UINT64_C(0x100000003),UINT64_C(0x100000004)};
    uint64_t report[32];
    CHECK(cerune_llvm_prepare(context,&request)==CERUNE_PROCESS_READY);
    CHECK(cerune_llvm_report(context,report,32)==0);
    CHECK(report[2]==request.node && report[3]==request.source);
    CHECK(report[4]==request.start && report[5]==request.end);
    CHECK(report[9]==request.stdout_limit && report[10]==request.stderr_limit && report[11]==request.timeout_ms);
    CHECK(report[6]==0 && report[19]==2 && report[20]==1); /* RequestLimit, Validate. */
    CHECK(cerune_llvm_destroy(&context)==0 && !context);
    return 0;
}
int main(int argc, char **argv) {
    if (argc==3 && !strcmp(argv[1],"--child")) return c_abi_assertions(argc,argv);
    CHECK(layout_and_wide_values()==0);
    CHECK(c_abi_assertions(argc,argv)==0);
    puts("LLVM ABI: target calls, 15 layout values, u64 high bits, C runtime assertions: ok");
    return 0;
}
