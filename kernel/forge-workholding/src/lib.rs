//! Workholding as a first-class part of CAM (research/cam/workholding.md): machine vises, soft
//! jaws, parallels, strap and toe clamps with step blocks, edge clamps, fixture plates with
//! dowels, zero-point systems and risers, self-centring and dovetail vises, tombstones, vacuum
//! tables and the user's own fixtures; on the lathe chucks, collets, face drivers, centres and
//! steady rests.
//!
//! A library record (`[[workholding]]`, TOML) holds a device's published sizes, each value cited
//! (`[[workholding.cite]]`: the fields, the source, the date it was read); a stand-in for a class
//! no maker publishes enough of says so (`standin = true`) and gives its `basis`. A size the maker does not publish is listed in `unknown`
//! and has no value: a body that needs it is refused with a typed error naming the field and the
//! fix (measure it, give it in the job), never filled in.
//!
//! A **placement** sizes a device from the stock and poses it in the part frame (z up from the
//! table, the tool coming down −z): its **bodies** (unions of blocks and cylinders, each with a
//! role and a keep-out) are what the certified checker keeps the tool from (G2), what the machine
//! simulation poses with the table, and what the leftover report calls "held". Lathe devices give
//! their envelope of revolution (lathe.rs).
pub mod cli;
pub mod generate;
pub mod geom;
pub mod lathe;
pub mod mill;
pub mod schema;

use forge_tooling::toml::{self, Table, Val, num, nums, tables, text};
pub use geom::{Pose, Prim, V3};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Vise,
    SelfCentring,
    Dovetail,
    SoftJaws,
    Parallels,
    StrapClamp,
    ToeClamp,
    StepBlock,
    EdgeClamp,
    FixturePlate,
    ZeroPoint,
    Riser,
    Tombstone,
    Vacuum,
    Custom,
    Chuck,
    Collet,
    FaceDriver,
    Centre,
    SteadyRest,
}

