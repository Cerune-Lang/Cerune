#include "cerune_process_v1.h"
/* Compile to LLVM text/ASM only. Never linked into the LLVM probe.
 * Independent evidence of Clang's lowering of the canonical header. */
uint32_t reference_create(uint64_t version, uint64_t target,
    const uint8_t *data, uint64_t length, cerune_process_context **out) {
    cerune_process_bytes config={data,length};
    return cerune_process_v1_create(version,target,config,out);
}
