//! The tooling and press-brake library: real makers' punches, dies and presses from their openly
//! published catalogues and specs, every value cited (research/bendsim/environment.md §1-2).
//!
//! Files are our TOML subset (forge_vendor::toml), mm, kN and degrees:
//! `[[punch]]`, `[[die]]` and `[[press]]` records, each with `[[punch.cite]]` (etc.) rows in
//! forge-machine's provenance format (docs/cam-machine.md, Provenance): `path`, `status`
//! (`published`, `estimate`, `unknown`, `conflict`), `source` + `taken` (published, conflict),
//! `value` (the source's statement, restated), `basis` (estimate, unknown), `note` (conflict).
//!
//! A record with `real = true` cites every value it holds. A value a record does not hold is
//! UNKNOWN: a tool whose working geometry is unknown is listed but refused
//! ([`ToolingError::Unknown`]); a press value that is unknown makes the rules needing it report
//! "not checked" ([`Press::unchecked`]), and a press whose geometry is unknown is refused too.
//! Search order as forge-machine's: a project's `bend/tooling/*.toml`, then the user's, then the
//! built-in files; an id found earlier hides a later one.
use crate::tooling::{
    Backgauge, Beam, Die, Finger, Frame, Press, Punch, PunchKind, Relief, Segment, Style, Tooling,
    ToolingError,
};
use forge_math::dmath;
use forge_vendor::toml::{self, Item, Table};

/// Built-in files: (name, text).
pub const BUILTIN: &[(&str, &str)] = &[
    ("builtin/trumpf.toml", include_str!("../data/tooling/trumpf.toml")),
    ("builtin/amada.toml", include_str!("../data/tooling/amada.toml")),
    (
        "builtin/bystronic.toml",
        include_str!("../data/tooling/bystronic.toml"),
    ),
    ("builtin/wila.toml", include_str!("../data/tooling/wila.toml")),
    ("builtin/wilson.toml", include_str!("../data/tooling/wilson.toml")),
    (
        "builtin/rolleri.toml",
        include_str!("../data/tooling/rolleri.toml"),
    ),
    ("builtin/mate.toml", include_str!("../data/tooling/mate.toml")),
    (
        "builtin/presses.toml",
        include_str!("../data/presses/presses.toml"),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Published,
    Estimate,
    Unknown,
    Conflict,
    /// The user's own value (entered in the library builder): not a maker's number; its basis
    /// says so. A real record holds none.
    User,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Published => "published",
            Status::Estimate => "estimate",
            Status::Unknown => "unknown",
            Status::Conflict => "conflict",
            Status::User => "user",
        }
    }

    fn parse(s: &str) -> Option<Status> {
        Some(match s {
            "published" => Status::Published,
            "estimate" => Status::Estimate,
            "unknown" => Status::Unknown,
            "conflict" => Status::Conflict,
            "user" => Status::User,
            _ => return None,
        })
    }
}

/// Where one value comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    pub path: String,
    pub status: Status,
    pub source: Option<String>,
    pub taken: Option<String>,
    pub value: Option<String>,
    pub basis: Option<String>,
    pub note: Option<String>,
}

/// A record's maker, name and provenance.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Prov {
    pub maker: String,
    pub name: String,
    /// A real maker's product: every value cited.
    pub real: bool,
    pub note: Option<String>,
    /// The file it was read from.
    pub file: String,
    pub cites: Vec<Cite>,
}

impl Prov {
    /// The cites of a path (several when sources agree or conflict).
    pub fn of(&self, path: &str) -> Vec<&Cite> {
        self.cites.iter().filter(|c| c.path == path).collect()
    }

    /// The weakest status among a path's cites (conflict < unknown < user < estimate <
    /// published), or None if uncited.
    pub fn status(&self, path: &str) -> Option<Status> {
        let rank = |s: Status| match s {
            Status::Conflict => 0,
            Status::Unknown => 1,
            Status::User => 2,
            Status::Estimate => 3,
            Status::Published => 4,
        };
        self.of(path)
            .into_iter()
            .map(|c| c.status)
            .min_by_key(|s| rank(*s))
    }
}

/// The clamping system a tool is made for (a press takes the tools of its own).
pub type Clamp = String;