/// Every kind: its name in the TOML, the numbers a record of it must give (or list as unknown),
/// the numbers it may give.
pub(crate) const KINDS: &[(Kind, &str, &[&str], &[&str])] = &[
    (
        Kind::Vise,
        "vise",
        &[
            "jaw_width",
            "jaw_height",
            "jaw_thickness",
            "max_opening",
            "body_length",
            "body_width",
            "height",
        ],
        &["clamp_force", "weight", "min_opening"],
    ),
    (
        Kind::SelfCentring,
        "self_centring",
        &[
            "jaw_width",
            "jaw_height",
            "jaw_thickness",
            "max_opening",
            "body_length",
            "body_width",
            "height",
        ],
        &["min_opening", "clamp_force", "grip", "repeatability", "weight"],
    ),
    (
        Kind::Dovetail,
        "dovetail",
        &[
            "body_length",
            "body_width",
            "height",
            "grip",
            "angle",
            "min_width",
            "max_width",
        ],
        &["repeatability", "weight", "clamp_force"],
    ),
    (
        Kind::SoftJaws,
        "soft_jaws",
        &["width", "height", "thickness"],
        &[],
    ),
    (
        Kind::Parallels,
        "parallels",
        &["heights", "width", "length"],
        &["accuracy"],
    ),
    (
        Kind::StrapClamp,
        "strap_clamp",
        &["length", "width", "thickness", "stud_d", "stud_above"],
        &["slot", "reach"],
    ),
    (
        Kind::ToeClamp,
        "toe_clamp",
        &["body_length", "body_width", "height"],
        &["range_min", "range_max", "clamp_force", "screw"],
    ),
    (
        Kind::StepBlock,
        "step_block",
        &["step", "steps", "tread", "width"],
        &[],
    ),
    (
        Kind::EdgeClamp,
        "edge_clamp",
        &["body_length", "body_width", "height"],
        &["range_min", "range_max", "clamp_force", "torque"],
    ),
    (
        Kind::FixturePlate,
        "fixture_plate",
        &["length", "width", "thickness", "pitch", "hole_d", "hole_offset"],
        &["dowel_d", "dowel_h"],
    ),
    (
        Kind::ZeroPoint,
        "zero_point",
        &["pitch", "module_d", "module_h", "stud_d"],
        &["pull_force", "hold_force", "repeatability"],
    ),
    (Kind::Riser, "riser", &["d", "height"], &[]),
    (
        Kind::Tombstone,
        "tombstone",
        &["width", "height", "depth"],
        &["faces"],
    ),
    (
        Kind::Vacuum,
        "vacuum",
        &["length", "width", "thickness", "vacuum", "seal_inset"],
        &["flow"],
    ),
    (Kind::Custom, "custom", &[], &[]),
    (
        Kind::Chuck,
        "chuck",
        &[
            "jaws",
            "body_d",
            "body_len",
            "jaw_height",
            "jaw_len",
            "grip_min",
            "grip_max",
        ],
        &["max_rpm", "through_hole", "stroke", "grip_force"],
    ),
    (
        Kind::Collet,
        "collet",
        &["range_min", "range_max", "nose_d", "nose_len"],
        &["max_rpm", "collet_len", "collet_d"],
    ),
    (
        Kind::FaceDriver,
        "face_driver",
        &["body_d", "body_len", "range_min", "range_max"],
        &["pins"],
    ),
    (
        Kind::Centre,
        "centre",
        &["angle", "body_d", "point_len"],
        &["quill_d", "max_rpm"],
    ),
    (
        Kind::SteadyRest,
        "steady_rest",
        &["range_min", "range_max", "frame_r", "width"],
        &[],
    ),
];

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        KINDS.iter().find(|k| k.1 == s).map(|k| k.0)
    }
    pub fn name(self) -> &'static str {
        KINDS.iter().find(|k| k.0 == self).map(|k| k.1).unwrap_or("?")
    }
    fn fields(self) -> (&'static [&'static str], &'static [&'static str]) {
        let k = KINDS.iter().find(|k| k.0 == self).expect("every kind is listed");
        (k.2, k.3)
    }
    pub fn all() -> impl Iterator<Item = Kind> {
        KINDS.iter().map(|k| k.0)
    }
    pub fn is_lathe(self) -> bool {
        matches!(
            self,
            Kind::Chuck | Kind::Collet | Kind::FaceDriver | Kind::Centre | Kind::SteadyRest
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WhError {
    Toml {
        file: String,
        line: usize,
        what: String,
    },
    /// A record lacks a field its kind needs (neither a value nor listed as unknown).
    Missing {
        record: String,
        field: String,
        fix: String,
    },
    /// The maker does not publish the field and the job did not give it.
    Unpublished {
        record: String,
        field: String,
        fix: String,
    },
    Bad {
        record: String,
        field: String,
        why: String,
    },
    Unknown {
        kind: &'static str,
        name: String,
        near: Vec<String>,
    },
    Duplicate {
        name: String,
        files: [String; 2],
    },
    /// A placement that cannot hold the stock as asked: what, and the fix.
    Placement {
        device: String,
        what: String,
        fix: String,
    },
}

impl std::fmt::Display for WhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhError::Toml { file, line, what } => write!(f, "{file}:{line}: {what}"),
            WhError::Missing { record, field, fix } => write!(f, "{record}: missing `{field}` ({fix})"),
            WhError::Unpublished { record, field, fix } => {
                write!(f, "{record}: `{field}` is not published by the maker ({fix})")
            }
            WhError::Bad { record, field, why } => write!(f, "{record}: `{field}` {why}"),
            WhError::Unknown { kind, name, near } => {
                write!(f, "no {kind} `{name}`; near: {}", near.join(", "))
            }
            WhError::Duplicate { name, files } => {
                write!(f, "workholding `{name}` is in both {} and {}", files[0], files[1])
            }
            WhError::Placement { device, what, fix } => write!(f, "{device}: {what}: {fix}"),
        }
    }
}

/// Where a value came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    pub fields: Vec<String>,
    pub source: String,
    pub taken: String,
}

/// A body given by a custom record, in the device's frame (its seat at z = 0).
#[derive(Clone, Debug, PartialEq)]
pub struct BodySpec {
    pub name: String,
    pub role: Role,
    pub prims: Vec<Prim>,
    pub part: Option<String>,
}

/// One library record.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub name: String,
    pub kind: Kind,
    pub vendor: String,
    pub product: String,
    /// A stand-in for a class no maker publishes enough of: `source` holds its basis (which
    /// published values it follows, which numbers are ours) and covers every value.
    pub standin: bool,
    pub vals: BTreeMap<String, f64>,
    pub lists: BTreeMap<String, Vec<f64>>,
    pub words: BTreeMap<String, String>,
    pub unknown: Vec<String>,
    /// Fields whose published values disagree (each said in `note`): no value, as unknown.
    pub conflict: Vec<String>,
    pub cites: Vec<Cite>,
    pub source: String,
    pub taken: String,
    pub note: String,
    /// Real parts by body name (`.fg`, relative to the library's parts directory).
    pub parts: BTreeMap<String, String>,
    pub bodies: Vec<BodySpec>,
}

const WORDS: &[&str] = &[
    "screw",
    "system",
    "mount",
    "interface",
    "material",
    "jaw_kind",
    "taper",
];

fn bad(rec: &str, field: &str, why: impl Into<String>) -> WhError {
    WhError::Bad {
        record: rec.to_string(),
        field: field.to_string(),
        why: why.into(),
    }
}

