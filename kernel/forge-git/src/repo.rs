//! A repository: found from a path inside its work tree, opened only when its format is one this
//! crate reads and writes faithfully; commits made from work-tree files onto HEAD's tree, and
//! annotated tags.

use crate::config::Config;
use crate::hash::{HashKind, Oid};
use crate::index::{self, IEntry};
use crate::object::{self, Commit, Kind, MODE_EXEC, MODE_FILE, MODE_LINK, Sig, Tag};
use crate::odb::Odb;
use crate::refs::{Expect, Refs};
use crate::{Error, tree, walk};
use std::path::{Path, PathBuf};

pub struct Repo {
    /// This work tree's git dir (`.git`, or `.git/worktrees/X` for a linked work tree).
    pub git_dir: PathBuf,
    /// Where objects, refs and config live.
    pub common: PathBuf,
    pub work: Option<PathBuf>,
    pub hash: HashKind,
    pub odb: Odb,
    pub refs: Refs,
    pub config: Config,
}

/// What a commit of work-tree files did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    pub id: Oid,
    pub parent: Option<Oid>,
    pub branch: String,
    /// The paths whose content changed.
    pub changed: Vec<String>,
    /// False when the index could not be kept in step (an index this crate leaves alone).
    pub index_updated: bool,
}

const EXTENSIONS: &[&str] = &[
    "objectformat",
    "worktreeconfig",
    "noop",
    "preciousobjects",
    "partialclone",
];

impl Repo {
    /// The repository whose work tree holds `start` (a file or directory).
    pub fn discover(start: &Path) -> Result<Repo, Error> {
        let abs = std::fs::canonicalize(start).map_err(|_| Error::NotARepo(start.to_path_buf()))?;
        let mut cur: Option<&Path> = Some(if abs.is_dir() {
            &abs
        } else {
            abs.parent().unwrap_or(&abs)
        });
        while let Some(d) = cur {
            let dot = d.join(".git");
            if dot.is_dir() && dot.join("HEAD").exists() {
                return Repo::open(&dot, Some(d));
            }
            if dot.is_file() {
                let t = std::fs::read_to_string(&dot)?;
                let g = t
                    .trim()
                    .strip_prefix("gitdir:")
                    .ok_or(Error::Corrupt(".git file"))?
                    .trim();
                let g = if Path::new(g).is_absolute() {
                    PathBuf::from(g)
                } else {
                    d.join(g)
                };
                return Repo::open(&g, Some(d));
            }
            cur = d.parent();
        }
        Err(Error::NotARepo(start.to_path_buf()))
    }

    pub fn open(git_dir: &Path, work: Option<&Path>) -> Result<Repo, Error> {
        let git_dir = std::fs::canonicalize(git_dir)?;
        let common = match std::fs::read_to_string(git_dir.join("commondir")) {
            Ok(c) => {
                let c = c.trim();
                let p = if Path::new(c).is_absolute() {
                    PathBuf::from(c)
                } else {
                    git_dir.join(c)
                };
                std::fs::canonicalize(p)?
            }
            Err(_) => git_dir.clone(),
        };
        let config = Config::load(&common, &git_dir);
        let version = config.int("core.repositoryformatversion").unwrap_or(0);
        if version > 1 {
            return Err(Error::Unsupported(format!("repository format version {version}")));
        }
        if version == 1 {
            for (k, _) in &config.entries {
                if let Some(x) = k.strip_prefix("extensions.")
                    && !EXTENSIONS.contains(&x)
                {
                    return Err(Error::Unsupported(format!("extensions.{x}")));
                }
            }
        }
        let hash = match config.get("extensions.objectformat").map(|s| s.to_lowercase()) {
            None => HashKind::Sha1,
            Some(s) if s == "sha1" => HashKind::Sha1,
            Some(s) if s == "sha256" => HashKind::Sha256,
            Some(s) => return Err(Error::Unsupported(format!("object format {s}"))),
        };
        let odb = Odb::open(&common.join("objects"), hash)?;
        let work = if config.bool("core.bare") == Some(true) {
            None
        } else {
            work.map(|w| std::fs::canonicalize(w).unwrap_or(w.to_path_buf()))
        };
        Ok(Repo {
            refs: Refs {
                git_dir: git_dir.clone(),
                common: common.clone(),
                hash,
            },
            git_dir,
            common,
            work,
            hash,
            odb,
            config,
        })
    }

