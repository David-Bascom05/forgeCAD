//! Whether one closed loop of a sketch lies inside another, decided with certainty: the winding
//! number of the outer loop about a point of the inner one, counted on the loop's own curves,
//! never on a polygon sampled from them.
//!
//! The count is of signed crossings of the ray from p along +x, each end classed by the
//! half-open rule (above p, or not). A line's crossing is decided by the exact orientation
//! predicate. A curve is split at its parameter's midpoint until each piece's control hull is
//! wholly left of p (no crossing), wholly above or below it (none), or wholly right of it (as
//! many as its ends' classes differ by). Every computed point carries a bound on its rounding
//! error; a piece too close to p to decide within it, or a split point too close to p's height
//! to class, makes the count unknown (None), never a guess. A point on the loop is unknown
//! too. Callers try other points.
//!
//! The loop is the chain its segments make with the gaps between one segment's end and the
//! next's start closed by straight links (segments meet within tolerance, not bit for bit), so
//! the count is of one closed curve.

use forge_math::{Point2, dmath, predicates::orient2d};
use forge_ops::{ProfileLoop, Segment};

const EPS: f64 = f64::EPSILON;
/// Splits a curve takes at most before its count is unknown.
const DEPTH: usize = 48;

/// Whether q is above p's height: the half-open class.
fn above(q: Point2, p: Point2) -> bool {
    q.y > p.y
}

/// The signed crossings of the ray from p with the line a to b: +1 going up through it right of
/// p, -1 going down; None if p is on the line.
fn line(a: Point2, b: Point2, p: Point2) -> Option<i32> {
    let within = a.x.min(b.x) <= p.x && p.x <= a.x.max(b.x) && a.y.min(b.y) <= p.y && p.y <= a.y.max(b.y);
    let o = if within { orient2d(a, b, p) } else { 1.0 };
    if o == 0.0 {
        // p on the line
        return None;
    }
    let (ua, ub) = (above(a, p), above(b, p));
    if ua == ub {
        return Some(0);
    }
    let o = if within { o } else { orient2d(a, b, p) };
    // up: crossing right of p when p is left of a->b; down: when p is right of it
    let right = if ub { o > 0.0 } else { o < 0.0 };
    Some(match (right, ub) {
        (false, _) => 0,
        (true, true) => 1,
        (true, false) => -1,
    })
}

/// A curve piece's ends as points with how far each may be off the true curve's (0: exact).
#[derive(Clone, Copy)]
struct End {
    at: Point2,
    err: f64,
}

/// What a decided piece of a curve is to p.
#[derive(Clone, Copy, PartialEq)]
enum Leaf {
    /// Wholly left of p: no crossing.
    Left,
    /// Wholly above p's height (true) or below it (false): no crossing, and its ends of that class.
    Level(bool),
    /// Wholly right of p: crossings as its ends' classes differ.
    Right,
}

/// What the hull's box says of the piece, if it decides.
fn decide(lo: Point2, hi: Point2, m: f64, p: Point2) -> Option<Leaf> {
    if lo.y > p.y + m {
        Some(Leaf::Level(true))
    } else if hi.y < p.y - m {
        Some(Leaf::Level(false))
    } else if hi.x < p.x - m {
        Some(Leaf::Left)
    } else if lo.x > p.x + m {
        Some(Leaf::Right)
    } else {
        None
    }
}

/// The count over a curve's decided pieces, taken in order as they come. A run of pieces right
/// of p counts its first start's class against its last end's (the split points between them
/// cancel); each of those two is a curve end (exact), or shared with a piece wholly above or
/// below (its class), or classed from its computed point if that is far enough from p's height
/// to be certain. Nothing is kept but the run open now.
struct Count {
    p: Point2,
    total: i32,
    prev: Option<Leaf>,
    /// The open run's start class and its last piece's end.
    run: Option<(bool, End)>,
    sure: bool,
}

impl Count {
    fn new(p: Point2) -> Count {
        Count {
            p,
            total: 0,
            prev: None,
            run: None,
            sure: true,
        }
    }

