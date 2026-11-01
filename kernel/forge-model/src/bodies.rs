//! Parts of several solid bodies (docs/multibody.md): features on chosen bodies, and features
//! that combine, split and move bodies. A history that never makes a second body goes through
//! lib.rs's single-body regens untouched.

use crate::extent::Target;
use crate::pattern::{Motion, PatternError, corners, moved};
use crate::{
    Combine, Edit, Feature, History, RegenError, SHORT_RUN, Tool, cut_root, modifies, modify, op_of,
    part_shape, starts, tools,
};
use forge_bool::{BoolError, Op, Shape, boolean, boolean_into, nary_with};
use forge_geom::{Cylinder, Plane, Surface};
use forge_math::{Box3, Frame, Point2, Point3, Vec3, tol};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::{Body, Name};
use std::borrow::Cow;
use std::collections::HashSet;

/// One body of a part: the feature (its place in the history) that made it, and its solid.
#[derive(Clone, Debug, PartialEq)]
pub struct Solid {
    pub id: usize,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BodyFeature {
    /// `feature` on these bodies, not the current one.
    In { bodies: Pick, feature: Feature },
    /// Joins (Add), cuts (Cut) or intersects (Keep) the tool bodies into the target, in order;
    /// `keep` leaves the tools as they were, else they are used up.
    Combine {
        op: Combine,
        target: usize,
        tools: Vec<usize>,
        keep: bool,
    },
    /// Cuts each body in two along a surface, keeping one side or both.
    Split { by: Cutter, keep: Keep },
    /// Splits faces where a surface crosses them; the solid stays the same.
    SplitFace { faces: Vec<Name>, by: Cutter },
    /// Moves bodies rigidly, the steps in order, or (`copy`) makes a moved copy of one body.
    Move { steps: Vec<Step>, copy: bool },
}

/// Which bodies a feature acts on, by the features that made them.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    All,
    Some(Vec<usize>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cutter {
    /// A work plane, or an earlier feature's face's surface.
    Target(Target),
    /// A frame's xy.
    Plane(Frame),
    /// A sketch curve swept along its frame's z: a line's plane, an arc's or circle's cylinder.
    Curve(Frame, Segment),
}

/// The sides of a split kept: `Plus` where the cutter's normal points (outside a cylinder or
/// sphere). Keeping both, the `Plus` piece is a new body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    Plus,
    Minus,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Translate(Vec3),
    /// About the line through `origin` along `axis`, counter-clockwise looking down it.
    Rotate {
        origin: Point3,
        axis: Vec3,
        angle: f64,
    },
    /// World to this frame.
    Frame(Frame),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BodyError {
    /// No body made by this feature is left.
    NoBody {
        id: usize,
    },
    /// The feature needs exactly one body (a copy, a split keeping both sides).
    NotOne,
    /// A body combined with itself.
    SameBody {
        id: usize,
    },
    /// A split leaves nothing on a side kept: the cutter misses the body.
    Misses,
    /// A face to split is on no body.
    NoFace {
        name: Name,
    },
    /// The cutter does not cross the face.
    NotCrossed {
        name: Name,
    },
    Unsupported(&'static str),
}

/// Whether a history makes or works on more than one body.
pub fn multi(features: &[Feature]) -> bool {
    features.iter().any(|f| matches!(f, Feature::Body(_)))
        || features
            .iter()
            .filter(|f| op(f) == Some(Combine::New))
            .nth(1)
            .is_some()
}

fn op(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } => Some(*op),
        Feature::Edit(Edit::Thicken { op, .. }) => Some(*op),
        _ => crate::swept::op(f),
    }
}

/// The history with `In` unwrapped (borrowed when there is none), and each feature's pick.
pub(crate) fn strip(features: &[Feature]) -> (Cow<'_, [Feature]>, Vec<Option<&Pick>>) {
    let wrapped = |f: &Feature| matches!(f, Feature::Body(b) if matches!(**b, BodyFeature::In { .. }));
    if !features.iter().any(wrapped) {
        return (Cow::Borrowed(features), vec![]);
    }
    let mut picks = Vec::with_capacity(features.len());
    let plain = features
        .iter()
        .map(|f| match f {
            Feature::Body(b) => match &**b {
                BodyFeature::In { bodies, feature } => {
                    picks.push(Some(bodies));
                    feature.clone()
                }
                _ => {
                    picks.push(None);
                    f.clone()
                }
            },
            _ => {
                picks.push(None);
                f.clone()
            }
        })
        .collect();
    (Cow::Owned(plain), picks)
}

/// The one body, or how many there are.
pub(crate) fn one(mut solids: Vec<Solid>) -> Result<Body, RegenError> {
    match solids.len() {
        1 => Ok(solids.pop().expect("one").body),
        count => Err(RegenError::Bodies { count }),
    }
}

/// The rigid motion the steps make, applied in order.
pub(crate) fn motion(steps: &[Step]) -> Motion {
    let mut m = Frame::WORLD;
    for s in steps {
        let f = match *s {
            Step::Translate(v) => Frame::WORLD.with_origin(Point3::ORIGIN + v),
            Step::Rotate { origin, axis, angle } => match axis.normalize() {
                Some(z) => {
                    let at = Frame::from_z(origin, z);
                    at.rotated_about_z(angle).compose(&at.inverse())
                }
                None => Frame::WORLD,
            },
            Step::Frame(f) => f,
        };
        m = f.compose(&m);
    }
    Motion::Rigid(m)
}

/// Feature `i` of a history applied to the bodies features `..i` left: `regenerate_solids`
/// one feature at a time. Only `features[..=i]` is read.
pub fn step_solids(features: History<'_>, i: usize, solids: Vec<Solid>) -> Result<Vec<Solid>, RegenError> {
    let solids = step_unchecked(features, i, solids)?;
    let open = |s: &[Solid]| s.iter().find_map(|s| forge_topo::check::open_at(&s.body));
    let Some(what) = open(&solids) else {
        return Ok(solids);
    };
    // (sides of a knife edge within tolerance can leave the body open as well as refused)
    let again = if multi(&features.upto(i + 1)) {
        all_blunted(features, i, i + 1)
    } else {
        let id = features.upto(i + 1).iter().rposition(starts).unwrap_or(0);
        crate::rebuilt_blunted(features, i)
            .map(|r| r.map(|b| b.map(|body| Solid { id, body }).into_iter().collect()))
    };
    match again {
        Some(Ok(s)) if open(&s).is_none() => Ok(s),
        _ => Err(RegenError::NotClosed { feature: i, what }),
    }
}

fn step_unchecked(features: History<'_>, i: usize, mut solids: Vec<Solid>) -> Result<Vec<Solid>, RegenError> {
    let upto = features.upto(i + 1);
    if !multi(&upto) {
        let id = upto.iter().rposition(starts).unwrap_or(0);
        let body = solids.pop().map(|s| s.body);
        // (a caller stepping keeps the part so far itself: a copy only for what may need a retry, and
        // no nesting: at worst the part once more with all its knife edges blunted)
        let tried = match crate::blunted(features, i) {
            None => crate::step_plain(features, i, body),
            Some(fs) => match crate::step_plain(features, i, body.clone()) {
                Err(e) if crate::lost_its_way(&e) => {
                    crate::step_plain(History::new(&fs, features.ids), i, body).map_err(|_| e)
                }
                r => r,
            },
        };
        let one = match tried {
            Err(e) if crate::lost_its_way(&e) => match crate::rebuilt_blunted(features, i) {
                Some(Ok(b)) => b,
                _ => return Err(e),
            },
            r => r?,
        };
        return Ok(one.map(|body| Solid { id, body }).into_iter().collect());
    }
    let (plain, picks) = strip(&upto);
    let plain = upto.on(&plain);
    let tried = match crate::blunted(plain, i) {
        None => apply_once(plain, &picks, i, solids),
        Some(fs) => match apply_once(plain, &picks, i, solids.clone()) {
            Err(e) if crate::lost_its_way(&e) => {
                apply_once(History::new(&fs, plain.ids), &picks, i, solids).map_err(|_| e)
            }
            r => r,
        },
    };
    match tried {
        // (a tip already in the part may be what this trips on: at worst the part once more)
        Err(e) if crate::lost_its_way(&e) => match all_blunted(features, i, i + 1) {
            Some(Ok(s)) => Ok(s),
            _ => Err(e),
        },
        r => r,
    }
}

pub(crate) fn sequential(features: History<'_>) -> Result<Vec<Solid>, RegenError> {
    let (plain, picks) = strip(&features);
    let plain = features.on(&plain);
    let mut solids = Vec::new();
    for i in 0..plain.len() {
        solids = match apply_once(plain, &picks, i, solids) {
            Err(e) if crate::lost_its_way(&e) => {
                return match careful(plain, &picks, 0, plain.len(), Vec::new()) {
                    Ok(s) => finish(s),
                    Err(_) => match all_blunted(features, features.len() - 1, features.len()) {
                        Some(Ok(s)) => finish(s),
                        _ => Err(e),
                    },
                };
            }
            r => r?,
        };
    }
    finish(solids)
}

fn finish(mut solids: Vec<Solid>) -> Result<Vec<Solid>, RegenError> {
    if solids.is_empty() {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    for s in &mut solids {
        s.body = crate::finish(std::mem::take(&mut s.body));
    }
    Ok(solids)
}

fn pick<'a>(picks: &[Option<&'a Pick>], i: usize) -> Option<&'a Pick> {
    picks.get(i).copied().flatten()
}

