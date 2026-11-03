//! Trims: an exact 2D region boolean, a wall's loops minus the union of cuts, on profile segments.
//! Every piece is split at every meeting; each sub-piece is kept or dropped by what lies on its two
//! sides at its midpoint (kept where the region is on exactly one side), and the kept ones chain
//! into loops. Pinches and near-coincident edges are refused, naming the edges, never snapped.
//! Measured against forge-bool on a flat prism and an exact scanline: research/sheet-metal/.
use forge_geom::spline::Spline;
use forge_geom::spline::meet::{cross_sections, same_curve};
use forge_math::{Interval, Point2, Point3, Vec2, Vec3, dmath, predicates, tol};
use forge_ops::{ProfileLoop, Segment};
use std::collections::HashMap;
use std::sync::Arc;

const TOL: f64 = tol::LINEAR;
/// How far a piece may lie from a boundary and still be along it. Inputs meant to coincide do so to
/// rounding; a wider band swallows real pieces beside a tangent contact and unbalances the chain.
const ALONG: f64 = 1e-9;
/// Side by side closer than this and not `ALONG`: the tolerance cannot tell them from coincident.
const NEAR: f64 = 2.0 * TOL;

/// A piece of a loop. Arcs run from angle t0 to t1, counter-clockwise when t1 > t0.
#[derive(Clone, Debug)]
pub enum Piece {
    Line { a: Point2, b: Point2 },
    Arc { c: Point2, r: f64, t0: f64, t1: f64 },
    Spline { s: Arc<Spline>, t0: f64, t1: f64 },
}

fn p2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}

impl Piece {
    /// The point at fraction u.
    pub fn at(&self, u: f64) -> Point2 {
        match *self {
            Piece::Line { a, b } => a.lerp(b, u),
            Piece::Arc { c, r, t0, t1 } => {
                let (s, k) = dmath::sin_cos(t0 + (t1 - t0) * u);
                Point2::new(c.x + r * k, c.y + r * s)
            }
            Piece::Spline { ref s, t0, t1 } => p2(s.point(t0 + (t1 - t0) * u)),
        }
    }
    pub fn tangent(&self, u: f64) -> Vec2 {
        match *self {
            Piece::Line { a, b } => b - a,
            Piece::Arc { r, t0, t1, .. } => {
                let (s, k) = dmath::sin_cos(t0 + (t1 - t0) * u);
                Vec2::new(-s, k) * (r * (t1 - t0))
            }
            Piece::Spline { ref s, t0, t1 } => {
                let d = s.deriv(t0 + (t1 - t0) * u).d1 * (t1 - t0);
                Vec2::new(d.x, d.y)
            }
        }
    }
    pub fn sub(&self, u0: f64, u1: f64) -> Piece {
        match *self {
            Piece::Line { .. } => Piece::Line {
                a: self.at(u0),
                b: self.at(u1),
            },
            Piece::Arc { c, r, t0, t1 } => Piece::Arc {
                c,
                r,
                t0: t0 + (t1 - t0) * u0,
                t1: t0 + (t1 - t0) * u1,
            },
            Piece::Spline { ref s, t0, t1 } => Piece::Spline {
                s: s.clone(),
                t0: t0 + (t1 - t0) * u0,
                t1: t0 + (t1 - t0) * u1,
            },
        }
    }
    pub fn reversed(&self) -> Piece {
        self.sub(1.0, 0.0)
    }
    /// The integral of x dy along it, closed form but for splines (Gauss on each of its spans).
    pub fn area_term(&self) -> f64 {
        match *self {
            Piece::Line { a, b } => 0.5 * a.to_vec().cross(b.to_vec()),
            Piece::Arc { c, r, t0, t1 } => {
                let (s0, c0) = dmath::sin_cos(t0);
                let (s1, c1) = dmath::sin_cos(t1);
                0.5 * r * r * (t1 - t0) + 0.5 * c.to_vec().cross(Vec2::new(r * (c1 - c0), r * (s1 - s0)))
            }
            Piece::Spline { ref s, t0, t1 } => {
                let mut cuts = vec![t0.min(t1)];
                cuts.extend(s.knots_in(Interval::new(t0.min(t1), t0.max(t1))));
                cuts.push(t0.max(t1));
                let mut sum = 0.0;
                for w in cuts.windows(2) {
                    for (x, g) in GL {
                        let t = 0.5 * (w[0] + w[1]) + 0.5 * (w[1] - w[0]) * x;
                        let d = s.deriv(t);
                        sum += 0.25 * (w[1] - w[0]) * g * (d.p.x * d.d1.y - d.p.y * d.d1.x);
                    }
                }
                if t1 >= t0 { sum } else { -sum }
            }
        }
    }
    pub(crate) fn bbox(&self) -> Bx {
        match *self {
            Piece::Line { a, b } => Bx::of(a).hull(Bx::of(b)),
            Piece::Arc { c, r, t0, t1 } => {
                let mut b = Bx::of(self.at(0.0)).hull(Bx::of(self.at(1.0)));
                let (lo, hi) = (t0.min(t1), t0.max(t1));
                let q = dmath::PI / 2.0;
                let mut k = (lo / q).ceil();
                while k * q <= hi {
                    let (s, co) = dmath::sin_cos(k * q);
                    b = b.hull(Bx::of(Point2::new(c.x + r * co, c.y + r * s)));
                    k += 1.0;
                }
                b
            }
            Piece::Spline { ref s, t0, t1 } => {
                let b = s.bbox(Interval::new(t0.min(t1), t0.max(t1)));
                Bx {
                    x0: b.x.lo,
                    x1: b.x.hi,
                    y0: b.y.lo,
                    y1: b.y.hi,
                }
            }
        }
    }
    /// The nearest point to p on the piece, as a fraction.
    pub fn nearest(&self, p: Point2) -> f64 {
        match *self {
            Piece::Line { a, b } => ((p - a).dot(b - a) / (b - a).length_sq().max(1e-300)).clamp(0.0, 1.0),
            Piece::Arc { .. } => {
                let f = self.arc_param(p);
                if (0.0..=1.0).contains(&f) {
                    f
                } else if self.at(0.0).distance(p) <= self.at(1.0).distance(p) {
                    0.0
                } else {
                    1.0
                }
            }
            Piece::Spline { ref s, t0, t1 } => {
                let t = s.closest(Point3::new(p.x, p.y, 0.0), Interval::new(t0.min(t1), t0.max(t1)));
                (t - t0) / (t1 - t0)
            }
        }
    }
    /// The fraction of p's angle along an arc, its own turn nearest the arc's middle.
    fn arc_param(&self, p: Point2) -> f64 {
        let Piece::Arc { c, t0, t1, .. } = *self else {
            return 0.0;
        };
        let mid = 0.5 * (t0 + t1);
        let mut a = dmath::atan2(p.y - c.y, p.x - c.x);
        a += dmath::TAU * ((mid - a) / dmath::TAU).round();
        (a - t0) / (t1 - t0)
    }
    pub fn to_segment(&self) -> Option<Segment> {
        Some(match *self {
            Piece::Line { a, b } => Segment::Line { a, b },
            Piece::Arc { c, r, t0, t1 } => Segment::Arc {
                center: c,
                radius: r,
                start: t0,
                sweep: t1 - t0,
            },
            Piece::Spline { ref s, t0, t1 } => {
                let snap = |t: f64| {
                    s.breaks()
                        .iter()
                        .copied()
                        .find(|b| (b - t).abs() <= 1e-9)
                        .unwrap_or(t)
                };
                let (lo, hi) = (snap(t0.min(t1)), snap(t0.max(t1)));
                let c = s.stretch(Interval::new(lo, hi))?;
                let c = if t1 >= t0 { c } else { c.reversed() };
                let (degree, knots, pts, weights) = c.control_form();
                let plane = forge_geom::spline::PlaneSpline::control(
                    degree,
                    pts.iter().map(|p| Point2::new(p.x, p.y)).collect(),
                    Some(knots),
                    weights,
                )?;
                Segment::Spline(Arc::new(plane))
            }
        })
    }
}

