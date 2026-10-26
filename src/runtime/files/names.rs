//! OSや作業ディレクトリに依存しない、初版のファイル名検査です。
use super::Code;
pub const MAX_NAME_BYTES: usize = 255;

/// UTF-8の1要素を検査します。名前の変換・正規化やファイルの存在確認はしません。
/// この検査だけでOSの作業範囲や、通常ファイルであることを保証しません。
pub fn validate_name(name: &str) -> Result<(), Code> {
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c <= '\u{1f}' || c == '\u{7f}' || "<>:\"/\\|?*".contains(c))
    {
        return Err(Code::InvalidName);
    }
    // Windowsのdevice名は拡張子が付いても予約名です。直前の空白も許可しません。
    let stem = name.split('.').next().unwrap().trim_end_matches(' ');
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .any(|n| stem.eq_ignore_ascii_case(n))
    {
        return Err(Code::InvalidName);
    }
    for prefix in ["COM", "LPT"] {
        if stem
            .get(..3)
            .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
            && matches!(
                &stem[3..],
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        {
            return Err(Code::InvalidName);
        }
    }
    Ok(())
}
