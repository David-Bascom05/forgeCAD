//! Trees by path: looking a path up, listing a tree's files, and a new tree from an old one with
//! some paths set or removed, written bottom-up (only the trees on the edited paths are new).

use crate::Error;
use crate::hash::Oid;
use crate::object::{self, Entry, Kind, MODE_TREE};
use crate::odb::Odb;
use std::collections::BTreeMap;

pub fn read(odb: &Odb, id: &Oid) -> Result<Vec<Entry>, Error> {
    object::parse_tree(odb.hash, &odb.read_kind(id, Kind::Tree)?)
}

/// The entry at `path` (`a/b/c.fg`) in a tree.
pub fn lookup(odb: &Odb, tree: &Oid, path: &str) -> Result<Option<Entry>, Error> {
    let mut cur = *tree;
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for (i, p) in parts.iter().enumerate() {
        let entries = read(odb, &cur)?;
        let Some(e) = entries.into_iter().find(|e| e.name == p.as_bytes()) else {
            return Ok(None);
        };
        if i + 1 == parts.len() {
            return Ok(Some(e));
        }
        if !e.is_tree() {
            return Ok(None);
        }
        cur = e.id;
    }
    Ok(None)
}

/// Every non-tree entry, by its full path, in path order.
pub fn files(odb: &Odb, tree: &Oid) -> Result<Vec<(String, Entry)>, Error> {
    let mut out = vec![];
    walk(odb, tree, "", &mut out)?;
    Ok(out)
}

fn walk(odb: &Odb, tree: &Oid, prefix: &str, out: &mut Vec<(String, Entry)>) -> Result<(), Error> {
    for e in read(odb, tree)? {
        let p = format!("{prefix}{}", String::from_utf8_lossy(&e.name));
        if e.is_tree() {
            walk(odb, &e.id, &format!("{p}/"), out)?;
        } else {
            out.push((p, e));
        }
    }
    Ok(())
}

/// A path set to a file (mode, blob), or removed (None).
pub type Edit = (String, Option<(u32, Oid)>);

/// `base` (or the empty tree) with `edits` made: the new tree's id. Trees left empty go.
pub fn apply(odb: &Odb, base: Option<&Oid>, edits: &[Edit]) -> Result<Oid, Error> {
    let mut by_first: BTreeMap<String, Vec<Edit>> = BTreeMap::new();
    let mut here: BTreeMap<String, Option<(u32, Oid)>> = BTreeMap::new();
    for (path, v) in edits {
        let path = path.trim_matches('/');
        if path.is_empty()
            || path
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == ".." || c == ".git")
        {
            return Err(Error::NoPath(path.to_string()));
        }
        match path.split_once('/') {
            Some((a, rest)) => by_first
                .entry(a.to_string())
                .or_default()
                .push((rest.to_string(), *v)),
            None => {
                here.insert(path.to_string(), *v);
            }
        }
    }
    let mut entries = match base {
        Some(b) => read(odb, b)?,
        None => vec![],
    };
    for (name, v) in here {
        entries.retain(|e| e.name != name.as_bytes());
        if let Some((mode, id)) = v {
            entries.push(Entry {
                mode,
                name: name.into_bytes(),
                id,
            });
        }
    }
    for (name, sub) in by_first {
        let old = entries.iter().position(|e| e.name == name.as_bytes());
        let old_tree = old.and_then(|i| entries[i].is_tree().then_some(entries[i].id));
        if let Some(i) = old {
            entries.remove(i);
        }
        let id = apply(odb, old_tree.as_ref(), &sub)?;
        if read(odb, &id)?.is_empty() {
            continue;
        }
        entries.push(Entry {
            mode: MODE_TREE,
            name: name.into_bytes(),
            id,
        });
    }
    odb.write(Kind::Tree, &object::write_tree(&entries))
}
