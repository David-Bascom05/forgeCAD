use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, dmath};

/// The parameter range a curve or surface direction is defined on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain1 {
    pub lo: f64,
    pub hi: f64,
    /// Set when the parameter wraps around: t and t + period are the same point.
    pub period: Option<f64>,
}

impl Domain1 {
    /// The parameter range, as an interval (one period for a wrapping parameter).
    pub fn interval(&self) -> Interval {
        Interval::new(self.lo, self.hi)
    }
    pub const LINE: Domain1 = Domain1 {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
        period: None,
    };
    pub const ANGLE: Domain1 = Domain1 {
        lo: 0.0,
        hi: dmath::TAU,
        period: Some(dmath::TAU),
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveDeriv {
    pub p: Point3,
    pub d1: Vec3,
    pub d2: Vec3,
}

pub trait CurveGeom {
    fn deriv(&self, t: f64) -> CurveDeriv;
    fn eval(&self, t: f64) -> Point3 {
        self.deriv(t).p
    }
    fn domain(&self) -> Domain1;
    /// Parameter of the point nearest `p` with t inside `range`.
    fn closest(&self, p: Point3, range: Interval) -> f64;
    /// A box guaranteed to contain the curve over `range`.
    fn bbox(&self, range: Interval) -> Box3;
    fn transformed(&self, f: &Frame) -> Self
    where
        Self: Sized;
    fn tangent(&self, t: f64) -> Option<UnitVec3> {
        self.deriv(t).d1.normalize()
    }
}

/// Unit-speed infinite line: t is distance from `origin` along `dir`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub origin: Point3,
    pub dir: UnitVec3,
}

/// Circle of `radius` in the frame's xy plane, centred on its origin, running
/// counter-clockwise about its z. t is the angle from x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub frame: Frame,
    pub radius: f64,
}

/// Ellipse with semi-axis `major` along the frame's x and `minor` along y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipse {
    pub frame: Frame,
    pub major: f64,
    pub minor: f64,
}

/// Parabola with vertex at the frame origin opening along x, focal length
/// `focal`: P(t) = (t^2 / 4f) x + t y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parabola {
    pub frame: Frame,
    pub focal: f64,
}

/// One branch of a hyperbola centred on the frame origin, opening along +x:
/// P(t) = a sqrt(1 + t^2) x + b t y. (sqrt instead of cosh keeps it algebraic.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hyperbola {
    pub frame: Frame,
    pub a: f64,
    pub b: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    Line(Line),
    Circle(Circle),
    Ellipse(Ellipse),
    Parabola(Parabola),
    Hyperbola(Hyperbola),
    /// Where two surfaces meet, for pairs with no closed form.
    Intersection(std::sync::Arc<crate::intcurve::IntCurve>),
    /// Defined by construction (a blend's spring curve, a corner patch's boundary).
    Procedural(ProcCurve),
}

/// A curve defined by construction rather than by a formula of its own, such as where a
/// rolling-ball blend touches a face (tangent there, so not a transversal intersection). Exact:
/// points are solved for to full precision from the definition.
pub trait ProceduralCurve: Send + Sync + std::fmt::Debug {
    fn deriv(&self, t: f64) -> CurveDeriv;
    fn domain(&self) -> Domain1;
    /// Parameter of the point nearest `p` with t inside `range`.
    fn closest(&self, p: Point3, range: Interval) -> f64;
    /// A box guaranteed to contain the curve over `range`.
    fn bbox(&self, range: Interval) -> Box3;
    fn transformed(&self, f: &Frame) -> ProcCurve;
    /// A bound on the speed |d/dt| over `range` (for subdivision searches).
    fn speed_bound(&self, range: Interval) -> f64;
    /// Chords that draw `range` within `chord` of the curve (equal parameter steps).
    fn segments(&self, range: Interval, chord: f64) -> usize;
    /// A surface holding the whole curve and crossing the faces it bounds, for telling which
    /// side of it another curve passes, where the faces' own surfaces can't (a helix between
    /// two faces of one swept loop).
    fn across(&self) -> Option<crate::Surface> {
        None
    }
}

