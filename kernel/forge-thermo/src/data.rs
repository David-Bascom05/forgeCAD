//! A small JSON reader for the material data (data/materials.json): the kernel takes no crates.
//! Numbers go through `str::parse`, which rounds correctly on every target.

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Val>),
    Obj(Vec<(String, Val)>),
}

impl Val {
    pub fn get(&self, k: &str) -> Option<&Val> {
        match self {
            Val::Obj(o) => o.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn num(&self) -> Option<f64> {
        match self {
            Val::Num(x) => Some(*x),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn arr(&self) -> &[Val] {
        match self {
            Val::Arr(a) => a,
            _ => &[],
        }
    }
    pub fn bool(&self) -> Option<bool> {
        match self {
            Val::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Where a read failed: the byte offset and what was wanted.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub at: usize,
    pub want: &'static str,
}

pub fn parse(s: &str) -> Result<Val, ParseError> {
    let b = s.as_bytes();
    let mut i = 0;
    let v = value(b, &mut i)?;
    ws(b, &mut i);
    if i != b.len() {
        return Err(ParseError {
            at: i,
            want: "the end",
        });
    }
    Ok(v)
}

fn ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && b[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn expect(b: &[u8], i: &mut usize, c: u8, want: &'static str) -> Result<(), ParseError> {
    ws(b, i);
    if b.get(*i) == Some(&c) {
        *i += 1;
        Ok(())
    } else {
        Err(ParseError { at: *i, want })
    }
}

fn value(b: &[u8], i: &mut usize) -> Result<Val, ParseError> {
    ws(b, i);
    let err = |at, want| ParseError { at, want };
    match b.get(*i) {
        Some(b'{') => {
            *i += 1;
            let mut o = vec![];
            ws(b, i);
            if b.get(*i) == Some(&b'}') {
                *i += 1;
                return Ok(Val::Obj(o));
            }
            loop {
                ws(b, i);
                let Val::Str(k) = string(b, i)? else {
                    unreachable!()
                };
                expect(b, i, b':', "':'")?;
                o.push((k, value(b, i)?));
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b'}') => {
                        *i += 1;
                        return Ok(Val::Obj(o));
                    }
                    _ => return Err(err(*i, "',' or '}'")),
                }
            }
        }
        Some(b'[') => {
            *i += 1;
            let mut a = vec![];
            ws(b, i);
            if b.get(*i) == Some(&b']') {
                *i += 1;
                return Ok(Val::Arr(a));
            }
            loop {
                a.push(value(b, i)?);
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b']') => {
                        *i += 1;
                        return Ok(Val::Arr(a));
                    }
                    _ => return Err(err(*i, "',' or ']'")),
                }
            }
        }
        Some(b'"') => string(b, i),
        Some(b't') if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(Val::Bool(true))
        }
        Some(b'f') if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(Val::Bool(false))
        }
        Some(b'n') if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(Val::Null)
        }
        Some(c) if *c == b'-' || c.is_ascii_digit() => {
            let s = *i;
            while *i < b.len() && matches!(b[*i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                *i += 1;
            }
            std::str::from_utf8(&b[s..*i])
                .ok()
                .and_then(|t| t.parse::<f64>().ok())
                .map(Val::Num)
                .ok_or(err(s, "a number"))
        }
        _ => Err(err(*i, "a value")),
    }
}

fn string(b: &[u8], i: &mut usize) -> Result<Val, ParseError> {
    if b.get(*i) != Some(&b'"') {
        return Err(ParseError {
            at: *i,
            want: "a string",
        });
    }
    *i += 1;
    let mut out = Vec::new();
    while *i < b.len() {
        match b[*i] {
            b'"' => {
                *i += 1;
                return String::from_utf8(out).map(Val::Str).map_err(|_| ParseError {
                    at: *i,
                    want: "UTF-8",
                });
            }
            b'\\' => {
                *i += 1;
                match b.get(*i) {
                    Some(b'n') => out.push(b'\n'),
                    Some(&c @ (b'"' | b'\\' | b'/')) => out.push(c),
                    _ => {
                        return Err(ParseError {
                            at: *i,
                            want: "an escape",
                        });
                    }
                }
                *i += 1;
            }
            c => {
                out.push(c);
                *i += 1;
            }
        }
    }
    Err(ParseError { at: *i, want: "'\"'" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_data_holds() {
        let v = parse(r#" {"a": [1, -2.5e-3, true, null], "b": {"c": "x\"y"}} "#).unwrap();
        assert_eq!(v.get("a").unwrap().arr()[1].num(), Some(-2.5e-3));
        assert_eq!(v.get("b").unwrap().get("c").unwrap().str(), Some("x\"y"));
        assert!(parse("[1, 2").is_err());
        assert!(parse("{\"a\" 1}").is_err());
    }
}
