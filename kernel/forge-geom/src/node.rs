//! Tangent nodes (research/curved-sites/blend-node.md, ledger C.node).
//!
//! Two tubes (a cylinder, a torus, or a rolling-ball pipe about a straight or circular spine) whose
//! spines both lie parallel to one plane S at their radius from it, on one side: each touches S
//! along its spring (its spine moved onto S), and where the springs cross the two share S's normal,
//! so they touch. Two rounds of a boolean's operands that roll off one common face meet so. Their
//! intersection crosses itself there (an X), and a marching trace cannot be trusted near it: it ends
//! a hair off the node (the heights over S differ to second order only, so a height error e moves the
//! trace's end by about √(2 r e)), or traces one branch twice, from both sides.
//!
//! What stands instead, and why it is sound (verify/forge-curved/ForgeCurved/BlendNode.lean):
//! - over S, within its radius of its spring, each tube's cap lies h(ρ) = r − √(r² − ρ²) below S
//!   at plane distance ρ from the spring (`cap_section`), and h is strictly increasing on [0, r]
//!   (`h_strictMonoOn`);
//! - so where both caps lie past their springs, hA(ρA) = hB(ρB) holds for at most one ρB per ρA,
//!   increasing with ρA, meeting either spring only at the node (`one_branch`, `tubes_one_branch`,
//!   `branch_increasing`, `branch_meets_springs_only_at_node`), and the node is on both (`h_zero`);
//! - the node is the springs' crossing, in closed form on S (lines and circles), well conditioned
//!   where they cross at an angle (`SIN_LEAST`), and mended only where no other crossing of the two
//!   springs lies within four reaches and the reach is within a quarter of each cap's (`reach_of`).
//!
//! A traced curve that comes within `reach` (two of its steps) of a node is cut there, and each
//! stretch that ran into the node is walked back into it along both surfaces, ending on it exactly
//! (`towards_touch`); a stretch running along one already kept is the same branch and is dropped.
//! Anything outside those conditions is left as the trace gave it. forge-bool's `ssi::search` asks
//! this of every pair it traces.

use crate::blend::Pipe;
use crate::intcurve::{IntCurve, direction, refine, towards_touch};
use crate::{Curve, CurveGeom, Surface};
use forge_math::{Interval, Point3, Vec3, tol};

/// The springs must cross at more than this sine (about 1.1 degrees): below it the crossing's place
/// along them is as uncertain as the trace's.
const SIN_LEAST: f64 = 0.02;

/// A tube's spine: a line (point, unit direction) or a circle (centre, unit axis, radius).
#[derive(Clone, Copy, Debug)]
enum Spine {
    Line(Point3, Vec3),
    Circle(Point3, Vec3, f64),
}

/// The surface as a tube: its spine and radius. (A spindle, a circle no larger than the radius,
/// is one too: its cap over S is a single sheet within the spine's radius of the spring, which
/// `reach_of` keeps to.)
fn tube(s: &Surface) -> Option<(Spine, f64)> {
    let (spine, r) = match s {
        Surface::Cylinder(c) => (Spine::Line(c.frame.origin, c.frame.z().get()), c.radius),
        Surface::Torus(t) => (Spine::Circle(t.frame.origin, t.frame.z().get(), t.major), t.minor),
        Surface::Procedural(p) => {
            let pipe = p.as_any()?.downcast_ref::<Pipe>()?;
            let spine = match &pipe.spine {
                Curve::Line(l) => Spine::Line(l.origin, l.dir.get()),
                Curve::Circle(c) => Spine::Circle(c.frame.origin, c.frame.z().get(), c.radius),
                _ => return None,
            };
            (spine, pipe.radius.abs())
        }
        _ => return None,
    };
    let big = match spine {
        Spine::Circle(_, _, big) => big,
        Spine::Line(..) => f64::INFINITY,
    };
    (r > 0.0 && r.is_finite() && big > 0.0).then_some((spine, r))
}

/// How far from its spring a tube's cap is the one sheet h(ρ) over S: its radius, and a circular
/// spine's own radius (past which the cap reaches the axis).
fn reach_of((spine, r): (Spine, f64)) -> f64 {
    match spine {
        Spine::Line(..) => r,
        Spine::Circle(_, _, big) => r.min(big),
    }
}

/// The tube's spring on the plane through `o` square to unit `n`: its spine moved onto the plane,
/// and which side the spine lies (+1 or −1), when the spine lies parallel to the plane at the
/// tube's radius from it.
fn spring((spine, r): (Spine, f64), o: Point3, n: Vec3) -> Option<(Spine, f64)> {
    let level = |p: Point3| {
        let h = (p - o).dot(n);
        ((h.abs() - r).abs() <= 1e-9 * (1.0 + r)).then_some(h)
    };
    match spine {
        Spine::Line(p, d) if d.dot(n).abs() <= 1e-12 => {
            let h = level(p)?;
            Some((Spine::Line(p - n * h, d), h.signum()))
        }
        Spine::Circle(c, a, big) if a.cross(n).length() <= 1e-12 => {
            let h = level(c)?;
            Some((Spine::Circle(c - n * h, a, big), h.signum()))
        }
        _ => None,
    }
}

