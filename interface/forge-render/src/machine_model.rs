//! A model the user attached to a machine (forge_machine_builder::models): its file read to
//! triangles in its own coordinates (STEP and IGES through forge-step, meshed as the app meshes a
//! file's body; STL as it is), kept per file, and placed in the base frame at home.
use crate::machine_mesh::Tris;
use forge_machine_builder::models::{Format, Model};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Read files, by path and (length, modified time): a file replaced is read again.
type Read = Mutex<HashMap<(PathBuf, u64, Option<std::time::SystemTime>), Result<Arc<Tris>, String>>>;

/// A model file's triangles in its own coordinates and units.
pub fn file_tris(path: &Path) -> Result<Arc<Tris>, String> {
    static READ: OnceLock<Read> = OnceLock::new();
    // through the app's filesystem: std's has no files in a page
    let meta = forge_dirs::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    let c = READ.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(r) = c.lock().expect("no panics while held").get(&key) {
        return r.clone();
    }
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let r = forge_dirs::fs::read(path)
        .map_err(|e| format!("{name}: {e}"))
        .and_then(|bytes| read(&name, &bytes))
        .map(Arc::new);
    c.lock().expect("no panics while held").insert(key, r.clone());
    r
}

/// A model's bytes to triangles (`name` picks the format).
pub fn read(name: &str, bytes: &[u8]) -> Result<Tris, String> {
    match Format::of(name)? {
        Format::Stl => {
            let (objs, _) =
                forge_mesh::read_stl(bytes).map_err(|e| format!("{name}: not an STL this reads ({e:?})"))?;
            let mut t = Tris::default();
            for o in &objs {
                push(&mut t, &o.positions, &o.triangles, &[]);
            }
            done(t, name)
        }
        Format::Step | Format::Iges => {
            // a file read before by this build: its meshes from the cache, the same triangles
            let key = cached::key(bytes);
            let (meshes, failed) = match cached::load(key) {
                Some(m) => m,
                None => {
                    let imp = forge_step::import(bytes, &forge_step::Limits::default())
                        .map_err(|_| format!("{name}: not a model this reads"))?;
                    let mut meshes = Vec::new();
                    let mut failed = 0;
                    for s in &imp.solids {
                        // (the read's own drawing at this chord where it kept one: the same triangles)
                        match s.display_mesh() {
                            Ok(m) => meshes.push(m.into_owned()),
                            Err(_) => failed += 1,
                        }
                    }
                    cached::store(key, &meshes, failed);
                    (meshes, failed)
                }
            };
            let mut t = Tris::default();
            for m in &meshes {
                push(&mut t, &m.positions, &m.triangles, &m.edges);
            }
            if t.tri.is_empty() && failed > 0 {
                return Err(format!("{name}: none of its {failed} solids could be meshed"));
            }
            done(t, name)
        }
    }
}

/// A STEP or IGES file's display meshes on disk, by the hash of its bytes and of the sources that
/// read and mesh it (`FORGE_KERNEL_HASH`): a file opened again is not read again. Only the
/// renderer reads this. A cache that cannot be read or written is no failure.
mod cached {
    use forge_math::Point3;
    use forge_mesh::TriMesh;
    use std::path::PathBuf;

    const MAGIC: &[u8; 8] = b"FCMODEL1";

    pub fn key(bytes: &[u8]) -> u128 {
        let mut h = forge_math::hash::Hash128::new();
        h.str("forge-render machine model display meshes 1");
        h.str(env!("FORGE_KERNEL_HASH"));
        h.bytes(bytes);
        h.finish()
    }

    fn path(key: u128) -> Option<PathBuf> {
        let base = std::env::var_os("FORGE_RENDER_CACHE")
            .map(PathBuf::from)
            .or_else(forge_dirs::cache)?;
        Some(
            base.join("forgecad")
                .join("models")
                .join(format!("{key:032x}.meshes")),
        )
    }

