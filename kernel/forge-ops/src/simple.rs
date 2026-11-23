//! Whether a profile loop with a free curve in it is simple, decided on its exact pieces
//! (research/spline-loops/lean/SplineSpec/Simple.lean holds the rule and the facts used here).
//!
//! Every segment is a chain of rational Bezier pieces: a line one of degree 1, an arc rational
//! quadratics of a quarter turn or less, a spline its own pieces. Joints within the tolerance
//! are made exact. The loop is refused:
//! - when two pieces that are not neighbours come within the kernel's tolerance (`tol::LINEAR`);
//! - when two neighbours meet anywhere but their joint, a piece crossing itself among them.
//!
//! Pairs are subdivided until each is decided:
//! - control boxes farther apart than the pair's allowance: they do not meet (`apart_of_boxes`);
//! - at a joint, the control points either side strictly on either side of a line through it:
//!   the two meet only there (`cone_sep`, `convex_in_cone`);
//! - a piece whose derivative keeps a positive component along one direction (the Bernstein
//!   coefficients of its hodograph's numerator, `bern_pos`) does not meet itself;
//! - two evaluated points within the allowance, or two nearly straight pieces whose chords
//!   cross by more than their flatness, meet: refused there.
//!
//! What the subdivision cannot decide within its budget is refused there, undecided.

use forge_math::predicates::orient2d;
use forge_math::{Point2, Vec2, tol};

use crate::profile::{ProfileLoop, Segment};

/// Homogeneous control points (x w, y w, w), w > 0.
type H = [f64; 3];

/// How a loop stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Simplicity {
    Simple,
    /// Two of its parts meet here: cross, touch, or come within the tolerance.
    Meets(Point2),
    /// Not decided here within the search's budget.
    Undecided(Point2),
}

fn pt(h: &H) -> Point2 {
    Point2::new(h[0] / h[2], h[1] / h[2])
}

/// de Casteljau at s, in homogeneous coordinates.
fn split(c: &[H], s: f64) -> (Vec<H>, Vec<H>) {
    let n = c.len();
    let mut w = c.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = vec![[0.0; 3]; n];
    left.push(w[0]);
    right[n - 1] = w[n - 1];
    for r in 1..n {
        for j in 0..n - r {
            let (lo, hi) = w.split_at_mut(j + 1);
            for (x, y) in lo[j].iter_mut().zip(hi[0]) {
                *x += (y - *x) * s;
            }
        }
        left.push(w[0]);
        right[n - 1 - r] = w[n - 1 - r];
    }
    (left, right)
}

#[derive(Clone, Copy)]
struct Bx {
    lo: Point2,
    hi: Point2,
}

fn bbox(c: &[H]) -> Bx {
    let mut b = Bx {
        lo: Point2::new(f64::INFINITY, f64::INFINITY),
        hi: Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };
    for h in c {
        let p = pt(h);
        b.lo = Point2::new(b.lo.x.min(p.x), b.lo.y.min(p.y));
        b.hi = Point2::new(b.hi.x.max(p.x), b.hi.y.max(p.y));
    }
    b
}

fn gap(a: &Bx, b: &Bx) -> f64 {
    (b.lo.x - a.hi.x)
        .max(a.lo.x - b.hi.x)
        .max(b.lo.y - a.hi.y)
        .max(a.lo.y - b.hi.y)
}

fn diag(b: &Bx) -> f64 {
    (b.hi - b.lo).length()
}

fn centre(a: &Bx, b: &Bx) -> Point2 {
    Point2::new(
        0.25 * (a.lo.x + a.hi.x + b.lo.x + b.hi.x),
        0.25 * (a.lo.y + a.hi.y + b.lo.y + b.hi.y),
    )
}

fn push_spline(out: &mut Vec<Vec<H>>, s: &forge_geom::spline::Spline) {
    let (_, pts) = s.homogeneous();
    for row in pts.chunks(s.degree() + 1) {
        out.push(row.iter().map(|h| [h[0], h[1], h[3]]).collect());
    }
}

