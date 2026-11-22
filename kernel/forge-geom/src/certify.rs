//! Certified surface–surface intersection (feature `certified_trace`): a check run on the
//! marched chains of `intcurve::trace`, after the march. research/kernel-audit/certified-tracing.md
//! has the design and the proofs' statements.
//!
//! The surfaces are the kernel's own implicit formulas made polynomial, evaluated in
//! `forge_math::Interval` (IEEE round to nearest, each bound pushed out one ulp: `+ - *` only).
//! A chain is certified when:
//! - each segment's box holds, on every plane of a pencil between the planes across the curve at
//!   its ends, exactly one zero of the pair (parametric Krawczyk);
//! - consecutive segments share their end zero (Krawczyk on the boxes' meet) and run on the same
//!   way;
//! - every zero in the region lies in some segment's claim (interval exclusion of the rest), and no
//!   zero is claimed by two segments that are not neighbours in one chain.
//!
//! Otherwise it refuses, naming the box where the proof failed. It never changes a chain.

use crate::intcurve::IntCurve;
use crate::surface::Surface;
use forge_math::{Box3, Interval as I, Point3, Vec3, tol};

/// Why the certificate refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// A step's Krawczyk test failed down to the least size: a tangency, a singular point, or a
    /// chain off the curve.
    Step,
    /// Two consecutive steps could not be shown to share their end point.
    Junction,
    /// A zero of the pair lies on no chain: a branch, loop or stretch the march missed.
    Uncovered,
    /// A zero claimed by two chains, or twice by one.
    Twice,
    /// A surface the certificate has no enclosure for (procedural, spindle torus).
    OutOfScope,
    /// The exclusion walk ran past its box budget.
    Budget,
}

/// Where and why the certificate refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Refusal {
    pub region: Box3,
    pub why: Why,
}

type V = [I; 3];

fn pt(p: Point3) -> V {
    [I::point(p.x), I::point(p.y), I::point(p.z)]
}

fn of_box(b: &Box3) -> V {
    [b.x, b.y, b.z]
}

fn to_box(v: &V) -> Box3 {
    Box3 {
        x: v[0],
        y: v[1],
        z: v[2],
    }
}

fn sub_p(v: &V, o: Point3) -> V {
    [v[0] - I::point(o.x), v[1] - I::point(o.y), v[2] - I::point(o.z)]
}

/// d . q, d a float vector.
fn dot(d: Vec3, q: &V) -> I {
    q[0] * d.x + q[1] * d.y + q[2] * d.z
}

/// a d (interval times a float vector).
fn scale(a: I, d: Vec3) -> V {
    [a * d.x, a * d.y, a * d.z]
}

fn add(u: V, v: V) -> V {
    [u[0] + v[0], u[1] + v[1], u[2] + v[2]]
}

fn meet(a: I, b: I) -> I {
    I {
        lo: a.lo.max(b.lo),
        hi: a.hi.min(b.hi),
    }
}

fn mag(a: I) -> f64 {
    a.lo.abs().max(a.hi.abs())
}

fn inside(a: &V, b: &V) -> bool {
    (0..3).all(|k| a[k].lo >= b[k].lo && a[k].hi <= b[k].hi)
}

fn meets(a: &V, b: &V) -> Option<V> {
    let m = [meet(a[0], b[0]), meet(a[1], b[1]), meet(a[2], b[2])];
    m.iter().all(|i| i.lo <= i.hi).then_some(m)
}

fn mid(v: &V) -> [f64; 3] {
    [v[0].mid(), v[1].mid(), v[2].mid()]
}

/// A surface as a polynomial in the stored parameters, zero where the kernel's implicit is.
#[derive(Clone, Copy, Debug)]
enum Form {
    /// z.q
    Plane { o: Point3, z: Vec3 },
    /// |q|^2 - r^2
    Sphere { o: Point3, r: f64 },
    /// lx^2 + ly^2 - r^2
    Cylinder { o: Point3, x: Vec3, y: Vec3, r: f64 },
    /// cos^2 (lx^2 + ly^2) - (r cos + sin lz)^2, on the nappe where r cos + sin lz >= 0
    Cone {
        o: Point3,
        x: Vec3,
        y: Vec3,
        z: Vec3,
        r: f64,
        sin: f64,
        cos: f64,
    },
    /// (rho^2 + lz^2 + R^2 - r^2)^2 - 4 R^2 rho^2, R >= r (every root rho is >= 0)
    Torus {
        o: Point3,
        x: Vec3,
        y: Vec3,
        z: Vec3,
        big: f64,
        r: f64,
    },
}

