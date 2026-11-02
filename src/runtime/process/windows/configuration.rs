use super::*;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CompareStringOrdinal(a: *const u16, an: i32, b: *const u16, bn: i32, ignore: i32) -> i32;
}
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
pub(super) struct Prepared {
    pub name: String,
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub environment: Vec<u16>,
}
pub(super) fn prepare(targets: Vec<Target>) -> Result<Vec<Prepared>, ConfigError> {
    if targets.len() > 64 {
        return Err(ConfigError::Capacity);
    }
    let mut result: Vec<Prepared> = Vec::new();
    let mut total = 0usize;
    for target in targets {
        if target.name.is_empty()
            || target.name.contains('\0')
            || !target.executable.is_absolute()
            || !target.directory.is_absolute()
        {
            return Err(ConfigError::Invalid);
        }
        if target.name.len() > 256 || target.environment.len() > 128 {
            return Err(ConfigError::Capacity);
        }
        if result.iter().any(|t| t.name == target.name) {
            return Err(ConfigError::Duplicate);
        }
        let mut units = target.name.len();
        for path in [&target.executable, &target.directory] {
            let n = path.as_os_str().encode_wide().count();
            if n > 32766 {
                return Err(ConfigError::Capacity);
            }
            if path.as_os_str().encode_wide().any(|c| c == 0) {
                return Err(ConfigError::Invalid);
            }
            units = units.checked_add(n * 2).ok_or(ConfigError::Capacity)?;
        }
        let mut entries: Vec<(Vec<u16>, Vec<u16>)> = Vec::new();
        for (key, value) in target.environment {
            if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                return Err(ConfigError::Invalid);
            }
            units = units
                .checked_add(key.len())
                .and_then(|n| n.checked_add(value.len()))
                .ok_or(ConfigError::Capacity)?;
            if units > 1_048_576 {
                return Err(ConfigError::Capacity);
            }
            let key: Vec<u16> = key.encode_utf16().collect();
            let value: Vec<u16> = value.encode_utf16().collect();
            let mut position = entries.len();
            for (i, (existing, _)) in entries.iter().enumerate() {
                match unsafe {
                    CompareStringOrdinal(
                        key.as_ptr(),
                        key.len() as i32,
                        existing.as_ptr(),
                        existing.len() as i32,
                        1,
                    )
                } {
                    1 => {
                        position = i;
                        break;
                    }
                    2 => return Err(ConfigError::Duplicate),
                    3 => {}
                    _ => return Err(ConfigError::Invalid),
                }
            }
            entries.insert(position, (key, value));
        }
        total = total.checked_add(units).ok_or(ConfigError::Capacity)?;
        if total > 1_048_576 {
            return Err(ConfigError::Capacity);
        }
        let size = entries
            .iter()
            .try_fold(1usize, |n, (k, v)| n.checked_add(k.len() + v.len() + 2))
            .ok_or(ConfigError::Capacity)?;
        if size > 32767 {
            return Err(ConfigError::Capacity);
        }
        let mut environment = Vec::with_capacity(size.max(2));
        for (key, value) in entries {
            environment.extend(key);
            environment.push(61);
            environment.extend(value);
            environment.push(0);
        }
        if environment.is_empty() {
            environment.push(0);
        }
        environment.push(0);
        result.push(Prepared {
            name: target.name,
            executable: target.executable,
            directory: target.directory,
            environment,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> Target {
        Target {
            name: "tool".into(),
            executable: r"C:\tool.exe".into(),
            directory: r"C:\".into(),
            environment: vec![],
        }
    }
    #[test]
    fn registration_rejects_invalid_and_oversized_os_strings() {
        let mut bad = target();
        bad.environment.push(("a=b".into(), "x".into()));
        assert!(matches!(prepare(vec![bad]), Err(ConfigError::Invalid)));
        let mut bad = target();
        bad.environment.push(("a".into(), "x\0y".into()));
        assert!(matches!(prepare(vec![bad]), Err(ConfigError::Invalid)));
        let mut bad = target();
        bad.environment.push(("a".into(), "x".repeat(32767)));
        assert!(matches!(prepare(vec![bad]), Err(ConfigError::Capacity)));
        let mut bad = target();
        bad.name = "x".repeat(257);
        assert!(matches!(prepare(vec![bad]), Err(ConfigError::Capacity)));
        assert!(matches!(
            prepare(vec![target(); 65]),
            Err(ConfigError::Capacity)
        ));
    }
    #[test]
    fn environment_is_sorted_without_normalizing_values() {
        let mut t = target();
        t.environment = vec![("z".into(), "e\u{301}".into()), ("A".into(), "é".into())];
        let result = prepare(vec![t]).unwrap();
        let text = String::from_utf16(&result[0].environment).unwrap();
        assert_eq!(text, "A=é\0z=e\u{301}\0\0");
    }
}
