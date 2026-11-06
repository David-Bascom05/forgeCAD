//! git-config files (git-config(1), CONFIGURATION FILE): sections, quoted subsections, quoted
//! values, escapes, continued lines, comments, and `[include] path`. Keys are kept as
//! `section.subsection.key`, section and key in lower case; the last value set wins.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub entries: Vec<(String, String)>,
}

impl Config {
    /// The global files then the repo's (later ones win).
    pub fn load(git_common: &Path, git_dir: &Path) -> Config {
        let mut c = Config::default();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(g) = std::env::var_os("GIT_CONFIG_GLOBAL") {
            c.read_file(Path::new(&g), 0);
        } else {
            let xdg = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home.as_ref().map(|h| h.join(".config")));
            if let Some(x) = xdg {
                c.read_file(&x.join("git/config"), 0);
            }
            if let Some(h) = &home {
                c.read_file(&h.join(".gitconfig"), 0);
            }
        }
        c.read_file(&git_common.join("config"), 0);
        if c.bool("extensions.worktreeconfig") == Some(true) {
            c.read_file(&git_dir.join("config.worktree"), 0);
        }
        c
    }

    pub fn read_file(&mut self, p: &Path, depth: usize) {
        if depth > 10 {
            return;
        }
        let Ok(t) = std::fs::read_to_string(p) else {
            return;
        };
        let before = self.entries.len();
        self.parse(&t);
        // includes, in place: what they set is as if written where the include is
        let added: Vec<(String, String)> = self.entries.drain(before..).collect();
        for (k, v) in added {
            let inc = k == "include.path";
            self.entries.push((k, v.clone()));
            if inc {
                let path = expand(&v, p.parent());
                self.read_file(&path, depth + 1);
            }
        }
    }

    pub fn parse(&mut self, t: &str) {
        let c: Vec<char> = t.chars().collect();
        let mut i = 0;
        let mut section = String::new();
        while i < c.len() {
            let ch = c[i];
            if ch.is_whitespace() {
                i += 1;
                continue;
            }
            if ch == '#' || ch == ';' {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if ch == '[' {
                i += 1;
                let mut name = String::new();
                let mut sub: Option<String> = None;
                while i < c.len() && c[i] != ']' {
                    if c[i] == '"' {
                        i += 1;
                        let mut s = String::new();
                        while i < c.len() && c[i] != '"' {
                            if c[i] == '\\' && i + 1 < c.len() {
                                i += 1;
                            }
                            s.push(c[i]);
                            i += 1;
                        }
                        sub = Some(s);
                    } else if !c[i].is_whitespace() {
                        name.push(c[i]);
                    }
                    i += 1;
                }
                i += 1;
                let name = name.to_lowercase();
                // old style [section.sub]: the subsection lower-cased, as git does
                section = match sub {
                    Some(s) => format!("{name}.{s}"),
                    None => name,
                };
                continue;
            }
            let mut key = String::new();
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '-') {
                key.push(c[i]);
                i += 1;
            }
            if key.is_empty() {
                // a line we can't read: skip it
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            while i < c.len() && (c[i] == ' ' || c[i] == '\t') {
                i += 1;
            }
            let value = if i < c.len() && c[i] == '=' {
                i += 1;
                let mut v = String::new();
                let mut quoted = false;
                let mut pending_ws = String::new();
                while i < c.len() {
                    let x = c[i];
                    if x == '\n' && !quoted {
                        break;
                    }
                    if !quoted && (x == '#' || x == ';') {
                        while i < c.len() && c[i] != '\n' {
                            i += 1;
                        }
                        break;
                    }
                    if x == '"' {
                        quoted = !quoted;
                        v.push_str(&pending_ws);
                        pending_ws.clear();
                        i += 1;
                        continue;
                    }
                    if x == '\\' && i + 1 < c.len() {
                        i += 1;
                        let e = c[i];
                        let out = match e {
                            '\n' => None,
                            'n' => Some('\n'),
                            't' => Some('\t'),
                            'b' => None,
                            o => Some(o),
                        };
                        if let Some(o) = out {
                            v.push_str(&pending_ws);
                            pending_ws.clear();
                            v.push(o);
                        }
                        i += 1;
                        continue;
                    }
                    if x.is_whitespace() && !quoted {
                        if !v.is_empty() {
                            pending_ws.push(x);
                        }
                    } else {
                        v.push_str(&pending_ws);
                        pending_ws.clear();
                        v.push(x);
                    }
                    i += 1;
                }
                v
            } else {
                "true".to_string()
            };
            self.entries
                .push((format!("{section}.{}", key.to_lowercase()), value));
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        let key = lower_ends(key);
        self.entries
            .iter()
            .rev()
            .find(|e| e.0 == key)
            .map(|e| e.1.as_str())
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.to_lowercase().as_str() {
            "true" | "yes" | "on" | "1" | "" => Some(true),
            "false" | "no" | "off" | "0" => Some(false),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key)?.trim().parse().ok()
    }
}

/// Section and key lower-cased, a subsection as written.
fn lower_ends(key: &str) -> String {
    let first = key.find('.').unwrap_or(key.len());
    let last = key.rfind('.').unwrap_or(key.len());
    if first == last {
        return key.to_lowercase();
    }
    format!(
        "{}{}{}",
        key[..first].to_lowercase(),
        &key[first..last],
        key[last..].to_lowercase()
    )
}

pub fn expand(v: &str, base: Option<&Path>) -> PathBuf {
    if let Some(rest) = v.strip_prefix("~/")
        && let Some(h) = std::env::var_os("HOME")
    {
        return PathBuf::from(h).join(rest);
    }
    let p = PathBuf::from(v);
    match base {
        Some(b) if p.is_relative() => b.join(p),
        _ => p,
    }
}

/// A `[section] key = value` appended to a config file's text (git's quoting where needed).
pub fn append_value(text: &str, section: &str, key: &str, value: &str) -> String {
    let mut s = text.to_string();
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    let needs = value.starts_with(' ') || value.ends_with(' ') || value.contains(['#', ';', '"', '\\']);
    let v = if needs {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    };
    s.push_str(&format!("[{section}]\n\t{key} = {v}\n"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_syntax() {
        let mut c = Config::default();
        c.parse(
            "[core]\n\tbare = false\n\tFileMode\n[user]\n  name = \"David  Bascom\" # me\n  email = d@x ; c\n\
             [remote \"Origin\"]\n url = a\\\n b\n[extensions]\nobjectFormat = sha256\n",
        );
        assert_eq!(c.bool("core.bare"), Some(false));
        assert_eq!(c.bool("core.filemode"), Some(true));
        assert_eq!(c.get("user.name"), Some("David  Bascom"));
        assert_eq!(c.get("user.email"), Some("d@x"));
        assert_eq!(c.get("remote.Origin.url"), Some("a b"));
        assert_eq!(c.get("Extensions.ObjectFormat"), Some("sha256"));
        let t = append_value("", "user", "name", "a # b");
        let mut d = Config::default();
        d.parse(&t);
        assert_eq!(d.get("user.name"), Some("a # b"));
    }
}
