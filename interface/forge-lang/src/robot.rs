//! The `robot` document: a URDF robot linked to an assembly or part, with explicit, persistent
//! overrides on top (research/export/urdf-editor.md). Its first statement is the bare name
//! `robot`; the rest are the language's own statements, read by the same parser:
//!
//! ```text
//! robot
//! shared eff = 20
//! src = source("arm2.fg")
//! root(src.base)
//! forearm = link(src.fore)                       # the link holding `fore`, named forearm
//! freeze(src.grip, at: 4)                        # fixed at 4 mm
//! merge(forearm, src.finger)                     # across a fixed joint only
//! drop(src.cable)
//! collision(src.upper, box(size: (144, 24, 10), at: pose(at: (60, 0, 5))))
//! collisions("hull")                             # every other link's: its convex hull (the default)
//! inertial(forearm, mass: 0.42, mass_was: 0.391)
//! joint(src.elbow, limit: (-90deg, 90deg), limit_was: (-135deg, 135deg), effort: eff, velocity: 1.5)
//! cut(src.j3)
//! tool0 = frame(on: forearm, at: pose(at: (144, 0, 5)))
//! transmission(src.shoulder, actuator: "m1", reduction: 50)
//! ```
//! Units: lengths mm, angles as written (`30deg`; a bare number is radians), mass kg, inertia
//! kg mm² (ixx, ixy, ixz, iyy, iyz, izz). A mesh file is read as STL in mm, beside the document.
//! `*_was` is the source's value an override replaced, which the editor records; when the source
//! no longer has it, the export is refused with the conflict (`UrdfError::Conflicts`).

use crate::syntax::{self, Arg, BinOp, Expr, ExprKind, StmtKind, Unit};
use crate::urdf::Options;
use forge_math::{Frame, Point3, Vec3};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What a statement names: `src.X` (an instance or joint of the source), or a name the robot
/// gave (a link renamed, a frame, a URDF joint by its name as a string).
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Source(String),
    Name(String),
}

impl Target {
    fn text(&self) -> String {
        match self {
            Target::Source(x) => format!("src.{x}"),
            Target::Name(x) => x.clone(),
        }
    }
}

