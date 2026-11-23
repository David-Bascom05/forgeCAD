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

use crate::Run;
use crate::props::{Piece, combine};
use forge_agent::json::Json;
use forge_asm as fa;
use forge_asm::{ConstraintKind, Freedom, JointKind};
use forge_math::{Frame, Point3, Vec3, dmath};
use forge_mesh::scene::{Scene, Solids};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::path::Path;

// ---- the solved structure ---------------------------------------------------------------------

/// An assembly's kinematic structure as solved: what the export reads (`AsmOut::kin`).
#[derive(Clone, Debug, Default)]
pub struct Kin {
    /// The top-level instances not suppressed, in document order.
    pub bodies: Vec<KinBody>,
    /// Joints and loose constraints not suppressed, in document order.
    pub links: Vec<KinLink>,
    pub relations: Vec<KinRelation>,
    /// Each of `AsmOut::occurrences`' top-level body (an index into `bodies`).
    pub occ_body: Vec<Option<usize>>,
}

#[derive(Clone, Debug)]
pub struct KinBody {
    pub name: String,
    pub ground: bool,
    /// Its solved pose.
    pub pose: Frame,
}

#[derive(Clone, Debug)]
pub struct KinLink {
    pub id: String,
    /// The joint's kind; None for a loose constraint (`constraint` names its kind).
    pub kind: Option<JointKind>,
    pub constraint: Option<ConstraintKind>,
    /// Each end's body (A, B), None where its instance is not placed.
    pub ends: [Option<usize>; 2],
    /// Each end's anchor frame in the world as solved: A's flipped when the joint says so.
    pub at: [Option<Frame>; 2],
    /// The solved values (mm, rad), and the limits as written.
    pub values: Vec<(Freedom, f64)>,
    pub limits: Vec<(Freedom, f64, f64)>,
}

impl KinLink {
    /// Rigid: a fastened joint or a lock constraint.
    pub fn rigid(&self) -> bool {
        self.kind == Some(JointKind::Fastened) || self.constraint == Some(ConstraintKind::Lock)
    }
}

#[derive(Clone, Debug)]
pub struct KinRelation {
    pub id: String,
    pub kind: &'static str,
    /// d follower / d leader, in forge's units (mm, rad).
    pub slope: f64,
    pub leader: (String, Freedom),
    pub follower: (String, Freedom),
    pub offset: Option<f64>,
}

type Namer<'a> = &'a dyn Fn(u64) -> Option<String>;

/// The structure of a solved assembly level, names given by id.
pub fn kin_of<P: fa::Parts>(
    asm: &fa::Assembly,
    sol: &fa::Solution,
    parts: &P,
    inst_name: Namer,
    link_name: Namer,
    rel_name: Namer,
) -> Kin {
    let mut kin = Kin::default();
    let mut index: BTreeMap<fa::InstanceId, usize> = BTreeMap::new();
    for inst in asm.instances.iter().filter(|i| !i.suppressed) {
        if index.contains_key(&inst.id) {
            continue;
        }
        index.insert(inst.id, kin.bodies.len());
        kin.bodies.push(KinBody {
            name: inst_name(inst.id.0).unwrap_or_else(|| format!("i{}", inst.id.0)),
            ground: inst.ground,
            pose: sol.pose(inst.id).unwrap_or(inst.pose),
        });
    }
    let world = |e: &fa::End| -> Option<Frame> {
        let local = |info: Option<&fa::PartInfo>| match &e.anchor {
            fa::AnchorRef::Local(r) => Some(r.frame),
            fa::AnchorRef::Named(n) => info?.refs.get(n)?.as_ref().ok().map(|r| r.frame),
        };
        if let Some(o) = sol.occurrences.iter().find(|o| o.path == e.path) {
            return Some(o.pose.compose(&local(parts.part(&o.part))?));
        }
        match (&e.anchor, e.path.as_slice()) {
            (fa::AnchorRef::Local(r), [only]) => Some(sol.pose(*only)?.compose(&r.frame)),
            _ => None,
        }
    };
    let body = |e: &fa::End| e.path.first().and_then(|i| index.get(i).copied());
    for j in asm.joints.iter().filter(|j| !j.suppressed) {
        let a = world(&j.a).map(|f| if j.flip { f.flipped() } else { f });
        kin.links.push(KinLink {
            id: link_name(j.id.0).unwrap_or_else(|| format!("j{}", j.id.0)),
            kind: Some(j.kind),
            constraint: None,
            ends: [body(&j.a), body(&j.b)],
            at: [a, world(&j.b)],
            values: sol
                .joints
                .iter()
                .find(|r| r.joint == j.id)
                .map(|r| r.values.clone())
                .unwrap_or_default(),
            limits: j.limits.clone(),
        });
    }
    for c in asm.constraints.iter().filter(|c| !c.suppressed) {
        let a = world(&c.a).map(|f| if c.flip { f.flipped() } else { f });
        kin.links.push(KinLink {
            id: link_name(c.id).unwrap_or_else(|| format!("c{}", c.id)),
            kind: None,
            constraint: Some(c.kind),
            ends: [body(&c.a), body(&c.b)],
            at: [a, world(&c.b)],
            values: vec![],
            limits: vec![],
        });
    }
    for r in asm.relations.iter().filter(|r| !r.suppressed) {
        let joint = |v: &fa::ValueRef| link_name(v.joint.0).unwrap_or_else(|| format!("j{}", v.joint.0));
        kin.relations.push(KinRelation {
            id: rel_name(r.id.0).unwrap_or_else(|| format!("r{}", r.id.0)),
            kind: r.kind.name(),
            slope: r.kind.slope(),
            leader: (joint(&r.leader), r.leader.freedom),
            follower: (joint(&r.follower), r.follower.freedom),
            offset: r.offset,
        });
    }
    kin.occ_body = sol
        .occurrences
        .iter()
        .map(|o| o.path.first().and_then(|i| index.get(i).copied()))
        .collect();
    kin
}

