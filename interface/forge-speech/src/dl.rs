//! Libraries opened at run time: the recogniser's and ALSA's. Nothing is linked, so this builds
//! with no C toolchain and runs (saying what's missing) without them.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

unsafe extern "C" {
    fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;
const RTLD_GLOBAL: c_int = 0x100;

pub struct Lib(*mut c_void);

// the handle is process-wide; its symbols are plain functions
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

fn last_error() -> String {
    // SAFETY: dlerror returns a thread-local string or null
    unsafe {
        let e = dlerror();
        if e.is_null() {
            "unknown".into()
        } else {
            CStr::from_ptr(e).to_string_lossy().into_owned()
        }
    }
}

impl Lib {
    /// Open, its symbols for the libraries opened after it (they find it by its soname).
    pub fn open(path: &str) -> Result<Lib, String> {
        let c = CString::new(path).map_err(|_| "a NUL in the path".to_string())?;
        // SAFETY: a valid C string; the library's constructors run here
        let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
        if h.is_null() {
            return Err(last_error());
        }
        Ok(Lib(h))
    }

    /// A symbol's address as the function type `F` (a `unsafe extern "C" fn`).
    ///
    /// # Safety
    /// `F` must be the symbol's real signature.
    pub unsafe fn get<F: Copy>(&self, name: &str) -> Result<F, String> {
        assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<*mut c_void>());
        let c = CString::new(name).map_err(|_| "a NUL in the name".to_string())?;
        // SAFETY: a valid handle and C string
        let p = unsafe { dlsym(self.0, c.as_ptr()) };
        if p.is_null() {
            return Err(format!("{name}: {}", last_error()));
        }
        // SAFETY: the caller says F is the function's type; sizes checked above
        Ok(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&p) })
    }
}
