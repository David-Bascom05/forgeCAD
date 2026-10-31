//! Name stability under upstream edits. A reference (a face, edge or vertex name) taken on the
//! part partway down its history must, after an edit above that point, name the entity that
//! geometrically corresponds to the one it named, or nothing. Never another one.
//!
//! The correspondence comes from geometry alone, never from names: a dimension or placement
//! edit is made in steps small enough that each entity moves less than the gap to any other,
//! and every entity is tracked step to step; an insert, delete or swap leaves the entities it
//! does not touch exactly where they were.

use crate::{
    At, AxisBy, BodyFeature, CoilSize, Combine, Cutter, Edit as PartEdit, End, Extent, Feature, Head, Keep,
    Layout, Part, PathEnds, Pick, PlaneBy, PointBy, Ref, RegenError, Source, Step, Target, ThreadLength,
    ThreadOn, Turn, UcsBy, Work,
};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Box3, Point2, Point3, Vec3};
use forge_ops::{ChamferSize, Profile, ProfileLoop, Radius, Segment};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};

#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Every size of feature `feature` scaled by `by` (its sketch about the sketch's origin).
    Resize { feature: usize, by: f64 },
    /// Feature `feature` moved by `by`.
    Move { feature: usize, by: Vec3 },
    /// `feature` put in at position `at`.
    Insert { at: usize, feature: Box<Feature> },
    /// Feature `at` taken out.
    Delete { at: usize },
    /// Features `at` and `at + 1` swapped.
    Swap { at: usize },
}

impl Edit {
    /// Where history position `i` (one not taken out) is after the edit.
    pub fn moves(&self, i: usize) -> usize {
        match *self {
            Edit::Insert { at, .. } if i >= at => i + 1,
            Edit::Delete { at } if i > at => i - 1,
            Edit::Swap { at } if i == at => at + 1,
            Edit::Swap { at } if i == at + 1 => at,
            _ => i,
        }
    }

    /// Whether the edit renumbers features already in the history.
    pub fn shifts(&self, before: usize) -> bool {
        (0..before).any(|i| self.moves(i) != i)
    }

    /// Applied to `part` (its features named by their places): the edited part, the ids that
    /// keep every feature its own (a new one takes the next id), and how far any point moves
    /// at most; None where the feature takes no such edit. A resize or move is made `t` of
    /// the way (0 to 1).
    pub fn apply(&self, part: &Part, t: f64) -> Option<(Part, Vec<u64>, f64)> {
        let mut fs = part.features.clone();
        let mut ids: Vec<u64> = (1..=fs.len() as u64).collect();
        let mut reach = 0.0;
        match self {
            Edit::Resize { feature, by } => {
                let (f, r) = resized(&fs[*feature], 1.0 + (by - 1.0) * t)?;
                fs[*feature] = f;
                reach = r * (by - 1.0).abs() * t;
            }
            Edit::Move { feature, by } => {
                fs[*feature] = moved(&fs[*feature], *by * t)?;
                reach = by.length() * t;
            }
            Edit::Insert { at, feature } => {
                fs.insert(*at, (**feature).clone());
                ids.insert(*at, ids.len() as u64 + 1);
            }
            Edit::Delete { at } => {
                fs.remove(*at);
                ids.remove(*at);
            }
            Edit::Swap { at } => {
                fs.swap(*at, at + 1);
                ids.swap(*at, at + 1);
            }
        }
        if self.shifts(fs.len() + 1) {
            for f in &mut fs {
                for r in feature_refs(f) {
                    *r = self.moves(*r);
                }
            }
        }
        Some((Part { features: fs }, ids, reach))
    }
}

// The plug-in points: a new feature kind adds its arm here to take edits. Kinds without one are
// left alone (never resized, moved, deleted or swapped), so they still build in histories.

/// `f` with its sizes scaled by `k`, and how far a point of it moves per unit of `k`.
pub fn resized(f: &Feature, k: f64) -> Option<(Feature, f64)> {
    let mut f = f.clone();
    let reach = match &mut f {
        Feature::Extrude { profile, extent, .. } => {
            let r = scale_profile(profile, k);
            match extent {
                Extent::Distance(d) | Extent::Symmetric(d) => {
                    *d *= k;
                    r.max(d.abs() / k)
                }
                Extent::TwoSided { forward, back } => {
                    *forward *= k;
                    *back *= k;
                    r.max(forward.abs().max(back.abs()) / k)
                }
                _ => r,
            }
        }
        Feature::Revolve { profile, axis, .. } => {
            axis.origin = Point2::new(axis.origin.x * k, axis.origin.y * k);
            scale_profile(profile, k)
        }
        Feature::Ball { radius, .. } => {
            *radius *= k;
            *radius / k
        }
        Feature::Hole(h) => {
            h.diameter *= k;
            let mut r = h.diameter / k;
            match &mut h.head {
                Head::Plain => {}
                Head::Counterbore { diameter, depth } => {
                    *diameter *= k;
                    *depth *= k;
                    r = r.max(*diameter / k);
                }
                Head::Countersink { diameter, .. } => {
                    *diameter *= k;
                    r = r.max(*diameter / k);
                }
            }
            if let End::Depth(d) = &mut h.end {
                *d *= k;
                r = r.max(*d / k + h.diameter / k);
            }
            r
        }
        Feature::Fillet { edges, .. } => {
            let mut r: f64 = 0.0;
            for (_, rad) in edges.iter_mut() {
                let Radius::Constant(x) = rad else { return None };
                *x *= k;
                r = r.max(2.0 * *x / k);
            }
            r
        }
        Feature::Chamfer {
            size: ChamferSize::Equal(d),
            ..
        } => {
            *d *= k;
            2.0 * *d / k
        }
        Feature::Shell { thickness, .. } => {
            *thickness *= k;
            *thickness / k
        }
        Feature::Work(Work::Plane(PlaneBy::Offset(_, d))) => {
            *d *= k;
            d.abs() / k
        }
        _ => return None,
    };
    Some((f, reach))
}