impl Form {
    fn of(s: &Surface) -> Option<Form> {
        Some(match s {
            Surface::Plane(p) => Form::Plane {
                o: p.frame.origin,
                z: p.frame.z().get(),
            },
            Surface::Sphere(s) => Form::Sphere {
                o: s.frame.origin,
                r: s.radius,
            },
            Surface::Cylinder(c) => Form::Cylinder {
                o: c.frame.origin,
                x: c.frame.x().get(),
                y: c.frame.y().get(),
                r: c.radius,
            },
            Surface::Cone(c) if c.cos_a() > 0.0 => Form::Cone {
                o: c.frame.origin,
                x: c.frame.x().get(),
                y: c.frame.y().get(),
                z: c.frame.z().get(),
                r: c.radius,
                sin: c.sin_a(),
                cos: c.cos_a(),
            },
            Surface::Torus(t) if t.major >= t.minor && t.minor > 0.0 => Form::Torus {
                o: t.frame.origin,
                x: t.frame.x().get(),
                y: t.frame.y().get(),
                z: t.frame.z().get(),
                big: t.major,
                r: t.minor,
            },
            _ => return None,
        })
    }

    /// Value and gradient over the box (the natural extension).
    fn eval(&self, b: &V) -> (I, V) {
        let two = I::point(2.0);
        match *self {
            Form::Plane { o, z } => {
                let q = sub_p(b, o);
                (dot(z, &q), pt(Point3::new(z.x, z.y, z.z)))
            }
            Form::Sphere { o, r } => {
                let q = sub_p(b, o);
                let v = q[0].sqr() + q[1].sqr() + q[2].sqr() - I::point(r * r).widened();
                (v, [q[0] * two, q[1] * two, q[2] * two])
            }
            Form::Cylinder { o, x, y, r } => {
                let q = sub_p(b, o);
                let (lx, ly) = (dot(x, &q), dot(y, &q));
                let v = lx.sqr() + ly.sqr() - I::point(r * r).widened();
                (v, add(scale(lx * two, x), scale(ly * two, y)))
            }
            Form::Cone {
                o,
                x,
                y,
                z,
                r,
                sin,
                cos,
            } => {
                let q = sub_p(b, o);
                let (lx, ly, lz) = (dot(x, &q), dot(y, &q), dot(z, &q));
                let c2 = I::point(cos * cos).widened();
                let w = I::point(r * cos).widened() + lz * sin;
                let v = c2 * (lx.sqr() + ly.sqr()) - w.sqr();
                let g = add(
                    add(scale(c2 * lx * two, x), scale(c2 * ly * two, y)),
                    scale(w * (two * sin) * I::point(-1.0), z),
                );
                (v, g)
            }
            Form::Torus { o, x, y, z, big, r } => {
                let q = sub_p(b, o);
                let (lx, ly, lz) = (dot(x, &q), dot(y, &q), dot(z, &q));
                let rho2 = lx.sqr() + ly.sqr();
                let k = I::point(big * big).widened() - I::point(r * r).widened();
                let w = rho2 + lz.sqr() + k;
                let bb = I::point(big * big).widened() * 4.0;
                let v = w.sqr() - bb * rho2;
                // 4 w (lx x + ly y + lz z) - 8 R^2 (lx x + ly y)
                let w4 = w * 4.0;
                let e = w4 - bb * two;
                let g = add(add(scale(e * lx, x), scale(e * ly, y)), scale(w4 * lz, z));
                (v, g)
            }
        }
    }

    /// The value over the box: the natural extension met with the mean-value form.
    fn value(&self, b: &V) -> I {
        let (nat, g) = self.eval(b);
        let c = mid(b);
        let (fc, _) = self.eval(&[I::point(c[0]), I::point(c[1]), I::point(c[2])]);
        let mv = (0..3).fold(fc, |acc, k| acc + g[k] * (b[k] - I::point(c[k])));
        meet(nat, mv)
    }

