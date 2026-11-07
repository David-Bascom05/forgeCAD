//! A ForgeCAD project's history, kept as plain git that any host stores: every save of a
//! document is a commit of that document (and the new files it reads); a named version is an
//! annotated tag `forgecad/v/<path>/<n>` whose message is the version's name and notes; Restore
//! is a new save of an old state. Nothing here rewrites history or deletes anything.

pub mod cache;
pub mod names;

use forge_git::object::Sig;
use forge_git::{Error, Oid, Repo};
use std::path::{Path, PathBuf};

pub use cache::Rec;

/// The starter `.gitignore` of a project whose history ForgeCAD turned on: derived files stay out.
pub const STARTER_IGNORE: &str = "\
# made from the documents, never kept in history
*.stl
*.3mf
*.obj
*.glb
/exports/
/releases/
.forgecad/
";

pub const TRAILER_SAVE: &str = "ForgeCAD-Save";
pub const TRAILER_AGENT: &str = "ForgeCAD-Agent";
pub const TRAILER_PATH: &str = "ForgeCAD-Path";
pub const TRAILER_VERSION: &str = "ForgeCAD-Version";

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Why a save wrote the file but no history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Paused {
    /// No name and email to sign saves with.
    NeedsIdentity,
    /// Another tool is mid-change here (a merge, rebase, ..), or HEAD is detached.
    Busy(String),
    /// Someone else holds a lock.
    Locked,
    /// Every file of the save is one the project ignores.
    Ignored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    /// A new save: its id.
    New(Oid),
    /// Nothing differed from the last save.
    Unchanged,
    Paused(Paused),
}

/// A named version of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub path: String,
    pub n: u32,
    pub name: String,
    pub notes: String,
    /// The save it marks.
    pub save: Oid,
    pub time: i64,
    pub author: String,
    pub agent: Option<String>,
}

pub struct History {
    pub repo: Repo,
}

