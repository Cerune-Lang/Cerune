use super::ir::Origin;
use std::fmt::Write;

// 呼び出しごとの不変の位置を渡し、正常経路で共有の「現在位置」を変更しません。
pub(super) fn argument(origin: Origin, output: &mut String) {
    output.push_str(", ");
    literal(origin, output);
}

pub(super) fn literal(origin: Origin, output: &mut String) {
    write!(
        output,
        "\" node={}{} bytes={}..{}\\n\"",
        origin.node_id.0,
        origin.span.source_id().record_field(),
        origin.span.start(),
        origin.span.end()
    )
    .unwrap();
}

// C標準ライブラリのstderrを使うため、生成時にOSのFILE構造やABIを推測しません。
pub(super) const SUPPORT: &str = r#"static void cerune_runtime_fail(const char *code, const char *origin) {
    fflush(stdout);
    fputs("cerune: runtime-v1 code=", stderr);
    fputs(code, stderr);
    fputs(origin, stderr);
    fflush(stderr);
    abort();
}

"#;
