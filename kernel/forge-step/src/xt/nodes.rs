//! A transmit file read into its nodes (XT Format Reference ch. 2): the header, the schema the
//! file declares as edits of the base, then node after node to the terminator. Every value is
//! kept as an f64 (integers, pointers and characters exactly), a null as NaN.

use super::lex::{Form, L, Lex, LexError, unwrap_text};
use super::schema::{Field, Kind, Layout, base, visible};
use crate::part21::Limits;
use std::collections::HashMap;

/// Why a file does not read as a transmit file.
#[derive(Clone, Debug, PartialEq)]
pub enum XtError {
    /// Not a transmit file: no text `T` or binary `PS` flag after the header.
    NotXt,
    /// The bytes break off or do not make what the format puts there.
    Bytes { at: usize, want: &'static str, node: Option<(u16, u32)> },
    /// A node type the file neither describes nor the base schema has.
    UnknownType { ty: u16, at: usize },
    /// A layout the file describes that cannot be one.
    BadSchema { ty: u16, why: String },
    /// Typed binary from a machine with VAX doubles or EBCDIC text.
    Machine,
    /// More than `Limits` allows.
    TooBig,
}

impl std::fmt::Display for XtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XtError::NotXt => write!(f, "not a Parasolid transmit file"),
            XtError::Bytes { at, want, node } => {
                write!(f, "byte {at}: wanted {want}")?;
                if let Some((t, i)) = node {
                    write!(f, " in node {i} of type {t}")?;
                }
                Ok(())
            }
            XtError::UnknownType { ty, at } => {
                write!(f, "byte {at}: node type {ty} is neither described by the file nor in the base schema")
            }
            XtError::BadSchema { ty, why } => write!(f, "node type {ty}: {why}"),
            XtError::Machine => write!(f, "binary from a machine with VAX doubles or EBCDIC text"),
            XtError::TooBig => write!(f, "more nodes or values than the reader's limits"),
        }
    }
}

/// What the file says of itself.
#[derive(Clone, Debug, Default)]
pub struct Header {
    /// Keyword values of the human-readable header (APPL, FRU, SCH, ...).
    pub keys: Vec<(String, String)>,
    /// `TRANSMIT FILE created by modeller version 3000298`.
    pub version: String,
    /// `SCH_3000298_30000`, with `_13006` when it carries its schema as edits of that base.
    pub schema: String,
    pub binary: bool,
}

impl Header {
    pub fn key(&self, k: &str) -> Option<&str> {
        self.keys.iter().find(|x| x.0 == k).map(|x| x.1.as_str())
    }
}

#[derive(Clone, Copy, Debug)]
struct Rec {
    ty: u16,
    lay: u16,
    start: u32,
    var: u32,
}

pub(crate) struct Store {
    pub header: Header,
    layouts: Vec<Layout>,
    recs: Vec<Rec>,
    /// Node index to position in `recs`, plus one; zero for none.
    by_index: Vec<u32>,
    vals: Vec<f64>,
    /// Node types the file uses, with how many of each.
    pub census: Vec<(u16, usize)>,
}

#[derive(Clone, Copy)]
pub(crate) struct Node<'s> {
    s: &'s Store,
    r: Rec,
    pub index: u32,
}

impl<'s> Node<'s> {
    pub(crate) fn ty(&self) -> u16 {
        self.r.ty
    }

