//! Refs as files (gitrepository-layout): loose refs, `packed-refs`, symbolic refs (HEAD), the
//! per-worktree ones in the worktree's own git dir, and reflogs. An update takes git's
//! `<ref>.lock`, checks the old value under it, and renames the new one into place.

use crate::Error;
use crate::fsutil::{self, Lock};
use crate::hash::{HashKind, Oid};
use crate::object::Sig;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Direct(Oid),
    Symbolic(String),
}

pub struct Refs {
    pub git_dir: PathBuf,
    pub common: PathBuf,
    pub hash: HashKind,
}

/// What a ref is checked to be before it moves.
#[derive(Clone, Copy, Debug)]
pub enum Expect {
    Any,
    /// Must not exist yet.
    Absent,
    Is(Oid),
}

/// Ref names git accepts (git-check-ref-format), as full names under `refs/` or `HEAD`.
pub fn valid_name(n: &str) -> bool {
    if n == "HEAD" {
        return true;
    }
    if !n.starts_with("refs/") || n.ends_with('/') || n.ends_with('.') || n.contains("..") || n.contains("@{")
    {
        return false;
    }
    if n.bytes()
        .any(|c| c < 0x20 || c == 0x7f || b" ~^:?*[\\".contains(&c))
    {
        return false;
    }
    n.split('/')
        .all(|c| !c.is_empty() && !c.starts_with('.') && !c.ends_with(".lock"))
}

impl Refs {
    fn per_worktree(name: &str) -> bool {
        !name.starts_with("refs/")
            || name.starts_with("refs/bisect/")
            || name.starts_with("refs/worktree/")
            || name.starts_with("refs/rewritten/")
    }

    fn file(&self, name: &str) -> PathBuf {
        if Self::per_worktree(name) {
            self.git_dir.join(name)
        } else {
            self.common.join(name)
        }
    }

    fn log_file(&self, name: &str) -> PathBuf {
        if Self::per_worktree(name) {
            self.git_dir.join("logs").join(name)
        } else {
            self.common.join("logs").join(name)
        }
    }

