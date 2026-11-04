//! The standard parts library (a content centre): dimension tables (`stdparts/*.csv`, format in
//! `stdparts/README.md`) and one generator part per family (`stdparts/gen/*.fg`). A standard part
//! is a part file made from a generator with one table row's values: `std("ISO 4762", size: "M6",
//! length: 20)` in an assembly, `fg std` on the command line. Its file key, `std:ISO 4762/M6/20`,
//! stands for the generated text wherever a part file is read (`asm::locate`), so it needs no
//! file on disk and two uses of one size are one part in the BOM.

use crate::error::{Error, Op};
use crate::syntax::Span;
use forge_agent::json::Json;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Every family's table, by file stem.
pub const TABLES: &[(&str, &str)] = &[("iso4762", include_str!("../stdparts/iso4762.csv"))];

/// Every generator, by file stem.
pub const GENERATORS: &[(&str, &str)] = &[("shcs", include_str!("../stdparts/gen/shcs.fg"))];

/// The prefix of a standard part's file key.
pub const KEY: &str = "std:";

#[derive(Debug)]
pub struct Family {
    /// The file stem: `iso4762`.
    pub id: &'static str,
    /// The standard's designation without its edition: `ISO 4762`.
    pub name: String,
    pub title: String,
    /// With its edition: `ISO 4762:2004`.
    pub standard: String,
    pub kind: String,
    /// Inches (`in`) or millimetres.
    pub inch: bool,
    pub head: Vec<(String, String)>,
    pub cols: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The standard's series of nominal lengths, if the family has lengths.
    pub lengths: Vec<f64>,
    /// `(name, allowed values)`; the first value is the default.
    pub options: Vec<(String, Vec<String>)>,
}

