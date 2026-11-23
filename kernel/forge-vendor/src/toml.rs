//! The TOML subset a vendor file is written in: `key = value` lines, `[table]` and `[[array]]`
//! headers (dotted, `[[material.stock]]` belonging to the last `[[material]]`), inline tables
//! `{ k = v, .. }`, arrays (nested too), basic strings with `\"`, `\\`, `\n`, `\t` escapes,
//! numbers, booleans and `#` comments. Keys are bare (`A-Za-z0-9_-`) or quoted. Nothing else:
//! dates are strings, there are no multi-line strings and no literal (`'..'`) strings.

/// A value as written.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Str(String),
    Num(f64),
    Bool(bool),
    Arr(Vec<Item>),
    Table(Table),
    /// `[[name]]` records.
    Tables(Vec<Table>),
}

/// Keys in the order written.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Table(pub Vec<(String, Item)>);

impl Table {
    pub fn get(&self, k: &str) -> Option<&Item> {
        self.0.iter().find(|x| x.0 == k).map(|x| &x.1)
    }

    fn get_mut(&mut self, k: &str) -> Option<&mut Item> {
        self.0.iter_mut().find(|x| x.0 == k).map(|x| &mut x.1)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|x| x.0.as_str())
    }

    pub fn set(&mut self, k: &str, v: Item) {
        match self.get_mut(k) {
            Some(x) => *x = v,
            None => self.0.push((k.into(), v)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    /// 1-based.
    pub line: usize,
    pub why: String,
}

fn err(line: usize, why: impl Into<String>) -> ParseError {
    ParseError {
        line,
        why: why.into(),
    }
}

struct Cur<'a> {
    s: &'a [u8],
    i: usize,
    line: usize,
}

impl Cur<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    /// Spaces and tabs; with `lines`, newlines and comments too (inside arrays).
    fn blank(&mut self, lines: bool) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' => self.i += 1,
                b'\n' if lines => {
                    self.line += 1;
                    self.i += 1;
                }
                b'#' if lines => {
                    while self.peek().is_some_and(|c| c != b'\n') {
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn key(&mut self) -> Result<String, ParseError> {
        self.blank(false);
        if self.peek() == Some(b'"') {
            return self.string();
        }
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            self.i += 1;
        }
        if start == self.i {
            return Err(err(self.line, "a key"));
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
    }

    /// `a.b.c`
    fn path(&mut self) -> Result<Vec<String>, ParseError> {
        let mut out = vec![self.key()?];
        loop {
            self.blank(false);
            if !self.eat(b'.') {
                return Ok(out);
            }
            out.push(self.key()?);
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        if !self.eat(b'"') {
            return Err(err(self.line, "a string"));
        }
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => return Err(err(self.line, "a string left open")),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(out).map_err(|_| err(self.line, "text that is not UTF-8"));
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = match self.peek() {
                        Some(b'"') => b'"',
                        Some(b'\\') => b'\\',
                        Some(b'n') => b'\n',
                        Some(b't') => b'\t',
                        _ => return Err(err(self.line, "an escape this does not read")),
                    };
                    out.push(c);
                    self.i += 1;
                }
                Some(c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn value(&mut self) -> Result<Item, ParseError> {
        self.blank(false);
        match self.peek() {
            Some(b'"') => Ok(Item::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut out = vec![];
                loop {
                    self.blank(true);
                    if self.eat(b']') {
                        return Ok(Item::Arr(out));
                    }
                    out.push(self.value()?);
                    self.blank(true);
                    if self.eat(b',') {
                        continue;
                    }
                    self.blank(true);
                    if self.eat(b']') {
                        return Ok(Item::Arr(out));
                    }
                    return Err(err(self.line, "a `,` or `]` in an array"));
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut t = Table::default();
                self.blank(false);
                if self.eat(b'}') {
                    return Ok(Item::Table(t));
                }
                loop {
                    let k = self.key()?;
                    self.blank(false);
                    if !self.eat(b'=') {
                        return Err(err(self.line, "a `=` after the key"));
                    }
                    let v = self.value()?;
                    if t.get(&k).is_some() {
                        return Err(err(self.line, format!("`{k}` twice")));
                    }
                    t.0.push((k, v));
                    self.blank(false);
                    if self.eat(b'}') {
                        return Ok(Item::Table(t));
                    }
                    if !self.eat(b',') {
                        return Err(err(self.line, "a `,` or `}` in an inline table"));
                    }
                }
            }
            _ => {
                let start = self.i;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'+' | b'_'))
                {
                    self.i += 1;
                }
                let w = String::from_utf8_lossy(&self.s[start..self.i]).replace('_', "");
                match w.as_str() {
                    "true" => Ok(Item::Bool(true)),
                    "false" => Ok(Item::Bool(false)),
                    _ => w
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .map(Item::Num)
                        .ok_or_else(|| err(self.line, "a value (a string, number, boolean, array or table)")),
                }
            }
        }
    }

    /// The rest of the line holds nothing but a comment.
    fn end(&mut self) -> Result<(), ParseError> {
        self.blank(false);
        match self.peek() {
            None | Some(b'\n') | Some(b'#') => {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.i += 1;
                }
                Ok(())
            }
            _ => Err(err(self.line, "more after the value")),
        }
    }
}

/// The table a header path names, made as needed; `[[..]]` paths step into the last record.
fn walk<'a>(root: &'a mut Table, path: &[String], line: usize) -> Result<&'a mut Table, ParseError> {
    let mut t = root;
    for k in path {
        if t.get(k).is_none() {
            t.0.push((k.clone(), Item::Table(Table::default())));
        }
        t = match t.get_mut(k).expect("just made") {
            Item::Table(x) => x,
            Item::Tables(xs) => xs.last_mut().expect("a record list is never empty"),
            _ => return Err(err(line, format!("`{k}` is a value, not a table"))),
        };
    }
    Ok(t)
}

