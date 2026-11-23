//! How a cone's face lies at its apex against another solid, decided in closed form.
//!
//! A cone has no tangent plane at its apex, so the touch loop's tangent-plane probe
//! (`boolean::local_class`) cannot say how the face lies there. Its tangent cone there is the cone
//! itself: the face's points near the apex are its generators, the rays g(u) = a + s (cos u e1 +
//! sin u e2) for every turn u (a the axis way into the face, scaled by cos of the half angle, s its
//! sine). Against a face of the other solid through the apex with outward normal n, a generator
//! leaves outward where n·g(u) > 0; over the whole turn n·g(u) = A + B cos(u − φ), A = n·a,
//! B = |s| √((n·e1)² + (n·e2)²), so the face's sheet lies wholly outside that face's half-space
//! when A > B, wholly inside when A < −B, and crosses it otherwise. To first order this is exact:
//! a strict sign on every generator is the sign near the apex, whatever the surfaces' curvature.
//! At an edge or vertex of the other solid, its half-spaces meet as an intersection where every
//! edge there is convex and as a union where every one is concave. Any other case is undecided:
//! the caller refuses at the apex and never guesses. A face going only part of the turn round its
//! apex is decided by the whole turn: a verdict on every generator is one on the face's.

use crate::classify::{Index, Where};
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point3, Vec3, dmath, tol};
use forge_topo::{Body, FaceId};

/// A cone face's generators at its apex: g(u) = axis + side (cos u e1 + sin u e2).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sheet {
    axis: Vec3,
    side: f64,
    e1: Vec3,
    e2: Vec3,
}

/// How a face lies at a point where it has no tangent plane: None where it has one (or `p` is no
/// apex of its cone: `local_class` decides there), Some(None) at an apex this cannot read (no
/// point of the face away from it to tell its nappe), else its sheet there: every generator of
/// the cone's nappe, whatever part of the turn the face has.
pub(crate) fn apex_sheet(body: &Body, f: FaceId, p: Point3) -> Option<Option<Sheet>> {
    let face = body.face(f);
    let Surface::Cone(c) = &face.surface else {
        return None;
    };
    if c.apex().distance(p) > tol::LINEAR || body.face_normal_at(f, p).is_some() {
        return None;
    }
    // (the face may go only part of the turn round its apex, as a rounded corner tapering to a
    // point does: `where_sheet` decides over the whole turn, and what holds for every generator
    // holds for the face's; a turn that crosses is undecided whatever part the face has)
    let away = face
        .loops
        .iter()
        .flat_map(|&l| body.lp(l).coedges())
        .map(|co| {
            let e = body.edge(body.coedge(co).edge);
            e.curve.eval(e.range.mid())
        })
        .find(|m| m.distance(p) > tol::LINEAR);
    let Some(m) = away else {
        return Some(None);
    };
    // the nappe the face is on: v grows away from the apex along +z by cos a per unit
    let z = c.frame.z().get();
    let up = if c.frame.point_to_local(m).z > c.apex_v() {
        1.0
    } else {
        -1.0
    };
    Some(Some(Sheet {
        axis: z * (up * c.cos_a()),
        side: c.sin_a().abs(),
        e1: c.frame.x().get(),
        e2: c.frame.y().get(),
    }))
}

/// Rounding in n·g for unit n and g: below it a sign is not told.
const SIGN: f64 = 64.0 * f64::EPSILON;

/// n·g(u) = a + b cos(u − phi) over the turn.
#[derive(Clone, Copy, Debug)]
enum Turn {
    Out,
    In,
    Both { a: f64, b: f64, phi: f64 },
}

fn turn(sheet: &Sheet, n: Vec3) -> Turn {
    let a = n.dot(sheet.axis);
    let (c, s) = (n.dot(sheet.e1) * sheet.side, n.dot(sheet.e2) * sheet.side);
    let b = (c * c + s * s).sqrt();
    if a - b > SIGN {
        Turn::Out
    } else if a + b < -SIGN {
        Turn::In
    } else {
        Turn::Both {
            a,
            b,
            phi: dmath::atan2(s, c),
        }
    }
}

