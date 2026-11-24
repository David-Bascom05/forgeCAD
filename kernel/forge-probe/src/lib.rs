//! Probing devices as library records: spindle work probes (a touch probe held in the spindle or
//! the tool changer like a tool) and tool setters (a contact stylus or a laser beam on the table).
//!
//! Each record is real data restated from its maker's openly published pages and cited per value
//! (`[[probe.cite]]`, the machine library's rule, docs/cam-machine.md "Provenance"). A number the
//! maker does not publish is left out and cited `unknown`; every check that needs it reports
//! "not checked", never a pass. The file format is forge-tooling's TOML subset.
//!
//! Frames (mm, degrees, mm/min, kg):
//! - a work probe: origin at the centre of the stylus mounting face (the body's lower end), +z
//!   along the spindle axis toward the spindle; the stylus hangs below it. `stylus = [ball
//!   diameter, length from the mounting face to the ball centre]` only where the maker ships a
//!   stated stylus (styli are usually the user's choice: then the setup gives it);
//! - a tool setter: origin at the centre of its mounting face on the table, +z up (away from the
//!   table); `trigger` is the point a tool is brought to (a contact disc's top centre, a laser's
//!   beam crossing).
//!
//! Bodies (`[[probe.cyl]]`, `[[probe.box]]`) are envelopes in that frame for the collision checks.
use forge_tooling::LibError;
pub use forge_tooling::cite::CiteStatus;
use forge_tooling::cite::{self, Cite};
use forge_tooling::toml::{self, Table, Val, nums, tables, text, texts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    /// A touch probe in the spindle.
    WorkProbe,
    /// A device on the table that measures the tool in the spindle.
    ToolSetter,
}

impl DeviceKind {
    pub fn word(self) -> &'static str {
        match self {
            DeviceKind::WorkProbe => "work-probe",
            DeviceKind::ToolSetter => "tool-setter",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensing {
    /// A stylus deflected by contact (kinematic or strain-gauge).
    Contact,
    /// A beam the tool breaks (non-contact).
    Laser,
}

impl Sensing {
    pub fn word(self) -> &'static str {
        match self {
            Sensing::Contact => "contact",
            Sensing::Laser => "laser",
        }
    }
}

/// How the trigger reaches the control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transmission {
    Optical,
    Radio,
    /// A cable (hard-wired; a passive continuity probe too).
    Wired,
}

impl Transmission {
    pub fn word(self) -> &'static str {
        match self {
            Transmission::Optical => "optical",
            Transmission::Radio => "radio",
            Transmission::Wired => "wired",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// A cylinder from `base` along unit `axis`, radius `r`, height `h`.
    Cyl {
        base: [f64; 3],
        axis: [f64; 3],
        r: f64,
        h: f64,
    },
    Box {
        min: [f64; 3],
        max: [f64; 3],
    },
}

/// Machines a maker says ship with or offer the device.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    /// forge-machine record names.
    pub machines: Vec<String>,
    /// `standard` (ships with it) or `option`.
    pub status: String,
    pub source: String,
    pub taken: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub id: String,
    pub kind: DeviceKind,
    pub vendor: String,
    pub product: String,
    pub note: String,
    pub real: bool,
    pub sensing: Sensing,
    /// None: cited unknown.
    pub transmission: Option<Transmission>,
    /// Unidirectional trigger repeatability, 2σ, mm. None: cited unknown.
    pub repeat: Option<f64>,
    /// The highest probing (measuring) feed the maker states, mm/min. None: cited unknown (the
    /// feed is then not checked). A data sheet's test velocity is not a maximum.
    pub max_feed: Option<f64>,
    /// Stylus overtravel: ± degrees in XY and mm in +Z (work probes). None: cited unknown (the
    /// overtravel past the nominal surface is then not checked).
    pub overtravel: Option<[f64; 2]>,
    /// Work probe: [ball diameter, length from the mounting face to the ball centre].
    pub stylus: Option<[f64; 2]>,
    /// Tool setter: the point a tool is brought to, device frame.
    pub trigger: Option<[f64; 3]>,
    /// Contact tool setter: the stylus disc (or cube) width, mm.
    pub disc_d: Option<f64>,
    /// Laser tool setter: the smallest tool diameter it measures, mm.
    pub min_tool_d: Option<f64>,
    /// Mass, kg. None: not stated (it is cited if given).
    pub mass: Option<f64>,
    /// Envelopes, device frame. Empty: cited unknown (the device is left out of the collision
    /// check, which says so).
    pub bodies: Vec<Prim>,
    pub offers: Vec<Offer>,
    pub cites: Vec<Cite>,
}

