//! Shape features: sweep, coil, loft, shell, draft, face offsets and slabs, ribs, face fillets,
//! full rounds and balls. Each checks its arguments, refuses an option that cannot apply as a
//! typed error, and exports anchors by role (`forge_ops::roles`).

use crate::builtins::{
    ANG, Args, LEN, carried, dim, dir_of, ent_of, face_plane, has_solid, not_in_sketch, num_of, point_of,
    positive, profile_of, role_set, word,
};
use crate::error::{EntKind, Error, Role};
use crate::eval::{Eval, Faces, Feat, R, Value};
use crate::sketch::Sk;
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Ty};
use forge_geom::Circle;
use forge_math::{Frame, Point2, Point3, dmath};
use forge_model::{Coil, CoilSize, Combine, Edit, Feature, Loft, PathEnds, Rib, Side, Sweep, Target};
use forge_ops::{Apex, Axis2, Handed, LoftGuides, LoftKind, LoftOptions, PathSeg, Profile, Segment, roles};
use forge_sketch::{Entity, EntityId};
use forge_topo::Name;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// The built-ins this module answers.
pub const NAMES: &[&str] = &[
    "sweep",
    "coil",
    "draft",
    "offset_faces",
    "delete_faces",
    "replace_face",
    "move_faces",
    "thicken",
    "rib",
    "facefillet",
    "fullround",
    "ball",
    "sphere",
    "dome",
];

/// Feature kinds that make solid (for the default of a new body versus adding).
pub const SOLID: &[&str] = &["sweep", "coil", "ball", "thicken"];

/// The ends a kernel profile's segment ids carry for pieces (a kinked curve, a split segment).
const PIECE: u64 = 0xffff << 48;
/// Where this module's own split pieces number from, clear of the kernel's.
const SPLIT: u64 = 0x8000;

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    match name {
        "sweep" => sweep(ev, args, span),
        "coil" => coil(ev, args, span),
        "draft" => draft(ev, args, span),
        "offset_faces" => offset_faces(ev, args, span),
        "delete_faces" | "replace_face" | "move_faces" => direct(ev, name, args, span),
        "thicken" => thicken(ev, args, span),
        "rib" => rib(ev, args, span),
        "facefillet" => facefillet(ev, args, span),
        "fullround" => fullround(ev, args, span),
        "ball" | "sphere" => ball(ev, name, args, span),
        _ => dome(ev, args, span),
    }
}

fn feat(id: String, kind: &'static str, idx: usize, hid: u64, prof: Option<Rc<RefCell<Sk>>>) -> Value {
    Value::Feature(Rc::new(Feat {
        id,
        kind,
        idx,
        hid,
        shape: None,
        axis: None,
        prof,
    }))
}

/// How a solid-making feature combines: `cut`, `keep` (intersect) or `new` (a body of its own),
/// at most one; else it adds (a new body for the first solid).
fn combine(ev: &Eval, a: &Args, func: &'static str) -> R<Combine> {
    let flags = [
        ("cut", Combine::Cut),
        ("keep", Combine::Keep),
        ("new", Combine::New),
    ];
    let mut set = None;
    for (k, c) in flags {
        if a.flag(k)? {
            if let Some((first, _)) = set {
                return Err(Error::NotWith {
                    span: a.span(k),
                    func,
                    arg: k,
                    with: first,
                });
            }
            set = Some((k, c));
        }
    }
    Ok(match set {
        Some((_, c)) => c,
        None if has_solid(ev) => Combine::Add,
        None => Combine::New,
    })
}

fn faces_arg(a: &Args, k: &'static str) -> R<Option<Rc<Faces>>> {
    match a.opt(k) {
        None | Some(Value::Unresolved) => Ok(None),
        Some(Value::Faces(f)) => Ok(Some(f)),
        Some(o) => Err(Error::WrongType {
            span: a.span(k),
            what: k,
            want: Ty::Faces,
            found: o.ty(),
        }),
    }
}

fn names_arg(a: &Args, k: &'static str) -> R<Vec<Name>> {
    Ok(faces_arg(a, k)?.map(|f| f.names.clone()).unwrap_or_default())
}

/// A face set's target for an end condition: the feature that made it and its first face's name.
fn target(ev: &Eval, f: &Faces) -> Target {
    let feature = ev.feats.iter().find(|x| x.id == f.owner).map_or(0, |x| x.idx);
    Target::Face {
        feature,
        name: f.names.first().copied().unwrap_or(Name::NONE),
    }
}

// ---- paths ----------------------------------------------------------------------------------

/// The sketch entities a path value names, in order: an entity, a loop (rect, poly, path ..), or
/// a list of them (from any sketches: a path in 3D).
fn path_ents(v: Value, span: Span, out: &mut Vec<(Rc<RefCell<Sk>>, EntityId)>) -> R<()> {
    match v {
        Value::List(xs) => {
            for x in xs.iter() {
                path_ents(x.clone(), span, out)?;
            }
            Ok(())
        }
        Value::Ent(e) if !e.loop_segs.is_empty() => {
            out.extend(e.loop_segs.iter().map(|&s| (e.sk.clone(), s)));
            Ok(())
        }
        Value::Ent(e) if matches!(e.kind, EntKind::Line | EntKind::Arc | EntKind::Spline) => {
            out.push((e.sk.clone(), e.id));
            Ok(())
        }
        Value::Ent(e) => Err(Error::WrongEntity {
            span,
            want: EntKind::Line,
            found: e.kind,
        }),
        o => Err(Error::WrongType {
            span,
            what: "a path (sketch lines, arcs, splines, or a list of them)",
            want: Ty::Entity,
            found: o.ty(),
        }),
    }
}

