use std::{fs::File, io::Read, path::Path};

/// Limits on distinct filesystem sources, including the entry file.
/// Bundled virtual standard modules are not filesystem sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadLimits {
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_files: usize,
}
impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 8 * 1024 * 1024,
            max_total_bytes: 64 * 1024 * 1024,
            max_files: 1024,
        }
    }
}

pub(super) struct SourceBudget {
    limits: LoadLimits,
    bytes: usize,
    files: usize,
}
impl SourceBudget {
    pub(super) fn new(limits: LoadLimits) -> Self {
        Self {
            limits,
            bytes: 0,
            files: 0,
        }
    }

    pub(super) fn read(&mut self, path: &Path) -> Result<String, String> {
        if self.files >= self.limits.max_files {
            return Err(format!(
                "source file count exceeds {}",
                self.limits.max_files
            ));
        }
        let remaining = self.limits.max_total_bytes - self.bytes;
        let allowed = self.limits.max_file_bytes.min(remaining);
        let probe = u64::try_from(allowed)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or("source byte limit is too large")?;
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(probe)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > allowed {
            return Err(if self.limits.max_file_bytes <= remaining {
                format!(
                    "source file byte limit exceeded (limit={})",
                    self.limits.max_file_bytes
                )
            } else {
                format!(
                    "total source byte limit exceeded (limit={}, loaded={})",
                    self.limits.max_total_bytes, self.bytes
                )
            });
        }
        let text = String::from_utf8(bytes).map_err(|_| "source is not valid UTF-8")?;
        self.bytes += text.len();
        self.files += 1;
        Ok(text)
    }
}
