//! ISO 10303-21 text. Files are untrusted: every input ends in a value or a clear error, memory
//! is bounded by `Limits`, recursion by `MAX_DEPTH`, and nothing here panics on any bytes.

use std::collections::BTreeMap;

/// How much of a file is held. A value takes 16 bytes, so `max_values` bounds the memory.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_values: usize,
    pub max_instances: usize,
    /// Evaluations of a file's spline surfaces one solid's check may spend (`forge_geom::budget`;
    /// measured, `research/step/README.md`).
    pub max_evals: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_bytes: 2 << 30,
            max_values: 64 << 20,
            max_instances: 16 << 20,
            max_evals: forge_geom::budget::FILE,
        }
    }
}

/// Lists inside lists, deeper than any schema here goes.
const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Real(f64),
    /// Byte range of a string's inside in the text, quotes doubled as written.
    Str(u32, u32),
    /// Byte range of an enumeration's name, without its dots.
    Enum(u32, u32),
    Ref(u32),
    Null,
    Derived,
    /// A run of `values`.
    List(u32, u32),
    /// A value with a type name: the name's number, and its one value in `values`.
    Typed(u16, u32),
}

/// One record of an instance: `KEYWORD(args)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub kw: u16,
    pub args: (u32, u32),
}

#[derive(Debug)]
pub struct File<'a> {
    pub text: &'a [u8],
    pub values: Vec<Value>,
    /// Keyword names, by number.
    pub names: Vec<&'a str>,
    /// Sorted by id; each with its run in `records` (more than one: a complex instance).
    pub ids: Vec<(u32, u32, u32)>,
    pub records: Vec<Record>,
    /// Ids given to two instances; the first is kept.
    pub duplicates: Vec<u32>,
    /// FILE_SCHEMA's first name, and FILE_NAME's originating system.
    pub schema: String,
    pub system: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub at: usize,
    pub line: usize,
    pub why: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {} (byte {}): {}", self.line, self.at, self.why)
    }
}

impl<'a> File<'a> {
    pub fn records_of(&self, id: u32) -> Option<&[Record]> {
        let i = self.ids.binary_search_by_key(&id, |x| x.0).ok()?;
        let (_, a, n) = self.ids[i];
        self.records.get(a as usize..(a + n) as usize)
    }
    pub fn name(&self, r: &Record) -> &'a str {
        self.names.get(r.kw as usize).copied().unwrap_or("")
    }
    pub fn args(&self, r: &Record) -> &[Value] {
        self.run(r.args.0, r.args.1)
    }
    pub fn run(&self, at: u32, n: u32) -> &[Value] {
        self.values
            .get(at as usize..at as usize + n as usize)
            .unwrap_or(&[])
    }
    pub fn str_of(&self, at: u32, n: u32) -> &'a str {
        self.text
            .get(at as usize..at as usize + n as usize)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
    f: File<'a>,
    lim: Limits,
    kw: BTreeMap<&'a [u8], u16>,
}

/// Parses a whole file.
pub fn parse<'a>(text: &'a [u8], lim: &Limits) -> Result<File<'a>, ParseError> {
    if text.len() > lim.max_bytes || text.len() > u32::MAX as usize {
        return Err(ParseError {
            at: 0,
            line: 1,
            why: format!("{} bytes is over the limit of {}", text.len(), lim.max_bytes),
        });
    }
    let mut p = P {
        b: text,
        i: 0,
        f: File {
            text,
            values: Vec::new(),
            names: Vec::new(),
            ids: Vec::new(),
            records: Vec::new(),
            duplicates: Vec::new(),
            schema: String::new(),
            system: String::new(),
        },
        lim: *lim,
        kw: BTreeMap::new(),
    };
    // a big file's instances read in pieces on all cores, where that reads them as one pass would
    match parse_pieces(text, lim) {
        Some(f) => p.f = f,
        None => p.file()?,
    }
    let mut ids = std::mem::take(&mut p.f.ids);
    // stable: the first of two instances with an id stays first
    ids.sort_by_key(|x| x.0);
    let mut out: Vec<(u32, u32, u32)> = Vec::with_capacity(ids.len());
    for x in ids {
        if out.last().is_some_and(|l| l.0 == x.0) {
            p.f.duplicates.push(x.0);
        } else {
            out.push(x);
        }
    }
    p.f.ids = out;
    Ok(p.f)
}

