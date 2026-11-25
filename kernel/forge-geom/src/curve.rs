use forge_math::{Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec2, Vec3, dmath};

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
#[non_exhaustive]
pub struct Ellipse {
    pub frame: Frame,
    pub major: f64,
    pub minor: f64,
}

impl Ellipse {
    /// An ellipse in `frame`'s xy plane: semi-axes `major` along x and `minor` along y, finite,
    /// `major > 0`, `minor >= 0` (H.curveOK, verify/forge-bool-proof/LEDGER.md: what `across` needs
    /// of an ellipse edge). Outside forge-geom this is the only way to make one (`#[non_exhaustive]`),
    /// so every producer is a call here; the condition is asserted in debug and test builds and
    /// costs nothing in release.
    #[inline]
    pub fn new(frame: Frame, major: f64, minor: f64) -> Ellipse {
        debug_assert!(
            major.is_finite() && major > 0.0 && minor.is_finite() && minor >= 0.0,
            "an ellipse's semi-axes must be finite, the major positive: {major} {minor}"
        );
        Ellipse { frame, major, minor }
    }
}

/// Parabola with vertex at the frame origin opening along x, focal length
/// `focal`: P(t) = (t^2 / 4f) x + t y.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Parabola {
    pub frame: Frame,
    pub focal: f64,
}

impl Parabola {
    /// A parabola in `frame`'s xy plane with focal length `focal`, finite and not zero (a negative
    /// one opens the other way; H.curveOK). Outside forge-geom the only way to make one
    /// (`#[non_exhaustive]`); asserted in debug and test builds, nothing in release.
    #[inline]
    pub fn new(frame: Frame, focal: f64) -> Parabola {
        debug_assert!(
            focal.is_finite() && focal != 0.0,
            "a parabola's focal length must be finite and not zero: {focal}"
        );
        Parabola { frame, focal }
    }
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
    /// The point alone (as `deriv`'s).
    fn point(&self, t: f64) -> Point3 {
        self.deriv(t).p
    }
    /// The point and first derivative alone (as `deriv`'s).
    fn deriv1(&self, t: f64) -> (Point3, Vec3) {
        let d = self.deriv(t);
        (d.p, d.d1)
    }
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
    /// Parameters strictly inside `range` where the parametrisation is only once
    /// differentiable (a traced spine's chain points), for integrators that need smooth pieces.
    fn kinks(&self, _range: Interval) -> Vec<f64> {
        Vec::new()
    }
    /// Whether this is a spline edge, asked before the costlier `as_any` on hot paths.
    fn is_spline(&self) -> bool {
        false
    }
    /// The curve as itself, for closed forms that know its kind.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
    /// Whether `deriv`'s first derivative is exact (not a difference), for integrators that
    /// take it as it is.
    fn exact_d1(&self) -> bool {
        false
    }
    /// The surface the curve lies on by construction, with its parameters there at t and their
    /// derivative (a blend's spring or cut), so integrators need not search for them.
    fn on_surface(&self, t: f64) -> Option<(crate::Surface, Point2, Vec2)> {
        let _ = t;
        None
    }
    /// The same curve with a smooth parameter where this one's is only once differentiable (a
    /// spring round a traced spine), for integrators, and `range` in its parameter.
    fn smooth(&self, range: Interval) -> Option<(&Curve, Interval)> {
        let _ = range;
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
        let (s, c) = range.sin_cos();
        conic_box(&self.frame, c * self.radius, s * self.radius)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Circle {
            frame: f.compose(&self.frame),
            radius: self.radius,
        }
    }
}