fn v3(t: &Table, rec: &str, k: &str) -> Result<V3, WhError> {
    let v = nums(t, k)
        .filter(|v| v.len() == 3 && v.iter().all(|x| x.is_finite()))
        .ok_or_else(|| bad(rec, k, "must be three numbers"))?;
    Ok([v[0], v[1], v[2]])
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        ROLES.iter().find(|r| r.1 == s).map(|r| r.0)
    }
    pub fn name(self) -> &'static str {
        ROLES.iter().find(|r| r.0 == self).map(|r| r.1).unwrap_or("?")
    }
}

/// Fields that may be 0 (a range from zero, a hole on the plate's corner); every other number
/// is > 0.
pub const ZERO_OK: &[&str] = &["range_min", "min_opening", "hole_offset", "grip_min"];

fn value_ok(k: &str, x: f64) -> bool {
    x.is_finite() && (x > 0.0 || (x == 0.0 && ZERO_OK.contains(&k)))
}

fn bound(k: &str) -> &'static str {
    if ZERO_OK.contains(&k) { "≥ 0" } else { "> 0" }
}

/// What a body does (for the reply, the renderer's colours, and "held").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Base,
    FixedJaw,
    MovingJaw,
    Jaw,
    Parallel,
    Clamp,
    Stud,
    StepBlock,
    Plate,
    Dowel,
    Module,
    Riser,
    Column,
    ChuckBody,
    Collet,
    Centre,
    Rest,
    Other,
}

const ROLES: &[(Role, &str)] = &[
    (Role::Base, "base"),
    (Role::FixedJaw, "fixed_jaw"),
    (Role::MovingJaw, "moving_jaw"),
    (Role::Jaw, "jaw"),
    (Role::Parallel, "parallel"),
    (Role::Clamp, "clamp"),
    (Role::Stud, "stud"),
    (Role::StepBlock, "step_block"),
    (Role::Plate, "plate"),
    (Role::Dowel, "dowel"),
    (Role::Module, "module"),
    (Role::Riser, "riser"),
    (Role::Column, "column"),
    (Role::ChuckBody, "chuck_body"),
    (Role::Collet, "collet"),
    (Role::Centre, "centre"),
    (Role::Rest, "rest"),
    (Role::Other, "other"),
];