/// 7-point Gauss-Legendre.
const GL: [(f64, f64); 7] = [
    (0.0, 0.417_959_183_673_469_4),
    (0.405_845_151_377_397_2, 0.381_830_050_505_118_9),
    (-0.405_845_151_377_397_2, 0.381_830_050_505_118_9),
    (0.741_531_185_599_394_4, 0.279_705_391_489_276_7),
    (-0.741_531_185_599_394_4, 0.279_705_391_489_276_7),
    (0.949_107_912_342_758_5, 0.129_484_966_168_869_7),
    (-0.949_107_912_342_758_5, 0.129_484_966_168_869_7),
];

#[derive(Clone, Copy, Debug)]
pub(crate) struct Bx {
    pub(crate) x0: f64,
    pub(crate) x1: f64,
    pub(crate) y0: f64,
    pub(crate) y1: f64,
}

impl Bx {
    const EMPTY: Bx = Bx {
        x0: f64::INFINITY,
        x1: f64::NEG_INFINITY,
        y0: f64::INFINITY,
        y1: f64::NEG_INFINITY,
    };
    fn of(p: Point2) -> Bx {
        Bx {
            x0: p.x,
            x1: p.x,
            y0: p.y,
            y1: p.y,
        }
    }
    fn hull(self, o: Bx) -> Bx {
        Bx {
            x0: self.x0.min(o.x0),
            x1: self.x1.max(o.x1),
            y0: self.y0.min(o.y0),
            y1: self.y1.max(o.y1),
        }
    }
    fn holds(&self, p: Point2, g: f64) -> bool {
        p.x >= self.x0 - g && p.x <= self.x1 + g && p.y >= self.y0 - g && p.y <= self.y1 + g
    }
    fn meets(&self, o: &Bx, g: f64) -> bool {
        self.x0 <= o.x1 + g && o.x0 <= self.x1 + g && self.y0 <= o.y1 + g && o.y0 <= self.y1 + g
    }
}

