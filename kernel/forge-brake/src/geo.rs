//! Exact 2D curves and regions (lines and arcs), 2D and 3D rigid motions, and signed distances:
//! the tools' sections and the walls' outlines, in closed form.
use crate::num::Sc;
use forge_math::{Point2, Vec2, dmath};

/// A line or a circular arc. An arc runs counter-clockwise from `start` when `sweep` > 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line {
        a: Point2,
        b: Point2,
    },
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
}

fn at(c: Point2, r: f64, ang: f64) -> Point2 {
    let (s, co) = dmath::sin_cos(ang);
    Point2::new(c.x + r * co, c.y + r * s)
}

/// `x` folded into [0, 2pi).
fn wrap(x: f64) -> f64 {
    let y = x.rem_euclid(dmath::TAU);
    if y >= dmath::TAU { 0.0 } else { y }
}

impl Seg {
    pub fn line(a: (f64, f64), b: (f64, f64)) -> Seg {
        Seg::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        }
    }

    pub fn start(&self) -> Point2 {
        match *self {
            Seg::Line { a, .. } => a,
            Seg::Arc { c, r, start, .. } => at(c, r, start),
        }
    }

    pub fn end(&self) -> Point2 {
        match *self {
            Seg::Line { b, .. } => b,
            Seg::Arc { c, r, start, sweep } => at(c, r, start + sweep),
        }
    }

    /// Whether angle `ang` (about the centre) falls on the arc.
    fn on_sweep(start: f64, sweep: f64, ang: f64) -> bool {
        if sweep >= 0.0 {
            wrap(ang - start) <= sweep
        } else {
            wrap(start - ang) <= -sweep
        }
    }

    pub fn closest(&self, p: Point2) -> Point2 {
        match *self {
            Seg::Line { a, b } => {
                let d = b - a;
                let l2 = d.dot(d);
                if l2 <= 0.0 {
                    return a;
                }
                let s = ((p - a).dot(d) / l2).clamp(0.0, 1.0);
                a + d * s
            }
            Seg::Arc { c, r, start, sweep } => {
                let v = p - c;
                if v.dot(v) > 0.0 {
                    let ang = dmath::atan2(v.y, v.x);
                    if Seg::on_sweep(start, sweep, ang) {
                        return at(c, r, ang);
                    }
                }
                let (s, e) = (self.start(), self.end());
                if p.distance(s) <= p.distance(e) { s } else { e }
            }
        }
    }

    pub fn dist(&self, p: Point2) -> f64 {
        p.distance(self.closest(p))
    }

    /// The least and greatest of d . q over the curve.
    pub fn extent(&self, d: Vec2) -> (f64, f64) {
        let (s, e) = (self.start(), self.end());
        let (mut lo, mut hi) = (d.dot(s - Point2::ORIGIN), d.dot(e - Point2::ORIGIN));
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        if let Seg::Arc { c, r, start, sweep } = *self {
            let n = d.length();
            if n > 0.0 {
                let ang = dmath::atan2(d.y, d.x);
                let base = d.dot(c - Point2::ORIGIN);
                if Seg::on_sweep(start, sweep, ang) {
                    hi = hi.max(base + r * n);
                }
                if Seg::on_sweep(start, sweep, ang + dmath::PI) {
                    lo = lo.min(base - r * n);
                }
            }
        }
        (lo, hi)
    }

    /// y-monotone pieces (an arc split where its tangent is horizontal), as (start angle, sweep).
    fn monotone(start: f64, sweep: f64) -> Vec<(f64, f64)> {
        let mut cuts: Vec<f64> = Vec::new();
        for k in [dmath::FRAC_PI_2, 3.0 * dmath::FRAC_PI_2] {
            let off = if sweep >= 0.0 {
                wrap(k - start)
            } else {
                wrap(start - k)
            };
            if off > 0.0 && off < sweep.abs() {
                cuts.push(off);
            }
        }
        cuts.sort_by(f64::total_cmp);
        let dir = sweep.signum();
        let mut out = Vec::new();
        let mut prev = 0.0;
        for c in cuts.into_iter().chain(std::iter::once(sweep.abs())) {
            out.push((start + dir * prev, dir * (c - prev)));
            prev = c;
        }
        out
    }

    /// Crossings of the ray from `p` towards +x (half-open in y, so vertices count once).
    pub fn crossings(&self, p: Point2) -> u32 {
        match *self {
            Seg::Line { a, b } => {
                if (a.y > p.y) != (b.y > p.y) {
                    let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                    u32::from(x > p.x)
                } else {
                    0
                }
            }
            Seg::Arc { c, r, start, sweep } => {
                let mut n = 0;
                for (s0, sw) in Seg::monotone(start, sweep) {
                    let (q0, q1) = (at(c, r, s0), at(c, r, s0 + sw));
                    if (q0.y > p.y) != (q1.y > p.y) {
                        // the piece's one point at height p.y
                        let dy = ((p.y - c.y) / r).clamp(-1.0, 1.0);
                        let dx = (1.0 - dy * dy).max(0.0).sqrt() * r;
                        let mid = at(c, r, s0 + 0.5 * sw);
                        let x = if mid.x >= c.x { c.x + dx } else { c.x - dx };
                        n += u32::from(x > p.x);
                    }
                }
                n
            }
        }
    }

    /// Ray parameters s > 0 where p + s d (d a unit vector) meets the curve.
    pub fn ray_hits(&self, p: Point2, d: Vec2, out: &mut Vec<f64>) {
        match *self {
            Seg::Line { a, b } => {
                let e = b - a;
                let den = d.x * e.y - d.y * e.x;
                if den.abs() <= 1e-300 {
                    return;
                }
                let w = a - p;
                let s = (w.x * e.y - w.y * e.x) / den;
                let u = (w.x * d.y - w.y * d.x) / den;
                if s > 0.0 && (0.0..=1.0).contains(&u) {
                    out.push(s);
                }
            }
            Seg::Arc { c, r, start, sweep } => {
                let w = p - c;
                let bq = w.dot(d);
                let cq = w.dot(w) - r * r;
                let disc = bq * bq - cq;
                if disc < 0.0 {
                    return;
                }
                let sq = disc.sqrt();
                for s in [-bq - sq, -bq + sq] {
                    if s > 0.0 {
                        let q = p + d * s - c;
                        if Seg::on_sweep(start, sweep, dmath::atan2(q.y, q.x)) {
                            out.push(s);
                        }
                    }
                }
            }
        }
    }

    /// The point at parameter t in [0, 1] (along a line; through the sweep of an arc).
    pub fn point(&self, t: f64) -> Point2 {
        match *self {
            Seg::Line { a, b } => a + (b - a) * t,
            Seg::Arc { c, r, start, sweep } => at(c, r, start + sweep * t),
        }
    }

    /// The piece between parameters t0 and t1.
    pub fn piece(&self, t0: f64, t1: f64) -> Seg {
        match *self {
            Seg::Line { .. } => Seg::Line {
                a: self.point(t0),
                b: self.point(t1),
            },
            Seg::Arc { c, r, start, sweep } => Seg::Arc {
                c,
                r,
                start: start + sweep * t0,
                sweep: sweep * (t1 - t0),
            },
        }
    }

    /// Parameters on this curve where it meets the line segment `e`.
    pub fn params_vs_line(&self, e: &Seg) -> Vec<f64> {
        let Seg::Line { a: ea, b: eb } = *e else {
            return Vec::new();
        };
        match *self {
            Seg::Line { a, b } => {
                let d = b - a;
                let f = eb - ea;
                let den = d.x * f.y - d.y * f.x;
                if den.abs() <= 1e-300 {
                    return Vec::new();
                }
                let w = ea - a;
                let t = (w.x * f.y - w.y * f.x) / den;
                let u = (w.x * d.y - w.y * d.x) / den;
                if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&t) {
                    vec![t]
                } else {
                    Vec::new()
                }
            }
            Seg::Arc { c, r, start, sweep } => {
                let f = eb - ea;
                let l2 = f.dot(f);
                if l2 <= 0.0 {
                    return Vec::new();
                }
                let w = ea - c;
                let (qa, qb, qc) = (l2, 2.0 * w.dot(f), w.dot(w) - r * r);
                let disc = qb * qb - 4.0 * qa * qc;
                if disc < 0.0 {
                    return Vec::new();
                }
                let sq = disc.sqrt();
                let mut out = Vec::new();
                for u in [(-qb - sq) / (2.0 * qa), (-qb + sq) / (2.0 * qa)] {
                    if !(0.0..=1.0).contains(&u) {
                        continue;
                    }
                    let q = ea + f * u - c;
                    let ang = dmath::atan2(q.y, q.x);
                    if Seg::on_sweep(start, sweep, ang) {
                        let off = if sweep >= 0.0 {
                            wrap(ang - start)
                        } else {
                            wrap(start - ang)
                        };
                        out.push(off / sweep.abs());
                    }
                }
                out
            }
        }
    }

    /// The parameter of a point on the curve.
    pub fn param_of(&self, p: Point2) -> f64 {
        match *self {
            Seg::Line { a, b } => {
                let d = b - a;
                let l2 = d.dot(d);
                if l2 <= 0.0 { 0.0 } else { (p - a).dot(d) / l2 }
            }
            Seg::Arc { c, start, sweep, .. } => {
                let ang = dmath::atan2(p.y - c.y, p.x - c.x);
                let off = if sweep >= 0.0 {
                    wrap(ang - start)
                } else {
                    wrap(start - ang)
                };
                off / sweep.abs()
            }
        }
    }

    /// Parameters on this curve where `o` meets it, and where `o`'s ends lie on it (so curves
    /// running along each other split at each other's ends).
    pub fn cuts_by(&self, o: &Seg) -> Vec<f64> {
        let mut out = Vec::new();
        for p in [o.start(), o.end()] {
            if self.dist(p) <= ON {
                out.push(self.param_of(p));
            }
        }
        match (*self, *o) {
            (Seg::Arc { .. }, Seg::Line { .. }) | (Seg::Line { .. }, Seg::Line { .. }) => {
                out.extend(self.params_vs_line(o))
            }
            (Seg::Line { .. }, Seg::Arc { .. }) => out.extend(self.line_params(o)),
            (
                Seg::Arc { c: c1, r: r1, .. },
                Seg::Arc {
                    c: c2,
                    r: r2,
                    start,
                    sweep,
                },
            ) => {
                // the circles' crossings, on both arcs
                let d = c2 - c1;
                let dd = d.length();
                if dd > 0.0 && dd <= r1 + r2 && dd >= (r1 - r2).abs() {
                    let a = (r1 * r1 - r2 * r2 + dd * dd) / (2.0 * dd);
                    let h = (r1 * r1 - a * a).max(0.0).sqrt();
                    let m = c1 + d * (a / dd);
                    let perp = Vec2::new(-d.y, d.x) * (1.0 / dd);
                    for p in [m + perp * h, m - perp * h] {
                        let ang2 = dmath::atan2(p.y - c2.y, p.x - c2.x);
                        if Seg::on_sweep(start, sweep, ang2) && self.dist(p) <= 1e-7 {
                            out.push(self.param_of(p));
                        }
                    }
                }
            }
        }
        out
    }

    /// Parameters on this line segment where the curve `g` meets it.
    pub fn line_params(&self, g: &Seg) -> Vec<f64> {
        let Seg::Line { a, b } = *self else {
            return Vec::new();
        };
        let d = b - a;
        let l = d.length();
        if l <= 0.0 {
            return Vec::new();
        }
        let mut hits = Vec::new();
        // along the line both ways from a, as rays
        let dir = d * (1.0 / l);
        g.ray_hits(a - dir * 1e-12, dir, &mut hits);
        hits.into_iter()
            .map(|s| (s - 1e-12) / l)
            .filter(|t| (0.0..=1.0).contains(t))
            .collect()
    }

    pub fn moved(&self, m: &Rigid2) -> Seg {
        match *self {
            Seg::Line { a, b } => Seg::Line {
                a: m.apply(a),
                b: m.apply(b),
            },
            Seg::Arc { c, r, start, sweep } => Seg::Arc {
                c: m.apply(c),
                r,
                start: start + m.angle(),
                sweep,
            },
        }
    }

    /// The points worth trying as extremes of a linear function over the curve: its ends and,
    /// on an arc, its quarter points that fall on it.
    pub fn key_points(&self) -> Vec<Point2> {
        let mut v = vec![self.start(), self.end()];
        if let Seg::Arc { c, r, start, sweep } = *self {
            for k in 0..4 {
                let a = k as f64 * dmath::FRAC_PI_2;
                if Seg::on_sweep(start, sweep, a) {
                    v.push(at(c, r, a));
                }
            }
        }
        v
    }
}

