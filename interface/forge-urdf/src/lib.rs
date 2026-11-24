//! A built part or assembly as a URDF robot description (`forgecad export F urdf`): one ZIP of
//! `NAME.urdf` and `meshes/*.stl`. Design and every rule: research/export/urdf.md.
//!
//! A part is one link. An assembly is one link per rigid group of top-level instances (fastened
//! joints, lock constraints, and ground as one body), joined by its other joints, which must form
//! a tree: a closed loop is refused with its joints named, unless the caller cuts one (`cut J`).
//! Joint values are forge's own: a link's frame is the anchor frame of its parent joint on its
//! side, so URDF's `q` is the joint's value (angles rad, lengths m). Joint kinds URDF lacks
//! (cylindrical, pin-slot, limited planar, ball) are chains of single-axis joints through massless
//! links. Inertia is each solid's exact mass properties times its material's density, composed in
//! the link frame; a solid with no material is refused, never given a density.
//!
//! Output is deterministic: document order throughout, shortest round-trip number text, trig
//! through `forge_math::dmath`.

use forge_agent::json::Json;
use forge_asm::{Freedom, JointKind};
use forge_lang::Run;
use forge_lang::kin::{Kin, KinLink};
use forge_lang::props::{Piece, combine};
use forge_math::{Frame, Point3, Vec3, dmath};
use forge_mesh::scene::{Scene, Solids};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::path::Path;

// ---- options and errors -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The meshes' chord tolerance (mm) and angle (degrees), as `meshfile`'s.
    pub tol: f64,
    pub angle: f64,
    /// `<limit effort velocity>` for every moving joint: URDF requires them, forgeCAD has no
    /// actuator data. Each joint's own (by forge joint or URDF joint name), else these; a joint
    /// with neither is refused (`MissingLimits`) unless `placeholder`.
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    pub effort_of: BTreeMap<String, f64>,
    pub velocity_of: BTreeMap<String, f64>,
    /// Write 0 where effort or velocity is unknown, each such joint marked a placeholder.
    pub placeholder: bool,
    /// ± this travel (mm) for a translation with no limit; None: such a joint is refused.
    pub travel: Option<f64>,
    /// Joints (or loose constraints) left out, to open closed loops.
    pub cut: Vec<String>,
    /// Mesh paths as `package://NAME/meshes/..`; None: relative `meshes/..`.
    pub package: Option<String>,
    /// The root link: the group of this source instance (default: the grounded group).
    pub root: Option<String>,
    /// Joints made fixed at these values (forge units, the kind's freedoms in order).
    pub freeze: Vec<(String, Vec<f64>)>,
}

impl Default for Options {
    fn default() -> Options {
        let m = forge_lang::meshfile::Options::new(forge_lang::meshfile::Format::Stl);
        Options {
            tol: m.tol,
            angle: m.angle,
            effort: None,
            velocity: None,
            effort_of: BTreeMap::new(),
            velocity_of: BTreeMap::new(),
            placeholder: false,
            travel: None,
            cut: vec![],
            package: None,
            root: None,
            freeze: vec![],
        }
    }
}

impl Options {
    /// `urdf [tol MM] [angle DEG] [effort [J] N].. [velocity [J] V].. [placeholder-limits]
    /// [travel MM] [cut J].. [package NAME]`.
    pub fn parse(words: &[String]) -> Result<Options, String> {
        let mut o = Options::default();
        let mut it = words.iter().skip_while(|w| *w == "urdf");
        let num = |w: Option<&String>, what: &str| -> Result<f64, String> {
            w.and_then(|t| t.parse::<f64>().ok())
                .filter(|t| t.is_finite() && *t > 0.0)
                .ok_or(format!("{what} takes a positive number"))
        };
        while let Some(w) = it.next() {
            match w.as_str() {
                "tol" => o.tol = num(it.next(), "tol")?,
                "angle" => o.angle = num(it.next(), "angle")?,
                "effort" | "velocity" => {
                    let first = it.next().ok_or(format!("{w} takes [JOINT] N"))?;
                    let (joint, v) = match first.parse::<f64>() {
                        Ok(_) => (None, num(Some(first), w)?),
                        Err(_) => (Some(first.clone()), num(it.next(), w)?),
                    };
                    match (w.as_str(), joint) {
                        ("effort", None) => o.effort = Some(v),
                        ("effort", Some(j)) => _ = o.effort_of.insert(j, v),
                        (_, None) => o.velocity = Some(v),
                        (_, Some(j)) => _ = o.velocity_of.insert(j, v),
                    }
                }
                "placeholder-limits" | "--placeholder-limits" => o.placeholder = true,
                "travel" => o.travel = Some(num(it.next(), "travel")?),
                "cut" => o.cut.push(it.next().ok_or("cut takes a joint's name")?.clone()),
                "package" => o.package = Some(it.next().ok_or("package takes a ROS package name")?.clone()),
                _ => {
                    return Err(format!(
                        "export urdf: unknown word '{w}' (tol MM, angle DEG, effort [J] N, velocity [J] V, placeholder-limits, travel MM, cut JOINT, package NAME)"
                    ));
                }
            }
        }
        Ok(o)
    }
}

/// Why a part or assembly has no faithful URDF. Nothing is written.
#[derive(Clone, Debug, PartialEq)]
pub enum UrdfError {
    /// The part or assembly did not build or solve.
    NotBuilt(String),
    /// Solids with no material: a mass needs a density, and none is assumed.
    NoMaterial(Vec<String>),
    /// URDF's prismatic joint needs limits.
    UnlimitedTranslation {
        joint: String,
        freedom: Freedom,
    },
    /// Moving joints with no effort or velocity given: URDF needs both, and 0 reads as "cannot
    /// move" to a planner.
    MissingLimits {
        joints: Vec<String>,
    },
    /// A ball's rotation has no values, so no limits URDF could carry.
    BallLimits {
        joint: String,
    },
    /// The joints of a closed loop, in order round it.
    ClosedLoop {
        joints: Vec<String>,
    },
    /// A loose constraint between two moving groups: no URDF joint is one.
    LooseConstraint {
        id: String,
        kind: &'static str,
    },
    Relation {
        id: String,
        why: String,
    },
    /// `cut` named nothing the assembly has.
    NoSuchJoint(String),
    /// A joint end not placed (its anchor did not resolve).
    Unresolved {
        joint: String,
    },
    Mass(String),
    Mesh(String),
    /// A robot document's statement that cannot apply.
    Robot(String),
    /// Overrides whose recorded source value (`*_was`) the source no longer has: each needs a
    /// choice (keep, reset), none is applied or dropped silently.
    Conflicts(Vec<Conflict>),
}

/// An override against a source value that has changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    /// The robot document's line.
    pub line: usize,
    /// The statement (`joint(elbow)`) and the field (`limit`).
    pub what: String,
    pub field: String,
    /// The source's value when the override was made, now, and the override's.
    pub was: String,
    pub now: String,
    pub set: String,
}

impl UrdfError {
    pub fn kind(&self) -> &'static str {
        match self {
            UrdfError::NotBuilt(_) => "NotBuilt",
            UrdfError::NoMaterial(_) => "NoMaterial",
            UrdfError::UnlimitedTranslation { .. } => "UnlimitedTranslation",
            UrdfError::MissingLimits { .. } => "MissingLimits",
            UrdfError::BallLimits { .. } => "BallLimits",
            UrdfError::ClosedLoop { .. } => "ClosedLoop",
            UrdfError::LooseConstraint { .. } => "LooseConstraint",
            UrdfError::Relation { .. } => "Relation",
            UrdfError::NoSuchJoint(_) => "NoSuchJoint",
            UrdfError::Unresolved { .. } => "Unresolved",
            UrdfError::Mass(_) => "Mass",
            UrdfError::Mesh(_) => "Mesh",
            UrdfError::Robot(_) => "Robot",
            UrdfError::Conflicts(_) => "Conflicts",
        }
    }

    pub fn text(&self) -> String {
        match self {
            UrdfError::NotBuilt(m) | UrdfError::Mass(m) | UrdfError::Mesh(m) | UrdfError::Robot(m) => {
                m.clone()
            }
            UrdfError::NoMaterial(s) => format!(
                "no material on {}: a link's mass needs one (properties(material: ..)); no density is assumed",
                s.join(", ")
            ),
            UrdfError::UnlimitedTranslation { joint, freedom } => format!(
                "joint {joint}'s {} has no limit, and a URDF prismatic joint needs one: give the joint a limit, or export with `travel MM`",
                freedom.name()
            ),
            UrdfError::MissingLimits { joints } => format!(
                "no effort or velocity for {} (URDF needs both; forgeCAD has no actuator data, and 0 reads as cannot move): export with `effort N velocity V`, per joint `effort J N velocity J V`, or `placeholder-limits` to write 0 marked as placeholders",
                joints.join(" ")
            ),
            UrdfError::BallLimits { joint } => {
                format!("ball joint {joint} has limits, which URDF cannot carry")
            }
            UrdfError::ClosedLoop { joints } => format!(
                "the joints {} close a loop, and URDF is a tree: export with `cut J` for one of them (it is then left out, and said so)",
                joints.join(" ")
            ),
            UrdfError::LooseConstraint { id, kind } => format!(
                "loose constraint {id} ({kind}) ties two moving groups, and URDF has no joint that is one: make it a joint, or export with `cut {id}`"
            ),
            UrdfError::Relation { id, why } => format!("relation {id}: {why}"),
            UrdfError::NoSuchJoint(j) => format!("cut {j}: the assembly has no joint or constraint {j}"),
            UrdfError::Conflicts(cs) => cs
                .iter()
                .map(|c| {
                    format!(
                        "line {}: {}'s {} in the source went from {} to {}; the robot overrides it to {}",
                        c.line, c.what, c.field, c.was, c.now, c.set
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
            UrdfError::Unresolved { joint } => {
                format!("joint {joint}: an end is not placed (its anchor did not resolve)")
            }
        }
    }

    /// Export words that would get past it, where there are any.
    pub fn fixes(&self) -> Vec<String> {
        match self {
            UrdfError::ClosedLoop { joints } => joints.iter().map(|j| format!("cut {j}")).collect(),
            UrdfError::LooseConstraint { id, .. } => vec![format!("cut {id}")],
            UrdfError::UnlimitedTranslation { .. } => vec!["travel MM".into()],
            UrdfError::Conflicts(cs) => cs
                .iter()
                .flat_map(|c| {
                    [
                        format!("line {}: keep ({}_was: {})", c.line, c.field, c.now),
                        format!("line {}: reset ({} back to the source's)", c.line, c.field),
                    ]
                })
                .collect(),
            UrdfError::MissingLimits { joints } => {
                let mut f: Vec<String> = joints
                    .iter()
                    .map(|j| format!("effort {j} N velocity {j} V"))
                    .collect();
                f.insert(0, "effort N velocity V".into());
                f.push("placeholder-limits".into());
                f
            }
            _ => vec![],
        }
    }

    pub fn json(&self) -> Json {
        Json::obj(vec![
            ("kind", self.kind().into()),
            ("message", self.text().into()),
            (
                "fixes",
                Json::Arr(self.fixes().into_iter().map(Json::from).collect()),
            ),
        ])
    }
}

impl std::fmt::Display for UrdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind(), self.text())
    }
}

// ---- the result -------------------------------------------------------------------------------

