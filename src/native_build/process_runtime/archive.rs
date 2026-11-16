//! Inspect regular ar members without resolving external paths or executing tools.
use cerune_lang::codegen::x86_64::Target;
pub(super) fn validate(bytes: &[u8], target: Target) -> Result<(), String> {
    let invalid = || {
        "invalid process runtime archive (regular ar with matching x86-64 object members required)"
            .to_owned()
    };
    if !bytes.starts_with(b"!<arch>\n") {
        return Err(invalid());
    }
    let mut pos = 8usize;
    let mut objects = 0;
    while pos < bytes.len() {
        let h = bytes
            .get(pos..pos.checked_add(60).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        if &h[58..] != b"`\n" {
            return Err(invalid());
        }
        let name = std::str::from_utf8(&h[..16]).map_err(|_| invalid())?.trim();
        let size = std::str::from_utf8(&h[48..58])
            .map_err(|_| invalid())?
            .trim()
            .parse::<usize>()
            .map_err(|_| invalid())?;
        pos += 60;
        let end = pos.checked_add(size).ok_or_else(invalid)?;
        let mut data = bytes.get(pos..end).ok_or_else(invalid)?;
        let mut symbol = matches!(name, "/" | "//" | "/SYM64/") || name.starts_with("__.SYMDEF");
        if let Some(n) = name.strip_prefix("#1/") {
            let n = n.parse::<usize>().map_err(|_| invalid())?;
            let extended = data.get(..n).ok_or_else(invalid)?;
            symbol = extended.starts_with(b"__.SYMDEF");
            data = &data[n..];
        }
        if !symbol {
            let correct = match target {
                Target::X86_64UnknownLinuxGnu => {
                    data.len() >= 64
                        && data.starts_with(b"\x7fELF")
                        && data[4..7] == [2, 1, 1]
                        && data[16..20] == [1, 0, 62, 0]
                }
                Target::X86_64PcWindowsMsvc => {
                    data.len() >= 20
                        && (data[..2] == [0x64, 0x86]
                            || (data[..4] == [0, 0, 255, 255] && data[6..8] == [0x64, 0x86]))
                }
            };
            if !correct {
                return Err(invalid());
            }
            objects += 1;
        }
        pos = end.checked_add(size % 2).ok_or_else(invalid)?;
        if pos > bytes.len() {
            return Err(invalid());
        }
    }
    if objects == 0 {
        return Err(invalid());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ar(data: &[u8]) -> Vec<u8> {
        let mut b = b"!<arch>\n".to_vec();
        b.extend(
            format!(
                "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                "runtime.o/",
                0,
                0,
                0,
                644,
                data.len()
            )
            .bytes(),
        );
        b.extend(data);
        if !data.len().is_multiple_of(2) {
            b.push(b'\n');
        }
        b
    }
    #[test]
    fn rejects_thin_empty_wrong_target_truncated_and_invalid_sizes() {
        let linux = Target::X86_64UnknownLinuxGnu;
        let win = Target::X86_64PcWindowsMsvc;
        let mut elf = vec![0; 64];
        elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        elf[16..20].copy_from_slice(&[1, 0, 62, 0]);
        let good = ar(&elf);
        assert!(validate(&good, linux).is_ok());
        assert!(validate(&good, win).is_err());
        let mut coff = vec![0; 20];
        coff[..2].copy_from_slice(&[0x64, 0x86]);
        assert!(validate(&ar(&coff), win).is_ok());
        assert!(validate(&ar(&coff), linux).is_err());
        for b in [
            b"!<thin>\n".as_slice(),
            b"!<arch>\n",
            &good[..good.len() - 1],
        ] {
            assert!(validate(b, linux).is_err());
        }
        let mut bad = good;
        bad[56..66].copy_from_slice(b"9999999999");
        assert!(validate(&bad, linux).is_err());
    }
}