/// A rigid motion of the plane: p -> o + x p.x + perp(x) p.y, x a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid2 {
    pub o: Point2,
    pub x: Vec2,
}

impl Rigid2 {
    pub const ID: Rigid2 = Rigid2 {
        o: Point2::ORIGIN,
        x: Vec2 { x: 1.0, y: 0.0 },
    };
    pub fn shift(dx: f64, dy: f64) -> Rigid2 {
        Rigid2 {
            o: Point2::new(dx, dy),
            x: Vec2::new(1.0, 0.0),
        }
    }
    pub fn y(&self) -> Vec2 {
        Vec2::new(-self.x.y, self.x.x)
    }
    pub fn apply(&self, p: Point2) -> Point2 {
        self.o + self.x * p.x + self.y() * p.y
    }
    pub fn vec(&self, v: Vec2) -> Vec2 {
        self.x * v.x + self.y() * v.y
    }
    pub fn then(&self, inner: &Rigid2) -> Rigid2 {
        Rigid2 {
            o: self.apply(inner.o),
            x: self.vec(inner.x),
        }
    }
    pub fn inverse(&self) -> Rigid2 {
        let d = Point2::ORIGIN - self.o;
        Rigid2 {
            o: Point2::new(d.dot(self.x), d.dot(self.y())),
            x: Vec2::new(self.x.x, -self.x.y),
        }
    }
    pub fn angle(&self) -> f64 {
        dmath::atan2(self.x.y, self.x.x)
    }
    /// The same motion in 3D, z kept.
    pub fn lift(&self) -> Iso3 {
        let y = self.y();
        Iso3 {
            o: [self.o.x, self.o.y, 0.0],
            c: [[self.x.x, self.x.y, 0.0], [y.x, y.y, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}

/// An axis-aligned rectangle [0, w] x [0, h] placed by a rigid motion (a bend zone's slot).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub at: Rigid2,
    pub w: f64,
    pub h: f64,
}

impl Slot {
    fn local(&self, p: Point2) -> Point2 {
        self.at.inverse().apply(p)
    }
    /// Strictly inside, by more than `eps`.
    pub fn inside(&self, p: Point2, eps: f64) -> bool {
        let q = self.local(p);
        q.x > eps && q.x < self.w - eps && q.y > eps && q.y < self.h - eps
    }
    /// From a point inside to the slot's boundary.
    pub fn depth(&self, p: Point2) -> f64 {
        let q = self.local(p);
        q.x.min(self.w - q.x).min(q.y).min(self.h - q.y)
    }
    pub fn segs(&self) -> [Seg; 4] {
        let c = [(0.0, 0.0), (self.w, 0.0), (self.w, self.h), (0.0, self.h)]
            .map(|(x, y)| self.at.apply(Point2::new(x, y)));
        [0, 1, 2, 3].map(|i| Seg::Line {
            a: c[i],
            b: c[(i + 1) % 4],
        })
    }
    pub fn moved(&self, m: &Rigid2) -> Slot {
        Slot {
            at: m.then(&self.at),
            w: self.w,
            h: self.h,
        }
    }
}

/// A planar region: loops of lines and arcs (inside by the even-odd rule: an outline and its
/// holes), within a second set of loops (`clip`, the flat pattern; empty: no clip), less some
/// slots (closed rectangles). Its boundary is kept exactly: the loops' pieces within the clip and
/// outside every slot, the clip's pieces within the loops, and the slots' edges within both; so
/// distances to it are exact.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Region {
    pub loops: Vec<Vec<Seg>>,
    pub clip: Vec<Vec<Seg>>,
    pub minus: Vec<Slot>,
    pub bound: Vec<Seg>,
}

/// Points this close to a boundary count as on it.
const ON: f64 = 1e-9;

fn even_odd<'a>(segs: impl Iterator<Item = &'a Seg>, p: Point2) -> bool {
    segs.map(|s| s.crossings(p)).sum::<u32>() % 2 == 1
}

fn near<'a>(mut segs: impl Iterator<Item = &'a Seg>, p: Point2) -> bool {
    segs.any(|s| s.dist(p) <= ON)
}