impl Family {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.head.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    pub fn all(&self, key: &str) -> impl Iterator<Item = &str> {
        self.head
            .iter()
            .filter(move |(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    pub fn col(&self, name: &str) -> Option<usize> {
        self.cols.iter().position(|c| c == name)
    }
    pub fn sizes(&self) -> Vec<&str> {
        self.rows.iter().map(|r| r[0].as_str()).collect()
    }
    pub fn row(&self, size: &str) -> Option<&[String]> {
        self.rows.iter().find(|r| r[0] == size).map(|r| r.as_slice())
    }
    pub fn value(&self, row: &[String], col: &str) -> Option<f64> {
        self.col(col).and_then(|i| row[i].parse().ok())
    }
    /// The lengths a size comes in: the series within its commercial range.
    pub fn lengths_of(&self, row: &[String]) -> Vec<f64> {
        let lo = self.value(row, "l_min").unwrap_or(f64::NEG_INFINITY);
        let hi = self.value(row, "l_max").unwrap_or(f64::INFINITY);
        self.lengths
            .iter()
            .copied()
            .filter(|l| *l >= lo && *l <= hi)
            .collect()
    }
    pub fn has_lengths(&self) -> bool {
        !self.lengths.is_empty()
    }
}

/// A table's text read into a family; Err names the first fault.
pub fn parse(id: &'static str, text: &str) -> Result<Family, String> {
    let mut head = vec![];
    let mut body = vec![];
    for (n, line) in text.lines().enumerate() {
        if let Some(h) = line.strip_prefix('#') {
            let (k, v) = h
                .split_once(':')
                .ok_or(format!("line {}: header without `key:`", n + 1))?;
            head.push((k.trim().to_string(), v.trim().to_string()));
        } else if !line.trim().is_empty() {
            body.push((n + 1, line));
        }
    }
    let (_, first) = body.first().ok_or("no column line")?;
    let cols: Vec<String> = cells(first);
    if cols.first().map(String::as_str) != Some("size") {
        return Err("the first column is not `size`".into());
    }
    let mut rows = vec![];
    for (n, line) in &body[1..] {
        let r = cells(line);
        if r.len() != cols.len() {
            return Err(format!("line {n}: {} cells, {} columns", r.len(), cols.len()));
        }
        if rows.iter().any(|x: &Vec<String>| x[0] == r[0]) {
            return Err(format!("line {n}: size {} twice", r[0]));
        }
        for (c, v) in cols.iter().zip(&r).skip(1) {
            if !v.is_empty() && !matches!(c.as_str(), "src" | "checked" | "note") && v.parse::<f64>().is_err()
            {
                return Err(format!("line {n}: {c} = {v:?} is not a number"));
            }
        }
        rows.push(r);
    }
    let get = |k: &str| {
        head.iter()
            .find(|(x, _)| x == k)
            .map(|(_, v): &(String, String)| v.clone())
    };
    let req = |k: &str| get(k).ok_or(format!("no `{k}:` header"));
    let standard = req("standard")?;
    let name = standard.split(':').next().unwrap_or(&standard).trim().to_string();
    let units = req("units")?;
    if units != "mm" && units != "in" {
        return Err(format!("units {units:?}: mm or in"));
    }
    if !head.iter().any(|(k, _)| k.starts_with("src ")) {
        return Err("no `src` header".into());
    }
    let lengths = match get("lengths") {
        None => vec![],
        Some(s) => s
            .split_whitespace()
            .map(|x| x.parse::<f64>().map_err(|_| format!("lengths: {x:?}")))
            .collect::<Result<_, _>>()?,
    };
    let options = head
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("option ").map(|o| {
                (
                    o.trim().to_string(),
                    v.split_whitespace().map(String::from).collect(),
                )
            })
        })
        .collect();
    Ok(Family {
        id,
        name,
        title: req("title")?,
        standard,
        kind: req("kind")?,
        inch: units == "in",
        head,
        cols,
        rows,
        lengths,
        options,
    })
}

fn cells(line: &str) -> Vec<String> {
    line.split(',').map(|c| c.trim().to_string()).collect()
}

pub fn families() -> &'static [Family] {
    static ALL: OnceLock<Vec<Family>> = OnceLock::new();
    ALL.get_or_init(|| {
        TABLES
            .iter()
            .map(|(id, t)| parse(id, t).unwrap_or_else(|e| panic!("stdparts/{id}.csv: {e}")))
            .collect()
    })
}

fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '/' || *c == '#')
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn family(name: &str) -> Option<&'static Family> {
    let q = squash(name);
    families()
        .iter()
        .find(|f| squash(&f.name) == q || f.id == q || squash(&f.standard) == q)
}

/// A standard part's choice: family, size, length and options.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub family: String,
    pub size: String,
    pub length: Option<f64>,
    pub options: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StdErr {
    /// No family by that name: the families there are, and the nearest name.
    UnknownFamily {
        span: Span,
        name: String,
        near: Option<String>,
        valid: Vec<String>,
    },
    /// No such size in the family: its sizes, and the one meant if it is clear.
    UnknownSize {
        span: Span,
        family: String,
        size: String,
        near: Option<String>,
        valid: Vec<String>,
    },
    /// The family has lengths and none was given.
    NeedsLength {
        span: Span,
        family: String,
        size: String,
        valid: Vec<f64>,
    },
    /// A length given to a family without lengths.
    NoLength { span: Span, family: String },
    /// A length the standard does not list for this size: its lengths, and the nearest.
    UnknownLength {
        span: Span,
        family: String,
        size: String,
        length: f64,
        nearest: f64,
        valid: Vec<f64>,
    },
    /// An option the family does not have, or a value it does not allow.
    UnknownOption {
        span: Span,
        family: String,
        option: String,
        value: String,
        allowed: Vec<String>,
    },
}

/// The instance an error's fixes edit, and the argument each edits.
#[derive(Clone, Debug, PartialEq)]
pub struct Located {
    pub err: StdErr,
    pub id: Option<String>,
}

