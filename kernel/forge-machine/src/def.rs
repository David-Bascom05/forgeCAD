//! The machine-definition format (docs/cam-machine.md): a machine tool as a kinematic tree of
//! joints from the static base, with spindles, turrets, a tool changer, mounts where tools and
//! parts attach, and collision bodies, in our TOML subset (forge_tooling::toml).
//!
//! Geometry is given in the base frame with every joint at 0 ("home pose"); a joint moves its
//! link and everything below it. A linear joint at value q translates by q·axis (mm); a rotary
//! joint at value q turns by q degrees about the line through `pivot` along `axis`,
//! right-handed. Machine and control are kept apart (VERICUT's model): `control` names a
//! forge-post dialect.
//!
//! Nothing is defaulted silently: a field a kind needs is a typed error naming the record, the
//! field and the fix. Optional fields say so and what their absence means.
use forge_tooling::toml::{self, Table, Val, boolean, num, nums, tables, text, texts};

/// What a definition can be wrong about. Every variant names where.
#[derive(Clone, Debug, PartialEq)]
pub enum DefError {
    Toml {
        line: usize,
        what: String,
    },
    Missing {
        record: String,
        field: &'static str,
        fix: String,
    },
    Bad {
        record: String,
        field: &'static str,
        why: String,
    },
    Unknown {
        record: String,
        field: &'static str,
        name: String,
        known: Vec<String>,
    },
    Duplicate {
        kind: &'static str,
        name: String,
    },
    /// The tree or the mounts do not make the machine its `kind` says.
    Shape {
        machine: String,
        why: String,
    },
}

impl std::fmt::Display for DefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefError::Toml { line, what } => write!(f, "line {line}: {what}"),
            DefError::Missing { record, field, fix } => write!(f, "{record}: missing `{field}`: {fix}"),
            DefError::Bad { record, field, why } => write!(f, "{record}: `{field}` {why}"),
            DefError::Unknown {
                record,
                field,
                name,
                known,
            } => {
                write!(
                    f,
                    "{record}: `{field}` = \"{name}\" is not one of: {}",
                    known.join(", ")
                )
            }
            DefError::Duplicate { kind, name } => write!(f, "two {kind}s named `{name}`"),
            DefError::Shape { machine, why } => write!(f, "machine `{machine}`: {why}"),
        }
    }
}

/// The machine's kinematic type (libraries-machines-posts-feeds.md §1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 3-axis vertical (or horizontal) machining centre: three linear joints, no rotaries.
    Vmc3,
    /// 3 linear and one table rotary (a 4th axis).
    Vmc4,
    /// 5-axis trunnion: two table rotaries, the second carried by the first.
    TableTable,
    /// one head rotary, one table rotary.
    HeadTable,
    /// two head rotaries, the second carried by the first.
    HeadHead,
    /// a lathe: turret on X and Z (Y optional), a turning spindle carrying the part.
    Lathe,
    /// a lathe that also mills: a live or milling spindle, C on the main spindle, optional B head,
    /// optional sub-spindle.
    MillTurn,
    /// a horizontal machining centre: 3 linear and a table rotary (B), usually with pallets.
    Hmc,
    /// a gantry router or mill: 3 linear on the head side (a dual-drive gantry axis is one
    /// joint), up to two head rotaries, no table rotary.
    Gantry,
    /// a live-turret lathe: X and Z (Y optional), C on the spindle optional, a turret whose live
    /// stations a milling spindle drives; no B head.
    TurnMill,
}

