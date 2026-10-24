//! Produce the object and observations, without invoking a linker.
use super::json;
use cerune_lang::{
    codegen::x86_64::{self as native, Target},
    ir, mir,
    modules::Compilation,
    runtime::arguments::Limits,
};
use sashimi_assembler::program::Linkage;

pub(super) struct Generated {
    pub files: Vec<(&'static str, Vec<u8>)>,
}
pub(super) fn prepare(
    source: &Compilation,
    hir: &ir::Program,
    target: Target,
    ssa: bool,
    sources: bool,
    limits: Limits,
) -> Result<Generated, String> {
    let mut files = Vec::new();
    let mut text = |name, value: String| files.push((name, value.into_bytes()));
    text("program.ceir", ir::text::emit(hir));
    let baseline = mir::lower(hir).map_err(|e| source.render(&e.diagnostic()))?;
    text("program.mir.txt", mir::text::emit(&baseline));
    let lowered;
    let selected = if ssa {
        let ssa = mir::ssa::construct(&baseline).map_err(|e| source.render(&e.diagnostic()))?;
        text(
            "program.ssa.txt",
            mir::ssa::text::emit(&ssa).map_err(|e| source.render(&e.diagnostic()))?,
        );
        text(
            "program.ssa-map.txt",
            mir::ssa::mapping::emit(&ssa).map_err(|e| source.render(&e.diagnostic()))?,
        );
        lowered = mir::ssa::lower(&ssa).map_err(|e| source.render(&e.diagnostic()))?;
        text(
            "program.ssa-lowered.mir.txt",
            mir::text::emit(&lowered.program),
        );
        text("program.ssa-lowered-map.txt", lowered.mapping.clone());
        &lowered.program
    } else {
        &baseline
    };
    let lir = native::lower_mir_with_argument_limits(selected, target, limits)
        .map_err(|e| source.render(&e))?;
    text(
        "program.lir.txt",
        native::lir::emit(&lir).map_err(|e| source.render(&e))?,
    );
    let program = native::build_program_from_lir(&lir, true).map_err(|e| source.render(&e))?;
    text(
        "program.origins.s",
        program.render_asm().map_err(|e| source.render(&e))?,
    );
    text("program.machine.txt", format!("{:#?}\n", program.program()));
    let artifact = program.assemble().map_err(|e| source.render(&e))?;
    // Object offsets are explicitly not linked virtual addresses.
    text(
        "program.object-map.txt",
        format!(
            "cerune-object-map-v1\npositions: object-section-offsets\nentries: {:#?}\nsymbols: {:#?}\nreferences: {:#?}\n",
            artifact.entries, artifact.symbols, artifact.references
        ),
    );
    let origins = program.origins().iter().enumerate().map(|(id, origin)| {
        let source = match origin.source {
            native::ir::Origin::Synthetic => "null".into(),
            native::ir::Origin::Source { node_id, span } => format!("{{\"node\":{},\"file\":{},\"start\":{},\"end\":{}}}", node_id.0, span.source_id().index(), span.start(), span.end()),
        };
        format!("{{\"id\":{id},\"source\":{source},\"mir\":{},\"function\":{},\"lir_instruction\":{}}}", json(&format!("{:?}", origin.mir)), origin.function.map(|f| f.to_string()).unwrap_or("null".into()), origin.instruction)
    }).collect::<Vec<_>>().join(",");
    text(
        "program.origins.json",
        format!("{{\"schema\":\"cerune-native-origins-v1\",\"origins\":[{origins}]}}\n"),
    );
    let externals = program
        .program()
        .symbols
        .iter()
        .filter(|s| s.linkage == Linkage::External)
        .map(|s| json(&s.name))
        .collect::<Vec<_>>()
        .join(",");
    text(
        "runtime.json",
        format!(
            "{{\"schema\":\"cerune-native-runtime-v1\",\"policy\":\"target-system-runtime\",\"target\":{},\"external_symbols\":[{externals}],\"cerune_runtime\":\"in-object\",\"system_libraries_bundled\":false,\"requirements\":{}}}\n",
            json(target.triple()),
            json(match target {
                Target::X86_64UnknownLinuxGnu =>
                    "x86-64 Linux with the glibc, libm and ELF loader required by the selected Clang toolchain; no minimum glibc version or cross-system portability is promised",
                Target::X86_64PcWindowsMsvc =>
                    "x64 Windows with Win32 APIs and the MSVC/UCRT runtime selected by the Clang toolchain; no runtime redistributables are bundled",
            })
        ),
    );
    let index = source
        .sources
        .files()
        .map(|file| {
            format!(
                "{{\"id\":{},\"name\":{},\"utf8_bytes\":{},\"kind\":{}}}",
                if source.modular { file.id().index() } else { 0 },
                json(file.name()),
                file.text().len(),
                match file.kind() {
                    cerune_lang::source::SourceKind::User => json("user"),
                    cerune_lang::source::SourceKind::Standard {
                        identifier,
                        version,
                    } => format!("{{\"module\":{},\"version\":{version}}}", json(identifier)),
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    text(
        "source-index.json",
        format!("{{\"schema\":\"cerune-source-index-v1\",\"files\":[{index}]}}\n"),
    );
    if sources {
        text("sources.json", source.source_manifest());
    }
    files.push(("program.o", artifact.object));
    Ok(Generated { files })
}
