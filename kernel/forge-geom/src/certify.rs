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

/// a + b, rounded outward: forge_math's `+`, inlined here (it is the hot path of every test).
#[inline]
fn ia(a: I, b: I) -> I {
    I {
        lo: (a.lo + b.lo).next_down(),
        hi: (a.hi + b.hi).next_up(),
    }
}

/// a * b, rounded outward: forge_math's `*`, inlined here (the same operations, the same bits).
#[inline]
fn im(a: I, b: I) -> I {
    let c = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi];
    I {
        lo: c[0].min(c[1]).min(c[2]).min(c[3]).next_down(),
        hi: c[0].max(c[1]).max(c[2]).max(c[3]).next_up(),
    }
}

/// a - x for a float x, rounded outward.
#[inline]
fn isf(a: I, x: f64) -> I {
    I {
        lo: (a.lo - x).next_down(),
        hi: (a.hi - x).next_up(),
    }
}

fn sub_p(v: &V, o: Point3) -> V {
    [isf(v[0], o.x), isf(v[1], o.y), isf(v[2], o.z)]
}

/// d . q, d a float vector.
fn dot(d: Vec3, q: &V) -> I {
    ia(ia(mf(q[0], d.x), mf(q[1], d.y)), mf(q[2], d.z))
}

/// a d (interval times a float vector).
fn scale(a: I, d: Vec3) -> V {
    [mf(a, d.x), mf(a, d.y), mf(a, d.z)]
}