/// Below this many bytes a file is read in one pass.
const PIECES_FROM: usize = 4 << 20;

/// The file read in pieces, on all cores, where that gives what one pass ([`P::file`]) gives
/// before its ids are sorted; `None` (then one pass reads it) otherwise, or where one pass would
/// fail (one pass then says where and why).
///
/// One pass reads up to the DATA section's first instance; the instances are cut into pieces at
/// guessed instance starts (`;`, a line end, `#`); each piece is read from its start as one pass
/// reads instances, keywords and values numbered from 0. A piece read from a place one pass
/// reaches, to the next piece's start exactly, between instances, is what one pass reads there, so
/// the next piece starts at a place one pass reaches too: when every piece ends at the next's
/// start, the pieces are one pass's reading cut up. Put together in order, with each piece's
/// keywords numbered as one pass first meets them and its values after those before it, they
/// are one pass's file. The limits one pass holds to are held to on the totals, which only grow.
fn parse_pieces<'a>(text: &'a [u8], lim: &Limits) -> Option<File<'a>> {
    let threads = forge_math::par::threads();
    if text.len() < PIECES_FROM || threads < 2 {
        return None;
    }
    let mut head = P {
        b: text,
        i: 0,
        f: File {
            text,
            values: Vec::new(),
            names: Vec::new(),
            ids: Vec::new(),
            records: Vec::new(),
            duplicates: Vec::new(),
            schema: String::new(),
            system: String::new(),
        },
        lim: *lim,
        kw: BTreeMap::new(),
    };
    let data = head.to_data().ok()??;
    // the pieces' starts: the first instance start at or past each even split
    let n = (2 * threads).min(text.len() / (1 << 20)).max(2);
    let mut starts = vec![data];
    for k in 1..n {
        let at = (data + (text.len() - data) * k / n).max(*starts.last()? + 1);
        let s = find_start(text, at)?;
        if s > *starts.last()? {
            starts.push(s);
        }
    }
    let last = starts.len() - 1;
    let pieces = forge_math::par::map_eager(starts.len(), |k| {
        let mut p = P {
            b: text,
            i: starts[k],
            f: File {
                text,
                values: Vec::new(),
                names: Vec::new(),
                ids: Vec::new(),
                records: Vec::new(),
                duplicates: Vec::new(),
                schema: String::new(),
                system: String::new(),
            },
            lim: *lim,
            kw: BTreeMap::new(),
        };
        let ok = if k == last {
            p.data().is_ok() && p.after_data().unwrap_or(false)
        } else {
            p.instances_to(starts[k + 1]).unwrap_or(false)
        };
        ok.then_some(p.f)
    });
    let mut f = head.f;
    let mut kw: BTreeMap<&'a str, u16> = f.names.iter().enumerate().map(|(i, &n)| (n, i as u16)).collect();
    for piece in pieces {
        let piece = piece?;
        // the piece's keywords as one pass numbers them: in the order first met, after all before
        let mut map = Vec::with_capacity(piece.names.len());
        for &name in &piece.names {
            let k = match kw.get(name) {
                Some(&k) => k,
                None => {
                    let k = f.names.len();
                    if k >= u16::MAX as usize {
                        return None;
                    }
                    f.names.push(name);
                    kw.insert(name, k as u16);
                    k as u16
                }
            };
            map.push(k);
        }
        let (vb, rb) = (f.values.len() as u32, f.records.len() as u32);
        if f.values.len() + piece.values.len() > lim.max_values
            || f.ids.len() + piece.ids.len() > lim.max_instances
            || f.values.len() + piece.values.len() > u32::MAX as usize
        {
            return None;
        }
        f.values.extend(piece.values.iter().map(|v| match *v {
            Value::List(a, n) => Value::List(a + vb, n),
            Value::Typed(k, a) => Value::Typed(map[k as usize], a + vb),
            v => v,
        }));
        f.records.extend(piece.records.iter().map(|r| Record {
            kw: map[r.kw as usize],
            args: (r.args.0 + vb, r.args.1),
        }));
        f.ids.extend(piece.ids.iter().map(|&(id, a, n)| (id, a + rb, n)));
    }
    if f.ids.is_empty() {
        return None;
    }
    Some(f)
}