fn line_line(a: Point2, b: Point2, c: Point2, d: Point2, out: &mut Vec<(f64, f64)>) {
    let (ab, cd) = (b - a, d - c);
    let (la, lc) = (ab.length(), cd.length());
    // the signed distances of c, d from ab
    let (dc, dd) = (
        predicates::orient2d(a, b, c) / la.max(1e-300),
        predicates::orient2d(a, b, d) / la.max(1e-300),
    );
    let (da, db) = (
        predicates::orient2d(c, d, a) / lc.max(1e-300),
        predicates::orient2d(c, d, b) / lc.max(1e-300),
    );
    if dc.abs() <= TOL && dd.abs() <= TOL {
        return; // along each other: `ends_on` splits at the ends of the overlap
    }
    if (dc > TOL && dd > TOL)
        || (dc < -TOL && dd < -TOL)
        || (da > TOL && db > TOL)
        || (da < -TOL && db < -TOL)
    {
        return;
    }
    let den = da - db;
    let den2 = dc - dd;
    if den == 0.0 || den2 == 0.0 {
        return;
    }
    let (u, v) = (da / den, dc / den2);
    let (ga, gc) = (TOL / la.max(1e-300), TOL / lc.max(1e-300));
    if u >= -ga && u <= 1.0 + ga && v >= -gc && v <= 1.0 + gc {
        out.push((u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)));
    }
}

/// Where the circle (c, r) meets the line through a, b, as points.
fn circle_line(c: Point2, r: f64, a: Point2, b: Point2) -> Vec<Point2> {
    let d = b - a;
    let l = d.length();
    let u = d * (1.0 / l.max(1e-300));
    let n = Vec2::new(-u.y, u.x);
    let h = n.dot(c - a);
    let foot = c - n * h;
    if h.abs() > r + TOL {
        return vec![];
    }
    let w2 = r * r - h * h;
    if w2 <= 0.0 {
        return vec![foot];
    }
    let w = w2.sqrt();
    vec![foot - u * w, foot + u * w]
}

fn circle_circle(c0: Point2, r0: f64, c1: Point2, r1: f64) -> Vec<Point2> {
    let v = c1 - c0;
    let d = v.length();
    if d <= TOL && (r0 - r1).abs() <= TOL {
        return vec![];
    }
    if d > r0 + r1 + TOL || d < (r0 - r1).abs() - TOL || d == 0.0 {
        return vec![];
    }
    let u = v * (1.0 / d);
    let a = (d * d + r0 * r0 - r1 * r1) / (2.0 * d);
    let m = c0 + u * a;
    let h2 = r0 * r0 - a * a;
    if h2 <= 0.0 {
        return vec![c0 + u * (if a >= 0.0 { r0 } else { -r0 })];
    }
    let h = h2.sqrt();
    let n = Vec2::new(-u.y, u.x);
    vec![m - n * h, m + n * h]
}

/// All the fractions on x and on y where the two pieces meet.
fn meet(x: &Piece, y: &Piece, ux: &mut Vec<f64>, uy: &mut Vec<f64>) {
    let mut pts: Vec<Point2> = Vec::new();
    match (x, y) {
        (&Piece::Line { a, b }, &Piece::Line { a: c, b: d }) => {
            let mut hits = Vec::new();
            line_line(a, b, c, d, &mut hits);
            for (u, v) in hits {
                ux.push(u);
                uy.push(v);
            }
        }
        (&Piece::Line { a, b }, &Piece::Arc { c, r, .. })
        | (&Piece::Arc { c, r, .. }, &Piece::Line { a, b }) => {
            pts = circle_line(c, r, a, b);
        }
        (&Piece::Arc { c, r, .. }, &Piece::Arc { c: e, r: q, .. }) => pts = circle_circle(c, r, e, q),
        (&Piece::Spline { ref s, t0, t1 }, other) | (other, &Piece::Spline { ref s, t0, t1 }) => {
            let range = Interval::new(t0.min(t1), t0.max(t1));
            let ts: Vec<f64> = match *other {
                Piece::Line { a, b } => {
                    let d = b - a;
                    let n = Vec2::new(-d.y, d.x) * (1.0 / d.length().max(1e-300));
                    s.plane_roots(Vec3::new(n.x, n.y, 0.0), -n.dot(a.to_vec()), range, TOL)
                        .into_iter()
                        .map(|r| r.0)
                        .collect()
                }
                Piece::Arc { c, r, .. } => {
                    let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
                    s.quadric_roots(
                        Point3::new(c.x, c.y, 0.0),
                        &id,
                        Vec3::ZERO,
                        -r * r,
                        range,
                        TOL * r,
                    )
                    .into_iter()
                    .map(|r| r.0)
                    .collect()
                }
                Piece::Spline { .. } => vec![],
            };
            // the spline's own parameter stands: `closest` could settle on another near stretch
            let x_is_spline = matches!(x, Piece::Spline { s: q, .. } if Arc::ptr_eq(q, s));
            // two splines: each's splits from its own roots, no nearest point in it
            if let Piece::Spline { s: q, t0: a0, t1: a1 } = other {
                let rb = Interval::new(a0.min(*a1), a0.max(*a1));
                if same_curve(s, q) {
                    return;
                }
                let on_s = cross_sections(s, range, q, rb);
                let on_q = cross_sections(q, rb, s, range);
                let (fs, fq): (Vec<f64>, Vec<f64>) = (
                    on_s.iter().map(|r| (r.0 - t0) / (t1 - t0)).collect(),
                    on_q.iter().map(|r| (r.0 - a0) / (a1 - a0)).collect(),
                );
                if x_is_spline {
                    ux.extend(fs);
                    uy.extend(fq);
                } else {
                    ux.extend(fq);
                    uy.extend(fs);
                }
                return;
            }
            for t in ts {
                let p = p2(s.point(t));
                let f = (t - t0) / (t1 - t0);
                let v = other.nearest(p);
                if other.at(v).distance(p) <= 2.0 * TOL {
                    if x_is_spline {
                        ux.push(f);
                        uy.push(v);
                    } else {
                        ux.push(v);
                        uy.push(f);
                    }
                }
            }
        }
    }
    for p in pts {
        let (u, v) = (x.nearest(p), y.nearest(p));
        if x.at(u).distance(p) <= 2.0 * TOL && y.at(v).distance(p) <= 2.0 * TOL {
            ux.push(u);
            uy.push(v);
        }
    }
}

