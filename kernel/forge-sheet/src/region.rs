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

/// A quick hash for small integer keys (grid cells, vertex pairs): no keys from outside.
#[derive(Default)]
struct Mix(u64);
impl std::hash::Hasher for Mix {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u32(&mut self, x: u32) {
        self.write_u64(x as u64);
    }
    fn write_i64(&mut self, x: i64) {
        self.write_u64(x as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
type Map<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<Mix>>;
use std::sync::Arc;

const TOL: f64 = tol::LINEAR;
/// Carriers further apart than the tolerance but within this are neither one carrier nor two the
/// tolerance can tell apart (the spread cap of research/kernel-audit/README.md §6.1): refused.
const NEAR: f64 = 2.0 * TOL;

/// |v|², as verify/sheet's Region.lean has `norm2`.
fn norm2(v: Vec2) -> f64 {
    v.x * v.x + v.y * v.y
}

/// `near`: two points within `l` of each other, |p - q|² <= l², the sign exact (`forge_sheet_core::exact::disc`:
/// a float filter, else expansions).
fn near(l: f64, p: Point2, q: Point2) -> bool {
    forge_sheet_core::exact::disc(p.x, p.y, q.x, q.y, l) <= 0.0
}

/// `within`: c within `l` of the line through a and b, orient(a, b, c)² <= l² |b - a|²
/// (Region.lean's `within_iff_dist`), the sign exact (`forge_sheet_core::exact::line_gap`).
fn within(l: f64, a: Point2, b: Point2, c: Point2) -> bool {
    forge_sheet_core::exact::line_gap(a.x, a.y, b.x, b.y, c.x, c.y, l) <= 0.0
}

/// Two pieces' verdict, Region.lean's `Carrier`: on one carrier, two, too near to tell, or a
/// piece of no length. Symmetric in the pair and in each line's direction.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Carrier {
    One,
    Two,
    Ambiguous(f64),
    Degenerate,
}

/// Whether two pieces lie on one carrier, decided once for the pair by the one tolerance rule
/// (README §6.1, Region.lean's `carrier`): lines, all four ends within the tolerance of the other
/// line (`One`), or all within twice it (`Ambiguous`), else `Two`; arcs, centres and radii within
/// it together (twice it ambiguous); splines, one curve. Arcs against lines are never one carrier
/// (a tangent contact is a crossing, split at the touch).
fn carrier(x: &Piece, y: &Piece) -> Carrier {
    match (x, y) {
        (&Piece::Line { a, b }, &Piece::Line { a: c, b: d }) => {
            if norm2(b - a) == 0.0 || norm2(d - c) == 0.0 {
                return Carrier::Degenerate;
            }
            let all =
                |l: f64| within(l, a, b, c) && within(l, a, b, d) && within(l, c, d, a) && within(l, c, d, b);
            if all(TOL) {
                Carrier::One
            } else if all(NEAR) {
                let off =
                    |p: Point2, q: Point2, r: Point2| predicates::orient2d(p, q, r).abs() / p.distance(q);
                Carrier::Ambiguous(off(a, b, c).max(off(a, b, d)).max(off(c, d, a)).max(off(c, d, b)))
            } else {
                Carrier::Two
            }
        }
        (&Piece::Arc { c, r, .. }, &Piece::Arc { c: e, r: q, .. }) => {
            let gap = c.distance(e) + (r - q).abs();
            if gap <= TOL {
                Carrier::One
            } else if gap <= NEAR {
                Carrier::Ambiguous(gap)
            } else {
                Carrier::Two
            }
        }
        (Piece::Spline { s, .. }, Piece::Spline { s: q, .. }) => {
            if same_curve(s, q) {
                Carrier::One
            } else {
                Carrier::Two
            }
        }
        _ => Carrier::Two,
    }
}

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
    pub(crate) fn arc_param(&self, p: Point2) -> f64 {
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
                // a closed spline is two pieces, as a circle is: no edge may end where it starts
                let closed = ps.c.point(d.lo).distance(ps.c.point(d.hi)) <= TOL;
                let cuts = if closed {
                    vec![d.lo, d.mid(), d.hi]
                } else {
                    vec![d.lo, d.hi]
                };
                for w in cuts.windows(2) {
                    out.push((
                        Piece::Spline {
                            s: ps.c.clone(),
                            t0: w[0],
                            t1: w[1],
                        },
                        *id,
                    ));
                }
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

/// `crosses`: the half-open crossing of the horizontal ray from p to +x with the edge a -> b
/// (Region.lean): it counts iff its ends lie on opposite sides of the ray's height, an end level
/// with p counting as below it, and p is left of the edge going up (right going down). Exact:
/// comparisons and orient2d's sign. No turned ray: the half-open rule decides an end level with
/// p, so no sketch's symmetry can lose a crossing.
fn crosses(p: Point2, a: Point2, b: Point2) -> bool {
    (a.y > p.y) != (b.y > p.y) && (b.y > a.y) == (side(a, b, p) > 0.0)
}

/// orient2d(a, b, p)'s sign at p moved by (δ³, δ²), δ → 0 (simulation of simplicity): exact where
/// p is off the line ab; on it, the sign the move gives, (b.x - a.x), else -(b.y - a.y). Never 0
/// for a ≠ b, so an edge run either way counts alike for every p (Region.lean's
/// `crosses_reverse`, there for p off the edge's line, holds for all p), and a point on a chord or
/// a level edge is decided as a point a hair above it is. The half-open height test (`a.y > p.y`)
/// is already this move's.
fn side(a: Point2, b: Point2, p: Point2) -> f64 {
    let o = predicates::orient2d(a, b, p);
    if o != 0.0 {
        o
    } else if b.x != a.x {
        b.x - a.x
    } else {
        a.y - b.y
    }
}

/// `inLens`: p in an arc's lens, the disc's part on the arc's side of its chord a -> b (a
/// counter-clockwise arc lies right of its chord, a clockwise one left, for every sweep).
fn in_lens(p: Point2, a: Point2, b: Point2, c: Point2, r: f64, ccw: bool) -> bool {
    let o = side(a, b, p);
    // in the disc, p moved by (δ³, δ²) as `side` moves it: |p + m - c|² - r² is then
    // 2δ²(p.y - c.y) + 2δ³(p.x - c.x) + O(δ⁴) on the circle, so below its centre's height (or
    // level and left of it) is in
    let s = forge_sheet_core::exact::disc(p.x, p.y, c.x, c.y, r);
    let disc = s < 0.0 || (s == 0.0 && (p.y < c.y || (p.y == c.y && p.x < c.x)));
    disc && if ccw { o < 0.0 } else { o > 0.0 }
}

/// The half-open crossings of the horizontal ray from p with a spline piece from a to b, its
/// ends counted as a line's are (an end level with p is below it): `crosses` for a curve, whose
/// count has the parity of its chord's plus its lens's (Region.lean leaves splines to exact
/// subdivision; this is the same ray, on the curve itself).
fn spline_crossings(p: Point2, s: &Spline, t0: f64, t1: f64) -> bool {
    let range = Interval::new(t0.min(t1), t0.max(t1));
    let mut odd = false;
    for (t, touch) in s.plane_roots(Vec3::new(0.0, 1.0, 0.0), -p.y, range, 1e-12) {
        let d = s.deriv(t);
        if d.p.x <= p.x {
            continue;
        }
        let cross = if t - range.lo <= 1e-9 * range.width() {
            d.d1.y > 0.0
        } else if range.hi - t <= 1e-9 * range.width() {
            d.d1.y < 0.0
        } else {
            !touch
        };
        if cross {
            odd = !odd;
        }
    }
    odd
}

/// `inLoop`: p in the closed loop of pieces, each with its start (a region's: `Bound::start`,
/// the vertex class's point, not the piece's own end recomputed), even-odd: the polygon over the
/// starts XOR
/// each arc's lens (Region.lean), each spline's own crossings for its chord's. The loop's
/// starting piece and the direction it runs do not change it, nor does where it lies
/// (`inPoly_rotate`, `crosses_reverse`, `inPoly_translate`).
fn in_loop<'a>(l: impl IntoIterator<Item = (&'a Piece, Point2)>, p: Point2) -> bool {
    let l: Vec<(&Piece, Point2)> = l.into_iter().collect();
    let n = l.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (l[i].1, l[(i + 1) % n].1);
        match *l[i].0 {
            Piece::Line { .. } => odd ^= crosses(p, a, b),
            Piece::Arc { c, r, t0, t1 } => odd ^= crosses(p, a, b) ^ in_lens(p, a, b, c, r, t1 > t0),
            // the curve's own crossings stand for its chord's and its lens's
            Piece::Spline { ref s, t0, t1 } => odd ^= spline_crossings(p, s, t0, t1),
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

/// Vertex classes: each split point's class, and each class's point.
struct Verts {
    /// Per piece (index into `all`), the class of each of its split points in order.
    class: Vec<Vec<u32>>,
    pts: Vec<Point2>,
}

impl Verts {
    fn of(&self, piece: usize, k: usize) -> u32 {
        self.class[piece][k]
    }

    fn classes(
        all: &[(usize, usize, Bx)],
        members: &[Member],
        us: &[Vec<f64>],
    ) -> Result<Verts, RegionError> {
        // every split point, with its name (its piece's place in the input: the wall's loops,
        // then the cuts, in the order the sheet has them; then its fraction along the piece),
        // where it is, and its segment id
        let mut pts: Vec<(Point2, (usize, usize, f64), u64)> = Vec::new();
        let mut first: Vec<usize> = Vec::with_capacity(all.len());
        for (idx, &(m, k, _)) in all.iter().enumerate() {
            let (piece, id) = (&members[m].pieces[k].0, members[m].pieces[k].1);
            first.push(pts.len());
            for &u in &us[idx] {
                pts.push((piece.at(u), (m, k, u), id));
            }
        }
        let n = pts.len();
        // pairs within the tolerance (`near`), found on a grid of that size
        let cell = |p: Point2| ((p.x / TOL).floor() as i64, (p.y / TOL).floor() as i64);
        let mut grid: Map<(i64, i64), Vec<u32>> = Map::default();
        for (i, q) in pts.iter().enumerate() {
            grid.entry(cell(q.0)).or_default().push(i as u32);
        }
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for i in 0..n {
            let (ci, cj) = cell(pts[i].0);
            for a in ci - 1..=ci + 1 {
                for b in cj - 1..=cj + 1 {
                    let Some(list) = grid.get(&(a, b)) else {
                        continue;
                    };
                    for &j in list {
                        if (j as usize) > i && near(TOL, pts[i].0, pts[j as usize].0) {
                            pairs.push((i as u32, j));
                        }
                    }
                }
            }
        }
        // the classes are forge-sheet-core's (proven): the points come in name order, so each
        // class is numbered by, and is, its least-named point
        let class_of = forge_sheet_core::classes(n, &pairs);
        let count = class_of.iter().map(|&c| c as usize + 1).max().unwrap_or(0);
        let mut rep: Vec<u32> = vec![u32::MAX; count];
        let mut boxes: Vec<Bx> = vec![Bx::EMPTY; count];
        for (i, &c) in class_of.iter().enumerate() {
            if rep[c as usize] == u32::MAX {
                rep[c as usize] = i as u32;
            }
            boxes[c as usize] = boxes[c as usize].hull(Bx::of(pts[i].0));
        }
        for (i, &c) in class_of.iter().enumerate() {
            let b = boxes[c as usize];
            // the box's diagonal within twice the tolerance (Region.lean's `box_spread`: it holds
            // every two points of the class within it, the pairwise cap)
            let (dx, dy) = (b.x1 - b.x0, b.y1 - b.y0);
            if dx * dx + dy * dy > NEAR * NEAR {
                let other = (0..n)
                    .find(|&j| class_of[j] == c && pts[j].2 != pts[i].2)
                    .unwrap_or(i);
                return Err(RegionError::NearCoincident {
                    a: pts[i].2,
                    b: pts[other].2,
                    at: pts[i].0,
                    gap: (dx * dx + dy * dy).sqrt(),
                    limit: 2.0 * TOL,
                });
            }
        }
        let class = (0..all.len())
            .map(|idx| class_of[first[idx]..first[idx] + us[idx].len()].to_vec())
            .collect();
        Ok(Verts {
            class,
            pts: rep.iter().map(|&r| pts[r as usize].0).collect(),
        })
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
    // the pairs on one carrier, decided here once and read wherever the pair meets again
    let mut along: std::collections::HashSet<(u32, u32), std::hash::BuildHasherDefault<Mix>> =
        Default::default();
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
            match carrier(x, y) {
                Carrier::One => {
                    along.insert((a, b));
                    along.insert((b, a));
                }
                Carrier::Two | Carrier::Degenerate => {}
                Carrier::Ambiguous(gap) => {
                    // side by side, the tolerance cannot say one carrier or two
                    let p = y.at(y.nearest(x.at(0.5)));
                    return Err(RegionError::NearCoincident {
                        a: members[ma].pieces[ka].1,
                        b: members[mb].pieces[kb].1,
                        at: p,
                        gap,
                        limit: TOL,
                    });
                }
            }
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
    // each piece's place in `all`, to read the pairs decided above
    let mut index: Vec<Vec<u32>> = members.iter().map(|m| vec![0; m.pieces.len()]).collect();
    for (i, &(m, k, _)) in all.iter().enumerate() {
        index[m][k] = i as u32;
    }
    let side_of = |g: usize, own: (usize, usize), p: Point2, dir: Vec2| -> Side {
        if g == own.0 {
            return Side::OnLeft;
        }
        // on the boundary of g: along a piece of it on this piece's carrier (decided once, for
        // the pair), its tangent saying which side; the piece is split at the overlap's ends, so
        // its middle is on the overlap or off it
        for m in group_loops(g) {
            if !members[m].bx.holds(p, TOL) {
                continue;
            }
            for (k, ((q, _), bq)) in members[m].pieces.iter().zip(&members[m].boxes).enumerate() {
                if !bq.holds(p, TOL) || !along.contains(&(own.1 as u32, index[m][k])) {
                    continue;
                }
                let v = q.nearest(p);
                if q.at(v).distance(p) > TOL {
                    continue;
                }
                let t = q.tangent(v);
                return if t.dot(dir) > 0.0 {
                    Side::OnLeft
                } else {
                    Side::OnRight
                };
            }
        }
        let mut odd = false;
        for m in group_loops(g) {
            if members[m].bx.holds(p, 0.0)
                && in_loop(members[m].pieces.iter().map(|q| (&q.0, q.0.at(0.0))), p)
            {
                odd = !odd;
            }
        }
        if odd { Side::In } else { Side::Out }
    };
    // vertex classes, decided once over every split point before any piece is kept: two points
    // within the tolerance are one vertex, transitively (union-find), so the classes do not hang
    // on the order the points come in; each class is its least-named point's (by the piece's
    // place in the input, then along it), and a class spread past twice the
    // tolerance is not one vertex the tolerance can tell: refused
    for list in us.iter_mut() {
        list.sort_by(f64::total_cmp);
    }
    let verts = Verts::classes(&all, &members, &us)?;
    let mut kept: Vec<Kept> = Vec::new();
    let mut near: Vec<u32> = Vec::new();
    let mut seen = vec![usize::MAX; group_count];
    let mut key: Map<(u32, u32), Vec<u32>> = Map::default();
    let mut sub_id = 0;
    for (idx, &(m, k, _)) in all.iter().enumerate() {
        let (source, sid) = (&members[m].pieces[k].0, members[m].pieces[k].1);
        let own = members[m].group;
        let list = &us[idx];
        for w in 0..list.len() - 1 {
            let (u0, u1) = (list[w], list[w + 1]);
            if u1 - u0 <= 0.0 {
                continue;
            }
            let pa = source.at(u0);
            let (va, vb) = (verts.of(idx, w), verts.of(idx, w + 1));
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
            match side_of(0, (own, idx), mp, dir) {
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
                match side_of(g, (own, idx), mp, dir) {
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
            // where boundaries coincide, each keeps the same piece: once is enough (the same ends,
            // on one carrier, decided above)
            let at = key.entry((from, to)).or_default();
            if at
                .iter()
                .any(|&q| q == idx as u32 || along.contains(&(q, idx as u32)))
            {
                continue;
            }
            at.push(idx as u32);
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
    chain(kept, &verts.pts)
}

fn chain(kept: Vec<Kept>, pts: &[Point2]) -> Result<Region, RegionError> {
    // the combinatorics are forge-sheet-core's `chain` (proven, verify/aeneas/forge-sheet-core):
    // degrees, pinches, then the walks
    let from: Vec<u32> = kept.iter().map(|e| e.from).collect();
    let to: Vec<u32> = kept.iter().map(|e| e.to).collect();
    let found = forge_sheet_core::chain(pts.len(), &from, &to).map_err(|e| match e {
        forge_sheet_core::ChainError::Unchained {
            vertex,
            piece,
            leaving,
            arriving,
        } => RegionError::Unchained {
            at: pts[vertex as usize],
            edge: kept[piece as usize].id,
            leaving,
            arriving,
        },
        forge_sheet_core::ChainError::Pinched { pinches } => RegionError::Pinched {
            pinches: pinches
                .into_iter()
                .map(|(v, a, b)| RegionPinch {
                    at: pts[v as usize],
                    a: kept[a as usize].id,
                    b: kept[b as usize].id,
                    width: 0.0,
                    limit: TOL,
                })
                .collect(),
        },
    })?;
    let loops: Vec<Vec<u32>> = found
        .starts
        .windows(2)
        .map(|w| found.pieces[w[0] as usize..w[1] as usize].to_vec())
        .collect();
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
    // each hole to its owner (Region.lean's `owner`): the outer loop holding the hole's vertex
    // that every other outer loop holding it holds too, the innermost. One exact query each, no
    // vote: after `chain` the loops are disjoint, so the vertex is on no other loop
    let reps: Vec<Point2> = parts.iter().map(|c| c.outer[0].start).collect();
    let holds_at = |c: &Component, p: Point2| in_loop(c.outer.iter().map(|b| (&b.piece, b.start)), p);
    let k = parts.len();
    for h in holes {
        let at = h[0].start;
        // the geometric facts decided exactly here (`in_loop`), the rule forge-sheet-core's
        // `owner` (proven); nest[o k + p]: outer p holds outer o's vertex, asked only where both
        // hold the hole's
        let held: Vec<bool> = parts.iter().map(|c| holds_at(c, at)).collect();
        let mut nest = vec![false; k * k];
        for o in (0..k).filter(|&o| held[o]) {
            for p in (0..k).filter(|&p| p != o && held[p]) {
                nest[o * k + p] = holds_at(&parts[p], reps[o]);
            }
        }
        let Some(i) = forge_sheet_core::owner(k, &held, &nest) else {
            return Err(RegionError::HoleOutside { edge: h[0].id, at });
        };
        parts[i as usize].holes.push(h);
    }
    Ok(Region { parts })
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
    in_loop(c.outer.iter().map(|b| (&b.piece, b.start)), p)
        && !c
            .holes
            .iter()
            .any(|h| in_loop(h.iter().map(|b| (&b.piece, b.start)), p))
}

#[cfg(test)]
mod vertex_tests {
    use super::*;

    fn member(group: usize, a: (f64, f64), b: (f64, f64), id: u64) -> Member {
        let p = Piece::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        };
        let bx = p.bbox();
        Member {
            group,
            pieces: vec![(p, id)],
            boxes: vec![bx],
            bx,
        }
    }

    /// The partition of the pieces' ends into classes, by segment id and end.
    fn partition(ms: Vec<Member>) -> Result<Vec<Vec<(u64, usize)>>, RegionError> {
        let all: Vec<(usize, usize, Bx)> = (0..ms.len()).map(|m| (m, 0, ms[m].bx)).collect();
        let us = vec![vec![0.0, 1.0]; ms.len()];
        let v = Verts::classes(&all, &ms, &us)?;
        let mut by: Vec<Vec<(u64, usize)>> = vec![vec![]; v.pts.len()];
        for (m, mm) in ms.iter().enumerate() {
            for k in 0..2 {
                by[v.of(m, k) as usize].push((mm.pieces[0].1, k));
            }
        }
        let mut out: Vec<Vec<(u64, usize)>> = by
            .into_iter()
            .map(|mut c| {
                c.sort();
                c
            })
            .collect();
        out.sort();
        Ok(out)
    }

    #[test]
    fn vertex_classes_are_transitive_and_do_not_hang_on_order() {
        // three ends 0.8 tolerances apart in a row: first-come within 0.9 made two vertices or
        // one by the order they came in; the classes are one, whichever comes first
        let t = TOL;
        let make = |order: [usize; 3]| {
            let ends = [(1.0, 0.0), (1.0 + 0.8 * t, 0.0), (1.0 + 1.6 * t, 0.0)];
            let far = [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
            order
                .iter()
                .map(|&i| member(usize::from(i != 0), far[i], ends[i], 10 + i as u64))
                .collect::<Vec<_>>()
        };
        let a = partition(make([0, 1, 2])).unwrap();
        for order in [[0, 2, 1], [1, 0, 2], [2, 1, 0]] {
            assert_eq!(partition(make(order)).unwrap(), a);
        }
        assert!(a.iter().any(|c| c.len() == 3), "{a:?}");
        // a row spread past twice the tolerance is no one vertex: refused
        let spread = vec![
            member(0, (0.0, 0.0), (1.0, 0.0), 1),
            member(1, (1.0, 1.0), (1.0 + 0.9 * t, 0.0), 2),
            member(1, (2.0, 1.0), (1.0 + 1.8 * t, 0.0), 3),
            member(1, (2.0, 0.0), (1.0 + 2.7 * t, 0.0), 4),
        ];
        assert!(matches!(
            partition(spread),
            Err(RegionError::NearCoincident { .. })
        ));
    }
}

#[cfg(test)]
mod in_loop_tests {
    use super::*;

    #[test]
    fn crossings_count_alike_either_way_even_on_the_line() {
        // `crosses_reverse` for every p: off the edge's line by orient2d, on it by the move
        let e = [
            (Point2::new(0.0, 0.0), Point2::new(4.0, 1e-16)),
            (Point2::new(1.0, 1.0), Point2::new(1.0, 5.0)),
        ];
        for (a, b) in e {
            for p in [
                a.lerp(b, -1.0),
                a.lerp(b, 0.5),
                a.lerp(b, 2.0),
                Point2::new(0.5, 0.3),
                a,
                b,
            ] {
                assert_eq!(crosses(p, a, b), crosses(p, b, a), "{p:?} on {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn a_circles_halves_hold_its_chords_line_rightly() {
        // the halves' chord is the near-level diameter (its far end 1.2e-16 up): points on its
        // line are in the disc between the ends and out beyond them, either way round
        for ccw in [true, false] {
            let l = ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::new(0.0, 0.0),
                    radius: 1.0,
                    ccw,
                },
                0,
            )]);
            let ps = pieces(&l);
            let at = |x: f64, y: f64| in_loop(ps.iter().map(|q| (&q.0, q.0.at(0.0))), Point2::new(x, y));
            let (_, b) = (ps[0].0.at(0.0), ps[0].0.at(1.0));
            for x in [-0.5, 0.0, 0.3, 0.99] {
                assert!(at(x, 0.0) && at(x, 0.5 * b.y), "{ccw} {x}");
            }
            for x in [-3.0, -1.5, 1.5, 3.0] {
                assert!(!at(x, 0.0) && !at(x, 0.5 * b.y), "{ccw} {x}");
            }
        }
    }
}