    /// The kernel's implicit (`Surface::implicit`) at a point, in intervals: a distance, 1-Lipschitz
    /// (to a factor within 1e-12 of 1 for frames off orthonormal by rounding).
    fn distance(&self, c: &V) -> I {
        match *self {
            Form::Plane { o, z } => dot(z, &sub_p(c, o)),
            Form::Sphere { o, r } => {
                let q = sub_p(c, o);
                (q[0].sqr() + q[1].sqr() + q[2].sqr()).sqrt() - I::point(r)
            }
            Form::Cylinder { o, x, y, r } => {
                let q = sub_p(c, o);
                (dot(x, &q).sqr() + dot(y, &q).sqr()).sqrt() - I::point(r)
            }
            Form::Cone {
                o,
                x,
                y,
                z,
                r,
                sin,
                cos,
            } => {
                let q = sub_p(c, o);
                let rho = (dot(x, &q).sqr() + dot(y, &q).sqr()).sqrt();
                (rho - I::point(r)) * cos - dot(z, &q) * sin
            }
            Form::Torus { o, x, y, z, big, r } => {
                let q = sub_p(c, o);
                let d = (dot(x, &q).sqr() + dot(y, &q).sqr()).sqrt() - I::point(big);
                (d.sqr() + dot(z, &q).sqr()).sqrt() - I::point(r)
            }
        }
    }

    /// Whether the box holds no point of the surface: the polynomial has no zero there, or the
    /// box lies wholly past the cone's apex, on the nappe the surface does not have.
    fn off(&self, b: &V) -> bool {
        // the box's half diagonal, rounded up, with the frame's slack
        let c = mid(b);
        let half = (0..3).fold(I::point(0.0), |acc, k| {
            acc + (I::point(b[k].hi) - I::point(c[k]))
                .sqr()
                .hull(&(I::point(c[k]) - I::point(b[k].lo)).sqr())
        });
        let reach = half.sqrt().hi * (1.0 + 1e-12);
        let d = self.distance(&[I::point(c[0]), I::point(c[1]), I::point(c[2])]);
        if d.lo > reach || d.hi < -reach {
            return true;
        }
        if let Form::Cone {
            o, z, r, sin, cos, ..
        } = *self
            && (I::point(r * cos).widened() + dot(z, &sub_p(b, o)) * sin).hi < 0.0
        {
            return true;
        }
        !self.value(b).contains_zero()
    }

    /// Whether every zero of the polynomial in the box is a point of the surface (the cone's other
    /// nappe is not).
    fn sheet(&self, b: &V) -> bool {
        match *self {
            Form::Cone {
                o, z, r, sin, cos, ..
            } => {
                let lz = dot(z, &sub_p(b, o));
                (I::point(r * cos).widened() + lz * sin).lo > 0.0
            }
            _ => true,
        }
    }
}

trait Component {
    fn dot_k(self, k: usize) -> f64;
}

impl Component for Vec3 {
    /// The k-th coordinate.
    fn dot_k(self, k: usize) -> f64 {
        [self.x, self.y, self.z][k]
    }
}

trait Widen {
    fn widened(self) -> I;
}

trait WidenPoint {
    fn widened_point(self) -> I;
}

impl WidenPoint for f64 {
    /// A rounded product, widened by an ulp each way: it holds the exact one.
    fn widened_point(self) -> I {
        I::point(self).widened()
    }
}

impl Widen for I {
    /// A rounded product of two stored floats, widened by an ulp each way: it holds the exact one.
    fn widened(self) -> I {
        I {
            lo: self.lo.next_down(),
            hi: self.hi.next_up(),
        }
    }
}

/// The inverse of a float 3x3 matrix, if it has one.
fn inverse(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let c = |i: usize, j: usize| {
        let (i1, i2) = ((i + 1) % 3, (i + 2) % 3);
        let (j1, j2) = ((j + 1) % 3, (j + 2) % 3);
        m[i1][j1] * m[i2][j2] - m[i1][j2] * m[i2][j1]
    };
    let det = m[0][0] * c(0, 0) + m[0][1] * c(0, 1) + m[0][2] * c(0, 2);
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let mut inv = [[0.0; 3]; 3];
    for (i, row) in inv.iter_mut().enumerate() {
        for (j, e) in row.iter_mut().enumerate() {
            *e = c(j, i) / det;
        }
    }
    inv.iter().flatten().all(|e| e.is_finite()).then_some(inv)
}