/// Splits y wherever an end of x lies on it (this is what finds every overlap's ends).
fn ends_on(x: &Piece, y: &Piece, by: Bx, ux: &mut Vec<f64>, uy: &mut Vec<f64>) {
    for (k, e) in [x.at(0.0), x.at(1.0)].into_iter().enumerate() {
        if !by.holds(e, TOL) {
            continue;
        }
        let v = y.nearest(e);
        if y.at(v).distance(e) <= TOL {
            ux.push(k as f64);
            uy.push(v);
        }
    }
}

/// A loop's pieces from profile segments, each with its segment's id; a whole circle is two arcs.
pub fn pieces(l: &ProfileLoop) -> Vec<(Piece, u64)> {
    let mut out = Vec::with_capacity(l.segments.len() + 1);
    for (s, id) in &l.segments {
        match s {
            Segment::Line { a, b } => out.push((Piece::Line { a: *a, b: *b }, *id)),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => out.push((
                Piece::Arc {
                    c: *center,
                    r: *radius,
                    t0: *start,
                    t1: start + sweep,
                },
                *id,
            )),
            Segment::Circle { center, radius, ccw } => {
                let k = if *ccw { 1.0 } else { -1.0 };
                for i in 0..2 {
                    out.push((
                        Piece::Arc {
                            c: *center,
                            r: *radius,
                            t0: k * dmath::PI * i as f64,
                            t1: k * dmath::PI * (i + 1) as f64,
                        },
                        *id,
                    ));
                }
            }
            Segment::Spline(ps) => {
                let d = ps.c.domain();
                out.push((
                    Piece::Spline {
                        s: ps.c.clone(),
                        t0: d.lo,
                        t1: d.hi,
                    },
                    *id,
                ));
            }
        }
    }
    out
}

/// A piece of a result loop: it starts at `start` (shared exactly with the previous piece's end)
/// and comes from segment `id` of the input.
#[derive(Clone, Debug)]
pub struct Bound {
    pub piece: Piece,
    pub id: u64,
    pub start: Point2,
}

/// One connected piece of the result: its outer loop counter-clockwise, holes clockwise, each
/// running with the region on its left.
#[derive(Clone, Debug)]
pub struct Component {
    pub outer: Vec<Bound>,
    pub holes: Vec<Vec<Bound>>,
}

#[derive(Clone, Debug, Default)]
pub struct Region {
    pub parts: Vec<Component>,
}

/// Green's area of a loop: the polygon through its vertices plus each piece's bulge off its chord,
/// so the loop has no gaps where an arc ends a rounding off its vertex.
pub fn loop_area(l: &[Bound]) -> f64 {
    let n = l.len();
    (0..n)
        .map(|i| {
            let (a, b) = (l[i].start, l[(i + 1) % n].start);
            let p = &l[i].piece;
            0.5 * a.to_vec().cross(b.to_vec()) + p.area_term()
                - 0.5 * p.at(0.0).to_vec().cross(p.at(1.0).to_vec())
        })
        .sum()
}

impl Region {
    pub fn area(&self) -> f64 {
        self.parts
            .iter()
            .map(|c| loop_area(&c.outer) + c.holes.iter().map(|h| loop_area(h)).sum::<f64>())
            .sum()
    }
}