impl Record {
    pub fn from_table(t: &Table) -> Result<Record, WhError> {
        let name = text(t, "name")
            .ok_or_else(|| WhError::Missing {
                record: "[[workholding]]".into(),
                field: "name".into(),
                fix: "give `name = \"...\"`".into(),
            })?
            .to_string();
        let rec = format!("workholding `{name}`");
        let ks = text(t, "kind").ok_or_else(|| WhError::Missing {
            record: rec.clone(),
            field: "kind".into(),
            fix: format!(
                "give `kind = \"vise\"` (or {})",
                KINDS.iter().map(|k| k.1).collect::<Vec<_>>().join(", ")
            ),
        })?;
        let kind =
            Kind::parse(ks).ok_or_else(|| bad(&rec, "kind", format!("`{ks}` is not a workholding kind")))?;
        let (need, may) = kind.fields();
        let unknown = toml::texts(t, "unknown").unwrap_or_default();
        let conflict = toml::texts(t, "conflict").unwrap_or_default();
        let mut vals = BTreeMap::new();
        let mut lists = BTreeMap::new();
        let mut words = BTreeMap::new();
        let skip = [
            "name", "kind", "vendor", "product", "standin", "basis", "unknown", "conflict", "source",
            "taken", "note", "cite", "part", "body",
        ];
        for (k, v) in t {
            if skip.contains(&k.as_str()) {
                continue;
            }
            let known = need.contains(&k.as_str()) || may.contains(&k.as_str());
            match v {
                Val::Num(x) if known => {
                    if !value_ok(k, *x) {
                        return Err(bad(&rec, k, format!("must be {} (is {x})", bound(k))));
                    }
                    vals.insert(k.clone(), *x);
                }
                Val::Arr(_) if known => {
                    let xs = nums(t, k).ok_or_else(|| bad(&rec, k, "must be numbers"))?;
                    if xs.is_empty() || xs.iter().any(|x| !value_ok(k, *x)) {
                        return Err(bad(&rec, k, format!("must be numbers {}", bound(k))));
                    }
                    lists.insert(k.clone(), xs);
                }
                Val::Str(s) if WORDS.contains(&k.as_str()) => {
                    words.insert(k.clone(), s.clone());
                }
                _ => {
                    return Err(bad(
                        &rec,
                        k,
                        format!(
                            "is not a field of a {} (fields: {})",
                            kind.name(),
                            [need, may].concat().join(", ")
                        ),
                    ));
                }
            }
        }
        for u in unknown.iter().chain(&conflict) {
            if vals.contains_key(u) || lists.contains_key(u) {
                return Err(bad(&rec, u, "has a value and is listed as unknown"));
            }
            if !need.contains(&u.as_str()) && !may.contains(&u.as_str()) {
                return Err(bad(
                    &rec,
                    u,
                    format!("is listed as unknown but is not a field of a {}", kind.name()),
                ));
            }
        }
        for f in need {
            if !vals.contains_key(*f)
                && !lists.contains_key(*f)
                && !unknown.iter().chain(&conflict).any(|u| u == f)
            {
                return Err(WhError::Missing {
                    record: rec.clone(),
                    field: f.to_string(),
                    fix: format!("give `{f}`, or list it in `unknown` if the maker does not publish it"),
                });
            }
        }
        let mut cites = Vec::new();
        for c in tables(t, "cite") {
            let fields = toml::texts(c, "fields").unwrap_or_default();
            let source = text(c, "source").unwrap_or("").to_string();
            let taken = text(c, "taken").unwrap_or("").to_string();
            // a user's value carries provenance "user" and needs no date read
            let user =
                source.starts_with("user") || source.starts_with("stand-in") || source == "given in the job";
            if fields.is_empty() || source.is_empty() || (taken.is_empty() && !user) {
                return Err(bad(
                    &rec,
                    "cite",
                    "each cite gives `fields`, `source` and `taken` (the date it was read; a user's own values: source = \"user\")",
                ));
            }
            cites.push(Cite {
                fields,
                source,
                taken,
            });
        }
        let standin = toml::boolean(t, "standin").unwrap_or(false);
        let source = text(t, if standin { "basis" } else { "source" })
            .unwrap_or("")
            .to_string();
        if standin && source.is_empty() {
            return Err(WhError::Missing {
                record: rec,
                field: "basis".into(),
                fix: "a stand-in says which published values it follows and which numbers are ours".into(),
            });
        }
        if !standin {
            // every value of a maker's product is cited
            for f in vals.keys().chain(lists.keys()) {
                if !cites.iter().any(|c| c.fields.iter().any(|x| x == f)) {
                    return Err(bad(
                        &rec,
                        f,
                        "has no [[workholding.cite]] (every value of a product is cited)",
                    ));
                }
            }
        }
        let mut parts = BTreeMap::new();
        for p in tables(t, "part") {
            let b = text(p, "body").ok_or_else(|| bad(&rec, "part", "gives `body`"))?;
            let f = text(p, "file").ok_or_else(|| bad(&rec, "part", "gives `file`"))?;
            parts.insert(b.to_string(), f.to_string());
        }
        let mut bodies = Vec::new();
        for b in tables(t, "body") {
            let bn = text(b, "name")
                .ok_or_else(|| bad(&rec, "body", "gives `name`"))?
                .to_string();
            let role = match text(b, "role") {
                None => Role::Other,
                Some(r) => {
                    Role::parse(r).ok_or_else(|| bad(&rec, "body.role", format!("`{r}` is not a role")))?
                }
            };
            let mut prims = Vec::new();
            for x in tables(b, "box") {
                let (lo, hi) = (v3(x, &rec, "min")?, v3(x, &rec, "max")?);
                if (0..3).any(|k| lo[k] >= hi[k]) {
                    return Err(bad(&rec, "body.box", "min < max on every axis"));
                }
                prims.push(Prim::aabb(lo, hi));
            }
            for x in tables(b, "cyl") {
                let base = v3(x, &rec, "base")?;
                let axis = v3(x, &rec, "axis")?;
                let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
                let r = num(x, "r")
                    .filter(|r| *r > 0.0)
                    .ok_or_else(|| bad(&rec, "body.cyl", "`r` > 0"))?;
                let h = num(x, "h")
                    .filter(|h| *h > 0.0)
                    .ok_or_else(|| bad(&rec, "body.cyl", "`h` > 0"))?;
                if !n.is_finite() || n <= 0.0 {
                    return Err(bad(&rec, "body.cyl", "`axis` is not zero"));
                }
                prims.push(Prim::Cyl {
                    base,
                    axis: [axis[0] / n, axis[1] / n, axis[2] / n],
                    r,
                    h,
                });
            }
            if prims.is_empty() {
                return Err(bad(&rec, "body", format!("`{bn}` has no [[..box]] or [[..cyl]]")));
            }
            bodies.push(BodySpec {
                part: text(b, "part").map(str::to_string),
                name: bn,
                role,
                prims,
            });
        }
        if kind == Kind::Custom && bodies.is_empty() {
            return Err(WhError::Missing {
                record: rec,
                field: "body".into(),
                fix: "a custom fixture gives its bodies: [[workholding.body]] with [[workholding.body.box]] min/max (its seat at z = 0)".into(),
            });
        }
        let s = |k: &str| text(t, k).unwrap_or("").to_string();
        Ok(Record {
            vendor: s("vendor"),
            product: s("product"),
            standin,
            vals,
            lists,
            words,
            unknown,
            conflict,
            cites,
            source,
            taken: s("taken"),
            note: s("note"),
            parts,
            bodies,
            kind,
            name,
        })
    }

    fn label(&self) -> String {
        format!("{} `{}`", self.kind.name(), self.name)
    }

