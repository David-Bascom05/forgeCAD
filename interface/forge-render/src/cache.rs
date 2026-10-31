//! Built meshes on disk, by content hash of the part file and the sources that mesh it. Only the
//! renderer reads this: the kernel and its benchmarks never see it.

use crate::mesh::PartMesh;
use std::path::{Path, PathBuf};

/// FNV-1a over 128 bits.
pub fn hash(parts: &[&[u8]]) -> u128 {
    let mut h = 0x6c62272e07bb014262b821756295c58du128;
    for p in parts {
        for &b in *p {
            h = (h ^ b as u128).wrapping_mul(0x0000000001000000000000000000013b);
        }
        h = (h ^ 0xff).wrapping_mul(0x0000000001000000000000000000013b);
    }
    h
}

fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("FORGE_RENDER_CACHE")
        .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("forgecad").join("mesh"))
}

fn path_in(dir: &Path, src: &str) -> PathBuf {
    let key = hash(&[
        src.as_bytes(),
        env!("FORGE_KERNEL_HASH").as_bytes(),
        crate::tess::ladder_key().as_bytes(),
    ]);
    dir.join(format!("{key:032x}.mesh"))
}

/// The part's cached mesh, if a whole one is there.
pub fn load(src: &str) -> Option<PartMesh> {
    load_from(&dir()?, src)
}

fn load_from(dir: &Path, src: &str) -> Option<PartMesh> {
    PartMesh::from_bytes(&std::fs::read(path_in(dir, src)).ok()?)
}

/// Stores it. A cache that can't be written is no failure: the next open builds again.
pub fn store(src: &str, mesh: &PartMesh) {
    if let Some(dir) = dir() {
        store_in(&dir, src, mesh);
    }
}

// written whole and then moved into place, so a reader never sees half a file
fn store_in(dir: &Path, src: &str, mesh: &PartMesh) {
    let path = path_in(dir, src);
    let _ = std::fs::create_dir_all(dir);
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&tmp, mesh.to_bytes()).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_follows_the_content() {
        assert_ne!(hash(&[b"ab", b"c"]), hash(&[b"a", b"bc"]));
        assert_eq!(hash(&[b"abc"]), hash(&[b"abc"]));
    }

    #[test]
    fn a_stored_mesh_comes_back_for_its_source_only_and_never_half() {
        let dir = std::env::temp_dir().join(format!("forge-render-test-{}", std::process::id()));
        let mesh = PartMesh {
            radius: 3.0,
            ..PartMesh::default()
        };
        store_in(&dir, "part a", &mesh);
        assert_eq!(load_from(&dir, "part a"), Some(mesh));
        assert_eq!(load_from(&dir, "part b"), None);
        std::fs::write(path_in(&dir, "part a"), b"FCMESH01 cut short").unwrap();
        assert_eq!(load_from(&dir, "part a"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
