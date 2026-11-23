//! Sheet-metal vendor profiles: what a fabricator stocks and makes, as data a person can read and
//! edit, turned into the `forge_sheet::ShopProfile` the sheet builder and its checks read. A
//! profile is one vendor: its materials, each with its stocked thicknesses, and per stock the
//! tooled bend radii, the bend rule and the minimum distances; the vendor's process limits, the
//! operations it offers, the hardware it presses in and the tolerances it quotes. Every field is
//! optional; a rule left out is not checked.
//!
//! The built-in profiles (`data/*.toml`) restate fabricators' public design guides, each with its
//! source and the date it was read. They are not the vendors' own and not endorsed by them:
//! verify with the vendor before ordering. A project's profiles (`vendors/*.toml` beside its
//! files) and the user's (`<config>/forgecad/vendors/*.toml`) are the same format, searched first.
//!
//! The format (version 1), a TOML subset (`toml`):
//!
//! ```toml
//! format = 1
//! id = "acme"                     # what a part names it by
//! name = "Acme Sheet Metal"
//! source = "https://.."           # where the data was read
//! taken = "2026-11-22"            # when
//! units = "in"                    # lengths in this file: "in" or "mm"; angles in degrees
//! note = ".."
//! operations = ["laser", "bend", "hardware"]
//! kerf = 0.006
//!
//! [limits]                        # the blank, (shorter, longer) either way round
//! max_flat = [44, 68]             # also max_flat_bent, min_flat, min_flat_bent, max_flat_coated
//! max_bend_length = 44
//! max_box_flange = 3              # box_flange_when = "hardware"
//! bend_angles = [0, 130]
//!
//! [tolerance]                     # quoted, as text a drawing or a report shows
//! cut = "+/-0.005"
//!
//! [rules]                         # for every stock; a material's and a stock's own win
//! min_hole = "0.5t"               # LEN: a number, or "F t + G r + C" (t thickness, r the bend's
//! hole_to_edge = "min(2t, 0.125)" # radius), min(..)/max(..) of such
//!
//! [[material]]
//! name = "5052-H32 aluminum"
//! aliases = ["5052"]
//! library = "5052-H32"            # forge-material's name: density, the drawing's note
//!
//! [[material.stock]]
//! t = 0.063
//! gauge = "14 ga"
//! radius = [0.035]                # tooled inside radii; the first is the default
//! k = 0.42                        # or ktable = [[r/t, k], ..], or bend_deduction = [BD at 90deg
//!                                 # for each radius]
//! min_flange = 0.303              # and any [rules] key
//!
//! [[hardware]]
//! kind = "nut"  size = "M3"  code = "S-M3-0"  sheet = [0.031, 0.5]  hole = 0.168  edge = 0.25
//! ```
//!
//! `tooling`: the press brake's, for the bend simulator: `tooling = "std-eu"`, or a `[tooling]`
//! table forge-brake reads (see `Tooling`).
//!
//! Rule keys: `min_flange`, `hole_to_bend`, `hole_to_edge`, `bend_to_edge`, `min_hole`,
//! `min_slot`, `min_web`, `bend_from` ("tangent" | "centre"), `bend_angles`, `max_bend_length`,
//! `relief = { shape, width, depth, corner_gap }`.

pub mod toml;

use forge_sheet as fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use toml::{Item, Table};

/// The format this reads and writes.
pub const FORMAT: f64 = 1.0;

/// The built-in profiles, by file.
pub const BUILTIN: [(&str, &str); 5] = [
    ("sendcutsend.toml", include_str!("../data/sendcutsend.toml")),
    ("fabworks.toml", include_str!("../data/fabworks.toml")),
    ("oshcut.toml", include_str!("../data/oshcut.toml")),
    ("xometry.toml", include_str!("../data/xometry.toml")),
    ("jlc.toml", include_str!("../data/jlc.toml")),
];

/// What every built-in profile's note ends with, and what a profile shown anywhere is told.
pub const VERIFY: &str = "Verify with the vendor before ordering.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Units {
    Mm,
    In,
}

impl Units {
    pub fn mm(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::In => 25.4,
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::In => "in",
        }
    }
}

/// A length a rule gives: `per_t` t + `per_r` r + `plus` (in the file's units), or the least or
/// greatest of such.
#[derive(Clone, Debug, PartialEq)]
pub enum Len {
    Lin { per_t: f64, per_r: f64, plus: f64 },
    Min(Vec<Len>),
    Max(Vec<Len>),
}

impl Len {
    pub fn constant(v: f64) -> Len {
        Len::Lin {
            per_t: 0.0,
            per_r: 0.0,
            plus: v,
        }
    }

    /// Its value in mm for thickness `t` and radius `r` (mm), lengths of the file scaled by `u`.
    pub fn at(&self, t: f64, r: f64, u: f64) -> f64 {
        match self {
            Len::Lin { per_t, per_r, plus } => per_t * t + per_r * r + plus * u,
            Len::Min(xs) => xs.iter().map(|x| x.at(t, r, u)).fold(f64::INFINITY, f64::min),
            Len::Max(xs) => xs.iter().map(|x| x.at(t, r, u)).fold(f64::NEG_INFINITY, f64::max),
        }
    }

    /// As the kernel's linear rule at thickness `t` (mm): a min or max is taken at `t` and the
    /// stock's default radius `r`, so it holds exactly at that radius.
    pub fn scaled(&self, t: f64, r: f64, u: f64) -> fs::Scaled {
        match self {
            Len::Lin { per_t, per_r, plus } => fs::Scaled {
                per_t: *per_t,
                per_r: *per_r,
                plus: plus * u,
            },
            _ => fs::Scaled {
                per_t: 0.0,
                per_r: 0.0,
                plus: self.at(t, r, u),
            },
        }
    }

