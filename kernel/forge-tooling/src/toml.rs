//! The TOML subset the CAM libraries are written in: `[table]`, `[[array.of.tables]]` (a
//! dotted name nests under the last element of its parent array), `key = value` with strings
//! (basic, with \" \\ \n \t escapes), integers and floats, booleans, arrays (nesting allowed),
//! and `#` comments. Anything else is a typed error with its line, never a guess.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Str(String),
    Num(f64),
    Bool(bool),
    Arr(Vec<Val>),
    Table(Table),
    Tables(Vec<Table>),
}

pub type Table = BTreeMap<String, Val>;

#[derive(Clone, Debug, PartialEq)]
pub struct TomlError {
    pub line: usize,
    pub what: String,
}

fn err(line: usize, what: &str) -> TomlError {
    TomlError {
        line,
        what: what.to_string(),
    }
}

/// The table at a dotted path, made as needed; for an array of tables, its last element.
fn walk<'a>(root: &'a mut Table, path: &[String], line: usize) -> Result<&'a mut Table, TomlError> {
    let mut t = root;
    for k in path {
        let e = t.entry(k.clone()).or_insert_with(|| Val::Table(Table::new()));
        t = match e {
            Val::Table(x) => x,
            Val::Tables(v) => v.last_mut().ok_or_else(|| err(line, "empty array of tables"))?,
            _ => return Err(err(line, &format!("`{k}` is not a table"))),
        };
    }
    Ok(t)
}

pub fn parse(src: &str) -> Result<Table, TomlError> {
    let mut root = Table::new();
    let mut cur: Vec<String> = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let mut s = strip_comment(lines[i]).trim().to_string();
        i += 1;
        if s.is_empty() {
            continue;
        }
        if s.starts_with("[[") {
            let name = s
                .strip_prefix("[[")
                .and_then(|x| x.strip_suffix("]]"))
                .ok_or_else(|| err(ln, "bad [[table]] header"))?;
            let path: Vec<String> = name.split('.').map(|x| x.trim().to_string()).collect();
            let (last, parent) = path.split_last().unwrap();
            let t = walk(&mut root, parent, ln)?;
            match t.entry(last.clone()).or_insert_with(|| Val::Tables(Vec::new())) {
                Val::Tables(v) => v.push(Table::new()),
                _ => return Err(err(ln, &format!("`{last}` is not an array of tables"))),
            }
            cur = path;
            continue;
        }
        if s.starts_with('[') {
            let name = s
                .strip_prefix('[')
                .and_then(|x| x.strip_suffix(']'))
                .ok_or_else(|| err(ln, "bad [table] header"))?;
            cur = name.split('.').map(|x| x.trim().to_string()).collect();
            walk(&mut root, &cur, ln)?;
            continue;
        }
        let eq = s.find('=').ok_or_else(|| err(ln, "expected key = value"))?;
        let key = s[..eq].trim().trim_matches('"').to_string();
        if key.is_empty() {
            return Err(err(ln, "empty key"));
        }
        // multi-line arrays: keep reading until the brackets balance
        while depth(&s[eq + 1..]) > 0 {
            if i >= lines.len() {
                return Err(err(ln, "unclosed array"));
            }
            s.push(' ');
            s.push_str(strip_comment(lines[i]).trim());
            i += 1;
        }
        let mut p = Parser {
            b: s[eq + 1..].trim().as_bytes(),
            at: 0,
            line: ln,
        };
        let v = p.value()?;
        p.ws();
        if p.at != p.b.len() {
            return Err(err(ln, "trailing text after the value"));
        }
        let t = walk(&mut root, &cur, ln)?;
        if t.insert(key.clone(), v).is_some() {
            return Err(err(ln, &format!("`{key}` given twice")));
        }
    }
    Ok(root)
}

fn strip_comment(l: &str) -> &str {
    let mut in_str = false;
    let mut esc = false;
    for (k, c) in l.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '#' {
            return &l[..k];
        }
    }
    l
}

fn depth(s: &str) -> i32 {
    let mut d = 0;
    let mut in_str = false;
    for c in s.chars() {
        match c {
            '"' => in_str = !in_str,
            '[' if !in_str => d += 1,
            ']' if !in_str => d -= 1,
            _ => {}
        }
    }
    d
}

