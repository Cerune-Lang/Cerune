use super::{
    Target,
    ir::{FailureOrigin, Instruction, Module},
};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    super::files::used(module)
        || module
            .instructions
            .iter()
            .chain(module.functions.iter().flat_map(|f| &f.instructions))
            .any(|i| {
                matches!(
                    i,
                    Instruction::ArgumentCount { .. } | Instruction::ArgumentGet { .. }
                )
            })
}
pub(super) fn origin_name(origin: FailureOrigin) -> String {
    format!(
        "cerune_arguments_origin_{}_{}_{}",
        origin.node.0,
        origin.span.start(),
        origin.span.end()
    )
}
fn data(out: &mut String, section: &str, name: &str, text: &str) {
    write!(out, "section \"{section}\" data ${name} = {{ ").unwrap();
    for (i, b) in text.bytes().chain([0]).enumerate() {
        if i != 0 {
            out.push_str(", ");
        }
        write!(out, "b {b}").unwrap();
    }
    out.push_str(" }\n");
}
pub(super) fn emit_support(module: &Module, out: &mut String) {
    if !used(module) {
        return;
    }
    let target = module.target.expect("arguments target");
    let section = target.read_only_section();
    let mut origins = Vec::new();
    for i in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::ArgumentCount { origin, .. } | Instruction::ArgumentGet { origin, .. } =
            i
            && !origins.contains(origin)
        {
            origins.push(*origin);
        }
    }
    for origin in origins {
        let span = origin.span;
        data(
            out,
            section,
            &origin_name(origin),
            &format!(
                " node={}{} bytes={}..{}\n",
                origin.node.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
        );
    }
    for code in [
        "invalid-limit",
        "count-limit-exceeded",
        "byte-limit-exceeded",
        "invalid-utf8",
        "embedded-nul",
        "invalid-unicode",
        "allocation-failed",
        "unconfigured",
        "invalid-index",
        "index-out-of-bounds",
        "budget-exceeded",
    ] {
        data(
            out,
            section,
            &format!("cerune_arguments_{}", code.replace('-', "_")),
            code,
        );
    }
    for (name, text) in [
        ("capture", "cerune: arguments-v1 phase=capture code=%s\n"),
        (
            "capture_index",
            "cerune: arguments-v1 phase=capture code=%s index=%lld\n",
        ),
        (
            "get",
            "cerune: arguments-v1 phase=execute operation=get code=%s index=%lld%s",
        ),
        (
            "count",
            "cerune: arguments-v1 phase=execute operation=count code=unconfigured%s",
        ),
    ] {
        data(
            out,
            section,
            &format!("cerune_arguments_format_{name}"),
            text,
        );
    }
    writeln!(out, "section \"{section}\" data $cerune_arguments_max_count = {{ l {} }}\nsection \"{section}\" data $cerune_arguments_max_bytes = {{ l {} }}", module.argument_limits.max_count, module.argument_limits.max_bytes).unwrap();
    if !out.contains("function w $cerune_utf8(") {
        out.push_str(super::utf8::SUPPORT);
    }
    out.push_str(FAILURE);
    out.push_str(if target == Target::X86_64PcWindowsMsvc {
        "  call $_write(w 2, l %text, w %count)\n"
    } else {
        "  %wide =l extuw %count\n  call $write(w 2, l %text, l %wide)\n"
    });
    out.push_str("  jmp @exit\n@exit\n  call $_Exit(w 1)\n  hlt\n}\n");
    out.push_str(SNAPSHOT);
    out.push_str(
        &GET.replace(
            "@LIMIT@",
            &module
                .string_heap_limit
                .expect("arguments string heap")
                .to_string(),
        ),
    );
}
const FAILURE: &str = r#"
function $cerune_arguments_failed(l %format, l %code, l %index, l %origin, w %flush) {
@start
  %text =l alloc8 512
  jnz %flush, @flush, @format
@flush
  call $fflush(l 0)
  jmp @format
@format
  %length =w call $snprintf(l %text, l 512, l %format, ..., l %code, l %index, l %origin)
  %positive =w csgtw %length, 0
  jnz %positive, @bound, @exit
@bound
  %fits =w cultw %length, 512
  jnz %fits, @fit, @limit
@fit
  jmp @write
@limit
  jmp @write
@write
  %count =w phi @fit %length, @limit 511
"#;
const SNAPSHOT: &str = r#"
data $cerune_arguments_values = { l 0 }
data $cerune_arguments_length = { l 0 }
data $cerune_arguments_configured = { w 0 }

function $cerune_arguments_capture_failed(l %code, l %index) {
@start
  %indexed =w csgel %index, 0
  jnz %indexed, @indexed, @plain
@indexed
  jmp @fail
@plain
  jmp @fail
@fail
  %format =l phi @indexed $cerune_arguments_format_capture_index, @plain $cerune_arguments_format_capture
  call $cerune_arguments_failed(l %format, l %code, l %index, l 0, w 0)
  hlt
}
function $cerune_arguments_free_values(l %values, l %count) {
@start
  jmp @loop
@loop
  %i =l phi @start 0, @advance %next
  %end =w ceql %i, %count
  jnz %end, @done, @value
@value
  %offset =l mul %i, 16
  %slot =l add %values, %offset
  %lenptr =l add %slot, 8
  %len =l loadl %lenptr
  %empty =w ceql %len, 0
  jnz %empty, @advance, @free
@free
  %data =l loadl %slot
  call $free(l %data)
  jmp @advance
@advance
  %next =l add %i, 1
  jmp @loop
@done
  call $free(l %values)
  ret
}
function $cerune_arguments_dispose() {
@start
  %values =l loadl $cerune_arguments_values
  %count =l loadl $cerune_arguments_length
  call $cerune_arguments_free_values(l %values, l %count)
  storel 0, $cerune_arguments_values
  storel 0, $cerune_arguments_length
  storew 0, $cerune_arguments_configured
  ret
}
# Validate all entries before copying and publishing the independent snapshot.
function $cerune_arguments_capture(l %count, l %values, l %max_count, l %max_bytes) {
@start
  %invalid =w cugtl %max_count, 9223372036854775807
  jnz %invalid, @invalid_limit, @check_count
@invalid_limit
  call $cerune_arguments_capture_failed(l $cerune_arguments_invalid_limit, l -1)
  hlt
@check_count
  %over =w cugtl %count, %max_count
  jnz %over, @count_failed, @validate
@count_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_count_limit_exceeded, l -1)
  hlt
@validate
  %i =l phi @check_count 0, @advance %next
  %bytes =l phi @check_count 0, @advance %total
  %end =w ceql %i, %count
  jnz %end, @table, @check_bytes
@check_bytes
  %offset =l mul %i, 16
  %slot =l add %values, %offset
  %data =l loadl %slot
  %lenptr =l add %slot, 8
  %len =l loadl %lenptr
  %space =l sub %max_bytes, %bytes
  %excess =w cugtl %len, %space
  jnz %excess, @bytes_failed, @check_utf8
@bytes_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_byte_limit_exceeded, l %i)
  hlt