/// A geometry a link's visual or collision is replaced by (mm, in the link frame).
#[derive(Clone, Debug, PartialEq)]
pub enum GeoSpec {
    Box {
        size: [f64; 3],
        at: Frame,
    },
    Cylinder {
        r: f64,
        len: f64,
        at: Frame,
    },
    Sphere {
        r: f64,
        at: Frame,
    },
    /// No URDF type: a cylinder and two spheres.
    Capsule {
        r: f64,
        len: f64,
        at: Frame,
    },
    Hull,
    /// A mesh file (`bytes` None: referenced, not read), its scale to metres (mm: 0.001).
    Mesh {
        file: String,
        bytes: Option<Vec<u8>>,
        at: Frame,
        scale: [f64; 3],
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoOv {
    pub line: usize,
    pub link: Target,
    pub collision: bool,
    pub spec: GeoSpec,
    /// A visual's material: its name and colour (0..1 each).
    pub material: Option<(String, [f64; 3])>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InertialOv {
    pub line: usize,
    pub link: Target,
    /// kg, mm, kg mm² (ixx ixy ixz iyy iyz izz).
    pub mass: Option<f64>,
    pub com: Option<[f64; 3]>,
    pub inertia: Option<[f64; 6]>,
    pub mass_was: Option<f64>,
    pub com_was: Option<[f64; 3]>,
    pub inertia_was: Option<[f64; 6]>,
}

impl Default for Target {
    fn default() -> Target {
        Target::Name(String::new())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JointOv {
    pub line: usize,
    pub joint: Target,
    pub kind: Option<String>,
    pub axis: Option<[f64; 3]>,
    pub axis_was: Option<[f64; 3]>,
    /// mm or rad, as the joint's kind.
    pub limit: Option<(f64, f64)>,
    pub limit_was: Option<(f64, f64)>,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    pub damping: Option<f64>,
    pub friction: Option<f64>,
}

/// A link an unlinked robot defines: its name, and its inertial (kg, mm, kg mm²) when it has
/// one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkDef {
    pub line: usize,
    pub name: String,
    pub mass: Option<f64>,
    pub com: [f64; 3],
    pub inertia: [f64; 6],
}

/// A joint an unlinked robot defines (`NAME = joint(parent: L, child: L, type: .., at: pose(..))`):
/// origin in the parent's frame (mm), limits mm or rad, a mimic's leader, multiplier (the
/// document's units) and offset.
#[derive(Clone, Debug, PartialEq)]
pub struct JointDef {
    pub line: usize,
    pub name: String,
    pub parent: Target,
    pub child: Target,
    pub kind: String,
    pub at: Frame,
    pub axis: Option<[f64; 3]>,
    pub limit: Option<(f64, f64)>,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    pub damping: Option<f64>,
    pub friction: Option<f64>,
    pub mimic: Option<(Target, f64, f64)>,
}

/// A robot document, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Robot {
    /// The source file as written, and where it is; None: unlinked (`source(none)`).
    pub source: Option<String>,
    pub source_path: Option<PathBuf>,
    pub root: Option<String>,
    pub cut: Vec<String>,
    pub freeze: Vec<(String, Vec<f64>)>,
    pub renames: Vec<(Target, String)>,
    pub merges: Vec<(Target, Vec<Target>)>,
    pub drops: Vec<Target>,
    pub geometry: Vec<GeoOv>,
    pub inertials: Vec<InertialOv>,
    pub joints: Vec<JointOv>,
    pub frames: Vec<(String, Target, Frame)>,
    pub transmissions: Vec<(Target, String, f64, Option<String>)>,
    /// Links with no collision statement collide as their convex hull (`collisions("hull")`,
    /// the default) or as their visual mesh (`collisions("mesh")`).
    pub hull_collisions: bool,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
    /// An unlinked robot's own links and joints, in document order.
    pub link_defs: Vec<LinkDef>,
    pub joint_defs: Vec<JointDef>,
}

impl Default for Robot {
    fn default() -> Robot {
        Robot {
            source: None,
            source_path: None,
            root: None,
            cut: vec![],
            freeze: vec![],
            renames: vec![],
            merges: vec![],
            drops: vec![],
            geometry: vec![],
            inertials: vec![],
            joints: vec![],
            frames: vec![],
            transmissions: vec![],
            hull_collisions: true,
            effort: None,
            velocity: None,
            link_defs: vec![],
            joint_defs: vec![],
        }
    }
}

impl Robot {
    /// What shapes the tree itself, into the export's options.
    pub fn shape(&self, o: &mut Options) {
        if self.root.is_some() {
            o.root = self.root.clone();
        }
        for c in &self.cut {
            if !o.cut.contains(c) {
                o.cut.push(c.clone());
            }
        }
        o.freeze.extend(self.freeze.iter().cloned());
        if self.effort.is_some() {
            o.effort = self.effort;
        }
        if self.velocity.is_some() {
            o.velocity = self.velocity;
        }
    }
}

/// A statement that could not be read: its line, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct RobotError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for RobotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Whether a source is a robot document.
pub fn is_robot(src: &str) -> bool {
    match syntax::parse(src) {
        Ok(p) => matches!(
            p.first().map(|s| &s.kind),
            Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "robot"
        ),
        Err(_) => false,
    }
}

#[derive(Clone, Debug)]
enum V {
    Num(f64),
    Str(String),
    Tuple(Vec<V>),
    /// The source (`source(..)`'s binding).
    Src,
    Target(Target),
    Frame(Frame),
    Geo(GeoSpec),
    None,
}

fn line_of(src: &str, at: usize) -> usize {
    src[..at.min(src.len())].matches('\n').count() + 1
}

type Env = BTreeMap<String, V>;

fn num(v: &V, what: &str) -> Result<f64, String> {
    match v {
        V::Num(x) => Ok(*x),
        _ => Err(format!("{what} takes a number")),
    }
}

fn nums<const N: usize>(v: &V, what: &str) -> Result<[f64; N], String> {
    match v {
        V::Tuple(t) if t.len() == N => {
            let mut out = [0.0; N];
            for (o, x) in out.iter_mut().zip(t) {
                *o = num(x, what)?;
            }
            Ok(out)
        }
        _ => Err(format!("{what} takes {N} numbers in brackets")),
    }
}

fn target(v: &V, what: &str) -> Result<Target, String> {
    match v {
        V::Target(t) => Ok(t.clone()),
        V::Str(s) => Ok(Target::Name(s.clone())),
        _ => Err(format!(
            "{what} takes a link or joint: src.NAME, a name the robot gave, or \"URDF name\""
        )),
    }
}

fn source_name(v: &V, what: &str) -> Result<String, String> {
    match v {
        V::Target(Target::Source(x)) => Ok(x.clone()),
        _ => Err(format!("{what} takes a source joint or instance: src.NAME")),
    }
}

fn frame(v: Option<&V>) -> Result<Frame, String> {
    match v {
        None => Ok(Frame::WORLD),
        Some(V::Frame(f)) => Ok(*f),
        Some(_) => Err("at: takes a pose(..)".into()),
    }
}

/// Arguments by name, positional ones as `#0`, `#1`, ..
struct Args(BTreeMap<String, V>);

impl Args {
    fn get(&self, k: &str) -> Option<&V> {
        self.0.get(k)
    }
    fn need(&self, k: &str, call: &str) -> Result<&V, String> {
        self.0.get(k).ok_or_else(|| {
            format!(
                "{call} needs {}",
                if k.starts_with('#') {
                    "its first argument".to_string()
                } else {
                    format!("`{k}:`")
                }
            )
        })
    }
    fn only(&self, call: &str, allowed: &[&str]) -> Result<(), String> {
        for k in self.0.keys() {
            if !allowed.contains(&k.as_str()) {
                return Err(format!(
                    "{call} takes no `{}` (it takes {})",
                    k.trim_start_matches('#'),
                    allowed.join(", ")
                ));
            }
        }
        Ok(())
    }
    fn num(&self, k: &str) -> Result<Option<f64>, String> {
        self.get(k).map(|v| num(v, k)).transpose()
    }
    fn n3(&self, k: &str) -> Result<Option<[f64; 3]>, String> {
        self.get(k).map(|v| nums::<3>(v, k)).transpose()
    }
    fn pair(&self, k: &str) -> Result<Option<(f64, f64)>, String> {
        self.get(k)
            .map(|v| nums::<2>(v, k).map(|p| (p[0], p[1])))
            .transpose()
    }
}

struct Reader<'a> {
    dir: Option<&'a Path>,
}

impl Reader<'_> {
    fn eval(&self, e: &Expr, env: &Env) -> Result<V, String> {
        Ok(match &e.kind {
            ExprKind::Num(x, u) => V::Num(match u {
                Unit::Deg => x.to_radians(),
                Unit::In => x * 25.4,
                Unit::None | Unit::Mm => *x,
            }),
            ExprKind::Str(s) => V::Str(s.clone()),
            ExprKind::Name(n) if n == "hull" => V::Geo(GeoSpec::Hull),
            ExprKind::Name(n) if n == "none" => V::None,
            ExprKind::Name(n) => env.get(n).cloned().ok_or_else(|| format!("unknown name `{n}`"))?,
            ExprKind::Tuple(xs) | ExprKind::List(xs) => {
                V::Tuple(xs.iter().map(|x| self.eval(x, env)).collect::<Result<_, _>>()?)
            }
            ExprKind::Neg(x) => V::Num(-num(&self.eval(x, env)?, "-")?),
            ExprKind::Bin(op, a, b) => {
                let (a, b) = (
                    num(&self.eval(a, env)?, "arithmetic")?,
                    num(&self.eval(b, env)?, "arithmetic")?,
                );
                V::Num(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    _ => return Err("only + - * / in a robot document".into()),
                })
            }
            ExprKind::Attr(x, name) => match self.eval(x, env)? {
                V::Src => V::Target(Target::Source(name.clone())),
                _ => return Err(format!("`.{name}` of something that is not the source")),
            },
            ExprKind::Call(f, args) => {
                let ExprKind::Name(fname) = &f.kind else {
                    return Err("a call of something that is not a built-in".into());
                };
                let a = self.args(args, env)?;
                self.value(fname, &a)?
            }
            _ => return Err("an expression a robot document does not take".into()),
        })
    }