#[derive(Clone, Debug, PartialEq)]
pub struct PunchRec {
    pub id: String,
    pub prov: Prov,
    pub clamp: Clamp,
    pub kind: PunchKind,
    /// The punch, or the working fields that are unknown.
    pub punch: Result<Punch, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DieRec {
    pub id: String,
    pub prov: Prov,
    pub clamp: Clamp,
    pub die: Result<Die, Vec<String>>,
}

/// Ram speeds (mm/s): the animation's timing; not checked.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Speeds {
    pub approach: Option<f64>,
    pub work: Option<f64>,
    pub ret: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PressRec {
    pub id: String,
    pub prov: Prov,
    pub model: String,
    pub clamp: Clamp,
    /// Backgauge axes the maker lists (X, R, Z1, Z2, X1, X2, ...).
    pub axes: Vec<String>,
    pub speeds: Speeds,
    /// Working-table height above the floor (the scene's floor), when published.
    pub table_height: Option<f64>,
    /// The press, or the geometry fields that are unknown.
    pub press: Result<Press, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CatalogError {
    Toml {
        file: String,
        line: usize,
        why: String,
    },
    /// A record's field is malformed (`what` says how).
    Field {
        file: String,
        id: String,
        field: String,
        what: String,
    },
    /// A cite row that is malformed, names nothing the record holds, or a held value of a real
    /// record that no row cites.
    Cite {
        file: String,
        id: String,
        path: String,
        what: String,
    },
    Duplicate {
        file: String,
        id: String,
    },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::Toml { file, line, why } => write!(f, "{file}:{line}: {why}"),
            CatalogError::Field {
                file,
                id,
                field,
                what,
            } => write!(f, "{file}: {id}: {field}: {what}"),
            CatalogError::Cite { file, id, path, what } => write!(f, "{file}: {id}: cite {path}: {what}"),
            CatalogError::Duplicate { file, id } => write!(f, "{file}: {id} twice"),
        }
    }
}

/// Every record the search order sees.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub punches: Vec<PunchRec>,
    pub dies: Vec<DieRec>,
    pub presses: Vec<PressRec>,
}

fn deg(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

struct Rd<'a> {
    file: &'a str,
    id: String,
    t: &'a Table,
    /// Paths of the values held, for the cite check.
    held: Vec<String>,
}

impl<'a> Rd<'a> {
    fn bad(&self, field: &str, what: &str) -> CatalogError {
        CatalogError::Field {
            file: self.file.into(),
            id: self.id.clone(),
            field: field.into(),
            what: what.into(),
        }
    }

    fn text(&self, k: &str) -> Result<Option<String>, CatalogError> {
        match self.t.get(k) {
            None => Ok(None),
            Some(Item::Str(s)) => Ok(Some(s.clone())),
            Some(_) => Err(self.bad(k, "a string")),
        }
    }

    /// A number at a dotted path (`relief.depth`), recorded as held.
    fn num(&mut self, path: &str) -> Result<Option<f64>, CatalogError> {
        let mut t = self.t;
        let parts: Vec<&str> = path.split('.').collect();
        for p in &parts[..parts.len() - 1] {
            match t.get(p) {
                None => return Ok(None),
                Some(Item::Table(x)) => t = x,
                Some(_) => return Err(self.bad(path, "a table")),
            }
        }
        match t.get(parts[parts.len() - 1]) {
            None => Ok(None),
            Some(Item::Num(x)) if x.is_finite() => {
                self.held.push(path.into());
                Ok(Some(*x))
            }
            Some(_) => Err(self.bad(path, "a number")),
        }
    }

    fn pair(&mut self, path: &str) -> Result<Option<(f64, f64)>, CatalogError> {
        let mut t = self.t;
        let parts: Vec<&str> = path.split('.').collect();
        for p in &parts[..parts.len() - 1] {
            match t.get(p) {
                None => return Ok(None),
                Some(Item::Table(x)) => t = x,
                Some(_) => return Err(self.bad(path, "a table")),
            }
        }
        match t.get(parts[parts.len() - 1]) {
            None => Ok(None),
            Some(Item::Arr(a)) => match a.as_slice() {
                [Item::Num(a), Item::Num(b)] if a <= b => {
                    self.held.push(path.into());
                    Ok(Some((*a, *b)))
                }
                _ => Err(self.bad(path, "[lo, hi]")),
            },
            Some(_) => Err(self.bad(path, "[lo, hi]")),
        }
    }

