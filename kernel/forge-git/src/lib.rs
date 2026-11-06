//! Git repositories in plain Rust, from git's documented formats: objects (loose and packed),
//! refs, the index, trees built from files, history and per-path history, and tree diffs.
//! It reads and writes repositories that git itself accepts; it never rewrites or deletes what
//! is there, and every write lands by rename.

pub mod config;
pub mod diff;
pub mod fsutil;
pub mod hash;
pub mod ignore;
pub mod index;
pub mod object;
pub mod odb;
pub mod pack;
pub mod refs;
pub mod repo;
pub mod tree;
pub mod walk;
pub mod zlib;

pub use hash::{HashKind, Oid};
pub use object::{Commit, Entry, Kind, Sig, Tag};
pub use repo::Repo;

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Inflate(zlib::InflateError),
    Corrupt(&'static str),
    Missing(Oid),
    WrongKind(Oid, Kind),
    /// No repository at or above the path.
    NotARepo(PathBuf),
    /// A repository feature this crate does not handle (an extension, reftable, a bare repo
    /// where a work tree is needed).
    Unsupported(String),
    /// Someone else holds this lock file.
    Locked(PathBuf),
    /// The ref was not what the update expected: something else moved it.
    RefMoved(String),
    BadRefName(String),
    /// HEAD names a commit, not a branch.
    Detached,
    /// A merge, rebase, cherry-pick or bisect is under way.
    Busy(&'static str),
    /// A path that is no file in the tree it was looked for in.
    NoPath(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Inflate(e) => write!(f, "inflate: {e}"),
            Error::Corrupt(w) => write!(f, "corrupt: {w}"),
            Error::Missing(id) => write!(f, "missing object {id}"),
            Error::WrongKind(id, k) => write!(f, "{id} is not a {}", k.name()),
            Error::NotARepo(p) => write!(f, "no repository at {}", p.display()),
            Error::Unsupported(w) => write!(f, "unsupported: {w}"),
            Error::Locked(p) => write!(f, "locked: {}", p.display()),
            Error::RefMoved(r) => write!(f, "{r} moved meanwhile"),
            Error::BadRefName(r) => write!(f, "bad ref name {r}"),
            Error::Detached => write!(f, "HEAD is detached"),
            Error::Busy(w) => write!(f, "a {w} is under way"),
            Error::NoPath(p) => write!(f, "no file {p}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<zlib::InflateError> for Error {
    fn from(e: zlib::InflateError) -> Self {
        Error::Inflate(e)
    }
}
