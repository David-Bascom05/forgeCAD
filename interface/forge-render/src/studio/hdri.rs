//! The Studio's HDRIs (`data/hdri/MANIFEST.csv`, Poly Haven, CC0): one bundled (512x256, so the
//! studio works offline and in tests), the rest fetched by `tools/fetch-hdri.sh` into the user's
//! cache and checked against their pinned sha256 here too. One not fetched is said so, and the
//! caller falls back to the drawn studio.

use super::env::{self, Env};

const MANIFEST: &str = include_str!("../../data/hdri/MANIFEST.csv");
const BUNDLED: &[u8] = include_bytes!("../../data/hdri/studio_small_09_512.hdr");

#[derive(Clone, Debug, PartialEq)]
pub struct Hdri {
    pub key: String,
    pub name: String,
    pub authors: String,
    pub licence: String,
    pub page: String,
    pub sha256: String,
    pub bundled: bool,
}

/// Every HDRI the manifest names, the bundled one first.
pub fn list() -> Vec<Hdri> {
    let mut v: Vec<Hdri> = MANIFEST
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f.len() >= 8).then(|| Hdri {
                key: f[0].into(),
                name: f[1].into(),
                authors: f[2].into(),
                licence: f[3].into(),
                page: f[4].into(),
                sha256: f[6].into(),
                bundled: !f[7].trim().is_empty(),
            })
        })
        .collect();
    v.sort_by_key(|h| !h.bundled);
    v
}

/// Where `tools/fetch-hdri.sh` puts them.
pub fn cache_dir() -> Option<std::path::PathBuf> {
    forge_dirs::cache().map(|c| c.join("forgecad").join("hdri"))
}

#[cfg(not(target_arch = "wasm32"))]
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// An HDRI's map: the bundled one from the binary, another from the cache if it's there and
/// its hash is the pinned one.
pub fn load(key: &str) -> Result<Env, String> {
    let h = list().into_iter().find(|h| h.key == key).ok_or(format!("no HDRI {key}"))?;
    let bytes = if h.bundled {
        BUNDLED.to_vec()
    } else {
        let path = cache_dir()
            .map(|d| d.join(format!("{key}.hdr")))
            .ok_or("no cache folder here: HDRIs other than the bundled one are native only")?;
        let b = forge_dirs::fs::read(&path)
            .map_err(|_| format!("{} is not fetched: run tools/fetch-hdri.sh", h.name))?;
        #[cfg(not(target_arch = "wasm32"))]
        if sha256_hex(&b) != h.sha256 {
            return Err(format!("{}: its file is not the pinned one (sha256): fetch it again", h.name));
        }
        b
    };
    let (w, hh, rgb) = env::read_hdr(&bytes)?;
    Ok(Env::new(w, hh, rgb))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_map_reads_and_every_entry_is_cited() {
        let l = list();
        assert_eq!(l.len(), 4);
        assert!(l[0].bundled && l[0].key == "studio_small_09");
        for h in &l {
            assert_eq!(h.licence, "CC0", "{}", h.key);
            assert!(h.page.starts_with("https://polyhaven.com/a/") && h.sha256.len() == 64, "{}", h.key);
        }
        let e = load("studio_small_09").unwrap();
        assert_eq!((e.w, e.h), (512, 256));
        // a lit studio: some pixels far brighter than the room
        let most = e.rgb.iter().map(|p| p[0].max(p[1]).max(p[2])).fold(0.0f32, f32::max);
        assert!(most > 5.0, "{most}");
        assert!(BUNDLED.len() <= 1 << 20);
    }
}