    fn args(&self, args: &[Arg], env: &Env) -> Result<Args, String> {
        let mut m = BTreeMap::new();
        let mut k = 0;
        for a in args {
            let v = self.eval(&a.value, env)?;
            let key = match &a.name {
                Some((n, _)) => n.clone(),
                None => {
                    k += 1;
                    format!("#{}", k - 1)
                }
            };
            if m.insert(key.clone(), v).is_some() {
                return Err(format!("`{key}` given twice"));
            }
        }
        Ok(Args(m))
    }

    /// The calls that make values: poses and geometries.
    fn value(&self, f: &str, a: &Args) -> Result<V, String> {
        Ok(match f {
            "pose" => {
                a.only("pose", &["at", "z", "x", "rz"])?;
                let o = a.n3("at")?.unwrap_or([0.0; 3]);
                let z = a.n3("z")?.unwrap_or([0.0, 0.0, 1.0]);
                let z = Vec3::new(z[0], z[1], z[2])
                    .normalize()
                    .ok_or("pose: z is (0, 0, 0)")?;
                let at = Point3::new(o[0], o[1], o[2]);
                let mut fr = match a.n3("x")? {
                    Some(x) => Frame::from_zx(at, z, Vec3::new(x[0], x[1], x[2])).ok_or("pose: x along z")?,
                    // world x, or where z is near it, world y
                    None if z.get().x.abs() < 0.9 => {
                        Frame::from_zx(at, z, Vec3::new(1.0, 0.0, 0.0)).ok_or("pose: no x")?
                    }
                    None => Frame::from_zx(at, z, Vec3::new(0.0, 1.0, 0.0)).ok_or("pose: no x")?,
                };
                if let Some(r) = a.num("rz")? {
                    fr = fr.rotated_about_z(r);
                }
                V::Frame(fr)
            }
            "box" => {
                a.only("box", &["size", "at"])?;
                let size = a.n3("size")?.ok_or("box needs `size: (x, y, z)`")?;
                V::Geo(GeoSpec::Box {
                    size,
                    at: frame(a.get("at"))?,
                })
            }
            "cylinder" | "capsule" => {
                a.only(f, &["r", "len", "at"])?;
                let r = a.num("r")?.ok_or(format!("{f} needs `r:`"))?;
                let len = a.num("len")?.ok_or(format!("{f} needs `len:` (along its z)"))?;
                let at = frame(a.get("at"))?;
                V::Geo(if f == "cylinder" {
                    GeoSpec::Cylinder { r, len, at }
                } else {
                    GeoSpec::Capsule { r, len, at }
                })
            }
            "sphere" => {
                a.only("sphere", &["r", "at"])?;
                let r = a.num("r")?.ok_or("sphere needs `r:`")?;
                V::Geo(GeoSpec::Sphere {
                    r,
                    at: frame(a.get("at"))?,
                })
            }
            "mesh" => {
                a.only("mesh", &["#0", "at", "scale"])?;
                let V::Str(file) = a.need("#0", "mesh")? else {
                    return Err("mesh(\"FILE.stl\")".into());
                };
                // a file beside the document is read and packaged; a package:// URI or a file
                // that is not there is written as it is
                let bytes = if file.contains("://") {
                    None
                } else {
                    let p = self.dir.map_or_else(|| PathBuf::from(file), |d| d.join(file));
                    std::fs::read(&p).ok()
                };
                let scale = match a.get("scale") {
                    None => [0.001; 3],
                    Some(V::Num(x)) => [*x; 3],
                    Some(v) => nums::<3>(v, "scale")?,
                };
                V::Geo(GeoSpec::Mesh {
                    file: file.clone(),
                    bytes,
                    at: frame(a.get("at"))?,
                    scale,
                })
            }
            other => {
                return Err(format!(
                    "`{other}` is not a robot document's (statements: source link frame root cut freeze merge drop visual collision collisions inertial joint transmission actuation; values: pose box cylinder sphere capsule hull mesh)"
                ));
            }
        })
    }
}