struct Parser<'a> {
    b: &'a [u8],
    at: usize,
    line: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.at < self.b.len() && (self.b[self.at] == b' ' || self.b[self.at] == b'\t') {
            self.at += 1;
        }
    }
    fn value(&mut self) -> Result<Val, TomlError> {
        self.ws();
        let c = *self
            .b
            .get(self.at)
            .ok_or_else(|| err(self.line, "missing value"))?;
        match c {
            b'"' => {
                self.at += 1;
                let mut out = String::new();
                loop {
                    let c = *self
                        .b
                        .get(self.at)
                        .ok_or_else(|| err(self.line, "unclosed string"))?;
                    self.at += 1;
                    match c {
                        b'"' => break,
                        b'\\' => {
                            let e = *self.b.get(self.at).ok_or_else(|| err(self.line, "bad escape"))?;
                            self.at += 1;
                            out.push(match e {
                                b'"' => '"',
                                b'\\' => '\\',
                                b'n' => '\n',
                                b't' => '\t',
                                _ => return Err(err(self.line, "unknown escape")),
                            });
                        }
                        _ => {
                            // copy the UTF-8 sequence whole
                            let start = self.at - 1;
                            let mut end = self.at;
                            while end < self.b.len() && (self.b[end] & 0xC0) == 0x80 {
                                end += 1;
                            }
                            out.push_str(
                                std::str::from_utf8(&self.b[start..end])
                                    .map_err(|_| err(self.line, "bad UTF-8"))?,
                            );
                            self.at = end;
                        }
                    }
                }
                Ok(Val::Str(out))
            }
            b'[' => {
                self.at += 1;
                let mut v = Vec::new();
                loop {
                    self.ws();
                    if self.b.get(self.at) == Some(&b']') {
                        self.at += 1;
                        break;
                    }
                    v.push(self.value()?);
                    self.ws();
                    match self.b.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            break;
                        }
                        _ => return Err(err(self.line, "expected , or ] in an array")),
                    }
                }
                Ok(Val::Arr(v))
            }
            b't' if self.b[self.at..].starts_with(b"true") => {
                self.at += 4;
                Ok(Val::Bool(true))
            }
            b'f' if self.b[self.at..].starts_with(b"false") => {
                self.at += 5;
                Ok(Val::Bool(false))
            }
            _ => {
                let start = self.at;
                while self.at < self.b.len()
                    && matches!(
                        self.b[self.at],
                        b'0'..=b'9' | b'.' | b'-' | b'+' | b'e' | b'E' | b'_'
                    )
                {
                    self.at += 1;
                }
                let t: String = std::str::from_utf8(&self.b[start..self.at])
                    .unwrap()
                    .replace('_', "");
                t.parse::<f64>()
                    .map(Val::Num)
                    .map_err(|_| err(self.line, &format!("not a value: `{t}`")))
            }
        }
    }
}

/// Typed getters for a record, each refusing with the field's name.
pub fn num(t: &Table, k: &str) -> Option<f64> {
    match t.get(k) {
        Some(Val::Num(x)) => Some(*x),
        _ => None,
    }
}
pub fn text<'a>(t: &'a Table, k: &str) -> Option<&'a str> {
    match t.get(k) {
        Some(Val::Str(x)) => Some(x),
        _ => None,
    }
}
pub fn boolean(t: &Table, k: &str) -> Option<bool> {
    match t.get(k) {
        Some(Val::Bool(x)) => Some(*x),
        _ => None,
    }
}
pub fn tables<'a>(t: &'a Table, k: &str) -> &'a [Table] {
    match t.get(k) {
        Some(Val::Tables(v)) => v,
        _ => &[],
    }
}
pub fn nums(t: &Table, k: &str) -> Option<Vec<f64>> {
    match t.get(k) {
        Some(Val::Arr(v)) => v
            .iter()
            .map(|x| if let Val::Num(n) = x { Some(*n) } else { None })
            .collect(),
        _ => None,
    }
}
pub fn texts(t: &Table, k: &str) -> Option<Vec<String>> {
    match t.get(k) {
        Some(Val::Arr(v)) => v
            .iter()
            .map(|x| {
                if let Val::Str(n) = x {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_subset() {
        let t = parse(
            "# c\n[[tool]]\nid = \"a\" # x\ndc = 6.0\nok = true\n[[tool.preset]]\nvc = 300\n[[tool]]\nid = \"b#\"\nprofile = [[0, 50.0],\n  [26, 42]]\n[meta]\nname = \"x \\\"y\\\"\"\n",
        )
        .unwrap();
        let tools = tables(&t, "tool");
        assert_eq!(tools.len(), 2);
        assert_eq!(num(&tools[0], "dc"), Some(6.0));
        assert_eq!(tables(&tools[0], "preset").len(), 1);
        assert_eq!(text(&tools[1], "id"), Some("b#"));
        assert!(matches!(tools[1].get("profile"), Some(Val::Arr(v)) if v.len() == 2));
        assert!(parse("x = \n").is_err());
        assert!(parse("x = 1\nx = 2\n").is_err());
    }
}