impl Circle {
    /// The box of an arc exactly, tilted or not: `bbox` is a bound, as it takes x and y apart.
    pub fn tight_box(&self, range: Interval) -> Box3 {
        let (x, y, o) = (self.frame.x().get(), self.frame.y().get(), self.frame.origin);
        let at = |t: f64| {
            let (sn, cs) = dmath::sin_cos(t);
            o + x * (self.radius * cs) + y * (self.radius * sn)
        };
        let mut b = Box3::from_points([at(range.lo), at(range.hi)]);
        for (a, c) in [(x.x, y.x), (x.y, y.y), (x.z, y.z)] {
            // an axis's extremes round the circle: half a turn apart
            let phi = dmath::atan2(c, a);
            for k in [0.0, dmath::PI] {
                let t = phi + k;
                let t = t + dmath::TAU * ((range.lo - t) / dmath::TAU).ceil();
                if t <= range.hi {
                    b = b.hull(&Box3::from_point(at(t)));
                }
            }
        }
        b
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

/// `ellipse_square` for semi-axes in either order (`Ellipse::new` does not order them: a STEP
/// ellipse's semi_axis_1 may be the shorter). With b > a, t = s + pi/2: the ellipse
/// (a cos t, b sin t) at the point (x, y) is the ellipse (b cos s, a sin s) at (y, -x), whose
/// major semi-axis is along x (EllipseFoot.lean `slope_swap`). The angles come back in
/// (-pi/2, 3pi/2].
fn ellipse_square_any(x: f64, y: f64, a: f64, b: f64) -> ([f64; 4], usize) {
    if b <= a {
        return ellipse_square(x, y, a, b);
    }
    let (mut ts, n) = ellipse_square(y, -x, b, a);
    for t in &mut ts[..n] {
        *t += dmath::FRAC_PI_2;
    }
    (ts, n)
}

/// Every angle t, in (-pi, pi], at which the ellipse (a cos t, b sin t), a >= b >= 0, a > 0, is square
/// to the point (x, y) (where the distance from it, in any plane parallel to the ellipse's, is
/// stationary): at most four.
///
/// Off the axes, a point square to (x, y) is cos t = a x / (d + e), sin t = b y / e, with
/// d = a^2 - b^2 and e a root of G(e) = U^2 / (d + e)^2 + V^2 / e^2 - 1, U = a|x|, V = b|y|
/// (Joachimsthal's, with e = mu + b^2), and every root gives one. G has poles at e = -d and 0:
/// beyond them it is monotone, one root each (the nearest point on e > 0, in [V, hypot(U, V)];
/// the farthest on d + e < 0, in [-hypot(U, V), -U]); between them, on (-d, 0), it is at least
/// (U^(2/3) + V^(2/3))^3 / d^2 - 1, so has no root unless the point is inside the evolute, and then
/// one each side of its least point e* = -d V^(2/3) / (U^(2/3) + V^(2/3)), each with its bracket.
/// On an axis, at the centre and for b = 0 the angles are closed forms. Each root is found by
/// Newton's method held inside its bracket, in the variable measured from the nearer pole so it
/// keeps its relative precision. verify/forge-curved/ForgeCurved/EllipseFoot.lean.
fn ellipse_square(x: f64, y: f64, a: f64, b: f64) -> ([f64; 4], usize) {
    use dmath::{FRAC_PI_2, PI};
    if b == 0.0 {
        // a segment run twice: its ends and the foot of the perpendicular, on either run
        let t = dmath::acos_clamped(x / a);
        return ([0.0, PI, t, -t], 4);
    }
    let d = (a - b) * (a + b);
    let (u, v) = (a * x.abs(), b * y.abs());
    match (u == 0.0, v == 0.0) {
        (true, true) => return ([0.0, FRAC_PI_2, PI, -FRAC_PI_2], 4),
        // on the major axis: its two vertices, and inside the evolute the pair X = a^2 x / d
        (false, true) => {
            if u < d {
                let c = a * x / d;
                let t = dmath::atan2((1.0 - c * c).sqrt(), c);
                return ([0.0, PI, t, -t], 4);
            }
            return ([0.0, PI, 0.0, 0.0], 2);
        }
        // on the minor axis: its two vertices, and inside the evolute the pair Y = -b^2 y / d
        (true, false) => {
            if v < d {
                let s = -b * y / d;
                let c = (1.0 - s * s).sqrt();
                return (
                    [FRAC_PI_2, -FRAC_PI_2, dmath::atan2(s, c), dmath::atan2(s, -c)],
                    4,
                );
            }
            return ([FRAC_PI_2, -FRAC_PI_2, 0.0, 0.0], 2);
        }
        (false, false) => {}
    }
    let mut out = [0.0; 4];
    // the nearest point: e > 0
    let g = |e: f64| {
        let (p, q) = (u / (d + e), v / e);
        (p * p + q * q - 1.0, -2.0 * (p * p / (d + e) + q * q / e))
    };
    // not `hypot`: the platform's libm rounds differently on each target (dmath's rule)
    let h = (u * u + v * v).sqrt();
    let e = held_newton(g, v, h, false);
    out[0] = dmath::atan2(b * y / e, a * x / (d + e));
    // in k = d + e, measured from the pole at e = -d
    let gk = |k: f64| {
        let (p, q) = (u / k, v / (k - d));
        (p * p + q * q - 1.0, -2.0 * (p * p / k + q * q / (k - d)))
    };
    // the farthest point: d + e < 0
    let k = held_newton(gk, -h, -u, true);
    out[1] = dmath::atan2(b * y / (k - d), a * x / k);
    if d == 0.0 {
        return (out, 2);
    }
    let (cu, cv) = (dmath::cbrt(u), dmath::cbrt(v));
    let (cu, cv) = (cu * cu, cv * cv);
    let sum = cu + cv;
    if sum * sum * sum > d * d {
        return (out, 2);
    }
    // inside the evolute: the root by e = 0 (e in [e*, -V], G rising) and by e = -d (k in
    // [U, d + e*], G falling); where U >> V or V >> U the far end rounds onto the pole
    let e = held_newton(g, d * cv / -sum, -v, true);
    out[2] = dmath::atan2(b * y / e, a * x / (d + e));
    let k = held_newton(gk, u, d * cu / sum, false);
    out[3] = dmath::atan2(b * y / (k - d), a * x / k);
    (out, 4)
}

/// The root of `f` (value and slope) between `lo` and `hi`, where it changes sign, rising or
/// falling as `rising` says (known, EllipseFoot.lean, not read off the ends: an end can round onto
/// a pole, where `f` is infinite): Newton's steps, a bisection wherever one would leave the
/// bracket, which shrinks every step; stops when a step moves nothing or the bracket is two
/// neighbouring doubles. `f` is never evaluated at the ends.
fn held_newton(f: impl Fn(f64) -> (f64, f64), mut lo: f64, mut hi: f64, rising: bool) -> f64 {
    if lo > hi {
        (lo, hi) = (hi, lo);
    }
    let mut t = 0.5 * (lo + hi);
    for _ in 0..200 {
        let (v, dv) = f(t);
        if v == 0.0 {
            break;
        }
        if (v < 0.0) == rising {
            lo = t;
        } else {
            hi = t;
        }
        let m = 0.5 * (lo + hi);
        if m <= lo || m >= hi {
            break;
        }
        let n = t - v / dv;
        let next = if n > lo && n < hi { n } else { m };
        if next == t {
            break;
        }
        t = next;
    }
    t
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
        if cfg!(feature = "ellipse-foot-sampled") {
            return closest_numeric(self, p, r);
        }
        // the nearest of the range's ends and every point square to `p` inside it: the least
        // distance over the range is at an end or at an interior point, which is square to `p`
        let l = self.frame.point_to_local(p);
        let (ts, n) = ellipse_square_any(l.x, l.y, self.major, self.minor);
        let mut best = (self.eval(r.lo).distance_sq(p), r.lo);
        let dhi = self.eval(r.hi).distance_sq(p);
        if dhi < best.0 {
            best = (dhi, r.hi);
        }
        for &t in &ts[..n] {
            if let Some(t) = angle_into(t, r) {
                let d = self.eval(t).distance_sq(p);
                if d < best.0 {
                    best = (d, t);
                }
            }
        }
        best.1
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let (s, c) = range.sin_cos();
        conic_box(&self.frame, c * self.major, s * self.minor)
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
        let r = if near.lo <= near.hi && range.width() > 4.0 * span {
            near
        } else {
            range
        };
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
            _ => self.eval_rest(t),
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
    /// Parameters strictly inside `range` where the parametrisation is only once
    /// differentiable: an intersection curve's chain points, or a procedural curve's own.
    pub fn kinks(&self, range: Interval) -> Vec<f64> {
        match self {
            Curve::Intersection(_) => {
                let mut k = range.lo.floor() + 1.0;
                let mut out = Vec::new();
                while k < range.hi {
                    out.push(k);
                    k += 1.0;
                }
                out
            }
            Curve::Procedural(c) => c.kinks(range),
            _ => Vec::new(),
        }
    }
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
            Curve::Hyperbola(_) => self.hyperbola_samples(range, n, out),
            _ => {
                for k in 1..n {
                    out.push(self.eval(range.lo + dt * k as f64));
                }
            }
        }
    }

