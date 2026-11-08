//! whisper.cpp (MIT), opened at run time: its CUDA, Vulkan or CPU build, whichever's installed.
//! The structs are whisper.h's of v1.9.5; the defaults are checked on load, so another version's
//! layout is refused, never misread.

use crate::dl::Lib;
use std::ffi::{CStr, CString, c_char, c_float, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
#[derive(Clone, Copy)]
struct Aheads {
    n_heads: usize,
    heads: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ContextParams {
    use_gpu: bool,
    flash_attn: bool,
    gpu_device: c_int,
    dtw_token_timestamps: bool,
    dtw_aheads_preset: c_int,
    dtw_n_top: c_int,
    dtw_aheads: Aheads,
    dtw_mem_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VadParams {
    threshold: c_float,
    min_speech_duration_ms: c_int,
    min_silence_duration_ms: c_int,
    max_speech_duration_s: c_float,
    speech_pad_ms: c_int,
    samples_overlap: c_float,
}

type Ptr = *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct FullParams {
    strategy: c_int,
    n_threads: c_int,
    n_max_text_ctx: c_int,
    offset_ms: c_int,
    duration_ms: c_int,
    translate: bool,
    no_context: bool,
    no_timestamps: bool,
    single_segment: bool,
    print_special: bool,
    print_progress: bool,
    print_realtime: bool,
    print_timestamps: bool,
    token_timestamps: bool,
    thold_pt: c_float,
    thold_ptsum: c_float,
    max_len: c_int,
    split_on_word: bool,
    max_tokens: c_int,
    debug_mode: bool,
    audio_ctx: c_int,
    tdrz_enable: bool,
    suppress_regex: *const c_char,
    initial_prompt: *const c_char,
    carry_initial_prompt: bool,
    prompt_tokens: Ptr,
    prompt_n_tokens: c_int,
    language: *const c_char,
    detect_language: bool,
    suppress_blank: bool,
    suppress_nst: bool,
    temperature: c_float,
    max_initial_ts: c_float,
    length_penalty: c_float,
    temperature_inc: c_float,
    entropy_thold: c_float,
    logprob_thold: c_float,
    no_speech_thold: c_float,
    greedy_best_of: c_int,
    beam_size: c_int,
    beam_patience: c_float,
    new_segment_callback: Ptr,
    new_segment_callback_user_data: Ptr,
    progress_callback: Ptr,
    progress_callback_user_data: Ptr,
    encoder_begin_callback: Ptr,
    encoder_begin_callback_user_data: Ptr,
    abort_callback: Option<unsafe extern "C" fn(*mut c_void) -> bool>,
    abort_callback_user_data: *mut c_void,
    logits_filter_callback: Ptr,
    logits_filter_callback_user_data: Ptr,
    grammar_rules: Ptr,
    n_grammar_rules: usize,
    i_start_rule: usize,
    grammar_penalty: c_float,
    vad: bool,
    vad_model_path: *const c_char,
    vad_params: VadParams,
}

type CtxDefault = unsafe extern "C" fn() -> *mut ContextParams;
type FullDefault = unsafe extern "C" fn(c_int) -> *mut FullParams;
type Init = unsafe extern "C" fn(*const c_char, ContextParams) -> *mut c_void;
type Full = unsafe extern "C" fn(*mut c_void, FullParams, *const c_float, c_int) -> c_int;
type NSegments = unsafe extern "C" fn(*mut c_void) -> c_int;
type SegmentText = unsafe extern "C" fn(*mut c_void, c_int) -> *const c_char;
type LogSet =
    unsafe extern "C" fn(Option<unsafe extern "C" fn(c_int, *const c_char, *mut c_void)>, *mut c_void);
type Version = unsafe extern "C" fn() -> *const c_char;

pub struct Whisper {
    ctx: *mut c_void,
    params: FullParams,
    full: Full,
    n_segments: NSegments,
    segment_text: SegmentText,
    pub label: String,
    /// The encoder over its whole 30 s window, not just the clip: slower, and better
    /// (`FORGECAD_SPEECH_CLIP_CTX=1`: the clip only).
    full_ctx: bool,
    _libs: Vec<Lib>,
}

// the context is used from one thread at a time (the engine's)
unsafe impl Send for Whisper {}

unsafe extern "C" fn quiet(_level: c_int, _text: *const c_char, _user: *mut c_void) {}

unsafe extern "C" fn aborted(flag: *mut c_void) -> bool {
    // SAFETY: the flag `decode` passes, alive for the call
    unsafe { (*(flag as *const AtomicBool)).load(Ordering::Relaxed) }
}

/// The libraries in the order they need each other; the GPU's are there if built.
pub const LIBS: [(&str, bool); 6] = [
    ("libggml-base.so", true),
    ("libggml-cpu.so", true),
    ("libggml-cuda.so", false),
    ("libggml-vulkan.so", false),
    ("libggml.so", true),
    ("libwhisper.so", true),
];

impl Whisper {
    /// The libraries in `lib_dir`, and the model `pick` names for a GPU (true) or the CPU.
    pub fn load(
        lib_dir: &std::path::Path,
        pick: &dyn Fn(bool) -> Result<std::path::PathBuf, String>,
        threads: i32,
    ) -> Result<Whisper, String> {
        let mut libs = Vec::new();
        let mut gpu = None;
        for (name, needed) in LIBS {
            let p = lib_dir.join(name);
            if !p.exists() {
                if needed {
                    return Err(format!("{} is missing", p.display()));
                }
                continue;
            }
            let lib = Lib::open(&p.display().to_string())?;
            // a GPU's library with no GPU behind it (no device, a driver too old) counts as none
            let count = |sym: &str| -> i32 {
                // SAFETY: ggml-cuda.h's and ggml-vulkan.h's device counts: int (void)
                unsafe { lib.get::<unsafe extern "C" fn() -> c_int>(sym).map_or(0, |f| f()) }
            };
            if name.contains("cuda") && count("ggml_backend_cuda_get_device_count") > 0 {
                gpu = Some("CUDA");
            } else if name.contains("vulkan")
                && gpu.is_none()
                && count("ggml_backend_vk_get_device_count") > 0
            {
                gpu = Some("Vulkan");
            }
            libs.push(lib);
        }
        let w = libs.last().expect("libwhisper");
        // SAFETY: the signatures are whisper.h's
        let (ctx_default, full_default, init, full, n_segments, segment_text, log_set, version): (
            CtxDefault,
            FullDefault,
            Init,
            Full,
            NSegments,
            SegmentText,
            LogSet,
            Version,
        ) = unsafe {
            (
                w.get("whisper_context_default_params_by_ref")?,
                w.get("whisper_full_default_params_by_ref")?,
                w.get("whisper_init_from_file_with_params")?,
                w.get("whisper_full")?,
                w.get("whisper_full_n_segments")?,
                w.get("whisper_full_get_segment_text")?,
                w.get("whisper_log_set")?,
                w.get("whisper_version")?,
            )
        };
        // Check the version before reading structs whose layout is tied to this ABI.
        let ver = unsafe { CStr::from_ptr(version()).to_string_lossy().into_owned() };
        if ver != "1.9.5" && ver != "1.9.5-dev" {
            return Err(format!("unsupported whisper.cpp {ver}; expected 1.9.5"));
        }
        // SAFETY: version-checked default structs, read once (process-lifetime allocation).
        let (mut cp, mut fp) = unsafe { (*ctx_default(), *full_default(0)) };
        let layout_ok = cp.use_gpu
            && cp.dtw_mem_size == 128 << 20
            && fp.n_max_text_ctx == 16384
            && fp.thold_pt == 0.01
            && fp.max_initial_ts == 1.0
            && fp.length_penalty == -1.0
            && fp.temperature_inc == 0.2
            && fp.entropy_thold == 2.4
            && fp.logprob_thold == -1.0
            && fp.no_speech_thold == 0.6
            && fp.greedy_best_of == 5
            && fp.beam_patience == -1.0;
        if !layout_ok {
            return Err(format!(
                "whisper.cpp {ver} isn't the version this was built for (1.9.5)"
            ));
        }
        // SAFETY: a no-op logger; whisper.cpp's own log would go to stderr
        unsafe { log_set(Some(quiet), std::ptr::null_mut()) };
        let model = pick(gpu.is_some())?;
        let model = model.as_path();
        cp.use_gpu = gpu.is_some();
        cp.flash_attn = gpu.is_some();
        let path = CString::new(model.display().to_string()).map_err(|_| "a NUL in the path")?;
        // SAFETY: a valid path and the library's params
        let ctx = unsafe { init(path.as_ptr(), cp) };
        if ctx.is_null() {
            return Err(format!("whisper.cpp couldn't load {}", model.display()));
        }
        fp.n_threads = threads;
        // timestamps end a short clip's decode; without them whisper can say a sentence over and over
        fp.no_timestamps = std::env::var("FORGECAD_SPEECH_NO_TS").is_ok_and(|v| v == "1");
        fp.single_segment = true;
        fp.print_progress = false;
        fp.print_timestamps = false;
        fp.print_realtime = false;
        fp.print_special = false;
        // a second pass hotter only when the first reads as a loop or nonsense (FORGECAD_SPEECH_FALLBACK=0: never)
        fp.temperature_inc = std::env::var("FORGECAD_SPEECH_FALLBACK")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.2);
        let full_ctx = !std::env::var("FORGECAD_SPEECH_CLIP_CTX").is_ok_and(|v| v == "1");
        fp.suppress_nst = true;
        let name = model.file_stem().map_or("whisper".into(), |s| {
            s.to_string_lossy().trim_start_matches("ggml-").to_string()
        });
        Ok(Whisper {
            ctx,
            params: fp,
            full,
            n_segments,
            segment_text,
            label: format!("whisper.cpp {ver} {name} {}", gpu.unwrap_or("CPU")),
            full_ctx,
            _libs: libs,
        })
    }

    /// The text of `pcm` (16 kHz mono); `prompt` as what came before (the document's names),
    /// stopped early when `abort` is set. None if it was stopped or failed.
    pub fn decode(&mut self, pcm: &[f32], prompt: Option<&CStr>, abort: &AtomicBool) -> Option<String> {
        let mut p = self.params;
        p.initial_prompt = prompt.map_or(std::ptr::null(), CStr::as_ptr);
        p.language = c"en".as_ptr();
        // the encoder over the clip only (and a second), not 30 s of padding
        let secs = pcm.len() as f32 / 16000.0;
        p.audio_ctx = if self.full_ctx {
            0
        } else {
            (((secs + 1.0) * 50.0) as c_int).min(1500)
        };
        // at most ~8 tokens a second said: a loop can't run on
        p.max_tokens = (secs * 8.0) as c_int + 16;
        p.abort_callback = Some(aborted);
        p.abort_callback_user_data = abort as *const AtomicBool as *mut c_void;
        // SAFETY: the context is ours; pcm and the strings outlive the call
        let r = unsafe { (self.full)(self.ctx, p, pcm.as_ptr(), pcm.len() as c_int) };
        if r != 0 || abort.load(Ordering::Relaxed) {
            return None;
        }
        let mut out = String::new();
        // SAFETY: segments of the run just made
        unsafe {
            for i in 0..(self.n_segments)(self.ctx) {
                let s = (self.segment_text)(self.ctx, i);
                if !s.is_null() {
                    out.push_str(&CStr::from_ptr(s).to_string_lossy());
                }
            }
        }
        Some(out.trim().to_string())
    }
}
