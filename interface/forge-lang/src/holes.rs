//! Holes and threads: every option the kernel's `Hole` and `ScrewThread` take, and their faces
//! by role (forge_model::hole::roles, thread::roles).

use crate::builtins::{
    ANG, Args, LEN, dim, ent_of, face_plane, not_in_sketch, num_of, plane_of, point_ent, point_of, positive,
};
use crate::error::{EntKind, Error, Role};
use crate::eval::{Eval, ExtentV, Faces, Feat, R, Value};
use crate::feat_error::{Because, FeatError};
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Ty};
use crate::wired::{conflict, feat, one_form, only_with, word};
use forge_math::{Frame, Point3};
use forge_model::pattern::instance_name;
use forge_model::{End, Feature, Head, Target, ThreadLength, ThreadOn, ThreadSpec};
use forge_ops::Handed;
use forge_topo::Name;
use std::rc::Rc;

const HOLE_ARGS: &[&str] = &[
    "at",
    "d",
    "depth",
    "cbore",
    "csink",
    "flip",
    "screw",
    "fit",
    "seat",
    "convention",
    "point",
    "thread",
    "on",
    "bodies",
];

pub fn hole(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "hole", span)?;
    let id = ev.take_id("hole", span)?;
    let a = Args::bind(ev, "hole", HOLE_ARGS, args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let func = "hole";
    conflict(&a, func, "cbore", "csink")?;
    conflict(&a, func, "d", "screw")?;
    conflict(&a, func, "screw", "thread")?;
    for k in ["cbore", "csink"] {
        conflict(&a, func, k, "seat")?;
    }
    let screw = a.opt("screw").is_some();
    for k in ["fit", "seat", "convention"] {
        only_with(&a, func, k, screw, Because::NoScrew)?;
    }
    // where: sketch points (one sketch), or points on a plane
    let pts: Vec<Value> = crate::wired::items(a.req("at")?);
    if pts.is_empty() {
        return Err(FeatError::NotOne {
            span: a.span("at"),
            what: "a hole's places",
            count: 0,
        }
        .into());
    }
    let sk = match &pts[0] {
        Value::Ent(e) => Some(e.sk.clone()),
        _ => None,
    };
    let mut at = vec![];
    let mut names = vec![];
    for p in &pts {
        match p {
            Value::Ent(e) => {
                let e = ent_of(Value::Ent(e.clone()), a.span("at"), EntKind::Point)?;
                if let Some(s) = &sk
                    && !Rc::ptr_eq(&e.sk, s)
                {
                    return Err(Error::OtherSketch {
                        span: a.span("at"),
                        sketch: e.sk.borrow().id.clone(),
                        here: s.borrow().id.clone(),
                    });
                }
                let id = point_ent(&e, a.span("at"))?;
                let s = e.sk.borrow();
                at.push(s.world(s.pos(id)));
                names.push(crate::work::point_name(&s, id).unwrap_or_default());
            }
            v => at.push(point_of(ev, v.clone(), a.span("at"))?),
        }
    }
    let plane = match (a.opt("on"), &sk) {
        (Some(v), _) => plane_of(ev, v, a.span("on"), "on")?,
        (None, Some(s)) => s.borrow().frame,
        (None, None) => {
            return Err(Error::MissingArg {
                span,
                func,
                arg: "on (the plane points worked out here are drilled from)",
            });
        }
    };
    // each place dropped onto the plane
    let z0 = plane.z().get();
    let at: Vec<Point3> = at.iter().map(|&p| p - z0 * (p - plane.origin).dot(z0)).collect();
    // the size: given, a standard screw's, with its thread
    let standard = match a.opt("screw") {
        Some(Value::Str(sz)) => {
            let screw = forge_model::standard::screw(&sz).ok_or_else(|| FeatError::UnknownSize {
                span: a.span("screw"),
                size: sz.to_string(),
            })?;
            let fit = match word(&a, "fit", &["close", "normal", "loose", "tap"])?.unwrap_or("normal") {
                "close" => forge_model::Fit::Close,
                "loose" => forge_model::Fit::Loose,
                "tap" => forge_model::Fit::Tap,
                _ => forge_model::Fit::Normal,
            };
            let convention = match word(&a, "convention", &["iso", "ansi"])? {
                Some("iso") => forge_model::Convention::Iso,
                Some(_) => forge_model::Convention::Ansi,
                None => screw.default_convention(),
            };
            Some(forge_model::Standard {
                screw,
                convention,
                fit,
            })
        }
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("screw"),
                what: "a screw size",
                want: Ty::Str,
                found: o.ty(),
            });
        }
        None => None,
    };
    let mut dims = vec![];
    let d = match &standard {
        Some(st) => st.diameter(),
        None => {
            let d = a.req_num("d", LEN)?;
            dims.push(dim("diameter", &d));
            positive(&d, a.span("d"), "a hole's diameter")?
        }
    };
    let end = match a.req("depth")? {
        Value::Extent(ExtentV::Thru | ExtentV::ThruAll) => End::Through,
        Value::Extent(ExtentV::Next) => End::ToNext,
        Value::Extent(x @ (ExtentV::UpTo(..) | ExtentV::UpToPlane(..))) => {
            let (target, offset) = target_of(ev, &x, a.span("depth"))?;
            End::To { target, offset }
        }
        Value::Extent(_) => {
            return Err(Error::WrongType {
                span: a.span("depth"),
                what: "a hole's depth (a length, thru, next or upto(..))",
                want: Ty::Num,
                found: Ty::Extent,
            });
        }
        v => {
            let n = num_of(v, LEN, a.span("depth"), "a hole's depth")?;
            positive(&n, a.span("depth"), "a hole's depth")?;
            dims.push(dim("depth", &n));
            End::Depth(n.v)
        }
    };
    only_with(&a, func, "point", end != End::Through, Because::Through)?;
    let point = match a.num("point", ANG)? {
        Some(p) => {
            dims.push(dim("point angle", &p));
            Some(p.v)
        }
        None => None,
    };
    let pair = |k: &'static str, second: &[Dim]| -> R<Option<(Num, Num)>> {
        match a.opt(k) {
            None => Ok(None),
            Some(Value::Tuple(t)) if t.len() == 2 => Ok(Some((
                num_of(t[0].clone(), LEN, a.span(k), k)?,
                num_of(t[1].clone(), second, a.span(k), k)?,
            ))),
            Some(o) => Err(Error::WrongType {
                span: a.span(k),
                what: k,
                want: Ty::Tuple,
                found: o.ty(),
            }),
        }
    };
    let head = match (pair("cbore", LEN)?, pair("csink", ANG)?) {
        (Some((dd, dp)), _) => {
            dims.push(dim("cbore diameter", &dd));
            dims.push(dim("cbore depth", &dp));
            Head::Counterbore {
                diameter: dd.v,
                depth: dp.v,
            }
        }
        (None, Some((dd, an))) => {
            dims.push(dim("csink diameter", &dd));
            dims.push(dim("csink angle", &an));
            Head::Countersink {
                diameter: dd.v,
                angle: an.v,
            }
        }
        _ => match (&standard, word(&a, "seat", &["cbore", "csink"])?) {
            (Some(st), Some("cbore")) => st.counterbore(),
            (Some(st), Some(_)) => st.countersink().ok_or(FeatError::NotApplicable {
                span: a.span("seat"),
                func,
                arg: "seat",
                because: Because::NoStandardSeat,
            })?,
            _ => Head::Plain,
        },
    };
    let thread = match a.opt("thread") {
        None => standard.and_then(|s| s.thread()),
        Some(v) => {
            let sp = spec_of(v, a.span("thread"), &mut dims)?;
            Some(forge_model::Thread {
                size: sp.major(),
                pitch: sp.pitch(),
            })
        }
    };
    let z = if a.flag("flip")? { plane.z() } else { -plane.z() };
    let hf = Frame::from_zx(at[0], z, plane.x().get()).expect("x not along z");
    let places = (at.len() > 1).then(|| forge_model::At::Sketch {
        feature: sk.as_ref().map_or(0, |s| s.borrow().idx),
        points: names.clone(),
        centers: false,
        at: at.clone(),
    });
    let h = forge_model::Hole {
        at: hf,
        diameter: d,
        head,
        end,
        point,
        thread,
        standard,
        places,
    };
    let f = crate::bodies::wrap_in(ev, &a, Feature::Hole(Box::new(h)))?;
    let (idx, hid) = ev.push_feature(&id, "hole", f, span, dims)?;
    Ok(feat(id, "hole", idx, hid, Some((at[0], z))))
}