pub fn parse(src: &str) -> Result<Table, ParseError> {
    let mut root = Table::default();
    // the header in force: its path, and whether it is a record list
    let mut at: Vec<String> = vec![];
    let mut c = Cur {
        s: src.as_bytes(),
        i: 0,
        line: 1,
    };
    loop {
        c.blank(false);
        match c.peek() {
            None => return Ok(root),
            Some(b'\n') => {
                c.i += 1;
                c.line += 1;
            }
            Some(b'#') => c.end()?,
            Some(b'[') => {
                c.i += 1;
                let many = c.eat(b'[');
                let path = c.path()?;
                c.blank(false);
                if !(c.eat(b']') && (!many || c.eat(b']'))) {
                    return Err(err(c.line, "a header's closing bracket"));
                }
                c.end()?;
                let (last, parent) = path.split_last().expect("a path has a key");
                let t = walk(&mut root, parent, c.line)?;
                match (many, t.get_mut(last)) {
                    (true, None) => t.0.push((last.clone(), Item::Tables(vec![Table::default()]))),
                    (true, Some(Item::Tables(xs))) => xs.push(Table::default()),
                    (false, None) => t.0.push((last.clone(), Item::Table(Table::default()))),
                    (false, Some(Item::Table(_))) => {
                        return Err(err(c.line, format!("[{}] twice", path.join("."))));
                    }
                    _ => return Err(err(c.line, format!("`{last}` written two ways"))),
                }
                at = path;
            }
            Some(_) => {
                let k = c.key()?;
                c.blank(false);
                if !c.eat(b'=') {
                    return Err(err(c.line, "a `=` after the key"));
                }
                let v = c.value()?;
                c.end()?;
                let line = c.line;
                let t = walk(&mut root, &at, line)?;
                if t.get(&k).is_some() {
                    return Err(err(line, format!("`{k}` twice")));
                }
                t.0.push((k, v));
            }
        }
    }
}