impl Kind {
    pub const ALL: [(&'static str, Kind); 10] = [
        ("vmc3", Kind::Vmc3),
        ("vmc4", Kind::Vmc4),
        ("table-table", Kind::TableTable),
        ("head-table", Kind::HeadTable),
        ("head-head", Kind::HeadHead),
        ("lathe", Kind::Lathe),
        ("mill-turn", Kind::MillTurn),
        ("turn-mill", Kind::TurnMill),
        ("hmc", Kind::Hmc),
        ("gantry", Kind::Gantry),
    ];
    /// A lathe of some kind: X is a diameter, tools on a turret, plane G18 at the start.
    pub fn turns(self) -> bool {
        matches!(self, Kind::Lathe | Kind::MillTurn | Kind::TurnMill)
    }

    pub fn name(self) -> &'static str {
        Kind::ALL.iter().find(|(_, k)| *k == self).unwrap().0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// On the path from the base to a tool mount.
    Head,
    /// On the path from the base to a part mount.
    Table,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointType {
    /// ISO 10303-105 prismatic pair: translation along `axis`, mm.
    Linear,
    /// ISO 10303-105 revolute pair: rotation about `axis` through `pivot`, degrees.
    Rotary,
}

/// When a rotary is reset to 0 (createAxis's `reset`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reset {
    Never,
    OpStart,
    Rewind,
    Both,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub name: String,
    /// "base" or another joint's name.
    pub parent: String,
    pub side: Side,
    pub ty: JointType,
    /// The G-code word that commands it: X Y Z U V W A B C, or with a channel digit (Z2, C2).
    pub letter: String,
    /// Unit vector (checked: length 1 within 1e-12 after normalising the given vector).
    pub axis: [f64; 3],
    /// A point on a rotary's axis, base frame, home pose. [0, 0, 0] for a linear joint.
    pub pivot: [f64; 3],
    /// Travel, mm or deg. A cyclic rotary's range is its output range (no physical stop).
    /// None: omitted, cited `unknown` (overtravel is then not checked).
    pub range: Option<[f64; 2]>,
    pub home: f64,
    /// Max rapid rate, mm/min or deg/min. None: cited `unknown` (unbounded in t_min).
    pub rapid: Option<f64>,
    /// Max programmed feed, mm/min or deg/min. None: cited `unknown` (no feed clamp check).
    pub max_feed: Option<f64>,
    /// mm/s² or deg/s²; None: time estimates take infinite acceleration and say so.
    pub accel: Option<f64>,
    pub cyclic: bool,
    /// The control can hold the tool tip (TCP / RTCP) through this rotary.
    pub tcp: bool,
    /// Rotary preference at an op's start: -1, 0, 1.
    pub preference: i8,
    pub reset: Reset,
    /// A lathe's X: the word is a diameter, the joint moves its half.
    pub diameter: bool,
    /// Where the machine's zero (reference point) sits in the travel: +1 at the plus end, -1 the
    /// minus end (`ref = "plus" | "minus"`), as makers publish; None: not stated.
    pub reference: Option<i8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpindleKind {
    /// Turns a cutter: a milling head or a live-tool drive.
    Milling,
    /// Turns the part: a lathe's main or sub spindle.
    Turning,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spindle {
    pub name: String,
    /// The link it rides on: "base" or a joint name.
    pub link: String,
    pub kind: SpindleKind,
    /// Spin axis (unit), base frame, home pose; for a turning spindle, the part's Z.
    pub axis: [f64; 3],
    /// A point on the spin axis: the nose (gauge line) for milling, the spindle face for turning.
    pub nose: [f64; 3],
    /// [min, max]; None: cited `unknown` (no rpm check).
    pub rpm: Option<[f64; 2]>,
    /// (rpm, kW) piecewise linear; empty: no power data (feeds then cannot check power).
    pub power: Vec<[f64; 2]>,
    /// Max tool length and diameter (milling), or max swing diameter and part length (turning).
    /// None: cited `unknown`.
    pub max: Option<[f64; 2]>,
    /// Taper or interface, e.g. "BT40", "A2-6"; informational.
    pub interface: String,
    /// A joint whose letter positions this spindle as a C axis (mill-turn), if any.
    pub c_joint: Option<String>,
    /// Duty ratings as makers publish them: (kW, duty) with duty "cont", "30min", "10min",
    /// "1min", "S3-25%", ... (`ratings_kw` / `ratings_duty`). Empty: none given.
    pub ratings: Vec<(f64, String)>,
    /// Torque, N·m: [max, continuous] (`torque_nm`); None: not given.
    pub torque_nm: Option<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Turret {
    pub name: String,
    pub link: String,
    /// None: cited `unknown` (T numbers not checked).
    pub stations: Option<u32>,
    /// Seconds per station index. None: cited `unknown` (0 in t_min).
    pub index_s: Option<f64>,
    /// Station face centre of station 1 and the turret's index axis (unit), home pose.
    pub center: [f64; 3],
    pub index_axis: [f64; 3],
    /// Live tooling: the milling spindle that drives it, if any.
    pub live: Option<String>,
    /// A manual tool post (`[[machine.toolpost]]`: quick-change holders, a Sherline's post): one
    /// station, every T a swap by the operator (a program stop), any T number.
    pub manual: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Changer {
    /// A manual change (`kind = "manual"`): M6 stops the program and the operator swaps the
    /// tool; there are no pockets (no pocket check). Automatic otherwise.
    pub manual: bool,
    /// None: cited `unknown` (T numbers not checked).
    pub pockets: Option<u32>,
    /// Chip-to-chip tool change time, seconds. None: cited `unknown` (0 in t_min).
    pub change_s: Option<f64>,
    pub max_tool_kg: Option<f64>,
    /// Where M6 moves the machine by itself before the swap (a Haas M06 sends Z to machine zero):
    /// (joint name, joint value), in the order given. Empty: not stated, and the sim says the
    /// change move was not checked.
    pub position: Vec<(String, f64)>,
}

/// An automatic pallet changer (an HMC's APC): M60 swaps the pallet in the machining position.
#[derive(Clone, Debug, PartialEq)]
pub struct Pallets {
    pub count: u32,
    /// Seconds per change. None: cited `unknown` (0 in t_min, reported not checked).
    pub change_s: Option<f64>,
    /// Max load per pallet, kg; pallet size [x, y] mm; the largest work [diameter, height] mm.
    pub max_kg: Option<f64>,
    pub size: Option<[f64; 2]>,
    pub max_work: Option<[f64; 2]>,
    /// Where M60 sends the machine before the swap: (joint, value), as the changer's position.
    pub position: Vec<(String, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MountKind {
    Tool,
    Part,
}

/// Where a tool or a part attaches.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    pub name: String,
    pub kind: MountKind,
    pub link: String,
    /// Tool mount: the gauge point (spindle nose centre); part mount: the fixture datum (table
    /// centre, chuck face centre). Base frame, home pose.
    pub at: [f64; 3],
    /// Tool mount: the direction from the gauge point to the tool tip (unit). Part mount: the
    /// part's +Z in the base frame (unit).
    pub dir: [f64; 3],
    /// Part mount: the part's +X (unit, orthogonal to dir). Tool mount: unused, [1, 0, 0].
    pub xdir: [f64; 3],
    /// Part mount: the most the table, pallet or rotary carries, kg, and its inertia limit,
    /// kg·m² (rotaries); None: not given (the load is then not checked, and replies say so).
    pub max_kg: Option<f64>,
    pub max_inertia: Option<f64>,
}

/// An analytic solid in a link's frame (home pose, base coordinates).
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// Axis-aligned box in the home pose.
    Box { min: [f64; 3], max: [f64; 3] },
    /// Solid cylinder: base centre, unit axis, radius, height (along axis).
    Cyl {
        base: [f64; 3],
        axis: [f64; 3],
        r: f64,
        h: f64,
    },
    /// Oriented box: the points o + s·axes[0] + t·axes[1] + r·axes[2], 0 ≤ s ≤ size[0], ...
    /// (axes orthonormal, checked on read): a workholding block placed at an angle.
    OBox {
        o: [f64; 3],
        axes: [[f64; 3]; 3],
        size: [f64; 3],
    },
}

impl Prim {
    /// An oriented box's point at fractions (a, b, c) of its sides.
    pub fn obox_at(o: &[f64; 3], axes: &[[f64; 3]; 3], size: &[f64; 3], f: [f64; 3]) -> [f64; 3] {
        let mut p = *o;
        for k in 0..3 {
            for i in 0..3 {
                p[i] += f[k] * size[k] * axes[k][i];
            }
        }
        p
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub name: String,
    pub link: String,
    /// The collision envelope; empty only when `body.NAME` is cited `unknown` (the body is then
    /// not collision-checked, and replies say so).
    pub prims: Vec<Prim>,
    /// The real part this body models (a `.fg` file beside the definition); its primitives are
    /// its collision envelope, checked to contain it (tests/parts.rs).
    pub part: Option<String>,
    /// Bodies it may touch by construction (neighbours in the chain, ways): not checked pairwise.
    pub ignore: Vec<String>,
    /// Its own least gap, mm, in place of the machine's `clearance` for any pair it is in (the
    /// smaller of two bodies' wins); None: the machine's.
    pub clearance: Option<f64>,
    /// The stock runs through it by construction (a Swiss guide bushing): stock against it is
    /// never a contact (`passes = "stock"`).
    pub passes_stock: bool,
}

/// A region of joint space the control forbids (Fanuc stored stroke check 2/3, an interference
/// zone): the open box min < q < max over the named joints, every other joint free. Machine data
/// stated per record (cite `zone.NAME`); the simulation reports a program entering it, and the
/// self-collision check of the record excludes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub name: String,
    pub joints: Vec<String>,
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    pub note: String,
}

impl Zone {
    /// Strictly inside, for joint values q in def.joints order (`idx`: the joints' indexes).
    pub fn contains(&self, idx: &[usize], q: &[f64]) -> bool {
        idx.iter()
            .enumerate()
            .all(|(k, &j)| self.min[k] < q[j] && q[j] < self.max[k])
    }
}

/// A spindle work probe the machine ships with or offers (`[[machine.probe]]`): a forge-probe
/// device record. In the spindle it is a tool (the setup gives its SimTool); cite `probe.NAME`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeSlot {
    pub name: String,
    /// The forge-probe device id.
    pub device: String,
    /// `standard` (ships with the machine) or `option`.
    pub offer: String,
    /// The T number / pocket it is kept in, if the record states one.
    pub t: Option<u32>,
}

/// A tool setter on the machine (`[[machine.setter]]`): a forge-probe device whose envelope is
/// the ordinary `[[machine.body]]` named `body`. `at` is the device's trigger point (the point a
/// tool is brought to) and `dir` the device's +z (away from the table), base frame, home pose;
/// cite `setter.NAME.at` and `.dir`.
#[derive(Clone, Debug, PartialEq)]
pub struct Setter {
    pub name: String,
    pub device: String,
    pub body: String,
    pub at: [f64; 3],
    pub dir: [f64; 3],
}

/// How a cited value stands (docs/cam-machine.md, "Provenance").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiteStatus {
    /// Stated by the source.
    Published,
    /// Not published: a conservative value derived as `basis` says (larger envelope, lower rate).
    Estimate,
    /// Nothing to derive it from: the value is a conservative stand-in, `basis` says which way.
    Unknown,
    /// Sources disagree: every value is cited, the record holds the conservative one.
    Conflict,
    /// The user's own value (measured or entered); source and taken optional.
    User,
}

/// Where one value of a machine record comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    /// The value cited: `joint.X.range`, `spindle.S1.rpm`, `changer.pockets`, `body.table`, ...
    /// (MachineDef::cite_paths lists those a real record must cite).
    pub path: String,
    pub status: CiteStatus,
    /// A URL or document (published, conflict); may be empty for estimate/unknown.
    pub source: String,
    /// When the source was read (YYYY-MM-DD); required with a source.
    pub taken: String,
    /// The source's own statement of the value, restated (e.g. "30 in (762 mm)").
    pub value: String,
    /// How an estimate was derived or why it is unknown; required for those.
    pub basis: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineDef {
    pub name: String,
    pub kind: Kind,
    /// A forge-post dialect name.
    pub control: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub joints: Vec<Joint>,
    pub spindles: Vec<Spindle>,
    pub turrets: Vec<Turret>,
    pub changer: Option<Changer>,
    pub pallets: Option<Pallets>,
    pub mounts: Vec<Mount>,
    pub bodies: Vec<Body>,
    /// The joint G43 length compensation moves (a mill's Z). None on a lathe (offsets are X/Z).
    pub length_joint: Option<String>,
    /// Coolant kinds present ("flood", "mist", "through", "air").
    pub coolants: Vec<String>,
    /// Least gap between two bodies the collision check holds, mm.
    pub clearance: f64,
    /// G83's peck clearance and G73's chip-break lift, mm (the control's parameters: Fanuc
    /// 5115/5114, Haas settings, ...). None: not stated; the simulation uses forge-post's and says
    /// so for every peck cycle.
    pub peck_clear: Option<f64>,
    pub chip_lift: Option<f64>,
    /// Forbidden regions of joint space the control enforces.
    pub zones: Vec<Zone>,
    /// Gang tool posts (`[[machine.gang]]`): each station is a tool mount named `GANG.T<n>`;
    /// T n selects it (no index, no stop).
    pub gangs: Vec<String>,
    /// Work probes the machine ships with or offers.
    pub probes: Vec<ProbeSlot>,
    /// Tool setters on the table.
    pub setters: Vec<Setter>,
    /// The maker's homing order (`[[machine.homing]] order`): groups of joints, each group homed
    /// together, the groups in turn (joints it does not name afterwards, one at a time, order not
    /// published); each joint toward its `ref` end. None: not published (a
    /// homing is then a generic reference return, and replies say so). Cite `homing`.
    pub homing: Option<Vec<Vec<String>>>,
    /// A record of a real maker's model: every value in `cite_paths` must be cited.
    pub real: bool,
    pub cites: Vec<Cite>,
}

fn rec_err(record: &str, field: &'static str, fix: &str) -> DefError {
    DefError::Missing {
        record: record.into(),
        field,
        fix: fix.into(),
    }
}

fn need_text<'a>(t: &'a Table, rec: &str, k: &'static str, fix: &str) -> Result<&'a str, DefError> {
    text(t, k).ok_or_else(|| rec_err(rec, k, fix))
}

fn need_num(t: &Table, rec: &str, k: &'static str, fix: &str) -> Result<f64, DefError> {
    let v = num(t, k).ok_or_else(|| rec_err(rec, k, fix))?;
    if !v.is_finite() {
        return Err(DefError::Bad {
            record: rec.into(),
            field: k,
            why: "is not finite".into(),
        });
    }
    Ok(v)
}

fn vec_n<const N: usize>(t: &Table, rec: &str, k: &'static str) -> Result<Option<[f64; N]>, DefError> {
    match t.get(k) {
        None => Ok(None),
        Some(_) => {
            let v = nums(t, k)
                .filter(|v| v.len() == N && v.iter().all(|x| x.is_finite()))
                .ok_or_else(|| DefError::Bad {
                    record: rec.into(),
                    field: k,
                    why: format!("must be a list of {N} finite numbers"),
                })?;
            let mut a = [0.0; N];
            a.copy_from_slice(&v);
            Ok(Some(a))
        }
    }
}

fn need_vec<const N: usize>(t: &Table, rec: &str, k: &'static str, fix: &str) -> Result<[f64; N], DefError> {
    vec_n::<N>(t, rec, k)?.ok_or_else(|| rec_err(rec, k, fix))
}

/// A direction, normalised; refused if zero.
fn unit(t: &Table, rec: &str, k: &'static str, fix: &str) -> Result<[f64; 3], DefError> {
    let a = need_vec::<3>(t, rec, k, fix)?;
    normalize(a).ok_or_else(|| DefError::Bad {
        record: rec.into(),
        field: k,
        why: "is the zero vector".into(),
    })
}

pub(crate) fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if n == 0.0 || !n.is_finite() {
        return None;
    }
    Some([a[0] / n, a[1] / n, a[2] / n])
}

fn pick<T: Copy>(t: &Table, rec: &str, k: &'static str, all: &[(&str, T)], fix: &str) -> Result<T, DefError> {
    let s = need_text(t, rec, k, fix)?;
    all.iter()
        .find(|(n, _)| *n == s)
        .map(|(_, v)| *v)
        .ok_or_else(|| DefError::Unknown {
            record: rec.into(),
            field: k,
            name: s.into(),
            known: all.iter().map(|(n, _)| n.to_string()).collect(),
        })
}

fn positive(rec: &str, k: &'static str, v: f64) -> Result<f64, DefError> {
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(DefError::Bad {
            record: rec.into(),
            field: k,
            why: format!("must be > 0 (is {v})"),
        })
    }
}

/// The paths a record cites `unknown`: those numbers may be omitted (docs, "Provenance").
fn unknown_cited(t: &Table) -> Vec<String> {
    tables(t, "cite")
        .iter()
        .filter(|c| text(c, "status") == Some("unknown"))
        .filter_map(|c| text(c, "path").map(str::to_string))
        .collect()
}

/// A required number, or None when it is absent and its path is cited `unknown`.
fn opt_num(
    t: &Table,
    rec: &str,
    k: &'static str,
    fix: &str,
    path: &str,
    unk: &[String],
) -> Result<Option<f64>, DefError> {
    if !t.contains_key(k) && unk.iter().any(|u| u == path) {
        return Ok(None);
    }
    need_num(t, rec, k, fix).map(Some)
}

fn range(t: &Table, rec: &str, k: &'static str, fix: &str) -> Result<[f64; 2], DefError> {
    let r = need_vec::<2>(t, rec, k, fix)?;
    if r[0] < r[1] {
        Ok(r)
    } else {
        Err(DefError::Bad {
            record: rec.into(),
            field: k,
            why: format!("needs lo < hi (is [{}, {}])", r[0], r[1]),
        })
    }
}

const LETTERS: [&str; 9] = ["X", "Y", "Z", "U", "V", "W", "A", "B", "C"];

impl Joint {
    fn from_table(t: &Table, m: &str, unk: &[String]) -> Result<Joint, DefError> {
        let name = need_text(t, &format!("{m} joint"), "name", "give `name`")?.to_string();
        let rec = format!("{m} joint `{name}`");
        let ty = pick(
            t,
            &rec,
            "type",
            &[("linear", JointType::Linear), ("rotary", JointType::Rotary)],
            "linear | rotary",
        )?;
        let side = pick(
            t,
            &rec,
            "side",
            &[("head", Side::Head), ("table", Side::Table)],
            "head | table",
        )?;
        let letter = text(t, "letter").unwrap_or(&name).to_string();
        let base = letter.trim_end_matches(|c: char| c.is_ascii_digit());
        if !LETTERS.contains(&base) || letter.len() > base.len() + 1 {
            return Err(DefError::Bad {
                record: rec,
                field: "letter",
                why: format!(
                    "`{letter}` is not an axis word (X Y Z U V W A B C, optionally with one channel digit)"
                ),
            });
        }
        let rot_letter = matches!(base, "A" | "B" | "C");
        if rot_letter != (ty == JointType::Rotary) {
            return Err(DefError::Bad {
                record: rec,
                field: "letter",
                why: format!(
                    "`{letter}` names a {} axis but the joint is {}",
                    if rot_letter { "rotary" } else { "linear" },
                    if ty == JointType::Rotary {
                        "rotary"
                    } else {
                        "linear"
                    }
                ),
            });
        }
        let axis = unit(t, &rec, "axis", "give `axis = [1, 0, 0]`")?;
        let pivot = match ty {
            JointType::Rotary => need_vec::<3>(
                t,
                &rec,
                "pivot",
                "a rotary needs `pivot = [x, y, z]`, a point on its axis (home pose)",
            )?,
            JointType::Linear => {
                if t.contains_key("pivot") {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "pivot",
                        why: "is for rotary joints only".into(),
                    });
                }
                [0.0; 3]
            }
        };
        let cyclic = boolean(t, "cyclic").unwrap_or(false);
        if cyclic && ty == JointType::Linear {
            return Err(DefError::Bad {
                record: rec,
                field: "cyclic",
                why: "is for rotary joints only".into(),
            });
        }
        let path = |f: &str| format!("joint.{name}.{f}");
        let range = if !t.contains_key("range") && unk.contains(&path("range")) {
            None
        } else {
            Some(range(
                t,
                &rec,
                "range",
                "give `range = [lo, hi]` (mm, or deg for a rotary), or cite it unknown",
            )?)
        };
        let home = num(t, "home").unwrap_or(0.0);
        if let Some(range) = range
            && !(range[0] <= home && home <= range[1])
        {
            return Err(DefError::Bad {
                record: rec,
                field: "home",
                why: format!("{home} is outside range [{}, {}]", range[0], range[1]),
            });
        }
        let rapid = match opt_num(
            t,
            &rec,
            "rapid",
            "give `rapid` (mm/min, deg/min), or cite it unknown",
            &path("rapid"),
            unk,
        )? {
            Some(v) => Some(positive(&rec, "rapid", v)?),
            None => None,
        };
        let max_feed = match opt_num(
            t,
            &rec,
            "max_feed",
            "give `max_feed` (mm/min, deg/min), or cite it unknown",
            &path("max_feed"),
            unk,
        )? {
            Some(v) => Some(positive(&rec, "max_feed", v)?),
            None => None,
        };
        let accel = match num(t, "accel") {
            Some(a) => Some(positive(&rec, "accel", a)?),
            None => None,
        };
        let preference = num(t, "preference").unwrap_or(0.0);
        if ![-1.0, 0.0, 1.0].contains(&preference) {
            return Err(DefError::Bad {
                record: rec,
                field: "preference",
                why: "must be -1, 0 or 1".into(),
            });
        }
        let reset = match text(t, "reset") {
            None => Reset::Never,
            Some(_) => pick(
                t,
                &rec,
                "reset",
                &[
                    ("never", Reset::Never),
                    ("op-start", Reset::OpStart),
                    ("rewind", Reset::Rewind),
                    ("both", Reset::Both),
                ],
                "",
            )?,
        };
        let diameter = boolean(t, "diameter").unwrap_or(false);
        if diameter && ty == JointType::Rotary {
            return Err(DefError::Bad {
                record: rec,
                field: "diameter",
                why: "is for a lathe's linear X".into(),
            });
        }
        let tcp = boolean(t, "tcp").unwrap_or(false);
        if tcp && ty == JointType::Linear {
            return Err(DefError::Bad {
                record: rec,
                field: "tcp",
                why: "is for rotary joints only".into(),
            });
        }
        Ok(Joint {
            parent: need_text(t, &rec, "parent", "give `parent` (\"base\" or a joint)")?.to_string(),
            side,
            ty,
            letter,
            axis,
            pivot,
            range,
            home,
            rapid,
            max_feed,
            accel,
            cyclic,
            tcp,
            preference: preference as i8,
            reset,
            diameter,
            reference: match text(t, "ref") {
                None => None,
                Some("plus") => Some(1),
                Some("minus") => Some(-1),
                Some(other) => {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "ref",
                        why: format!("`{other}`: plus | minus"),
                    });
                }
            },
            name,
        })
    }
}

