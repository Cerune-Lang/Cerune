//! Read-only Native constants. Preserve LIR bits and raw string contents.
use super::{
    ir::{FloatConstant, Module},
    machine::*,
};

/// Add data to an explicit-target Program; no assembly is parsed.
pub(super) fn append(program: &mut Program, module: &Module) {
    push(program, EntryKind::Section(Section::ReadOnlyData));
    let mask32 = program.declare(".Lcerune_sign_f32", Linkage::Local);
    push(program, EntryKind::Align(4));
    push(program, EntryKind::Label(mask32));
    for bits in [0x8000_0000, 0, 0, 0] {
        push(program, EntryKind::Data32(bits));
    }
    let mask64 = program.declare(".Lcerune_sign_f64", Linkage::Local);
    push(program, EntryKind::Align(4));
    push(program, EntryKind::Label(mask64));
    push(program, EntryKind::Data64(0x8000_0000_0000_0000));
    push(program, EntryKind::Data64(0));
    if module.uses_strings {
        for (id, value) in module.strings.iter().enumerate() {
            let label = program.declare(format!(".Lcerune_string_{id}"), Linkage::Local);
            push(program, EntryKind::Align(3));
            push(program, EntryKind::Label(label));
            push(
                program,
                EntryKind::Data64(u64::try_from(value.len()).expect("x86-64 string length")),
            );
            // Empty contents retain a zero-byte entry, distinct from the length header.
            push(program, EntryKind::Data(value.as_bytes().to_vec()));
        }
    }
    for constant in &module.float_constants {
        let (name, align, data) = match *constant {
            FloatConstant::F32 { id, bits } => {
                (format!(".Lcerune_f32_{id}"), 2, EntryKind::Data32(bits))
            }
            FloatConstant::F64 { id, bits } => {
                (format!(".Lcerune_f64_{id}"), 3, EntryKind::Data64(bits))
            }
        };
        let label = program.declare(name, Linkage::Local);
        push(program, EntryKind::Align(align));
        push(program, EntryKind::Label(label));
        push(program, data);
    }
}
#[cfg(test)]
pub(super) fn render(module: &Module) -> String {
    let mut output = if module.target.is_linux() {
        "# target: x86_64-unknown-linux-gnu\n".to_owned()
    } else {
        String::new()
    };
    output.push_str(&super::machine::render_support(module.target, |p| {
        append(p, module)
    }));
    output
}
#[cfg(test)]
#[path = "../../../tests/support/typed_native_data.rs"]
pub(super) mod tests;
