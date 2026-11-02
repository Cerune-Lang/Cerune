use std::{
    io,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub environment: Vec<(String, String)>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    Invalid,
    Duplicate,
    Capacity,
}
pub(super) const MAX_REQUEST: usize = 65536;
pub(super) fn path(p: &Path) -> bool {
    p.is_absolute()
        && p.as_os_str().as_bytes().len() <= 4095
        && !p.as_os_str().as_bytes().contains(&0)
}
pub(super) fn validate(targets: &[Target]) -> Result<(), ConfigError> {
    if targets.len() > 64 {
        return Err(ConfigError::Capacity);
    }
    let mut total = 0usize;
    for (i, t) in targets.iter().enumerate() {
        if t.name.is_empty() || t.name.contains('\0') || !path(&t.executable) || !path(&t.directory)
        {
            return Err(ConfigError::Invalid);
        }
        if t.name.len() > 256 || t.environment.len() > 128 {
            return Err(ConfigError::Capacity);
        }
        if targets[..i].iter().any(|x| x.name == t.name) {
            return Err(ConfigError::Duplicate);
        }
        let mut size = t.executable.as_os_str().len() + t.directory.as_os_str().len();
        for (j, (k, v)) in t.environment.iter().enumerate() {
            if k.is_empty() || k.contains(['=', '\0']) || v.contains('\0') {
                return Err(ConfigError::Invalid);
            }
            if t.environment[..j].iter().any(|(old, _)| old == k) {
                return Err(ConfigError::Duplicate);
            }
            size = size
                .checked_add(k.len())
                .and_then(|n| n.checked_add(v.len()))
                .ok_or(ConfigError::Capacity)?;
        }
        if size > 32768 {
            return Err(ConfigError::Capacity);
        }
        total = total
            .checked_add(size + t.name.len())
            .ok_or(ConfigError::Capacity)?;
        if total > 1_048_576 {
            return Err(ConfigError::Capacity);
        }
    }
    Ok(())
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid process registration")
}
fn put(out: &mut Vec<u8>, bytes: &[u8]) -> io::Result<()> {
    if bytes.contains(&0) || bytes.len() > MAX_REQUEST || out.len() + 4 + bytes.len() > MAX_REQUEST
    {
        return Err(invalid());
    }
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}
pub(super) fn encode(t: &Target, args: &[&str]) -> io::Result<Vec<u8>> {
    if args.len() > 4096
        || args
            .iter()
            .try_fold(0usize, |n, s| n.checked_add(s.len()))
            .is_none_or(|n| n > 16384)
    {
        return Err(invalid());
    }
    let mut out = Vec::new();
    put(&mut out, t.executable.as_os_str().as_bytes())?;
    put(&mut out, t.directory.as_os_str().as_bytes())?;
    out.extend_from_slice(&(args.len() as u32).to_le_bytes());
    for a in args {
        put(&mut out, a.as_bytes())?;
    }
    out.extend_from_slice(&(t.environment.len() as u32).to_le_bytes());
    for (k, v) in &t.environment {
        put(&mut out, k.as_bytes())?;
        put(&mut out, v.as_bytes())?;
    }
    Ok(out)
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn count(&mut self, max: usize) -> io::Result<usize> {
        let bytes = self.0.get(..4).ok_or_else(invalid)?;
        let n = u32::from_le_bytes(bytes.try_into().unwrap()) as usize;
        self.0 = &self.0[4..];
        if n > max { Err(invalid()) } else { Ok(n) }
    }
    fn bytes(&mut self) -> io::Result<&'a [u8]> {
        let n = self.count(MAX_REQUEST)?;
        let bytes = self.0.get(..n).ok_or_else(invalid)?;
        if bytes.contains(&0) {
            return Err(invalid());
        }
        self.0 = &self.0[n..];
        Ok(bytes)
    }
    fn text(&mut self) -> io::Result<String> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| invalid())
    }
}
pub(super) fn decode(bytes: &[u8]) -> io::Result<(Target, Vec<String>)> {
    if bytes.len() > MAX_REQUEST {
        return Err(invalid());
    }
    let mut r = Reader(bytes);
    let executable = std::ffi::OsString::from_vec(r.bytes()?.to_vec()).into();
    let directory = std::ffi::OsString::from_vec(r.bytes()?.to_vec()).into();
    let count = r.count(4096)?;
    let mut args = Vec::new();
    let mut size = 0;
    for _ in 0..count {
        let a = r.text()?;
        size += a.len();
        if size > 16384 {
            return Err(invalid());
        }
        args.push(a);
    }
    let count = r.count(128)?;
    let mut environment = Vec::new();
    for _ in 0..count {
        environment.push((r.text()?, r.text()?));
    }
    if !r.0.is_empty() {
        return Err(invalid());
    }
    let t = Target {
        name: "registered".into(),
        executable,
        directory,
        environment,
    };
    validate(std::slice::from_ref(&t)).map_err(|_| invalid())?;
    Ok((t, args))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_roundtrip_and_rejections() {
        let t = Target {
            name: "x".into(),
            executable: "/tmp/tool".into(),
            directory: "/tmp".into(),
            environment: vec![
                ("Path".into(), "e\u{301}".into()),
                ("PATH".into(), "é".into()),
            ],
        };
        let bytes = encode(&t, &["", "日本語\r\n", ";&|$()"]).unwrap();
        let (got, args) = decode(&bytes).unwrap();
        assert_eq!(got.environment, t.environment);
        assert_eq!(args, ["", "日本語\r\n", ";&|$()"]);
        for n in 0..bytes.len() {
            assert!(decode(&bytes[..n]).is_err());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(decode(&extra).is_err());
        let mut dup = t.clone();
        dup.environment.push(("PATH".into(), "x".into()));
        assert_eq!(validate(&[dup]), Err(ConfigError::Duplicate));
        assert!(encode(&t, &["a\0b"]).is_err());
        let mut relative = t;
        relative.directory = ".".into();
        assert_eq!(validate(&[relative]), Err(ConfigError::Invalid));
    }
}
