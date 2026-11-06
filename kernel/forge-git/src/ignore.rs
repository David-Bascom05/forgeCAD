//! Which paths git ignores (gitignore(5)): `core.excludesFile`, `info/exclude`, and every
//! `.gitignore` from the root down to the path, the last matching pattern winning; a path under
//! an ignored directory is ignored whatever later patterns say.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Rule {
    pat: Vec<u8>,
    negate: bool,
    dir_only: bool,
    /// Matched against the path from the file's directory, not the last component alone.
    anchored: bool,
}

fn parse(text: &str) -> Vec<Rule> {
    let mut out = vec![];
    for line in text.lines() {
        let mut l = line.as_bytes().to_vec();
        if l.first() == Some(&b'#') {
            continue;
        }
        // trailing spaces go unless escaped
        while l.last() == Some(&b' ') && !(l.len() >= 2 && l[l.len() - 2] == b'\\') {
            l.pop();
        }
        if l.is_empty() {
            continue;
        }
        let negate = l[0] == b'!';
        if negate {
            l.remove(0);
        }
        let dir_only = l.last() == Some(&b'/');
        if dir_only {
            l.pop();
        }
        let anchored = l.contains(&b'/');
        if l.first() == Some(&b'/') {
            l.remove(0);
        }
        if l.is_empty() {
            continue;
        }
        out.push(Rule {
            pat: l,
            negate,
            dir_only,
            anchored,
        });
    }
    out
}

pub struct Ignore {
    root: PathBuf,
    /// Pattern files and the directory (relative to the root) they are in.
    base: Vec<(String, Vec<Rule>)>,
    cache: HashMap<String, Vec<Rule>>,
}

impl Ignore {
    pub fn new(root: &Path, git_dir: &Path, excludes_file: Option<PathBuf>) -> Ignore {
        let mut base = vec![];
        let global = excludes_file.or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                .map(|x| x.join("git/ignore"))
        });
        if let Some(g) = global
            && let Ok(t) = std::fs::read_to_string(g)
        {
            base.push((String::new(), parse(&t)));
        }
        if let Ok(t) = std::fs::read_to_string(git_dir.join("info/exclude")) {
            base.push((String::new(), parse(&t)));
        }
        Ignore {
            root: root.to_path_buf(),
            base,
            cache: HashMap::new(),
        }
    }

    fn rules_in(&mut self, dir: &str) -> Vec<Rule> {
        if let Some(r) = self.cache.get(dir) {
            return r.clone();
        }
        let p = if dir.is_empty() {
            self.root.join(".gitignore")
        } else {
            self.root.join(dir).join(".gitignore")
        };
        let r = std::fs::read_to_string(p).map(|t| parse(&t)).unwrap_or_default();
        self.cache.insert(dir.to_string(), r.clone());
        r
    }

    fn one(&mut self, path: &str, is_dir: bool) -> bool {
        let mut files: Vec<(String, Vec<Rule>)> = self.base.clone();
        let comps: Vec<&str> = path.split('/').collect();
        for k in 0..comps.len() {
            let d = comps[..k].join("/");
            files.push((d.clone(), self.rules_in(&d)));
        }
        let name = comps.last().copied().unwrap_or("");
        let mut ignored = false;
        for (dir, rules) in &files {
            let rel = if dir.is_empty() {
                path
            } else {
                match path.strip_prefix(dir.as_str()).and_then(|r| r.strip_prefix('/')) {
                    Some(r) => r,
                    None => continue,
                }
            };
            for r in rules {
                if r.dir_only && !is_dir {
                    continue;
                }
                let hit = if r.anchored {
                    wildmatch(&r.pat, rel.as_bytes(), true)
                } else {
                    wildmatch(&r.pat, name.as_bytes(), true)
                };
                if hit {
                    ignored = !r.negate;
                }
            }
        }
        ignored
    }

    /// Whether git ignores `path` (relative to the work tree, `/`-separated).
    pub fn ignored(&mut self, path: &str, is_dir: bool) -> bool {
        let comps: Vec<&str> = path.split('/').collect();
        for k in 1..comps.len() {
            if self.one(&comps[..k].join("/"), true) {
                return true;
            }
        }
        self.one(path, is_dir)
    }
}

/// git's wildmatch: `*` and `?` stop at `/` (with `pathname`), `**/`, `/**` and `/**/` cross
/// directories, `[..]` classes with ranges, `!` or `^` negation and `[:name:]`, `\` escapes.
pub fn wildmatch(p: &[u8], t: &[u8], pathname: bool) -> bool {
    m(p, t, pathname)
}