/// The Krawczyk test: with y in X, fy enclosing F(y) (over every parameter) and j enclosing F's
/// Jacobian over X (and the parameters), whether K = y - C fy + (I - C j)(X - y) lies inside X
/// and ||I - C j|| < 1 (C the inverse of j's midpoint). Then F has exactly one zero in X for each
/// parameter (verify/krawczyk: `krawczyk_unique`).
fn krawczyk(x: &V, y: [f64; 3], fy: V, j: &[[I; 3]; 3]) -> bool {
    let jm = [0, 1, 2].map(|r| [0, 1, 2].map(|c| j[r][c].mid()));
    let Some(c) = inverse(jm) else { return false };
    for r in 0..3 {
        let mut cf = I::point(0.0);
        let mut norm = I::point(0.0);
        let mut k = I::point(y[r]);
        for (i, f) in fy.iter().enumerate() {
            cf = cf + *f * c[r][i];
        }
        k = k - cf;
        for col in 0..3 {
            let mut m = I::point(if r == col { 1.0 } else { 0.0 });
            for i in 0..3 {
                m = m - j[i][col] * c[r][i];
            }
            norm = norm + I::point(mag(m));
            k = k + m * (x[col] - I::point(y[col]));
        }
        if !(norm.hi < 1.0 && k.lo > x[r].lo && k.hi < x[r].hi) {
            return false;
        }
    }
    true
}

/// One certified step: on every plane g(x, s) = (1 - s) t0.(x - p0) + s t1.(x - p1) = 0 with s in
/// S, exactly one zero of the pair lies in `x`.
#[derive(Clone, Copy, Debug)]
struct Seg {
    p0: Point3,
    t0: Vec3,
    p1: Point3,
    t1: Vec3,
    x: V,
    /// The claim's range of s, within the proven [-SIGMA, 1 + SIGMA]: short of the far half of
    /// each neighbouring step, so that only neighbours' claims meet.
    lo: f64,
    hi: f64,
}

/// The pencil's range: a little past each end, so consecutive claims overlap.
const SIGMA: f64 = 0.125;

impl Seg {
    /// g over the box at one s, as n(s).x - k(s): the box's coordinates appear once.
    fn g(&self, b: &V, s: f64) -> I {
        let s = I::point(s);
        let u = I::point(1.0) - s;
        (0..3).fold(I::point(0.0), |acc, k| {
            let (t0, t1) = (self.t0.dot_k(k), self.t1.dot_k(k));
            let n = u * t0 + s * t1;
            acc + n * b[k]
                - (u * (t0 * self.p0.to_vec().dot_k(k)).widened_point()
                    + s * (t1 * self.p1.to_vec().dot_k(k)).widened_point())
        })
    }

    /// Whether every point of the box lies on a plane of the pencil (so every zero in it is the
    /// segment's).
    fn claims(&self, b: &V) -> bool {
        if !inside(b, &self.x) {
            return false;
        }
        let (lo, hi) = (self.g(b, self.lo), self.g(b, self.hi));
        (lo.hi <= 0.0 && hi.lo >= 0.0) || (lo.lo >= 0.0 && hi.hi <= 0.0)
    }

    /// Whether no point of the box lies on a plane of the pencil.
    fn disowns(&self, b: &V) -> bool {
        let (lo, hi) = (self.g(b, self.lo), self.g(b, self.hi));
        (lo.lo > 0.0 && hi.lo > 0.0) || (lo.hi < 0.0 && hi.hi < 0.0)
    }
}

struct Pair {
    a: Form,
    b: Form,
}

impl Pair {
    fn excludes(&self, b: &V) -> bool {
        self.a.off(b) || self.b.off(b)
    }

    fn step(&self, s: &Seg) -> bool {
        if !self.a.sheet(&s.x) || !self.b.sheet(&s.x) {
            return false;
        }
        let y = mid(&s.x);
        let yv = [I::point(y[0]), I::point(y[1]), I::point(y[2])];
        let sv = I::new(-SIGMA, 1.0 + SIGMA);
        let (fa, _) = self.a.eval(&yv);
        let (fb, _) = self.b.eval(&yv);
        // g = a + S (b - a) and its gradient t0 + S (t1 - t0): S once in each
        let ga = dot(s.t0, &sub_p(&yv, s.p0));
        let gb = dot(s.t1, &sub_p(&yv, s.p1));
        let g = ga + sv * (gb - ga);
        let (_, da) = self.a.eval(&s.x);
        let (_, db) = self.b.eval(&s.x);
        let dg = [0, 1, 2]
            .map(|k| I::point(s.t0.dot_k(k)) + sv * (I::point(s.t1.dot_k(k)) - I::point(s.t0.dot_k(k))));
        krawczyk(&s.x, y, [fa, fb, g], &[da, db, dg])
    }