fn add(u: V, v: V) -> V {
    [ia(u[0], v[0]), ia(u[1], v[1]), ia(u[2], v[2])]
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

    /// The Hessian over the box.
    fn hessian(&self, b: &V) -> [[I; 3]; 3] {
        let outer = |u: Vec3, v: Vec3, w: I| -> [[I; 3]; 3] {
            std::array::from_fn(|i| std::array::from_fn(|j| w * (u.dot_k(i) * v.dot_k(j)).widened_point()))
        };
        let sum = |a: [[I; 3]; 3], b: [[I; 3]; 3]| -> [[I; 3]; 3] {
            std::array::from_fn(|i| std::array::from_fn(|j| a[i][j] + b[i][j]))
        };
        let zero = [[I::point(0.0); 3]; 3];
        let two = I::point(2.0);
        match *self {
            Form::Plane { .. } => zero,
            Form::Sphere { .. } => {
                std::array::from_fn(|i| std::array::from_fn(|j| if i == j { two } else { I::point(0.0) }))
            }
            Form::Cylinder { x, y, .. } => sum(outer(x, x, two), outer(y, y, two)),
            Form::Cone {
                x, y, z, sin, cos, ..
            } => {
                let c2 = (cos * cos).widened_point() * 2.0;
                let s2 = (sin * sin).widened_point() * -2.0;
                sum(sum(outer(x, x, c2), outer(y, y, c2)), outer(z, z, s2))
            }
            Form::Torus { o, x, y, z, big, r } => {
                // 8 L L^T + 4 w (x x^T + y y^T + z z^T) - 8 R^2 (x x^T + y y^T), L = lx x + ly y + lz z
                let q = sub_p(b, o);
                let (lx, ly, lz) = (dot(x, &q), dot(y, &q), dot(z, &q));
                let k = I::point(big * big).widened() - I::point(r * r).widened();
                let w = lx.sqr() + ly.sqr() + lz.sqr() + k;
                let l = add(add(scale(lx, x), scale(ly, y)), scale(lz, z));
                let ll: [[I; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| l[i] * l[j] * 8.0));
                let w4 = w * 4.0;
                let e = w4 - I::point(big * big).widened() * 8.0;
                sum(ll, sum(sum(outer(x, x, e), outer(y, y, e)), outer(z, z, w4)))
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

    /// The origin and the radii that bound `Surface::implicit`'s rounding (its formulas are
    /// `distance`'s): at c, the float value is within 32 u (|c - o|_1 + k) of the exact one. Each dot
    /// product is off by at most 3u |c - o|_1, each square, sum, root and subtraction by a few u of
    /// its operands, and a norm passes its inputs' absolute errors on unchanged at most (the
    /// triangle inequality), so the constant is generous.
    fn rounding(&self) -> (Point3, f64) {
        match *self {
            Form::Plane { o, .. } => (o, 0.0),
            Form::Sphere { o, r } | Form::Cylinder { o, r, .. } | Form::Cone { o, r, .. } => (o, r),
            Form::Torus { o, big, r, .. } => (o, big + r),
        }
    }

    /// The surface's own singular point, where its polynomial's gradient vanishes on it: a cone's
    /// apex, a horn torus's centre. A point within rounding of it.
    fn singular(&self) -> Option<Point3> {
        match *self {
            Form::Cone {
                o, z, r, sin, cos, ..
            } if sin != 0.0 => Some(o + z * (-r * cos / sin)),
            Form::Torus { o, big, r, .. } if big == r => Some(o),
            _ => None,
        }
    }

    /// Whether the box holds no point of the surface: the polynomial has no zero there, or the
    /// box lies wholly past the cone's apex, on the nappe the surface does not have.
    fn off(&self, b: &V) -> bool {
        self.off_cheap(b) || !self.value(b).contains_zero()
    }

    /// `off`'s Lipschitz and nappe tests alone.
    fn off_cheap(&self, b: &V) -> bool {
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
        false
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

/// The inverse of a float N x N matrix (Gauss-Jordan, partial pivoting), if it has one.
fn inverse<const N: usize>(m: [[f64; N]; N]) -> Option<[[f64; N]; N]> {
    let mut a = m;
    let mut inv = [[0.0; N]; N];
    for (i, row) in inv.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for c in 0..N {
        let p = (c..N).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[p][c] == 0.0 || !a[p][c].is_finite() {
            return None;
        }
        a.swap(c, p);
        inv.swap(c, p);
        let d = a[c][c];
        for k in 0..N {
            a[c][k] /= d;
            inv[c][k] /= d;
        }
        for r in 0..N {
            if r != c {
                let f = a[r][c];
                for k in 0..N {
                    a[r][k] -= f * a[c][k];
                    inv[r][k] -= f * inv[c][k];
                }
            }
        }
    }
    inv.iter().flatten().all(|e| e.is_finite()).then_some(inv)
}

/// The Krawczyk test: with y in X, fy enclosing F(y) (over every parameter) and j enclosing F's
/// Jacobian over X (and the parameters), whether K = y - C fy + (I - C j)(X - y) lies inside X
/// and ||I - C j|| < 1 (C a float inverse of j's midpoint). Then F has exactly one zero in X for
/// each parameter (verify/krawczyk: `unique_zero`, `maps_into`).
fn krawczyk<const N: usize>(x: &[I; N], y: [f64; N], fy: [I; N], j: &[[I; N]; N]) -> bool {
    let jm = std::array::from_fn(|r| std::array::from_fn(|c| j[r][c].mid()));
    let Some(c) = inverse::<N>(jm) else { return false };
    for r in 0..N {
        let mut cf = I::point(0.0);
        let mut norm = I::point(0.0);
        let mut k = I::point(y[r]);
        for (i, f) in fy.iter().enumerate() {
            cf = cf + *f * c[r][i];
        }
        k = k - cf;
        for col in 0..N {
            let mut m = I::point(if r == col { 1.0 } else { 0.0 });
            for (i, row) in j.iter().enumerate() {
                m = m - row[col] * c[r][i];
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
    /// A step inside a singular site's box: not proven, and claiming nothing (the site claims it).
    site: bool,
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

/// The most of the march's steps one proven step may span.
const LONG: usize = 16;

/// Half the sides tried for a singular site's box, least first. Near a touch the surfaces stay
/// within about k d^2 of each other at a distance d from it, so the exclusion walk needs boxes of
/// about that size there: the box must reach out to where that is affordable.
const SITES: [f64; 4] = [32.0 * tol::LINEAR, 2e-3, 2e-2, 1e-1];

/// The kernel's contact rule, decided once per site: the surfaces touch at x when the other's
/// distance there is within LINEAR (Some(true)), they miss when it is past LINEAR (Some(false));
/// None when the enclosure straddles the rule's edge.
fn touches(d: I) -> Option<bool> {
    if d.lo >= -tol::LINEAR && d.hi <= tol::LINEAR {
        Some(true)
    } else if d.lo > tol::LINEAR || d.hi < -tol::LINEAR {
        Some(false)
    } else {
        None
    }
}

fn cube(c: Point3, h: f64) -> V {
    grown(c, c, h)
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
    /// The singular sites' boxes (cubes about each site's point, half side `site`), and points.
    sites: Vec<(V, Point3)>,
    site: f64,
    /// The region, grown by a step: a step wholly outside it is not proven (it claims nothing the
    /// region needs).
    region: V,
}

impl Cert<'_> {
    /// The singular sites the chains pass: each surface's own singular point where the other
    /// surface touches it, and each point where the surfaces touch (a critical point of f_a on
    /// f_b = 0, isolated by Krawczyk on the Lagrange system ∇f_a + μ ∇f_b = 0, f_b = 0), where the
    /// contact rule says they touch. Refused where the rule cannot be decided.
    fn find_sites(&self, curves: &[IntCurve], region: &Box3) -> Result<Vec<(V, Point3)>, Refusal> {
        let mut sites: Vec<(V, Point3)> = Vec::new();
        let add = |c: Point3, d: I, sites: &mut Vec<(V, Point3)>| -> Result<(), Refusal> {
            if sites.iter().any(|(q, _)| inside(&pt(c), q)) {
                return Ok(());
            }
            match touches(d) {
                Some(true) => sites.push((cube(c, self.site), c)),
                Some(false) => {}
                None => {
                    return Err(Refusal {
                        region: to_box(&cube(c, self.site)),
                        why: Why::Step,
                    });
                }
            }
            Ok(())
        };
        let (fa, fb) = (self.pair.a, self.pair.b);
        for (own, other) in [(fa, fb), (fb, fa)] {
            if let Some(x) = own.singular()
                && region.grow(self.site).contains(x)
            {
                let b = cube(x, 1e-9 * (1.0 + x.to_vec().length()));
                add(x, other.distance(&b), &mut sites)?;
            }
        }
        for c in curves {
            for (i, &p) in c.pts.iter().enumerate() {
                let (_, ga) = fa.eval(&pt(p));
                let (_, gb) = fb.eval(&pt(p));
                let (na, nb) = (
                    Vec3::new(ga[0].mid(), ga[1].mid(), ga[2].mid()),
                    Vec3::new(gb[0].mid(), gb[1].mid(), gb[2].mid()),
                );
                let sin =
                    |na: Vec3, nb: Vec3| na.cross(nb).length() / (na.length() * nb.length()).max(1e-300);
                let here = sin(na, nb);
                if here >= 0.2 {
                    continue;
                }
                // a local least angle along the chain
                let at = |k: usize| {
                    let (_, ga) = fa.eval(&pt(c.pts[k]));
                    let (_, gb) = fb.eval(&pt(c.pts[k]));
                    sin(
                        Vec3::new(ga[0].mid(), ga[1].mid(), ga[2].mid()),
                        Vec3::new(gb[0].mid(), gb[1].mid(), gb[2].mid()),
                    )
                };
                let n = c.pts.len();
                let prev = (i > 0 || c.closed).then(|| at((i + n - 1) % n));
                let next = (i + 1 < n || c.closed).then(|| at((i + 1) % n));
                if prev.is_some_and(|v| v < here) || next.is_some_and(|v| v < here) {
                    continue;
                }
                if let Some((x, box3)) = self.lagrange(p, na, nb) {
                    add(x, fa.distance(&box3), &mut sites)?;
                }
            }
        }
        Ok(sites)
    }

    /// The critical point of f_a on f_b = 0 near p, certified unique in a small box: Newton on the
    /// Lagrange system, then Krawczyk in 4 dimensions. The point and its box.
    fn lagrange(&self, p: Point3, na: Vec3, nb: Vec3) -> Option<(Point3, V)> {
        let (fa, fb) = (self.pair.a, self.pair.b);
        let system = |x: &V, mu: I| -> ([I; 4], [[I; 4]; 4]) {
            let (_, ga) = fa.eval(x);
            let (vb, gb) = fb.eval(x);
            let (ha, hb) = (fa.hessian(x), fb.hessian(x));
            let g = [ga[0] + mu * gb[0], ga[1] + mu * gb[1], ga[2] + mu * gb[2], vb];
            let j = std::array::from_fn(|r| {
                std::array::from_fn(|c| match (r < 3, c < 3) {
                    (true, true) => ha[r][c] + mu * hb[r][c],
                    (true, false) => gb[r],
                    (false, true) => gb[c],
                    (false, false) => I::point(0.0),
                })
            });
            (g, j)
        };
        let mut x = [p.x, p.y, p.z];
        let mut mu = -na.dot(nb) / nb.dot(nb);
        let scale = 1.0 + p.to_vec().length();
        for _ in 0..40 {
            let xv = [I::point(x[0]), I::point(x[1]), I::point(x[2])];
            let (g, j) = system(&xv, I::point(mu));
            let jm: [[f64; 4]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| j[r][c].mid()));
            let inv = inverse::<4>(jm)?;
            let gm = [g[0].mid(), g[1].mid(), g[2].mid(), g[3].mid()];
            let d: [f64; 4] = std::array::from_fn(|r| (0..4).map(|c| inv[r][c] * gm[c]).sum());
            for k in 0..3 {
                x[k] -= d[k];
            }
            mu -= d[3];
            if d[..3].iter().map(|v| v.abs()).fold(0.0, f64::max) <= 1e-14 * scale {
                break;
            }
        }
        let c = Point3::new(x[0], x[1], x[2]);
        if c.distance(p) > 1.0 + 0.5 * scale {
            return None;
        }
        for w in [1e-10, 1e-8, 1e-6] {
            let xb = cube(c, w * scale);
            let mb = I::new(mu - w * (1.0 + mu.abs()), mu + w * (1.0 + mu.abs()));
            let y = [c.x, c.y, c.z, mu];
            let (gy, _) = system(&pt(c), I::point(mu));
            let (_, j) = system(&xb, mb);
            let x4 = [xb[0], xb[1], xb[2], mb];
            if krawczyk::<4>(&x4, y, gy, &j) {
                return Some((c, xb));
            }
        }
        None
    }

    /// One step from (p0, t0) to (p1, t1) proven at once, away from every site and inside the
    /// region's reach: None if not.
    fn one_step(&self, p0: Point3, t0: Vec3, p1: Point3, t1: Vec3) -> Option<Seg> {
        let h = p0.distance(p1);
        let reach = grown(p0, p1, 0.5 * h);
        if meets(&reach, &self.region).is_none() || self.sites.iter().any(|(q, _)| meets(q, &reach).is_some())
        {
            return None;
        }
        [0.3].into_iter().find_map(|f| {
            let s = Seg {
                p0,
                t0,
                p1,
                t1,
                x: grown(p0, p1, f * h + 4.0 * f64::EPSILON * p0.to_vec().length().max(1.0)),
                lo: -SIGMA,
                hi: 1.0 + SIGMA,
                site: false,
            };
            self.pair.step(&s).then_some(s)
        })
    }

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
        // a step with both ends in a site's box lies in it (the box is convex): the site claims it
        let held = |p: Point3| self.sites.iter().position(|(q, _)| inside(&pt(p), q));
        let away = meets(&grown(p0, p1, 0.5 * h), &self.region).is_none();
        let site = match (held(p0), held(p1)) {
            (Some(i), Some(j)) if i == j => Some(self.sites[i].0),
            _ => None,
        };
        if away || site.is_some() {
            out.push(Seg {
                p0,
                t0,
                p1,
                t1,
                x: site.unwrap_or_else(|| grown(p0, p1, 0.5 * h)),
                lo: 0.0,
                hi: 1.0,
                site: true,
            });
            return Ok(());
        }
        for f in [0.25, 0.375, 0.5] {
            let s = Seg {
                p0,
                t0,
                p1,
                t1,
                x: grown(p0, p1, f * h + 4.0 * f64::EPSILON * p0.to_vec().length().max(1.0)),
                lo: -SIGMA,
                hi: 1.0 + SIGMA,
                site: false,
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
        if h <= LEAST || depth >= 40 {
            return Err(refuse);
        }
        // a step that runs through a site is cut there, each part then refined towards it
        if let Some(&(_, c)) = self.sites.iter().find(|(q, c)| {
            !inside(&pt(p0), q) && !inside(&pt(p1), q) && inside(&pt(*c), &grown(p0, p1, 0.25 * h))
        }) {
            self.steps(p0, t0, c, t0, out, depth + 1)?;
            return self.steps(c, t1, p1, t1, out, depth + 1);
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
        // an open chain that stops short of a site (the march stalls at a touch) runs on into it
        let (mut pts, mut tans) = (c.pts.clone(), c.tans.clone());
        if !c.closed {
            let reach = |p: Point3, q: Point3| 1.5 * p.distance(q);
            let near = |end: Point3, next: Point3| {
                self.sites
                    .iter()
                    .map(|&(_, x)| x)
                    .filter(|&x| x.distance(end) <= reach(end, next) && x.distance(end) > 0.0)
                    .min_by(|x, y| x.distance(end).total_cmp(&y.distance(end)))
            };
            if let Some(x) = near(pts[pts.len() - 1], pts[pts.len() - 2]) {
                let t = tans[tans.len() - 1];
                pts.push(x);
                tans.push(t);
            }
            if let Some(x) = near(pts[0], pts[1]) {
                let t = tans[0];
                pts.insert(0, x);
                tans.insert(0, t);
            }
        }
        let n = pts.len();
        let segs = if c.closed { n } else { n - 1 };
        let mut out = Vec::new();
        let mut i = 0;
        // the span tried first: the last that was proven, doubled (a curve's bends change slowly)
        let mut span = LONG;
        while i < segs {
            // the longest step over up to LONG of the march's nodes that is proven at once, every
            // node it passes lying in its claim (where every zero is the step's arc): fewer, wider
            // claims for the coverage walk to fit
            let mut long = None;
            let mut k = span.min(segs - i);
            while k >= 2 {
                let j = (i + k) % n;
                let (p0, p1) = (pts[i], pts[j]);
                let (t0, t1) = (tans[i], tans[j]);
                let (t0, t1) = (
                    if t0.dot(p1 - p0) < 0.0 { -t0 } else { t0 },
                    if t1.dot(p1 - p0) < 0.0 { -t1 } else { t1 },
                );
                if t0.dot(t1) >= 0.5
                    && let Some(s) = self.one_step(p0, t0, p1, t1)
                    && (i + 1..i + k).all(|m| s.claims(&pt(pts[m % n])))
                {
                    long = Some((s, k));
                    break;
                }
                k /= 2;
            }
            if let Some((s, k)) = long {
                out.push(s);
                i += k;
                span = (2 * k).min(LONG);
                continue;
            }
            span = 2;
            let j = (i + 1) % n;
            let (p0, p1) = (pts[i], pts[j]);
            let orient = |t: Vec3| if t.dot(p1 - p0) < 0.0 { -t } else { t };
            self.steps(p0, orient(tans[i]), p1, orient(tans[j]), &mut out, 0)?;
            i += 1;
        }
        // a short loop could be run round twice by steps all neighbours of each other
        if c.closed && out.len() < 10 {
            let mut finer = Vec::new();
            for s in &out {
                if s.site {
                    finer.push(*s);
                    continue;
                }
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
            // a step into or out of a site joins nothing proven: the site claims what lies between
            if s.site || t.site {
                continue;
            }
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

/// A surface's parametrisation over a (u, v) box, for the completeness walk: it covers the
/// surface's polynomial zero set (the cone's nappe), so that walking its domain visits every point
/// of the surface. P(u, v) = o + B l(u, v), with l the local coordinates the form is written in
/// and B an interval enclosure of the exact inverse of the frame's rows (x; y; z).
#[derive(Clone, Copy, Debug)]
struct Host {
    form: Form,
    /// B: the float matrix m, each entry widened by e.
    m: [[f64; 3]; 3],
    e: f64,
}

/// An interval enclosure of the inverse of the near-orthonormal matrix with rows x, y, z: its
/// transpose X, widened by |X| rho / (1 - rho), rho bounding ||I - M X|| (the Neumann series).
fn inverse_rows(x: Vec3, y: Vec3, z: Vec3) -> Option<([[f64; 3]; 3], f64)> {
    let m = [x, y, z];
    let xt: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| m[j].dot_k(i)));
    let mut rho = I::point(0.0);
    let mut row_max: f64 = 0.0;
    for i in 0..3 {
        let mut sum = I::point(0.0);
        #[allow(clippy::needless_range_loop)]
        for j in 0..3 {
            let mut r = I::point(if i == j { 1.0 } else { 0.0 });
            for (k, mk) in m[i].to_vec_k().iter().enumerate() {
                r = r - (*mk * xt[k][j]).widened_point();
            }
            sum = sum + I::point(mag(r));
        }
        rho = rho.hull(&sum);
        row_max = row_max.max(xt[i].iter().map(|v| v.abs()).sum());
    }
    let rho = rho.hi;
    if rho >= 0.5 {
        return None;
    }
    let e = I::point(row_max) * I::point(rho).div(&(I::point(1.0) - I::point(rho)));
    Some((xt, e.hi.next_up()))
}

trait Comps {
    fn to_vec_k(self) -> [f64; 3];
}

impl Comps for Vec3 {
    fn to_vec_k(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

/// a s, a float: two products, rounded outward (forge_math's `*` takes four).
#[inline]
fn mf(a: I, s: f64) -> I {
    let (p, q) = (a.lo * s, a.hi * s);
    I {
        lo: p.min(q).next_down(),
        hi: p.max(q).next_up(),
    }
}

/// B l, with B the float matrix m widened entrywise by e: m l, then e (|l0| + |l1| + |l2|).
fn mat_v(m: &[[f64; 3]; 3], e: f64, l: &V) -> V {
    let spread = if e == 0.0 {
        0.0
    } else {
        (I::point(mag(l[0])) + I::point(mag(l[1])) + I::point(mag(l[2]))).hi * e
    };
    std::array::from_fn(|i| {
        let v = ia(ia(mf(l[0], m[i][0]), mf(l[1], m[i][1])), mf(l[2], m[i][2]));
        I {
            lo: (v.lo - spread).next_down(),
            hi: (v.hi + spread).next_up(),
        }
    })
}

/// The floats just above 2 pi and pi / 2 (dmath's TAU and FRAC_PI_2 lie just below).
const TAU_UP: f64 = forge_math::dmath::TAU.next_up();
const HALF_PI_UP: f64 = forge_math::dmath::FRAC_PI_2.next_up();

impl Host {
    fn of(form: Form) -> Option<Host> {
        let id = (
            std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 })),
            0.0,
        );
        let (m, e) = match form {
            Form::Sphere { .. } => id,
            Form::Plane { o: _, z } => {
                // any two directions completing z: the plane's points are o + u x + v y, and z.q = 0
                // fixes q's z-component only, so x, y need only span the plane with z
                let x = if z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
                let x = (x - z * z.dot(x)).normalize()?.get();
                let y = z.cross(x);
                inverse_rows(x, y, z)?
            }
            Form::Cylinder { x, y, .. } => inverse_rows(x, y, x.cross(y))?,
            Form::Cone { x, y, z, .. } | Form::Torus { x, y, z, .. } => inverse_rows(x, y, z)?,
        };
        Some(Host { form, m, e })
    }

    /// A (u, v) box whose image holds every point of the surface in the region.
    fn domain(&self, region: &V) -> Option<(I, I)> {
        let whole = I::new(0.0, TAU_UP);
        let along = |d: Vec3, o: Point3| dot(d, &sub_p(region, o));
        Some(match self.form {
            Form::Plane { o, z } => {
                // u, v are the local coordinates of B's first two columns' duals: x.q, y.q with x, y
                // the rows B inverts; from the region, through the rows themselves
                let x = if z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
                let x = (x - z * z.dot(x)).normalize()?.get();
                let y = z.cross(x);
                (along(x, o), along(y, o))
            }
            Form::Cylinder { o, x, y, .. } => (whole, along(x.cross(y), o)),
            Form::Cone {
                o, z, r, sin, cos, ..
            } => {
                let v = along(z, o);
                let apex = (I::point(-r * cos).widened().div(&I::point(sin))).lo;
                // (empty when the region lies wholly past the apex: no point of the nappe in it)
                (
                    whole,
                    I {
                        lo: v.lo.max(apex),
                        hi: v.hi,
                    },
                )
            }
            Form::Sphere { .. } => (whole, I::new(-HALF_PI_UP, HALF_PI_UP)),
            Form::Torus { .. } => (whole, whole),
        })
    }
}

/// A polynomial in (t, v) with interval coefficients, c[i * 9 + j] of t^i v^j, of degree at most 8
/// in each (`over`: a product went past that, and the polynomial is not held).
#[derive(Clone, Copy, Debug)]
struct Bp {
    dt: usize,
    dv: usize,
    over: bool,
    c: [I; 81],
}

const ZERO: I = I { lo: 0.0, hi: 0.0 };

impl Bp {
    fn zero(dt: usize, dv: usize) -> Bp {
        Bp {
            dt: dt.min(8),
            dv: dv.min(8),
            over: dt > 8 || dv > 8,
            c: [ZERO; 81],
        }
    }
    #[inline]
    fn at(&self, i: usize, j: usize) -> I {
        if i <= self.dt && j <= self.dv {
            self.c[i * 9 + j]
        } else {
            ZERO
        }
    }
    fn of(terms: &[(usize, usize, I)]) -> Bp {
        let dt = terms.iter().map(|t| t.0).max().unwrap_or(0);
        let dv = terms.iter().map(|t| t.1).max().unwrap_or(0);
        let mut p = Bp::zero(dt, dv);
        for &(i, j, x) in terms {
            if i <= 8 && j <= 8 {
                p.c[i * 9 + j] = ia(p.c[i * 9 + j], x);
            }
        }
        p
    }
    fn add(&self, o: &Bp) -> Bp {
        let mut p = Bp::zero(self.dt.max(o.dt), self.dv.max(o.dv));
        p.over |= self.over || o.over;
        for i in 0..=p.dt {
            for j in 0..=p.dv {
                p.c[i * 9 + j] = ia(self.at(i, j), o.at(i, j));
            }
        }
        p
    }
    fn scale(&self, x: I) -> Bp {
        let mut p = *self;
        for i in 0..=p.dt {
            for j in 0..=p.dv {
                p.c[i * 9 + j] = im(p.c[i * 9 + j], x);
            }
        }
        p
    }
    fn mul(&self, o: &Bp) -> Bp {
        let mut p = Bp::zero(self.dt + o.dt, self.dv + o.dv);
        p.over |= self.over || o.over;
        if p.over {
            return p;
        }
        for i in 0..=self.dt {
            for j in 0..=self.dv {
                let a = self.c[i * 9 + j];
                if a == ZERO {
                    continue;
                }
                for k in 0..=o.dt {
                    for l in 0..=o.dv {
                        let b = o.c[k * 9 + l];
                        if b != ZERO {
                            let at = (i + k) * 9 + j + l;
                            p.c[at] = ia(p.c[at], im(a, b));
                        }
                    }
                }
            }
        }
        p
    }
}

/// The Bernstein coefficients over [t0, t1] x [v0, v1] of a polynomial: t = t0 + (t1 - t0) a,
/// v = v0 + (v1 - v0) b, each direction shifted, scaled and turned into Bernstein form.
fn bernstein(p: &Bp, t: I, v: I) -> Coeffs {
    let (n, m) = (p.dt, p.dv);
    debug_assert!(n <= 8 && m <= 8);
    // C(k, i) / C(d, i) for d = n and d = m, each enclosed once
    let binom =
        |n: usize, k: usize| -> f64 { (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64) };
    let ratios = |d: usize| {
        let mut r = [[I::point(0.0); 9]; 9];
        for (k, row) in r.iter_mut().enumerate().take(d + 1) {
            for (i, x) in row.iter_mut().enumerate().take(k + 1) {
                *x = I::point(binom(k, i)).div(&I::point(binom(d, i)));
            }
        }
        r
    };
    // univariate power-to-Bernstein on [lo, hi] of coefficients c[..=d] (ascending), in place
    let uni = |c: &mut [I; 9], d: usize, lo: f64, hi: f64, r: &[[I; 9]; 9]| {
        // shift: p(lo + h x), Horner-like Taylor shift
        for k in 0..d {
            for i in (k..d).rev() {
                c[i] = ia(c[i], im(c[i + 1], I::point(lo)));
            }
        }
        let h = I::point(hi) - I::point(lo);
        let mut hp = I::point(1.0);
        for ci in c.iter_mut().take(d + 1) {
            *ci = im(*ci, hp);
            hp = im(hp, h);
        }
        // power to Bernstein: b_k = sum_{i <= k} C(k, i) / C(d, i) a_i
        let a = *c;
        for k in 0..=d {
            c[k] = (0..=k).fold(I::point(0.0), |acc, i| ia(acc, im(a[i], r[k][i])));
        }
    };
    let (rn, rm) = (ratios(n), if m == n { ratios(n) } else { ratios(m) });
    let mut rows = [I::point(0.0); BERN];
    // along v for each power of t
    for i in 0..=n {
        let mut a = [I::point(0.0); 9];
        for (j, x) in a.iter_mut().enumerate().take(m + 1) {
            *x = p.at(i, j);
        }
        uni(&mut a, m, v.lo, v.hi, &rm);
        rows[i * (m + 1)..i * (m + 1) + m + 1].copy_from_slice(&a[..=m]);
    }
    // then along t for each Bernstein index of v
    let mut out = [I::point(0.0); BERN];
    for j in 0..=m {
        let mut a = [I::point(0.0); 9];
        for (i, x) in a.iter_mut().enumerate().take(n + 1) {
            *x = rows[i * (m + 1) + j];
        }
        uni(&mut a, n, t.lo, t.hi, &rn);
        for i in 0..=n {
            out[i * (m + 1) + j] = a[i];
        }
    }
    out
}

/// The most Bernstein coefficients a chart has: bidegree (8, 8).
const BERN: usize = 81;
type Coeffs = [I; BERN];

/// (a + b) / 2, rounded outward: the sum rounded out, then halved exactly.
fn avg(a: I, b: I) -> I {
    I {
        lo: (a.lo + b.lo).next_down() * 0.5,
        hi: (a.hi + b.hi).next_up() * 0.5,
    }
}

/// De Casteljau at the middle, along t (`along_t`) or v: the two halves' coefficients.
///
/// The lo ends and the hi ends each run in plain floats, then every output is widened once. The
/// exact chain's values are averages of the inputs, so |x| <= M (the inputs' largest magnitude).
/// One average, (a + b) * 0.5, is off by at most 2^-53 |a + b| <= 2^-52 M from rounding the sum,
/// plus 2^-1075 where the halving lands below the normal range. An average of two values that are
/// each off by at most e is off by at most e plus that new error, so after `len` levels every
/// output is within E = len (2^-52 M + 2^-1074) of the exact chain on the same ends. The exact
/// chain on the lo ends is a convex combination of them, so it is at most the true coefficient,
/// and lo - E (rounded down) is a lower bound. The same holds for hi. E itself is computed with
/// a 2^-40 margin. Where M is not a modest finite number, the outward chain decides instead.
fn halve(c: &Coeffs, n: usize, m: usize, along_t: bool) -> (Coeffs, Coeffs) {
    debug_assert!(n <= 8 && m <= 8);
    let count = (n + 1) * (m + 1);
    let big = c[..count]
        .iter()
        .fold(0.0f64, |a, x| a.max(x.lo.abs()).max(x.hi.abs()));
    let finite = c[..count].iter().all(|x| x.lo.is_finite() && x.hi.is_finite());
    if !finite || big >= 1e300 {
        return halve_outward(c, n, m, along_t);
    }
    let (len, lines) = if along_t { (n, m + 1) } else { (m, n + 1) };
    let idx = |k: usize, line: usize| {
        if along_t {
            k * (m + 1) + line
        } else {
            line * (m + 1) + k
        }
    };
    let e = len as f64 * (big * f64::EPSILON + f64::from_bits(1)) * (1.0 + 1.0 / (1u64 << 40) as f64);
    let (mut l, mut r) = (*c, *c);
    let (mut lo, mut hi) = ([0.0f64; 9], [0.0f64; 9]);
    let (mut llo, mut lhi, mut rlo, mut rhi) = ([0.0f64; 9], [0.0f64; 9], [0.0f64; 9], [0.0f64; 9]);
    for line in 0..lines {
        for k in 0..=len {
            let x = c[idx(k, line)];
            lo[k] = x.lo;
            hi[k] = x.hi;
        }
        llo[0] = lo[0];
        lhi[0] = hi[0];
        rlo[len] = lo[len];
        rhi[len] = hi[len];
        for step in 1..=len {
            for k in 0..=len - step {
                lo[k] = (lo[k] + lo[k + 1]) * 0.5;
                hi[k] = (hi[k] + hi[k + 1]) * 0.5;
            }
            llo[step] = lo[0];
            lhi[step] = hi[0];
            rlo[len - step] = lo[len - step];
            rhi[len - step] = hi[len - step];
        }
        // the ends of the chain (k = 0 on the left, len on the right) are inputs, exact
        l[idx(0, line)] = c[idx(0, line)];
        r[idx(len, line)] = c[idx(len, line)];
        for k in 1..=len {
            l[idx(k, line)] = I {
                lo: (llo[k] - e).next_down(),
                hi: (lhi[k] + e).next_up(),
            };
            r[idx(len - k, line)] = I {
                lo: (rlo[len - k] - e).next_down(),
                hi: (rhi[len - k] + e).next_up(),
            };
        }
    }
    (l, r)
}

/// `halve` with every average rounded outward: for inputs too large for its one error bound.
fn halve_outward(c: &Coeffs, n: usize, m: usize, along_t: bool) -> (Coeffs, Coeffs) {
    let (mut l, mut r) = (*c, *c);
    let (len, lines) = if along_t { (n, m + 1) } else { (m, n + 1) };
    let idx = |k: usize, line: usize| {
        if along_t {
            k * (m + 1) + line
        } else {
            line * (m + 1) + k
        }
    };
    let mut b = [I::point(0.0); 9];
    for line in 0..lines {
        for (k, bk) in b.iter_mut().enumerate().take(len + 1) {
            *bk = c[idx(k, line)];
        }
        l[idx(0, line)] = b[0];
        r[idx(len, line)] = b[len];
        for step in 1..=len {
            for k in 0..=len - step {
                b[k] = avg(b[k], b[k + 1]);
            }
            l[idx(step, line)] = b[0];
            r[idx(len - step, line)] = b[len - step];
        }
    }
    (l, r)
}

/// Whether the coefficients are all of one strict sign (the polynomial has no zero on the box).
fn one_sign(c: &[I]) -> bool {
    c.iter().all(|x| x.lo > 0.0) || c.iter().all(|x| x.hi < 0.0)
}

/// A line host's chart: points P = o + B l(t, v) / w(t) over (t, v), with l and w polynomials.
struct Chart {
    /// The other form times w^deg, a polynomial in (t, v) with the same zeros.
    f: Bp,
    t: I,
    v: I,
    /// The half turns of u's and v's charts (1 or -1: cos and sin negated).
    turn: f64,
    turn_v: f64,
    /// False for a chart left out (`charts_in` with `skip`): its image misses the region or is
    /// clear of the other form, and f was not built.
    live: bool,
}

impl Host {
    /// The charts of a line host (plane: one, in (u, v); cylinder and cone: two, t = tan(u/2)
    /// over [-1, 1] and the half turn), with the other form as a polynomial on each.
    fn charts(&self, other: &Form, region: &V) -> Option<Vec<Chart>> {
        self.charts_in(other, region, false)
    }

    /// `charts`, each chart whose image misses the region or is clear of the other form left
    /// out with `skip` (`live` false, no polynomial built).
    fn charts_in(&self, other: &Form, region: &V, skip: bool) -> Option<Vec<Chart>> {
        let (_, v) = self.domain(region)?;
        if v.is_empty() {
            return Some(Vec::new());
        }
        let z = I::point(0.0);
        let one = I::point(1.0);
        let b: [[I; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| I::point(self.m[i][j]).grow(self.e)));
        let oh = match self.form {
            Form::Plane { o, .. } | Form::Cylinder { o, .. } | Form::Cone { o, .. } => o,
            Form::Sphere { o, .. } | Form::Torus { o, .. } => o,
        };
        let of = match *other {
            Form::Plane { o, .. } | Form::Sphere { o, .. } | Form::Cylinder { o, .. } => o,
            Form::Cone { o, .. } | Form::Torus { o, .. } => o,
        };
        let d = [
            I::point(oh.x) - I::point(of.x),
            I::point(oh.y) - I::point(of.y),
            I::point(oh.z) - I::point(of.z),
        ];
        let mut out = Vec::new();
        let turns: &[(f64, f64)] = match self.form {
            Form::Plane { .. } => &[(0.0, 0.0)],
            Form::Cylinder { .. } | Form::Cone { .. } => &[(1.0, 0.0), (-1.0, 0.0)],
            Form::Sphere { .. } | Form::Torus { .. } => &[(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)],
        };
        // cos and sin of a half-angle chart, times its weight 1 + x^2, in the variable at (i, j)
        let cosw = |turn: f64, in_t: bool| -> Bp {
            if in_t {
                Bp::of(&[(0, 0, I::point(turn)), (2, 0, I::point(-turn))])
            } else {
                Bp::of(&[(0, 0, I::point(turn)), (0, 2, I::point(-turn))])
            }
        };
        let sinw = |turn: f64, in_t: bool| -> Bp {
            if in_t {
                Bp::of(&[(1, 0, I::point(2.0 * turn))])
            } else {
                Bp::of(&[(0, 1, I::point(2.0 * turn))])
            }
        };
        for &(turn, turn_v) in turns {
            let t = match self.form {
                Form::Plane { .. } => self.domain(region)?.0,
                _ => I::new(-1.0, 1.0),
            };
            let v = match self.form {
                Form::Sphere { .. } | Form::Torus { .. } => I::new(-1.0, 1.0),
                _ => v,
            };
            if skip {
                let dead = Chart {
                    f: Bp::zero(0, 0),
                    t,
                    v,
                    turn,
                    turn_v,
                    live: false,
                };
                let image = self.chart_box(&dead, t, v);
                if meets(&image, region).is_none() || other.off_cheap(&image) {
                    out.push(dead);
                    continue;
                }
            }
            // l(t, v) homogenised by w (plane: w = 1, l = (u, v, 0); a line host: w = 1 + t^2;
            // sphere and torus: w = (1 + t^2)(1 + s^2), s the v chart's variable)
            let (w, l): (Bp, [Bp; 3]) = match self.form {
                Form::Plane { .. } => (
                    Bp::of(&[(0, 0, one)]),
                    [
                        Bp::of(&[(1, 0, one)]),
                        Bp::of(&[(0, 1, one)]),
                        Bp::of(&[(0, 0, z)]),
                    ],
                ),
                Form::Cylinder { r, .. } => (
                    Bp::of(&[(0, 0, one), (2, 0, one)]),
                    [
                        cosw(turn, true).scale(I::point(r)),
                        sinw(turn, true).scale(I::point(r)),
                        Bp::of(&[(0, 1, one), (2, 1, one)]),
                    ],
                ),
                Form::Cone { r, sin, cos, .. } => {
                    let tan = I::point(sin).div(&I::point(cos));
                    let rho = Bp::of(&[(0, 0, I::point(r)), (0, 1, tan)]);
                    (
                        Bp::of(&[(0, 0, one), (2, 0, one)]),
                        [
                            rho.mul(&cosw(turn, true)),
                            rho.mul(&sinw(turn, true)),
                            Bp::of(&[(0, 1, one), (2, 1, one)]),
                        ],
                    )
                }
                Form::Sphere { r, .. } | Form::Torus { r, .. } => {
                    let big = match self.form {
                        Form::Torus { big, .. } => big,
                        _ => 0.0,
                    };
                    let (wu, wv) = (
                        Bp::of(&[(0, 0, one), (2, 0, one)]),
                        Bp::of(&[(0, 0, one), (0, 2, one)]),
                    );
                    // (R + r cos v) w_v = R w_v + r cos_w(v)
                    let ring = wv
                        .scale(I::point(big))
                        .add(&cosw(turn_v, false).scale(I::point(r)));
                    (
                        wu.mul(&wv),
                        [
                            ring.mul(&cosw(turn, true)),
                            ring.mul(&sinw(turn, true)),
                            sinw(turn_v, false).scale(I::point(r)).mul(&wu),
                        ],
                    )
                }
            };
            // Q = w (o_host - o_other) + B l, the point relative to the other's origin, times w
            let q: [Bp; 3] =
                std::array::from_fn(|i| (0..3).fold(w.scale(d[i]), |acc, k| acc.add(&l[k].scale(b[i][k]))));
            let dotq = |e: Vec3| {
                q[0].scale(I::point(e.x))
                    .add(&q[1].scale(I::point(e.y)))
                    .add(&q[2].scale(I::point(e.z)))
            };
            let sq = |p: &Bp| p.mul(p);
            let w2 = w.mul(&w);
            let f = match *other {
                Form::Plane { z, .. } => dotq(z),
                Form::Sphere { r, .. } => sq(&q[0])
                    .add(&sq(&q[1]))
                    .add(&sq(&q[2]))
                    .add(&w2.scale(-I::point(r * r).widened())),
                Form::Cylinder { x, y, r, .. } => sq(&dotq(x))
                    .add(&sq(&dotq(y)))
                    .add(&w2.scale(-I::point(r * r).widened())),
                Form::Cone {
                    x, y, z, r, sin, cos, ..
                } => {
                    let rho2 = sq(&dotq(x)).add(&sq(&dotq(y)));
                    let lin = w
                        .scale(I::point(r * cos).widened())
                        .add(&dotq(z).scale(I::point(sin)));
                    rho2.scale(I::point(cos * cos).widened())
                        .add(&sq(&lin).scale(-one))
                }
                Form::Torus { x, y, z, big, r, .. } => {
                    let rho2 = sq(&dotq(x)).add(&sq(&dotq(y)));
                    let k = I::point(big * big).widened() - I::point(r * r).widened();
                    let a = rho2.add(&sq(&dotq(z))).add(&w2.scale(k));
                    sq(&a).add(&w2.mul(&rho2).scale(-(I::point(big * big).widened() * 4.0)))
                }
            };
            if f.over {
                return None;
            }
            out.push(Chart {
                f,
                t,
                v,
                turn,
                turn_v,
                live: true,
            });
        }
        Some(out)
    }

    /// Bounds on how far a chart's image moves per unit of t and of v over the region (for choosing
    /// which way to split; any positive values are sound).
    fn chart_rates(&self, region: &V) -> (f64, f64) {
        let col =
            |j: usize| (0..3).map(|i| self.m[i][j].abs() + self.e).fold(0.0, f64::max) * 1.7320508075688772;
        match self.form {
            Form::Plane { .. } => (col(0), col(1)),
            // d(cos, sin)/dt is at most 2 in length on [-1, 1]
            Form::Cylinder { r, .. } => (2.0 * r * col(0).max(col(1)), col(2)),
            Form::Cone { r, sin, cos, .. } => {
                let tan = (sin / cos).abs();
                let v = self.domain(region).map_or(0.0, |(_, v)| mag(v));
                (
                    2.0 * (r + v * tan) * col(0).max(col(1)),
                    col(2) + tan * col(0).max(col(1)),
                )
            }
            Form::Sphere { r, .. } => (
                2.0 * r * col(0).max(col(1)).max(col(2)),
                2.0 * r * col(0).max(col(1)).max(col(2)),
            ),
            Form::Torus { big, r, .. } => {
                let c = col(0).max(col(1)).max(col(2));
                (2.0 * (big + r) * c, 2.0 * r * c)
            }
        }
    }

    /// A chart's (t, v) point, in floats: a guide, not an enclosure (chart_box is that).
    fn chart_point(&self, ch: &Chart, t: f64, v: f64) -> Point3 {
        let half = |x: f64, turn: f64| -> (f64, f64) {
            let w = 1.0 + x * x;
            ((1.0 - x * x) / w * turn, 2.0 * x / w * turn)
        };
        let (l, o): ([f64; 3], Point3) = match self.form {
            Form::Plane { o, .. } => ([t, v, 0.0], o),
            Form::Cylinder { o, r, .. } | Form::Cone { o, r, .. } => {
                let (c, s) = half(t, ch.turn);
                let rho = match self.form {
                    Form::Cone { sin, cos, .. } => r + v * (sin / cos),
                    _ => r,
                };
                ([c * rho, s * rho, v], o)
            }
            Form::Sphere { o, r } | Form::Torus { o, r, .. } => {
                let big = match self.form {
                    Form::Torus { big, .. } => big,
                    _ => 0.0,
                };
                let (cu, su) = half(t, ch.turn);
                let (cv, sv) = half(v, ch.turn_v);
                let ring = big + cv * r;
                ([cu * ring, su * ring, sv * r], o)
            }
        };
        let m = &self.m;
        Point3::new(
            o.x + m[0][0] * l[0] + m[0][1] * l[1] + m[0][2] * l[2],
            o.y + m[1][0] * l[0] + m[1][1] * l[1] + m[1][2] * l[2],
            o.z + m[2][0] * l[0] + m[2][1] * l[1] + m[2][2] * l[2],
        )
    }

    /// The points of a chart's (t, v) box.
    fn chart_box(&self, ch: &Chart, t: I, v: I) -> V {
        // cos and sin over a half-angle chart's x in [-1, 1]: (1 - x^2)/(1 + x^2) falls with |x|,
        // 2x/(1 + x^2) rises
        let half = |x: I, turn: f64| -> (I, I) {
            let at = |x: f64| -> (I, I) {
                let x = I::point(x);
                let w = I::point(1.0) + x.sqr();
                ((I::point(1.0) - x.sqr()).div(&w), (x * 2.0).div(&w))
            };
            let (cl, sl) = at(x.lo);
            let (ch_, sh) = at(x.hi);
            let mut c = cl.hull(&ch_);
            if x.contains_zero() {
                c.hi = 1.0;
            }
            (c * turn, sl.hull(&sh) * turn)
        };
        let z0 = I::point(0.0);
        let (l, o): (V, Point3) = match self.form {
            Form::Plane { o, .. } => ([t, v, z0], o),
            Form::Cylinder { o, r, .. } | Form::Cone { o, r, .. } => {
                let (c, s) = half(t, ch.turn);
                let rho = match self.form {
                    Form::Cone { sin, cos, .. } => I::point(r) + v * I::point(sin).div(&I::point(cos)),
                    _ => I::point(r),
                };
                ([c * rho, s * rho, v], o)
            }
            Form::Sphere { o, r } | Form::Torus { o, r, .. } => {
                let big = match self.form {
                    Form::Torus { big, .. } => big,
                    _ => 0.0,
                };
                let (cu, su) = half(t, ch.turn);
                let (cv, sv) = half(v, ch.turn_v);
                let ring = I::point(big) + cv * r;
                ([cu * ring, su * ring, sv * r], o)
            }
        };
        let p = mat_v(&self.m, self.e, &l);
        [p[0] + I::point(o.x), p[1] + I::point(o.y), p[2] + I::point(o.z)]
    }
}

/// The completeness walk for a line host in Bernstein form: each (t, v) box is gone when its image
/// misses the region, when its Bernstein coefficients are all of one sign (no zero: the convex hull
/// property), or when the other surface's Lipschitz or nappe test clears its image; done when its
/// image lies in a step's claim or a site; else halved by de Casteljau.
fn walk_bernstein(
    host: &Host,
    other: &Form,
    region: &V,
    segs: &[Seg],
    sites: &[(V, Point3)],
    budget: &mut usize,
) -> Result<(), (V, bool)> {
    let Some(charts) = host.charts(other, region) else {
        return Err((*region, false));
    };
    let near = |p: &V, from: &[u32]| -> Vec<u32> {
        from.iter()
            .copied()
            .filter(|&i| !segs[i as usize].site && meets(&segs[i as usize].x, p).is_some())
            .collect()
    };
    let all: Vec<u32> = (0..segs.len() as u32).collect();
    // how far the image moves per unit of t and of v, bounded above
    let (rt, rv) = host.chart_rates(region);
    for ch in &charts {
        let (n, m) = (ch.f.dt, ch.f.dv);
        if (n + 1) * (m + 1) > BERN {
            return Err((*region, false));
        }
        let first = bernstein(&ch.f, ch.t, ch.v);
        let mut stack = vec![(ch.t, ch.v, first, all.clone())];
        while let Some((t, v, c, from)) = stack.pop() {
            if *budget == 0 {
                return Err((host.chart_box(ch, t, v), true));
            }
            *budget -= 1;
            if one_sign(&c[..(n + 1) * (m + 1)]) {
                continue;
            }
            let p = host.chart_box(ch, t, v);
            if meets(&p, region).is_none() || other.off_cheap(&p) {
                continue;
            }
            let cand = near(&p, &from);
            if sites.iter().any(|(q, _)| inside(&p, q)) || cand.iter().any(|&i| segs[i as usize].claims(&p)) {
                continue;
            }
            let (lt, lv) = (rt * t.width(), rv * v.width());
            if lt.max(lv) <= LEAST {
                return Err((p, false));
            }
            let along_t = lt >= lv;
            let (cl, cr) = halve(&c, n, m, along_t);
            if along_t {
                let (a, b) = t.split();
                stack.push((a, v, cl, cand.clone()));
                stack.push((b, v, cr, cand));
            } else {
                let (a, b) = v.split();
                stack.push((t, a, cl, cand.clone()));
                stack.push((t, b, cr, cand));
            }
        }
    }
    Ok(())
}

/// A step proven in a chart: the zeros of the chart's polynomial in [t] x [s] are one graph (over t
/// or over s), so one connected arc of one branch, holding the step's nodes.
#[derive(Clone, Copy, Debug)]
struct GBox {
    chart: usize,
    t: I,
    s: I,
    /// A graph over t (else over s).
    over_t: bool,
}

/// Whether the zeros of the polynomial in the box form one graph: over t when F is monotone in s
/// (dF/ds's Bernstein coefficients one-signed) with F one-signed and of opposite signs on the two
/// s-sides (exactly one zero on every t-line, by the intermediate value theorem), or the same with t
/// and s swapped.
fn graph_box(f: &Bp, t: I, s: I) -> bool {
    graph_way(f, t, s).is_some()
}

/// `graph_box`, with the variable the graph is over: Some(true) for t, Some(false) for s.
fn graph_way(f: &Bp, t: I, s: I) -> Option<bool> {
    let (n, m) = (f.dt, f.dv);
    if n == 0 || m == 0 || (n + 1) * (m + 1) > BERN {
        return None;
    }
    let c = bernstein(f, t, s);
    let at = |i: usize, j: usize| c[i * (m + 1) + j];
    let sign = |it: &mut dyn Iterator<Item = I>| -> i8 {
        let (mut pos, mut neg) = (true, true);
        for x in it {
            pos &= x.lo > 0.0;
            neg &= x.hi < 0.0;
        }
        if pos {
            1
        } else if neg {
            -1
        } else {
            0
        }
    };
    let over_t = {
        let d = sign(
            &mut (0..=n)
                .flat_map(|i| (0..m).map(move |j| (i, j)))
                .map(|(i, j)| at(i, j + 1) - at(i, j)),
        );
        let lo = sign(&mut (0..=n).map(|i| at(i, 0)));
        let hi = sign(&mut (0..=n).map(|i| at(i, m)));
        d != 0 && lo != 0 && hi != 0 && lo != hi
    };
    if over_t {
        return Some(true);
    }
    let d = sign(
        &mut (0..n)
            .flat_map(|i| (0..=m).map(move |j| (i, j)))
            .map(|(i, j)| at(i + 1, j) - at(i, j)),
    );
    let lo = sign(&mut (0..=m).map(|j| at(0, j)));
    let hi = sign(&mut (0..=m).map(|j| at(n, j)));
    (d != 0 && lo != 0 && hi != 0 && lo != hi).then_some(false)
}

/// -1/x over an interval clear of 0 (the half turn between two half-angle charts), rounded out:
/// it holds the true image.
fn flip(x: I) -> Option<I> {
    (!x.contains_zero() && x.lo.is_finite() && x.hi.is_finite()).then(|| I::point(-1.0).div(&x))
}

/// -1/x rounded in: it lies inside the true image (-1/x rises on either side of 0, and each bound,
/// rounded to nearest, is pushed a float inward), for claims and joins, which must not reach past
/// the box they stand for.
fn flip_in(x: I) -> Option<I> {
    if x.contains_zero() || !(x.lo.is_finite() && x.hi.is_finite()) {
        return None;
    }
    let r = I {
        lo: (-1.0 / x.lo).next_up(),
        hi: (-1.0 / x.hi).next_down(),
    };
    (r.lo <= r.hi).then_some(r)
}

impl Host {
    /// A point's chart and (t, s) there (floats: the point is located, not proven).
    fn locate(&self, p: Point3) -> Option<(usize, f64, f64)> {
        let rel = |o: Point3| p - o;
        // tan(a/2) of the angle with cosine c / r and sine y / r, in the chart with |t| <= 1
        let half = |c: f64, y: f64, r: f64| -> (bool, f64) {
            if c >= 0.0 {
                (false, y / (r + c))
            } else {
                (true, -y / (r - c))
            }
        };
        match self.form {
            Form::Plane { o, z } => {
                let x = if z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
                let x = (x - z * z.dot(x)).normalize()?.get();
                let y = z.cross(x);
                let q = rel(o);
                Some((0, x.dot(q), y.dot(q)))
            }
            Form::Cylinder { o, x, y, .. } => {
                let q = rel(o);
                let (lx, ly) = (x.dot(q), y.dot(q));
                let rho = (lx * lx + ly * ly).sqrt();
                let (flip, t) = half(lx, ly, rho);
                Some((flip as usize, t, x.cross(y).dot(q)))
            }
            Form::Cone { o, x, y, z, .. } => {
                let q = rel(o);
                let (lx, ly) = (x.dot(q), y.dot(q));
                let rho = (lx * lx + ly * ly).sqrt();
                let (flip, t) = half(lx, ly, rho);
                Some((flip as usize, t, z.dot(q)))
            }
            Form::Sphere { o, r } | Form::Torus { o, r, .. } => {
                let q = rel(o);
                let (lx, ly, lz) = match self.form {
                    Form::Torus { x, y, z, .. } => (x.dot(q), y.dot(q), z.dot(q)),
                    _ => (q.x, q.y, q.z),
                };
                let big = match self.form {
                    Form::Torus { big, .. } => big,
                    _ => 0.0,
                };
                let rho = (lx * lx + ly * ly).sqrt();
                let (fu, t) = half(lx, ly, rho);
                let (fv, s) = half(rho - big, lz, r);
                Some((2 * fu as usize + fv as usize, t, s))
            }
        }
    }

    /// Whether flipping u (t to -1/t) and v (s to -1/s) takes chart `from` to chart `to`.
    fn flips(&self, from: usize, to: usize) -> (bool, bool) {
        match self.form {
            Form::Plane { .. } => (false, false),
            Form::Cylinder { .. } | Form::Cone { .. } => (from != to, false),
            _ => ((from ^ to) & 2 != 0, (from ^ to) & 1 != 0),
        }
    }

    /// A chart's (t, s) point in chart `to`, if it is finite there.
    fn move_point(&self, from: usize, to: usize, t: f64, s: f64) -> Option<(f64, f64)> {
        let (fu, fv) = self.flips(from, to);
        let t = if fu {
            if t == 0.0 { return None } else { -1.0 / t }
        } else {
            t
        };
        let s = if fv {
            if s == 0.0 { return None } else { -1.0 / s }
        } else {
            s
        };
        Some((t, s))
    }

    /// A chart box in chart `to`, if the flip keeps it a box: holding its points (`outer`), or held
    /// by them.
    fn move_box(&self, g: &GBox, to: usize, outer: bool) -> Option<GBox> {
        let (fu, fv) = self.flips(g.chart, to);
        let f = |x: I| if outer { flip(x) } else { flip_in(x) };
        Some(GBox {
            chart: to,
            t: if fu { f(g.t)? } else { g.t },
            s: if fv { f(g.s)? } else { g.s },
            over_t: g.over_t,
        })
    }
}

/// A chain's steps as graph boxes in the host's charts, each over as many of its nodes as one box
/// holds as a graph, consecutive boxes joined by a graph box in their meet. None if any step or
/// join fails (the chain then takes the 3D certificate).
fn param_chain(host: &Host, charts: &[Chart], c: &IntCurve, sa: &Surface, sb: &Surface) -> Option<Vec<GBox>> {
    // the certificate's nodes: the chain's, with refined midpoints put in where one step fails
    let mut pts = c.pts.clone();
    let mut nodes: Vec<(usize, f64, f64)> = pts.iter().map(|&p| host.locate(p)).collect::<Option<_>>()?;
    let angular = !matches!(host.form, Form::Plane { .. });
    let mut segs = if c.closed { pts.len() } else { pts.len() - 1 };
    let mut added = 0;
    let mut out: Vec<GBox> = Vec::new();
    let mut ends: Vec<usize> = Vec::new();
    let mut firsts: Vec<usize> = Vec::new();
    let mut i = 0;
    let mut span = LONG;
    while i < segs {
        let n = pts.len();
        let mut got = None;
        let mut k = span.min(segs - i);
        while k >= 1 {
            let chart = nodes[i].0;
            let pts: Option<Vec<(f64, f64)>> = (0..=k)
                .map(|d| {
                    let (ch, t, s) = nodes[(i + d) % n];
                    host.move_point(ch, chart, t, s)
                })
                .collect();
            if let Some(pts) = pts
                && pts
                    .iter()
                    .all(|(t, s)| (!angular || t.abs() <= 4.0) && t.is_finite() && s.is_finite())
            {
                let tl = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
                let th = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
                let sl = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
                let sh = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                let (wt, ws) = (th - tl, sh - sl);
                // a graph over t wants the box narrow in t and generous in s; over s, the reverse
                let shapes = [
                    (0.125 * wt + 1e-12, 0.5 * (wt + ws) + 1e-9),
                    (0.5 * (wt + ws) + 1e-9, 0.125 * ws + 1e-12),
                ];
                let mut g = GBox {
                    chart,
                    t: I::new(tl, th),
                    s: I::new(sl, sh),
                    over_t: true,
                };
                let found = shapes.iter().any(|&(gt, gs)| {
                    let (t, s) = (I::new(tl - gt, th + gt), I::new(sl - gs, sh + gs));
                    // the chain runs along the arc once: its nodes strictly monotone in the graph's variable
                    let monotone = |over_t: bool| {
                        let v: Vec<f64> = pts.iter().map(|p| if over_t { p.0 } else { p.1 }).collect();
                        v.windows(2).all(|w| w[1] > w[0]) || v.windows(2).all(|w| w[1] < w[0])
                    };
                    match graph_way(&charts[chart].f, t, s) {
                        Some(over_t) if monotone(over_t) => {
                            g = GBox { chart, t, s, over_t };
                            true
                        }
                        _ => false,
                    }
                });
                if found {
                    got = Some((g, k));
                    break;
                }
            }
            k /= 2;
        }
        if let Some((_, k)) = got {
            firsts.push(i);
            let _ = k;
        }
        let Some((g, k)) = got else {
            // one step that no box holds: a refined midpoint node, then try again (as the 3D steps do)
            if added >= 64 {
                return None;
            }
            let j = (i + 1) % n;
            let m = crate::intcurve::refine(sa, sb, pts[i].lerp(pts[j], 0.5))?;
            let h = pts[i].distance(pts[j]);
            if m.distance(pts[i]) >= h || m.distance(pts[j]) >= h || h <= LEAST {
                return None;
            }
            pts.insert(i + 1, m);
            nodes.insert(i + 1, host.locate(m)?);
            segs += 1;
            added += 1;
            span = 1;
            continue;
        };
        out.push(g);
        i += k;
        ends.push(i);
        span = (2 * k).min(LONG);
    }
    // trimmed in each box's graph variable to its nodes and a margin: a box narrowed in the variable
    // it is a graph over is still one, so its claim stands, and the arcs of steps two apart no longer
    // reach each other
    let n = pts.len();
    let mb = out.len();
    // the range of nodes f0..=f1 in a box's graph variable
    let range = |g: &GBox, f0: usize, f1: usize| -> Option<(f64, f64)> {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for j in f0..=f1 {
            let (ch, t, s) = nodes[j % n];
            let (t, s) = host.move_point(ch, g.chart, t, s)?;
            let v = if g.over_t { t } else { s };
            lo = lo.min(v);
            hi = hi.max(v);
        }
        (lo <= hi).then_some((lo, hi))
    };
    let mut trimmed = out.clone();
    for (b, g) in trimmed.iter_mut().enumerate() {
        let (lo, hi) = range(&out[b], firsts[b], ends[b])?;
        // past each end by an eighth of the box, and by less than half its neighbour there (in this
        // box's variable), so that only neighbours' claims meet
        let next = |k: usize| -> f64 { range(&out[b], firsts[k], ends[k]).map_or(0.0, |(a, b)| b - a) };
        let mut margin = 0.125 * (hi - lo);
        if b + 1 < mb || c.closed {
            margin = margin.min(0.4 * next((b + 1) % mb));
        }
        if b > 0 || c.closed {
            margin = margin.min(0.4 * next((b + mb - 1) % mb));
        }
        let x = I::new(lo - margin - 1e-12, hi + margin + 1e-12);
        if g.over_t {
            g.t = meet(g.t, x);
        } else {
            g.s = meet(g.s, x);
        }
    }
    let out = trimmed;
    // joins: a graph box inside the meet of consecutive boxes, about their shared node: its arc is
    // a connected set of zeros in both, so their arcs are one
    let m = out.len();
    let n = pts.len();
    let joins = if c.closed { m } else { m.saturating_sub(1) };
    for (i, &end) in ends.iter().enumerate().take(joins) {
        let (a, b) = (out[i], out[(i + 1) % m]);
        // in a's chart, or b's where b's box is not a box in a's
        // (or b cut to the node's side of its flip's 0, which is all a join needs: the join box
        // lies in the cut, so in b)
        let node_b = nodes[end % n];
        let cut = |b: &GBox| -> Option<GBox> {
            let (t, s) = host.move_point(node_b.0, b.chart, node_b.1, node_b.2)?;
            let (fu, fv) = host.flips(b.chart, a.chart);
            let side = |x: I, at: f64| -> I {
                if at > 0.0 {
                    meet(x, I::new(0.5 * at, f64::INFINITY))
                } else {
                    meet(x, I::new(f64::NEG_INFINITY, 0.5 * at))
                }
            };
            Some(GBox {
                t: if fu && b.t.contains_zero() {
                    side(b.t, t)
                } else {
                    b.t
                },
                s: if fv && b.s.contains_zero() {
                    side(b.s, s)
                } else {
                    b.s
                },
                ..*b
            })
        };
        let (a, b) = match host.move_box(&b, a.chart, false) {
            Some(b) => (a, b),
            None => match host.move_box(&a, b.chart, false) {
                Some(a) => (a, b),
                None => match cut(&b).and_then(|b| host.move_box(&b, a.chart, false)) {
                    Some(b) => (a, b),
                    None => {
                        return None;
                    }
                },
            },
        };
        let q = GBox {
            chart: a.chart,
            t: meet(a.t, b.t),
            s: meet(a.s, b.s),
            over_t: a.over_t,
        };
        // (not a sliver: the arc, which crosses it, then has points inside both)
        let thin = |x: I| x.width() <= 1e-12 * (1.0 + mag(x));
        if q.t.is_empty() || q.s.is_empty() || thin(q.t) || thin(q.s) {
            return None;
        }
        let (ch, t, s) = nodes[end % n];
        let (t, s) = host.move_point(ch, a.chart, t, s)?;
        // boxes about the node shaped by the arc's way there (across F's gradient): narrow along the
        // variable the arc is a graph over, enough across for the arc's slope; inside the meet
        let f = &charts[a.chart].f;
        let (mut ft, mut fs) = (0.0, 0.0);
        for i in 0..=f.dt {
            for j in 0..=f.dv {
                let c = f.at(i, j).mid();
                if i > 0 {
                    ft += c * i as f64 * pow(t, i - 1) * pow(s, j);
                }
                if j > 0 {
                    fs += c * j as f64 * pow(t, i) * pow(s, j - 1);
                }
            }
        }
        let e0 = 0.25 * q.t.width().min(q.s.width());
        let mut tries = vec![(q.t, q.s)];
        for k in [1.0, 0.25, 1.0 / 16.0, 1.0 / 64.0] {
            let e = e0 * k;
            let (et, es) = if fs.abs() >= ft.abs() {
                (e, 2.0 * e * (ft / fs).abs() + e)
            } else {
                (2.0 * e * (fs / ft).abs() + e, e)
            };
            tries.push((
                meet(q.t, I::new(t - et, t + et)),
                meet(q.s, I::new(s - es, s + es)),
            ));
        }
        if !tries.iter().any(|&(bt, bs)| {
            !bt.is_empty()
                && !bs.is_empty()
                && !thin(bt)
                && !thin(bs)
                && graph_box(&charts[a.chart].f, bt, bs)
        }) {
            return None;
        }
        // no turning back at the node: a's first node clearly outside b, and b's last outside a (b's
        // arc, connected and holding both its ends, would otherwise run back along a's)
        let clear_out = |g: &GBox, node: (usize, f64, f64)| -> Option<bool> {
            // (a node at a flip's 0 is at infinity in the other chart: outside every box there)
            let Some((t, s)) = host.move_point(node.0, g.chart, node.1, node.2) else {
                return Some(true);
            };
            let e = |x: I| 1e-9 * (1.0 + mag(x));
            Some(t < g.t.lo - e(g.t) || t > g.t.hi + e(g.t) || s < g.s.lo - e(g.s) || s > g.s.hi + e(g.s))
        };
        let (a0, b1) = (out[i], out[(i + 1) % m]);
        let first_a = nodes[firsts[i] % n];
        let last_b = nodes[ends[(i + 1) % m] % n];
        if clear_out(&b1, first_a) != Some(true) || clear_out(&a0, last_b) != Some(true) {
            return None;
        }
    }
    Some(out)
}

/// Whether the polynomial has no zero in the box: Bernstein coefficients one-signed, halving to
/// `depth` levels.
fn zero_free(f: &Bp, t: I, s: I, depth: u32, budget: &mut usize) -> bool {
    let (n, m) = (f.dt, f.dv);
    let mut stack = vec![(t, s, bernstein(f, t, s), 0u32)];
    while let Some((t, s, c, d)) = stack.pop() {
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        if one_sign(&c[..(n + 1) * (m + 1)]) {
            continue;
        }
        if d >= depth {
            return false;
        }
        let along_t = t.width() >= s.width();
        let (l, r) = halve(&c, n, m, along_t);
        if along_t {
            let (a, b) = t.split();
            stack.push((a, s, l, d + 1));
            stack.push((b, s, r, d + 1));
        } else {
            let (a, b) = s.split();
            stack.push((t, a, l, d + 1));
            stack.push((t, b, r, d + 1));
        }
    }
    true
}

/// x^n by n multiplications (IEEE ops only, the same bits everywhere; no powi).
fn pow(x: f64, n: usize) -> f64 {
    let mut p = 1.0;
    for _ in 0..n {
        p *= x;
    }
    p
}

/// The certificate in the host's charts, when every chain's steps are graph boxes and no singular
/// site is near: steps, joins, no-twice and completeness all in (t, s), with no 3D step at all.
/// None where it does not apply (the 3D certificate then decides); never a wrong Some.
fn certify_param(a: &Surface, b: &Surface, region: Box3, curves: &[IntCurve]) -> Option<()> {
    let (fa, fb) = (Form::of(a)?, Form::of(b)?);
    let cert = Cert {
        pair: Pair { a: fa, b: fb },
        sa: a,
        sb: b,
        sites: Vec::new(),
        site: SITES[0],
        region: of_box(&region.grow(tol::LINEAR)),
    };
    let Ok(sites) = cert.find_sites(curves, &region) else {
        return None;
    };
    if !sites.is_empty() {
        return None;
    }
    let (host, other) = host_of(fa, fb)?;
    let reg = of_box(&region);
    let charts = host.charts(&other, &reg)?;
    let mut boxes: Vec<(usize, usize, usize, GBox)> = Vec::new(); // (chain, index, chain's count, box)
    for (ci, c) in curves.iter().enumerate() {
        if c.pts.len() < 2 {
            return None;
        }
        let gs = param_chain(&host, &charts, c, a, b)?;
        let k = gs.len();
        if c.closed && k < 3 {
            return None;
        }
        for (i, g) in gs.into_iter().enumerate() {
            boxes.push((ci, i, k, g));
        }
    }
    let mut budget = BUDGET;
    // no zero twice: boxes not neighbours in one chain meet in no zero
    for x in 0..boxes.len() {
        for y in x + 1..boxes.len() {
            let ((ci, i, k, g), (cj, j, _, h)) = (boxes[x], boxes[y]);
            if ci == cj && (i.abs_diff(j) <= 1 || (curves[ci].closed && i.abs_diff(j) == k - 1)) {
                continue;
            }
            // boxes whose points lie apart in space share no zero
            if meets(
                &host.chart_box(&charts[g.chart], g.t, g.s),
                &host.chart_box(&charts[h.chart], h.t, h.s),
            )
            .is_none()
            {
                continue;
            }
            // h in g's chart: whole, or in pieces clear of its flip's 0 (the points near that 0 lie
            // past |x| = 1 / delta in g's chart, beyond g)
            let pieces: Vec<GBox> = match host.move_box(&h, g.chart, true) {
                Some(h) => vec![h],
                None => {
                    let (fu, fv) = host.flips(h.chart, g.chart);
                    let split = |x: I, flipped: bool, bound: f64| -> Vec<I> {
                        if !flipped || !x.contains_zero() {
                            return vec![x];
                        }
                        let d = 0.5 / (bound + 1.0);
                        [I::new(x.lo, -d), I::new(d, x.hi)]
                            .into_iter()
                            .filter(|p| p.lo <= p.hi)
                            .collect()
                    };
                    let mut out = Vec::new();
                    for t in split(h.t, fu, mag(g.t)) {
                        for s in split(h.s, fv, mag(g.s)) {
                            match host.move_box(&GBox { t, s, ..h }, g.chart, true) {
                                Some(p) => out.push(p),
                                None => {
                                    return None;
                                }
                            }
                        }
                    }
                    out
                }
            };
            for h in pieces {
                let (t, s) = (meet(g.t, h.t), meet(g.s, h.s));
                if t.is_empty() || s.is_empty() {
                    continue;
                }
                if !zero_free(&charts[g.chart].f, t, s, 30, &mut budget) {
                    return None;
                }
            }
        }
    }
    // completeness: every zero of every chart in the region in some box
    let mut claims: Vec<Vec<GBox>> = vec![Vec::new(); charts.len()];
    for &(_, _, _, g) in &boxes {
        for (to, list) in claims.iter_mut().enumerate() {
            if let Some(h) = host.move_box(&g, to, false) {
                list.push(h);
            }
        }
    }
    for (k, ch) in charts.iter().enumerate() {
        let (n, m) = (ch.f.dt, ch.f.dv);
        if (n + 1) * (m + 1) > BERN {
            return None;
        }
        let first = bernstein(&ch.f, ch.t, ch.v);
        let mut stack = vec![(ch.t, ch.v, first)];
        while let Some((t, v, c)) = stack.pop() {
            if budget == 0 {
                return None;
            }
            budget -= 1;
            if one_sign(&c[..(n + 1) * (m + 1)]) {
                continue;
            }
            let inside = |g: &GBox| t.lo >= g.t.lo && t.hi <= g.t.hi && v.lo >= g.s.lo && v.hi <= g.s.hi;
            if claims[k].iter().any(inside) {
                continue;
            }
            let p = host.chart_box(ch, t, v);
            if meets(&p, &reg).is_none() || other.off_cheap(&p) {
                continue;
            }
            let (rt, rv) = host.chart_rates(&reg);
            let (lt, lv) = (rt * t.width(), rv * v.width());
            if lt.max(lv) <= LEAST {
                return None;
            }
            // where the box meets a claim, split the side that keeps it from fitting the claim
            let meet = claims[k]
                .iter()
                .find(|g| t.lo < g.t.hi && g.t.lo < t.hi && v.lo < g.s.hi && g.s.lo < v.hi);
            let along_t = match meet {
                Some(g) if g.t.width() > 0.0 && g.s.width() > 0.0 => {
                    t.width() / g.t.width() >= v.width() / g.s.width()
                }
                _ => lt >= lv,
            };
            let (cl, cr) = halve(&c, n, m, along_t);
            if along_t {
                let (a, b) = t.split();
                stack.push((a, v, cl));
                stack.push((b, v, cr));
            } else {
                let (a, b) = v.split();
                stack.push((t, a, cl));
                stack.push((t, b, cr));
            }
        }
    }
    Some(())
}

/// A leaf of the charts' walk that holds one arc. F is monotone along each of its sides (one
/// partial one-signed over the box and the sides across it one-signed, or both partials
/// one-signed), so each side holds at most one zero, exactly where its two corners differ in sign,
/// and the zeros inside are one arc joining the two such sides.
#[derive(Clone, Copy, Debug)]
struct Leaf {
    chart: usize,
    t: I,
    s: I,
    /// F's sign at (t.lo, s.lo), (t.hi, s.lo), (t.lo, s.hi), (t.hi, s.hi).
    corner: [f64; 4],
    /// The signs of dF/dt and dF/ds over the box (0: not one-signed), and lower bounds on their
    /// sizes, from the Bernstein coefficients.
    dt: f64,
    ds: f64,
    slope_t: f64,
    slope_s: f64,
    /// The sides the arc ends on (0: t.lo, 1: t.hi, 2: s.lo, 3: s.hi).
    sides: [usize; 2],
    /// The arc's points are found along t (s solved), or along s.
    by_t: bool,
}

/// The two corners of each side, low end of its stretch first.
const SIDE_CORNERS: [(usize, usize); 4] = [(0, 2), (1, 3), (0, 1), (2, 3)];

/// The arc's two ends: each the leaf and the end (0 or 1) it joins, or None where the arc leaves the
/// walked region.
type Ends = [Option<(usize, usize)>; 2];

/// A leaf from a box's Bernstein coefficients (bidegree n, m), if they make one: Some(None) where
/// the box holds no zero after all (monotone with no sign change on its boundary).
fn leaf_of(chart: usize, t: I, s: I, c: &[I], n: usize, m: usize) -> Option<Option<Leaf>> {
    let at = |i: usize, j: usize| c[i * (m + 1) + j];
    let sign = |x: I| {
        if x.lo > 0.0 {
            1.0
        } else if x.hi < 0.0 {
            -1.0
        } else {
            0.0
        }
    };
    // one-signed differences along s (dt = false) or t: the sign and the least size
    let rate = |along_t: bool| -> (f64, f64) {
        let (len, across) = if along_t { (m, n) } else { (n, m) };
        if across == 0 {
            return (0.0, 0.0);
        }
        let (mut pos, mut neg, mut least) = (true, true, f64::INFINITY);
        for i in 0..=len {
            for j in 0..across {
                let d = if along_t {
                    at(j + 1, i) - at(j, i)
                } else {
                    at(i, j + 1) - at(i, j)
                };
                pos &= d.lo > 0.0;
                neg &= d.hi < 0.0;
                if !(pos || neg) {
                    return (0.0, 0.0);
                }
                least = least.min(if pos { d.lo } else { -d.hi });
            }
        }
        (if pos { 1.0 } else { -1.0 }, least * across as f64)
    };
    let side_sign = |side: usize| -> f64 {
        let vals: Vec<I> = match side {
            0 => (0..=m).map(|j| at(0, j)).collect(),
            1 => (0..=m).map(|j| at(n, j)).collect(),
            2 => (0..=n).map(|i| at(i, 0)).collect(),
            _ => (0..=n).map(|i| at(i, m)).collect(),
        };
        if vals.iter().all(|x| x.lo > 0.0) {
            1.0
        } else if vals.iter().all(|x| x.hi < 0.0) {
            -1.0
        } else {
            0.0
        }
    };
    let corner = [sign(at(0, 0)), sign(at(n, 0)), sign(at(0, m)), sign(at(n, m))];
    let (dt, lt) = rate(true);
    let (ds, ls) = rate(false);
    let over_t = ds != 0.0 && side_sign(2) != 0.0 && side_sign(3) != 0.0;
    let over_s = dt != 0.0 && side_sign(0) != 0.0 && side_sign(1) != 0.0;
    let both = dt != 0.0 && ds != 0.0 && corner.iter().all(|&x| x != 0.0);
    if !(over_t || over_s || both) {
        return None;
    }
    let sides: Vec<usize> = (0..4)
        .filter(|&k| corner[SIDE_CORNERS[k].0] != corner[SIDE_CORNERS[k].1])
        .collect();
    match sides.len() {
        0 => return Some(None),
        2 => {}
        _ => return None,
    }
    // (an arc between the t-sides is found along t; between the s-sides along s; otherwise
    // along either, both partials being one-signed)
    let by_t = if over_t {
        true
    } else if over_s {
        false
    } else {
        sides.contains(&0) || sides.contains(&1)
    };
    let shrink = 1.0 - 1e-12;
    Some(Some(Leaf {
        chart,
        t,
        s,
        corner,
        dt,
        ds,
        slope_t: lt / t.width() * shrink,
        slope_s: ls / s.width() * shrink,
        sides: [sides[0], sides[1]],
        by_t,
    }))
}

/// F at (t, s) in floats, with a bound on its distance from the exact value of the polynomial
/// with interval coefficients (rounding of the sum and the powers, the coefficients' radii).
fn value_bound(f: &Bp, t: f64, s: f64) -> (f64, f64) {
    let (mut v, mut abs, mut rad) = (0.0, 0.0, 0.0);
    let mut tp = 1.0;
    for i in 0..=f.dt {
        let mut sp = 1.0;
        for j in 0..=f.dv {
            let c = f.c[i * 9 + j];
            let x = c.mid() * tp * sp;
            v += x;
            abs += x.abs();
            rad += (c.hi - c.lo) * (tp * sp).abs();
            sp *= s;
        }
        tp *= t;
    }
    // at most 100 roundings along any term's way into the sum (degrees <= 8, 81 terms): 2^-53 each,
    // doubled for slack; the radii's own sum rounded up likewise
    (
        v,
        (abs * 2e-14 + rad * (1.0 + 1e-12)) * (1.0 + 1e-12) + f64::MIN_POSITIVE,
    )
}

/// The root of F(x, ·) (`solve_s`) or F(·, x) in `range`, F rising there if `up` > 0, by Newton
/// kept in its bracket.
fn root_along(f: &Bp, solve_s: bool, x: f64, range: I, up: f64, guess: Option<f64>) -> f64 {
    // the univariate restriction's coefficients in the other variable
    let mut g = [0.0f64; 9];
    let (len, across) = if solve_s { (f.dt, f.dv) } else { (f.dv, f.dt) };
    let mut xp = 1.0;
    for i in 0..=len {
        for (j, gj) in g.iter_mut().enumerate().take(across + 1) {
            let c = if solve_s { f.c[i * 9 + j] } else { f.c[j * 9 + i] };
            *gj += c.mid() * xp;
        }
        xp *= x;
    }
    let eval = |y: f64| -> (f64, f64) {
        let (mut v, mut d) = (0.0, 0.0);
        for j in (0..=across).rev() {
            d = d * y + v;
            v = v * y + g[j];
        }
        (v, d)
    };
    let (mut lo, mut hi) = (range.lo, range.hi);
    let mut y = guess.filter(|g| *g > lo && *g < hi).unwrap_or(0.5 * (lo + hi));
    for _ in 0..64 {
        let (v, d) = eval(y);
        if v == 0.0 {
            return y;
        }
        // below the root F has the sign -up
        if (v > 0.0) != (up > 0.0) {
            lo = y;
        } else {
            hi = y;
        }
        let newton = y - v / d;
        let next = if d != 0.0 && newton > lo && newton < hi {
            newton
        } else {
            0.5 * (lo + hi)
        };
        if (next - y).abs() <= 4.0 * f64::EPSILON * (1.0 + y.abs())
            || hi - lo <= f64::EPSILON * (1.0 + lo.abs())
        {
            return next;
        }
        y = next;
    }
    y
}

/// Whether no curve of the pair meets `grown`, by the march's own octree (`trace_marched`: the same
/// root cube, cells, leaf size, touch test and `Surface::implicit`), each cell cleared soundly: a
/// surface's float implicit at its centre, less its rounding bound (`Form::rounding`), past the
/// cell's radius (the implicits are `Form::distance`, 1-Lipschitz to a factor within 1e-12 of 1). False at the first cell that reaches the leaf size uncleared: the
/// charts then decide.
#[allow(clippy::too_many_arguments)]
fn octree_empty(
    a: &Surface,
    b: &Surface,
    fa: Form,
    fb: Form,
    region: &Box3,
    grown: &Box3,
    leaf: f64,
    most: usize,
) -> bool {
    let half = 0.5
        * region
            .x
            .width()
            .max(region.y.width())
            .max(region.z.width())
            .max(1e-6);
    let touches = |c: Point3, r: f64| {
        let d = |i: forge_math::Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
        let (dx, dy, dz) = (d(grown.x, c.x), d(grown.y, c.y), d(grown.z, c.z));
        dx * dx + dy * dy + dz * dz <= r * r
    };
    // each surface's rounding bound over the whole root cube (every cell centre lies in it): at
    // most 32 u (|c - o|_1 + k), |c - o|_1 at most the cube's farthest corner's
    let bound = |f: Form| {
        let (o, k) = f.rounding();
        let c = region.center();
        let far = |x: f64, oc: f64| (x - oc).abs() + half;
        16.0 * f64::EPSILON * (far(c.x, o.x) + far(c.y, o.y) + far(c.z, o.z) + k) * (1.0 + 1e-12) + 1e-300
    };
    let (ea, eb) = (bound(fa), bound(fb));
    let clear = |s: &Surface, e: f64, c: Point3, r: f64| s.implicit(c).abs() - e > r * (1.0 + 1e-12);
    let mut stack = vec![(region.center(), half * 3f64.sqrt())];
    let mut looked = 0;
    while let Some((c, r)) = stack.pop() {
        looked += 1;
        if looked > most {
            return false;
        }
        if !touches(c, r) || clear(a, ea, c, r) || clear(b, eb, c, r) {
            continue;
        }
        if 2.0 * r <= leaf {
            return false;
        }
        let h = r * 0.5;
        let o = h / 3f64.sqrt();
        for dx in [-o, o] {
            for dy in [-o, o] {
                for dz in [-o, o] {
                    stack.push((c + Vec3::new(dx, dy, dz), h));
                }
            }
        }
    }
    true
}

/// Whether the pair has no curve in the region, by the march's octree (`octree_empty`), tried before
/// anything else: an empty trace then costs what the march's search does.
fn quickly_empty(a: &Surface, b: &Surface, region: Box3) -> bool {
    let (Some(fa), Some(fb)) = (Form::of(a), Form::of(b)) else {
        return false;
    };
    let size = region.diagonal().max(1e-6);
    let step = 0.5 * (size / 64.0).max(10.0 * tol::LINEAR);
    let grown = region.grow(step);
    // no curve meets `grown` when the march's octree clears every cell (empty traces then cost
    // what the march's search does)
    // (a torus makes the charts' polynomials of high degree: there the octree is the cheaper way
    // to an empty answer, whatever its depth; else the charts are, past a few cells)
    let torus = matches!(fa, Form::Torus { .. }) || matches!(fb, Form::Torus { .. });
    let axis = |f: &Form| match *f {
        Form::Cylinder { x, y, .. } | Form::Cone { x, y, .. } => Some(x.cross(y)),
        Form::Torus { z, .. } | Form::Plane { z, .. } => Some(z),
        Form::Sphere { .. } => None,
    };
    // (two round surfaces on parallel axes with a wide gap between them: the octree clears it in
    // a few layers; a narrow one it clears only near its leaves, and the charts are cheaper)
    let round = |f: &Form| match *f {
        Form::Cylinder { o, r, .. } | Form::Cone { o, r, .. } => Some((o, r)),
        _ => None,
    };
    let wide = match (axis(&fa), axis(&fb), round(&fa), round(&fb)) {
        (Some(p), Some(q), Some((oa, ra)), Some((ob, rb))) if p.cross(q).length() < 1e-9 => {
            let d = (ob - oa) - p * p.dot(ob - oa);
            let d = d.length();
            let gap = (d - (ra + rb)).abs().min((d - (ra - rb).abs()).abs());
            gap >= size / 16.0
        }
        _ => false,
    };
    let most = if torus || wide { usize::MAX } else { 64 };
    octree_empty(a, b, fa, fb, &region, &grown, 2.0 * step, most)
}

/// The trace in the host's charts, with no march: one Bernstein subdivision of each chart into
/// leaves that are zero-free, off the region, or one arc (a graph box); the arcs joined across
/// their leaves' edges into chains; points along each chain at the march's spacing, each pulled
/// onto both surfaces and proven within LINEAR / 2 of its arc. The charts partition the host (a
/// sphere's two charts of v in [-pi/2, pi/2]), and the leaves the charts, so every zero in the
/// region is in exactly one arc: complete and none twice by construction. None where it does not
/// apply (a singular point near, a leaf that is none of the three at the least size, a chain that
/// does not join): the march and its certificate then decide.
fn param_trace(a: &Surface, b: &Surface, region: Box3) -> Option<Vec<IntCurve>> {
    let (fa, fb) = (Form::of(a)?, Form::of(b)?);
    let size = region.diagonal().max(1e-6);
    let step = 0.5 * (size / 64.0).max(10.0 * tol::LINEAR);
    let grown = region.grow(step);
    // the walked region: past `grown`, so an arc's end in it lies outside `grown`
    let walked = region.grow(3.0 * step);
    for f in [fa, fb] {
        if f.singular().is_some_and(|x| walked.grow(SITES[0]).contains(x)) {
            return None;
        }
    }
    let reg = of_box(&walked);
    let (host, other) = host_of(fa, fb)?;
    let all = host.charts_in(&other, &reg, true)?;
    let sphere = matches!(host.form, Form::Sphere { .. });
    // (a sphere's charts with turn_v = -1 cover it again)
    let used: Vec<usize> = (0..all.len())
        .filter(|&k| all[k].live && (!sphere || all[k].turn_v > 0.0))
        .collect();
    let (rt, rv) = host.chart_rates(&reg);
    let least = (step / 16.0).max(LEAST);
    let mut budget = 1usize << 14;
    let mut leaves: Vec<Leaf> = Vec::new();
    for &k in &used {
        let ch = &all[k];
        let (n, m) = (ch.f.dt, ch.f.dv);
        if (n + 1) * (m + 1) > BERN {
            return None;
        }
        let mut stack = Vec::with_capacity(64);
        stack.push((ch.t, ch.v, bernstein(&ch.f, ch.t, ch.v)));
        while let Some((t, v, c)) = stack.pop() {
            budget = budget.checked_sub(1)?;
            let c_ = &c[..(n + 1) * (m + 1)];
            if one_sign(c_) {
                continue;
            }
            let p = host.chart_box(ch, t, v);
            if meets(&p, &reg).is_none() || other.off_cheap(&p) {
                continue;
            }
            match leaf_of(k, t, v, c_, n, m) {
                Some(Some(l)) => {
                    leaves.push(l);
                    continue;
                }
                Some(None) => continue,
                None => {}
            }
            let (lt, lv) = (rt * t.width(), rv * v.width());
            if lt.max(lv) <= least {
                return None;
            }
            let along_t = lt >= lv;
            let (cl, cr) = halve(&c, n, m, along_t);
            if along_t {
                let (x, y) = t.split();
                stack.push((x, v, cl));
                stack.push((y, v, cr));
            } else {
                let (x, y) = v.split();
                stack.push((t, x, cl));
                stack.push((t, y, cr));
            }
        }
    }
    let ends = join_leaves(&host, &all, &leaves)?;
    let mut curves = Vec::new();
    let mut seen = vec![false; leaves.len()];
    for start in 0..leaves.len() {
        if seen[start] {
            continue;
        }
        // back to an open end (or round to the start), then forward collecting leaves with the end
        // each is entered by
        let (mut at, mut out) = (start, 0usize);
        let mut closed = false;
        let mut guard = 0;
        while let Some((next, by)) = ends[at][out] {
            guard += 1;
            if guard > leaves.len() {
                return None;
            }
            at = next;
            out = 1 - by;
            if at == start {
                closed = true;
                break;
            }
        }
        // `at` is entered by its end `out` (open there, or the loop's start)
        let first = at;
        let mut way: Vec<(usize, usize)> = Vec::new();
        let mut entered = out;
        loop {
            if seen[at] {
                return None;
            }
            seen[at] = true;
            way.push((at, entered));
            match ends[at][1 - entered] {
                Some((next, by)) => {
                    if next == first && closed {
                        break;
                    }
                    at = next;
                    entered = by;
                }
                None => break,
            }
        }
        let mut pts = chain_points(&host, &all, &leaves, &way, closed, step, rt, rv, a, b)?;
        if pts.len() < 3 {
            return None;
        }
        // along na x nb, as the march runs (its consumers read the curve's way from it)
        let d = crate::intcurve::direction(a, b, pts[0])?;
        let first_step = d.dot(pts[1] - pts[0]);
        if first_step == 0.0 || first_step.is_nan() {
            return None;
        }
        if first_step < 0.0 {
            pts.reverse();
            if closed {
                // (the same first point)
                pts.rotate_right(1);
            }
        }
        if closed {
            if pts.iter().all(|p| grown.contains(*p)) {
                curves.push(crate::intcurve::finish(a, b, pts, true, step));
                continue;
            }
            // a loop through the region's edge: open runs, starting where it is outside
            let k = pts.iter().position(|p| !grown.contains(*p)).expect("one outside");
            let mut turned = pts[k..].to_vec();
            turned.extend_from_slice(&pts[..=k]);
            runs(a, b, &turned, &grown, step, &mut curves);
        } else {
            if grown.contains(pts[0]) || grown.contains(*pts.last().expect("points")) {
                return None;
            }
            runs(a, b, &pts, &grown, step, &mut curves);
        }
    }
    Some(curves)
}

/// The open chains of `pts` (both ends outside `grown`): each stretch inside, with the point
/// before it and the one after, as the march leaves them.
fn runs(a: &Surface, b: &Surface, pts: &[Point3], grown: &Box3, step: f64, out: &mut Vec<IntCurve>) {
    let mut i = 0;
    while i < pts.len() {
        if !grown.contains(pts[i]) {
            i += 1;
            continue;
        }
        let from = i - 1;
        while grown.contains(pts[i]) {
            i += 1;
        }
        out.push(crate::intcurve::finish(a, b, pts[from..=i].to_vec(), false, step));
    }
}

/// Each leaf's arc ends joined to its neighbours': an end on a side (or a chart seam) shared with
/// another leaf with an end on its side there is that leaf's when F changes sign over the two
/// sides' common stretch (each end of it a corner of one of them, of known sign: a side holds at
/// most one zero). An end with no such neighbour lies in a box off the walked region (a zero-free
/// box cannot hold it). None if an end has two, if the joins are not mutual, or if an arc ends at
/// a sphere's pole.
fn join_leaves(host: &Host, charts: &[Chart], leaves: &[Leaf]) -> Option<Vec<Ends>> {
    let sphere = matches!(host.form, Form::Sphere { .. });
    let angular_t = !matches!(host.form, Form::Plane { .. });
    let angular_s = matches!(host.form, Form::Torus { .. });
    // the chart across a seam of chart k: u's half turn (t = +-1) or v's (s = +-1)
    let across = |k: usize, in_t: bool| -> Option<usize> {
        let (tu, tv) = (charts[k].turn, charts[k].turn_v);
        let (tu, tv) = if in_t { (-tu, tv) } else { (tu, -tv) };
        charts.iter().position(|c| c.turn == tu && c.turn_v == tv)
    };
    let line = |l: &Leaf, side: usize| match side {
        0 => l.t.lo,
        1 => l.t.hi,
        2 => l.s.lo,
        _ => l.s.hi,
    };
    let stretch = |l: &Leaf, side: usize| if side < 2 { l.s } else { l.t };
    // every arc end, by its side and the side's line, for the neighbour search
    let mut index: Vec<(usize, u64, usize, usize)> = Vec::new();
    for (i, l) in leaves.iter().enumerate() {
        for (e, &side) in l.sides.iter().enumerate() {
            index.push((side, line(l, side).to_bits(), i, e));
        }
    }
    index.sort_unstable();
    let mut ends: Vec<Ends> = vec![[None, None]; leaves.len()];
    for (i, l) in leaves.iter().enumerate() {
        for (e, &side) in l.sides.iter().enumerate() {
            let x = line(l, side);
            if sphere && side >= 2 && x.abs() == 1.0 {
                return None;
            }
            let seam = if side < 2 { angular_t } else { angular_s };
            let (chart, there) = if seam && x.abs() == 1.0 {
                (across(l.chart, side < 2)?, -x)
            } else {
                (l.chart, x)
            };
            let facing = side ^ 1;
            let key = (facing, there.to_bits());
            let from = index.partition_point(|q| (q.0, q.1) < key);
            let span = stretch(l, side);
            let (sl, sh) = (l.corner[SIDE_CORNERS[side].0], l.corner[SIDE_CORNERS[side].1]);
            let mut hit = None;
            for &(_, _, j, f) in index[from..].iter().take_while(|q| (q.0, q.1) == key) {
                let o = &leaves[j];
                if o.chart != chart || j == i {
                    continue;
                }
                let ospan = stretch(o, facing);
                let (lo, hi) = (span.lo.max(ospan.lo), span.hi.min(ospan.hi));
                if lo.is_nan() || hi.is_nan() || lo >= hi {
                    continue;
                }
                let (ol, oh) = (o.corner[SIDE_CORNERS[facing].0], o.corner[SIDE_CORNERS[facing].1]);
                let sign_at = |x: f64| -> f64 {
                    if x == span.lo {
                        sl
                    } else if x == ospan.lo {
                        ol
                    } else if x == span.hi {
                        sh
                    } else {
                        oh
                    }
                };
                if sign_at(lo) != sign_at(hi) {
                    if hit.is_some() {
                        return None;
                    }
                    hit = Some((j, f));
                }
            }
            ends[i][e] = hit;
        }
    }
    for (i, e) in ends.iter().enumerate() {
        for (k, x) in e.iter().enumerate() {
            if let Some((j, f)) = *x
                && ends[j][f] != Some((i, k))
            {
                return None;
            }
        }
    }
    Some(ends)
}

/// Where a leaf's arc meets its side, in the leaf's own variable (t if `by_t`).
fn end_value(f: &Bp, l: &Leaf, side: usize) -> f64 {
    match (l.by_t, side) {
        (true, 0) => l.t.lo,
        (true, 1) => l.t.hi,
        (true, _) => root_along(f, false, if side == 2 { l.s.lo } else { l.s.hi }, l.t, l.dt, None),
        (false, 2) => l.s.lo,
        (false, 3) => l.s.hi,
        (false, _) => root_along(f, true, if side == 0 { l.t.lo } else { l.t.hi }, l.s, l.ds, None),
    }
}

/// The chain's points: along each leaf's arc in its order, at the march's spacing (`step`), each
/// on both surfaces and proven within LINEAR / 2 of the arc. `way`: (leaf, the end it is entered by).
#[allow(clippy::too_many_arguments)]
fn chain_points(
    host: &Host,
    charts: &[Chart],
    leaves: &[Leaf],
    way: &[(usize, usize)],
    closed: bool,
    step: f64,
    rt: f64,
    rv: f64,
    a: &Surface,
    b: &Surface,
) -> Option<Vec<Point3>> {
    // the arc's point at x in the leaf's own variable, from a guess at the other: the point, t, s
    let point = |l: &Leaf, x: f64, guess: Option<f64>| -> (Point3, f64, f64) {
        let f = &charts[l.chart].f;
        let (t, s) = if l.by_t {
            (x, root_along(f, true, x, l.s, l.ds, guess))
        } else {
            (root_along(f, false, x, l.t, l.dt, guess), x)
        };
        (host.chart_point(&charts[l.chart], t, s), t, s)
    };
    let other_of = |l: &Leaf, t: f64, s: f64| if l.by_t { s } else { t };
    // each leaf's way along its own variable, from the end it is entered by to the other
    let spans: Vec<(f64, f64)> = way
        .iter()
        .map(|&(li, by)| {
            let l = &leaves[li];
            let f = &charts[l.chart].f;
            (end_value(f, l, l.sides[by]), end_value(f, l, l.sides[1 - by]))
        })
        .collect();
    // dense: each leaf's span cut until chords are at most `step`
    let mut dense: Vec<(usize, f64, f64, Point3)> = Vec::new(); // (place in `way`, value, other, point)
    for (w, &(li, _)) in way.iter().enumerate() {
        let l = &leaves[li];
        let (x0, x1) = spans[w];
        let (p0, t0, s0) = point(l, x0, None);
        let (p1, t1, s1) = point(l, x1, None);
        let (y0, y1) = (other_of(l, t0, s0), other_of(l, t1, s1));
        let mut stack = vec![(x0, y0, p0, x1, y1, p1, 0u32)];
        let mut out = Vec::new();
        while let Some((xa, ya, pa, xb, yb, pb, d)) = stack.pop() {
            if pa.distance(pb) <= step || d >= 24 {
                out.push((xa, ya, pa));
                continue;
            }
            let xm = 0.5 * (xa + xb);
            let (pm, tm, sm) = point(l, xm, Some(0.5 * (ya + yb)));
            let ym = other_of(l, tm, sm);
            stack.push((xm, ym, pm, xb, yb, pb, d + 1));
            stack.push((xa, ya, pa, xm, ym, pm, d + 1));
        }
        // the leaf's last point is the next leaf's first
        if w + 1 == way.len() && !closed {
            out.push((x1, y1, p1));
        }
        dense.extend(out.into_iter().map(|(x, y, p)| (w, x, y, p)));
    }
    if closed {
        let first = dense[0];
        dense.push(first);
    }
    // arc length along the dense chain, then points at even spacing at most `step`
    let mut at = vec![0.0f64];
    for k in 1..dense.len() {
        let l = at[k - 1] + dense[k - 1].3.distance(dense[k].3);
        at.push(l);
    }
    let total = *at.last()?;
    // (the chords run a little short of the arc: a margin keeps the true spacing within `step`)
    let count = ((total * 1.001 / step).ceil() as usize).max(1);
    let gap = total / count as f64;
    let mut pts = Vec::with_capacity(count + 1);
    let mut k = 0;
    let last = if closed { count } else { count + 1 };
    for q in 0..last {
        let want = gap * q as f64;
        while k + 2 < dense.len() && at[k + 1] < want {
            k += 1;
        }
        let (w0, x0, y0, _) = dense[k];
        let (w1, x1, y1, _) = dense[k + 1];
        let frac = if at[k + 1] > at[k] {
            ((want - at[k]) / (at[k + 1] - at[k])).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // within one leaf the value is interpolated; towards the next leaf, to this one's exit
        let end = if w0 == w1 { x1 } else { spans[w0].1 };
        let l = &leaves[way[w0].0];
        let guess = if w0 == w1 { y0 + (y1 - y0) * frac } else { y0 };
        let (p, t, s) = point(l, x0 + (end - x0) * frac, Some(guess));
        // how far p is from the arc: the root's distance (|F| / slope) times the rate, with the
        // chart map's own enclosure
        let f = &charts[l.chart].f;
        let (v, e) = value_bound(f, t, s);
        let (slope, rate) = if l.by_t { (l.slope_s, rv) } else { (l.slope_t, rt) };
        let off = (v.abs() + e) / slope * (1.0 + 1e-12);
        // the float point's distance from the exact image of (t, s): inside the enclosure
        let pb = host.chart_box(&charts[l.chart], I::point(t), I::point(s));
        let pc = [p.x, p.y, p.z];
        let map = (0..3)
            .map(|i| (pc[i] - pb[i].lo).max(pb[i].hi - pc[i]).max(0.0))
            .fold(0.0, f64::max)
            * 1.7320508075688772;
        let q = crate::intcurve::refine(a, b, p)?;
        if q.distance(p) + off * rate + map > 0.5 * tol::LINEAR {
            return None;
        }
        pts.push(q);
    }
    Some(pts)
}

/// The cheaper host of the pair to walk, and the other form.
fn host_of(fa: Form, fb: Form) -> Option<(Host, Form)> {
    let rank = |f: &Form| match f {
        Form::Plane { .. } => 0,
        Form::Cylinder { .. } => 1,
        Form::Cone { .. } => 2,
        Form::Sphere { .. } => 3,
        Form::Torus { .. } => 4,
    };
    let (h, o) = if rank(&fa) <= rank(&fb) {
        (fa, fb)
    } else {
        (fb, fa)
    };
    Host::of(h)
        .map(|host| (host, o))
        .or_else(|| Host::of(o).map(|host| (host, h)))
}

/// Certifies the chains `curves` of the pair `a`, `b` as every branch inside `region`, each once.
pub fn certify(a: &Surface, b: &Surface, region: Box3, curves: &[IntCurve]) -> Result<(), Refusal> {
    if certify_param(a, b, region, curves).is_some() {
        count(4);
        return Ok(());
    }
    count(5);
    let mut last = None;
    for (k, &site) in SITES.iter().enumerate() {
        let scale = region.diagonal();
        if k > 0 && site > 0.05 * scale {
            break;
        }
        match certify_with(a, b, region, curves, site) {
            Ok(n) => return Ok(n),
            // a larger site box helps only where the sites are what ran the walk out
            Err((r, sited)) => {
                last = Some(r);
                if !sited || !matches!(r.why, Why::Budget | Why::Uncovered | Why::Step) {
                    break;
                }
            }
        }
    }
    Err(last.expect("one try"))
}

fn certify_with(
    a: &Surface,
    b: &Surface,
    region: Box3,
    curves: &[IntCurve],
    site: f64,
) -> Result<(), (Refusal, bool)> {
    let out = |why| (Refusal { region, why }, false);
    let (Some(fa), Some(fb)) = (Form::of(a), Form::of(b)) else {
        return Err(out(Why::OutOfScope));
    };
    let mut cert = Cert {
        pair: Pair { a: fa, b: fb },
        sa: a,
        sb: b,
        sites: Vec::new(),
        site,
        region: of_box(&region.grow(tol::LINEAR)),
    };
    cert.sites = cert.find_sites(curves, &region).map_err(|r| (r, false))?;
    let sited = !cert.sites.is_empty();

    let sites = cert.sites.clone();
    let in_site = |b: &V| sites.iter().any(|(q, _)| inside(b, q));
    // every segment of every chain, with its chain and place in it
    let mut segs: Vec<Seg> = Vec::new();
    let mut whose: Vec<(usize, usize, usize)> = Vec::new(); // (chain, index, chain's length)
    let mut closed = Vec::new();
    for (ci, c) in curves.iter().enumerate() {
        if c.pts.len() < 2 || c.tans.len() != c.pts.len() {
            return Err(out(Why::Step));
        }
        let ss = cert.chain(c).map_err(|r| (r, sited))?;
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
    // completeness: every zero in the region lies in some step's claim or a site, shown over the
    // domain of the surface whose walk is the cheaper
    let host = [fa, fb]
        .into_iter()
        .enumerate()
        .filter_map(|(k, f)| Host::of(f).map(|h| (k, h)))
        .min_by_key(|(_, h)| match h.form {
            Form::Plane { .. } => 0,
            Form::Cylinder { .. } => 1,
            Form::Cone { .. } => 2,
            Form::Sphere { .. } => 3,
            Form::Torus { .. } => 4,
        });
    let Some((k, host)) = host else {
        return Err(out(Why::OutOfScope));
    };
    let other = if k == 0 { fb } else { fa };
    walk_bernstein(&host, &other, &of_box(&region), &segs, &sites, &mut budget)
        .map_err(|e| (fail(e, Why::Uncovered), sited))?;
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
            if i == j || neighbours(i, j) || segs[i].site || segs[j].site {
                continue;
            }
            let Some(both) = meets(&segs[i].x, &segs[j].x) else {
                continue;
            };
            let (si, sj) = (segs[i], segs[j]);
            walk(
                both,
                &[],
                |b| cert.pair.excludes(b) || in_site(b) || si.disowns(b) || sj.disowns(b),
                |_, _| false,
                &mut budget,
            )
            .map_err(|e| (fail(e, Why::Twice), sited))?;
        }
    }
    Ok(())
}

/// Whether the certificate covers the pair (both surfaces have enclosures).
pub fn in_scope(a: &Surface, b: &Surface) -> bool {
    Form::of(a).is_some() && Form::of(b).is_some()
}

/// The traced pairs this thread certified last, with their answers: a regen asks the same pair in
/// the same region from more than one place (the sequential and n-ary paths both). An answer is a
/// function of its key alone (the two surfaces in order, and the region, compared exactly; the
/// curves keep their surfaces in the order asked).
type Traced = Vec<(Surface, Surface, Box3, Option<Vec<IntCurve>>)>;
const TRACED_MOST: usize = 32;

thread_local! {
    static TRACED: std::cell::RefCell<Traced> = const { std::cell::RefCell::new(Vec::new()) };
    static LAST: std::cell::Cell<Option<Refusal>> = const { std::cell::Cell::new(None) };
    static COUNTS: std::cell::Cell<[usize; 7]> = const { std::cell::Cell::new([0; 7]) };
}

/// The last refusal on this thread, taken.
pub fn last_refusal() -> Option<Refusal> {
    LAST.with(|l| l.take())
}

/// On this thread so far: [certified first time, certified after a finer trace, refused, out of
/// scope, certificates proven in the charts alone, certificates that took the 3D steps, traces made
/// in the charts with no march].
pub fn counts() -> [usize; 7] {
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
    let known = TRACED.with(|t| {
        t.borrow()
            .iter()
            .find(|(p, q, r, _)| p == a && q == b && *r == region)
            .map(|e| e.3.clone())
    });
    if let Some(answer) = known {
        return answer;
    }
    // (an empty answer, by the march's octree, needs no noting)
    let answer = if quickly_empty(a, b, region) {
        count(6);
        Some(Vec::new())
    } else {
        crate::intcurve::not_noting(|| traced_fresh(a, b, region))
    };
    TRACED.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() >= TRACED_MOST {
            t.remove(0);
        }
        t.push((a.clone(), b.clone(), region, answer.clone()));
    });
    answer
}

/// `traced`, computed.
fn traced_fresh(a: &Surface, b: &Surface, region: Box3) -> Option<Vec<IntCurve>> {
    if let Some(cs) = param_trace(a, b, region) {
        count(6);
        return Some(cs);
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

    fn report(a: &Surface, b: &Surface, r: Box3) -> Result<usize, Refusal> {
        let cs = crate::intcurve::trace_marched(a, b, r, 1.0).ok_or(Refusal {
            region: r,
            why: Why::Step,
        })?;
        certify(a, b, r, &cs).map(|()| cs.len())
    }

    #[test]
    fn cylinders_touching_inside_certify_through_the_touch() {
        // the scoreboard's cylinder_cylinder/perpendicular_tangent_inside: the tool's wall reaches
        // the base's at (0, 10, -10), where two loops cross
        let a = cyl(Point3::ORIGIN, Vec3::Z, 10.0);
        let b = cyl(Point3::new(0.0, 5.0, -10.0), Vec3::X, 5.0);
        let r = Box3::from_points([Point3::new(-12.0, -12.0, -20.0), Point3::new(12.0, 12.0, 0.0)]);
        assert!(report(&a, &b, r).is_ok(), "{:?}", report(&a, &b, r));
    }

    #[test]
    fn a_cone_with_its_apex_on_a_cylinder_certifies() {
        // cylinder_cone/apex_on_top_rim: the apex at (10, 0, 0) on the base's wall
        let a = cyl(Point3::new(0.0, 0.0, -20.0), Vec3::Z, 10.0);
        let b = Surface::Cone(
            crate::surface::Cone::new(
                Frame::from_z(Point3::new(10.0, 0.0, 10.0), UnitVec3::Z),
                5.0,
                forge_math::dmath::atan(0.5),
            )
            .expect("cone"),
        );
        let r = Box3::from_points([Point3::new(-12.0, -12.0, -20.0), Point3::new(12.0, 12.0, 0.0)]);
        assert!(report(&a, &b, r).is_ok(), "{:?}", report(&a, &b, r));
    }

    #[test]
    fn hessians_match_their_gradients() {
        let t = Surface::Torus(Torus {
            frame: Frame::from_z(
                Point3::new(0.3, -0.2, 0.1),
                (Vec3::new(0.2, 0.3, 1.0)).normalize().expect("z"),
            ),
            major: 4.0,
            minor: 1.5,
        });
        let c = Surface::Cone(
            crate::surface::Cone::new(Frame::from_z(Point3::ORIGIN, UnitVec3::X), 2.0, 0.4).expect("cone"),
        );
        for s in [t, c, cyl(Point3::ORIGIN, Vec3::new(1.0, 2.0, 3.0), 2.0)] {
            let f = Form::of(&s).expect("in scope");
            let p = Point3::new(1.1, 2.3, -0.7);
            let h = f.hessian(&pt(p));
            #[allow(clippy::needless_range_loop)]
            for k in 0..3 {
                let e = 1e-6;
                let mut d = Vec3::ZERO;
                match k {
                    0 => d.x = e,
                    1 => d.y = e,
                    _ => d.z = e,
                }
                let (_, g1) = f.eval(&pt(p + d));
                let (_, g0) = f.eval(&pt(p - d));
                for i in 0..3 {
                    let fd = (g1[i].mid() - g0[i].mid()) / (2.0 * e);
                    assert!(
                        (fd - h[i][k].mid()).abs() <= 1e-5 * (1.0 + fd.abs()),
                        "{s:?} {i} {k}: {fd} {:?}",
                        h[i][k]
                    );
                }
            }
        }
    }

    /// The sphere-cone pairs of forge-lang's ball_over_tapered_pocket (a ball of radius 13 on a
    /// block's top over a 1 deg tapered pocket with corners of radius 4): meet.rs's closed form and
    /// the march, each certified, must agree.
    #[test]
    fn a_ball_over_a_tapered_pocket_certifies() {
        let ball = Surface::Sphere(Sphere {
            frame: Frame::from_axes(Point3::new(0.0, 0.0, 18.0), Vec3::X, -Vec3::Z, Vec3::Y).expect("frame"),
            radius: 13.0,
        });
        let r = |x0: f64, x1: f64, y0: f64, y1: f64| {
            Box3::from_points([
                Point3::new(x0, y0, 4.99997999984771),
                Point3::new(x1, y1, 18.00002000015229),
            ])
        };
        let (a, b, c, d) = (
            10.99997999984771,
            13.00002000015229,
            2.99997999984771,
            8.99997999984771,
        );
        let cases = [
            (-11.0, -7.0, r(-b, -a, -7.00002000015229, -c)),
            (-11.0, -9.0, r(-b, -a, -b, -d)),
            (11.0, -7.0, r(a, b, -7.00002000015229, -c)),
            (11.0, -9.0, r(a, b, -b, -d)),
        ];
        for (x, y, region) in cases {
            let cone = Surface::Cone(
                crate::surface::Cone::new(
                    Frame::from_z(Point3::new(x, y, 18.0), UnitVec3::Z),
                    4.0,
                    (1.0f64).to_radians(),
                )
                .expect("cone"),
            );
            let marched = crate::intcurve::trace_marched(&cone, &ball, region, 1.0).expect("marched");
            let m = certify(&cone, &ball, region, &marched);
            let closed = crate::meet::intersect(&cone, &ball, region);
            let chains: Option<Vec<IntCurve>> = match &closed {
                Some(crate::intersect::SurfaceSurface::Curves(cs)) => cs
                    .iter()
                    .map(|(c, _)| match c {
                        crate::Curve::Intersection(c) => Some((**c).clone()),
                        _ => None,
                    })
                    .collect(),
                Some(crate::intersect::SurfaceSurface::Empty) => Some(Vec::new()),
                _ => None,
            };
            let k = chains.as_ref().map(|cs| certify(&cone, &ball, region, cs));
            assert_eq!(chains.as_ref().map(Vec::len), Some(marched.len()), "({x}, {y})");
            assert_eq!(k, Some(Ok(())), "the closed form at ({x}, {y})");
            assert!(m.is_ok(), "the march at ({x}, {y}): {m:?}");
        }
    }

    /// halve's one-widening against exact de Casteljau: integers near 2^60, whose float sums round,
    /// halved in i128 scaled by 2^8 (every average of them is then an integer).
    #[test]
    fn halving_encloses_the_exact_halves() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for case in 0..200 {
            let (n, m) = (1 + case % 8, 1 + (case / 8) % 8);
            let mut c = [I::point(0.0); BERN];
            let mut q = [0i128; BERN];
            for k in 0..(n + 1) * (m + 1) {
                let x = ((1u64 << 59) + (next() >> 5)) as f64;
                c[k] = I::point(x);
                q[k] = (x as i128) << 8;
            }
            for along_t in [true, false] {
                let (l, r) = halve(&c, n, m, along_t);
                let (len, lines) = if along_t { (n, m + 1) } else { (m, n + 1) };
                let idx = |k: usize, line: usize| {
                    if along_t {
                        k * (m + 1) + line
                    } else {
                        line * (m + 1) + k
                    }
                };
                for line in 0..lines {
                    let mut b: Vec<i128> = (0..=len).map(|k| q[idx(k, line)]).collect();
                    let inside =
                        |x: I, v: i128| ((x.lo * 256.0) as i128) <= v && v <= ((x.hi * 256.0) as i128);
                    assert!(inside(l[idx(0, line)], b[0]) && inside(r[idx(len, line)], b[len]));
                    for step in 1..=len {
                        for k in 0..=len - step {
                            b[k] = (b[k] + b[k + 1]) / 2;
                        }
                        assert!(inside(l[idx(step, line)], b[0]), "left {case} {step}");
                        assert!(
                            inside(r[idx(len - step, line)], b[len - step]),
                            "right {case} {step}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bernstein_charts_match_their_forms() {
        let torus = Surface::Torus(Torus {
            frame: Frame::from_z(
                Point3::new(0.3, -0.2, 0.1),
                (Vec3::new(0.2, 0.3, 1.0)).normalize().expect("z"),
            ),
            major: 4.0,
            minor: 1.5,
        });
        let cone = Surface::Cone(
            crate::surface::Cone::new(Frame::from_z(Point3::new(0.5, 0.1, -1.0), UnitVec3::X), 2.0, 0.4)
                .expect("cone"),
        );
        let plane = Surface::Plane(crate::surface::Plane {
            frame: Frame::from_z(
                Point3::new(0.2, 0.0, 0.3),
                (Vec3::new(0.3, -0.4, 1.0)).normalize().expect("z"),
            ),
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::from_z(Point3::new(1.0, 2.0, 3.0), UnitVec3::Z),
            radius: 2.5,
        });
        let region = pt(Point3::ORIGIN).map(|i| i.grow(6.0));
        for host in [
            cyl(Point3::new(0.1, 0.2, 0.0), Vec3::new(1.0, 2.0, 3.0), 2.0),
            cone.clone(),
            plane,
            torus.clone(),
            sph.clone(),
        ] {
            let h = Host::of(Form::of(&host).expect("host")).expect("chartable");
            for other in [&torus, &cone, &sph] {
                let f = Form::of(other).expect("in scope");
                for ch in h.charts(&f, &region).expect("charts") {
                    let bern = bernstein(&ch.f, ch.t, ch.v);
                    let (lo, hi) = bern
                        .iter()
                        .fold((f64::MAX, f64::MIN), |(a, b), c| (a.min(c.lo), b.max(c.hi)));
                    for (a, b) in [(0.1, 0.2), (0.5, 0.9), (0.95, 0.05), (0.3, 0.6)] {
                        let (t, v) = (ch.t.lo + a * ch.t.width(), ch.v.lo + b * ch.v.width());
                        let p = h.chart_box(&ch, I::point(t), I::point(v));
                        let x = Point3::new(p[0].mid(), p[1].mid(), p[2].mid());
                        let (want, _) = f.eval(&pt(x));
                        let w = match h.form {
                            Form::Plane { .. } => 1.0,
                            Form::Sphere { .. } | Form::Torus { .. } => (1.0 + t * t) * (1.0 + v * v),
                            _ => 1.0 + t * t,
                        };
                        let deg = match f {
                            Form::Torus { .. } => 4,
                            _ => 2,
                        };
                        // the chart's polynomial at (t, v), by its power form
                        let mut got = 0.0;
                        for i in 0..=ch.f.dt {
                            for j in 0..=ch.f.dv {
                                got += ch.f.at(i, j).mid() * t.powi(i as i32) * v.powi(j as i32);
                            }
                        }
                        let want = want.mid() * w.powi(deg);
                        assert!(
                            (got - want).abs() <= 1e-7 * (1.0 + want.abs()),
                            "{host:?} / {other:?}: {got} {want}"
                        );
                        assert!(lo <= got && got <= hi, "outside the Bernstein hull");
                    }
                }
            }
        }
    }
}
