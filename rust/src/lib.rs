#![allow(clippy::missing_safety_doc)]
mod config;
mod engine;
mod neutrino_label;
pub mod neutrino_score;
mod speaker;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisTaskPayload {
    start_time: f64,
    end_time: f64,
    duration: f64,
    part_properties: std::collections::HashMap<String, serde_json::Value>,
    notes: Vec<SynthesisNotePayload>,
    pitch: PitchPayload,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisNotePayload {
    start_time: f64,
    end_time: f64,
    pitch: i32,
    lyric: String,
    last_index: Option<usize>,
    next_index: Option<usize>,
    properties: std::collections::HashMap<String, serde_json::Value>,
    phonemes: Vec<SynthesisPhonemePayload>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisPhonemePayload {
    symbol: String,
    start_time: f64,
    end_time: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PitchPayload {
    times: Vec<f64>,
    values: Vec<f64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScaffoldSynthesisResponse {
    sample_rate: i32,
    sample_count: i32,
    note_count: usize,
    phoneme_count: usize,
    property_count: usize,
}

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
    match engine.engine.load_voices() {
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
pub extern "C" fn neutrino_tau_default_sample_rate() -> i32 {
    44_100
}

#[no_mangle]
pub extern "C" fn neutrino_tau_calculate_sample_count(duration_sec: f64, sample_rate: i32) -> i32 {
    clamp_sample_count(duration_sec, sample_rate)
}

#[no_mangle]
pub unsafe extern "C" fn neutrino_tau_scaffold_synthesis_task(
    synthesis_task_json: *const std::ffi::c_char,
    err: *mut *mut std::ffi::c_char,
) -> *mut std::ffi::c_char {
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

    let payload: SynthesisTaskPayload = match serde_json::from_str(payload_json) {
        Ok(payload) => payload,
        Err(e) => {
            if !err.is_null() {
                let err_msg =
                    create_c_string(&format!("Failed to parse synthesis task payload: {}", e));
                unsafe {
                    *err = err_msg;
                }
            }
            return std::ptr::null_mut();
        }
    };

    let sample_rate = neutrino_tau_default_sample_rate();
    let duration = if payload.duration.is_finite() {
        payload.duration
    } else {
        payload.end_time - payload.start_time
    };
    let sample_count = clamp_sample_count(duration.max(0.0), sample_rate);
    let phoneme_count = payload.notes.iter().map(|note| note.phonemes.len()).sum();
    let property_count = payload.part_properties.len()
        + payload
            .notes
            .iter()
            .map(|note| note.properties.len())
            .sum::<usize>();

    let _pitch_points = payload.pitch.times.len().min(payload.pitch.values.len());
    let _note_span_sum = payload
        .notes
        .iter()
        .map(|note| (note.end_time - note.start_time).max(0.0))
        .sum::<f64>();
    let _lyric_chars = payload
        .notes
        .iter()
        .map(|note| note.lyric.chars().count())
        .sum::<usize>();
    let _pitch_sum = payload.notes.iter().map(|note| note.pitch).sum::<i32>();
    let _linked_notes = payload
        .notes
        .iter()
        .filter(|note| note.last_index.is_some() || note.next_index.is_some())
        .count();
    let _phoneme_span_sum = payload
        .notes
        .iter()
        .flat_map(|note| &note.phonemes)
        .map(|phoneme| (phoneme.end_time - phoneme.start_time).max(0.0))
        .sum::<f64>();
    let _phoneme_symbol_chars = payload
        .notes
        .iter()
        .flat_map(|note| &note.phonemes)
        .map(|phoneme| phoneme.symbol.chars().count())
        .sum::<usize>();

    let response = ScaffoldSynthesisResponse {
        sample_rate,
        sample_count,
        note_count: payload.notes.len(),
        phoneme_count,
        property_count,
    };

    match serde_json::to_string(&response) {
        Ok(json) => create_c_string(&json),
        Err(e) => {
            if !err.is_null() {
                let err_msg =
                    create_c_string(&format!("Failed to serialize scaffold response: {}", e));
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
