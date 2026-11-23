//! The filesystem every ForgeCAD crate reads and writes through. Natively (and on WASI) it is
//! `std::fs`, re-exported, so nothing there changes or costs more. A browser page has no
//! filesystem std can reach (every call fails on wasm32-unknown-unknown), so there it is `mem`: a
//! tree of files in memory that the page fills from where the project is kept (the browser's own
//! storage, or `forgecad web`'s files on a machine), and whose every change the page is told of
//! (`set_hook`) to keep it there. So a part's assembly, fonts, imports, undo log and versions work
//! in the browser exactly as on the desktop.
//!
//! Besides std's API: `exists`, `is_file`, `is_dir`, for `Path`'s methods of those names, which
//! go to std's filesystem directly.

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::fs::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use mem::*;

/// Whether anything is at `p` (`Path::exists`, through this filesystem).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn exists(p: impl AsRef<std::path::Path>) -> bool {
    p.as_ref().exists()
}

/// A plain file at `p` (`Path::is_file`, through this filesystem).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn is_file(p: impl AsRef<std::path::Path>) -> bool {
    p.as_ref().is_file()
}

/// A directory at `p` (`Path::is_dir`, through this filesystem).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn is_dir(p: impl AsRef<std::path::Path>) -> bool {
    p.as_ref().is_dir()
}

