//! CAM libraries as data, in our own TOML (research/cam/libraries-tools.md §5,
//! libraries-machines-posts-feeds.md §5): tools, holders, assemblies, machines and cutting data,
//! and the feeds engine.
//!
//! Every record carries `source`, `taken` and `note`. A required field that is missing is a
//! typed error naming the field and the fix, never a silent default. The built-in set is ours:
//! generic tools, conservative holders, a generic 3-axis machine and per-group cutting data, each
//! marked "verify with the vendor". Search order for a named library: the project's
//! `cam/` directory, then the user's, then the built-in set.
pub mod build;
pub mod cite;
pub mod feeds;
pub mod resolve;
pub mod toml;
pub mod turning;

pub use cite::{Cite, CiteStatus};

use forge_cam_geom::tool::{self as gtool, Cutter, Shape, ToolError};
use toml::{Table, num, tables, text};

#[derive(Clone, Debug, PartialEq)]
pub enum LibError {
    Toml {
        file: String,
        line: usize,
        what: String,
    },
    /// A record is missing a field its type needs: the record, the field, how to fix it.
    Missing {
        record: String,
        field: &'static str,
        fix: String,
    },
    /// A field has a value out of its range or of the wrong kind.
    Bad {
        record: String,
        field: &'static str,
        why: String,
    },
    Unknown {
        kind: &'static str,
        name: String,
        near: Vec<String>,
    },
    Duplicate {
        kind: &'static str,
        name: String,
    },
}

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LibError::Toml { file, line, what } => write!(f, "{file}:{line}: {what}"),
            LibError::Missing { record, field, fix } => write!(f, "{record}: missing `{field}` ({fix})"),
            LibError::Bad { record, field, why } => write!(f, "{record}: `{field}` {why}"),
            LibError::Unknown { kind, name, near } => {
                write!(f, "no {kind} `{name}`; near: {}", near.join(", "))
            }
            LibError::Duplicate { kind, name } => write!(f, "{kind} `{name}` defined twice"),
        }
    }
}

/// Tool types (ISO 13399-flavoured names, Fusion's set restricted to what 2.5D needs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Flat,
    Ball,
    Bull,
    Chamfer,
    Spot,
    Drill,
    Tap,
    ThreadMill,
    Reamer,
    Bore,
    Counterbore,
    Countersink,
    Slot,
    Engrave,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "flat" => Kind::Flat,
            "ball" => Kind::Ball,
            "bull" => Kind::Bull,
            "chamfer" => Kind::Chamfer,
            "spot" => Kind::Spot,
            "drill" => Kind::Drill,
            "tap" => Kind::Tap,
            "thread_mill" => Kind::ThreadMill,
            "reamer" => Kind::Reamer,
            "bore" => Kind::Bore,
            "counterbore" => Kind::Counterbore,
            "countersink" => Kind::Countersink,
            "slot" => Kind::Slot,
            "engrave" => Kind::Engrave,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Flat => "flat",
            Kind::Ball => "ball",
            Kind::Bull => "bull",
            Kind::Chamfer => "chamfer",
            Kind::Spot => "spot",
            Kind::Drill => "drill",
            Kind::Tap => "tap",
            Kind::ThreadMill => "thread_mill",
            Kind::Reamer => "reamer",
            Kind::Bore => "bore",
            Kind::Counterbore => "counterbore",
            Kind::Countersink => "countersink",
            Kind::Slot => "slot",
            Kind::Engrave => "engrave",
        }
    }
    /// Makes holes (axial moves only) rather than mills.
    pub fn is_hole_tool(self) -> bool {
        matches!(
            self,
            Kind::Spot | Kind::Drill | Kind::Tap | Kind::Reamer | Kind::Counterbore | Kind::Countersink
        )
    }
}