/// Reads a robot document: every statement, or every statement's error.
pub fn read(src: &str, dir: Option<&Path>) -> Result<Robot, Vec<RobotError>> {
    let prog = syntax::parse(src).map_err(|e| {
        vec![RobotError {
            line: 1,
            message: format!("{e:?}"),
        }]
    })?;
    if !is_robot(src) {
        return Err(vec![RobotError {
            line: 1,
            message: "a robot document starts with the bare name `robot`".into(),
        }]);
    }
    let rd = Reader { dir };
    let mut r = Robot::default();
    let mut env: Env = BTreeMap::new();
    let mut errors = vec![];
    for st in prog.iter().skip(1) {
        let line = line_of(src, st.span.lo as usize);
        let res: Result<(), String> = (|| {
            match &st.kind {
                StmtKind::Shared { name, value } => {
                    let v = rd.eval(value, &env)?;
                    env.insert(name.clone(), v);
                }
                StmtKind::Assign { name, value } => {
                    let v = match &value.kind {
                        ExprKind::Call(f, args) => match &f.kind {
                            ExprKind::Name(fname) => {
                                let a = rd.args(args, &env)?;
                                match fname.as_str() {
                                    "source" => {
                                        a.only("source", &["#0"])?;
                                        if r.source.is_some() || r.source_path.is_some() {
                                            return Err("one source per robot".into());
                                        }
                                        match a.need("#0", "source")? {
                                            V::Str(f) => {
                                                r.source = Some(f.clone());
                                                r.source_path =
                                                    Some(dir.map_or_else(|| PathBuf::from(f), |d| d.join(f)));
                                            }
                                            V::None => {}
                                            _ => return Err("source(\"FILE.fg\") or source(none)".into()),
                                        }
                                        V::Src
                                    }
                                    "link" if a.get("#0").is_none() => {
                                        a.only("link", &["name", "mass", "com", "inertia"])?;
                                        if r.source_path.is_some() {
                                            return Err("a linked robot's links are its source's (frame(..) adds one)".into());
                                        }
                                        let n = match a.get("name") {
                                            Some(V::Str(s)) => s.clone(),
                                            Some(_) => return Err("name: takes a string".into()),
                                            None => name.clone(),
                                        };
                                        r.link_defs.push(LinkDef {
                                            line,
                                            name: n.clone(),
                                            mass: a.num("mass")?,
                                            com: a.n3("com")?.unwrap_or([0.0; 3]),
                                            inertia: match a.get("inertia") {
                                                Some(v) => nums::<6>(v, "inertia")?,
                                                None => [0.0; 6],
                                            },
                                        });
                                        V::Target(Target::Name(n))
                                    }
                                    "joint" if a.get("parent").is_some() => {
                                        a.only(
                                            "joint",
                                            &[
                                                "name", "parent", "child", "type", "at", "axis", "limit",
                                                "effort", "velocity", "damping", "friction", "mimic",
                                            ],
                                        )?;
                                        if r.source_path.is_some() {
                                            return Err("a linked robot's joints are its source's (joint(src.J, ..) overrides one)".into());
                                        }
                                        let n = match a.get("name") {
                                            Some(V::Str(s)) => s.clone(),
                                            Some(_) => return Err("name: takes a string".into()),
                                            None => name.clone(),
                                        };
                                        let kind = match a.get("type") {
                                            Some(V::Str(s)) => s.clone(),
                                            _ => return Err("a joint needs type: \"revolute\" | \"continuous\" | \"prismatic\" | \"fixed\" | \"floating\" | \"planar\"".into()),
                                        };
                                        let mimic = match a.get("mimic") {
                                            None => None,
                                            Some(V::Tuple(t)) if t.len() == 3 => Some((
                                                target(&t[0], "mimic")?,
                                                num(&t[1], "mimic")?,
                                                num(&t[2], "mimic")?,
                                            )),
                                            Some(_) => {
                                                return Err("mimic: (JOINT, multiplier, offset)".into());
                                            }
                                        };
                                        r.joint_defs.push(JointDef {
                                            line,
                                            name: n.clone(),
                                            parent: target(a.need("parent", "joint")?, "parent:")?,
                                            child: target(a.need("child", "joint")?, "child:")?,
                                            kind,
                                            at: frame(a.get("at"))?,
                                            axis: a.n3("axis")?,
                                            limit: a.pair("limit")?,
                                            effort: a.num("effort")?,
                                            velocity: a.num("velocity")?,
                                            damping: a.num("damping")?,
                                            friction: a.num("friction")?,
                                            mimic,
                                        });
                                        V::Target(Target::Name(n))
                                    }
                                    "link" => {
                                        a.only("link", &["#0", "name"])?;
                                        let t = target(a.need("#0", "link")?, "link")?;
                                        let n = match a.get("name") {
                                            Some(V::Str(s)) => s.clone(),
                                            Some(_) => return Err("name: takes a string".into()),
                                            None => name.clone(),
                                        };
                                        r.renames.push((t, n.clone()));
                                        V::Target(Target::Name(n))
                                    }
                                    "frame" => {
                                        a.only("frame", &["on", "at"])?;
                                        let on = target(a.need("on", "frame")?, "on:")?;
                                        r.frames.push((name.clone(), on, frame(a.get("at"))?));
                                        V::Target(Target::Name(name.clone()))
                                    }
                                    _ => rd.value(fname, &a)?,
                                }
                            }
                            _ => return Err("a call of a non-built-in".into()),
                        },
                        _ => rd.eval(value, &env)?,
                    };
                    env.insert(name.clone(), v);
                }
                StmtKind::Expr(e) => {
                    let ExprKind::Call(f, args) = &e.kind else {
                        return Err("a statement a robot document does not take".into());
                    };
                    let ExprKind::Name(fname) = &f.kind else {
                        return Err("a call of a non-built-in".into());
                    };
                    let a = rd.args(args, &env)?;
                    statement(&mut r, fname, &a, line)?;
                }
                _ => {
                    return Err(
                        "a statement a robot document does not take (shared, NAME = call(..), call(..))"
                            .into(),
                    );
                }
            }
            Ok(())
        })();
        if let Err(message) = res {
            errors.push(RobotError { line, message });
        }
    }
    if errors.is_empty() { Ok(r) } else { Err(errors) }
}