fn nums(xs: &[f64]) -> String {
    xs.iter()
        .map(|x| crate::values::fmt(*x))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Located {
    pub fn span(&self) -> Span {
        use StdErr::*;
        match &self.err {
            UnknownFamily { span, .. }
            | UnknownSize { span, .. }
            | NeedsLength { span, .. }
            | NoLength { span, .. }
            | UnknownLength { span, .. }
            | UnknownOption { span, .. } => *span,
        }
    }
    pub fn kind(&self) -> &'static str {
        use StdErr::*;
        match &self.err {
            UnknownFamily { .. } => "StdUnknownFamily",
            UnknownSize { .. } => "StdUnknownSize",
            NeedsLength { .. } => "StdNeedsLength",
            NoLength { .. } => "StdNoLength",
            UnknownLength { .. } => "StdUnknownLength",
            UnknownOption { .. } => "StdUnknownOption",
        }
    }
    pub fn text(&self) -> String {
        use StdErr::*;
        match &self.err {
            UnknownFamily {
                name, near, valid, ..
            } => format!(
                "no standard part family {name:?}{}; there are {}",
                near.as_ref()
                    .map_or(String::new(), |n| format!(" (did you mean {n:?}?)")),
                valid.join(", ")
            ),
            UnknownSize {
                family,
                size,
                near,
                valid,
                ..
            } => format!(
                "{family} has no size {size:?}{}; its sizes: {}",
                near.as_ref()
                    .map_or(String::new(), |n| format!(" (did you mean {n:?}?)")),
                valid.join(", ")
            ),
            NeedsLength {
                family, size, valid, ..
            } => {
                format!("{family} {size} needs a length: {}", nums(valid))
            }
            NoLength { family, .. } => format!("{family} has no lengths"),
            UnknownLength {
                family,
                size,
                length,
                nearest,
                valid,
                ..
            } => format!(
                "{family} {size} does not come {} long (nearest {}); its lengths: {}",
                crate::values::fmt(*length),
                crate::values::fmt(*nearest),
                nums(valid)
            ),
            UnknownOption {
                family,
                option,
                value,
                allowed,
                ..
            } => {
                if allowed.is_empty() {
                    format!("{family} has no option {option}")
                } else {
                    format!("{family}: {option} {value:?} is none of {}", allowed.join(", "))
                }
            }
        }
    }
    pub fn data(&self) -> Vec<(&'static str, Json)> {
        use StdErr::*;
        let s = |x: &str| Json::Str(x.into());
        let strs = |xs: &[String]| Json::Arr(xs.iter().map(|x| s(x)).collect());
        let ns = |xs: &[f64]| Json::Arr(xs.iter().map(|x| Json::Num(*x)).collect());
        let opt = |x: &Option<String>| x.as_deref().map_or(Json::Null, s);
        match &self.err {
            UnknownFamily {
                name, near, valid, ..
            } => {
                vec![("name", s(name)), ("near", opt(near)), ("valid", strs(valid))]
            }
            UnknownSize {
                family,
                size,
                near,
                valid,
                ..
            } => vec![
                ("family", s(family)),
                ("size", s(size)),
                ("near", opt(near)),
                ("valid", strs(valid)),
            ],
            NeedsLength {
                family, size, valid, ..
            } => {
                vec![("family", s(family)), ("size", s(size)), ("valid", ns(valid))]
            }
            NoLength { family, .. } => vec![("family", s(family))],
            UnknownLength {
                family,
                size,
                length,
                nearest,
                valid,
                ..
            } => vec![
                ("family", s(family)),
                ("size", s(size)),
                ("length", Json::Num(*length)),
                ("nearest", Json::Num(*nearest)),
                ("valid", ns(valid)),
            ],
            UnknownOption {
                family,
                option,
                value,
                allowed,
                ..
            } => vec![
                ("family", s(family)),
                ("option", s(option)),
                ("value", s(value)),
                ("allowed", strs(allowed)),
            ],
        }
    }
    /// The argument set to the near match, where there is one.
    pub fn fixes(&self) -> Vec<Op> {
        use StdErr::*;
        let Some(id) = &self.id else { return vec![] };
        let set = |arg: &str, text: String| Op::SetArg {
            id: id.clone(),
            arg: arg.into(),
            text,
        };
        match &self.err {
            UnknownFamily { near: Some(n), .. } => vec![set("family", format!("{n:?}"))],
            UnknownSize { near: Some(n), .. } => vec![set("size", format!("{n:?}"))],
            UnknownLength { nearest, .. } => vec![set("length", crate::values::fmt(*nearest))],
            NeedsLength { valid, .. } if !valid.is_empty() => {
                vec![set("length", crate::values::fmt(valid[0]))]
            }
            _ => vec![],
        }
    }
}

