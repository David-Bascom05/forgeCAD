//! The files a run read, each with a hash of what it held (or that it was not there): what a
//! result kept on disk is checked against before it is used. Recorded per thread; work done on
//! other threads hands its record back with `merge`.
//!
//! Only a file a loader let through (its fence, kind and size checks) is read here, and no more
//! than a loader would read; of any other path only whether something is there is kept.

use forge_math::hash::{Hash128, hash128};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// What a path held when it was read or looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seen {
    /// Nothing there.
    Absent,
    /// Something there, not read: a refused path, or not a plain file.
    There,
    /// A plain file: the hash of its bytes.
    Bytes(u128),
    /// A plain file larger than any loader reads: its size and time.
    Big(u64, u64),
}

/// The most of a file that is read to hash it: what the loaders read at most.
pub const LIMIT: u64 = crate::asm::LIMIT;

/// Each path and what it held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Deps(pub BTreeMap<PathBuf, Seen>);

thread_local! {
    static WATCHING: RefCell<Vec<Deps>> = const { RefCell::new(Vec::new()) };
}

/// Records what is read on this thread until `done`.
pub struct Watch(usize);

pub fn watch() -> Watch {
    WATCHING.with(|w| {
        let mut w = w.borrow_mut();
        w.push(Deps::default());
        Watch(w.len())
    })
}

impl Watch {
    /// What was read; an enclosing watch gets it too.
    pub fn done(self) -> Deps {
        let d = WATCHING.with(|w| {
            let mut w = w.borrow_mut();
            w.truncate(self.0);
            w.pop().unwrap_or_default()
        });
        merge(&d);
        std::mem::forget(self);
        d
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        WATCHING.with(|w| w.borrow_mut().truncate(self.0 - 1));
    }
}

/// What this thread's watch has recorded so far (None: nothing is recording).
pub fn so_far() -> Option<Deps> {
    WATCHING.with(|w| w.borrow().last().cloned())
}

/// Whether anything is recording on this thread (so a caller can skip the hashing).
pub fn watching() -> bool {
    WATCHING.with(|w| !w.borrow().is_empty())
}

/// Adds another thread's record to this thread's watch.
pub fn merge(d: &Deps) {
    WATCHING.with(|w| {
        if let Some(top) = w.borrow_mut().last_mut() {
            for (p, h) in &d.0 {
                top.0.insert(p.clone(), *h);
            }
        }
    });
}

fn note(path: &Path, h: Seen) {
    WATCHING.with(|w| {
        if let Some(top) = w.borrow_mut().last_mut() {
            top.0.insert(path.to_path_buf(), h);
        }
    });
}

/// A file's bytes as a loader read them (`bytes`), or that it could not (None).
pub fn read(path: &Path, bytes: Option<&[u8]>) {
    if watching() {
        note(
            path,
            bytes.map_or_else(|| presence(path), |b| Seen::Bytes(hash128(b))),
        );
    }
}

/// A file a loader let through: its bytes now (hashed once per size and time, at most `LIMIT`
/// of them, and only of a plain file).
pub fn file(path: &Path) {
    if watching() {
        note(path, contents(path, LIMIT));
    }
}

/// A path a loader looked for and refused or did not find: only whether something is there.
pub fn looked(path: &Path) {
    if watching() {
        note(path, presence(path));
    }
}

fn presence(path: &Path) -> Seen {
    if std::fs::symlink_metadata(path).is_ok() {
        Seen::There
    } else {
        Seen::Absent
    }
}

/// The material libraries a document in `doc_dir` sees: its project's and the user's.
pub fn materials(doc_dir: Option<&Path>) {
    if !watching() {
        return;
    }
    if let Some(d) = doc_dir {
        tomls(&forge_material::project_dir(d));
    }
    if let Some(u) = forge_material::user_dir() {
        tomls(&u);
    }
}

/// A folder's `.toml` files (a material library): which there are, and each one's bytes.
pub fn tomls(dir: &Path) {
    if !watching() {
        return;
    }
    note(&listing_key(dir), Seen::Bytes(listing(dir)));
    for f in tomls_in(dir) {
        file(&f);
    }
}

fn listing_key(dir: &Path) -> PathBuf {
    dir.join("*.toml")
}

fn tomls_in(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn listing(dir: &Path) -> u128 {
    let mut h = Hash128::new();
    for f in tomls_in(dir) {
        h.str(&f.to_string_lossy());
    }
    h.finish()
}

type Stamp = (u64, Option<SystemTime>);
static HASHED: Mutex<Option<HashMap<PathBuf, (Stamp, u128)>>> = Mutex::new(None);

/// What a plain file holds now: its hash (once per size and time), read no further than `limit`.
fn contents(path: &Path, limit: u64) -> Seen {
    use std::io::Read;
    if path.file_name().is_some_and(|n| n == "*.toml") {
        return path.parent().map_or(Seen::Absent, |d| Seen::Bytes(listing(d)));
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return presence(path);
    };
    if !meta.is_file() {
        return Seen::There;
    }
    let time = meta.modified().ok();
    let nanos = time
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as u64);
    if meta.len() > limit {
        return Seen::Big(meta.len(), nanos);
    }
    let stamp = (meta.len(), time);
    let seen = HASHED
        .lock()
        .ok()
        .and_then(|m| m.as_ref().and_then(|m| m.get(path).copied()));
    if let Some((s, h)) = seen
        && s == stamp
        && stamp.1.is_some()
    {
        return Seen::Bytes(h);
    }
    let mut data = Vec::new();
    let Ok(f) = std::fs::File::open(path) else {
        return Seen::There;
    };
    if f.take(limit + 1).read_to_end(&mut data).is_err() {
        return Seen::There;
    }
    if data.len() as u64 > limit {
        return Seen::Big(data.len() as u64, nanos);
    }
    let h = hash128(&data);
    if let Ok(mut m) = HASHED.lock() {
        m.get_or_insert_with(Default::default)
            .insert(path.to_path_buf(), (stamp, h));
    }
    Seen::Bytes(h)
}