/// One path piece in the world, its ends, as drawn.
fn world_piece(sk: &Sk, id: EntityId) -> (PathSeg, Point3, Point3) {
    let plane = sk.frame;
    match sk.s.entity(id) {
        Some(Entity::Line { a, b }) => {
            let (p, q) = (sk.world(sk.pos(a)), sk.world(sk.pos(b)));
            (PathSeg::Line { a: p, b: q }, p, q)
        }
        Some(Entity::Arc {
            center,
            start,
            end,
            ccw,
        }) => {
            let (c, a, b) = (sk.pos(center), sk.pos(start), sk.pos(end));
            let (a0, a1) = (
                dmath::atan2(a.y - c.y, a.x - c.x),
                dmath::atan2(b.y - c.y, b.x - c.x),
            );
            let sweep = if ccw {
                dmath::wrap_tau(a1 - a0)
            } else {
                -dmath::wrap_tau(a0 - a1)
            };
            let circle = Circle {
                frame: plane.with_origin(plane.plane_point(c)),
                radius: a.distance(c),
            };
            (
                PathSeg::Arc {
                    circle,
                    start: a0,
                    sweep,
                },
                sk.world(a),
                sk.world(b),
            )
        }
        _ => {
            let c = sk.s.curve(id).expect("a curve entity");
            let w = c.c.mapped(|p| plane.point_to_world(p));
            (
                PathSeg::Spline(Arc::new(w)),
                sk.world(c.start()),
                sk.world(c.end()),
            )
        }
    }
}

fn reversed(p: PathSeg) -> PathSeg {
    match p {
        PathSeg::Line { a, b } => PathSeg::Line { a: b, b: a },
        PathSeg::Arc { circle, start, sweep } => PathSeg::Arc {
            circle,
            start: start + sweep,
            sweep: -sweep,
        },
        PathSeg::Spline(c) => PathSeg::Spline(Arc::new(c.reversed())),
    }
}

const MEET: f64 = 1e-6;

/// The pieces chained end to end in the order given, each turned to start where the one before
/// ends (the first, to end where the second meets it).
fn chain<T>(pieces: Vec<(T, Point3, Point3)>, rev: impl Fn(T) -> T, span: Span) -> R<Vec<T>> {
    let mut out = Vec::with_capacity(pieces.len());
    let mut at: Option<Point3> = None;
    for (k, (p, a, b)) in pieces.into_iter().enumerate() {
        let forward = match at {
            Some(x) if x.distance(a) <= MEET => true,
            Some(x) if x.distance(b) <= MEET => false,
            Some(_) => return Err(Error::PathGap { span, index: k }),
            None => true,
        };
        let (p, end) = if forward { (p, b) } else { (rev(p), a) };
        out.push(p);
        at = Some(end);
    }
    Ok(out)
}

/// A path value as kernel path pieces, chained; the first piece turned when it is the second
/// that meets its start.
pub fn path_of(v: Value, span: Span) -> R<Vec<PathSeg>> {
    let mut ents = vec![];
    path_ents(v, span, &mut ents)?;
    if ents.is_empty() {
        return Err(Error::EmptySelection { span });
    }
    let mut pieces: Vec<(PathSeg, Point3, Point3)> = ents
        .iter()
        .map(|(sk, id)| world_piece(&sk.borrow(), *id))
        .collect();
    if pieces.len() > 1 {
        let (a, b) = (pieces[0].1, pieces[0].2);
        let (c, d) = (pieces[1].1, pieces[1].2);
        let end_meets = b.distance(c) <= MEET || b.distance(d) <= MEET;
        let start_meets = a.distance(c) <= MEET || a.distance(d) <= MEET;
        if !end_meets && start_meets {
            let (p, a, b) = pieces.remove(0);
            pieces.insert(0, (reversed(p), b, a));
        }
    }
    chain(pieces, reversed, span)
}

// ---- sweep ----------------------------------------------------------------------------------