fn statement(r: &mut Robot, f: &str, a: &Args, line: usize) -> Result<(), String> {
    match f {
        "root" => {
            a.only("root", &["#0"])?;
            r.root = Some(source_name(a.need("#0", "root")?, "root")?);
        }
        "cut" => {
            a.only("cut", &["#0"])?;
            r.cut.push(source_name(a.need("#0", "cut")?, "cut")?);
        }
        "freeze" => {
            a.only("freeze", &["#0", "at"])?;
            let j = source_name(a.need("#0", "freeze")?, "freeze")?;
            let at = match a.need("at", "freeze")? {
                V::Num(x) => vec![*x],
                V::Tuple(t) => t.iter().map(|x| num(x, "at")).collect::<Result<_, _>>()?,
                _ => {
                    return Err("freeze(J, at: v) or at: (v1, v2, ..) in the joint's freedoms' order".into());
                }
            };
            r.freeze.push((j, at));
        }
        "merge" => {
            let mut ts = vec![];
            let mut k = 0;
            while let Some(v) = a.get(&format!("#{k}")) {
                ts.push(target(v, "merge")?);
                k += 1;
            }
            if ts.len() < 2 || a.0.len() != ts.len() {
                return Err("merge(LINK, LINK, ..): two links or more".into());
            }
            let into = ts.remove(0);
            r.merges.push((into, ts));
        }
        "drop" => {
            a.only("drop", &["#0"])?;
            r.drops.push(target(a.need("#0", "drop")?, "drop")?);
        }
        "visual" | "collision" => {
            a.only(f, &["#0", "#1", "material", "colour"])?;
            let material = match (a.get("material"), a.n3("colour")?) {
                (None, None) => None,
                (Some(V::Str(n)), c) => Some((n.clone(), c.unwrap_or([0.5; 3]))),
                (None, Some(c)) => Some(("colour".into(), c)),
                _ => return Err("material: takes a string, colour: (r, g, b) from 0 to 1".into()),
            };
            let link = target(a.need("#0", f)?, f)?;
            let V::Geo(spec) = a.need("#1", f)? else {
                return Err(format!(
                    "{f}(LINK, box(..) | cylinder(..) | sphere(..) | capsule(..) | hull | mesh(..))"
                ));
            };
            r.geometry.push(GeoOv {
                line,
                link,
                collision: f == "collision",
                spec: spec.clone(),
                material,
            });
        }
        "collisions" => {
            a.only("collisions", &["#0"])?;
            r.hull_collisions = match a.need("#0", "collisions")? {
                V::Str(s) if s == "hull" => true,
                V::Str(s) if s == "mesh" => false,
                V::Geo(GeoSpec::Hull) => true,
                _ => return Err("collisions(\"hull\") or collisions(\"mesh\")".into()),
            };
        }
        "actuation" => {
            a.only("actuation", &["effort", "velocity"])?;
            r.effort = a.num("effort")?;
            r.velocity = a.num("velocity")?;
        }
        "inertial" => {
            a.only(
                "inertial",
                &[
                    "#0",
                    "mass",
                    "com",
                    "inertia",
                    "mass_was",
                    "com_was",
                    "inertia_was",
                ],
            )?;
            let six = |k: &str| a.get(k).map(|v| nums::<6>(v, k)).transpose();
            r.inertials.push(InertialOv {
                line,
                link: target(a.need("#0", "inertial")?, "inertial")?,
                mass: a.num("mass")?,
                com: a.n3("com")?,
                inertia: six("inertia")?,
                mass_was: a.num("mass_was")?,
                com_was: a.n3("com_was")?,
                inertia_was: six("inertia_was")?,
            });
        }
        "joint" => {
            a.only(
                "joint",
                &[
                    "#0",
                    "type",
                    "axis",
                    "axis_was",
                    "limit",
                    "limit_was",
                    "effort",
                    "velocity",
                    "damping",
                    "friction",
                ],
            )?;
            let kind = match a.get("type") {
                Some(V::Str(s)) => Some(s.clone()),
                Some(_) => return Err("type: takes a string".into()),
                None => None,
            };
            r.joints.push(JointOv {
                line,
                joint: target(a.need("#0", "joint")?, "joint")?,
                kind,
                axis: a.n3("axis")?,
                axis_was: a.n3("axis_was")?,
                limit: a.pair("limit")?,
                limit_was: a.pair("limit_was")?,
                effort: a.num("effort")?,
                velocity: a.num("velocity")?,
                damping: a.num("damping")?,
                friction: a.num("friction")?,
            });
        }
        "transmission" => {
            a.only("transmission", &["#0", "actuator", "reduction", "interface"])?;
            let t = target(a.need("#0", "transmission")?, "transmission")?;
            let V::Str(act) = a.need("actuator", "transmission")? else {
                return Err("actuator: takes a string".into());
            };
            let red = a.num("reduction")?.unwrap_or(1.0);
            let iface = match a.get("interface") {
                Some(V::Str(s)) => Some(s.clone()),
                Some(_) => return Err("interface: takes a string".into()),
                None => None,
            };
            r.transmissions.push((t, act.clone(), red, iface));
        }
        other => {
            return Err(format!(
                "`{other}` is not a robot document's statement (root cut freeze merge drop visual collision collisions inertial joint transmission actuation)"
            ));
        }
    }
    let _ = Target::text;
    Ok(())
}

