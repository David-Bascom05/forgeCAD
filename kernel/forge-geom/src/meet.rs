//! Curves where two surfaces meet, found in closed form.
//!
//! The curve runs along u of a host surface (a cylinder, cone or torus). At
//! each u the host's other parameter s is a root of F(u, s), the other
//! surface's polynomial on the host: degree 2 in the axial v for a quadric on
//! a cylinder or cone, degree 4 for a torus on one, and a trigonometric
//! polynomial of degree 1 or 2 in the tube angle for a plane, sphere or torus
//! on a torus.
//!
//! Roots of F(u, .) meet and part where its discriminant in s vanishes (the
//! curve turns back in u there, or crosses itself). The curve is cut at those
//! u into arcs, each one root over its u range, and the arcs are joined into
//! loops. Each arc is walked at the tracer's spacing, its root followed by
//! Newton steps and checked against all the roots, so every branch is found
//! without seeding and the result is the curve the tracer would give
//! (an `IntCurve`, exact by its own Newton step).

use crate::intersect::{Contact, SurfaceSurface};
use crate::meet_gen::{quartic_disc, torus_line, trig2_quartic};
use crate::surface::{Surface, SurfaceGeom};
use crate::{Curve, Torus};
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec3, dmath, poly, tol};

/// The other surface as a polynomial in world coordinates, zero on the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Alg {
    /// q.A.q + 2 b.q + c with q = p - o.
    Quadric {
        o: Point3,
        a: [[f64; 3]; 3],
        b: Vec3,
        c: f64,
    },
    /// (|q|^2 + R^2 - r^2)^2 - 4 R^2 (qx^2 + qy^2) in the frame's coordinates.
    Torus { frame: Frame, big: f64, r: f64 },
}

pub(crate) fn mul(a: &[[f64; 3]; 3], v: Vec3) -> Vec3 {
    Vec3::new(
        a[0][0] * v.x + a[0][1] * v.y + a[0][2] * v.z,
        a[1][0] * v.x + a[1][1] * v.y + a[1][2] * v.z,
        a[2][0] * v.x + a[2][1] * v.y + a[2][2] * v.z,
    )
}

/// I - w z z^T: the matrix of |q|^2 - w (q.z)^2.
fn less_axis(z: Vec3, w: f64) -> [[f64; 3]; 3] {
    let z = [z.x, z.y, z.z];
    let mut a = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] = if i == j { 1.0 } else { 0.0 } - w * z[i] * z[j];
        }
    }
    a
}

impl Alg {
    pub(crate) fn of(s: &Surface) -> Option<Alg> {
        Some(match s {
            Surface::Plane(p) => Alg::Quadric {
                o: p.frame.origin,
                a: [[0.0; 3]; 3],
                b: p.frame.z().get() * 0.5,
                c: 0.0,
            },
            Surface::Cylinder(c) => Alg::Quadric {
                o: c.frame.origin,
                a: less_axis(c.frame.z().get(), 1.0),
                b: Vec3::ZERO,
                c: -c.radius * c.radius,
            },
            Surface::Cone(c) => {
                // |q|^2 - (q.z)^2 - (r + k q.z)^2, both nappes
                let (k, z) = (c.tan_a(), c.frame.z().get());
                Alg::Quadric {
                    o: c.frame.origin,
                    a: less_axis(z, 1.0 + k * k),
                    b: z * (-c.radius * k),
                    c: -c.radius * c.radius,
                }
            }
            Surface::Sphere(s) => Alg::Quadric {
                o: s.frame.origin,
                a: less_axis(Vec3::Z, 0.0),
                b: Vec3::ZERO,
                c: -s.radius * s.radius,
            },
            Surface::Torus(t) => Alg::Torus {
                frame: t.frame,
                big: t.major,
                r: t.minor,
            },
            Surface::Procedural(_) => return None,
        })
    }

    /// Value and gradient at p.
    pub(crate) fn grad(&self, p: Point3) -> (f64, Vec3) {
        match *self {
            Alg::Quadric { o, a, b, c } => {
                let q = p - o;
                let aq = mul(&a, q);
                (q.dot(aq) + 2.0 * b.dot(q) + c, (aq + b) * 2.0)
            }
            Alg::Torus { frame, big, r } => {
                let q = p - frame.origin;
                let z = frame.z().get();
                let (g, h) = (q.dot(q), q.dot(z));
                let k = big * big - r * r;
                let v = (g + k) * (g + k) - 4.0 * big * big * (g - h * h);
                (v, q * (4.0 * (g + k)) - (q - z * h) * (8.0 * big * big))
            }
        }
    }

    /// x.H.y with H the Hessian at p.
    fn hess(&self, p: Point3, x: Vec3, y: Vec3) -> f64 {
        match *self {
            Alg::Quadric { a, .. } => 2.0 * x.dot(mul(&a, y)),
            Alg::Torus { frame, big, r } => {
                let q = p - frame.origin;
                let z = frame.z().get();
                let k = big * big - r * r;
                let bb = 8.0 * big * big;
                8.0 * q.dot(x) * q.dot(y) + (4.0 * (q.dot(q) + k) - bb) * x.dot(y) + bb * x.dot(z) * y.dot(z)
            }
        }
    }
}

/// How F(u, .) depends on s.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Form {
    /// c0 + c1 v + c2 v^2.
    Poly2,
    /// Ascending quartic in v.
    Poly4,
    /// A0 + A1 cos s + B1 sin s.
    Trig1,
    /// A0 + A1 cos s + B1 sin s + A2 cos 2s + B2 sin 2s.
    Trig2,
}

#[derive(Clone, Debug, PartialEq)]
struct Eqn {
    host: Surface,
    alg: Alg,
    form: Form,
    /// The host's frame, radius at v = 0 and slope dr/dv, for a cylinder or cone.
    line: Option<(Frame, f64, f64)>,
    /// A quadric on a cylinder or cone: F(u, .)'s coefficients are trigonometric in u, from
    /// [C, g.x, g.y, g.z, xAx, yAy, xAy, xAz, yAz, zAz] (in the host's frame; q0 the host's
    /// origin from the quadric's, g = A q0 + b, C = q0.A.q0 + 2 b.q0 + c).
    table: Option<[f64; 10]>,
}

/// Products of a0 + a1 cos + a2 sin and b0 + b1 cos + b2 sin, as [A0, A1, B1, A2, B2].
fn trig_mul(a: [f64; 3], b: [f64; 3]) -> [f64; 5] {
    [
        a[0] * b[0] + 0.5 * (a[1] * b[1] + a[2] * b[2]),
        a[0] * b[1] + a[1] * b[0],
        a[0] * b[2] + a[2] * b[0],
        0.5 * (a[1] * b[1] - a[2] * b[2]),
        0.5 * (a[1] * b[2] + a[2] * b[1]),
    ]
}

fn wrap_pi(x: f64) -> f64 {
    x - dmath::TAU * (x / dmath::TAU).round()
}

const MAX_ROOTS: usize = 4;
/// More events than a closed form keeps: a pair this degenerate (near tangent over a
/// stretch) is left to the tracer, and finding them stops there.
const MAX_EVENTS: usize = 256;
type Roots = [(f64, f64); MAX_ROOTS];

impl Eqn {
    fn new(host: Surface, alg: Alg, form: Form) -> Eqn {
        let line = match &host {
            Surface::Cylinder(c) => Some((c.frame, c.radius, 0.0)),
            Surface::Cone(c) => Some((c.frame, c.radius, c.tan_a())),
            _ => None,
        };
        let table = match (line, alg) {
            (Some((f, _, _)), Alg::Quadric { o, a, b, c }) => {
                let q0 = f.origin - o;
                let aq = mul(&a, q0);
                let g = aq + b;
                let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
                let at = |p: Vec3, q: Vec3| p.dot(mul(&a, q));
                Some([
                    q0.dot(aq) + 2.0 * b.dot(q0) + c,
                    g.dot(x),
                    g.dot(y),
                    g.dot(z),
                    at(x, x),
                    at(y, y),
                    at(x, y),
                    at(x, z),
                    at(y, z),
                    at(z, z),
                ])
            }
            _ => None,
        };
        Eqn {
            host,
            alg,
            form,
            line,
            table,
        }
    }