/// The first `to` features with every knife edge up to `last` blunted, if there are any.
fn all_blunted(features: History<'_>, last: usize, to: usize) -> Option<Result<Vec<Solid>, RegenError>> {
    let all = crate::all_blunted(features, last)?;
    let h = History::new(&all, features.ids).upto(to);
    let (plain, picks) = strip(&h);
    let plain = h.on(&plain);
    Some((0..to).try_fold(Vec::new(), |s, k| apply_once(plain, &picks, k, s)))
}

/// Features `from..to`, each extrude with knife edges tried plain and then blunted, for which a copy
/// is kept. Never nested: at most two tries of each feature, once.
fn careful(
    plain: History<'_>,
    picks: &[Option<&Pick>],
    from: usize,
    to: usize,
    mut solids: Vec<Solid>,
) -> Result<Vec<Solid>, RegenError> {
    for i in from..to {
        solids = match crate::blunted(plain, i) {
            None => apply_once(plain, picks, i, solids)?,
            Some(fs) => match apply_once(plain, picks, i, solids.clone()) {
                Err(e) if crate::lost_its_way(&e) => {
                    apply_once(History::new(&fs, plain.ids), picks, i, solids).map_err(|_| e)?
                }
                r => r?,
            },
        };
    }
    Ok(solids)
}