/// `g` split wherever any of `by` meets it.
fn split<'a>(g: &Seg, by: impl Iterator<Item = &'a Seg>) -> Vec<(f64, f64)> {
    let mut ts = vec![0.0, 1.0];
    for o in by {
        ts.extend(g.cuts_by(o));
    }
    spans(ts)
}

impl Region {
    /// The region of `loops` alone.
    pub fn of(loops: Vec<Vec<Seg>>) -> Region {
        let bound = loops.iter().flatten().copied().collect();
        Region {
            loops,
            clip: Vec::new(),
            minus: Vec::new(),
            bound,
        }
    }

    /// `loops` within `clip` (if any), less the closed slots.
    pub fn with(loops: Vec<Vec<Seg>>, clip: Vec<Vec<Seg>>, minus: Vec<Slot>) -> Region {
        let mut r = Region {
            loops,
            clip,
            minus,
            bound: Vec::new(),
        };
        let edges: Vec<Seg> = r.minus.iter().flat_map(|s| s.segs()).collect();
        let l: Vec<Seg> = r.loops.iter().flatten().copied().collect();
        let c: Vec<Seg> = r.clip.iter().flatten().copied().collect();
        let has_clip = !c.is_empty();
        let in_c = |p: Point2| !has_clip || even_odd(c.iter(), p) || near(c.iter(), p);
        let in_c_strict = |p: Point2| !has_clip || (even_odd(c.iter(), p) && !near(c.iter(), p));
        let in_l_strict = |p: Point2| even_odd(l.iter(), p) && !near(l.iter(), p);
        let in_slot = |p: Point2, skip: Option<usize>| {
            r.minus
                .iter()
                .enumerate()
                .any(|(k, s)| Some(k) != skip && s.inside(p, -ON))
        };
        let mut bound = Vec::new();
        for g in &l {
            for (t0, t1) in split(g, c.iter().chain(edges.iter())) {
                let mid = g.point(0.5 * (t0 + t1));
                if in_c(mid) && !in_slot(mid, None) {
                    bound.push(g.piece(t0, t1));
                }
            }
        }
        for g in &c {
            for (t0, t1) in split(g, l.iter().chain(edges.iter())) {
                let mid = g.point(0.5 * (t0 + t1));
                if in_l_strict(mid) && !in_slot(mid, None) {
                    bound.push(g.piece(t0, t1));
                }
            }
        }
        for (i, e) in edges.iter().enumerate() {
            for (t0, t1) in split(e, l.iter().chain(c.iter()).chain(edges.iter())) {
                let mid = e.point(0.5 * (t0 + t1));
                if in_l_strict(mid) && in_c_strict(mid) && !in_slot(mid, Some(i / 4)) {
                    bound.push(e.piece(t0, t1));
                }
            }
        }
        r.bound = bound;
        r
    }

