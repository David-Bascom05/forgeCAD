//! The four object kinds as git writes them (gitformat-objects): parsed for reading, and written
//! back byte for byte, so an object read and written again has its own id.

use crate::Error;
use crate::hash::{HashKind, Oid};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Commit => "commit",
            Kind::Tree => "tree",
            Kind::Blob => "blob",
            Kind::Tag => "tag",
        }
    }

    pub fn from_name(n: &[u8]) -> Option<Kind> {
        Some(match n {
            b"commit" => Kind::Commit,
            b"tree" => Kind::Tree,
            b"blob" => Kind::Blob,
            b"tag" => Kind::Tag,
            _ => return None,
        })
    }

    /// The type number in a pack.
    pub fn from_pack(t: u8) -> Option<Kind> {
        Some(match t {
            1 => Kind::Commit,
            2 => Kind::Tree,
            3 => Kind::Blob,
            4 => Kind::Tag,
            _ => return None,
        })
    }
}

/// `type size\0`: what an object's id hashes before its content.
pub fn header(kind: Kind, len: usize) -> Vec<u8> {
    format!("{} {len}\0", kind.name()).into_bytes()
}

pub fn id_of(hash: HashKind, kind: Kind, data: &[u8]) -> Oid {
    let mut h = hash.hasher();
    h.update(&header(kind, data.len()));
    h.update(data);
    h.finish()
}

pub const MODE_TREE: u32 = 0o40000;
pub const MODE_FILE: u32 = 0o100644;
pub const MODE_EXEC: u32 = 0o100755;
pub const MODE_LINK: u32 = 0o120000;
pub const MODE_GITLINK: u32 = 0o160000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub mode: u32,
    pub name: Vec<u8>,
    pub id: Oid,
}

impl Entry {
    pub fn is_tree(&self) -> bool {
        self.mode == MODE_TREE
    }
}

/// A tree's entries in git's order: by name, a tree's name taken as if it ended in `/`.
pub fn parse_tree(hash: HashKind, d: &[u8]) -> Result<Vec<Entry>, Error> {
    let mut out = vec![];
    let mut i = 0;
    let n = hash.len();
    while i < d.len() {
        let sp = d[i..]
            .iter()
            .position(|&c| c == b' ')
            .ok_or(Error::Corrupt("tree mode"))?
            + i;
        let mut mode = 0u32;
        for &c in &d[i..sp] {
            if !(b'0'..=b'7').contains(&c) {
                return Err(Error::Corrupt("tree mode"));
            }
            mode = mode.checked_mul(8).ok_or(Error::Corrupt("tree mode"))? + (c - b'0') as u32;
        }
        let nul = d[sp..]
            .iter()
            .position(|&c| c == 0)
            .ok_or(Error::Corrupt("tree name"))?
            + sp;
        let id = Oid::from_bytes(
            hash,
            d.get(nul + 1..nul + 1 + n).ok_or(Error::Corrupt("tree id"))?,
        )
        .ok_or(Error::Corrupt("tree id"))?;
        out.push(Entry {
            mode,
            name: d[sp + 1..nul].to_vec(),
            id,
        });
        i = nul + 1 + n;
    }
    Ok(out)
}

pub fn tree_order(a: &Entry, b: &Entry) -> std::cmp::Ordering {
    let key = |e: &Entry, i: usize| -> u8 {
        match e.name.get(i) {
            Some(&c) => c,
            None if e.is_tree() => b'/',
            None => 0,
        }
    };
    let n = a.name.len().max(b.name.len()) + 1;
    for i in 0..n {
        let (x, y) = (key(a, i), key(b, i));
        if x != y {
            return x.cmp(&y);
        }
        if i >= a.name.len() && i >= b.name.len() {
            break;
        }
    }
    std::cmp::Ordering::Equal
}

pub fn write_tree(entries: &[Entry]) -> Vec<u8> {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| tree_order(a, b));
    let mut out = vec![];
    for e in sorted {
        out.extend_from_slice(format!("{:o} ", e.mode).as_bytes());
        out.extend_from_slice(&e.name);
        out.push(0);
        out.extend_from_slice(e.id.as_bytes());
    }
    out
}

/// Who and when: `Name <email> 1762430000 +0100`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sig {
    pub name: String,
    pub email: String,
    pub time: i64,
    /// Minutes east of UTC.
    pub tz: i32,
}

impl Sig {
    pub fn parse(v: &[u8]) -> Option<Sig> {
        let lt = v.iter().position(|&c| c == b'<')?;
        let gt = v.iter().rposition(|&c| c == b'>')?;
        let name = String::from_utf8_lossy(v[..lt].strip_suffix(b" ").unwrap_or(&v[..lt])).to_string();
        let email = String::from_utf8_lossy(&v[lt + 1..gt]).to_string();
        let rest = std::str::from_utf8(&v[gt + 1..]).ok()?.trim();
        let mut it = rest.split(' ');
        let time = it.next()?.parse().ok()?;
        let tz = it.next().unwrap_or("+0000");
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let digits: i32 = tz.trim_start_matches(['+', '-']).parse().ok()?;
        Some(Sig {
            name,
            email,
            time,
            tz: sign * ((digits / 100) * 60 + digits % 100),
        })
    }

    pub fn write(&self) -> String {
        let a = self.tz.abs();
        format!(
            "{} <{}> {} {}{:02}{:02}",
            self.name,
            self.email,
            self.time,
            if self.tz < 0 { '-' } else { '+' },
            a / 60,
            a % 60
        )
    }
}

/// A commit or tag: its header lines (a continued value keeps its newlines) and its message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Headed {
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub message: Vec<u8>,
}