fn apply_once(
    plain: History<'_>,
    picks: &[Option<&Pick>],
    i: usize,
    mut solids: Vec<Solid>,
) -> Result<Vec<Solid>, RegenError> {
    let f = &plain[i];
    let p = pick(picks, i);
    match f {
        Feature::Plane(_) => return Ok(solids),
        Feature::Work(_) => {
            crate::work::datum(plain, i)?;
            return Ok(solids);
        }
        Feature::Body(b) => {
            body_feature(plain, i, b, p, &mut solids)?;
            return Ok(solids);
        }
        _ if modifies(f) => {
            modify_on(plain, i, p, &mut solids, 1)?;
            return Ok(solids);
        }
        _ => {}
    }
    let ts = tools(plain, i, true)?;
    // parameters and sketches make nothing
    if ts.is_empty() {
        return Ok(solids);
    }
    if starts(f) {
        let b = ts.into_iter().next().expect("one tool").0;
        crate::no_pinch(plain, i, &b)?;
        solids.push(Solid { id: i, body: b });
        return Ok(solids);
    }
    let mut gone: Vec<usize> = Vec::new();
    for (n, k) in targets(&ids(&solids), p, i)?.into_iter().enumerate() {
        let salt = (n > 0).then(|| plain.id(solids[k].id));
        let body = std::mem::take(&mut solids[k].body);
        match add_tools(body, &ts, (i, plain.id(i)), salt) {
            // a cut taking one whole body deletes it (`Model::removed_bodies` says so)
            Err(RegenError::Empty { .. }) => gone.push(k),
            r => {
                solids[k].body = r?;
                crate::no_pinch(plain, i, &solids[k].body)?;
            }
        }
    }
    if !gone.is_empty() {
        let mut at = 0;
        solids.retain(|_| {
            at += 1;
            !gone.contains(&(at - 1))
        });
        // taking every body leaves no part
        if solids.is_empty() {
            return Err(RegenError::Empty { feature: i });
        }
    }
    Ok(solids)
}

fn ids(solids: &[Solid]) -> Vec<usize> {
    solids.iter().map(|s| s.id).collect()
}

/// The bodies (positions in `ids`) a feature picks, or the current one: the last made.
fn targets(ids: &[usize], p: Option<&Pick>, i: usize) -> Result<Vec<usize>, RegenError> {
    let find = |id: usize| {
        ids.iter().position(|&x| x == id).ok_or(RegenError::Body {
            feature: i,
            error: BodyError::NoBody { id },
        })
    };
    match p {
        Some(Pick::All) => Ok((0..ids.len()).collect()),
        Some(Pick::Some(want)) => want.iter().map(|&id| find(id)).collect(),
        None if ids.is_empty() => Err(RegenError::NothingToCombine { feature: i }),
        None => Ok(vec![ids.len() - 1]),
    }
}

/// The names a feature works on (edges, faces), which say which bodies it is on.
fn names(f: &Feature) -> Vec<Name> {
    match f {
        Feature::Fillet {
            edges,
            starts,
            setbacks,
        } => edges
            .iter()
            .map(|e| e.0)
            .chain(starts.iter().flat_map(|s| [s.0, s.1]))
            .chain(setbacks.iter().flat_map(|s| [s.0].into_iter().chain(s.1)))
            .collect(),
        Feature::FaceFillet { first, second, .. } => first.iter().chain(second).copied().collect(),
        Feature::FullRound { side, middle, other } => vec![*side, *middle, *other],
        Feature::Chamfer { edges, .. } => edges.clone(),
        Feature::Shell { open, .. } => open.clone(),
        Feature::Draft { faces, .. } => faces.clone(),
        Feature::Edit(e) => match e {
            Edit::Offset { faces, .. }
            | Edit::Thicken { faces, .. }
            | Edit::Delete { faces }
            | Edit::Move { faces, .. } => faces.clone(),
            Edit::Replace { face, .. } => vec![*face],
            Edit::Rib(_) => vec![],
        },
        _ => vec![],
    }
}