/// `sheet` against another cone's apex at the same point, sheet `other`: a generator g (unit) is
/// within that cone where g·a2 > cos a2, a2 its unit axis into its face, so g·axis2 > cos² a2 over
/// the turn as A + B cos(u − φ) against it; its solid is that inside for an outward face
/// (`same_sense`), the outside for a cavity's.
fn in_cone(sheet: &Sheet, other: &Sheet, same_sense: bool) -> Turn {
    let c = other.axis.dot(other.axis);
    let a = sheet.axis.dot(other.axis);
    let (x, y) = (
        sheet.e1.dot(other.axis) * sheet.side,
        sheet.e2.dot(other.axis) * sheet.side,
    );
    let b = (x * x + y * y).sqrt();
    let (inside, outside) = (a - b > c + SIGN, a + b < c - SIGN);
    match (inside, outside, same_sense) {
        (true, _, true) | (_, true, false) => Turn::In,
        (true, _, false) | (_, true, true) => Turn::Out,
        _ => Turn::Both {
            a: 0.0,
            b: 0.0,
            phi: 0.0,
        },
    }
}

/// Two turns alike: both wholly out, both wholly in, or both crossing at the same place.
fn same(x: &Turn, y: &Turn) -> bool {
    match (x, y) {
        (Turn::Out, Turn::Out) | (Turn::In, Turn::In) => true,
        (
            Turn::Both { a, b, phi },
            Turn::Both {
                a: a2,
                b: b2,
                phi: p2,
            },
        ) => (a - a2).abs() <= SIGN && (b - b2).abs() <= SIGN && (phi - p2).abs() <= SIGN,
        _ => false,
    }
}

/// Whether arcs (centre, half width) cover the whole turn, each overlapping the next by more than
/// the rounding: Some(true) or Some(false) told, None too close to tell.
fn covers(mut arcs: Vec<(f64, f64)>) -> Option<bool> {
    const ANGLE: f64 = 1e-9;
    if arcs.is_empty() {
        return Some(false);
    }
    let tau = dmath::TAU;
    let mut spans: Vec<(f64, f64)> = arcs
        .drain(..)
        .map(|(c, w)| {
            let lo = (c - w).rem_euclid(tau);
            (lo, lo + 2.0 * w)
        })
        .collect();
    spans.sort_by(|x, y| x.0.total_cmp(&y.0));
    // sweep twice round, from the first start: covered when the reach passes start + tau
    let start = spans[0].0;
    let mut reach = spans[0].1;
    let mut close = false;
    for k in 1..2 * spans.len() {
        let (lo, hi) = spans[k % spans.len()];
        let (lo, hi) = if k >= spans.len() {
            (lo + tau, hi + tau)
        } else {
            (lo, hi)
        };
        if reach >= start + tau + ANGLE {
            break;
        }
        if lo > reach + ANGLE {
            return if close { None } else { Some(false) };
        }
        if lo > reach - ANGLE {
            close = true;
        }
        reach = reach.max(hi);
    }
    if reach >= start + tau + ANGLE {
        if close { None } else { Some(true) }
    } else if reach >= start + tau - ANGLE {
        None
    } else {
        Some(false)
    }
}

/// The arc of the turn where a + b cos(u − phi) > 0 (`pos`) or < 0.
fn arc(a: f64, b: f64, phi: f64, pos: bool) -> (f64, f64) {
    let w = dmath::acos((-a / b).clamp(-1.0, 1.0));
    if pos {
        (phi, w)
    } else {
        (phi + dmath::PI, dmath::PI - w)
    }
}