    /// The parameter of `sample_interior`'s k-th point (k in 1..n): equal steps of the parameter,
    /// but for a hyperbola, equal steps of u = asinh t (its point a cosh u x + b sinh u y), whose
    /// tangent turns at most a / b a unit of u everywhere while t's speeds up as cosh u: equal
    /// steps of t over a wide range put almost all of them where it is nearly straight.
    pub fn sample_param(&self, range: Interval, n: usize, k: usize) -> f64 {
        let mut ts = Vec::with_capacity(n);
        self.sample_params(range, n, &mut ts);
        ts[k - 1]
    }

    /// A hyperbola's interior samples (out of line: sample_interior's other arms stay as they were).
    #[inline(never)]
    fn hyperbola_samples(&self, range: Interval, n: usize, out: &mut Vec<Point3>) {
        let mut ts = Vec::with_capacity(n);
        self.sample_params(range, n, &mut ts);
        out.extend(ts.iter().map(|&t| self.eval(t)));
    }

    /// `sample_param` for k in 1..n, into `out` (a hyperbola's by the addition formula from one
    /// sinh and cosh of the first point and of the step: sinh(u + h) = sinh u cosh h + cosh u sinh h).
    #[inline(never)]
    pub fn sample_params(&self, range: Interval, n: usize, out: &mut Vec<f64>) {
        out.clear();
        if let Curve::Hyperbola(_) = self {
            let (u0, u1) = (asinh(range.lo), asinh(range.hi));
            let h = (u1 - u0) / n as f64;
            let (sh, ch) = (sinh(h), (1.0 + sinh(h) * sinh(h)).sqrt());
            let (mut s, mut c) = (range.lo, (1.0 + range.lo * range.lo).sqrt());
            for _ in 1..n {
                (s, c) = (s * ch + c * sh, c * ch + s * sh);
                out.push(s.clamp(range.lo, range.hi));
            }
            return;
        }
        let dt = range.width() / n as f64;
        out.extend((1..n).map(|k| range.lo + dt * k as f64));
    }

