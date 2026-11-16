#![cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
#[path = "../src/process.rs"]
mod host;
use host::Process;
use serde_json::json;
use wasmi::{Engine, Linker, Module, Store};
fn target() -> u64 {
    if cfg!(windows) { 1 } else { 2 }
}
fn empty() -> Process {
    Process::new(target(), &[]).unwrap()
}
fn field(p: &mut Process, kind: i32, index: i64, value: &[u8]) {
    assert_eq!(p.field(kind, index, value.len() as i64), 0);
    for chunk in value.chunks(8) {
        let mut bits = [0; 8];
        bits[..chunk.len()].copy_from_slice(chunk);
        assert_eq!(p.chunk(i64::from_le_bytes(bits), chunk.len() as i32), 0);
    }
}
fn request(p: &mut Process, name: &[u8], input: &[u8], timeout: u64) {
    assert_eq!(p.begin([1, 32, 32, timeout as i64, 123, 9, 30, 41]), 0);
    field(p, 0, 0, name);
    field(p, 1, 0, "日本語\r\n".as_bytes());
    field(p, 2, 0, input);
}
fn report(p: &Process) -> [u64; 32] {
    std::array::from_fn(|i| {
        let (word, status) = p.report(i as i32);
        assert_eq!(status, 0);
        word as u64
    })
}
#[test]
fn transport_preserves_u64_origins_and_common_validation_order() {
    let mut p = empty();
    request(
        &mut p,
        b"fixture",
        &(0..=255).collect::<Vec<u8>>(),
        u64::MAX,
    );
    assert_eq!(p.prepare(), 2);
    let mut expected = [0u64; 32];
    expected[..12].copy_from_slice(&[1, 32, 123, 9, 30, 41, 0, 256, 0, 32, 32, u64::MAX]);
    expected[16] = 1;
    expected[19] = 3;
    expected[20] = 1; // Unconfigured / Validate.
    assert_eq!(report(&p), expected);
    for i in (0..32).rev() {
        assert_eq!(p.report(i), (expected[i as usize] as i64, 0));
    }
    assert_eq!(p.copy(1, 0, 8), (0, 0, 0));
    assert_eq!(p.copy(2, 0, 1), (0, 0, 0));
    assert_eq!(p.copy(1, 1, 1), (0, 0, 13));
    assert_eq!(p.report(32), (0, 13));
    assert_eq!(p.execute(0), 12);
    assert_eq!(report(&p), expected);
    assert_eq!(p.release(), 0);
    for name in [b"".as_slice(), b"bad\0name"] {
        request(&mut p, name, &[], 1);
        assert_eq!(p.prepare(), 2);
        assert_eq!(report(&p)[19], 1); // InvalidRequest precedes Unconfigured.
        assert_eq!(p.release(), 0);
    }
    assert_eq!(p.begin([0, -1, 1, 1, (1i64 << 54) + 3, 9, 30, 41]), 0);
    field(&mut p, 0, 0, b"fixture");
    field(&mut p, 2, 0, &[]);
    assert_eq!(p.prepare(), 2);
    assert_eq!(report(&p)[2], (1u64 << 54) + 3);
    assert_eq!(report(&p)[9], u64::MAX);
    assert_eq!(report(&p)[19], 2); // Sum overflow is common-runtime RequestLimit.
}
#[test]
fn rejects_incomplete_duplicate_out_of_order_and_invalid_transport() {
    let mut p = empty();
    assert_eq!(p.prepare(), 12);
    assert_eq!(p.release(), 12);
    assert_eq!(p.begin([-1, 0, 0, 1, 0, 0, 0, 0]), 13);
    assert_eq!(p.begin([0, 0, 0, 1, 0, 0, 2, 1]), 13);
    assert_eq!(p.begin([1, 0, 0, 1, 0, 0, 0, 0]), 0);
    assert_eq!(p.begin([0; 8]), 12);
    assert_eq!(p.field(1, 0, 0), 12);
    assert_eq!(p.chunk(0, 1), 12);
    assert_eq!(p.field(0, 0, -1), 13);
    assert_eq!(p.field(0, 0, 2), 0);
    assert_eq!(p.chunk(0, 0), 13);
    assert_eq!(p.chunk(0, 9), 13);
    assert_eq!(p.chunk(0, 3), 11);
    assert_eq!(p.chunk(0x61, 1), 0);
    assert_eq!(p.field(1, 0, 0), 12);
    assert_eq!(p.prepare(), 12);
    assert_eq!(p.chunk(0x62, 1), 0);
    assert_eq!(p.chunk(0, 1), 11);
    assert_eq!(p.field(1, 1, 0), 12);
    field(&mut p, 1, 0, b"");
    field(&mut p, 2, 0, b"");
    assert_eq!(p.field(2, 0, 0), 12);
    assert_eq!(p.prepare(), 2);
    assert_eq!(p.prepare(), 12);
    assert_eq!(p.field(0, 0, 0), 12);
    assert_eq!(p.release(), 0);
    request(&mut p, b"\xff", &[], 1);
    assert_eq!(p.prepare(), 14); // Malformed UTF-8 is ABI failure, not an Outcome.
    assert_eq!(p.release(), 0);
    assert_eq!(p.report(0), (0, 12));
    request(&mut p, b"fixture", &[], 0);
    assert_eq!(p.prepare(), 2);
    assert_eq!(report(&p)[19], 1);
}
fn configured() -> Process {
    // The supervisor path is explicit but never launched in reservation/cancellation tests.
    let config = json!({"schema":"cerune-process-v1","platform":if cfg!(windows){"windows"}else{"linux"},
      "linux_supervisor":if cfg!(windows){serde_json::Value::Null}else{json!(std::env::current_exe().unwrap())},
      "limits":{"arguments":8,"argument_bytes":16384,"stdin_bytes":4096,"output_bytes":4096,"timeout_ms":10000,"cleanup_timeout_ms":1000},
      "transfer_bytes":32768,"targets":[]});
    Process::new(target(), &serde_json::to_vec(&config).unwrap()).unwrap()
}
#[test]
fn reservation_and_cancellation_use_common_runtime_without_launching() {
    let mut p = configured();
    for reservation in [1, 2] {
        request(&mut p, b"unregistered", b"body", 1000);
        assert_eq!(p.prepare(), 1);
        let before = report(&p);
        assert_eq!(before[6], 0);
        assert_eq!(before[19], 0);
        assert_eq!(p.copy(1, 0, 8), (0, 0, 12));
        assert_eq!(p.execute(3), 13);
        assert_eq!(report(&p), before);
        assert_eq!(p.execute(reservation), 2);
        let after = report(&p);
        assert_eq!(after[6], 0);
        assert_eq!(after[19], reservation as u64 + 5);
        assert_eq!(after[20], 2);
        assert_eq!(p.execute(0), 12);
        assert_eq!(p.release(), 0);
    }
    request(&mut p, b"unregistered", b"body", 1000);
    assert_eq!(p.prepare(), 1);
    assert_eq!(p.release(), 0);
    // Discard a partial request, then prove that the next operation starts fresh.
    assert_eq!(p.begin([0, 0, 0, 1, 0, 0, 0, 0]), 0);
    assert_eq!(p.field(0, 0, 10), 0);
    assert_eq!(p.release(), 0);
    request(&mut p, b"unregistered", &[], 1000);
    assert_eq!(p.prepare(), 1);
    assert_eq!(p.execute(0), 2);
    assert_eq!(report(&p)[19], 4); // Unregistered, no child.
    assert_eq!(p.release(), 0);
}
#[test]
fn wasm_import_types_missing_host_and_private_memory_are_explicit() {
    let wat = include_str!("../../subprocess/wat_abi/probe.wat");
    let engine = Engine::default();
    let module = Module::new(&engine, wat.as_bytes()).unwrap();
    assert_eq!(module.imports().count(), 8);
    assert!(!wat.contains("(memory"));
    let mut store = Store::new(&engine, empty());
    let old = Linker::<Process>::new(&engine);
    assert!(old.instantiate_and_start(&mut store, &module).is_err());
    let mut linker = Linker::new(&engine);
    host::imports(&mut linker).unwrap();
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    let begin = instance
        .get_typed_func::<(i64, i64, i64, i64, i64, i64, i64, i64), i32>(&store, "begin")
        .unwrap();
    assert_eq!(
        begin
            .call(&mut store, (0, 0, 0, -1, 123, 9, 30, 41))
            .unwrap(),
        0
    );
    let field = instance
        .get_typed_func::<(i32, i64, i64), i32>(&store, "field")
        .unwrap();
    assert_eq!(field.call(&mut store, (0, 0, 1)).unwrap(), 0);
    let chunk = instance
        .get_typed_func::<(i64, i32), i32>(&store, "chunk")
        .unwrap();
    assert_eq!(chunk.call(&mut store, (0x61, 1)).unwrap(), 0);
    assert_eq!(field.call(&mut store, (2, 0, 0)).unwrap(), 0);
    let prepare = instance
        .get_typed_func::<(), i32>(&store, "prepare")
        .unwrap();
    assert_eq!(prepare.call(&mut store, ()).unwrap(), 2);
    let report = instance
        .get_typed_func::<i32, (i64, i32)>(&store, "report")
        .unwrap();
    assert_eq!(report.call(&mut store, 11).unwrap(), (-1, 0));
    assert_eq!(report.call(&mut store, 19).unwrap(), (3, 0));
    let copy = instance
        .get_typed_func::<(i32, i64, i32), (i64, i32, i32)>(&store, "copy")
        .unwrap();
    assert_eq!(copy.call(&mut store, (1, 0, 8)).unwrap(), (0, 0, 0));
    let execute = instance
        .get_typed_func::<i32, i32>(&store, "execute")
        .unwrap();
    assert_eq!(execute.call(&mut store, 0).unwrap(), 12);
    let release = instance
        .get_typed_func::<(), i32>(&store, "release")
        .unwrap();
    assert_eq!(release.call(&mut store, ()).unwrap(), 0);
    assert_eq!(report.call(&mut store, 0).unwrap(), (0, 12));
}