    /// An end's class: exact, or the class of the piece beside it wholly above or below, or its
    /// own point's if far enough from p's height.
    fn class(&mut self, e: End, beside: Option<Leaf>) -> bool {
        if e.err == 0.0 {
            return above(e.at, self.p);
        }
        if let Some(Leaf::Level(c)) = beside {
            return c;
        }
        if (e.at.y - self.p.y).abs() <= e.err {
            self.sure = false;
        }
        above(e.at, self.p)
    }

    fn push(&mut self, leaf: Leaf, a: End, b: End) {
        if leaf == Leaf::Right {
            match &mut self.run {
                Some(run) => run.1 = b,
                None => {
                    let c = self.class(a, self.prev);
                    self.run = Some((c, b));
                }
            }
        } else if let Some((start, end)) = self.run.take() {
            let c = self.class(end, Some(leaf));
            self.total += c as i32 - start as i32;
        }
        self.prev = Some(leaf);
    }

    fn finish(mut self) -> Option<i32> {
        if let Some((start, end)) = self.run.take() {
            let c = self.class(end, None);
            self.total += c as i32 - start as i32;
        }
        self.sure.then_some(self.total)
    }
}

type H = [f64; 4];

/// A rational Bézier piece in homogeneous points, split at its midpoint until decided; its
/// leaves pushed in order. False if some piece is not decided within the splits allowed.
fn bezier(h: &[H], a: End, b: End, p: Point2, depth: usize, out: &mut Count) -> bool {
    // the hull, and how far its points may be off: each midpoint level adds at most one
    // rounding of the largest coordinate, and projecting divides by the least weight
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    let (mut big, mut wmin) = (0.0f64, f64::INFINITY);
    for q in h {
        let (x, y) = (q[0] / q[3], q[1] / q[3]);
        lo = Point2::new(lo.x.min(x), lo.y.min(y));
        hi = Point2::new(hi.x.max(x), hi.y.max(y));
        big = big.max(q[0].abs()).max(q[1].abs()).max(q[3].abs());
        wmin = wmin.min(q[3]);
    }
    if wmin.is_nan() || wmin <= 0.0 || !big.is_finite() {
        return false;
    }
    let m = 4.0 * (depth as f64 + 2.0) * EPS * big / wmin;
    if let Some(leaf) = decide(lo, hi, m, p) {
        out.push(leaf, a, b);
        return true;
    }
    if depth >= DEPTH {
        return false;
    }
    // de Casteljau at the middle
    let n = h.len();
    let mut rows: Vec<H> = h.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    left.push(rows[0]);
    right.push(rows[n - 1]);
    for k in 1..n {
        for i in 0..n - k {
            let next = rows[i + 1];
            for (x, y) in rows[i].iter_mut().zip(next) {
                *x = (*x + y) / 2.0;
            }
        }
        left.push(rows[0]);
        right.push(rows[n - 1 - k]);
    }
    right.reverse();
    let mid = rows[0];
    let e = End {
        at: Point2::new(mid[0] / mid[3], mid[1] / mid[3]),
        err: m,
    };
    bezier(&left, a, e, p, depth + 1, out) && bezier(&right, e, b, p, depth + 1, out)
}

/// An arc piece from angle t0 to t1 (t0 < t1, at most a quarter turn), split at its middle
/// angle until decided; its leaves pushed in order.
fn arc(
    c: Point2,
    r: f64,
    (t0, a): (f64, End),
    (t1, b): (f64, End),
    p: Point2,
    depth: usize,
    out: &mut Count,
) -> bool {
    // the box of the arc: its ends and any axis extreme between them, each point within a few
    // roundings of the trigonometry and the sums
    let mut lo = Point2::new(a.at.x.min(b.at.x), a.at.y.min(b.at.y));
    let mut hi = Point2::new(a.at.x.max(b.at.x), a.at.y.max(b.at.y));
    let q = dmath::FRAC_PI_2;
    let mut k = (t0 / q).ceil();
    while k * q <= t1 {
        let x = forge_ops::profile::arc_point(c, r, k * q);
        lo = Point2::new(lo.x.min(x.x), lo.y.min(x.y));
        hi = Point2::new(hi.x.max(x.x), hi.y.max(x.y));
        k += 1.0;
    }
    let m = 16.0 * EPS * (c.x.abs().max(c.y.abs()) + r) + 8.0 * EPS * r * (1.0 + t0.abs().max(t1.abs()));
    if let Some(leaf) = decide(lo, hi, m, p) {
        out.push(leaf, a, b);
        return true;
    }
    if depth >= DEPTH {
        return false;
    }
    let tm = (t0 + t1) / 2.0;
    let e = End {
        at: forge_ops::profile::arc_point(c, r, tm),
        err: m,
    };
    arc(c, r, (t0, a), (tm, e), p, depth + 1, out) && arc(c, r, (tm, e), (t1, b), p, depth + 1, out)
}