/// Where two springs on one plane cross, each with the sine of the angle they cross at.
fn crossings(x: Spine, y: Spine) -> Vec<(Point3, f64)> {
    // the tangent of a spring at a point on it
    let tangent = |s: Spine, p: Point3| match s {
        Spine::Line(_, d) => Some(d),
        Spine::Circle(c, a, _) => a.cross(p - c).normalize().map(|u| u.get()),
    };
    let points: Vec<Point3> = match (x, y) {
        (Spine::Line(p, d), Spine::Line(q, e)) => {
            // p + s d = q + t e, in the plane: s from the component square to e
            let w = d - e * d.dot(e);
            let ww = w.dot(w);
            if ww <= 1e-24 {
                return Vec::new();
            }
            vec![p + d * ((q - p).dot(w) / ww)]
        }
        (Spine::Line(p, d), Spine::Circle(c, _, big)) | (Spine::Circle(c, _, big), Spine::Line(p, d)) => {
            // the foot of c on the line, then ± half the chord
            let f = p + d * (c - p).dot(d);
            let m2 = (f - c).length_sq();
            let half2 = big * big - m2;
            if half2 < 0.0 {
                return Vec::new();
            }
            let half = half2.sqrt();
            vec![f - d * half, f + d * half]
        }
        (Spine::Circle(c1, _, r1), Spine::Circle(c2, _, r2)) => {
            let v = c2 - c1;
            let dd = v.length();
            if dd <= 0.0 || dd > r1 + r2 || dd < (r1 - r2).abs() {
                return Vec::new();
            }
            let u = v * (1.0 / dd);
            let along = (dd * dd + r1 * r1 - r2 * r2) / (2.0 * dd);
            let half = (r1 * r1 - along * along).max(0.0).sqrt();
            // square to u in the plane: the circles' common axis crossed with u
            let Spine::Circle(_, a, _) = x else {
                return Vec::new();
            };
            let w = a.cross(u);
            let m = c1 + u * along;
            vec![m - w * half, m + w * half]
        }
    };
    points
        .into_iter()
        .filter_map(|p| {
            let (s, t) = (tangent(x, p)?, tangent(y, p)?);
            Some((p, s.cross(t).length()))
        })
        .collect()
}

/// A node: the springs' crossing, the distance to the nearest other crossing of the same two
/// springs (infinite if none), and how far from it both caps are single sheets (`reach_of`).
#[derive(Clone, Copy, Debug)]
struct Node {
    at: Point3,
    apart: f64,
    radius: f64,
}

/// Whether one plane can lie a radius over both spines, parallel to each, on one side: the cheap
/// test before any face is read. Its normal n is square to every spine direction (a circle's:
/// along its axis), and the spines' offsets along it differ by the radii's difference.
fn one_plane_fits((x, rx): (Spine, f64), (y, ry): (Spine, f64)) -> bool {
    let level = |n: Vec3, p: Point3, q: Point3| {
        ((q - p).dot(n).abs() - (rx - ry).abs()).abs() <= 1e-9 * (1.0 + rx + ry)
    };
    match (x, y) {
        (Spine::Line(p, d), Spine::Line(q, e)) => match d.cross(e).normalize() {
            Some(n) => level(n.get(), p, q),
            // parallel lines: a plane square to their common way through both levels
            None => true,
        },
        (Spine::Line(p, d), Spine::Circle(c, n, _)) | (Spine::Circle(c, n, _), Spine::Line(p, d)) => {
            d.dot(n).abs() <= 1e-12 && level(n, p, c)
        }
        (Spine::Circle(c, a, _), Spine::Circle(k, b, _)) => a.cross(b).length() <= 1e-12 && level(a, c, k),
    }
}