/// Cutting data stored on a tool, per material group (Fusion's presets).
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub name: String,
    pub group: String,
    pub vc: f64,
    pub fz: f64,
    pub ap: Option<f64>,
    pub ae: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub id: String,
    pub kind: Kind,
    /// Cutting diameter, mm.
    pub dc: f64,
    /// Corner radius (bull), mm.
    pub re: f64,
    /// Included angle (chamfer, spot, countersink, engrave, drill point), degrees.
    pub angle: f64,
    /// Tip diameter (chamfer, engrave), mm.
    pub tip_d: f64,
    /// Thread pitch (tap, thread mill), mm.
    pub tp: f64,
    pub nof: u32,
    /// Flute length.
    pub lcf: f64,
    /// Shank diameter.
    pub dcon: f64,
    pub oal: f64,
    pub hand: String,
    /// The substrate as the record writes it ("carbide", "Solid Carbide", "HSS-Co", ...):
    /// feeds::Substrate::of reads it.
    pub material: String,
    /// The coating as the record writes it ("AlTiN", "uncoated", ...; "" when not stated).
    pub coating: String,
    pub vendor: String,
    pub product_id: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub presets: Vec<Preset>,
    /// A real maker's product: every value cited (cite.rs).
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// Fields a tool may cite beyond the required ones.
const TOOL_EXTRA: &[&str] = &[
    "nof", "re", "angle", "sig", "tip_d", "tp", "hand", "material", "coating", "helix", "neck_d", "reach",
    "preset", "id", "type", "max_rpm",
];

fn need(t: &Table, rec: &str, k: &'static str) -> Result<f64, LibError> {
    num(t, k).ok_or_else(|| LibError::Missing {
        record: rec.to_string(),
        field: k,
        fix: format!("give `{k} = <mm>`"),
    })
}

fn positive(rec: &str, k: &'static str, v: f64) -> Result<f64, LibError> {
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(LibError::Bad {
            record: rec.to_string(),
            field: k,
            why: format!("must be > 0 (is {v})"),
        })
    }
}