// ---- edits: what the editor sends ------------------------------------------------------------

/// A change to a robot document's text, as the editor makes it: a field set on an override
/// statement, or taken off it (back to the source's value). Statements are found by line, or by
/// their call and first argument as written (`joint`, `src.elbow`); a statement missing is
/// added at the end.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// `field: text` on the statement (replacing the field's value if it has one); with `was`,
    /// `field_was: was` too.
    Set {
        line: Option<usize>,
        call: String,
        target: String,
        field: String,
        text: String,
        was: Option<String>,
    },
    /// `field` (and `field_was`) off the statement; a statement left with only its target goes.
    Reset { line: usize, field: String },
    /// A statement added at the end.
    Add { text: String },
    /// The statement on this line taken out.
    Remove { line: usize },
}

/// The document's statements as calls: (statement span, call name, its arguments).
fn calls(src: &str) -> Result<Vec<(crate::syntax::Stmt, String, Vec<Arg>)>, String> {
    let prog = syntax::parse(src).map_err(|e| format!("{e:?}"))?;
    Ok(prog
        .into_iter()
        .filter_map(|st| {
            let (f, args) = match &st.kind {
                StmtKind::Expr(Expr {
                    kind: ExprKind::Call(f, args),
                    ..
                }) => (f, args.clone()),
                _ => return None,
            };
            let ExprKind::Name(n) = &f.kind else { return None };
            let n = n.clone();
            Some((st, n, args))
        })
        .collect())
}