/// The tangent nodes of two tubes: for each plane lying a radius over both spines, parallel to
/// each, on one side (there are at most two: the spines' common level plus or minus the radius),
/// the crossings of their springs at a sine of at least `SIN_LEAST`.
fn nodes(ta: (Spine, f64), tb: (Spine, f64)) -> Vec<Node> {
    let (x, rx) = ta;
    let (p, n) = match (x, tb.0) {
        (Spine::Line(p, d), Spine::Line(_, e)) => match d.cross(e).normalize() {
            Some(n) => (p, n.get()),
            // parallel springs never cross
            None => return Vec::new(),
        },
        (Spine::Line(p, _), Spine::Circle(_, a, _)) => (p, a),
        (Spine::Circle(c, a, _), _) => (c, a),
    };
    let reach = reach_of(ta).min(reach_of(tb));
    let mut out: Vec<Node> = Vec::new();
    for side in [1.0, -1.0] {
        let o = p + n * (side * rx);
        let (Some((xa, sa)), Some((xb, sb))) = (spring(ta, o, n), spring(tb, o, n)) else {
            continue;
        };
        // both spines under the plane, on one side: their caps meet there, not a cap and a far side
        if sa != sb {
            continue;
        }
        let all = crossings(xa, xb);
        for (k, &(at, sin)) in all.iter().enumerate() {
            if sin < SIN_LEAST {
                continue;
            }
            let apart = all
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != k)
                .map(|(_, q)| q.0.distance(at))
                .fold(f64::INFINITY, f64::min);
            out.push(Node {
                at,
                apart,
                radius: reach,
            });
        }
    }
    out
}

/// Traced curves of surfaces `a` and `b` (forge-bool `ssi::search`'s), mended at their tangent nodes (see the
/// module's note); as they came wherever that does not apply.
#[cold]
#[inline(never)]
pub fn mend(a: &Surface, b: &Surface, curves: Vec<IntCurve>) -> Vec<IntCurve> {
    let (Some(ta), Some(tb)) = (tube(a), tube(b)) else {
        return curves;
    };
    if !one_plane_fits(ta, tb) {
        return curves;
    }
    let found = nodes(ta, tb);
    if found.is_empty() {
        return curves;
    }
    mended(a, b, &curves, &found).unwrap_or(curves)
}

/// A chain of points on both surfaces as a curve, as forge-geom's trace ends one: its sag (each
/// chord's middle from the curve, 1.5 times, plus LINEAR) and its directions (na x nb, turned to
/// follow the chain; the chord's where the normals are parallel).
fn chain(sa: &Surface, sb: &Surface, pts: Vec<Point3>, step: f64) -> IntCurve {
    let n = pts.len();
    let sag = (0..n - 1)
        .map(|i| {
            let m = pts[i].lerp(pts[i + 1], 0.5);
            refine(sa, sb, m).map_or(step, |q| q.distance(m))
        })
        .fold(0.0, f64::max);
    let tans = (0..n)
        .map(|i| {
            let along = pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)];
            let d = direction(sa, sb, pts[i])
                .or_else(|| along.normalize().map(|u| u.get()))
                .unwrap_or(Vec3::ZERO);
            if d.dot(along) < 0.0 { -d } else { d }
        })
        .collect();
    IntCurve::new(sa.clone(), sb.clone(), pts, tans, false, 1.5 * sag + tol::LINEAR)
}

/// A chain's longest step.
fn step_of(k: &IntCurve) -> f64 {
    k.pts
        .windows(2)
        .map(|w| w[0].distance(w[1]))
        .fold(0.0, f64::max)
        .max(10.0 * tol::LINEAR)
}

/// The stretches of a chain away from the nodes, each with the node it was cut at before and after.
type Stretch = (Option<usize>, Vec<Point3>, Option<usize>);

fn stretches(k: &IntCurve, near: impl Fn(Point3) -> Option<usize>) -> Vec<Stretch> {
    let n = k.pts.len();
    let mut out: Vec<Stretch> = Vec::new();
    let mut cur: Option<(Option<usize>, Vec<Point3>)> = None;
    let mut last: Option<usize> = None;
    // a closed chain is read from a point off the nodes, round to it again
    let start = if k.closed {
        (0..n).find(|&i| near(k.pts[i]).is_none()).unwrap_or(0)
    } else {
        0
    };
    let count = if k.closed { n + 1 } else { n };
    for j in 0..count {
        let p = k.pts[(start + j) % n];
        match near(p) {
            Some(m) => {
                if let Some((from, pts)) = cur.take() {
                    out.push((from, pts, Some(m)));
                }
                last = Some(m);
            }
            None => match &mut cur {
                Some((_, pts)) => pts.push(p),
                None => cur = Some((last.take(), vec![p])),
            },
        }
    }
    if let Some((from, pts)) = cur {
        out.push((from, pts, None));
    }
    out
}