impl From<Located> for Error {
    fn from(e: Located) -> Error {
        Error::Std(Box::new(e))
    }
}

/// The size meant by a misspelling, if one is clear: the same once spaces, case and a trailing
/// `.0` go, or the coarse size of a fine one written with its coarse pitch (`M6x1` is `M6`).
fn near_size(f: &Family, size: &str) -> Option<String> {
    let norm = |s: &str| {
        squash(s)
            .replace(".0x", "x")
            .trim_end_matches(".0")
            .replace('-', "x")
    };
    let q = norm(size);
    if let Some(s) = f.sizes().into_iter().find(|s| norm(s) == q) {
        return Some(s.to_string());
    }
    let (base, pitch) = q.split_once('x')?;
    let row = f.rows.iter().find(|r| norm(&r[0]) == base)?;
    (f.value(row, "P")? == pitch.parse::<f64>().ok()?).then(|| row[0].clone())
}

fn near_family(name: &str) -> Option<String> {
    let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    let hits: Vec<&Family> = families()
        .iter()
        .filter(|f| {
            !digits.is_empty() && f.name.chars().filter(|c| c.is_ascii_digit()).collect::<String>() == digits
        })
        .collect();
    match hits[..] {
        [one] => Some(one.name.clone()),
        _ => None,
    }
}

/// The spans each argument of a `std(..)` call was written at, for its errors.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spans {
    pub family: Span,
    pub size: Span,
    pub length: Span,
    pub options: Span,
}

/// A choice checked against the tables: its family and row.
pub fn check(spec: &Spec, at: Spans) -> Result<(&'static Family, &'static [String]), StdErr> {
    let f = family(&spec.family).ok_or_else(|| StdErr::UnknownFamily {
        span: at.family,
        name: spec.family.clone(),
        near: near_family(&spec.family),
        valid: families().iter().map(|f| f.name.clone()).collect(),
    })?;
    let row = f.row(&spec.size).ok_or_else(|| StdErr::UnknownSize {
        span: at.size,
        family: f.name.clone(),
        size: spec.size.clone(),
        near: near_size(f, &spec.size),
        valid: f.sizes().into_iter().map(String::from).collect(),
    })?;
    match (f.has_lengths(), spec.length) {
        (false, Some(_)) => {
            return Err(StdErr::NoLength {
                span: at.length,
                family: f.name.clone(),
            });
        }
        (true, None) => {
            return Err(StdErr::NeedsLength {
                span: at.size,
                family: f.name.clone(),
                size: spec.size.clone(),
                valid: f.lengths_of(row),
            });
        }
        (true, Some(l)) => {
            let valid = f.lengths_of(row);
            if !valid.iter().any(|v| (v - l).abs() < 1e-9) {
                let nearest = valid
                    .iter()
                    .copied()
                    .min_by(|a, b| (a - l).abs().total_cmp(&(b - l).abs()))
                    .unwrap_or(l);
                return Err(StdErr::UnknownLength {
                    span: at.length,
                    family: f.name.clone(),
                    size: spec.size.clone(),
                    length: l,
                    nearest,
                    valid,
                });
            }
        }
        (false, None) => {}
    }
    for (k, v) in &spec.options {
        let allowed = f.options.iter().find(|(o, _)| o == k).map(|(_, a)| a.clone());
        if !allowed
            .as_ref()
            .is_some_and(|a| a.contains(v) || a.iter().any(|x| x == "<length>"))
        {
            return Err(StdErr::UnknownOption {
                span: at.options,
                family: f.name.clone(),
                option: k.clone(),
                value: v.clone(),
                allowed: allowed.unwrap_or_default(),
            });
        }
    }
    Ok((f, row))
}