fn sweep(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "sweep", span)?;
    let id = ev.take_id("sweep", span)?;
    let a = Args::bind(
        ev,
        "sweep",
        &[
            "profile", "along", "extend", "upto", "offset", "cut", "keep", "new",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (sk, profile, _) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
    let path = path_of(a.req("along")?, a.span("along"))?;
    let mut dims = vec![];
    let ends = match (a.opt("extend"), faces_arg(&a, "upto")?) {
        (Some(_), Some(_)) => {
            return Err(Error::NotWith {
                span: a.span("extend"),
                func: "sweep",
                arg: "extend",
                with: "upto",
            });
        }
        (Some(Value::Tuple(t)), None) if t.len() == 2 => {
            let s = num_of(t[0].clone(), LEN, a.span("extend"), "extend")?;
            let e = num_of(t[1].clone(), LEN, a.span("extend"), "extend")?;
            dims.push(dim("extend start", &s));
            dims.push(dim("extend end", &e));
            PathEnds::Extended { start: s.v, end: e.v }
        }
        (Some(o), None) => {
            return Err(Error::WrongType {
                span: a.span("extend"),
                what: "extend: (before the start, after the end)",
                want: Ty::Tuple,
                found: o.ty(),
            });
        }
        (None, Some(f)) => {
            let off = a.num("offset", LEN)?;
            if let Some(o) = &off {
                dims.push(dim("offset", o));
            }
            PathEnds::To {
                target: target(ev, &f),
                offset: off.map_or(0.0, |o| o.v),
            }
        }
        (None, None) => {
            if a.opt("offset").is_some() {
                return Err(Error::NotWith {
                    span: a.span("offset"),
                    func: "sweep",
                    arg: "offset",
                    with: "no upto",
                });
            }
            PathEnds::Path
        }
    };
    let op = combine(ev, &a, "sweep")?;
    let f = Feature::Sweep(Box::new(Sweep {
        profile,
        path,
        ends,
        op,
    }));
    let (idx, hid) = ev.push_feature(&id, "sweep", f, span, dims)?;
    Ok(feat(id, "sweep", idx, hid, Some(sk)))
}

// ---- coil -----------------------------------------------------------------------------------

fn coil(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "coil", span)?;
    let id = ev.take_id("coil", span)?;
    let a = Args::bind(
        ev,
        "coil",
        &[
            "profile", "axis", "pitch", "turns", "height", "upto", "offset", "taper", "left", "cut", "keep",
            "new",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (sk, profile, _) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
    let ax = ent_of(a.req("axis")?, a.span("axis"), EntKind::Line)?;
    if !Rc::ptr_eq(&ax.sk, &sk) || ax.kind != EntKind::Line {
        return Err(Error::WrongEntity {
            span: a.span("axis"),
            want: EntKind::Line,
            found: ax.kind,
        });
    }
    let axis = {
        let s = sk.borrow();
        let Some(Entity::Line { a: p, b: q }) = s.s.entity(ax.id) else {
            unreachable!("a line")
        };
        Axis2 {
            origin: s.pos(p),
            dir: s.pos(q) - s.pos(p),
        }
    };
    let mut dims = vec![];
    let size = |k: &'static str, what: &'static str, dims: &mut Vec<_>| -> R<Option<Num>> {
        let n = match k {
            "turns" => a.num(k, &[Dim::Count, Dim::Ratio, Dim::Any])?,
            _ => a.num(k, LEN)?,
        };
        if let Some(n) = &n {
            positive(n, a.span(k), what)?;
            dims.push(dim(what, n));
        }
        Ok(n)
    };
    let pitch = size("pitch", "pitch", &mut dims)?;
    let turns = size("turns", "turns", &mut dims)?;
    let height = size("height", "height", &mut dims)?;
    let upto = faces_arg(&a, "upto")?;
    let csize = match (pitch, turns, height, upto) {
        (Some(p), None, None, Some(f)) => {
            let off = a.num("offset", LEN)?;
            if let Some(o) = &off {
                dims.push(dim("offset", o));
            }
            CoilSize::To {
                pitch: p.v,
                target: target(ev, &f),
                offset: off.map_or(0.0, |o| o.v),
            }
        }
        (_, _, _, Some(_)) => {
            return Err(Error::NotWith {
                span: a.span("upto"),
                func: "coil",
                arg: "upto",
                with: "turns or height (upto takes a pitch alone)",
            });
        }
        (Some(p), Some(t), None, None) => CoilSize::PitchTurns {
            pitch: p.v,
            turns: t.v,
        },
        (None, Some(t), Some(h), None) => CoilSize::TurnsHeight {
            turns: t.v,
            height: h.v,
        },
        (Some(p), None, Some(h), None) => CoilSize::PitchHeight {
            pitch: p.v,
            height: h.v,
        },
        (Some(_), Some(_), Some(_), None) => {
            return Err(Error::NotWith {
                span: a.span("height"),
                func: "coil",
                arg: "height",
                with: "pitch and turns (two of the three size a coil)",
            });
        }
        _ => {
            return Err(Error::MissingArg {
                span,
                func: "coil",
                arg: "two of pitch, turns, height (or pitch and upto)",
            });
        }
    };
    if a.opt("offset").is_some() && !matches!(csize, CoilSize::To { .. }) {
        return Err(Error::NotWith {
            span: a.span("offset"),
            func: "coil",
            arg: "offset",
            with: "no upto",
        });
    }
    let taper = match a.num("taper", ANG)? {
        Some(t) => {
            dims.push(dim("taper", &t));
            t.v
        }
        None => 0.0,
    };
    let handed = if a.flag("left")? {
        Handed::Left
    } else {
        Handed::Right
    };
    let op = combine(ev, &a, "coil")?;
    let f = Feature::Coil(Box::new(Coil {
        profile,
        axis,
        size: csize,
        taper,
        handed,
        op,
    }));
    let (idx, hid) = ev.push_feature(&id, "coil", f, span, dims)?;
    let (wa, wd) = {
        let s = sk.borrow();
        let o = s.world(axis.origin);
        (o, crate::eval::unit_or_z(s.world(axis.origin + axis.dir) - o))
    };
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "coil",
        idx,
        hid,
        shape: None,
        axis: Some((wa, wd)),
        prof: Some(sk),
    })))
}

// ---- loft -----------------------------------------------------------------------------------

/// `dome(P, DIR, w)`: a loft's end closing on point P, rounded, its walls arriving square to DIR
/// at w times unit speed.
fn dome(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, "dome", &["at", "dir", "w"], args, span)?;
    let p = point_of(ev, a.req("at")?, a.span("at"))?;
    let d = dir_of(a.req("dir")?, a.span("dir"))?;
    let w = match a.num("w", &[Dim::Ratio, Dim::Any])? {
        Some(w) => positive(&w, a.span("w"), "a dome's weight")?,
        None => 1.0,
    };
    Ok(Value::Tuple(Rc::new(vec![
        Value::Point(p),
        Value::Dir(d),
        Value::Num(Num::konst(w, Dim::Ratio)),
    ])))
}