// ---- options and errors -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The meshes' chord tolerance (mm) and angle (degrees), as `meshfile`'s.
    pub tol: f64,
    pub angle: f64,
    /// `<limit effort velocity>` for every moving joint: URDF requires them, forgeCAD has no
    /// actuator data. None: 0, with a warning.
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    /// ± this travel (mm) for a translation with no limit; None: such a joint is refused.
    pub travel: Option<f64>,
    /// Joints (or loose constraints) left out, to open closed loops.
    pub cut: Vec<String>,
    /// Mesh paths as `package://NAME/meshes/..`; None: relative `meshes/..`.
    pub package: Option<String>,
}

impl Default for Options {
    fn default() -> Options {
        let m = crate::meshfile::Options::new(crate::meshfile::Format::Stl);
        Options {
            tol: m.tol,
            angle: m.angle,
            effort: None,
            velocity: None,
            travel: None,
            cut: vec![],
            package: None,
        }
    }
}

impl Options {
    /// `urdf [tol MM] [angle DEG] [effort N] [velocity V] [travel MM] [cut J].. [package NAME]`.
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
                "effort" => o.effort = Some(num(it.next(), "effort")?),
                "velocity" => o.velocity = Some(num(it.next(), "velocity")?),
                "travel" => o.travel = Some(num(it.next(), "travel")?),
                "cut" => o.cut.push(it.next().ok_or("cut takes a joint's name")?.clone()),
                "package" => o.package = Some(it.next().ok_or("package takes a ROS package name")?.clone()),
                _ => {
                    return Err(format!(
                        "export urdf: unknown word '{w}' (tol MM, angle DEG, effort N, velocity V, travel MM, cut JOINT, package NAME)"
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
}

impl UrdfError {
    pub fn kind(&self) -> &'static str {
        match self {
            UrdfError::NotBuilt(_) => "NotBuilt",
            UrdfError::NoMaterial(_) => "NoMaterial",
            UrdfError::UnlimitedTranslation { .. } => "UnlimitedTranslation",
            UrdfError::BallLimits { .. } => "BallLimits",
            UrdfError::ClosedLoop { .. } => "ClosedLoop",
            UrdfError::LooseConstraint { .. } => "LooseConstraint",
            UrdfError::Relation { .. } => "Relation",
            UrdfError::NoSuchJoint(_) => "NoSuchJoint",
            UrdfError::Unresolved { .. } => "Unresolved",
            UrdfError::Mass(_) => "Mass",
            UrdfError::Mesh(_) => "Mesh",
        }
    }

    pub fn text(&self) -> String {
        match self {
            UrdfError::NotBuilt(m) | UrdfError::Mass(m) | UrdfError::Mesh(m) => m.clone(),
            UrdfError::NoMaterial(s) => format!(
                "no material on {}: a link's mass needs one (properties(material: ..)); no density is assumed",
                s.join(", ")
            ),
            UrdfError::UnlimitedTranslation { joint, freedom } => format!(
                "joint {joint}'s {} has no limit, and a URDF prismatic joint needs one: give the joint a limit, or export with `travel MM`",
                freedom.name()
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
}

/// One URDF link to write.
struct Wl {
    name: String,
    bodies: Vec<String>,
    visuals: Vec<Visual>,
    inertial: Option<Inertial>,
    dummy: bool,
}

struct Visual {
    name: String,
    mesh: String,
    /// In the link frame.
    at: Frame,
    material: Option<(String, [u8; 3])>,
}

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
        inertia: i.map(|r| r.map(|x| x * 1e-6)),
        err: finite.then_some([em, ec / 1000.0, ei * 1e-6]),
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
    if let Some(e) = run.errors.first() {
        return Err(UrdfError::NotBuilt(format!(
            "the {} did not build: {}: {}",
            if run.asm.is_some() { "assembly" } else { "part" },
            e.kind(),
            e.text()
        )));
    }
    let robot = ident(name);
    let mut ex = Exported {
        name: robot.clone(),
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
    };
    let (links, joints) = match &run.asm {
        Some(asm) => assembly(run, asm, o, &mut ex)?,
        None => (vec![part(run, name, dir, o, &mut ex)?], vec![]),
    };
    if joints.iter().any(|j| j.limit.is_some()) && (o.effort.is_none() || o.velocity.is_none()) {
        ex.warnings.push(
            "effort and velocity unknown (forgeCAD has no actuator data): written as 0 where not given; export with `effort N velocity V`"
                .into(),
        );
    }
    ex.urdf = write(&robot, &links, &joints, o, &ex);
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
fn part(run: &Run, name: &str, dir: Option<&Path>, o: &Options, ex: &mut Exported) -> Result<Wl, UrdfError> {
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
                .map(|m| [k * m.err[0], m.err[2] / 1000.0, k * m.err[3] * 1e-6])
        }
        _ => {
            ex.notes
                .push("solids of several densities: the mass bounds are UNKNOWN (not composed)".into());
            None
        }
    };
    let mut mo = crate::meshfile::Options::new(crate::meshfile::Format::Stl);
    mo.tol = o.tol;
    mo.angle = o.angle;
    let scene = crate::meshfile::scene(run, name, dir, &mo).map_err(UrdfError::Mesh)?;
    let mesh_name = format!("meshes/{}.stl", ident(name));
    let (bytes, ok) = stl(scene.flatten().objects, name);
    ex.watertight &= ok;
    ex.meshes.push((mesh_name.clone(), bytes));
    let material = phys.solids.iter().find_map(|s| s.material.clone()).map(|m| {
        let libs = forge_material::Libraries::cached(dir);
        let rgb = libs.find(&m).map_or([0x80; 3], |x| x.1.colour);
        (m, rgb)
    });
    Ok(Wl {
        name: ident(name),
        bodies: vec![name.to_string()],
        visuals: vec![Visual {
            name: ident(name),
            mesh: mesh_name,
            at: Frame::WORLD,
            material,
        }],
        inertial: Some(Inertial {
            mass,
            com: phys.centroid.map(|x| x / 1000.0),
            inertia: i.map(|r| r.map(|x| x * 1e-6)),
            err,
        }),
        dummy: false,
    })
}

/// A solid in a link: its occurrence, material, unit-density mass properties, and where it is
/// in the link's frame.
type Held<'a> = (
    String,
    Option<&'a forge_material::Material>,
    &'a forge_topo::mass::MassProps,
    Frame,
);

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
    asm: &crate::asm::AsmOut,
    o: &Options,
    ex: &mut Exported,
) -> Result<(Vec<Wl>, Vec<Wj>), UrdfError> {
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
    let root = first_ground.map_or(0, |g| group_of[g]);
    if first_ground.is_none() {
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
        order: vec![root],
        wj: vec![],
        dummies: vec![],
        carries: BTreeMap::new(),
        native_planar: BTreeSet::new(),
        lnames,
        jnames,
    };
    st.frame[root] = Some(Frame::WORLD);
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
        });
        tree_cx.grow_from(g, &mut st, ex)?;
    }
    let State {
        frame,
        order,
        mut wj,
        dummies,
        carries,
        native_planar,
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
        .filter(|l| !l.owner.is_empty() && l.family == crate::asm::LinkFamily::Joint && !l.suppressed)
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
        let mut mo = crate::meshfile::Options::new(crate::meshfile::Format::Stl);
        mo.tol = o.tol;
        mo.angle = o.angle;
        mo.quality()
    };
    let mut mesh_of: Vec<Option<String>> = vec![None; asm.parts.len()];
    let mut mnames = Names::default();
    let mut visuals: Vec<Vec<Visual>> = (0..groups.len()).map(|_| vec![]).collect();
    let mut pieces: Vec<Vec<Held>> = (0..groups.len()).map(|_| vec![]).collect();
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
        let lf = frame[g].expect("every group placed");
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
                        UrdfError::Mesh(format!("meshing {stem} failed: {}", crate::error::words(&e)))
                    })?;
                    let f = format!("meshes/{}.stl", mnames.take(&stem));
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
                mesh: file,
                at,
                material,
            });
        }
        let Some(sum) = sum else { continue };
        let masses = sum.masses().map_err(|e| UrdfError::Mass(e.to_string()))?;
        for (s, m) in sum.solids.iter().zip(masses) {
            if body != 0 && crate::asm::hid(&s.0) != body {
                continue;
            }
            if s.1.is_none() {
                missing.push(format!("{path} ({})", s.0));
            }
            pieces[g].push((path.clone(), s.1.as_ref(), m, at));
        }
    }
    if !missing.is_empty() {
        return Err(UrdfError::NoMaterial(missing));
    }
    let _ = run;
    let mut links = vec![];
    for &g in &order {
        let ps: Vec<Piece> = pieces[g]
            .iter()
            .map(|(id, mat, m, at)| Piece {
                id: id.clone(),
                material: *mat,
                mass: m,
                at: *at,
            })
            .collect();
        let inertial = inertial(&ps);
        if inertial.is_none() {
            ex.notes.push(format!(
                "link {} has no solids (references only): massless",
                gname[g]
            ));
        }
        links.push(Wl {
            name: gname[g].clone(),
            bodies: groups[g].iter().map(|&b| kin.bodies[b].name.clone()).collect(),
            visuals: std::mem::take(&mut visuals[g]),
            inertial,
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
    Ok((links, wj))
}

/// What the tree walk builds.
struct State {
    frame: Vec<Option<Frame>>,
    order: Vec<usize>,
    wj: Vec<Wj>,
    dummies: Vec<Wl>,
    /// The URDF joint carrying each forge (joint, freedom), and its value's unit (1 or 1e-3).
    carries: BTreeMap<(String, Freedom), (String, f64)>,
    native_planar: BTreeSet<String>,
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
        let (jo_world, other) = if forward { (bw, aw) } else { (aw, bw) };
        let mut cur = jo_world;
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
                origin: pf.inverse().compose(&jo_world),
                axis: Some([0.0, 0.0, 1.0]),
                limit: None,
                position: vec![
                    m.origin.x / 1000.0,
                    m.origin.y / 1000.0,
                    dmath::atan2(m.x().get().y, m.x().get().x),
                ],
                mimic: None,
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
                    pf.inverse().compose(&jo_world)
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
                });
                if !last {
                    // the massless link between, where the chain has got to
                    st.dummies.push(Wl {
                        name: child.clone(),
                        bodies: vec![],
                        visuals: vec![],
                        inertial: None,
                        dummy: true,
                    });
                    parent = child;
                }
            }
        }
        ex.residual = ex.residual.max(frame_gap(&cur, &other, self.size));
        st.frame[to] = Some(cur);
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