    fn segments(&mut self) -> Result<Vec<Segment>, CatalogError> {
        let Some(it) = self.t.get("segments") else {
            return Ok(Vec::new());
        };
        let Item::Arr(a) = it else {
            return Err(self.bad("segments", "[{ length, count }, ..]"));
        };
        let mut out = Vec::new();
        for x in a {
            let Item::Table(s) = x else {
                return Err(self.bad("segments", "[{ length, count }, ..]"));
            };
            match (s.get("length"), s.get("count")) {
                (Some(Item::Num(l)), Some(Item::Num(c))) if *l > 0.0 && *c >= 1.0 && c.fract() == 0.0 => out
                    .push(Segment {
                        length: *l,
                        count: *c as u32,
                    }),
                _ => return Err(self.bad("segments", "each { length > 0, count >= 1 }")),
            }
        }
        self.held.push("segments".into());
        Ok(out)
    }

    /// An explicit outline: `[[x, z], ..]`, one closed polygon.
    fn outline(&mut self) -> Result<Option<Vec<crate::geo::Seg>>, CatalogError> {
        let Some(it) = self.t.get("outline") else {
            return Ok(None);
        };
        let Item::Arr(a) = it else {
            return Err(self.bad("outline", "[[x, z], ..]"));
        };
        let mut pts = Vec::new();
        for p in a {
            match p {
                Item::Arr(xz) => match xz.as_slice() {
                    [Item::Num(x), Item::Num(z)] => pts.push((*x, *z)),
                    _ => return Err(self.bad("outline", "[[x, z], ..]")),
                },
                _ => return Err(self.bad("outline", "[[x, z], ..]")),
            }
        }
        if pts.len() < 3 {
            return Err(self.bad("outline", "at least three points"));
        }
        self.held.push("outline".into());
        let n = pts.len();
        Ok(Some(
            (0..n)
                .map(|i| crate::geo::Seg::line(pts[i], pts[(i + 1) % n]))
                .collect(),
        ))
    }

    fn prov(&self, kind: &str) -> Result<Prov, CatalogError> {
        let real = matches!(self.t.get("real"), Some(Item::Bool(true)));
        let mut cites = Vec::new();
        if let Some(Item::Tables(rows)) = self.t.get("cite") {
            for r in rows {
                let s = |k: &str| match r.get(k) {
                    Some(Item::Str(s)) => Some(s.clone()),
                    _ => None,
                };
                let path = s("path").unwrap_or_default();
                let cerr = |what: &str| CatalogError::Cite {
                    file: self.file.into(),
                    id: self.id.clone(),
                    path: path.clone(),
                    what: what.into(),
                };
                let status = s("status")
                    .as_deref()
                    .and_then(Status::parse)
                    .ok_or_else(|| cerr("status: published, estimate, unknown, conflict or user"))?;
                let c = Cite {
                    path: path.clone(),
                    status,
                    source: s("source"),
                    taken: s("taken"),
                    value: s("value"),
                    basis: s("basis"),
                    note: s("note"),
                };
                if path.is_empty() {
                    return Err(cerr("no path"));
                }
                match status {
                    Status::Published | Status::Conflict if c.source.is_none() || c.taken.is_none() => {
                        return Err(cerr("a published or conflict cite needs source and taken"));
                    }
                    Status::Estimate | Status::Unknown | Status::User if c.basis.is_none() => {
                        return Err(cerr("an estimate, unknown or user cite needs a basis"));
                    }
                    Status::User if real => {
                        return Err(cerr("a real record holds no user value (real = false)"));
                    }
                    Status::Conflict if c.note.is_none() => return Err(cerr("a conflict cite needs a note")),
                    _ => {}
                }
                cites.push(c);
            }
        }
        let _ = kind;
        Ok(Prov {
            maker: self.text("maker")?.unwrap_or_default(),
            name: self.text("name")?.or(self.text("model")?).unwrap_or_default(),
            real,
            note: self.text("note")?,
            file: self.file.into(),
            cites,
        })
    }