fn m(p: &[u8], t: &[u8], pn: bool) -> bool {
    let (mut pi, mut ti) = (0, 0);
    while pi < p.len() {
        match p[pi] {
            b'?' => {
                if ti >= t.len() || (pn && t[ti] == b'/') {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
            b'*' => {
                let double = pi + 1 < p.len() && p[pi + 1] == b'*';
                if double && pn {
                    let start_ok = pi == 0 || p[pi - 1] == b'/';
                    let mut q = pi;
                    while q < p.len() && p[q] == b'*' {
                        q += 1;
                    }
                    let end_ok = q == p.len() || p[q] == b'/';
                    if start_ok && end_ok {
                        if q == p.len() {
                            return true;
                        }
                        // `**/` matches zero or more whole directories
                        let rest = &p[q + 1..];
                        let mut k = ti;
                        loop {
                            if m(rest, &t[k..], pn) {
                                return true;
                            }
                            match t[k..].iter().position(|&c| c == b'/') {
                                Some(s) => k += s + 1,
                                None => return false,
                            }
                        }
                    }
                }
                let mut q = pi;
                while q < p.len() && p[q] == b'*' {
                    q += 1;
                }
                let rest = &p[q..];
                let mut k = ti;
                loop {
                    if m(rest, &t[k..], pn) {
                        return true;
                    }
                    if k >= t.len() || (pn && t[k] == b'/') {
                        return false;
                    }
                    k += 1;
                }
            }
            b'[' => {
                if ti >= t.len() || (pn && t[ti] == b'/') {
                    return false;
                }
                let Some((hit, used)) = class(&p[pi..], t[ti]) else {
                    // no closing bracket: a literal `[`
                    if t[ti] != b'[' {
                        return false;
                    }
                    pi += 1;
                    ti += 1;
                    continue;
                };
                if !hit {
                    return false;
                }
                pi += used;
                ti += 1;
            }
            b'\\' if pi + 1 < p.len() => {
                if ti >= t.len() || t[ti] != p[pi + 1] {
                    return false;
                }
                pi += 2;
                ti += 1;
            }
            c => {
                if ti >= t.len() || t[ti] != c {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }
    ti == t.len()
}

/// A `[..]` class at the start of `p`: whether `c` is in it, and its length.
fn class(p: &[u8], c: u8) -> Option<(bool, usize)> {
    let mut i = 1;
    let neg = matches!(p.get(i), Some(b'!') | Some(b'^'));
    if neg {
        i += 1;
    }
    let mut hit = false;
    let mut first = true;
    loop {
        let x = *p.get(i)?;
        if x == b']' && !first {
            return Some((hit != neg, i + 1));
        }
        first = false;
        if x == b'[' && p.get(i + 1) == Some(&b':') {
            let end = p[i + 2..].windows(2).position(|w| w == b":]")? + i + 2;
            let name = &p[i + 2..end];
            hit |= match name {
                b"alpha" => c.is_ascii_alphabetic(),
                b"digit" => c.is_ascii_digit(),
                b"alnum" => c.is_ascii_alphanumeric(),
                b"space" => c.is_ascii_whitespace(),
                b"upper" => c.is_ascii_uppercase(),
                b"lower" => c.is_ascii_lowercase(),
                b"punct" => c.is_ascii_punctuation(),
                b"xdigit" => c.is_ascii_hexdigit(),
                b"blank" => c == b' ' || c == b'\t',
                b"cntrl" => c.is_ascii_control(),
                b"graph" => c.is_ascii_graphic(),
                b"print" => c.is_ascii_graphic() || c == b' ',
                _ => false,
            };
            i = end + 2;
            continue;
        }
        let lo = if x == b'\\' {
            i += 1;
            *p.get(i)?
        } else {
            x
        };
        if p.get(i + 1) == Some(&b'-') && p.get(i + 2).is_some_and(|&y| y != b']') {
            let mut hi = p[i + 2];
            let mut used = 3;
            if hi == b'\\' {
                hi = *p.get(i + 3)?;
                used = 4;
            }
            hit |= lo <= c && c <= hi;
            i += used;
        } else {
            hit |= lo == c;
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildmatch_cases() {
        let w = |p: &str, t: &str| wildmatch(p.as_bytes(), t.as_bytes(), true);
        assert!(w("*.stl", "a.stl"));
        assert!(!w("*.stl", "d/a.stl"));
        assert!(w("**/a.stl", "d/e/a.stl"));
        assert!(w("**/a.stl", "a.stl"));
        assert!(w("d/**", "d/e/f"));
        assert!(w("a/**/b", "a/b"));
        assert!(w("a/**/b", "a/x/y/b"));
        assert!(w("[a-c]x", "bx"));
        assert!(!w("[!a-c]x", "bx"));
        assert!(w("[[:digit:]]", "7"));
        assert!(w("\\*", "*"));
        assert!(w("target*", "target-mesh"));
    }
}