    /// A new repository in `work/.git`, HEAD on `main`; refuses one already there.
    pub fn init(work: &Path, hash: HashKind) -> Result<Repo, Error> {
        let g = work.join(".git");
        if g.exists() {
            return Err(Error::Unsupported(format!("{} already exists", g.display())));
        }
        for d in ["objects/info", "objects/pack", "refs/heads", "refs/tags", "info"] {
            std::fs::create_dir_all(g.join(d))?;
        }
        let cfg = match hash {
            HashKind::Sha1 => {
                "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n".to_string()
            }
            HashKind::Sha256 => "[core]\n\trepositoryformatversion = 1\n\tfilemode = true\n\tbare = false\n\
                                 [extensions]\n\tobjectformat = sha256\n"
                .to_string(),
        };
        crate::fsutil::write_new(&g.join("config"), cfg.as_bytes())?;
        crate::fsutil::write_new(&g.join("HEAD"), b"ref: refs/heads/main\n")?;
        crate::fsutil::write_new(&g.join("description"), b"Unnamed repository\n")?;
        Repo::open(&g, Some(work))
    }

    pub fn work_dir(&self) -> Result<&Path, Error> {
        self.work
            .as_deref()
            .ok_or(Error::Unsupported("a bare repository".into()))
    }

    pub fn head(&self) -> Result<Option<Oid>, Error> {
        self.refs.resolve("HEAD")
    }