impl Device {
    /// The paths a real record cites (each present, or omitted and cited unknown).
    pub fn cite_paths(kind: DeviceKind, sensing: Sensing) -> Vec<String> {
        let mut p = vec!["transmission", "repeat", "max_feed", "body"];
        match (kind, sensing) {
            (DeviceKind::WorkProbe, _) => p.extend(["stylus", "overtravel"]),
            (DeviceKind::ToolSetter, Sensing::Contact) => p.extend(["trigger", "disc_d"]),
            (DeviceKind::ToolSetter, Sensing::Laser) => p.extend(["trigger", "min_tool_d"]),
        }
        p.into_iter().map(String::from).collect()
    }

    /// How a cited value stands: the first cite's status naming `path`, None if uncited.
    pub fn status(&self, path: &str) -> Option<CiteStatus> {
        self.cites.iter().find(|c| c.path == path).map(|c| c.status)
    }

    /// The values a check would need that are unknown or estimated, by path.
    pub fn not_checked(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for c in &self.cites {
            if matches!(c.status, CiteStatus::Unknown | CiteStatus::Estimate) && !v.contains(&c.path) {
                v.push(c.path.clone());
            }
        }
        v
    }
}

/// Cite fields besides the required paths.
const ALLOWED: [&str; 3] = ["mass", "sensing", "device"];

fn bad(rec: &str, field: &'static str, why: impl Into<String>) -> LibError {
    LibError::Bad {
        record: rec.to_string(),
        field,
        why: why.into(),
    }
}

fn missing(rec: &str, field: &'static str, fix: &str) -> LibError {
    LibError::Missing {
        record: rec.to_string(),
        field,
        fix: fix.to_string(),
    }
}

fn vec3(t: &Table, k: &'static str, rec: &str) -> Result<Option<[f64; 3]>, LibError> {
    match nums(t, k) {
        None if t.contains_key(k) => Err(bad(rec, k, "is not a list of numbers")),
        None => Ok(None),
        Some(v) if v.len() == 3 && v.iter().all(|x| x.is_finite()) => Ok(Some([v[0], v[1], v[2]])),
        Some(_) => Err(bad(rec, k, "needs three finite numbers")),
    }
}

fn pair(t: &Table, k: &'static str, rec: &str) -> Result<Option<[f64; 2]>, LibError> {
    match nums(t, k) {
        None if t.contains_key(k) => Err(bad(rec, k, "is not a list of numbers")),
        None => Ok(None),
        Some(v) if v.len() == 2 && v.iter().all(|x| x.is_finite()) => Ok(Some([v[0], v[1]])),
        Some(_) => Err(bad(rec, k, "needs two finite numbers")),
    }
}

fn positive(t: &Table, k: &'static str, rec: &str) -> Result<Option<f64>, LibError> {
    match t.get(k) {
        None => Ok(None),
        Some(Val::Num(x)) if x.is_finite() && *x > 0.0 => Ok(Some(*x)),
        Some(_) => Err(bad(rec, k, "needs a positive number")),
    }
}

