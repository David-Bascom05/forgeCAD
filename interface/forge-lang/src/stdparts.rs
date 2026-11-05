//! The standard parts library (a content centre): dimension tables (`stdparts/*.csv`, format in
//! `stdparts/README.md`) and one generator part per family (`stdparts/gen/*.fg`). A standard part
//! is a part file made from a generator with one table row's values: `std("ISO 4762", size: "M6",
//! length: 20)` in an assembly, `fg std` on the command line. Its file key, `std:ISO 4762|M6|20`,
//! stands for the generated text wherever a part file is read (`asm::locate`), so it needs no
//! file on disk and two uses of one size are one part in the BOM.

use crate::error::{Error, Op};
use crate::syntax::Span;
use forge_agent::json::Json;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Every family's table, by file stem.
pub const TABLES: &[(&str, &str)] = &[
    ("as568", include_str!("../stdparts/as568.csv")),
    ("asme_b18_22_1", include_str!("../stdparts/asme_b18_22_1.csv")),
    (
        "asme_b18_2_1_hex_cap",
        include_str!("../stdparts/asme_b18_2_1_hex_cap.csv"),
    ),
    (
        "asme_b18_2_2_hex_nut",
        include_str!("../stdparts/asme_b18_2_2_hex_nut.csv"),
    ),
    (
        "asme_b18_3_button",
        include_str!("../stdparts/asme_b18_3_button.csv"),
    ),
    ("asme_b18_3_flat", include_str!("../stdparts/asme_b18_3_flat.csv")),
    ("asme_b18_3_shcs", include_str!("../stdparts/asme_b18_3_shcs.csv")),
    ("asme_clearance", include_str!("../stdparts/asme_clearance.csv")),
    (
        "cbore_asme_b18_3",
        include_str!("../stdparts/cbore_asme_b18_3.csv"),
    ),
    ("cbore_iso4762", include_str!("../stdparts/cbore_iso4762.csv")),
    ("csink_iso10642", include_str!("../stdparts/csink_iso10642.csv")),
    ("din471", include_str!("../stdparts/din471.csv")),
    ("din472", include_str!("../stdparts/din472.csv")),
    ("din6885", include_str!("../stdparts/din6885.csv")),
    ("iso10511", include_str!("../stdparts/iso10511.csv")),
    ("iso10642", include_str!("../stdparts/iso10642.csv")),
    ("iso15_bearings", include_str!("../stdparts/iso15_bearings.csv")),
    ("iso2338", include_str!("../stdparts/iso2338.csv")),
    ("iso273", include_str!("../stdparts/iso273.csv")),
    ("iso4014", include_str!("../stdparts/iso4014.csv")),
    ("iso4017", include_str!("../stdparts/iso4017.csv")),
    ("iso4026", include_str!("../stdparts/iso4026.csv")),
    ("iso4027", include_str!("../stdparts/iso4027.csv")),
    ("iso4028", include_str!("../stdparts/iso4028.csv")),
    ("iso4029", include_str!("../stdparts/iso4029.csv")),
    ("iso4032", include_str!("../stdparts/iso4032.csv")),
    ("iso4035", include_str!("../stdparts/iso4035.csv")),
    ("iso4762", include_str!("../stdparts/iso4762.csv")),
    ("iso7040", include_str!("../stdparts/iso7040.csv")),
    ("iso7089", include_str!("../stdparts/iso7089.csv")),
    ("iso7090", include_str!("../stdparts/iso7090.csv")),
    ("iso7092", include_str!("../stdparts/iso7092.csv")),
    ("iso7093_1", include_str!("../stdparts/iso7093_1.csv")),
    ("iso7379", include_str!("../stdparts/iso7379.csv")),
    ("iso7380_1", include_str!("../stdparts/iso7380_1.csv")),
    ("iso7380_2", include_str!("../stdparts/iso7380_2.csv")),
    ("iso8734", include_str!("../stdparts/iso8734.csv")),
    ("pem_fh", include_str!("../stdparts/pem_fh.csv")),
    ("pem_fh_in", include_str!("../stdparts/pem_fh_in.csv")),
    ("pem_s", include_str!("../stdparts/pem_s.csv")),
    ("pem_s_in", include_str!("../stdparts/pem_s_in.csv")),
    ("pem_so", include_str!("../stdparts/pem_so.csv")),
    ("pem_so_in", include_str!("../stdparts/pem_so_in.csv")),
    (
        "tap_drill_metric",
        include_str!("../stdparts/tap_drill_metric.csv"),
    ),
    (
        "tap_drill_unified",
        include_str!("../stdparts/tap_drill_unified.csv"),
    ),
];

