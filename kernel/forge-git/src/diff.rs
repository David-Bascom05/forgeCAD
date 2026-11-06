//! What changed between two trees, file by file; equal subtrees are skipped by id.

use crate::Error;
use crate::hash::Oid;
use crate::object::{Entry, tree_order};
use crate::odb::Odb;
use crate::tree;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(String, Entry),
    Removed(String, Entry),
    Modified(String, Entry, Entry),
}

impl Change {
    pub fn path(&self) -> &str {
        match self {
            Change::Added(p, _) | Change::Removed(p, _) | Change::Modified(p, _, _) => p,
        }
    }
}

pub fn trees(odb: &Odb, a: Option<&Oid>, b: Option<&Oid>) -> Result<Vec<Change>, Error> {
    let mut out = vec![];
    rec(odb, a, b, "", &mut out)?;
    Ok(out)
}

fn rec(
    odb: &Odb,
    a: Option<&Oid>,
    b: Option<&Oid>,
    prefix: &str,
    out: &mut Vec<Change>,
) -> Result<(), Error> {
    if a == b {
        return Ok(());
    }
    let ea = match a {
        Some(t) => tree::read(odb, t)?,
        None => vec![],
    };
    let eb = match b {
        Some(t) => tree::read(odb, t)?,
        None => vec![],
    };
    let (mut i, mut j) = (0, 0);
    while i < ea.len() || j < eb.len() {
        let ord = match (ea.get(i), eb.get(j)) {
            (Some(x), Some(y)) => {
                // a file and a tree of one name sort apart; compare names alone
                match x.name.cmp(&y.name) {
                    Ordering::Equal => Ordering::Equal,
                    _ => tree_order(x, y),
                }
            }
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Greater,
        };
        match ord {
            Ordering::Less => {
                side(odb, &ea[i], prefix, true, out)?;
                i += 1;
            }
            Ordering::Greater => {
                side(odb, &eb[j], prefix, false, out)?;
                j += 1;
            }
            Ordering::Equal => {
                let (x, y) = (&ea[i], &eb[j]);
                let p = format!("{prefix}{}", String::from_utf8_lossy(&x.name));
                match (x.is_tree(), y.is_tree()) {
                    (true, true) => rec(odb, Some(&x.id), Some(&y.id), &format!("{p}/"), out)?,
                    (false, false) => {
                        if x.id != y.id || x.mode != y.mode {
                            out.push(Change::Modified(p, x.clone(), y.clone()));
                        }
                    }
                    _ => {
                        side(odb, x, prefix, true, out)?;
                        side(odb, y, prefix, false, out)?;
                    }
                }
                i += 1;
                j += 1;
            }
        }
    }
    Ok(())
}

fn side(odb: &Odb, e: &Entry, prefix: &str, removed: bool, out: &mut Vec<Change>) -> Result<(), Error> {
    let p = format!("{prefix}{}", String::from_utf8_lossy(&e.name));
    if e.is_tree() {
        let (a, b) = if removed {
            (Some(&e.id), None)
        } else {
            (None, Some(&e.id))
        };
        return rec(odb, a, b, &format!("{p}/"), out);
    }
    out.push(if removed {
        Change::Removed(p, e.clone())
    } else {
        Change::Added(p, e.clone())
    });
    Ok(())
}