const KEYS: [&str; 21] = [
    "id",
    "kind",
    "vendor",
    "product",
    "note",
    "real",
    "sensing",
    "transmission",
    "repeat",
    "max_feed",
    "overtravel",
    "stylus",
    "trigger",
    "disc_d",
    "min_tool_d",
    "mass",
    "body",
    "cyl",
    "box",
    "offer",
    "cite",
];

impl Prim {
    /// A cylinder about the device's z axis (written as a `body` ring): (r, from, to).
    pub fn ring(&self) -> Option<(f64, f64, f64)> {
        match self {
            Prim::Cyl { base, axis, r, h }
                if base[0] == 0.0 && base[1] == 0.0 && *axis == [0.0, 0.0, 1.0] =>
            {
                Some((*r, base[2], base[2] + h))
            }
            _ => None,
        }
    }
}

/// The device's turned outline for a preview: (z, r) points around its rings, from the lowest
/// z up and back down the axis (None when it has a body that is not coaxial, or none).
pub fn outline(d: &Device) -> Option<Vec<[f64; 2]>> {
    let rings: Vec<(f64, f64, f64)> = d.bodies.iter().map(Prim::ring).collect::<Option<_>>()?;
    if rings.is_empty() {
        return None;
    }
    let mut rs = rings;
    rs.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut out = vec![[rs[0].1, 0.0]];
    for (r, a, b) in &rs {
        out.push([*a, *r]);
        out.push([*b, *r]);
    }
    out.push([rs[rs.len() - 1].2, 0.0]);
    Some(out)
}