/// `f` moved by `d`.
pub fn moved(f: &Feature, d: Vec3) -> Option<Feature> {
    let mut f = f.clone();
    let frame = match &mut f {
        Feature::Extrude { profile, .. } | Feature::Revolve { profile, .. } => &mut profile.plane,
        Feature::Ball { frame, .. } | Feature::Plane(frame) => frame,
        Feature::Hole(h) => &mut h.at,
        _ => return None,
    };
    *frame = frame.with_origin(frame.origin + d);
    Some(f)
}

/// The earlier features `f` refers to by position, to renumber when the history shifts.
fn feature_refs(f: &mut Feature) -> Vec<&mut usize> {
    fn target(t: &mut Target) -> &mut usize {
        match t {
            Target::Plane(i) => i,
            Target::Face { feature, .. } => feature,
        }
    }
    match f {
        Feature::Extrude {
            extent: Extent::To { target: t, .. },
            ..
        }
        | Feature::Revolve {
            extent: Turn::To { target: t, .. },
            ..
        } => vec![target(t)],
        Feature::Hole(h) => match &mut h.end {
            End::To { target: t, .. } => vec![target(t)],
            _ => vec![],
        },
        Feature::Pattern { of, layout } => {
            let mut out: Vec<&mut usize> = match of {
                Source::Features(src) => src.iter_mut().collect(),
                Source::Body => vec![],
            };
            match layout {
                Layout::Path { along, .. } => out.extend(along.iter_mut().map(|r| &mut r.feature)),
                Layout::Points(at) => out.extend(at.iter_mut().filter_map(|a| match a {
                    At::Ref(r) => Some(&mut r.feature),
                    At::Sketch { feature, .. } => Some(feature),
                    At::Xyz(_) => None,
                })),
                _ => {}
            }
            out
        }
        Feature::Thread(t) => {
            let mut out = match &mut t.on {
                ThreadOn::Face { feature, .. } | ThreadOn::Hole(feature) => vec![feature],
                ThreadOn::Axis { .. } => vec![],
            };
            if let ThreadLength::To { target: t, .. } = &mut t.length {
                out.push(target(t));
            }
            out
        }
        Feature::Work(w) => {
            let refs: Vec<&mut Ref> = match w {
                Work::Plane(PlaneBy::Offset(a, _))
                | Work::Axis(AxisBy::Along(a))
                | Work::Point(PointBy::At(a) | PointBy::Mid(a)) => vec![a],
                Work::Plane(PlaneBy::Angle {
                    plane: a, axis: b, ..
                })
                | Work::Plane(PlaneBy::Tangent { face: a, plane: b })
                | Work::Plane(PlaneBy::Mid(a, b))
                | Work::Plane(PlaneBy::Normal { curve: a, at: b })
                | Work::Axis(
                    AxisBy::Points(a, b) | AxisBy::Meet(a, b) | AxisBy::Normal { plane: a, at: b },
                )
                | Work::Point(PointBy::Meet(a, b)) => vec![a, b],
                Work::Plane(PlaneBy::Points(rs)) | Work::Ucs(UcsBy::Points(rs)) => rs.iter_mut().collect(),
                Work::Ucs(UcsBy::Frame(_)) => vec![],
            };
            refs.into_iter().map(|r| &mut r.feature).collect()
        }
        Feature::Sweep(s) => match &mut s.ends {
            PathEnds::To { target: t, .. } => vec![target(t)],
            _ => vec![],
        },
        Feature::Edit(PartEdit::Replace { with, .. }) => vec![target(with)],
        Feature::Body(b) => match &mut **b {
            BodyFeature::In { bodies, feature } => {
                let mut out = match bodies {
                    Pick::Some(ids) => ids.iter_mut().collect(),
                    Pick::All => vec![],
                };
                out.extend(feature_refs(feature));
                out
            }
            BodyFeature::Combine { target, tools, .. } => std::iter::once(target).chain(tools).collect(),
            BodyFeature::Split {
                by: Cutter::Target(t),
                ..
            }
            | BodyFeature::SplitFace {
                by: Cutter::Target(t),
                ..
            } => vec![target(t)],
            _ => vec![],
        },
        Feature::Coil(c) => match &mut c.size {
            CoilSize::To { target: t, .. } => vec![target(t)],
            _ => vec![],
        },
        _ => vec![],
    }
}

