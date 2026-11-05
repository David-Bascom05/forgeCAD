//! An endpoint's API key in the OS keyring (the Secret Service), by its base URL. Called on the
//! backend's or a short-lived thread: the keyring may block to unlock.

use keyring_core::{Entry, Error};
use std::sync::OnceLock;

const SERVICE: &str = "forgecad-agent";

fn entry(url: &str) -> Result<Entry, String> {
    static STORE: OnceLock<Result<(), String>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let s = zbus_secret_service_keyring_store::Store::new().map_err(|e| format!("keyring: {e}"))?;
            keyring_core::set_default_store(s);
            Ok(())
        })
        .clone()?;
    Entry::new(SERVICE, url.trim()).map_err(|e| format!("keyring: {e}"))
}

pub fn get(url: &str) -> Result<Option<String>, String> {
    match entry(url)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring: {e}")),
    }
}

/// Stores the key, or forgets the endpoint's with an empty one.
pub fn set(url: &str, key: &str) -> Result<(), String> {
    let e = entry(url)?;
    if key.is_empty() {
        return match e.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(err) => Err(format!("keyring: {err}")),
        };
    }
    e.set_password(key).map_err(|e| format!("keyring: {e}"))
}