fn prims(t: &Table, rec: &str) -> Result<Vec<Prim>, DefError> {
    let mut out = Vec::new();
    for b in tables(t, "box") {
        let min = need_vec::<3>(b, rec, "min", "a box needs `min = [x, y, z]`")?;
        let max = need_vec::<3>(b, rec, "max", "a box needs `max = [x, y, z]`")?;
        if (0..3).any(|i| min[i] >= max[i]) {
            return Err(DefError::Bad {
                record: rec.into(),
                field: "box",
                why: "needs min < max on every axis".into(),
            });
        }
        out.push(Prim::Box { min, max });
    }
    for c in tables(t, "cyl") {
        out.push(Prim::Cyl {
            base: need_vec::<3>(c, rec, "base", "a cyl needs `base = [x, y, z]`")?,
            axis: unit(c, rec, "axis", "a cyl needs `axis = [x, y, z]`")?,
            r: positive(rec, "r", need_num(c, rec, "r", "a cyl needs `r`")?)?,
            h: positive(rec, "h", need_num(c, rec, "h", "a cyl needs `h`")?)?,
        });
    }
    if out.is_empty() {
        return Err(rec_err(
            rec,
            "box",
            "a body needs at least one [[machine.body.box]] or [[machine.body.cyl]]",
        ));
    }
    Ok(out)
}