/// The names `f` refers to on the part before it.
pub fn stored_refs(f: &Feature) -> Vec<(Kind, Name)> {
    match f {
        Feature::Fillet {
            edges,
            starts,
            setbacks,
        } => {
            let mut out: Vec<_> = edges.iter().map(|e| (Kind::Edge, e.0)).collect();
            out.extend(
                starts
                    .iter()
                    .flat_map(|&(e, v)| [(Kind::Edge, e), (Kind::Vertex, v)]),
            );
            for &(v, e, _) in setbacks {
                out.push((Kind::Vertex, v));
                out.extend(e.map(|e| (Kind::Edge, e)));
            }
            out
        }
        Feature::FaceFillet { first, second, .. } => {
            first.iter().chain(second).map(|&n| (Kind::Face, n)).collect()
        }
        Feature::FullRound { side, middle, other } => {
            [side, middle, other].iter().map(|&&n| (Kind::Face, n)).collect()
        }
        Feature::Chamfer { edges, size } => {
            let mut out: Vec<_> = edges.iter().map(|&e| (Kind::Edge, e)).collect();
            if let ChamferSize::Distances { face, .. } | ChamferSize::DistanceAngle { face, .. } = size {
                out.push((Kind::Face, *face));
            }
            out
        }
        Feature::Shell { open, .. } => open.iter().map(|&n| (Kind::Face, n)).collect(),
        Feature::Draft { faces, .. } => faces.iter().map(|&n| (Kind::Face, n)).collect(),
        Feature::Body(b) => match &**b {
            BodyFeature::In { feature, .. } => stored_refs(feature),
            BodyFeature::SplitFace { faces, .. } => faces.iter().map(|&n| (Kind::Face, n)).collect(),
            _ => vec![],
        },
        _ => vec![],
    }
}

/// The part `features` make (with `ids`, or by places), several bodies side by side as one.
pub fn regen(features: &[Feature], ids: Option<&[u64]>) -> Result<Body, RegenError> {
    let part = Part {
        features: features.to_vec(),
    };
    if !crate::bodies::multi(features) {
        return part.regenerate_ids(ids);
    }
    let mut all = Body::new();
    for s in part.regenerate_solids_ids(ids)? {
        all.append(&s.body);
    }
    Ok(all)
}

/// Whether `f` stands alone: a solid tool sized by numbers only, so taking it out or swapping
/// it with a like one changes nothing else.
fn plain(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Extrude {
            extent: Extent::Distance(_) | Extent::Symmetric(_) | Extent::TwoSided { .. },
            op,
            ..
        }
        | Feature::Revolve {
            extent: Turn::Angle(_),
            op,
            ..
        }
        | Feature::Ball { op, .. } => Some(*op),
        Feature::Hole(h) if matches!(h.end, End::Depth(_)) => Some(Combine::Cut),
        _ => None,
    }
}

fn scale_profile(p: &mut Profile, k: f64) -> f64 {
    let s = |q: &mut Point2| *q = Point2::new(q.x * k, q.y * k);
    let mut r: f64 = 0.0;
    for l in &mut p.loops {
        for (seg, _) in &mut l.segments {
            match seg {
                Segment::Line { a, b } => {
                    r = r.max(a.to_vec().length()).max(b.to_vec().length());
                    s(a);
                    s(b);
                }
                Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                    r = r.max(center.to_vec().length() + *radius);
                    s(center);
                    *radius *= k;
                }
            }
        }
    }
    r
}

/// A random edit of `part` above position `before` (the reference point); None if the dice
/// found nothing to edit. `big`: dimensions and moves well past small tweaks.
pub fn random_edit(part: &Part, before: usize, body: &Body, big: bool, r: &mut Rng) -> Option<Edit> {
    let fs = &part.features;
    let pick = r.below(before);
    let mag = if big {
        r.range(0.1, 0.5)
    } else {
        r.range(0.002, 0.02)
    };
    let sign = if r.bool() { 1.0 } else { -1.0 };
    match r.below(10) {
        0..=3 => resized(&fs[pick], 1.0).map(|_| Edit::Resize {
            feature: pick,
            by: 1.0 + sign * mag,
        }),
        4..=6 => {
            let dir = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
            let size = if big { 0.1 * body_box(body).diagonal() } else { 1.0 };
            let by = dir.normalize()? * (mag * size);
            moved(&fs[pick], by).map(|_| Edit::Move { feature: pick, by })
        }
        7 => {
            let at = 1 + r.below(before);
            Some(Edit::Insert {
                at,
                feature: Box::new(unrelated(body, r)),
            })
        }
        8 => deletable(fs, pick).then_some(Edit::Delete { at: pick }),
        _ => swappable(fs, pick, before).then_some(Edit::Swap { at: pick }),
    }
}

/// Whether feature `at` stands alone, no other one referring to it by position.
fn deletable(fs: &[Feature], at: usize) -> bool {
    let referred = fs
        .iter()
        .any(|f| feature_refs(&mut f.clone()).iter().any(|x| **x == at));
    at > 0 && !referred && plain(&fs[at]).is_some_and(|op| op != Combine::New)
}

/// Whether features `at` and `at + 1`, both above `before`, stand alone and combine alike.
fn swappable(fs: &[Feature], at: usize, before: usize) -> bool {
    at + 1 < before && plain(&fs[at]).is_some_and(|op| op != Combine::New && plain(&fs[at + 1]) == Some(op))
}