    /// Every held value of a real record is cited; every cite names a held value, a record-wide
    /// fact (`record`), or a value the record says is unknown (status unknown).
    fn check_cites(&self, prov: &Prov) -> Result<(), CatalogError> {
        for c in &prov.cites {
            let ok = c.path == "record"
                || self
                    .held
                    .iter()
                    .any(|h| *h == c.path || c.path.starts_with(&format!("{h}.")))
                || c.status == Status::Unknown;
            if !ok {
                return Err(CatalogError::Cite {
                    file: self.file.into(),
                    id: self.id.clone(),
                    path: c.path.clone(),
                    what: "names no value the record holds (an unknown value: status unknown)".into(),
                });
            }
        }
        if prov.real {
            for h in &self.held {
                if prov.of(h).is_empty() {
                    return Err(CatalogError::Cite {
                        file: self.file.into(),
                        id: self.id.clone(),
                        path: h.clone(),
                        what: "a real record's value with no cite".into(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn style_of(clamp: &str) -> Style {
    if clamp == "american" { Style::Us } else { Style::Eu }
}

fn punch_rec(file: &str, t: &Table) -> Result<PunchRec, CatalogError> {
    let id = match t.get("id") {
        Some(Item::Str(s)) => s.clone(),
        _ => {
            return Err(CatalogError::Field {
                file: file.into(),
                id: "?".into(),
                field: "id".into(),
                what: "a string".into(),
            });
        }
    };
    let mut r = Rd {
        file,
        id: id.clone(),
        t,
        held: Vec::new(),
    };
    let clamp = r.text("clamp")?.ok_or_else(|| r.bad("clamp", "required"))?;
    let kind = match r.text("kind")?.as_deref() {
        Some("straight") => PunchKind::Straight,
        Some("gooseneck") => PunchKind::Gooseneck,
        Some("acute") => PunchKind::Acute,
        Some("sash") => PunchKind::Gooseneck,
        _ => return Err(r.bad("kind", "straight, gooseneck, acute or sash")),
    };
    let height = r.num("height")?;
    let tip_radius = r.num("tip_radius")?;
    let tip_angle = r.num("tip_angle")?;
    let width = r.num("width")?;
    let tip_height = r.num("tip_height")?;
    let max_load = r.num("max_load")?;
    let rd = (r.num("relief.depth")?, r.num("relief.z0")?, r.num("relief.z1")?);
    let segments = r.segments()?;
    let outline = r.outline()?;
    let prov = r.prov("punch")?;
    r.check_cites(&prov)?;
    let mut unknown = Vec::new();
    for (k, v) in [
        ("height", height),
        ("tip_radius", tip_radius),
        ("tip_angle", tip_angle),
        ("width", width),
    ] {
        if v.is_none() {
            unknown.push(k.to_string());
        }
    }
    if kind == PunchKind::Gooseneck && rd.0.is_none() && outline.is_none() {
        unknown.push("relief.depth".into());
    }
    let punch = if unknown.is_empty() {
        let (h, rp, a, w) = (
            height.unwrap(),
            tip_radius.unwrap(),
            tip_angle.unwrap(),
            width.unwrap(),
        );
        let mut p = Punch {
            id: id.clone(),
            style: style_of(&clamp),
            kind,
            height: h,
            tip_radius: rp,
            tip_angle: deg(a),
            tip_height: 0.0,
            width: w,
            relief: None,
            profile: outline,
            segments,
            max_load,
        };
        // the working part: the flanks up to where they reach the body's width (or 12 mm, the
        // stand-ins' height, whichever is lower), unless the record states it
        p.tip_height = tip_height.unwrap_or_else(|| p.flank_z(0.5 * w).clamp(0.5, 12.0).min(h));
        if let Some(depth) = rd.0 {
            // a throat whose heights are not stated: the smallest that holds a 1 mm sheet's
            // return (errs small: more tool, never less)
            let z0 = rd.1.unwrap_or(p.tip_height.max(0.4 * h)).max(p.tip_height);
            let z1 = rd.2.unwrap_or(z0 + 0.25 * (h - z0));
            p.relief = Some(Relief { depth, z0, z1 });
        }
        match p.validate() {
            Ok(()) => Ok(p),
            Err(e) => Err(vec![format!("inconsistent: {e:?}")]),
        }
    } else {
        Err(unknown)
    };
    Ok(PunchRec {
        id,
        prov,
        clamp,
        kind,
        punch,
    })
}

fn die_rec(file: &str, t: &Table) -> Result<DieRec, CatalogError> {
    let id = match t.get("id") {
        Some(Item::Str(s)) => s.clone(),
        _ => {
            return Err(CatalogError::Field {
                file: file.into(),
                id: "?".into(),
                field: "id".into(),
                what: "a string".into(),
            });
        }
    };
    let mut r = Rd {
        file,
        id: id.clone(),
        t,
        held: Vec::new(),
    };
    let clamp = r.text("clamp")?.ok_or_else(|| r.bad("clamp", "required"))?;
    let v = r.num("v")?;
    let angle = r.num("angle")?;
    let rs = r.num("shoulder_radius")?;
    let height = r.num("height")?;
    let width = r.num("width")?;
    let floor = r.num("floor")?;
    let max_load = r.num("max_load")?;
    let segments = r.segments()?;
    let outline = r.outline()?;
    let prov = r.prov("die")?;
    r.check_cites(&prov)?;
    let mut unknown = Vec::new();
    for (k, x) in [
        ("v", v),
        ("angle", angle),
        ("shoulder_radius", rs),
        ("height", height),
        ("width", width),
    ] {
        if x.is_none() {
            unknown.push(k.to_string());
        }
    }
    let die = if unknown.is_empty() {
        let d = Die {
            id: id.clone(),
            style: style_of(&clamp),
            v: v.unwrap(),
            angle: deg(angle.unwrap()),
            shoulder_radius: rs.unwrap(),
            height: height.unwrap(),
            width: width.unwrap(),
            floor,
            profile: outline,
            segments,
            max_load,
        };
        match d.validate() {
            Ok(()) => Ok(d),
            Err(e) => Err(vec![format!("inconsistent: {e:?}")]),
        }
    } else {
        Err(unknown)
    };
    Ok(DieRec { id, prov, clamp, die })
}

fn press_rec(file: &str, t: &Table) -> Result<PressRec, CatalogError> {
    let id = match t.get("id") {
        Some(Item::Str(s)) => s.clone(),
        _ => {
            return Err(CatalogError::Field {
                file: file.into(),
                id: "?".into(),
                field: "id".into(),
                what: "a string".into(),
            });
        }
    };
    let mut r = Rd {
        file,
        id: id.clone(),
        t,
        held: Vec::new(),
    };
    let clamp = r.text("clamp")?.unwrap_or_else(|| "unknown".into());
    let model = r.text("model")?.unwrap_or_default();
    let force = r.num("force")?;
    let bed_length = r.num("bed_length")?;
    let housing_gap = r.num("housing_gap")?;
    let open_height = r.num("open_height")?;
    let stroke = r.num("stroke")?;
    let throat = r.num("throat")?;
    let table_height = r.num("table_height")?;
    let ft = r.num("frame.thickness")?;
    let fd = r.num("frame.depth")?;
    let beam = |r: &mut Rd, k: &str| -> Result<Option<Beam>, CatalogError> {
        Ok(
            match (r.num(&format!("{k}.front"))?, r.num(&format!("{k}.back"))?) {
                (Some(front), Some(back)) => Some(Beam { front, back }),
                _ => None,
            },
        )
    };
    let ram = beam(&mut r, "ram")?;
    let bed = beam(&mut r, "bed")?;
    let speeds = Speeds {
        approach: r.num("speeds.approach")?,
        work: r.num("speeds.work")?,
        ret: r.num("speeds.return")?,
    };
    let bx = r.pair("backgauge.x")?;
    let br = r.pair("backgauge.r")?;
    let bz = r.pair("backgauge.z")?;
    let fingers = r.num("backgauge.fingers")?;
    let fdp = r.num("backgauge.finger.depth")?;
    let fh = r.num("backgauge.finger.height")?;
    let fw = r.num("backgauge.finger.width")?;
    let retract = r.num("backgauge.retract")?;
    let axes = match t.get("backgauge").and_then(|b| {
        if let Item::Table(b) = b {
            b.get("axes")
        } else {
            None
        }
    }) {
        Some(Item::Arr(a)) => {
            r.held.push("backgauge.axes".into());
            a.iter()
                .filter_map(|x| {
                    if let Item::Str(s) = x {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };
    let prov = r.prov("press")?;
    r.check_cites(&prov)?;
    // geometry the simulation cannot go without
    let mut unknown = Vec::new();
    for (k, v) in [
        ("open_height", open_height),
        ("stroke", stroke),
        ("bed_length", bed_length),
        ("housing_gap", housing_gap),
    ] {
        if v.is_none() {
            unknown.push(k.to_string());
        }
    }
    let press = if unknown.is_empty() {
        let mut unchecked = Vec::new();
        if force.is_none() {
            unchecked.push("force".to_string());
        }
        // a maker's open height is the daylight between table and beam (or its clamps): the
        // clamps' and holders' heights are not published with it, so neither the room left for
        // the tools nor how far the ram must come down is known (open height and stroke report
        // not checked)
        if !matches!(t.get("clamps_included"), Some(Item::Bool(true))) {
            unchecked.push("open_height".to_string());
        }
        // beams: unstated, a block 200 mm each side of the tool line (errs large)
        let big = Beam {
            front: 200.0,
            back: 200.0,
        };
        let finger = match (fdp, fh, fw) {
            (Some(depth), Some(height), Some(width)) => Finger { depth, height, width },
            // unstated: a 60 x 60 x 40 block (errs large against a typical finger)
            _ => Finger {
                depth: 60.0,
                height: 60.0,
                width: 40.0,
            },
        };
        let (x, rr) = match (bx, br) {
            (Some(x), Some(rr)) => (x, rr),
            _ => {
                unchecked.push("backgauge".to_string());
                (bx.unwrap_or((0.0, 0.0)), br.unwrap_or((0.0, 0.0)))
            }
        };
        let mut estimated = Vec::new();
        if ram.is_none() {
            estimated.push("ram".to_string());
        }
        if bed.is_none() {
            estimated.push("bed".to_string());
        }
        if fdp.is_none() || fh.is_none() || fw.is_none() {
            estimated.push("backgauge".to_string());
        }
        if throat.is_some() && (ft.is_none() || fd.is_none()) {
            estimated.push("frame".to_string());
        }
        let p = Press {
            id: id.clone(),
            open_height: open_height.unwrap(),
            stroke: stroke.unwrap(),
            bed_length: bed_length.unwrap(),
            housing_gap: housing_gap.unwrap(),
            ram: ram.unwrap_or(big),
            bed: bed.unwrap_or(big),
            max_force: force,
            backgauge: Backgauge {
                fingers: fingers.map(|f| f as u32).unwrap_or(2),
                x,
                r: rr,
                finger,
                retract: retract.unwrap_or(0.0),
                z: bz,
            },
            frame: throat.map(|throat| Frame {
                throat,
                // unstated: 100 mm plates, 1500 mm deep behind the throat (errs large)
                thickness: ft.unwrap_or(100.0),
                depth: fd.unwrap_or(1500.0),
            }),
            unchecked,
            estimated,
        };
        Ok(p)
    } else {
        Err(unknown)
    };
    Ok(PressRec {
        id,
        prov,
        model,
        clamp,
        axes,
        speeds,
        table_height,
        press,
    })
}

impl Catalog {
    /// Records from (file, text) sources in search order: an id found earlier hides a later one.
    pub fn from_sources(sources: &[(&str, &str)]) -> Result<Catalog, CatalogError> {
        let mut c = Catalog::default();
        for (file, src) in sources {
            c.add(file, src)?;
        }
        Ok(c)
    }

    /// The built-in library.
    pub fn builtin() -> Catalog {
        Catalog::from_sources(BUILTIN).unwrap_or_else(|e| panic!("built-in tooling library: {e}"))
    }

    /// Adds a file's records after those held (a later level: ids already held win).
    pub fn add(&mut self, file: &str, src: &str) -> Result<(), CatalogError> {
        let root = toml::parse(src).map_err(|e| CatalogError::Toml {
            file: file.into(),
            line: e.line,
            why: e.why,
        })?;
        let rows = |k: &str| match root.get(k) {
            Some(Item::Tables(v)) => v.clone(),
            _ => Vec::new(),
        };
        let mut seen: Vec<String> = Vec::new();
        let mut dup = |id: &str| {
            if seen.iter().any(|s| s == id) {
                return Err(CatalogError::Duplicate {
                    file: file.into(),
                    id: id.into(),
                });
            }
            seen.push(id.into());
            Ok(())
        };
        for t in rows("punch") {
            let p = punch_rec(file, &t)?;
            dup(&p.id)?;
            if !self.punches.iter().any(|q| q.id == p.id) {
                self.punches.push(p);
            }
        }
        for t in rows("die") {
            let d = die_rec(file, &t)?;
            dup(&d.id)?;
            if !self.dies.iter().any(|q| q.id == d.id) {
                self.dies.push(d);
            }
        }
        for t in rows("press") {
            let p = press_rec(file, &t)?;
            dup(&p.id)?;
            if !self.presses.iter().any(|q| q.id == p.id) {
                self.presses.push(p);
            }
        }
        Ok(())
    }

    pub fn punch(&self, id: &str) -> Option<&PunchRec> {
        self.punches.iter().find(|p| p.id == id)
    }

    pub fn die(&self, id: &str) -> Option<&DieRec> {
        self.dies.iter().find(|p| p.id == id)
    }

    pub fn press(&self, id: &str) -> Option<&PressRec> {
        self.presses.iter().find(|p| p.id == id)
    }

    /// A tooling set: a press and tools from the library. Unknown geometry refuses, naming the
    /// record and its unknown fields; a tool for another clamping system than the press's is
    /// refused too (`clamp`), unless the press's own system is unknown.
    pub fn tooling(&self, press: &str, punches: &[&str], dies: &[&str]) -> Result<Tooling, ToolingError> {
        let missing = |id: &str| ToolingError::Unknown {
            id: id.into(),
            fields: vec!["record".into()],
        };
        let pr = self.press(press).ok_or_else(|| missing(press))?;
        let p = pr.press.clone().map_err(|f| ToolingError::Unknown {
            id: press.into(),
            fields: f,
        })?;
        let fits = |c: &str| pr.clamp == "unknown" || c == pr.clamp;
        let mut ps = Vec::new();
        for id in punches {
            let r = self.punch(id).ok_or_else(|| missing(id))?;
            if !fits(&r.clamp) {
                return Err(ToolingError::Unknown {
                    id: (*id).into(),
                    fields: vec![format!("clamp {} on a {} press", r.clamp, pr.clamp)],
                });
            }
            ps.push(r.punch.clone().map_err(|f| ToolingError::Unknown {
                id: (*id).into(),
                fields: f,
            })?);
        }
        let mut ds = Vec::new();
        for id in dies {
            let r = self.die(id).ok_or_else(|| missing(id))?;
            if !fits(&r.clamp) {
                return Err(ToolingError::Unknown {
                    id: (*id).into(),
                    fields: vec![format!("clamp {} on a {} press", r.clamp, pr.clamp)],
                });
            }
            ds.push(r.die.clone().map_err(|f| ToolingError::Unknown {
                id: (*id).into(),
                fields: f,
            })?);
        }
        let t = Tooling {
            id: format!("{press}+library"),
            punches: ps,
            dies: ds,
            press: p,
            clearance: 0.5,
            self_clearance: 0.0,
            short_end: None,
            // springback is a material property: forge-material's, when cited (none yet); a
            // library set carries no rows of its own, so the overbend is 0 and said unknown
            springback: Vec::new(),
            v_rule: 8.0,
            stations: Vec::new(),
        };
        t.validate()?;
        Ok(t)
    }

    /// The tools a press gets when none are named: those of its clamping system whose geometry
    /// is known; of those, the press maker's own when the maker publishes any (machine
    /// vendors' tooling first). With neither the maker's tools nor the press's clamping system
    /// known: none.
    pub fn default_tools(&self, pr: &PressRec, punches: bool) -> Vec<&str> {
        let fits = |c: &str| pr.clamp == "unknown" || c == pr.clamp;
        let all: Vec<(&str, &str)> = if punches {
            self.punches
                .iter()
                .filter(|r| fits(&r.clamp) && r.punch.is_ok())
                .map(|r| (r.id.as_str(), r.prov.maker.as_str()))
                .collect()
        } else {
            self.dies
                .iter()
                .filter(|r| fits(&r.clamp) && r.die.is_ok())
                .map(|r| (r.id.as_str(), r.prov.maker.as_str()))
                .collect()
        };
        let own: Vec<&str> = all
            .iter()
            .filter(|x| x.1.eq_ignore_ascii_case(&pr.prov.maker))
            .map(|x| x.0)
            .collect();
        if own.is_empty() && pr.clamp != "unknown" {
            all.into_iter().map(|x| x.0).collect()
        } else {
            // a maker with no tools held and no clamping system published: none by default
            // (a punch from one system on another's die is no setup); the user names them
            own
        }
    }

    /// The press's default tools ([`Catalog::default_tools`]) as a tooling set.
    pub fn tooling_for(&self, press: &str) -> Result<Tooling, ToolingError> {
        let pr = self.press(press).ok_or_else(|| ToolingError::Unknown {
            id: press.into(),
            fields: vec!["record".into()],
        })?;
        let ps = self.default_tools(pr, true);
        let ds = self.default_tools(pr, false);
        if ps.is_empty() {
            return Err(ToolingError::NoPunches);
        }
        if ds.is_empty() {
            return Err(ToolingError::NoDies);
        }
        self.tooling(press, &ps, &ds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_library_reads_and_cites_every_value() {
        let c = Catalog::builtin();
        assert!(!c.punches.is_empty() && !c.dies.is_empty());
        for p in &c.punches {
            assert!(p.prov.real, "{}", p.id);
            if let Ok(x) = &p.punch {
                for f in ["height", "tip_radius", "tip_angle"] {
                    assert_eq!(p.prov.status(f), Some(Status::Published), "{} {f}", p.id);
                }
                x.validate().unwrap();
            }
        }
        for d in &c.dies {
            if d.die.is_ok() {
                for f in ["v", "angle", "shoulder_radius", "height"] {
                    assert_eq!(d.prov.status(f), Some(Status::Published), "{} {f}", d.id);
                }
            }
        }
        let known = |m: &str| {
            (
                c.punches
                    .iter()
                    .filter(|p| p.prov.maker == m && p.punch.is_ok())
                    .count(),
                c.dies
                    .iter()
                    .filter(|p| p.prov.maker == m && p.die.is_ok())
                    .count(),
            )
        };
        eprintln!(
            "trumpf {:?} bystronic {:?} amada {:?}",
            known("Trumpf"),
            known("Bystronic"),
            known("Amada")
        );
        assert!(known("Trumpf").0 > 0 && known("Trumpf").1 > 0);
    }

    #[test]
    fn unknown_geometry_refuses() {
        let src = r#"
[[punch]]
id = "x/p"
maker = "X"
real = true
clamp = "promecam"
kind = "straight"
height = 100
tip_angle = 88
[[punch.cite]]
path = "height"
status = "published"
source = "https://example.invalid"
taken = "2026-11-24"
[[punch.cite]]
path = "tip_angle"
status = "published"
source = "https://example.invalid"
taken = "2026-11-24"
"#;
        let c = Catalog::from_sources(&[("t.toml", src)]).unwrap();
        assert_eq!(
            c.punch("x/p").unwrap().punch,
            Err(vec!["tip_radius".to_string(), "width".to_string()])
        );
        // an uncited value of a real record is refused
        let bad = src.replace(
            "[[punch.cite]]\npath = \"height\"",
            "[[punch.cite]]\npath = \"nothing\"",
        );
        assert!(matches!(
            Catalog::from_sources(&[("t.toml", &bad)]),
            Err(CatalogError::Cite { .. })
        ));
    }

    #[test]
    fn a_users_value_is_cited_user_and_never_in_a_real_record() {
        let mine = r#"
[[punch]]
id = "me/p"
maker = "Mine"
real = false
clamp = "promecam"
kind = "straight"
height = 100
[[punch.cite]]
path = "height"
status = "user"
basis = "the user's own value, entered in forgecad's library builder"
"#;
        let c = Catalog::from_sources(&[("t.toml", mine)]).unwrap();
        assert_eq!(c.punch("me/p").unwrap().prov.status("height"), Some(Status::User));
        // a user cite needs its basis; a real record holds no user value
        let bare = mine.replace(
            "basis = \"the user's own value, entered in forgecad's library builder\"\n",
            "",
        );
        assert!(Catalog::from_sources(&[("t.toml", &bare)]).is_err());
        let real = mine.replace("real = false", "real = true");
        assert!(Catalog::from_sources(&[("t.toml", &real)]).is_err());
    }
}

#[cfg(test)]
mod press_tests {
    use super::*;

    #[test]
    fn presses_read_and_known_ones_build() {
        let c = Catalog::builtin();
        let usable: Vec<&str> = c
            .presses
            .iter()
            .filter(|p| p.press.is_ok())
            .map(|p| p.id.as_str())
            .collect();
        eprintln!("{} presses, usable {usable:?}", c.presses.len());
        assert!(c.presses.len() >= 20);
        let p = c.press("trumpf-trubend-5130").unwrap();
        let pr = p.press.as_ref().unwrap();
        assert_eq!(
            (pr.open_height, pr.stroke, pr.housing_gap),
            (385.0, 215.0, 2690.0)
        );
        assert_eq!(pr.frame.map(|f| f.throat), Some(420.0));
        assert!(pr.unchecked.contains(&"open_height".to_string()));
        let t = c.tooling_for("trumpf-trubend-5130").unwrap();
        assert!(!t.punches.is_empty() && !t.dies.is_empty());
    }
}