/// What a path holds now, looked at as it was when recorded.
fn now(path: &Path, was: Seen) -> Seen {
    match was {
        Seen::Absent | Seen::There => presence(path),
        Seen::Bytes(_) | Seen::Big(..) => contents(path, LIMIT),
    }
}

impl Deps {
    /// Whether every file still holds what it held (and every missing one is still missing).
    pub fn current(&self) -> bool {
        self.0.iter().all(|(p, h)| now(p, *h) == *h)
    }

    /// One hash of all of it.
    pub fn digest(&self) -> u128 {
        let mut h = Hash128::new();
        for (p, x) in &self.0 {
            h.str(&p.to_string_lossy());
            match x {
                Seen::Absent => h.word(0),
                Seen::There => h.word(1),
                Seen::Bytes(b) => {
                    h.word(2);
                    h.u128(*b);
                }
                Seen::Big(l, t) => {
                    h.word(3);
                    h.word(*l);
                    h.word(*t);
                }
            }
        }
        h.finish()
    }

    pub fn write(&self, o: &mut crate::store::Out) {
        o.u64(self.0.len() as u64);
        for (p, h) in &self.0 {
            o.str(&p.to_string_lossy());
            let (tag, a, b) = match h {
                Seen::Absent => (0, 0, 0),
                Seen::There => (1, 0, 0),
                Seen::Bytes(x) => (2, *x as u64, (*x >> 64) as u64),
                Seen::Big(l, t) => (3, *l, *t),
            };
            o.u64(tag);
            o.u64(a);
            o.u64(b);
        }
    }

    pub fn read(i: &mut crate::store::In) -> Option<Deps> {
        let n = i.count(32)?;
        let mut d = Deps::default();
        for _ in 0..n {
            let p = PathBuf::from(i.str()?);
            let (tag, a, b) = (i.u64()?, i.u64()?, i.u64()?);
            let seen = match tag {
                0 => Seen::Absent,
                1 => Seen::There,
                2 => Seen::Bytes(a as u128 | (b as u128) << 64),
                3 => Seen::Big(a, b),
                _ => return None,
            };
            d.0.insert(p, seen);
        }
        Some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_sees_reads_and_its_parent_gets_them() {
        let d = std::env::temp_dir().join(format!("forge-deps-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let (a, b) = (d.join("a.fg"), d.join("b.fg"));
        std::fs::write(&a, "one").unwrap();
        let _ = std::fs::remove_file(&b);
        let outer = watch();
        let inner = watch();
        file(&a);
        file(&b);
        let got = inner.done();
        assert_eq!(got.0.len(), 2);
        assert!(got.current());
        let all = outer.done();
        assert_eq!(all, got);
        std::fs::write(&b, "now here").unwrap();
        assert!(!all.current(), "a file that came is a change");
        std::fs::remove_file(&b).unwrap();
        assert!(all.current());
        std::fs::write(&a, "two").unwrap();
        assert!(!all.current(), "new bytes are a change");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_special_file_is_never_read() {
        let w = watch();
        file(Path::new("/dev/zero"));
        let d = w.done();
        assert_eq!(d.0.get(Path::new("/dev/zero")), Some(&Seen::There));
        assert!(d.current());
    }

    #[test]
    fn a_large_file_is_not_read_past_the_limit() {
        let d = std::env::temp_dir().join(format!("forge-deps-big-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("big.fg");
        std::fs::write(&f, vec![b'x'; 100]).unwrap();
        assert!(matches!(contents(&f, 10), Seen::Big(100, _)));
        assert!(matches!(contents(&f, 1000), Seen::Bytes(_)));
    }

    /// A root with a part in it, a file outside it, and a link inside it to that file.
    fn fenced(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("forge-deps-fence-{tag}-{}", std::process::id()));
        let (root, out) = (base.join("root"), base.join("outside"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("secret.fg"), "shared s = 1\n").unwrap();
        let link = root.join("link.fg");
        if std::fs::symlink_metadata(&link).is_err() {
            std::os::unix::fs::symlink(out.join("secret.fg"), &link).unwrap();
        }
        (root, out)
    }

    fn located(root: &Path, file: &str) -> (Option<PathBuf>, Deps) {
        let w = watch();
        let fence = Some(std::fs::canonicalize(root).unwrap());
        let got = crate::asm::locate(&Some(root.to_path_buf()), &fence, file);
        (got, w.done())
    }

    fn read_any(d: &Deps) -> bool {
        d.0.values().any(|s| matches!(s, Seen::Bytes(_) | Seen::Big(..)))
    }

    #[test]
    fn a_path_out_of_the_root_is_not_read() {
        let (root, _) = fenced("out");
        let (got, d) = located(&root, "../outside/secret.fg");
        assert_eq!(got, None);
        assert!(!read_any(&d), "{d:?}");
        let (got, d) = located(&root, "../../../../../../dev/zero");
        assert_eq!(got, None);
        assert!(!read_any(&d), "{d:?}");
    }

    #[test]
    fn a_link_out_of_the_root_is_not_read() {
        let (root, _) = fenced("link");
        let (got, d) = located(&root, "link.fg");
        assert_eq!(got, None);
        assert!(!read_any(&d), "{d:?}");
        assert_eq!(d.0.get(&root.join("link.fg")), Some(&Seen::There));
    }

    #[test]
    fn nothing_is_kept_unwatched() {
        file(Path::new("/nonexistent/x.fg"));
        let w = watch();
        assert!(w.done().0.is_empty());
    }
}