/// One link as written: its mass (kg), centre of mass (m, link frame), and the bounds on them
/// from the solids' integration (first order; the composition's rounding is not included).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkOut {
    pub name: String,
    /// The bodies (top-level instances, or the part) it is.
    pub bodies: Vec<String>,
    pub mass: Option<f64>,
    pub com: [f64; 3],
    /// kg m², about the centre of mass in link axes.
    pub inertia: Option<[[f64; 3]; 3]>,
    /// Bounds: mass kg, centre of mass m, an inertia entry kg m². None: UNKNOWN.
    pub err: Option<[f64; 3]>,
    /// A chain's massless link.
    pub dummy: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JointOut {
    pub name: String,
    pub kind: &'static str,
    /// The forge joint (or relation's follower) it is part of.
    pub of: String,
    pub parent: String,
    pub child: String,
    /// Its value at the export pose (rad or m); a native planar's three, a floating joint's none.
    pub position: Vec<f64>,
    pub mimic: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Exported {
    pub name: String,
    pub urdf: String,
    /// `meshes/X.stl` and its bytes.
    pub meshes: Vec<(String, Vec<u8>)>,
    /// The ZIP: `NAME.urdf` and the meshes.
    pub zip: Vec<u8>,
    pub links: Vec<LinkOut>,
    pub joints: Vec<JointOut>,
    pub cut: Vec<String>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
    /// The largest gap (mm) between a link placed by the URDF's joints at the export pose and
    /// where the solve put it: the solve's own residual.
    pub residual: f64,
    /// Each mesh closed and manifold.
    pub watertight: bool,
    /// A robot document's overrides as applied: (line, statement, field, the source's value,
    /// the override's), as the document writes them; the source's is empty where the field has
    /// none to link to (effort, a robot-made geometry).
    pub overrides: Vec<(usize, String, String, String, String)>,
    /// The robot as an editor draws and drives it.
    pub view: RobotView,
}

impl Exported {
    pub fn json(&self) -> Json {
        let v3 = |v: [f64; 3]| Json::Arr(v.iter().map(|x| Json::Num(*x)).collect());
        let opt = |x: Option<f64>| x.map_or(Json::Null, Json::Num);
        let strs = |v: &[String]| Json::Arr(v.iter().map(|s| s.as_str().into()).collect());
        let links = self
            .links
            .iter()
            .map(|l| {
                Json::obj(vec![
                    ("name", l.name.as_str().into()),
                    ("bodies", strs(&l.bodies)),
                    ("mass", opt(l.mass)),
                    ("com", v3(l.com)),
                    (
                        "inertia",
                        l.inertia
                            .map_or(Json::Null, |m| Json::Arr(m.iter().map(|r| v3(*r)).collect())),
                    ),
                    (
                        "err",
                        l.err.map_or(Json::Null, |e| {
                            Json::obj(vec![
                                ("mass", Json::Num(e[0])),
                                ("com", Json::Num(e[1])),
                                ("inertia", Json::Num(e[2])),
                            ])
                        }),
                    ),
                    ("dummy", l.dummy.into()),
                ])
            })
            .collect();
        let joints = self
            .joints
            .iter()
            .map(|j| {
                Json::obj(vec![
                    ("name", j.name.as_str().into()),
                    ("type", j.kind.into()),
                    ("of", j.of.as_str().into()),
                    ("parent", j.parent.as_str().into()),
                    ("child", j.child.as_str().into()),
                    (
                        "position",
                        Json::Arr(j.position.iter().map(|x| Json::Num(*x)).collect()),
                    ),
                    ("mimic", j.mimic.as_deref().map_or(Json::Null, Json::from)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("ok", true.into()),
            ("format", "urdf".into()),
            ("robot", self.name.as_str().into()),
            ("bytes", self.zip.len().into()),
            (
                "files",
                Json::Arr(
                    std::iter::once(format!("{}.urdf", self.name))
                        .chain(self.meshes.iter().map(|m| m.0.clone()))
                        .map(Json::from)
                        .collect(),
                ),
            ),
            ("links", Json::Arr(links)),
            ("joints", Json::Arr(joints)),
            ("cut", strs(&self.cut)),
            ("notes", strs(&self.notes)),
            ("warnings", strs(&self.warnings)),
            ("residual", Json::Num(self.residual)),
            ("watertight", self.watertight.into()),
            (
                "overrides",
                Json::Arr(
                    self.overrides
                        .iter()
                        .map(|(line, what, field, linked, set)| {
                            Json::obj(vec![
                                ("line", (*line).into()),
                                ("what", what.as_str().into()),
                                ("field", field.as_str().into()),
                                ("linked", linked.as_str().into()),
                                ("set", set.as_str().into()),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    pub fn text(&self) -> String {
        let moving = self.joints.iter().filter(|j| j.kind != "fixed").count();
        let mut t = format!(
            "{}.urdf: {} links, {} joints ({} moving), {} meshes{}, {} bytes",
            self.name,
            self.links.len(),
            self.joints.len(),
            moving,
            self.meshes.len(),
            if self.watertight {
                ""
            } else {
                " (NOT all watertight)"
            },
            self.zip.len()
        );
        if !self.cut.is_empty() {
            t += &format!("; cut: {}", self.cut.join(" "));
        }
        for w in &self.warnings {
            t += &format!("\nwarning: {w}");
        }
        for n in &self.notes {
            t += &format!("\nnote: {n}");
        }
        t
    }
}

// ---- numbers, names, frames -------------------------------------------------------------------

/// Shortest round-trip text, the same native and wasm32: plain from 1e-4 up to 1e15, else with
/// an exponent (`4.8e-21`); `-0` as `0`.
fn num(x: f64) -> String {
    let a = x.abs();
    if x == 0.0 {
        "0".into()
    } else if (1e-4..1e15).contains(&a) {
        format!("{x}")
    } else {
        format!("{x:e}")
    }
}

fn nums(v: &[f64]) -> String {
    v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(" ")
}

/// `[A-Za-z0-9_]`, anything else `_`, a leading digit after `_`.
fn ident(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Names unique within one kind (links, joints, meshes): a taken one numbered `_2`, `_3`, ..
#[derive(Default)]
struct Names(BTreeSet<String>);

impl Names {
    fn take(&mut self, want: &str) -> String {
        let base = ident(want);
        let mut n = base.clone();
        let mut k = 2;
        while self.0.contains(&n) {
            n = format!("{base}_{k}");
            k += 1;
        }
        self.0.insert(n.clone());
        n
    }
}

/// A comment's text: no `--` inside one.
fn comment(s: &str) -> String {
    let mut t = s.replace("--", "- -");
    if t.ends_with('-') {
        t.push(' ');
    }
    t
}

fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// URDF's `rpy`: fixed axes x, y, z, so R = Rz(yaw) Ry(pitch) Rx(roll).
fn rpy(f: &Frame) -> [f64; 3] {
    let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
    // R's columns are the axes: R[i][0] = x[i], R[i][1] = y[i], R[i][2] = z[i]
    let pitch = dmath::atan2(-x.z, (x.x * x.x + x.y * x.y).sqrt());
    let (roll, yaw) = if (x.x * x.x + x.y * x.y).sqrt() > 1e-12 {
        (dmath::atan2(y.z, z.z), dmath::atan2(x.y, x.x))
    } else {
        // gimbal lock: the roll folded into the yaw
        (0.0, dmath::atan2(-y.x, y.y))
    };
    [roll, pitch, yaw]
}

/// `<origin>` for a frame: metres.
fn origin(f: &Frame) -> String {
    let o = f.origin;
    format!(
        "<origin xyz=\"{}\" rpy=\"{}\"/>",
        nums(&[o.x / 1000.0, o.y / 1000.0, o.z / 1000.0]),
        nums(&rpy(f))
    )
}

/// The turn about one axis (0 x, 1 y, 2 z) by `t`, or the move along it.
fn motion(axis: usize, prismatic: bool, t: f64) -> Frame {
    if prismatic {
        let mut v = [0.0; 3];
        v[axis] = t;
        return Frame::WORLD.with_origin(Point3::new(v[0], v[1], v[2]));
    }
    let (s, c) = dmath::sin_cos(t);
    let (z, x) = match axis {
        0 => (Vec3::new(0.0, -s, c), Vec3::new(1.0, 0.0, 0.0)),
        1 => (Vec3::new(s, 0.0, c), Vec3::new(c, 0.0, -s)),
        _ => (Vec3::new(0.0, 0.0, 1.0), Vec3::new(c, s, 0.0)),
    };
    let z = z.normalize().expect("a unit axis");
    Frame::from_zx(Point3::ORIGIN, z, x).expect("an orthogonal x")
}

fn frame_gap(p: &Frame, q: &Frame, size: f64) -> f64 {
    let d = (p.origin - q.origin).length();
    let ax = (p.x().get() - q.x().get())
        .length()
        .max((p.z().get() - q.z().get()).length());
    d.max(ax * size)
}

// ---- joints -----------------------------------------------------------------------------------

/// One single-axis motion of a joint's chain: its axis, its sign, prismatic or not, and the forge
/// freedom whose value it is.
#[derive(Clone, Copy, Debug)]
struct Step {
    axis: usize,
    sign: f64,
    prismatic: bool,
    freedom: Freedom,
}

/// The kind's motion `M` as single-axis steps, in order.
fn steps(kind: JointKind) -> Vec<Step> {
    let s = |axis, prismatic, freedom| Step {
        axis,
        sign: 1.0,
        prismatic,
        freedom,
    };
    use Freedom::*;
    match kind {
        JointKind::Fastened => vec![],
        JointKind::Revolute => vec![s(2, false, Rz)],
        JointKind::Slider => vec![s(2, true, Tz)],
        JointKind::Cylindrical => vec![s(2, true, Tz), s(2, false, Rz)],
        JointKind::PinSlot => vec![s(0, true, Tx), s(2, false, Rz)],
        JointKind::Planar => vec![s(0, true, Tx), s(1, true, Ty), s(2, false, Rz)],
        // R = Rz(yaw) Ry(pitch) Rx(roll), each named by the freedom it turns about
        JointKind::Ball => vec![s(2, false, Rz), s(1, false, Ry), s(0, false, Rx)],
    }
}

/// A ball's rotation `rel` as its three values (Rz, Ry, Rx).
fn ball_values(rel: &Frame) -> Vec<(Freedom, f64)> {
    let [roll, pitch, yaw] = rpy(rel);
    vec![(Freedom::Rz, yaw), (Freedom::Ry, pitch), (Freedom::Rx, roll)]
}

/// One URDF joint to write.
struct Wj {
    name: String,
    kind: &'static str,
    of: String,
    parent: String,
    child: String,
    origin: Frame,
    axis: Option<[f64; 3]>,
    /// (lower, upper) in URDF units.
    limit: Option<(f64, f64)>,
    position: Vec<f64>,
    mimic: Option<(String, f64, f64)>,
    /// (damping, friction).
    dynamics: Option<(f64, f64)>,
}

/// One URDF link to write.
struct Wl {
    name: String,
    bodies: Vec<String>,
    visuals: Vec<Visual>,
    collisions: Vec<Visual>,
    /// The solids its mass is composed of (an assembly's); empty for a part, whose `inertial`
    /// is given whole.
    pieces: Vec<Own>,
    inertial: Option<Inertial>,
    /// `inertial` is final: a part's, or a robot document's override.
    inertial_set: bool,
    dummy: bool,
}

/// What a visual or collision element is (mm).
#[derive(Clone, Debug, PartialEq)]
enum Geo {
    /// A mesh file, URDF's scale (its units to metres), and whether it is in the package
    /// (`meshes/..`) or a reference written as it is (`package://..`, a file not read).
    Mesh {
        file: String,
        scale: [f64; 3],
        packaged: bool,
    },
    Box([f64; 3]),
    Cylinder {
        r: f64,
        len: f64,
    },
    Sphere(f64),
    /// The link's convex hull, made when the model is finished.
    Hull,
}

#[derive(Clone)]
struct Visual {
    name: String,
    /// The source body (top-level instance) it draws; empty for one a robot document added.
    body: String,
    /// The occurrence it draws, its path as the assembly names it; empty if none.
    occ: String,
    geo: Geo,
    /// In the link frame.
    at: Frame,
    material: Option<(String, [u8; 3])>,
}

/// A solid of a link, owned so a link can be merged into another.
#[derive(Clone)]
struct Own {
    id: String,
    body: String,
    material: Option<forge_material::Material>,
    mass: forge_topo::mass::MassProps,
    at: Frame,
}

/// The robot before it is written: links, joints, each mesh's points (mm, its own frame) for
/// hulls, and transmissions.
struct Model {
    links: Vec<Wl>,
    joints: Vec<Wj>,
    points: BTreeMap<String, Vec<Point3>>,
    transmissions: Vec<(String, String, f64, Option<String>)>,
    root: String,
    /// A link merged into another, by its old name: the link it is now part of.
    aliases: BTreeMap<String, String>,
    /// Joints a merge or a drop took out, by their URDF and source names, and why.
    gone: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
struct Inertial {
    mass: f64,
    /// m, link frame.
    com: [f64; 3],
    /// kg m².
    inertia: [[f64; 3]; 3],
    err: Option<[f64; 3]>,
}

// ---- mass -------------------------------------------------------------------------------------

/// The pieces' physical properties in the link frame, and their first-order bounds.
fn inertial(pieces: &[Piece]) -> Option<Inertial> {
    if pieces.is_empty() {
        return None;
    }
    let p = combine(pieces);
    let (Some(mass), Some(i)) = (p.mass, p.inertia) else {
        return None;
    };
    // bounds: each solid's (volume e0, centroid ec, inertia entry e3) carried through the
    // composition to first order; a rotated tensor's entry is within 3 times the largest entry's
    let c = p.centroid;
    let (mut em, mut num_c) = (0.0, 0.0);
    let mut rows = vec![];
    for q in pieces {
        let k = q.material.map_or(0.0, |m| m.kg_per_mm3());
        let ci = q.at.point_to_world(q.mass.centroid);
        let (dx, dy, dz) = (ci.x - c[0], ci.y - c[1], ci.z - c[2]);
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        let m = k * q.mass.volume;
        em += k * q.mass.err[0];
        num_c += m * q.mass.err[2] + d * k * q.mass.err[0];
        rows.push((k, m, d, q.mass.err));
    }
    let ec = num_c / mass;
    let ei: f64 = rows
        .iter()
        .map(|&(k, m, d, e)| 3.0 * k * e[3] + 2.0 * d * d * k * e[0] + 4.0 * m * d * (e[2] + ec))
        .sum();
    let finite = em.is_finite() && ec.is_finite() && ei.is_finite();
    Some(Inertial {
        mass,
        com: c.map(|x| x / 1000.0),
        inertia: i.map(|r| r.map(|x| x / 1e6)),
        err: finite.then_some([em, ec / 1000.0, ei / 1e6]),
    })
}

// ---- the export -------------------------------------------------------------------------------

fn stl(objects: Vec<forge_mesh::Object>, title: &str) -> (Vec<u8>, bool) {
    let scene = Scene::of_objects(title, objects, vec![]);
    let ok = scene.checks().iter().all(|c| c.watertight() && c.manifold());
    (forge_mesh::scene_binary_stl(&scene), ok)
}

fn mesh_uri(o: &Options, file: &str) -> String {
    match &o.package {
        Some(p) => format!("package://{p}/{file}"),
        None => file.to_string(),
    }
}

/// The run as URDF. An assembly's run must have been made with `crate::Options::render` and
/// `kin` (its parts' bodies, its structure).
pub fn export(run: &Run, name: &str, dir: Option<&Path>, o: &Options) -> Result<Exported, UrdfError> {
    export_robot(run, name, dir, o, None)
}

/// The run as URDF with a robot document's overrides on top (`crate::robot`).
pub fn export_robot(
    run: &Run,
    name: &str,
    dir: Option<&Path>,
    o: &Options,
    doc: Option<&crate::robot::Robot>,
) -> Result<Exported, UrdfError> {
    let mut o = o.clone();
    if let Some(r) = doc {
        r.shape(&mut o);
    }
    let o = &o;
    if let Some(e) = run.errors.first() {
        return Err(UrdfError::NotBuilt(format!(
            "the {} did not build: {}: {}",
            if run.asm.is_some() { "assembly" } else { "part" },
            e.kind(),
            e.text()
        )));
    }
    let mut ex = Exported::new(name);
    let model = match &run.asm {
        Some(asm) => assembly(run, asm, o, &mut ex)?,
        None => part(run, name, dir, o, &mut ex)?,
    };
    complete(model, o, doc, ex)
}

/// An unlinked robot document (`source(none)`): its own links and joints, then its overrides.
pub fn export_unlinked(name: &str, o: &Options, doc: &crate::robot::Robot) -> Result<Exported, UrdfError> {
    let mut o = o.clone();
    doc.shape(&mut o);
    let mut ex = Exported::new(name);
    let model = unlinked(doc, &mut o, &mut ex)?;
    complete(model, &o, Some(doc), ex)
}

impl Exported {
    fn new(name: &str) -> Exported {
        Exported {
            name: ident(name),
            urdf: String::new(),
            meshes: vec![],
            zip: vec![],
            links: vec![],
            joints: vec![],
            cut: vec![],
            notes: vec![],
            warnings: vec![],
            residual: 0.0,
            watertight: true,
            overrides: vec![],
            view: RobotView::default(),
        }
    }
}

/// The model with the document's overrides, made whole and written.
fn complete(
    mut model: Model,
    o: &Options,
    doc: Option<&crate::robot::Robot>,
    mut ex: Exported,
) -> Result<Exported, UrdfError> {
    let robot = ex.name.clone();
    let mut o = o.clone();
    if let Some(r) = doc {
        apply(&mut model, r, &mut o, &mut ex)?;
    }
    let o = &o;
    finish(&mut model, &mut ex)?;
    let Model {
        links,
        joints,
        transmissions,
        root,
        ..
    } = model;
    ex.view = view_of(&links, &joints, &root, o);
    let missing: Vec<String> = joints
        .iter()
        .filter(|j| has_limit(j) && (actuation(o, j).0.is_none() || actuation(o, j).1.is_none()))
        .map(|j| j.name.clone())
        .collect();
    if !missing.is_empty() {
        if !o.placeholder {
            return Err(UrdfError::MissingLimits { joints: missing });
        }
        ex.warnings.push(format!(
            "effort or velocity unknown for {}: written as 0, each marked a placeholder (planners read velocity 0 as cannot move)",
            missing.join(" ")
        ));
    }
    ex.urdf = write(&robot, &links, &joints, &transmissions, o, &ex);
    let mut members: Vec<(&str, &[u8])> = vec![];
    let urdf_name = format!("{robot}.urdf");
    members.push((&urdf_name, ex.urdf.as_bytes()));
    for (n, b) in &ex.meshes {
        members.push((n, b));
    }
    ex.zip = forge_mesh::zip::write(&members).map_err(|e| UrdfError::Mesh(format!("zip: {e:?}")))?;
    ex.links = links
        .iter()
        .map(|l| LinkOut {
            name: l.name.clone(),
            bodies: l.bodies.clone(),
            mass: l.inertial.as_ref().map(|i| i.mass),
            com: l.inertial.as_ref().map_or([0.0; 3], |i| i.com),
            inertia: l.inertial.as_ref().map(|i| i.inertia),
            err: l.inertial.as_ref().and_then(|i| i.err),
            dummy: l.dummy,
        })
        .collect();
    ex.joints = joints
        .iter()
        .map(|j| JointOut {
            name: j.name.clone(),
            kind: j.kind,
            of: j.of.clone(),
            parent: j.parent.clone(),
            child: j.child.clone(),
            position: j.position.clone(),
            mimic: j.mimic.as_ref().map(|m| m.0.clone()),
        })
        .collect();
    Ok(ex)
}

/// A part: one link at its origin.
fn part(
    run: &Run,
    name: &str,
    dir: Option<&Path>,
    o: &Options,
    ex: &mut Exported,
) -> Result<Model, UrdfError> {
    let body = run
        .body
        .as_ref()
        .ok_or_else(|| UrdfError::NotBuilt("the part has no solid".into()))?;
    let phys = run
        .physical
        .as_ref()
        .ok_or_else(|| UrdfError::Mass("the part's physical properties were not worked out".into()))?;
    if !phys.missing.is_empty() {
        return Err(UrdfError::NoMaterial(phys.missing.clone()));
    }
    let (Some(mass), Some(i)) = (phys.mass, phys.inertia) else {
        return Err(UrdfError::NoMaterial(vec![name.to_string()]));
    };
    // bounds: one density over the whole body scales its integration's; several: UNKNOWN
    let densities: BTreeSet<u64> = phys
        .solids
        .iter()
        .filter_map(|s| s.density.map(f64::to_bits))
        .collect();
    let err = match densities.iter().next() {
        Some(&d) if densities.len() == 1 => {
            let k = f64::from_bits(d) * 1e-9;
            forge_topo::mass::mass(body)
                .ok()
                .map(|m| [k * m.err[0], m.err[2] / 1000.0, k * m.err[3] / 1e6])
        }
        _ => {
            ex.notes
                .push("solids of several densities: the mass bounds are UNKNOWN (not composed)".into());
            None
        }
    };
    let mut mo = forge_lang::meshfile::Options::new(forge_lang::meshfile::Format::Stl);
    mo.tol = o.tol;
    mo.angle = o.angle;
    let scene = forge_lang::meshfile::scene(run, name, dir, &mo).map_err(UrdfError::Mesh)?;
    let mesh_name = format!("meshes/{}.stl", ident(name));
    let objects = scene.flatten().objects;
    let pts: Vec<Point3> = objects.iter().flat_map(|o| o.positions.iter().copied()).collect();
    let (bytes, ok) = stl(objects, name);
    ex.watertight &= ok;
    ex.meshes.push((mesh_name.clone(), bytes));
    let material = phys.solids.iter().find_map(|s| s.material.clone()).map(|m| {
        let libs = forge_material::Libraries::cached(dir);
        let rgb = libs.find(&m).map_or([0x80; 3], |x| x.1.colour);
        (m, rgb)
    });
    let v = Visual {
        name: ident(name),
        body: name.to_string(),
        occ: name.to_string(),
        geo: Geo::Mesh {
            file: mesh_name.clone(),
            scale: [0.001; 3],
            packaged: true,
        },
        at: Frame::WORLD,
        material,
    };
    Ok(Model {
        root: ident(name),
        links: vec![Wl {
            name: ident(name),
            bodies: vec![name.to_string()],
            collisions: vec![Visual {
                material: None,
                ..v.clone()
            }],
            visuals: vec![v],
            pieces: vec![],
            inertial: Some(Inertial {
                mass,
                com: phys.centroid.map(|x| x / 1000.0),
                inertia: i.map(|r| r.map(|x| x / 1e6)),
                err,
            }),
            inertial_set: true,
            dummy: false,
        }],
        joints: vec![],
        points: BTreeMap::from([(mesh_name, pts)]),
        transmissions: vec![],
        aliases: BTreeMap::new(),
        gone: BTreeMap::new(),
    })
}

/// Union-find over a few bodies.
fn find(p: &mut [usize], mut i: usize) -> usize {
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}

#[allow(clippy::too_many_lines)]
fn assembly(
    run: &Run,
    asm: &forge_lang::asm::AsmOut,
    o: &Options,
    ex: &mut Exported,
) -> Result<Model, UrdfError> {
    let kin = asm.kin.as_ref().ok_or_else(|| {
        UrdfError::NotBuilt("the assembly was run without its structure (Options::kin)".into())
    })?;
    if kin.bodies.is_empty() {
        return Err(UrdfError::NotBuilt("the assembly has no instances".into()));
    }
    if asm.occ_part.iter().all(Option::is_none) {
        return Err(UrdfError::NotBuilt(
            "the assembly has no parts to export (built without its parts' bodies)".into(),
        ));
    }
    for c in &o.cut {
        if !kin.links.iter().any(|l| l.id == *c) {
            return Err(UrdfError::NoSuchJoint(c.clone()));
        }
    }
    let n = kin.bodies.len();
    let resolved = |l: &KinLink| -> Result<[usize; 2], UrdfError> {
        match (l.ends, l.at) {
            ([Some(a), Some(b)], [Some(_), Some(_)]) => Ok([a, b]),
            _ => Err(UrdfError::Unresolved { joint: l.id.clone() }),
        }
    };
    // rigid groups: fastenings, lock constraints, and ground as one body
    let mut uf: Vec<usize> = (0..n).collect();
    let mut first_ground = None;
    for (i, _) in kin.bodies.iter().enumerate().filter(|(_, b)| b.ground) {
        match first_ground {
            None => first_ground = Some(i),
            Some(g) => {
                let (r, s) = (find(&mut uf, g), find(&mut uf, i));
                uf[s.max(r)] = s.min(r);
            }
        }
    }
    for l in kin.links.iter().filter(|l| l.rigid() && !o.cut.contains(&l.id)) {
        let [a, b] = resolved(l)?;
        let (r, s) = (find(&mut uf, a), find(&mut uf, b));
        uf[s.max(r)] = s.min(r);
    }
    for l in kin.links.iter().filter(|l| l.rigid() && o.cut.contains(&l.id)) {
        ex.cut.push(l.id.clone());
    }
    let mut group_of = vec![usize::MAX; n];
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut slot: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, of) in group_of.iter_mut().enumerate() {
        let r = find(&mut uf, i);
        let g = *slot.entry(r).or_insert_with(|| {
            groups.push(vec![]);
            groups.len() - 1
        });
        groups[g].push(i);
        *of = g;
    }
    let root = match &o.root {
        Some(r) => {
            let b = kin
                .bodies
                .iter()
                .position(|b| b.name == *r)
                .ok_or_else(|| UrdfError::Robot(format!("root({r}): the source has no instance {r}")))?;
            group_of[b]
        }
        None => first_ground.map_or(0, |g| group_of[g]),
    };
    if first_ground.is_none() && o.root.is_none() {
        ex.notes.push(format!(
            "nothing is grounded: the root link is {}'s, the first instance's",
            kin.bodies[0].name
        ));
    }
    // the tree: moving joints between groups, in document order; a cycle is refused
    let mut tree: Vec<Vec<(usize, usize)>> = vec![vec![]; groups.len()];
    let mut guf: Vec<usize> = (0..groups.len()).collect();
    let mut edges: Vec<(usize, [usize; 2])> = vec![];
    for (k, l) in kin.links.iter().enumerate().filter(|(_, l)| !l.rigid()) {
        if o.cut.contains(&l.id) {
            ex.cut.push(l.id.clone());
            continue;
        }
        let [a, b] = resolved(l)?;
        let (ga, gb) = (group_of[a], group_of[b]);
        if l.kind.is_none() {
            if ga == gb {
                ex.notes.push(format!(
                    "loose constraint {} is within one rigid group: implied, not written",
                    l.id
                ));
                continue;
            }
            return Err(UrdfError::LooseConstraint {
                id: l.id.clone(),
                kind: l.constraint.map_or("constraint", |c| c.name()),
            });
        }
        let (r, s) = (find(&mut guf, ga), find(&mut guf, gb));
        if r == s {
            // the loop: the tree path from gb to ga, then this joint back to gb
            let mut prev: Vec<Option<(usize, usize)>> = vec![None; groups.len()];
            let mut seen = vec![false; groups.len()];
            let mut q = VecDeque::from([ga]);
            seen[ga] = true;
            while let Some(u) = q.pop_front() {
                for &(e, v) in &tree[u] {
                    if !seen[v] {
                        seen[v] = true;
                        prev[v] = Some((e, u));
                        q.push_back(v);
                    }
                }
            }
            let mut joints = vec![];
            let mut at = gb;
            while let Some((e, u)) = prev[at] {
                joints.push(kin.links[e].id.clone());
                at = u;
            }
            joints.push(l.id.clone());
            return Err(UrdfError::ClosedLoop { joints });
        }
        uf_join(&mut guf, r, s);
        tree[ga].push((k, gb));
        tree[gb].push((k, ga));
        edges.push((k, [ga, gb]));
    }
    for l in kin.links.iter() {
        if l.kind == Some(JointKind::Ball) && !l.limits.is_empty() && !o.cut.contains(&l.id) {
            return Err(UrdfError::BallLimits { joint: l.id.clone() });
        }
    }
    // links: each group's name is its first instance's
    let mut lnames = Names::default();
    let jnames = Names::default();
    let gname: Vec<String> = groups
        .iter()
        .map(|g| lnames.take(&kin.bodies[g[0]].name))
        .collect();
    let mut st = State {
        frame: vec![None; groups.len()],
        anchor: vec![None; groups.len()],
        order: vec![root],
        wj: vec![],
        dummies: vec![],
        carries: BTreeMap::new(),
        native_planar: BTreeSet::new(),
        frozen: BTreeSet::new(),
        lnames,
        jnames,
    };
    st.frame[root] = Some(Frame::WORLD);
    st.anchor[root] = Some(Frame::WORLD);
    let size = {
        let o: Vec<Point3> = kin.bodies.iter().map(|b| b.pose.origin).collect();
        let c = o.iter().fold(Vec3::ZERO, |s, p| s + p.to_vec()) / o.len() as f64;
        o.iter().map(|p| (p.to_vec() - c).length()).fold(1.0f64, f64::max)
    };
    let tree_cx = Tree {
        kin,
        tree: &tree,
        group_of: &group_of,
        gname: &gname,
        o,
        size,
    };
    tree_cx.grow_from(root, &mut st, ex)?;
    // groups the tree did not reach: free bodies, floating from the root where the solve put them
    for g in 0..groups.len() {
        if st.frame[g].is_some() {
            continue;
        }
        let f = kin.bodies[groups[g][0]].pose;
        st.frame[g] = Some(f);
        st.anchor[g] = Some(f);
        st.order.push(g);
        ex.notes.push(format!(
            "{} is joined to nothing that reaches the root: a floating joint",
            gname[g]
        ));
        let name = st.jnames.take(&format!("{}_float", gname[g]));
        st.wj.push(Wj {
            name,
            kind: "floating",
            of: String::new(),
            parent: gname[root].clone(),
            child: gname[g].clone(),
            origin: f,
            axis: None,
            limit: None,
            position: vec![],
            mimic: None,
            dynamics: None,
        });
        tree_cx.grow_from(g, &mut st, ex)?;
    }
    let State {
        anchor,
        order,
        mut wj,
        dummies,
        carries,
        native_planar,
        frozen,
        ..
    } = st;
    // relations: the follower's URDF joint mimics the leader's
    let positions: BTreeMap<String, f64> = wj
        .iter()
        .filter(|j| j.position.len() == 1)
        .map(|j| (j.name.clone(), j.position[0]))
        .collect();
    for r in &kin.relations {
        let carried = |v: &(String, Freedom)| -> Result<(String, f64), UrdfError> {
            if o.cut.contains(&v.0) {
                return Err(UrdfError::Relation {
                    id: r.id.clone(),
                    why: format!("its joint {} is cut", v.0),
                });
            }
            if frozen.contains(&v.0) {
                return Err(UrdfError::Relation {
                    id: r.id.clone(),
                    why: format!("its joint {} is frozen", v.0),
                });
            }
            if native_planar.contains(&v.0) {
                return Err(UrdfError::Relation {
                    id: r.id.clone(),
                    why: format!(
                        "{} is a URDF planar joint, whose values cannot be mimicked one by one (give it limits: it is then a chain)",
                        v.0
                    ),
                });
            }
            carries.get(v).cloned().ok_or_else(|| UrdfError::Relation {
                id: r.id.clone(),
                why: format!("{}'s {} is carried by no URDF joint", v.0, v.1.name()),
            })
        };
        let (lj, lu) = carried(&r.leader)?;
        let (fj, fu) = carried(&r.follower)?;
        let mult = r.slope * fu / lu;
        let offset = match r.offset {
            Some(x) => x * fu,
            None => positions[&fj] - mult * positions[&lj],
        };
        if let Some(j) = wj.iter_mut().find(|j| j.name == fj) {
            j.mimic = Some((lj, mult, offset));
        }
    }
    // subassemblies are rigid in forge-asm v1
    let inner: Vec<&str> = asm
        .links
        .iter()
        .filter(|l| !l.owner.is_empty() && l.family == forge_lang::asm::LinkFamily::Joint && !l.suppressed)
        .map(|l| l.id.as_str())
        .collect();
    if !inner.is_empty() {
        ex.notes.push(format!(
            "subassemblies are rigid (forge-asm v1): their joints {} are held at their solved values",
            inner.join(" ")
        ));
    }
    // meshes: one per part body, in the order first used
    let q = {
        let mut mo = forge_lang::meshfile::Options::new(forge_lang::meshfile::Format::Stl);
        mo.tol = o.tol;
        mo.angle = o.angle;
        mo.quality()
    };
    let mut mesh_of: Vec<Option<String>> = vec![None; asm.parts.len()];
    let mut mnames = Names::default();
    let mut visuals: Vec<Vec<Visual>> = (0..groups.len()).map(|_| vec![]).collect();
    let mut pieces: Vec<Vec<Own>> = (0..groups.len()).map(|_| vec![]).collect();
    let mut points: BTreeMap<String, Vec<Point3>> = BTreeMap::new();
    let mut missing = vec![];
    let mut vnames = Names::default();
    for (k, (path, doc, pose)) in asm.occurrences.iter().enumerate() {
        let (body, reference) = asm.occ_body.get(k).copied().unwrap_or((0, false));
        let Some(Some(b)) = kin.occ_body.get(k) else {
            continue;
        };
        if reference {
            continue;
        }
        let g = group_of[*b];
        let lf = anchor[g].expect("every group placed");
        let at = lf.inverse().compose(pose);
        let sum = asm.part_docs.get(doc);
        if let Some(Some(p)) = asm.occ_part.get(k) {
            let file = match &mesh_of[*p] {
                Some(f) => f.clone(),
                None => {
                    let stem = Path::new(doc.trim_start_matches('#'))
                        .file_stem()
                        .map_or_else(|| doc.clone(), |s| s.to_string_lossy().into_owned());
                    let by = Solids {
                        name: &stem,
                        solids: &[],
                    };
                    let objects = forge_mesh::body_objects(&asm.parts[*p], &q, &by, false).map_err(|e| {
                        UrdfError::Mesh(format!("meshing {stem} failed: {}", forge_lang::error::words(&e)))
                    })?;
                    let f = format!("meshes/{}.stl", mnames.take(&stem));
                    points.insert(
                        f.clone(),
                        objects.iter().flat_map(|o| o.positions.iter().copied()).collect(),
                    );
                    let (bytes, ok) = stl(objects, &stem);
                    ex.watertight &= ok;
                    ex.meshes.push((f.clone(), bytes));
                    mesh_of[*p] = Some(f.clone());
                    f
                }
            };
            let material = sum.and_then(|d| {
                d.solids
                    .iter()
                    .find_map(|s| s.1.as_ref())
                    .map(|m| (m.name.clone(), m.colour))
            });
            visuals[g].push(Visual {
                name: vnames.take(path),
                body: kin.bodies[*b].name.clone(),
                occ: path.clone(),
                geo: Geo::Mesh {
                    file,
                    scale: [0.001; 3],
                    packaged: true,
                },
                at,
                material,
            });
        }
        let Some(sum) = sum else { continue };
        let masses = sum.masses().map_err(|e| UrdfError::Mass(e.to_string()))?;
        for (s, m) in sum.solids.iter().zip(masses) {
            if body != 0 && forge_lang::build::history_id(&s.0) != body {
                continue;
            }
            if s.1.is_none() {
                missing.push(format!("{path} ({})", s.0));
            }
            pieces[g].push(Own {
                id: path.clone(),
                body: kin.bodies[*b].name.clone(),
                material: s.1.clone(),
                mass: m.clone(),
                at,
            });
        }
    }
    if !missing.is_empty() {
        return Err(UrdfError::NoMaterial(missing));
    }
    let _ = run;
    let mut links = vec![];
    for &g in &order {
        let v = std::mem::take(&mut visuals[g]);
        links.push(Wl {
            name: gname[g].clone(),
            bodies: groups[g].iter().map(|&b| kin.bodies[b].name.clone()).collect(),
            collisions: v
                .iter()
                .map(|x| Visual {
                    material: None,
                    ..x.clone()
                })
                .collect(),
            visuals: v,
            pieces: std::mem::take(&mut pieces[g]),
            inertial: None,
            inertial_set: false,
            dummy: false,
        });
    }
    links.extend(dummies);
    if links.iter().any(|l| l.dummy) {
        ex.notes.push(
            "joints URDF lacks (cylindrical, pin_slot, limited planar, ball) are chains through massless links; physics engines that drop massless links need SDF or MJCF"
                .into(),
        );
    }
    Ok(Model {
        root: gname[root].clone(),
        links,
        joints: wj,
        points,
        transmissions: vec![],
        aliases: BTreeMap::new(),
        gone: BTreeMap::new(),
    })
}

/// What the tree walk builds.
struct State {
    /// Each group's link frame as the URDF places it at the export pose.
    frame: Vec<Option<Frame>>,
    /// Each group's link frame where the solve has it: its parts are drawn from it. The same as
    /// `frame` (to the solve's residual) unless a joint above it is frozen at another value.
    anchor: Vec<Option<Frame>>,
    order: Vec<usize>,
    wj: Vec<Wj>,
    dummies: Vec<Wl>,
    /// The URDF joint carrying each forge (joint, freedom), and its value's unit (1 or 1e-3).
    carries: BTreeMap<(String, Freedom), (String, f64)>,
    native_planar: BTreeSet<String>,
    frozen: BTreeSet<String>,
    lnames: Names,
    jnames: Names,
}

/// The group graph the walk reads.
struct Tree<'a> {
    kin: &'a Kin,
    tree: &'a [Vec<(usize, usize)>],
    group_of: &'a [usize],
    gname: &'a [String],
    o: &'a Options,
    size: f64,
}

impl Tree<'_> {
    /// Places every group reached from `start` (already placed), breadth first, edges in
    /// document order.
    fn grow_from(&self, start: usize, st: &mut State, ex: &mut Exported) -> Result<(), UrdfError> {
        let mut q = VecDeque::from([start]);
        while let Some(from) = q.pop_front() {
            for &(k, to) in &self.tree[from] {
                if st.frame[to].is_some() {
                    continue;
                }
                self.edge(k, from, to, st, ex)?;
                st.order.push(to);
                q.push_back(to);
            }
        }
        Ok(())
    }

    /// The URDF joints of forge link `k`, from group `from` (placed) to `to`.
    fn edge(
        &self,
        k: usize,
        from: usize,
        to: usize,
        st: &mut State,
        ex: &mut Exported,
    ) -> Result<(), UrdfError> {
        let (kin, o, gname) = (self.kin, self.o, self.gname);
        let l = &kin.links[k];
        let kind = l.kind.expect("a joint");
        let (Some(a), Some(aw), Some(bw)) = (l.ends[0], l.at[0], l.at[1]) else {
            return Err(UrdfError::Unresolved { joint: l.id.clone() });
        };
        // forward: B on the parent's side, A in the child
        let forward = self.group_of[a] == to;
        let rel = bw.inverse().compose(&aw);
        let mut values = if kind == JointKind::Ball {
            ball_values(&rel)
        } else {
            l.values.clone()
        };
        for s in steps(kind) {
            if !values.iter().any(|v| v.0 == s.freedom) {
                // not reported: read off the frames
                values.push((s.freedom, measure(s.freedom, &rel)));
            }
        }
        let value = |f: Freedom| values.iter().find(|v| v.0 == f).map_or(0.0, |v| v.1);
        let pf = st.frame[from].expect("placed");
        let pa = st.anchor[from].expect("placed");
        let (jo_world, other) = if forward { (bw, aw) } else { (aw, bw) };
        let mut cur = jo_world;
        if let Some(fv) = o.freeze.iter().find(|f| f.0 == l.id).map(|f| &f.1) {
            // frozen: fixed at the values given, the child's subtree moved with it
            let fs = steps(kind);
            if fv.len() != fs.len() {
                return Err(UrdfError::Robot(format!(
                    "freeze({}): {} takes {} value(s) ({}), not {}",
                    l.id,
                    kind.name(),
                    fs.len(),
                    fs.iter().map(|s| s.freedom.name()).collect::<Vec<_>>().join(" "),
                    fv.len()
                )));
            }
            let m = fs.iter().zip(fv).fold(Frame::WORLD, |f, (s, v)| {
                f.compose(&motion(s.axis, s.prismatic, *v))
            });
            let m = if forward { m } else { m.inverse() };
            let origin = pa.inverse().compose(&jo_world).compose(&m);
            let name = st.jnames.take(&l.id);
            st.wj.push(Wj {
                name,
                kind: "fixed",
                of: l.id.clone(),
                parent: gname[from].clone(),
                child: gname[to].clone(),
                origin,
                axis: None,
                limit: None,
                position: vec![],
                mimic: None,
                dynamics: None,
            });
            st.frozen.insert(l.id.clone());
            st.frame[to] = Some(pf.compose(&origin));
            st.anchor[to] = Some(other);
            return Ok(());
        }
        if kind == JointKind::Planar && l.limits.is_empty() {
            let m = steps(kind).iter().fold(Frame::WORLD, |f, s| {
                f.compose(&motion(s.axis, s.prismatic, value(s.freedom)))
            });
            let m = if forward { m } else { m.inverse() };
            cur = cur.compose(&m);
            st.native_planar.insert(l.id.clone());
            let name = st.jnames.take(&l.id);
            st.wj.push(Wj {
                name,
                kind: "planar",
                of: l.id.clone(),
                parent: gname[from].clone(),
                child: gname[to].clone(),
                origin: pa.inverse().compose(&jo_world),
                axis: Some([0.0, 0.0, 1.0]),
                limit: None,
                position: vec![
                    m.origin.x / 1000.0,
                    m.origin.y / 1000.0,
                    dmath::atan2(m.x().get().y, m.x().get().x),
                ],
                mimic: None,
                dynamics: None,
            });
        } else {
            let mut chain = steps(kind);
            if !forward {
                // A'⁻¹ B = M⁻¹: the same steps the other way round, each turned back
                chain.reverse();
                for s in &mut chain {
                    s.sign = -1.0;
                }
            }
            let mut parent = gname[from].clone();
            for (i, s) in chain.iter().enumerate() {
                let last = i + 1 == chain.len();
                let v = value(s.freedom);
                let child = if last {
                    gname[to].clone()
                } else {
                    st.lnames.take(&format!("{}__{}", l.id, i + 1))
                };
                let jorigin = if i == 0 {
                    pa.inverse().compose(&jo_world)
                } else {
                    Frame::WORLD
                };
                cur = cur.compose(&motion(s.axis, s.prismatic, s.sign * v));
                let unit = if s.prismatic { 1e-3 } else { 1.0 };
                let (kname, limit) = match (s.prismatic, l.limits.iter().find(|x| x.0 == s.freedom)) {
                    (_, Some(&(_, lo, hi))) => (
                        if s.prismatic { "prismatic" } else { "revolute" },
                        Some((lo * unit, hi * unit)),
                    ),
                    (false, None) => ("continuous", None),
                    (true, None) => match o.travel {
                        Some(t) => {
                            ex.notes.push(format!(
                                "joint {}'s {} has no limit: written ±{} mm (travel)",
                                l.id,
                                s.freedom.name(),
                                num(t)
                            ));
                            ("prismatic", Some((-t * unit, t * unit)))
                        }
                        None => {
                            return Err(UrdfError::UnlimitedTranslation {
                                joint: l.id.clone(),
                                freedom: s.freedom,
                            });
                        }
                    },
                };
                let name = if chain.len() == 1 {
                    st.jnames.take(&l.id)
                } else {
                    st.jnames
                        .take(&format!("{}_{}", l.id, s.freedom.name().to_ascii_lowercase()))
                };
                let mut ax = [0.0; 3];
                ax[s.axis] = s.sign;
                st.carries.insert((l.id.clone(), s.freedom), (name.clone(), unit));
                st.wj.push(Wj {
                    name,
                    kind: kname,
                    of: l.id.clone(),
                    parent: parent.clone(),
                    child: child.clone(),
                    origin: jorigin,
                    axis: Some(ax),
                    limit,
                    position: vec![v * unit],
                    mimic: None,
                    dynamics: None,
                });
                if !last {
                    // the massless link between, where the chain has got to
                    st.dummies.push(Wl {
                        name: child.clone(),
                        bodies: vec![],
                        visuals: vec![],
                        collisions: vec![],
                        pieces: vec![],
                        inertial: None,
                        inertial_set: true,
                        dummy: true,
                    });
                    parent = child;
                }
            }
        }
        ex.residual = ex.residual.max(frame_gap(&cur, &other, self.size));
        st.frame[to] = Some(pf.compose(&pa.inverse()).compose(&cur));
        st.anchor[to] = Some(other);
        Ok(())
    }
}

fn uf_join(p: &mut [usize], r: usize, s: usize) {
    p[s.max(r)] = s.min(r);
}

/// A freedom's value read off `B⁻¹ ∘ A'` (as forge-asm measures it).
fn measure(f: Freedom, rel: &Frame) -> f64 {
    match f {
        Freedom::Tx => rel.origin.x,
        Freedom::Ty => rel.origin.y,
        Freedom::Tz => rel.origin.z,
        Freedom::Rz => dmath::atan2(rel.x().get().y, rel.x().get().x),
        Freedom::Rx | Freedom::Ry => 0.0,
    }
}

/// A joint that writes `<limit>`: every moving single-axis joint.
fn has_limit(j: &Wj) -> bool {
    j.limit.is_some() || j.kind == "continuous"
}

/// A joint's effort and velocity: its own (by its forge joint, then its URDF name), else the
/// export's.
fn actuation(o: &Options, j: &Wj) -> (Option<f64>, Option<f64>) {
    let pick = |of: &BTreeMap<String, f64>, all: Option<f64>| {
        of.get(&j.of).or_else(|| of.get(&j.name)).copied().or(all)
    };
    (pick(&o.effort_of, o.effort), pick(&o.velocity_of, o.velocity))
}

// ---- the XML ----------------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
fn write(
    robot: &str,
    links: &[Wl],
    joints: &[Wj],
    transmissions: &[(String, String, f64, Option<String>)],
    o: &Options,
    ex: &Exported,
) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "<?xml version=\"1.0\"?>");
    let _ = writeln!(
        s,
        "<!-- {} -->",
        comment(
            "written by forgeCAD (export urdf): metres, kilograms, radians; meshes in mm, scaled 0.001; research/export/urdf.md"
        )
    );
    for c in &ex.cut {
        let _ = writeln!(
            s,
            "<!-- {} -->",
            comment(&format!("CUT: joint {c} is left out to open a closed loop"))
        );
    }
    for n in &ex.notes {
        let _ = writeln!(s, "<!-- {} -->", comment(&format!("note: {n}")));
    }
    let _ = writeln!(s, "<robot name=\"{}\">", attr(robot));
    for l in links {
        let _ = writeln!(s, "  <link name=\"{}\">", attr(&l.name));
        if let Some(i) = &l.inertial {
            let m = &i.inertia;
            let _ = writeln!(s, "    <inertial>");
            let _ = writeln!(s, "      <origin xyz=\"{}\" rpy=\"0 0 0\"/>", nums(&i.com));
            let _ = writeln!(s, "      <mass value=\"{}\"/>", num(i.mass));
            let _ = writeln!(
                s,
                "      <inertia ixx=\"{}\" ixy=\"{}\" ixz=\"{}\" iyy=\"{}\" iyz=\"{}\" izz=\"{}\"/>",
                num(m[0][0]),
                num(m[0][1]),
                num(m[0][2]),
                num(m[1][1]),
                num(m[1][2]),
                num(m[2][2])
            );
            let _ = writeln!(s, "    </inertial>");
        }
        for (tag, list) in [("visual", &l.visuals), ("collision", &l.collisions)] {
            for v in list {
                let _ = writeln!(s, "    <{tag} name=\"{}\">", attr(&v.name));
                let _ = writeln!(s, "      {}", origin(&v.at));
                let g = match &v.geo {
                    Geo::Mesh {
                        file,
                        scale,
                        packaged,
                    } => format!(
                        "<mesh filename=\"{}\" scale=\"{}\"/>",
                        attr(&if *packaged {
                            mesh_uri(o, file)
                        } else {
                            file.clone()
                        }),
                        nums(scale)
                    ),
                    Geo::Box(b) => format!("<box size=\"{}\"/>", nums(&b.map(|x| x / 1000.0))),
                    Geo::Cylinder { r, len } => format!(
                        "<cylinder radius=\"{}\" length=\"{}\"/>",
                        num(r / 1000.0),
                        num(len / 1000.0)
                    ),
                    Geo::Sphere(r) => format!("<sphere radius=\"{}\"/>", num(r / 1000.0)),
                    Geo::Hull => unreachable!("hulls are made in finish"),
                };
                let _ = writeln!(s, "      <geometry>{g}</geometry>");
                if let (Some((name, rgb)), "visual") = (&v.material, tag) {
                    let c = rgb.map(|x| x as f64 / 255.0);
                    let _ = writeln!(
                        s,
                        "      <material name=\"{}\"><color rgba=\"{} 1\"/></material>",
                        attr(&ident(name)),
                        nums(&c)
                    );
                }
                let _ = writeln!(s, "    </{tag}>");
            }
        }
        let _ = writeln!(s, "  </link>");
    }
    for j in joints {
        let _ = writeln!(s, "  <joint name=\"{}\" type=\"{}\">", attr(&j.name), j.kind);
        let _ = writeln!(s, "    <parent link=\"{}\"/>", attr(&j.parent));
        let _ = writeln!(s, "    <child link=\"{}\"/>", attr(&j.child));
        let _ = writeln!(s, "    {}", origin(&j.origin));
        if let Some(a) = j.axis {
            let _ = writeln!(s, "    <axis xyz=\"{}\"/>", nums(&a));
        }
        let (effort, velocity) = actuation(o, j);
        if has_limit(j) && (effort.is_none() || velocity.is_none()) {
            let _ = writeln!(
                s,
                "    <!-- PLACEHOLDER: effort and velocity unknown, written as 0; not real limits -->"
            );
        }
        let (effort, velocity) = (effort.unwrap_or(0.0), velocity.unwrap_or(0.0));
        match (j.kind, j.limit) {
            (_, Some((lo, hi))) => {
                let _ = writeln!(
                    s,
                    "    <limit lower=\"{}\" upper=\"{}\" effort=\"{}\" velocity=\"{}\"/>",
                    num(lo),
                    num(hi),
                    num(effort),
                    num(velocity)
                );
            }
            ("continuous", None) => {
                let _ = writeln!(
                    s,
                    "    <limit effort=\"{}\" velocity=\"{}\"/>",
                    num(effort),
                    num(velocity)
                );
            }
            _ => {}
        }
        if let Some((d, f)) = j.dynamics {
            let _ = writeln!(
                s,
                "    <dynamics damping=\"{}\" friction=\"{}\"/>",
                num(d),
                num(f)
            );
        }
        if let Some((leader, m, off)) = &j.mimic {
            let _ = writeln!(
                s,
                "    <mimic joint=\"{}\" multiplier=\"{}\" offset=\"{}\"/>",
                attr(leader),
                num(*m),
                num(*off)
            );
        }
        let _ = writeln!(s, "  </joint>");
    }
    for (joint, actuator, reduction, interface) in transmissions {
        let _ = writeln!(s, "  <transmission name=\"{}\">", attr(&format!("{joint}_trans")));
        let _ = writeln!(s, "    <type>transmission_interface/SimpleTransmission</type>");
        let _ = writeln!(s, "    <joint name=\"{}\">", attr(joint));
        if let Some(i) = interface {
            let _ = writeln!(s, "      <hardwareInterface>{}</hardwareInterface>", attr(i));
        }
        let _ = writeln!(s, "    </joint>");
        let _ = writeln!(s, "    <actuator name=\"{}\">", attr(actuator));
        let _ = writeln!(
            s,
            "      <mechanicalReduction>{}</mechanicalReduction>",
            num(*reduction)
        );
        let _ = writeln!(s, "    </actuator>");
        let _ = writeln!(s, "  </transmission>");
    }
    let _ = writeln!(s, "</robot>");
    s
}

// ---- a robot document's overrides ------------------------------------------------------------

use crate::robot::{GeoSpec, Robot, Target};

/// A link by a robot document's name for it: `src.X` is the link holding source instance X.
fn link_of(m: &Model, t: &Target) -> Result<usize, UrdfError> {
    match t {
        Target::Source(b) => m.links.iter().position(|l| l.bodies.contains(b)).ok_or_else(|| {
            let have: Vec<&str> = m
                .links
                .iter()
                .flat_map(|l| l.bodies.iter().map(String::as_str))
                .collect();
            UrdfError::Robot(format!(
                "src.{b}: the source has no instance {b} in the robot (has: {})",
                have.join(" ")
            ))
        }),
        Target::Name(n) => {
            // a merged link by its old name: the one it is part of now
            let mut n = n.clone();
            for _ in 0..m.aliases.len() {
                match m.aliases.get(&n) {
                    Some(to) if !m.links.iter().any(|l| l.name == n) => n = to.clone(),
                    _ => break,
                }
            }
            m.links.iter().position(|l| l.name == n).ok_or_else(|| {
                let have: Vec<&str> = m.links.iter().map(|l| l.name.as_str()).collect();
                UrdfError::Robot(format!("no link {n} (links: {})", have.join(" ")))
            })
        }
    }
}

/// A joint by a robot document's name for it: `src.J` is source joint J when it is one URDF
/// joint (a chain's are named one by one: `"J_tz"`).
fn joint_of(m: &Model, t: &Target) -> Result<usize, UrdfError> {
    let n = match t {
        Target::Source(j) => {
            let of: Vec<usize> = (0..m.joints.len()).filter(|&k| m.joints[k].of == *j).collect();
            match of.as_slice() {
                [k] => return Ok(*k),
                [] => j,
                many => {
                    let names: Vec<&str> = many.iter().map(|&k| m.joints[k].name.as_str()).collect();
                    return Err(UrdfError::Robot(format!(
                        "src.{j} is several URDF joints ({}): name the one meant, as \"{}\"",
                        names.join(" "),
                        names[0]
                    )));
                }
            }
        }
        Target::Name(n) => n,
    };
    m.joints.iter().position(|j| j.name == *n).ok_or_else(|| {
        let have: Vec<&str> = m.joints.iter().map(|j| j.name.as_str()).collect();
        UrdfError::Robot(format!("no joint {n} (joints: {})", have.join(" ")))
    })
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()) + 1e-12
}

fn fmt_vals(v: &[f64]) -> String {
    fmt_in(v, false)
}

/// Values as a robot document writes them: angles in degrees (to 1e-9 of a degree).
fn fmt_in(v: &[f64], deg: bool) -> String {
    let t: Vec<String> = v
        .iter()
        .map(|x| {
            if deg {
                format!("{}deg", num((x.to_degrees() * 1e9).round() / 1e9))
            } else {
                num(*x)
            }
        })
        .collect();
    if t.len() == 1 {
        t[0].clone()
    } else {
        format!("({})", t.join(", "))
    }
}

/// One override whose recorded linked value (`was`) is no longer the source's.
#[allow(clippy::too_many_arguments)]
fn conflict(
    out: &mut Vec<Conflict>,
    line: usize,
    what: &str,
    field: &str,
    was: &[f64],
    now: &[f64],
    set: &[f64],
    deg: bool,
) {
    if was.len() == now.len() && was.iter().zip(now).all(|(a, b)| same(*a, *b)) {
        return;
    }
    out.push(Conflict {
        line,
        what: what.to_string(),
        field: field.to_string(),
        was: fmt_in(was, deg),
        now: fmt_in(now, deg),
        set: fmt_in(set, deg),
    });
}

/// Whether a tensor (kg m²) is one a body can have: symmetric positive definite, and its
/// principal moments within the triangle inequality.
fn physical(i: &[[f64; 3]; 3]) -> bool {
    let (p, _) = forge_topo::mass::eigen(*i);
    let tol = 1e-12 * p[2].abs();
    p[0] > 0.0 && p[0] + p[1] >= p[2] - tol
}

#[allow(clippy::too_many_lines)]
fn apply(m: &mut Model, r: &Robot, o: &mut Options, ex: &mut Exported) -> Result<(), UrdfError> {
    let mut conflicts = vec![];
    // names
    for (t, name) in &r.renames {
        let k = link_of(m, t)?;
        let new = ident(name);
        if m.links.iter().enumerate().any(|(i, l)| i != k && l.name == new) {
            return Err(UrdfError::Robot(format!("link name {new} is taken")));
        }
        let old = std::mem::replace(&mut m.links[k].name, new.clone());
        for j in &mut m.joints {
            if j.parent == old {
                j.parent = new.clone();
            }
            if j.child == old {
                j.child = new.clone();
            }
        }
        if m.root == old {
            m.root = new;
        }
    }
    // merges: across fixed joints only
    for (into, others) in &r.merges {
        for other in others {
            let (a, b) = (link_of(m, into)?, link_of(m, other)?);
            if a == b {
                continue;
            }
            let (an, bn) = (m.links[a].name.clone(), m.links[b].name.clone());
            let Some(jk) = m
                .joints
                .iter()
                .position(|j| (j.parent == an && j.child == bn) || (j.parent == bn && j.child == an))
            else {
                return Err(UrdfError::Robot(format!(
                    "merge({an}, {bn}): they are not joined directly; merge along the tree one joint at a time"
                )));
            };
            let j = &m.joints[jk];
            if j.kind != "fixed" {
                let v: Vec<String> = j.position.iter().map(|x| num(*x)).collect();
                return Err(UrdfError::Robot(format!(
                    "merge({an}, {bn}): joint {} moves ({}); freeze it first: freeze(src.{}, at: ..) (now at {} in URDF units)",
                    j.name,
                    j.kind,
                    j.of,
                    v.join(" ")
                )));
            }
            let (pn, cn, t) = (j.parent.clone(), j.child.clone(), j.origin);
            let why = format!("merged away (merge of {cn} into {pn})");
            m.gone.insert(j.name.clone(), why.clone());
            m.gone.insert(j.of.clone(), why);
            m.joints.remove(jk);
            let ci = m.links.iter().position(|l| l.name == cn).expect("the child");
            let c = m.links.remove(ci);
            if c.dummy || c.inertial_set && !c.pieces.is_empty() {
                return Err(UrdfError::Robot(format!("merge: {cn} cannot be merged")));
            }
            let pi = m.links.iter().position(|l| l.name == pn).expect("the parent");
            let p = &mut m.links[pi];
            let mv = |v: Visual| Visual {
                at: t.compose(&v.at),
                ..v
            };
            p.visuals.extend(c.visuals.into_iter().map(mv));
            p.collisions.extend(c.collisions.into_iter().map(mv));
            p.pieces.extend(c.pieces.into_iter().map(|x| Own {
                at: t.compose(&x.at),
                ..x
            }));
            p.bodies.extend(c.bodies);
            for j in m.joints.iter_mut().filter(|j| j.parent == cn) {
                j.parent = pn.clone();
                j.origin = t.compose(&j.origin);
            }
            m.aliases.insert(cn.clone(), pn.clone());
            // merged under the parent's name, unless the one named first was the child
            if cn == an {
                let pi = m.links.iter().position(|l| l.name == pn).expect("the parent");
                m.links[pi].name = an.clone();
                for j in &mut m.joints {
                    if j.parent == pn {
                        j.parent = an.clone();
                    }
                    if j.child == pn {
                        j.child = an.clone();
                    }
                }
                if m.root == pn {
                    m.root = an.clone();
                }
                m.aliases.insert(pn.clone(), an.clone());
                m.aliases.remove(&an);
            }
        }
    }
    // drops
    for t in &r.drops {
        let k = link_of(m, t)?;
        let whole = match t {
            Target::Source(b) => m.links[k].bodies == [b.clone()],
            Target::Name(_) => true,
        };
        if !whole {
            let Target::Source(b) = t else { unreachable!() };
            let l = &mut m.links[k];
            l.visuals.retain(|v| v.body != *b);
            l.collisions.retain(|v| v.body != *b);
            l.pieces.retain(|v| v.body != *b);
            l.bodies.retain(|x| x != b);
            continue;
        }
        let name = m.links[k].name.clone();
        if name == m.root {
            return Err(UrdfError::Robot(format!(
                "drop({name}): it is the root (re-root first)"
            )));
        }
        let kids: Vec<&str> = m
            .joints
            .iter()
            .filter(|j| j.parent == name)
            .map(|j| j.child.as_str())
            .collect();
        if !kids.is_empty() {
            return Err(UrdfError::Robot(format!(
                "drop({name}): {} hang from it and would be left floating; drop or merge them first",
                kids.join(" ")
            )));
        }
        if let Some(jk) = m.joints.iter().position(|j| j.child == name) {
            let jn = m.joints[jk].name.clone();
            if m.joints
                .iter()
                .any(|j| j.mimic.as_ref().is_some_and(|x| x.0 == jn))
            {
                return Err(UrdfError::Robot(format!("drop({name}): joint {jn} is mimicked")));
            }
            let why = format!("dropped with {name}");
            m.gone.insert(m.joints[jk].of.clone(), why.clone());
            m.gone.insert(jn, why);
            m.joints.remove(jk);
        }
        m.links.remove(k);
    }
    // frames: URDF-only fixed links
    for (name, on, at) in &r.frames {
        let k = link_of(m, on)?;
        let n = ident(name);
        if m.links.iter().any(|l| l.name == n) {
            return Err(UrdfError::Robot(format!("frame {n}: the name is taken")));
        }
        let parent = m.links[k].name.clone();
        let jn = format!("{n}_joint");
        if m.joints.iter().any(|j| j.name == jn) {
            return Err(UrdfError::Robot(format!("frame {n}: joint name {jn} is taken")));
        }
        m.links.push(Wl {
            name: n.clone(),
            bodies: vec![],
            visuals: vec![],
            collisions: vec![],
            pieces: vec![],
            inertial: None,
            inertial_set: true,
            dummy: false,
        });
        m.joints.push(Wj {
            name: jn,
            kind: "fixed",
            of: String::new(),
            parent,
            child: n,
            origin: *at,
            axis: None,
            limit: None,
            position: vec![],
            mimic: None,
            dynamics: None,
        });
    }
    // geometry: a statement replaces the link's visuals or collisions; more for the same add
    let mut replaced: BTreeSet<(usize, bool)> = BTreeSet::new();
    let mut explicit_collision: BTreeSet<String> = BTreeSet::new();
    let mut mnames = Names::default();
    for (n, _) in &ex.meshes {
        mnames.take(n);
    }
    // each mesh file packaged once, however many visuals use it
    let mut packaged: BTreeMap<String, String> = BTreeMap::new();
    for g in &r.geometry {
        let k = link_of(m, &g.link)?;
        let lname = m.links[k].name.clone();
        let base = |geo: Geo, at: Frame| Visual {
            name: format!("{lname}_{}", if g.collision { "collision" } else { "visual" }),
            body: String::new(),
            occ: String::new(),
            geo,
            at,
            material: None,
        };
        let new: Vec<Visual> = match &g.spec {
            GeoSpec::Box { size, at } => vec![base(Geo::Box(*size), *at)],
            GeoSpec::Cylinder { r, len, at } => vec![base(Geo::Cylinder { r: *r, len: *len }, *at)],
            GeoSpec::Sphere { r, at } => vec![base(Geo::Sphere(*r), *at)],
            GeoSpec::Capsule { r, len, at } => {
                // URDF has no capsule: a cylinder and a sphere at each end
                let end = |z: f64| at.compose(&Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z)));
                vec![
                    base(Geo::Cylinder { r: *r, len: *len }, *at),
                    base(Geo::Sphere(*r), end(len / 2.0)),
                    base(Geo::Sphere(*r), end(-len / 2.0)),
                ]
            }
            GeoSpec::Hull => vec![base(Geo::Hull, Frame::WORLD)],
            GeoSpec::Mesh {
                file,
                bytes,
                at,
                scale,
            } => {
                let p = Path::new(file);
                let ext = p
                    .extension()
                    .map_or_else(String::new, |x| x.to_string_lossy().to_ascii_lowercase());
                match bytes {
                    Some(_) if packaged.contains_key(file) => vec![base(
                        Geo::Mesh {
                            file: packaged[file].clone(),
                            scale: *scale,
                            packaged: true,
                        },
                        *at,
                    )],
                    Some(bytes) if ext == "stl" || ext == "obj" => {
                        let stem = p
                            .file_stem()
                            .map_or_else(|| "mesh".into(), |s| s.to_string_lossy().into_owned());
                        let f = format!("meshes/{}.{ext}", mnames.take(&stem));
                        let objects = if ext == "stl" {
                            forge_mesh::read_stl(bytes)
                                .map_err(|e| UrdfError::Mesh(format!("{file}: not an STL: {e:?}")))?
                                .0
                        } else {
                            let text = String::from_utf8_lossy(bytes);
                            forge_mesh::read_obj(&text)
                                .map_err(|e| UrdfError::Mesh(format!("{file}: not an OBJ: {e:?}")))?
                        };
                        m.points.insert(
                            f.clone(),
                            objects.iter().flat_map(|o| o.positions.iter().copied()).collect(),
                        );
                        ex.meshes.push((f.clone(), bytes.clone()));
                        packaged.insert(file.clone(), f.clone());
                        vec![base(
                            Geo::Mesh {
                                file: f,
                                scale: *scale,
                                packaged: true,
                            },
                            *at,
                        )]
                    }
                    _ => {
                        ex.warnings.push(format!(
                            "mesh {file} is referenced as written, not packaged (not a readable STL or OBJ)"
                        ));
                        vec![base(
                            Geo::Mesh {
                                file: file.clone(),
                                scale: *scale,
                                packaged: false,
                            },
                            *at,
                        )]
                    }
                }
            }
        };
        let l = &mut m.links[k];
        let list = if g.collision {
            &mut l.collisions
        } else {
            &mut l.visuals
        };
        if replaced.insert((k, g.collision)) {
            let material = list.iter().find_map(|v| v.material.clone());
            list.clear();
            if !g.collision {
                let material = g
                    .material
                    .as_ref()
                    .map(|(n, c)| (n.clone(), c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8)))
                    .or(material);
                list.extend(new.into_iter().map(|v| Visual {
                    material: material.clone(),
                    ..v
                }));
                continue;
            }
        }
        let material = g
            .material
            .as_ref()
            .map(|(n, c)| (n.clone(), c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8)));
        list.extend(new.into_iter().map(|v| Visual {
            material: if g.collision { None } else { material.clone() },
            ..v
        }));
        if g.collision {
            explicit_collision.insert(lname);
        }
    }
    if r.hull_collisions {
        for l in m
            .links
            .iter_mut()
            .filter(|l| !explicit_collision.contains(&l.name))
        {
            if l.visuals.iter().any(|v| matches!(v.geo, Geo::Mesh { .. })) && !replaced_any(&l.collisions) {
                l.collisions = vec![Visual {
                    name: format!("{}_hull", l.name),
                    body: String::new(),
                    occ: String::new(),
                    geo: Geo::Hull,
                    at: Frame::WORLD,
                    material: None,
                }];
            }
        }
    }
    // inertia
    for ov in &r.inertials {
        let k = link_of(m, &ov.link)?;
        let l = &mut m.links[k];
        let linked = if l.inertial_set {
            l.inertial.clone()
        } else {
            inertial(&pieces_of(l))
        };
        let what = format!("inertial({})", l.name);
        let (lm, lc, li) = match &linked {
            Some(i) => (
                i.mass,
                i.com.map(|x| x * 1000.0),
                i.inertia.map(|r| r.map(|x| x * 1e6)),
            ),
            None => (0.0, [0.0; 3], [[0.0; 3]; 3]),
        };
        let six = |i: &[[f64; 3]; 3]| [i[0][0], i[0][1], i[0][2], i[1][1], i[1][2], i[2][2]];
        if let (Some(was), Some(set)) = (ov.mass_was, ov.mass) {
            conflict(
                &mut conflicts,
                ov.line,
                &what,
                "mass",
                &[was],
                &[lm],
                &[set],
                false,
            );
        }
        if let (Some(was), Some(set)) = (ov.com_was, ov.com) {
            conflict(&mut conflicts, ov.line, &what, "com", &was, &lc, &set, false);
        }
        if let (Some(was), Some(set)) = (ov.inertia_was, ov.inertia) {
            conflict(
                &mut conflicts,
                ov.line,
                &what,
                "inertia",
                &was,
                &six(&li),
                &set,
                false,
            );
        }
        let mass = ov.mass.unwrap_or(lm);
        let com = ov.com.unwrap_or(lc);
        let inertia = match ov.inertia {
            Some(v) => [[v[0], v[1], v[2]], [v[1], v[3], v[4]], [v[2], v[4], v[5]]],
            None => li,
        };
        let inertia = inertia.map(|r| r.map(|x| x / 1e6));
        if mass.is_nan() || mass <= 0.0 || mass.is_infinite() {
            return Err(UrdfError::Robot(format!(
                "{what}: mass {} is not positive",
                num(mass)
            )));
        }
        if !physical(&inertia) {
            return Err(UrdfError::Robot(format!(
                "{what}: the inertia is not one a body can have (not positive definite, or a principal moment above the other two's sum)"
            )));
        }
        for (field, linked, set) in [
            ("mass", vec![lm], ov.mass.map(|v| vec![v])),
            ("com", lc.to_vec(), ov.com.map(|v| v.to_vec())),
            ("inertia", six(&li).to_vec(), ov.inertia.map(|v| v.to_vec())),
        ] {
            if let Some(set) = set {
                ex.overrides.push((
                    ov.line,
                    what.clone(),
                    field.into(),
                    fmt_vals(&linked),
                    fmt_vals(&set),
                ));
            }
        }
        l.inertial = Some(Inertial {
            mass,
            com: com.map(|x| x / 1000.0),
            inertia,
            err: None,
        });
        l.inertial_set = true;
        ex.notes
            .push(format!("{what} overridden: its bounds are not known"));
    }
    // joints
    for ov in &r.joints {
        let k = match joint_of(m, &ov.joint) {
            Ok(k) => k,
            Err(e) => {
                let name = match &ov.joint {
                    Target::Source(x) | Target::Name(x) => x,
                };
                match m.gone.get(name) {
                    Some(why) => {
                        ex.warnings.push(format!(
                            "line {}: joint {name} was {why}, so its override does nothing (remove it)",
                            ov.line
                        ));
                        continue;
                    }
                    None => return Err(e),
                }
            }
        };
        let what = format!("joint({})", m.joints[k].name);
        let j = &mut m.joints[k];
        // a fixed joint (frozen, or made fixed) has no motion: its motion's overrides say nothing
        if j.kind == "fixed" && ov.kind.is_none() {
            let moot: Vec<&str> = [
                ("axis", ov.axis.is_some()),
                ("limit", ov.limit.is_some()),
                ("effort", ov.effort.is_some()),
                ("velocity", ov.velocity.is_some()),
                ("damping", ov.damping.is_some()),
                ("friction", ov.friction.is_some()),
            ]
            .iter()
            .filter(|x| x.1)
            .map(|x| x.0)
            .collect();
            if !moot.is_empty() {
                ex.warnings.push(format!(
                    "line {}: {what} is fixed, so its {} override{} nothing (remove it, or unfreeze the joint)",
                    ov.line,
                    moot.join(", "),
                    if moot.len() == 1 { " does" } else { "s do" }
                ));
            }
            continue;
        }
        if let Some(kind) = &ov.kind {
            let kind: &'static str = match kind.as_str() {
                "revolute" => "revolute",
                "continuous" => "continuous",
                "prismatic" => "prismatic",
                "fixed" => "fixed",
                other => {
                    return Err(UrdfError::Robot(format!(
                        "{what}: type {other} (revolute, continuous, prismatic, fixed; freeze(..) fixes a source joint at a value)"
                    )));
                }
            };
            if j.kind == "planar" || j.kind == "floating" {
                return Err(UrdfError::Robot(format!(
                    "{what}: a {} joint's type is not changed here",
                    j.kind
                )));
            }
            ex.overrides
                .push((ov.line, what.clone(), "type".into(), j.kind.into(), kind.into()));
            j.kind = kind;
            if kind == "fixed" {
                j.axis = None;
                j.limit = None;
                j.position.clear();
            } else if j.axis.is_none() {
                j.axis = Some([0.0, 0.0, 1.0]);
                j.position = vec![0.0];
            }
            if kind == "continuous" {
                j.limit = None;
            }
        }
        let unit = if j.kind == "prismatic" { 1e-3 } else { 1.0 };
        if let Some(a) = ov.axis {
            let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            if n.is_nan() || n <= 0.0 {
                return Err(UrdfError::Robot(format!("{what}: axis (0, 0, 0)")));
            }
            if let (Some(was), Some(now)) = (ov.axis_was, j.axis) {
                conflict(&mut conflicts, ov.line, &what, "axis", &was, &now, &a, false);
            }
            ex.overrides.push((
                ov.line,
                what.clone(),
                "axis".into(),
                j.axis.map_or(String::new(), |x| fmt_vals(&x)),
                fmt_vals(&a),
            ));
            j.axis = Some(a.map(|x| x / n));
        }
        if let Some((lo, hi)) = ov.limit {
            if let Some(was) = ov.limit_was {
                let now = j.limit.map_or(vec![], |(a, b)| vec![a / unit, b / unit]);
                conflict(
                    &mut conflicts,
                    ov.line,
                    &what,
                    "limit",
                    &[was.0, was.1],
                    &now,
                    &[lo, hi],
                    unit == 1.0,
                );
            }
            if lo > hi {
                return Err(UrdfError::Robot(format!("{what}: limit lower above upper")));
            }
            if j.kind == "continuous" {
                j.kind = "revolute";
            }
            ex.overrides.push((
                ov.line,
                what.clone(),
                "limit".into(),
                j.limit
                    .map_or(String::new(), |(a, b)| fmt_in(&[a / unit, b / unit], unit == 1.0)),
                fmt_in(&[lo, hi], unit == 1.0),
            ));
            j.limit = Some((lo * unit, hi * unit));
        }
        if matches!(j.kind, "revolute" | "prismatic") && j.limit.is_none() {
            return Err(UrdfError::Robot(format!(
                "{what}: a {} joint needs a limit",
                j.kind
            )));
        }
        for (field, v) in [
            ("effort", ov.effort),
            ("velocity", ov.velocity),
            ("damping", ov.damping),
            ("friction", ov.friction),
        ] {
            if let Some(v) = v {
                ex.overrides
                    .push((ov.line, what.clone(), field.into(), String::new(), num(v)));
            }
        }
        if let Some(e) = ov.effort {
            o.effort_of.insert(j.name.clone(), e);
        }
        if let Some(v) = ov.velocity {
            o.velocity_of.insert(j.name.clone(), v);
        }
        if ov.damping.is_some() || ov.friction.is_some() {
            let (d0, f0) = j.dynamics.unwrap_or((0.0, 0.0));
            j.dynamics = Some((ov.damping.unwrap_or(d0), ov.friction.unwrap_or(f0)));
        }
    }
    let fixed: Vec<String> = m
        .joints
        .iter()
        .filter(|j| j.kind == "fixed")
        .map(|j| j.name.clone())
        .collect();
    for j in &m.joints {
        if let Some((leader, ..)) = &j.mimic
            && fixed.contains(leader)
        {
            return Err(UrdfError::Robot(format!(
                "joint {} mimics {leader}, which is now fixed",
                j.name
            )));
        }
    }
    // transmissions
    for (t, actuator, reduction, interface) in &r.transmissions {
        let k = joint_of(m, t)?;
        if m.joints[k].kind == "fixed" {
            return Err(UrdfError::Robot(format!(
                "transmission: joint {} is fixed",
                m.joints[k].name
            )));
        }
        m.transmissions.push((
            m.joints[k].name.clone(),
            actuator.clone(),
            *reduction,
            interface.clone(),
        ));
    }
    if !conflicts.is_empty() {
        return Err(UrdfError::Conflicts(conflicts));
    }
    Ok(())
}

/// An unlinked robot's model from its own links and joints: one root, every link reached, no
/// link a child twice. Positions are 0 (there is no solved pose).
fn unlinked(r: &Robot, o: &mut Options, ex: &mut Exported) -> Result<Model, UrdfError> {
    let at = |line: usize, m: String| UrdfError::Robot(format!("line {line}: {m}"));
    if r.link_defs.is_empty() {
        return Err(UrdfError::Robot(
            "an unlinked robot (source(none)) defines its links: NAME = link(mass: .., com: .., inertia: ..)"
                .into(),
        ));
    }
    let mut links = vec![];
    let mut names = BTreeSet::new();
    for d in &r.link_defs {
        let n = ident(&d.name);
        if !names.insert(n.clone()) {
            return Err(at(d.line, format!("link name {n} is taken")));
        }
        let inertial = match d.mass {
            None => None,
            Some(mass) => {
                let v = d.inertia;
                let i =
                    [[v[0], v[1], v[2]], [v[1], v[3], v[4]], [v[2], v[4], v[5]]].map(|r| r.map(|x| x / 1e6));
                if mass.is_nan() || mass <= 0.0 {
                    return Err(at(d.line, format!("{n}: mass {} is not positive", num(mass))));
                }
                // a defined link may be a point mass (zero inertia), never an impossible one
                let (p, _) = forge_topo::mass::eigen(i);
                let tol = 1e-12 * p[2].abs().max(1e-300);
                if p[0] < -tol || p[0] + p[1] < p[2] - tol {
                    return Err(at(d.line, format!("{n}: the inertia is not one a body can have")));
                }
                Some(Inertial {
                    mass,
                    com: d.com.map(|x| x / 1000.0),
                    inertia: i,
                    err: None,
                })
            }
        };
        links.push(Wl {
            name: n,
            bodies: vec![],
            visuals: vec![],
            collisions: vec![],
            pieces: vec![],
            inertial,
            inertial_set: true,
            dummy: false,
        });
    }
    let link = |t: &Target, line: usize| -> Result<String, UrdfError> {
        match t {
            Target::Name(n) if names.contains(&ident(n)) => Ok(ident(n)),
            Target::Name(n) => Err(at(line, format!("no link {n}"))),
            Target::Source(x) => Err(at(line, format!("src.{x}: an unlinked robot has no source"))),
        }
    };
    let mut joints: Vec<Wj> = vec![];
    let mut units = vec![];
    for d in &r.joint_defs {
        let n = ident(&d.name);
        if joints.iter().any(|j| j.name == n) {
            return Err(at(d.line, format!("joint name {n} is taken")));
        }
        let kind: &'static str = match d.kind.as_str() {
            "revolute" => "revolute",
            "continuous" => "continuous",
            "prismatic" => "prismatic",
            "fixed" => "fixed",
            "floating" => "floating",
            "planar" => "planar",
            other => return Err(at(d.line, format!("type {other}"))),
        };
        let unit = if kind == "prismatic" { 1e-3 } else { 1.0 };
        let axis = match kind {
            "fixed" | "floating" => None,
            // URDF's default axis
            _ => {
                let a = d.axis.unwrap_or([1.0, 0.0, 0.0]);
                let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                if l.is_nan() || l <= 0.0 {
                    return Err(at(d.line, format!("{n}: axis (0, 0, 0)")));
                }
                Some(a.map(|x| x / l))
            }
        };
        let limit = d.limit.map(|(a, b)| (a * unit, b * unit));
        if matches!(kind, "revolute" | "prismatic") && limit.is_none() {
            return Err(at(d.line, format!("{n}: a {kind} joint needs a limit")));
        }
        if let Some(e) = d.effort {
            o.effort_of.insert(n.clone(), e);
        }
        if let Some(v) = d.velocity {
            o.velocity_of.insert(n.clone(), v);
        }
        joints.push(Wj {
            name: n.clone(),
            kind,
            of: n,
            parent: link(&d.parent, d.line)?,
            child: link(&d.child, d.line)?,
            origin: d.at,
            axis,
            limit: if kind == "continuous" { None } else { limit },
            position: match kind {
                "fixed" | "floating" => vec![],
                "planar" => vec![0.0; 3],
                _ => vec![0.0],
            },
            mimic: None,
            dynamics: (d.damping.is_some() || d.friction.is_some())
                .then(|| (d.damping.unwrap_or(0.0), d.friction.unwrap_or(0.0))),
        });
        units.push(unit);
    }
    for (k, d) in r.joint_defs.iter().enumerate() {
        let Some((t, mult, off)) = &d.mimic else { continue };
        let Target::Name(leader) = t else {
            return Err(at(d.line, "mimic: a joint of this robot".into()));
        };
        let l = joints
            .iter()
            .position(|j| j.name == ident(leader))
            .ok_or_else(|| at(d.line, format!("mimic: no joint {leader}")))?;
        let (fu, lu) = (units[k], units[l]);
        joints[k].mimic = Some((joints[l].name.clone(), mult * fu / lu, off * fu));
    }
    // a tree: each link a child once, one root, every link reached from it
    let mut parent_of: BTreeMap<&str, &str> = BTreeMap::new();
    for j in &joints {
        if parent_of.insert(&j.child, &j.name).is_some() {
            return Err(UrdfError::Robot(format!(
                "link {} is the child of two joints",
                j.child
            )));
        }
    }
    let roots: Vec<&str> = links
        .iter()
        .map(|l| l.name.as_str())
        .filter(|n| !parent_of.contains_key(n))
        .collect();
    let [root] = roots.as_slice() else {
        return Err(UrdfError::Robot(format!(
            "an unlinked robot has one root link, not {} ({})",
            roots.len(),
            roots.join(" ")
        )));
    };
    let mut reached = BTreeSet::from([root.to_string()]);
    let mut grew = true;
    while grew {
        grew = false;
        for j in &joints {
            if reached.contains(&j.parent) && reached.insert(j.child.clone()) {
                grew = true;
            }
        }
    }
    if reached.len() != links.len() {
        let lost: Vec<&str> = links
            .iter()
            .map(|l| l.name.as_str())
            .filter(|n| !reached.contains(*n))
            .collect();
        return Err(UrdfError::Robot(format!(
            "links {} are not reached from the root {root} (a loop)",
            lost.join(" ")
        )));
    }
    let _ = ex;
    Ok(Model {
        root: root.to_string(),
        links,
        joints,
        points: BTreeMap::new(),
        transmissions: vec![],
        aliases: BTreeMap::new(),
        gone: BTreeMap::new(),
    })
}

fn replaced_any(c: &[Visual]) -> bool {
    c.iter().any(|v| v.body.is_empty())
}

fn pieces_of(l: &Wl) -> Vec<Piece<'_>> {
    l.pieces
        .iter()
        .map(|x| Piece {
            id: x.id.clone(),
            material: x.material.as_ref(),
            mass: &x.mass,
            at: x.at,
        })
        .collect()
}

/// The model made whole: each link's mass composed, its hull made.
fn finish(m: &mut Model, ex: &mut Exported) -> Result<(), UrdfError> {
    for l in &mut m.links {
        if !l.inertial_set {
            l.inertial = inertial(&pieces_of(l));
            if l.inertial.is_none() && !l.dummy {
                ex.notes.push(format!(
                    "link {} has no solids (references only): massless",
                    l.name
                ));
            }
        }
        if !l.collisions.iter().any(|c| c.geo == Geo::Hull) {
            continue;
        }
        let mut pts = vec![];
        for v in &l.visuals {
            if let Geo::Mesh { file, scale, .. } = &v.geo
                && let Some(p) = m.points.get(file)
            {
                // the file's units to mm
                let k = scale.map(|x| x * 1000.0);
                pts.extend(p.iter().map(|x| {
                    v.at.point_to_world(Point3::new(x.x * k[0], x.y * k[1], x.z * k[2]))
                }));
            }
        }
        let (positions, triangles) = hull(&pts).ok_or_else(|| {
            UrdfError::Mesh(format!(
                "link {}: no convex hull (its meshes are flat or empty)",
                l.name
            ))
        })?;
        let file = format!("meshes/{}_hull.stl", ident(&l.name));
        let object = forge_mesh::Object {
            name: format!("{}_hull", l.name),
            positions,
            triangles,
            ..forge_mesh::Object::default()
        };
        let (bytes, ok) = stl(vec![object], &file);
        ex.watertight &= ok;
        ex.meshes.push((file.clone(), bytes));
        for c in &mut l.collisions {
            if c.geo == Geo::Hull {
                c.geo = Geo::Mesh {
                    file: file.clone(),
                    scale: [0.001; 3],
                    packaged: true,
                };
                c.at = Frame::WORLD;
            }
        }
    }
    Ok(())
}

// ---- convex hull ------------------------------------------------------------------------------

/// The convex hull of points: its corners and outward triangles, or None when the points span
/// no volume. Incremental (each point in turn: the faces it sees go, the horizon is fanned to
/// it), in the points' order, so the same points give the same hull. A point within `eps` of a
/// face's plane counts as on it: the hull holds every point to within `eps` (1e-9 of the size).
#[allow(clippy::type_complexity)]
fn hull(pts: &[Point3]) -> Option<(Vec<Point3>, Vec<[u32; 3]>)> {
    // distinct points, in order
    let mut seen = BTreeSet::new();
    let p: Vec<Point3> = pts
        .iter()
        .filter(|q| seen.insert([q.x.to_bits(), q.y.to_bits(), q.z.to_bits()]))
        .copied()
        .collect();
    if p.len() < 4 {
        return None;
    }
    let size = p
        .iter()
        .fold(0.0f64, |m, q| m.max(q.x.abs()).max(q.y.abs()).max(q.z.abs()));
    let eps = 1e-9 * size.max(1.0);
    // a first tetrahedron: the extremes in x, the farthest from their line, from their plane
    let (mut i0, mut i1) = (0, 0);
    for (k, q) in p.iter().enumerate() {
        if q.x < p[i0].x {
            i0 = k;
        }
        if q.x > p[i1].x {
            i1 = k;
        }
    }
    if i0 == i1 {
        i1 = (0..p.len()).max_by(|&a, &b| (p[a] - p[i0]).length().total_cmp(&(p[b] - p[i0]).length()))?;
    }
    let d01 = p[i1] - p[i0];
    let i2 = (0..p.len()).max_by(|&a, &b| {
        d01.cross(p[a] - p[i0])
            .length()
            .total_cmp(&d01.cross(p[b] - p[i0]).length())
    })?;
    let n = d01.cross(p[i2] - p[i0]);
    if n.length() <= eps * d01.length() {
        return None;
    }
    let i3 = (0..p.len()).max_by(|&a, &b| n.dot(p[a] - p[i0]).abs().total_cmp(&n.dot(p[b] - p[i0]).abs()))?;
    if n.dot(p[i3] - p[i0]).abs() <= eps * n.length() {
        return None;
    }
    let plane = |f: &[usize; 3]| {
        let n = (p[f[1]] - p[f[0]]).cross(p[f[2]] - p[f[0]]);
        let l = n.length();
        (n / l, f[0])
    };
    let mut faces: Vec<[usize; 3]> = if n.dot(p[i3] - p[i0]) < 0.0 {
        vec![[i0, i1, i2], [i0, i3, i1], [i1, i3, i2], [i2, i3, i0]]
    } else {
        vec![[i0, i2, i1], [i0, i1, i3], [i1, i2, i3], [i2, i0, i3]]
    };
    let mut planes: Vec<(Vec3, usize)> = faces.iter().map(plane).collect();
    // farthest from the first tetrahedron's middle first: the outermost points make the hull
    // early, and points on its faces after them are skipped (ties in index order)
    let mid = Point3::new(
        (p[i0].x + p[i1].x + p[i2].x + p[i3].x) / 4.0,
        (p[i0].y + p[i1].y + p[i2].y + p[i3].y) / 4.0,
        (p[i0].z + p[i1].z + p[i2].z + p[i3].z) / 4.0,
    );
    let mut order: Vec<usize> = (0..p.len()).filter(|k| ![i0, i1, i2, i3].contains(k)).collect();
    order.sort_by(|&a, &b| {
        (p[b] - mid)
            .length()
            .total_cmp(&(p[a] - mid).length())
            .then(a.cmp(&b))
    });
    for k in order {
        let q = &p[k];
        let visible: Vec<bool> = planes.iter().map(|(n, a)| n.dot(*q - p[*a]) > eps).collect();
        if !visible.iter().any(|&v| v) {
            continue;
        }
        let mut edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        for (f, _) in faces.iter().zip(&visible).filter(|x| *x.1) {
            for e in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                edges.insert(e);
            }
        }
        let horizon: Vec<(usize, usize)> = faces
            .iter()
            .zip(&visible)
            .filter(|x| *x.1)
            .flat_map(|(f, _)| [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])])
            .filter(|&(a, b)| !edges.contains(&(b, a)))
            .collect();
        let mut keep_f = vec![];
        let mut keep_p = vec![];
        for ((f, pl), v) in faces.iter().zip(&planes).zip(&visible) {
            if !v {
                keep_f.push(*f);
                keep_p.push(*pl);
            }
        }
        faces = keep_f;
        planes = keep_p;
        for (a, b) in horizon {
            let f = [a, b, k];
            planes.push(plane(&f));
            faces.push(f);
        }
    }
    // the corners used, renumbered in order
    let mut index = BTreeMap::new();
    let mut positions = vec![];
    let mut triangles = vec![];
    for f in &faces {
        let t = f.map(|i| {
            *index.entry(i).or_insert_with(|| {
                positions.push(p[i]);
                (positions.len() - 1) as u32
            })
        });
        triangles.push(t);
    }
    Some((positions, triangles))
}