/// An `upto(..)` end as a kernel target: a work plane, or a face by its owner's name.
pub fn target_of(ev: &mut Eval, x: &ExtentV, span: Span) -> R<(Target, f64)> {
    match x {
        ExtentV::UpToPlane(k, off) => Ok((Target::Plane(*k), *off)),
        ExtentV::UpTo(fs, off) => {
            let Some(&n) = fs.names.first() else {
                return Err(Error::NoFace {
                    span,
                    id: fs.owner.clone(),
                    role: fs.role,
                });
            };
            let r = crate::work::owner(ev, n, true, span)?;
            Ok((
                Target::Face {
                    feature: r.feature,
                    name: r.name.unwrap_or(n),
                },
                *off,
            ))
        }
        _ => unreachable!("an upto extent"),
    }
}

/// A thread size: a designation (`"M6"`, `"M6x0.75"`, `"1/4-20"`), or (major, pitch) lengths.
fn spec_of(v: Value, span: Span, dims: &mut Vec<crate::eval::Dimension>) -> R<ThreadSpec> {
    match v {
        Value::Str(s) => ThreadSpec::parse(&s).ok_or_else(|| {
            FeatError::UnknownSize {
                span,
                size: s.to_string(),
            }
            .into()
        }),
        Value::Tuple(t) if t.len() == 2 => {
            let major = num_of(t[0].clone(), LEN, span, "a thread's major diameter")?;
            let pitch = num_of(t[1].clone(), LEN, span, "a thread's pitch")?;
            positive(&major, span, "a thread's major diameter")?;
            positive(&pitch, span, "a thread's pitch")?;
            dims.push(dim("thread diameter", &major));
            dims.push(dim("thread pitch", &pitch));
            Ok(ThreadSpec::Iso {
                major: major.v,
                pitch: pitch.v,
            })
        }
        o => Err(Error::WrongType {
            span,
            what: "a thread size (\"M6\", or (major, pitch))",
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

const THREAD_FORMS: &[&str] = &["hole", "face", "on"];

pub fn thread(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "thread", span)?;
    let id = ev.take_id("thread", span)?;
    let a = Args::bind(
        ev,
        "thread",
        &[
            "hole", "face", "on", "spec", "at", "internal", "flip", "length", "left", "chamfer", "runout",
            "bodies",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let func = "thread";
    let form = one_form(&a, func, THREAD_FORMS, span)?;
    only_with(&a, func, "spec", form != "hole", Because::HoleGivesSize)?;
    only_with(&a, func, "flip", form != "hole", Because::OtherForm)?;
    for k in ["at", "internal"] {
        only_with(&a, func, k, form == "on", Because::OtherForm)?;
    }
    let mut dims = vec![];
    let spec = |ev: &mut Eval, dims: &mut Vec<_>| -> R<ThreadSpec> {
        let _ = ev;
        spec_of(a.req("spec")?, a.span("spec"), dims)
    };
    let on = match form {
        "hole" => match a.req("hole")? {
            Value::Feature(h) if h.kind == "hole" => ThreadOn::Hole(h.idx),
            o => {
                return Err(Error::WrongType {
                    span: a.span("hole"),
                    what: "a hole",
                    want: Ty::Feature,
                    found: o.ty(),
                });
            }
        },
        "face" => {
            let r = crate::work::to_ref(ev, a.req("face")?, a.span("face"))?;
            ThreadOn::Face {
                feature: r.feature,
                name: r.name.unwrap_or(Name::NONE),
                spec: spec(ev, &mut dims)?,
                flip: a.flag("flip")?,
            }
        }
        _ => {
            let plane = plane_of(ev, a.req("on")?, a.span("on"), "on")?;
            let o = match a.opt("at") {
                Some(v) => {
                    let p = point_of(ev, v, a.span("at"))?;
                    p - plane.z().get() * (p - plane.origin).dot(plane.z().get())
                }
                None => plane.origin,
            };
            let z = if a.flag("flip")? { plane.z() } else { -plane.z() };
            ThreadOn::Axis {
                at: Frame::from_zx(o, z, plane.x().get()).expect("x not along z"),
                spec: spec(ev, &mut dims)?,
                internal: a.flag("internal")?,
            }
        }
    };
    let length = match a.req("length")? {
        Value::Str(w) if &*w == "full" => ThreadLength::Full,
        Value::Extent(x @ (ExtentV::UpTo(..) | ExtentV::UpToPlane(..))) => {
            let (target, offset) = target_of(ev, &x, a.span("length"))?;
            ThreadLength::To { target, offset }
        }
        v => {
            let n = num_of(v, LEN, a.span("length"), "a thread's length")?;
            positive(&n, a.span("length"), "a thread's length")?;
            dims.push(dim("length", &n));
            ThreadLength::Distance(n.v)
        }
    };
    let t = forge_model::ScrewThread {
        on,
        length,
        handed: if a.flag("left")? {
            Handed::Left
        } else {
            Handed::Right
        },
        chamfer: a.flag("chamfer")?,
        runout: a.flag("runout")?,
    };
    // a drawing's thread: the envelope stays plain, the thread is recorded for its lines
    let kernel = match (ev.cosmetic_threads, t.on, t.length) {
        (true, ThreadOn::Axis { at, spec, internal }, ThreadLength::Distance(length)) => {
            let (major, pitch) = (spec.major(), spec.pitch());
            ev.threads.push(Cosmetic {
                at,
                major: major / 2.0,
                minor: major / 2.0 - 5.0 / 16.0 * 3f64.sqrt() * pitch,
                length,
                internal,
            });
            Feature::Plane(at)
        }
        _ => Feature::Thread(Box::new(t)),
    };
    let f = crate::bodies::wrap_in(ev, &a, kernel)?;
    let (idx, hid) = ev.push_feature(&id, "thread", f, span, dims)?;
    Ok(feat(id, "thread", idx, hid, None))
}

/// A thread a drawing draws by convention (ISO 6410, ASME Y14.6): from `at`'s origin along its
/// z for `length`, between the radii, outside a rod (`internal` false) or inside a bore.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cosmetic {
    pub at: forge_math::Frame,
    pub major: f64,
    pub minor: f64,
    pub length: f64,
    pub internal: bool,
}

/// The kernel feature a statement made, through any `bodies:` wrapper.
pub fn kernel_feature<'a>(ev: &'a Eval, idx: usize) -> Option<&'a Feature> {
    match ev.b.features.get(idx)? {
        Feature::Body(b) => match &**b {
            forge_model::BodyFeature::In { feature, .. } => Some(feature),
            _ => None,
        },
        f => Some(f),
    }
}

/// A hole's places other than the first: their keys in its copies' names.
fn copies(ev: &Eval, idx: usize) -> usize {
    match kernel_feature(ev, idx) {
        Some(Feature::Hole(h)) => match &h.places {
            Some(forge_model::At::Sketch { at, .. }) => at.len().saturating_sub(1),
            _ => 0,
        },
        _ => 0,
    }
}

/// Each name, and the same at each of the hole's (feature `idx`, id `hid`) other places.
fn at_places(ev: &Eval, idx: usize, hid: u64, names: &[Name]) -> Vec<Name> {
    let mut out = names.to_vec();
    for k in 1..=copies(ev, idx) {
        out.extend(names.iter().map(|&n| instance_name(hid, [k as u64, 0], n)));
    }
    out
}

/// The part's faces carrying these role names (whole, pieces, or joined); none is `NoFace`.
fn role_anchor(ev: &mut Eval, f: &Feat, role: Role, names: &[Name], span: Span) -> R<Value> {
    match role_faces(ev, f, role, names) {
        Value::Faces(fs) if fs.names.is_empty() => Err(Error::NoFace {
            span,
            id: f.id.clone(),
            role,
        }),
        v => Ok(v),
    }
}

/// The part's faces carrying these role names (whole, pieces, or joined).
fn role_faces(ev: &mut Eval, f: &Feat, role: Role, names: &[Name]) -> Value {
    if !ev.b.building || ev.b.built() <= f.idx {
        return Value::Unresolved;
    }
    let body = ev.b.body().expect("built");
    let mut faces = vec![];
    for &n in names {
        for x in crate::build::role_faces(body, n) {
            if !faces.contains(&x) {
                faces.push(x);
            }
        }
    }
    let plane = faces.first().and_then(|&x| face_plane(body, x));
    Value::Faces(Rc::new(Faces {
        names: faces.iter().map(|&x| body.face(x).name).collect(),
        owner: f.id.clone(),
        role,
        plane,
    }))
}

pub fn hole_member(ev: &mut Eval, f: &Rc<Feat>, name: &str, span: Span) -> Option<R<Value>> {
    use forge_model::hole::roles;
    let root = Name::feature(f.hid);
    let (role, own): (Role, [Name; 2]) = match name {
        "wall" => (Role::Wall, roles::wall(root)),
        "floor" => (Role::Floor, roles::bottom(root)),
        "bore" => (Role::Bore, roles::bore(root)),
        "seat" => (Role::Seat, roles::seat(root)),
        "sink" => (Role::Sink, roles::sink(root)),
        _ => return None,
    };
    let names = at_places(ev, f.idx, f.hid, &own);
    Some(role_anchor(ev, f, role, &names, span))
}

pub fn thread_member(ev: &mut Eval, f: &Rc<Feat>, name: &str, span: Span) -> Option<R<Value>> {
    use forge_model::thread::roles;
    let id = f.hid;
    let (role, own): (Role, Vec<Name>) = match name {
        "flanks" => (Role::Flanks, roles::flanks(id).to_vec()),
        "root" => (Role::RootFlat, vec![roles::root_flat(id), roles::runout_flat(id)]),
        "chamfer" => (Role::Chamfer, roles::chamfer(id).to_vec()),
        _ => return None,
    };
    // a hole's thread is cut at each of the hole's places
    let names = match kernel_feature(ev, f.idx) {
        Some(Feature::Thread(t)) => match t.on {
            ThreadOn::Hole(k) => {
                let hid = ev.b.ids[k];
                let mut out = own.clone();
                for c in 1..=copies(ev, k) {
                    out.extend(own.iter().map(|&n| instance_name(hid, [c as u64, 0], n)));
                }
                out
            }
            _ => own,
        },
        _ => own,
    };
    Some(role_anchor(ev, f, role, &names, span))
}