/// The text of the statement's line range: from its line's start to its end.
fn line_span(src: &str, lo: usize) -> (usize, usize) {
    let start = src[..lo].rfind('\n').map_or(0, |i| i + 1);
    let end = src[lo..].find('\n').map_or(src.len(), |i| lo + i + 1);
    (start, end)
}

/// Applies an edit to a robot document's text: the new text, read back to check it.
pub fn edit(src: &str, e: &Edit) -> Result<String, String> {
    let cs = calls(src)?;
    let on_line = |line: usize| {
        cs.iter()
            .find(|c| line_of(src, c.0.span.lo as usize) == line)
            .ok_or_else(|| format!("no statement on line {line}"))
    };
    let out = match e {
        Edit::Add { text } => {
            let mut s = src.to_string();
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(text.trim_end());
            s.push('\n');
            s
        }
        Edit::Remove { line } => {
            let c = on_line(*line)?;
            let (a, b) = line_span(src, c.0.span.lo as usize);
            format!("{}{}", &src[..a], &src[b..])
        }
        Edit::Set {
            line,
            call,
            target,
            field,
            text,
            was,
        } => {
            let found = match line {
                Some(l) => Some(on_line(*l)?),
                None => cs.iter().find(|c| {
                    c.1 == *call
                        && c.2.first().is_some_and(|a| {
                            a.name.is_none()
                                && src[a.value.span.lo as usize..a.value.span.hi as usize].trim() == target
                        })
                }),
            };
            let mut fields = vec![(field.clone(), text.clone())];
            if let Some(w) = was {
                fields.push((format!("{field}_was"), w.clone()));
            }
            match found {
                None => {
                    let args: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                    let mut s = src.to_string();
                    if !s.ends_with('\n') {
                        s.push('\n');
                    }
                    s.push_str(&format!("{call}({target}, {})\n", args.join(", ")));
                    s
                }
                Some((st, _, args)) => {
                    // edits from the end backwards, so earlier offsets hold
                    let mut changes: Vec<(usize, usize, String)> = vec![];
                    let close = src[..st.span.hi as usize].rfind(')').ok_or("a call with no )")?;
                    let mut add = String::new();
                    for (k, v) in &fields {
                        match args.iter().find(|a| a.name.as_ref().is_some_and(|n| n.0 == *k)) {
                            Some(a) => {
                                changes.push((a.value.span.lo as usize, a.value.span.hi as usize, v.clone()))
                            }
                            None => add.push_str(&format!(", {k}: {v}")),
                        }
                    }
                    if !add.is_empty() {
                        changes.push((close, close, add));
                    }
                    changes.sort_by_key(|c| std::cmp::Reverse(c.0));
                    let mut s = src.to_string();
                    for (a, b, t) in changes {
                        s.replace_range(a..b, &t);
                    }
                    s
                }
            }
        }
        Edit::Reset { line, field } => {
            let (st, _, args) = on_line(*line)?;
            let was = format!("{field}_was");
            let gone: Vec<&Arg> = args
                .iter()
                .filter(|a| a.name.as_ref().is_some_and(|n| n.0 == *field || n.0 == was))
                .collect();
            if gone.is_empty() {
                return Err(format!("line {line} sets no {field}"));
            }
            let named_left = args.iter().filter(|a| a.name.is_some()).count() - gone.len();
            let positional = args.iter().filter(|a| a.name.is_none()).count();
            if named_left == 0 && positional <= 1 {
                // nothing left to override: the statement goes
                let (a, b) = line_span(src, st.span.lo as usize);
                format!("{}{}", &src[..a], &src[b..])
            } else {
                let mut s = src.to_string();
                let mut spans: Vec<(usize, usize)> = gone
                    .iter()
                    .map(|a| {
                        let lo = a.name.as_ref().map_or(a.value.span.lo, |n| n.1.lo) as usize;
                        // with the comma and spaces before it
                        let before = src[..lo].trim_end();
                        let lo = if before.ends_with(',') {
                            before.len() - 1
                        } else {
                            lo
                        };
                        (lo, a.value.span.hi as usize)
                    })
                    .collect();
                spans.sort_by_key(|x| std::cmp::Reverse(x.0));
                for (a, b) in spans {
                    s.replace_range(a..b, "");
                }
                s
            }
        }
    };
    syntax::parse(&out).map_err(|e| format!("the edit leaves the document unreadable: {e:?}"))?;
    Ok(out)
}