    /// A number the device needs: its value, or a typed error (unknown: not published; absent).
    pub fn get(&self, k: &str) -> Result<f64, WhError> {
        if let Some(v) = self.vals.get(k) {
            return Ok(*v);
        }
        if self.conflict.iter().any(|u| u == k) {
            return Err(WhError::Unpublished {
                record: self.label(),
                field: k.to_string(),
                fix: format!(
                    "its published values disagree (see the record's note): measure it and give `{k}` with the device"
                ),
            });
        }
        if self.unknown.iter().any(|u| u == k) {
            return Err(WhError::Unpublished {
                record: self.label(),
                field: k.to_string(),
                fix: format!("measure it and give `{k}` with the device in the job (or a project record)"),
            });
        }
        Err(WhError::Missing {
            record: self.label(),
            field: k.to_string(),
            fix: format!("give `{k}`"),
        })
    }

    pub fn list(&self, k: &str) -> Result<&[f64], WhError> {
        if let Some(v) = self.lists.get(k) {
            return Ok(v);
        }
        self.get(k).map(|_| &[][..])
    }

    pub fn opt(&self, k: &str) -> Option<f64> {
        self.vals.get(k).copied()
    }

    /// The record with a value the job gives (a measured size, or a published one corrected).
    pub fn with(&self, k: &str, v: f64) -> Result<Record, WhError> {
        let (need, may) = self.kind.fields();
        if !need.contains(&k) && !may.contains(&k) {
            return Err(bad(
                &self.label(),
                k,
                format!("is not a field of a {}", self.kind.name()),
            ));
        }
        if !value_ok(k, v) {
            return Err(bad(&self.label(), k, format!("must be {} (is {v})", bound(k))));
        }
        let mut r = self.clone();
        r.unknown.retain(|u| u != k);
        r.conflict.retain(|u| u != k);
        r.vals.insert(k.to_string(), v);
        // the given value replaces the cited one: no older cite may claim it
        for c in &mut r.cites {
            c.fields.retain(|f| f != k);
        }
        r.cites.retain(|c| !c.fields.is_empty());
        r.cites.push(Cite {
            fields: vec![k.to_string()],
            source: "given in the job".into(),
            taken: String::new(),
        });
        Ok(r)
    }

    /// The record with a list the user gives (a parallel set's heights), provenance user.
    pub fn with_list(&self, k: &str, v: Vec<f64>) -> Result<Record, WhError> {
        let (need, may) = self.kind.fields();
        if !need.contains(&k) && !may.contains(&k) {
            return Err(bad(
                &self.label(),
                k,
                format!("is not a field of a {}", self.kind.name()),
            ));
        }
        if v.is_empty() || v.iter().any(|x| !value_ok(k, *x)) {
            return Err(bad(&self.label(), k, format!("must be numbers {}", bound(k))));
        }
        let mut r = self.clone();
        r.unknown.retain(|u| u != k);
        r.conflict.retain(|u| u != k);
        r.vals.remove(k);
        r.lists.insert(k.to_string(), v);
        r.user_cite(k);
        Ok(r)
    }

    /// The record with a word the user gives (a screw, a collet system).
    pub fn with_word(&self, k: &str, v: &str) -> Result<Record, WhError> {
        if !WORDS.contains(&k) {
            return Err(bad(
                &self.label(),
                k,
                format!("is not a word field ({})", WORDS.join(", ")),
            ));
        }
        let mut r = self.clone();
        if v.is_empty() {
            r.words.remove(k);
        } else {
            r.words.insert(k.to_string(), v.to_string());
        }
        Ok(r)
    }

    fn user_cite(&mut self, k: &str) {
        for c in &mut self.cites {
            c.fields.retain(|f| f != k);
        }
        self.cites.retain(|c| !c.fields.is_empty());
        self.cites.push(Cite {
            fields: vec![k.to_string()],
            source: "user".into(),
            taken: String::new(),
        });
    }

    /// The citation of a field (a stand-in's basis for a stand-in).
    pub fn cite(&self, k: &str) -> Option<String> {
        if self.standin {
            return Some(self.source.clone());
        }
        self.cites
            .iter()
            .find(|c| c.fields.iter().any(|f| f == k))
            .map(|c| format!("{} (read {})", c.source, c.taken))
    }
}

/// The library: records from files in search order (project, then the user's, then built-in).
#[derive(Clone, Debug, Default)]
pub struct Library {
    /// (record, the file it came from), the first of a name wins.
    pub records: Vec<(Record, String)>,
}

pub const BUILTIN: [(&str, &str); 4] = [
    (
        "builtin/workholding/vendors.toml",
        include_str!("../data/vendors.toml"),
    ),
    (
        "builtin/workholding/products.toml",
        include_str!("../data/products.toml"),
    ),
    (
        "builtin/workholding/lathe.toml",
        include_str!("../data/lathe.toml"),
    ),
    (
        "builtin/workholding/standins.toml",
        include_str!("../data/standins.toml"),
    ),
];