    /// The meshes (positions, triangles, edges) and how many solids could not be meshed, as bytes.
    pub fn to_bytes(key: u128, meshes: &[TriMesh], failed: usize) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&key.to_le_bytes());
        let n = |b: &mut Vec<u8>, x: usize| b.extend_from_slice(&(x as u64).to_le_bytes());
        n(&mut b, failed);
        n(&mut b, meshes.len());
        for m in meshes {
            n(&mut b, m.positions.len());
            for p in &m.positions {
                for x in [p.x, p.y, p.z] {
                    b.extend_from_slice(&x.to_bits().to_le_bytes());
                }
            }
            n(&mut b, m.triangles.len());
            for t in &m.triangles {
                for i in t {
                    b.extend_from_slice(&i.to_le_bytes());
                }
            }
            n(&mut b, m.edges.len());
            for e in &m.edges {
                n(&mut b, e.len());
                for i in e {
                    b.extend_from_slice(&i.to_le_bytes());
                }
            }
        }
        b
    }

    /// `to_bytes` read back, if they are whole and for this key.
    pub fn from_bytes(key: u128, b: &[u8]) -> Option<(Vec<TriMesh>, usize)> {
        struct R<'a>(&'a [u8]);
        impl R<'_> {
            fn take<const K: usize>(&mut self) -> Option<[u8; K]> {
                let (a, rest) = self.0.split_first_chunk::<K>()?;
                self.0 = rest;
                Some(*a)
            }
            fn n(&mut self) -> Option<usize> {
                let x = usize::try_from(u64::from_le_bytes(self.take()?)).ok()?;
                // no count past what the bytes could hold
                (x <= self.0.len()).then_some(x)
            }
            fn f(&mut self) -> Option<f64> {
                Some(f64::from_bits(u64::from_le_bytes(self.take()?)))
            }
            fn u(&mut self) -> Option<u32> {
                Some(u32::from_le_bytes(self.take()?))
            }
        }
        let mut r = R(b);
        if &r.take::<8>()? != MAGIC || u128::from_le_bytes(r.take()?) != key {
            return None;
        }
        let failed = r.n()?;
        let mut meshes = Vec::with_capacity(r.n()?);
        for _ in 0..meshes.capacity() {
            let mut m = TriMesh::default();
            for _ in 0..r.n()? {
                m.positions.push(Point3::new(r.f()?, r.f()?, r.f()?));
            }
            for _ in 0..r.n()? {
                let t = [r.u()?, r.u()?, r.u()?];
                if t.iter().any(|&i| i as usize >= m.positions.len()) {
                    return None;
                }
                m.triangles.push(t);
            }
            for _ in 0..r.n()? {
                let mut e = Vec::new();
                for _ in 0..r.n()? {
                    let i = r.u()?;
                    if i as usize >= m.positions.len() {
                        return None;
                    }
                    e.push(i);
                }
                m.edges.push(e);
            }
            meshes.push(m);
        }
        r.0.is_empty().then_some((meshes, failed))
    }

    pub fn load(key: u128) -> Option<(Vec<TriMesh>, usize)> {
        from_bytes(key, &forge_dirs::fs::read(path(key)?).ok()?)
    }

    // written whole and then moved into place, so a reader never sees half a file
    pub fn store(key: u128, meshes: &[TriMesh], failed: usize) {
        let Some(path) = path(key) else { return };
        if let Some(dir) = path.parent() {
            let _ = forge_dirs::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension(format!("{}.tmp", forge_dirs::process_id()));
        if forge_dirs::fs::write(&tmp, to_bytes(key, meshes, failed)).is_ok()
            && forge_dirs::fs::rename(&tmp, &path).is_err()
        {
            let _ = forge_dirs::fs::remove_file(&tmp);
        }
    }
}

fn done(t: Tris, name: &str) -> Result<Tris, String> {
    if t.tri.is_empty() {
        Err(format!("{name}: no solid or triangle in it"))
    } else {
        Ok(t)
    }
}

