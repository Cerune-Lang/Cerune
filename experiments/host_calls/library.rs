//! Generate a checked WAT library and its observable representations.
use cerune_lang::{bytecode, codegen::wat, ir, modules};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err("usage: wat_library source.ceru output-directory export [export ...]".into());
    }
    let compilation = modules::load(std::path::Path::new(&args[0])).map_err(|e| e.render())?;
    let selected = args[2..].iter().map(String::as_str).collect::<Vec<_>>();
    let library = wat::emit_library(&compilation, &selected).map_err(|e| compilation.render(&e))?;
    let dir = PathBuf::from(&args[1]);
    let bytecode = bytecode::lower(library.program()).map_err(|e| compilation.render(&e))?;
    let plain = wat::emit_wat(library.program()).map_err(|e| compilation.render(&e))?;
    fs::create_dir_all(&dir)?;
    for (name, contents) in [
        ("library.wat", library.wat().to_owned()),
        ("exports.json", library.export_manifest()),
        ("library.mjs", library.javascript()),
        ("library.d.mts", library.typescript()),
        ("sources.json", compilation.source_manifest()),
        ("source.ceir", ir::text::emit(library.program())),
        ("source.cebc", bytecode::format_program(&bytecode)),
        ("plain.wat", plain),
    ] {
        fs::write(dir.join(name), contents)?;
    }
    Ok(())
}