    pub fn parse(s: &str) -> Result<Len, String> {
        let s = s.trim();
        for (word, many) in [("min(", true), ("max(", false)] {
            if let Some(body) = s.strip_prefix(word).and_then(|b| b.strip_suffix(')')) {
                let xs = body.split(',').map(Len::parse).collect::<Result<Vec<_>, _>>()?;
                if xs.len() < 2 {
                    return Err(format!("`{s}`: {word}..) takes two or more lengths"));
                }
                return Ok(if many { Len::Min(xs) } else { Len::Max(xs) });
            }
        }
        let (mut per_t, mut per_r, mut plus) = (0.0, 0.0, 0.0);
        let spaced = s
            .replace(' ', "")
            .replace('-', "+-")
            .replace("e+-", "e-")
            .replace("E+-", "E-");
        for term in spaced.split('+').filter(|x| !x.is_empty()) {
            let (num, unit) = match term.strip_suffix('t').or(term.strip_suffix('T')) {
                Some(n) => (n.trim(), 't'),
                None => match term.strip_suffix('r').or(term.strip_suffix('R')) {
                    Some(n) => (n.trim(), 'r'),
                    None => (term, ' '),
                },
            };
            let num = num.trim_end_matches('*').trim();
            let v = match num {
                "" => 1.0,
                "-" => -1.0,
                n => n
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| format!("`{s}`: `{term}` is not a number, `N t` or `N r`"))?,
            };
            match unit {
                't' => per_t += v,
                'r' => per_r += v,
                _ => plus += v,
            }
        }
        Ok(Len::Lin { per_t, per_r, plus })
    }

    pub fn text(&self) -> String {
        match self {
            Len::Lin { per_t, per_r, plus } => {
                let mut parts = vec![];
                let term = |v: f64, s: &str| match v {
                    1.0 => s.to_string(),
                    v => format!("{}{s}", toml::num(v)),
                };
                if *per_t != 0.0 {
                    parts.push(term(*per_t, "t"));
                }
                if *per_r != 0.0 {
                    parts.push(term(*per_r, "r"));
                }
                if *plus != 0.0 || parts.is_empty() {
                    parts.push(toml::num(*plus));
                }
                parts.join(" + ").replace("+ -", "- ")
            }
            Len::Min(xs) | Len::Max(xs) => format!(
                "{}({})",
                if matches!(self, Len::Min(_)) { "min" } else { "max" },
                xs.iter().map(Len::text).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    fn item(&self) -> Item {
        match self {
            Len::Lin {
                per_t: 0.0,
                per_r: 0.0,
                plus,
            } => Item::Num(*plus),
            _ => Item::Str(self.text()),
        }
    }
}

/// A relief rule as a vendor states it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReliefSpec {
    /// "rect" | "obround" | "tear"
    pub shape: Option<String>,
    pub width: Option<Len>,
    /// From the bend line.
    pub depth: Option<Len>,
    /// The gap left where two flanges meet at a corner.
    pub corner_gap: Option<Len>,
}

/// Rules for stocks: the vendor's, a material's, a stock's own. Lengths are `Len`s.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Rules {
    pub min_flange: Option<Len>,
    pub hole_to_bend: Option<Len>,
    pub hole_to_edge: Option<Len>,
    pub bend_to_edge: Option<Len>,
    pub min_hole: Option<Len>,
    pub min_slot: Option<Len>,
    pub min_web: Option<Len>,
    /// Hole to bend and bend to edge from the bend's "tangent" line (default) or its "centre".
    pub bend_from: Option<String>,
    /// Degrees.
    pub bend_angles: Option<(f64, f64)>,
    pub max_bend_length: Option<f64>,
    pub relief: Option<ReliefSpec>,
}

pub const RULE_KEYS: [&str; 11] = [
    "min_flange",
    "hole_to_bend",
    "hole_to_edge",
    "bend_to_edge",
    "min_hole",
    "min_slot",
    "min_web",
    "bend_from",
    "bend_angles",
    "max_bend_length",
    "relief",
];