/// Triangles flat-shaded, each with its own corners.
fn push(t: &mut Tris, pos: &[forge_math::Point3], tris: &[[u32; 3]], edges: &[Vec<u32>]) {
    let (t0, v0) = (t.tri.len(), t.pos.len());
    let p = |i: u32| {
        let q = pos[i as usize];
        [q.x, q.y, q.z]
    };
    for tri in tris {
        let [a, b, c] = tri.map(p);
        let n = forge_machine::kin::cross(forge_machine::kin::sub(b, a), forge_machine::kin::sub(c, a));
        let l = forge_machine::kin::norm(n);
        let n = if l > 0.0 {
            n.map(|x| x / l)
        } else {
            [0.0, 0.0, 1.0]
        };
        let k = t.pos.len() as u32;
        t.pos.extend([a, b, c]);
        t.nrm.extend([n; 3]);
        t.tri.push([k, k + 1, k + 2]);
    }
    for e in edges {
        if e.len() >= 2 {
            t.edges.push(e.iter().map(|&i| p(i)).collect());
        }
    }
    t.patches.push((
        t0 as u32,
        (t.tri.len() - t0) as u32,
        v0 as u32,
        (t.pos.len() - v0) as u32,
    ));
}

/// The model's triangles placed in the base frame at home.
pub fn placed(t: &Tris, m: &Model) -> Tris {
    let r = m.rigid();
    Tris {
        pos: t.pos.iter().map(|p| m.place(*p)).collect(),
        nrm: t.nrm.iter().map(|n| r.dir(*n)).collect(),
        tri: t.tri.clone(),
        edges: t
            .edges
            .iter()
            .map(|e| e.iter().map(|p| m.place(*p)).collect())
            .collect(),
        patches: t.patches.clone(),
    }
}

/// At most this many of a model's points are checked against its envelope (evenly spread).
pub const CHECKED: usize = 200_000;

/// The model's distinct-enough points, placed (a sample of at most `CHECKED`).
pub fn placed_points(t: &Tris, m: &Model) -> Vec<[f64; 3]> {
    let step = t.pos.len().div_ceil(CHECKED).max(1);
    t.pos.iter().step_by(step).map(|p| m.place(*p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Meshes cached come back the same, and only for their key and whole.
    #[test]
    fn cached_meshes_come_back_whole_or_not_at_all() {
        let m = forge_mesh::TriMesh {
            positions: vec![
                forge_math::Point3::new(0.0, -0.0, 1.5),
                forge_math::Point3::new(1e-300, 2.0, 3.0),
                forge_math::Point3::new(4.0, 5.0, 6.0),
            ],
            triangles: vec![[0, 1, 2]],
            edges: vec![vec![0, 1], vec![]],
            ..Default::default()
        };
        let b = cached::to_bytes(7, std::slice::from_ref(&m), 2);
        let (back, failed) = cached::from_bytes(7, &b).unwrap();
        assert_eq!(failed, 2);
        assert_eq!(format!("{back:?}"), format!("{:?}", vec![m]));
        assert!(cached::from_bytes(8, &b).is_none());
        for cut in 0..b.len() {
            assert!(cached::from_bytes(7, &b[..cut]).is_none());
        }
    }

    #[test]
    fn an_ascii_stl_reads_and_lands_where_its_model_says() {
        let stl = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
        let t = read("part.stl", stl).unwrap();
        assert_eq!(t.tri.len(), 1);
        assert_eq!(t.nrm[0], [0.0, 0.0, 1.0]);
        let mut m = Model::new(None, "part.stl".into());
        m.scale = 25.4;
        m.offset = [0.0, 0.0, 100.0];
        let p = placed(&t, &m);
        assert_eq!(p.pos[1], [25.4, 0.0, 100.0]);
        assert_eq!(placed_points(&t, &m).len(), 3);
        assert!(read("part.x_t", stl).unwrap_err().contains("STEP"));
        assert!(read("empty.stl", b"solid x\nendsolid x\n").is_err());
    }
}