fn bare(k: &str) -> bool {
    !k.is_empty()
        && k.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn key_text(k: &str) -> String {
    if bare(k) { k.into() } else { quote(k) }
}

pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A number as short as reads back the same.
pub fn num(v: f64) -> String {
    format!("{v}")
}

fn inline(v: &Item) -> String {
    match v {
        Item::Str(s) => quote(s),
        Item::Num(x) => num(*x),
        Item::Bool(b) => b.to_string(),
        Item::Arr(xs) => format!("[{}]", xs.iter().map(inline).collect::<Vec<_>>().join(", ")),
        Item::Table(t) => {
            if t.0.is_empty() {
                return "{}".into();
            }
            let parts: Vec<String> =
                t.0.iter()
                    .map(|(k, v)| format!("{} = {}", key_text(k), inline(v)))
                    .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        Item::Tables(ts) => format!(
            "[{}]",
            ts.iter()
                .map(|t| inline(&Item::Table(t.clone())))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Canonical text: a table's values, then its sub-tables and record lists under headers.
/// `inline_keys` are tables written inline wherever they appear (`relief = { .. }`).
pub fn write(root: &Table, inline_keys: &[&str]) -> String {
    let mut out = String::new();
    write_into(&mut out, root, &[], inline_keys);
    out
}

fn write_into(out: &mut String, t: &Table, path: &[String], inline_keys: &[&str]) {
    let is_inline =
        |k: &str, v: &Item| !matches!(v, Item::Table(_) | Item::Tables(_)) || inline_keys.contains(&k);
    for (k, v) in t.0.iter().filter(|(k, v)| is_inline(k, v)) {
        out.push_str(&format!("{} = {}\n", key_text(k), inline(v)));
    }
    for (k, v) in t.0.iter().filter(|(k, v)| !is_inline(k, v)) {
        let mut p = path.to_vec();
        p.push(key_text(k));
        match v {
            Item::Table(sub) => {
                out.push_str(&format!("\n[{}]\n", p.join(".")));
                write_into(out, sub, &p, inline_keys);
            }
            Item::Tables(subs) => {
                for sub in subs {
                    out.push_str(&format!("\n[[{}]]\n", p.join(".")));
                    write_into(out, sub, &p, inline_keys);
                }
            }
            _ => unreachable!("values written above"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_headers_records_and_inline_tables() {
        let src = r#"
format = 1   # the version
name = "A \"shop\""
[limits]
max_flat = [44, 30]
[[material]]
name = "steel"
[[material.stock]]
t = 0.06
relief = { shape = "rect", width = "1t" }
ktable = [[0.5, 0.33],
          [2, 0.42]]   # rows
[[material.stock]]
t = 0.12
[[material]]
name = "alu"
"#;
        let t = parse(src).unwrap();
        assert_eq!(t.get("format"), Some(&Item::Num(1.0)));
        assert_eq!(t.get("name"), Some(&Item::Str("A \"shop\"".into())));
        let Some(Item::Tables(ms)) = t.get("material") else {
            panic!()
        };
        assert_eq!(ms.len(), 2);
        let Some(Item::Tables(ss)) = ms[0].get("stock") else {
            panic!()
        };
        assert_eq!(ss.len(), 2);
        assert!(matches!(ss[0].get("relief"), Some(Item::Table(_))));
        assert_eq!(
            ss[0].get("ktable"),
            Some(&Item::Arr(vec![
                Item::Arr(vec![Item::Num(0.5), Item::Num(0.33)]),
                Item::Arr(vec![Item::Num(2.0), Item::Num(0.42)]),
            ]))
        );
        // written and read back: the same tables
        let again = parse(&write(&t, &["relief"])).unwrap();
        assert_eq!(again, t);
    }

    #[test]
    fn refuses_with_the_line() {
        assert_eq!(parse("a = 1\na = 2\n").unwrap_err().line, 2);
        assert_eq!(parse("a = \"open\n").unwrap_err().line, 1);
        assert_eq!(parse("\n\nx = 1 2\n").unwrap_err().line, 3);
        assert!(parse("[t]\n[t]\n").is_err());
        assert!(parse("a = nope\n").is_err());
    }
}