/// The major diameter and pitch a thread designation names, in the table's units: `M6`
/// (coarse, from the row's P), `M8x1`, `#10-24`, `1/4-20`, `1-1/4-7`.
fn thread_of(f: &Family, row: &[String]) -> Option<(f64, f64)> {
    let size = row[0].as_str();
    if let Some(m) = size.strip_prefix('M') {
        let (d, p) = match m.split_once('x') {
            Some((d, p)) => (d.parse().ok()?, p.parse().ok()?),
            None => (m.parse().ok()?, f.value(row, "P")?),
        };
        return Some((d, p));
    }
    let (d, tpi) = size.rsplit_once('-')?;
    let tpi: f64 = tpi.parse().ok()?;
    let d = if let Some(n) = d.strip_prefix('#') {
        0.060 + 0.013 * n.parse::<f64>().ok()?
    } else {
        let frac = |s: &str| -> Option<f64> {
            match s.split_once('/') {
                Some((a, b)) => Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok()?),
                None => s.parse().ok(),
            }
        };
        match d.split_once('-') {
            Some((w, fr)) => w.parse::<f64>().ok()? + frac(fr)?,
            None => frac(d)?,
        }
    };
    Some((d, 1.0 / tpi))
}

/// The plain number a size names (`6` of a washer for M6, `M6` itself), for `@d` of tables
/// that are not threads.
fn plain_d(size: &str) -> Option<f64> {
    size.trim_start_matches('M').parse().ok()
}

fn designation(f: &Family, spec: &Spec) -> String {
    match spec.length {
        Some(l) if f.inch => format!("{} {} x {}", f.name, spec.size, crate::values::fmt(l)),
        Some(l) => format!("{} {}x{}", f.name, spec.size, crate::values::fmt(l)),
        None => format!("{} {}", f.name, spec.size),
    }
}

/// The file key standing for this part: `std:ISO 4762/M6/20`, options other than the
/// defaults after it (`/thread=modelled`).
pub fn key(spec: &Spec) -> String {
    let f = family(&spec.family);
    let mut k = format!(
        "{KEY}{}/{}",
        f.map_or(spec.family.as_str(), |f| f.name.as_str()),
        spec.size
    );
    if let Some(l) = spec.length {
        k += &format!("/{}", crate::values::fmt(l));
    }
    for (o, v) in &spec.options {
        let default = f
            .and_then(|f| f.options.iter().find(|x| &x.0 == o))
            .and_then(|x| x.1.first());
        if default != Some(v) {
            k += &format!("/{o}={v}");
        }
    }
    k
}

/// A key read back into its choice (None: not a standard part key, or malformed).
pub fn spec_of(key: &str) -> Option<Spec> {
    let mut parts = key.strip_prefix(KEY)?.split('/');
    let family = parts.next()?.to_string();
    let size = parts.next()?.to_string();
    let mut length = None;
    let mut options = BTreeMap::new();
    for p in parts {
        match p.split_once('=') {
            Some((k, v)) => {
                options.insert(k.to_string(), v.to_string());
            }
            None => length = Some(p.parse().ok()?),
        }
    }
    Some(Spec {
        family,
        size,
        length,
        options,
    })
}