@check_utf8
  %valid =w call $cerune_utf8(l %data, l %len)
  jnz %valid, @check_empty, @utf8_failed
@utf8_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_invalid_utf8, l %i)
  hlt
@check_empty
  %empty =w ceql %len, 0
  jnz %empty, @advance, @check_nul
@check_nul
  %nul =l call $memchr(l %data, w 0, l %len)
  %nul_nonnull =w cnel %nul, 0
  jnz %nul_nonnull, @nul_failed, @advance
@nul_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_embedded_nul, l %i)
  hlt
@advance
  %next =l add %i, 1
  %total =l add %bytes, %len
  jmp @validate
@table
  %no_entries =w ceql %count, 0
  jnz %no_entries, @copy_empty, @table_size
@table_size
  %too_large =w cugtl %count, 1152921504606846975
  jnz %too_large, @table_failed, @table_allocate
@table_allocate
  %size =l mul %count, 16
  %allocated =l call $malloc(l %size)
  %allocated_nonnull =w cnel %allocated, 0
  jnz %allocated_nonnull, @copy_table, @table_failed
@table_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_allocation_failed, l -1)
  hlt
@copy_empty
  jmp @copy
@copy_table
  jmp @copy
@copy
  %copied =l phi @copy_empty 0, @copy_table %allocated
  jmp @copy_loop
@copy_loop
  %j =l phi @copy 0, @copy_advance %j_next
  %finished =w ceql %j, %count
  jnz %finished, @publish, @copy_value