/// Tables kept out until a disagreement is decided, and why.
pub const HELD: &[(&str, &str)] = &[(
    "iso3601",
    "ISO 3601-1:2012 not read: whether the AS568A metric sizes carry its class A tolerances is open",
)];

/// Every generator, by file stem.
pub const GENERATORS: &[(&str, &str)] = &[
    ("bearing", include_str!("../stdparts/gen/bearing.fg")),
    ("button", include_str!("../stdparts/gen/button.fg")),
    ("csk", include_str!("../stdparts/gen/csk.fg")),
    ("hexhead", include_str!("../stdparts/gen/hexhead.fg")),
    ("key", include_str!("../stdparts/gen/key.fg")),
    ("nut", include_str!("../stdparts/gen/nut.fg")),
    ("oring", include_str!("../stdparts/gen/oring.fg")),
    ("pem_nut", include_str!("../stdparts/gen/pem_nut.fg")),
    ("pem_standoff", include_str!("../stdparts/gen/pem_standoff.fg")),
    ("pem_stud", include_str!("../stdparts/gen/pem_stud.fg")),
    ("pin", include_str!("../stdparts/gen/pin.fg")),
    ("ring", include_str!("../stdparts/gen/ring.fg")),
    ("setscrew", include_str!("../stdparts/gen/setscrew.fg")),
    ("shcs", include_str!("../stdparts/gen/shcs.fg")),
    ("shoulder", include_str!("../stdparts/gen/shoulder.fg")),
    ("washer", include_str!("../stdparts/gen/washer.fg")),
];

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
        // a table with no range per size (the ASME ones) offers no length under 1.5 d
        let floor = match (self.col("l_min"), thread_of(self, row)) {
            (None, Some((d, _))) => 1.5 * d,
            _ => f64::NEG_INFINITY,
        };
        let lo = self.value(row, "l_min").unwrap_or(floor);
        let hi = self.value(row, "l_max").unwrap_or(f64::INFINITY);
        self.lengths
            .iter()
            .copied()
            .filter(|l| *l >= lo && *l <= hi)
            .collect()
    }
    /// Whether a generator makes parts of it (hole tables are data only).
    pub fn placeable(&self) -> bool {
        self.get("gen").is_some()
    }
    pub fn has_lengths(&self) -> bool {
        !self.lengths.is_empty()
    }
    /// Where the family sits in a browser: `Fasteners/Bolts/Socket head`.
    pub fn category(&self) -> &str {
        self.get("category").unwrap_or("Other")
    }
    /// One of the family, as a BOM describes it: `Hexagon socket head cap screw`.
    pub fn item(&self) -> &str {
        self.get("item").unwrap_or(&self.title)
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
        rows.push(r);
    }
    let get = |k: &str| {
        head.iter()
            .find(|(x, _)| x == k)
            .map(|(_, v): &(String, String)| v.clone())
    };
    let req = |k: &str| get(k).ok_or(format!("no `{k}:` header"));
    let standard = req("standard")?;
    let name =
        get("name").unwrap_or_else(|| standard.split(':').next().unwrap_or(&standard).trim().to_string());
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
    line.split(',')
        .map(|c| c.trim().trim_matches('"').to_string())
        .collect()
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
        .find(|f| squash(&f.name) == q || f.id == name || squash(&f.standard) == q)
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
        .filter(|f| f.placeable())
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
    // hole tables are data for `hole`, not parts
    let f = family(&spec.family)
        .filter(|f| f.placeable())
        .ok_or_else(|| StdErr::UnknownFamily {
            span: at.family,
            name: spec.family.clone(),
            near: near_family(&spec.family),
            valid: families()
                .iter()
                .filter(|f| f.placeable())
                .map(|f| f.name.clone())
                .collect(),
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
    if let Some(s) = spec.options.get("sheet").and_then(|s| s.parse::<f64>().ok())
        && let Some(min) = f.value(row, "sheet_min")
        && s < min
    {
        return Err(StdErr::UnknownOption {
            span: at.options,
            family: f.name.clone(),
            option: "sheet".into(),
            value: crate::values::fmt(s),
            allowed: vec![format!(">= {}", crate::values::fmt(min))],
        });
    }
    Ok((f, row))
}

/// The major diameter and pitch a thread designation names, in the table's units: `M6`
/// (coarse, from the row's P), `M8x1`, `#10-24`, `1/4-20`, `1-1/4-7`.
fn thread_of(f: &Family, row: &[String]) -> Option<(f64, f64)> {
    // the thread is the size, or a column of its own (`thread`, or `d` written `M6`)
    let size = match (f.col("thread"), f.col("d")) {
        (Some(c), _) => row[c].as_str(),
        (None, Some(c)) if row[c].starts_with('M') => row[c].as_str(),
        _ => row[0].as_str(),
    };
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

/// The file key standing for this part: `std:ISO 4762|M6|20`, options other than the
/// defaults after it (`|thread=cosmetic`).
pub fn key(spec: &Spec) -> String {
    let f = family(&spec.family);
    let mut k = format!(
        "{KEY}{}|{}",
        f.map_or(spec.family.as_str(), |f| f.name.as_str()),
        spec.size
    );
    if let Some(l) = spec.length {
        k += &format!("|{}", crate::values::fmt(l));
    }
    for (o, v) in &spec.options {
        let default = f
            .and_then(|f| f.options.iter().find(|x| &x.0 == o))
            .and_then(|x| x.1.first());
        if default != Some(v) {
            k += &format!("|{o}={v}");
        }
    }
    k
}

/// A key read back into its choice (None: not a standard part key, or malformed).
pub fn spec_of(key: &str) -> Option<Spec> {
    let mut parts = key.strip_prefix(KEY)?.split('|');
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
        // an angle written out (`ang=90deg`) goes in as written
        if let Some(n) = from.trim().strip_suffix("deg")
            && n.parse::<f64>().is_ok()
        {
            set.insert(param.trim(), from.trim().to_string());
            continue;
        }
        let v = match from.trim() {
            "@d" => thread.map(|t| t.0).or_else(|| plain_d(&row[0])).map(|v| v * unit),
            "@P" => thread.map(|t| t.1 * unit),
            "@l" => spec.length.map(|l| l * unit),
            "@sheet" => opt("sheet")
                .and_then(|v| v.parse::<f64>().ok())
                .or_else(|| f.value(row, "sheet_min"))
                .map(|v| v * unit),
            "@modelled" => Some(if opt("thread").as_deref() == Some("modelled") {
                1.0
            } else {
                0.0
            }),
            o if o.starts_with('@') => opt(&o[1..]).and_then(|v| v.parse::<f64>().ok()).map(|v| v * unit),
            // a word column as a switch: `style==blind` is 1 on blind rows
            c if c.contains("==") => {
                let (col, word) = c.split_once("==").expect("contains ==");
                f.col(col).map(|i| if row[i] == word { 1.0 } else { 0.0 })
            }
            c => match c.parse::<f64>() {
                Ok(n) => Some(n),
                Err(_) => f.value(row, c).map(|v| v * unit),
            },
        };
        // a cell the standard leaves empty for this size: the generator's own value stands
        let Some(v) = v else { continue };
        set.insert(param.trim(), crate::values::fmt(round(v)));
    }
    let name = designation(f, spec);
    let mut out = String::new();
    out += &format!("# {} {}\n", f.item(), name.trim_start_matches(&f.name).trim());
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
    // the material: the grade's (`grade 12.9: ISO 898-1 class 12.9`), else the family's
    let material = opt("grade")
        .and_then(|g| f.get(&format!("grade {g}")).map(String::from))
        .or_else(|| f.get("material").map(String::from))
        .expect("a family names its material");
    out += &format!(
        "\nproperties(part_number: {name:?}, description: {:?}, material: {material:?}, custom: [(\"Standard\", {:?})])\n",
        format!("{} {}", f.item(), name.trim_start_matches(&f.name).trim()),
        f.standard,
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

/// The hole sizes the hole feature drills (`hole(screw: ..)`), for a dialog to offer: the kernel's
/// table (`forge_model::standard::SCREWS`), which `.fc` parts read too. `tables_hole` gives the
/// same rows from these tables; `tests/stdparts.rs` holds the two together, every difference a
/// row of `resolutions/hole_sizes.csv`.
pub fn hole_screws() -> &'static [forge_model::standard::Screw] {
    forge_model::standard::SCREWS
}

/// The hole size a designation names (`M6`, `M8x1`, `1/4-20`, `10-32` or `#10-32`).
pub fn hole_screw(name: &str) -> Option<&'static forge_model::standard::Screw> {
    forge_model::standard::screw(name.trim_start_matches('#'))
}

/// A kernel hole row with every value these tables hold put in from them (ISO 273 and ASME
/// B18.2.8 clearances, ISO 2306 and 75% inch tap drills, DIN 974-1 and ASME B18.3 counterbores,
/// ISO 10642 and ASME B18.3 countersinks). Metric holes of the ANSI convention take the ISO tap
/// drills and counterbores: there is no ASME metric tap drill table, and ASME B18.3.1M is
/// withdrawn.
pub fn tables_hole(k: &forge_model::standard::Screw) -> forge_model::standard::Screw {
    let mut s = forge_model::standard::Screw {
        name: k.name,
        size: k.size,
        pitch: k.pitch,
        clearance: k.clearance,
        tap: k.tap,
        counterbore: k.counterbore,
        countersink: k.countersink,
    };
    fill_hole(&mut s);
    s
}

fn table_value(fam: &str, size: &str, col: &str) -> Option<f64> {
    let f = families().iter().find(|f| f.id == fam)?;
    let unit = if f.inch { 25.4 } else { 1.0 };
    f.value(f.row(size)?, col).map(|v| v * unit)
}

fn fill_hole(s: &mut forge_model::standard::Screw) {
    let metric = s.name.starts_with('M');
    // the plain size a clearance or seat row is keyed by: M8 for M8x1, #10 for 10-32, 1/4 for 1/4-20
    // numbered inch sizes (#0 to #12) are under 1/4 inch; 1-8 is a whole inch
    let numbered = !metric && s.size < 6.3;
    let base: String = if metric {
        s.name.split('x').next().unwrap_or(s.name).to_string()
    } else {
        let b = s.name.rsplit_once('-').map_or(s.name, |x| x.0);
        if numbered { format!("#{b}") } else { b.to_string() }
    };
    let thread = if numbered {
        format!("#{}", s.name)
    } else {
        s.name.to_string()
    };
    let set = |v: &mut f64, x: Option<f64>| {
        if let Some(x) = x {
            *v = x;
        }
    };
    if metric {
        for (k, col) in ["fine", "medium", "coarse"].iter().enumerate() {
            set(&mut s.clearance[k], table_value("iso273", &base, col));
        }
        if let Some(t) = table_value("tap_drill_metric", &thread, "drill") {
            s.tap = [t, t];
        }
        if let (Some(d), Some(t)) = (
            table_value("cbore_iso4762", &base, "d1"),
            table_value("cbore_iso4762", &base, "t"),
        ) {
            s.counterbore = [(d, t), (d, t)];
        }
        if let (Some(d), Some((_, a))) = (table_value("csink_iso10642", &base, "dk_10642"), s.countersink) {
            s.countersink = Some((d, a));
        }
    } else {
        for (k, col) in ["close_min", "normal_min", "loose_min"].iter().enumerate() {
            set(&mut s.clearance[k], table_value("asme_clearance", &base, col));
        }
        if let Some(t) = table_value("tap_drill_unified", &thread, "drill_d") {
            s.tap = [t, t];
        }
        if let (Some(d), Some(h)) = (
            table_value("cbore_asme_b18_3", &base, "b"),
            table_value("cbore_asme_b18_3", &base, "h"),
        ) {
            s.counterbore = [(d, h), (d, h)];
        }
        if let (Some(d), Some((_, a))) = (
            table_value("asme_b18_3_flat", &thread, "dk_theor_max"),
            s.countersink,
        ) {
            s.countersink = Some((d, a));
        }
    }
}