/// Where the region's boundary touches itself: segments `a` and `b` meet at `at`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionPinch {
    pub at: Point2,
    pub a: u64,
    pub b: u64,
    pub width: f64,
    pub limit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegionError {
    /// Every point where the boundary touches itself.
    Pinched { pinches: Vec<RegionPinch> },
    /// Segments `a` and `b` run side by side `gap` apart at `at`, closer than the tolerance can
    /// tell from coincident, and are not coincident.
    NearCoincident {
        a: u64,
        b: u64,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// The kept pieces do not chain at `at`: `leaving` leave it and `arriving` reach it.
    Unchained {
        at: Point2,
        edge: u64,
        leaving: u32,
        arriving: u32,
    },
    /// A hole that no outer loop holds.
    HoleOutside { edge: u64, at: Point2 },
}

struct Member {
    group: usize,
    pieces: Vec<(Piece, u64)>,
    boxes: Vec<Bx>,
    bx: Bx,
}

/// How far the test ray is turned from +x. Sketches are symmetric, so a midpoint is often level with
/// a circle's centre or a joint; neighbours' rounded end heights could then straddle a level ray and
/// the crossing be lost. No sketch has anything at this angle.
const RAY: f64 = 0.013_7;

/// Is p inside the closed loop, by crossings of the ray from p at angle `RAY`.
fn inside<'a>(l: impl IntoIterator<Item = &'a Piece>, p: Point2) -> bool {
    let (sn, cs) = dmath::sin_cos(RAY);
    // x' along the ray, y' across it
    let turn = |q: Point2| Point2::new(cs * q.x + sn * q.y, -sn * q.x + cs * q.y);
    let pt = turn(p);
    let mut odd = false;
    for q in l {
        match *q {
            Piece::Line { a, b } => {
                let (a, b) = (turn(a), turn(b));
                if (a.y > pt.y) != (b.y > pt.y) {
                    let o = predicates::orient2d(a, b, pt);
                    if (b.y > a.y) == (o > 0.0) {
                        odd = !odd;
                    }
                }
            }
            Piece::Arc { c, r, t0, t1 } => {
                let c = turn(c);
                if (pt.y - c.y).abs() > r {
                    continue;
                }
                // y'-monotone stretches between the quarter points where sin turns
                let (lo, hi) = (t0.min(t1) - RAY, t0.max(t1) - RAY);
                let half = dmath::PI;
                let mut a = lo;
                let mut k = ((lo - half / 2.0) / half).ceil();
                loop {
                    let b = (k * half + half / 2.0).min(hi);
                    if b > a {
                        let (ya, yb) = (c.y + r * dmath::sin(a), c.y + r * dmath::sin(b));
                        if (ya > pt.y) != (yb > pt.y) {
                            let (_, cm) = dmath::sin_cos(0.5 * (a + b));
                            let dy = pt.y - c.y;
                            let x = c.x + cm.signum() * (r * r - dy * dy).max(0.0).sqrt();
                            if x > pt.x {
                                odd = !odd;
                            }
                        }
                    }
                    if b >= hi {
                        break;
                    }
                    a = b;
                    k += 1.0;
                }
            }
            Piece::Spline { ref s, t0, t1 } => {
                let range = Interval::new(t0.min(t1), t0.max(t1));
                for (t, touch) in s.plane_roots(Vec3::new(-sn, cs, 0.0), -pt.y, range, 1e-12) {
                    let d = s.deriv(t);
                    if cs * d.p.x + sn * d.p.y <= pt.x {
                        continue;
                    }
                    // an end on the ray counts as below it, as a line's ends do
                    let dy = -sn * d.d1.x + cs * d.d1.y;
                    let cross = if t - range.lo <= 1e-9 * range.width() {
                        dy > 0.0
                    } else if range.hi - t <= 1e-9 * range.width() {
                        dy < 0.0
                    } else {
                        !touch
                    };
                    if cross {
                        odd = !odd;
                    }
                }
            }
        }
    }
    odd
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    In,
    Out,
    /// On the group's boundary, its inside on the piece's left.
    OnLeft,
    /// On the group's boundary, its inside on the piece's right.
    OnRight,
}

/// Vertices by position: the first point within tolerance names them all.
#[derive(Default)]
struct Verts {
    grid: HashMap<(i64, i64), Vec<u32>>,
    pts: Vec<Point2>,
}

impl Verts {
    fn id(&mut self, p: Point2) -> u32 {
        let h = 4.0 * TOL;
        let (i, j) = ((p.x / h).floor() as i64, (p.y / h).floor() as i64);
        for a in i - 1..=i + 1 {
            for b in j - 1..=j + 1 {
                if let Some(l) = self.grid.get(&(a, b)) {
                    for &v in l {
                        if self.pts[v as usize].distance(p) <= 0.9 * TOL {
                            return v;
                        }
                    }
                }
            }
        }
        let v = self.pts.len() as u32;
        self.pts.push(p);
        self.grid.entry((i, j)).or_default().push(v);
        v
    }
}