/// An arc from `start` sweeping `sweep` (either way), its ends exactly the segment's.
fn arc_segment(
    c: Point2,
    r: f64,
    start: f64,
    sweep: f64,
    from: Point2,
    to: Point2,
    p: Point2,
) -> Option<i32> {
    // run it counter-clockwise, in pieces of at most a quarter turn; a clockwise one counts
    // the negative
    let (t0, a, b, sign) = if sweep >= 0.0 {
        (start, from, to, 1)
    } else {
        (start + sweep, to, from, -1)
    };
    let span = sweep.abs();
    let n = (span / dmath::FRAC_PI_2).ceil().max(1.0) as usize;
    let exact = |at: Point2| End { at, err: 0.0 };
    let m = 16.0 * EPS * (c.x.abs().max(c.y.abs()) + r) + 8.0 * EPS * r * (1.0 + t0.abs() + span);
    let mut leaves = Count::new(p);
    let mut prev = (t0, exact(a));
    for k in 1..=n {
        let t = if k == n {
            t0 + span
        } else {
            t0 + span * k as f64 / n as f64
        };
        let e = if k == n {
            exact(b)
        } else {
            End {
                at: forge_ops::profile::arc_point(c, r, t),
                err: m,
            }
        };
        if !arc(c, r, prev, (t, e), p, 0, &mut leaves) {
            return None;
        }
        prev = (t, e);
    }
    Some(sign * leaves.finish()?)
}

/// The signed crossings of the ray from p with one segment, its ends as the segment gives them.
fn segment(s: &Segment, p: Point2) -> Option<i32> {
    match s {
        Segment::Line { a, b } => line(*a, *b, p),
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => arc_segment(*center, *radius, *start, *sweep, s.start(), s.end(), p),
        Segment::Circle { center, radius, ccw } => {
            let at = s.start();
            let w = arc_segment(*center, *radius, 0.0, dmath::TAU, at, at, p)?;
            Some(if *ccw { w } else { -w })
        }
        Segment::Spline(c) => {
            let (deg, pts) = c.c.homogeneous();
            let n = deg + 1;
            let pieces = pts.len() / n;
            let mut leaves = Count::new(p);
            let mut a = End {
                at: s.start(),
                err: 0.0,
            };
            for k in 0..pieces {
                let h = &pts[k * n..(k + 1) * n];
                let last = h[n - 1];
                let b = if k + 1 == pieces {
                    End {
                        at: s.end(),
                        err: 0.0,
                    }
                } else {
                    // a joint between pieces: its point as stored, projected
                    End {
                        at: Point2::new(last[0] / last[3], last[1] / last[3]),
                        err: 2.0 * EPS * (last[0].abs().max(last[1].abs()) / last[3]).max(1.0),
                    }
                };
                if !bezier(h, a, b, p, 0, &mut leaves) {
                    return None;
                }
                a = b;
            }
            leaves.finish()
        }
    }
}

/// The winding number of the loop about p, if certain.
pub(crate) fn winding(l: &ProfileLoop, p: Point2) -> Option<i32> {
    let n = l.segments.len();
    let mut total = 0;
    for k in 0..n {
        let s = &l.segments[k].0;
        total += segment(s, p)?;
        // the link from this segment's end to the next one's start
        let next = l.segments[(k + 1) % n].0.start();
        let end = s.end();
        if end != next {
            total += line(end, next, p)?;
        }
    }
    Some(total)
}