/// A shared handle to a procedural curve; two handles are equal when they are the same curve.
#[derive(Clone, Debug)]
pub struct ProcCurve(pub std::sync::Arc<dyn ProceduralCurve>);

impl std::ops::Deref for ProcCurve {
    type Target = dyn ProceduralCurve;
    fn deref(&self) -> &(dyn ProceduralCurve + 'static) {
        &*self.0
    }
}

impl PartialEq for ProcCurve {
    fn eq(&self, o: &ProcCurve) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &o.0)
    }
}

impl Line {
    pub fn new(origin: Point3, dir: UnitVec3) -> Self {
        Line { origin, dir }
    }
    pub fn through(a: Point3, b: Point3) -> Option<Self> {
        Some(Line {
            origin: a,
            dir: (b - a).normalize()?,
        })
    }
    pub fn param_of(&self, p: Point3) -> f64 {
        self.dir.dot(p - self.origin)
    }
}

impl CurveGeom for Line {
    fn deriv(&self, t: f64) -> CurveDeriv {
        CurveDeriv {
            p: self.origin + self.dir * t,
            d1: self.dir.get(),
            d2: Vec3::ZERO,
        }
    }
    fn eval(&self, t: f64) -> Point3 {
        self.origin + self.dir * t
    }
    fn domain(&self) -> Domain1 {
        Domain1::LINE
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        self.param_of(p).clamp(range.lo, range.hi)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let d = self.dir.get();
        let o = self.origin;
        Box3 {
            x: Interval::point(o.x) + range * d.x,
            y: Interval::point(o.y) + range * d.y,
            z: Interval::point(o.z) + range * d.z,
        }
    }
    fn transformed(&self, f: &Frame) -> Self {
        Line {
            origin: f.point_to_world(self.origin),
            dir: f.unit_to_world(self.dir),
        }
    }
}

/// Brings angle `t` into the window starting at `range.lo`, or returns the
/// nearer end of `range` (by angular distance) if it falls outside.
fn angle_into(t: f64, range: Interval) -> Option<f64> {
    let shifted = range.lo + dmath::wrap_tau(t - range.lo);
    (shifted <= range.hi).then_some(shifted)
}

fn closest_on_arc(p: Point3, range: Interval, t_global: f64, eval: impl Fn(f64) -> Point3) -> f64 {
    if range.width() >= dmath::TAU {
        return range.lo + dmath::wrap_tau(t_global - range.lo);
    }
    match angle_into(t_global, range) {
        Some(t) => t,
        None => {
            if eval(range.lo).distance_sq(p) <= eval(range.hi).distance_sq(p) {
                range.lo
            } else {
                range.hi
            }
        }
    }
}

impl CurveGeom for Circle {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (s, c) = dmath::sin_cos(t);
        let f = &self.frame;
        let r = self.radius;
        CurveDeriv {
            p: f.origin + f.x() * (r * c) + f.y() * (r * s),
            d1: f.x() * (-r * s) + f.y() * (r * c),
            d2: f.x() * (-r * c) + f.y() * (-r * s),
        }
    }
    fn eval(&self, t: f64) -> Point3 {
        let (s, c) = dmath::sin_cos(t);
        let f = &self.frame;
        f.origin + f.x() * (self.radius * c) + f.y() * (self.radius * s)
    }
    fn domain(&self) -> Domain1 {
        Domain1::ANGLE
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let l = self.frame.point_to_local(p);
        let t = if l.x == 0.0 && l.y == 0.0 {
            range.lo
        } else {
            dmath::atan2(l.y, l.x)
        };
        closest_on_arc(p, range, t, |t| self.eval(t))
    }
    fn bbox(&self, range: Interval) -> Box3 {
        conic_box(&self.frame, range.cos() * self.radius, range.sin() * self.radius)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Circle {
            frame: f.compose(&self.frame),
            radius: self.radius,
        }
    }
}