/// Where `sheet`, a cone face's generators at `p`, lies against solid `other`: Inside, Outside,
/// or None undecided (tangent within rounding, crossing, a vertex both convex and concave, or a
/// face there with no tangent plane of its own).
pub(crate) fn where_sheet(other: &Body, index: &Index, p: Point3, sheet: &Sheet) -> Option<Where> {
    if !index.bbox.grow(tol::LINEAR).contains(p) {
        return Some(Where::Outside);
    }
    let through: Vec<FaceId> = other
        .face_ids()
        .filter(|&g| {
            index.face_box(g).grow(tol::LINEAR).contains(p)
                && other.face(g).surface.distance(p) <= tol::LINEAR
                && index.point_in_face(g, p) != Where::Outside
        })
        .collect();
    if through.is_empty() {
        return match index.classify(p) {
            Where::On => None,
            w => Some(w),
        };
    }
    let mut turns = Vec::with_capacity(through.len());
    for &g in &through {
        let t = match other.face_normal_at(g, p) {
            Some(n) => turn(sheet, n.get()),
            // another cone's apex: inside it where the generator is within its half angle
            None if through.len() == 1 => {
                in_cone(sheet, &apex_sheet(other, g, p)??, other.face(g).same_sense)
            }
            None => return None,
        };
        turns.push(t);
    }
    if let [t] = turns[..] {
        return match t {
            Turn::Out => Some(Where::Outside),
            Turn::In => Some(Where::Inside),
            Turn::Both { .. } => None,
        };
    }
    // an edge or vertex: its half-spaces' intersection if every edge there is convex, union if
    // every one is concave; with no crease there (faces meeting smoothly) the one half-space
    let convex = match convexity(other, &through, p) {
        Some(c) => c,
        None if turns.windows(2).all(|w| same(&w[0], &w[1])) => {
            return match turns[0] {
                Turn::Out => Some(Where::Outside),
                Turn::In => Some(Where::Inside),
                Turn::Both { .. } => None,
            };
        }
        None => return None,
    };
    let (all, any) = if convex {
        (Turn::In, Turn::Out)
    } else {
        (Turn::Out, Turn::In)
    };
    let is = |t: &Turn, x: &Turn| std::mem::discriminant(t) == std::mem::discriminant(x);
    let (w_all, w_any) = if convex {
        (Where::Inside, Where::Outside)
    } else {
        (Where::Outside, Where::Inside)
    };
    if turns.iter().all(|t| is(t, &all)) {
        return Some(w_all);
    }
    if turns.iter().any(|t| is(t, &any)) {
        return Some(w_any);
    }
    // the `any` side for every generator when the arcs where some face gives it cover the turn
    let arcs: Vec<(f64, f64)> = turns
        .iter()
        .filter_map(|t| match *t {
            Turn::Both { a, b, phi } => Some(arc(a, b, phi, convex)),
            _ => None,
        })
        .collect();
    match covers(arcs)? {
        true => Some(w_any),
        false => None,
    }
}

/// Whether every edge of `other` through `p` between faces of `through` is convex (Some(true)),
/// every one concave (Some(false)); None if mixed, tangent, or none found.
fn convexity(other: &Body, through: &[FaceId], p: Point3) -> Option<bool> {
    let mut seen = Vec::new();
    let mut sign: Option<bool> = None;
    for &g in through {
        for &l in &other.face(g).loops {
            for co in other.lp(l).coedges() {
                let e = other.coedge(co).edge;
                if seen.contains(&e) {
                    continue;
                }
                let edge = other.edge(e);
                let t = edge.curve.closest(p, edge.range);
                if edge.curve.eval(t).distance(p) > tol::LINEAR {
                    continue;
                }
                seen.push(e);
                let [c1, c2] = edge.coedges[..] else {
                    return None;
                };
                let (f1, f2) = (other.coedge_face(c1), other.coedge_face(c2));
                let n1 = other.face_normal_at(f1, p)?.get();
                let n2 = other.face_normal_at(f2, p)?.get();
                // a seam, or an edge the faces run on smoothly across: no crease, one half-space
                if (n1 - n2).length() <= SIGN {
                    continue;
                }
                let d = edge.curve.deriv(t).d1;
                let d = if other.coedge(c1).forward { d } else { -d };
                let d = d * (1.0 / d.length());
                // face 1 lies left of its coedge seen from outside: n1 x d points into it, and on
                // a convex edge face 2's half-space holds it
                let k = n2.dot(n1.cross(d));
                if k.abs() <= SIGN {
                    return None;
                }
                let here = k < 0.0;
                if sign.is_some_and(|s| s != here) {
                    return None;
                }
                sign = Some(here);
            }
        }
    }
    sign
}
