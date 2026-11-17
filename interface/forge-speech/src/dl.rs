//! Libraries opened at run time: the recogniser's (and on Linux, ALSA's). Nothing is linked, so
//! this builds with no C toolchain and runs (saying what's missing) without them. Unix opens with
//! dlopen; Windows with LoadLibraryExW, searching the library's own folder for what it needs.

use std::ffi::{CString, c_void};

#[cfg(unix)]
mod sys {
    use std::ffi::{CStr, c_char, c_int, c_void};

    unsafe extern "C" {
        fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        fn dlerror() -> *const c_char;
    }

    const RTLD_NOW: c_int = 2;
    #[cfg(target_os = "macos")]
    const RTLD_GLOBAL: c_int = 0x8;
    #[cfg(not(target_os = "macos"))]
    const RTLD_GLOBAL: c_int = 0x100;

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

    pub fn open(path: &str) -> Result<*mut c_void, String> {
        let c = std::ffi::CString::new(path).map_err(|_| "a NUL in the path".to_string())?;
        // SAFETY: a valid C string; the library's constructors run here
        let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
        if h.is_null() { Err(last_error()) } else { Ok(h) }
    }

    pub fn sym(h: *mut c_void, name: &CStr) -> Result<*mut c_void, String> {
        // SAFETY: a valid handle and C string
        let p = unsafe { dlsym(h, name.as_ptr()) };
        if p.is_null() { Err(last_error()) } else { Ok(p) }
    }
}

#[cfg(windows)]
mod sys {
    use std::ffi::{CStr, c_char, c_void};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        fn GetLastError() -> u32;
    }

    /// The library's own folder searched for the DLLs it loads, then the system's.
    const LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR: u32 = 0x100;
    const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x1000;

    pub fn open(path: &str) -> Result<*mut c_void, String> {
        // an absolute path is required by SEARCH_DLL_LOAD_DIR
        let abs = std::path::absolute(path).map_err(|e| e.to_string())?;
        let mut wide: Vec<u16> = abs.as_os_str().to_string_lossy().encode_utf16().collect();
        wide.push(0);
        // SAFETY: a valid wide C string; the library's DllMain runs here
        let h = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        };
        if h.is_null() {
            // SAFETY: reads this thread's last error
            Err(format!("Windows error {}", unsafe { GetLastError() }))
        } else {
            Ok(h)
        }
    }

    pub fn sym(h: *mut c_void, name: &CStr) -> Result<*mut c_void, String> {
        // SAFETY: a valid module handle and C string
        let p = unsafe { GetProcAddress(h, name.as_ptr()) };
        if p.is_null() {
            // SAFETY: reads this thread's last error
            Err(format!("Windows error {}", unsafe { GetLastError() }))
        } else {
            Ok(p)
        }
    }
}

pub struct Lib(*mut c_void);

// the handle is process-wide; its symbols are plain functions
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    /// Open, its symbols for the libraries opened after it (they find it by its name).
    pub fn open(path: &str) -> Result<Lib, String> {
        sys::open(path).map(Lib)
    }

    /// A symbol's address as the function type `F` (a `unsafe extern "C" fn`).
    ///
    /// # Safety
    /// `F` must be the symbol's real signature.
    pub unsafe fn get<F: Copy>(&self, name: &str) -> Result<F, String> {
        assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<*mut c_void>());
        let c = CString::new(name).map_err(|_| "a NUL in the name".to_string())?;
        let p = sys::sym(self.0, &c).map_err(|e| format!("{name}: {e}"))?;
        // SAFETY: the caller says F is the function's type; sizes checked above
        Ok(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&p) })
    }
}