@copy_value
  %source_offset =l mul %j, 16
  %source_slot =l add %values, %source_offset
  %source_data =l loadl %source_slot
  %source_lenptr =l add %source_slot, 8
  %source_len =l loadl %source_lenptr
  %source_empty =w ceql %source_len, 0
  jnz %source_empty, @copy_zero, @copy_allocate
@copy_allocate
  %owned =l call $malloc(l %source_len)
  %owned_nonnull =w cnel %owned, 0
  jnz %owned_nonnull, @copy_bytes, @copy_failed
@copy_failed
  call $cerune_arguments_free_values(l %copied, l %j)
  call $cerune_arguments_capture_failed(l $cerune_arguments_allocation_failed, l %j)
  hlt
@copy_bytes
  call $memcpy(l %owned, l %source_data, l %source_len)
  jmp @copy_advance
@copy_zero
  jmp @copy_advance
@copy_advance
  %owned_data =l phi @copy_zero $cerune_string_empty, @copy_bytes %owned
  %destination =l add %copied, %source_offset
  storel %owned_data, %destination
  %destination_len =l add %destination, 8
  storel %source_len, %destination_len
  %j_next =l add %j, 1
  jmp @copy_loop
@publish
  call $cerune_arguments_dispose()
  storel %copied, $cerune_arguments_values
  storel %count, $cerune_arguments_length
  storew 1, $cerune_arguments_configured
  ret
}
function l $cerune_arguments_count(l %origin) {
@start
  %configured =w loadw $cerune_arguments_configured
  jnz %configured, @ready, @failed
@failed
  call $cerune_arguments_failed(l $cerune_arguments_format_count, l %origin, l 0, l 0, w 1)
  hlt
@ready
  %count =l loadl $cerune_arguments_length
  ret %count
}
"#;
const GET: &str = r#"
function l $cerune_arguments_get(l %index, l %origin) {
@start
  %configured =w loadw $cerune_arguments_configured
  jnz %configured, @check_index, @unconfigured
@unconfigured
  jmp @failed
@check_index
  %negative =w csltl %index, 0
  jnz %negative, @negative, @check_bounds
@negative
  jmp @failed
@check_bounds
  %count =l loadl $cerune_arguments_length
  %out =w cugel %index, %count
  jnz %out, @bounds, @value
@bounds
  jmp @failed
@value
  %values =l loadl $cerune_arguments_values
  %offset =l mul %index, 16
  %slot =l add %values, %offset
  %lenptr =l add %slot, 8
  %length =l loadl %lenptr
  %empty =w ceql %length, 0
  jnz %empty, @zero, @budget
@zero
  ret $cerune_string_empty
@budget
  %live =l loadl $cerune_string_live
  %space =l sub @LIMIT@, %live
  %excess =w cugtl %length, %space
  jnz %excess, @budget_failed, @size
@budget_failed
  jmp @failed
@size
  %overflow =w cugtl %length, -25
  jnz %overflow, @allocation_failed, @allocate
@allocate
  %bytes =l add %length, 24
  %owner =l call $malloc(l %bytes)
  %owner_nonnull =w cnel %owner, 0
  jnz %owner_nonnull, @copy, @allocation_failed
@allocation_failed
  jmp @failed
@copy
  %head =l loadl $cerune_string_owners
  storel %head, %owner
  %refs =l add %owner, 8
  storel 1, %refs
  %result =l add %owner, 16
  storel %length, %result
  %data =l add %owner, 24
  %source =l loadl %slot
  call $memcpy(l %data, l %source, l %length)
  storel %owner, $cerune_string_owners
  %total =l add %live, %length
  storel %total, $cerune_string_live
  ret %result
@failed
  %code =l phi @unconfigured $cerune_arguments_unconfigured, @negative $cerune_arguments_invalid_index, @bounds $cerune_arguments_index_out_of_bounds, @budget_failed $cerune_arguments_budget_exceeded, @allocation_failed $cerune_arguments_allocation_failed
  call $cerune_arguments_failed(l $cerune_arguments_format_get, l %code, l %index, l %origin, w 1)
  hlt
}
"#;
pub(super) fn emit_entry(module: &Module, out: &mut String) {
    if !used(module) {
        return;
    }
    let windows = module.target.expect("arguments target") == Target::X86_64PcWindowsMsvc;
    out.push_str(if windows { WINDOWS } else { LINUX });
    let entry = if super::files::used(module) {
        super::files::entry::adapt(ENTRY)
    } else {
        ENTRY.to_owned()
    };
    out.push_str(
        &entry
            .replace("@ENTRY@", if windows { "wmain" } else { "main" })
            .replace(
                "@TEMP_FREE@",
                if windows {
                    "  call $cerune_arguments_free_values(l %values, l %count)"
                } else {
                    "  call $free(l %values)"
                },
            ),
    );
    out.push_str("# End explicit target startup\n");
}
const LINUX: &str = r#"
function $cerune_arguments_os_value(l %text, l %index, l %result) {
@start
  %length =l call $strlen(l %text)
  %valid =w call $cerune_utf8(l %text, l %length)
  jnz %valid, @done, @failed
@failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_invalid_unicode, l %index)
  hlt