fn holds(b: &Body, n: Name) -> bool {
    b.face_by_name(n).is_some() || b.edge_by_name(n).is_some() || b.vertices.iter().any(|v| v.name == n)
}

/// The bodies a feature naming `names` is on: its pick, else those holding any of them, else
/// the current one.
fn named_targets(
    solids: &[Solid],
    p: Option<&Pick>,
    names: &[Name],
    i: usize,
) -> Result<Vec<usize>, RegenError> {
    if p.is_none() {
        let on: Vec<usize> = (0..solids.len())
            .filter(|&k| names.iter().any(|&n| holds(&solids[k].body, n)))
            .collect();
        if !on.is_empty() {
            return Ok(on);
        }
    }
    targets(&ids(solids), p, i)
}

/// A fillet, chamfer, shell or draft cut down to the names `b` holds.
fn only(f: &Feature, b: &Body) -> Feature {
    let mut g = f.clone();
    match &mut g {
        Feature::Fillet {
            edges,
            starts,
            setbacks,
        } => {
            edges.retain(|e| holds(b, e.0));
            starts.retain(|s| holds(b, s.0));
            setbacks.retain(|s| holds(b, s.0));
        }
        Feature::Chamfer { edges, .. } => edges.retain(|&n| holds(b, n)),
        Feature::Shell { open, .. } => open.retain(|&n| holds(b, n)),
        Feature::Draft { faces, .. } => faces.retain(|&n| holds(b, n)),
        Feature::Edit(
            Edit::Offset { faces, .. }
            | Edit::Thicken { faces, .. }
            | Edit::Delete { faces }
            | Edit::Move { faces, .. },
        ) => faces.retain(|&n| holds(b, n)),
        _ => {}
    }
    g
}

/// A part-changing feature applied to each body it is on.
fn modify_on(
    plain: History<'_>,
    i: usize,
    p: Option<&Pick>,
    solids: &mut Vec<Solid>,
    threads: usize,
) -> Result<(), RegenError> {
    let f = &plain[i];
    let all = names(f);
    // finished, as `step` hands a part-changing feature its part: finishing renames what it joins,
    // and the feature's names are those, so one not found yet may be on any body
    let missing = |solids: &[Solid], on: &[usize]| {
        all.iter()
            .find(|&&n| !on.iter().any(|&k| holds(&solids[k].body, n)))
            .copied()
    };
    let mut on = named_targets(solids, p, &all, i)?;
    let finishing: Vec<usize> = match missing(solids, &on) {
        Some(_) => (0..solids.len()).collect(),
        None => on.clone(),
    };
    for k in finishing {
        solids[k].body = crate::finish(std::mem::take(&mut solids[k].body));
    }
    on = named_targets(solids, p, &all, i)?;
    // a name on no body this feature works on is an error, never a step left out
    if let Some(name) = missing(solids, &on) {
        return Err(RegenError::MissingEdge { feature: i, name });
    }
    let mut new = Vec::new();
    for k in on {
        let g = if all.is_empty() {
            Cow::Borrowed(f)
        } else {
            Cow::Owned(only(f, &solids[k].body))
        };
        // on every body picked, none of the names on this one: nothing to do here
        if crate::names_nothing(&g) && !crate::names_nothing(f) {
            continue;
        }
        let out = modify(plain, &g, i, &solids[k].body, threads)?;
        if op(f) == Some(Combine::New) {
            // a slab thickened as a new body
            new.push(Solid { id: i, body: out });
        } else {
            solids[k].body = out;
        }
    }
    solids.extend(new);
    Ok(())
}

/// Every name in `b` passed through `rename`.
fn renamed(b: &Body, rename: impl Fn(Name) -> Name) -> Body {
    let mut out = b.clone();
    out.vertices.iter_mut().for_each(|v| v.name = rename(v.name));
    out.edges.iter_mut().for_each(|e| e.name = rename(e.name));
    out.faces.iter_mut().for_each(|f| f.name = rename(f.name));
    out.split = Vec::new();
    for &w in &b.split {
        out.note_split(rename(w));
    }
    out
}