fn apex_of(v: &Value) -> Option<Apex> {
    match v {
        Value::Point(p) => Some(Apex {
            point: *p,
            dome: None,
        }),
        Value::Tuple(t) => match &t[..] {
            [Value::Point(p), Value::Dir(d), Value::Num(w)] => Some(Apex {
                point: *p,
                dome: Some((*d, w.v)),
            }),
            _ => None,
        },
        _ => None,
    }
}

/// How many segments the kernel counts in a loop (a whole circle is two).
fn seg_count(l: &forge_ops::ProfileLoop) -> usize {
    l.segments
        .iter()
        .map(|s| {
            if matches!(s.0, Segment::Circle { .. }) {
                2
            } else {
                1
            }
        })
        .sum()
}

fn seg_len(s: &Segment) -> Option<f64> {
    match *s {
        Segment::Line { a, b } => Some(a.distance(b)),
        Segment::Arc { radius, sweep, .. } => Some(radius * sweep.abs()),
        _ => None,
    }
}

/// Loop `l` split until it has `want` segments: a whole circle into two half arcs first, then
/// the longest line or arc halved, again and again. Splines are never split.
fn split_to(l: &mut forge_ops::ProfileLoop, want: usize, next: &mut u64) -> bool {
    let mut piece = |id: u64| {
        *next += 1;
        (id & !PIECE) | (*next << 48)
    };
    if let [(Segment::Circle { center, radius, ccw }, id)] = l.segments[..] {
        let h = if ccw { dmath::PI } else { -dmath::PI };
        l.segments = vec![
            (
                Segment::Arc {
                    center,
                    radius,
                    start: 0.0,
                    sweep: h,
                },
                id,
            ),
            (
                Segment::Arc {
                    center,
                    radius,
                    start: h,
                    sweep: h,
                },
                piece(id),
            ),
        ];
    }
    while l.segments.len() < want {
        let Some(k) = (0..l.segments.len())
            .filter(|&k| seg_len(&l.segments[k].0).is_some())
            .max_by(|&x, &y| {
                seg_len(&l.segments[x].0)
                    .partial_cmp(&seg_len(&l.segments[y].0))
                    .expect("finite")
            })
        else {
            return false;
        };
        let (s, id) = l.segments[k].clone();
        let (p, q) = match s {
            Segment::Line { a, b } => {
                let m = a.lerp(b, 0.5);
                (Segment::Line { a, b: m }, Segment::Line { a: m, b })
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => (
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep: sweep / 2.0,
                },
                Segment::Arc {
                    center,
                    radius,
                    start: start + sweep / 2.0,
                    sweep: sweep / 2.0,
                },
            ),
            _ => unreachable!("only lines and arcs are split"),
        };
        l.segments[k] = (p, id);
        l.segments.insert(k + 1, (q, piece(id)));
    }
    true
}

/// The profiles' loops matched segment for segment: each loop split to the most any profile has
/// at that loop.
fn match_profiles(ps: &mut [Profile], spans: &[Span]) -> R<()> {
    let loops = ps[0].loops.len();
    for (k, p) in ps.iter().enumerate() {
        if p.loops.len() != loops {
            return Err(Error::LoftLoops {
                span: spans[k],
                profile: k,
                want: loops,
                got: p.loops.len(),
            });
        }
    }
    let mut next = SPLIT;
    for j in 0..loops {
        let want = ps
            .iter()
            .map(|p| seg_count(&p.loops[j]))
            .max()
            .expect("two at least");
        let r = ps
            .iter()
            .position(|p| seg_count(&p.loops[j]) == want)
            .expect("the most");
        let corners: Vec<Point3> = loop_starts(&ps[r].loops[j])
            .into_iter()
            .map(|v| ps[r].plane.plane_point(v))
            .collect();
        for (k, p) in ps.iter_mut().enumerate() {
            if seg_count(&p.loops[j]) == want {
                continue;
            }
            if let [(Segment::Circle { center, radius, ccw }, id)] = p.loops[j].segments[..] {
                let plane = p.plane;
                if let Some(arcs) = circle_at(center, radius, ccw, id, &corners, plane, &mut next) {
                    p.loops[j].segments = arcs;
                    continue;
                }
            }
            if !split_to(&mut p.loops[j], want, &mut next) {
                return Err(Error::LoftCurves {
                    span: spans[k],
                    profile: k,
                });
            }
        }
    }
    Ok(())
}

/// Where a loop's segments start (a circle as two halves).
fn loop_starts(l: &forge_ops::ProfileLoop) -> Vec<Point2> {
    let mut out = vec![];
    for (s, _) in &l.segments {
        match *s {
            Segment::Circle { center, radius, ccw } => {
                let h = if ccw { dmath::PI } else { -dmath::PI };
                for a in [0.0, h] {
                    let (sn, cs) = dmath::sin_cos(a);
                    out.push(Point2::new(center.x + radius * cs, center.y + radius * sn));
                }
            }
            _ => out.push(s.start()),
        }
    }
    out
}

/// A whole circle as arcs from the angles of `corners` (another profile's segment starts, in the
/// world) seen from its centre, the first arc starting at the first corner: so the loft's walls
/// run from each corner to the circle square across from it. None when the corners do not go
/// round the circle once in its own sense.
fn circle_at(
    center: Point2,
    radius: f64,
    ccw: bool,
    id: u64,
    corners: &[Point3],
    plane: Frame,
    next: &mut u64,
) -> Option<Vec<(Segment, u64)>> {
    let angles: Vec<f64> = corners
        .iter()
        .map(|&w| {
            let l = plane.point_to_local(w);
            dmath::atan2(l.y - center.y, l.x - center.x)
        })
        .collect();
    let n = angles.len();
    let sweeps: Vec<f64> = (0..n)
        .map(|i| {
            let d = angles[(i + 1) % n] - angles[i];
            if ccw {
                dmath::wrap_tau(d)
            } else {
                -dmath::wrap_tau(-d)
            }
        })
        .collect();
    let total: f64 = sweeps.iter().sum();
    if n < 2 || (total.abs() - dmath::TAU).abs() > 1e-6 || sweeps.iter().any(|s| s.abs() < 1e-9) {
        return None;
    }
    Some(
        (0..n)
            .map(|i| {
                let sid = if i == 0 {
                    id
                } else {
                    *next += 1;
                    (id & !PIECE) | (*next << 48)
                };
                (
                    Segment::Arc {
                        center,
                        radius,
                        start: angles[i],
                        sweep: sweeps[i],
                    },
                    sid,
                )
            })
            .collect(),
    )
}