impl MachineDef {
    /// Every `[[machine]]` in a file.
    pub fn parse_all(src: &str) -> Result<Vec<MachineDef>, DefError> {
        let root = toml::parse(src).map_err(|e| DefError::Toml {
            line: e.line,
            what: e.what,
        })?;
        tables(&root, "machine")
            .iter()
            .map(MachineDef::from_table)
            .collect()
    }

    pub fn from_table(t: &Table) -> Result<MachineDef, DefError> {
        let name = need_text(t, "[[machine]]", "name", "give `name`")?.to_string();
        let m = format!("machine `{name}`");
        let kind = pick(
            t,
            &m,
            "kind",
            &Kind::ALL,
            "vmc3 | vmc4 | table-table | head-table | head-head | lathe | mill-turn | turn-mill | hmc | gantry",
        )?;
        let control = need_text(
            t,
            &m,
            "control",
            "name its dialect (forge-post): fanuc, haas, linuxcnc, grbl, ...",
        )?
        .to_string();
        let source = need_text(t, &m, "source", "say where the numbers come from")?.to_string();
        let unk = unknown_cited(t);
        let joints = tables(t, "joint")
            .iter()
            .map(|j| Joint::from_table(j, &m, &unk))
            .collect::<Result<Vec<_>, _>>()?;
        let mut spindles = Vec::new();
        for s in tables(t, "spindle") {
            let sn = need_text(s, &format!("{m} spindle"), "name", "give `name`")?.to_string();
            let rec = format!("{m} spindle `{sn}`");
            let spath = |f: &str| format!("spindle.{sn}.{f}");
            let rpm = if !s.contains_key("rpm") && unk.contains(&spath("rpm")) {
                None
            } else {
                Some(range(
                    s,
                    &rec,
                    "rpm",
                    "give `rpm = [min, max]`, or cite it unknown",
                )?)
            };
            let cr = nums(s, "curve_rpm").unwrap_or_default();
            let ck = nums(s, "curve_kw").unwrap_or_default();
            if cr.len() != ck.len() || cr.windows(2).any(|w| w[0] >= w[1]) {
                return Err(DefError::Bad {
                    record: rec,
                    field: "curve_rpm",
                    why: "and curve_kw must have equal lengths, rpm increasing".into(),
                });
            }
            spindles.push(Spindle {
                link: need_text(s, &rec, "link", "give `link` (\"base\" or the joint it rides on)")?
                    .to_string(),
                kind: pick(
                    s,
                    &rec,
                    "kind",
                    &[
                        ("milling", SpindleKind::Milling),
                        ("turning", SpindleKind::Turning),
                    ],
                    "milling | turning",
                )?,
                axis: unit(s, &rec, "axis", "give `axis`, the spin axis")?,
                nose: need_vec::<3>(
                    s,
                    &rec,
                    "nose",
                    "give `nose = [x, y, z]`, a point on the spin axis at the gauge line or face",
                )?,
                rpm,
                power: cr.iter().zip(&ck).map(|(a, b)| [*a, *b]).collect(),
                max: if !s.contains_key("max") && unk.contains(&spath("max")) {
                    None
                } else {
                    Some(need_vec::<2>(
                        s,
                        &rec,
                        "max",
                        "give `max = [length, diameter]` (milling) or `[swing, length]` (turning), or cite it unknown",
                    )?)
                },
                interface: text(s, "interface").unwrap_or("").to_string(),
                c_joint: text(s, "c_joint").map(str::to_string),
                ratings: {
                    let kw = nums(s, "ratings_kw").unwrap_or_default();
                    let duty = texts(s, "ratings_duty").unwrap_or_default();
                    if kw.len() != duty.len() || kw.iter().any(|k| k.is_nan() || *k <= 0.0) {
                        return Err(DefError::Bad {
                            record: rec,
                            field: "ratings_kw",
                            why: "and ratings_duty need one positive kW per duty".into(),
                        });
                    }
                    kw.into_iter().zip(duty).collect()
                },
                torque_nm: vec_n::<2>(s, &rec, "torque_nm")?,
                name: sn,
            });
        }
        let mut turrets = Vec::new();
        for s in tables(t, "turret") {
            let tn = need_text(s, &format!("{m} turret"), "name", "give `name`")?.to_string();
            let rec = format!("{m} turret `{tn}`");
            let tpath = |f: &str| format!("turret.{tn}.{f}");
            let st = opt_num(
                s,
                &rec,
                "stations",
                "give `stations`, or cite it unknown",
                &tpath("stations"),
                &unk,
            )?;
            if st.is_some_and(|st| st < 1.0 || st.fract() != 0.0) {
                return Err(DefError::Bad {
                    record: rec,
                    field: "stations",
                    why: "must be a positive whole number".into(),
                });
            }
            turrets.push(Turret {
                link: need_text(s, &rec, "link", "give `link`")?.to_string(),
                stations: st.map(|st| st as u32),
                index_s: match opt_num(
                    s,
                    &rec,
                    "index_s",
                    "give `index_s`, seconds per station, or cite it unknown",
                    &tpath("index_s"),
                    &unk,
                )? {
                    Some(v) => Some(positive(&rec, "index_s", v)?),
                    None => None,
                },
                center: need_vec::<3>(s, &rec, "center", "give `center`")?,
                index_axis: unit(s, &rec, "index_axis", "give `index_axis`")?,
                live: text(s, "live").map(str::to_string),
                manual: false,
                name: tn,
            });
        }
        for s in tables(t, "toolpost") {
            let tn = need_text(s, &format!("{m} toolpost"), "name", "give `name`")?.to_string();
            let rec = format!("{m} toolpost `{tn}`");
            turrets.push(Turret {
                link: need_text(s, &rec, "link", "give `link`")?.to_string(),
                stations: None,
                // the operator's swap: given, or 0 in t_min (and reported not checked)
                index_s: match num(s, "change_s") {
                    Some(v) => Some(positive(&rec, "change_s", v)?),
                    None => None,
                },
                center: need_vec::<3>(s, &rec, "center", "give `center`, the holder's datum")?,
                index_axis: [0.0, 0.0, 1.0],
                live: None,
                manual: true,
                name: tn,
            });
        }
        let pallets = match tables(t, "pallets") {
            [] => None,
            [p] => {
                let rec = format!("{m} pallets");
                let count = need_num(p, &rec, "count", "give `count`, the number of pallets")?;
                if count < 2.0 || count.fract() != 0.0 {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "count",
                        why: "must be a whole number, at least 2".into(),
                    });
                }
                let pj = texts(p, "position_joints").unwrap_or_default();
                let pv = nums(p, "position").unwrap_or_default();
                if pj.len() != pv.len() {
                    return Err(rec_err(
                        &rec,
                        "position",
                        "give one value per joint of `position_joints`",
                    ));
                }
                Some(Pallets {
                    count: count as u32,
                    change_s: match opt_num(
                        p,
                        &rec,
                        "change_s",
                        "give `change_s`, seconds per pallet change, or cite it unknown",
                        "pallets.change_s",
                        &unk,
                    )? {
                        Some(v) => Some(positive(&rec, "change_s", v)?),
                        None => None,
                    },
                    max_kg: match num(p, "max_kg") {
                        Some(v) => Some(positive(&rec, "max_kg", v)?),
                        None => None,
                    },
                    size: vec_n::<2>(p, &rec, "size")?,
                    max_work: vec_n::<2>(p, &rec, "max_work")?,
                    position: pj.into_iter().zip(pv).collect(),
                })
            }
            _ => {
                return Err(DefError::Shape {
                    machine: name,
                    why: "more than one [[machine.pallets]]".into(),
                });
            }
        };
        let changer = match tables(t, "changer") {
            [] => None,
            [c] => {
                let rec = format!("{m} changer");
                let manual = match text(c, "kind") {
                    None | Some("automatic") => false,
                    Some("manual") => true,
                    Some(other) => {
                        return Err(DefError::Bad {
                            record: rec,
                            field: "kind",
                            why: format!("`{other}`: automatic | manual"),
                        });
                    }
                };
                // a manual change has no pockets and no magazine weight limit
                let unk_m: Vec<String> = if manual {
                    let mut u = unk.clone();
                    u.push("changer.pockets".into());
                    u.push("changer.max_tool_kg".into());
                    u
                } else {
                    unk.clone()
                };
                let unk = &unk_m;
                let p = opt_num(
                    c,
                    &rec,
                    "pockets",
                    "give `pockets`, or cite it unknown",
                    "changer.pockets",
                    unk,
                )?;
                if p.is_some_and(|p| p < 1.0 || p.fract() != 0.0) {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "pockets",
                        why: "must be a positive whole number".into(),
                    });
                }
                let pos = |k: &'static str, v: Option<f64>| match v {
                    Some(v) => positive(&rec, k, v).map(Some),
                    None => Ok(None),
                };
                let pj = texts(c, "position_joints").unwrap_or_default();
                let pv = nums(c, "position").unwrap_or_default();
                if pj.len() != pv.len() {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "position",
                        why: format!(
                            "needs one value per joint of `position_joints` ({} joints, {} values)",
                            pj.len(),
                            pv.len()
                        ),
                    });
                }
                Some(Changer {
                    manual,
                    position: pj.into_iter().zip(pv).collect(),
                    pockets: p.map(|p| p as u32),
                    change_s: pos(
                        "change_s",
                        opt_num(
                            c,
                            &rec,
                            "change_s",
                            "give `change_s`, or cite it unknown",
                            "changer.change_s",
                            unk,
                        )?,
                    )?,
                    max_tool_kg: pos(
                        "max_tool_kg",
                        opt_num(
                            c,
                            &rec,
                            "max_tool_kg",
                            "give `max_tool_kg`, or cite it unknown",
                            "changer.max_tool_kg",
                            unk,
                        )?,
                    )?,
                })
            }
            _ => {
                return Err(DefError::Shape {
                    machine: name,
                    why: "more than one [[machine.changer]]".into(),
                });
            }
        };
        let mut mounts = Vec::new();
        for s in tables(t, "mount") {
            let mn = need_text(s, &format!("{m} mount"), "name", "give `name`")?.to_string();
            let rec = format!("{m} mount `{mn}`");
            let kind = pick(
                s,
                &rec,
                "kind",
                &[("tool", MountKind::Tool), ("part", MountKind::Part)],
                "tool | part",
            )?;
            let dir = unit(
                s,
                &rec,
                "dir",
                "give `dir` (tool: gauge to tip; part: the part's +Z)",
            )?;
            let xdir = match kind {
                MountKind::Tool => [1.0, 0.0, 0.0],
                MountKind::Part => {
                    let x = unit(s, &rec, "xdir", "a part mount needs `xdir`, the part's +X")?;
                    let d = x[0] * dir[0] + x[1] * dir[1] + x[2] * dir[2];
                    if d.abs() > 1e-12 {
                        return Err(DefError::Bad {
                            record: rec,
                            field: "xdir",
                            why: format!("must be orthogonal to dir (dot = {d})"),
                        });
                    }
                    x
                }
            };
            mounts.push(Mount {
                link: need_text(s, &rec, "link", "give `link`")?.to_string(),
                kind,
                at: need_vec::<3>(s, &rec, "at", "give `at = [x, y, z]`")?,
                dir,
                xdir,
                max_kg: match num(s, "max_kg") {
                    Some(v) => Some(positive(&rec, "max_kg", v)?),
                    None => None,
                },
                max_inertia: match num(s, "max_inertia") {
                    Some(v) => Some(positive(&rec, "max_inertia", v)?),
                    None => None,
                },
                name: mn,
            });
        }
        let mut gangs = Vec::new();
        for g in tables(t, "gang") {
            let gn = need_text(g, &format!("{m} gang"), "name", "give `name`")?.to_string();
            let rec = format!("{m} gang `{gn}`");
            let link = need_text(g, &rec, "link", "give `link`, the slide the tools ride on")?.to_string();
            let ts = nums(g, "t").ok_or_else(|| rec_err(&rec, "t", "give `t = [1, 2, ...]`"))?;
            let at = nums(g, "at")
                .ok_or_else(|| rec_err(&rec, "at", "give `at = [x, y, z, ...]`, three per tool"))?;
            let dir = nums(g, "dir")
                .ok_or_else(|| rec_err(&rec, "dir", "give `dir = [x, y, z, ...]`, three per tool"))?;
            if ts.is_empty() || at.len() != 3 * ts.len() || dir.len() != 3 * ts.len() {
                return Err(DefError::Bad {
                    record: rec,
                    field: "at",
                    why: "and dir need three numbers per T".into(),
                });
            }
            for (i, tv) in ts.iter().enumerate() {
                if *tv < 1.0 || tv.fract() != 0.0 {
                    return Err(rec_err(&rec, "t", "T numbers are positive whole numbers"));
                }
                let d = normalize([dir[3 * i], dir[3 * i + 1], dir[3 * i + 2]])
                    .ok_or_else(|| rec_err(&rec, "dir", "a tool direction must not be zero"))?;
                mounts.push(Mount {
                    name: format!("{gn}.T{}", *tv as u32),
                    kind: MountKind::Tool,
                    link: link.clone(),
                    at: [at[3 * i], at[3 * i + 1], at[3 * i + 2]],
                    dir: d,
                    xdir: [1.0, 0.0, 0.0],
                    max_kg: None,
                    max_inertia: None,
                });
            }
            gangs.push(gn);
        }
        let mut bodies = Vec::new();
        for s in tables(t, "body") {
            let bn = need_text(s, &format!("{m} body"), "name", "give `name`")?.to_string();
            let rec = format!("{m} body `{bn}`");
            let no_envelope = tables(s, "box").is_empty()
                && tables(s, "cyl").is_empty()
                && unk.contains(&format!("body.{bn}"));
            bodies.push(Body {
                link: need_text(s, &rec, "link", "give `link`")?.to_string(),
                prims: if no_envelope { Vec::new() } else { prims(s, &rec)? },
                part: text(s, "part").map(str::to_string),
                ignore: texts(s, "ignore").unwrap_or_default(),
                clearance: match num(s, "clearance") {
                    Some(c) if c >= 0.0 && c.is_finite() => Some(c),
                    Some(c) => {
                        return Err(DefError::Bad {
                            record: rec,
                            field: "clearance",
                            why: format!("must be >= 0 (is {c})"),
                        });
                    }
                    None => None,
                },
                passes_stock: match text(s, "passes") {
                    None => false,
                    Some("stock") => true,
                    Some(other) => {
                        return Err(DefError::Bad {
                            record: rec,
                            field: "passes",
                            why: format!("`{other}`: only \"stock\" runs through a body"),
                        });
                    }
                },
                name: bn,
            });
        }
        let clearance = match num(t, "clearance") {
            Some(c) if c >= 0.0 && c.is_finite() => c,
            Some(c) => {
                return Err(DefError::Bad {
                    record: m,
                    field: "clearance",
                    why: format!("must be >= 0 (is {c})"),
                });
            }
            None => {
                return Err(rec_err(
                    &m,
                    "clearance",
                    "give `clearance`, the least gap between machine bodies, mm",
                ));
            }
        };
        for k in ["rpm", "curve_rpm"] {
            if t.contains_key(k) {
                return Err(DefError::Bad {
                    record: m,
                    field: "rpm",
                    why: format!("`{k}` belongs on a [[machine.spindle]]"),
                });
            }
        }
        let mut cites = Vec::new();
        for c in tables(t, "cite") {
            let path = need_text(
                c,
                &format!("{m} cite"),
                "path",
                "give `path`, e.g. \"joint.X.range\"",
            )?
            .to_string();
            let rec = format!("{m} cite `{path}`");
            let status = pick(
                c,
                &rec,
                "status",
                &[
                    ("published", CiteStatus::Published),
                    ("estimate", CiteStatus::Estimate),
                    ("unknown", CiteStatus::Unknown),
                    ("conflict", CiteStatus::Conflict),
                    ("user", CiteStatus::User),
                ],
                "published | estimate | unknown | conflict | user",
            )?;
            let source = text(c, "source").unwrap_or("").to_string();
            let taken = text(c, "taken").unwrap_or("").to_string();
            let basis = text(c, "basis").unwrap_or("").to_string();
            let note = text(c, "note").unwrap_or("").to_string();
            match status {
                CiteStatus::Published | CiteStatus::Conflict if source.is_empty() => {
                    return Err(rec_err(
                        &rec,
                        "source",
                        "give `source`, the URL or document the value is from",
                    ));
                }
                CiteStatus::Estimate | CiteStatus::Unknown if basis.is_empty() => {
                    return Err(rec_err(
                        &rec,
                        "basis",
                        "say how the value was derived (estimate) or which way it errs (unknown)",
                    ));
                }
                CiteStatus::Conflict if note.is_empty() => {
                    return Err(rec_err(
                        &rec,
                        "note",
                        "say what the sources disagree on and which value the record holds",
                    ));
                }
                _ => {}
            }
            if !source.is_empty() && taken.is_empty() {
                return Err(rec_err(
                    &rec,
                    "taken",
                    "give `taken = \"YYYY-MM-DD\"`, when the source was read",
                ));
            }
            cites.push(Cite {
                value: text(c, "value").unwrap_or("").to_string(),
                path,
                status,
                source,
                taken,
                basis,
                note,
            });
        }
        let nonneg = |k: &'static str| -> Result<Option<f64>, DefError> {
            match num(t, k) {
                Some(v) if v >= 0.0 && v.is_finite() => Ok(Some(v)),
                Some(v) => Err(DefError::Bad {
                    record: m.clone(),
                    field: k,
                    why: format!("must be >= 0 mm (is {v})"),
                }),
                None => Ok(None),
            }
        };
        let mut zones = Vec::new();
        for z in tables(t, "zone") {
            let zn = need_text(z, &format!("{m} zone"), "name", "give `name`")?.to_string();
            let rec = format!("{m} zone `{zn}`");
            let joints = texts(z, "joints")
                .ok_or_else(|| rec_err(&rec, "joints", "give `joints = [\"X\", \"Z\"]`"))?;
            let min =
                nums(z, "min").ok_or_else(|| rec_err(&rec, "min", "give `min`, one value per joint"))?;
            let max =
                nums(z, "max").ok_or_else(|| rec_err(&rec, "max", "give `max`, one value per joint"))?;
            if joints.is_empty() || min.len() != joints.len() || max.len() != joints.len() {
                return Err(DefError::Bad {
                    record: rec,
                    field: "joints",
                    why: "needs at least one joint and one min and max per joint".into(),
                });
            }
            if min
                .iter()
                .zip(&max)
                .any(|(a, b)| a >= b || a.is_nan() || b.is_nan())
            {
                return Err(DefError::Bad {
                    record: rec,
                    field: "min",
                    why: "needs min < max for every joint".into(),
                });
            }
            zones.push(Zone {
                joints,
                min,
                max,
                note: text(z, "note").unwrap_or("").to_string(),
                name: zn,
            });
        }
        let mut probes = Vec::new();
        for p in tables(t, "probe") {
            let pn = need_text(p, &format!("{m} probe"), "name", "give `name`")?.to_string();
            let rec = format!("{m} probe `{pn}`");
            let device = need_text(p, &rec, "device", "give `device`, a forge-probe device id")?.to_string();
            let offer = need_text(p, &rec, "offer", "give `offer`: standard | option")?.to_string();
            if offer != "standard" && offer != "option" {
                return Err(DefError::Unknown {
                    record: rec,
                    field: "offer",
                    name: offer,
                    known: vec!["standard".into(), "option".into()],
                });
            }
            let tn = match num(p, "t") {
                None => None,
                Some(v) if v >= 1.0 && v.fract() == 0.0 && v <= u32::MAX as f64 => Some(v as u32),
                Some(_) => return Err(rec_err(&rec, "t", "`t` is a positive whole number")),
            };
            probes.push(ProbeSlot {
                name: pn,
                device,
                offer,
                t: tn,
            });
        }
        let mut setters = Vec::new();
        for p in tables(t, "setter") {
            let sn = need_text(p, &format!("{m} setter"), "name", "give `name`")?.to_string();
            let rec = format!("{m} setter `{sn}`");
            setters.push(Setter {
                device: need_text(p, &rec, "device", "give `device`, a forge-probe device id")?.to_string(),
                body: need_text(
                    p,
                    &rec,
                    "body",
                    "give `body`, the [[machine.body]] that is its envelope",
                )?
                .to_string(),
                at: need_vec::<3>(p, &rec, "at", "give `at`, the trigger point, home pose")?,
                dir: unit(p, &rec, "dir", "give `dir`, the device's up direction")?,
                name: sn,
            });
        }
        let homing = match tables(t, "homing") {
            [] => None,
            [h] => {
                let rec = format!("{m} homing");
                let Some(Val::Arr(groups)) = h.get("order") else {
                    return Err(rec_err(&rec, "order", "give `order = [[\"Z\"], [\"X\", \"Y\"]]`"));
                };
                let mut order = Vec::new();
                for g in groups {
                    let Val::Arr(js) = g else {
                        return Err(rec_err(&rec, "order", "each group is a list of joint names"));
                    };
                    let mut grp = Vec::new();
                    for j in js {
                        let Val::Str(j) = j else {
                            return Err(rec_err(&rec, "order", "each group is a list of joint names"));
                        };
                        grp.push(j.clone());
                    }
                    order.push(grp);
                }
                Some(order)
            }
            _ => {
                return Err(DefError::Shape {
                    machine: name,
                    why: "more than one [[machine.homing]]".into(),
                });
            }
        };
        let def = MachineDef {
            probes,
            setters,
            homing,
            zones,
            gangs,
            pallets,
            peck_clear: nonneg("peck_clear")?,
            chip_lift: nonneg("chip_lift")?,
            real: boolean(t, "real").unwrap_or(false),
            cites,
            kind,
            control,
            source,
            taken: text(t, "taken").unwrap_or("").to_string(),
            note: text(t, "note").unwrap_or("").to_string(),
            joints,
            spindles,
            turrets,
            changer,
            mounts,
            bodies,
            length_joint: text(t, "length_joint").map(str::to_string),
            coolants: texts(t, "coolants").unwrap_or_default(),
            clearance,
            name,
        };
        def.validate()?;
        def.validate_probing()?;
        def.validate_cites()?;
        Ok(def)
    }

    /// The values a `real = true` record must cite: per joint its range, rapid and max_feed (and
    /// accel and pivot when given), per spindle rpm, nose and max (and the power curve when
    /// given), per turret stations, index_s and center, the changer's three numbers, every
    /// mount's `at` and every body's envelope.
    pub fn cite_paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        for j in &self.joints {
            for f in ["range", "rapid", "max_feed"] {
                out.push(format!("joint.{}.{f}", j.name));
            }
            if j.accel.is_some() {
                out.push(format!("joint.{}.accel", j.name));
            }
            if j.ty == JointType::Rotary {
                out.push(format!("joint.{}.pivot", j.name));
            }
        }
        for s in &self.spindles {
            for f in ["rpm", "nose", "max"] {
                out.push(format!("spindle.{}.{f}", s.name));
            }
            if !s.power.is_empty() {
                out.push(format!("spindle.{}.curve", s.name));
            }
        }
        for t in &self.turrets {
            let fs: &[&str] = if t.manual {
                &["center"]
            } else {
                &["stations", "index_s", "center"]
            };
            for f in fs {
                out.push(format!("turret.{}.{f}", t.name));
            }
        }
        if let Some(c) = &self.changer {
            let fs: &[&str] = if c.manual {
                &["change_s"]
            } else {
                &["pockets", "change_s", "max_tool_kg"]
            };
            for f in fs {
                out.push(format!("changer.{f}"));
            }
        }
        if self.pallets.is_some() {
            for f in ["count", "change_s"] {
                out.push(format!("pallets.{f}"));
            }
        }
        for mt in &self.mounts {
            out.push(format!("mount.{}.at", mt.name));
        }
        for b in &self.bodies {
            out.push(format!("body.{}", b.name));
        }
        for p in &self.probes {
            out.push(format!("probe.{}", p.name));
        }
        for st in &self.setters {
            out.push(format!("setter.{}.at", st.name));
            out.push(format!("setter.{}.dir", st.name));
        }
        if self.homing.is_some() {
            out.push("homing".into());
        }
        out
    }

    /// The maker does not publish where this machine's own bodies can meet within travel (the
    /// record cites `zones` unknown): a contact between them is possible, not checked.
    pub fn zones_unpublished(&self) -> bool {
        self.cite_status("zones") == Some(CiteStatus::Unknown)
    }

    /// Probes, setters and the homing order: unique names; a setter's body exists and rides the
    /// table side (a table joint's link, or the base); the homing order names each joint at
    /// most once.
    pub fn validate_probing(&self) -> Result<(), DefError> {
        for (i, p) in self.probes.iter().enumerate() {
            if self.probes[..i].iter().any(|q| q.name == p.name) {
                return Err(DefError::Duplicate {
                    kind: "probe",
                    name: p.name.clone(),
                });
            }
        }
        for (i, st) in self.setters.iter().enumerate() {
            if self.setters[..i].iter().any(|q| q.name == st.name) {
                return Err(DefError::Duplicate {
                    kind: "setter",
                    name: st.name.clone(),
                });
            }
            let rec = format!("machine `{}` setter `{}`", self.name, st.name);
            let Some(b) = self.bodies.iter().find(|b| b.name == st.body) else {
                return Err(DefError::Unknown {
                    record: rec,
                    field: "body",
                    name: st.body.clone(),
                    known: self.bodies.iter().map(|b| b.name.clone()).collect(),
                });
            };
            let table_side = b.link == "base" || self.joint(&b.link).is_some_and(|j| j.side == Side::Table);
            if !table_side {
                return Err(DefError::Bad {
                    record: rec,
                    field: "body",
                    why: format!(
                        "rides `{}`, a head joint: a tool setter sits on the table side",
                        b.link
                    ),
                });
            }
        }
        if let Some(order) = &self.homing {
            let rec = format!("machine `{}` homing", self.name);
            let mut seen: Vec<&str> = Vec::new();
            for j in order.iter().flatten() {
                if self.joint(j).is_none() {
                    return Err(DefError::Unknown {
                        record: rec,
                        field: "order",
                        name: j.clone(),
                        known: self.joints.iter().map(|j| j.name.clone()).collect(),
                    });
                }
                if seen.contains(&j.as_str()) {
                    return Err(DefError::Bad {
                        record: rec,
                        field: "order",
                        why: format!("names `{j}` twice"),
                    });
                }
                seen.push(j);
            }
            if order.iter().any(|g| g.is_empty()) {
                return Err(DefError::Bad {
                    record: rec,
                    field: "order",
                    why: "has an empty group".into(),
                });
            }
        }
        Ok(())
    }

    /// A zone's joints as indexes into `joints` (validated to exist).
    pub fn zone_joints(&self, z: &Zone) -> Vec<usize> {
        z.joints.iter().filter_map(|j| self.joint_index(j)).collect()
    }

    /// The gang station tool `t` sits in, if any.
    pub fn gang_mount(&self, t: u32) -> Option<&Mount> {
        self.gangs.iter().find_map(|g| self.mount(&format!("{g}.T{t}")))
    }

    /// The zones q is strictly inside.
    pub fn zones_at(&self, q: &[f64]) -> Vec<&Zone> {
        self.zones
            .iter()
            .filter(|z| z.contains(&self.zone_joints(z), q))
            .collect()
    }

    /// How a value stands, from its cites: the weakest of them (unknown, then estimate, then
    /// conflict, then user, then published); None if uncited.
    pub fn cite_status(&self, path: &str) -> Option<CiteStatus> {
        let rank = |s: CiteStatus| match s {
            CiteStatus::Unknown => 0,
            CiteStatus::Estimate => 1,
            CiteStatus::Conflict => 2,
            CiteStatus::User => 3,
            CiteStatus::Published => 4,
        };
        self.cites
            .iter()
            .filter(|c| c.path == path)
            .map(|c| c.status)
            .min_by_key(|s| rank(*s))
    }

    /// Provenance: every cite names a value of this record; a `real` record cites every value
    /// of `cite_paths`.
    pub fn validate_cites(&self) -> Result<(), DefError> {
        let paths = self.cite_paths();
        // values a record may cite beyond the required ones
        let extra = |p: &str| -> bool {
            let mut it = p.splitn(3, '.');
            match (it.next(), it.next(), it.next()) {
                (Some("joint"), Some(n), Some(f)) => {
                    self.joint(n).is_some() && ["axis", "home", "accel", "pivot"].contains(&f)
                }
                (Some("spindle"), Some(n), Some(f)) => {
                    self.spindles.iter().any(|s| s.name == n) && ["axis", "interface", "curve"].contains(&f)
                }
                (Some("turret"), Some(n), Some(_)) => self.turrets.iter().any(|t| t.name == n),
                (Some("mount"), Some(n), Some(_)) => self.mount(n).is_some(),
                (Some("machine"), Some(_), None) => true,
                (Some("changer"), Some("position"), None) => self.changer.is_some(),
                (Some("zone"), Some(n), None) => self.zones.iter().any(|z| z.name == n),
                (Some("pallets"), Some(_), None) => self.pallets.is_some(),
                // where the machine's own bodies may meet in travel (crash zones), as a whole:
                // cited `unknown` when the maker does not publish them
                (Some("zones"), None, None) => true,
                _ => false,
            }
        };
        for c in &self.cites {
            if !paths.contains(&c.path) && !extra(&c.path) {
                return Err(DefError::Bad {
                    record: format!("machine `{}` cite `{}`", self.name, c.path),
                    field: "path",
                    why: format!("names no value of this record; cite one of: {}", paths.join(", ")),
                });
            }
        }
        if self.real {
            let missing: Vec<&String> = paths
                .iter()
                .filter(|p| !self.cites.iter().any(|c| &c.path == *p))
                .collect();
            if !missing.is_empty() {
                return Err(DefError::Bad {
                    record: format!("machine `{}`", self.name),
                    field: "real",
                    why: format!(
                        "a real machine cites every value; uncited: {}",
                        missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }
        Ok(())
    }

    pub fn joint(&self, name: &str) -> Option<&Joint> {
        self.joints.iter().find(|j| j.name == name)
    }
    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }
    pub fn by_letter(&self, letter: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.letter == letter)
    }
    pub fn mount(&self, name: &str) -> Option<&Mount> {
        self.mounts.iter().find(|j| j.name == name)
    }

    /// The joints from the base to `link`, base first; None if `link` is unknown.
    pub fn chain(&self, link: &str) -> Option<Vec<usize>> {
        let mut out = Vec::new();
        let mut cur = link;
        while cur != "base" {
            let i = self.joint_index(cur)?;
            if out.contains(&i) {
                return None;
            }
            out.push(i);
            cur = &self.joints[i].parent;
        }
        out.reverse();
        Some(out)
    }

    fn shape(&self, why: String) -> DefError {
        DefError::Shape {
            machine: self.name.clone(),
            why,
        }
    }

    /// The structural rules: unique names and letters, a tree rooted at the base, every link
    /// known, sides matching the mounts' chains, and the kind's joint and mount counts.
    pub fn validate(&self) -> Result<(), DefError> {
        let mut names: Vec<&str> = vec!["base"];
        for j in &self.joints {
            if names.contains(&j.name.as_str()) {
                return Err(DefError::Duplicate {
                    kind: "joint",
                    name: j.name.clone(),
                });
            }
            names.push(&j.name);
        }
        for (i, j) in self.joints.iter().enumerate() {
            if self.joints[..i].iter().any(|k| k.letter == j.letter) {
                return Err(DefError::Duplicate {
                    kind: "axis letter",
                    name: j.letter.clone(),
                });
            }
            if !names.contains(&j.parent.as_str()) {
                return Err(DefError::Unknown {
                    record: format!("joint `{}`", j.name),
                    field: "parent",
                    name: j.parent.clone(),
                    known: names.iter().map(|s| s.to_string()).collect(),
                });
            }
            if self.chain(&j.name).is_none() {
                return Err(self.shape(format!("joint `{}` is on a cycle", j.name)));
            }
        }
        let known = |rec: String, link: &str| -> Result<(), DefError> {
            if names.contains(&link) {
                Ok(())
            } else {
                Err(DefError::Unknown {
                    record: rec,
                    field: "link",
                    name: link.into(),
                    known: names.iter().map(|s| s.to_string()).collect(),
                })
            }
        };
        let mut seen: Vec<(&'static str, String)> = Vec::new();
        let mut uniq = |kind: &'static str, name: &str| -> Result<(), DefError> {
            if seen.iter().any(|(k, x)| *k == kind && x == name) {
                return Err(DefError::Duplicate {
                    kind,
                    name: name.into(),
                });
            }
            seen.push((kind, name.to_string()));
            Ok(())
        };
        for s in &self.spindles {
            uniq("spindle", &s.name)?;
            known(format!("spindle `{}`", s.name), &s.link)?;
            if let Some(c) = &s.c_joint {
                match self.joint(c) {
                    Some(j) if j.ty == JointType::Rotary => {}
                    _ => {
                        return Err(self.shape(format!(
                            "spindle `{}`: c_joint `{c}` is not a rotary joint",
                            s.name
                        )));
                    }
                }
            }
        }
        for t in &self.turrets {
            uniq("turret", &t.name)?;
            known(format!("turret `{}`", t.name), &t.link)?;
            if let Some(l) = &t.live
                && !self
                    .spindles
                    .iter()
                    .any(|s| &s.name == l && s.kind == SpindleKind::Milling)
            {
                return Err(self.shape(format!(
                    "turret `{}`: live = `{l}` is not a milling spindle",
                    t.name
                )));
            }
        }
        for mt in &self.mounts {
            uniq("mount", &mt.name)?;
            known(format!("mount `{}`", mt.name), &mt.link)?;
        }
        for b in &self.bodies {
            uniq("body", &b.name)?;
            known(format!("body `{}`", b.name), &b.link)?;
        }
        for b in &self.bodies {
            for i in &b.ignore {
                if !self.bodies.iter().any(|c| &c.name == i) {
                    return Err(self.shape(format!("body `{}` ignores unknown body `{i}`", b.name)));
                }
            }
        }
        if let Some(l) = &self.length_joint {
            match self.joint(l) {
                // head side (a VMC's Z) or table side (an HMC moving the table to the column):
                // the joint's value takes H either way, since a table joint's axis is the
                // table's motion for +word
                Some(j) if j.ty == JointType::Linear => {}
                _ => return Err(self.shape(format!("length_joint `{l}` must be a linear joint"))),
            }
        }
        // sides: every joint on a tool mount's chain is head, on a part mount's chain is table
        let tools: Vec<&Mount> = self.mounts.iter().filter(|m| m.kind == MountKind::Tool).collect();
        let parts: Vec<&Mount> = self.mounts.iter().filter(|m| m.kind == MountKind::Part).collect();
        if tools.is_empty() || parts.is_empty() {
            return Err(self.shape("needs at least one tool mount and one part mount".into()));
        }
        for (ms, side) in [(&tools, Side::Head), (&parts, Side::Table)] {
            for m in ms.iter() {
                for i in self.chain(&m.link).unwrap() {
                    if self.joints[i].side != side {
                        return Err(self.shape(format!(
                            "joint `{}` is side = {:?} but lies on mount `{}`'s chain",
                            self.joints[i].name, self.joints[i].side, m.name
                        )));
                    }
                }
            }
        }
        for j in &self.joints {
            let on_tool = tools.iter().any(|m| {
                self.chain(&m.link)
                    .unwrap()
                    .contains(&self.joint_index(&j.name).unwrap())
            });
            let on_part = parts.iter().any(|m| {
                self.chain(&m.link)
                    .unwrap()
                    .contains(&self.joint_index(&j.name).unwrap())
            });
            if !on_tool && !on_part {
                return Err(self.shape(format!("joint `{}` moves no mount (a dead branch)", j.name)));
            }
        }
        // the kind
        let count = |ty: JointType, side: Option<Side>| {
            self.joints
                .iter()
                .filter(|j| j.ty == ty && side.is_none_or(|s| j.side == s))
                .count()
        };
        let lin = count(JointType::Linear, None);
        let rot_h = count(JointType::Rotary, Some(Side::Head));
        let rot_t = count(JointType::Rotary, Some(Side::Table));
        let mill = self
            .spindles
            .iter()
            .filter(|s| s.kind == SpindleKind::Milling)
            .count();
        let turn = self
            .spindles
            .iter()
            .filter(|s| s.kind == SpindleKind::Turning)
            .count();
        let carried = |side: Side| {
            let r: Vec<&Joint> = self
                .joints
                .iter()
                .filter(|j| j.ty == JointType::Rotary && j.side == side)
                .collect();
            r.len() == 2 && (r[1].parent == r[0].name || r[0].parent == r[1].name)
        };
        let ok = match self.kind {
            Kind::Vmc3 => lin == 3 && rot_h + rot_t == 0 && mill >= 1 && turn == 0,
            Kind::Vmc4 | Kind::Hmc => lin == 3 && rot_h == 0 && rot_t == 1 && mill >= 1 && turn == 0,
            // a turning spindle on the table's C (a mill that also turns, Brother M140X2): its X
            // is a radius, the milling spindle holds the stationary tool
            Kind::TableTable => {
                let turns_on_c = |sp: &Spindle| {
                    sp.c_joint
                        .as_ref()
                        .and_then(|c| self.joint(c))
                        .is_some_and(|j| j.ty == JointType::Rotary && j.side == Side::Table)
                };
                lin == 3
                    && rot_h == 0
                    && rot_t == 2
                    && carried(Side::Table)
                    && mill >= 1
                    && self
                        .spindles
                        .iter()
                        .filter(|s| s.kind == SpindleKind::Turning)
                        .all(turns_on_c)
                    && turn <= 1
            }
            Kind::Gantry => {
                lin == 3
                    && rot_t == 0
                    && rot_h <= 2
                    && mill >= 1
                    && turn == 0
                    && self.joints.iter().all(|j| j.side == Side::Head)
            }
            Kind::HeadTable => lin == 3 && rot_h == 1 && rot_t == 1 && mill >= 1 && turn == 0,
            Kind::HeadHead => {
                lin == 3 && rot_h == 2 && rot_t == 0 && carried(Side::Head) && mill >= 1 && turn == 0
            }
            Kind::Lathe => {
                (2..=3).contains(&lin)
                    && rot_h + rot_t == 0
                    && turn >= 1
                    && mill == 0
                    && (!self.turrets.is_empty() || !self.gangs.is_empty())
            }
            Kind::MillTurn => lin >= 3 && turn >= 1 && mill >= 1 && rot_t >= 1,
            Kind::TurnMill => {
                (2..=3).contains(&lin)
                    && rot_h == 0
                    && turn >= 1
                    && mill >= 1
                    && self.turrets.iter().any(|t| t.live.is_some())
            }
        };
        if !ok {
            return Err(self.shape(format!(
                "kind {} does not fit: {lin} linear, {rot_h} head rotary, {rot_t} table rotary, {mill} milling and {turn} turning spindles, {} turrets",
                self.kind.name(),
                self.turrets.len()
            )));
        }
        if self.kind.turns() {
            if !self.joints.iter().any(|j| j.diameter) {
                return Err(self.shape("a lathe's X (diameter = true) is missing".into()));
            }
        } else if self.joints.iter().any(|j| j.diameter) {
            return Err(self.shape("diameter = true is for lathes".into()));
        }
        if !self.kind.turns() && self.length_joint.is_none() {
            return Err(self.shape("a mill needs `length_joint` (the joint G43 moves)".into()));
        }
        for z in &self.zones {
            if let Some(j) = z.joints.iter().find(|j| self.joint(j).is_none()) {
                return Err(self.shape(format!("zone `{}` names no joint `{j}`", z.name)));
            }
        }
        let positions = self
            .changer
            .iter()
            .flat_map(|c| c.position.iter().map(|p| ("changer", p)))
            .chain(
                self.pallets
                    .iter()
                    .flat_map(|p| p.position.iter().map(|q| ("pallets", q))),
            );
        for (what, (jn, v)) in positions {
            let Some(j) = self.joint(jn) else {
                return Err(self.shape(format!("the {what}' position names no joint `{jn}`")));
            };
            if let Some(r) = j.range
                && !(r[0] <= *v && *v <= r[1])
            {
                return Err(self.shape(format!(
                    "the {what}' position {jn} = {v} is outside its range [{}, {}]",
                    r[0], r[1]
                )));
            }
        }
        if let Some(c) = &self.changer {
            for (jn, v) in &c.position {
                let Some(j) = self.joint(jn) else {
                    return Err(self.shape(format!("the changer's position names no joint `{jn}`")));
                };
                if let Some(r) = j.range
                    && !(r[0] <= *v && *v <= r[1])
                {
                    return Err(self.shape(format!(
                        "the changer's position {jn} = {v} is outside its range [{}, {}]",
                        r[0], r[1]
                    )));
                }
            }
        }
        let rapids_ok = self.joints.iter().all(|j| match (j.max_feed, j.rapid) {
            (Some(f), Some(r)) => f <= r,
            _ => true,
        });
        if !rapids_ok {
            return Err(self.shape("a joint's max_feed exceeds its rapid".into()));
        }
        Ok(())
    }

    /// The milling spindle and joint data as forge-tooling's 3-axis Machine (what the 2.5D ops
    /// and the feeds engine read today).
    pub fn to_tooling(&self) -> Option<forge_tooling::Machine> {
        let s = self.spindles.iter().find(|s| s.kind == SpindleKind::Milling)?;
        Some(forge_tooling::Machine {
            name: self.name.clone(),
            kind: self.kind.name().to_string(),
            control: self.control.clone(),
            joints: self
                .joints
                .iter()
                .map(|j| forge_tooling::Joint {
                    name: j.letter.clone(),
                    axis: j.axis,
                    // stand-ins for unknowns: no travel limit, an unbounded rate
                    range: j.range.unwrap_or([f64::NEG_INFINITY, f64::INFINITY]),
                    rapid: j.rapid.unwrap_or(f64::INFINITY),
                })
                .collect(),
            rpm: s.rpm.unwrap_or([0.0, f64::INFINITY]),
            // no curve published but a continuous rating and torque: the conservative curve,
            // torque-limited to the base speed P / T, then the continuous power
            power: if s.power.is_empty() {
                match (
                    s.ratings.iter().find(|(_, d)| d == "cont").map(|(kw, _)| *kw),
                    s.torque_nm.map(|t| t[1]),
                    s.rpm.map(|r| r[1]),
                ) {
                    (Some(kw), Some(t), Some(top)) if t > 0.0 => {
                        let base = kw * 1000.0 * 60.0 / (2.0 * std::f64::consts::PI * t);
                        if base < top {
                            vec![[0.0, 0.0], [base, kw], [top, kw]]
                        } else {
                            vec![[0.0, 0.0], [top, t * 2.0 * std::f64::consts::PI * top / 60_000.0]]
                        }
                    }
                    _ => Vec::new(),
                }
            } else {
                s.power.clone()
            },
            max_feed: self
                .joints
                .iter()
                .filter(|j| j.ty == JointType::Linear)
                .map(|j| j.max_feed.unwrap_or(f64::INFINITY))
                .fold(f64::INFINITY, f64::min),
            coolants: self.coolants.clone(),
            source: self.source.clone(),
        })
    }
}