/// A message's trailers (`Key: value` lines of its last paragraph).
pub fn trailers(msg: &str) -> Vec<(String, String)> {
    let last = msg.trim_end().rsplit("\n\n").next().unwrap_or("");
    let lines: Vec<&str> = last.lines().collect();
    if lines.is_empty()
        || !lines
            .iter()
            .all(|l| l.split_once(": ").is_some_and(|(k, _)| !k.contains(' ')))
    {
        return vec![];
    }
    lines
        .iter()
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn trailer(msg: &str, key: &str) -> Option<String> {
    trailers(msg).into_iter().find(|t| t.0 == key).map(|t| t.1)
}

/// The message without its trailers.
fn body(msg: &str) -> &str {
    let t = msg.trim_end();
    if trailers(t).is_empty() {
        return t;
    }
    t.rsplit_once("\n\n").map_or("", |x| x.0)
}

impl History {
    /// The history of the project holding `file`, if it has one.
    pub fn open(file: &Path) -> Option<History> {
        let repo = Repo::discover(file).ok()?;
        repo.work.as_ref()?;
        Some(History { repo })
    }

    /// Starts a history in `dir`: a new repository and the starter `.gitignore` (kept if there).
    pub fn turn_on(dir: &Path) -> Result<History, Error> {
        let repo = Repo::init(dir, forge_git::HashKind::Sha1)?;
        let gi = dir.join(".gitignore");
        if !gi.exists() {
            forge_git::fsutil::write_new(&gi, STARTER_IGNORE.as_bytes())?;
        }
        Ok(History { repo })
    }

    pub fn work(&self) -> &Path {
        self.repo.work.as_deref().expect("opened with a work tree")
    }

    pub fn rel(&self, file: &Path) -> Result<String, Error> {
        self.repo.rel_path(file)
    }

    pub fn abs(&self, rel: &str) -> PathBuf {
        self.work().join(rel)
    }

    pub fn head(&self) -> Result<Option<Oid>, Error> {
        self.repo.head()
    }

    /// Who signs saves now: git's `user.name` and `user.email`.
    pub fn who(&self) -> Option<Sig> {
        self.repo.signature(now())
    }

    /// Sets this project's `user.name` and `user.email` (in its own config).
    pub fn set_identity(&mut self, name: &str, email: &str) -> Result<(), Error> {
        let p = self.repo.common.join("config");
        let mut lock = forge_git::fsutil::Lock::take(&p)?;
        let t = std::fs::read_to_string(&p).unwrap_or_default();
        let t = forge_git::config::append_value(&t, "user", "name", name);
        let t = forge_git::config::append_value(&t, "user", "email", email);
        lock.write(t.as_bytes())?;
        lock.commit()?;
        let git_dir = self.repo.git_dir.clone();
        let work = self.repo.work.clone();
        self.repo = Repo::open(&git_dir, work.as_deref())?;
        Ok(())
    }

    /// One save: `doc` as it is on disk now, and those of `inputs` (files it reads) that history
    /// does not hold yet and the project does not ignore. `summary` is the first line;
    /// `agent` names the agent that saved, if one did.
    pub fn save(
        &self,
        doc: &Path,
        inputs: &[PathBuf],
        summary: &str,
        agent: Option<&str>,
    ) -> Result<Saved, Error> {
        let Some(who) = self.who() else {
            return Ok(Saved::Paused(Paused::NeedsIdentity));
        };
        self.save_as(doc, inputs, summary, agent, &who)
    }

    pub fn save_as(
        &self,
        doc: &Path,
        inputs: &[PathBuf],
        summary: &str,
        agent: Option<&str>,
        who: &Sig,
    ) -> Result<Saved, Error> {
        let mut ign = forge_git::ignore::Ignore::new(
            self.work(),
            &self.repo.git_dir,
            self.repo
                .config
                .get("core.excludesfile")
                .map(|p| forge_git::config::expand(p, None)),
        );
        let head_tree = match self.head()? {
            Some(h) => Some(self.repo.commit(&h)?.tree),
            None => None,
        };
        let mut paths = vec![];
        let d = self.rel(doc)?;
        if !ign.ignored(&d, false) {
            paths.push(d);
        }
        // the project's ignore rules go in with the first save that finds them untracked
        let gi = self.work().join(".gitignore");
        for i in inputs.iter().chain(std::iter::once(&gi)) {
            let Ok(r) = self.rel(i) else {
                continue;
            };
            let held = match &head_tree {
                Some(t) => forge_git::tree::lookup(&self.repo.odb, t, &r)?.is_some(),
                None => false,
            };
            if !held && !ign.ignored(&r, false) && !paths.contains(&r) && i.is_file() {
                paths.push(r);
            }
        }
        if paths.is_empty() {
            return Ok(Saved::Paused(Paused::Ignored));
        }
        let mut msg = format!(
            "{}\n\n{TRAILER_SAVE}: 1\n",
            summary.lines().next().unwrap_or("saved")
        );
        if let Some(a) = agent {
            msg.push_str(&format!("{TRAILER_AGENT}: {a}\n"));
        }
        match self.repo.commit_paths(&paths, &msg, who) {
            Ok(Some(c)) => {
                cache::appended(self, &c.id, c.parent, who, &msg, &c.changed);
                Ok(Saved::New(c.id))
            }
            Ok(None) => Ok(Saved::Unchanged),
            Err(Error::Busy(w)) => Ok(Saved::Paused(Paused::Busy(w.to_string()))),
            Err(Error::Detached) => Ok(Saved::Paused(Paused::Busy("detached".into()))),
            Err(Error::Locked(_)) | Err(Error::RefMoved(_)) => Ok(Saved::Paused(Paused::Locked)),
            Err(e) => Err(e),
        }
    }

    /// Save an explicit set of changed project files as one commit, preserving unrelated work.
    pub fn save_group(&self, files: &[PathBuf], summary: &str, agent: Option<&str>) -> Result<Saved, Error> {
        let Some(who) = self.who() else {
            return Ok(Saved::Paused(Paused::NeedsIdentity));
        };
        let mut ignore = forge_git::ignore::Ignore::new(
            self.work(),
            &self.repo.git_dir,
            self.repo
                .config
                .get("core.excludesfile")
                .map(|p| forge_git::config::expand(p, None)),
        );
        let mut paths = Vec::new();
        for file in files {
            let rel = self.rel(file)?;
            if file.is_file() && !ignore.ignored(&rel, false) && !paths.contains(&rel) {
                paths.push(rel);
            }
        }
        if paths.is_empty() {
            return Ok(Saved::Unchanged);
        }
        let mut msg = format!(
            "{}\n\n{TRAILER_SAVE}: 1\n",
            summary.lines().next().unwrap_or("Agent changes")
        );
        if let Some(agent) = agent {
            msg.push_str(&format!("{TRAILER_AGENT}: {agent}\n"));
        }
        match self.repo.commit_paths(&paths, &msg, &who) {
            Ok(Some(c)) => {
                cache::appended(self, &c.id, c.parent, &who, &msg, &c.changed);
                Ok(Saved::New(c.id))
            }
            Ok(None) => Ok(Saved::Unchanged),
            Err(Error::Busy(w)) => Ok(Saved::Paused(Paused::Busy(w.to_string()))),
            Err(Error::Detached) => Ok(Saved::Paused(Paused::Busy("detached".into()))),
            Err(Error::Locked(_)) | Err(Error::RefMoved(_)) => Ok(Saved::Paused(Paused::Locked)),
            Err(e) => Err(e),
        }
    }

    /// Every save reachable from HEAD, newest first (from the cache, brought up to date).
    pub fn log(&self) -> Result<Vec<Rec>, Error> {
        cache::log(self)
    }

    /// The saves that changed `path` (None: every save), newest first.
    pub fn saves(&self, path: Option<&str>) -> Result<Vec<Rec>, Error> {
        let all = self.log()?;
        Ok(match path {
            None => all,
            Some(p) => all
                .into_iter()
                .filter(|r| r.paths.iter().any(|x| x == p))
                .collect(),
        })
    }

    /// A file's named versions, oldest first; with `None`, every file's.
    pub fn versions(&self, path: Option<&str>) -> Result<Vec<Version>, Error> {
        let prefix = match path {
            Some(p) => format!("refs/tags/{}/", names::version_dir(p)),
            None => "refs/tags/forgecad/v/".to_string(),
        };
        let mut out = vec![];
        for (name, id) in self.repo.refs.list(&prefix)? {
            let Some((p, n)) = names::parse_version_ref(&name) else {
                continue;
            };
            let Some(tag) = self.repo.read_tag(&id)? else {
                continue;
            };
            let b = body(&tag.message);
            let (title, notes) = b.split_once("\n\n").unwrap_or((b, ""));
            let tagger = tag.tagger.clone();
            out.push(Version {
                path: p,
                n,
                name: title.trim().to_string(),
                notes: notes.trim().to_string(),
                save: tag.object,
                time: tagger.as_ref().map_or(0, |t| t.time),
                author: tagger.map_or_else(String::new, |t| t.name),
                agent: trailer(&tag.message, TRAILER_AGENT),
            });
        }
        out.sort_by(|a, b| (a.time, &a.path, a.n).cmp(&(b.time, &b.path, b.n)));
        Ok(out)
    }

    /// The newest named version of a file.
    pub fn newest(&self, path: &str) -> Result<Option<Version>, Error> {
        Ok(self.versions(Some(path))?.into_iter().max_by_key(|v| v.n))
    }

    /// Marks the last save of `path` as named version n+1. The caller saves first.
    pub fn create_version(
        &self,
        path: &str,
        name: &str,
        notes: &str,
        agent: Option<&str>,
    ) -> Result<Version, Error> {
        let who = self
            .who()
            .ok_or(Error::Unsupported("no user.name and user.email".into()))?;
        self.create_version_as(path, name, notes, agent, &who)
    }

    pub fn create_version_as(
        &self,
        path: &str,
        name: &str,
        notes: &str,
        agent: Option<&str>,
        who: &Sig,
    ) -> Result<Version, Error> {
        let head = self.head()?.ok_or(Error::NoPath(path.to_string()))?;
        if self.repo.file_at(&head, path)?.is_none() {
            return Err(Error::NoPath(path.to_string()));
        }
        let taken: Vec<u32> = self.versions(Some(path))?.iter().map(|v| v.n).collect();
        let mut n = taken.iter().max().map_or(1, |m| m + 1);
        loop {
            let name_line = if name.trim().is_empty() {
                format!("v{n}")
            } else {
                name.trim().lines().next().unwrap_or("").to_string()
            };
            let mut msg = name_line.clone();
            if !notes.trim().is_empty() {
                msg.push_str(&format!("\n\n{}", notes.trim()));
            }
            msg.push_str(&format!("\n\n{TRAILER_PATH}: {path}\n{TRAILER_VERSION}: {n}\n"));
            if let Some(a) = agent {
                msg.push_str(&format!("{TRAILER_AGENT}: {a}\n"));
            }
            match self.repo.tag(&names::version_tag(path, n), &head, &msg, who) {
                Ok(_) => {
                    return Ok(Version {
                        path: path.to_string(),
                        n,
                        name: name_line,
                        notes: notes.trim().to_string(),
                        save: head,
                        time: who.time,
                        author: who.name.clone(),
                        agent: agent.map(String::from),
                    });
                }
                // another writer took n meanwhile: the next one
                Err(Error::RefMoved(_)) | Err(Error::Locked(_)) if n < 1_000_000 => n += 1,
                Err(e) => return Err(e),
            }
        }
    }

    /// The save a reference names, for `path`: `v3` (its named version 3), a save id or a
    /// unique prefix of one; None for `current` (the work tree).
    pub fn resolve(&self, path: &str, r: &str) -> Result<Option<Oid>, Error> {
        let r = r.trim();
        if r == "current" || r.is_empty() {
            return Ok(None);
        }
        if let Some(n) = r.strip_prefix('v').and_then(|x| x.parse::<u32>().ok()) {
            let v = self.versions(Some(path))?.into_iter().find(|v| v.n == n);
            return v
                .map(|v| Some(v.save))
                .ok_or(Error::NoPath(format!("{path}@v{n}")));
        }
        if let Some(id) = Oid::from_hex(self.repo.hash, r) {
            return Ok(Some(id));
        }
        let lower = r.to_lowercase();
        let hits: Vec<Oid> = self
            .log()?
            .into_iter()
            .filter(|c| c.id.hex().starts_with(&lower))
            .map(|c| c.id)
            .collect();
        match hits[..] {
            [one] if lower.len() >= 4 => Ok(Some(one)),
            _ => Err(Error::NoPath(format!("no single save {r}"))),
        }
    }

    /// A file's content at a save (None: `current`, read from the work tree).
    pub fn content(&self, path: &str, at: Option<&Oid>) -> Result<Option<Vec<u8>>, Error> {
        match at {
            Some(c) => self.repo.file_at(c, path),
            None => match std::fs::read(self.abs(path)) {
                Ok(d) => Ok(Some(d)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            },
        }
    }

    /// Restore: `paths` as they were at `at`, written to the work tree and saved as a new save
    /// (history is never rewritten). A path the old save lacks is removed.
    pub fn restore(
        &self,
        paths: &[String],
        at: &Oid,
        summary: &str,
        agent: Option<&str>,
    ) -> Result<Saved, Error> {
        let Some(who) = self.who() else {
            return Ok(Saved::Paused(Paused::NeedsIdentity));
        };
        self.restore_as(paths, at, summary, agent, &who)
    }

    pub fn restore_as(
        &self,
        paths: &[String],
        at: &Oid,
        summary: &str,
        agent: Option<&str>,
        who: &Sig,
    ) -> Result<Saved, Error> {
        if let Some(b) = self.repo.busy() {
            return Ok(Saved::Paused(Paused::Busy(b.to_string())));
        }
        for p in paths {
            let dest = self.abs(p);
            match self.repo.file_at(at, p)? {
                Some(data) => {
                    if let Some(d) = dest.parent() {
                        std::fs::create_dir_all(d)?;
                    }
                    let mut lock = forge_git::fsutil::Lock::take(&dest)?;
                    lock.write(&data)?;
                    lock.commit()?;
                }
                None => {
                    if dest.is_file() {
                        std::fs::remove_file(&dest)?;
                    }
                }
            }
        }
        let mut msg = format!("{}\n\n{TRAILER_SAVE}: 1\n", summary);
        if let Some(a) = agent {
            msg.push_str(&format!("{TRAILER_AGENT}: {a}\n"));
        }
        match self.repo.commit_paths(paths, &msg, who) {
            Ok(Some(c)) => {
                cache::appended(self, &c.id, c.parent, who, &msg, &c.changed);
                Ok(Saved::New(c.id))
            }
            Ok(None) => Ok(Saved::Unchanged),
            Err(Error::Locked(_)) | Err(Error::RefMoved(_)) => Ok(Saved::Paused(Paused::Locked)),
            Err(Error::Busy(w)) => Ok(Saved::Paused(Paused::Busy(w.to_string()))),
            Err(Error::Detached) => Ok(Saved::Paused(Paused::Busy("detached".into()))),
            Err(e) => Err(e),
        }
    }
}
