use std::cell::Cell;
use std::cell::RefCell;

thread_local! {
    static HANDLE: RefCell<Option<v8::IsolateHandle>> = const { RefCell::new(None) };
    static EXCEEDED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn install(isolate: &mut v8::OwnedIsolate) {
    HANDLE.with_borrow_mut(|handle| *handle = Some(isolate.thread_safe_handle()));
    isolate.add_near_heap_limit_callback(terminate, std::ptr::null_mut());
}

pub(crate) fn exceeded() -> bool {
    EXCEEDED.get()
}

unsafe extern "C" fn terminate(
    _data: *mut std::ffi::c_void,
    limit: usize,
    _initial: usize,
) -> usize {
    EXCEEDED.set(true);
    HANDLE.with_borrow(|handle| {
        if let Some(handle) = handle {
            handle.terminate_execution();
        }
    });
    limit.saturating_add(64 * 1024 * 1024)
}