    pub fn segs(&self) -> impl Iterator<Item = &Seg> {
        self.loops.iter().flatten()
    }

    /// Inside the loops (closed), ignoring the slots.
    pub fn in_loops(&self, p: Point2) -> bool {
        self.segs().map(|s| s.crossings(p)).sum::<u32>() % 2 == 1
    }

    pub fn contains(&self, p: Point2) -> bool {
        self.in_loops(p)
            && (self.clip.is_empty() || even_odd(self.clip.iter().flatten(), p))
            && !self.minus.iter().any(|s| s.inside(p, -ON))
    }

    pub fn boundary_dist(&self, p: Point2) -> f64 {
        self.bound.iter().map(|s| s.dist(p)).fold(f64::INFINITY, f64::min)
    }

    /// The distance from `p` to the region (0 inside it).
    pub fn dist_lower(&self, p: Point2) -> f64 {
        if self.contains(p) {
            0.0
        } else {
            self.boundary_dist(p)
        }
    }

    /// Signed distance, negative inside.
    pub fn sd(&self, p: Point2) -> f64 {
        let d = self.boundary_dist(p);
        if self.contains(p) { -d } else { d }
    }

    /// A point of the region nearest `p`: p itself, or the nearest point of its boundary.
    pub fn rep(&self, p: Point2) -> Option<Point2> {
        if self.contains(p) {
            return Some(p);
        }
        self.bound
            .iter()
            .map(|s| s.closest(p))
            .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
    }

