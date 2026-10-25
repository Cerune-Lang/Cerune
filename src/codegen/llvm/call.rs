use super::ir::{Instruction, Module, Origin};
use crate::codegen::call::Site;
use std::fmt::Write;
fn site(function: usize, origin: Origin) -> Site {
    Site {
        function,
        origin: match origin {
            Origin::Source { node_id, span } => Some((node_id, span)),
            Origin::Synthetic => None,
        },
    }
}
pub(super) fn enter(m: &Module, function: usize, origin: Origin, out: &mut String) {
    if m.call_limit.is_some() && m.source_functions[function] {
        writeln!(
            out,
            "  call void @cerune_call_enter(ptr @{})",
            site(function, origin).name()
        )
        .unwrap();
    }
}
pub(super) fn leave(m: &Module, function: usize, out: &mut String) {
    if m.call_limit.is_some() && m.source_functions[function] {
        out.push_str("  call void @cerune_call_leave()\n");
    }
}
fn data(name: &str, value: &str, out: &mut String) {
    let bytes: String = value.bytes().map(|b| format!("\\{b:02X}")).collect();
    writeln!(
        out,
        "@{name} = private constant [{} x i8] c\"{bytes}\"",
        value.len()
    )
    .unwrap();
}
pub(super) fn support(m: &Module, out: &mut String) {
    let Some(limit) = m.call_limit else { return };
    let windows = m.target.expect("call limit target") == super::Target::X86_64PcWindowsMsvc;
    for declaration in [
        "declare ptr @realloc(ptr, i64)\n",
        "declare void @free(ptr)\n",
        "declare void @_Exit(i32) noreturn\n",
        "declare i32 @fflush(ptr)\n",
        if windows {
            "declare ptr @__acrt_iob_func(i32)\n"
        } else {
            "@stdout = external global ptr\n"
        },
        if windows {
            "declare i32 @_write(i32, ptr, i32)\n"
        } else {
            "declare i64 @write(i32, ptr, i64)\n"
        },
    ] {
        if !out.contains(declaration) {
            out.push_str(declaration);
        }
    }
    let mut sites = Vec::new();
    for i in m
        .instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::Call { function_id, .. } = i.instruction
            && m.source_functions[function_id]
        {
            let s = site(function_id, i.origin);
            if !sites.contains(&s) {
                sites.push(s);
            }
        }
    }
    if let Some(id) = m.explicit_main
        && m.source_functions[id]
    {
        sites.push(site(id, Origin::Synthetic));
    }
    for s in sites {
        let name = s.name();
        let suffix = s.suffix();
        let allocation = s.allocation();
        data(&format!("{name}_text"), &suffix, out);
        data(&format!("{name}_alloc"), &allocation, out);
        writeln!(out,"@{name} = private constant {{ptr,i64,ptr,i64}} {{ptr @{name}_text,i64 {},ptr @{name}_alloc,i64 {}}}",suffix.len(),allocation.len()).unwrap();
    }
    let header = format!("call-v1 reason=depth-limit limit={limit}");
    data("cerune_call_header", &header, out);
    data("cerune_call_frame_prefix", "call-v1 frame=", out);
    out.push_str("define internal void @cerune_call_write(ptr %p, i64 %n) {\nentry:\n");
    out.push_str(if windows {
        "  %n32 = trunc i64 %n to i32\n  call i32 @_write(i32 2, ptr %p, i32 %n32)\n"
    } else {
        "  call i64 @write(i32 2, ptr %p, i64 %n)\n"
    });
    out.push_str("  ret void\n}\ndefine internal void @cerune_call_flush() {\nentry:\n");
    out.push_str(if windows {
        "  %stream = call ptr @__acrt_iob_func(i32 1)\n"
    } else {
        "  %stream = load ptr, ptr @stdout\n"
    });
    out.push_str("  call i32 @fflush(ptr %stream)\n  ret void\n}\n");
    out.push_str(
        &SOURCE
            .replace("$LIMIT", &limit.to_string())
            .replace("$HEADER_LENGTH", &header.len().to_string()),
    );
}
const SOURCE: &str = r#"
@cerune_call_depth = internal global i64 0
@cerune_call_capacity = internal global i64 0
@cerune_call_frames = internal global ptr null
; History stores immutable call-site pointers, separate from language heap budgets.
define internal void @cerune_call_site_write(ptr %site, i64 %field) {
entry:
  %p = getelementptr {ptr,i64,ptr,i64}, ptr %site, i64 0, i32 0
  %pair = getelementptr {ptr,i64}, ptr %p, i64 %field
  %record = load {ptr,i64}, ptr %pair
  %data = extractvalue {ptr,i64} %record, 0
  %len = extractvalue {ptr,i64} %record, 1
  call void @cerune_call_write(ptr %data, i64 %len)
  ret void
}
define internal void @cerune_call_number(i64 %number) {
entry:
  %buffer = alloca [20 x i8]
  br label %digits
digits:
  %v = phi i64 [%number,%entry],[%q,%digits]
  %end = phi i64 [20,%entry],[%index,%digits]
  %q = udiv i64 %v, 10
  %r = urem i64 %v, 10
  %ascii = add i64 %r, 48
  %byte = trunc i64 %ascii to i8
  %index = sub i64 %end, 1
  %p = getelementptr i8, ptr %buffer, i64 %index
  store i8 %byte, ptr %p
  %more = icmp ne i64 %q, 0
  br i1 %more, label %digits, label %done
done:
  %length = sub i64 20, %index
  call void @cerune_call_write(ptr %p, i64 %length)
  ret void
}
define internal void @cerune_call_enter(ptr %site) {
entry:
  %depth = load i64, ptr @cerune_call_depth
  %full = icmp uge i64 %depth, $LIMIT
  br i1 %full, label %overflow, label %capacity
overflow:
  call void @cerune_call_flush()
  call void @cerune_call_write(ptr @cerune_call_header, i64 $HEADER_LENGTH)
  call void @cerune_call_site_write(ptr %site, i64 0)
  %history = load ptr, ptr @cerune_call_frames
  br label %history_loop
history_loop:
  %i = phi i64 [0,%overflow],[%next_i,%history_loop]
  call void @cerune_call_write(ptr @cerune_call_frame_prefix, i64 14)
  call void @cerune_call_number(i64 %i)
  %slot = getelementptr ptr, ptr %history, i64 %i
  %saved = load ptr, ptr %slot
  call void @cerune_call_site_write(ptr %saved, i64 0)
  %next_i = add i64 %i, 1
  %more = icmp ult i64 %next_i, %depth
  br i1 %more, label %history_loop, label %stop
stop:
  call void @_Exit(i32 1)
  unreachable
capacity:
  %cap = load i64, ptr @cerune_call_capacity
  %grow = icmp eq i64 %depth, %cap
  br i1 %grow, label %allocate, label %push
allocate:
  %zero = icmp eq i64 %cap, 0
  %double = mul i64 %cap, 2
  %want = select i1 %zero, i64 8, i64 %double
  %exceeds = icmp ugt i64 %want, $LIMIT
  %next_cap = select i1 %exceeds, i64 $LIMIT, i64 %want
  %bytes = mul i64 %next_cap, 8
  %old = load ptr, ptr @cerune_call_frames
  %new = call ptr @realloc(ptr %old, i64 %bytes)
  %failed = icmp eq ptr %new, null
  br i1 %failed, label %allocation_failed, label %allocated
allocation_failed:
  call void @cerune_call_flush()
  call void @cerune_call_site_write(ptr %site, i64 1)
  call void @_Exit(i32 1)
  unreachable
allocated:
  store ptr %new, ptr @cerune_call_frames
  store i64 %next_cap, ptr @cerune_call_capacity
  br label %push
push:
  %frames = load ptr, ptr @cerune_call_frames
  %destination = getelementptr ptr, ptr %frames, i64 %depth
  store ptr %site, ptr %destination
  %entered = add i64 %depth, 1
  store i64 %entered, ptr @cerune_call_depth
  ret void
}
define internal void @cerune_call_leave() {
entry:
  %depth = load i64, ptr @cerune_call_depth
  %left = sub i64 %depth, 1
  store i64 %left, ptr @cerune_call_depth
  ret void
}
define internal void @cerune_call_finish() {
entry:
  %frames = load ptr, ptr @cerune_call_frames
  call void @free(ptr %frames)
  store ptr null, ptr @cerune_call_frames
  store i64 0, ptr @cerune_call_capacity
  ret void
}
"#;