/// Records from one file.
pub fn parse_file(file: &str, src: &str) -> Result<Vec<Record>, WhError> {
    let t = toml::parse(src).map_err(|e| WhError::Toml {
        file: file.to_string(),
        line: e.line,
        what: e.what,
    })?;
    tables(&t, "workholding").iter().map(Record::from_table).collect()
}

impl Library {
    /// The built-in set alone.
    pub fn builtin() -> Library {
        let mut l = Library::default();
        let files: Vec<(String, String)> = BUILTIN
            .iter()
            .map(|(f, s)| (f.to_string(), s.to_string()))
            .collect();
        l.add_level(&files)
            .expect("the built-in workholding library reads");
        l
    }

    /// Adds a level above what is loaded (call the user's, then the project's): a name in it hides
    /// the same name below; a name twice within one level is an error.
    pub fn add_level(&mut self, files: &[(String, String)]) -> Result<(), WhError> {
        let mut level: Vec<(Record, String)> = Vec::new();
        for (file, src) in files {
            for r in parse_file(file, src)? {
                if let Some((_, f)) = level.iter().find(|(x, _)| x.name == r.name) {
                    return Err(WhError::Duplicate {
                        name: r.name,
                        files: [f.clone(), file.clone()],
                    });
                }
                level.push((r, file.clone()));
            }
        }
        self.records
            .retain(|(r, _)| !level.iter().any(|(x, _)| x.name == r.name));
        level.append(&mut self.records);
        self.records = level;
        Ok(())
    }

    /// Reads a level from a directory's `*.toml` (sorted by name; a missing directory is empty).
    pub fn add_dir(&mut self, dir: &std::path::Path) -> Result<(), WhError> {
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut paths: Vec<_> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .collect();
            paths.sort();
            for p in paths {
                let s = std::fs::read_to_string(&p).map_err(|e| WhError::Toml {
                    file: p.display().to_string(),
                    line: 0,
                    what: e.to_string(),
                })?;
                files.push((p.display().to_string(), s));
            }
        }
        self.add_level(&files)
    }

    /// The built-in set, then the user's `<config>/cam/workholding`, then the project's
    /// `<project>/cam/workholding` on top.
    pub fn search(
        user: Option<&std::path::Path>,
        project: Option<&std::path::Path>,
    ) -> Result<Library, WhError> {
        let mut l = Library::builtin();
        for d in [user, project].into_iter().flatten() {
            l.add_dir(&d.join("cam").join("workholding"))?;
        }
        Ok(l)
    }

    pub fn get(&self, name: &str) -> Result<&Record, WhError> {
        self.records
            .iter()
            .map(|(r, _)| r)
            .find(|r| r.name == name)
            .ok_or_else(|| WhError::Unknown {
                kind: "workholding",
                name: name.to_string(),
                near: near(name, self.records.iter().map(|(r, _)| r.name.as_str())),
            })
    }

    pub fn of_kind(&self, k: Kind) -> impl Iterator<Item = &Record> {
        self.records.iter().map(|(r, _)| r).filter(move |r| r.kind == k)
    }
}

fn near<'a>(name: &str, all: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut v: Vec<(usize, &str)> = all.map(|a| (edit_distance(name, a), a)).collect();
    v.sort();
    v.into_iter().take(3).map(|(_, a)| a.to_string()).collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1)
                .min(cur[j - 1] + 1)
                .min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// A body of a placed device, in the part frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub name: String,
    pub role: Role,
    pub prims: Vec<Prim>,
    /// The tool keeps this much further away than the fixture clearance (a stud's thread, a
    /// clamp's screw head).
    pub keepout: f64,
    pub part: Option<String>,
}

impl Body {
    pub fn new(name: impl Into<String>, role: Role, prims: Vec<Prim>) -> Body {
        Body {
            name: name.into(),
            role,
            prims,
            keepout: 0.0,
            part: None,
        }
    }
    pub fn bounds(&self) -> (V3, V3) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &self.prims {
            let (a, b) = p.grown(self.keepout).bounds();
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(b[k]);
            }
        }
        (lo, hi)
    }
}

/// The stock as placed: an axis-aligned box in the part frame (z up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: V3,
    pub max: V3,
}

impl Box3 {
    pub fn size(&self) -> V3 {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
    pub fn mid(&self) -> V3 {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }
}

/// How a device sits on the machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Mount {
    /// On the table (T-slots), its underside at z.
    Table { z: f64 },
    /// On the device below it (a riser, a zero-point module, a plate).
    On { below: String, z: f64 },
    /// The lathe spindle's nose.
    Spindle,
}

