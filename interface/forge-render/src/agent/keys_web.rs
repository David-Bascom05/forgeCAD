//! In the browser an endpoint's API key is kept in the page's `localStorage` (this origin's,
//! readable by any script it serves): there is no OS keyring to hand it to.

const PREFIX: &str = "agent-key:";

pub fn get(url: &str) -> Result<Option<String>, String> {
    Ok(crate::plat::kv_get(&format!("{PREFIX}{}", url.trim())))
}

/// Stores the key, or forgets the endpoint's with an empty one.
pub fn set(url: &str, key: &str) -> Result<(), String> {
    let k = format!("{PREFIX}{}", url.trim());
    if key.is_empty() {
        crate::plat::kv_remove(&k);
    } else {
        crate::plat::kv_set(&k, key);
    }
    Ok(())
}