    fn layout(&self) -> &'s Layout {
        &self.s.layouts[self.r.lay as usize]
    }

    fn slot(&self, name: &str) -> Option<(usize, &'s Field, usize)> {
        let l = self.layout();
        let i = l.find(name)?;
        let f = &l.fields[i];
        let n = if f.n == 1 { self.r.var as usize } else { f.n.max(1) as usize };
        Some((self.r.start as usize + l.offset[i], f, n * f.kind.width()))
    }

    /// The field's values: one for a scalar, all of an array or the variable field.
    pub(crate) fn all(&self, name: &str) -> &'s [f64] {
        match self.slot(name) {
            Some((at, _, n)) => self.s.vals.get(at..at + n).unwrap_or(&[]),
            None => &[],
        }
    }

    pub(crate) fn has(&self, name: &str) -> bool {
        self.layout().find(name).is_some()
    }

    /// A scalar: NaN when null or not in this file's layout.
    pub(crate) fn f(&self, name: &str) -> f64 {
        self.all(name).first().copied().unwrap_or(f64::NAN)
    }

    /// A pointer: 0 when null.
    pub(crate) fn p(&self, name: &str) -> u32 {
        let x = self.f(name);
        if x.is_finite() && x >= 0.0 && x <= f64::from(u32::MAX) { x as u32 } else { 0 }
    }

    pub(crate) fn int(&self, name: &str) -> Option<i64> {
        let x = self.f(name);
        x.is_finite().then_some(x as i64)
    }

    pub(crate) fn c(&self, name: &str) -> u8 {
        let x = self.f(name);
        if x.is_finite() { x as u8 } else { b'?' }
    }

    /// A vector field: None when null.
    pub(crate) fn v(&self, name: &str) -> Option<[f64; 3]> {
        match self.all(name) {
            [a, b, c, ..] if a.is_finite() && b.is_finite() && c.is_finite() => Some([*a, *b, *c]),
            _ => None,
        }
    }

    /// Length of the variable field.
    pub(crate) fn var_len(&self) -> usize {
        self.r.var as usize
    }
}

impl Store {
    pub(crate) fn node(&self, index: u32) -> Option<Node<'_>> {
        let k = *self.by_index.get(index as usize)?;
        let r = *self.recs.get(k.checked_sub(1)? as usize)?;
        Some(Node { s: self, r, index })
    }

    /// The node at `index` if it is of type `ty`.
    pub(crate) fn of(&self, index: u32, ty: u16) -> Option<Node<'_>> {
        self.node(index).filter(|n| n.ty() == ty)
    }

    /// Every node of type `ty`, in file order.
    pub(crate) fn all_of(&self, ty: u16) -> impl Iterator<Item = Node<'_>> + '_ {
        self.by_index
            .iter()
            .enumerate()
            .filter(move |(_, k)| **k != 0 && self.recs[**k as usize - 1].ty == ty)
            .map(move |(i, k)| Node {
                s: self,
                r: self.recs[*k as usize - 1],
                index: i as u32,
            })
    }

    pub(crate) fn root(&self) -> Option<Node<'_>> {
        self.node(1)
    }
}

/// The human-readable header's keywords (3.1): `NAME=value;`, `^` escapes undone.
fn header_keys(h: &[u8]) -> Vec<(String, String)> {
    let text: String = h
        .split(|&c| c == b'\n')
        .map(|l| String::from_utf8_lossy(l).trim_end_matches(['\r', ' ']).to_string())
        .collect();
    let mut out = Vec::new();
    for part in text.split(';') {
        let part = part.rsplit("**").next().unwrap_or(part);
        let part = part.strip_prefix("PART1").or(part.strip_prefix("PART2")).unwrap_or(part);
        let part = part.trim_start_matches(';');
        if let Some((k, v)) = part.split_once('=')
            && !k.is_empty()
            && k.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
        {
            let v = v.replace("^-", " ").replace("^n", "\n").replace("^;", ";").replace("^^", "^");
            out.push((k.to_string(), v));
        }
    }
    out
}