/// A kept sub-piece, running with the region on its left.
struct Kept {
    from: u32,
    to: u32,
    piece: Piece,
    id: u64,
    /// The source piece and fractions along it, to merge neighbours back together.
    src: usize,
    u: (f64, f64),
}

/// `wall` (outline counter-clockwise, then holes clockwise) minus the union of `cuts` (each one
/// counter-clockwise loop). Segment ids name the edges in the result and in refusals.
pub fn subtract(wall: &[ProfileLoop], cuts: &[ProfileLoop]) -> Result<Region, RegionError> {
    let member = |group: usize, l: &ProfileLoop| {
        let ps = pieces(l);
        let boxes: Vec<Bx> = ps.iter().map(|p| p.0.bbox()).collect();
        let bx = boxes.iter().fold(Bx::EMPTY, |b, p| b.hull(*p));
        Member {
            group,
            pieces: ps,
            boxes,
            bx,
        }
    };
    let mut members: Vec<Member> = Vec::with_capacity(wall.len() + cuts.len());
    members.extend(wall.iter().map(|l| member(0, l)));
    members.extend(cuts.iter().enumerate().map(|(i, l)| member(i + 1, l)));
    // every piece, with a box, sorted along x for the broad phase
    let mut all: Vec<(usize, usize, Bx)> = Vec::new();
    for (m, mm) in members.iter().enumerate() {
        for (k, b) in mm.boxes.iter().enumerate() {
            all.push((m, k, *b));
        }
    }
    let n = all.len();
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_by(|a, b| all[*a as usize].2.x0.total_cmp(&all[*b as usize].2.x0));
    let mut us: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; n];
    for (ai, &a) in order.iter().enumerate() {
        let (ma, ka, ba) = all[a as usize];
        for &b in &order[ai + 1..] {
            let (mb, kb, bb) = all[b as usize];
            if bb.x0 > ba.x1 + 2.0 * TOL {
                break;
            }
            if members[ma].group == members[mb].group || !ba.meets(&bb, 2.0 * TOL) {
                continue;
            }
            let (x, y) = (&members[ma].pieces[ka].0, &members[mb].pieces[kb].0);
            let (mut ux, mut uy) = (Vec::new(), Vec::new());
            meet(x, y, &mut ux, &mut uy);
            ends_on(x, y, bb, &mut ux, &mut uy);
            ends_on(y, x, ba, &mut uy, &mut ux);
            us[a as usize].extend(ux);
            us[b as usize].extend(uy);
        }
    }
    // groups by box, on a grid sized to the input, so each midpoint asks only the groups near it
    let hull = members.iter().fold(Bx::EMPTY, |b, m| b.hull(m.bx));
    let side = ((members.len() as f64).sqrt().ceil() as usize).clamp(1, 64);
    let (cw, ch) = (
        (hull.x1 - hull.x0).max(1e-9) / side as f64,
        (hull.y1 - hull.y0).max(1e-9) / side as f64,
    );
    let cell = |x: f64, y: f64| {
        (
            (((x - hull.x0) / cw) as isize).clamp(0, side as isize - 1) as usize,
            (((y - hull.y0) / ch) as isize).clamp(0, side as isize - 1) as usize,
        )
    };
    let mut grid: Vec<Vec<u32>> = vec![Vec::new(); side * side];
    for (m, mm) in members.iter().enumerate() {
        let (i0, j0) = cell(mm.bx.x0 - 2.0 * TOL, mm.bx.y0 - 2.0 * TOL);
        let (i1, j1) = cell(mm.bx.x1 + 2.0 * TOL, mm.bx.y1 + 2.0 * TOL);
        for i in i0..=i1 {
            for j in j0..=j1 {
                grid[i * side + j].push(m as u32);
            }
        }
    }
    let group_count = cuts.len() + 1;
    let group_loops = |g: usize| -> std::ops::Range<usize> {
        if g == 0 {
            0..wall.len()
        } else {
            wall.len() + g - 1..wall.len() + g
        }
    };
    // the first near miss seen: (own id, other id, where, gap)
    let mut near_miss: Option<(u64, u64, Point2, f64)> = None;
    let mut side_of = |g: usize, own: (usize, u64), p: Point2, dir: Vec2, also: [Point2; 2]| -> Side {
        if g == own.0 {
            return Side::OnLeft;
        }
        // on the boundary of g? its tangent says which side. A piece that only passes close
        // (near a tangent contact) has other points off it, and the ray decides.
        for m in group_loops(g) {
            if !members[m].bx.holds(p, 2.0 * TOL) {
                continue;
            }
            for ((q, qid), bq) in members[m].pieces.iter().zip(&members[m].boxes) {
                if !bq.holds(p, 2.0 * TOL) {
                    continue;
                }
                let v = q.nearest(p);
                let gap = q.at(v).distance(p);
                // (a short piece only near a tangent contact is no such thing: curves part as s^2)
                if near_miss.is_none()
                    && gap > ALONG
                    && gap <= NEAR
                    && also[0].distance(also[1]) > 0.1
                    && also.iter().all(|&e| q.at(q.nearest(e)).distance(e) <= NEAR)
                {
                    let t = q.tangent(v);
                    if (t.dot(dir) / (t.length() * dir.length()).max(1e-300)).abs() > 0.99 {
                        near_miss = Some((own.1, *qid, p, gap));
                    }
                }
                if gap <= ALONG && also.iter().all(|&e| q.at(q.nearest(e)).distance(e) <= ALONG) {
                    let t = q.tangent(v);
                    let c = t.dot(dir) / (t.length() * dir.length()).max(1e-300);
                    if c > 0.99 {
                        return Side::OnLeft;
                    }
                    if c < -0.99 {
                        return Side::OnRight;
                    }
                }
            }
        }
        let mut odd = false;
        for m in group_loops(g) {
            if members[m].bx.holds(p, 0.0) && inside(members[m].pieces.iter().map(|q| &q.0), p) {
                odd = !odd;
            }
        }
        if odd { Side::In } else { Side::Out }
    };
    let mut verts = Verts::default();
    let mut kept: Vec<Kept> = Vec::new();
    let mut near: Vec<u32> = Vec::new();
    let mut seen = vec![usize::MAX; group_count];
    let mut key: HashMap<(u32, u32), Vec<Point2>> = HashMap::new();
    let mut sub_id = 0;
    for (idx, &(m, k, _)) in all.iter().enumerate() {
        let (source, sid) = (&members[m].pieces[k].0, members[m].pieces[k].1);
        let own = members[m].group;
        let list = &mut us[idx];
        list.sort_by(f64::total_cmp);
        for w in 0..list.len() - 1 {
            let (u0, u1) = (list[w], list[w + 1]);
            if u1 - u0 <= 0.0 {
                continue;
            }
            let (pa, pb) = (source.at(u0), source.at(u1));
            let (va, vb) = (verts.id(pa), verts.id(pb));
            let um = 0.5 * (u0 + u1);
            let mp = source.at(um);
            if va == vb && mp.distance(pa) <= 2.0 * TOL {
                continue; // too short to see
            }
            let dir = source.tangent(um);
            sub_id += 1;
            let (i, j) = cell(mp.x, mp.y);
            near.clear();
            near.extend(&grid[i * side + j]);
            let (mut left_in, mut right_in) = (true, true);
            // the wall first: most pieces leave here
            let also = [source.at(u0 + 0.25 * (u1 - u0)), source.at(u0 + 0.75 * (u1 - u0))];
            match side_of(0, (own, sid), mp, dir, also) {
                Side::In => {}
                Side::Out => {
                    left_in = false;
                    right_in = false;
                }
                Side::OnLeft => right_in = false,
                Side::OnRight => left_in = false,
            }
            for &q in &near {
                if !(left_in || right_in) {
                    break;
                }
                let g = members[q as usize].group;
                if g == 0 || seen[g] == sub_id {
                    continue;
                }
                seen[g] = sub_id;
                match side_of(g, (own, sid), mp, dir, also) {
                    Side::In => {
                        left_in = false;
                        right_in = false;
                    }
                    Side::Out => {}
                    Side::OnLeft => left_in = false,
                    Side::OnRight => right_in = false,
                }
            }
            if own != 0 && seen[own] != sub_id && left_in {
                left_in = false; // a cut's own inside, had the grid missed it
            }
            if left_in == right_in {
                continue;
            }
            let (from, to, piece, u) = if left_in {
                (va, vb, source.sub(u0, u1), (u0, u1))
            } else {
                (vb, va, source.sub(u1, u0), (u1, u0))
            };
            // a line ends exactly on its vertices, so loops close
            let piece = match piece {
                Piece::Line { .. } => Piece::Line {
                    a: verts.pts[from as usize],
                    b: verts.pts[to as usize],
                },
                p => p,
            };
            // where boundaries coincide, each keeps the same piece: once is enough
            let at = key.entry((from, to)).or_default();
            if at.iter().any(|q| q.distance(mp) <= 4.0 * TOL) {
                continue;
            }
            at.push(mp);
            kept.push(Kept {
                from,
                to,
                piece,
                id: sid,
                src: idx,
                u,
            });
        }
    }
    if let Some((a, b, at, gap)) = near_miss {
        return Err(RegionError::NearCoincident {
            a,
            b,
            at,
            gap,
            limit: NEAR,
        });
    }
    chain(kept, &verts.pts)
}