/// The loop's pieces in order, joints made exact (each piece starts where the one before ends),
/// at least three of them (halving the first while fewer).
fn pieces(l: &ProfileLoop) -> Vec<Vec<H>> {
    let mut out = Vec::new();
    for (s, _) in &l.segments {
        match s {
            Segment::Line { a, b } => out.push(vec![[a.x, a.y, 1.0], [b.x, b.y, 1.0]]),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let c = forge_math::Point3::new(center.x, center.y, 0.0);
                let x = forge_math::Vec3::new(*radius, 0.0, 0.0);
                let y = forge_math::Vec3::new(0.0, *radius, 0.0);
                if let Some(e) = forge_geom::spline::Spline::ellipse(c, x, y, *start, *sweep) {
                    push_spline(&mut out, &e);
                }
            }
            Segment::Circle { .. } => {}
            Segment::Spline(c) => push_spline(&mut out, &c.c),
        }
    }
    let n = out.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let e = pt(out[i].last().expect("a piece"));
        let w = out[j][0][2];
        out[j][0] = [e.x * w, e.y * w, w];
    }
    while !out.is_empty() && out.len() < 3 {
        let first = out.remove(0);
        let (l, r) = split(&first, 0.5);
        out.insert(0, r);
        out.insert(0, l);
    }
    out
}

fn binom(n: usize, k: usize) -> f64 {
    let mut r = 1.0;
    for i in 0..k {
        r = r * (n - i) as f64 / (i + 1) as f64;
    }
    r
}

/// Whether the piece's derivative keeps a positive component along a direction (its chord or the
/// mean of its control legs): then it is one to one. The derivative points along A' W - A W' (A
/// the homogeneous point, W the weight), of degree 2p - 1; its Bernstein coefficients, all past a
/// margin for their rounding, bound it.
fn one_to_one(c: &[H]) -> bool {
    let p = c.len() - 1;
    let mut legs = Vec2::new(0.0, 0.0);
    for k in 0..p {
        legs += pt(&c[k + 1]) - pt(&c[k]);
    }
    let dirs = [pt(&c[p]) - pt(&c[0]), legs];
    let da: Vec<H> = (0..p)
        .map(|k| {
            let (u, v) = (c[k + 1], c[k]);
            let f = p as f64;
            [f * (u[0] - v[0]), f * (u[1] - v[1]), f * (u[2] - v[2])]
        })
        .collect();
    let q = 2 * p - 1;
    let mut nx = vec![0.0; q + 1];
    let mut ny = vec![0.0; q + 1];
    let mut size = 0.0f64;
    for (j, dj) in da.iter().enumerate() {
        for (k, ck) in c.iter().enumerate() {
            let f = binom(p - 1, j) * binom(p, k) / binom(q, j + k);
            nx[j + k] += f * (dj[0] * ck[2] - ck[0] * dj[2]);
            ny[j + k] += f * (dj[1] * ck[2] - ck[1] * dj[2]);
            size = size.max(
                f * ((dj[0] * ck[2]).abs()
                    + (ck[0] * dj[2]).abs()
                    + (dj[1] * ck[2]).abs()
                    + (ck[1] * dj[2]).abs()),
            );
        }
    }
    let margin = 1e-12 * size * (q + 1) as f64;
    dirs.into_iter().any(|d| {
        d.normalize()
            .is_some_and(|u| (0..=q).all(|i| u.x * nx[i] + u.y * ny[i] > margin))
    })
}