/// A device sized from the stock and posed in the part frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub name: String,
    pub kind: Kind,
    pub bodies: Vec<Body>,
    pub mount: Mount,
    /// Boxes (min, max) of stock the device holds: no op may reach in, and the leftover there
    /// is "held".
    pub held: Vec<(V3, V3)>,
    /// What the placement chose or found (the reply's text).
    pub notes: Vec<String>,
}

impl Placed {
    /// The lowest z of its bodies (what the next device down carries).
    pub fn bottom(&self) -> f64 {
        self.bodies
            .iter()
            .map(|b| b.bounds().0[2])
            .fold(f64::INFINITY, f64::min)
    }
}

/// A setup's workholding: every placed device.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Workholding {
    pub placed: Vec<Placed>,
}

/// Face ids: 3000 + 1000·(body index) + (face within the body), as forge-cam's fixture does.
pub const FACE_ID0: u32 = 3000;

impl Workholding {
    pub fn bodies(&self) -> impl Iterator<Item = (&Placed, &Body)> {
        self.placed
            .iter()
            .flat_map(|p| p.bodies.iter().map(move |b| (p, b)))
    }

    /// Every body's faces for the certified checker (keep-outs applied), and the name of the
    /// body each id range belongs to.
    pub fn faces(&self, mu: f64) -> Vec<forge_cam_geom::check::Face> {
        let mut out = Vec::new();
        for (i, (_, b)) in self.bodies().enumerate() {
            let mut id = FACE_ID0 + 1000 * i as u32;
            for p in &b.prims {
                let fs = p.grown(b.keepout).faces(id, mu);
                id += fs.len() as u32;
                out.extend(fs);
            }
        }
        out
    }

    /// The device and body a face id names.
    pub fn body_of(&self, id: u32) -> Option<(&str, &str)> {
        let k = (id.checked_sub(FACE_ID0)? / 1000) as usize;
        self.bodies()
            .nth(k)
            .map(|(p, b)| (p.name.as_str(), b.name.as_str()))
    }

    /// Every primitive in the part frame with its device and body name, keep-outs applied (what
    /// the machine simulation poses with the table).
    pub fn prims(&self) -> Vec<(String, Prim)> {
        let mut v = Vec::new();
        for (p, b) in self.bodies() {
            for x in &b.prims {
                v.push((format!("{}/{}", p.name, b.name), x.grown(b.keepout)));
            }
        }
        v
    }

    /// One collision body per device body, named "device/body", its prims in the part frame
    /// with keep-outs applied (forge-machine's `SetupSpec.fixture`: a `Fixture {name, prims}`
    /// each). A block whose axes are not the frame's is given as is (`Prim::Block` with axes);
    /// `aligned` turns each into its axis-aligned bound instead (a superset: safe for a clearance
    /// check, it may report contact the exact block would not).
    pub fn fixtures(&self, aligned: bool) -> Vec<(String, Vec<Prim>)> {
        self.bodies()
            .map(|(p, b)| {
                let prims = b
                    .prims
                    .iter()
                    .map(|x| {
                        let g = x.grown(b.keepout);
                        match g {
                            Prim::Block { axes, .. } if aligned && axes != geom::IDENT => {
                                let (lo, hi) = g.bounds();
                                Prim::aabb(lo, hi)
                            }
                            _ => g,
                        }
                    })
                    .collect();
                (format!("{}/{}", p.name, b.name), prims)
            })
            .collect()
    }

    /// Whether a box of leftover stock (min, max) is held: inside a device's held box grown by
    /// `slack` (the fixture clearance: a tool kept that far from the device cannot reach it).
    pub fn held(&self, lo: V3, hi: V3, slack: f64) -> Option<&str> {
        self.placed
            .iter()
            .find(|p| {
                p.held
                    .iter()
                    .any(|(a, b)| (0..3).all(|k| lo[k] >= a[k] - slack && hi[k] <= b[k] + slack))
            })
            .map(|p| p.name.as_str())
    }