/// The first instance start at or past `at`: a `#` beginning a line that follows a `;`.
fn find_start(text: &[u8], at: usize) -> Option<usize> {
    let mut i = at;
    while i < text.len() {
        let j = i + text.get(i..)?.iter().position(|&c| c == b'#')?;
        let before = &text[..j];
        let line = before
            .strip_suffix(b"\n")
            .map(|b| b.strip_suffix(b"\r").unwrap_or(b));
        if line.is_some_and(|b| b.ends_with(b";")) {
            return Some(j);
        }
        i = j + 1;
    }
    None
}

impl<'a> P<'a> {
    fn err<T>(&self, why: impl Into<String>) -> Result<T, ParseError> {
        let at = self.i.min(self.b.len());
        let line = 1 + self.b[..at].iter().filter(|&&c| c == b'\n').count();
        Err(ParseError {
            at,
            line,
            why: why.into(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn space(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.i += 1,
                Some(b'/') if self.b.get(self.i + 1) == Some(&b'*') => {
                    let start = self.i;
                    self.i += 2;
                    loop {
                        match self.peek() {
                            None => {
                                self.i = start;
                                return self.err("a comment is never closed");
                            }
                            Some(b'*') if self.b.get(self.i + 1) == Some(&b'/') => {
                                self.i += 2;
                                break;
                            }
                            Some(_) => self.i += 1,
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn eat(&mut self, c: u8) -> Result<bool, ParseError> {
        self.space()?;
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), ParseError> {
        if self.eat(c)? {
            Ok(())
        } else {
            let got = self
                .peek()
                .map_or("the end of the file".to_string(), |x| format!("'{}'", x as char));
            self.err(format!("expected '{}', found {got}", c as char))
        }
    }

    fn word(&mut self) -> Result<&'a [u8], ParseError> {
        self.space()?;
        let a = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
                self.i += 1;
            } else {
                break;
            }
        }
        Ok(&self.b[a..self.i])
    }

    fn keyword(&mut self, w: &'a [u8]) -> Result<u16, ParseError> {
        if let Some(&k) = self.kw.get(w) {
            return Ok(k);
        }
        let k = self.f.names.len();
        if k >= u16::MAX as usize {
            return self.err("too many different keywords");
        }
        let Ok(s) = std::str::from_utf8(w) else {
            return self.err("a keyword that is not text");
        };
        self.f.names.push(s);
        self.kw.insert(w, k as u16);
        Ok(k as u16)
    }

    fn file(&mut self) -> Result<(), ParseError> {
        let w = self.word()?;
        if w != b"ISO-10303-21" {
            return self.err("not a STEP file: it does not start with ISO-10303-21;");
        }
        self.expect(b';')?;
        loop {
            let w = self.word()?;
            match w {
                b"HEADER" => {
                    self.expect(b';')?;
                    self.header()?;
                }
                b"DATA" => {
                    // DATA may carry a name and schema list before its semicolon
                    self.space()?;
                    if self.peek() == Some(b'(') {
                        let mut sink = Vec::new();
                        self.value(&mut sink, 0)?;
                    }
                    self.expect(b';')?;
                    self.data()?;
                }
                b"ANCHOR" | b"REFERENCE" | b"SIGNATURE" => {
                    self.skip_section()?;
                }
                b"END-ISO-10303-21" => {
                    self.expect(b';')?;
                    return Ok(());
                }
                // a file cut short after its data is still its data
                b"" if self.peek().is_none() && !self.f.ids.is_empty() => return Ok(()),
                b"" if self.peek().is_none() => return self.err("the file ends with no data"),
                _ => return self.err("expected HEADER, DATA or END-ISO-10303-21"),
            }
        }
    }

    /// One pass's start of a file, to the DATA section's first instance: where that is, or `None`
    /// where the file goes another way before it (one pass then reads it).
    fn to_data(&mut self) -> Result<Option<usize>, ParseError> {
        let w = self.word()?;
        if w != b"ISO-10303-21" {
            return Ok(None);
        }
        self.expect(b';')?;
        loop {
            let w = self.word()?;
            match w {
                b"HEADER" => {
                    self.expect(b';')?;
                    self.header()?;
                }
                b"DATA" => {
                    self.space()?;
                    if self.peek() == Some(b'(') {
                        let mut sink = Vec::new();
                        self.value(&mut sink, 0)?;
                    }
                    self.expect(b';')?;
                    self.space()?;
                    return Ok((self.peek() == Some(b'#')).then_some(self.i));
                }
                b"ANCHOR" | b"REFERENCE" | b"SIGNATURE" => {
                    self.skip_section()?;
                }
                _ => return Ok(None),
            }
        }
    }

    /// Instances, as `data` reads them, up to `end`: whether they end there exactly.
    fn instances_to(&mut self, end: usize) -> Result<bool, ParseError> {
        let mut scratch = Vec::new();
        loop {
            self.space()?;
            if self.i >= end {
                return Ok(self.i == end);
            }
            if self.peek() != Some(b'#') {
                return Ok(false);
            }
            self.instance(&mut scratch)?;
        }
    }

    /// What follows the DATA section, as `file` reads it, where it is only other sections and the
    /// end: whether it is.
    fn after_data(&mut self) -> Result<bool, ParseError> {
        loop {
            let w = self.word()?;
            match w {
                b"ANCHOR" | b"REFERENCE" | b"SIGNATURE" => {
                    self.skip_section()?;
                }
                b"END-ISO-10303-21" => {
                    self.expect(b';')?;
                    return Ok(true);
                }
                b"" if self.peek().is_none() && !self.f.ids.is_empty() => return Ok(true),
                _ => return Ok(false),
            }
        }
    }

    fn skip_section(&mut self) -> Result<(), ParseError> {
        loop {
            self.space()?;
            match self.peek() {
                None => return self.err("a section is never closed"),
                Some(b'\'') => {
                    let mut sink = Vec::new();
                    self.value(&mut sink, 0)?;
                }
                Some(_) => {
                    let w = self.word()?;
                    if w == b"ENDSEC" {
                        return self.expect(b';');
                    }
                    if w.is_empty() {
                        self.i += 1;
                    }
                }
            }
        }
    }

    fn header(&mut self) -> Result<(), ParseError> {
        loop {
            let w = self.word()?;
            if w == b"ENDSEC" {
                return self.expect(b';');
            }
            if w.is_empty() {
                return self.err("expected a header entity or ENDSEC");
            }
            self.expect(b'(')?;
            let mut args = Vec::new();
            self.values_until_close(&mut args, 1)?;
            self.expect(b';')?;
            let text = |v: &Value| match *v {
                Value::Str(a, n) => self.f.str_of(a, n).to_string(),
                _ => String::new(),
            };
            match w {
                b"FILE_SCHEMA" => {
                    if let Some(Value::List(a, n)) = args.first()
                        && let Some(v) = self
                            .f
                            .values
                            .get(*a as usize..(*a + *n) as usize)
                            .and_then(|r| r.first())
                    {
                        let s = text(v);
                        let name = s.split([' ', '{']).next().unwrap_or("");
                        self.f.schema = name.to_ascii_uppercase();
                    }
                }
                b"FILE_NAME" => {
                    // name, time, authors, organisations, preprocessor, originating system, auth
                    if let Some(v) = args.get(5) {
                        self.f.system = text(v);
                    }
                }
                _ => {}
            }
        }
    }

    fn data(&mut self) -> Result<(), ParseError> {
        let mut scratch = Vec::new();
        loop {
            self.space()?;
            match self.peek() {
                Some(b'#') => self.instance(&mut scratch)?,
                Some(b'E') => {
                    let w = self.word()?;
                    if w != b"ENDSEC" {
                        return self.err("expected an instance or ENDSEC");
                    }
                    return self.expect(b';');
                }
                // a file cut short in its data
                None => return Ok(()),
                Some(c) => {
                    return self.err(format!("expected an instance, found '{}'", c as char));
                }
            }
        }
    }

    fn id(&mut self) -> Result<u32, ParseError> {
        self.i += 1; // the #
        let a = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        let digits = &self.b[a..self.i];
        let n = std::str::from_utf8(digits)
            .ok()
            .and_then(|s| s.parse::<u32>().ok());
        match n {
            Some(n) => Ok(n),
            None => self.err("an instance number that is empty or over 32 bits"),
        }
    }

    fn instance(&mut self, scratch: &mut Vec<Value>) -> Result<(), ParseError> {
        if self.f.ids.len() >= self.lim.max_instances {
            return self.err(format!("over the limit of {} instances", self.lim.max_instances));
        }
        let id = self.id()?;
        self.expect(b'=')?;
        let first = self.f.records.len();
        self.space()?;
        if self.eat(b'(')? {
            // complex: records until the closing bracket
            loop {
                if self.eat(b')')? {
                    break;
                }
                self.record(scratch)?;
            }
        } else {
            self.record(scratch)?;
        }
        let n = self.f.records.len() - first;
        if n == 0 {
            return self.err("an instance with no records");
        }
        self.expect(b';')?;
        self.f.ids.push((id, first as u32, n as u32));
        Ok(())
    }

    fn record(&mut self, scratch: &mut Vec<Value>) -> Result<(), ParseError> {
        let w = self.word()?;
        if w.is_empty() {
            return self.err("expected a keyword");
        }
        let kw = self.keyword(w)?;
        self.expect(b'(')?;
        scratch.clear();
        self.values_until_close(scratch, 1)?;
        let at = self.push_run(scratch)?;
        self.f.records.push(Record {
            kw,
            args: (at, scratch.len() as u32),
        });
        Ok(())
    }

    fn push_run(&mut self, run: &[Value]) -> Result<u32, ParseError> {
        let at = self.f.values.len();
        if at + run.len() > self.lim.max_values {
            return self.err(format!("over the limit of {} values", self.lim.max_values));
        }
        self.f.values.extend_from_slice(run);
        Ok(at as u32)
    }

    /// Values up to and past the closing bracket, the opening one eaten.
    fn values_until_close(&mut self, out: &mut Vec<Value>, depth: usize) -> Result<(), ParseError> {
        if self.eat(b')')? {
            return Ok(());
        }
        loop {
            self.value(out, depth)?;
            if self.eat(b',')? {
                continue;
            }
            return self.expect(b')');
        }
    }

    fn value(&mut self, out: &mut Vec<Value>, depth: usize) -> Result<(), ParseError> {
        if depth > MAX_DEPTH {
            return self.err("lists nested too deep");
        }
        self.space()?;
        let Some(c) = self.peek() else {
            return self.err("the file ends inside a value");
        };
        let v = match c {
            b'#' => Value::Ref(self.id()?),
            b'$' => {
                self.i += 1;
                Value::Null
            }
            b'*' => {
                self.i += 1;
                Value::Derived
            }
            b'\'' => self.string()?,
            b'"' => {
                // binary: skipped to its closing quote, kept as an empty string
                self.i += 1;
                while let Some(x) = self.peek() {
                    self.i += 1;
                    if x == b'"' {
                        break;
                    }
                }
                Value::Str(0, 0)
            }
            b'.' if self
                .b
                .get(self.i + 1)
                .is_some_and(|x| x.is_ascii_alphabetic() || *x == b'_') =>
            {
                self.i += 1;
                let a = self.i;
                while self
                    .peek()
                    .is_some_and(|x| x.is_ascii_alphanumeric() || x == b'_')
                {
                    self.i += 1;
                }
                let n = self.i - a;
                if self.peek() != Some(b'.') {
                    return self.err("an enumeration is never closed");
                }
                self.i += 1;
                Value::Enum(a as u32, n as u32)
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.number()?,
            b'(' => {
                self.i += 1;
                let mut inner = Vec::new();
                self.values_until_close(&mut inner, depth + 1)?;
                let at = self.push_run(&inner)?;
                Value::List(at, inner.len() as u32)
            }
            x if x.is_ascii_alphabetic() || x == b'_' => {
                let w = self.word()?;
                let kw = self.keyword(w)?;
                self.expect(b'(')?;
                let mut inner = Vec::new();
                self.values_until_close(&mut inner, depth + 1)?;
                // a type name wraps one value; several are kept as a list
                let v = match inner.len() {
                    1 => inner[0],
                    _ => {
                        let at = self.push_run(&inner)?;
                        Value::List(at, inner.len() as u32)
                    }
                };
                let at = self.push_run(&[v])?;
                Value::Typed(kw, at)
            }
            x => return self.err(format!("unexpected '{}' in a value", x as char)),
        };
        if out.len() >= self.lim.max_values {
            return self.err(format!("over the limit of {} values", self.lim.max_values));
        }
        out.push(v);
        Ok(())
    }

    fn string(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        self.i += 1;
        let a = self.i;
        loop {
            match self.peek() {
                None => {
                    self.i = start;
                    return self.err("a string is never closed");
                }
                Some(b'\'') if self.b.get(self.i + 1) == Some(&b'\'') => self.i += 2,
                Some(b'\'') => break,
                Some(_) => self.i += 1,
            }
        }
        let n = self.i - a;
        self.i += 1;
        Ok(Value::Str(a as u32, n as u32))
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let a = self.i;
        let mut real = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' | b'+' | b'-' => self.i += 1,
                b'.' | b'e' | b'E' | b'd' | b'D' => {
                    real = true;
                    self.i += 1;
                }
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.b[a..self.i]).unwrap_or("");
        if !real {
            return match s.parse::<i64>() {
                Ok(n) => Ok(Value::Int(n)),
                Err(_) => match s.parse::<f64>() {
                    Ok(x) if x.is_finite() => Ok(Value::Real(x)),
                    _ => self.err(format!("a bad number '{s}'")),
                },
            };
        }
        // Rust's parser reads "1.", ".5", "1.E-5"; some writers put D for E
        let owned;
        let t = if s.contains(['d', 'D']) {
            owned = s.replace(['d', 'D'], "E");
            owned.as_str()
        } else {
            s
        };
        match t.parse::<f64>() {
            Ok(x) if x.is_finite() => Ok(Value::Real(x)),
            _ => self.err(format!("a bad number '{s}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &[u8] = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('x'),'2;1');\n\
        FILE_NAME('a','b',(''),(''),'pre','SYS ''quoted''','');\n\
        FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\nENDSEC;\nDATA;\n\
        /* a comment */\n#1=CARTESIAN_POINT('',(1.,-2.5E-1,3));\n#2=DIRECTION('a;b',(0.,0.,1.));\n\
        #3=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
        #4=X(#1,$,*,.T.,LENGTH_MEASURE(1.E-3),((1,2),(3,4)));\nENDSEC;\nEND-ISO-10303-21;\n";

    /// One pass over a file, as `parse` reads one too small for pieces.
    fn one_pass(text: &[u8]) -> Result<File<'_>, ParseError> {
        let mut p = P {
            b: text,
            i: 0,
            f: File {
                text,
                values: Vec::new(),
                names: Vec::new(),
                ids: Vec::new(),
                records: Vec::new(),
                duplicates: Vec::new(),
                schema: String::new(),
                system: String::new(),
            },
            lim: Limits::default(),
            kw: BTreeMap::new(),
        };
        p.file().map(|()| p.f)
    }

    /// A big file: every `fake`th instance a string holding what looks like an instance start.
    fn big(fake: usize) -> Vec<u8> {
        let mut text =
            b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));\nENDSEC;\nDATA;\n".to_vec();
        let mut k = 0;
        while text.len() < PIECES_FROM + (1 << 20) {
            k += 1;
            let line = match k % 4 {
                _ if k % fake == 0 => format!("#{k}=LABEL('a;\n#{k}=FAKE(1);\n');\n"),
                0 => format!("#{k}=CARTESIAN_POINT('p{k}',({k}.5,-2.E-1,3));\n"),
                1 => format!(
                    "#{k}=(A_{}()B(#{},$,*,.T.)C(LENGTH_MEASURE({k}.)));\n",
                    k % 7,
                    k - 1
                ),
                2 => format!("#{k}=DIRECTION('',(0.,0.,1.));\n"),
                _ => format!(
                    "#{k}=KW_{}(#{},((1,2),(3,4)),T_{}(1.,2.));\n",
                    k % 101,
                    k - 2,
                    k % 13
                ),
            };
            text.extend_from_slice(line.as_bytes());
        }
        text.extend_from_slice(b"ENDSEC;\nEND-ISO-10303-21;\n");
        text
    }

    /// `parse` gives one pass's file, its ids sorted.
    fn parse_is_one_pass(text: &[u8]) {
        let mut whole = one_pass(text).unwrap();
        let mut ids = std::mem::take(&mut whole.ids);
        ids.sort_by_key(|x| x.0);
        whole.ids = ids;
        let got = parse(text, &Limits::default()).unwrap();
        assert_eq!(format!("{got:?}"), format!("{whole:?}"));
    }

    /// A big file read in pieces is what one pass reads, keyword numbers and value places too;
    /// where a guessed piece start is inside a string, one pass reads the file.
    #[test]
    fn pieces_read_as_one_pass() {
        let text = big(1 << 30);
        if forge_math::par::threads() >= 2 {
            let pieces = parse_pieces(&text, &Limits::default()).expect("read in pieces");
            assert_eq!(format!("{pieces:?}"), format!("{:?}", one_pass(&text).unwrap()));
        }
        parse_is_one_pass(&text);
        // a fake start on every other line: a guess lands in a string, and one pass reads it
        let text = big(2);
        if forge_math::par::threads() >= 2 {
            assert!(parse_pieces(&text, &Limits::default()).is_none());
        }
        parse_is_one_pass(&text);
    }

    #[test]
    fn reads_a_small_file() {
        let f = parse(SMALL, &Limits::default()).unwrap();
        assert_eq!(f.schema, "AUTOMOTIVE_DESIGN");
        assert_eq!(f.system, "SYS ''quoted''");
        assert_eq!(f.ids.len(), 4);
        let r = f.records_of(1).unwrap();
        assert_eq!(f.name(&r[0]), "CARTESIAN_POINT");
        let Value::List(a, n) = f.args(&r[0])[1] else {
            panic!()
        };
        assert_eq!(
            f.run(a, n),
            &[Value::Real(1.0), Value::Real(-0.25), Value::Int(3)]
        );
        assert_eq!(f.records_of(3).unwrap().len(), 3);
        let x = f.records_of(4).unwrap()[0];
        let v = f.args(&x);
        assert_eq!(v[0], Value::Ref(1));
        assert_eq!((v[1], v[2]), (Value::Null, Value::Derived));
        assert!(matches!(v[3], Value::Enum(..)));
        let Value::Typed(_, at) = v[4] else { panic!() };
        assert_eq!(f.values[at as usize], Value::Real(1e-3));
    }

    #[test]
    fn every_prefix_and_flip_ends_in_a_value_or_an_error() {
        for n in 0..SMALL.len() {
            let _ = parse(&SMALL[..n], &Limits::default());
        }
        let mut b = SMALL.to_vec();
        for i in 0..b.len() {
            let keep = b[i];
            for x in [0u8, b'(', b')', b'\'', b'#', b';', 0xff] {
                b[i] = x;
                let _ = parse(&b, &Limits::default());
            }
            b[i] = keep;
        }
    }

    #[test]
    fn deep_lists_and_big_lists_are_refused() {
        let mut deep = b"ISO-10303-21;DATA;#1=X(".to_vec();
        deep.extend(std::iter::repeat_n(b'(', 100_000));
        assert!(parse(&deep, &Limits::default()).unwrap_err().why.contains("deep"));
        let mut big = b"ISO-10303-21;DATA;#1=X((".to_vec();
        big.extend(std::iter::repeat_n(&b"1,"[..], 2000).flatten());
        big.extend(b"1));ENDSEC;");
        let lim = Limits {
            max_values: 1000,
            ..Limits::default()
        };
        assert!(parse(&big, &lim).unwrap_err().why.contains("limit"));
    }

    #[test]
    fn a_duplicate_keeps_the_first() {
        let t = b"ISO-10303-21;DATA;#1=A();#1=B();ENDSEC;";
        let f = parse(t, &Limits::default()).unwrap();
        assert_eq!(f.duplicates, vec![1]);
        assert_eq!(f.name(&f.records_of(1).unwrap()[0]), "A");
    }
}