impl Rules {
    /// These rules over `under`'s, key by key.
    pub fn over(&self, under: &Rules) -> Rules {
        let relief = match (&self.relief, &under.relief) {
            (Some(a), Some(b)) => Some(ReliefSpec {
                shape: a.shape.clone().or(b.shape.clone()),
                width: a.width.clone().or(b.width.clone()),
                depth: a.depth.clone().or(b.depth.clone()),
                corner_gap: a.corner_gap.clone().or(b.corner_gap.clone()),
            }),
            (a, b) => a.clone().or(b.clone()),
        };
        Rules {
            min_flange: self.min_flange.clone().or(under.min_flange.clone()),
            hole_to_bend: self.hole_to_bend.clone().or(under.hole_to_bend.clone()),
            hole_to_edge: self.hole_to_edge.clone().or(under.hole_to_edge.clone()),
            bend_to_edge: self.bend_to_edge.clone().or(under.bend_to_edge.clone()),
            min_hole: self.min_hole.clone().or(under.min_hole.clone()),
            min_slot: self.min_slot.clone().or(under.min_slot.clone()),
            min_web: self.min_web.clone().or(under.min_web.clone()),
            bend_from: self.bend_from.clone().or(under.bend_from.clone()),
            bend_angles: self.bend_angles.or(under.bend_angles),
            max_bend_length: self.max_bend_length.or(under.max_bend_length),
            relief,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct StockRow {
    /// Nominal thickness, file units.
    pub t: f64,
    pub gauge: Option<String>,
    /// Tooled inside radii (file units), the default first. Empty: the vendor does not bend it.
    pub radius: Vec<f64>,
    /// How its bends develop: K, else K by r/t (rows of (r/t, K), increasing), else the bend
    /// deduction at 90 degrees for each tooled radius (file units). All kept as written; the
    /// first stated is the one bent by.
    pub k: Option<f64>,
    pub ktable: Option<Vec<(f64, f64)>>,
    pub bend_deduction: Option<Vec<f64>>,
    pub rules: Rules,
    /// Operations this stock does not get though the vendor offers them (`["bend"]`).
    pub not: Vec<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MaterialRow {
    pub name: String,
    pub aliases: Vec<String>,
    /// forge-material's name for it.
    pub library: Option<String>,
    /// Rockwell B.
    pub hardness: Option<f64>,
    pub rules: Rules,
    /// Operations this material does not get though the vendor offers them.
    pub not: Vec<String>,
    pub note: Option<String>,
    pub stock: Vec<StockRow>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Limits {
    pub max_flat: Option<(f64, f64)>,
    pub max_flat_bent: Option<(f64, f64)>,
    pub max_flat_coated: Option<(f64, f64)>,
    pub min_flat: Option<(f64, f64)>,
    pub min_flat_bent: Option<(f64, f64)>,
    pub max_bend_length: Option<f64>,
    pub max_box_flange: Option<f64>,
    /// "always" | "hardware"
    pub box_flange_when: Option<String>,
    /// Degrees.
    pub bend_angles: Option<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HardwareRow {
    pub kind: String,
    pub size: String,
    pub code: String,
    /// Sheet thickness range (file units).
    pub sheet: (f64, f64),
    pub hole: f64,
    /// Least hole centre to edge or bend.
    pub edge: f64,
    pub max_hardness: Option<f64>,
    /// Materials it goes in (names or aliases); empty: any.
    pub materials: Vec<String>,
}

/// The press-brake tooling a vendor runs, for the bend simulator (kernel/forge-brake): a set's
/// id (`tooling = "std-eu"`), or a table (`[tooling]`: `base`, `punches`, `dies`, `press`,
/// `clearance`, `self_clearance`, `springback`, as forge-brake names them; lengths in the file's
/// `units`, which forge-brake scales by, angles in degrees). Kept
/// as written and written back unchanged: forge-brake reads and checks it, not this crate.
#[derive(Clone, Debug, PartialEq)]
pub enum Tooling {
    Set(String),
    Table(Table),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vendor {
    pub id: String,
    pub name: String,
    pub source: Option<String>,
    pub taken: Option<String>,
    pub units: Units,
    pub note: Option<String>,
    pub operations: Option<Vec<String>>,
    pub kerf: Option<f64>,
    pub limits: Limits,
    /// Quoted tolerances, as written.
    pub tolerance: Vec<(String, String)>,
    pub rules: Rules,
    pub materials: Vec<MaterialRow>,
    pub hardware: Vec<HardwareRow>,
    pub tooling: Option<Tooling>,
}

/// Why a profile does not read.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub line: Option<usize>,
    pub why: String,
}

fn bad(why: impl Into<String>) -> ReadError {
    ReadError {
        line: None,
        why: why.into(),
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.why),
            None => write!(f, "{}", self.why),
        }
    }
}

/// A table's keys read one by one, any left over refused.
struct Reader<'a> {
    t: &'a Table,
    at: String,
    used: Vec<&'static str>,
}

impl<'a> Reader<'a> {
    fn new(t: &'a Table, at: impl Into<String>) -> Reader<'a> {
        Reader {
            t,
            at: at.into(),
            used: vec![],
        }
    }
    fn get(&mut self, k: &'static str) -> Option<&'a Item> {
        self.used.push(k);
        self.t.get(k)
    }
    fn place(&self, k: &str) -> String {
        if self.at.is_empty() {
            k.into()
        } else {
            format!("{}.{k}", self.at)
        }
    }
    fn str(&mut self, k: &'static str) -> Result<Option<String>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Str(s)) => Ok(Some(s.clone())),
            Some(_) => Err(bad(format!("`{}` takes text", self.place(k)))),
        }
    }
    fn num(&mut self, k: &'static str) -> Result<Option<f64>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Num(v)) => Ok(Some(*v)),
            Some(_) => Err(bad(format!("`{}` takes a number", self.place(k)))),
        }
    }
    fn pos(&mut self, k: &'static str) -> Result<Option<f64>, ReadError> {
        let v = self.num(k)?;
        if v.is_some_and(|v| v < 0.0) {
            return Err(bad(format!("`{}` is negative", self.place(k))));
        }
        Ok(v)
    }
    fn strs(&mut self, k: &'static str) -> Result<Option<Vec<String>>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Arr(xs)) => xs
                .iter()
                .map(|x| match x {
                    Item::Str(s) => Ok(s.clone()),
                    _ => Err(bad(format!("`{}` takes a list of text", self.place(k)))),
                })
                .collect::<Result<_, _>>()
                .map(Some),
            Some(_) => Err(bad(format!("`{}` takes a list of text", self.place(k)))),
        }
    }
    fn nums(&mut self, k: &'static str) -> Result<Option<Vec<f64>>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Num(v)) => Ok(Some(vec![*v])),
            Some(Item::Arr(xs)) => xs
                .iter()
                .map(|x| match x {
                    Item::Num(v) => Ok(*v),
                    _ => Err(bad(format!("`{}` takes numbers", self.place(k)))),
                })
                .collect::<Result<_, _>>()
                .map(Some),
            Some(_) => Err(bad(format!("`{}` takes numbers", self.place(k)))),
        }
    }
    fn pair(&mut self, k: &'static str) -> Result<Option<(f64, f64)>, ReadError> {
        let place = self.place(k);
        match self.nums(k)? {
            None => Ok(None),
            Some(v) if v.len() == 2 => Ok(Some((v[0], v[1]))),
            Some(_) => Err(bad(format!("`{place}` takes two numbers"))),
        }
    }
    fn len(&mut self, k: &'static str) -> Result<Option<Len>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Num(v)) => Ok(Some(Len::constant(*v))),
            Some(Item::Str(s)) => Len::parse(s)
                .map(Some)
                .map_err(|e| bad(format!("`{}`: {e}", self.place(k)))),
            Some(_) => Err(bad(format!(
                "`{}` takes a length: a number or text like \"2t + r\"",
                self.place(k)
            ))),
        }
    }
    fn table(&mut self, k: &'static str) -> Result<Option<&'a Table>, ReadError> {
        match self.get(k) {
            None => Ok(None),
            Some(Item::Table(t)) => Ok(Some(t)),
            Some(_) => Err(bad(format!("`{}` takes a table", self.place(k)))),
        }
    }
    fn tables(&mut self, k: &'static str) -> Result<&'a [Table], ReadError> {
        match self.get(k) {
            None => Ok(&[]),
            Some(Item::Tables(ts)) => Ok(ts),
            Some(_) => Err(bad(format!("`{}` takes [[{}]] records", self.place(k), k))),
        }
    }
    /// The keys not read: refused, so a misspelt rule is never silently unchecked.
    fn done(self) -> Result<(), ReadError> {
        match self.t.keys().find(|k| !self.used.contains(k)) {
            Some(k) => Err(bad(format!("`{}` is not a key this format has", self.place(k)))),
            None => Ok(()),
        }
    }
}

fn rules_of(r: &mut Reader) -> Result<Rules, ReadError> {
    let relief = match r.table("relief")? {
        None => None,
        Some(t) => {
            let mut q = Reader::new(t, r.place("relief"));
            let spec = ReliefSpec {
                shape: q.str("shape")?,
                width: q.len("width")?,
                depth: q.len("depth")?,
                corner_gap: q.len("corner_gap")?,
            };
            if let Some(s) = &spec.shape
                && !["rect", "obround", "tear"].contains(&s.as_str())
            {
                return Err(bad(format!("`{}.shape`: rect, obround or tear", q.at)));
            }
            q.done()?;
            Some(spec)
        }
    };
    let rules = Rules {
        min_flange: r.len("min_flange")?,
        hole_to_bend: r.len("hole_to_bend")?,
        hole_to_edge: r.len("hole_to_edge")?,
        bend_to_edge: r.len("bend_to_edge")?,
        min_hole: r.len("min_hole")?,
        min_slot: r.len("min_slot")?,
        min_web: r.len("min_web")?,
        bend_from: r.str("bend_from")?,
        bend_angles: r.pair("bend_angles")?,
        max_bend_length: r.pos("max_bend_length")?,
        relief,
    };
    if let Some(b) = &rules.bend_from
        && b != "tangent"
        && b != "centre"
    {
        return Err(bad(format!("`{}`: tangent or centre", r.place("bend_from"))));
    }
    Ok(rules)
}

fn rules_into(t: &mut Table, r: &Rules) {
    let mut put = |k: &str, v: &Option<Len>| {
        if let Some(v) = v {
            t.set(k, v.item());
        }
    };
    put("min_flange", &r.min_flange);
    put("hole_to_bend", &r.hole_to_bend);
    put("hole_to_edge", &r.hole_to_edge);
    put("bend_to_edge", &r.bend_to_edge);
    put("min_hole", &r.min_hole);
    put("min_slot", &r.min_slot);
    put("min_web", &r.min_web);
    if let Some(b) = &r.bend_from {
        t.set("bend_from", Item::Str(b.clone()));
    }
    if let Some((a, b)) = r.bend_angles {
        t.set("bend_angles", Item::Arr(vec![Item::Num(a), Item::Num(b)]));
    }
    if let Some(v) = r.max_bend_length {
        t.set("max_bend_length", Item::Num(v));
    }
    if let Some(s) = &r.relief {
        let mut x = Table::default();
        if let Some(v) = &s.shape {
            x.set("shape", Item::Str(v.clone()));
        }
        for (k, v) in [
            ("width", &s.width),
            ("depth", &s.depth),
            ("corner_gap", &s.corner_gap),
        ] {
            if let Some(v) = v {
                x.set(k, v.item());
            }
        }
        t.set("relief", Item::Table(x));
    }
}

fn ops_checked(ops: &[String], at: &str) -> Result<(), ReadError> {
    match ops.iter().find(|o| fs::Operation::from_name(o).is_none()) {
        Some(o) => Err(bad(format!(
            "`{at}`: `{o}` is not an operation; one of {}",
            fs::Operation::ALL.map(|o| o.name()).join(", ")
        ))),
        None => Ok(()),
    }
}

impl Vendor {
    /// A profile from its text.
    pub fn parse(src: &str) -> Result<Vendor, ReadError> {
        let t = toml::parse(src).map_err(|e| ReadError {
            line: Some(e.line),
            why: e.why,
        })?;
        Vendor::from_table(&t)
    }

    pub fn from_table(t: &Table) -> Result<Vendor, ReadError> {
        let mut r = Reader::new(t, "");
        match r.num("format")? {
            Some(FORMAT) => {}
            Some(v) => {
                return Err(bad(format!(
                    "format {v}: this reads vendor profiles of format {FORMAT}"
                )));
            }
            None => return Err(bad("`format = 1` is missing")),
        }
        let id = r.str("id")?.ok_or_else(|| bad("`id` is missing"))?;
        if id.is_empty()
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(bad(format!("`id` \"{id}\": letters, digits, - and _ only")));
        }
        let units = match r.str("units")?.as_deref() {
            None | Some("mm") => Units::Mm,
            Some("in") => Units::In,
            Some(o) => return Err(bad(format!("`units` \"{o}\": mm or in"))),
        };
        let operations = r.strs("operations")?;
        if let Some(ops) = &operations {
            ops_checked(ops, "operations")?;
        }
        let mut v = Vendor {
            name: r.str("name")?.unwrap_or_else(|| id.clone()),
            id,
            source: r.str("source")?,
            taken: r.str("taken")?,
            units,
            note: r.str("note")?,
            operations,
            kerf: r.pos("kerf")?,
            limits: Limits::default(),
            tolerance: vec![],
            rules: Rules::default(),
            materials: vec![],
            hardware: vec![],
            tooling: match r.get("tooling") {
                None => None,
                Some(Item::Str(id)) => Some(Tooling::Set(id.clone())),
                Some(Item::Table(t)) => Some(Tooling::Table(t.clone())),
                Some(_) => return Err(bad("`tooling` takes a set's id or a [tooling] table")),
            },
        };
        if let Some(l) = r.table("limits")? {
            let mut q = Reader::new(l, "limits");
            v.limits = Limits {
                max_flat: q.pair("max_flat")?,
                max_flat_bent: q.pair("max_flat_bent")?,
                max_flat_coated: q.pair("max_flat_coated")?,
                min_flat: q.pair("min_flat")?,
                min_flat_bent: q.pair("min_flat_bent")?,
                max_bend_length: q.pos("max_bend_length")?,
                max_box_flange: q.pos("max_box_flange")?,
                box_flange_when: q.str("box_flange_when")?,
                bend_angles: q.pair("bend_angles")?,
            };
            if let Some(w) = &v.limits.box_flange_when
                && w != "always"
                && w != "hardware"
            {
                return Err(bad("`limits.box_flange_when`: always or hardware"));
            }
            q.done()?;
        }
        if let Some(tol) = r.table("tolerance")? {
            for (k, x) in &tol.0 {
                let text = match x {
                    Item::Str(s) => s.clone(),
                    Item::Num(n) => toml::num(*n),
                    _ => return Err(bad(format!("`tolerance.{k}` takes text or a number"))),
                };
                v.tolerance.push((k.clone(), text));
            }
        }
        if let Some(rt) = r.table("rules")? {
            let mut q = Reader::new(rt, "rules");
            v.rules = rules_of(&mut q)?;
            q.done()?;
        }
        for (i, m) in r.tables("material")?.iter().enumerate() {
            let mut q = Reader::new(m, format!("material[{i}]"));
            let name = q
                .str("name")?
                .ok_or_else(|| bad(format!("material[{i}] has no `name`")))?;
            let mut row = MaterialRow {
                aliases: q.strs("aliases")?.unwrap_or_default(),
                library: q.str("library")?,
                hardness: q.pos("hardness")?,
                not: q.strs("not")?.unwrap_or_default(),
                note: q.str("note")?,
                rules: rules_of(&mut q)?,
                stock: vec![],
                name,
            };
            ops_checked(&row.not, &q.place("not"))?;
            for (j, s) in q.tables("stock")?.iter().enumerate() {
                let mut p = Reader::new(s, format!("material[{i}].stock[{j}]"));
                let t = p.pos("t")?.ok_or_else(|| bad(format!("{} has no `t`", p.at)))?;
                if t <= 0.0 {
                    return Err(bad(format!("{}: `t` must be positive", p.at)));
                }
                let radius = p.nums("radius")?.unwrap_or_default();
                if radius.iter().any(|r| *r <= 0.0) {
                    return Err(bad(format!("{}: a radius must be positive", p.at)));
                }
                let k = p.num("k")?;
                let ktable = match p.get("ktable") {
                    None => None,
                    Some(Item::Arr(rows)) => Some(
                        rows.iter()
                            .map(|row| match row {
                                Item::Arr(xs) => match xs.as_slice() {
                                    [Item::Num(a), Item::Num(b)] => Ok((*a, *b)),
                                    _ => Err(bad(format!("{}.ktable: rows of [r/t, K]", p.at))),
                                },
                                _ => Err(bad(format!("{}.ktable: rows of [r/t, K]", p.at))),
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                    Some(_) => return Err(bad(format!("{}.ktable: rows of [r/t, K]", p.at))),
                };
                let bend_deduction = p.nums("bend_deduction")?;
                if bend_deduction.as_ref().is_some_and(|d| d.len() != radius.len()) {
                    return Err(bad(format!(
                        "{}: bend_deduction takes one value for each radius",
                        p.at
                    )));
                }
                // a table is read by r/t: two rows of one r/t (or one radius twice, where the
                // radii make it) are refused, naming the second
                if let Some(rows) = &ktable
                    && let Some(k) = repeated(rows.iter().map(|r| r.0))
                {
                    return Err(bad(format!("{}.ktable: row {k} repeats r/t {}", p.at, rows[k].0)));
                }
                if bend_deduction.is_some()
                    && let Some(k) = repeated(radius.iter().copied())
                {
                    return Err(bad(format!(
                        "{}.radius: radius {k} repeats {} (bend_deduction gives one K per radius)",
                        p.at, radius[k]
                    )));
                }
                if let Some(k) = k
                    && !(0.0..=1.0).contains(&k)
                {
                    return Err(bad(format!("{}: K {k} is not in [0, 1]", p.at)));
                }
                let stock = StockRow {
                    t,
                    gauge: p.str("gauge")?,
                    radius,
                    k,
                    ktable,
                    bend_deduction,
                    not: p.strs("not")?.unwrap_or_default(),
                    note: p.str("note")?,
                    rules: rules_of(&mut p)?,
                };
                ops_checked(&stock.not, &p.place("not"))?;
                p.done()?;
                row.stock.push(stock);
            }
            q.done()?;
            v.materials.push(row);
        }
        for (i, h) in r.tables("hardware")?.iter().enumerate() {
            let mut q = Reader::new(h, format!("hardware[{i}]"));
            let need = |x: Option<f64>, k: &str| x.ok_or_else(|| bad(format!("hardware[{i}] has no `{k}`")));
            let row = HardwareRow {
                kind: q.str("kind")?.unwrap_or_default(),
                size: q.str("size")?.unwrap_or_default(),
                code: q.str("code")?.unwrap_or_default(),
                sheet: q
                    .pair("sheet")?
                    .ok_or_else(|| bad(format!("hardware[{i}] has no `sheet`")))?,
                hole: need(q.pos("hole")?, "hole")?,
                edge: need(q.pos("edge")?, "edge")?,
                max_hardness: q.pos("max_hardness")?,
                materials: q.strs("materials")?.unwrap_or_default(),
            };
            if fs::HardwareKind::from_name(&row.kind).is_none() {
                return Err(bad(format!("hardware[{i}].kind: nut, stud or standoff")));
            }
            if row.size.is_empty() {
                return Err(bad(format!("hardware[{i}] has no `size`")));
            }
            q.done()?;
            v.hardware.push(row);
        }
        r.done()?;
        Ok(v)
    }

    /// The canonical text (what `parse` reads back to this).
    pub fn to_text(&self) -> String {
        let mut t = Table::default();
        t.set("format", Item::Num(FORMAT));
        t.set("id", Item::Str(self.id.clone()));
        t.set("name", Item::Str(self.name.clone()));
        let mut opt = |k: &str, v: &Option<String>| {
            if let Some(v) = v {
                t.set(k, Item::Str(v.clone()));
            }
        };
        opt("source", &self.source);
        opt("taken", &self.taken);
        t.set("units", Item::Str(self.units.word().into()));
        if let Some(n) = &self.note {
            t.set("note", Item::Str(n.clone()));
        }
        let strs = |v: &[String]| Item::Arr(v.iter().map(|s| Item::Str(s.clone())).collect());
        let pair = |(a, b): (f64, f64)| Item::Arr(vec![Item::Num(a), Item::Num(b)]);
        if let Some(ops) = &self.operations {
            t.set("operations", strs(ops));
        }
        if let Some(k) = self.kerf {
            t.set("kerf", Item::Num(k));
        }
        let l = &self.limits;
        let mut lt = Table::default();
        for (k, v) in [
            ("max_flat", l.max_flat),
            ("max_flat_bent", l.max_flat_bent),
            ("max_flat_coated", l.max_flat_coated),
            ("min_flat", l.min_flat),
            ("min_flat_bent", l.min_flat_bent),
        ] {
            if let Some(v) = v {
                lt.set(k, pair(v));
            }
        }
        if let Some(v) = l.max_bend_length {
            lt.set("max_bend_length", Item::Num(v));
        }
        if let Some(v) = l.max_box_flange {
            lt.set("max_box_flange", Item::Num(v));
        }
        if let Some(v) = &l.box_flange_when {
            lt.set("box_flange_when", Item::Str(v.clone()));
        }
        if let Some(v) = l.bend_angles {
            lt.set("bend_angles", pair(v));
        }
        if !lt.0.is_empty() {
            t.set("limits", Item::Table(lt));
        }
        if !self.tolerance.is_empty() {
            let mut tt = Table::default();
            for (k, v) in &self.tolerance {
                tt.set(k, Item::Str(v.clone()));
            }
            t.set("tolerance", Item::Table(tt));
        }
        let mut rt = Table::default();
        rules_into(&mut rt, &self.rules);
        if !rt.0.is_empty() {
            t.set("rules", Item::Table(rt));
        }
        let mats: Vec<Table> = self
            .materials
            .iter()
            .map(|m| {
                let mut mt = Table::default();
                mt.set("name", Item::Str(m.name.clone()));
                if !m.aliases.is_empty() {
                    mt.set("aliases", strs(&m.aliases));
                }
                if let Some(l) = &m.library {
                    mt.set("library", Item::Str(l.clone()));
                }
                if let Some(h) = m.hardness {
                    mt.set("hardness", Item::Num(h));
                }
                if !m.not.is_empty() {
                    mt.set("not", strs(&m.not));
                }
                if let Some(n) = &m.note {
                    mt.set("note", Item::Str(n.clone()));
                }
                rules_into(&mut mt, &m.rules);
                let stocks: Vec<Table> = m
                    .stock
                    .iter()
                    .map(|s| {
                        let mut st = Table::default();
                        st.set("t", Item::Num(s.t));
                        if let Some(g) = &s.gauge {
                            st.set("gauge", Item::Str(g.clone()));
                        }
                        if !s.radius.is_empty() {
                            st.set(
                                "radius",
                                Item::Arr(s.radius.iter().map(|r| Item::Num(*r)).collect()),
                            );
                        }
                        if let Some(k) = s.k {
                            st.set("k", Item::Num(k));
                        }
                        if let Some(rows) = &s.ktable {
                            st.set("ktable", Item::Arr(rows.iter().map(|r| pair(*r)).collect()));
                        }
                        if let Some(d) = &s.bend_deduction {
                            st.set(
                                "bend_deduction",
                                Item::Arr(d.iter().map(|x| Item::Num(*x)).collect()),
                            );
                        }
                        if !s.not.is_empty() {
                            st.set("not", strs(&s.not));
                        }
                        if let Some(n) = &s.note {
                            st.set("note", Item::Str(n.clone()));
                        }
                        rules_into(&mut st, &s.rules);
                        st
                    })
                    .collect();
                if !stocks.is_empty() {
                    mt.set("stock", Item::Tables(stocks));
                }
                mt
            })
            .collect();
        if !mats.is_empty() {
            t.set("material", Item::Tables(mats));
        }
        let hw: Vec<Table> = self
            .hardware
            .iter()
            .map(|h| {
                let mut ht = Table::default();
                ht.set("kind", Item::Str(h.kind.clone()));
                ht.set("size", Item::Str(h.size.clone()));
                if !h.code.is_empty() {
                    ht.set("code", Item::Str(h.code.clone()));
                }
                ht.set("sheet", pair(h.sheet));
                ht.set("hole", Item::Num(h.hole));
                ht.set("edge", Item::Num(h.edge));
                if let Some(x) = h.max_hardness {
                    ht.set("max_hardness", Item::Num(x));
                }
                if !h.materials.is_empty() {
                    ht.set("materials", strs(&h.materials));
                }
                ht
            })
            .collect();
        if !hw.is_empty() {
            t.set("hardware", Item::Tables(hw));
        }
        match &self.tooling {
            None => {}
            Some(Tooling::Set(id)) => t.set("tooling", Item::Str(id.clone())),
            Some(Tooling::Table(x)) => t.set("tooling", Item::Table(x.clone())),
        }
        format!(
            "# ForgeCAD sheet-metal vendor profile (forge-vendor format {FORMAT})\n{}",
            toml::write(&t, &["relief"])
        )
    }

    /// The material by name or alias, ignoring case, spaces and punctuation.
    pub fn material(&self, name: &str) -> Option<&MaterialRow> {
        let n = forge_material::norm(name);
        self.materials.iter().find(|m| {
            forge_material::norm(&m.name) == n || m.aliases.iter().any(|a| forge_material::norm(a) == n)
        })
    }

    /// The thicknesses (mm) a material is stocked in, with their gauges.
    pub fn thicknesses(&self, m: &MaterialRow) -> Vec<(f64, Option<String>)> {
        let u = self.units.mm();
        m.stock.iter().map(|s| (s.t * u, s.gauge.clone())).collect()
    }

    /// The shop this vendor is for sheets of material `m`: its stocks (bend rule and radii, the
    /// rules merged stock over material over vendor), process limits, operations and hardware.
    pub fn shop(&self, m: &MaterialRow) -> fs::ShopProfile {
        let u = self.units.mm();
        let mm2 = |p: Option<(f64, f64)>| p.map(|(a, b)| (a.min(b) * u, a.max(b) * u));
        let deg = |p: Option<(f64, f64)>| p.map(|(a, b)| (a.to_radians(), b.to_radians()));
        let kerf = self.kerf.map_or(0.2, |k| k * u);
        let mut stock = vec![];
        for s in &m.stock {
            let t = s.t * u;
            let rules = s.rules.over(&m.rules.over(&self.rules));
            let radii: Vec<f64> = s.radius.iter().map(|r| r * u).collect();
            let r0 = radii.first().copied().unwrap_or(t);
            let rule = match (s.k, &s.ktable, &s.bend_deduction) {
                (Some(k), _, _) => fs::BendRule::K(k),
                (None, Some(rows), _) => fs::BendRule::KTable(
                    sorted(rows)
                        .iter()
                        .map(|&(q, k)| fs::KRow { r_over_t: q, k })
                        .collect(),
                ),
                (None, None, Some(d)) => {
                    // BA = 2 (R + t) tan 45deg - BD; K from BA = pi/2 (R + K t), at each radius
                    let ks: Vec<(f64, f64)> = radii
                        .iter()
                        .zip(d)
                        .map(|(&r, &bd)| {
                            let ba = 2.0 * (r + t) - bd * u;
                            (r / t, (ba / std::f64::consts::FRAC_PI_2 - r) / t)
                        })
                        .collect();
                    match ks.as_slice() {
                        [(_, k)] => fs::BendRule::K(*k),
                        // by r/t, as the kernel interpolates it, whichever radius is the default
                        _ => fs::BendRule::KTable(
                            sorted(&ks)
                                .iter()
                                .map(|&(q, k)| fs::KRow { r_over_t: q, k })
                                .collect(),
                        ),
                    }
                }
                // no bend data: the common rule of thumb, never reached by a bend the stock is
                // not tooled for without RadiusNotTooled
                (None, None, None) => fs::BendRule::K(0.44),
            };
            let at = |l: &Option<Len>| l.as_ref().map_or(fs::Scaled::NONE, |l| l.scaled(t, r0, u));
            let mut relief = fs::ReliefRule::default();
            if let Some(rs) = &rules.relief {
                if let Some(shape) = &rs.shape {
                    relief.bend.kind = match shape.as_str() {
                        "obround" => fs::ReliefKind::Obround,
                        "tear" => fs::ReliefKind::Tear,
                        _ => fs::ReliefKind::Rect,
                    };
                }
                if rs.width.is_some() {
                    relief.bend.width = at(&rs.width);
                }
                if rs.depth.is_some() {
                    relief.bend.depth = at(&rs.depth);
                }
                if rs.corner_gap.is_some() {
                    relief.corner.gap = at(&rs.corner_gap);
                }
            }
            relief.kerf = kerf;
            stock.push(fs::Stock {
                thickness: t,
                rule,
                radii,
                min_flange: at(&rules.min_flange),
                // fabricators quote a flange's outside dimension
                flange_outside: true,
                hole_to_bend: at(&rules.hole_to_bend),
                hole_to_edge: at(&rules.hole_to_edge),
                min_slot: at(&rules.min_slot),
                min_hole: at(&rules.min_hole),
                bend_to_edge: at(&rules.bend_to_edge),
                min_web: at(&rules.min_web),
                angles: deg(rules.bend_angles),
                bend_ref: match rules.bend_from.as_deref() {
                    Some("centre") => fs::BendRef::Centre,
                    _ => fs::BendRef::Tangent,
                },
                bend_length_max: rules.max_bend_length.map(|v| v * u),
                not: m
                    .not
                    .iter()
                    .chain(&s.not)
                    .filter_map(|o| fs::Operation::from_name(o))
                    .collect(),
                relief,
                hardness: m.hardness,
            });
        }
        let named = |n: &String| {
            let x = forge_material::norm(n);
            forge_material::norm(&m.name) == x || m.aliases.iter().any(|a| forge_material::norm(a) == x)
        };
        let hardware = self
            .hardware
            .iter()
            .filter(|h| h.materials.is_empty() || h.materials.iter().any(named))
            .map(|h| fs::Hardware {
                kind: fs::HardwareKind::from_name(&h.kind).expect("a checked kind"),
                size: h.size.clone(),
                code: h.code.clone(),
                sheet_min: h.sheet.0.min(h.sheet.1) * u,
                sheet_max: h.sheet.0.max(h.sheet.1) * u,
                hole: h.hole * u,
                edge: h.edge * u,
                max_hardness: h.max_hardness.unwrap_or(f64::INFINITY),
                source: fs::Source::Cited(format!(
                    "{} hardware list{}",
                    self.name,
                    self.source.as_ref().map_or(String::new(), |s| format!(", {s}"))
                )),
            })
            .collect();
        let l = &self.limits;
        let operations = self.operations.as_ref().map(|ops| {
            ops.iter()
                .filter(|o| !m.not.contains(o))
                .filter_map(|o| fs::Operation::from_name(o))
                .collect()
        });
        fs::ShopProfile {
            stock,
            hardware,
            kerf,
            process: fs::Process {
                flat_max: mm2(l.max_flat),
                coated_max: mm2(l.max_flat_coated),
                bend_length_max: l.max_bend_length.map(|v| v * u),
                box_flange_max: l.max_box_flange.map(|v| v * u),
                box_flange_with_hardware: l.box_flange_when.as_deref() == Some("hardware"),
                angles: deg(l.bend_angles),
                part_min: mm2(l.min_flat),
                bent_max: mm2(l.max_flat_bent),
                bent_min: mm2(l.min_flat_bent),
            },
            operations,
            ..Default::default()
        }
    }
}

/// Rows by increasing first value (a table's r/t), whatever order they were written in.
fn sorted(rows: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut v = rows.to_vec();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

/// The index of the first value equal to one before it.
fn repeated(xs: impl Iterator<Item = f64>) -> Option<usize> {
    let xs: Vec<f64> = xs.collect();
    (1..xs.len()).find(|&k| {
        xs[..k]
            .iter()
            .any(|x| (x - xs[k]).abs() <= 1e-12 * xs[k].abs().max(1.0))
    })
}

/// Where a profile was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lib {
    Project,
    User,
    ForgeCad,
}

impl Lib {
    pub fn label(self) -> &'static str {
        match self {
            Lib::Project => "Project",
            Lib::User => "My vendors",
            Lib::ForgeCad => "ForgeCAD (read-only)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub lib: Lib,
    /// None: built in.
    pub file: Option<PathBuf>,
    pub vendor: Vendor,
}

/// The profiles a document sees, in search order, and the files that did not read.
#[derive(Clone, Debug, Default)]
pub struct Profiles {
    pub list: Vec<Entry>,
    pub errors: Vec<(PathBuf, ReadError)>,
}

/// The user's folder: `<config>/forgecad/vendors`.
pub fn user_dir() -> Option<PathBuf> {
    Some(forge_dirs::config()?.join("forgecad").join("vendors"))
}

/// A project's folder: `vendors/` beside its documents.
pub fn project_dir(doc_dir: &Path) -> PathBuf {
    doc_dir.join("vendors")
}

fn toml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// A regular file's text, at most 4 MB.
fn read_plain(p: &Path) -> Option<String> {
    let m = std::fs::metadata(p).ok()?;
    if !m.is_file() || m.len() > 4 << 20 {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

/// The built-in profiles.
pub fn builtin() -> &'static [Vendor] {
    static B: OnceLock<Vec<Vendor>> = OnceLock::new();
    B.get_or_init(|| {
        BUILTIN
            .iter()
            .map(|(f, s)| Vendor::parse(s).unwrap_or_else(|e| panic!("built-in {f}: {e}")))
            .collect()
    })
}

impl Profiles {
    pub fn builtin() -> Profiles {
        Profiles {
            list: builtin()
                .iter()
                .map(|v| Entry {
                    lib: Lib::ForgeCad,
                    file: None,
                    vendor: v.clone(),
                })
                .collect(),
            errors: vec![],
        }
    }

    /// The profiles for documents in `doc_dir` (none: the user's and the built-in ones).
    pub fn load(doc_dir: Option<&Path>) -> Profiles {
        let mut out = Profiles::default();
        for (lib, dir) in [(Lib::Project, doc_dir.map(project_dir)), (Lib::User, user_dir())] {
            let Some(dir) = dir else { continue };
            for f in toml_files(&dir) {
                let Some(text) = read_plain(&f) else { continue };
                match Vendor::parse(&text) {
                    Ok(vendor) => out.list.push(Entry {
                        lib,
                        file: Some(f),
                        vendor,
                    }),
                    Err(e) => out.errors.push((f, e)),
                }
            }
        }
        out.list.extend(Profiles::builtin().list);
        out
    }

    /// `load`, kept while the profile files are unchanged (their names and bytes).
    pub fn cached(doc_dir: Option<&Path>) -> Arc<Profiles> {
        type Key = (Option<PathBuf>, Vec<(PathBuf, Option<Vec<u8>>)>);
        type Cache = Mutex<Vec<(Key, Arc<Profiles>)>>;
        static CACHE: OnceLock<Cache> = OnceLock::new();
        let stamp = |d: Option<PathBuf>| {
            d.map(|d| toml_files(&d))
                .unwrap_or_default()
                .into_iter()
                .map(|f| {
                    let b = std::fs::read(&f).ok();
                    (f, b)
                })
                .collect::<Vec<_>>()
        };
        let mut files = stamp(doc_dir.map(project_dir));
        files.extend(stamp(user_dir()));
        let key: Key = (doc_dir.map(Path::to_path_buf), files);
        let cache = CACHE.get_or_init(Default::default);
        let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hit) = c.iter().find(|x| x.0 == key) {
            return hit.1.clone();
        }
        let p = Arc::new(Profiles::load(doc_dir));
        c.retain(|x| x.0.0 != key.0);
        c.push((key, p.clone()));
        p
    }

    /// A vendor by id (or by name), the project's first.
    pub fn find(&self, id: &str) -> Option<&Entry> {
        let n = forge_material::norm(id);
        self.list.iter().find(|e| e.vendor.id == id).or_else(|| {
            self.list
                .iter()
                .find(|e| forge_material::norm(&e.vendor.name) == n)
        })
    }

    /// The ids, each once, in search order.
    pub fn ids(&self) -> Vec<&str> {
        let mut out: Vec<&str> = vec![];
        for e in &self.list {
            if !out.contains(&e.vendor.id.as_str()) {
                out.push(&e.vendor.id);
            }
        }
        out
    }
}

/// Writes a profile to `file` (its folder made).
pub fn save(file: &Path, v: &Vendor) -> Result<(), String> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, v.to_text()).map_err(|e| e.to_string())
}

/// Reads a profile file and saves it into the user's folder as `<id>.toml`: where it went.
pub fn import(file: &Path) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let v = Vendor::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let dir = user_dir().ok_or("no config folder on this machine")?;
    let to = dir.join(format!("{}.toml", v.id));
    save(&to, &v)?;
    Ok(to)
}

#[cfg(test)]
mod tests;