    /// The whole parameter range, clipped to `bound` where the curve is unbounded.
    pub fn full_range(&self, bound: f64) -> Interval {
        let d = self.domain();
        Interval::new(d.lo.max(-bound), d.hi.min(bound))
    }
}

/// asinh by dmath's ln: ln(|x| + sqrt(x² + 1)) with x's sign (the series near 0, where the
/// sum cancels).
pub fn asinh(x: f64) -> f64 {
    let a = x.abs();
    let r = if a < 1e-4 {
        a * (1.0 - a * a / 6.0)
    } else {
        dmath::ln(a + (a * a + 1.0).sqrt())
    };
    if x < 0.0 { -r } else { r }
}

/// sinh by dmath's exp (the series near 0, where the difference cancels).
pub fn sinh(x: f64) -> f64 {
    let a = x.abs();
    let r = if a < 1e-4 {
        a * (1.0 + a * a / 6.0)
    } else {
        let e = dmath::exp(a);
        0.5 * (e - 1.0 / e)
    };
    if x < 0.0 { -r } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use forge_math::ipow::Powi as _;
    use forge_math::rng::Rng;

    /// The ellipses and points the foot point must get right: near-circular to flat (and b = 0),
    /// points on and near the axes, at the evolute's cusps, at and near the centre, on the curve,
    /// far off and out of plane; each with the whole turn, random arcs and tiny ones.
    fn ellipse_cases(r: &mut Rng) -> Vec<(Ellipse, Point3, Interval)> {
        let mut out = vec![];
        let mut shapes: Vec<(f64, f64)> = vec![];
        for k in [0, 3, 6, 9, 12, 15] {
            shapes.push((1.0, 1.0 - if k == 0 { 0.0 } else { 10f64.ipow(-k) }));
        }
        for k in 1..=8 {
            shapes.push((1.0, 10f64.ipow(-k)));
        }
        shapes.push((1.0, 0.0));
        for _ in 0..40 {
            let a = r.range(0.1, 50.0);
            shapes.push((a, a * r.range(0.01, 1.0)));
        }
        // semi-axes the other way round (a STEP ellipse's semi_axis_1 may be the shorter)
        let n = shapes.len();
        for i in 0..n {
            let (a, b) = shapes[i];
            if b > 0.0 && b < a {
                shapes.push((b, a));
            }
        }
        for (a, b) in shapes {
            let s = r.range(0.5, 200.0);
            let (a, b) = (a * s, b * s);
            let d = (a - b) * (a + b);
            let (cx, cy) = (d / a, if b > 0.0 { d / b } else { 0.0 });
            let mut pts = vec![
                (0.0, 0.0),
                (cx, 0.0),
                (0.0, cy),
                (cx * 0.5, 0.0),
                (0.0, cy * 0.5),
                (cx * 0.999, 1e-9 * a),
                (1e-12 * a, cy * 0.999),
                (1e-9 * a, 1e-9 * a),
                (a, 0.0),
                (0.0, b),
                (a * 1e6, b * 3e5),
                (-a * 0.3, b * 1e-13),
            ];
            for _ in 0..30 {
                let t = r.range(-4.0, 4.0);
                let k = r.range(0.0, 2.0);
                pts.push((k * a * dmath::cos(t), k * b * dmath::sin(t)));
            }
            for _ in 0..20 {
                pts.push((r.range(-2.0, 2.0) * a, r.range(-2.0, 2.0) * (b + a * 0.1)));
            }
            let frame = rand_frame(r);
            let e = Ellipse::new(frame, a, b);
            for (x, y) in pts {
                let z = if r.bool() { 0.0 } else { r.range(-2.0, 2.0) * a };
                let p = frame.point_to_world(Point3::new(x, y, z));
                let lo = r.range(-10.0, 10.0);
                for range in [
                    Interval::new(0.0, dmath::TAU),
                    Interval::new(lo, lo + r.range(0.0, dmath::TAU)),
                    Interval::new(lo, lo + 1e-6),
                    Interval::new(lo - 5.0, lo + 5.0),
                ] {
                    out.push((e, p, range));
                }
            }
        }
        out
    }

    /// The minor semi-axis longer than the major (a STEP ellipse may be so): the second review's
    /// repros, against fine sampling. The first assumed a >= b and gave t = 0 at 1.41421 for the
    /// first (the truth t = 0.3973 at 1.10136).
    #[test]
    fn an_ellipse_with_its_axes_the_other_way_finds_its_nearest_point() {
        let e = Ellipse::new(Frame::WORLD, 1.0, 2.0);
        for (x, y, want) in [(2.0, 1.0, 1.10136), (0.3, 0.5, 0.66153)] {
            let p = Point3::new(x, y, 0.0);
            let t = e.closest(p, Interval::new(0.0, dmath::TAU));
            let d = e.eval(t).distance(p);
            let best = (0..=200_000)
                .map(|i| e.eval(dmath::TAU * i as f64 / 200_000.0).distance(p))
                .fold(f64::INFINITY, f64::min);
            assert!(
                d <= best + 1e-12 && (d - want).abs() < 1e-5,
                "({x}, {y}): t {t} at {d}, best {best}"
            );
        }
    }

    /// Old against new: the exact foot point is never farther than the sampled one (to rounding),
    /// and where it is nearer, the sampler missed a minimum between two of its samples.
    #[test]
    fn the_ellipse_foot_point_is_never_farther_than_the_sampled_one() {
        let mut r = Rng::new(0x5eed_e111);
        let (mut n, mut nearer, mut tie, mut same) = (0, 0, 0, 0);
        for (e, p, range) in ellipse_cases(&mut r) {
            let full = if range.width() >= dmath::TAU {
                Interval::new(0.0, dmath::TAU)
            } else {
                range
            };
            let t_old = closest_numeric(&e, p, full);
            let t_new = e.closest(p, range);
            assert!(t_new >= full.lo && t_new <= full.hi, "{t_new} outside {full:?}");
            let (d_old, d_new) = (e.eval(t_old).distance(p), e.eval(t_new).distance(p));
            let tol = 1e-12 * (e.major.max(e.minor) + p.distance(e.frame.origin));
            assert!(
                d_new <= d_old + tol,
                "farther: {e:?} {p:?} {range:?}: new {t_new} at {d_new}, old {t_old} at {d_old}"
            );
            n += 1;
            // every case is one of: the same point (to 1e-9 in the angle, or the same point by
            // another angle: 0 and 2 pi), nearer (the sampler missed a minimum between its
            // samples), or a tie (another point at the same distance: a mirror image)
            let p_old = e.eval(t_old);
            if (t_new - t_old).abs() <= 1e-9 || e.eval(t_new).distance(p_old) <= tol {
                same += 1;
            } else if d_new < d_old - tol {
                nearer += 1;
            } else {
                assert!((d_new - d_old).abs() <= tol);
                tie += 1;
            }
        }
        assert_eq!(n, same + nearer + tie);
        eprintln!("{n} cases: the same point {same}, nearer than the sampler {nearer}, a tie {tie}");
    }

    /// Against brute force: no point of 20 001 on the range is nearer than the foot point. (Not
    /// with the old sampler switched back on: it misses minima between its samples.)
    #[cfg(not(feature = "ellipse-foot-sampled"))]
    #[test]
    fn no_sample_of_the_range_is_nearer_than_the_ellipse_foot_point() {
        let mut r = Rng::new(0xb2_07e);
        for (e, p, range) in ellipse_cases(&mut r).into_iter().step_by(7) {
            let full = if range.width() >= dmath::TAU {
                Interval::new(0.0, dmath::TAU)
            } else {
                range
            };
            let d = e.eval(e.closest(p, range)).distance(p);
            let tol = 1e-12 * (e.major.max(e.minor) + p.distance(e.frame.origin));
            for i in 0..=20_000 {
                let t = full.lo + full.width() * (i as f64 / 20_000.0);
                let s = e.eval(t).distance(p);
                assert!(d <= s + tol, "{e:?} {p:?} {range:?}: {d} but {s} at {t}");
            }
        }
    }

    /// Every angle `ellipse_square` gives is square to the point: the distance's slope there is
    /// zero to rounding (a cosine between the offset and the tangent of 1e-7, or the offset along
    /// the tangent within 1e-12 of the scale).
    #[test]
    fn every_ellipse_square_angle_is_square_to_the_point() {
        let mut r = Rng::new(0x5_90a7e);
        for (e, p, _) in ellipse_cases(&mut r) {
            let l = e.frame.point_to_local(p);
            let (ts, n) = ellipse_square_any(l.x, l.y, e.major, e.minor);
            for &t in &ts[..n] {
                let dv = e.deriv(t);
                let off = Point3::new(l.x, l.y, 0.0) - e.frame.point_to_local(dv.p);
                let tan = e.frame.vec_to_local(dv.d1);
                let (lo, lt) = (off.length(), tan.length());
                // the point on the curve, or the tangent vanishing (a flat ellipse's ends, where
                // sin pi rounds to 1e-16): square there whatever the direction
                let size = e.major.max(e.minor);
                if lo <= 1e-12 * size || lt <= 1e-12 * size {
                    continue;
                }
                // square to a cosine of 1e-7, or along the tangent within the tests' 1e-12 of the
                // scale (where roots merge, at the evolute's cusps, the angle is ill-conditioned:
                // the offset is then tiny and its direction rounding)
                let cos = off.dot(tan) / (lo * lt);
                let along = (off.dot(tan) / lt).abs();
                let tol = 1e-12 * (e.major.max(e.minor) + p.distance(e.frame.origin));
                assert!(
                    cos.abs() <= 1e-7 || along <= tol,
                    "{e:?} at ({}, {}): t {t}, cos {cos}, along {along}",
                    l.x,
                    l.y
                );
            }
        }
    }

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