/// Every insert (of a work plane: it makes nothing, so it moves nothing), delete and swap
/// above each finishing feature of `corpus` and the part's end, each judged on every entity
/// of the part before it.
pub fn run_shifts(corpus: &[(String, Part)]) -> Vec<Case> {
    let mut out = Vec::new();
    for (name, part) in corpus {
        let fs = &part.features;
        let stops = (1..fs.len()).filter(|&i| !stored_refs(&fs[i]).is_empty());
        for before in stops.chain([fs.len()]) {
            let Ok(b0) = regen(&fs[..before], None) else {
                continue;
            };
            let mut edits: Vec<Edit> = (1..before)
                .map(|at| Edit::Insert {
                    at,
                    feature: Box::new(Feature::Plane(forge_math::Frame::WORLD)),
                })
                .collect();
            edits.extend(
                (1..before)
                    .filter(|&at| deletable(fs, at))
                    .map(|at| Edit::Delete { at }),
            );
            edits.extend(
                (0..before)
                    .filter(|&at| swappable(fs, at, before))
                    .map(|at| Edit::Swap { at }),
            );
            for edit in edits {
                if let Some(outcome) = check(part, before, &b0, &edit, 1, &[]) {
                    out.push(Case {
                        part: name.clone(),
                        before,
                        edit,
                        outcome,
                    });
                }
            }
        }
    }
    out
}

/// `per_stop` random inserts (`splitter`) above each finishing feature of `corpus` and the
/// part's end, each judged on every entity of the part before it; and how many edited parts
/// failed to build.
pub fn run_inserts(corpus: &[(String, Part)], seed: u64, per_stop: usize) -> (Vec<Case>, usize) {
    let mut r = Rng::new(seed);
    let (mut out, mut failed) = (Vec::new(), 0);
    for (name, part) in corpus {
        let fs = &part.features;
        let stops = (1..fs.len()).filter(|&i| !stored_refs(&fs[i]).is_empty());
        for before in stops.chain([fs.len()]) {
            let Ok(b0) = regen(&fs[..before], None) else {
                continue;
            };
            for _ in 0..per_stop {
                let edit = Edit::Insert {
                    at: 1 + r.below(before),
                    feature: Box::new(splitter(&b0, &mut r)),
                };
                match check(part, before, &b0, &edit, 1, &[]) {
                    Some(outcome) => out.push(Case {
                        part: name.clone(),
                        before,
                        edit,
                        outcome,
                    }),
                    None => failed += 1,
                }
            }
        }
    }
    (out, failed)
}

/// A random feature that splits faces of `body` somewhere: a hole, pocket or slot cut in from
/// one side of its box, or a round or square boss standing on that side.
fn splitter(body: &Body, r: &mut Rng) -> Feature {
    let b = body_box(body);
    let (lo, hi) = ([b.x.lo, b.y.lo, b.z.lo], [b.x.hi, b.y.hi, b.z.hi]);
    let mut p = [0.0; 3];
    for i in 0..3 {
        p[i] = r.range(lo[i], hi[i]);
    }
    let axis = r.below(3);
    let out = if r.bool() { 1.0 } else { -1.0 };
    let mut n = [0.0; 3];
    n[axis] = out;
    let n = Vec3::new(n[0], n[1], n[2]);
    let side = if out > 0.0 { hi[axis] } else { lo[axis] };
    let x = [Vec3::X, Vec3::Y, Vec3::Z][(axis + 1) % 3];
    let kind = r.below(4);
    let boss = kind == 3;
    p[axis] = if boss { side - 0.5 * out } else { side + out };
    let z = if boss { n } else { -n };
    let plane = forge_math::Frame::from_zx(Point3::new(p[0], p[1], p[2]), z.normalize().expect("an axis"), x)
        .expect("a frame");
    let (w, h) = (r.range(1.0, 4.0), r.range(1.0, 2.0));
    let shape = match kind {
        0 => ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::ORIGIN,
                radius: r.range(0.5, 2.0),
                ccw: true,
            },
            0,
        )]),
        1 => Profile::rectangle(Point2::new(-w / 2.0, -h), w, 2.0 * h, 0),
        2 => Profile::rounded_rectangle(Point2::new(-2.0 * w, -h / 2.0), 4.0 * w, h, 0.45 * h, 0),
        _ if r.bool() => Profile::circle(Point2::ORIGIN, w / 2.0, 0),
        _ => Profile::rectangle(Point2::new(-w / 2.0, -w / 2.0), w, w, 0),
    };
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![shape],
        },
        extent: Extent::Distance(if boss {
            0.5 + r.range(1.0, 3.0)
        } else {
            1.0 + r.range(0.5, 3.0)
        }),
        op: if boss { Combine::Add } else { Combine::Cut },
    }
}

/// A small hole drilled straight down somewhere over the part.
fn unrelated(body: &Body, r: &mut Rng) -> Feature {
    let b = body_box(body);
    let x = r.range(b.x.lo, b.x.hi);
    let y = r.range(b.y.lo, b.y.hi);
    let d = r.range(0.5, 2.0);
    Feature::Extrude {
        profile: Profile {
            plane: forge_math::Frame::WORLD
                .with_origin(Point3::new(x, y, b.z.hi + 1.0))
                .flipped(),
            loops: vec![ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::ORIGIN,
                    radius: d,
                    ccw: true,
                },
                0,
            )])],
        },
        extent: Extent::Distance(1.0 + r.range(0.5, 3.0)),
        op: Combine::Cut,
    }
}