/// `b` with the names `taken` holds moved off: what the cut trimmed on both sides becomes a
/// piece, an edge named by its ends and a face by its edges, as if the cut had split it in one
/// body; anything else by `salt`.
fn split_off(mut b: Body, taken: &HashSet<Name>, salt: impl Fn(Name) -> Name) -> Body {
    for v in b.vertices.iter_mut().filter(|v| taken.contains(&v.name)) {
        v.name = salt(v.name);
    }
    // an edge is taken where any piece of its whole is: the one piece the boolean left on this
    // side may have taken the whole's name back (`Body::sole_survivors`)
    let wholes: HashSet<Name> = taken.iter().map(|n| n.whole()).collect();
    for i in 0..b.edges.len() {
        let e = &b.edges[i];
        if wholes.contains(&e.name.whole()) {
            let ends = b.vertices[e.start.index()]
                .name
                .with_name(b.vertices[e.end.index()].name);
            let whole = e.name.whole();
            b.edges[i].name = e.name.piece(ends);
            // a piece in its own body whether or not the edge was split before: never given
            // the whole's name back here
            b.split.retain(|w| w.whole() != whole);
        }
    }
    for f in b.face_ids().collect::<Vec<_>>() {
        let name = b.face(f).name;
        if taken.contains(&name) {
            let least = b
                .face(f)
                .loops
                .iter()
                .flat_map(|&l| b.lp(l).coedges().map(|c| b.edge(b.coedge(c).edge).name))
                .min();
            b.faces[f.index()].name = least.map_or(salt(name), |n| name.piece(n));
        }
    }
    b
}

/// A name moved off the names another body holds, by what moved it (the id of the feature
/// that made that body, or of the feature that moves it).
fn salted(n: Name, why: &str, id: u64) -> Name {
    n.salted(Name::NONE.with_str(why).with(id))
}

/// The id a boolean of the feature of id `id` names what it makes after (`n`: which of several).
fn bool_id(id: u64, n: u64) -> u64 {
    (id + 1_000_000) | (n << 40)
}

/// A feature's tools (feature `i`, of id `id`) combined into `b`; `salt`, the tools' names
/// moved off the ones they left in another body.
fn add_tools(b: Body, ts: &[Tool], (i, id): (usize, u64), salt: Option<u64>) -> Result<Body, RegenError> {
    let moved_off: Vec<Body>;
    let bodies: Vec<&Body> = match salt {
        None => ts.iter().map(|t| &t.0).collect(),
        Some(id) => {
            moved_off = ts
                .iter()
                .map(|t| renamed(&t.0, |n| salted(n, "body", id)))
                .collect();
            moved_off.iter().collect()
        }
    };
    let ops: Vec<Op> = ts.iter().map(|t| op_of(t.1)).collect();
    let shapes: Vec<&Shape> = ts.iter().map(|t| &t.2).collect();
    combine(b, &bodies, &ops, &shapes, (i, id), salt)
}

/// `add_tools` with the tools' names already moved off.
fn combine(
    mut b: Body,
    bodies: &[&Body],
    ops: &[Op],
    shapes: &[&Shape],
    (i, id): (usize, u64),
    salt: Option<u64>,
) -> Result<Body, RegenError> {
    let err = |error| RegenError::Boolean { feature: i, error };
    let n = salt.map_or(0, |s| s + 1);
    if let [tool] = bodies {
        boolean_into(&mut b, tool, ops[0], bool_id(id, n)).map_err(err)?;
        b = crate::not_empty(b, i)?;
        if b.dead > b.faces.len() / 2 {
            b = b.into_compact();
        }
        return Ok(b);
    }
    if bodies.is_empty() {
        return Ok(b);
    }
    let base = if b.dead > 0 { b.into_compact() } else { b };
    let mut all = vec![&base];
    all.extend(bodies);
    let root = salt.map_or(cut_root(i), |s| salted(cut_root(i), "body", s));
    let mut sh = vec![part_shape(&base)];
    sh.extend(shapes.iter().map(|&s| s.clone()));
    crate::not_empty(nary_with(&all, ops, Some(&sh), None, 1, root).map_err(err)?, i)
}

fn bad(i: usize) -> impl Fn(BodyError) -> RegenError {
    move |error| RegenError::Body { feature: i, error }
}

fn boolean_err(i: usize) -> impl Fn(BoolError) -> RegenError {
    move |error| RegenError::Boolean { feature: i, error }
}

fn find(solids: &[Solid], id: usize, i: usize) -> Result<usize, RegenError> {
    solids
        .iter()
        .position(|s| s.id == id)
        .ok_or(bad(i)(BodyError::NoBody { id }))
}