// ---- the robot as an editor draws and drives it -----------------------------------------------

/// The robot as an editor draws and drives it: every link with its geometry and centre of mass,
/// every joint with its origin, axis, limits, value at the export pose and mimic. Lengths mm
/// (positions and limits in URDF units: m or rad, as the joint's kind).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RobotView {
    pub root: String,
    pub links: Vec<ViewLink>,
    pub joints: Vec<ViewJoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewLink {
    pub name: String,
    /// The source instances it holds.
    pub bodies: Vec<String>,
    pub mass: Option<f64>,
    /// mm, link frame.
    pub com: [f64; 3],
    pub visuals: Vec<ViewGeo>,
    pub collisions: Vec<ViewGeo>,
    /// A chain's massless link (or a frame a robot added).
    pub dummy: bool,
}

/// One visual or collision: for a source part's own mesh, `name` is its occurrence's path.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewGeo {
    pub name: String,
    pub shape: Shape,
    /// mm, link frame.
    pub at: Frame,
}

/// A geometry's shape, mm (a mesh's file in its own units, `scale` to metres).
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Mesh { file: String, scale: [f64; 3] },
    Box([f64; 3]),
    Cylinder { r: f64, len: f64 },
    Sphere(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewJoint {
    pub name: String,
    pub kind: &'static str,
    /// The source joint it carries (or its own name).
    pub of: String,
    pub parent: String,
    pub child: String,
    /// mm, the parent link's frame.
    pub origin: Frame,
    pub axis: Option<[f64; 3]>,
    pub limit: Option<(f64, f64)>,
    pub position: Vec<f64>,
    /// (leader, multiplier, offset), URDF units.
    pub mimic: Option<(String, f64, f64)>,
    /// Effort and velocity as the export writes them (None: not given), and (damping, friction).
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    pub dynamics: Option<(f64, f64)>,
}

/// A turn by `t` about the unit axis `u` (Rodrigues), as a frame at the origin.
fn turn(u: [f64; 3], t: f64) -> Frame {
    let (s, c) = dmath::sin_cos(t);
    let k = 1.0 - c;
    let (x, y, z) = (u[0], u[1], u[2]);
    let col = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
    let ex = col([c + x * x * k, y * x * k + z * s, z * x * k - y * s]);
    let ez = col([x * z * k + y * s, y * z * k - x * s, c + z * z * k]);
    let ez = ez.normalize().unwrap_or(forge_math::UnitVec3::Z);
    Frame::from_zx(Point3::ORIGIN, ez, ex).unwrap_or(Frame::WORLD)
}

impl ViewJoint {
    /// Its motion at these values (URDF units), mm.
    pub fn motion(&self, q: &[f64]) -> Frame {
        let v = |i: usize| q.get(i).copied().unwrap_or(0.0);
        let a = self.axis.unwrap_or([1.0, 0.0, 0.0]);
        match self.kind {
            "revolute" | "continuous" => turn(a, v(0)),
            "prismatic" => Frame::WORLD.with_origin(Point3::new(
                a[0] * v(0) * 1000.0,
                a[1] * v(0) * 1000.0,
                a[2] * v(0) * 1000.0,
            )),
            "planar" => Frame::WORLD
                .with_origin(Point3::new(v(0) * 1000.0, v(1) * 1000.0, 0.0))
                .compose(&turn([0.0, 0.0, 1.0], v(2))),
            _ => Frame::WORLD,
        }
    }

    /// A value held within the joint's limits.
    pub fn clamp(&self, v: f64) -> f64 {
        match self.limit {
            Some((lo, hi)) => v.clamp(lo, hi),
            None => v,
        }
    }
}

impl RobotView {
    /// Each link's frame in the robot's (mm) with these joint values (URDF units; a joint not
    /// given keeps its export position); a mimic follows its leader.
    pub fn place(&self, q: &BTreeMap<String, f64>) -> BTreeMap<String, Frame> {
        let value = |j: &ViewJoint| -> Vec<f64> {
            if let Some((leader, m, off)) = &j.mimic {
                let l = self.joints.iter().find(|x| x.name == *leader);
                let lv = q
                    .get(leader)
                    .copied()
                    .or_else(|| l.and_then(|l| l.position.first().copied()))
                    .unwrap_or(0.0);
                return vec![m * lv + off];
            }
            match q.get(&j.name) {
                Some(v) => vec![*v],
                None => j.position.clone(),
            }
        };
        let mut at = BTreeMap::from([(self.root.clone(), Frame::WORLD)]);
        let mut grew = true;
        while grew {
            grew = false;
            for j in &self.joints {
                if at.contains_key(&j.child) {
                    continue;
                }
                if let Some(p) = at.get(&j.parent).copied() {
                    at.insert(
                        j.child.clone(),
                        p.compose(&j.origin).compose(&j.motion(&value(j))),
                    );
                    grew = true;
                }
            }
        }
        at
    }

    /// Every visual of a source part, by its occurrence's path, where these link frames put it.
    pub fn occurrences(&self, frames: &BTreeMap<String, Frame>) -> Vec<(String, Frame)> {
        let mut out = vec![];
        for l in &self.links {
            let Some(f) = frames.get(&l.name) else { continue };
            for v in &l.visuals {
                out.push((v.name.clone(), f.compose(&v.at)));
            }
        }
        out
    }
}

fn view_geo(v: &Visual) -> Option<ViewGeo> {
    let shape = match &v.geo {
        Geo::Mesh { file, scale, .. } => Shape::Mesh {
            file: file.clone(),
            scale: *scale,
        },
        Geo::Box(b) => Shape::Box(*b),
        Geo::Cylinder { r, len } => Shape::Cylinder { r: *r, len: *len },
        Geo::Sphere(r) => Shape::Sphere(*r),
        Geo::Hull => return None,
    };
    Some(ViewGeo {
        name: if v.occ.is_empty() {
            v.name.clone()
        } else {
            v.occ.clone()
        },
        shape,
        at: v.at,
    })
}

fn view_of(links: &[Wl], joints: &[Wj], root: &str, o: &Options) -> RobotView {
    RobotView {
        root: root.to_string(),
        links: links
            .iter()
            .map(|l| ViewLink {
                name: l.name.clone(),
                bodies: l.bodies.clone(),
                mass: l.inertial.as_ref().map(|i| i.mass),
                com: l
                    .inertial
                    .as_ref()
                    .map_or([0.0; 3], |i| i.com.map(|x| x * 1000.0)),
                visuals: l.visuals.iter().filter_map(view_geo).collect(),
                collisions: l.collisions.iter().filter_map(view_geo).collect(),
                dummy: l.dummy || l.bodies.is_empty(),
            })
            .collect(),
        joints: joints
            .iter()
            .map(|j| ViewJoint {
                name: j.name.clone(),
                kind: j.kind,
                of: j.of.clone(),
                parent: j.parent.clone(),
                child: j.child.clone(),
                origin: j.origin,
                axis: j.axis,
                limit: j.limit,
                position: j.position.clone(),
                mimic: j.mimic.clone(),
                effort: has_limit(j).then(|| actuation(o, j).0).flatten(),
                velocity: has_limit(j).then(|| actuation(o, j).1).flatten(),
                dynamics: j.dynamics,
            })
            .collect(),
    }
}

pub mod import;
pub mod robot;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        let mut n = Names::default();
        assert_eq!(n.take("lid.hp"), "lid_hp");
        assert_eq!(n.take("lid_hp"), "lid_hp_2");
        assert_eq!(n.take("3d"), "_3d");
        assert_eq!(ident("bolt[1]"), "bolt_1_");
    }

    #[test]
    fn numbers() {
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(0.001), "0.001");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(4.5e-21), "4.5e-21");
        assert_eq!(num(-2e-5), "-2e-5");
        assert_eq!(comment("a--b-"), "a- -b- ");
    }

    #[test]
    fn a_hull_holds_its_points_and_is_closed() {
        // a cube's corners, points inside it and on its faces, and a sliver off one face
        let mut pts = vec![];
        for i in 0..27 {
            let c = |k: usize| ((i / 3usize.pow(k as u32)) % 3) as f64 * 5.0;
            pts.push(Point3::new(c(0), c(1), c(2)));
        }
        pts.push(Point3::new(5.0, 5.0, 10.000_001));
        let (p, t) = hull(&pts).expect("a hull");
        // V − E + F = 2 for a closed surface of triangles: E = 3F/2
        assert_eq!(p.len() as i64 - (3 * t.len() / 2) as i64 + t.len() as i64, 2);
        // every extreme point is a corner (a first-tetrahedron point on a face may stay too)
        for i in [0, 2, 6, 8, 18, 20, 24, 26, 27] {
            assert!(p.contains(&pts[i]), "{:?} is a corner", pts[i]);
        }
        assert!(p.len() <= 10, "{} corners", p.len());
        for q in &pts {
            for tri in &t {
                let [a, b, c] = tri.map(|i| p[i as usize]);
                let n = (b - a).cross(c - a);
                assert!(n.dot(*q - a) <= 1e-9 * n.length(), "a point outside");
            }
        }
        // flat points have no hull
        let flat: Vec<Point3> = (0..10)
            .map(|i| Point3::new(i as f64, (i * i) as f64, 0.0))
            .collect();
        assert!(hull(&flat).is_none());
    }

    #[test]
    fn rpy_round_trips() {
        for &(r, p, y) in &[(0.1, 0.2, 0.3), (-2.0, 1.2, 3.0), (0.0, -1.5, -0.7)] {
            let f = motion(2, false, y)
                .compose(&motion(1, false, p))
                .compose(&motion(0, false, r));
            let [r2, p2, y2] = rpy(&f);
            assert!((r - r2).abs() < 1e-12 && (p - p2).abs() < 1e-12 && (y - y2).abs() < 1e-12);
        }
    }
}