/// One `[[probe]]` table.
pub fn read(t: &Table) -> Result<Device, LibError> {
    let id = text(t, "id")
        .ok_or_else(|| missing("probe", "id", "give `id`, a unique name"))?
        .to_string();
    let rec = format!("probe `{id}`");
    if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(LibError::Bad {
            record: rec,
            field: "key",
            why: format!("`{k}` is not a probe field; fields: {}", KEYS.join(", ")),
        });
    }
    let kind = match text(t, "kind") {
        Some("work-probe") => DeviceKind::WorkProbe,
        Some("tool-setter") => DeviceKind::ToolSetter,
        Some(k) => {
            return Err(bad(
                &rec,
                "kind",
                format!("`{k}` is not work-probe | tool-setter"),
            ));
        }
        None => return Err(missing(&rec, "kind", "work-probe | tool-setter")),
    };
    let sensing = match text(t, "sensing") {
        Some("contact") => Sensing::Contact,
        Some("laser") => Sensing::Laser,
        Some(k) => return Err(bad(&rec, "sensing", format!("`{k}` is not contact | laser"))),
        None => return Err(missing(&rec, "sensing", "contact | laser")),
    };
    if kind == DeviceKind::WorkProbe && sensing == Sensing::Laser {
        return Err(bad(&rec, "sensing", "a work probe here is a contact probe"));
    }
    let transmission = match text(t, "transmission") {
        Some("optical") => Some(Transmission::Optical),
        Some("radio") => Some(Transmission::Radio),
        Some("wired") => Some(Transmission::Wired),
        Some(k) => {
            return Err(bad(
                &rec,
                "transmission",
                format!("`{k}` is not optical | radio | wired"),
            ));
        }
        None => None,
    };
    let s = |k: &str| text(t, k).unwrap_or("").to_string();
    let required = Device::cite_paths(kind, sensing);
    let (real, cites) = cite::read(t, &rec, &required, &ALLOWED)?;
    let repeat = positive(t, "repeat", &rec)?;
    let max_feed = positive(t, "max_feed", &rec)?;
    let overtravel = pair(t, "overtravel", &rec)?;
    if overtravel.is_some_and(|[a, z]| a <= 0.0 || z <= 0.0) {
        return Err(bad(
            &rec,
            "overtravel",
            "needs [± degrees in XY, mm in +Z], both positive",
        ));
    }
    let stylus = pair(t, "stylus", &rec)?;
    if stylus.is_some_and(|[d, l]| d <= 0.0 || l <= d / 2.0) {
        return Err(bad(
            &rec,
            "stylus",
            "needs [ball diameter, length past the ball's radius] > 0",
        ));
    }
    let trigger = vec3(t, "trigger", &rec)?;
    let disc_d = positive(t, "disc_d", &rec)?;
    let min_tool_d = positive(t, "min_tool_d", &rec)?;
    let mass = positive(t, "mass", &rec)?;
    let mut bodies = Vec::new();
    // coaxial rings: `body = [[r, from, to], ...]`, along +z from the origin (the turned form)
    if let Some(v) = t.get("body") {
        let Val::Arr(rows) = v else {
            return Err(bad(&rec, "body", "is a list of [r, from, to] rows"));
        };
        for row in rows {
            let Val::Arr(c) = row else {
                return Err(bad(&rec, "body", "is a list of [r, from, to] rows"));
            };
            let n: Vec<f64> = c
                .iter()
                .filter_map(|x| if let Val::Num(n) = x { Some(*n) } else { None })
                .collect();
            match n.as_slice() {
                [r, a, b]
                    if c.len() == 3
                        && r.is_finite()
                        && *r > 0.0
                        && a.is_finite()
                        && b.is_finite()
                        && a < b =>
                {
                    bodies.push(Prim::Cyl {
                        base: [0.0, 0.0, *a],
                        axis: [0.0, 0.0, 1.0],
                        r: *r,
                        h: b - a,
                    })
                }
                _ => return Err(bad(&rec, "body", "rows are [r > 0, from, to > from], mm")),
            }
        }
    }
    for c in tables(t, "cyl") {
        let base = vec3(c, "base", &rec)?.ok_or_else(|| missing(&rec, "base", "cyl: base = [x, y, z]"))?;
        let axis = vec3(c, "axis", &rec)?.ok_or_else(|| missing(&rec, "axis", "cyl: axis = [x, y, z]"))?;
        let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if n.is_nan() || n == 0.0 {
            return Err(bad(&rec, "axis", "a cylinder's axis needs a direction"));
        }
        let axis = [axis[0] / n, axis[1] / n, axis[2] / n];
        let r = positive(c, "r", &rec)?.ok_or_else(|| missing(&rec, "r", "cyl: r, the radius"))?;
        let h = positive(c, "h", &rec)?.ok_or_else(|| missing(&rec, "h", "cyl: h, the height"))?;
        bodies.push(Prim::Cyl { base, axis, r, h });
    }
    for b in tables(t, "box") {
        let min = vec3(b, "min", &rec)?.ok_or_else(|| missing(&rec, "min", "box: min = [x, y, z]"))?;
        let max = vec3(b, "max", &rec)?.ok_or_else(|| missing(&rec, "max", "box: max = [x, y, z]"))?;
        if (0..3).any(|i| min[i] >= max[i]) {
            return Err(bad(&rec, "box", "needs min < max on every axis"));
        }
        bodies.push(Prim::Box { min, max });
    }
    // rings first (as the writer writes them), each kind in its order
    bodies.sort_by_key(|p| p.ring().is_none());
    let mut offers = Vec::new();
    for o in tables(t, "offer") {
        let st = text(o, "status").unwrap_or("");
        if st != "standard" && st != "option" {
            return Err(bad(
                &rec,
                "offer",
                format!("status `{st}` is not standard | option"),
            ));
        }
        let machines = texts(o, "machines").unwrap_or_default();
        if machines.is_empty() {
            return Err(missing(
                &rec,
                "offer",
                "offer: machines = [\"<machine name>\", ...]",
            ));
        }
        let os = |k: &str| text(o, k).unwrap_or("").to_string();
        if os("source").is_empty() || os("taken").is_empty() {
            return Err(missing(&rec, "offer", "an offer cites `source` and `taken`"));
        }
        offers.push(Offer {
            machines,
            status: st.to_string(),
            source: os("source"),
            taken: os("taken"),
            note: os("note"),
        });
    }
    let d = Device {
        id,
        kind,
        vendor: s("vendor"),
        product: s("product"),
        note: s("note"),
        real,
        sensing,
        transmission,
        repeat,
        max_feed,
        overtravel,
        stylus,
        trigger,
        disc_d,
        min_tool_d,
        mass,
        bodies,
        offers,
        cites,
    };
    check_present(&d, &required, &rec)?;
    Ok(d)
}