    pub fn bbox(&self) -> (Point2, Point2) {
        let (mut lo, mut hi) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for s in &self.bound {
            let (x0, x1) = s.extent(Vec2::new(1.0, 0.0));
            let (y0, y1) = s.extent(Vec2::new(0.0, 1.0));
            lo = Point2::new(lo.x.min(x0), lo.y.min(y0));
            hi = Point2::new(hi.x.max(x1), hi.y.max(y1));
        }
        (lo, hi)
    }

    pub fn moved(&self, m: &Rigid2) -> Region {
        Region {
            loops: self
                .loops
                .iter()
                .map(|l| l.iter().map(|s| s.moved(m)).collect())
                .collect(),
            clip: self
                .clip
                .iter()
                .map(|l| l.iter().map(|s| s.moved(m)).collect())
                .collect(),
            minus: self.minus.iter().map(|s| s.moved(m)).collect(),
            bound: self.bound.iter().map(|s| s.moved(m)).collect(),
        }
    }
}

/// Sorted, deduplicated parameters in [0, 1] as consecutive spans.
fn spans(mut ts: Vec<f64>) -> Vec<(f64, f64)> {
    ts.retain(|t| (0.0..=1.0).contains(t));
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    ts.windows(2)
        .filter(|w| w[1] - w[0] > 1e-12)
        .map(|w| (w[0], w[1]))
        .collect()
}