/// Where the data starts, the human header's text, and the form of what follows.
fn split_header(b: &[u8]) -> Result<(usize, &[u8], Form, bool), XtError> {
    let mut at = 0;
    let mut head: &[u8] = &[];
    if b.starts_with(b"**") {
        let marker = b"**END_OF_HEADER";
        let k = b.windows(marker.len()).position(|w| w == marker).ok_or(XtError::NotXt)?;
        head = &b[..k];
        at = k + b[k..].iter().position(|&c| c == b'\n').ok_or(XtError::NotXt)? + 1;
    }
    let rest = &b[at..];
    // a text file's data may start on a later line
    let skip = rest.iter().position(|c| !matches!(c, b'\r' | b'\n')).unwrap_or(rest.len());
    let rest = &rest[skip..];
    at += skip;
    match rest {
        [b'T', ..] => Ok((at + 1, head, Form::Text, false)),
        [b'P', b'S', 0, 0, ..] => Ok((at + 4, head, Form::Binary { little: false }, true)),
        [b'P', b'S', 0, 1, order, double, chars, ..] => {
            if *double != 0 || *chars != 0 {
                return Err(XtError::Machine);
            }
            Ok((at + 7, head, Form::Binary { little: *order == 1 }, true))
        }
        // bare binary: the writer's own byte order, today's machines little-endian
        [b'B', ..] => Ok((at + 1, head, Form::Binary { little: true }, true)),
        _ => Err(XtError::NotXt),
    }
}

/// Whether the bytes are a transmit file, by their header or flag.
pub fn is_xt(b: &[u8]) -> bool {
    let head = &b[..b.len().min(4096)];
    if head.starts_with(b"**ABCDEFGHIJKLMNOPQRSTUVWXYZ") {
        return head.windows(11).any(|w| w == b"**PARASOLID");
    }
    if let Ok((at, _, form, _)) = split_header(head) {
        let rest = &head[at..];
        return match form {
            Form::Text => rest.windows(13).take(16).any(|w| w == b"TRANSMIT FILE"),
            Form::Binary { .. } => rest.windows(13).take(16).any(|w| w == b"TRANSMIT FILE"),
        };
    }
    false
}

struct Reader<'a> {
    lex: Lex<'a>,
    embedded: bool,
    /// The layout of each node type met, by type.
    declared: HashMap<u16, u16>,
    layouts: Vec<Layout>,
}