/// The in-memory filesystem: absolute paths (a relative one is taken from `/`), `.` and `..`
/// resolved by their text, no links. Built on every target so its tests run natively.
pub mod mem {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::io::{self, Read, Seek, SeekFrom, Write};
    use std::path::{Component, Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Clone)]
    enum Node {
        File { bytes: Arc<Vec<u8>>, stamp: u64 },
        Dir,
    }

    struct Tree {
        nodes: BTreeMap<PathBuf, Node>,
        /// Counts changes: a file's stamp is the count when it was written (its modified time,
        /// in microseconds, so a change is always seen as one: Windows' SystemTime ticks in
        /// 100 ns, which would merge stamps a nanosecond apart).
        clock: u64,
    }

    static TREE: Mutex<Tree> = Mutex::new(Tree {
        nodes: BTreeMap::new(),
        clock: 1,
    });

    /// What changed, for the host to keep it.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Change<'a> {
        /// A file's new bytes.
        Wrote(&'a [u8]),
        /// A file or a directory and all under it gone.
        Removed,
    }

    pub type Hook = fn(&Path, Change<'_>);
    static HOOK: Mutex<Option<Hook>> = Mutex::new(None);

    /// Tells `hook` of every change from now on (writes, removals; a rename is the new file
    /// written and the old removed). Files put with `load` are not changes.
    pub fn set_hook(hook: Option<Hook>) {
        *HOOK.lock().unwrap_or_else(|e| e.into_inner()) = hook;
    }

    fn tell(p: &Path, c: Change<'_>) {
        let h = *HOOK.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = h {
            h(p, c);
        }
    }

    fn tree() -> std::sync::MutexGuard<'static, Tree> {
        TREE.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The path as the tree keys it: absolute, `.` and `..` resolved.
    pub fn norm(p: &Path) -> PathBuf {
        let mut out = PathBuf::from("/");
        for c in p.components() {
            match c {
                Component::Normal(n) => out.push(n),
                Component::ParentDir => {
                    out.pop();
                }
                Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
            }
        }
        out
    }

    fn not_found(p: &Path) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, format!("{}: no such file", p.display()))
    }

    impl Tree {
        fn tick(&mut self) -> u64 {
            self.clock += 1;
            self.clock
        }

        /// Every directory above `p`, made.
        fn parents(&mut self, p: &Path) {
            let mut d = p.parent();
            while let Some(x) = d {
                if self.nodes.contains_key(x) {
                    break;
                }
                self.nodes.insert(x.to_path_buf(), Node::Dir);
                d = x.parent();
            }
        }

        fn put(&mut self, p: &Path, bytes: Vec<u8>) -> io::Result<()> {
            if matches!(self.nodes.get(p), Some(Node::Dir)) || p == Path::new("/") {
                return Err(io::Error::new(
                    io::ErrorKind::IsADirectory,
                    format!("{}: a directory", p.display()),
                ));
            }
            self.parents(p);
            let stamp = self.tick();
            self.nodes.insert(
                p.to_path_buf(),
                Node::File {
                    bytes: Arc::new(bytes),
                    stamp,
                },
            );
            Ok(())
        }

        fn is_dir(&self, p: &Path) -> bool {
            p == Path::new("/") || matches!(self.nodes.get(p), Some(Node::Dir))
        }

        fn under(&self, p: &Path) -> Vec<PathBuf> {
            self.nodes
                .range(p.to_path_buf()..)
                .take_while(|(k, _)| k.starts_with(p))
                .map(|(k, _)| k.clone())
                .collect()
        }
    }

    /// Puts a file in without telling the hook: the host loading what's kept.
    pub fn load(p: impl AsRef<Path>, bytes: Vec<u8>) {
        let _ = tree().put(&norm(p.as_ref()), bytes);
    }

    /// Forgets everything under `p` without telling the hook (`/` for all).
    pub fn unload(p: impl AsRef<Path>) {
        let p = norm(p.as_ref());
        let mut t = tree();
        for k in t.under(&p) {
            t.nodes.remove(&k);
        }
    }

    /// Every file under `p` and its bytes.
    pub fn files_under(p: impl AsRef<Path>) -> Vec<(PathBuf, Arc<Vec<u8>>)> {
        let p = norm(p.as_ref());
        let t = tree();
        t.nodes
            .range(p.clone()..)
            .take_while(|(k, _)| k.starts_with(&p))
            .filter_map(|(k, n)| match n {
                Node::File { bytes, .. } => Some((k.clone(), bytes.clone())),
                Node::Dir => None,
            })
            .collect()
    }

    pub fn exists(p: impl AsRef<Path>) -> bool {
        let p = norm(p.as_ref());
        let t = tree();
        t.is_dir(&p) || t.nodes.contains_key(&p)
    }

    pub fn is_file(p: impl AsRef<Path>) -> bool {
        matches!(tree().nodes.get(&norm(p.as_ref())), Some(Node::File { .. }))
    }

    pub fn is_dir(p: impl AsRef<Path>) -> bool {
        tree().is_dir(&norm(p.as_ref()))
    }

    pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let p = norm(p.as_ref());
        // the bytes copied with the tree let go: a read holds it no longer than a lookup
        let got = match tree().nodes.get(&p) {
            Some(Node::File { bytes, .. }) => Ok(bytes.clone()),
            Some(Node::Dir) => Err(io::Error::new(io::ErrorKind::IsADirectory, "a directory")),
            None => Err(not_found(&p)),
        };
        got.map(|b| b.as_ref().clone())
    }

    pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
        String::from_utf8(read(p)?).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8"))
    }

    pub fn write(p: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
        let p = norm(p.as_ref());
        let bytes = contents.as_ref();
        tree().put(&p, bytes.to_vec())?;
        tell(&p, Change::Wrote(bytes));
        Ok(())
    }

    pub fn create_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
        let p = norm(p.as_ref());
        let mut t = tree();
        if matches!(t.nodes.get(&p), Some(Node::File { .. })) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "a file is there"));
        }
        t.parents(&p);
        t.nodes.insert(p, Node::Dir);
        Ok(())
    }

    pub fn create_dir(p: impl AsRef<Path>) -> io::Result<()> {
        let q = norm(p.as_ref());
        if exists(&q) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "already there"));
        }
        if !q.parent().is_none_or(is_dir) {
            return Err(not_found(&q));
        }
        create_dir_all(q)
    }

    pub fn remove_file(p: impl AsRef<Path>) -> io::Result<()> {
        let p = norm(p.as_ref());
        match tree().nodes.get(&p) {
            Some(Node::File { .. }) => {}
            Some(Node::Dir) => return Err(io::Error::new(io::ErrorKind::IsADirectory, "a directory")),
            None => return Err(not_found(&p)),
        }
        tree().nodes.remove(&p);
        tell(&p, Change::Removed);
        Ok(())
    }

    pub fn remove_dir(p: impl AsRef<Path>) -> io::Result<()> {
        let p = norm(p.as_ref());
        {
            let mut t = tree();
            if !matches!(t.nodes.get(&p), Some(Node::Dir)) {
                return Err(not_found(&p));
            }
            if t.under(&p).len() > 1 {
                return Err(io::Error::new(io::ErrorKind::DirectoryNotEmpty, "not empty"));
            }
            t.nodes.remove(&p);
        }
        tell(&p, Change::Removed);
        Ok(())
    }

    pub fn remove_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
        let p = norm(p.as_ref());
        {
            let mut t = tree();
            if !t.is_dir(&p) {
                return Err(not_found(&p));
            }
            for k in t.under(&p) {
                t.nodes.remove(&k);
            }
        }
        tell(&p, Change::Removed);
        Ok(())
    }

    /// A file, or a directory with all under it, moved; what was at `to` replaced.
    pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        let (from, to) = (norm(from.as_ref()), norm(to.as_ref()));
        if from == to {
            return Ok(());
        }
        let moved: Vec<(PathBuf, Node)> = {
            let mut t = tree();
            let keys = t.under(&from);
            if keys.is_empty() {
                return Err(not_found(&from));
            }
            let mut moved = vec![];
            for k in keys {
                let n = t.nodes.remove(&k).expect("listed");
                let rel = k.strip_prefix(&from).expect("under").to_path_buf();
                let dest = if rel.as_os_str().is_empty() {
                    to.clone()
                } else {
                    to.join(rel)
                };
                moved.push((dest, n));
            }
            for k in t.under(&to) {
                t.nodes.remove(&k);
            }
            t.parents(&to);
            for (d, n) in &moved {
                t.nodes.insert(d.clone(), n.clone());
            }
            moved
        };
        for (d, n) in &moved {
            if let Node::File { bytes, .. } = n {
                tell(d, Change::Wrote(bytes));
            }
        }
        tell(&from, Change::Removed);
        Ok(())
    }

    pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
        let b = read(from)?;
        let n = b.len() as u64;
        write(to, b)?;
        Ok(n)
    }

    pub fn canonicalize(p: impl AsRef<Path>) -> io::Result<PathBuf> {
        let p = norm(p.as_ref());
        if exists(&p) { Ok(p) } else { Err(not_found(&p)) }
    }

    pub fn read_link(p: impl AsRef<Path>) -> io::Result<PathBuf> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: not a link", p.as_ref().display()),
        ))
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct FileType {
        dir: bool,
    }

    impl FileType {
        pub fn is_file(&self) -> bool {
            !self.dir
        }
        pub fn is_dir(&self) -> bool {
            self.dir
        }
        pub fn is_symlink(&self) -> bool {
            false
        }
    }

    #[derive(Clone, Debug)]
    pub struct Metadata {
        len: u64,
        dir: bool,
        stamp: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Permissions {
        readonly: bool,
    }

    impl Permissions {
        pub fn readonly(&self) -> bool {
            self.readonly
        }
        pub fn set_readonly(&mut self, r: bool) {
            self.readonly = r;
        }
    }

    impl Metadata {
        pub fn len(&self) -> u64 {
            self.len
        }
        pub fn is_empty(&self) -> bool {
            self.len == 0
        }
        pub fn is_file(&self) -> bool {
            !self.dir
        }
        pub fn is_dir(&self) -> bool {
            self.dir
        }
        pub fn is_symlink(&self) -> bool {
            false
        }
        pub fn file_type(&self) -> FileType {
            FileType { dir: self.dir }
        }
        pub fn permissions(&self) -> Permissions {
            Permissions { readonly: false }
        }
        /// When it was last written: the tree's count of changes then, in microseconds after 1970.
        pub fn modified(&self) -> io::Result<SystemTime> {
            Ok(UNIX_EPOCH + Duration::from_micros(self.stamp))
        }
        /// Not kept: the last write's time.
        pub fn created(&self) -> io::Result<SystemTime> {
            self.modified()
        }
        pub fn accessed(&self) -> io::Result<SystemTime> {
            self.modified()
        }
    }

    pub fn metadata(p: impl AsRef<Path>) -> io::Result<Metadata> {
        let p = norm(p.as_ref());
        let t = tree();
        match t.nodes.get(&p) {
            Some(Node::File { bytes, stamp }) => Ok(Metadata {
                len: bytes.len() as u64,
                dir: false,
                stamp: *stamp,
            }),
            _ if t.is_dir(&p) => Ok(Metadata {
                len: 0,
                dir: true,
                stamp: 0,
            }),
            _ => Err(not_found(&p)),
        }
    }

    pub fn symlink_metadata(p: impl AsRef<Path>) -> io::Result<Metadata> {
        metadata(p)
    }

    pub fn set_permissions(p: impl AsRef<Path>, _: Permissions) -> io::Result<()> {
        metadata(p).map(|_| ())
    }

    pub struct DirEntry {
        path: PathBuf,
        meta: Metadata,
    }

    impl DirEntry {
        pub fn path(&self) -> PathBuf {
            self.path.clone()
        }
        pub fn file_name(&self) -> OsString {
            self.path
                .file_name()
                .map(|n| n.to_os_string())
                .unwrap_or_default()
        }
        pub fn file_type(&self) -> io::Result<FileType> {
            Ok(self.meta.file_type())
        }
        pub fn metadata(&self) -> io::Result<Metadata> {
            Ok(self.meta.clone())
        }
    }

    pub struct ReadDir(std::vec::IntoIter<DirEntry>);

    impl Iterator for ReadDir {
        type Item = io::Result<DirEntry>;
        fn next(&mut self) -> Option<io::Result<DirEntry>> {
            self.0.next().map(Ok)
        }
    }

    /// What's directly in the directory, by name.
    pub fn read_dir(p: impl AsRef<Path>) -> io::Result<ReadDir> {
        let p = norm(p.as_ref());
        let t = tree();
        if !t.is_dir(&p) {
            return Err(not_found(&p));
        }
        let got: Vec<DirEntry> = t
            .nodes
            .range(p.clone()..)
            .take_while(|(k, _)| k.starts_with(&p))
            .filter(|(k, _)| k.parent() == Some(p.as_path()))
            .map(|(k, n)| DirEntry {
                path: k.clone(),
                meta: match n {
                    Node::File { bytes, stamp } => Metadata {
                        len: bytes.len() as u64,
                        dir: false,
                        stamp: *stamp,
                    },
                    Node::Dir => Metadata {
                        len: 0,
                        dir: true,
                        stamp: 0,
                    },
                },
            })
            .collect();
        Ok(ReadDir(got.into_iter()))
    }

    /// An open file: its bytes held while it's open, put back in the tree on each flush and when
    /// it's dropped. Like std's, `&File` reads and writes too.
    pub struct File(Mutex<Open>);

    #[derive(Clone)]
    struct Open {
        path: PathBuf,
        bytes: Vec<u8>,
        at: usize,
        write: bool,
        append: bool,
        dirty: bool,
    }

    impl Open {
        fn commit(&mut self) -> io::Result<()> {
            if self.dirty {
                self.dirty = false;
                write(&self.path, &self.bytes)?;
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct OpenOptions {
        read: bool,
        write: bool,
        append: bool,
        truncate: bool,
        create: bool,
        create_new: bool,
    }

    impl OpenOptions {
        pub fn new() -> OpenOptions {
            OpenOptions::default()
        }
        pub fn read(&mut self, v: bool) -> &mut Self {
            self.read = v;
            self
        }
        pub fn write(&mut self, v: bool) -> &mut Self {
            self.write = v;
            self
        }
        pub fn append(&mut self, v: bool) -> &mut Self {
            self.append = v;
            self
        }
        pub fn truncate(&mut self, v: bool) -> &mut Self {
            self.truncate = v;
            self
        }
        pub fn create(&mut self, v: bool) -> &mut Self {
            self.create = v;
            self
        }
        pub fn create_new(&mut self, v: bool) -> &mut Self {
            self.create_new = v;
            self
        }
        pub fn open(&self, p: impl AsRef<Path>) -> io::Result<File> {
            let path = norm(p.as_ref());
            let write = self.write || self.append;
            let was = read(&path);
            let bytes = match was {
                Ok(_) if self.create_new => {
                    return Err(io::Error::new(io::ErrorKind::AlreadyExists, "already there"));
                }
                Ok(b) if self.truncate => {
                    let _ = b;
                    vec![]
                }
                Ok(b) => b,
                Err(e)
                    if e.kind() == io::ErrorKind::NotFound && write && (self.create || self.create_new) =>
                {
                    if !path.parent().is_none_or(is_dir) {
                        return Err(not_found(&path));
                    }
                    vec![]
                }
                Err(e) => return Err(e),
            };
            let made = !exists(&path);
            let at = if self.append { bytes.len() } else { 0 };
            Ok(File(Mutex::new(Open {
                path,
                bytes,
                at,
                write,
                append: self.append,
                dirty: made || (self.truncate && write),
            })))
        }
    }

    impl File {
        fn lock(&self) -> std::sync::MutexGuard<'_, Open> {
            self.0.lock().unwrap_or_else(|e| e.into_inner())
        }
        pub fn open(p: impl AsRef<Path>) -> io::Result<File> {
            OpenOptions::new().read(true).open(p)
        }
        pub fn create(p: impl AsRef<Path>) -> io::Result<File> {
            OpenOptions::new().write(true).create(true).truncate(true).open(p)
        }
        pub fn create_new(p: impl AsRef<Path>) -> io::Result<File> {
            OpenOptions::new().write(true).create_new(true).open(p)
        }
        pub fn options() -> OpenOptions {
            OpenOptions::new()
        }
        pub fn metadata(&self) -> io::Result<Metadata> {
            let o = self.lock();
            Ok(Metadata {
                len: o.bytes.len() as u64,
                dir: false,
                stamp: metadata(&o.path).map_or(0, |m| m.stamp),
            })
        }
        pub fn set_len(&self, n: u64) -> io::Result<()> {
            let mut o = self.lock();
            o.bytes.resize(n as usize, 0);
            o.dirty = true;
            Ok(())
        }
        pub fn sync_all(&self) -> io::Result<()> {
            self.lock().commit()
        }
        pub fn sync_data(&self) -> io::Result<()> {
            self.sync_all()
        }
        pub fn set_modified(&self, _: SystemTime) -> io::Result<()> {
            Ok(())
        }
        /// Another handle on the same file, at the same place (its writes kept apart till flushed).
        pub fn try_clone(&self) -> io::Result<File> {
            let mut o = self.lock().clone();
            o.dirty = false;
            Ok(File(Mutex::new(o)))
        }
    }

    impl Read for &File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut o = self.lock();
            let rest = o.bytes.get(o.at..).unwrap_or(&[]);
            let n = rest.len().min(buf.len());
            buf[..n].copy_from_slice(&rest[..n]);
            o.at += n;
            Ok(n)
        }
    }

    impl Write for &File {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut o = self.lock();
            if !o.write {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "not open to write",
                ));
            }
            if o.append {
                o.at = o.bytes.len();
            }
            let end = o.at + buf.len();
            if o.bytes.len() < end {
                o.bytes.resize(end, 0);
            }
            let at = o.at;
            o.bytes[at..end].copy_from_slice(buf);
            o.at = end;
            o.dirty = true;
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            self.lock().commit()
        }
    }

    impl Seek for &File {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            let mut o = self.lock();
            let at = match to {
                SeekFrom::Start(n) => n as i64,
                SeekFrom::End(d) => o.bytes.len() as i64 + d,
                SeekFrom::Current(d) => o.at as i64 + d,
            };
            if at < 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "before the start"));
            }
            o.at = at as usize;
            Ok(at as u64)
        }
    }

    impl Read for File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            (&*self).read(buf)
        }
    }

    impl Write for File {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            (&*self).write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            (&*self).flush()
        }
    }

    impl Seek for File {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            (&*self).seek(to)
        }
    }

    impl Drop for File {
        fn drop(&mut self) {
            let _ = self.lock().commit();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::Mutex as M;

        // the tree is one for the process: these tests take turns, each under its own root
        static ONE: M<()> = M::new(());
        static SEEN: M<Vec<(PathBuf, Option<Vec<u8>>)>> = M::new(Vec::new());

        fn seen(p: &Path, c: Change<'_>) {
            SEEN.lock().unwrap().push((
                p.to_path_buf(),
                match c {
                    Change::Wrote(b) => Some(b.to_vec()),
                    Change::Removed => None,
                },
            ));
        }

        #[test]
        fn files_read_back_what_was_written_and_the_hook_hears_each_change() {
            let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
            SEEN.lock().unwrap().clear();
            set_hook(Some(seen));
            write("/t1/a/b.fg", "x = 1").unwrap();
            assert_eq!(read_to_string("/t1/a/./c/../b.fg").unwrap(), "x = 1");
            assert!(is_dir("/t1/a") && is_file("/t1/a/b.fg") && !exists("/t1/z"));
            assert_eq!(
                canonicalize("/t1/a/../a/b.fg").unwrap(),
                PathBuf::from("/t1/a/b.fg")
            );
            {
                let mut f = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("/t1/a/log")
                    .unwrap();
                f.write_all(b"one\n").unwrap();
            }
            {
                let mut f = File::options().append(true).open("/t1/a/log").unwrap();
                f.write_all(b"two\n").unwrap();
            }
            assert_eq!(read_to_string("/t1/a/log").unwrap(), "one\ntwo\n");
            assert!(File::create_new("/t1/a/log").is_err());
            let names: Vec<_> = read_dir("/t1/a")
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(names, ["b.fg", "log"]);
            rename("/t1/a", "/t1/b").unwrap();
            assert!(!exists("/t1/a/b.fg") && is_file("/t1/b/b.fg"));
            remove_file("/t1/b/log").unwrap();
            remove_dir_all("/t1").unwrap();
            assert!(!exists("/t1/b/b.fg"));
            set_hook(None);
            let s = SEEN.lock().unwrap();
            assert!(s.contains(&(PathBuf::from("/t1/a/b.fg"), Some(b"x = 1".to_vec()))));
            assert!(s.contains(&(PathBuf::from("/t1/a/log"), Some(b"one\ntwo\n".to_vec()))));
            assert!(s.contains(&(PathBuf::from("/t1/b/b.fg"), Some(b"x = 1".to_vec()))));
            assert!(s.contains(&(PathBuf::from("/t1/a"), None)));
            assert!(s.contains(&(PathBuf::from("/t1"), None)));
        }

        #[test]
        fn a_write_changes_the_modified_time_and_load_tells_no_one() {
            let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
            SEEN.lock().unwrap().clear();
            set_hook(Some(seen));
            load("/t2/p.fg", b"a".to_vec());
            assert!(SEEN.lock().unwrap().is_empty());
            let t0 = metadata("/t2/p.fg").unwrap().modified().unwrap();
            write("/t2/p.fg", "b").unwrap();
            let t1 = metadata("/t2/p.fg").unwrap().modified().unwrap();
            assert!(t1 > t0);
            assert_eq!(files_under("/t2").len(), 1);
            unload("/t2");
            assert!(!exists("/t2/p.fg"));
            set_hook(None);
        }
    }
}