/// Box of origin + X x + Y y, with X and Y given as intervals.
fn conic_box(f: &Frame, xs: Interval, ys: Interval) -> Box3 {
    let (x, y, o) = (f.x().get(), f.y().get(), f.origin);
    Box3 {
        x: Interval::point(o.x) + xs * x.x + ys * y.x,
        y: Interval::point(o.y) + xs * x.y + ys * y.y,
        z: Interval::point(o.z) + xs * x.z + ys * y.z,
    }
}

/// Nearest point by finding every critical point of the squared distance on a
/// sampled grid (sign changes of (P - p).P') and bisecting each, plus the ends.
/// Conics have at most four critical points, well separated except near the
/// evolute, so 64 samples find them all in practice.
fn closest_numeric(c: &impl CurveGeom, p: Point3, range: Interval) -> f64 {
    const N: usize = 64;
    let g = |t: f64| {
        let d = c.deriv(t);
        (d.p - p).dot(d.d1)
    };
    let mut best = (c.eval(range.lo).distance_sq(p), range.lo);
    let dhi = c.eval(range.hi).distance_sq(p);
    if dhi < best.0 {
        best = (dhi, range.hi);
    }
    let mut t0 = range.lo;
    let mut g0 = g(t0);
    for i in 1..=N {
        let t1 = if i == N {
            range.hi
        } else {
            range.lo + range.width() * (i as f64 / N as f64)
        };
        let g1 = g(t1);
        if g0 < 0.0 && g1 >= 0.0 {
            let t = bisect_root(&g, t0, t1);
            let d = c.eval(t).distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        t0 = t1;
        g0 = g1;
    }
    best.1
}

fn bisect_root(g: &impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    for _ in 0..100 {
        let m = 0.5 * (a + b);
        if m <= a || m >= b {
            break;
        }
        if g(m) < 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    0.5 * (a + b)
}

impl CurveGeom for Ellipse {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (s, c) = dmath::sin_cos(t);
        let f = &self.frame;
        let (a, b) = (self.major, self.minor);
        CurveDeriv {
            p: f.origin + f.x() * (a * c) + f.y() * (b * s),
            d1: f.x() * (-a * s) + f.y() * (b * c),
            d2: f.x() * (-a * c) + f.y() * (-b * s),
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1::ANGLE
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let r = if range.width() >= dmath::TAU {
            Interval::new(0.0, dmath::TAU)
        } else {
            range
        };
        closest_numeric(self, p, r)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        conic_box(&self.frame, range.cos() * self.major, range.sin() * self.minor)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Ellipse {
            frame: f.compose(&self.frame),
            ..*self
        }
    }
}

impl CurveGeom for Parabola {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let f = &self.frame;
        let k = 1.0 / (4.0 * self.focal);
        CurveDeriv {
            p: f.origin + f.x() * (k * t * t) + f.y() * t,
            d1: f.x() * (2.0 * k * t) + f.y().get(),
            d2: f.x() * (2.0 * k),
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1::LINE
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        // d/dt |P - p|^2 / 2 = 2k^2 t^3 + (1 - 2k x) t - y, a cubic solved exactly.
        let l = self.frame.point_to_local(p);
        let k = 1.0 / (4.0 * self.focal);
        let c = [-l.y, 1.0 - 2.0 * k * l.x, 0.0, 2.0 * k * k];
        let mut best = (self.eval(range.lo).distance_sq(p), range.lo);
        for t in forge_math::poly::all_real_roots(&c)
            .into_iter()
            .map(|r| r.x)
            .chain([range.hi])
        {
            let t = t.clamp(range.lo, range.hi);
            let d = self.eval(t).distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        best.1
    }
    fn bbox(&self, range: Interval) -> Box3 {
        conic_box(&self.frame, range.sqr() * (1.0 / (4.0 * self.focal)), range)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Parabola {
            frame: f.compose(&self.frame),
            ..*self
        }
    }
}

impl CurveGeom for Hyperbola {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let f = &self.frame;
        let q = (1.0 + t * t).sqrt();
        CurveDeriv {
            p: f.origin + f.x() * (self.a * q) + f.y() * (self.b * t),
            d1: f.x() * (self.a * t / q) + f.y() * self.b,
            d2: f.x() * (self.a / (q * q * q)),
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1::LINE
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        // past this span the curve runs away from p: a wide range, sampled whole, would miss it
        let l = self.frame.point_to_local(p);
        let span = 4.0 * (l.x.abs() + l.y.abs()) / self.a.min(self.b) + 4.0;
        let near = Interval::new(range.lo.max(-span), range.hi.min(span));
        let r = if near.lo <= near.hi { near } else { range };
        closest_numeric(self, p, r)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        conic_box(&self.frame, (range.sqr() + 1.0).sqrt() * self.a, range * self.b)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Hyperbola {
            frame: f.compose(&self.frame),
            ..*self
        }
    }
}

macro_rules! dispatch {
    ($self:ident, $c:ident => $e:expr) => {
        match $self {
            Curve::Line($c) => $e,
            Curve::Circle($c) => $e,
            Curve::Ellipse($c) => $e,
            Curve::Parabola($c) => $e,
            Curve::Hyperbola($c) => $e,
            Curve::Intersection($c) => $e,
            Curve::Procedural($c) => $e,
        }
    };
}

impl CurveGeom for Curve {
    fn deriv(&self, t: f64) -> CurveDeriv {
        dispatch!(self, c => c.deriv(t))
    }
    fn eval(&self, t: f64) -> Point3 {
        match self {
            Curve::Line(c) => c.eval(t),
            Curve::Circle(c) => c.eval(t),
            Curve::Intersection(c) => c.eval(t),
            _ => self.deriv(t).p,
        }
    }
    fn domain(&self) -> Domain1 {
        dispatch!(self, c => c.domain())
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        dispatch!(self, c => c.closest(p, range))
    }
    fn bbox(&self, range: Interval) -> Box3 {
        dispatch!(self, c => c.bbox(range))
    }
    fn transformed(&self, f: &Frame) -> Self {
        match self {
            Curve::Line(c) => Curve::Line(c.transformed(f)),
            Curve::Circle(c) => Curve::Circle(c.transformed(f)),
            Curve::Ellipse(c) => Curve::Ellipse(c.transformed(f)),
            Curve::Parabola(c) => Curve::Parabola(c.transformed(f)),
            Curve::Hyperbola(c) => Curve::Hyperbola(c.transformed(f)),
            Curve::Intersection(c) => Curve::Intersection(std::sync::Arc::new(c.as_ref().transformed(f))),
            Curve::Procedural(c) => Curve::Procedural(c.0.transformed(f)),
        }
    }
}

impl Curve {
    /// Points at lo + k (hi - lo) / n for k in 1..n (the interior samples of an
    /// edge cut into n equal parameter steps), with stepped angles for conics.
    pub fn sample_interior(
        &self,
        range: Interval,
        n: usize,
        trig: &mut Vec<(f64, f64)>,
        out: &mut Vec<Point3>,
    ) {
        out.clear();
        if n < 2 {
            return;
        }
        let dt = range.width() / n as f64;
        let conic = |f: &Frame, a: f64, b: f64, trig: &mut Vec<(f64, f64)>, out: &mut Vec<Point3>| {
            dmath::sin_cos_steps(range.lo + dt, dt, n - 1, trig);
            let (x, y) = (f.x().get(), f.y().get());
            for &(s, c) in trig.iter() {
                out.push(f.origin + x * (a * c) + y * (b * s));
            }
        };
        match self {
            Curve::Circle(c) => conic(&c.frame, c.radius, c.radius, trig, out),
            Curve::Ellipse(e) => conic(&e.frame, e.major, e.minor, trig, out),
            _ => {
                for k in 1..n {
                    out.push(self.eval(range.lo + dt * k as f64));
                }
            }
        }
    }

    /// The whole parameter range, clipped to `bound` where the curve is unbounded.
    pub fn full_range(&self, bound: f64) -> Interval {
        let d = self.domain();
        Interval::new(d.lo.max(-bound), d.hi.min(bound))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use forge_math::rng::Rng;

    fn rand_curves(r: &mut Rng) -> Vec<Curve> {
        let f = rand_frame(r);
        vec![
            Curve::Line(Line::new(rand_point(r, 50.0), rand_unit(r))),
            Curve::Circle(Circle {
                frame: f,
                radius: r.range(0.1, 30.0),
            }),
            Curve::Ellipse(Ellipse {
                frame: f,
                major: r.range(5.0, 30.0),
                minor: r.range(0.5, 5.0),
            }),
            Curve::Parabola(Parabola {
                frame: f,
                focal: r.range(0.2, 10.0),
            }),
            Curve::Hyperbola(Hyperbola {
                frame: f,
                a: r.range(0.5, 10.0),
                b: r.range(0.5, 10.0),
            }),
        ]
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(100);
        for _ in 0..500 {
            for c in rand_curves(&mut r) {
                let t = r.range(-3.0, 3.0);
                let h = 1e-5;
                let d = c.deriv(t);
                let fd1 = (c.eval(t + h) - c.eval(t - h)) / (2.0 * h);
                let fd2 = (c.deriv(t + h).d1 - c.deriv(t - h).d1) / (2.0 * h);
                assert!((fd1 - d.d1).length() < 1e-5 * (1.0 + d.d1.length()), "{c:?}");
                assert!((fd2 - d.d2).length() < 1e-4 * (1.0 + d.d2.length()), "{c:?}");
            }
        }
    }

    #[test]
    fn closest_beats_dense_sampling() {
        let mut r = Rng::new(101);
        for _ in 0..300 {
            for c in rand_curves(&mut r) {
                let range = match c.domain().period {
                    Some(_) if r.bool() => Interval::new(0.0, dmath::TAU),
                    _ => {
                        let a = r.range(-4.0, 4.0);
                        Interval::new(a, a + r.range(0.1, 4.0))
                    }
                };
                let p = rand_point(&mut r, 40.0);
                let t = c.closest(p, range);
                assert!(range.contains(t), "{c:?} {range:?} {t}");
                let best = c.eval(t).distance(p);
                for i in 0..=2000 {
                    let s = range.lo + range.width() * i as f64 / 2000.0;
                    assert!(best <= c.eval(s).distance(p) + 1e-9, "{c:?} t={t} s={s}");
                }
            }
        }
    }

    #[test]
    fn bbox_contains_samples() {
        let mut r = Rng::new(102);
        for _ in 0..300 {
            for c in rand_curves(&mut r) {
                let a = r.range(-4.0, 4.0);
                let range = Interval::new(a, a + r.range(0.0, 7.0));
                let b = c.bbox(range);
                for i in 0..=100 {
                    let s = (range.lo + range.width() * i as f64 / 100.0).min(range.hi);
                    assert!(b.contains(c.eval(s)), "{c:?} {range:?}");
                }
            }
        }
    }

    #[test]
    fn transform_moves_points() {
        let mut r = Rng::new(103);
        for _ in 0..200 {
            let f = rand_frame(&mut r);
            for c in rand_curves(&mut r) {
                let t = r.range(-2.0, 2.0);
                let moved = c.transformed(&f);
                assert!(moved.eval(t).distance(f.point_to_world(c.eval(t))) < 1e-9);
            }
        }
    }
}