/// Where each of a profile's segments starts, in the kernel's order (a circle as two halves).
fn vertex_points(p: &Profile) -> Vec<Point2> {
    let mut out = vec![];
    for l in &p.loops {
        for (s, _) in &l.segments {
            match *s {
                Segment::Circle { center, radius, ccw } => {
                    let h = if ccw { dmath::PI } else { -dmath::PI };
                    for a in [0.0, h] {
                        let (sn, cs) = dmath::sin_cos(a);
                        out.push(Point2::new(center.x + radius * cs, center.y + radius * sn));
                    }
                }
                _ => out.push(s.start()),
            }
        }
    }
    out
}

pub fn loft(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "loft", span)?;
    let id = ev.take_id("loft", span)?;
    let a = Args::bind(
        ev,
        "loft",
        &[
            "profiles", "smooth", "start", "end", "align", "seams", "rails", "centre", "cut", "keep", "new",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let xs = match a.req("profiles")? {
        Value::List(xs) => xs,
        o => {
            return Err(Error::WrongType {
                span: a.span("profiles"),
                what: "profiles",
                want: Ty::List,
                found: o.ty(),
            });
        }
    };
    let sp = a.span("profiles");
    let mut apex = [None, None];
    let mut profiles = vec![];
    let mut sketches: Vec<Rc<RefCell<Sk>>> = vec![];
    let last = xs.len().saturating_sub(1);
    for (k, x) in xs.iter().enumerate() {
        if let Some(ap) = apex_of(x) {
            if k != 0 && k != last {
                return Err(Error::WrongType {
                    span: sp,
                    what: "a loft's point (only first or last)",
                    want: Ty::Sketch,
                    found: Ty::Point,
                });
            }
            apex[usize::from(k != 0)] = Some(ap);
            continue;
        }
        let (sk, p, _) = profile_of(ev, x.clone(), sp)?;
        profiles.push(p);
        sketches.push(sk);
    }
    if profiles.is_empty() {
        return Err(Error::MissingArg {
            span: sp,
            func: "loft",
            arg: "a profile",
        });
    }
    let mut dims = vec![];
    let smooth = a.flag("smooth")?;
    let weight = |k: &'static str, dims: &mut Vec<_>| -> R<Option<f64>> {
        match a.num(k, &[Dim::Ratio, Dim::Any])? {
            Some(_) if !smooth => Err(Error::NotWith {
                span: a.span(k),
                func: "loft",
                arg: k,
                with: "a ruled loft (smooth: true takes end weights)",
            }),
            Some(w) => {
                positive(&w, a.span(k), "an end weight")?;
                dims.push(dim(k, &w));
                Ok(Some(w.v))
            }
            None => Ok(None),
        }
    };
    let start = weight("start", &mut dims)?;
    let end = weight("end", &mut dims)?;
    let kind = if smooth {
        LoftKind::Smooth { start, end }
    } else {
        LoftKind::Ruled
    };
    let spans = vec![sp; profiles.len()];
    if profiles.len() > 1 {
        match_profiles(&mut profiles, &spans)?;
    }
    // seams: a point of a profile's sketch, matched to its profile by the sketch
    let mut seams = vec![];
    if let Some(v) = a.opt("seams") {
        let Value::List(ps) = v else {
            return Err(Error::WrongType {
                span: a.span("seams"),
                what: "seams",
                want: Ty::List,
                found: v.ty(),
            });
        };
        seams = vec![None; profiles.len()];
        for p in ps.iter() {
            let e = ent_of(p.clone(), a.span("seams"), EntKind::Point)?;
            let k = sketches
                .iter()
                .position(|s| Rc::ptr_eq(s, &e.sk))
                .ok_or(Error::OtherSketch {
                    span: a.span("seams"),
                    sketch: e.sk.borrow().id.clone(),
                    here: sketches[0].borrow().id.clone(),
                })?;
            let pid = crate::builtins::point_ent(&e, a.span("seams"))?;
            seams[k] = Some(e.sk.borrow().pos(pid));
        }
    }
    let guides = match (a.opt("rails"), a.opt("centre")) {
        (Some(_), Some(_)) => {
            return Err(Error::NotWith {
                span: a.span("centre"),
                func: "loft",
                arg: "centre",
                with: "rails",
            });
        }
        (Some(Value::List(rs)), None) => {
            let corners = vertex_points(&profiles[0]);
            let mut out = vec![];
            for r in rs.iter() {
                let Value::Tuple(t) = r else {
                    return Err(Error::WrongType {
                        span: a.span("rails"),
                        what: "a rail (a corner of the first profile, a path)",
                        want: Ty::Tuple,
                        found: r.ty(),
                    });
                };
                let [corner, path] = &t[..] else {
                    return Err(Error::WrongType {
                        span: a.span("rails"),
                        what: "a rail (a corner of the first profile, a path)",
                        want: Ty::Tuple,
                        found: Ty::Tuple,
                    });
                };
                let e = ent_of(corner.clone(), a.span("rails"), EntKind::Point)?;
                if !Rc::ptr_eq(&e.sk, &sketches[0]) {
                    return Err(Error::NotAVertex {
                        span: a.span("rails"),
                    });
                }
                let pid = crate::builtins::point_ent(&e, a.span("rails"))?;
                let at = e.sk.borrow().pos(pid);
                let i = corners
                    .iter()
                    .position(|c| c.distance(at) <= MEET)
                    .ok_or(Error::NotAVertex {
                        span: a.span("rails"),
                    })?;
                let pp = path_of(path.clone(), a.span("rails"))?;
                out.push((i, pp));
            }
            LoftGuides::Rails(out)
        }
        (Some(o), None) => {
            return Err(Error::WrongType {
                span: a.span("rails"),
                what: "rails",
                want: Ty::List,
                found: o.ty(),
            });
        }
        (None, Some(c)) => LoftGuides::CentreLine(path_of(c, a.span("centre"))?),
        (None, None) => LoftGuides::None,
    };
    let options = LoftOptions {
        apex,
        align: a.flag("align")?,
        seams,
    };
    let op = combine(ev, &a, "loft")?;
    let f = Feature::Loft(Box::new(Loft {
        profiles,
        kind,
        guides,
        options,
        op,
    }));
    let (idx, hid) = ev.push_feature(&id, "loft", f, span, dims)?;
    Ok(feat(id, "loft", idx, hid, sketches.into_iter().next()))
}

