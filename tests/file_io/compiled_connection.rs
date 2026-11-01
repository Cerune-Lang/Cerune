// LLVM・QBE・Nativeの応答注入に同じバイト列・応答・操作ログを使います。
use super::test_file_reference::{Memory, Step};
use crate::runtime::files::OpenError;

#[derive(Clone)]
pub(super) struct Case {
    pub(super) bytes: Vec<u8>,
    pub(super) exists: bool,
    pub(super) steps: Vec<Step>,
    pub(super) open_error: Option<OpenError>,
    pub(super) close_error: bool,
    pub(super) configured: bool,
    pub(super) write: bool,
}
impl Case {
    pub(super) fn read(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            exists: true,
            steps: vec![],
            open_error: None,
            close_error: false,
            configured: true,
            write: false,
        }
    }
    pub(super) fn write() -> Self {
        Self {
            exists: false,
            write: true,
            ..Self::read(b"")
        }
    }
    pub(super) fn memory(&self, name: &str) -> Memory {
        let memory = Memory::default();
        {
            let mut s = memory.0.borrow_mut();
            if self.exists {
                s.files.insert(name.into(), self.bytes.clone());
            }
            if self.write {
                s.writes.extend(self.steps.clone());
            } else {
                s.reads.extend(self.steps.clone());
            }
            s.open_error = self.open_error;
            s.close_error = self.close_error;
        }
        memory
    }
    pub(super) fn host(&self) -> String {
        let bytes = self
            .bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let steps = self
            .steps
            .iter()
            .map(|s| match s {
                Step::Count(n) => n.to_string(),
                Step::Interrupted => "-2".into(),
                Step::Fail => "-1".into(),
                Step::Invalid => "-3".into(),
                Step::Zero => "0".into(),
            })
            .collect::<Vec<_>>()
            .join(",");
        let open = match self.open_error {
            None => 0,
            Some(OpenError::NotFound) => 1,
            Some(OpenError::AlreadyExists) => 2,
            Some(OpenError::UnsupportedTarget) => 3,
            Some(OpenError::Failed) => 4,
            Some(OpenError::InvalidResponse) => 5,
        };
        format!(
            "#define _CRT_SECURE_NO_WARNINGS\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\
            static unsigned char data[16384] = {{{}}};\nstatic uint64_t length = {};\n\
            static int exists = {}, configured = {}, open_error = {}, close_error = {};\n\
            static int64_t steps[] = {{{}}};\nstatic uint64_t step_count = {}, next_step, position;\nstatic int inject_write = {};\n{}",
            if bytes.is_empty() { "0" } else { &bytes },
            self.bytes.len(),
            self.exists as u8,
            self.configured as u8,
            open,
            self.close_error as u8,
            if steps.is_empty() { "0" } else { &steps },
            self.steps.len(),
            self.write as u8,
            HOST
        )
    }
}
const HOST: &str = r#"
static FILE *trace(void) { FILE *f=fopen("host-trace.txt","ab"); if(!f) abort(); return f; }
static void save(void) { FILE *f=fopen("host-bytes.bin","wb"); if(!f) abort(); if(fwrite(data,1,(size_t)length,f)!=length) abort(); if(fclose(f)) abort(); }
int32_t cerune_file_host_configured(void) { return configured; }
int32_t cerune_file_host_open(const unsigned char *name, uint64_t n, int32_t write, int64_t *handle) {
    FILE *f=trace(); fprintf(f,"open:%s:%.*s\n",write?"WriteNew":"ReadText",(int)n,name); fclose(f);
    if(open_error) return open_error;
    if(write && exists) return 2;
    if(!write && !exists) return 1;
    if(write) { exists=1; length=0; save(); }
    position=0; *handle=7; return 0;
}
static int64_t step(uint64_t n, int write) {
    if(write!=inject_write || next_step==step_count) return (int64_t)n;
    int64_t s=steps[next_step++]; if(s==-3) return (int64_t)n+1;
    return s>0 && (uint64_t)s>n ? (int64_t)n : s;
}
int64_t cerune_file_host_read(int64_t h, unsigned char *into, uint64_t n) {
    if(h!=7) abort(); FILE *f=trace(); fprintf(f,"read:%llu\n",(unsigned long long)n); fclose(f);
    int64_t s=step(n,0); if(s<0 || (uint64_t)s>n) return s;
    uint64_t count=(uint64_t)s; if(count>length-position) count=length-position;
    memcpy(into,data+position,(size_t)count); position+=count; return (int64_t)count;
}
int64_t cerune_file_host_write(int64_t h, const unsigned char *bytes, uint64_t n) {
    if(h!=7) abort(); FILE *f=trace(); fprintf(f,"write:%llu\n",(unsigned long long)n); fclose(f);
    int64_t s=step(n,1); if(s<0 || (uint64_t)s>n) return s;
    if(length+(uint64_t)s>sizeof(data)) abort();
    memcpy(data+length,bytes,(size_t)s); length+=(uint64_t)s; save(); return s;
}
int32_t cerune_file_host_close(int64_t h) {
    if(h!=7) abort(); FILE *f=trace(); fputs("close\n",f); fclose(f); return close_error;
}
"#;