    /// The zero on the plane t.(x - p) = 0 is the only one in `y`.
    fn junction(&self, y: &V, p: Point3, t: Vec3) -> bool {
        if !self.a.sheet(y) || !self.b.sheet(y) {
            return false;
        }
        let c = mid(y);
        let cv = [I::point(c[0]), I::point(c[1]), I::point(c[2])];
        let (fa, _) = self.a.eval(&cv);
        let (fb, _) = self.b.eval(&cv);
        let g = dot(t, &sub_p(&cv, p));
        let (_, da) = self.a.eval(y);
        let (_, db) = self.b.eval(y);
        krawczyk(y, c, [fa, fb, g], &[da, db, pt(Point3::new(t.x, t.y, t.z))])
    }
}

/// The certificate's least box side.
const LEAST: f64 = tol::LINEAR;
/// Boxes the exclusion walks may visit, per certificate.
const BUDGET: usize = 400_000;

fn grown(p: Point3, q: Point3, r: f64) -> V {
    let b = Box3::from_points([p, q]).grow(r);
    // grow by an ulp more: the hull's bounds are the points' own
    let w = |i: I| I {
        lo: i.lo.next_down(),
        hi: i.hi.next_up(),
    };
    [w(b.x), w(b.y), w(b.z)]
}

struct Cert<'a> {
    pair: Pair,
    sa: &'a Surface,
    sb: &'a Surface,
}

impl Cert<'_> {
    /// Certified steps from (p0, t0) to (p1, t1), split at the curve's midpoint where they fail.
    fn steps(
        &self,
        p0: Point3,
        t0: Vec3,
        p1: Point3,
        t1: Vec3,
        out: &mut Vec<Seg>,
        depth: u32,
    ) -> Result<(), Refusal> {
        let h = p0.distance(p1);
        for f in [0.25, 0.375, 0.5] {
            let s = Seg {
                p0,
                t0,
                p1,
                t1,
                x: grown(p0, p1, f * h + 4.0 * f64::EPSILON * p0.to_vec().length().max(1.0)),
                lo: -SIGMA,
                hi: 1.0 + SIGMA,
            };
            if self.pair.step(&s) {
                out.push(s);
                return Ok(());
            }
        }
        let refuse = Refusal {
            region: to_box(&grown(p0, p1, 0.5 * h)),
            why: Why::Step,
        };
        if h <= LEAST || depth >= 24 {
            return Err(refuse);
        }
        let m = crate::intcurve::refine(self.sa, self.sb, p0.lerp(p1, 0.5)).ok_or(refuse)?;
        let tm = crate::intcurve::direction(self.sa, self.sb, m).ok_or(refuse)?;
        let tm = if tm.dot(p1 - p0) < 0.0 { -tm } else { tm };
        if m.distance(p0) >= h || m.distance(p1) >= h {
            return Err(refuse);
        }
        self.steps(p0, t0, m, tm, out, depth + 1)?;
        self.steps(m, tm, p1, t1, out, depth + 1)
    }

    /// A chain's steps, each junction certified.
    fn chain(&self, c: &IntCurve) -> Result<Vec<Seg>, Refusal> {
        let n = c.pts.len();
        let segs = if c.closed { n } else { n - 1 };
        let mut out = Vec::new();
        for i in 0..segs {
            let j = (i + 1) % n;
            let (p0, p1) = (c.pts[i], c.pts[j]);
            let orient = |t: Vec3| if t.dot(p1 - p0) < 0.0 { -t } else { t };
            self.steps(p0, orient(c.tans[i]), p1, orient(c.tans[j]), &mut out, 0)?;
        }
        // a short loop could be run round twice by steps all neighbours of each other
        if c.closed && out.len() < 10 {
            let mut finer = Vec::new();
            for s in &out {
                let h = s.p0.distance(s.p1);
                let refuse = Refusal {
                    region: to_box(&s.x),
                    why: Why::Step,
                };
                let m = crate::intcurve::refine(self.sa, self.sb, s.p0.lerp(s.p1, 0.5)).ok_or(refuse)?;
                let tm = crate::intcurve::direction(self.sa, self.sb, m).ok_or(refuse)?;
                let tm = if tm.dot(s.p1 - s.p0) < 0.0 { -tm } else { tm };
                if m.distance(s.p0) >= h || m.distance(s.p1) >= h {
                    return Err(refuse);
                }
                self.steps(s.p0, s.t0, m, tm, &mut finer, 1)?;
                self.steps(m, tm, s.p1, s.t1, &mut finer, 1)?;
            }
            out = finer;
        }
        let m = out.len();
        // (a sub-range of the proven one: every claim in it stands)
        let len = |s: &Seg| s.p0.distance(s.p1);
        for i in 0..m {
            let h = len(&out[i]);
            let before = (i > 0 || c.closed).then(|| len(&out[(i + m - 1) % m]));
            let after = (i + 1 < m || c.closed).then(|| len(&out[(i + 1) % m]));
            let reach = |n: Option<f64>| n.map_or(SIGMA, |n| SIGMA.min(0.4 * n / h));
            out[i].lo = -reach(before);
            out[i].hi = 1.0 + reach(after);
        }
        let joins = if c.closed { m } else { m.saturating_sub(1) };
        for i in 0..joins {
            let (s, t) = (&out[i], &out[(i + 1) % m]);
            // both steps end on the plane across the curve at their shared node
            let tn = s.t1;
            let refuse = Refusal {
                region: to_box(&s.x),
                why: Why::Junction,
            };
            if s.p1 != t.p0 || s.t1 != t.t0 {
                return Err(refuse);
            }
            let both = meets(&s.x, &t.x).ok_or(refuse)?;
            // a box about the node, inside both steps' boxes: the shared zero is shown in it, and
            // s's start plane and t's end plane lie either side of it (so the steps run on the same way)
            let h = s.p0.distance(s.p1).min(t.p0.distance(t.p1));
            let joined = [0.25, 0.1, 0.04].into_iter().any(|f| {
                let Some(y) = meets(&both, &grown(s.p1, s.p1, f * h)) else {
                    return false;
                };
                let a = dot(s.t0, &sub_p(&y, s.p0));
                let e = dot(t.t1, &sub_p(&y, t.p1));
                ((a.lo > 0.0 && e.hi < 0.0) || (a.hi < 0.0 && e.lo > 0.0)) && self.pair.junction(&y, s.p1, tn)
            });
            if !joined {
                return Err(refuse);
            }
        }
        Ok(out)
    }
}