fn body_box(b: &Body) -> Box3 {
    b.face_ids().fold(Box3::EMPTY, |acc, f| acc.hull(&b.face_box(f)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Face,
    Edge,
    Vertex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ent {
    Face(FaceId),
    Edge(EdgeId),
    Vertex(VertexId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    /// An entity the edit did not touch lost its name.
    Renamed,
    /// An entity the edit moved or reshaped, but that survives as itself, lost its name.
    Lost,
    /// A name now names a different entity: the worst case.
    Wrong,
    /// The edit changed the entity past tracking (a feature's extent flipped, an edge's end ran
    /// along it), and its name is now on something else that is not a piece of it: perhaps the
    /// same role on new geometry, perhaps another entity. Needs a look.
    Jumped,
    /// Two entities of one part share a name.
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub class: Class,
    pub kind: Kind,
    pub name: Name,
    /// Where the entity was (a point of it), and where the name now points, if anywhere.
    pub was: Point3,
    pub now: Option<Point3>,
    /// The entity's name after the edit, where geometry found it.
    pub truth: Option<Name>,
}

/// How the references of one edited history fared.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// Names that followed their entity.
    pub kept: usize,
    /// Names whose entity truly went, or split, and that name nothing.
    pub gone: usize,
    /// Names now on a piece of what they named (it split, or a sliver of it survived).
    pub piece: usize,
    /// References the geometry could not judge (ambiguous, or the tracking lost them).
    pub unjudged: usize,
    /// Whether the edit changed the part's topology: its counts of faces, edges and vertices.
    /// Where it did not, every name must survive.
    pub reshaped: bool,
    pub findings: Vec<Finding>,
}

/// Each live entity of `b`, with its name.
fn ents(b: &Body) -> Vec<(Ent, Kind, Name)> {
    let mut out: Vec<_> = b
        .face_ids()
        .map(|f| (Ent::Face(f), Kind::Face, b.face(f).name))
        .collect();
    out.extend(b.edge_ids().map(|e| (Ent::Edge(e), Kind::Edge, b.edge(e).name)));
    let mut used = vec![false; b.vertices.len()];
    for e in b.edge_ids() {
        used[b.edge(e).start.index()] = true;
        used[b.edge(e).end.index()] = true;
    }
    out.extend(
        b.vertex_ids()
            .filter(|v| used[v.index()])
            .map(|v| (Ent::Vertex(v), Kind::Vertex, b.vertex(v).name)),
    );
    out
}

/// What an entity looks like: its kind, its surface or curve kind, and points that pin it down.
struct Geo {
    sub: u8,
    pts: Vec<Point3>,
}

fn geo(b: &Body, e: Ent) -> Geo {
    let corners = |x: Box3| {
        [
            Point3::new(x.x.lo, x.y.lo, x.z.lo),
            Point3::new(x.x.hi, x.y.hi, x.z.hi),
        ]
    };
    match e {
        Ent::Face(f) => {
            let face = b.face(f);
            let mut pts = corners(b.face_box(f)).to_vec();
            let (mut sum, mut n) = (Vec3::ZERO, 0.0f64);
            for &l in &face.loops {
                for c in b.lp(l).coedges() {
                    for p in b.edge_points(b.coedge(c).edge, 4) {
                        sum += p.to_vec();
                        n += 1.0;
                    }
                }
            }
            pts.push(Point3::ORIGIN + sum * (1.0 / n.max(1.0)));
            let sub = match &face.surface {
                forge_geom::Surface::Plane(p) => {
                    // which way it faces, as a point a unit off
                    let z = if face.same_sense {
                        p.frame.z()
                    } else {
                        -p.frame.z()
                    };
                    pts.push(Point3::ORIGIN + z.get());
                    0
                }
                forge_geom::Surface::Cylinder(_) => 1,
                forge_geom::Surface::Cone(_) => 2,
                forge_geom::Surface::Sphere(_) => 3,
                forge_geom::Surface::Torus(_) => 4,
                forge_geom::Surface::Procedural(_) => 5,
            };
            Geo { sub, pts }
        }
        Ent::Edge(x) => {
            let edge = b.edge(x);
            let s = b.vertex(edge.start).point;
            let t = b.vertex(edge.end).point;
            // the ends in a fixed order, so a reversed edge looks the same
            let (s, t) = if (s.x, s.y, s.z) <= (t.x, t.y, t.z) {
                (s, t)
            } else {
                (t, s)
            };
            let mut pts = vec![s, t, edge.curve.eval(edge.range.mid())];
            pts.extend(corners(b.edge_box(x)));
            let sub = match edge.curve {
                forge_geom::Curve::Line(_) => 10,
                forge_geom::Curve::Circle(_) => 11,
                forge_geom::Curve::Ellipse(_) => 12,
                _ => 13,
            };
            Geo { sub, pts }
        }
        Ent::Vertex(v) => Geo {
            sub: 20,
            pts: vec![b.vertex(v).point],
        },
    }
}

fn apart(a: &Geo, b: &Geo) -> f64 {
    if a.sub != b.sub || a.pts.len() != b.pts.len() {
        return f64::INFINITY;
    }
    let d = a
        .pts
        .iter()
        .zip(&b.pts)
        .fold(0.0f64, |m, (p, q)| m.max(p.distance(*q)));
    if a.sub >= 10 && a.sub < 20 {
        // ends that swapped order (nearly level) still match
        let swapped = a.pts[0].distance(b.pts[1]).max(a.pts[1].distance(b.pts[0]));
        let rest = a.pts[2..]
            .iter()
            .zip(&b.pts[2..])
            .fold(0.0f64, |m, (p, q)| m.max(p.distance(*q)));
        return d.min(swapped.max(rest));
    }
    d
}

/// A point of entity `e`.
fn point(b: &Body, e: Ent) -> Point3 {
    match e {
        Ent::Face(f) => b.face_box(f).center(),
        Ent::Edge(x) => b.edge(x).curve.eval(b.edge(x).range.mid()),
        Ent::Vertex(v) => b.vertex(v).point,
    }
}

/// Whether `r` (of `nb`) is a piece of what `e` (of `ob`) was, or `e` a piece of it, within
/// `tol`: the same carrier, overlapping.
fn piece_of(ob: &Body, e: Ent, nb: &Body, r: Ent, tol: f64) -> bool {
    match (e, r) {
        (Ent::Face(f), Ent::Face(g)) => {
            let s = &ob.face(f).surface;
            ob.face_box(f).grow(tol).overlaps(&nb.face_box(g))
                && nb.face(g).loops.iter().all(|&l| {
                    nb.lp(l).coedges().all(|c| {
                        nb.edge_points(nb.coedge(c).edge, 2)
                            .iter()
                            .all(|p| s.distance(*p) <= tol.max(1e-6))
                    })
                })
        }
        (Ent::Edge(x), Ent::Edge(y)) => {
            // on one line, overlapping: trimmed differently
            if let (forge_geom::Curve::Line(l), true) = (
                &ob.edge(x).curve,
                ob.edge_box(x).grow(tol).overlaps(&nb.edge_box(y)),
            ) {
                let off = |p: &Point3| {
                    let d = *p - l.origin;
                    (d - l.dir.get() * d.dot(l.dir.get())).length()
                };
                if nb.edge_points(y, 2).iter().all(|p| off(p) <= tol.max(1e-6)) {
                    return true;
                }
            }
            let within = |a: &Body, x: EdgeId, b: &Body, y: EdgeId| {
                let (c, range) = (&a.edge(x).curve, a.edge(x).range);
                b.edge_points(y, 4)
                    .iter()
                    .all(|p| c.eval(c.closest(*p, range)).distance(*p) <= tol.max(1e-6))
            };
            within(ob, x, nb, y) || within(nb, y, ob, x)
        }
        _ => false,
    }
}

/// How the references `refs` (names of `b0`'s entities; all of them if empty) fare when `part`
/// (whose first `before` features made `b0`) is edited by `edit` in `steps` steps.
pub fn check(
    part: &Part,
    before: usize,
    b0: &Body,
    edit: &Edit,
    steps: usize,
    refs: &[(Kind, Name)],
) -> Option<Outcome> {
    let continuous = matches!(edit, Edit::Resize { .. } | Edit::Move { .. });
    let steps = if continuous { steps.max(1) } else { 1 };
    let prefix = Part {
        features: part.features[..before].to_vec(),
    };
    let all0 = ents(b0);
    let picked: Vec<(Ent, Kind, Name)> = if refs.is_empty() {
        all0.clone()
    } else {
        refs.iter()
            .filter_map(|&(k, n)| all0.iter().find(|x| x.1 == k && x.2 == n).copied())
            .collect()
    };
    let mut bodies = vec![b0.clone()];
    let mut reaches = vec![0.0];
    for s in 1..=steps {
        let (p, ids, reach) = edit.apply(&prefix, s as f64 / steps as f64)?;
        let b = regen(&p.features, Some(&ids)).ok()?;
        // an edit that parts the solid in two (or joins it) leaves no part to name things on
        if b.shells.len() != b0.shells.len() {
            return None;
        }
        bodies.push(b);
        reaches.push(reach);
    }
    let end = &bodies[steps];
    let tables: Vec<Vec<(Ent, Kind, Geo)>> = bodies
        .iter()
        .map(|b| ents(b).into_iter().map(|x| (x.0, x.1, geo(b, x.0))).collect())
        .collect();
    let mut out = Outcome::default();
    let dup = |b: &Body, all: &[(Ent, Kind, Name)], out: &mut Outcome| {
        let mut names: Vec<(Kind, Name)> = all.iter().map(|x| (x.1, x.2)).collect();
        names.sort();
        for w in names.windows(2) {
            if w[0] == w[1] && picked.iter().any(|p| (p.1, p.2) == w[0]) {
                let e = all.iter().find(|x| (x.1, x.2) == w[0]).expect("listed");
                out.findings.push(Finding {
                    class: Class::Duplicate,
                    kind: w[0].0,
                    name: w[0].1,
                    was: point(b, e.0),
                    now: None,
                    truth: None,
                });
            }
        }
    };
    dup(b0, &all0, &mut out);
    let all1 = ents(end);
    dup(end, &all1, &mut out);
    let count = |all: &[(Ent, Kind, Name)], k| all.iter().filter(|x| x.1 == k).count();
    out.reshaped = [Kind::Face, Kind::Edge, Kind::Vertex]
        .iter()
        .any(|&k| bodies.iter().any(|b| count(&ents(b), k) != count(&all0, k)));
    for &(e0, kind, name) in &picked {
        // follow the entity step by step: (step last seen on, entity there), moved at all
        let (mut at, mut e, mut moved) = (0, e0, false);
        let mut ambiguous = false;
        while at < steps {
            let tol = 3.0 * (reaches[at + 1] - reaches[at]) + 1e-6;
            let g = geo(&bodies[at], e);
            let near: Vec<(Ent, f64)> = tables[at + 1]
                .iter()
                .filter(|x| x.1 == kind)
                .map(|x| (x.0, apart(&g, &x.2)))
                .filter(|x| x.1 <= tol)
                .collect();
            match near[..] {
                [(n, d)] => {
                    moved |= d > 1e-9;
                    e = n;
                    at += 1;
                }
                _ => {
                    ambiguous = near.len() > 1;
                    break;
                }
            }
        }
        let hits: Vec<Ent> = all1
            .iter()
            .filter(|x| x.1 == kind && x.2 == name)
            .map(|x| x.0)
            .collect();
        let r = hits.first().copied();
        // an entity another one of its part lies on exactly (bodies touching) is not told apart
        let twin = || {
            let g = geo(b0, e0);
            tables[0]
                .iter()
                .any(|x| x.1 == kind && x.0 != e0 && apart(&g, &x.2) <= 1e-9)
        };
        let tracked = at == steps;
        let twinned = tracked && !hits.contains(&e) && twin();
        let truth = tracked
            .then(|| all1.iter().find(|x| x.0 == e).map(|x| x.2))
            .flatten();
        let mut finding = |class| {
            out.findings.push(Finding {
                class,
                kind,
                name,
                was: point(b0, e0),
                now: r.map(|r| point(end, r)),
                truth,
            })
        };
        match r {
            _ if twinned => out.unjudged += 1,
            _ if tracked && hits.contains(&e) => out.kept += 1,
            None if tracked && moved => finding(Class::Lost),
            None if tracked => finding(Class::Renamed),
            Some(r) if tracked => {
                if piece_of(b0, e0, end, r, 3.0 * reaches[steps] + 1e-6) {
                    out.piece += 1;
                } else {
                    finding(Class::Wrong)
                }
            }
            _ if ambiguous => out.unjudged += 1,
            None => out.gone += 1,
            Some(r) => {
                // a piece of what it was last seen as, or something else
                let tol = 3.0 * (reaches[steps] - reaches[at]) + 1e-6;
                if piece_of(&bodies[at], e, end, r, tol) {
                    out.piece += 1;
                } else {
                    finding(Class::Jumped)
                }
            }
        }
    }
    Some(out)
}

/// Histories to edit: the suite parts, random histories, and the fillet and finishing corpora.
pub fn corpus() -> Vec<(String, Part)> {
    let mut out: Vec<(String, Part)> = crate::suite::parts()
        .into_iter()
        .filter(|(n, _)| !matches!(*n, "plate" | "hole-grid" | "shaft" | "coil" | "torture-2"))
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    out.extend((1..=8).map(|s| (format!("fuzz{s}"), crate::suite::fuzz(s, 12))));
    out.extend(
        crate::fillets::cases()
            .into_iter()
            .map(|(n, p, _)| (format!("fillet:{n}"), p)),
    );
    out.extend(
        crate::finishing::parts()
            .into_iter()
            .map(|(n, p, _)| (format!("finish:{n}"), p)),
    );
    out.extend((1..=8).map(|s| (format!("finish-fuzz{s}"), crate::finishing::fuzz(s))));
    out
}

/// Histories whose names come from features a shifting edit leaves where they were but at
/// another place: finishing features, face fillets, full rounds, several bodies, a work plane.
pub fn shift_corpus() -> Vec<(String, Part)> {
    let mut out: Vec<(String, Part)> = crate::finishing::parts()
        .into_iter()
        .map(|(n, p, _)| (format!("finish:{n}"), p))
        .collect();
    out.extend((1..=8).map(|s| (format!("finish-fuzz{s}"), crate::finishing::fuzz(s))));
    out.extend(
        crate::fillets::variant_cases()
            .into_iter()
            .map(|(n, p, _)| (format!("variant:{n}"), p)),
    );
    out.extend(body_parts());
    out.push(("work".into(), work_part()));
    out
}

/// Parts of several bodies: features on chosen bodies, a chamfer and a shell found by their
/// edges and faces, splits, a moved copy, a split face and a kept combine.
pub fn body_parts() -> Vec<(String, Part)> {
    let block = |x: f64, y: f64, z: f64, w: f64, h: f64, op| Feature::Extrude {
        profile: Profile {
            plane: forge_math::Frame::WORLD.with_origin(Point3::new(x, y, z)),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, 10.0, 0)],
        },
        extent: Extent::Distance(h),
        op,
    };
    let rod = |x: f64, y: f64, z: f64, r: f64, h: f64, op| Feature::Extrude {
        profile: Profile {
            plane: forge_math::Frame::WORLD.with_origin(Point3::new(x, y, z)),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: Extent::Distance(h),
        op,
    };
    let body = |b| Feature::Body(Box::new(b));
    let on = |ids: &[usize], feature| {
        body(BodyFeature::In {
            bodies: Pick::Some(ids.to_vec()),
            feature,
        })
    };
    let level = |z: f64| Cutter::Plane(forge_math::Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z)));
    // a finishing feature on what `fs` make, naming what lies nearest the points
    let named = |fs: &[Feature], at: &[Point3], make: &dyn Fn(Vec<Name>) -> Feature| {
        let b = regen(fs, None).expect("the part builds");
        make(at.iter().map(|&p| crate::fillets::edge_near(&b, p)).collect())
    };
    let faced = |fs: &[Feature], at: &[Point3], make: &dyn Fn(Vec<Name>) -> Feature| {
        let b = regen(fs, None).expect("the part builds");
        make(at.iter().map(|&p| crate::fillets::face_near(&b, p)).collect())
    };
    let chamfer = |edges: Vec<Name>| Feature::Chamfer {
        edges,
        size: ChamferSize::Equal(1.0),
    };
    let shell = |open: Vec<Name>| Feature::Shell {
        thickness: 1.0,
        outward: false,
        open,
    };
    let mut out = Vec::new();

    let mut f = vec![
        block(0.0, 0.0, 0.0, 20.0, 10.0, Combine::New),
        block(30.0, 0.0, 0.0, 10.0, 10.0, Combine::New),
        on(&[0], rod(5.0, 5.0, -1.0, 2.0, 12.0, Combine::Cut)),
        rod(35.0, 5.0, 10.0, 2.0, 5.0, Combine::Add),
        on(&[0, 1], block(-5.0, -8.0, 8.0, 50.0, 3.0, Combine::Cut)),
    ];
    f.push(named(&f, &[Point3::new(15.0, 10.0, 10.0)], &chamfer));
    f.push(on(
        &[1],
        body(BodyFeature::Split {
            by: level(4.0),
            keep: Keep::Both,
        }),
    ));
    f.push(faced(&f, &[Point3::new(20.0, 5.0, 5.0)], &|n| on(&[0], shell(n))));
    out.push(("bodies:picks".to_string(), Part { features: f }));

    let mut f = vec![
        block(0.0, 0.0, 0.0, 20.0, 10.0, Combine::New),
        on(
            &[0],
            body(BodyFeature::Move {
                steps: vec![Step::Translate(forge_math::Vec3::new(0.0, 20.0, 0.0))],
                copy: true,
            }),
        ),
        on(&[0], rod(10.0, 5.0, -1.0, 3.0, 12.0, Combine::Cut)),
        block(15.0, 25.0, 5.0, 10.0, 10.0, Combine::New),
    ];
    f.push(faced(&f, &[Point3::new(10.0, 0.0, 8.0)], &|faces| {
        body(BodyFeature::SplitFace {
            faces,
            by: Cutter::Plane(
                forge_math::Frame::from_zx(
                    Point3::new(5.0, 0.0, 0.0),
                    forge_math::Vec3::X.normalize().expect("x"),
                    forge_math::Vec3::Y,
                )
                .expect("a frame"),
            ),
        })
    }));
    f.push(body(BodyFeature::Combine {
        op: Combine::Cut,
        target: 1,
        tools: vec![3],
        keep: true,
    }));
    f.push(named(&f, &[Point3::new(10.0, 30.0, 10.0)], &|e| {
        on(&[1], chamfer(e))
    }));
    out.push(("bodies:copy-combine".to_string(), Part { features: f }));
    out
}

