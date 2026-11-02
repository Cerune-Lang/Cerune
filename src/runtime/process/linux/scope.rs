pub(super) use crate::runtime::process::Exit;
pub(super) struct Pipes {
    pub input: std::fs::File,
    pub output: std::fs::File,
    pub error: std::fs::File,
}
use std::{
    io,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
};

/// 専用の監督プロセスだけをsubreaperにします。埋込みhostへ適用しません。
pub fn init() -> io::Result<()> {
    if unsafe {
        libc::prctl(
            libc::PR_SET_CHILD_SUBREAPER,
            1 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
        )
    } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub struct Scope {
    child: Child,
    signalled: bool,
    reaped: bool,
    descendants: usize,
}
impl Scope {
    pub fn spawn(
        exe: &Path,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<(Self, Pipes)> {
        let mut command = Command::new(exe);
        // 対象へ制御fdや他の監督fdを渡しません。全fdをCLOEXECにするだけでexec通知fdは閉じません。
        unsafe {
            command.pre_exec(|| super::inherit::prepare(-1, false));
        }
        let mut child = command
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        let pipes = Pipes {
            input: std::fs::File::from(std::os::fd::OwnedFd::from(child.stdin.take().unwrap())),
            output: std::fs::File::from(std::os::fd::OwnedFd::from(child.stdout.take().unwrap())),
            error: std::fs::File::from(std::os::fd::OwnedFd::from(child.stderr.take().unwrap())),
        };
        Ok((
            Self {
                child,
                signalled: false,
                reaped: false,
                descendants: 0,
            },
            pipes,
        ))
    }
    /// groupへの終了要求まではleaderをreapせず、PIDの再利用を防ぎます。
    pub fn peek(&mut self) -> io::Result<Option<Exit>> {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { info.si_pid() } == 0 {
            return Ok(None);
        }
        let status = unsafe { info.si_status() };
        Ok(Some(if info.si_code == libc::CLD_EXITED {
            Exit::Code(status as u32)
        } else {
            Exit::Signal(status as u32)
        }))
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        if self.signalled {
            return Ok(());
        }
        // leaderはまだwait可能な子として保持しているので、別のPIDへ送信しません。
        if unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) } == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        self.signalled = true;
        Ok(())
    }
    pub fn collect(&mut self) -> io::Result<bool> {
        assert!(self.signalled);
        if !self.reaped {
            self.reaped = self.child.try_wait()?.is_some();
            if !self.reaped {
                return Ok(false);
            }
        }
        for _ in 0..64 {
            let mut status = 0;
            let pid =
                unsafe { libc::waitpid(-(self.child.id() as i32), &mut status, libc::WNOHANG) };
            if pid > 0 {
                self.descendants += 1;
                continue;
            }
            if pid == 0 {
                return Ok(false);
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ECHILD) {
                return Ok(true);
            }
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
        Ok(false)
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        // 例外経路でも無期限のwaitはせず、残る後始末を明示的なcollectへ任せます。
        if self.terminate().is_ok() {
            let _ = self.collect();
        }
    }
}