fn chain(kept: Vec<Kept>, pts: &[Point2]) -> Result<Region, RegionError> {
    let mut out_of: Vec<Vec<u32>> = vec![Vec::new(); pts.len()];
    let mut into: Vec<Vec<u32>> = vec![Vec::new(); pts.len()];
    for (i, e) in kept.iter().enumerate() {
        out_of[e.from as usize].push(i as u32);
        into[e.to as usize].push(i as u32);
    }
    let mut pinches = Vec::new();
    for v in 0..pts.len() {
        let (o, a) = (out_of[v].len(), into[v].len());
        if o != a {
            let e = out_of[v].first().or(into[v].first()).copied().unwrap_or(0);
            return Err(RegionError::Unchained {
                at: pts[v],
                edge: kept[e as usize].id,
                leaving: o as u32,
                arriving: a as u32,
            });
        }
        if o > 1 {
            pinches.push(RegionPinch {
                at: pts[v],
                a: kept[into[v][0] as usize].id,
                b: kept[into[v][1] as usize].id,
                width: 0.0,
                limit: TOL,
            });
        }
    }
    if !pinches.is_empty() {
        return Err(RegionError::Pinched { pinches });
    }
    // every vertex has one piece in and one out: the loops follow
    let mut used = vec![false; kept.len()];
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for start in 0..kept.len() {
        if used[start] {
            continue;
        }
        let mut lp = Vec::new();
        let mut at = start as u32;
        while !used[at as usize] {
            used[at as usize] = true;
            lp.push(at);
            at = out_of[kept[at as usize].to as usize][0];
        }
        if at != start as u32 {
            let e = &kept[at as usize];
            return Err(RegionError::Unchained {
                at: pts[e.from as usize],
                edge: e.id,
                leaving: 1,
                arriving: 1,
            });
        }
        loops.push(lp);
    }
    let mut parts: Vec<Component> = Vec::new();
    let mut holes: Vec<Vec<Bound>> = Vec::new();
    for l in &loops {
        let b = merge(l, &kept, pts);
        let a = loop_area(&b);
        if a > 0.0 {
            parts.push(Component {
                outer: b,
                holes: Vec::new(),
            });
        } else if a < 0.0 {
            holes.push(b);
        }
    }
    for h in holes {
        let at = h[0].piece.at(0.5);
        let mut best: Option<(f64, usize)> = None;
        for (i, c) in parts.iter().enumerate() {
            let a = loop_area(&c.outer);
            if best.is_none_or(|b| a < b.0) && holds_hole(&c.outer, &h) {
                best = Some((a, i));
            }
        }
        let Some((_, i)) = best else {
            return Err(RegionError::HoleOutside { edge: h[0].id, at });
        };
        parts[i].holes.push(h);
    }
    Ok(Region { parts })
}

