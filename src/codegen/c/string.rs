use std::fmt::Write;

use super::ir::{Module, Type};

pub(super) fn uses_type(module: &Module) -> bool {
    fn contains(ty: &Type) -> bool {
        match ty {
            Type::String => true,
            Type::Array { element, .. } | Type::DynamicArray { element } => contains(element),
            Type::Bool | Type::U64 | Type::I64 | Type::Float | Type::Double | Type::Named(_) => {
                false
            }
        }
    }
    module
        .type_definitions
        .iter()
        .any(|definition| definition.fields.iter().any(|field| contains(&field.ty)))
        || module.array_types.iter().any(contains)
        || module.functions.iter().any(|function| {
            function.return_type.as_ref().is_some_and(contains)
                || function
                    .parameters
                    .iter()
                    .any(|parameter| contains(&parameter.ty))
        })
}

/// C文字列リテラルの静的な保存期間を使い、関数から返してもデータを失いません。
pub(super) fn literal(value: &str, output: &mut String) {
    output.push_str("(cerune_string){ (const unsigned char *)\"");
    // 全バイトを固定3桁の8進表記にし、Cの文字コード設定や後続文字に左右されません。
    for byte in value.bytes() {
        write!(output, "\\{byte:03o}").unwrap();
    }
    write!(output, "\", {} }}", value.len()).unwrap();
}

pub(super) const SUPPORT: &str = r#"typedef struct cerune_string {
    const unsigned char *data;
    size_t length;
} cerune_string;

static inline bool cerune_string_equal(cerune_string left, cerune_string right) {
    return left.length == right.length &&
        (left.length == 0 || memcmp(left.data, right.data, left.length) == 0);
}

"#;

pub(super) const BYTE_AT: &str = r#"
static int64_t cerune_string_byte_at(cerune_string value, int64_t index) {
    if (index < 0 || (uint64_t)index >= value.length) return -1;
    return (int64_t)value.data[(size_t)index];
}
"#;