impl Tool {
    pub fn from_table(t: &Table) -> Result<Tool, LibError> {
        let id = text(t, "id")
            .ok_or_else(|| LibError::Missing {
                record: "[[tool]]".into(),
                field: "id",
                fix: "give `id = \"...\"`".into(),
            })?
            .to_string();
        let rec = format!("tool `{id}`");
        let ty = text(t, "type").ok_or_else(|| LibError::Missing { record: rec.clone(), field: "type", fix: "give `type = \"flat\"` (or ball, bull, chamfer, spot, drill, tap, thread_mill, reamer, bore, counterbore, countersink, slot, engrave)".into() })?;
        let kind = Kind::parse(ty).ok_or_else(|| LibError::Bad {
            record: rec.clone(),
            field: "type",
            why: format!("`{ty}` is not a tool type"),
        })?;
        let dc = positive(&rec, "dc", need(t, &rec, "dc")?)?;
        let lcf = positive(&rec, "lcf", need(t, &rec, "lcf")?)?;
        let dcon = positive(&rec, "dcon", need(t, &rec, "dcon")?)?;
        let oal = positive(&rec, "oal", need(t, &rec, "oal")?)?;
        if oal <= lcf {
            return Err(LibError::Bad {
                record: rec,
                field: "oal",
                why: format!("{oal} is not longer than the flutes ({lcf})"),
            });
        }
        let re = if kind == Kind::Bull {
            positive(&rec, "re", need(t, &rec, "re")?)?
        } else {
            num(t, "re").unwrap_or(0.0)
        };
        let angle = match kind {
            Kind::Chamfer | Kind::Spot | Kind::Countersink | Kind::Engrave => {
                positive(&rec, "angle", need(t, &rec, "angle")?)?
            }
            Kind::Drill => positive(&rec, "sig", need(t, &rec, "sig")?)?,
            _ => num(t, "angle").unwrap_or(0.0),
        };
        let tp = match kind {
            Kind::Tap | Kind::ThreadMill => positive(&rec, "tp", need(t, &rec, "tp")?)?,
            _ => num(t, "tp").unwrap_or(0.0),
        };
        let mut presets = Vec::new();
        for p in tables(t, "preset") {
            presets.push(Preset {
                name: text(p, "name").unwrap_or("").to_string(),
                group: text(p, "group")
                    .ok_or_else(|| LibError::Missing {
                        record: format!("{rec} preset"),
                        field: "group",
                        fix: "give `group = \"N\"` (ISO P M K N S H)".into(),
                    })?
                    .to_string(),
                vc: need(p, &rec, "vc")?,
                fz: need(p, &rec, "fz")?,
                ap: num(p, "ap"),
                ae: num(p, "ae"),
            });
        }
        let mut required: Vec<String> = ["dc", "lcf", "dcon", "oal"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        match kind {
            Kind::Bull => required.push("re".into()),
            Kind::Chamfer | Kind::Spot | Kind::Countersink | Kind::Engrave => required.push("angle".into()),
            Kind::Drill => required.push("sig".into()),
            Kind::Tap | Kind::ThreadMill => required.push("tp".into()),
            _ => {}
        }
        if num(t, "nof").is_some() {
            required.push("nof".into());
        }
        for p in &presets {
            required.push(format!(
                "preset.{}",
                if p.name.is_empty() { &p.group } else { &p.name }
            ));
        }
        let (real, cites) = cite::read(t, &rec, &required, TOOL_EXTRA)?;
        let s = |k: &str| text(t, k).unwrap_or("").to_string();
        Ok(Tool {
            real,
            cites,
            kind,
            dc,
            re,
            angle,
            tip_d: num(t, "tip_d").unwrap_or(0.0),
            tp,
            nof: num(t, "nof").unwrap_or(if kind == Kind::Drill { 2.0 } else { 0.0 }) as u32,
            lcf,
            dcon,
            oal,
            hand: text(t, "hand").unwrap_or("right").to_string(),
            material: s("material"),
            coating: s("coating"),
            vendor: s("vendor"),
            product_id: s("product_id"),
            source: s("source"),
            taken: s("taken"),
            note: s("note"),
            presets,
            id,
        })
    }

    /// The cutting end as an exact profile type.
    pub fn cutter(&self) -> Cutter {
        match self.kind {
            Kind::Ball => Cutter::Ball { d: self.dc },
            Kind::Bull => Cutter::Bull {
                d: self.dc,
                re: self.re,
            },
            Kind::Chamfer | Kind::Spot | Kind::Countersink | Kind::Engrave => Cutter::Chamfer {
                d_tip: self.tip_d,
                angle: self.angle,
                d: self.dc,
            },
            Kind::Drill => Cutter::Drill {
                d: self.dc,
                angle: self.angle,
            },
            // taps, thread mills, reamers, bores, slot cutters: their flat envelope
            _ => Cutter::Flat { d: self.dc },
        }
    }

    /// The whole tool as exact bodies, held at `stickout` (tip to the holder's nose).
    pub fn shape(&self, stickout: f64, holder: Option<&Holder>) -> Result<Shape, ToolError> {
        let h = holder.map(|h| gtool::Holder {
            profile: h.profile.clone(),
        });
        gtool::assemble(&self.cutter(), self.lcf, self.dcon, stickout, h.as_ref())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Holder {
    pub id: String,
    pub interface: String,
    pub clamp: String,
    pub gauge_length: f64,
    /// [z from the nose, diameter], z rising.
    pub profile: Vec<[f64; 2]>,
    pub max_rpm: f64,
    pub source: String,
    pub note: String,
    pub vendor: String,
    pub product_id: String,
    pub taken: String,
    /// The collet nut this holder takes (a [[collet_nut]] id), when known.
    pub nut: Option<String>,
    /// A real maker's product: every value cited (cite.rs).
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// Fields a holder may cite beyond the required ones.
const HOLDER_EXTRA: &[&str] = &[
    "nut",
    "max_rpm",
    "balance",
    "coolant",
    "nose_d",
    "id",
    "pull_stud",
    "clamp_range",
];

impl Holder {
    pub fn from_table(t: &Table) -> Result<Holder, LibError> {
        let id = text(t, "id")
            .ok_or_else(|| LibError::Missing {
                record: "[[holder]]".into(),
                field: "id",
                fix: "give `id`".into(),
            })?
            .to_string();
        let rec = format!("holder `{id}`");
        let mut profile = Vec::new();
        match t.get("profile") {
            Some(toml::Val::Arr(v)) => {
                for p in v {
                    match p {
                        toml::Val::Arr(q) if q.len() == 2 => match (&q[0], &q[1]) {
                            (toml::Val::Num(z), toml::Val::Num(d)) => profile.push([*z, *d]),
                            _ => {
                                return Err(LibError::Bad {
                                    record: rec,
                                    field: "profile",
                                    why: "each ring is [z, d] numbers".into(),
                                });
                            }
                        },
                        _ => {
                            return Err(LibError::Bad {
                                record: rec,
                                field: "profile",
                                why: "each ring is [z, d]".into(),
                            });
                        }
                    }
                }
            }
            _ => {
                return Err(LibError::Missing {
                    record: rec,
                    field: "profile",
                    fix: "give `profile = [[z, d], ...]` from the nose up".into(),
                });
            }
        }
        if profile.len() < 2 || profile.windows(2).any(|w| w[1][0] < w[0][0]) || profile[0][0] != 0.0 {
            return Err(LibError::Bad {
                record: rec,
                field: "profile",
                why: "needs two or more rings from z = 0, z never falling".into(),
            });
        }
        let mut required: Vec<String> = ["interface", "clamp", "gauge_length", "profile"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if num(t, "max_rpm").is_some() {
            required.push("max_rpm".into());
        }
        let (real, cites) = cite::read(t, &rec, &required, HOLDER_EXTRA)?;
        Ok(Holder {
            real,
            cites,
            vendor: text(t, "vendor").unwrap_or("").to_string(),
            product_id: text(t, "product_id").unwrap_or("").to_string(),
            taken: text(t, "taken").unwrap_or("").to_string(),
            nut: text(t, "nut").map(str::to_string),
            gauge_length: need(t, &rec, "gauge_length")?,
            interface: text(t, "interface").unwrap_or("").to_string(),
            clamp: text(t, "clamp").unwrap_or("").to_string(),
            max_rpm: num(t, "max_rpm").unwrap_or(f64::INFINITY),
            source: text(t, "source").unwrap_or("").to_string(),
            note: text(t, "note").unwrap_or("").to_string(),
            profile,
            id,
        })
    }
}

/// A tool in a holder, in a pocket of the machine: what a program calls by T.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembly {
    pub t: u32,
    pub h: u32,
    pub d: u32,
    pub tool: String,
    pub holder: Option<String>,
    pub stickout: f64,
}

impl Assembly {
    pub fn from_table(t: &Table) -> Result<Assembly, LibError> {
        let tn = num(t, "t").ok_or_else(|| LibError::Missing {
            record: "[[assembly]]".into(),
            field: "t",
            fix: "give the tool number `t = 1`".into(),
        })?;
        let rec = format!("assembly T{tn}");
        let tool = text(t, "tool")
            .ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: "tool",
                fix: "name a [[tool]] id".into(),
            })?
            .to_string();
        let stickout = positive(&rec, "stickout", need(t, &rec, "stickout")?)?;
        Ok(Assembly {
            t: tn as u32,
            h: num(t, "h").unwrap_or(tn) as u32,
            d: num(t, "d").unwrap_or(tn) as u32,
            tool,
            holder: text(t, "holder").map(str::to_string),
            stickout,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub name: String,
    pub axis: [f64; 3],
    pub range: [f64; 2],
    /// mm/min.
    pub rapid: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Machine {
    pub name: String,
    pub kind: String,
    /// The dialect its control speaks (forge-post).
    pub control: String,
    pub joints: Vec<Joint>,
    pub rpm: [f64; 2],
    /// Spindle power curve: (rpm, kW) points, piecewise linear.
    pub power: Vec<[f64; 2]>,
    pub max_feed: f64,
    pub coolants: Vec<String>,
    pub source: String,
}

impl Machine {
    pub fn from_table(t: &Table) -> Result<Machine, LibError> {
        let name = text(t, "name")
            .ok_or_else(|| LibError::Missing {
                record: "[[machine]]".into(),
                field: "name",
                fix: "give `name`".into(),
            })?
            .to_string();
        let rec = format!("machine `{name}`");
        let mut joints = Vec::new();
        for j in tables(t, "joint") {
            let jn = text(j, "name").unwrap_or("?").to_string();
            let axis = toml::nums(j, "axis")
                .filter(|v| v.len() == 3)
                .ok_or_else(|| LibError::Missing {
                    record: format!("{rec} joint {jn}"),
                    field: "axis",
                    fix: "give `axis = [1, 0, 0]`".into(),
                })?;
            let range = toml::nums(j, "range")
                .filter(|v| v.len() == 2 && v[0] < v[1])
                .ok_or_else(|| LibError::Missing {
                    record: format!("{rec} joint {jn}"),
                    field: "range",
                    fix: "give `range = [lo, hi]`".into(),
                })?;
            joints.push(Joint {
                name: jn,
                axis: [axis[0], axis[1], axis[2]],
                range: [range[0], range[1]],
                rapid: num(j, "rapid").unwrap_or(10_000.0),
            });
        }
        let rpm = toml::nums(t, "rpm")
            .filter(|v| v.len() == 2)
            .ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: "rpm",
                fix: "give `rpm = [min, max]`".into(),
            })?;
        let cr = toml::nums(t, "curve_rpm").unwrap_or_default();
        let ck = toml::nums(t, "curve_kw").unwrap_or_default();
        Ok(Machine {
            kind: text(t, "kind").unwrap_or("vmc3").to_string(),
            control: text(t, "control")
                .ok_or_else(|| LibError::Missing {
                    record: rec.clone(),
                    field: "control",
                    fix: "name its dialect: fanuc, haas, linuxcnc or grbl".into(),
                })?
                .to_string(),
            joints,
            rpm: [rpm[0], rpm[1]],
            power: cr.iter().zip(ck.iter()).map(|(a, b)| [*a, *b]).collect(),
            max_feed: num(t, "max_feed").unwrap_or(10_000.0),
            coolants: toml::texts(t, "coolants").unwrap_or_default(),
            source: text(t, "source").unwrap_or("").to_string(),
            name,
        })
    }

    /// Spindle power available at an rpm (kW), from the curve; None without one.
    pub fn power_at(&self, rpm: f64) -> Option<f64> {
        let c = &self.power;
        if c.is_empty() {
            return None;
        }
        if rpm <= c[0][0] {
            return Some(c[0][1]);
        }
        for w in c.windows(2) {
            if rpm <= w[1][0] {
                let f = (rpm - w[0][0]) / (w[1][0] - w[0][0]);
                return Some(w[0][1] + f * (w[1][1] - w[0][1]));
            }
        }
        Some(c[c.len() - 1][1])
    }
}

/// Everything read from one or more library files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    pub tools: Vec<Tool>,
    pub holders: Vec<Holder>,
    pub assemblies: Vec<Assembly>,
    pub machines: Vec<Machine>,
    pub cutting: Vec<feeds::Group>,
    /// Cutting data for specific stock materials (forge-material names), over their group's.
    pub stock: Vec<feeds::Stock>,
    pub vises: Vec<Vise>,
    pub jaw_blanks: Vec<JawBlank>,
    pub collets: Vec<Collet>,
    pub collet_nuts: Vec<ColletNut>,
    pub pull_studs: Vec<PullStud>,
    pub inserts: Vec<turning::Insert>,
    pub turn_holders: Vec<turning::TurnHolder>,
    /// Each record's own table as read: (TOML key, id, table), for `build::derive`.
    pub sources: Vec<(String, String, Table)>,
}

/// A tool-side collet (ER, TG, R8, ...): the bore it is named for and the range it clamps.
#[derive(Clone, Debug, PartialEq)]
pub struct Collet {
    pub id: String,
    /// The collet system: ER11, ER16, ER20, ER25, ER32, ER40, TG100, R8, ...
    pub system: String,
    /// Nominal bore, mm.
    pub bore: f64,
    /// [min, max] shank diameter it clamps, mm (absent: the bore only).
    pub clamp: Option<[f64; 2]>,
    pub length: Option<f64>,
    pub od: Option<f64>,
    pub vendor: String,
    pub product_id: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// A collet nut: its system, outside diameter, length and thread.
#[derive(Clone, Debug, PartialEq)]
pub struct ColletNut {
    pub id: String,
    pub system: String,
    pub od: f64,
    pub length: f64,
    pub thread: String,
    pub vendor: String,
    pub product_id: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// A pull stud (retention knob) for a taper interface.
#[derive(Clone, Debug, PartialEq)]
pub struct PullStud {
    pub id: String,
    pub interface: String,
    pub thread: String,
    pub length: f64,
    /// The knob's angle as the source states it (degrees), if given.
    pub angle: Option<f64>,
    pub coolant: bool,
    pub vendor: String,
    pub product_id: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

fn rec_id(t: &Table, kind: &str) -> Result<String, LibError> {
    Ok(text(t, "id")
        .ok_or_else(|| LibError::Missing {
            record: format!("[[{kind}]]"),
            field: "id",
            fix: "give `id = \"...\"`".into(),
        })?
        .to_string())
}

fn s_of(t: &Table, k: &str) -> String {
    text(t, k).unwrap_or("").to_string()
}

impl Collet {
    pub fn from_table(t: &Table) -> Result<Collet, LibError> {
        let id = rec_id(t, "collet")?;
        let rec = format!("collet `{id}`");
        let system = text(t, "system")
            .ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: "system",
                fix: "give `system = \"ER20\"` (ER11 ... ER50, TG, R8, ...)".into(),
            })?
            .to_string();
        let bore = positive(&rec, "bore", need(t, &rec, "bore")?)?;
        let clamp = match toml::nums(t, "clamp") {
            Some(v) if v.len() == 2 && v[0] > 0.0 && v[0] <= v[1] => Some([v[0], v[1]]),
            Some(_) => {
                return Err(LibError::Bad {
                    record: rec,
                    field: "clamp",
                    why: "is [min, max] mm, 0 < min <= max".into(),
                });
            }
            None => None,
        };
        let mut required = vec!["system".to_string(), "bore".to_string()];
        for (k, v) in [
            ("clamp", clamp.is_some()),
            ("length", num(t, "length").is_some()),
            ("od", num(t, "od").is_some()),
        ] {
            if v {
                required.push(k.into());
            }
        }
        let (real, cites) = cite::read(t, &rec, &required, &["id", "runout", "collapse"])?;
        Ok(Collet {
            system,
            bore,
            clamp,
            length: num(t, "length"),
            od: num(t, "od"),
            vendor: s_of(t, "vendor"),
            product_id: s_of(t, "product_id"),
            source: s_of(t, "source"),
            taken: s_of(t, "taken"),
            note: s_of(t, "note"),
            real,
            cites,
            id,
        })
    }
}

impl ColletNut {
    pub fn from_table(t: &Table) -> Result<ColletNut, LibError> {
        let id = rec_id(t, "collet_nut")?;
        let rec = format!("collet nut `{id}`");
        let system = s_of(t, "system");
        if system.is_empty() {
            return Err(LibError::Missing {
                record: rec,
                field: "system",
                fix: "give `system = \"ER20\"`".into(),
            });
        }
        let od = positive(&rec, "od", need(t, &rec, "od")?)?;
        let length = positive(&rec, "length", need(t, &rec, "length")?)?;
        let required: Vec<String> = ["system", "od", "length", "thread"]
            .iter()
            .filter(|k| **k != "thread" || text(t, "thread").is_some())
            .map(|s| s.to_string())
            .collect();
        let (real, cites) = cite::read(t, &rec, &required, &["id", "thread"])?;
        Ok(ColletNut {
            system,
            od,
            length,
            thread: s_of(t, "thread"),
            vendor: s_of(t, "vendor"),
            product_id: s_of(t, "product_id"),
            source: s_of(t, "source"),
            taken: s_of(t, "taken"),
            note: s_of(t, "note"),
            real,
            cites,
            id,
        })
    }
}

impl PullStud {
    pub fn from_table(t: &Table) -> Result<PullStud, LibError> {
        let id = rec_id(t, "pull_stud")?;
        let rec = format!("pull stud `{id}`");
        let interface = s_of(t, "interface");
        if interface.is_empty() {
            return Err(LibError::Missing {
                record: rec,
                field: "interface",
                fix: "give `interface = \"CAT40\"` (BT30, BT40, ...)".into(),
            });
        }
        let length = positive(&rec, "length", need(t, &rec, "length")?)?;
        let mut required: Vec<String> = vec!["interface".into(), "length".into()];
        if text(t, "thread").is_some() {
            required.push("thread".into());
        }
        if num(t, "angle").is_some() {
            required.push("angle".into());
        }
        let (real, cites) = cite::read(t, &rec, &required, &["id", "coolant"])?;
        Ok(PullStud {
            interface,
            length,
            thread: s_of(t, "thread"),
            angle: num(t, "angle"),
            coolant: toml::boolean(t, "coolant").unwrap_or(false),
            vendor: s_of(t, "vendor"),
            product_id: s_of(t, "product_id"),
            source: s_of(t, "source"),
            taken: s_of(t, "taken"),
            note: s_of(t, "note"),
            real,
            cites,
            id,
        })
    }
}

/// A machine vise, generic: two jaws that close along the clamp axis on the stock, and a bed
/// (parallels) the stock sits on. All in mm.
#[derive(Clone, Debug, PartialEq)]
pub struct Vise {
    pub name: String,
    /// Along the jaw face (across the clamp axis).
    pub jaw_width: f64,
    /// From the bed (the stock's seat) to the jaw top.
    pub jaw_height: f64,
    /// Through the jaw, along the clamp axis.
    pub jaw_thickness: f64,
    pub max_opening: f64,
    /// The bed's depth below the stock's seat (modelled as one block between the jaws).
    pub bed_depth: f64,
    pub source: String,
}

/// A soft-jaw blank: a block machined to grip one part's outline.
#[derive(Clone, Debug, PartialEq)]
pub struct JawBlank {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub material: String,
    pub source: String,
    pub note: String,
}

impl Vise {
    pub fn from_table(t: &Table) -> Result<Vise, LibError> {
        let name = text(t, "name")
            .ok_or_else(|| LibError::Missing {
                record: "[[vise]]".into(),
                field: "name",
                fix: "give `name`".into(),
            })?
            .to_string();
        let rec = format!("vise `{name}`");
        let p = |k: &'static str| need(t, &rec, k).and_then(|v| positive(&rec, k, v));
        Ok(Vise {
            jaw_width: p("jaw_width")?,
            jaw_height: p("jaw_height")?,
            jaw_thickness: p("jaw_thickness")?,
            max_opening: p("max_opening")?,
            bed_depth: p("bed_depth")?,
            source: text(t, "source").unwrap_or("").to_string(),
            name,
        })
    }
}

impl JawBlank {
    pub fn from_table(t: &Table) -> Result<JawBlank, LibError> {
        let name = text(t, "name")
            .ok_or_else(|| LibError::Missing {
                record: "[[jaw_blank]]".into(),
                field: "name",
                fix: "give `name`".into(),
            })?
            .to_string();
        let rec = format!("jaw blank `{name}`");
        let p = |k: &'static str| need(t, &rec, k).and_then(|v| positive(&rec, k, v));
        Ok(JawBlank {
            width: p("width")?,
            height: p("height")?,
            thickness: p("thickness")?,
            material: text(t, "material").unwrap_or("").to_string(),
            source: text(t, "source").unwrap_or("").to_string(),
            note: text(t, "note").unwrap_or("").to_string(),
            name,
        })
    }
}

impl Library {
    /// Adds a file's records (later files may not redefine an id already present).
    pub fn add(&mut self, file: &str, src: &str) -> Result<(), LibError> {
        let t = toml::parse(src).map_err(|e| LibError::Toml {
            file: file.to_string(),
            line: e.line,
            what: e.what,
        })?;
        for r in tables(&t, "tool") {
            let x = Tool::from_table(r)?;
            if self.tools.iter().any(|o| o.id == x.id) {
                return Err(LibError::Duplicate {
                    kind: "tool",
                    name: x.id,
                });
            }
            self.sources.push(("tool".into(), x.id.clone(), r.clone()));
            self.tools.push(x);
        }
        for r in tables(&t, "holder") {
            let x = Holder::from_table(r)?;
            self.sources.push(("holder".into(), x.id.clone(), r.clone()));
            if self.holders.iter().any(|o| o.id == x.id) {
                return Err(LibError::Duplicate {
                    kind: "holder",
                    name: x.id,
                });
            }
            self.holders.push(x);
        }
        for r in tables(&t, "assembly") {
            let x = Assembly::from_table(r)?;
            if self.assemblies.iter().any(|o| o.t == x.t) {
                return Err(LibError::Duplicate {
                    kind: "assembly",
                    name: format!("T{}", x.t),
                });
            }
            self.assemblies.push(x);
        }
        for r in tables(&t, "machine") {
            self.machines.push(Machine::from_table(r)?);
        }
        for r in tables(&t, "group") {
            self.cutting.push(feeds::Group::from_table(r)?);
        }
        for r in tables(&t, "stock") {
            let x = feeds::Stock::from_table(r)?;
            if self.stock.iter().any(|o| o.material == x.material) {
                return Err(LibError::Duplicate {
                    kind: "stock",
                    name: x.material,
                });
            }
            self.stock.push(x);
        }
        for r in tables(&t, "vise") {
            self.vises.push(Vise::from_table(r)?);
        }
        for r in tables(&t, "jaw_blank") {
            self.jaw_blanks.push(JawBlank::from_table(r)?);
        }
        macro_rules! unique {
            ($key:literal, $vec:ident, $ty:ty, $kind:literal) => {
                for r in tables(&t, $key) {
                    let x = <$ty>::from_table(r)?;
                    self.sources.push(($key.into(), x.id.clone(), r.clone()));
                    if self.$vec.iter().any(|o| o.id == x.id) {
                        return Err(LibError::Duplicate {
                            kind: $kind,
                            name: x.id,
                        });
                    }
                    self.$vec.push(x);
                }
            };
        }
        unique!("collet", collets, Collet, "collet");
        unique!("collet_nut", collet_nuts, ColletNut, "collet nut");
        unique!("pull_stud", pull_studs, PullStud, "pull stud");
        unique!("insert", inserts, turning::Insert, "insert");
        unique!("turn_holder", turn_holders, turning::TurnHolder, "turning holder");
        Ok(())
    }

    /// The built-in library: ours, generic.
    pub fn builtin() -> Library {
        let mut l = Library::default();
        for (f, s) in BUILTIN {
            l.add(f, s).expect("the built-in library reads");
        }
        l
    }

    pub fn tool(&self, id: &str) -> Result<&Tool, LibError> {
        self.tools
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| LibError::Unknown {
                kind: "tool",
                name: id.to_string(),
                near: near(id, self.tools.iter().map(|t| t.id.as_str())),
            })
    }
    pub fn holder(&self, id: &str) -> Result<&Holder, LibError> {
        self.holders
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| LibError::Unknown {
                kind: "holder",
                name: id.to_string(),
                near: near(id, self.holders.iter().map(|t| t.id.as_str())),
            })
    }
    pub fn machine(&self, name: &str) -> Result<&Machine, LibError> {
        self.machines
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| LibError::Unknown {
                kind: "machine",
                name: name.to_string(),
                near: near(name, self.machines.iter().map(|t| t.name.as_str())),
            })
    }
    pub fn vise(&self, name: &str) -> Result<&Vise, LibError> {
        self.vises
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| LibError::Unknown {
                kind: "vise",
                name: name.to_string(),
                near: near(name, self.vises.iter().map(|t| t.name.as_str())),
            })
    }
    pub fn jaw_blank(&self, name: &str) -> Result<&JawBlank, LibError> {
        self.jaw_blanks
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| LibError::Unknown {
                kind: "jaw blank",
                name: name.to_string(),
                near: near(name, self.jaw_blanks.iter().map(|t| t.name.as_str())),
            })
    }
    pub fn group(&self, g: &str) -> Option<&feeds::Group> {
        self.cutting.iter().find(|x| x.group == g || x.name == g)
    }
    /// The cutting data for a stock material by its forge-material name.
    pub fn stock(&self, material: &str) -> Option<&feeds::Stock> {
        self.stock.iter().find(|x| x.material == material)
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

// BUILTIN: [(file, text)], generated by build.rs: the generic files (fixtures, tools, holders,
// machines, cutting) first, then every vendor file under data/{tools,holders,collets,turning}/.
include!(concat!(env!("OUT_DIR"), "/builtin.rs"));

/// The built-in library files, in reading order: (`builtin/<path>`, contents). The generic files
/// come first, then each vendor file (`builtin/tools/<vendor>.toml`, `builtin/holders/...`,
/// `builtin/collets/...`, `builtin/turning/...`), so a library builder can group records by file.
pub fn builtin_files() -> &'static [(&'static str, &'static str)] {
    BUILTIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_reads_and_shapes() {
        let l = Library::builtin();
        assert!(l.tools.len() >= 15 && l.holders.len() >= 3 && l.machines.len() == 2 && l.cutting.len() == 6);
        let m = l.machine("generic-vmc3").unwrap();
        assert_eq!(m.joints.len(), 3);
        assert_eq!(m.power_at(750.0), Some(5.5));
        for t in &l.tools {
            let h = l.holder("bt40-er32").unwrap();
            t.shape(t.lcf + 5.0, Some(h))
                .unwrap_or_else(|e| panic!("{}: {e:?}", t.id));
        }
        assert!(
            matches!(l.tool("flat-7"), Err(LibError::Unknown { near, .. }) if near.contains(&"flat-6".to_string()))
        );
        let mut m = Library::default();
        let e = m
            .add(
                "x.toml",
                "[[tool]]\nid = \"a\"\ntype = \"flat\"\ndc = 6\ndcon = 6\noal = 50\n",
            )
            .unwrap_err();
        assert!(matches!(e, LibError::Missing { field: "lcf", .. }), "{e:?}");
    }
}
