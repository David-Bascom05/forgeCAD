//! Weldments in Forge, on forge-weld: structural members along the segments of a layout, each a
//! body of its own, meeting at nodes by one rule (forge_weld::frame), with end trims, end caps,
//! corner plates and a cut list.
//!
//! `members(LAYOUT, section: "SHS 40x40x3" | LOOP, ..)` makes one kernel feature per segment
//! (`F[k]`, a member body: an extrusion along a line, a revolve along an arc, cut on planes at
//! its ends). Everything is exact: lines, arcs and planes, the cut list from the section's
//! support function.

use crate::builtins::{
    ANG, Args, LEN, dim, not_in_sketch, num_of, plane_of, point_of, profile_of, role_set, word,
};
use crate::error::{Error, Op, Role};
use crate::eval::{Eval, Feat, R, Value};
use crate::sketch::Sk;
use crate::syntax::{Arg, Key, Span};
use crate::values::{Dim, Ty};
use forge_agent::json::Json;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_model::weld::{Member, MemberPath, Trim, TrimTool, cut_id};
use forge_model::{BodyFeature, Cutter, Feature, Keep, Pick};
use forge_ops::{Axis2, roles};
use forge_sketch::{Entity, EntityId};
use forge_topo::Name;
use forge_weld::{Corner, Cut, End, FrameError, MemberSpec, Path, Section, SectionError};
use std::cell::RefCell;
use std::rc::Rc;

pub const BUILTINS: &[&str] = &["members", "end_trim", "end_cap", "corner_plate"];

/// One member as made: its feature, path, section and every cut on it.
#[derive(Clone, Debug)]
pub struct MemberRec {
    pub id: String,
    pub idx: usize,
    pub hid: u64,
    pub spec: MemberSpec,
    /// The cuts at its start and its end: the frame's, then end trims' planes.
    pub cuts: [Vec<Cut>; 2],
    /// How many of each end's cuts the member feature itself makes (the rest are end trims).
    pub own: [usize; 2],
    /// End trims against bodies or envelopes (copes): the target's id, by end.
    pub copes: [Vec<String>; 2],
}

/// A `members(..)` feature: its members, in layout order.
#[derive(Clone, Debug)]
pub struct WeldFrame {
    pub id: String,
    pub members: Vec<MemberRec>,
    pub nodes: Vec<forge_weld::Node>,
}

/// A plate (an end cap or a corner plate).
#[derive(Clone, Debug)]
pub struct Plate {
    pub id: String,
    pub kind: &'static str,
    pub t: f64,
    pub area: f64,
    /// The member(s) it sits on.
    pub on: Vec<String>,
}

/// The part's weldments (members, plates) as evaluated.
#[derive(Clone, Debug, Default)]
pub struct Welds {
    pub frames: Vec<WeldFrame>,
    pub plates: Vec<Plate>,
}