/// A box round the loop, every point of it inside (its curves' control hulls, the arcs' boxes).
pub(crate) fn bbox(l: &ProfileLoop) -> (Point2, Point2) {
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    let mut add = |q: Point2| {
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
    };
    for (s, _) in &l.segments {
        match s {
            Segment::Line { a, b } => {
                add(*a);
                add(*b);
            }
            Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                add(Point2::new(center.x - radius, center.y - radius));
                add(Point2::new(center.x + radius, center.y + radius));
            }
            Segment::Spline(c) => {
                for q in c.c.homogeneous().1 {
                    add(Point2::new(q[0] / q[3], q[1] / q[3]));
                }
            }
        }
    }
    (lo, hi)
}

/// Whether loop `inner` lies inside loop `outer` (the two neither cross nor overlap): the
/// winding about the first of `inner`'s segment starts that decides it. None if none does.
pub(crate) fn inside(inner: &ProfileLoop, outer: &ProfileLoop, outer_box: (Point2, Point2)) -> Option<bool> {
    let (lo, hi) = outer_box;
    let pad = 4.0 * EPS * (lo.x.abs().max(lo.y.abs()).max(hi.x.abs()).max(hi.y.abs()) + 1.0);
    for (s, _) in &inner.segments {
        let p = s.start();
        if p.x < lo.x - pad || p.x > hi.x + pad || p.y < lo.y - pad || p.y > hi.y + pad {
            return Some(false);
        }
        if let Some(w) = winding(outer, p) {
            return Some(w != 0);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::spline::PlaneSpline;
    use std::sync::Arc;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> ProfileLoop {
        let c = [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)];
        ProfileLoop::new(
            (0..4)
                .map(|k| {
                    (
                        Segment::Line {
                            a: c[k],
                            b: c[(k + 1) % 4],
                        },
                        k as u64,
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn lines_arcs_and_curves() {
        let sq = square(0.0, 0.0, 10.0, 10.0);
        assert_eq!(winding(&sq, p(5.0, 5.0)), Some(1));
        assert_eq!(winding(&sq, p(15.0, 5.0)), Some(0));
        // level with the corners: the half-open rule counts each side once
        assert_eq!(winding(&sq, p(-5.0, 10.0)), Some(0));
        assert_eq!(winding(&sq, p(-5.0, 0.0)), Some(0));
        assert_eq!(winding(&sq, p(5.0, 10.0 - 1e-12)), Some(1));
        // on the loop: unknown
        assert_eq!(winding(&sq, p(10.0, 5.0)), None);
        let circle = ProfileLoop::new(vec![(
            Segment::Circle {
                center: p(0.0, 0.0),
                radius: 3.0,
                ccw: true,
            },
            0,
        )]);
        assert_eq!(winding(&circle, p(0.0, 0.0)), Some(1));
        assert_eq!(winding(&circle, p(2.9, 0.5)), Some(1));
        assert_eq!(winding(&circle, p(3.1, 0.0)), Some(0));
        assert_eq!(winding(&circle, p(2.2, 2.2)), Some(0));
        // a D: a quadratic bulging right from (0,0) to (0,10) round (10,5), closed by a line
        let q = PlaneSpline::control(2, vec![p(0.0, 0.0), p(10.0, 5.0), p(0.0, 10.0)], None, None)
            .expect("a curve");
        let d = ProfileLoop::new(vec![
            (Segment::Spline(Arc::new(q)), 0),
            (
                Segment::Line {
                    a: p(0.0, 10.0),
                    b: p(0.0, 0.0),
                },
                1,
            ),
        ]);
        // the curve reaches x = 5 at y = 5: just inside and just outside, decided exactly
        assert_eq!(winding(&d, p(4.999_999, 5.0)), Some(1));
        assert_eq!(winding(&d, p(5.000_001, 5.0)), Some(0));
        assert_eq!(winding(&d, p(2.0, 5.0)), Some(1));
        assert_eq!(winding(&d, p(5.0, 9.0)), Some(0));
        assert_eq!(winding(&d, p(5.0, 5.0)), None);
    }
}
