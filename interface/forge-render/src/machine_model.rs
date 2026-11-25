//! A model the user attached to a machine (forge_machine_builder::models): its file read to
//! triangles in its own coordinates (STEP and IGES through forge-step, meshed as the app meshes a
//! file's body; STL as it is), kept per file, and placed in the base frame at home.
use crate::machine_mesh::Tris;
use forge_machine_builder::models::{Format, Model};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Read files, by path and (length, modified time): a file replaced is read again.
type Key = (PathBuf, u64, Option<std::time::SystemTime>);
type Read = Mutex<HashMap<Key, Result<Arc<Tris>, String>>>;

fn read_files() -> &'static Read {
    static READ: OnceLock<Read> = OnceLock::new();
    READ.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Files shown before their reads' checks finished (`forge_step::open`): their final triangles
/// come through these, and replace what is shown ([`poll`]).
type Pending = Mutex<Vec<(Key, std::sync::mpsc::Receiver<Result<Arc<Tris>, String>>)>>;

fn pending_files() -> &'static Pending {
    static PENDING: OnceLock<Pending> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(Vec::new()))
}

/// Counts the files whose final triangles replaced what was shown: a view drawn from
/// [`file_tris`] is drawn again when it moves.
static FINISHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How many shown files have been replaced by their final triangles so far (see [`poll`]).
pub fn generation() -> u64 {
    FINISHED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether a file shown is still being checked (its final triangles to come).
pub fn pending() -> bool {
    !pending_files().lock().expect("no panics while held").is_empty()
}

/// Takes in the final triangles of the files whose checks finished: whether any came (then
/// [`generation`] moved, and [`file_tris`] gives them).
pub fn poll() -> bool {
    let mut p = pending_files().lock().expect("no panics while held");
    let mut came = false;
    p.retain(|(key, rx)| match rx.try_recv() {
        Ok(r) => {
            read_files()
                .lock()
                .expect("no panics while held")
                .insert(key.clone(), r);
            came = true;
            false
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => true,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
    });
    if came {
        FINISHED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    came
}

/// A model file's triangles in its own coordinates and units. A STEP or IGES file not read before
/// is shown as soon as its solids are drawn (`forge_step::open`): the checks that may still refuse
/// some of them run on, and when they finish its final triangles replace these ([`poll`]).
pub fn file_tris(path: &Path) -> Result<Arc<Tris>, String> {
    poll();
    // through the app's filesystem: std's has no files in a page
    let meta = forge_dirs::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    if let Some(r) = read_files().lock().expect("no panics while held").get(&key) {
        return r.clone();
    }
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let r = forge_dirs::fs::read(path)
        .map_err(|e| format!("{name}: {e}"))
        .and_then(|bytes| match Format::of(&name)? {
            Format::Step | Format::Iges => shown(&name, bytes, &key),
            Format::Stl => read(&name, &bytes).map(Arc::new),
        });
    read_files()
        .lock()
        .expect("no panics while held")
        .insert(key, r.clone());
    r
}

/// A STEP or IGES file's triangles as soon as they can be shown: from the cache, else from
/// `forge_step::open`, its checks finishing on a job whose final triangles come through
/// [`pending_files`] (where jobs run inline, the single-threaded page, the final ones at once).
fn shown(name: &str, bytes: Vec<u8>, key: &Key) -> Result<Arc<Tris>, String> {
    let ck = cached::key(&bytes);
    if let Some((meshes, failed)) = cached::load(ck) {
        return tris_of(name, meshes.iter(), failed).map(Arc::new);
    }
    let (shown_tx, shown_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let owned = name.to_string();
    crate::plat::spawn("model read", move || {
        let name = owned;
        let opened = match forge_step::open(&bytes, &forge_step::Limits::default()) {
            Ok(o) => o,
            Err(_) => {
                let e = Err(format!("{name}: not a model this reads"));
                let _ = shown_tx.send(Some(e.clone()));
                let _ = done_tx.send(e);
                return;
            }
        };
        let now = opened.shown();
        // nothing to show yet: the final triangles are what is shown
        let _ = shown_tx.send(
            (!now.is_empty()).then(|| tris_of(&name, now.into_iter().map(|(_, m)| m), 0).map(Arc::new)),
        );
        let done = match opened.finish() {
            Ok(imp) => {
                let (meshes, failed) = drawn(imp);
                cached::store(ck, &meshes, failed);
                tris_of(&name, meshes.iter(), failed).map(Arc::new)
            }
            Err(_) => Err(format!("{name}: not a model this reads")),
        };
        let _ = done_tx.send(done);
    });
    match shown_rx.recv() {
        Ok(Some(now)) => {
            // (finished already where the job ran inline)
            match done_rx.try_recv() {
                Ok(done) => done,
                Err(_) => {
                    pending_files()
                        .lock()
                        .expect("no panics while held")
                        .push((key.clone(), done_rx));
                    now
                }
            }
        }
        _ => done_rx
            .recv()
            .unwrap_or_else(|_| Err(format!("{name}: its read stopped"))),
    }
}

/// A read's solids' display meshes (`import_drawn`'s: each solid's wherever it can be drawn), and
/// how many could not be drawn.
fn drawn(imp: forge_step::Imported) -> (Vec<forge_mesh::TriMesh>, usize) {
    let failed = imp.solids.iter().filter(|s| s.mesh.is_none()).count();
    (imp.solids.into_iter().filter_map(|s| s.mesh).collect(), failed)
}

/// Meshes to the app's triangles: an error where there are none.
fn tris_of<'a>(
    name: &str,
    meshes: impl Iterator<Item = &'a forge_mesh::TriMesh>,
    failed: usize,
) -> Result<Tris, String> {
    let mut t = Tris::default();
    for m in meshes {
        push(&mut t, &m.positions, &m.triangles, &m.edges);
    }
    if t.tri.is_empty() && failed > 0 {
        return Err(format!("{name}: none of its {failed} solids could be meshed"));
    }
    done(t, name)
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
                    let imp = forge_step::import_drawn(bytes, &forge_step::Limits::default())
                        .map_err(|_| format!("{name}: not a model this reads"))?;
                    let (meshes, failed) = drawn(imp);
                    cached::store(key, &meshes, failed);
                    (meshes, failed)
                }
            };
            tris_of(name, meshes.iter(), failed)
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