/// Whether A's control points (all but its last, the joint J) and B's (all but its first, J)
/// lie strictly on either side of a line through J.
fn cones_apart(a: &[H], b: &[H]) -> bool {
    let j = pt(&a[a.len() - 1]);
    let v: Vec<Vec2> = a[..a.len() - 1]
        .iter()
        .map(|h| pt(h) - j)
        .filter(|d| d.length() > 0.0)
        .collect();
    let w: Vec<Vec2> = b[1..]
        .iter()
        .map(|h| pt(h) - j)
        .filter(|d| d.length() > 0.0)
        .collect();
    if v.is_empty() || w.is_empty() {
        return false;
    }
    let ok = |n: Vec2| {
        v.iter().all(|d| n.dot(*d) < -1e-9 * d.length()) && w.iter().all(|d| n.dot(*d) > 1e-9 * d.length())
    };
    // a separating line, if any, can be turned to bisect a ray of each cone: try those
    for x in &v {
        for y in &w {
            let (Some(ux), Some(uy)) = (x.normalize(), y.normalize()) else {
                continue;
            };
            // the two bisectors of the rays, each a line between them
            let d = ux - uy;
            for l in [ux + uy, Vec2::new(-d.y, d.x)] {
                if let Some(l) = l.normalize() {
                    let n = Vec2::new(-l.y, l.x);
                    if ok(n) || ok(n * -1.0) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// The same allowing a tangent joint (a knife edge): a line through J and a control point with
/// A's control points on it or behind it and B's on it or ahead, B's far end strictly ahead (or
/// A's far end strictly behind). Then B is strictly ahead but at J (its offsets' Bernstein
/// coefficients nonnegative, the last positive) and A is not, so they meet only at J. The sides
/// are exact (`orient2d`).
fn half_planes_apart(a: &[H], b: &[H]) -> bool {
    let j = pt(&a[a.len() - 1]);
    let pa: Vec<Point2> = a[..a.len() - 1].iter().map(pt).collect();
    let pb: Vec<Point2> = b[1..].iter().map(pt).collect();
    for &p in pa.iter().chain(&pb) {
        if p == j {
            continue;
        }
        let side = |x: Point2| orient2d(j, p, x);
        for sign in [1.0, -1.0] {
            let sa: Vec<f64> = pa.iter().map(|&x| sign * side(x)).collect();
            let sb: Vec<f64> = pb.iter().map(|&x| sign * side(x)).collect();
            if sa.iter().all(|&x| x <= 0.0)
                && sb.iter().all(|&x| x >= 0.0)
                && (sb.last().is_some_and(|&x| x > 0.0) || sa.first().is_some_and(|&x| x < 0.0))
            {
                return true;
            }
        }
    }
    false
}

/// The largest distance of a piece's control points from its chord.
fn flatness(c: &[H]) -> f64 {
    let (a, b) = (pt(&c[0]), pt(&c[c.len() - 1]));
    let d = b - a;
    let l = d.length();
    c.iter()
        .map(|h| {
            let p = pt(h) - a;
            if l > 0.0 {
                (d.x * p.y - d.y * p.x).abs() / l
            } else {
                p.length()
            }
        })
        .fold(0.0, f64::max)
}

/// Two nearly straight pieces that cross for certain: each one's ends farther than both
/// flatnesses on either side of the other's chord.
fn surely_cross(a: &[H], b: &[H]) -> Option<Point2> {
    let f = flatness(a) + flatness(b);
    let (p0, p1) = (pt(&a[0]), pt(&a[a.len() - 1]));
    let (q0, q1) = (pt(&b[0]), pt(&b[b.len() - 1]));
    let side = |a: Point2, b: Point2, x: Point2| {
        let d = b - a;
        let l = d.length();
        if l > 0.0 {
            (d.x * (x - a).y - d.y * (x - a).x) / l
        } else {
            0.0
        }
    };
    let apart = |x: f64, y: f64| (x > f && y < -f) || (x < -f && y > f);
    (apart(side(p0, p1, q0), side(p0, p1, q1)) && apart(side(q0, q1, p0), side(q0, q1, p1)))
        .then(|| p0.lerp(p1, 0.5))
}

struct Search {
    budget: usize,
    tiny: f64,
}

impl Search {
    /// Two pieces with no joint, meeting when within `near`.
    fn apart(&mut self, a: &[H], b: &[H], near: f64, depth: usize) -> Simplicity {
        let (ba, bb) = (bbox(a), bbox(b));
        if gap(&ba, &bb) > near + self.tiny {
            return Simplicity::Simple;
        }
        for p in [pt(&a[0]), pt(&a[a.len() - 1])] {
            for q in [pt(&b[0]), pt(&b[b.len() - 1])] {
                if p.distance(q) <= near {
                    return Simplicity::Meets(p.lerp(q, 0.5));
                }
            }
        }
        if let Some(x) = surely_cross(a, b) {
            return Simplicity::Meets(x);
        }
        if self.budget == 0 || depth > 64 || diag(&ba).max(diag(&bb)) < self.tiny {
            return Simplicity::Undecided(centre(&ba, &bb));
        }
        self.budget -= 1;
        let (big, other, first) = if diag(&ba) >= diag(&bb) {
            (a, b, true)
        } else {
            (b, a, false)
        };
        let (l, r) = split(big, 0.5);
        for half in [l, r] {
            let x = if first {
                self.apart(&half, other, near, depth + 1)
            } else {
                self.apart(other, &half, near, depth + 1)
            };
            if x != Simplicity::Simple {
                return x;
            }
        }
        Simplicity::Simple
    }

    /// Neighbours: A ends where B starts. They may meet only there.
    fn neighbours(&mut self, a: &[H], b: &[H], depth: usize) -> Simplicity {
        if cones_apart(a, b) || half_planes_apart(a, b) {
            return Simplicity::Simple;
        }
        let (ba, bb) = (bbox(a), bbox(b));
        if self.budget == 0 || depth > 64 || diag(&ba).max(diag(&bb)) < self.tiny {
            return Simplicity::Undecided(pt(&a[a.len() - 1]));
        }
        self.budget -= 1;
        let tiny = self.tiny;
        if diag(&ba) >= diag(&bb) {
            let (l, r) = split(a, 0.5);
            match self.apart(&l, b, tiny, depth + 1) {
                Simplicity::Simple => self.neighbours(&r, b, depth + 1),
                x => x,
            }
        } else {
            let (l, r) = split(b, 0.5);
            match self.apart(a, &r, tiny, depth + 1) {
                Simplicity::Simple => self.neighbours(a, &l, depth + 1),
                x => x,
            }
        }
    }

    /// A piece against itself.
    fn single(&mut self, c: &[H], depth: usize) -> Simplicity {
        if one_to_one(c) {
            return Simplicity::Simple;
        }
        if self.budget == 0 || depth > 64 {
            let b = bbox(c);
            return Simplicity::Undecided(b.lo.lerp(b.hi, 0.5));
        }
        self.budget -= 1;
        let (l, r) = split(c, 0.5);
        for x in [self.single(&l, depth + 1), self.single(&r, depth + 1)] {
            if x != Simplicity::Simple {
                return x;
            }
        }
        self.neighbours(&l, &r, depth + 1)
    }
}

/// Whether a loop is simple (see the module's head).
pub fn simplicity(l: &ProfileLoop) -> Simplicity {
    let ps = pieces(l);
    let n = ps.len();
    if n == 0 {
        return Simplicity::Simple;
    }
    let boxes: Vec<Bx> = ps.iter().map(|c| bbox(c)).collect();
    let size = boxes
        .iter()
        .map(|b| b.lo.to_vec().length().max(b.hi.to_vec().length()))
        .fold(1.0f64, f64::max);
    let mut s = Search {
        budget: 1_000_000,
        tiny: 1e-12 * size,
    };
    for c in &ps {
        let x = s.single(c, 0);
        if x != Simplicity::Simple {
            return x;
        }
    }
    // pairs, swept by their boxes' x extent
    let eps = tol::LINEAR;
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by(|&i, &j| boxes[i].lo.x.total_cmp(&boxes[j].lo.x));
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if boxes[j].lo.x > boxes[i].hi.x + eps {
                break;
            }
            let (i, j) = (i.min(j), i.max(j));
            let x = if j == i + 1 {
                s.neighbours(&ps[i], &ps[j], 0)
            } else if i == 0 && j == n - 1 {
                s.neighbours(&ps[j], &ps[i], 0)
            } else {
                s.apart(&ps[i], &ps[j], eps, 0)
            };
            if x != Simplicity::Simple {
                return x;
            }
        }
    }
    Simplicity::Simple
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::spline::PlaneSpline;
    use forge_math::predicates::orient2d;
    use std::sync::Arc;

    fn closed(pts: &[(f64, f64)]) -> ProfileLoop {
        let mut p: Vec<Point2> = pts.iter().map(|&(x, y)| Point2::new(x, y)).collect();
        p.push(p[0]);
        let s = PlaneSpline::through_periodic(p).expect("a closed spline");
        ProfileLoop::new(vec![(Segment::Spline(Arc::new(s)), 0)])
    }

    /// Whether the polygon of `per` samples a piece crosses itself (two edges properly cross).
    fn polygon_crosses(l: &ProfileLoop, per: usize) -> bool {
        let Segment::Spline(c) = &l.segments[0].0 else {
            unreachable!()
        };
        let d = c.c.domain();
        let n = per * c.c.pieces();
        let p: Vec<Point2> = (0..n)
            .map(|i| {
                let q = c.c.point(d.lo + d.width() * i as f64 / n as f64);
                Point2::new(q.x, q.y)
            })
            .collect();
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let (a, b, c, e) = (p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]);
                let s = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
                if s(orient2d(a, b, c), orient2d(a, b, e)) && s(orient2d(c, e, a), orient2d(c, e, b)) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn a_round_loop_is_simple() {
        let l = closed(&[(0.0, 0.0), (10.0, 1.0), (12.0, 8.0), (4.0, 11.0), (-2.0, 6.0)]);
        assert_eq!(simplicity(&l), Simplicity::Simple);
    }

    #[test]
    fn a_figure_eight_meets_itself() {
        let l = closed(&[
            (0.0, 0.0),
            (10.0, 10.0),
            (20.0, 0.0),
            (10.0, -10.0),
            (0.0, 10.0),
            (-10.0, 0.0),
        ]);
        assert!(
            matches!(simplicity(&l), Simplicity::Meets(_)),
            "{:?}",
            simplicity(&l)
        );
    }

    /// The repro, in my own numbers: a long hairpin whose tip, drawn through points close to a
    /// cusp, turns a small loop. Sixteen samples a piece (what the outline check took) see no
    /// crossing; four thousand do, and the exact search finds it.
    #[test]
    fn a_near_cusp_hairpin_that_crosses_is_refused() {
        let l = closed(&HAIRPIN);
        assert!(!polygon_crosses(&l, 16), "the sampled outline sees it after all");
        assert!(polygon_crosses(&l, 4000), "it does not cross");
        assert!(
            matches!(simplicity(&l), Simplicity::Meets(_)),
            "{:?}",
            simplicity(&l)
        );
    }

    const HAIRPIN: [(f64, f64); 8] = [
        (0.0, 0.0),
        (20.0, 1.47),
        (40.0, 0.035),
        (40.7, -0.001),
        (39.77, -0.035),
        (20.0, -1.25),
        (0.0, -0.41),
        (-5.0, -3.0),
    ];

    /// Search for a repro (run by hand: cargo test -p forge-ops find_hairpin -- --ignored).
    #[test]
    #[ignore]
    fn find_hairpin() {
        let mut r = forge_math::rng::Rng::new(7);
        for _ in 0..20000 {
            let tip = 40.0 + r.range(0.05, 1.5);
            let w = r.range(0.02, 0.6);
            let pts = [
                (0.0, 0.0),
                (20.0, r.range(0.2, 2.0)),
                (40.0, w),
                (tip, r.range(-0.2, 0.2)),
                (40.0 + r.range(-0.5, 0.5), -w),
                (20.0, -r.range(0.2, 2.0)),
                (0.0, -r.range(0.0, 0.5)),
                (-5.0, -3.0),
            ];
            let l = closed(&pts);
            if !polygon_crosses(&l, 16) && polygon_crosses(&l, 4000) {
                println!("{pts:?} -> {:?}", simplicity(&l));
                return;
            }
        }
        panic!("none found");
    }
}