/// Whether `outer` holds the hole: a hole may touch it, so a few of its points vote.
fn holds_hole(outer: &[Bound], h: &[Bound]) -> bool {
    let mut votes = 0i32;
    for p in h {
        for u in [0.3, 0.7] {
            votes += if inside(outer.iter().map(|b| &b.piece), p.piece.at(u)) {
                1
            } else {
                -1
            };
        }
    }
    votes > 0
}

/// Neighbours from one source piece, end to end, become one piece again.
fn merge(l: &[u32], kept: &[Kept], pts: &[Point2]) -> Vec<Bound> {
    let mut out: Vec<(usize, (f64, f64), Bound)> = Vec::with_capacity(l.len());
    for &i in l {
        let e = &kept[i as usize];
        if let Some(last) = out.last_mut()
            && last.0 == e.src
            && last.1.1 == e.u.0
        {
            last.1.1 = e.u.1;
            last.2.piece = stretch(&last.2.piece, &e.piece);
            continue;
        }
        let b = Bound {
            piece: e.piece.clone(),
            id: e.id,
            start: pts[e.from as usize],
        };
        out.push((e.src, e.u, b));
    }
    if out.len() > 1 {
        let (a, b) = (out[out.len() - 1].clone(), out[0].clone());
        if a.0 == b.0 && a.1.1 == b.1.0 {
            out[0].1.0 = a.1.0;
            out[0].2.piece = stretch(&a.2.piece, &b.2.piece);
            out[0].2.start = a.2.start;
            out.pop();
        }
    }
    out.into_iter().map(|x| x.2).collect()
}

fn stretch(a: &Piece, b: &Piece) -> Piece {
    match (a, b) {
        (&Piece::Line { a: s, .. }, &Piece::Line { b: e, .. }) => Piece::Line { a: s, b: e },
        (&Piece::Arc { c, r, t0, .. }, &Piece::Arc { t1, .. }) => Piece::Arc { c, r, t0, t1 },
        (&Piece::Spline { ref s, t0, .. }, &Piece::Spline { t1, .. }) => {
            Piece::Spline { s: s.clone(), t0, t1 }
        }
        _ => a.clone(),
    }
}

/// Whether p is in the component.
pub fn holds(c: &Component, p: Point2) -> bool {
    inside(c.outer.iter().map(|b| &b.piece), p)
        && !c.holes.iter().any(|h| inside(h.iter().map(|b| &b.piece), p))
}