    fn periodic(&self) -> bool {
        matches!(self.form, Form::Trig1 | Form::Trig2)
    }

    /// F(u, .) as coefficients: ascending powers of v, or [A0, A1, B1, A2, B2].
    fn coeffs(&self, u: f64) -> [f64; 5] {
        let (su, cu) = dmath::sin_cos(u);
        match (&self.host, self.line, self.table) {
            (Surface::Torus(t), _, _) => self.tube_coeffs(t, su, cu),
            (_, Some((_, rho, k)), Some(m)) => {
                let (c2, s2) = (cu * cu - su * su, 2.0 * su * cu);
                let gr = m[1] * cu + m[2] * su;
                let rar = 0.5 * (m[4] + m[5]) + 0.5 * (m[4] - m[5]) * c2 + m[6] * s2;
                let raz = m[7] * cu + m[8] * su;
                [
                    m[0] + 2.0 * rho * gr + rho * rho * rar,
                    2.0 * (m[3] + k * gr + rho * k * rar + rho * raz),
                    m[9] + 2.0 * k * raz + k * k * rar,
                    0.0,
                    0.0,
                ]
            }
            (_, Some((f, rho, k)), None) => {
                let Alg::Torus { frame, big, r } = self.alg else {
                    unreachable!("a quadric on a line has its table")
                };
                let rad = f.x() * cu + f.y() * su;
                let p = frame.point_to_local(f.origin + rad * rho);
                let v = frame.vec_to_local(rad * k + f.z().get());
                torus_line([p.x, p.y, p.z], [v.x, v.y, v.z], big, r)
            }
            _ => unreachable!("hosts are cylinders, cones and tori"),
        }
    }

    fn tube_coeffs(&self, t: &Torus, su: f64, cu: f64) -> [f64; 5] {
        let f = &t.frame;
        let rad = f.x() * cu + f.y() * su;
        let z = f.z().get();
        let centre = f.origin + rad * t.major;
        let m = t.minor;
        match self.alg {
            Alg::Quadric { o, a, b, c } => {
                let q = centre - o;
                let aq = mul(&a, q);
                let g = (aq + b) * 2.0;
                let (rar, zaz, raz) = (rad.dot(mul(&a, rad)), z.dot(mul(&a, z)), rad.dot(mul(&a, z)));
                [
                    q.dot(aq) + 2.0 * b.dot(q) + c + 0.5 * m * m * (rar + zaz),
                    m * g.dot(rad),
                    m * g.dot(z),
                    0.5 * m * m * (rar - zaz),
                    m * m * raz,
                ]
            }
            Alg::Torus { frame, big, r } => {
                let w = frame.point_to_local(centre).to_vec();
                let (er, ez) = (frame.vec_to_local(rad), frame.vec_to_local(z));
                let g = [w.dot(w) + m * m, 2.0 * m * w.dot(er), 2.0 * m * w.dot(ez)];
                let h = [w.z, m * er.z, m * ez.z];
                let k = big * big - r * r;
                let gk = [g[0] + k, g[1], g[2]];
                let (sq, hh) = (trig_mul(gk, gk), trig_mul(h, h));
                let bb = 4.0 * big * big;
                [
                    sq[0] - bb * g[0] + bb * hh[0],
                    sq[1] - bb * g[1] + bb * hh[1],
                    sq[2] - bb * g[2] + bb * hh[2],
                    sq[3] + bb * hh[3],
                    sq[4] + bb * hh[4],
                ]
            }
        }
    }

    /// F, dF/ds and d2F/ds2 at s from the coefficients.
    fn f_s(&self, c: &[f64; 5], s: f64) -> (f64, f64, f64) {
        match self.form {
            Form::Poly2 | Form::Poly4 => {
                let (mut f, mut d1, mut d2) = (0.0, 0.0, 0.0);
                for k in (0..5).rev() {
                    d2 = d2 * s + 2.0 * d1;
                    d1 = d1 * s + f;
                    f = f * s + c[k];
                }
                (f, d1, d2)
            }
            Form::Trig1 | Form::Trig2 => {
                let (sn, cs) = dmath::sin_cos(s);
                let (s2, c2) = (2.0 * sn * cs, cs * cs - sn * sn);
                (
                    c[0] + c[1] * cs + c[2] * sn + c[3] * c2 + c[4] * s2,
                    -c[1] * sn + c[2] * cs - 2.0 * c[3] * s2 + 2.0 * c[4] * c2,
                    -c[1] * cs - c[2] * sn - 4.0 * c[3] * c2 - 4.0 * c[4] * s2,
                )
            }
        }
    }