    pub fn packed(&self) -> Result<BTreeMap<String, (Oid, Option<Oid>)>, Error> {
        let mut m = BTreeMap::new();
        let Ok(t) = std::fs::read_to_string(self.common.join("packed-refs")) else {
            return Ok(m);
        };
        let mut last: Option<String> = None;
        for line in t.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(p) = line.strip_prefix('^') {
                let id = Oid::from_hex(self.hash, p.trim()).ok_or(Error::Corrupt("packed-refs"))?;
                if let Some(n) = &last
                    && let Some(e) = m.get_mut(n)
                {
                    let e: &mut (Oid, Option<Oid>) = e;
                    e.1 = Some(id);
                }
                continue;
            }
            let (h, n) = line.split_once(' ').ok_or(Error::Corrupt("packed-refs"))?;
            let id = Oid::from_hex(self.hash, h).ok_or(Error::Corrupt("packed-refs"))?;
            m.insert(n.to_string(), (id, None));
            last = Some(n.to_string());
        }
        Ok(m)
    }

    fn parse_loose(&self, t: &str) -> Result<Value, Error> {
        let t = t.trim_end();
        if let Some(s) = t.strip_prefix("ref: ") {
            return Ok(Value::Symbolic(s.trim().to_string()));
        }
        Oid::from_hex(self.hash, t)
            .map(Value::Direct)
            .ok_or(Error::Corrupt("loose ref"))
    }

    pub fn read(&self, name: &str) -> Result<Option<Value>, Error> {
        match std::fs::read_to_string(self.file(name)) {
            Ok(t) => return self.parse_loose(&t).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            // a directory of refs where a ref would be: not one
            Err(e) if e.kind() == std::io::ErrorKind::IsADirectory => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        Ok(self.packed()?.get(name).map(|e| Value::Direct(e.0)))
    }

    pub fn resolve(&self, name: &str) -> Result<Option<Oid>, Error> {
        let mut n = name.to_string();
        for _ in 0..10 {
            match self.read(&n)? {
                None => return Ok(None),
                Some(Value::Direct(id)) => return Ok(Some(id)),
                Some(Value::Symbolic(s)) => n = s,
            }
        }
        Err(Error::Corrupt("symbolic ref loop"))
    }

    /// The branch HEAD names (possibly not born yet), or None when HEAD is detached.
    pub fn head_branch(&self) -> Result<Option<String>, Error> {
        match self.read("HEAD")? {
            Some(Value::Symbolic(s)) => Ok(Some(s)),
            _ => Ok(None),
        }
    }

    /// Every ref under `prefix` (e.g. `refs/tags/forgecad/`), loose ones over packed ones.
    pub fn list(&self, prefix: &str) -> Result<BTreeMap<String, Oid>, Error> {
        let mut m: BTreeMap<String, Oid> = self
            .packed()?
            .into_iter()
            .filter(|(n, _)| n.starts_with(prefix))
            .map(|(n, e)| (n, e.0))
            .collect();
        let root = self.common.clone();
        let start = prefix.rfind('/').map_or("", |i| &prefix[..i]);
        let mut stack = vec![root.join(start)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                let ft = e.file_type()?;
                if ft.is_dir() {
                    stack.push(p);
                    continue;
                }
                let rel = p
                    .strip_prefix(&root)
                    .expect("under root")
                    .to_string_lossy()
                    .to_string();
                if !rel.starts_with(prefix) || rel.ends_with(".lock") {
                    continue;
                }
                let Ok(t) = std::fs::read_to_string(&p) else {
                    continue;
                };
                match self.parse_loose(&t) {
                    Ok(Value::Direct(id)) => {
                        m.insert(rel, id);
                    }
                    Ok(Value::Symbolic(s)) => {
                        if let Some(id) = self.resolve(&s)? {
                            m.insert(rel, id);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        Ok(m)
    }

    /// Moves `name` to `new`, if it is what `expect` says; logs the move.
    pub fn update(&self, name: &str, new: Oid, expect: Expect, who: &Sig, msg: &str) -> Result<(), Error> {
        if !valid_name(name) {
            return Err(Error::BadRefName(name.to_string()));
        }
        let path = self.file(name);
        let mut lock = Lock::take(&path)?;
        let old = match self.read(name)? {
            Some(Value::Direct(id)) => Some(id),
            Some(Value::Symbolic(_)) => return Err(Error::Corrupt("update of a symbolic ref")),
            None => None,
        };
        match (expect, old) {
            (Expect::Any, _) | (Expect::Absent, None) => {}
            (Expect::Is(want), Some(o)) if want == o => {}
            _ => return Err(Error::RefMoved(name.to_string())),
        }
        lock.write(format!("{new}\n").as_bytes())?;
        lock.commit()?;
        let old = old.unwrap_or(self.hash.null());
        let line = format!("{old} {new} {}\t{msg}\n", who.write());
        fsutil::append(&self.log_file(name), line.as_bytes())?;
        Ok(())
    }

    /// Moves the branch HEAD names (and logs HEAD too).
    pub fn update_head(&self, new: Oid, expect: Expect, who: &Sig, msg: &str) -> Result<String, Error> {
        let branch = self.head_branch()?.ok_or(Error::Detached)?;
        self.update(&branch, new, expect, who, msg)?;
        let old = match expect {
            Expect::Is(o) => o,
            _ => self.hash.null(),
        };
        let line = format!("{old} {new} {}\t{msg}\n", who.write());
        fsutil::append(&self.log_file("HEAD"), line.as_bytes())?;
        Ok(branch)
    }

    pub fn write_symbolic(&self, name: &str, target: &str) -> Result<(), Error> {
        let mut lock = Lock::take(&self.file(name))?;
        lock.write(format!("ref: {target}\n").as_bytes())?;
        lock.commit()
    }
}

pub fn exists(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

#[cfg(test)]
mod tests {
    use super::valid_name;

    #[test]
    fn ref_names() {
        assert!(valid_name("refs/tags/forgecad/v/parts%2Fbracket.fg/3"));
        assert!(!valid_name("refs/tags/a b"));
        assert!(!valid_name("refs/tags/.x"));
        assert!(!valid_name("refs/tags/x.lock"));
        assert!(!valid_name("refs/tags/a@{1}"));
        assert!(!valid_name("refs/heads/a..b"));
    }
}