// ---- the XML ----------------------------------------------------------------------------------

fn write(robot: &str, links: &[Wl], joints: &[Wj], o: &Options, ex: &Exported) -> String {
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
    let (effort, velocity) = (o.effort.unwrap_or(0.0), o.velocity.unwrap_or(0.0));
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
        for v in &l.visuals {
            let mesh = format!(
                "<mesh filename=\"{}\" scale=\"0.001 0.001 0.001\"/>",
                attr(&mesh_uri(o, &v.mesh))
            );
            let _ = writeln!(s, "    <visual name=\"{}\">", attr(&v.name));
            let _ = writeln!(s, "      {}", origin(&v.at));
            let _ = writeln!(s, "      <geometry>{mesh}</geometry>");
            if let Some((name, rgb)) = &v.material {
                let c = rgb.map(|x| x as f64 / 255.0);
                let _ = writeln!(
                    s,
                    "      <material name=\"{}\"><color rgba=\"{} 1\"/></material>",
                    attr(&ident(name)),
                    nums(&c)
                );
            }
            let _ = writeln!(s, "    </visual>");
            let _ = writeln!(s, "    <collision name=\"{}\">", attr(&v.name));
            let _ = writeln!(s, "      {}", origin(&v.at));
            let _ = writeln!(s, "      <geometry>{mesh}</geometry>");
            let _ = writeln!(s, "    </collision>");
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
    let _ = writeln!(s, "</robot>");
    s
}

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
