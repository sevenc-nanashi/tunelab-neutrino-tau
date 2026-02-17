#![allow(clippy::missing_safety_doc)]
mod config;
mod engine;
mod neutrino_label;
mod neutrino_score;
mod speaker;
mod synthesizer;

static ENGINE_POINTERS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<usize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
static CANCEL_TOKEN_POINTERS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<usize>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
static CSTRING_POINTERS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<usize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn create_c_string(s: &str) -> *mut std::ffi::c_char {
    let cstring = std::ffi::CString::new(s).unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
    let ptr = cstring.into_raw();
    {
        let mut pointers = CSTRING_POINTERS.lock().unwrap();
        pointers.insert(ptr as usize);
    }
    ptr
}

pub struct CEngine {
    engine: std::sync::Mutex<engine::Engine>,
}

pub struct CancelToken {
    token: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
#[no_mangle]
pub extern "C" fn neutrino_tau_create_cancel_token() -> *mut CancelToken {
    let token = CancelToken {
        token: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let ptr = Box::into_raw(Box::new(token));
    {
        let mut pointers = CANCEL_TOKEN_POINTERS.lock().unwrap();
        pointers.insert(ptr as usize);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_cancel_token_cancel(token: *mut CancelToken) {
    if token.is_null() {
        return;
    }
    let token = unsafe { &*token };
    token.token.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_destroy_cancel_token(token: *mut CancelToken) {
    if !token.is_null() {
        let ptr = token as usize;
        let mut pointers = CANCEL_TOKEN_POINTERS.lock().unwrap();
        if pointers.remove(&ptr) {
            unsafe {
                let _ = Box::from_raw(token);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_create_engine(
    dll_path: *const std::ffi::c_char,
    err: *mut *mut std::ffi::c_char,
) -> *mut CEngine {
    let dll_path = unsafe {
        let cstr = std::ffi::CStr::from_ptr(dll_path);
        match cstr.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                if !err.is_null() {
                    let err_msg = create_c_string("Invalid DLL path string");
                    *err = err_msg;
                }
                return std::ptr::null_mut();
            }
        }
    };
    let dll_path = std::path::PathBuf::from(dll_path);

    let engine = match engine::Engine::new(dll_path) {
        Ok(engine) => engine,
        Err(e) => {
            if !err.is_null() {
                let err_msg = create_c_string(&format!("Failed to create engine: {}", e));
                unsafe {
                    *err = err_msg;
                }
            }
            return std::ptr::null_mut();
        }
    };

    let ptr = Box::into_raw(Box::new(CEngine {
        engine: std::sync::Mutex::new(engine),
    }));
    {
        let mut pointers = ENGINE_POINTERS.lock().unwrap();
        pointers.insert(ptr as usize);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_load_voice_sources_json(
    engine: *mut CEngine,
    err: *mut *mut std::ffi::c_char,
) -> *mut std::ffi::c_char {
    if engine.is_null() {
        if !err.is_null() {
            let err_msg = create_c_string("Engine is null");
            *err = err_msg;
        }
        return std::ptr::null_mut();
    }

    let engine = unsafe { &*engine };
    let engine = match engine.engine.lock() {
        Ok(guard) => guard,
        Err(e) => {
            if !err.is_null() {
                let err_msg = create_c_string(&format!("Failed to acquire engine lock: {}", e));
                *err = err_msg;
            }
            return std::ptr::null_mut();
        }
    };
    match engine.load_voices() {
        Ok(voices) => match serde_json::to_string(&voices) {
            Ok(json) => create_c_string(&json),
            Err(e) => {
                if !err.is_null() {
                    let err_msg =
                        create_c_string(&format!("Failed to serialize voice sources: {}", e));
                    *err = err_msg;
                }
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            if !err.is_null() {
                let err_msg = create_c_string(&format!("Failed to load voice sources: {}", e));
                *err = err_msg;
            }
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn neutrino_tau_destroy_engine(engine: *mut CEngine) {
    if !engine.is_null() {
        let ptr = engine as usize;
        let mut pointers = ENGINE_POINTERS.lock().unwrap();
        if pointers.remove(&ptr) {
            unsafe {
                let _ = Box::from_raw(engine);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_synthesize(
    engine: *mut CEngine,
    synthesis_task_json: *const std::ffi::c_char,
    cancel_token: *const CancelToken,
    err: *mut *mut std::ffi::c_char,
) -> *mut std::ffi::c_char {
    if engine.is_null() {
        if !err.is_null() {
            let err_msg = create_c_string("Engine is null");
            unsafe {
                *err = err_msg;
            }
        }
        return std::ptr::null_mut();
    }

    if synthesis_task_json.is_null() {
        if !err.is_null() {
            let err_msg = create_c_string("Synthesis task payload is null");
            unsafe {
                *err = err_msg;
            }
        }
        return std::ptr::null_mut();
    }

    let payload_json = unsafe {
        let cstr = std::ffi::CStr::from_ptr(synthesis_task_json);
        match cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                if !err.is_null() {
                    let err_msg = create_c_string("Synthesis task payload is not valid UTF-8");
                    *err = err_msg;
                }
                return std::ptr::null_mut();
            }
        }
    };

    let engine = unsafe { &*engine };
    let mut engine = match engine.engine.lock() {
        Ok(guard) => guard,
        Err(e) => {
            if !err.is_null() {
                let err_msg = create_c_string(&format!("Failed to acquire engine lock: {}", e));
                unsafe {
                    *err = err_msg;
                }
            }
            return std::ptr::null_mut();
        }
    };
    let cancel_token = if cancel_token.is_null() {
        panic!("Cancel token is null");
    } else {
        unsafe { &*cancel_token }
    };
    let cancel_token = cancel_token.token.clone();
    if cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
        if !err.is_null() {
            let err_msg = create_c_string("Synthesis cancelled");
            unsafe {
                *err = err_msg;
            }
        }
        return std::ptr::null_mut();
    }

    match engine.synthesize(payload_json) {
        Ok(json) => create_c_string(&json),
        Err(e) => {
            if !err.is_null() {
                let err_msg = create_c_string(&e.to_string());
                unsafe {
                    *err = err_msg;
                }
            }
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_free_c_string(cstr: *mut std::ffi::c_char) {
    if !cstr.is_null() {
        let ptr = cstr as usize;
        let mut pointers = CSTRING_POINTERS.lock().unwrap();
        if pointers.remove(&ptr) {
            unsafe {
                let _ = std::ffi::CString::from_raw(cstr);
            }
        }
    }
}
