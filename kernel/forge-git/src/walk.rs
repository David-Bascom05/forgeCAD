//! History: commits newest first by committer time from some starting commits, and the history
//! of one path with git's default simplification (`git log -- PATH`): a commit is listed when its
//! file differs from every parent's; at a merge whose file matches a parent, only that parent is
//! followed.

use crate::Error;
use crate::hash::Oid;
use crate::object::{Commit, Kind};
use crate::odb::Odb;
use crate::tree;
use std::collections::{BinaryHeap, HashSet};

pub fn commit(odb: &Odb, id: &Oid) -> Result<Commit, Error> {
    Commit::parse(odb.hash, &odb.read_kind(id, Kind::Commit)?)
}

pub struct Walk<'a> {
    odb: &'a Odb,
    heap: BinaryHeap<(i64, u64, Oid)>,
    seen: HashSet<Oid>,
    seq: u64,
}

impl<'a> Walk<'a> {
    pub fn new(odb: &'a Odb, starts: &[Oid]) -> Result<Walk<'a>, Error> {
        let mut w = Walk {
            odb,
            heap: BinaryHeap::new(),
            seen: HashSet::new(),
            seq: u64::MAX,
        };
        for s in starts {
            w.push(*s)?;
        }
        Ok(w)
    }

    fn push(&mut self, id: Oid) -> Result<(), Error> {
        if self.seen.insert(id) {
            let c = commit(self.odb, &id)?;
            // ties keep the order they were pushed in
            self.seq -= 1;
            self.heap.push((c.committer.time, self.seq, id));
        }
        Ok(())
    }

    /// Queues only some parents of the commit just returned (for path simplification).
    fn next_raw(&mut self) -> Option<Result<(Oid, Commit), Error>> {
        let (_, _, id) = self.heap.pop()?;
        Some(commit(self.odb, &id).map(|c| (id, c)))
    }
}

impl Iterator for Walk<'_> {
    type Item = Result<(Oid, Commit), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.next_raw()?;
        if let Ok((_, c)) = &r {
            for p in c.parents.clone() {
                if let Err(e) = self.push(p) {
                    return Some(Err(e));
                }
            }
        }
        Some(r)
    }
}

/// The blob (and mode) at `path` in a commit's tree.
pub fn file_at(odb: &Odb, c: &Commit, path: &str) -> Result<Option<(u32, Oid)>, Error> {
    Ok(tree::lookup(odb, &c.tree, path)?
        .filter(|e| !e.is_tree())
        .map(|e| (e.mode, e.id)))
}

/// One listed commit of a path's history: the file there (None: the commit removed it).
#[derive(Clone, Debug)]
pub struct PathCommit {
    pub id: Oid,
    pub commit: Commit,
    pub file: Option<Oid>,
}

/// `git log -- path` from `start`, newest first, at most `limit` entries.
pub fn path_history(odb: &Odb, start: &Oid, path: &str, limit: usize) -> Result<Vec<PathCommit>, Error> {
    let mut w = Walk::new(odb, &[*start])?;
    let mut out = vec![];
    while let Some(r) = w.next_raw() {
        let (id, c) = r?;
        let here = file_at(odb, &c, path)?;
        let mut same = None;
        let mut parents = vec![];
        for p in &c.parents {
            let pc = commit(odb, p)?;
            let there = file_at(odb, &pc, path)?;
            if there == here && same.is_none() {
                same = Some(*p);
            }
            parents.push(*p);
        }
        match same {
            Some(p) => w.push(p)?,
            None => {
                for p in parents {
                    w.push(p)?;
                }
                if here.is_some() || !c.parents.is_empty() {
                    out.push(PathCommit {
                        id,
                        commit: c,
                        file: here.map(|h| h.1),
                    });
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    Ok(out)
}
