use super::ir::{FailureOrigin, Instruction, Module};
use crate::codegen::call::Site;
use std::fmt::Write;
fn site(function: usize, origin: Option<FailureOrigin>) -> Site {
    Site {
        function,
        origin: origin.map(|o| (o.node, o.span)),
    }
}
pub(super) fn enter(m: &Module, function: usize, origin: Option<FailureOrigin>, out: &mut String) {
    if m.call_limit.is_some() && m.source_functions[function] {
        writeln!(
            out,
            "  call $cerune_call_enter(l ${})",
            site(function, origin).name()
        )
        .unwrap();
    }
}
pub(super) fn leave(m: &Module, function: usize, out: &mut String) {
    if m.call_limit.is_some() && m.source_functions[function] {
        out.push_str("  call $cerune_call_leave()\n");
    }
}
fn data(m: &Module, name: &str, value: &str, out: &mut String) {
    let bytes = value
        .bytes()
        .map(|b| format!("b {b}"))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(
        out,
        "section \"{}\" data ${name} = {{ {bytes} }}",
        m.target.unwrap().read_only_section()
    )
    .unwrap();
}
pub(super) fn support(m: &Module, out: &mut String) {
    let Some(limit) = m.call_limit else { return };
    let windows = m.target.expect("call target") == super::Target::X86_64PcWindowsMsvc;
    let mut sites = Vec::new();
    for i in m
        .instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::Call {
            function_id,
            origin,
            ..
        } = i
            && m.source_functions[*function_id]
        {
            let s = site(*function_id, Some(*origin));
            if !sites.contains(&s) {
                sites.push(s);
            }
        }
    }
    if let Some(id) = m.explicit_main
        && m.source_functions[id]
    {
        sites.push(site(id, None));
    }
    for s in sites {
        let name = s.name();
        let suffix = s.suffix();
        let allocation = s.allocation();
        data(m, &format!("{name}_text"), &suffix, out);
        data(m, &format!("{name}_alloc"), &allocation, out);
        writeln!(
            out,
            "section \"{}\" data ${name} = {{l ${name}_text,l {},l ${name}_alloc,l {}}}",
            m.target.unwrap().read_only_section(),
            suffix.len(),
            allocation.len()
        )
        .unwrap();
    }
    let header = format!("call-v1 reason=depth-limit limit={limit}");
    data(m, "cerune_call_header", &header, out);
    data(m, "cerune_call_frame_prefix", "call-v1 frame=", out);
    out.push_str("function $cerune_call_write(l %p, l %n) {\n@start\n");
    out.push_str(if windows {
        "  call $_write(w 2, l %p, w %n)\n"
    } else {
        "  call $write(w 2, l %p, l %n)\n"
    });
    out.push_str("  ret\n}\nfunction $cerune_call_flush() {\n@start\n");
    out.push_str(if windows {
        "  %stream =l call $__acrt_iob_func(w 1)\n"
    } else {
        "  %stream =l loadl $stdout\n"
    });
    out.push_str("  call $fflush(l %stream)\n  ret\n}\n");
    out.push_str(
        &SOURCE
            .replace("$LIMIT", &limit.to_string())
            .replace("$HEADER_LENGTH", &header.len().to_string()),
    );
}
const SOURCE: &str = r#"
data $cerune_call_depth = {l 0}
data $cerune_call_capacity = {l 0}
data $cerune_call_frames = {l 0}
function $cerune_call_site_write(l %site, l %field) {
@start
  %offset =l mul %field, 16
  %pair =l add %site, %offset
  %length_slot =l add %pair, 8
  %p =l loadl %pair
  %n =l loadl %length_slot
  call $cerune_call_write(l %p, l %n)
  ret
}
function $cerune_call_number(l %number) {
@start
  %buffer =l alloc8 24
  jmp @digits
@digits
  %v =l phi @start %number, @digits %q
  %end =l phi @start 20, @digits %index
  %q =l udiv %v, 10
  %r =l urem %v, 10
  %ascii =l add %r, 48
  %index =l sub %end, 1
  %p =l add %buffer, %index
  storeb %ascii, %p
  %more =w cnel %q, 0
  jnz %more, @digits, @done
@done
  %length =l sub 20, %index
  call $cerune_call_write(l %p, l %length)
  ret
}
function $cerune_call_enter(l %site) {
@start
  %depth =l loadl $cerune_call_depth
  %full =w cugel %depth, $LIMIT
  jnz %full, @overflow, @capacity
@overflow
  call $cerune_call_flush()
  call $cerune_call_write(l $cerune_call_header, l $HEADER_LENGTH)
  call $cerune_call_site_write(l %site, l 0)
  %history =l loadl $cerune_call_frames
  jmp @history_loop
@history_loop
  %i =l phi @overflow 0, @history_loop %next_i
  call $cerune_call_write(l $cerune_call_frame_prefix, l 14)
  call $cerune_call_number(l %i)
  %offset =l mul %i, 8
  %slot =l add %history, %offset
  %saved =l loadl %slot
  call $cerune_call_site_write(l %saved, l 0)
  %next_i =l add %i, 1
  %more =w cultl %next_i, %depth
  jnz %more, @history_loop, @stop
@stop
  call $_Exit(w 1)
  hlt
@capacity
  %cap =l loadl $cerune_call_capacity
  %grow =w ceql %depth, %cap
  jnz %grow, @allocate, @push
@allocate
  %double =l mul %cap, 2
  %zero =w ceql %cap, 0
  jnz %zero, @initial, @grown
@initial
  jmp @bound
@grown
  jmp @bound
@bound
  %want =l phi @initial 8, @grown %double
  %exceeds =w cugtl %want, $LIMIT
  jnz %exceeds, @capped, @wanted
@capped
  jmp @resize
@wanted
  jmp @resize
@resize
  %next_cap =l phi @capped $LIMIT, @wanted %want
  %bytes =l mul %next_cap, 8
  %old =l loadl $cerune_call_frames
  %new =l call $realloc(l %old, l %bytes)
  %failed =w ceql %new, 0
  jnz %failed, @allocation_failed, @allocated
@allocation_failed
  call $cerune_call_flush()
  call $cerune_call_site_write(l %site, l 1)
  call $_Exit(w 1)
  hlt
@allocated
  storel %new, $cerune_call_frames
  storel %next_cap, $cerune_call_capacity
  jmp @push
@push
  %frames =l loadl $cerune_call_frames
  %destination_offset =l mul %depth, 8
  %destination =l add %frames, %destination_offset
  storel %site, %destination
  %entered =l add %depth, 1
  storel %entered, $cerune_call_depth
  ret
}
function $cerune_call_leave() {
@start
  %depth =l loadl $cerune_call_depth
  %left =l sub %depth, 1
  storel %left, $cerune_call_depth
  ret
}
function $cerune_call_finish() {
@start
  %frames =l loadl $cerune_call_frames
  call $free(l %frames)
  storel 0, $cerune_call_frames
  storel 0, $cerune_call_capacity
  ret
}
"#;