/// Combines, splits or moves bodies (feature `i`).
fn body_feature(
    plain: History<'_>,
    i: usize,
    bf: &BodyFeature,
    p: Option<&Pick>,
    solids: &mut Vec<Solid>,
) -> Result<(), RegenError> {
    let bad = bad(i);
    let me = plain.id(i);
    match bf {
        BodyFeature::In { .. } => unreachable!("strip unwraps In"),
        BodyFeature::Combine {
            op,
            target,
            tools,
            keep,
        } => {
            let t = find(solids, *target, i)?;
            let mut body = solids[t].body.compact();
            for (n, &id) in tools.iter().enumerate() {
                if id == *target {
                    return Err(bad(BodyError::SameBody { id }));
                }
                let k = find(solids, id, i)?;
                let tool = if *keep {
                    renamed(&solids[k].body, |x| salted(x, "combine", me))
                } else {
                    solids[k].body.clone()
                };
                body = boolean(&body, &tool, op_of(*op), bool_id(me, n as u64)).map_err(boolean_err(i))?;
            }
            solids[t].body = body;
            if !keep {
                solids.retain(|s| !tools.contains(&s.id));
            }
        }
        BodyFeature::Split { by, keep } => {
            let surface = cutter(plain, i, by)?;
            let on = targets(&ids(solids), p, i)?;
            if *keep == Keep::Both && on.len() != 1 {
                return Err(bad(BodyError::NotOne));
            }
            for k in on {
                let body = solids[k].body.compact();
                let (h, plus_in) = half(&surface, &body_box(&body), (i, me))?;
                let (plus, minus) = if plus_in {
                    (Op::Intersect, Op::Subtract)
                } else {
                    (Op::Subtract, Op::Intersect)
                };
                let piece = |op: Op, h: &Body, n: u64| -> Result<Body, RegenError> {
                    let b = boolean(&body, h, op, bool_id(me, n)).map_err(boolean_err(i))?;
                    if b.face_ids().next().is_none() {
                        return Err(bad(BodyError::Misses));
                    }
                    Ok(b)
                };
                match keep {
                    Keep::Plus => solids[k].body = piece(plus, &h, 0)?,
                    Keep::Minus => solids[k].body = piece(minus, &h, 0)?,
                    Keep::Both => {
                        let below = piece(minus, &h, 0)?;
                        let h2 = renamed(&h, |x| salted(x, "split", me));
                        let above = piece(plus, &h2, 1)?;
                        let taken: HashSet<Name> = all_names(&below).collect();
                        let above = split_off(above, &taken, |x| salted(x, "split", me));
                        solids[k].body = below;
                        solids.push(Solid { id: i, body: above });
                    }
                }
            }
        }
        BodyFeature::SplitFace { faces, by } => {
            let surface = cutter(plain, i, by)?;
            for &n in faces {
                if !solids.iter().any(|s| s.body.face_by_name(n).is_some()) {
                    return Err(bad(BodyError::NoFace { name: n }));
                }
            }
            for k in named_targets(solids, p, faces, i)? {
                let mine: Vec<Name> = faces
                    .iter()
                    .copied()
                    .filter(|&n| solids[k].body.face_by_name(n).is_some())
                    .collect();
                solids[k].body =
                    crate::splitface::split(&solids[k].body.compact(), &mine, &surface, me).map_err(&bad)?;
            }
        }
        BodyFeature::Move { steps, copy } => {
            let m = motion(steps);
            let on = targets(&ids(solids), p, i)?;
            let pe = |error: PatternError| RegenError::Pattern { feature: i, error };
            if *copy {
                let [k] = on[..] else {
                    return Err(bad(BodyError::NotOne));
                };
                let body = moved(&solids[k].body.compact(), &m, |n| salted(n, "copy", me)).map_err(pe)?;
                solids.push(Solid { id: i, body });
            } else {
                for k in on {
                    solids[k].body = moved(&solids[k].body.compact(), &m, |n| n).map_err(pe)?;
                }
            }
        }
    }
    Ok(())
}

fn all_names(b: &Body) -> impl Iterator<Item = Name> + '_ {
    b.vertices
        .iter()
        .map(|v| v.name)
        .chain(b.edges.iter().map(|e| e.name))
        .chain(b.faces.iter().map(|f| f.name))
}

fn body_box(b: &Body) -> Box3 {
    b.face_ids().fold(Box3::EMPTY, |acc, f| acc.hull(&b.face_box(f)))
}

/// The surface a cutter cuts along.
fn cutter(plain: History<'_>, i: usize, by: &Cutter) -> Result<Surface, RegenError> {
    Ok(match *by {
        Cutter::Target(t) => crate::extent::surface(plain, i, &t)?,
        Cutter::Plane(frame) => Surface::Plane(Plane { frame }),
        Cutter::Curve(frame, seg) => match seg {
            Segment::Line { a, b } => {
                let (a, b) = (frame.plane_point(a), frame.plane_point(b));
                let along = (b - a)
                    .normalize()
                    .ok_or(bad(i)(BodyError::Unsupported("a line of no length")))?;
                let z = frame
                    .z()
                    .cross(along.get())
                    .normalize()
                    .expect("the line lies in its sketch plane");
                Surface::Plane(Plane {
                    frame: Frame::from_zx(a, z, along.get()).expect("square axes"),
                })
            }
            Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                Surface::Cylinder(Cylinder {
                    frame: frame.with_origin(frame.plane_point(center)),
                    radius,
                })
            }
        },
    })
}

