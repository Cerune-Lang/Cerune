use super::*;
fn empty() -> *mut Context {
    let mut c = ptr::null_mut();
    assert_eq!(
        unsafe {
            cerune_process_v1_create(
                1,
                if cfg!(windows) { 1 } else { 2 },
                Bytes {
                    data: ptr::null(),
                    len: 0,
                },
                &mut c,
            )
        },
        OK
    );
    c
}
#[test]
fn caught_panic_poisons_context_and_destroy_remains_available() {
    let mut c = empty();
    assert_eq!(
        unsafe { with_context(c, |_| panic!("injected ABI test panic")) },
        PANIC
    );
    let mut words = [0; 32];
    assert_eq!(
        unsafe { cerune_process_v1_report(c, words.as_mut_ptr(), 32) },
        STATE
    );
    assert_eq!(unsafe { cerune_process_v1_release(c) }, STATE);
    assert_eq!(unsafe { cerune_process_v1_destroy(&mut c) }, OK);
    assert!(c.is_null());
}
#[test]
fn request_owns_arguments_after_move_and_source_drop() {
    let owned = {
        let target = String::from("fixture");
        let argument = String::from("日本語\0\r\n");
        let args = [Bytes {
            data: argument.as_ptr(),
            len: argument.len() as u64,
        }];
        let input = [255u8, 0];
        let r = RequestV1 {
            version: 1,
            size: size_of::<RequestV1>() as u64,
            target: Bytes {
                data: target.as_ptr(),
                len: target.len() as u64,
            },
            arguments: args.as_ptr(),
            argument_count: 1,
            stdin: Bytes {
                data: input.as_ptr(),
                len: 2,
            },
            stdout_limit: 0,
            stderr_limit: 0,
            timeout_ms: u64::MAX,
            node: 0,
            source: 0,
            start: 0,
            end: 0,
        };
        unsafe { OwnedRequest::copy(&r).unwrap() }
    };
    // 移動・元の領域解放後もviewの寿命はOwnedRequestの借用に限定します。
    let moved = Box::new(owned);
    let r = moved.view();
    assert_eq!(r.target, "fixture");
    assert_eq!(r.arguments, ["日本語\0\r\n"]);
    assert_eq!(r.stdin, b"\xff\0");
    assert_eq!(r.timeout, Duration::from_millis(u64::MAX));
}
#[test]
fn native_layout_and_invalid_pointer_shapes_are_explicit() {
    assert_eq!(size_of::<Bytes>(), 16);
    assert_eq!(size_of::<RequestV1>(), 120);
    assert_eq!(std::mem::offset_of!(RequestV1, stdin), 48);
    assert_eq!(std::mem::offset_of!(RequestV1, node), 88);
    assert!(matches!(
        unsafe { input_slice::<u64>(ptr::null(), 1) },
        Err(SHAPE)
    ));
    assert!(matches!(
        unsafe { input_slice::<u64>(ptr::null(), u64::MAX) },
        Err(RANGE)
    ));
    assert_eq!(
        unsafe { cerune_process_v1_prepare(ptr::null_mut(), ptr::null()) },
        SHAPE
    );
}
