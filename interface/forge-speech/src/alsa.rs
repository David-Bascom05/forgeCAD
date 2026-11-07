//! The microphone, through ALSA's "default" device (PipeWire's or PulseAudio's when they run):
//! 16 kHz mono 16-bit, read a 20 ms period at a time.

use crate::dl::Lib;
use std::ffi::{CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};

type Open = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type SetParams = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type ReadI = unsafe extern "C" fn(*mut c_void, *mut c_void, c_ulong) -> c_long;
type Recover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type Close = unsafe extern "C" fn(*mut c_void) -> c_int;
type Strerror = unsafe extern "C" fn(c_int) -> *const c_char;

const CAPTURE: c_int = 1;
const S16_LE: c_int = 2;
const RW_INTERLEAVED: c_int = 3;

pub struct Alsa {
    open: Open,
    set_params: SetParams,
    readi: ReadI,
    recover: Recover,
    close: Close,
    strerror: Strerror,
    _lib: Lib,
}

pub struct Mic<'a> {
    a: &'a Alsa,
    pcm: *mut c_void,
}

impl Alsa {
    pub fn load() -> Result<Alsa, String> {
        let lib = Lib::open("libasound.so.2").map_err(|e| format!("no ALSA library: {e}"))?;
        // SAFETY: the signatures are alsa-lib's (pcm.h)
        unsafe {
            Ok(Alsa {
                open: lib.get("snd_pcm_open")?,
                set_params: lib.get("snd_pcm_set_params")?,
                readi: lib.get("snd_pcm_readi")?,
                recover: lib.get("snd_pcm_recover")?,
                close: lib.get("snd_pcm_close")?,
                strerror: lib.get("snd_strerror")?,
                _lib: lib,
            })
        }
    }

    fn err(&self, e: c_int) -> String {
        // SAFETY: snd_strerror returns a static string
        unsafe {
            std::ffi::CStr::from_ptr((self.strerror)(e))
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn mic(&self, rate: u32) -> Result<Mic<'_>, String> {
        let dev = CString::new("default").expect("no NUL");
        let mut pcm = std::ptr::null_mut();
        // SAFETY: out-pointer and C string valid for the call
        let r = unsafe { (self.open)(&mut pcm, dev.as_ptr(), CAPTURE, 0) };
        if r < 0 {
            return Err(format!("no microphone: {}", self.err(r)));
        }
        let m = Mic { a: self, pcm };
        // SAFETY: an open capture handle; 1 channel, resampled to `rate`, 60 ms of buffer
        let r = unsafe { (self.set_params)(pcm, S16_LE, RW_INTERLEAVED, 1, rate, 1, 60_000) };
        if r < 0 {
            return Err(format!("the microphone refused 16 kHz mono: {}", self.err(r)));
        }
        Ok(m)
    }
}

impl Mic<'_> {
    /// The next samples, as many as `buf` holds (blocking until they're there).
    pub fn read(&mut self, buf: &mut [i16]) -> Result<usize, String> {
        loop {
            // SAFETY: buf holds buf.len() mono frames
            let n = unsafe { (self.a.readi)(self.pcm, buf.as_mut_ptr().cast(), buf.len() as c_ulong) };
            if n >= 0 {
                return Ok(n as usize);
            }
            // an overrun: recovered, and read again
            // SAFETY: the open handle
            let r = unsafe { (self.a.recover)(self.pcm, n as c_int, 1) };
            if r < 0 {
                return Err(self.a.err(r));
            }
        }
    }
}

impl Drop for Mic<'_> {
    fn drop(&mut self) {
        // SAFETY: the handle opened in `mic`, closed once
        unsafe {
            (self.a.close)(self.pcm);
        }
    }
}