    /// The reach report of a path (3-axis, tool along −z): the least stick-out (tip to the
    /// holder's nose) at which a holder of radius `holder_r` keeps `clearance` from every body,
    /// over the tips `tips`, with the body and tip that set it. Conservative: a body is taken
    /// as its bounding box (a larger body needs at least as much stick-out). None: no body near.
    pub fn reach(&self, tips: &[V3], holder_r: f64, clearance: f64) -> Option<Reach> {
        let mut best: Option<Reach> = None;
        for (p, b) in self.bodies() {
            let (lo, hi) = b.bounds();
            for t in tips {
                let d = geom::xy_dist([t[0], t[1]], [lo[0], lo[1]], [hi[0], hi[1]]);
                if d >= holder_r + clearance {
                    continue;
                }
                let need = hi[2] + clearance - t[2];
                if best.as_ref().is_none_or(|r| need > r.stickout) {
                    best = Some(Reach {
                        stickout: need,
                        body: format!("{}/{}", p.name, b.name),
                        at: *t,
                    });
                }
            }
        }
        best
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reach {
    pub stickout: f64,
    pub body: String,
    pub at: V3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_reads_and_every_record_validates() {
        let l = Library::builtin();
        assert!(l.records.len() >= 20, "{}", l.records.len());
        for k in Kind::all() {
            assert!(l.of_kind(k).count() >= 1, "no built-in {}", k.name());
        }
        for (r, f) in &l.records {
            assert!(!r.name.is_empty(), "{f}");
            if !r.standin {
                assert!(
                    !r.vendor.is_empty() && !r.cites.is_empty(),
                    "{}: a product names its maker and sources",
                    r.name
                );
                for c in &r.cites {
                    assert!(
                        c.source.starts_with("http"),
                        "{}: cite a public source ({})",
                        r.name,
                        c.source
                    );
                    assert_eq!(c.taken.len(), 10, "{}: the date it was read, YYYY-MM-DD", r.name);
                }
            }
            for k in r.vals.keys() {
                assert!(r.cite(k).is_some(), "{} {k}", r.name);
            }
        }
    }

    #[test]
    fn levels_hide_lower_and_refuse_twice() {
        let mut l = Library::builtin();
        let mine = "[[workholding]]\nname = \"standin-vise-150\"\nkind = \"riser\"\nstandin = true\nbasis = \"mine\"\nd = 50\nheight = 100\n";
        l.add_level(&[("user/a.toml".into(), mine.into())]).unwrap();
        assert_eq!(l.get("standin-vise-150").unwrap().kind, Kind::Riser);
        let e = l
            .add_level(&[("p/a.toml".into(), mine.into()), ("p/b.toml".into(), mine.into())])
            .unwrap_err();
        assert!(matches!(e, WhError::Duplicate { .. }), "{e:?}");
        assert!(
            matches!(l.get("standin-vise-15"), Err(WhError::Unknown { near, .. }) if near.contains(&"standin-vise-150".to_string()))
        );
    }

    #[test]
    fn records_refuse_what_they_cannot_hold() {
        let p = |s: &str| parse_file("t.toml", s);
        // a field missing, neither given nor unknown
        let e = p("[[workholding]]\nname = \"r\"\nkind = \"riser\"\nstandin = true\nbasis = \"s\"\nd = 5\n")
            .unwrap_err();
        assert!(
            matches!(&e, WhError::Missing { field, .. } if field == "height"),
            "{e:?}"
        );
        // a product's value without a cite
        let e = p("[[workholding]]\nname = \"r\"\nkind = \"riser\"\nvendor = \"v\"\nd = 5\nheight = 9\n[[workholding.cite]]\nfields = [\"d\"]\nsource = \"https://x\"\ntaken = \"2026-11-23\"\n").unwrap_err();
        assert!(
            matches!(&e, WhError::Bad { field, .. } if field == "height"),
            "{e:?}"
        );
        // unknown is not a value: using it is refused, giving it in the job works
        let r = p("[[workholding]]\nname = \"r\"\nkind = \"riser\"\nvendor = \"v\"\nunknown = [\"height\"]\nd = 5\n[[workholding.cite]]\nfields = [\"d\"]\nsource = \"https://x\"\ntaken = \"2026-11-23\"\n").unwrap();
        assert!(matches!(r[0].get("height"), Err(WhError::Unpublished { .. })));
        assert_eq!(r[0].with("height", 80.0).unwrap().get("height"), Ok(80.0));
        // a field that is not the kind's
        let e = p("[[workholding]]\nname = \"r\"\nkind = \"riser\"\nstandin = true\nbasis = \"s\"\nd = 5\nheight = 9\njaw_width = 3\n").unwrap_err();
        assert!(
            matches!(&e, WhError::Bad { field, .. } if field == "jaw_width"),
            "{e:?}"
        );
        // 0 only where it means something (a range from zero)
        let z = p("[[workholding]]\nname = \"c\"\nkind = \"collet\"\nstandin = true\nbasis = \"s\"\nrange_min = 0\nrange_max = 9\nnose_d = 20\nnose_len = 10\n").unwrap();
        assert_eq!(z[0].get("range_min"), Ok(0.0));
        assert!(z[0].with("nose_d", 0.0).is_err());
        let h = Library::builtin()
            .get("standin-parallels")
            .unwrap()
            .with_list("heights", vec![12.7, 25.4])
            .unwrap();
        assert_eq!(h.list("heights").unwrap(), &[12.7, 25.4]);
        assert!(h.cite("heights").is_some());
        assert!(h.with_word("screw", "M8").unwrap().words["screw"] == "M8");
        assert!(h.with_word("nope", "x").is_err());
        let e = p("[[workholding]]\nname = \"r\"\nkind = \"riser\"\nstandin = true\nbasis = \"s\"\nd = -5\nheight = 9\n").unwrap_err();
        assert!(matches!(&e, WhError::Bad { field, .. } if field == "d"), "{e:?}");
    }
}