/// A solid bounded by `s` holding all of box `b` on one side of it, and whether that side is
/// the plus one (where the surface's normal points).
fn half(s: &Surface, b: &Box3, (i, id): (usize, u64)) -> Result<(Body, bool), RegenError> {
    let ee = |error| RegenError::Extrude { feature: i, error };
    // an empty body has an empty box: nothing for the cutter to miss or cross
    if !b.diagonal().is_finite() {
        return Err(RegenError::Body {
            feature: i,
            error: BodyError::Misses,
        });
    }
    let b = b.grow(1.0 + 0.01 * b.diagonal());
    let local = |f: &Frame| Box3::from_points(corners(&b).map(|p| f.point_to_local(p)));
    match s {
        Surface::Plane(pl) => {
            let l = local(&pl.frame);
            let profile = Profile {
                plane: pl.frame,
                loops: vec![Profile::rectangle(
                    Point2::new(l.x.lo, l.y.lo),
                    l.x.width(),
                    l.y.width(),
                    0,
                )],
            };
            Ok((
                extrude(&profile, l.z.hi.max(tol::LINEAR * 1e3), id).map_err(ee)?,
                true,
            ))
        }
        Surface::Cylinder(c) => {
            let l = local(&c.frame);
            let profile = Profile {
                plane: c
                    .frame
                    .with_origin(c.frame.point_to_world(Point3::new(0.0, 0.0, l.z.lo))),
                loops: vec![ProfileLoop::new(vec![(
                    Segment::Circle {
                        center: Point2::ORIGIN,
                        radius: c.radius,
                        ccw: true,
                    },
                    0,
                )])],
            };
            Ok((extrude(&profile, l.z.width(), id).map_err(ee)?, false))
        }
        Surface::Sphere(sp) => {
            let ball = Feature::Ball {
                frame: sp.frame,
                radius: sp.radius,
                op: Combine::Add,
            };
            Ok((crate::tool(i, id, &ball, true)?.0, false))
        }
        _ => Err(bad(i)(BodyError::Unsupported(
            "splitting by a cone, torus or blend",
        ))),
    }
}

/// A body's run of tools in the n-ary regen: its base and the tools since, in order.
struct Run {
    id: usize,
    base: Result<Body, Tool>,
    tools: Vec<(Body, Op, Shape)>,
    /// Each feature the tools came from, in order: its index, the salt its tools' names were
    /// moved off with (on a body other than the feature's first), and how many tools it gave.
    feats: Vec<(usize, Option<u64>, usize)>,
    /// The last feature it took a tool from.
    last: usize,
}

impl Run {
    fn flush(self, plain: History<'_>, threads: usize) -> Result<Solid, RegenError> {
        let id = self.id;
        if self.tools.is_empty() {
            let body = self.base.unwrap_or_else(|t| t.0);
            return Ok(Solid { id, body });
        }
        let base = match &self.base {
            Ok(b) => b,
            Err(t) => &t.0,
        };
        if self.tools.len() <= SHORT_RUN
            && self.feats.iter().all(|f| f.2 == 1)
            && let Some(body) = self.sequential(base.clone(), plain)
        {
            return Ok(Solid { id, body });
        }
        let root = salted(cut_root(self.last), "body", plain.id(id));
        let ops: Vec<Op> = self.tools.iter().map(|t| t.1).collect();
        let mut all = vec![base];
        all.extend(self.tools.iter().map(|t| &t.0));
        let mut shapes = vec![match &self.base {
            Ok(b) => part_shape(b),
            Err(t) => t.2.clone(),
        }];
        shapes.extend(self.tools.iter().map(|t| t.2.clone()));
        let body =
            nary_with(&all, &ops, Some(&shapes), None, threads, root).map_err(boolean_err(self.last))?;
        Ok(Solid { id, body })
    }

    /// The tools combined into `b` one feature at a time, as the sequential regen does: the same
    /// body for less work on a short run. None where a boolean refuses.
    fn sequential(&self, mut b: Body, plain: History<'_>) -> Option<Body> {
        let mut at = 0;
        for &(i, salt, n) in &self.feats {
            let ts = &self.tools[at..at + n];
            let bodies: Vec<&Body> = ts.iter().map(|t| &t.0).collect();
            let ops: Vec<Op> = ts.iter().map(|t| t.1).collect();
            let shapes: Vec<&Shape> = ts.iter().map(|t| &t.2).collect();
            b = combine(b, &bodies, &ops, &shapes, (i, plain.id(i)), salt).ok()?;
            at += n;
        }
        Some(b)
    }
}

fn flush(runs: Vec<Run>, plain: History<'_>, threads: usize) -> Result<Vec<Solid>, RegenError> {
    runs.into_iter().map(|r| r.flush(plain, threads)).collect()
}

