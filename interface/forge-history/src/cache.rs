//! The saves list, kept beside the repository (`<git dir>/forgecad/history-v1`) so that a long
//! history opens at once: one line per commit, appended as saves are made and when a walk from
//! HEAD meets commits the file lacks. It is derived: dropped or stale, it is rebuilt; what is
//! listed is always what HEAD reaches, so a history moved by another tool lists right.

use crate::History;
use forge_git::object::Sig;
use forge_git::{Error, Oid, diff};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub id: Oid,
    pub parents: Vec<Oid>,
    pub time: i64,
    pub author: String,
    pub agent: Option<String>,
    pub summary: String,
    /// The paths it changed (against its first parent).
    pub paths: Vec<String>,
}

fn file(h: &History) -> PathBuf {
    h.repo.common.join("forgecad/history-v1")
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn line(r: &Rec) -> String {
    let parents: Vec<String> = r.parents.iter().map(|p| p.hex()).collect();
    let mut s = format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        r.id.hex(),
        parents.join(" "),
        r.time,
        esc(&r.author),
        esc(r.agent.as_deref().unwrap_or("")),
        esc(&r.summary)
    );
    for p in &r.paths {
        s.push('\t');
        s.push_str(&esc(p));
    }
    s.push('\n');
    s
}

fn parse(h: &History, l: &str) -> Option<Rec> {
    let f: Vec<&str> = l.split('\t').collect();
    if f.len() < 6 {
        return None;
    }
    let k = h.repo.hash;
    let parents = f[1]
        .split(' ')
        .filter(|x| !x.is_empty())
        .map(|x| Oid::from_hex(k, x))
        .collect::<Option<Vec<_>>>()?;
    let agent = unesc(f[4]);
    Some(Rec {
        id: Oid::from_hex(k, f[0])?,
        parents,
        time: f[2].parse().ok()?,
        author: unesc(f[3]),
        agent: (!agent.is_empty()).then_some(agent),
        summary: unesc(f[5]),
        paths: f[6..].iter().map(|p| unesc(p)).collect(),
    })
}

fn append(h: &History, recs: &[Rec]) {
    if recs.is_empty() {
        return;
    }
    let p = file(h);
    let _ = std::fs::create_dir_all(p.parent().expect("dir"));
    let mut text = String::new();
    for r in recs {
        text.push_str(&line(r));
    }
    // one write of whole lines: concurrent appenders interleave by line at worst
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = f.write_all(text.as_bytes());
    }
}

fn rec_of(h: &History, id: &Oid) -> Result<Rec, Error> {
    let c = h.repo.commit(id)?;
    let base = match c.parents.first() {
        Some(p) => Some(h.repo.commit(p)?.tree),
        None => None,
    };
    let paths = diff::trees(&h.repo.odb, base.as_ref(), Some(&c.tree))?
        .iter()
        .map(|x| x.path().to_string())
        .collect();
    Ok(Rec {
        id: *id,
        parents: c.parents.clone(),
        time: c.committer.time,
        author: c.author.name.clone(),
        agent: crate::trailer(&c.message, crate::TRAILER_AGENT),
        summary: c.summary().to_string(),
        paths,
    })
}

/// A save just made: on the end of the file, if there is one (else the next look builds it).
pub fn appended(h: &History, id: &Oid, parent: Option<Oid>, who: &Sig, msg: &str, changed: &[String]) {
    if !file(h).exists() {
        return;
    }
    append(
        h,
        &[Rec {
            id: *id,
            parents: parent.into_iter().collect(),
            time: who.time,
            author: who.name.clone(),
            agent: crate::trailer(msg, crate::TRAILER_AGENT),
            summary: msg.lines().next().unwrap_or("").to_string(),
            paths: changed.to_vec(),
        }],
    );
}

/// Every commit HEAD reaches, newest first.
pub fn log(h: &History) -> Result<Vec<Rec>, Error> {
    let Some(head) = h.head()? else {
        return Ok(vec![]);
    };
    let mut known: HashMap<Oid, Rec> = HashMap::new();
    if let Ok(t) = std::fs::read_to_string(file(h)) {
        for l in t.lines() {
            if let Some(r) = parse(h, l) {
                known.insert(r.id, r);
            }
        }
    }
    let mut new = vec![];
    let mut stack = vec![head];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if known.contains_key(&id) || !seen.insert(id) {
            continue;
        }
        let r = rec_of(h, &id)?;
        stack.extend(r.parents.iter().copied());
        new.push(r);
    }
    append(h, &new);
    for r in new {
        known.insert(r.id, r);
    }
    // what HEAD reaches, through the parents the file records
    let mut out = vec![];
    let mut stack = vec![head];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(r) = known.get(&id) else {
            continue;
        };
        stack.extend(r.parents.iter().copied());
        out.push(r.clone());
    }
    // newest first; a tie keeps the walk's order (children before parents)
    let order: HashMap<Oid, usize> = out.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
    out.sort_by(|a, b| b.time.cmp(&a.time).then(order[&a.id].cmp(&order[&b.id])));
    Ok(out)
}