/// Whether the walk from `start` shows every box either `gone` or `done`, splitting the rest down
/// to LEAST; the box it could not, if not. `segs` are the segments `near` may look at, filtered to
/// each box's.
fn walk(
    start: V,
    segs: &[Seg],
    mut gone: impl FnMut(&V) -> bool,
    mut done: impl FnMut(&V, &[u32]) -> bool,
    budget: &mut usize,
) -> Result<(), (V, bool)> {
    let all: Vec<u32> = (0..segs.len() as u32)
        .filter(|&i| meets(&segs[i as usize].x, &start).is_some())
        .collect();
    let mut stack = vec![(start, all)];
    while let Some((b, near)) = stack.pop() {
        if *budget == 0 {
            return Err((b, true));
        }
        *budget -= 1;
        if gone(&b) || done(&b, &near) {
            continue;
        }
        let k = (0..3)
            .max_by(|&i, &j| b[i].width().total_cmp(&b[j].width()))
            .expect("three sides");
        if b[k].width() <= LEAST {
            return Err((b, false));
        }
        let (lo, hi) = b[k].split();
        for half in [lo, hi] {
            let mut c = b;
            c[k] = half;
            let n: Vec<u32> = near
                .iter()
                .copied()
                .filter(|&i| meets(&segs[i as usize].x, &c).is_some())
                .collect();
            stack.push((c, n));
        }
    }
    Ok(())
}