    /// Real roots of F(u, .) with dF/ds at each; a double root comes out twice,
    /// once for each sign.
    fn roots(&self, c: &[f64; 5], out: &mut Roots) -> usize {
        let mut n = 0;
        match self.form {
            Form::Poly2 => {
                let (a, b, cc) = (c[2], c[1], c[0]);
                let disc = b * b - 4.0 * a * cc;
                let slack = 8.0 * f64::EPSILON * (b * b).max((4.0 * a * cc).abs());
                if a == 0.0 {
                    if b != 0.0 {
                        out[0] = (-cc / b, b);
                        n = 1;
                    }
                } else if disc >= -slack {
                    let sq = disc.max(0.0).sqrt();
                    let q = -0.5 * (b + if b >= 0.0 { sq } else { -sq });
                    let (x1, x2) = if q == 0.0 { (0.0, 0.0) } else { (q / a, cc / q) };
                    for x in [x1, x2] {
                        out[n] = (x, 2.0 * a * x + b);
                        n += 1;
                    }
                    if sq == 0.0 {
                        out[0].1 = 0.0;
                        out[1].1 = 0.0;
                    }
                }
            }
            Form::Poly4 => {
                let bound = 1.0 + (0..4).fold(0.0f64, |m, k| m.max((c[k] / c[4]).abs()));
                let mut xs = [0.0; 4];
                let k = real_roots(c, -bound, bound, &mut xs);
                for &x in &xs[..k] {
                    out[n] = (x, self.f_s(c, x).1);
                    n += 1;
                }
            }
            Form::Trig1 => {
                let rho = (c[1] * c[1] + c[2] * c[2]).sqrt();
                if rho > 0.0 {
                    let x = -c[0] / rho;
                    if x.abs() <= 1.0 + 1e-14 {
                        let phi = dmath::atan2(c[2], c[1]);
                        let w = dmath::acos_clamped(x);
                        for s in [phi + w, phi - w] {
                            out[n] = (s, self.f_s(c, s).1);
                            n += 1;
                        }
                        if x.abs() >= 1.0 {
                            out[0].1 = 0.0;
                            out[1].1 = 0.0;
                        }
                    }
                }
            }
            Form::Trig2 => {
                // tan of the half angle about 0 and about pi, each kept to |w| <= 1
                for (base, sg) in [(0.0, 1.0), (dmath::PI, -1.0)] {
                    let q = trig2_quartic(c[0], sg * c[1], sg * c[2], c[3], c[4]);
                    let mut xs = [0.0; 4];
                    let k = real_roots(&q, -1.01, 1.01, &mut xs);
                    for &w in &xs[..k] {
                        let s = base + 2.0 * dmath::atan(w);
                        if out[..n].iter().any(|r| wrap_pi(r.0 - s).abs() <= 1e-9) || n == MAX_ROOTS {
                            continue;
                        }
                        out[n] = (s, self.f_s(c, s).1);
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// The discriminant of a quadratic or a degree-1 trigonometric F(u, .), as it is.
    fn disc_raw(&self, u: f64) -> f64 {
        let c = self.coeffs(u);
        match self.form {
            Form::Poly2 => c[1] * c[1] - 4.0 * c[2] * c[0],
            _ => c[1] * c[1] + c[2] * c[2] - c[0] * c[0],
        }
    }

    /// The discriminant of F(u, .) in s, zero where two roots meet. A polynomial in v is
    /// taken with v scaled to put its roots within about 1, and the coefficients scaled to
    /// at most 1: its size then says how near two roots are next to their spread.
    fn disc(&self, u: f64) -> f64 {
        let c = self.scaled(self.coeffs(u));
        match self.form {
            Form::Poly2 => c[1] * c[1] - 4.0 * c[2] * c[0],
            Form::Poly4 | Form::Trig2 => quartic_disc(&c),
            Form::Trig1 => c[1] * c[1] + c[2] * c[2] - c[0] * c[0],
        }
    }

    /// F(u, .) as the polynomial whose discriminant `disc` takes, scaled as it says.
    fn scaled(&self, c: [f64; 5]) -> [f64; 5] {
        let c = match self.form {
            Form::Trig2 => trig2_quartic(c[0], c[1], c[2], c[3], c[4]),
            Form::Poly2 | Form::Poly4 => {
                let top = if self.form == Form::Poly2 { 2 } else { 4 };
                // the root bound max |c_k / c_top|^(1 / (top - k)), rounded up to a power of
                // two (the scaling is then exact)
                let e = (0..top)
                    .map(|k| {
                        let r = (c[k] / c[top]).abs();
                        let e = ((r.to_bits() >> 52) & 0x7ff) as i64 - 1022;
                        if r > 0.0 && r.is_finite() {
                            e.div_euclid((top - k) as i64) + 1
                        } else {
                            0
                        }
                    })
                    .fold(-60, i64::max)
                    .clamp(-60, 60);
                let lam = f64::from_bits(((1023 + e) as u64) << 52);
                let mut p = 1.0;
                let mut out = [0.0; 5];
                for k in 0..=top {
                    out[k] = c[k] * p;
                    p *= lam;
                }
                out
            }
            Form::Trig1 => c,
        };
        let k = 1.0 / c.iter().fold(f64::MIN_POSITIVE, |m, x| m.max(x.abs()));
        c.map(|x| x * k)
    }

    /// Where the discriminant vanishes over a turn of u: crossings of zero to full
    /// precision, and places it touches zero (flagged true: a multiple root). Samples are
    /// halved wherever the discriminant bends more than its distance from zero, so a pair
    /// of zeros close together shows as a dip below zero.
    fn events(&self, n: usize) -> Vec<(f64, bool)> {
        let f = |u: f64| self.disc(u);
        // a sign change, by regula falsi with the Illinois halving (the turn is polished after)
        let cross = |mut a: f64, mut b: f64, mut fa: f64| {
            let mut fb = f(b);
            let mut side = 0;
            for _ in 0..60 {
                let m = if fa != fb {
                    b - fb * (b - a) / (fb - fa)
                } else {
                    0.5 * (a + b)
                };
                let m = if m > a.min(b) && m < a.max(b) {
                    m
                } else {
                    0.5 * (a + b)
                };
                let fm = f(m);
                if fm == 0.0 || (b - a).abs() <= 1e-13 * (1.0 + m.abs()) {
                    return m;
                }
                if (fm < 0.0) == (fb < 0.0) {
                    (b, fb) = (m, fm);
                    if side == -1 {
                        fa *= 0.5;
                    }
                    side = -1;
                } else {
                    (a, fa) = (m, fm);
                    if side == 1 {
                        fb *= 0.5;
                    }
                    side = 1;
                }
                if (b - a).abs() <= 1e-13 * (1.0 + m.abs()) {
                    return m;
                }
            }
            0.5 * (a + b)
        };
        let mut out: Vec<(f64, bool)> = Vec::new();
        let at = |i: usize| dmath::TAU * i as f64 / n as f64;
        let mut stack: Vec<(f64, f64, f64, f64, u32)> = Vec::new();
        let f0 = f(0.0);
        let mut fb = f0;
        for i in (0..n).rev() {
            let fa = if i == 0 { f0 } else { f(at(i)) };
            stack.push((at(i), fa, at(i + 1), fb, 0));
            fb = fa;
        }
        while let Some((a, fa, b, fb, depth)) = stack.pop() {
            if out.len() > MAX_EVENTS {
                break;
            }
            if fa == 0.0 {
                out.push((a, false));
                continue;
            }
            let m = 0.5 * (a + b);
            let fm = f(m);
            let near = fa.abs().min(fb.abs()).min(fm.abs());
            let bent = (fm - 0.5 * (fa + fb)).abs() > 0.25 * near && depth < 12;
            if fb != 0.0 && (fa < 0.0) != (fb < 0.0) && !bent {
                out.push((cross(a, b, fa), false));
                continue;
            }
            if (fm != 0.0 && (fm < 0.0) != (fa < 0.0)) || bent {
                stack.push((m, fm, b, fb, depth + 1));
                stack.push((a, fa, m, fm, depth + 1));
            } else if near <= 1e-12 {
                let u = [(a, fa), (m, fm), (b, fb)]
                    .iter()
                    .fold(
                        (a, f64::INFINITY),
                        |q, &(u, v)| {
                            if v.abs() < q.1 { (u, v.abs()) } else { q }
                        },
                    );
                // a real double root there, not a near pair of complex ones; the least sample
                // only brackets it (two pairs of roots meeting at once, as on a tube square
                // to a torus's axis, keep the discriminant above zero on both sides), so it is
                // polished onto the turn first
                let u = self
                    .polish_turn(u.0, b - a)
                    .filter(|x| (a..=b).contains(x))
                    .unwrap_or(u.0);
                if self.double_root(u) && out.last().is_none_or(|l| !l.1 || u - l.0 > 1e-6) {
                    out.push((u, true));
                }
            }
        }
        out
    }

    /// The root at u with dF/ds of sign `sign` nearest `guess` (any root if none has
    /// that sign). With fewer than `count` roots a double root was lost to rounding, and
    /// the critical point nearest `guess` stands in for it if it is nearer.
    fn pick(&self, u: f64, guess: f64, sign: f64, count: usize) -> (f64, usize) {
        self.pick_in(&self.coeffs(u), guess, sign, count)
    }

    fn pick_in(&self, c: &[f64; 5], guess: f64, sign: f64, count: usize) -> (f64, usize) {
        let c = *c;
        let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
        let n = self.roots(&c, &mut r);
        let off = |s: f64| {
            if self.periodic() {
                wrap_pi(s - guess)
            } else {
                s - guess
            }
        };
        let mut best: Option<(bool, f64)> = None;
        for &(s, fs) in &r[..n] {
            let d = off(s);
            let ok = fs * sign >= 0.0;
            best = match best {
                Some((bok, bd)) if (bok && !ok) || (bok == ok && bd.abs() <= d.abs()) => Some((bok, bd)),
                _ => Some((ok, d)),
            };
        }
        let mut d = best.map_or(f64::INFINITY, |b| b.1);
        if n < count || best.is_none() {
            let k = self.critical(&c, &mut r);
            for &(s, _) in &r[..k] {
                if off(s).abs() < d.abs() {
                    d = off(s);
                }
            }
        }
        (if d.is_finite() { guess + d } else { guess }, n)
    }

    /// Critical points of F(u, .): where dF/ds = 0.
    fn critical(&self, c: &[f64; 5], out: &mut Roots) -> usize {
        match self.form {
            Form::Poly2 | Form::Poly4 => {
                let d = [c[1], 2.0 * c[2], 3.0 * c[3], 4.0 * c[4]];
                let top = d.iter().rposition(|x| *x != 0.0).unwrap_or(0);
                let bound = 1.0 + (0..top).fold(0.0f64, |m, k| m.max((d[k] / d[top]).abs()));
                let mut xs = [0.0; 4];
                let k = real_roots(&d[..=top], -bound, bound, &mut xs);
                for i in 0..k {
                    out[i] = (xs[i], 0.0);
                }
                k
            }
            Form::Trig1 | Form::Trig2 => self.roots(&[0.0, c[2], -c[1], 2.0 * c[4], -2.0 * c[3]], out),
        }
    }

    /// A turning point near u, to full precision: F = dF/ds = 0, by Newton in (u, s) from
    /// each critical point of F(u, .); the nearest that converges within `reach`.
    fn polish_turn(&self, u: f64, reach: f64) -> Option<f64> {
        let c = self.coeffs(u);
        let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
        let k = self.critical(&c, &mut r);
        let mut best = (reach, None);
        for &(s0, _) in &r[..k] {
            let (mut x, mut s) = (u, s0);
            let mut converged = false;
            for _ in 0..30 {
                let (d, [fu, fs, _, fus, fss]) = self.partials(x, s);
                let f = self.alg.grad(d.p).0;
                let det = fu * fss - fs * fus;
                if det == 0.0 || !det.is_finite() {
                    break;
                }
                let dx = (-f * fss + fs * fs) / det;
                let ds = (-fu * fs + fus * f) / det;
                x += dx;
                s += ds;
                if dx.abs() <= 1e-15 * (1.0 + x.abs()) && ds.abs() <= 1e-12 * (1.0 + s.abs()) {
                    converged = true;
                    break;
                }
            }
            if converged && (x - u).abs() < best.0 {
                best = ((x - u).abs(), Some(x));
            }
        }
        best.1
    }

    /// Whether F(u, .) has a real double root: a critical point where F vanishes within
    /// rounding.
    fn double_root(&self, u: f64) -> bool {
        let c = self.coeffs(u);
        let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
        let k = self.critical(&c, &mut r);
        r[..k].iter().any(|&(s, _)| {
            let (f, _, _) = self.f_s(&c, s);
            let size = match self.form {
                Form::Poly2 | Form::Poly4 => c.iter().rev().fold(0.0, |m, x| m * s.abs() + x.abs()),
                Form::Trig1 | Form::Trig2 => c.iter().map(|x| x.abs()).sum(),
            };
            f.abs() <= 1e-9 * size
        })
    }

    /// The root of F(u, .) Newton converges to from x, if it does without dF/ds leaving
    /// the sign `sign`.
    fn newton(&self, c: &[f64; 5], mut x: f64, sign: f64) -> Option<f64> {
        for _ in 0..8 {
            let (f, d1, _) = self.f_s(c, x);
            if d1 == 0.0 || d1 * sign < 0.0 {
                return None;
            }
            let dx = f / d1;
            x -= dx;
            if dx.abs() <= 1e-13 * (1.0 + x.abs()) {
                return Some(x);
            }
        }
        None
    }

    /// How many roots F(u, .) has. A quartic's count follows from its discriminant and
    /// two more invariants, unless it is near a double root or a root at infinity (a
    /// tube angle of pi), where the roots are found to count them.
    fn count(&self, u: f64) -> usize {
        let c = self.coeffs(u);
        if matches!(self.form, Form::Poly4 | Form::Trig2) {
            let q = self.scaled(c);
            let d = quartic_disc(&q);
            if q[4].abs() > 1e-6 && d.abs() > 1e-9 {
                if d < 0.0 {
                    return 2;
                }
                let [e, dd, cc, b, a] = q;
                // Rees invariants: research/closed-forms/
                let p = 8.0 * a * cc - 3.0 * b * b;
                let dq = 64.0 * a * a * a * e - 16.0 * a * a * cc * cc + 16.0 * a * b * b * cc
                    - 16.0 * a * a * b * dd
                    - 3.0 * b * b * b * b;
                return if p < 0.0 && dq < 0.0 { 4 } else { 0 };
            }
        }
        let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
        self.roots(&c, &mut r)
    }

    /// Host derivatives at (u, s) and F_u, F_s, F_uu, F_us, F_ss there.
    fn partials(&self, u: f64, s: f64) -> (crate::surface::SurfDeriv, [f64; 5]) {
        let d = self.host.deriv(Point2::new(u, s));
        let (_, g) = self.alg.grad(d.p);
        let h = |x: Vec3, y: Vec3| self.alg.hess(d.p, x, y);
        let f = [
            g.dot(d.du),
            g.dot(d.dv),
            h(d.du, d.du) + g.dot(d.duu),
            h(d.du, d.dv) + g.dot(d.duv),
            h(d.dv, d.dv) + g.dot(d.dvv),
        ];
        (d, f)
    }
}

/// Real roots of the polynomial `c` (ascending, degree at most 4) in [lo, hi], ascending.
/// A root where the value only touches zero (within rounding) is included.
fn real_roots(c: &[f64], lo: f64, hi: f64, out: &mut [f64; 4]) -> usize {
    let mut deg = c.len();
    while deg > 0 && c[deg - 1] == 0.0 {
        deg -= 1;
    }
    let c = &c[..deg];
    let mut n = 0;
    let push = |x: f64, out: &mut [f64; 4], n: &mut usize| {
        if *n < 4 && (*n == 0 || x - out[*n - 1] > 1e-14 * (1.0 + x.abs())) {
            out[*n] = x;
            *n += 1;
        }
    };
    match deg {
        0 | 1 => return 0,
        2 => {
            let x = -c[0] / c[1];
            if lo <= x && x <= hi {
                out[0] = x;
                return 1;
            }
            return 0;
        }
        _ => {}
    }
    let mut d = [0.0; 4];
    for k in 1..deg {
        d[k - 1] = c[k] * k as f64;
    }
    let mut crit = [0.0; 4];
    let nc = real_roots(&d[..deg - 1], lo, hi, &mut crit);
    let mut cuts = [0.0; 6];
    cuts[0] = lo;
    cuts[1..=nc].copy_from_slice(&crit[..nc]);
    cuts[nc + 1] = hi;
    for w in cuts[..nc + 2].windows(2) {
        let (a, b) = (w[0], w[1]);
        let (fa, ea) = poly::eval_with_bound(c, a);
        let (fb, eb) = poly::eval_with_bound(c, b);
        let (za, zb) = (fa.abs() <= ea, fb.abs() <= eb);
        if za {
            push(a, out, &mut n);
        }
        if !za && !zb && (fa < 0.0) != (fb < 0.0) {
            push(bisect(c, &d[..deg - 1], a, b, fa), out, &mut n);
        }
        if zb {
            push(b, out, &mut n);
        }
    }
    n
}

fn bisect(c: &[f64], d: &[f64], mut a: f64, mut b: f64, fa: f64) -> f64 {
    let neg = fa < 0.0;
    let mut x = 0.5 * (a + b);
    for _ in 0..200 {
        let f = poly::eval(c, x);
        if f == 0.0 {
            return x;
        }
        if (f < 0.0) == neg {
            a = x;
        } else {
            b = x;
        }
        let df = poly::eval(d, x);
        let newton = x - f / df;
        let next = if df != 0.0 && newton > a && newton < b {
            newton
        } else {
            0.5 * (a + b)
        };
        if (next - x).abs() <= 2.0 * f64::EPSILON * x.abs() || b - a <= f64::EPSILON * a.abs().max(b.abs()) {
            return next;
        }
        x = next;
    }
    x
}

/// A trigonometric polynomial sum a_j cos(j u) + b_j sin(j u).
struct Trig {
    a: Vec<f64>,
    b: Vec<f64>,
}

impl Trig {
    /// The trigonometric polynomial through `n` equally spaced samples of `f` (exact when
    /// f has degree below n / 2).
    fn fit(f: impl Fn(f64) -> f64, n: usize) -> Trig {
        let ys: Vec<f64> = (0..n).map(|k| f(dmath::TAU * k as f64 / n as f64)).collect();
        let deg = (n - 1) / 2;
        let mut a = vec![0.0; deg + 1];
        let mut b = vec![0.0; deg + 1];
        for (k, &y) in ys.iter().enumerate() {
            let (s1, c1) = dmath::sin_cos(dmath::TAU * k as f64 / n as f64);
            let (mut s, mut c) = (0.0, 1.0);
            for j in 0..=deg {
                a[j] += y * c;
                b[j] += y * s;
                (s, c) = (s * c1 + c * s1, c * c1 - s * s1);
            }
        }
        a[0] /= n as f64;
        for j in 1..=deg {
            a[j] *= 2.0 / n as f64;
            b[j] *= 2.0 / n as f64;
        }
        Trig { a, b }
    }

    /// Value, first and second derivative.
    fn eval(&self, u: f64) -> (f64, f64, f64) {
        let (s1, c1) = dmath::sin_cos(u);
        let (mut s, mut c) = (0.0, 1.0);
        let (mut v, mut d, mut dd) = (0.0, 0.0, 0.0);
        for j in 0..self.a.len() {
            let (x, y) = (self.a[j] * c + self.b[j] * s, self.b[j] * c - self.a[j] * s);
            let jf = j as f64;
            v += x;
            d += jf * y;
            dd -= jf * jf * x;
            (s, c) = (s * c1 + c * s1, c * c1 - s * s1);
        }
        (v, d, dd)
    }

    fn size(&self) -> f64 {
        self.a.iter().chain(&self.b).map(|x| x.abs()).sum()
    }

    /// Roots in [0, 2 pi): simple ones to full precision, and places where the value and
    /// slope both vanish within rounding (flagged true: a multiple root).
    fn roots(&self) -> Option<Vec<(f64, bool)>> {
        // bounds on the second and third derivative: Taylor models of the value and slope
        let (mut l1, mut l2, mut l3) = (0.0, 0.0, 0.0);
        for j in 0..self.a.len() {
            let m = self.a[j].abs() + self.b[j].abs();
            let jf = j as f64;
            l1 += jf * m;
            l2 += jf * jf * m;
            l3 += jf * jf * jf * m;
        }
        let err = 1e-13 * self.size();
        let derr = 1e-13 * l1;
        let hmin = 1e-9;
        let mut out: Vec<(f64, bool)> = Vec::new();
        let mut leaves: Vec<(f64, f64, f64)> = Vec::new();
        let mut stack = vec![(0.0, dmath::TAU)];
        let mut budget = 200_000;
        while let Some((x0, x1)) = stack.pop() {
            budget -= 1;
            if budget == 0 {
                return None;
            }
            let (m, h) = (0.5 * (x0 + x1), 0.5 * (x1 - x0));
            let (v, d, dd) = self.eval(m);
            if v.abs() > d.abs() * h + 0.5 * l2 * h * h + err {
                continue;
            }
            if d.abs() > dd.abs() * h + 0.5 * l3 * h * h + derr {
                let (f0, f1) = (self.eval(x0).0, self.eval(x1).0);
                if f0 == 0.0 {
                    out.push((x0, false));
                } else if (f0 < 0.0) != (f1 < 0.0) && f1 != 0.0 {
                    out.push((self.bisect(x0, x1, f0), false));
                }
                continue;
            }
            if h < hmin {
                leaves.push((x0, x1, v.abs()));
                if leaves.len() > 4000 {
                    return None;
                }
                continue;
            }
            stack.push((m, x1));
            stack.push((x0, m));
        }
        // runs of touching leaves: one multiple root each, where the value is least
        leaves.sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut i = 0;
        while i < leaves.len() {
            let mut j = i;
            let mut best = i;
            while j + 1 < leaves.len() && leaves[j + 1].0 <= leaves[j].1 + 1e-15 {
                j += 1;
                if leaves[j].2 < leaves[best].2 {
                    best = j;
                }
            }
            out.push((0.5 * (leaves[best].0 + leaves[best].1), true));
            i = j + 1;
        }
        out.sort_by(|p, q| p.0.total_cmp(&q.0));
        // merge roots a hair apart (the ends of the range are one point)
        let mut merged: Vec<(f64, bool)> = Vec::new();
        for r in out {
            match merged.last_mut() {
                Some(l) if r.0 - l.0 <= 1e-10 => l.1 |= r.1,
                _ => merged.push(r),
            }
        }
        if merged.len() > 1 {
            let (f, l) = (merged[0], merged[merged.len() - 1]);
            if f.0 + dmath::TAU - l.0 <= 1e-10 {
                merged[0].1 |= l.1;
                merged.pop();
            }
        }
        Some(merged)
    }

    fn bisect(&self, mut a: f64, mut b: f64, fa: f64) -> f64 {
        let neg = fa < 0.0;
        for _ in 0..100 {
            let m = 0.5 * (a + b);
            if m <= a || m >= b {
                break;
            }
            if (self.eval(m).0 < 0.0) == neg {
                a = m;
            } else {
                b = m;
            }
        }
        0.5 * (a + b)
    }
}

/// One root of F(u, .) followed from one event to the next.
#[derive(Clone, Debug, PartialEq)]
struct Piece {
    u0: f64,
    du: f64,
    /// Whether each end is a turning point (u stops and reverses there).
    turns: [bool; 2],
}

impl Piece {
    /// u at sigma in [0, 1]: it slows to a stop at a turning end, where the root moves
    /// like the square root of u.
    fn u(&self, sigma: f64) -> f64 {
        let (h, pi) = (self.du, dmath::PI);
        match self.turns {
            [true, true] => self.u0 + h * 0.5 * (1.0 - dmath::cos(pi * sigma)),
            [true, false] => self.u0 + h * (1.0 - dmath::cos(0.5 * pi * sigma)),
            [false, true] => self.u0 + h * dmath::sin(0.5 * pi * sigma),
            [false, false] => self.u0 + h * sigma,
        }
    }

    /// Points along the piece about `step` apart, both ends included. The root is
    /// followed by Newton steps from its last two values; every few steps, and wherever
    /// Newton falters, it is checked against all the roots.
    fn march(
        &self,
        eq: &Eqn,
        t: &Track,
        s0: f64,
        step: f64,
        span: (f64, f64),
        region: &Box3,
    ) -> Option<Vec<Point3>> {
        // away from the region the links stretch: half the way to it at most
        let near = region.grow(2.0 * step);
        let away = |p: Point3| {
            let d = |x: f64, i: Interval| (i.lo - x).max(x - i.hi).max(0.0);
            let (x, y, z) = (d(p.x, near.x), d(p.y, near.y), d(p.z, near.z));
            step.max(0.5 * (x * x + y * y + z * z).sqrt())
        };
        let point = |u: f64, s: f64| eq.host.eval(Point2::new(u, s));
        // the start is the event itself (s0 was found a hair inside the span)
        let u0 = self.u(0.0);
        let s0 = eq.pick(u0, s0, t.sign, t.count).0;
        let mut pts = vec![point(u0, s0)];
        let (mut sig, mut s, mut ds, mut last): (f64, f64, f64, f64) = (0.0, s0, 0.0, 1.0);
        let mut dsig = 1.0 / 64.0;
        let mut k = 0;
        while sig < 1.0 {
            let next = (sig + dsig).min(1.0);
            let u = self.u(next);
            // ahead along the last link (a turn in s is never more than half of one)
            let ahead = ds * (next - sig) / last;
            let guess = s + if ahead.abs() <= 1.0 { ahead } else { 0.0 };
            let c = eq.coeffs(u);
            let newton = eq.newton(&c, guess, t.sign);
            let (mut x, ok) = (newton.unwrap_or(guess), newton.is_some());
            k += 1;
            if !ok || next == 1.0 || k % 16 == 0 {
                let (y, n) = eq.pick_in(&c, guess, t.sign, t.count);
                let inner = (u - span.0).abs().min((u - span.1).abs()) > 1e-6;
                if inner && (n != t.count || (ok && (y - x).abs() > 1e-9 * (1.0 + x.abs()))) {
                    return None; // a missed event, or the root hopped to another
                }
                x = y;
            }
            if eq.periodic() {
                x = s + wrap_pi(x - s);
            }
            let p = point(u, x);
            let back = pts[pts.len() - 1];
            let chord = p.distance(back);
            // a link turns at most about 20 degrees from the one before (unless it is
            // already much the shorter: a long link's chord leaves the tangent itself)
            let bent = pts.len() > 1 && {
                let (d0, d1) = (back - pts[pts.len() - 2], p - back);
                chord > 0.5 * d0.length() && d0.dot(d1) < 0.94 * d0.length() * chord
            };
            let reach = away(back).min(away(p));
            if chord > 1.5 * reach || bent {
                if dsig <= 1e-9 {
                    return None; // the root jumps: something the events did not see
                }
                dsig *= 0.5;
                continue;
            }
            (ds, last) = (x - s, next - sig);
            let grow = if chord > 0.0 {
                (reach / chord).clamp(0.5, 2.0)
            } else {
                2.0
            };
            (sig, s) = (next, x);
            dsig *= grow;
            pts.push(p);
            if pts.len() > 100_000 {
                return None;
            }
        }
        Some(pts)
    }
}

/// Which surface carries the parameter, and the form of the equation on it.
fn setup(a: &Surface, b: &Surface) -> Option<(Surface, Surface, Form)> {
    use Surface::*;
    let ruled_quadric = |h: &Surface, o: &Surface| Some((h.clone(), o.clone(), Form::Poly2));
    match (a, b) {
        (Cylinder(c), Cylinder(d)) => {
            if c.radius <= d.radius {
                ruled_quadric(a, b)
            } else {
                ruled_quadric(b, a)
            }
        }
        // a cone whose apex lies on the other would have a root there at every u
        (Cone(c), Cone(_)) if b.implicit(c.apex()).abs() <= tol::LINEAR => ruled_quadric(b, a),
        (Cylinder(_), Cone(_) | Sphere(_)) | (Cone(_), Sphere(_) | Cone(_)) => ruled_quadric(a, b),
        (Cone(_) | Sphere(_), Cylinder(_)) | (Sphere(_), Cone(_)) => ruled_quadric(b, a),
        (Cylinder(_) | Cone(_), Torus(_)) => Some((a.clone(), b.clone(), Form::Poly4)),
        (Torus(_), Cylinder(_) | Cone(_)) => Some((b.clone(), a.clone(), Form::Poly4)),
        (Torus(_), Plane(_) | Sphere(_)) => Some((a.clone(), b.clone(), Form::Trig1)),
        (Plane(_) | Sphere(_), Torus(_)) => Some((b.clone(), a.clone(), Form::Trig1)),
        (Torus(s), Torus(t)) => {
            if s.minor <= t.minor {
                Some((a.clone(), b.clone(), Form::Trig2))
            } else {
                Some((b.clone(), a.clone(), Form::Trig2))
            }
        }
        _ => None,
    }
}

/// One root of F(u, .) over an interval between events.
struct Track {
    interval: usize,
    sign: f64,
    count: usize,
    /// Values just inside the interval's two ends.
    ends: [f64; 2],
}

/// Where two surfaces meet, in closed form, or None for configurations this does not
/// cover (the caller falls back to tracing). Loops whose box misses `region` are left out.
pub fn intersect(a: &Surface, b: &Surface, region: Box3) -> Option<SurfaceSurface> {
    if let Some(r) = coaxial(a, b) {
        return Some(r);
    }
    let (host, other, form) = setup(a, b)?;
    let eq = Eqn::new(host, Alg::of(&other)?, form);
    if form == Form::Poly2 {
        // the leading coefficient must keep its sign, or a branch runs off to infinity
        let lead: Vec<[f64; 5]> = (0..64).map(|k| eq.coeffs(dmath::TAU * k as f64 / 64.0)).collect();
        let size = lead.iter().flatten().fold(0.0f64, |m, x| m.max(x.abs()));
        if lead.iter().any(|c| c[2] * lead[0][2] <= 1e-18 * size * size) {
            return None;
        }
    }
    let mut events = match form {
        // the discriminant is a trigonometric polynomial of degree 4: fitted exactly, its
        // roots are certain
        Form::Poly2 | Form::Trig1 => {
            let disc = Trig::fit(|u| eq.disc_raw(u), 16);
            if !(disc.size() > 0.0 && disc.size().is_finite()) {
                return None;
            }
            disc.roots()?
        }
        // of degree 24, it is too poorly conditioned to fit: sampled where it is scaled
        Form::Poly4 | Form::Trig2 => eq.events(64),
    };
    // turning points to full precision (from the bisection they sit within its rounding)
    for e in &mut events {
        if !e.1 {
            e.0 = eq.polish_turn(e.0, 1e-6).unwrap_or(e.0).rem_euclid(dmath::TAU);
        }
    }
    events.sort_by(|p, q| p.0.total_cmp(&q.0));
    // events a hair apart (across u = 0 too) are one place where roots meet: a tangency
    // the rounding has split, kept as one multiple root
    let near = |x: f64, y: f64| wrap_pi(x - y).abs() <= 1e-7;
    let mut merged: Vec<(f64, bool)> = Vec::with_capacity(events.len());
    for e in events {
        match merged.last_mut() {
            Some(l) if near(l.0, e.0) => l.1 = true,
            _ => merged.push(e),
        }
    }
    if merged.len() > 1 && near(merged[0].0, merged[merged.len() - 1].0) {
        let l = merged.pop().expect("two or more");
        merged[0] = (if merged[0].1 { merged[0].0 } else { l.0 }, true);
    }
    let mut events = merged;
    if events.len() > MAX_EVENTS {
        return None;
    }
    // a simple event that changes no root count is a sign flicker of the rounding; one
    // that changes it by other than two is a pair of events seen as one
    // (a removal merges two intervals: only that one is counted again)
    let count_in = |events: &[(f64, bool)], j: usize| {
        let hi = if j + 1 < events.len() {
            events[j + 1].0
        } else {
            events[0].0 + dmath::TAU
        };
        eq.count(0.5 * (events[j].0 + hi))
    };
    let mut counts: Vec<usize> = (0..events.len()).map(|j| count_in(&events, j)).collect();
    loop {
        let n = events.len();
        let step = |j: usize| counts[(j + n - 1) % n].abs_diff(counts[j]);
        match (0..n).find(|&j| !events[j].1 && step(j) == 0) {
            Some(j) => {
                events.remove(j);
                counts.remove(j);
                if n > 1 {
                    let k = (j + n - 2) % (n - 1);
                    counts[k] = count_in(&events, k);
                }
            }
            None if (0..n).any(|j| !events[j].1 && step(j) != 2) => return None,
            None => break,
        }
    }
    // two events polished onto one point: the isolation was off somewhere
    if events.windows(2).any(|w| w[1].0 - w[0].0 <= 1e-9) {
        return None;
    }
    if events.len() > 64 {
        return None;
    }
    if events.iter().all(|e| e.1) && events.len() > 16 {
        return None;
    }
    // intervals between events; with none, the seam at u = 0 stands in
    let us: Vec<(f64, bool, bool)> = if events.is_empty() {
        vec![(0.0, false, true)]
    } else {
        events.iter().map(|&(u, m)| (u, m, false)).collect()
    };
    let m = us.len();
    let span = |j: usize| {
        let lo = us[j].0;
        let hi = if j + 1 < m {
            us[j + 1].0
        } else {
            us[0].0 + dmath::TAU
        };
        (lo, hi)
    };
    let step_in = |j: usize, width: f64| {
        let (_, multiple, seam) = us[j % m];
        let d: f64 = if seam {
            0.0
        } else if multiple {
            1e-5
        } else {
            1e-7
        };
        d.min(0.25 * width)
    };
    let mut tracks: Vec<Track> = Vec::new();
    let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
    for j in 0..m {
        let (lo, hi) = span(j);
        let mid = 0.5 * (lo + hi);
        let c = eq.coeffs(mid);
        let k = eq.roots(&c, &mut r);
        // an event the discriminant missed would change the count inside
        if [0.25, 0.75].iter().any(|f| eq.count(lo + (hi - lo) * f) != k) {
            return None;
        }
        let ends_at = [lo + step_in(j, hi - lo), hi - step_in(j + 1, hi - lo)];
        for &(s0, fs) in &r[..k] {
            if fs == 0.0 {
                return None; // a double root inside an interval: the events missed something
            }
            let sign = fs.signum();
            // Newton steps toward each end, and all the roots at the end itself (where two
            // may be close)
            let follow = |to: f64| {
                let mut s = s0;
                let steps = 6;
                for i in 1..=steps {
                    let u = mid + (to - mid) * i as f64 / steps as f64;
                    let c = eq.coeffs(u);
                    s = match eq.newton(&c, s, sign) {
                        Some(x) if i < steps => x,
                        x => eq.pick_in(&c, x.unwrap_or(s), sign, k).0,
                    };
                }
                s
            };
            tracks.push(Track {
                interval: j,
                sign,
                count: k,
                ends: [follow(ends_at[0]), follow(ends_at[1])],
            });
        }
    }
    // link the track ends at each event: (end, end, turn)
    let periodic = eq.periodic();
    let diff = |x: f64, y: f64| {
        if periodic {
            wrap_pi(x - y).abs()
        } else {
            (x - y).abs()
        }
    };
    let mut link: Vec<Option<(usize, bool)>> = vec![None; 2 * tracks.len()];
    let mut acnodes: Vec<Point3> = Vec::new();
    for (e, &(ue, multiple, _)) in us.iter().enumerate() {
        let left_j = (e + m - 1) % m;
        let (llo, lhi) = span(left_j);
        let (rlo, rhi) = span(e);
        // the left interval of the first event ends a period on
        let ul = lhi - step_in(e, lhi - llo) - if e == 0 { dmath::TAU } else { 0.0 };
        let ur = rlo + step_in(e, rhi - rlo);
        let slope = |u: f64, s: f64| {
            let (_, f) = eq.partials(u, s);
            if f[1] == 0.0 { f64::INFINITY } else { -f[0] / f[1] }
        };
        // ends: (end id, value, sign, slope, is left)
        let mut ends: Vec<(usize, f64, f64, f64, bool)> = Vec::new();
        for (i, t) in tracks.iter().enumerate() {
            if t.interval == left_j {
                ends.push((2 * i + 1, t.ends[1], t.sign, slope(ul, t.ends[1]), true));
            }
            if t.interval == e {
                ends.push((2 * i, t.ends[0], t.sign, slope(ur, t.ends[0]), false));
            }
        }
        let gap = ur - ul;
        let mut cands: Vec<(f64, usize, usize, bool)> = Vec::new();
        for x in 0..ends.len() {
            for y in x + 1..ends.len() {
                let (p, q) = (ends[x], ends[y]);
                if p.4 != q.4 {
                    let (l, r) = if p.4 { (p, q) } else { (q, p) };
                    // at a crossing, where each would be across the gap by its slope; elsewhere
                    // the gap is too narrow to matter (and slopes near a turn run wild)
                    let ahead = diff(l.1 + l.3 * gap, r.1).min(diff(r.1 - r.3 * gap, l.1));
                    let cost = if multiple && ahead.is_finite() {
                        ahead
                    } else {
                        diff(l.1, r.1)
                    };
                    cands.push((cost, x, y, false));
                } else if p.2 != q.2 {
                    cands.push((diff(p.1, q.1), x, y, true));
                }
            }
        }
        cands.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut used = vec![false; ends.len()];
        for (_, x, y, turn) in cands {
            if used[x] || used[y] {
                continue;
            }
            used[x] = true;
            used[y] = true;
            link[ends[x].0] = Some((ends[y].0, turn));
            link[ends[y].0] = Some((ends[x].0, turn));
        }
        if used.iter().any(|u| !u) {
            return None;
        }
        // branches crossing where the surfaces are tangent: left to the tracer, which
        // declines them too (the boolean has no contact for such a point yet)
        if multiple
            && ends.iter().any(|end| {
                let p = eq.host.eval(Point2::new(ue, end.1));
                // (the ends sit a step off the crossing; a cone's apex has no normal)
                let (na, nb) = (eq.host.gradient(p), other.gradient(p));
                let (la, lb) = (na.length(), nb.length());
                la > 1e-3 && lb > 1e-3 && na.cross(nb).length() <= 1e-2 * la * lb
            })
        {
            return None;
        }
        if multiple && ends.is_empty() {
            // the surfaces touch at an isolated point: a double root with no branch through it
            let u = ue;
            let c = eq.coeffs(u);
            let mut r: Roots = [(0.0, 0.0); MAX_ROOTS];
            let k = eq.roots(&c, &mut r);
            let mut cands: Vec<f64> = r[..k].iter().map(|x| x.0).collect();
            if k == 0 {
                cands.extend((0..8).map(|i| eq.pick(u, -40.0 + 10.0 * i as f64, 1.0, 0).0));
            }
            for s in cands {
                let p = eq.host.eval(Point2::new(u, s));
                if other.implicit(p).abs() <= tol::LINEAR
                    && eq.host.implicit(p).abs() <= tol::LINEAR
                    && !acnodes.iter().any(|q| q.distance(p) <= tol::LINEAR)
                {
                    acnodes.push(p);
                }
            }
        }
    }
    // walk the links into loops, marching each piece's root at the tracer's spacing
    let step = 0.5 * (region.diagonal().max(1e-6) / 64.0).max(10.0 * tol::LINEAR);
    let mut done = vec![false; tracks.len()];
    let mut curves: Vec<crate::intcurve::IntCurve> = Vec::new();
    for start in 0..tracks.len() {
        if done[start] {
            continue;
        }
        let mut chain: Vec<Point3> = Vec::new();
        let mut joins: Vec<usize> = Vec::new();
        let (mut cur, mut fwd) = (start, true);
        loop {
            if done[cur] {
                return None;
            }
            done[cur] = true;
            let t = &tracks[cur];
            let (lo, hi) = span(t.interval);
            let (entry, exit) = if fwd {
                (2 * cur, 2 * cur + 1)
            } else {
                (2 * cur + 1, 2 * cur)
            };
            let turn = |end: usize| link[end].expect("every end linked").1;
            let (u0, du) = if fwd { (lo, hi - lo) } else { (hi, lo - hi) };
            let piece = Piece {
                u0,
                du,
                turns: [turn(entry), turn(exit)],
            };
            let pts = piece.march(&eq, t, t.ends[if fwd { 0 } else { 1 }], step, (lo, hi), &region)?;
            // each piece starts where the last one ended
            joins.push(chain.len());
            chain.extend(&pts[..pts.len() - 1]);
            let (next, _) = link[exit].expect("every end linked");
            cur = next / 2;
            fwd = next % 2 == 0;
            if cur == start {
                if !fwd {
                    return None;
                }
                break;
            }
        }
        // beside a turn the two roots are so close that a piece's last point can fall beyond the
        // next piece's first: the two change places, or the chain folds over itself there
        let n = chain.len();
        for &j in joins.iter().filter(|_| n >= 4) {
            for i in [j + n - 2, j + n - 1, j, j + 1] {
                let [a, b, c] = [i + n - 1, i, i + 1].map(|k| chain[k % n]);
                let (ab, bc) = (b - a, c - b);
                if ab.dot(bc) >= 0.0 {
                    continue;
                }
                let cd = chain[(i + 2) % n] - c;
                if bc.dot(cd) < 0.0 && ab.dot(cd) > 0.0 {
                    chain.swap(i % n, (i + 1) % n);
                }
            }
        }
        // links much shorter than the step (where u eases into a turn) would only make
        // slivers: such a point goes when its neighbours stay within reach of each other
        let mut kept: Vec<Point3> = Vec::with_capacity(chain.len());
        for i in 0..chain.len() {
            let p = chain[i];
            let next = chain[(i + 1) % chain.len()];
            match kept.last() {
                Some(&q) if q.distance(p) < 0.2 * step && q.distance(next) <= 1.5 * step => {}
                _ => kept.push(p),
            }
        }
        let chain = kept;
        let keep = chain.iter().filter(|&&q| on_nappes(&eq.host, &other, q)).count();
        if keep < chain.len() {
            if keep > 0 {
                return None; // a loop through a cone's apex
            }
            continue;
        }
        let n = chain.len();
        if (0..n).map(|i| chain[i].distance(chain[(i + 1) % n])).sum::<f64>() <= 10.0 * tol::LINEAR {
            acnodes.push(chain[0]);
            continue;
        }
        // next to a turning point a root is only as good as its u: pulled onto both surfaces
        let tight = |p: Point3| a.implicit(p).abs().max(b.implicit(p).abs()) <= 1e-3 * tol::LINEAR;
        let exact = |p: Point3| {
            if tight(p) {
                p
            } else {
                crate::intcurve::refine(a, b, p).unwrap_or(p)
            }
        };
        // along na x nb, as the tracer runs
        let mut chain = chain;
        if let Some(d) = crate::intcurve::direction(a, b, chain[0])
            && (chain[1] - chain[0]).dot(d) < 0.0
        {
            chain.reverse();
        }
        // the runs inside the region, each with a point beyond either end, as the tracer
        // lays them
        let grown = region.grow(step);
        let inside: Vec<bool> = chain.iter().map(|&p| grown.contains(p)).collect();
        if inside.iter().all(|&x| x) {
            curves.push(crate::intcurve::finish(
                a,
                b,
                chain.into_iter().map(exact).collect(),
                true,
                step,
            ));
            continue;
        }
        for i in 0..n {
            if inside[i] || !inside[(i + 1) % n] {
                continue;
            }
            let mut run = vec![exact(chain[i])];
            let mut j = (i + 1) % n;
            loop {
                run.push(exact(chain[j]));
                if !inside[j] {
                    break;
                }
                j = (j + 1) % n;
            }
            curves.push(crate::intcurve::finish(a, b, run, false, step));
        }
    }
    if !curves.is_empty() {
        return Some(SurfaceSurface::Curves(
            curves
                .into_iter()
                .map(|c| (Curve::Intersection(std::sync::Arc::new(c)), Contact::Crossing))
                .collect(),
        ));
    }
    if !acnodes.is_empty() {
        return Some(SurfaceSurface::Points(acnodes));
    }
    Some(SurfaceSurface::Empty)
}

/// False for a point on the mirror nappe of either surface's cone.
fn on_nappes(host: &Surface, other: &Surface, p: Point3) -> bool {
    [host, other].iter().all(|s| match s {
        Surface::Cone(c) => c.frame.point_to_local(p).z >= c.apex_v() - tol::LINEAR,
        _ => true,
    })
}

/// A surface's meridian profile in (distance from the axis, height along it).
#[derive(Clone, Copy, Debug)]
enum Profile {
    /// Through p along unit d; only where the distance is not negative (a half-cone).
    Line([f64; 2], [f64; 2]),
    Circle([f64; 2], f64),
}

fn axis_of(s: &Surface) -> Option<(Point3, forge_math::UnitVec3)> {
    match s {
        Surface::Cylinder(c) => Some((c.frame.origin, c.frame.z())),
        Surface::Cone(c) => Some((c.frame.origin, c.frame.z())),
        Surface::Torus(t) => Some((t.frame.origin, t.frame.z())),
        _ => None,
    }
}

fn profile(s: &Surface, o: Point3, z: forge_math::UnitVec3) -> Option<Profile> {
    let h = |p: Point3| z.dot(p - o);
    Some(match s {
        Surface::Cylinder(c) => Profile::Line([c.radius, h(c.frame.origin)], [0.0, 1.0]),
        Surface::Cone(c) => {
            let sg = c.frame.z().dot(z).signum();
            Profile::Line([c.radius, h(c.frame.origin)], [c.sin_a(), sg * c.cos_a()])
        }
        Surface::Sphere(sp) => Profile::Circle([0.0, h(sp.frame.origin)], sp.radius),
        Surface::Torus(t) => Profile::Circle([t.major, h(t.frame.origin)], t.minor),
        _ => return None,
    })
}

fn sub(p: [f64; 2], q: [f64; 2]) -> [f64; 2] {
    [p[0] - q[0], p[1] - q[1]]
}

/// Points (distance, height, touching) where a profile line meets a profile circle.
fn line_circle(p: [f64; 2], d: [f64; 2], c: [f64; 2], r: f64, hits: &mut Vec<(f64, f64, bool)>) {
    let w = sub(c, p);
    let t0 = w[0] * d[0] + w[1] * d[1];
    let foot = [p[0] + d[0] * t0, p[1] + d[1] * t0];
    let off = sub(c, foot);
    let dist = (off[0] * off[0] + off[1] * off[1]).sqrt();
    if dist > r + tol::LINEAR {
        return;
    }
    if (dist - r).abs() <= tol::LINEAR {
        hits.push((foot[0], foot[1], true));
        return;
    }
    let k = (r * r - dist * dist).sqrt();
    for sg in [-1.0, 1.0] {
        hits.push((foot[0] + d[0] * sg * k, foot[1] + d[1] * sg * k, false));
    }
}

/// Surfaces of revolution about one axis (a sphere centred on it counts) meet in circles
/// about it, where their profiles meet in a half-plane through the axis. None when the
/// surfaces do not share an axis.
pub fn coaxial(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    let (o, z) = axis_of(a).or_else(|| axis_of(b))?;
    let on_axis = |p: Point3| {
        let q = p - o;
        (q - z * z.dot(q)).length() <= tol::LINEAR
    };
    for s in [a, b] {
        let ok = match axis_of(s) {
            Some((p, d)) => d.cross(z).length() <= tol::ANGULAR && on_axis(p),
            None => matches!(s, Surface::Sphere(sp) if on_axis(sp.frame.origin)),
        };
        if !ok {
            return None;
        }
    }
    let (pa, pb) = (profile(a, o, z)?, profile(b, o, z)?);
    let mut hits: Vec<(f64, f64, bool)> = Vec::new();
    match (pa, pb) {
        (Profile::Line(p, d), Profile::Line(q, e)) => {
            let cross = d[0] * e[1] - d[1] * e[0];
            let w = sub(q, p);
            if cross.abs() <= tol::ANGULAR {
                let off = (w[0] * d[1] - w[1] * d[0]).abs();
                return Some(if off <= tol::LINEAR {
                    SurfaceSurface::Coincident
                } else {
                    SurfaceSurface::Empty
                });
            }
            let t = (w[0] * e[1] - w[1] * e[0]) / cross;
            hits.push((p[0] + d[0] * t, p[1] + d[1] * t, false));
        }
        (Profile::Line(p, d), Profile::Circle(c, r)) | (Profile::Circle(c, r), Profile::Line(p, d)) => {
            line_circle(p, d, c, r, &mut hits)
        }
        (Profile::Circle(c1, r1), Profile::Circle(c2, r2)) => {
            let w = sub(c2, c1);
            let d = (w[0] * w[0] + w[1] * w[1]).sqrt();
            if d <= tol::LINEAR {
                return Some(if (r1 - r2).abs() <= tol::LINEAR {
                    SurfaceSurface::Coincident
                } else {
                    SurfaceSurface::Empty
                });
            }
            if d > r1 + r2 + tol::LINEAR || d < (r1 - r2).abs() - tol::LINEAR {
                return Some(SurfaceSurface::Empty);
            }
            let x = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
            let (ex, ey) = ([w[0] / d, w[1] / d], [-w[1] / d, w[0] / d]);
            let at = |y: f64| (c1[0] + ex[0] * x + ey[0] * y, c1[1] + ex[1] * x + ey[1] * y);
            let h2 = r1 * r1 - x * x;
            let tangent = (d - r1 - r2).abs() <= tol::LINEAR || (d - (r1 - r2).abs()).abs() <= tol::LINEAR;
            if tangent || h2 <= 0.0 {
                let p = at(0.0);
                hits.push((p.0, p.1, true));
            } else {
                for y in [-h2.sqrt(), h2.sqrt()] {
                    let p = at(y);
                    hits.push((p.0, p.1, false));
                }
            }
        }
    }
    let mut curves: Vec<(Curve, Contact)> = Vec::new();
    let mut points: Vec<Point3> = Vec::new();
    let mut seen: Vec<(f64, f64)> = Vec::new();
    for (rho, h, touch) in hits {
        // the far side of the axis is the profile's mirror (a half-cone stops at its apex)
        let dup = seen
            .iter()
            .any(|q| (q.0 - rho).abs() <= tol::LINEAR && (q.1 - h).abs() <= tol::LINEAR);
        if rho < -tol::LINEAR || dup {
            continue;
        }
        seen.push((rho, h));
        let centre = o + z * h;
        if rho <= tol::LINEAR {
            points.push(centre);
            continue;
        }
        let contact = if touch {
            Contact::Touching
        } else {
            Contact::Crossing
        };
        let frame = Frame::from_z(centre, z);
        curves.push((Curve::Circle(crate::Circle { frame, radius: rho }), contact));
    }
    Some(if !curves.is_empty() {
        SurfaceSurface::Curves(curves)
    } else if !points.is_empty() {
        SurfaceSurface::Points(points)
    } else {
        SurfaceSurface::Empty
    })
}

#[cfg(test)]
#[path = "meet_tests.rs"]
mod tests;