// ---- face edits -----------------------------------------------------------------------------

fn draft(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "draft", span)?;
    let id = ev.take_id("draft", span)?;
    let a = Args::bind(ev, "draft", &["faces", "angle", "neutral"], args, span)?;
    let faces = names_arg(&a, "faces")?;
    if faces.is_empty() && ev.b.building && !a.poisoned() {
        return Err(Error::EmptySelection {
            span: a.span("faces"),
        });
    }
    let angle = a.req_num("angle", ANG)?;
    let neutral = crate::builtins::plane_of(ev, a.req("neutral")?, a.span("neutral"), "neutral")?;
    let f = Feature::Draft {
        faces,
        neutral,
        angle: angle.v,
    };
    let (idx, hid) = ev.push_feature(&id, "draft", f, span, vec![dim("angle", &angle)])?;
    Ok(feat(id, "draft", idx, hid, None))
}

fn offset_faces(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "offset_faces", span)?;
    let id = ev.take_id("offset_faces", span)?;
    let a = Args::bind(ev, "offset_faces", &["faces", "d"], args, span)?;
    let faces = names_arg(&a, "faces")?;
    if faces.is_empty() && ev.b.building && !a.poisoned() {
        return Err(Error::EmptySelection {
            span: a.span("faces"),
        });
    }
    let d = a.req_num("d", LEN)?;
    let f = Feature::Edit(Edit::Offset { faces, distance: d.v });
    let (idx, hid) = ev.push_feature(&id, "offset_faces", f, span, vec![dim("distance", &d)])?;
    Ok(feat(id, "offset_faces", idx, hid, None))
}

/// Direct edits: faces taken away (the gap closed by their neighbours), one face put on another
/// face's surface, faces moved along a direction or turned about an axis (neighbours re-met).
fn direct(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let keys: &[&'static str] = match name {
        "delete_faces" => &["faces"],
        "replace_face" => &["face", "with"],
        _ => &["faces", "along", "d", "about", "by"],
    };
    let a = Args::bind(ev, name, keys, args, span)?;
    let first = keys[0];
    let faces = names_arg(&a, first)?;
    let building = ev.b.building && !a.poisoned();
    if faces.is_empty() && building {
        return Err(Error::EmptySelection { span: a.span(first) });
    }
    let mut dims = vec![];
    let edit = match name {
        "delete_faces" => Edit::Delete { faces },
        "replace_face" => {
            let face = match faces[..] {
                [n] => n,
                _ if !building => Name::NONE,
                _ => {
                    return Err(Error::OneFace {
                        span: a.span("face"),
                        what: "face",
                        count: faces.len(),
                    });
                }
            };
            let with = match faces_arg(&a, "with")? {
                Some(f) => target(ev, &f),
                None if !building => Target::Face {
                    feature: 0,
                    name: Name::NONE,
                },
                None => {
                    return Err(Error::MissingArg {
                        span,
                        func: name,
                        arg: "with",
                    });
                }
            };
            Edit::Replace { face, with }
        }
        _ => {
            let motion = match (a.opt("along"), a.opt("about")) {
                (Some(_), Some(_)) => {
                    return Err(Error::NotWith {
                        span: a.span("about"),
                        func: name,
                        arg: "about",
                        with: "along",
                    });
                }
                (Some(v), None) => {
                    let d = a.req_num("d", LEN)?;
                    dims.push(dim("distance", &d));
                    let dir = dir_of(v, a.span("along"))?;
                    let u = dir.normalize().ok_or(Error::BadValue {
                        span: a.span("along"),
                        what: "a zero direction",
                        value: 0.0,
                    })?;
                    forge_model::Motion::Translate(u.get() * d.v)
                }
                (None, Some(v)) => {
                    let (o, ax) = crate::builtins::axis_of(ev, v, a.span("about"))?;
                    let by = a.req_num("by", ANG)?;
                    dims.push(dim("angle", &by));
                    forge_model::Motion::Rotate {
                        origin: o,
                        axis: ax.get(),
                        angle: by.v,
                    }
                }
                (None, None) => {
                    return Err(Error::MissingArg {
                        span,
                        func: name,
                        arg: "along: DIR, d: D, or about: AXIS, by: A",
                    });
                }
            };
            Edit::Move { faces, motion }
        }
    };
    let (idx, hid) = ev.push_feature(&id, name, Feature::Edit(edit), span, dims)?;
    Ok(feat(id, name, idx, hid, None))
}

