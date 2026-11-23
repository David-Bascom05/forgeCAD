//! Where ForgeCAD keeps the user's things, by each platform's own rules: settings in the config
//! folder, rebuildable files in the cache folder, conversations and sockets in the state and
//! runtime folders. An explicitly set XDG variable wins on every platform.
//!
//! | | Linux | macOS | Windows |
//! |---|---|---|---|
//! | config | `$XDG_CONFIG_HOME`, `~/.config` | `~/Library/Application Support` | `%APPDATA%` |
//! | cache | `$XDG_CACHE_HOME`, `~/.cache` | `~/Library/Caches` | `%LOCALAPPDATA%` |
//! | state | `$XDG_STATE_HOME`, `~/.local/state` | `~/Library/Application Support` | `%LOCALAPPDATA%` |
//! | runtime | `$XDG_RUNTIME_DIR`, else state | state | state |
//!
//! Each is the base folder; callers join their own name (`forgecad/...`) as they always have.

use std::path::PathBuf;

pub mod fs;

/// This process's id, for names of files made beside others. In a browser page (which has no
/// process ids: std panics) 1. Under WASI (the wasm test suite: std panics there too) a tag drawn
/// once from the time of day, so one test binary after another names its scratch files apart.
pub fn process_id() -> u32 {
    if BROWSER {
        1
    } else if WASI {
        static TAG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *TAG.get_or_init(|| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            (t ^ (t >> 32) ^ (t >> 64)) as u32
        })
    } else {
        std::process::id()
    }
}

/// The folder for scratch files: std's `temp_dir` natively. Under WASI, where std panics, `TMPDIR`
/// or `/tmp` (what the host preopens); in a browser page `/tmp` in the page's filesystem (`fs`).
pub fn temp_dir() -> PathBuf {
    if BROWSER {
        PathBuf::from("/tmp")
    } else if WASI {
        var("TMPDIR").unwrap_or_else(|| PathBuf::from("/tmp"))
    } else {
        std::env::temp_dir()
    }
}

/// Under WASI: no process ids and no temp folder std can name (the wasm test suite runs here).
const WASI: bool = cfg!(target_os = "wasi");

/// In a browser page: the user's folder is `/home` in the page's filesystem (`fs`), kept with
/// the page's storage; there is no cache folder (a result kept is rebuilt instead).
const BROWSER: bool = cfg!(all(target_arch = "wasm32", target_os = "unknown"));

fn var(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The user's home folder: `HOME`, or on Windows `USERPROFILE`.
pub fn home() -> Option<PathBuf> {
    if BROWSER {
        return Some(PathBuf::from("/home"));
    }
    var("HOME").or_else(|| if cfg!(windows) { var("USERPROFILE") } else { None })
}

/// Settings and the user's own libraries.
pub fn config() -> Option<PathBuf> {
    var("XDG_CONFIG_HOME").or_else(|| {
        if cfg!(windows) {
            var("APPDATA")
        } else if cfg!(target_os = "macos") {
            home().map(|h| h.join("Library/Application Support"))
        } else {
            home().map(|h| h.join(".config"))
        }
    })
}

/// Files that can be rebuilt: build caches, downloaded models.
pub fn cache() -> Option<PathBuf> {
    if BROWSER {
        return None;
    }
    var("XDG_CACHE_HOME").or_else(|| {
        if cfg!(windows) {
            var("LOCALAPPDATA")
        } else if cfg!(target_os = "macos") {
            home().map(|h| h.join("Library/Caches"))
        } else {
            home().map(|h| h.join(".cache"))
        }
    })
}

/// What the app keeps between runs that is not a setting: conversations, logs.
pub fn state() -> Option<PathBuf> {
    var("XDG_STATE_HOME").or_else(|| {
        if cfg!(windows) {
            var("LOCALAPPDATA")
        } else if cfg!(target_os = "macos") {
            home().map(|h| h.join("Library/Application Support"))
        } else {
            home().map(|h| h.join(".local/state"))
        }
    })
}

/// Live files of running windows (their sockets): `$XDG_RUNTIME_DIR` where there is one.
pub fn runtime() -> Option<PathBuf> {
    var("XDG_RUNTIME_DIR").or_else(state)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_folder_is_found_on_this_machine() {
        for d in [super::config(), super::cache(), super::state(), super::runtime()] {
            assert!(d.is_some_and(|p| p.is_absolute()));
        }
    }
}