/// Certifies the chains `curves` of the pair `a`, `b` as every branch inside `region`, each once.
pub fn certify(a: &Surface, b: &Surface, region: Box3, curves: &[IntCurve]) -> Result<(), Refusal> {
    let out = |why| Refusal { region, why };
    let (Some(fa), Some(fb)) = (Form::of(a), Form::of(b)) else {
        return Err(out(Why::OutOfScope));
    };
    let cert = Cert {
        pair: Pair { a: fa, b: fb },
        sa: a,
        sb: b,
    };
    // every segment of every chain, with its chain and place in it
    let mut segs: Vec<Seg> = Vec::new();
    let mut whose: Vec<(usize, usize, usize)> = Vec::new(); // (chain, index, chain's length)
    let mut closed = Vec::new();
    for (ci, c) in curves.iter().enumerate() {
        if c.pts.len() < 2 || c.tans.len() != c.pts.len() {
            return Err(out(Why::Step));
        }
        let ss = cert.chain(c)?;
        let n = ss.len();
        for (i, s) in ss.into_iter().enumerate() {
            segs.push(s);
            whose.push((ci, i, n));
        }
        closed.push(c.closed);
    }
    let mut budget = BUDGET;
    let fail = |(b, over): (V, bool), why| Refusal {
        region: to_box(&b),
        why: if over { Why::Budget } else { why },
    };
    // completeness: every zero in the region lies in some segment's claim
    walk(
        of_box(&region),
        &segs,
        |b| cert.pair.excludes(b),
        |b, near| near.iter().any(|&i| segs[i as usize].claims(b)),
        &mut budget,
    )
    .map_err(|e| fail(e, Why::Uncovered))?;
    // no zero twice: segments not neighbours in one chain claim no zero in common
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by(|&i, &j| segs[i].x[0].lo.total_cmp(&segs[j].x[0].lo));
    let neighbours = |i: usize, j: usize| {
        let ((ci, ii, n), (cj, ij, _)) = (whose[i], whose[j]);
        ci == cj && (ii.abs_diff(ij) <= 1 || (closed[ci] && ii.abs_diff(ij) == n - 1))
    };
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if segs[j].x[0].lo > segs[i].x[0].hi {
                break;
            }
            if i == j || neighbours(i, j) {
                continue;
            }
            let Some(both) = meets(&segs[i].x, &segs[j].x) else {
                continue;
            };
            let (si, sj) = (segs[i], segs[j]);
            walk(
                both,
                &[],
                |b| cert.pair.excludes(b) || si.disowns(b) || sj.disowns(b),
                |_, _| false,
                &mut budget,
            )
            .map_err(|e| fail(e, Why::Twice))?;
        }
    }
    Ok(())
}

/// Whether the certificate covers the pair (both surfaces have enclosures).
pub fn in_scope(a: &Surface, b: &Surface) -> bool {
    Form::of(a).is_some() && Form::of(b).is_some()
}

thread_local! {
    static LAST: std::cell::Cell<Option<Refusal>> = const { std::cell::Cell::new(None) };
    static COUNTS: std::cell::Cell<[usize; 4]> = const { std::cell::Cell::new([0; 4]) };
}

/// The last refusal on this thread, taken.
pub fn last_refusal() -> Option<Refusal> {
    LAST.with(|l| l.take())
}

/// On this thread so far: [certified first time, certified after a finer trace, refused, out of
/// scope].
pub fn counts() -> [usize; 4] {
    COUNTS.with(|c| c.get())
}

fn count(k: usize) {
    COUNTS.with(|c| {
        let mut v = c.get();
        v[k] += 1;
        c.set(v)
    });
}