impl Reader<'_> {
    fn field_desc(&mut self) -> L<(Field, bool)> {
        let name = self.lex.short_string("a field name")?;
        let ptr = self.lex.short("a field's pointer class")?.unwrap_or(0);
        let n = self.lex.index("a field's element count")?.unwrap_or(0);
        let kind = if ptr != 0 {
            Some(Kind::Ptr)
        } else {
            let t = self.lex.short_string("a field type")?;
            Kind::of(&t)
        };
        let xmt = if n == 1 { self.lex.logical("a field's transmit flag")? } else { true };
        let Some(kind) = kind else {
            return Err(LexError {
                at: self.lex.at,
                want: "a known field type",
            });
        };
        Ok((
            Field {
                name: String::from_utf8_lossy(&name).into_owned(),
                kind,
                n: u32::try_from(n).unwrap_or(u32::MAX),
            },
            xmt,
        ))
    }

    /// The first node of a type: its layout, from the file's edits of the base or its own words.
    fn declare(&mut self, ty: u16) -> Result<u16, XtError> {
        let at = self.lex.at;
        let lexe = |e: LexError| XtError::Bytes {
            at: e.at,
            want: e.want,
            node: None,
        };
        let based = base(ty);
        let mut skipped = false;
        let fields = match (self.embedded, based) {
            (false, Some(b)) => b,
            (false, None) => return Err(XtError::UnknownType { ty, at }),
            (true, None) => {
                let n = self.lex.byte("a new node type's field count").map_err(lexe)?.unwrap_or(0);
                if n == 255 {
                    // a type the file takes as unchanged from a base this reader lacks
                    return Err(XtError::UnknownType { ty, at });
                }
                self.lex.short_string("a node type's name").map_err(lexe)?;
                self.lex.short_string("a node type's description").map_err(lexe)?;
                let mut f = Vec::new();
                for _ in 0..n {
                    let (fd, xmt) = self.field_desc().map_err(lexe)?;
                    skipped |= !xmt;
                    f.push(fd);
                }
                f
            }
            (true, Some(b)) => {
                let n = self.lex.byte("a node type's field count").map_err(lexe)?.unwrap_or(0);
                if n == 255 {
                    b
                } else {
                    let mut cur = Vec::new();
                    let mut bi = 0;
                    loop {
                        match self.lex.char("an edit of the base schema").map_err(lexe)? {
                            b'C' => {
                                let f = b.get(bi).ok_or_else(|| XtError::BadSchema {
                                    ty,
                                    why: "copies more fields than the base has".into(),
                                })?;
                                cur.push(f.clone());
                                bi += 1;
                            }
                            b'D' => bi += 1,
                            b'I' | b'A' => {
                                let (fd, xmt) = self.field_desc().map_err(lexe)?;
                                skipped |= !xmt;
                                cur.push(fd);
                            }
                            b'Z' => break,
                            _ => {
                                return Err(XtError::BadSchema {
                                    ty,
                                    why: "an edit that is not C, D, I, A or Z".into(),
                                });
                            }
                        }
                    }
                    if bi != b.len() || cur.len() != n as usize {
                        return Err(XtError::BadSchema {
                            ty,
                            why: format!(
                                "edits use {bi} of the base's {} fields and make {} of {n}",
                                b.len(),
                                cur.len()
                            ),
                        });
                    }
                    cur
                }
            }
        };
        if skipped {
            return Err(XtError::BadSchema {
                ty,
                why: "a variable field that is not transmitted".into(),
            });
        }
        let lay = Layout::new(fields).ok_or_else(|| XtError::BadSchema {
            ty,
            why: "a variable field before the last".into(),
        })?;
        self.layouts.push(lay);
        let id = u16::try_from(self.layouts.len() - 1).map_err(|_| XtError::TooBig)?;
        self.declared.insert(ty, id);
        Ok(id)
    }

    fn value(&mut self, k: Kind, out: &mut Vec<f64>) -> L<()> {
        let f = |x: Option<i64>| x.map_or(f64::NAN, |v| v as f64);
        match k {
            Kind::Byte => {
                let x = self.lex.byte("a byte")?;
                out.push(f(x));
            }
            Kind::Char => {
                let c = self.lex.char("a character")?;
                out.push(f64::from(c));
            }
            Kind::Logical => {
                let x = self.lex.logical("a logical")?;
                out.push(if x { 1.0 } else { 0.0 });
            }
            Kind::Short => {
                let x = self.lex.short("a short")?;
                out.push(f(x));
            }
            Kind::Int => {
                let x = self.lex.int("an int")?;
                out.push(f(x));
            }
            Kind::Ptr => {
                let x = self.lex.index("a pointer")?;
                out.push(f(x));
            }
            Kind::Real => {
                let x = self.lex.real("a real")?;
                out.push(x);
            }
            Kind::Interval => self.lex.reals(2, out, "an interval")?,
            Kind::Vector => self.lex.reals(3, out, "a vector")?,
            Kind::Box => self.lex.reals(6, out, "a box")?,
        }
        Ok(())
    }
}