impl Welds {
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty() && self.plates.is_empty()
    }
    fn member(&self, idx: usize) -> Option<(usize, usize)> {
        self.frames
            .iter()
            .enumerate()
            .find_map(|(f, fr)| fr.members.iter().position(|m| m.idx == idx).map(|k| (f, k)))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WeldErr {
    /// The frame rule refused: which member, why.
    Frame {
        span: Span,
        id: String,
        error: FrameError,
        names: Vec<String>,
    },
    Section {
        span: Span,
        id: String,
        error: SectionError,
    },
    /// Two members at a node overlap once trimmed (an arc member curving into another).
    Overlap {
        span: Span,
        a: String,
        b: String,
        volume: f64,
    },
    /// Not a member of a `members(..)` feature.
    NotAMember { span: Span },
    /// An end cap on an end cut on the slant.
    CapOnSlantEnd { span: Span, member: String },
    /// A corner plate on two members that share no node, or run in line there.
    NoCorner { span: Span, a: String, b: String },
}

fn weld_err(e: WeldErr) -> Error {
    Error::Weld(Box::new(e))
}

impl WeldErr {
    pub fn span(&self) -> Option<Span> {
        match self {
            WeldErr::Frame { span, .. }
            | WeldErr::Section { span, .. }
            | WeldErr::Overlap { span, .. }
            | WeldErr::NotAMember { span }
            | WeldErr::CapOnSlantEnd { span, .. }
            | WeldErr::NoCorner { span, .. } => Some(*span),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            WeldErr::Frame { error, .. } => match error {
                FrameError::Empty { .. } => "MemberEmpty",
                FrameError::NotSquare { .. } => "MemberNotSquare",
                FrameError::ButtParallel { .. } => "ButtParallel",
                FrameError::MiterTooSharp { .. } => "MiterTooSharp",
                FrameError::Crossing { .. } => "CrossingMembers",
                FrameError::CutAlong { .. } => "CutAlongMember",
                FrameError::CutAway { .. } => "MemberCutAway",
                FrameError::ArcTooTight { .. } => "ArcTooTight",
                FrameError::NoSuchMember { .. } => "NoSuchMember",
            },
            WeldErr::Section { error, .. } => match error {
                SectionError::UnknownSize(_) => "UnknownSize",
                SectionError::BadSize(_) => "BadSection",
                SectionError::Curve => "SectionCurve",
            },
            WeldErr::Overlap { .. } => "MembersOverlap",
            WeldErr::NotAMember { .. } => "NotAMember",
            WeldErr::CapOnSlantEnd { .. } => "CapOnSlantEnd",
            WeldErr::NoCorner { .. } => "NoCorner",
        }
    }

    pub fn fixes(&self) -> Vec<Op> {
        match self {
            WeldErr::Section {
                id,
                error: SectionError::UnknownSize(d),
                ..
            } => nearest(d)
                .into_iter()
                .map(|n| Op::SetArg {
                    id: id.clone(),
                    arg: "section".into(),
                    text: format!("\"{n}\""),
                })
                .collect(),
            WeldErr::Frame {
                id,
                error: FrameError::MiterTooSharp { .. } | FrameError::ButtParallel { .. },
                ..
            } => vec![Op::SetArg {
                id: id.clone(),
                arg: "corner".into(),
                text: if matches!(
                    self,
                    WeldErr::Frame {
                        error: FrameError::MiterTooSharp { .. },
                        ..
                    }
                ) {
                    "\"butt\"".into()
                } else {
                    "\"miter\"".into()
                },
            }],
            _ => vec![],
        }
    }

    pub fn text(&self) -> String {
        let nm = |names: &[String], k: usize| names.get(k).cloned().unwrap_or_else(|| format!("member {k}"));
        match self {
            WeldErr::Frame { error, names, .. } => match *error {
                FrameError::Empty { member } => format!("{} has no length", nm(names, member)),
                FrameError::NotSquare { member } => {
                    format!(
                        "{}: its sketch's normal is not square to its path",
                        nm(names, member)
                    )
                }
                FrameError::ButtParallel { member, to } => format!(
                    "{} cannot be butted to {}: they run in line, no face of one faces the other",
                    nm(names, member),
                    nm(names, to)
                ),
                FrameError::MiterTooSharp { member, to } => format!(
                    "{} and {} fold back onto each other: no miter plane between them",
                    nm(names, member),
                    nm(names, to)
                ),
                FrameError::Crossing { member, to } => format!(
                    "{} and {} both pass through one node: neither ends there to be trimmed",
                    nm(names, member),
                    nm(names, to)
                ),
                FrameError::CutAlong { member, end } => format!(
                    "{}: the cut at its {} runs along it or faces back into it",
                    nm(names, member),
                    if end == End::Start { "start" } else { "end" }
                ),
                FrameError::CutAway { member } => {
                    format!("{}: the cuts at its two ends meet inside it", nm(names, member))
                }
                FrameError::ArcTooTight { member } => format!(
                    "{}: its arc is no larger than its section, or an end cut lies more than a quarter turn round it",
                    nm(names, member)
                ),
                FrameError::NoSuchMember { index } => {
                    format!("rank names member {index}, which the layout has not")
                }
            },
            WeldErr::Section { error, .. } => match error {
                SectionError::UnknownSize(d) => {
                    let near = nearest(d);
                    if near.is_empty() {
                        format!("no section {d} in the library")
                    } else {
                        format!("no section {d} in the library; nearest: {}", near.join(", "))
                    }
                }
                SectionError::BadSize(why) => format!("the section cannot be made: {why}"),
                SectionError::Curve => {
                    "a member's section must be lines and arcs (a spline or ellipse is not exact as a sweep)"
                        .into()
                }
            },
            WeldErr::Overlap { a, b, volume, .. } => format!(
                "{a} and {b} overlap by {} mm3 once trimmed",
                crate::values::fmt((volume * 1000.0).round() / 1000.0)
            ),
            WeldErr::NotAMember { .. } => "not a member of a members(..) feature".into(),
            WeldErr::CapOnSlantEnd { member, .. } => {
                format!("{member}'s end is cut on the slant: an end cap needs a square end")
            }
            WeldErr::NoCorner { a, b, .. } => {
                format!("{a} and {b} share no node where they meet at an angle")
            }
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        match self {
            WeldErr::Frame { error, names, .. } => {
                let one = |k: usize| -> Json { names.get(k).map_or(Json::Null, |n| n.as_str().into()) };
                match *error {
                    FrameError::Empty { member }
                    | FrameError::NotSquare { member }
                    | FrameError::CutAway { member }
                    | FrameError::ArcTooTight { member } => vec![("member", one(member))],
                    FrameError::CutAlong { member, end } => vec![
                        ("member", one(member)),
                        ("end", if end == End::Start { "start" } else { "end" }.into()),
                    ],
                    FrameError::ButtParallel { member, to }
                    | FrameError::MiterTooSharp { member, to }
                    | FrameError::Crossing { member, to } => vec![("member", one(member)), ("to", one(to))],
                    FrameError::NoSuchMember { index } => vec![("index", (index as f64).into())],
                }
            }
            WeldErr::Section { error, .. } => match error {
                SectionError::UnknownSize(d) => vec![
                    ("designation", d.as_str().into()),
                    (
                        "nearest",
                        Json::Arr(nearest(d).into_iter().map(|n| n.as_str().into()).collect()),
                    ),
                ],
                SectionError::BadSize(why) => vec![("why", (*why).into())],
                SectionError::Curve => vec![],
            },
            WeldErr::Overlap { a, b, volume, .. } => vec![
                ("a", a.as_str().into()),
                ("b", b.as_str().into()),
                ("volume", (*volume).into()),
            ],
            WeldErr::NotAMember { .. } => vec![],
            WeldErr::CapOnSlantEnd { member, .. } => vec![("member", member.as_str().into())],
            WeldErr::NoCorner { a, b, .. } => vec![("a", a.as_str().into()), ("b", b.as_str().into())],
        }
    }
}

/// The library's designations nearest an unknown one: its family's, by how far their sizes are.
fn nearest(d: &str) -> Vec<String> {
    let fam: String = d
        .trim()
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .collect::<String>()
        .trim()
        .to_ascii_uppercase();
    let sizes = |s: &str| -> Vec<f64> {
        let i = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
        s[i..]
            .split(['x', 'X'])
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .collect()
    };
    let mut want = sizes(d);
    // the short forms: SHS a x t and L a x t are a x a x t
    if want.len() == 2 && (fam == "SHS" || fam == "L") {
        want.insert(0, want[0]);
    }
    let mut all: Vec<(f64, String)> = forge_weld::designations()
        .into_iter()
        .filter(|n| n.split(' ').next() == Some(fam.as_str()))
        .map(|n| {
            let s = sizes(&n);
            let far: f64 = s.iter().zip(&want).map(|(a, b)| (a - b).abs()).sum::<f64>()
                + 1000.0 * (s.len() as f64 - want.len() as f64).abs();
            (far, n)
        })
        .collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    all.into_iter().take(3).map(|x| x.1).collect()
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    match name {
        "members" => members(ev, args, span),
        "end_trim" => end_trim(ev, args, span),
        "end_cap" => end_cap(ev, args, span),
        _ => corner_plate(ev, args, span),
    }
}

// ---- the layout ----------------------------------------------------------------------------

/// A layout entity as a member path and its sketch's normal.
fn path_of_ent(sk: &Sk, id: EntityId) -> Option<(Path, UnitVec3)> {
    let up = sk.frame.z();
    match sk.s.entity(id) {
        Some(Entity::Line { a, b }) => Some((
            Path::Line {
                a: sk.world(sk.pos(a)),
                b: sk.world(sk.pos(b)),
            },
            up,
        )),
        Some(Entity::Arc {
            center,
            start,
            end,
            ccw,
        }) => {
            let (c, a, b) = (sk.pos(center), sk.pos(start), sk.pos(end));
            let (a0, a1) = (
                forge_math::dmath::atan2(a.y - c.y, a.x - c.x),
                forge_math::dmath::atan2(b.y - c.y, b.x - c.x),
            );
            let (axis, sweep) = if ccw {
                (up, forge_math::dmath::wrap_tau(a1 - a0))
            } else {
                (-up, forge_math::dmath::wrap_tau(a0 - a1))
            };
            let sweep = if sweep <= 0.0 {
                forge_math::dmath::TAU
            } else {
                sweep
            };
            Some((
                Path::Arc {
                    center: sk.world(c),
                    axis,
                    start: sk.world(a),
                    sweep,
                },
                up,
            ))
        }
        _ => None,
    }
}

fn layout(v: Value, span: Span) -> R<Vec<(Path, UnitVec3)>> {
    let mut ents: Vec<(Rc<RefCell<Sk>>, EntityId)> = vec![];
    crate::shapes::path_ents(v, span, &mut ents)?;
    if ents.is_empty() {
        return Err(Error::EmptySelection { span });
    }
    ents.iter()
        .map(|(sk, id)| {
            path_of_ent(&sk.borrow(), *id).ok_or(Error::WrongEntity {
                span,
                want: crate::error::EntKind::Line,
                found: crate::error::EntKind::Spline,
            })
        })
        .collect()
}

// ---- sections ------------------------------------------------------------------------------

/// The section an argument names: a designation from the library, or a sketch's region (its
/// own coordinates, the origin its pierce point).
fn section_of(ev: &Eval, a: &Args, id: &str) -> R<Section> {
    let span = a.span("section");
    let serr = |error| {
        weld_err(WeldErr::Section {
            span,
            id: id.into(),
            error,
        })
    };
    match a.req("section")? {
        Value::Str(s) => forge_weld::lookup(&s).map_err(serr),
        v @ (Value::Sketch(_) | Value::Ent(_)) => {
            let (sk, p, _) = profile_of(ev, v, span)?;
            let name = format!("sketch {}", sk.borrow().id);
            let loops = p
                .loops
                .iter()
                .map(|l| l.segments.iter().map(|s| s.0.clone()).collect())
                .collect();
            Section::sketched(name, loops).map_err(serr)
        }
        o => Err(Error::WrongType {
            span,
            what: "a section (a designation, as \"SHS 40x40x3\", or a sketch loop)",
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

const ALIGN: &[&str] = &[
    "center",
    "centroid",
    "origin",
    "top",
    "bottom",
    "left",
    "right",
    "top_left",
    "top_right",
    "bottom_left",
    "bottom_right",
];

/// The section placed in the member's frame: its pierce point (by `align`, moved by `offset`)
/// on the path, turned by `turn`, mirrored.
fn placed(s: &Section, a: &Args, sketched: bool, dims: &mut Vec<crate::eval::Dimension>) -> R<Section> {
    let align = word(a, "align", ALIGN)?.unwrap_or(if sketched { "origin" } else { "center" });
    let (lo, hi) = s.bbox();
    let mid = Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let p = match align {
        "origin" => Point2::ORIGIN,
        "centroid" => s.centroid(),
        "center" => mid,
        w => {
            let x = if w.ends_with("left") || w == "left" {
                lo.x
            } else if w.ends_with("right") || w == "right" {
                hi.x
            } else {
                mid.x
            };
            let y = if w.starts_with("top") {
                hi.y
            } else if w.starts_with("bottom") {
                lo.y
            } else {
                mid.y
            };
            Point2::new(x, y)
        }
    };
    let off = match a.opt("offset") {
        None => (0.0, 0.0),
        Some(Value::Tuple(t)) if t.len() == 2 => {
            let x = num_of(t[0].clone(), LEN, a.span("offset"), "offset")?;
            let y = num_of(t[1].clone(), LEN, a.span("offset"), "offset")?;
            dims.push(dim("offset x", &x));
            dims.push(dim("offset y", &y));
            (x.v, y.v)
        }
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("offset"),
                what: "offset: (dx, dy)",
                want: Ty::Vec2,
                found: o.ty(),
            });
        }
    };
    let turn = match a.num("turn", ANG)? {
        Some(n) => {
            dims.push(dim("turn", &n));
            n.v
        }
        None => 0.0,
    };
    let mirror = a.flag("mirror")?;
    // the section moves by the offset: the path runs through the pierce point less it
    let origin = Point2::new(p.x - off.0, p.y - off.1);
    Ok(s.placed(origin, turn, mirror))
}

fn corner_word(v: &Value, span: Span, gap: f64) -> R<Corner> {
    match v {
        Value::Str(s) if &**s == "miter" => Ok(Corner::Miter { gap }),
        Value::Str(s) if &**s == "butt" => Ok(Corner::Butt { gap }),
        Value::Str(_) => Err(Error::BadValue {
            span,
            what: "corner (\"miter\" or \"butt\")",
            value: f64::NAN,
        }),
        o => Err(Error::WrongType {
            span,
            what: "a corner (\"miter\" or \"butt\")",
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

fn gap_num(v: Value, span: Span, what: &'static str) -> R<f64> {
    let g = num_of(v, LEN, span, what)?;
    if g.v < 0.0 || !g.v.is_finite() {
        return Err(Error::BadValue {
            span,
            what,
            value: g.v,
        });
    }
    Ok(g.v)
}

// ---- members ---------------------------------------------------------------------------------

fn frame_at(spec: &MemberSpec) -> Frame {
    let (x, y, z) = spec.axes(0.0);
    Frame::from_axes(spec.path.point(0.0), x, y, z).expect("a member's frame")
}

fn cut_frame(c: &Cut) -> Frame {
    Frame::from_z(c.point, c.normal)
}

/// The kernel member for a spec and its cuts.
fn kernel_member(spec: &MemberSpec, cuts: &[Vec<Cut>; 2], reach: [f64; 2], outline: bool) -> Member {
    let plane = frame_at(spec);
    let sec = if outline {
        spec.section.outline()
    } else {
        spec.section.clone()
    };
    let path = match spec.path {
        Path::Line { a, b } => MemberPath::Line {
            length: a.distance(b),
        },
        Path::Arc {
            center, axis, sweep, ..
        } => {
            let c = plane.point_to_local(center);
            let d = plane.vec_to_local(axis.into());
            MemberPath::Arc {
                axis: Axis2 {
                    origin: Point2::new(c.x, c.y),
                    dir: forge_math::Vec2::new(d.x, d.y),
                },
                sweep,
            }
        }
    };
    Member {
        profile: sec.profile(plane, 1),
        path,
        reach,
        cuts: cuts.iter().flatten().map(cut_frame).collect(),
    }
}

fn feat_value(id: String, idx: usize, hid: u64) -> Value {
    Value::Feature(Rc::new(Feat {
        id,
        kind: "member",
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    }))
}

fn members(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "members", span)?;
    let id = ev.take_id("members", span)?;
    let a = Args::bind(
        ev,
        "members",
        &[
            "layout", "section", "align", "offset", "turn", "mirror", "corner", "gap", "corners", "rank",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let paths = layout(a.req("layout")?, a.span("layout"))?;
    let section = section_of(ev, &a, &id)?;
    let mut dims = vec![];
    let sketched = section.kind == forge_weld::Kind::Sketched;
    let sec = placed(&section, &a, sketched, &mut dims)?;
    let gap = match a.opt("gap") {
        Some(v) => {
            let g = num_of(v.clone(), LEN, a.span("gap"), "gap")?;
            dims.push(dim("gap", &g));
            gap_num(v, a.span("gap"), "gap")?
        }
        None => 0.0,
    };
    let corner = match a.opt("corner") {
        Some(v) => corner_word(&v, a.span("corner"), gap)?,
        None => Corner::Miter { gap },
    };
    let mut corners = vec![];
    if let Some(v) = a.opt("corners") {
        let sp = a.span("corners");
        let Value::List(xs) = v else {
            return Err(Error::WrongType {
                span: sp,
                what: "corners: [(POINT, \"miter\"|\"butt\", gap), ..]",
                want: Ty::List,
                found: v.ty(),
            });
        };
        for x in xs.iter() {
            match x {
                Value::Tuple(t) if t.len() == 2 || t.len() == 3 => {
                    let p = point_of(ev, t[0].clone(), sp)?;
                    let g = match t.get(2) {
                        Some(g) => gap_num(g.clone(), sp, "a corner's gap")?,
                        None => gap,
                    };
                    corners.push((p, corner_word(&t[1], sp, g)?));
                }
                o => {
                    return Err(Error::WrongType {
                        span: sp,
                        what: "a corner: (POINT, \"miter\"|\"butt\") or (POINT, word, gap)",
                        want: Ty::Tuple,
                        found: o.ty(),
                    });
                }
            }
        }
    }
    let mut rank = vec![];
    if let Some(v) = a.opt("rank") {
        let sp = a.span("rank");
        let Value::List(xs) = v else {
            return Err(Error::WrongType {
                span: sp,
                what: "rank: [k, ..] (members by their index)",
                want: Ty::List,
                found: v.ty(),
            });
        };
        for x in xs.iter() {
            let n = num_of(x.clone(), &[Dim::Count, Dim::Length], sp, "rank")?;
            if !n.is_whole() || n.v < 0.0 {
                return Err(Error::NotWhole { span: sp, value: n.v });
            }
            rank.push(n.v as usize);
        }
    }
    let specs: Vec<MemberSpec> = paths
        .into_iter()
        .map(|(path, up)| MemberSpec {
            path,
            up,
            section: sec.clone(),
        })
        .collect();
    let names: Vec<String> = (0..specs.len()).map(|k| format!("{id}[{k}]")).collect();
    let ferr = |error| {
        weld_err(WeldErr::Frame {
            span,
            id: id.clone(),
            error,
            names: names.clone(),
        })
    };
    let plan = forge_weld::plan(&specs, corner, &corners, &rank).map_err(ferr)?;
    let mut recs = Vec::with_capacity(specs.len());
    let mut out = Vec::with_capacity(specs.len());
    for (k, spec) in specs.into_iter().enumerate() {
        let cuts = plan.cuts[k].clone();
        let reach = forge_weld::reach(&spec, &cuts, k).map_err(ferr)?;
        let m = kernel_member(&spec, &cuts, reach, false);
        let mid = names[k].clone();
        let (idx, hid) = ev.push_feature(&mid, "member", Feature::Member(Box::new(m)), span, dims.clone())?;
        out.push((Key::Ix(k as i64), feat_value(mid.clone(), idx, hid)));
        recs.push(MemberRec {
            id: mid,
            idx,
            hid,
            own: [cuts[0].len(), cuts[1].len()],
            spec,
            cuts,
            copes: [vec![], vec![]],
        });
    }
    overlaps(ev, &recs, &plan.nodes, span)?;
    ev.welds.frames.push(WeldFrame {
        id,
        members: recs,
        nodes: plan.nodes,
    });
    Ok(Value::Map(Rc::new(out)))
}

/// Members meeting at a node with an arc among them have no plane between them by construction:
/// each such pair is intersected, and any overlap refused.
fn overlaps(ev: &Eval, recs: &[MemberRec], nodes: &[forge_weld::Node], span: Span) -> R<()> {
    if !ev.b.building {
        return Ok(());
    }
    let Some(st) = ev.b.stages.last() else {
        return Ok(());
    };
    let body = |idx: usize| st.solids.iter().find(|s| s.id == idx).map(|s| &s.body);
    for n in nodes {
        for (i, &(a, _)) in n.members.iter().enumerate() {
            for &(b, _) in &n.members[i + 1..] {
                let arc = |k: usize| matches!(recs[k].spec.path, Path::Arc { .. });
                if !(arc(a) || arc(b)) {
                    continue;
                }
                let (Some(x), Some(y)) = (body(recs[a].idx), body(recs[b].idx)) else {
                    continue;
                };
                let both = forge_bool::boolean(x, y, forge_bool::Op::Intersect, 0x5745_4c44_0000 + i as u64);
                let volume = match both {
                    Ok(v) if v.face_ids().next().is_some() => {
                        forge_topo::mass::mass(&v).map_or(0.0, |m| m.volume)
                    }
                    _ => 0.0,
                };
                if volume > 1e-6 {
                    return Err(weld_err(WeldErr::Overlap {
                        span,
                        a: recs[a].id.clone(),
                        b: recs[b].id.clone(),
                        volume,
                    }));
                }
            }
        }
    }
    Ok(())
}

/// The member a value names: (frame, member) in the part's welds.
fn member_of(ev: &Eval, v: &Value, span: Span) -> R<(usize, usize)> {
    match v {
        Value::Feature(f) if f.kind == "member" => ev
            .welds
            .member(f.idx)
            .ok_or(weld_err(WeldErr::NotAMember { span })),
        _ => Err(weld_err(WeldErr::NotAMember { span })),
    }
}

fn members_of(ev: &Eval, v: &Value, span: Span) -> R<Vec<(usize, usize)>> {
    match v {
        Value::List(xs) => xs.iter().map(|x| member_of(ev, x, span)).collect(),
        Value::Map(m) => m.iter().map(|x| member_of(ev, &x.1, span)).collect(),
        o => Ok(vec![member_of(ev, o, span)?]),
    }
}

/// A member's role anchors: `start`, `end` (its end faces: the cuts', or the section's caps).
pub fn member(ev: &mut Eval, f: &Feat, name: &str) -> Option<Value> {
    if f.kind != "member" || !matches!(name, "start" | "end") {
        return None;
    }
    let (fi, k) = ev.welds.member(f.idx)?;
    let m = &ev.welds.frames[fi].members[k];
    let root = Name::feature(f.hid);
    let s = usize::from(name == "end");
    let first = if s == 0 { 0 } else { m.own[0] };
    let mut names = vec![if s == 0 {
        roles::start(root)
    } else {
        roles::end(root)
    }];
    names.extend((first..first + m.own[s]).map(|c| roles::start(Name::feature(cut_id(f.hid, c)))));
    let role = if s == 0 { Role::Start } else { Role::End };
    Some(role_set(ev, f, role, &names, None))
}

// ---- end trims -----------------------------------------------------------------------------

/// The end of a member nearer a point.
fn nearer_end(spec: &MemberSpec, p: Point3) -> usize {
    let [a, b] = spec.path.ends();
    usize::from(b.distance(p) < a.distance(p))
}

fn end_word(a: &Args) -> R<Option<usize>> {
    Ok(word(a, "end", &["start", "end"])?.map(|w| usize::from(w == "end")))
}

fn end_trim(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "end_trim", span)?;
    let id = ev.take_id("end_trim", span)?;
    let a = Args::bind(ev, "end_trim", &["members", "to", "gap", "end"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let which = members_of(ev, &a.req("members")?, a.span("members"))?;
    let to = a.req("to")?;
    let to_span = a.span("to");
    let end = end_word(&a)?;
    let mut dims = vec![];
    let gap = match a.opt("gap") {
        Some(v) => {
            dims.push(dim("gap", &num_of(v.clone(), LEN, a.span("gap"), "gap")?));
            Some(gap_num(v, a.span("gap"), "gap")?)
        }
        None => None,
    };
    // a member, or another body: a cope (the member's envelope, or the body, taken away at one
    // end); else a plane
    let target_member = match &to {
        Value::Feature(f) if f.kind == "member" => Some(member_of(ev, &to, to_span)?),
        _ => None,
    };
    let target_body = match &to {
        Value::Feature(f) if f.kind != "member" => Some((f.idx, f.id.clone())),
        _ => None,
    };
    if (target_member.is_some() || target_body.is_some()) && gap.is_some() {
        return Err(Error::NotWith {
            span: a.span("gap"),
            func: "end_trim",
            arg: "gap",
            with: "a body to trim to",
        });
    }
    let mut last = None;
    for (n, &(fi, k)) in which.iter().enumerate() {
        let rec = ev.welds.frames[fi].members[k].clone();
        let fid = if which.len() == 1 {
            id.clone()
        } else {
            format!("{id}[{n}]")
        };
        if let Some((tf, tk)) = target_member {
            let t = ev.welds.frames[tf].members[tk].clone();
            let near = t.spec.path.point(t.spec.path.end_t() / 2.0);
            let s = end.unwrap_or_else(|| nearer_to_path(&rec.spec, &t.spec.path));
            let _ = near;
            // the target's envelope, run on well past its ends, uncut
            let run = 2.0 * (rec.spec.section.radius() + t.spec.section.radius());
            let reach = match t.spec.path {
                Path::Line { .. } => [run, run],
                Path::Arc { .. } => [0.0, 0.0],
            };
            let env = kernel_member(&t.spec, &[vec![], vec![]], reach, true);
            let trim = Trim {
                member: rec.idx,
                region: region(&rec.spec, s),
                tools: vec![TrimTool::Envelope(Box::new(env))],
            };
            last = Some(push_trim(ev, &fid, trim, rec.idx, span, dims.clone())?);
            ev.welds.frames[fi].members[k].copes[s].push(t.id.clone());
        } else if let Some((bidx, bid)) = &target_body {
            let s = match end {
                Some(s) => s,
                None => {
                    let c = body_centre(ev, *bidx).unwrap_or(rec.spec.path.point(0.0));
                    nearer_end(&rec.spec, c)
                }
            };
            let trim = Trim {
                member: rec.idx,
                region: region(&rec.spec, s),
                tools: vec![TrimTool::Body(*bidx)],
            };
            last = Some(push_trim(ev, &fid, trim, rec.idx, span, dims.clone())?);
            ev.welds.frames[fi].members[k].copes[s].push(bid.clone());
        } else {
            let plane = plane_of(ev, to.clone(), to_span, "to")?;
            let s = end.unwrap_or_else(|| {
                let [p, q] = rec.spec.path.ends();
                let d = |x: Point3| (x - plane.origin).dot(plane.z().into()).abs();
                usize::from(d(q) < d(p))
            });
            // the cut keeps the side the member's other end lies on, gap short of the plane
            let other = rec.spec.path.ends()[1 - s];
            let z: Vec3 = plane.z().into();
            let side = (other - plane.origin).dot(z);
            let normal = if side < 0.0 { plane.z() } else { -plane.z() };
            let g = gap.unwrap_or(0.0);
            let cut = Cut {
                point: plane.origin - Vec3::from(normal) * g,
                normal,
            };
            let keep = if side < 0.0 { Keep::Minus } else { Keep::Plus };
            let mut cuts = rec.cuts.clone();
            cuts[s].push(cut);
            forge_weld::reach(&rec.spec, &cuts, k).map_err(|error| {
                weld_err(WeldErr::Frame {
                    span,
                    id: fid.clone(),
                    error,
                    names: vec![rec.id.clone()],
                })
            })?;
            let f = Feature::Body(Box::new(BodyFeature::In {
                bodies: Pick::Some(vec![rec.idx]),
                feature: Feature::Body(Box::new(BodyFeature::Split {
                    by: Cutter::Plane(Frame::from_z(cut.point, plane.z())),
                    keep,
                })),
            }));
            let (idx, hid) = ev.push_feature(&fid, "end_trim", f, span, dims.clone())?;
            last = Some(feat_plain(fid.clone(), "end_trim", idx, hid));
            ev.welds.frames[fi].members[k].cuts[s].push(cut);
        }
    }
    Ok(last.unwrap_or(Value::Unresolved))
}

/// Which end of a member is nearer another path (its nearest end to any point of it).
fn nearer_to_path(spec: &MemberSpec, other: &Path) -> usize {
    let dist = |p: Point3| -> f64 {
        match *other {
            Path::Line { a, b } => {
                let d = b - a;
                let t = ((p - a).dot(d) / d.dot(d)).clamp(0.0, 1.0);
                (a + d * t).distance(p)
            }
            Path::Arc {
                center, axis, start, ..
            } => {
                let r = center.distance(start);
                let v = p - center;
                let along = Vec3::from(axis) * v.dot(axis.into());
                let inplane = v - along;
                let l = inplane.length();
                let on = if l > 0.0 {
                    inplane * (r / l)
                } else {
                    start - center
                };
                (center + on).distance(p)
            }
        }
    };
    let [a, b] = spec.path.ends();
    usize::from(dist(b) < dist(a))
}

/// The plane through a member's middle, z towards end `s`: an end trim reaches no further.
fn region(spec: &MemberSpec, s: usize) -> Frame {
    let t = spec.path.end_t() / 2.0;
    let z = spec.path.tangent(t);
    let z = if s == 0 { -z } else { z };
    Frame::from_z(spec.path.point(t), z.normalize().expect("a tangent"))
}

fn body_centre(ev: &Eval, idx: usize) -> Option<Point3> {
    let st = ev.b.stages.last()?;
    let b = &st.solids.iter().find(|s| s.id == idx)?.body;
    let bx = b
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |acc, f| acc.hull(&b.face_box(f)));
    Some(Point3::new(bx.x.mid(), bx.y.mid(), bx.z.mid()))
}

fn push_trim(
    ev: &mut Eval,
    fid: &str,
    trim: Trim,
    _member: usize,
    span: Span,
    dims: Vec<crate::eval::Dimension>,
) -> R<Value> {
    let f = Feature::Body(Box::new(BodyFeature::Trim(trim)));
    let (idx, hid) = ev.push_feature(fid, "end_trim", f, span, dims)?;
    Ok(feat_plain(fid.into(), "end_trim", idx, hid))
}

fn feat_plain(id: String, kind: &'static str, idx: usize, hid: u64) -> Value {
    Value::Feature(Rc::new(Feat {
        id,
        kind,
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    }))
}

// ---- plates --------------------------------------------------------------------------------

fn plate_profile(frame: Frame, pts: &[Point2]) -> forge_ops::Profile {
    let n = pts.len();
    forge_ops::Profile {
        plane: frame,
        loops: vec![forge_ops::ProfileLoop::new(
            (0..n)
                .map(|i| {
                    (
                        forge_ops::Segment::Line {
                            a: pts[i],
                            b: pts[(i + 1) % n],
                        },
                        i as u64 + 1,
                    )
                })
                .collect(),
        )],
    }
}

fn thickness(a: &Args, dims: &mut Vec<crate::eval::Dimension>) -> R<f64> {
    let t = a.req_num("t", LEN)?;
    dims.push(dim("t", &t));
    crate::builtins::positive(&t, a.span("t"), "t")
}

fn end_cap(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "end_cap", span)?;
    let id = ev.take_id("end_cap", span)?;
    let a = Args::bind(ev, "end_cap", &["member", "end", "t"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (fi, k) = member_of(ev, &a.req("member")?, a.span("member"))?;
    let s = end_word(&a)?.unwrap_or(1);
    let mut dims = vec![];
    let t = thickness(&a, &mut dims)?;
    let rec = ev.welds.frames[fi].members[k].clone();
    let et = if s == 0 { 0.0 } else { rec.spec.path.end_t() };
    let (x, y, z) = rec.spec.axes(et);
    // a square end: every cut there square to the axis (the path end, or a cut's plane)
    let mut at = rec.spec.path.point(et);
    for c in &rec.cuts[s] {
        let n: Vec3 = c.normal.into();
        if n.cross(z).length() > 1e-9 {
            return Err(weld_err(WeldErr::CapOnSlantEnd {
                span,
                member: rec.id.clone(),
            }));
        }
        // where the plane crosses the path
        let along = (c.point - at).dot(z);
        at = at + z * along;
    }
    if !rec.copes[s].is_empty() || matches!(rec.spec.path, Path::Arc { .. }) && !rec.cuts[s].is_empty() {
        return Err(weld_err(WeldErr::CapOnSlantEnd {
            span,
            member: rec.id.clone(),
        }));
    }
    // outward: along the path at its end, against it at its start
    let out = if s == 0 { -z } else { z };
    let xx = if s == 0 { -x } else { x };
    let frame = Frame::from_axes(at, xx, y, out).expect("a cap's frame");
    // the outline in the cap's frame: mirrored in x at the start, where x runs the other way
    let sec = rec.spec.section.outline();
    let sec = if s == 0 {
        sec.placed(Point2::ORIGIN, 0.0, true)
    } else {
        sec
    };
    let profile = sec.profile(frame, 1);
    let f = Feature::Extrude {
        profile,
        extent: forge_model::Extent::Distance(t),
        op: forge_model::Combine::New,
    };
    let (idx, hid) = ev.push_feature(&id, "end_cap", f, span, dims)?;
    ev.welds.plates.push(Plate {
        id: id.clone(),
        kind: "end_cap",
        t,
        area: sec.area(),
        on: vec![rec.id.clone()],
    });
    Ok(feat_plain(id, "end_cap", idx, hid))
}

fn corner_plate(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "corner_plate", span)?;
    let id = ev.take_id("corner_plate", span)?;
    let a = Args::bind(
        ev,
        "corner_plate",
        &["a", "b", "size", "t", "chamfer"],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (fa, ka) = member_of(ev, &a.req("a")?, a.span("a"))?;
    let (fb, kb) = member_of(ev, &a.req("b")?, a.span("b"))?;
    let mut dims = vec![];
    let size = a.req_num("size", LEN)?;
    dims.push(dim("size", &size));
    let d = crate::builtins::positive(&size, a.span("size"), "size")?;
    let t = thickness(&a, &mut dims)?;
    let c = match a.num("chamfer", LEN)? {
        Some(n) => {
            dims.push(dim("chamfer", &n));
            if n.v < 0.0 || n.v >= d {
                return Err(Error::BadValue {
                    span: a.span("chamfer"),
                    what: "chamfer (under the size)",
                    value: n.v,
                });
            }
            n.v
        }
        None => 0.0,
    };
    let ra = ev.welds.frames[fa].members[ka].clone();
    let rb = ev.welds.frames[fb].members[kb].clone();
    let no = || {
        weld_err(WeldErr::NoCorner {
            span,
            a: ra.id.clone(),
            b: rb.id.clone(),
        })
    };
    // the node they share, and each one's direction away from it there
    let at = |r: &MemberRec, p: Point3| -> Option<(f64, Vec3)> {
        let [s, e] = r.spec.path.ends();
        if s.distance(p) <= 1e-5 {
            Some((0.0, r.spec.path.tangent(0.0)))
        } else if e.distance(p) <= 1e-5 {
            let et = r.spec.path.end_t();
            Some((et, -r.spec.path.tangent(et)))
        } else {
            None
        }
    };
    let mut found = None;
    for p in ra.spec.path.ends() {
        if let (Some(x), Some(y)) = (at(&ra, p), at(&rb, p)) {
            found = Some((p, x, y));
            break;
        }
    }
    let (p, (ta, ua), (tb, ub)) = found.ok_or_else(no)?;
    let n = ua.cross(ub).normalize().ok_or_else(no)?;
    let nv: Vec3 = n.into();
    // each one's inside face: its section's reach towards the other, in their plane
    let va = (ub - ua * ub.dot(ua)).normalize().ok_or_else(no)?;
    let vb = (ua - ub * ua.dot(ub)).normalize().ok_or_else(no)?;
    let ha = ra.spec.support(ta, va.into());
    let hb = rb.spec.support(tb, vb.into());
    // the corner where the two inside lines meet: p + va ha + s ua = p + vb hb + r ub
    let rhs = Vec3::from(vb) * hb - Vec3::from(va) * ha;
    // solve s ua - r ub = rhs in the plane (2 x 2 by the plane's axes)
    let e1 = ua;
    let e2 = nv.cross(ua);
    let (a11, a12, b1) = (ua.dot(e1), -ub.dot(e1), rhs.dot(e1));
    let (a21, a22, b2) = (ua.dot(e2), -ub.dot(e2), rhs.dot(e2));
    let det = a11 * a22 - a12 * a21;
    if det.abs() < 1e-12 {
        return Err(no());
    }
    let sa = (b1 * a22 - a12 * b2) / det;
    let k = p + Vec3::from(va) * ha + ua * sa;
    let frame = Frame::from_axes(k - nv * (t / 2.0), e1, e2, nv).expect("a plate's frame");
    let loc = |q: Point3| {
        let l = frame.point_to_local(q);
        Point2::new(l.x, l.y)
    };
    let mut pts = vec![];
    if c > 0.0 {
        pts.push(loc(k + ua * c));
    } else {
        pts.push(loc(k));
    }
    pts.push(loc(k + ua * d));
    pts.push(loc(k + ub * d));
    if c > 0.0 {
        pts.push(loc(k + ub * c));
    }
    // counter-clockwise in the plate's frame
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (u, w) = (pts[i], pts[(i + 1) % pts.len()]);
            u.x * w.y - w.x * u.y
        })
        .sum::<f64>()
        / 2.0;
    if area < 0.0 {
        pts.reverse();
    }
    let f = Feature::Extrude {
        profile: plate_profile(frame, &pts),
        extent: forge_model::Extent::Distance(t),
        op: forge_model::Combine::New,
    };
    let (idx, hid) = ev.push_feature(&id, "corner_plate", f, span, dims)?;
    ev.welds.plates.push(Plate {
        id: id.clone(),
        kind: "corner_plate",
        t,
        area: area.abs(),
        on: vec![ra.id.clone(), rb.id.clone()],
    });
    Ok(feat_plain(id, "corner_plate", idx, hid))
}

// ---- the cut list --------------------------------------------------------------------------

/// One line of the cut list: identical members together.
#[derive(Clone, Debug)]
pub struct CutRow {
    pub section: String,
    pub kind: &'static str,
    pub members: Vec<String>,
    pub cut: forge_weld::Cutting,
    pub copes: [Vec<String>; 2],
}

/// An angle in degrees in [0, 360), a hair under 360 taken as 0.
fn turn360(t: f64) -> f64 {
    let a = t.rem_euclid(360.0);
    if a > 360.0 - 1e-6 { 0.0 } else { a }
}

fn key(c: &forge_weld::Cutting, section: &str, copes: &[Vec<String>; 2]) -> String {
    let r = |x: f64| format!("{:.6}", x);
    let end = |e: &[forge_weld::EndCut], cp: &[String]| -> String {
        let mut parts: Vec<String> = e
            .iter()
            .map(|x| {
                format!(
                    "{}@{}",
                    r(x.angle),
                    x.toward.map_or("-".into(), |t| r(turn360(t)))
                )
            })
            .collect();
        parts.sort();
        if !cp.is_empty() {
            parts.push("cope".into());
        }
        parts.join("+")
    };
    let mut ends = [end(&c.ends[0], &copes[0]), end(&c.ends[1], &copes[1])];
    ends.sort();
    format!(
        "{section}|{}|{}|{}|{}",
        r(c.length),
        c.overall.map_or("-".into(), r),
        c.arc.map_or("-".into(), |a| format!("{}/{}", r(a.0), r(a.1))),
        ends.join("|")
    )
}

/// The cut list of every member of the part, identical members on one row (by section, lengths
/// and end cuts, either way round), in the order each first appears.
pub fn cutlist(w: &Welds) -> Vec<CutRow> {
    let mut rows: Vec<(String, CutRow)> = vec![];
    for fr in &w.frames {
        for m in &fr.members {
            let c = forge_weld::cutting(&m.spec, &m.cuts);
            let k = key(&c, &m.spec.section.name, &m.copes);
            match rows.iter_mut().find(|r| r.0 == k) {
                Some(r) => r.1.members.push(m.id.clone()),
                None => rows.push((
                    k,
                    CutRow {
                        section: m.spec.section.name.clone(),
                        kind: m.spec.section.kind.word(),
                        members: vec![m.id.clone()],
                        cut: c,
                        copes: m.copes.clone(),
                    },
                )),
            }
        }
    }
    rows.into_iter().map(|r| r.1).collect()
}

fn round(x: f64, places: i32) -> Json {
    let p = 10f64.powi(places);
    ((x * p).round() / p).into()
}

impl CutRow {
    pub fn json(&self) -> Json {
        let ends = |s: usize| {
            Json::Arr(
                self.cut.ends[s]
                    .iter()
                    .map(|e| {
                        Json::obj(vec![
                            ("angle", round(e.angle, 6)),
                            ("toward", e.toward.map_or(Json::Null, |t| round(turn360(t), 6))),
                        ])
                    })
                    .collect(),
            )
        };
        let mut f = vec![
            ("section", self.section.as_str().into()),
            ("kind", self.kind.into()),
            ("qty", (self.members.len() as f64).into()),
            (
                "members",
                Json::Arr(self.members.iter().map(|m| m.as_str().into()).collect()),
            ),
            ("length", round(self.cut.length, 6)),
            ("overall", self.cut.overall.map_or(Json::Null, |o| round(o, 6))),
            ("start", ends(0)),
            ("end", ends(1)),
        ];
        if let Some((r, a)) = self.cut.arc {
            f.push(("radius", round(r, 6)));
            f.push(("arc", round(a, 6)));
        }
        for (s, k) in [(0, "start_cope"), (1, "end_cope")] {
            if !self.copes[s].is_empty() {
                f.push((
                    k,
                    Json::Arr(self.copes[s].iter().map(|c| c.as_str().into()).collect()),
                ));
            }
        }
        Json::obj(f)
    }

    pub fn text(&self) -> String {
        let ends = |s: usize| {
            if !self.copes[s].is_empty() {
                return format!("cope to {}", self.copes[s].join(", "));
            }
            let cuts: Vec<String> = self.cut.ends[s]
                .iter()
                .filter(|e| e.angle > 1e-9)
                .map(|e| format!("{} deg", crate::values::fmt((e.angle * 100.0).round() / 100.0)))
                .collect();
            if cuts.is_empty() {
                "square".into()
            } else {
                cuts.join(" + ")
            }
        };
        let len = crate::values::fmt((self.cut.length * 1000.0).round() / 1000.0);
        let all = self
            .cut
            .overall
            .map(|o| format!(", overall {}", crate::values::fmt((o * 1000.0).round() / 1000.0)))
            .unwrap_or_default();
        let arc = self
            .cut
            .arc
            .map(|(r, a)| {
                format!(
                    ", arc R{} {} deg",
                    crate::values::fmt((r * 1000.0).round() / 1000.0),
                    crate::values::fmt((a * 100.0).round() / 100.0)
                )
            })
            .unwrap_or_default();
        format!(
            "{} x {} ({}): length {len}{all}{arc}; start {}, end {}  [{}]",
            self.members.len(),
            self.section,
            self.kind,
            ends(0),
            ends(1),
            self.members.join(", ")
        )
    }
}

/// The cut list as JSON: rows, then plates.
pub fn json(w: &Welds) -> Json {
    Json::obj(vec![
        ("rows", Json::Arr(cutlist(w).iter().map(CutRow::json).collect())),
        (
            "plates",
            Json::Arr(
                w.plates
                    .iter()
                    .map(|p| {
                        Json::obj(vec![
                            ("id", p.id.as_str().into()),
                            ("kind", p.kind.into()),
                            ("t", round(p.t, 6)),
                            ("area", round(p.area, 6)),
                            ("on", Json::Arr(p.on.iter().map(|m| m.as_str().into()).collect())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}
