//! Inspect direct runtime requirements without loading or executing the binary.
use cerune_lang::codegen::x86_64::Target;
use serde_json::{Value, json};

pub(super) fn inspect(bytes: &[u8], target: Target) -> Result<Value, String> {
    let details = match target {
        Target::X86_64UnknownLinuxGnu => {
            let elf =
                goblin::elf::Elf::parse(bytes).map_err(|e| format!("invalid release ELF: {e}"))?;
            if !elf.is_64
                || !elf.little_endian
                || elf.header.e_machine != 62
                || elf.header.e_type != 3
                || elf.entry == 0
            {
                return Err("release requires an x86-64 Linux PIE executable".into());
            }
            let interpreter = elf
                .interpreter
                .filter(|s| !s.is_empty())
                .ok_or("ELF loader requirement is missing")?;
            let dynamic = elf.dynamic.as_ref().ok_or("missing ELF dynamic table")?;
            if dynamic
                .dyns
                .iter()
                .filter(|d| d.d_tag == goblin::elf::dynamic::DT_NEEDED)
                .count()
                != elf.libraries.len()
            {
                return Err("invalid ELF library names".into());
            }
            if !elf.rpaths.is_empty() || !elf.runpaths.is_empty() {
                return Err("release does not yet support ELF RPATH/RUNPATH dependencies".into());
            }
            let mut versions = Vec::new();
            let mut files = 0;
            if let Some(section) = &elf.verneed {
                for need in section.iter() {
                    files += 1;
                    if need.vn_version != 1 {
                        return Err("unsupported ELF version requirement format".into());
                    }
                    let library = elf
                        .dynstrtab
                        .get_at(need.vn_file)
                        .ok_or("invalid ELF version library")?;
                    let mut entries = Vec::new();
                    for aux in need.iter() {
                        let name = elf
                            .dynstrtab
                            .get_at(aux.vna_name)
                            .ok_or("invalid ELF version name")?;
                        entries.push(json!({"name":name,"flags":aux.vna_flags}));
                    }
                    if entries.len() != need.vn_cnt as usize {
                        return Err("truncated ELF version requirements".into());
                    }
                    versions.push(json!({"library":library,"versions":entries}));
                }
            }
            if files != dynamic.info.verneednum {
                return Err("missing or truncated ELF version requirement sections".into());
            }
            json!({"format":"ELF64-PIE","interpreter":interpreter,"libraries":elf.libraries,"symbol_versions":versions,"rpaths":elf.rpaths,"runpaths":elf.runpaths})
        }
        Target::X86_64PcWindowsMsvc => {
            let pe =
                goblin::pe::PE::parse(bytes).map_err(|e| format!("invalid release PE: {e}"))?;
            if !pe.is_64
                || pe.is_lib
                || pe.header.coff_header.machine != 0x8664
                || pe.header.coff_header.characteristics & 2 == 0
                || pe.entry == 0
            {
                return Err("release requires an x86-64 Windows executable".into());
            }
            let optional = pe
                .header
                .optional_header
                .ok_or("missing PE optional header")?;
            if optional
                .data_directories
                .get_delay_import_descriptor()
                .is_some()
            {
                return Err("release does not yet support PE delay imports".into());
            }
            // Goblin's synthesized Import.ordinal also holds hints for named imports.
            // Preserve the lookup kind instead of misreporting a hint as an ordinal.
            use goblin::pe::import::SyntheticImportLookupTableEntry::{
                HintNameTableRVA, OrdinalNumber,
            };
            let mut imports = Vec::new();
            if let Some(data) = &pe.import_data {
                for library in &data.import_data {
                    let lookup = library
                        .import_lookup_table
                        .as_ref()
                        .ok_or("missing PE import lookup table")?;
                    for entry in lookup {
                        imports.push(match entry {
                            HintNameTableRVA((_, name)) => json!({"library":library.name,"kind":"name","name":name.name,"hint":name.hint}),
                            OrdinalNumber(ordinal) => json!({"library":library.name,"kind":"ordinal","ordinal":ordinal}),
                        });
                    }
                }
            }
            let fields = optional.windows_fields;
            json!({"format":"PE32+","libraries":pe.libraries,
                "imports":imports,
                "subsystem":fields.subsystem,
                "header_os_version":[fields.major_operating_system_version,fields.minor_operating_system_version],
                "header_subsystem_version":[fields.major_subsystem_version,fields.minor_subsystem_version]})
        }
    };
    Ok(
        json!({"schema":"cerune-native-requirements-v1","target":target.triple(),
        "policy":"target-system-runtime","inspection":"binary-metadata-only","executed":false,
        "libraries_bundled":false,"transitive_dependencies_resolved":false,"deployment_verified":false,
        "direct":details}),
    )
}
