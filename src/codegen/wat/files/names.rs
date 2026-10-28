//! 共通の名前集合を検査し、検査のために本文を書き換えません。
use std::fmt::Write;
pub(super) fn emit(out: &mut String) {
    out.push_str(SOURCE);
    for byte in [127, 60, 62, 58, 34, 47, 92, 124, 63, 42] {
        writeln!(
            out,
            "      (if (i32.eq (local.get $b) (i32.const {byte})) (then (return (i32.const 0))))"
        )
        .unwrap();
    }
    out.push_str(r#"
      (if (i32.and (i32.eq (local.get $b) (i32.const 46)) (i32.eq (local.get $stem) (local.get $length)))
        (then (local.set $stem (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $scan)))
    (block $trim_done (loop $trim
      (br_if $trim_done (i32.eqz (local.get $stem)))
      (br_if $trim_done (i32.ne (i32.load8_u (i32.add (local.get $data) (i32.sub (local.get $stem) (i32.const 1)))) (i32.const 32)))
      (local.set $stem (i32.sub (local.get $stem) (i32.const 1))) (br $trim)))
"#);
    for word in ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"] {
        let mut check = format!("(i32.eq (local.get $stem) (i32.const {}))", word.len());
        for (i, b) in word.bytes().enumerate() {
            check = format!(
                "(i32.and {check} (i32.eq (call $cerune_file_upper (i32.load8_u offset={i} (local.get $data))) (i32.const {b})))"
            );
        }
        // Do not load bytes beyond the length of short names.
        writeln!(out,"    (if (i32.eq (local.get $stem) (i32.const {})) (then (if {check} (then (return (i32.const 0))))))",word.len()).unwrap();
    }
    out.push_str(r#"
    (if (i32.ge_u (local.get $stem) (i32.const 4)) (then
      (local.set $b (call $cerune_file_upper (i32.load8_u (local.get $data))))
      (local.set $port (i32.or
        (i32.and (i32.eq (local.get $b) (i32.const 67)) (i32.and
          (i32.eq (call $cerune_file_upper (i32.load8_u offset=1 (local.get $data))) (i32.const 79))
          (i32.eq (call $cerune_file_upper (i32.load8_u offset=2 (local.get $data))) (i32.const 77))))
        (i32.and (i32.eq (local.get $b) (i32.const 76)) (i32.and
          (i32.eq (call $cerune_file_upper (i32.load8_u offset=1 (local.get $data))) (i32.const 80))
          (i32.eq (call $cerune_file_upper (i32.load8_u offset=2 (local.get $data))) (i32.const 84))))))
      (if (local.get $port) (then
        (local.set $b (i32.load8_u offset=3 (local.get $data)))
        (if (i32.and (i32.eq (local.get $stem) (i32.const 4)) (i32.lt_u (i32.sub (local.get $b) (i32.const 49)) (i32.const 9))) (then (return (i32.const 0))))
        (if (i32.and (i32.eq (local.get $stem) (i32.const 5)) (i32.eq (local.get $b) (i32.const 194))) (then
          (local.set $b (i32.load8_u offset=4 (local.get $data)))
          (if (i32.or (i32.eq (local.get $b) (i32.const 185)) (i32.or (i32.eq (local.get $b) (i32.const 178)) (i32.eq (local.get $b) (i32.const 179))))
            (then (return (i32.const 0))))))))))
    (i32.const 1))
"#);
}
const SOURCE: &str = r#"
  (func $cerune_file_upper (param $b i32) (result i32)
    (select (i32.sub (local.get $b) (i32.const 32)) (local.get $b)
      (i32.lt_u (i32.sub (local.get $b) (i32.const 97)) (i32.const 26))))
  (func $cerune_file_name_valid (param $name i32) (result i32)
    (local $length i32) (local $data i32) (local $i i32) (local $b i32) (local $stem i32) (local $port i32)
    (if (i64.gt_u (i64.sub (i64.load (local.get $name)) (i64.const 1)) (i64.const 254))
      (then (return (i32.const 0))))
    (local.set $length (i32.wrap_i64 (i64.load (local.get $name))))
    (local.set $data (i32.add (local.get $name) (i32.const 8)))
    (local.set $stem (local.get $length))
    (local.set $b (i32.load8_u (i32.add (local.get $data) (i32.sub (local.get $length) (i32.const 1)))))
    (if (i32.or (i32.eq (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 46)))
      (then (return (i32.const 0))))
    (block $scanned (loop $scan
      (br_if $scanned (i32.eq (local.get $i) (local.get $length)))
      (local.set $b (i32.load8_u (i32.add (local.get $data) (local.get $i))))
      (if (i32.lt_u (local.get $b) (i32.const 32)) (then (return (i32.const 0))))
"#;