/// `mend`'s work: None where any step of it does not go through (the trace is then kept whole).
fn mended(sa: &Surface, sb: &Surface, curves: &[IntCurve], nodes: &[Node]) -> Option<Vec<IntCurve>> {
    let mut runs: Vec<IntCurve> = Vec::new();
    let mut out: Vec<IntCurve> = Vec::new();
    for k in curves {
        let step = step_of(k);
        let reach = 2.0 * step;
        let near = |p: Point3| nodes.iter().position(|m| m.at.distance(p) <= reach);
        if !k.pts.iter().any(|&p| near(p).is_some()) {
            out.push(k.clone());
            continue;
        }
        // the mend stays where the one-branch argument holds: well inside both caps, and clear of
        // any other crossing of the springs
        if nodes
            .iter()
            .any(|m| 4.0 * reach > m.radius || 4.0 * reach > m.apart)
        {
            return None;
        }
        for (from, mut pts, to) in stretches(k, near) {
            if let Some(m) = from {
                let head = towards_touch(sa, sb, pts[0], nodes[m].at, 0.25 * step)?;
                let mut h = head.pts;
                h.reverse();
                h.pop();
                h.extend(pts);
                pts = h;
            }
            if let Some(m) = to {
                let tail = towards_touch(sa, sb, *pts.last()?, nodes[m].at, 0.25 * step)?;
                pts.extend(tail.pts.into_iter().skip(1));
            }
            if pts.len() < 2 {
                continue;
            }
            let mut r = chain(sa, sb, pts, step);
            // at a node the normals are parallel and their cross product says nothing: the way in
            // is the last chord's
            let n = r.pts.len();
            let chord = |i: usize, j: usize| (r.pts[j] - r.pts[i]).normalize().map(|u| u.get());
            if from.is_some()
                && let Some(d) = chord(0, 1)
            {
                r.tans[0] = d;
            }
            if to.is_some()
                && let Some(d) = chord(n - 2, n - 1)
            {
                r.tans[n - 1] = d;
            }
            // one branch traced from both of its ends: the second runs along the first all the way
            let dup = runs.iter().any(|o| {
                let w = o.sag + r.sag + 2.0 * tol::LINEAR;
                let (hi_r, hi_o) = (r.domain().hi, o.domain().hi);
                (1..8).all(|i| {
                    let p = CurveGeom::eval(&r, hi_r * i as f64 / 8.0);
                    let t = CurveGeom::closest(o, p, Interval::new(0.0, hi_o));
                    CurveGeom::eval(o, t).distance(p) <= w
                })
            });
            if !dup {
                runs.push(r);
            }
        }
    }
    out.extend(runs);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line and a circle on z = 0 cross where the chord says, at the angle the radius says.
    #[test]
    fn a_line_crosses_a_circle_where_its_chord_ends() {
        let line = Spine::Line(Point3::new(0.0, 3.0, 0.0), Vec3::X);
        let circle = Spine::Circle(Point3::ORIGIN, Vec3::Z, 5.0);
        let mut got = crossings(line, circle);
        got.sort_by(|p, q| p.0.x.total_cmp(&q.0.x));
        assert_eq!(got.len(), 2);
        assert!(got[0].0.distance(Point3::new(-4.0, 3.0, 0.0)) <= 1e-12);
        assert!(got[1].0.distance(Point3::new(4.0, 3.0, 0.0)) <= 1e-12);
        // the radius at (4, 3) makes 0.8 with the line's normal: the tangent's sine with x is 0.8
        assert!((got[1].1 - 0.8).abs() <= 1e-12, "{}", got[1].1);
    }

    /// Two circles of radius 5 and 5, centres 6 apart, cross at (3, ±4).
    #[test]
    fn two_circles_cross_on_their_common_chord() {
        let x = Spine::Circle(Point3::ORIGIN, Vec3::Z, 5.0);
        let y = Spine::Circle(Point3::new(6.0, 0.0, 0.0), Vec3::Z, 5.0);
        let mut got = crossings(x, y);
        got.sort_by(|p, q| p.0.y.total_cmp(&q.0.y));
        assert_eq!(got.len(), 2);
        assert!(got[0].0.distance(Point3::new(3.0, -4.0, 0.0)) <= 1e-12);
        assert!(got[1].0.distance(Point3::new(3.0, 4.0, 0.0)) <= 1e-12);
    }

    /// A tube's spring is its spine on the plane, only when the spine lies a radius under it,
    /// parallel.
    #[test]
    fn a_spring_needs_the_spine_a_radius_under_the_plane() {
        let o = Point3::new(0.0, 0.0, 6.0);
        let cyl = (Spine::Line(Point3::new(0.0, 50.0, -5.0), Vec3::X), 11.0);
        let Some((Spine::Line(p, _), side)) = spring(cyl, o, Vec3::Z) else {
            panic!()
        };
        assert!(p.distance(Point3::new(0.0, 50.0, 6.0)) <= 1e-12 && side < 0.0);
        // a radius off by a micron: not tangent
        assert!(spring((cyl.0, 11.001), o, Vec3::Z).is_none());
        // tilted: not parallel
        let tilted = (
            Spine::Line(Point3::new(0.0, 50.0, -5.0), Vec3::new(0.6, 0.0, 0.8)),
            11.0,
        );
        assert!(spring(tilted, o, Vec3::Z).is_none());
    }
}