    /// What is under way that a commit must not land in the middle of.
    pub fn busy(&self) -> Option<&'static str> {
        let g = &self.git_dir;
        [
            ("MERGE_HEAD", "merge"),
            ("rebase-merge", "rebase"),
            ("rebase-apply", "rebase"),
            ("CHERRY_PICK_HEAD", "cherry-pick"),
            ("REVERT_HEAD", "revert"),
            ("BISECT_LOG", "bisect"),
        ]
        .into_iter()
        .find(|(f, _)| g.join(f).exists())
        .map(|x| x.1)
    }

    /// The configured `user.name` and `user.email` at `time`, or None if either is unset.
    pub fn signature(&self, time: i64) -> Option<Sig> {
        Some(Sig {
            name: self.config.get("user.name")?.to_string(),
            email: self.config.get("user.email")?.to_string(),
            time,
            tz: 0,
        })
    }

    pub fn commit(&self, id: &Oid) -> Result<Commit, Error> {
        walk::commit(&self.odb, id)
    }

    /// A file's content at a commit.
    pub fn file_at(&self, commit: &Oid, path: &str) -> Result<Option<Vec<u8>>, Error> {
        let c = self.commit(commit)?;
        match walk::file_at(&self.odb, &c, path)? {
            Some((_, id)) => Ok(Some(self.odb.read_kind(&id, Kind::Blob)?)),
            None => Ok(None),
        }
    }

    /// A path relative to the work tree, `/`-separated, for a file inside it.
    pub fn rel_path(&self, file: &Path) -> Result<String, Error> {
        let w = self.work_dir()?;
        let abs = match std::fs::canonicalize(file) {
            Ok(a) => a,
            // a removed file: its directory's real place
            Err(_) => {
                let parent = file.parent().ok_or(Error::NoPath(file.display().to_string()))?;
                std::fs::canonicalize(parent)?.join(
                    file.file_name()
                        .ok_or(Error::NoPath(file.display().to_string()))?,
                )
            }
        };
        let r = abs
            .strip_prefix(w)
            .map_err(|_| Error::NoPath(file.display().to_string()))?;
        let s = r.to_string_lossy().replace('\\', "/");
        if s.is_empty() || s.split('/').any(|c| c == ".git") {
            return Err(Error::NoPath(s));
        }
        Ok(s)
    }

    /// Commits the work tree's current content of `paths` (relative, `/`-separated) onto HEAD's
    /// tree on HEAD's branch: a path whose file is gone is removed. Nothing is written when no
    /// path's content differs from HEAD's (Ok(None)). The index's entries for those paths are
    /// brought in step where the index is one this crate writes.
    pub fn commit_paths(
        &self,
        paths: &[String],
        message: &str,
        who: &Sig,
    ) -> Result<Option<Committed>, Error> {
        if let Some(b) = self.busy() {
            return Err(Error::Busy(b));
        }
        let branch = self.refs.head_branch()?.ok_or(Error::Detached)?;
        let work = self.work_dir()?.to_path_buf();
        let parent = self.refs.resolve(&branch)?;
        let base_tree = match &parent {
            Some(p) => Some(self.commit(p)?.tree),
            None => None,
        };
        let mut edits: Vec<tree::Edit> = vec![];
        let mut entries: Vec<(String, PathBuf, u32, Oid)> = vec![];
        let mut gone: Vec<String> = vec![];
        for p in paths {
            let file = work.join(p);
            let old = match &base_tree {
                Some(t) => tree::lookup(&self.odb, t, p)?.filter(|e| !e.is_tree()),
                None => None,
            };
            let meta = match std::fs::symlink_metadata(&file) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    if old.is_some() {
                        edits.push((p.clone(), None));
                        gone.push(p.clone());
                    }
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let (mode, data) = if meta.file_type().is_symlink() {
                let t = std::fs::read_link(&file)?;
                (MODE_LINK, t.to_string_lossy().as_bytes().to_vec())
            } else if meta.is_file() {
                let exec = executable(&meta) && self.config.bool("core.filemode") != Some(false);
                (if exec { MODE_EXEC } else { MODE_FILE }, std::fs::read(&file)?)
            } else {
                return Err(Error::NoPath(p.clone()));
            };
            let id = object::id_of(self.hash, Kind::Blob, &data);
            if old.as_ref().is_some_and(|o| o.id == id && o.mode == mode) {
                entries.push((p.clone(), file, mode, id));
                continue;
            }
            self.odb.write(Kind::Blob, &data)?;
            edits.push((p.clone(), Some((mode, id))));
            entries.push((p.clone(), file, mode, id));
        }
        if edits.is_empty() {
            return Ok(None);
        }
        let new_tree = tree::apply(&self.odb, base_tree.as_ref(), &edits)?;
        let c = Commit {
            tree: new_tree,
            parents: parent.into_iter().collect(),
            author: who.clone(),
            committer: who.clone(),
            message: if message.ends_with('\n') {
                message.to_string()
            } else {
                format!("{message}\n")
            },
        };
        let id = self.odb.write(Kind::Commit, &c.write())?;
        let expect = match parent {
            Some(p) => Expect::Is(p),
            None => Expect::Absent,
        };
        let summary = message.lines().next().unwrap_or("");
        let kind = if parent.is_some() {
            "commit"
        } else {
            "commit (initial)"
        };
        self.refs
            .update_head(id, expect, who, &format!("{kind}: {summary}"))?;
        let index_updated = index::update(&self.git_dir.join("index"), self.hash, |idx| {
            for (p, file, mode, oid) in &entries {
                if let Ok(e) = IEntry::from_file(p, file, *mode, *oid) {
                    idx.set(e);
                }
            }
            for p in &gone {
                idx.remove(p);
            }
        })
        .is_ok();
        Ok(Some(Committed {
            id,
            parent,
            branch,
            changed: edits.into_iter().map(|e| e.0).collect(),
            index_updated,
        }))
    }

    /// An annotated tag `refs/tags/<name>` on `target`; refused if the tag exists.
    pub fn tag(&self, name: &str, target: &Oid, message: &str, who: &Sig) -> Result<Oid, Error> {
        let kind = self.odb.read(target)?.kind;
        let t = Tag {
            object: *target,
            kind,
            name: name.to_string(),
            tagger: Some(who.clone()),
            message: if message.ends_with('\n') {
                message.to_string()
            } else {
                format!("{message}\n")
            },
        };
        let id = self.odb.write(Kind::Tag, &t.write())?;
        let full = format!("refs/tags/{name}");
        self.refs.update(&full, id, Expect::Absent, who, "tag")?;
        Ok(id)
    }

    pub fn read_tag(&self, id: &Oid) -> Result<Option<Tag>, Error> {
        let o = self.odb.read(id)?;
        if o.kind != Kind::Tag {
            return Ok(None);
        }
        Tag::parse(self.hash, &o.data).map(Some)
    }
}

#[cfg(unix)]
fn executable(m: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    m.permissions().mode() & 0o111 != 0
}

/// No execute bit where the platform keeps none (the browser's WASI).
#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> bool {
    false
}