/// The part file text a key stands for, if it is a valid standard part.
pub fn text_of(key: &str) -> Option<String> {
    generate(&spec_of(key)?).ok().map(|g| g.text)
}

pub struct Generated {
    pub key: String,
    /// `ISO 4762 M6x20`
    pub designation: String,
    pub text: String,
}

fn generator(name: &str) -> Option<&'static str> {
    GENERATORS.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// The part file of a choice: the family's generator with every param its `gen:` header maps
/// written from the row, then its properties.
pub fn generate(spec: &Spec) -> Result<Generated, StdErr> {
    let (f, row) = check(spec, Spans::default())?;
    let opt = |o: &str| -> Option<String> {
        spec.options.get(o).cloned().or_else(|| {
            f.options
                .iter()
                .find(|x| x.0 == o)
                .and_then(|x| x.1.first().cloned())
        })
    };
    let header = f.get("gen").expect("a family the tables list has a generator");
    let mut parts = header.split('|').map(str::trim);
    let template = generator(parts.next().unwrap_or_default()).expect("a generator the header names");
    let unit = if f.inch { 25.4 } else { 1.0 };
    let thread = thread_of(f, row);
    let mut set: BTreeMap<&str, String> = BTreeMap::new();
    for p in parts {
        let (param, from) = p.split_once('=').expect("gen: param=source");
        let v = match from.trim() {
            "@d" => thread.map(|t| t.0).or_else(|| plain_d(&row[0])).map(|v| v * unit),
            "@P" => thread.map(|t| t.1 * unit),
            "@l" => spec.length.map(|l| l * unit),
            "@modelled" => Some(if opt("thread").as_deref() == Some("modelled") {
                1.0
            } else {
                0.0
            }),
            o if o.starts_with('@') => opt(&o[1..]).and_then(|v| v.parse::<f64>().ok()).map(|v| v * unit),
            c => f
                .value(row, c)
                .map(|v| v * unit)
                .or_else(|| c.parse::<f64>().ok()),
        };
        let v = v.unwrap_or_else(|| panic!("{}: {from} has no value for {}", f.id, row[0]));
        set.insert(param.trim(), crate::values::fmt(round(v)));
    }
    let name = designation(f, spec);
    let mut out = String::new();
    out += &format!("# {} {}\n", f.title, name.trim_start_matches(&f.name).trim());
    out += &format!(
        "# made by the standard parts library from stdparts/{}.csv ({}), row {}\n",
        f.id, f.standard, row[0]
    );
    for src in f.all("model") {
        out += &format!("# model: {src}\n");
    }
    for line in template.lines() {
        let rewritten = line.strip_prefix("shared ").and_then(|rest| {
            let (n, _) = rest.split_once('=')?;
            set.get(n.trim()).map(|v| format!("shared {} = {v}", n.trim()))
        });
        out += rewritten.as_deref().unwrap_or(line);
        out.push('\n');
    }
    let grade = opt("grade");
    let material = f.get("material").unwrap_or("Steel");
    let (material, density) = match grade.as_deref() {
        Some(g) if g.starts_with('A') => (format!("Stainless steel {g}"), 7.93),
        Some(g) => (format!("{material}, property class {g}"), 7.85),
        None => (
            material.to_string(),
            f.get("density").and_then(|d| d.parse().ok()).unwrap_or(7.85),
        ),
    };
    out += &format!(
        "\nabout = properties(part_number: {name:?}, description: {:?}, standard: {:?}, material: {material:?}, density: {})\n",
        format!("{} {}", f.title, name.trim_start_matches(&f.name).trim()),
        f.standard,
        crate::values::fmt(density)
    );
    Ok(Generated {
        key: key(spec),
        designation: name,
        text: out,
    })
}

/// Tables' values are keyed to a few decimals; inch ones times 25.4 to a millionth of a mm.
fn round(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}
