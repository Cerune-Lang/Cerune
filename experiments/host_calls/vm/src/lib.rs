//! 比較用の固定関数アダプター。製品の公開JS APIではありません。
use cerune_lang::{
    bytecode,
    embedding::{self, HostValue, ResolvedFunction},
    modules,
};
use std::{cell::RefCell, sync::Arc};

struct State {
    step: ResolvedFunction,
    checked: ResolvedFunction,
    error: String,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

#[unsafe(no_mangle)]
pub extern "C" fn vm_prepare() {
    STATE.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let ir = modules::from_source(include_str!("../../../../examples/host_calls/math.ceru"))
            .unwrap()
            .to_ir()
            .unwrap();
        let program = Arc::new(bytecode::lower(&ir).unwrap());
        let resolve = |name: &str| {
            let name = &program
                .functions
                .iter()
                .find(|f| f.name.ends_with(&format!("_{name}")))
                .unwrap()
                .name;
            embedding::resolve_function(Arc::clone(&program), name).unwrap()
        };
        *slot.borrow_mut() = Some(State {
            step: resolve("step"),
            checked: resolve("checked"),
            error: String::new(),
        });
    });
}
fn call(checked: bool, args: &[HostValue]) -> f64 {
    STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let state = state.as_mut().expect("prepare first");
        state.error.clear();
        let function = if checked { &state.checked } else { &state.step };
        match embedding::invoke_function(function, args) {
            Ok(result) => {
                assert!(result.output().is_empty());
                let Some(HostValue::F64(value)) = result.return_value() else {
                    panic!("f64 result required")
                };
                *value
            }
            Err(embedding::FunctionInvocationError::Execution(error)) => {
                state.error = error.runtime_failure().unwrap().record();
                f64::NAN
            }
            Err(error) => panic!("{error:?}"),
        }
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn step(position: f64, velocity: f64, delta: f64) -> f64 {
    call(
        false,
        &[
            HostValue::F64(position),
            HostValue::F64(velocity),
            HostValue::F64(delta),
        ],
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn checked(value: f64) -> f64 {
    call(true, &[HostValue::F64(value)])
}
#[unsafe(no_mangle)]
pub extern "C" fn vm_error_len() -> usize {
    STATE.with(|s| s.borrow().as_ref().unwrap().error.len())
}
#[unsafe(no_mangle)]
pub extern "C" fn vm_error_byte(index: usize) -> i32 {
    STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .error
            .as_bytes()
            .get(index)
            .map_or(-1, |b| i32::from(*b))
    })
}
