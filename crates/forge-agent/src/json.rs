//! Just enough JSON for replies and JSON-RPC requests.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(f) => f.iter().find(|x| x.0 == key).map(|x| &x.1),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) if n.is_finite() => out.push_str(&crate::short(*n)),
            Json::Num(_) => out.push_str("null"),
            Json::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(f) => {
                out.push('{');
                for (i, (k, v)) in f.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    Json::Str(k.clone()).write(out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    pub fn parse(s: &str) -> Result<Json, String> {
        let b = s.as_bytes();
        let mut k = 0;
        let v = value(b, &mut k)?;
        ws(b, &mut k);
        if k != b.len() {
            return Err(format!("trailing characters at {k}"));
        }
        Ok(v)
    }
}

impl std::fmt::Display for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        self.write(&mut s);
        f.write_str(&s)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

impl From<f64> for Json {
    fn from(n: f64) -> Json {
        Json::Num(n)
    }
}

impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Num(n as f64)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

fn ws(b: &[u8], k: &mut usize) {
    while *k < b.len() && b[*k].is_ascii_whitespace() {
        *k += 1;
    }
}

fn value(b: &[u8], k: &mut usize) -> Result<Json, String> {
    ws(b, k);
    let lit = |k: &mut usize, w: &str, v: Json| {
        if b[*k..].starts_with(w.as_bytes()) {
            *k += w.len();
            Ok(v)
        } else {
            Err(format!("bad literal at {k}"))
        }
    };
    match b.get(*k) {
        None => Err("unexpected end".into()),
        Some(b'n') => lit(k, "null", Json::Null),
        Some(b't') => lit(k, "true", Json::Bool(true)),
        Some(b'f') => lit(k, "false", Json::Bool(false)),
        Some(b'"') => string(b, k).map(Json::Str),
        Some(b'[') => {
            *k += 1;
            let mut a = Vec::new();
            ws(b, k);
            if b.get(*k) == Some(&b']') {
                *k += 1;
                return Ok(Json::Arr(a));
            }
            loop {
                a.push(value(b, k)?);
                ws(b, k);
                match b.get(*k) {
                    Some(b',') => *k += 1,
                    Some(b']') => {
                        *k += 1;
                        return Ok(Json::Arr(a));
                    }
                    _ => return Err(format!("expected , or ] at {k}")),
                }
            }
        }
        Some(b'{') => {
            *k += 1;
            let mut f = Vec::new();
            ws(b, k);
            if b.get(*k) == Some(&b'}') {
                *k += 1;
                return Ok(Json::Obj(f));
            }
            loop {
                ws(b, k);
                let key = string(b, k)?;
                ws(b, k);
                if b.get(*k) != Some(&b':') {
                    return Err(format!("expected : at {k}"));
                }
                *k += 1;
                f.push((key, value(b, k)?));
                ws(b, k);
                match b.get(*k) {
                    Some(b',') => *k += 1,
                    Some(b'}') => {
                        *k += 1;
                        return Ok(Json::Obj(f));
                    }
                    _ => return Err(format!("expected , or }} at {k}")),
                }
            }
        }
        Some(_) => {
            let start = *k;
            while *k < b.len() && matches!(b[*k], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                *k += 1;
            }
            std::str::from_utf8(&b[start..*k])
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Json::Num)
                .ok_or_else(|| format!("bad value at {start}"))
        }
    }
}

fn string(b: &[u8], k: &mut usize) -> Result<String, String> {
    if b.get(*k) != Some(&b'"') {
        return Err(format!("expected a string at {k}"));
    }
    *k += 1;
    let mut out: Vec<u8> = Vec::new();
    loop {
        match b.get(*k) {
            None => return Err("unterminated string".into()),
            Some(b'"') => {
                *k += 1;
                return String::from_utf8(out).map_err(|_| "bad utf-8".to_string());
            }
            Some(b'\\') => {
                let c = *b.get(*k + 1).ok_or("unterminated escape")?;
                *k += 2;
                match c {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'u' => {
                        let hex = |k: usize| {
                            std::str::from_utf8(b.get(k..k + 4).unwrap_or(&[]))
                                .ok()
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                        };
                        let mut cp = hex(*k).ok_or("bad \\u escape")?;
                        *k += 4;
                        if (0xd800..0xdc00).contains(&cp) && b.get(*k..*k + 2) == Some(b"\\u") {
                            let lo = hex(*k + 2).ok_or("bad \\u escape")?;
                            cp = 0x10000 + ((cp - 0xd800) << 10) + (lo.wrapping_sub(0xdc00) & 0x3ff);
                            *k += 6;
                        }
                        let ch = char::from_u32(cp).unwrap_or('\u{fffd}');
                        out.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                    }
                    c => out.push(c),
                }
            }
            Some(&c) => {
                out.push(c);
                *k += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let src = r#"{"a":[1,-2.5,true,null],"b":"x\"y\né😀","c":{}}"#;
        let v = Json::parse(src).expect("parses");
        assert_eq!(v.get("b").and_then(Json::str), Some("x\"y\né😀"));
        assert_eq!(Json::parse(&v.to_string()).expect("reparses"), v);
    }
}