/// `trace`, certified: the march's chains if they certify, else a march at a quarter of the step
/// if that certifies, else None (the refusal kept for `last_refusal`). A pair out of the
/// certificate's scope is traced as before.
pub(crate) fn traced(a: &Surface, b: &Surface, region: Box3) -> Option<Vec<IntCurve>> {
    if !in_scope(a, b) {
        count(3);
        return crate::intcurve::trace_marched(a, b, region, 1.0);
    }
    let first = crate::intcurve::trace_marched(a, b, region, 1.0);
    let why = match first.as_deref().map(|cs| certify(a, b, region, cs)) {
        Some(Ok(())) => {
            count(0);
            return first;
        }
        Some(Err(r)) => r,
        None => Refusal {
            region,
            why: Why::Step,
        },
    };
    let finer = crate::intcurve::trace_marched(a, b, region, 0.25);
    match finer.as_deref().map(|cs| certify(a, b, region, cs)) {
        Some(Ok(())) => {
            count(1);
            finer
        }
        r => {
            count(2);
            LAST.with(|l| l.set(Some(r.and_then(Result::err).unwrap_or(why))));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::{Cylinder, Sphere, Torus};
    use forge_math::{Frame, UnitVec3};

    fn cyl(o: Point3, z: Vec3, r: f64) -> Surface {
        Surface::Cylinder(Cylinder {
            frame: Frame::from_z(o, z.normalize().expect("axis")),
            radius: r,
        })
    }

    fn region(h: f64) -> Box3 {
        Box3::from_points([Point3::new(-h, -h, -h), Point3::new(h, h, h)])
    }

    #[test]
    fn crossing_cylinders_certify() {
        let a = cyl(Point3::ORIGIN, Vec3::Z, 5.0);
        let b = cyl(Point3::new(0.0, 0.0, 1.0), Vec3::X, 3.0);
        let r = region(12.0);
        let cs = crate::intcurve::trace_marched(&a, &b, r, 1.0).expect("traced");
        assert_eq!(cs.len(), 2);
        assert_eq!(certify(&a, &b, r, &cs), Ok(()));
    }

    #[test]
    fn sphere_through_cylinder_certifies() {
        let a = cyl(Point3::ORIGIN, Vec3::Z, 4.0);
        let b = Surface::Sphere(Sphere {
            frame: Frame::from_z(Point3::new(3.0, 0.0, 0.0), UnitVec3::Z),
            radius: 5.0,
        });
        let r = region(12.0);
        let cs = crate::intcurve::trace_marched(&a, &b, r, 1.0).expect("traced");
        assert_eq!(certify(&a, &b, r, &cs), Ok(()));
    }

    #[test]
    fn a_missing_branch_is_refused() {
        let a = cyl(Point3::ORIGIN, Vec3::Z, 5.0);
        let b = cyl(Point3::new(0.0, 0.0, 1.0), Vec3::X, 3.0);
        let r = region(12.0);
        let cs = crate::intcurve::trace_marched(&a, &b, r, 1.0).expect("traced");
        let e = certify(&a, &b, r, &cs[..1]).expect_err("one of two loops");
        assert_eq!(e.why, Why::Uncovered);
    }

    #[test]
    fn a_branch_twice_is_refused() {
        let a = cyl(Point3::ORIGIN, Vec3::Z, 5.0);
        let b = cyl(Point3::new(0.0, 0.0, 1.0), Vec3::X, 3.0);
        let r = region(12.0);
        let cs = crate::intcurve::trace_marched(&a, &b, r, 1.0).expect("traced");
        let twice = vec![cs[0].clone(), cs[0].clone(), cs[1].clone()];
        assert_eq!(certify(&a, &b, r, &twice).expect_err("twice").why, Why::Twice);
    }

    #[test]
    fn a_chain_that_jumps_arms_is_refused() {
        // the census bug's shape: a chain whose points run on one loop, then the other
        let a = cyl(Point3::ORIGIN, Vec3::Z, 5.0);
        let b = cyl(Point3::new(0.0, 0.0, 1.0), Vec3::X, 3.0);
        let r = region(12.0);
        let cs = crate::intcurve::trace_marched(&a, &b, r, 1.0).expect("traced");
        let (c, d) = (&cs[0], &cs[1]);
        let k = c.pts.len() / 2;
        let mut pts = c.pts[..k].to_vec();
        let near = d
            .pts
            .iter()
            .copied()
            .min_by(|p, q| p.distance(c.pts[k]).total_cmp(&q.distance(c.pts[k])))
            .expect("points");
        pts.push(near);
        let jumped = crate::intcurve::finish(&a, &b, pts, false, 0.1);
        let e = certify(&a, &b, r, &[jumped, d.clone()]).expect_err("jumped");
        assert!(
            matches!(e.why, Why::Step | Why::Junction | Why::Uncovered),
            "{e:?}"
        );
    }

    #[test]
    fn a_horn_torus_is_refused_at_its_centre_not_traced_wrong() {
        let t = Surface::Torus(Torus {
            frame: Frame::from_z(Point3::ORIGIN, UnitVec3::Z),
            major: 4.0,
            minor: 4.0,
        });
        let p = Surface::Plane(crate::surface::Plane {
            frame: Frame::from_z(Point3::ORIGIN, UnitVec3::X),
        });
        let r = region(10.0);
        if let Some(cs) = traced(&t, &p, r) {
            // certified: then every claim was proven
            assert_eq!(certify(&t, &p, r, &cs), Ok(()));
        } else {
            let e = last_refusal().expect("named");
            assert!(
                e.region.contains(Point3::ORIGIN) || e.region.diagonal() < 20.0,
                "{e:?}"
            );
        }
    }
}