@done
  storel %text, %result
  %lenptr =l add %result, 8
  storel %length, %lenptr
  ret
}
"#;
const WINDOWS: &str = r#"
function $cerune_arguments_os_value(l %text, l %index, l %result) {
@start
  %size =w call $WideCharToMultiByte(w 65001, w 128, l %text, w -1, l 0, w 0, l 0, l 0)
  %invalid =w ceqw %size, 0
  jnz %invalid, @unicode_failed, @check_empty
@unicode_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_invalid_unicode, l %index)
  hlt
@check_empty
  %empty =w ceqw %size, 1
  jnz %empty, @zero, @allocate
@zero
  storel $cerune_string_empty, %result
  %zero_len =l add %result, 8
  storel 0, %zero_len
  ret
@allocate
  %wide =l extuw %size
  %data =l call $malloc(l %wide)
  %data_nonnull =w cnel %data, 0
  jnz %data_nonnull, @convert, @allocation_failed
@allocation_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_allocation_failed, l %index)
  hlt
@convert
  %written =w call $WideCharToMultiByte(w 65001, w 128, l %text, w -1, l %data, w %size, l 0, l 0)
  %valid =w ceqw %written, %size
  jnz %valid, @done, @unicode_failed
@done
  %length =l sub %wide, 1
  storel %data, %result
  %lenptr =l add %result, 8
  storel %length, %lenptr
  ret
}
"#;
const ENTRY: &str = r#"
# Explicit target startup: validate OS Unicode, capture, then execute Cerune.
export function w $@ENTRY@(w %argc, l %argv) {
@start
  %has_name =w csgtw %argc, 0
  jnz %has_name, @named, @nameless
@named
  %less =w sub %argc, 1
  jmp @size
@nameless
  jmp @size
@size
  %n =w phi @named %less, @nameless 0
  %count =l extuw %n
  %empty =w ceql %count, 0
  jnz %empty, @prepare_empty, @allocate
@allocate
  %size =l mul %count, 16
  %table =l call $malloc(l %size)
  %table_nonnull =w cnel %table, 0
  jnz %table_nonnull, @prepare_table, @allocation_failed
@allocation_failed
  call $cerune_arguments_capture_failed(l $cerune_arguments_allocation_failed, l -1)
  hlt
@prepare_empty
  jmp @prepare
@prepare_table
  jmp @prepare
@prepare
  %values =l phi @prepare_empty 0, @prepare_table %table
  jmp @loop
@loop
  %i =l phi @prepare 0, @value %next
  %end =w ceql %i, %count
  jnz %end, @capture, @value
@value
  %next =l add %i, 1
  %source_offset =l mul %next, 8
  %source_slot =l add %argv, %source_offset
  %text =l loadl %source_slot
  %destination_offset =l mul %i, 16
  %destination =l add %values, %destination_offset
  call $cerune_arguments_os_value(l %text, l %i, l %destination)
  jmp @loop
@capture
  %max_count =l loadl $cerune_arguments_max_count
  %max_bytes =l loadl $cerune_arguments_max_bytes
  call $cerune_arguments_capture(l %count, l %values, l %max_count, l %max_bytes)
@TEMP_FREE@
  %result =w call $cerune_program()
  call $cerune_arguments_dispose()
  ret %result
}
"#;