/// A block and a boss, a work plane off the block's top, and a peg up to the plane.
pub fn work_part() -> Part {
    let ex = |o: Point3, l: ProfileLoop, extent, op| Feature::Extrude {
        profile: Profile {
            plane: forge_math::Frame::WORLD.with_origin(o),
            loops: vec![l],
        },
        extent,
        op,
    };
    let top = Ref {
        feature: 0,
        name: Some(Name::feature(1).with_str("end")),
    };
    Part {
        features: vec![
            ex(
                Point3::ORIGIN,
                Profile::rectangle(Point2::ORIGIN, 40.0, 20.0, 0),
                Extent::Distance(10.0),
                Combine::New,
            ),
            ex(
                Point3::new(30.0, 10.0, 10.0),
                Profile::circle(Point2::ORIGIN, 4.0, 0),
                Extent::Distance(15.0),
                Combine::Add,
            ),
            Feature::Work(Work::Plane(PlaneBy::Offset(top, 6.0))),
            ex(
                Point3::new(5.0, 5.0, 10.0),
                Profile::circle(Point2::ORIGIN, 3.0, 0),
                Extent::To {
                    target: Target::Plane(2),
                    offset: 0.0,
                },
                Combine::Add,
            ),
        ],
    }
}

/// One edited history and how its references fared.
#[derive(Clone, Debug)]
pub struct Case {
    pub part: String,
    /// The references were taken on the part after this many features.
    pub before: usize,
    pub edit: Edit,
    pub outcome: Outcome,
}

