//! 同一のコンパイル結果を保存します。実行や外部ツール起動は担当しません。
use cerune_lang::{codegen::x86_64, ir, mir, modules::Compilation};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub(super) fn save(
    source: &Compilation,
    hir: &ir::Program,
    target: x86_64::Target,
    output: &Path,
    use_ssa: bool,
    use_lir: bool,
    call_depth_limit: std::num::NonZeroU32,
) -> Result<(), String> {
    // frontendもMIR変換も繰り返さず、観測するsnapshotをNativeへ渡します。
    let mir = mir::lower(hir).map_err(|e| source.render(&e.diagnostic()))?;
    let lir = x86_64::lower_mir_with_call_limit(&mir, target, call_depth_limit)
        .map_err(|e| source.render(&e))?;
    let assembly = x86_64::emit_asm_from_lir(&lir, true).map_err(|e| source.render(&e))?;
    let ssa = use_ssa
        .then(|| mir::ssa::construct(&mir))
        .transpose()
        .map_err(|e| source.render(&e.diagnostic()))?;
    let mut files = vec![
        ("sources.json", source.source_manifest()),
        ("program.ceir", ir::text::emit(hir)),
        ("program.mir.txt", mir::text::emit(&mir)),
        ("program.origins.s", assembly),
    ];
    if use_lir {
        files.push((
            "program.lir.txt",
            x86_64::lir::emit(&lir).map_err(|e| source.render(&e))?,
        ));
    }
    if let Some(ssa) = &ssa {
        files.push((
            "program.ssa.txt",
            mir::ssa::text::emit(ssa).map_err(|e| source.render(&e.diagnostic()))?,
        ));
        files.push((
            "program.ssa-map.txt",
            mir::ssa::mapping::emit(ssa).map_err(|e| source.render(&e.diagnostic()))?,
        ));
        let lowered = mir::ssa::lower(ssa).map_err(|e| source.render(&e.diagnostic()))?;
        let lir = x86_64::lower_mir_with_call_limit(&lowered.program, target, call_depth_limit)
            .map_err(|e| source.render(&e))?;
        let assembly = x86_64::emit_asm_from_lir(&lir, true).map_err(|e| source.render(&e))?;
        if use_lir {
            files.push((
                "program.ssa.lir.txt",
                x86_64::lir::emit(&lir).map_err(|e| source.render(&e))?,
            ));
        }
        files.push((
            "program.ssa-lowered.mir.txt",
            mir::text::emit(&lowered.program),
        ));
        files.push(("program.ssa-lowered-map.txt", lowered.mapping));
        files.push(("program.ssa.origins.s", assembly));
    }
    // 指定なしのv1は変更せず、SSA→MIRとNativeを含む索引はv3とします。
    let (schema, transformations, artifacts) = if use_ssa {
        (
            "cerune-observation-v3",
            concat!(
                "  \"transformations\": [\n",
                "    {\"order\": 0, \"kind\": \"representation\", \"pass\": \"scalar-ssa-v1\", \"options\": {}, \"input\": \"mir\", \"output\": \"ssa\", \"mapping\": \"ssa_mapping\"},\n",
                "    {\"order\": 1, \"kind\": \"representation\", \"pass\": \"ssa-lower-v1\", \"options\": {}, \"input\": \"ssa\", \"output\": \"ssa_lowered_mir\", \"mapping\": \"ssa_lowering_mapping\"}\n",
                "  ],\n",
                "  \"artifact_inputs\": {\"hir\": \"sources\", \"mir\": \"hir\", \"ssa\": \"mir\", \"assembly\": \"mir\", \"ssa_lowered_mir\": \"ssa\", \"ssa_assembly\": \"ssa_lowered_mir\"},\n"
            ),
            "    \"ssa\": \"program.ssa.txt\",\n    \"ssa_mapping\": \"program.ssa-map.txt\",\n    \"ssa_lowered_mir\": \"program.ssa-lowered.mir.txt\",\n    \"ssa_lowering_mapping\": \"program.ssa-lowered-map.txt\",\n    \"ssa_assembly\": \"program.ssa.origins.s\",\n",
        )
    } else {
        ("cerune-observation-v1", "", "")
    };
    // v1/v3は維持し、LIRを明示した場合だけv4へ拡張します。
    let mut transformations = transformations.to_string();
    let mut artifacts = artifacts.to_string();
    let schema = if use_lir {
        if use_ssa {
            transformations = transformations.replace("\n  ],", concat!(
                ",\n    {\"order\": 2, \"kind\": \"representation\", \"pass\": \"mir-lir-v1\", \"options\": {}, \"input\": \"mir\", \"output\": \"lir\"},",
                "\n    {\"order\": 3, \"kind\": \"representation\", \"pass\": \"mir-lir-v1\", \"options\": {}, \"input\": \"ssa_lowered_mir\", \"output\": \"ssa_lir\"}\n  ],"
            ));
            transformations = transformations
                .replace(r#""assembly": "mir""#, r#""lir": "mir", "assembly": "lir""#)
                .replace(
                    r#""ssa_assembly": "ssa_lowered_mir""#,
                    r#""ssa_lir": "ssa_lowered_mir", "ssa_assembly": "ssa_lir""#,
                );
            artifacts.push_str("    \"ssa_lir\": \"program.ssa.lir.txt\",\n");
        } else {
            transformations = concat!(
                "  \"transformations\": [{\"order\": 0, \"kind\": \"representation\", \"pass\": \"mir-lir-v1\", \"options\": {}, \"input\": \"mir\", \"output\": \"lir\"}],\n",
                "  \"artifact_inputs\": {\"hir\": \"sources\", \"mir\": \"hir\", \"lir\": \"mir\", \"assembly\": \"lir\"},\n"
            ).into();
        }
        artifacts.push_str("    \"lir\": \"program.lir.txt\",\n");
        "cerune-observation-v4"
    } else {
        schema
    };
    let manifest = format!(
        concat!(
            "{{\n",
            "  \"schema\": \"{}\",\n",
            "  \"cerune_version\": \"{}\",\n",
            "  \"target\": \"{}\",\n",
            "  \"string_heap_limit\": {},\n",
            "  \"array_heap_limit\": {},\n",
            "  \"call_depth_limit\": {},\n",
            "  \"optimization_passes\": [],\n",
            "{}  \"executed\": false,\n",
            "  \"artifacts\": {{\n",
            "    \"sources\": \"sources.json\",\n",
            "    \"hir\": \"program.ceir\",\n",
            "    \"mir\": \"program.mir.txt\",\n",
            "{}    \"assembly\": \"program.origins.s\"\n",
            "  }}\n",
            "}}\n"
        ),
        schema,
        env!("CARGO_PKG_VERSION"),
        target.triple(),
        hir.string_heap_limit,
        hir.array_heap_limit,
        call_depth_limit,
        transformations,
        artifacts,
    );
    files.push(("manifest.json", manifest));
    write_new_directory(output, &files)
}

fn write_new_directory(output: &Path, files: &[(&str, String)]) -> Result<(), String> {
    // create_dirは既存ディレクトリやsymlinkも拒否し、親を暗黙に作りません。
    fs::create_dir(output).map_err(|e| {
        format!(
            "failed to create new observation directory {}: {e}",
            output.display()
        )
    })?;
    let mut created = Vec::new();
    for (name, content) in files {
        let path = output.join(name);
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            created.push(path.clone());
            file.write_all(content.as_bytes())
        })();
        if let Err(error) = result {
            // この操作が作ったファイルだけを削除します。他のファイルは消しません。
            let mut cleanup_failed = false;
            for path in created.iter().rev() {
                cleanup_failed |= fs::remove_file(path).is_err();
            }
            cleanup_failed |= fs::remove_dir(output).is_err();
            let suffix = if cleanup_failed {
                "; incomplete observation directory remains"
            } else {
                ""
            };
            return Err(format!(
                "failed to write {}: {error}{suffix}",
                path.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_write_removes_only_its_new_directory() {
        let root = std::env::temp_dir().join(format!(
            "cerune-bundle-write-failure-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let marker = root.join("keep.txt");
        fs::write(&marker, b"keep").unwrap();
        let output = root.join("bundle");
        let error = write_new_directory(
            &output,
            &[
                ("first.txt", "written".into()),
                ("missing/next.txt", "fail".into()),
            ],
        )
        .unwrap_err();
        assert!(error.contains("failed to write"));
        assert!(!output.exists());
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
        fs::remove_file(marker).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
