// CLI試験用。Ceruneの環境・引数・作業場所と生バイト転送を外部から確認します。
use std::io::{Read, Write};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("sleep") {
        std::thread::sleep(std::time::Duration::from_secs(20));
        return;
    }
    assert_eq!(args, ["echo", "", "日本語", "a\"b", "tail\\", "--"]);
    assert_eq!(std::env::var("CERUNE_CHILD_VALUE").unwrap(), "日本語\r\n");
    assert!(std::env::var_os("CERUNE_PARENT_SENTINEL").is_none());
    assert_eq!(std::fs::read("child-directory-marker").unwrap(), b"registered cwd");
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, [255, 0, 13, 10]);
    std::io::stdout().write_all(&bytes).unwrap();
    std::io::stderr().write_all(b"err\0\r\n").unwrap();
    std::process::exit(7);
}