/// `cases` random edits of `corpus` histories, each judged on up to `refs` sampled references
/// plus the names a finishing feature at the reference point holds. `big`: large edits, made
/// in `steps` steps.
pub fn run(
    corpus: &[(String, Part)],
    seed: u64,
    cases: usize,
    big: bool,
    steps: usize,
    refs: usize,
) -> Vec<Case> {
    let mut r = Rng::new(seed);
    let mut out = Vec::with_capacity(cases);
    let mut tries = 0;
    while out.len() < cases && tries < 20 * cases {
        tries += 1;
        let (name, part) = &corpus[r.below(corpus.len())];
        let fs = &part.features;
        let stops: Vec<usize> = (1..fs.len())
            .filter(|&i| !stored_refs(&fs[i]).is_empty())
            .collect();
        let before = if !stops.is_empty() && r.bool() {
            stops[r.below(stops.len())]
        } else {
            fs.len()
        };
        let Ok(b0) = regen(&fs[..before], None) else {
            continue;
        };
        let Some(edit) = random_edit(part, before, &b0, big, &mut r) else {
            continue;
        };
        let mut want: Vec<(Kind, Name)> = fs.get(before).map(stored_refs).unwrap_or_default();
        let all = ents(&b0);
        for _ in 0..refs.min(all.len()) {
            let e = all[r.below(all.len())];
            want.push((e.1, e.2));
        }
        want.sort();
        want.dedup();
        if let Some(outcome) = check(part, before, &b0, &edit, steps, &want) {
            out.push(Case {
                part: name.clone(),
                before,
                edit,
                outcome,
            });
        }
    }
    out
}