/// A required value may be absent if and only if a cite names it `unknown`; a value a kind does
/// not use may not be given.
fn check_present(d: &Device, required: &[String], rec: &str) -> Result<(), LibError> {
    let present = |p: &str| match p {
        "transmission" => d.transmission.is_some(),
        "repeat" => d.repeat.is_some(),
        "max_feed" => d.max_feed.is_some(),
        "body" => !d.bodies.is_empty(),
        "stylus" => d.stylus.is_some(),
        "overtravel" => d.overtravel.is_some(),
        "trigger" => d.trigger.is_some(),
        "disc_d" => d.disc_d.is_some(),
        "min_tool_d" => d.min_tool_d.is_some(),
        _ => true,
    };
    for p in required {
        if !present(p) && d.status(p) != Some(CiteStatus::Unknown) {
            return Err(LibError::Bad {
                record: rec.to_string(),
                field: "cite",
                why: format!("`{p}` is absent: give it, or cite it with status = \"unknown\" and a basis"),
            });
        }
    }
    let foreign = [
        ("stylus", d.stylus.is_some()),
        ("overtravel", d.overtravel.is_some()),
        ("trigger", d.trigger.is_some()),
        ("disc_d", d.disc_d.is_some()),
        ("min_tool_d", d.min_tool_d.is_some()),
    ];
    for (k, given) in foreign {
        if given && !required.iter().any(|p| p == k) {
            return Err(LibError::Bad {
                record: rec.to_string(),
                field: "key",
                why: format!("`{k}` is not a field of a {} {}", d.sensing.word(), d.kind.word()),
            });
        }
    }
    Ok(())
}

/// Every `[[probe]]` record in a file's text; ids unique within it.
pub fn parse(src: &str) -> Result<Vec<Device>, LibError> {
    let t = toml::parse(src).map_err(|e| LibError::Toml {
        file: String::new(),
        line: e.line,
        what: e.what,
    })?;
    if let Some(k) = t.keys().find(|k| k.as_str() != "probe") {
        return Err(LibError::Bad {
            record: "file".into(),
            field: "key",
            why: format!("`{k}`: a probe file holds [[probe]] records only"),
        });
    }
    let mut out: Vec<Device> = Vec::new();
    for r in tables(&t, "probe") {
        let d = read(r)?;
        if out.iter().any(|x| x.id == d.id) {
            return Err(LibError::Duplicate {
                kind: "probe",
                name: d.id,
            });
        }
        out.push(d);
    }
    Ok(out)
}

/// The built-in files: (name, text).
pub const BUILTIN: [(&str, &str); 1] = [("probes.toml", include_str!("../data/probes.toml"))];

/// Every built-in device.
pub fn builtin() -> Vec<Device> {
    BUILTIN
        .iter()
        .flat_map(|(_, s)| parse(s).expect("the built-in probes read"))
        .collect()
}

/// A device by id from a list, or the ids nearest it.
pub fn find<'a>(all: &'a [Device], id: &str) -> Result<&'a Device, LibError> {
    all.iter().find(|d| d.id == id).ok_or_else(|| {
        let mut near: Vec<(usize, &str)> = all.iter().map(|d| (dist(&d.id, id), d.id.as_str())).collect();
        near.sort();
        LibError::Unknown {
            kind: "probe",
            name: id.to_string(),
            near: near.iter().take(3).map(|(_, s)| s.to_string()).collect(),
        }
    })
}

fn dist(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != b[j])).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

pub mod write;