fn side_word(a: &Args) -> R<Side> {
    Ok(match word(a, "side", &["out", "in", "both"])? {
        Some("in") => Side::Neg,
        Some("both") => Side::Both,
        _ => Side::Pos,
    })
}

fn thicken(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "thicken", span)?;
    let id = ev.take_id("thicken", span)?;
    let a = Args::bind(
        ev,
        "thicken",
        &["faces", "t", "side", "cut", "keep", "new"],
        args,
        span,
    )?;
    let faces = names_arg(&a, "faces")?;
    if faces.is_empty() && ev.b.building && !a.poisoned() {
        return Err(Error::EmptySelection {
            span: a.span("faces"),
        });
    }
    let t = a.req_num("t", LEN)?;
    positive(&t, a.span("t"), "a thickness")?;
    let side = side_word(&a)?;
    let op = match combine(ev, &a, "thicken")? {
        Combine::New if !a.flag("new")? => Combine::Add,
        c => c,
    };
    let f = Feature::Edit(Edit::Thicken {
        faces,
        thickness: t.v,
        side,
        op,
    });
    let (idx, hid) = ev.push_feature(&id, "thicken", f, span, vec![dim("thickness", &t)])?;
    Ok(feat(id, "thicken", idx, hid, None))
}

/// A rib's path: lines and arcs of one sketch, end to end, in its plane.
fn rib_path(v: Value, span: Span) -> R<(Rc<RefCell<Sk>>, Vec<Segment>)> {
    let mut ents = vec![];
    path_ents(v, span, &mut ents)?;
    let Some(sk) = ents.first().map(|e| e.0.clone()) else {
        return Err(Error::EmptySelection { span });
    };
    let s = sk.borrow();
    let mut pieces = vec![];
    for (k, id) in &ents {
        if !Rc::ptr_eq(k, &sk) {
            return Err(Error::OtherSketch {
                span,
                sketch: k.borrow().id.clone(),
                here: s.id.clone(),
            });
        }
        let seg = match s.s.entity(*id) {
            Some(Entity::Line { a, b }) => Segment::Line {
                a: s.pos(a),
                b: s.pos(b),
            },
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }) => {
                let (c, p, q) = (s.pos(center), s.pos(start), s.pos(end));
                let (a0, a1) = (
                    dmath::atan2(p.y - c.y, p.x - c.x),
                    dmath::atan2(q.y - c.y, q.x - c.x),
                );
                Segment::Arc {
                    center: c,
                    radius: p.distance(c),
                    start: a0,
                    sweep: if ccw {
                        dmath::wrap_tau(a1 - a0)
                    } else {
                        -dmath::wrap_tau(a0 - a1)
                    },
                }
            }
            _ => {
                return Err(Error::WrongEntity {
                    span,
                    want: EntKind::Line,
                    found: s.kind(*id),
                });
            }
        };
        let (p, q) = (s.world(seg.start()), s.world(seg.end()));
        pieces.push((seg, p, q));
    }
    if pieces.len() > 1 {
        let (a, b) = (pieces[0].1, pieces[0].2);
        let (c, d) = (pieces[1].1, pieces[1].2);
        if !(b.distance(c) <= MEET || b.distance(d) <= MEET)
            && (a.distance(c) <= MEET || a.distance(d) <= MEET)
        {
            let (p, a, b) = pieces.remove(0);
            pieces.insert(0, (p.reversed(), b, a));
        }
    }
    let segs = chain(pieces, |x: Segment| x.reversed(), span)?;
    drop(s);
    Ok((sk, segs))
}

fn rib(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "rib", span)?;
    let id = ev.take_id("rib", span)?;
    let a = Args::bind(
        ev,
        "rib",
        &["along", "t", "side", "web", "flip", "draft"],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (sk, path) = rib_path(a.req("along")?, a.span("along"))?;
    let t = a.req_num("t", LEN)?;
    positive(&t, a.span("t"), "a rib's thickness")?;
    let mut dims = vec![dim("thickness", &t)];
    let side = match word(&a, "side", &["both", "pos", "neg"])? {
        Some("pos") => Side::Pos,
        Some("neg") => Side::Neg,
        _ => Side::Both,
    };
    let draft = match a.num("draft", ANG)? {
        Some(d) => {
            dims.push(dim("draft", &d));
            d.v
        }
        None => 0.0,
    };
    let plane = sk.borrow().frame;
    let f = Feature::Edit(Edit::Rib(Box::new(Rib {
        plane,
        path,
        thickness: t.v,
        side,
        web: a.flag("web")?,
        flip: a.flag("flip")?,
        draft,
    })));
    let (idx, hid) = ev.push_feature(&id, "rib", f, span, dims)?;
    Ok(feat(id, "rib", idx, hid, None))
}

fn facefillet(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "facefillet", span)?;
    let id = ev.take_id("facefillet", span)?;
    let a = Args::bind(ev, "facefillet", &["a", "b", "r"], args, span)?;
    let first = names_arg(&a, "a")?;
    let second = names_arg(&a, "b")?;
    if ev.b.building && !a.poisoned() {
        for (k, f) in [("a", &first), ("b", &second)] {
            if f.is_empty() {
                return Err(Error::EmptySelection { span: a.span(k) });
            }
        }
    }
    let r = a.req_num("r", LEN)?;
    positive(&r, a.span("r"), "a fillet radius")?;
    let f = Feature::FaceFillet {
        first,
        second,
        radius: r.v,
    };
    let (idx, hid) = ev.push_feature(&id, "facefillet", f, span, vec![dim("radius", &r)])?;
    Ok(feat(id, "facefillet", idx, hid, None))
}

