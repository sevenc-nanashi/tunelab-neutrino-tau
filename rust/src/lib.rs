#![allow(clippy::missing_safety_doc)]
mod config;
mod engine;
mod speaker;

fn clamp_sample_count(duration_sec: f64, sample_rate: i32) -> i32 {
    if !duration_sec.is_finite() || duration_sec <= 0.0 {
        return 0;
    }
    if sample_rate <= 0 {
        return 0;
    }

    let count = (duration_sec * sample_rate as f64).round();
    if !count.is_finite() || count <= 0.0 {
        return 0;
    }

    if count >= i32::MAX as f64 {
        i32::MAX
    } else {
        count as i32
    }
}

static ENGINE_POINTERS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<usize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
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
    engine: engine::Engine,
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

    let ptr = Box::into_raw(Box::new(CEngine { engine }));
    {
        let mut pointers = ENGINE_POINTERS.lock().unwrap();
        pointers.insert(ptr as usize);
    }
    ptr
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
pub extern "C" fn neutrino_tau_default_sample_rate() -> i32 {
    44_100
}

#[no_mangle]
pub extern "C" fn neutrino_tau_calculate_sample_count(duration_sec: f64, sample_rate: i32) -> i32 {
    clamp_sample_count(duration_sec, sample_rate)
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
