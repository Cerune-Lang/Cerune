//! ABI prototype only: this does not claim that emit-wat supports std:fs yet.
#[path = "support/process.rs"]
mod process;
use std::{fs, path::PathBuf, process::Command, time::Duration};

#[test]
fn proposed_file_host_abi_runs_in_wasm_without_public_memory() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let node_config = std::env::var_os("CERUNE_TEST_NODE");
    let node = node_config.clone().unwrap_or_else(|| "node".into());
    let wabt_config = std::env::var_os("CERUNE_TEST_WAT2WASM_JS");
    let wabt = wabt_config
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/wasm-tools/node_modules/wabt/bin/wat2wasm"));
    if !Command::new(&node)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
        || !wabt.is_file()
    {
        assert!(
            node_config.is_none() && wabt_config.is_none(),
            "required WAT tools unavailable"
        );
        eprintln!("WAT file ABI prototype skipped; configure Node and WABT");
        return;
    }
    let dir = root
        .join("target")
        .join(format!("wat-files-protocol-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    // Forwarders test actual Wasm multi-value/i64 transport, not JS-only calls.
    // They are probe exports only; generated programs will export main only.
    let signatures = [
        ("files_status_v1", "", "(result i32)", ""),
        (
            "file_name_begin_v1",
            "(param i32)",
            "(result i32)",
            "local.get 0",
        ),
        (
            "file_name_chunk_v1",
            "(param i64 i32)",
            "(result i32)",
            "local.get 0 local.get 1",
        ),
        (
            "file_open_v1",
            "(param i32)",
            "(result i32 i32)",
            "local.get 0",
        ),
        (
            "file_read_v1",
            "(param i32 i32)",
            "(result i64 i32)",
            "local.get 0 local.get 1",
        ),
        (
            "file_write_v1",
            "(param i32 i64 i32)",
            "(result i32)",
            "local.get 0 local.get 1 local.get 2",
        ),
        (
            "file_close_v1",
            "(param i32)",
            "(result i32)",
            "local.get 0",
        ),
    ];
    let mut wat = String::from("(module\n");
    for (name, params, results, _) in signatures {
        wat.push_str(&format!(
            "(import \"cerune\" \"{name}\" (func ${name} {params} {results}))\n"
        ));
    }
    wat.push_str("(memory 1)\n");
    for (name, params, results, args) in signatures {
        wat.push_str(&format!(
            "(func (export \"{name}\") {params} {results} {args} call ${name})\n"
        ));
    }
    wat.push(')');
    let source = dir.join("probe.wat");
    let binary = dir.join("probe.wasm");
    fs::write(&source, wat).unwrap();
    for (label, mut command) in [
        ("assemble", {
            let mut c = Command::new(&node);
            c.arg(wabt).arg(source).arg("-o").arg(&binary);
            c
        }),
        ("contract", {
            let mut c = Command::new(&node);
            c.arg(root.join("tests/support/wat_files_protocol.cjs"))
                .arg(&binary);
            c
        }),
    ] {
        let out =
            process::bounded_output(&mut command, &dir, label, Duration::from_secs(30)).unwrap();
        assert!(
            out.status.success(),
            "{label}: {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