fn fullround(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "fullround", span)?;
    let id = ev.take_id("fullround", span)?;
    let a = Args::bind(ev, "fullround", &["side", "middle", "other"], args, span)?;
    let one = |k: &'static str| -> R<Name> {
        let f = names_arg(&a, k)?;
        match f[..] {
            [n] => Ok(n),
            _ if !ev.b.building || a.poisoned() => Ok(Name::NONE),
            _ => Err(Error::OneFace {
                span: a.span(k),
                what: k,
                count: f.len(),
            }),
        }
    };
    let (side, middle, other) = (one("side")?, one("middle")?, one("other")?);
    let f = Feature::FullRound { side, middle, other };
    let (idx, hid) = ev.push_feature(&id, "fullround", f, span, vec![])?;
    Ok(feat(id, "fullround", idx, hid, None))
}

fn ball(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, &["center", "r", "cut", "keep", "new"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let cv = a.req("center")?;
    // the ball's frame: the sketch's, when the centre is a sketch point (its seam follows it)
    let axes = match &cv {
        Value::Ent(e) => e.sk.borrow().frame,
        Value::Axis(o, d) => Frame::from_z(*o, *d),
        _ => Frame::WORLD,
    };
    let c = point_of(ev, cv, a.span("center"))?;
    let r = a.req_num("r", LEN)?;
    positive(&r, a.span("r"), "a ball's radius")?;
    let op = combine(ev, &a, name)?;
    let f = Feature::Ball {
        frame: axes.with_origin(c),
        radius: r.v,
        op,
    };
    let (idx, hid) = ev.push_feature(&id, "ball", f, span, vec![dim("radius", &r)])?;
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "ball",
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    })))
}

// ---- anchors --------------------------------------------------------------------------------

/// The kernel profiles a skinned feature (sweep, coil, loft) made its walls from, and how many
/// stretches each segment's walls run over.
fn skin_of(ev: &Eval, f: &Feat) -> Option<(Vec<u64>, u64)> {
    let ids = |p: &Profile| -> Vec<u64> {
        p.loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|s| s.1))
            .collect()
    };
    match ev.b.features.get(f.idx)? {
        Feature::Sweep(s) => Some((ids(&s.profile), s.path.len() as u64 + 2)),
        Feature::Coil(c) => Some((ids(&c.profile), 1)),
        Feature::Loft(l) => Some((ids(l.profiles.first()?), l.profiles.len().max(2) as u64 - 1)),
        _ => None,
    }
}

/// `F.side(entity)` for a sweep, coil or loft: the walls the profile entities' segments made, over
/// every stretch (for a loft, entities of its first profile).
pub fn side_names(ev: &Eval, f: &Feat, segs: &[u64]) -> Option<Vec<Name>> {
    let (ids, stretches) = skin_of(ev, f)?;
    let root = Name::feature(f.hid);
    let mut out = vec![];
    for &id in ids.iter().filter(|&&id| segs.contains(&(id & !PIECE))) {
        for half in 0..2 {
            for k in 0..stretches {
                out.push(roles::skin_side(root, id, half, k));
            }
        }
    }
    Some(out)
}

/// Role anchors of this module's features: a sweep's, coil's or loft's `start` and `end` caps; a
/// shell's `inner` skin and `rim`s.
pub fn member(ev: &mut Eval, f: &Feat, name: &str) -> Option<Value> {
    let root = Name::feature(f.hid);
    match (f.kind, name) {
        ("sweep" | "coil" | "loft", "start" | "end") => {
            let n = if name == "start" {
                roles::start(root)
            } else {
                roles::end(root)
            };
            let role = if name == "start" { Role::Start } else { Role::End };
            let v = role_set(ev, f, role, &[n], None);
            let reference = f.prof.as_ref().map_or(Frame::WORLD, |s| s.borrow().frame);
            Some(match (v, ev.b.body()) {
                (Value::Faces(fs), Some(body)) => {
                    let plane = fs
                        .names
                        .first()
                        .and_then(|n| body.face_ids().find(|&x| body.face(x).name == *n))
                        .and_then(|x| face_plane(body, x))
                        .map(|fp| carried(fp, reference));
                    Value::Faces(Rc::new(Faces {
                        plane,
                        ..(*fs).clone()
                    }))
                }
                (v, _) => v,
            })
        }
        ("shell", "inner" | "rim") => {
            let before = f.idx.checked_sub(1).and_then(|i| ev.b.stages.get(i).cloned());
            let mut names = vec![];
            let open = match ev.b.features.get(f.idx) {
                Some(Feature::Shell { open, .. }) => open.clone(),
                _ => vec![],
            };
            if name == "inner" {
                if let Some(st) = before {
                    for s in &st.solids {
                        for x in s.body.face_ids() {
                            let n = s.body.face(x).name;
                            if !open.contains(&n) {
                                names.push(roles::offset(root, n));
                            }
                        }
                    }
                }
            } else {
                for &o in &open {
                    for k in 0..8 {
                        names.push(roles::rim(root, o, k));
                    }
                }
            }
            Some(role_set(
                ev,
                f,
                if name == "inner" { Role::Faces } else { Role::Wall },
                &names,
                None,
            ))
        }
        _ => None,
    }
}
