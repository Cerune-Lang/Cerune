use super::generated::run;
use super::{execute, hir};
use crate::program_arguments::c::build;
use cerune_lang::{codegen::c, runtime::files::Files};
use std::fs;

#[test]
fn generated_c_injected_transfers_and_close_failures_match_common_runtime() {
    use super::{Memory, Step};
    use cerune_lang::runtime::files::OpenError;
    for writing in [false, true] {
        let source = if writing {
            r#"import "std:fs" as fs; print("before"); fs::write_new("x","abc",3); print("after");"#
        } else {
            r#"import "std:fs" as fs; print("before"); print(fs::read_text("x",3)); print("after");"#
        };
        let program = hir(source);
        let generated = c::emit_c(&program).unwrap();
        let end = generated
            .find("/* Selected by the C compilation target")
            .unwrap();
        let text = format!(
            "#define CERUNE_FILE_TEST_CONNECTION\n{}\n{}",
            &generated[..end],
            TRANSFER_HOST
        );
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&text, opt, &[]) else {
                return;
            };
            for case in 0..7 {
                let actual = run(&dir, &exe, &[case.to_string().as_ref()]);
                for route in 0..4 {
                    let memory = Memory::default();
                    {
                        let mut state = memory.0.borrow_mut();
                        if !writing {
                            state.files.insert("x".into(), b"abc".to_vec());
                        }
                        let steps = if writing {
                            &mut state.writes
                        } else {
                            &mut state.reads
                        };
                        steps.extend([Step::Interrupted, Step::Count(1)]);
                        match case {
                            1 | 5 => steps.push_back(Step::Fail),
                            2 => steps.push_back(Step::Invalid),
                            3 if writing => steps.push_back(Step::Zero),
                            _ => steps.extend([Step::Count(1), Step::Count(1)]),
                        }
                        state.close_error = case == 4 || case == 5;
                        if case == 6 {
                            state.open_error = Some(OpenError::InvalidResponse);
                        }
                    }
                    let mut files = Files::new(memory.clone());
                    match execute(route, &program, &mut files) {
                        Ok(output) => {
                            assert!(actual.status.success(), "{actual:?}");
                            assert_eq!(actual.stdout, output.as_bytes());
                        }
                        Err((failure, output)) => {
                            assert_eq!(actual.status.code(), Some(1));
                            assert_eq!(actual.stdout, output.as_bytes());
                            assert_eq!(
                                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                                format!("cerune: {}\n", failure.record()),
                                "write={writing} case={case}"
                            );
                        }
                    }
                    if case != 6 {
                        assert_eq!(fs::read(dir.join("closes")).unwrap(), b"1");
                        if writing {
                            assert_eq!(
                                fs::read(dir.join("accepted")).unwrap(),
                                memory.0.borrow().files["x"]
                            );
                        }
                    }
                }
            }
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
const TRANSFER_HOST: &str = r#"
static int case_id, step, closes;
static size_t transferred;
static unsigned char accepted[3];
static const char *cerune_file_open(cerune_string name, int create, intptr_t *handle) {
    (void)name; (void)create; *handle=42;
    return case_id==6 ? "invalid-response" : NULL;
}
static int64_t transfer(size_t request, int writing) {
    if (++step==1) {errno=EINTR;return -1;}
    if (step==3) {
        if (case_id==1||case_id==5) {errno=EIO;return -1;}
        if (case_id==2) return (int64_t)request+1;
        if (case_id==3&&writing) return 0;
    }
    return transferred==3 ? 0 : 1;
}
static int64_t cerune_file_read(intptr_t handle,unsigned char *data,size_t length) {
    if (handle!=42) abort();
    int64_t count=transfer(length,0);
    if (count==1) { *data=((const unsigned char *)"abc")[transferred++]; }
    return count;
}
static int64_t cerune_file_write(intptr_t handle,const unsigned char *data,size_t length) {
    if (handle!=42) abort();
    int64_t count=transfer(length,1);
    if (count==1) accepted[transferred++]=*data;
    return count;
}
static int cerune_file_close(intptr_t handle) {
    if (handle!=42||++closes!=1) abort();
    FILE *f=fopen("closes","wb");if(!f)abort();fputs("1",f);fclose(f);
    f=fopen("accepted","wb");if(!f)abort();fwrite(accepted,1,transferred,f);fclose(f);
    return case_id==4||case_id==5 ? -1 : 0;
}
int main(int argc,char **argv) {
    case_id=argc>1?atoi(argv[1]):0; cerune_file_configured=1; return cerune_program();
}
"#;