/// A 3D vector over a scalar (a point, or an enclosure of points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct V3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: Sc> V3<T> {
    pub fn new(x: T, y: T, z: T) -> V3<T> {
        V3 { x, y, z }
    }
    pub fn plus(self, o: V3<T>) -> V3<T> {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn minus(self, o: V3<T>) -> V3<T> {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn scale(self, s: T) -> V3<T> {
        V3::new(self.x * s, self.y * s, self.z * s)
    }
    pub fn dot(self, o: V3<T>) -> T {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
}

impl V3<f64> {
    pub fn arr(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
}

/// A rigid motion of space with double coefficients: p -> o + c[0] p.x + c[1] p.y + c[2] p.z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iso3 {
    pub o: [f64; 3],
    /// The images of the axes (columns).
    pub c: [[f64; 3]; 3],
}

impl Iso3 {
    pub const ID: Iso3 = Iso3 {
        o: [0.0; 3],
        c: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub fn shift(x: f64) -> Iso3 {
        Iso3 {
            o: [x, 0.0, 0.0],
            ..Iso3::ID
        }
    }

    pub fn apply<T: Sc>(&self, p: V3<T>) -> V3<T> {
        let row = |i: usize| p.x * self.c[0][i] + p.y * self.c[1][i] + p.z * self.c[2][i] + self.o[i];
        V3::new(row(0), row(1), row(2))
    }

    pub fn vec(&self, v: [f64; 3]) -> [f64; 3] {
        [0, 1, 2].map(|i| v[0] * self.c[0][i] + v[1] * self.c[1][i] + v[2] * self.c[2][i])
    }

    pub fn then(&self, inner: &Iso3) -> Iso3 {
        let o = self.apply(V3::new(inner.o[0], inner.o[1], inner.o[2])).arr();
        Iso3 {
            o,
            c: [self.vec(inner.c[0]), self.vec(inner.c[1]), self.vec(inner.c[2])],
        }
    }

    pub fn inverse(&self) -> Iso3 {
        // the transpose
        let r = |i: usize| [self.c[i][0], self.c[i][1], self.c[i][2]];
        let rows = [r(0), r(1), r(2)];
        let c = [0, 1, 2].map(|j| [rows[0][j], rows[1][j], rows[2][j]]);
        let mo =
            [0, 1, 2].map(|i| -(rows[i][0] * self.o[0] + rows[i][1] * self.o[1] + rows[i][2] * self.o[2]));
        Iso3 { o: mo, c }
    }
}

/// The signed distance from p to a region of the (x, z) plane extruded along y over [y0, y1]:
/// exact for an extrusion.
pub fn sd_prism(sec: &Region, y0: f64, y1: f64, p: [f64; 3]) -> f64 {
    let s2 = sec.sd(Point2::new(p[0], p[2]));
    let dy = (y0 - p[1]).max(p[1] - y1);
    if s2 <= 0.0 && dy <= 0.0 {
        s2.max(dy)
    } else {
        (s2.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Region {
        let pts = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        Region::of(vec![
            (0..4).map(|i| Seg::line(pts[i], pts[(i + 1) % 4])).collect(),
        ])
    }

    #[test]
    fn square_distances() {
        let r = square();
        assert!(r.contains(Point2::new(1.0, 1.0)));
        assert!(!r.contains(Point2::new(3.0, 1.0)));
        assert_eq!(r.sd(Point2::new(1.0, 1.5)), -0.5);
        assert_eq!(r.sd(Point2::new(3.0, 1.0)), 1.0);
        assert!((r.sd(Point2::new(3.0, 3.0)) - 2f64.sqrt()).abs() < 1e-15);
    }

    #[test]
    fn disc_from_two_arcs() {
        let c = Point2::new(0.0, 0.0);
        let r = Region::of(vec![vec![
            Seg::Arc {
                c,
                r: 1.0,
                start: 0.3,
                sweep: dmath::PI,
            },
            Seg::Arc {
                c,
                r: 1.0,
                start: 0.3 + dmath::PI,
                sweep: dmath::PI,
            },
        ]]);
        for k in 0..50 {
            let a = k as f64 * 0.37;
            for rad in [0.2, 0.9, 1.1, 2.0] {
                let (sa, ca) = dmath::sin_cos(a);
                let p = Point2::new(rad * ca, rad * sa);
                assert_eq!(r.contains(p), rad < 1.0, "{p:?}");
                assert!((r.sd(p) - (rad - 1.0)).abs() < 1e-12);
            }
        }
        let (lo, hi) = r.bbox();
        assert!((lo.x + 1.0).abs() < 1e-12 && (hi.y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn slot_taken_out() {
        let r = square();
        let r = Region::with(
            r.loops,
            vec![],
            vec![Slot {
                at: Rigid2::shift(1.5, -1.0),
                w: 1.0,
                h: 4.0,
            }],
        );
        assert!(!r.contains(Point2::new(1.8, 1.0)));
        assert!(r.contains(Point2::new(1.2, 1.0)));
        assert!((r.dist_lower(Point2::new(1.8, 1.0)) - 0.3).abs() < 1e-12);
        let q = r.rep(Point2::new(1.8, 1.0)).unwrap();
        assert!((q.x - 1.5).abs() < 1e-12);
    }

    #[test]
    fn a_slot_flush_with_the_outline_leaves_no_sliver() {
        // the slot spans the square's whole height: nothing of x in [1.5, 2] is left
        let r = Region::with(
            square().loops,
            vec![],
            vec![Slot {
                at: Rigid2::shift(1.5, 0.0),
                w: 0.5,
                h: 2.0,
            }],
        );
        assert!(!r.contains(Point2::new(1.8, 0.0)));
        // just above the bottom edge in the slot: the material is at x = 1.5
        assert!((r.sd(Point2::new(1.8, 1e-7)) - 0.3).abs() < 1e-9);
        assert!((r.sd(Point2::new(1.0, 1.0)) + 0.5).abs() < 1e-12);
        let (_, hi) = r.bbox();
        assert!((hi.x - 1.5).abs() < 1e-12);
    }

    #[test]
    fn isos_invert() {
        let r = Rigid2 {
            o: Point2::new(3.0, -1.0),
            x: Vec2::new(0.6, 0.8),
        };
        let m = r.lift().then(&Iso3::shift(2.0));
        let p = V3::new(1.0, 2.0, 3.0);
        let q = m.inverse().apply(m.apply(p));
        assert!((q.x - 1.0).abs() + (q.y - 2.0).abs() + (q.z - 3.0).abs() < 1e-14);
        let p2 = Point2::new(0.5, 0.25);
        let back = r.inverse().apply(r.apply(p2));
        assert!(back.distance(p2) < 1e-15);
    }

    #[test]
    fn prism_distance() {
        let r = square();
        assert_eq!(sd_prism(&r, 0.0, 10.0, [1.0, 5.0, 1.0]), -1.0);
        assert_eq!(sd_prism(&r, 0.0, 10.0, [1.0, 12.0, 1.0]), 2.0);
        assert!((sd_prism(&r, 0.0, 10.0, [3.0, 11.0, 1.0]) - 2f64.sqrt()).abs() < 1e-15);
    }
}