/// Reads a whole transmit file into its nodes.
pub(crate) fn read(b: &[u8], lim: &Limits) -> Result<Store, XtError> {
    if b.len() > lim.max_bytes {
        return Err(XtError::TooBig);
    }
    let (at, head, form, binary) = split_header(b)?;
    let text;
    let data = match form {
        Form::Text => {
            text = unwrap_text(&b[at..]);
            &text[..]
        }
        Form::Binary { .. } => &b[at..],
    };
    let lexe = |e: LexError| XtError::Bytes {
        at: e.at,
        want: e.want,
        node: None,
    };
    let mut r = Reader {
        lex: Lex::new(data, form),
        embedded: false,
        declared: HashMap::new(),
        layouts: Vec::new(),
    };
    let version = r.lex.header_string(false, "the modeller version").map_err(lexe)?;
    let schema = r.lex.header_string(true, "the schema name").map_err(lexe)?;
    let schema = String::from_utf8_lossy(&schema).into_owned();
    r.embedded = schema.split('_').count() >= 4;
    if r.embedded {
        r.lex.short("the most node types").map_err(lexe)?;
    }
    let user = r.lex.int("the user field size").map_err(lexe)?.unwrap_or(0);
    let user = usize::try_from(user).ok().filter(|&u| u <= 64).ok_or(XtError::Bytes {
        at: r.lex.at,
        want: "a user field size of 0 to 64",
        node: None,
    })?;
    let mut recs: Vec<Rec> = Vec::new();
    let mut by_index: Vec<u32> = Vec::new();
    let mut vals: Vec<f64> = Vec::new();
    let mut census: HashMap<u16, usize> = HashMap::new();
    loop {
        let at = r.lex.at;
        let ty = r.lex.short("a node type").map_err(lexe)?;
        let Some(ty) = ty.and_then(|t| u16::try_from(t).ok()) else {
            return Err(XtError::Bytes {
                at,
                want: "a node type",
                node: None,
            });
        };
        if ty == 1 {
            // the terminator, then an index of 0
            r.lex.index("the terminator's index").map_err(lexe)?;
            break;
        }
        let lay = match r.declared.get(&ty) {
            Some(&l) => l,
            None => r.declare(ty)?,
        };
        let layout = &r.layouts[lay as usize];
        let var = if layout.variable {
            r.lex.int("a variable length").map_err(lexe)?.unwrap_or(0)
        } else {
            0
        };
        let at = r.lex.at;
        let index = r.lex.index("a node index").map_err(lexe)?.unwrap_or(-1);
        let (Ok(var), Ok(index)) = (u32::try_from(var), u32::try_from(index)) else {
            return Err(XtError::Bytes {
                at,
                want: "a node index and length",
                node: Some((ty, 0)),
            });
        };
        if recs.len() >= lim.max_instances || index as usize >= lim.max_instances.max(1 << 20) * 4 {
            return Err(XtError::TooBig);
        }
        let nerr = |e: LexError| XtError::Bytes {
            at: e.at,
            want: e.want,
            node: Some((ty, index)),
        };
        let start = vals.len();
        let fields = layout.fields.clone();
        for f in &fields {
            let n = if f.n == 1 { var as usize } else { f.n.max(1) as usize };
            if vals.len() + n * f.kind.width() > lim.max_values {
                return Err(XtError::TooBig);
            }
            if f.kind == Kind::Char && f.n == 1 {
                let s = r.lex.chars(n, "characters").map_err(nerr)?;
                vals.extend(s.iter().map(|&c| f64::from(c)));
            } else {
                for _ in 0..n {
                    r.value(f.kind, &mut vals).map_err(nerr)?;
                }
            }
        }
        if user > 0 && visible(ty) {
            for _ in 0..user {
                r.lex.int("a user field").map_err(nerr)?;
            }
        }
        let start = u32::try_from(start).map_err(|_| XtError::TooBig)?;
        recs.push(Rec { ty, lay, start, var });
        let i = index as usize;
        if by_index.len() <= i {
            by_index.resize(i + 1, 0);
        }
        by_index[i] = recs.len() as u32;
        *census.entry(ty).or_default() += 1;
    }
    let mut census: Vec<(u16, usize)> = census.into_iter().collect();
    census.sort_unstable();
    Ok(Store {
        header: Header {
            keys: header_keys(head),
            version: String::from_utf8_lossy(&version).trim().to_string(),
            schema,
            binary,
        },
        layouts: r.layouts,
        recs,
        by_index,
        vals,
        census,
    })
}
