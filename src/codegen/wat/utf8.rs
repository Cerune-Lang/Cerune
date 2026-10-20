pub(super) const SUPPORT: &str = r#"
  (func $cerune_utf8 (param $data i32) (param $length i32) (result i32)
    (local $i i32) (local $b i32) (local $remaining i32) (local $low i32) (local $high i32)
    (local.set $low (i32.const 128))
    (local.set $high (i32.const 191))
    (block $done
      (loop $bytes
        (br_if $done (i32.eq (local.get $i) (local.get $length)))
        (local.set $b (i32.load8_u (i32.add (local.get $data) (local.get $i))))
        (if (local.get $remaining)
          (then
            (if (i32.or (i32.lt_u (local.get $b) (local.get $low)) (i32.gt_u (local.get $b) (local.get $high)))
              (then (return (i32.const 0))))
            (local.set $remaining (i32.sub (local.get $remaining) (i32.const 1)))
            (local.set $low (i32.const 128))
            (local.set $high (i32.const 191)))
          (else
            (if (i32.ge_u (local.get $b) (i32.const 128))
              (then
                (if (i32.lt_u (i32.sub (local.get $b) (i32.const 194)) (i32.const 30))
                  (then (local.set $remaining (i32.const 1)))
                  (else
                    (if (i32.lt_u (i32.sub (local.get $b) (i32.const 224)) (i32.const 16))
                      (then
                        (local.set $remaining (i32.const 2))
                        (if (i32.eq (local.get $b) (i32.const 224)) (then (local.set $low (i32.const 160))))
                        (if (i32.eq (local.get $b) (i32.const 237)) (then (local.set $high (i32.const 159)))))
                      (else
                        (if (i32.lt_u (i32.sub (local.get $b) (i32.const 240)) (i32.const 5))
                          (then
                            (local.set $remaining (i32.const 3))
                            (if (i32.eq (local.get $b) (i32.const 240)) (then (local.set $low (i32.const 144))))
                            (if (i32.eq (local.get $b) (i32.const 244)) (then (local.set $high (i32.const 143)))))
                          (else (return (i32.const 0))))))))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $bytes)))
    (i32.eqz (local.get $remaining)))
"#;