fn unflush(solids: Vec<Solid>, last: usize) -> Vec<Run> {
    solids
        .into_iter()
        .map(|s| Run {
            id: s.id,
            base: Ok(s.body),
            tools: vec![],
            feats: vec![],
            last,
        })
        .collect()
}

/// Every tool feature's tools from `start` on, built on up to `threads` threads.
fn build_each(plain: History<'_>, start: usize, threads: usize) -> Result<Vec<Vec<Tool>>, RegenError> {
    let some = |r: std::ops::Range<usize>| -> Result<Vec<Vec<Tool>>, RegenError> {
        r.map(|i| match &plain[i] {
            f if modifies(f) || matches!(f, Feature::Plane(_)) => Ok(vec![]),
            _ => tools(plain, i, true),
        })
        .collect()
    };
    let n = plain.len() - start;
    let mut out: Vec<Vec<Tool>> = (0..start).map(|_| vec![]).collect();
    if threads <= 1 || n <= 1 {
        out.extend(some(start..plain.len())?);
        return Ok(out);
    }
    let chunk = n.div_ceil(threads);
    let built: Vec<Result<Vec<Vec<Tool>>, RegenError>> = std::thread::scope(|s| {
        let hs: Vec<_> = (start..plain.len())
            .step_by(chunk)
            .map(|a| {
                let some = &some;
                s.spawn(move || some(a..(a + chunk).min(plain.len())))
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("tool builder")).collect()
    });
    for b in built {
        out.extend(b?);
    }
    Ok(out)
}

/// The n-ary regen of a multi-body history: each body's tools between features that need it
/// built (part-changing and body features) go in one n-ary boolean.
pub(crate) fn nary(features: History<'_>, threads: usize) -> Result<Vec<Solid>, RegenError> {
    if crate::any_touching(&features) {
        return sequential(features);
    }
    match nary_pass(features, threads) {
        Err(e) if crate::names_missing(&e) => {
            crate::count_name_fallback();
            sequential(features)
        }
        // a run that empties a body: it goes at that feature, and later ones act on what is left
        Ok(s) if s.iter().any(|x| x.body.face_ids().next().is_none()) => sequential(features),
        Err(RegenError::Empty { .. }) => sequential(features),
        // (one at a time, a tool's knife edge can be blunted)
        Err(e)
            if crate::lost_its_way(&e) && {
                let (plain, _) = strip(&features);
                let plain = features.on(&plain);
                (0..plain.len()).any(|i| crate::blunted(plain, i).is_some())
            } =>
        {
            sequential(features)
        }
        r => r,
    }
}

fn nary_pass(features: History<'_>, threads: usize) -> Result<Vec<Solid>, RegenError> {
    let (plain, picks) = strip(&features);
    let plain = features.on(&plain);
    let start = plain.iter().position(starts).unwrap_or(0);
    if plain.get(start).is_none_or(|f| !starts(f)) {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    let mut built = build_each(plain, start, threads)?;
    let mut runs: Vec<Run> = Vec::new();
    for i in start..plain.len() {
        let f = &plain[i];
        let p = pick(&picks, i);
        let mut ts = std::mem::take(&mut built[i]);
        if starts(f) {
            runs.push(Run {
                id: i,
                base: Err(ts.swap_remove(0)),
                tools: vec![],
                feats: vec![],
                last: i,
            });
            continue;
        }
        match f {
            Feature::Plane(_) => {}
            Feature::Work(_) => {
                crate::work::datum(plain, i)?;
            }
            Feature::Body(b) => {
                let mut solids = flush(std::mem::take(&mut runs), plain, threads)?;
                body_feature(plain, i, b, p, &mut solids)?;
                runs = unflush(solids, i);
            }
            _ if modifies(f) => {
                let mut solids = flush(std::mem::take(&mut runs), plain, threads)?;
                modify_on(plain, i, p, &mut solids, threads)?;
                runs = unflush(solids, i);
            }
            _ if ts.is_empty() => {}
            _ => {
                let ids: Vec<usize> = runs.iter().map(|r| r.id).collect();
                let on = targets(&ids, p, i)?;
                // the first body takes the tools themselves, the others copies named apart
                for &k in on.iter().skip(1) {
                    let run = &mut runs[k];
                    let salt = plain.id(run.id);
                    for t in &ts {
                        let body = renamed(&t.0, |x| salted(x, "body", salt));
                        run.tools.push((body, op_of(t.1), t.2.clone()));
                    }
                    run.feats.push((i, Some(salt), ts.len()));
                    run.last = i;
                }
                let run = &mut runs[on[0]];
                run.feats.push((i, None, ts.len()));
                run.tools.extend(ts.into_iter().map(|t| (t.0, op_of(t.1), t.2)));
                run.last = i;
            }
        }
    }
    finish(flush(runs, plain, threads)?)
}