impl Headed {
    pub fn parse(d: &[u8]) -> Result<Headed, Error> {
        let mut headers: Vec<(Vec<u8>, Vec<u8>)> = vec![];
        let mut i = 0;
        loop {
            if i >= d.len() {
                return Ok(Headed {
                    headers,
                    message: vec![],
                });
            }
            let end = d[i..].iter().position(|&c| c == b'\n').map_or(d.len(), |p| p + i);
            let line = &d[i..end];
            let next = (end + 1).min(d.len());
            if line.is_empty() {
                return Ok(Headed {
                    headers,
                    message: d[next..].to_vec(),
                });
            }
            if line[0] == b' ' {
                let last = headers.last_mut().ok_or(Error::Corrupt("header"))?;
                last.1.push(b'\n');
                last.1.extend_from_slice(&line[1..]);
            } else {
                let sp = line.iter().position(|&c| c == b' ').unwrap_or(line.len());
                let v = line.get(sp + 1..).unwrap_or(&[]).to_vec();
                headers.push((line[..sp].to_vec(), v));
            }
            if end >= d.len() {
                return Ok(Headed {
                    headers,
                    message: vec![],
                });
            }
            i = next;
        }
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = vec![];
        for (k, v) in &self.headers {
            out.extend_from_slice(k);
            out.push(b' ');
            for &c in v {
                out.push(c);
                if c == b'\n' {
                    out.push(b' ');
                }
            }
            out.push(b'\n');
        }
        out.push(b'\n');
        out.extend_from_slice(&self.message);
        out
    }

    pub fn get(&self, k: &str) -> Option<&[u8]> {
        self.headers
            .iter()
            .find(|h| h.0 == k.as_bytes())
            .map(|h| h.1.as_slice())
    }

    pub fn all<'a>(&'a self, k: &'a str) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.headers
            .iter()
            .filter(move |h| h.0 == k.as_bytes())
            .map(|h| h.1.as_slice())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub author: Sig,
    pub committer: Sig,
    pub message: String,
}

impl Commit {
    pub fn parse(hash: HashKind, d: &[u8]) -> Result<Commit, Error> {
        let h = Headed::parse(d)?;
        let id = |v: &[u8]| {
            std::str::from_utf8(v)
                .ok()
                .and_then(|s| Oid::from_hex(hash, s))
                .ok_or(Error::Corrupt("commit id"))
        };
        let sig = |k: &str| {
            h.get(k)
                .and_then(Sig::parse)
                .ok_or(Error::Corrupt("commit signature"))
        };
        Ok(Commit {
            tree: id(h.get("tree").ok_or(Error::Corrupt("commit tree"))?)?,
            parents: h.all("parent").map(id).collect::<Result<_, _>>()?,
            author: sig("author")?,
            committer: sig("committer")?,
            message: String::from_utf8_lossy(&h.message).to_string(),
        })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut s = format!("tree {}\n", self.tree);
        for p in &self.parents {
            s.push_str(&format!("parent {p}\n"));
        }
        s.push_str(&format!("author {}\n", self.author.write()));
        s.push_str(&format!("committer {}\n\n", self.committer.write()));
        s.push_str(&self.message);
        s.into_bytes()
    }

    pub fn summary(&self) -> &str {
        self.message.lines().next().unwrap_or("")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub object: Oid,
    pub kind: Kind,
    pub name: String,
    pub tagger: Option<Sig>,
    pub message: String,
}

impl Tag {
    pub fn parse(hash: HashKind, d: &[u8]) -> Result<Tag, Error> {
        let h = Headed::parse(d)?;
        let s = |k: &str| h.get(k).and_then(|v| std::str::from_utf8(v).ok());
        Ok(Tag {
            object: s("object")
                .and_then(|x| Oid::from_hex(hash, x))
                .ok_or(Error::Corrupt("tag object"))?,
            kind: h
                .get("type")
                .and_then(Kind::from_name)
                .ok_or(Error::Corrupt("tag type"))?,
            name: s("tag").ok_or(Error::Corrupt("tag name"))?.to_string(),
            tagger: h.get("tagger").and_then(Sig::parse),
            message: String::from_utf8_lossy(&h.message).to_string(),
        })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut s = format!(
            "object {}\ntype {}\ntag {}\n",
            self.object,
            self.kind.name(),
            self.name
        );
        if let Some(t) = &self.tagger {
            s.push_str(&format!("tagger {}\n", t.write()));
        }
        s.push('\n');
        s.push_str(&self.message);
        s.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headed_round_trips_continued_lines() {
        let d = b"tree 1\nparent 2\ngpgsig -----BEGIN\n line two\n \n end\nauthor A <a> 1 +0000\n\nmsg\n";
        let h = Headed::parse(d).unwrap();
        assert_eq!(h.write(), d);
        assert_eq!(h.get("gpgsig").unwrap(), b"-----BEGIN\nline two\n\nend");
    }

    #[test]
    fn sig_tz() {
        let s = Sig::parse(b"David Bascom <d@x> 1762430000 -0530").unwrap();
        assert_eq!(s.tz, -330);
        assert_eq!(s.write(), "David Bascom <d@x> 1762430000 -0530");
    }

    #[test]
    fn tree_sorts_dirs_with_slash() {
        let z = HashKind::Sha1.null();
        let f = |n: &str, m| Entry {
            mode: m,
            name: n.into(),
            id: z,
        };
        let t = write_tree(&[f("a.b", MODE_FILE), f("a", MODE_TREE), f("a-", MODE_FILE)]);
        let back = parse_tree(HashKind::Sha1, &t).unwrap();
        let names: Vec<_> = back
            .iter()
            .map(|e| String::from_utf8_lossy(&e.name).to_string())
            .collect();
        assert_eq!(names, ["a-", "a.b", "a"]);
    }
}
