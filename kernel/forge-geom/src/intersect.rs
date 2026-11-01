//! Exact intersections between analytic shapes.
//!
//! Every result is classified: a `Crossing` passes through, a `Touching`
//! contact meets without crossing (a tangency). Tangency is decided by the
//! tolerance rule, not by exact arithmetic: a line that passes within
//! `tol::LINEAR` of a surface without leaving its tolerance band touches it,
//! even if the exact math says it misses or cuts through by a hair.

use crate::curve::{Circle, Curve, CurveGeom, Ellipse, Hyperbola, Line, Parabola};
use crate::surface::{Cone, Cylinder, Plane, Sphere, Surface, SurfaceGeom, Torus};
use forge_math::dmath;
use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, poly, tol};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Crossing,
    Touching,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub t: f64,
    pub point: Point3,
    pub contact: Contact,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LineSurface {
    /// Ordered by t.
    Hits(Vec<LineHit>),
    /// The whole line lies in the surface.
    OnSurface,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceSurface {
    Empty,
    /// The surfaces are the same surface.
    Coincident,
    /// Isolated points where the surfaces touch.
    Points(Vec<Point3>),
    Curves(Vec<(Curve, Contact)>),
    /// A pair whose intersection is not closed-form; the numeric marcher handles it.
    NotAnalytic,
}

pub fn line_surface(line: &Line, s: &Surface) -> LineSurface {
    match s {
        Surface::Plane(p) => line_plane(line, p),
        Surface::Cylinder(c) => line_cylinder(line, c),
        Surface::Cone(c) => line_cone(line, c),
        Surface::Sphere(sp) => line_sphere(line, sp),
        Surface::Torus(t) => line_torus(line, t),
        Surface::Procedural(pr) => line_procedural(line, s, pr),
    }
}

/// A line against a procedural surface: clipped to the surface's box, then subdivided.
fn line_procedural(line: &Line, s: &Surface, pr: &crate::surface::Proc) -> LineSurface {
    let d = pr.domain();
    let b = pr.bbox(forge_math::Box2::new(d.u.interval(), d.v.interval()));
    let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
    let (o, dir) = (line.origin, line.dir.get());
    for (oi, di, iv) in [(o.x, dir.x, b.x), (o.y, dir.y, b.y), (o.z, dir.z, b.z)] {
        if di.abs() < 1e-300 {
            if oi < iv.lo || oi > iv.hi {
                return LineSurface::Hits(Vec::new());
            }
            continue;
        }
        let (a, c) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        lo = lo.max(a.min(c));
        hi = hi.min(a.max(c));
    }
    if lo >= hi {
        return LineSurface::Hits(Vec::new());
    }
    let range = Interval::new(lo, hi);
    let found = crate::ruled::line_wall(line, range, s)
        .or_else(|| crate::spline::meet::line_extruded(line, range, s))
        .unwrap_or_else(|| curve_surface_subdivide(&Curve::Line(*line), range, s, None));
    match found {
        // the implicit holds more than the patch (a blend's: its balls whole), so only points
        // of the patch are hits
        CurveSurface::Hits(h) => LineSurface::Hits(
            h.into_iter()
                .filter(|h| s.eval(s.project(h.point)).distance(h.point) <= tol::LINEAR)
                .map(|h| LineHit {
                    t: h.t,
                    point: h.point,
                    contact: h.contact,
                })
                .collect(),
        ),
        CurveSurface::OnSurface | CurveSurface::Overlap => LineSurface::OnSurface,
    }
}

pub fn line_plane(line: &Line, p: &Plane) -> LineSurface {
    let denom = p.normal_vec().dot(line.dir);
    let d0 = p.signed_distance(line.origin);
    if denom.abs() <= tol::ANGULAR {
        return if d0.abs() <= tol::LINEAR {
            LineSurface::OnSurface
        } else {
            LineSurface::Hits(vec![])
        };
    }
    let t = -d0 / denom;
    LineSurface::Hits(vec![LineHit {
        t,
        point: line.eval(t),
        contact: Contact::Crossing,
    }])
}

/// Line origin and direction in a surface's local frame.
fn local_line(line: &Line, f: &Frame) -> (Vec3, Vec3) {
    (
        f.point_to_local(line.origin).to_vec(),
        f.vec_to_local(line.dir.get()),
    )
}

/// Parameter bound covering the model box from the line's origin.
fn model_span(line: &Line) -> f64 {
    line.origin.to_vec().length() + 4.0 * tol::MODEL_HALF_SIZE
}

pub fn line_cylinder(line: &Line, c: &Cylinder) -> LineSurface {
    let (o, d) = local_line(line, &c.frame);
    let coeffs = [
        o.x * o.x + o.y * o.y - c.radius * c.radius,
        2.0 * (o.x * d.x + o.y * d.y),
        d.x * d.x + d.y * d.y,
    ];
    let span = model_span(line);
    line_implicit(line, &Surface::Cylinder(*c), &coeffs, -span, span, |_| true)
}

pub fn line_sphere(line: &Line, s: &Sphere) -> LineSurface {
    let (o, d) = local_line(line, &s.frame);
    let coeffs = [o.length_sq() - s.radius * s.radius, 2.0 * o.dot(d), 1.0];
    let b = -o.dot(d);
    let span = s.radius + 1.0;
    line_implicit(line, &Surface::Sphere(*s), &coeffs, b - span, b + span, |_| true)
}

pub fn line_cone(line: &Line, c: &Cone) -> LineSurface {
    let (o, d) = local_line(line, &c.frame);
    let k = c.tan_a();
    let r0 = c.radius + k * o.z;
    let coeffs = [
        o.x * o.x + o.y * o.y - r0 * r0,
        2.0 * (o.x * d.x + o.y * d.y - k * r0 * d.z),
        d.x * d.x + d.y * d.y - k * k * d.z * d.z,
    ];
    let span = model_span(line);
    let apex_v = c.apex_v();
    // the implicit equation also holds on the mirror nappe below the apex
    let upper = move |t: f64| o.z + t * d.z >= apex_v - tol::LINEAR;
    line_implicit(line, &Surface::Cone(*c), &coeffs, -span, span, upper)
}

pub fn line_torus(line: &Line, t: &Torus) -> LineSurface {
    let (o, d) = local_line(line, &t.frame);
    let big2 = 4.0 * t.major * t.major;
    let s = o.dot(d);
    let a0 = o.length_sq() + t.major * t.major - t.minor * t.minor;
    let w0 = o.x * o.x + o.y * o.y;
    let w1 = 2.0 * (o.x * d.x + o.y * d.y);
    let w2 = d.x * d.x + d.y * d.y;
    // (|p|^2 + R^2 - r^2)^2 - 4R^2 (x^2 + y^2) with p = o + t d
    let coeffs = [
        a0 * a0 - big2 * w0,
        4.0 * s * a0 - big2 * w1,
        4.0 * s * s + 2.0 * a0 - big2 * w2,
        4.0 * s,
        1.0,
    ];
    let span = t.major.abs() + t.minor + 1.0;
    let surface = Surface::Torus(*t);
    // the quartic holds a spindle's other sheet too (apple for lemon), which is not on the surface
    let used = |h: f64| !t.is_spindle() || surface.distance(line.eval(h)) <= tol::LINEAR;
    line_implicit(line, &surface, &coeffs, -s - span, -s + span, used)
}

/// Shared by every surface with a polynomial implicit equation. `f(t)` is that
/// equation along the line. Crossings are its roots; touching contacts are its
/// local extremes where the line comes within LINEAR of the surface. A
/// touching contact absorbs the roots next to it (the line dips in and out
/// within tolerance there).
fn line_implicit(
    line: &Line,
    s: &Surface,
    f: &[f64],
    lo: f64,
    hi: f64,
    valid: impl Fn(f64) -> bool,
) -> LineSurface {
    let near = |t: f64| s.distance(line.eval(t)) <= tol::LINEAR;
    let scale = f.iter().fold(0.0f64, |m, a| m.max(a.abs()));
    let degenerate = f.iter().skip(1).all(|a| a.abs() <= 1e-12 * scale.max(1.0));
    if degenerate || scale == 0.0 {
        let probes: Vec<f64> = [0.0, 1.0, 100.0, -100.0, 1000.0, -1000.0]
            .into_iter()
            .filter(|&t| valid(t))
            .collect();
        if probes.len() >= 3 && probes.iter().all(|&t| near(t)) {
            return LineSurface::OnSurface;
        }
        if degenerate {
            return LineSurface::Hits(vec![]);
        }
    }

    let roots: Vec<f64> = poly::real_roots(f, lo, hi)
        .into_iter()
        .map(|r| r.x)
        .filter(|&t| valid(t))
        .collect();
    let crit: Vec<f64> = poly::real_roots(&poly::derivative(f), lo, hi)
        .into_iter()
        .map(|r| r.x)
        .collect();
    let touching: Vec<f64> = crit.iter().copied().filter(|&c| valid(c) && near(c)).collect();

    let mut hits: Vec<LineHit> = Vec::new();
    for &r in &roots {
        let absorbed = touching
            .iter()
            .any(|&c| !crit.iter().any(|&o| o != c && between(o, r, c)));
        if absorbed && touching.iter().any(|&c| near_between(&near, r, c)) {
            continue;
        }
        hits.push(LineHit {
            t: r,
            point: line.eval(r),
            contact: Contact::Crossing,
        });
    }
    for &c in &touching {
        hits.push(LineHit {
            t: c,
            point: line.eval(c),
            contact: Contact::Touching,
        });
    }
    hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    LineSurface::Hits(hits)
}

fn between(x: f64, a: f64, b: f64) -> bool {
    (a < x && x < b) || (b < x && x < a)
}

/// The segment from root r to extreme c stays within tolerance (checked at its midpoint and quarter points).
fn near_between(near: &impl Fn(f64) -> bool, r: f64, c: f64) -> bool {
    [0.25, 0.5, 0.75].iter().all(|&k| near(r + (c - r) * k))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveHit {
    pub t: f64,
    pub point: Point3,
    pub contact: Contact,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CurveSurface {
    /// Ordered by t.
    Hits(Vec<CurveHit>),
    /// The curve lies in the surface over the whole range.
    OnSurface,
    /// The curve runs within tolerance of the surface along part of the range
    /// (an overlap that needs the caller's coincidence handling).
    Overlap,
}

/// An upper bound on |C'(t)| over the range.
pub(crate) fn speed_bound(c: &Curve, range: Interval) -> f64 {
    match c {
        Curve::Line(_) => 1.0,
        Curve::Circle(k) => k.radius,
        Curve::Ellipse(e) => e.major,
        Curve::Parabola(p) => {
            let t = range.lo.abs().max(range.hi.abs());
            (t * t / (4.0 * p.focal * p.focal) + 1.0).sqrt()
        }
        Curve::Hyperbola(h) => (h.a * h.a + h.b * h.b).sqrt(),
        Curve::Intersection(c) => c.speed_bound(),
        Curve::Procedural(c) => c.speed_bound(range),
    }
}

/// Where a curve meets a surface over a finite parameter range.
///
/// The surface's implicit value is 1-Lipschitz and the curve's speed is
/// bounded, so `|g(t)|` can only reach zero within `|g(m)| / speed` of a
/// sample m: subdividing until every remaining piece is within tolerance
/// finds every crossing and every touch, with no initial guess. Pieces near
/// the surface are grouped; a group with an odd number of sign changes is a
/// crossing (bisected to full precision), an even number or none is a touch.
pub fn curve_surface(c: &Curve, range: Interval, s: &Surface) -> CurveSurface {
    curve_surface_in(c, range, s, || None)
}

/// `curve_surface`, the hits wanted only inside box `near` (others may or may not be found):
/// a subdivision skips the stretches of the curve that cannot reach it (`near` is asked only
/// then; a closed form needs no box).
pub fn curve_surface_in(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: impl FnOnce() -> Option<Box3>,
) -> CurveSurface {
    match c {
        Curve::Line(l) if !matches!(s, Surface::Procedural(_)) => return line_in_range(l, range, s),
        Curve::Circle(k) => {
            if let Some(r) = conic_quadric(&k.frame, k.radius, k.radius, range, s) {
                return r;
            }
        }
        Curve::Ellipse(e) => {
            if let Some(r) = conic_quadric(&e.frame, e.major, e.minor, range, s) {
                return r;
            }
        }
        Curve::Parabola(p) => {
            if let Some(r) =
                parabola_quadric(p, range, s).or_else(|| crate::ruled::parabola_wall(p, range, s))
            {
                return r;
            }
        }
        Curve::Line(l) => {
            if let Some(r) = crate::ruled::line_wall(l, range, s)
                .or_else(|| crate::spline::meet::line_extruded(l, range, s))
            {
                return r;
            }
        }
        Curve::Procedural(p) => {
            if p.is_spline() {
                if let Some(c) = p
                    .as_any()
                    .and_then(|a| a.downcast_ref::<crate::spline::SplineCurve>())
                    && let Some(r) = crate::spline::meet::spline_surface(c, range, s)
                {
                    return r;
                }
            } else if let Some(w) = p.as_any().and_then(|a| a.downcast_ref::<crate::wrap::Wrap>())
                && let Some(r) =
                    crate::wrap::wrap_surface(w, range, s).or_else(|| crate::ruled::wrap_wall(w, range, s))
            {
                return r;
            }
        }
        _ => {}
    }
    // the box is asked for at most once, by whichever needs it
    let (asked, near) = (std::cell::OnceCell::new(), std::cell::Cell::new(Some(near)));
    let near = || *asked.get_or_init(|| near.take().and_then(|f| f()));
    if let Some(r) = crate::screw::along::curve_along(c, range, s, &near) {
        return r;
    }
    curve_surface_subdivide(c, range, s, near())
}

fn line_in_range(l: &Line, range: Interval, s: &Surface) -> CurveSurface {
    match line_surface(l, s) {
        LineSurface::OnSurface => CurveSurface::OnSurface,
        LineSurface::Hits(h) => CurveSurface::Hits(
            h.into_iter()
                .filter(|h| range.contains(h.t))
                .map(|h| CurveHit {
                    t: h.t,
                    point: h.point,
                    contact: h.contact,
                })
                .collect(),
        ),
    }
}

/// (frame, A, b, c) of Q(p) = p.A.p + b.p + c.
type Quadric = (Frame, [[f64; 3]; 3], [f64; 3], f64);

/// A quadric surface as Q(p) = p.A.p + b.p + c in its frame's local
/// coordinates, or None for surfaces of higher degree.
fn quadric(s: &Surface) -> Option<Quadric> {
    match s {
        Surface::Plane(p) => Some((p.frame, [[0.0; 3]; 3], [0.0, 0.0, 1.0], 0.0)),
        Surface::Cylinder(c) => Some((
            c.frame,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 3]],
            [0.0; 3],
            -c.radius * c.radius,
        )),
        Surface::Sphere(sp) => Some((
            sp.frame,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            -sp.radius * sp.radius,
        )),
        Surface::Cone(c) => {
            // x^2 + y^2 - (r + k z)^2 (both nappes; the caller drops the mirror one)
            let k = c.tan_a();
            Some((
                c.frame,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -k * k]],
                [0.0, 0.0, -2.0 * c.radius * k],
                -c.radius * c.radius,
            ))
        }
        Surface::Torus(_) | Surface::Procedural(_) => None,
    }
}

/// Conic o + a cos t x + b sin t y against a quadric, exactly: Q along the
/// conic is a quadratic form in (cos t, sin t), which the half-angle
/// substitution u = tan(t/2) turns into a quartic in u.
fn conic_quadric(f: &Frame, a: f64, b: f64, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let (qf, m, lin, c0) = quadric(s)?;
    let o = qf.point_to_local(f.origin).to_vec();
    let ex0 = qf.vec_to_local(f.x() * a);
    let ey0 = qf.vec_to_local(f.y() * b);
    let mv = |v: Vec3| {
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    };
    let lv = Vec3::new(lin[0], lin[1], lin[2]);
    let curve = Curve::Ellipse(Ellipse {
        frame: *f,
        major: a,
        minor: b,
    });
    let probes = 9;
    let on = (0..probes).all(|i| {
        s.distance(curve.eval(range.lo + range.width() * i as f64 / (probes - 1) as f64)) <= tol::LINEAR
    });
    if on {
        return Some(CurveSurface::OnSurface);
    }
    // u = tan(tau / 2) is well conditioned only for |tau| <= pi/2, so solve
    // around t = 0 and around t = pi and keep each half's own roots
    let mut ts: Vec<(f64, Contact)> = Vec::new();
    let mut turns: Vec<f64> = Vec::new();
    // the implicit's gradient along the conic, so how far a value within LINEAR of the surface can be
    // from zero (times 4 for the 1 + u^2 squared multiplied through below)
    let lip = if matches!(s, Surface::Plane(_)) {
        1.0
    } else {
        let mmax = m.iter().flatten().fold(0.0f64, |a, x| a.max(x.abs()));
        6.0 * mmax * (o.length() + ex0.length() + ey0.length()) + lv.length()
    };
    for phi in [0.0, dmath::PI] {
        let (sp, cp) = dmath::sin_cos(phi);
        let ex = ex0 * cp + ey0 * sp;
        let ey = ey0 * cp - ex0 * sp;
        let (mo, mx, my) = (mv(o), mv(ex), mv(ey));
        // Q = k0 + k1 c + k2 s + k3 c^2 + k4 c s + k5 s^2, in tau
        let k0 = o.dot(mo) + lv.dot(o) + c0;
        let k1 = 2.0 * ex.dot(mo) + lv.dot(ex);
        let k2 = 2.0 * ey.dot(mo) + lv.dot(ey);
        let k3 = ex.dot(mx);
        let k4 = 2.0 * ex.dot(my);
        let k5 = ey.dot(my);
        if [k0, k1, k2, k3, k4, k5].iter().all(|v| *v == 0.0) {
            return Some(CurveSurface::OnSurface);
        }
        // c = (1 - u^2) / w, s = 2u / w, w = 1 + u^2; multiplied through by w^2
        let poly = [
            k0 + k1 + k3,
            2.0 * k2 + 2.0 * k4,
            2.0 * k0 - 2.0 * k3 + 4.0 * k5,
            2.0 * k2 - 2.0 * k4,
            k0 - k1 + k3,
        ];
        let (mark, slack) = (turns.len(), 4.0 * tol::LINEAR * lip);
        for r in poly::real_roots_turns(&poly, -1.0, 1.0, slack, &mut turns) {
            let contact = if r.touching {
                Contact::Touching
            } else {
                Contact::Crossing
            };
            ts.push((phi + 2.0 * dmath::atan(r.x), contact));
        }
        for t in &mut turns[mark..] {
            *t = phi + 2.0 * dmath::atan(*t);
        }
    }
    let cone_apex = match s {
        Surface::Cone(c) => Some((c.frame, c.apex_v())),
        _ => None,
    };
    let mut hits: Vec<CurveHit> = Vec::new();
    for (t0, contact) in ts {
        // bring into the range (the conic is periodic)
        let t = range.lo + dmath::wrap_tau(t0 - range.lo);
        if t > range.hi + 1e-12 {
            continue;
        }
        let t = t.min(range.hi);
        let p = curve.eval(t);
        if let Some((cf, apex_v)) = cone_apex
            && cf.point_to_local(p).z < apex_v - tol::LINEAR
        {
            continue;
        }
        if s.distance(p) > 100.0 * tol::LINEAR {
            continue; // a root lost to cancellation; the subdivision finds the truth
        }
        hits.push(CurveHit { t, point: p, contact });
    }
    // as for a line, a touch is where the value turns within tolerance of the surface: rounding
    // may keep it off zero, and then no root says so
    let near = |t: f64| s.distance(curve.eval(t)) <= tol::LINEAR;
    let apart = |x: f64, t: f64| dmath::wrap_tau(x - t + dmath::PI) - dmath::PI;
    for t0 in turns {
        let t = range.lo + dmath::wrap_tau(t0 - range.lo);
        if t > range.hi + 1e-12 {
            continue;
        }
        let t = t.min(range.hi);
        let absorb = ABSORB_HAIR_ROOTS.with(|k| k.get());
        if near(t) && (absorb || !hits.iter().any(|h| near_between(&near, t + apart(h.t, t), t))) {
            // the roots beside it that never leave the tolerance are that touch, as for a line
            if absorb {
                hits.retain(|h| !near_between(&near, t + apart(h.t, t), t));
            }
            hits.push(CurveHit {
                t,
                point: curve.eval(t),
                contact: Contact::Touching,
            });
        }
    }
    hits.sort_by(|x, y| x.t.total_cmp(&y.t));
    hits.dedup_by(|x, y| x.point.distance(y.point) <= tol::LINEAR);
    Some(CurveSurface::Hits(hits))
}

thread_local! {
    /// A conic's two roots a hair apart become one touch: a boolean's retry, when as two crossings
    /// they leave its face split crossed (a rim tangent to the edges of a flat it sits on).
    pub static ABSORB_HAIR_ROOTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A parabola on a quadric: Q(P(t)) is a quartic in t, P(t) = o + k t^2 x + t y.
fn parabola_quadric(par: &Parabola, range: Interval, s: &Surface) -> Option<CurveSurface> {
    if !(range.lo.is_finite() && range.hi.is_finite()) {
        return None;
    }
    let (qf, m, lin, c0) = quadric(s)?;
    let f = &par.frame;
    let k = 1.0 / (4.0 * par.focal);
    let (o, x, y) = (
        qf.point_to_local(f.origin).to_vec(),
        qf.vec_to_local(f.x().get()),
        qf.vec_to_local(f.y().get()),
    );
    let mv = |v: Vec3| {
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    };
    let lv = Vec3::new(lin[0], lin[1], lin[2]);
    let (mo, mx, my) = (mv(o), mv(x), mv(y));
    // coefficients: research/closed-forms/
    let poly = [
        o.dot(mo) + lv.dot(o) + c0,
        2.0 * y.dot(mo) + lv.dot(y),
        2.0 * k * x.dot(mo) + y.dot(my) + k * lv.dot(x),
        2.0 * k * x.dot(my),
        k * k * x.dot(mx),
    ];
    let curve = Curve::Parabola(*par);
    if (0..9).all(|i| s.distance(curve.eval(range.lo + range.width() * i as f64 / 8.0)) <= tol::LINEAR) {
        return Some(CurveSurface::OnSurface);
    }
    let mut hits = Vec::new();
    for r in poly::real_roots(&poly, range.lo, range.hi) {
        let p = curve.eval(r.x);
        if s.distance(p) > 100.0 * tol::LINEAR {
            // a root lost to cancellation: the subdivision finds the truth
            return None;
        }
        let contact = if r.touching {
            Contact::Touching
        } else {
            Contact::Crossing
        };
        hits.push(CurveHit {
            t: r.x,
            point: p,
            contact,
        });
    }
    hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    hits.dedup_by(|a, b| a.point.distance(b.point) <= tol::LINEAR);
    Some(CurveSurface::Hits(hits))
}

pub(crate) fn curve_surface_subdivide(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: Option<Box3>,
) -> CurveSurface {
    let g = |t: f64| s.implicit(c.eval(t));
    let lip = speed_bound(c, range) * (1.0 + 1e-9) + f64::MIN_POSITIVE;
    let probes = 9;
    if (0..probes).all(|i| g(range.lo + range.width() * i as f64 / (probes - 1) as f64).abs() <= tol::LINEAR)
    {
        return CurveSurface::OnSurface;
    }
    // pieces with their own speed bound, rightmost first so the leftmost pops first: a traced
    // curve's chain segments (each bounded by its own chord, those surely clear of the surface
    // dropped unevaluated), else the whole range
    let stack: Vec<(f64, f64, f64)> = match (c, s) {
        (Curve::Intersection(ic), s) if !matches!(s, Surface::Procedural(_)) => chain_pieces(ic, range, s),
        _ => vec![(range.lo, range.hi, lip)],
    };
    subdivide_pieces(c, range, s, near, stack)
}

/// The search over only these stretches of the range, in order: a closed form's narrow spots.
pub(crate) fn subdivide_stretches(
    c: &Curve,
    range: Interval,
    stretches: &[Interval],
    s: &Surface,
    near: Option<Box3>,
) -> CurveSurface {
    let lip = speed_bound(c, range) * (1.0 + 1e-9) + f64::MIN_POSITIVE;
    let stack = stretches.iter().rev().map(|i| (i.lo, i.hi, lip)).collect();
    subdivide_pieces(c, range, s, near, stack)
}

fn subdivide_pieces(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: Option<Box3>,
    mut stack: Vec<(f64, f64, f64)>,
) -> CurveSurface {
    let g = |t: f64| s.implicit(c.eval(t));
    let off = |p: Point3| {
        near.map_or(0.0, |b| {
            let d = |x: f64, i: Interval| (i.lo - x).max(x - i.hi).max(0.0);
            Vec3::new(d(p.x, b.x), d(p.y, b.y), d(p.z, b.z)).length()
        })
    };
    // a curve running beside the surface a little over the tolerance away is never pruned by
    // the Lipschitz bound until its pieces are that small: past a budget, a piece 1/4096 of the
    // range is kept only where the surface's sign changes across it
    let (mut budget, coarse) = (100_000usize, range.width() / 4096.0);
    let mut leaves: Vec<(f64, f64)> = Vec::new();
    while let Some((a, b, lip)) = stack.pop() {
        let m = 0.5 * (a + b);
        let p = c.eval(m);
        let reach = lip * 0.5 * (b - a) + tol::LINEAR;
        if off(p) > reach || s.implicit(p).abs() > reach {
            continue;
        }
        budget = budget.saturating_sub(1);
        if budget == 0 && b - a <= coarse && b - a > tol::LINEAR / lip {
            if (g(a) < 0.0) != (g(b) < 0.0) {
                leaves.push((a, b));
            }
            continue;
        }
        if b - a <= tol::LINEAR / lip {
            leaves.push((a, b));
            if leaves.len() > 20_000 {
                return CurveSurface::Overlap;
            }
            continue;
        }
        // right half first so the left half pops first: leaves come out in order
        stack.push((m, b, lip));
        stack.push((a, m, lip));
    }
    let mut hits = Vec::new();
    let mut i = 0;
    while i < leaves.len() {
        let (lo, mut hi) = leaves[i];
        let mut j = i + 1;
        while j < leaves.len() && leaves[j].0 <= hi {
            hi = leaves[j].1;
            j += 1;
        }
        // sign changes across the group's leaf boundaries
        let mut marks: Vec<f64> = vec![lo];
        marks.extend(leaves[i..j].iter().map(|l| l.1));
        let vals: Vec<f64> = marks.iter().map(|&t| g(t)).collect();
        let changes: Vec<usize> = (1..vals.len())
            .filter(|&k| (vals[k - 1] < 0.0) != (vals[k] < 0.0))
            .collect();
        if changes.len() % 2 == 1 {
            let k = changes[0];
            let t = bisect_sign(&g, marks[k - 1], marks[k]);
            hits.push(CurveHit {
                t,
                point: c.eval(t),
                contact: Contact::Crossing,
            });
        } else {
            let (k, v) = vals.iter().enumerate().fold((0, f64::INFINITY), |b, (k, v)| {
                if v.abs() < b.1 { (k, v.abs()) } else { b }
            });
            if v <= tol::LINEAR {
                let t = marks[k];
                hits.push(CurveHit {
                    t,
                    point: c.eval(t),
                    contact: Contact::Touching,
                });
            }
        }
        i = j;
    }
    CurveSurface::Hits(hits)
}

/// A traced curve's range as chain segments, each with the speed bound of its own stretch (as
/// IntCurve::speed_bound is for all of them), rightmost first. A segment's stretch of curve lies
/// within its chord and sag of either end and the surface's implicit is a distance, so a segment
/// whose farther end is clear of the surface by more than that (and the 1.5 tolerances a leaf
/// needs) holds no leaf and is left out unevaluated.
fn chain_pieces(ic: &crate::intcurve::IntCurve, range: Interval, s: &Surface) -> Vec<(f64, f64, f64)> {
    let (n, segs) = (ic.pts.len() as i64, ic.segments() as i64);
    let mut out = Vec::new();
    let mut k = range.lo.floor() as i64;
    while (k as f64) < range.hi {
        let (a, b) = (range.lo.max(k as f64), range.hi.min((k + 1) as f64));
        // the chain segment holding [k, k + 1] (an open curve is clamped at its ends)
        let i = if ic.closed {
            k.rem_euclid(segs)
        } else {
            k.clamp(0, segs - 1)
        };
        let (p, q) = (ic.pts[i as usize], ic.pts[((i + 1) % n) as usize]);
        let chord = p.distance(q);
        let clear = s.implicit(p).abs().max(s.implicit(q).abs()) - chord - ic.sag;
        if b > a && clear <= 2.0 * tol::LINEAR + 1e-6 {
            out.push((
                a,
                b,
                (1.5 * chord + 2.0 * ic.sag) * (1.0 + 1e-9) + f64::MIN_POSITIVE,
            ));
        }
        k += 1;
    }
    out.reverse();
    out
}

fn bisect_sign(g: &impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    // Illinois: secant steps, the kept end's value halved when an end stays twice. Over a leaf a
    // surface's value along a curve is all but straight, so two or three steps reach rounding.
    let (mut fa, mut fb) = (g(a), g(b));
    // an end on the surface is the crossing: a chain's sample that lies on it, say
    if fb.abs() <= 1e-13 {
        return b;
    }
    if fa.abs() <= 1e-13 {
        return a;
    }
    let neg_a = fa < 0.0;
    for _ in 0..100 {
        let x = if fa != fb {
            b - fb * (b - a) / (fb - fa)
        } else {
            0.5 * (a + b)
        };
        let x = if x > a && x < b { x } else { 0.5 * (a + b) };
        if x <= a || x >= b {
            break;
        }
        let fx = g(x);
        if fx.abs() <= 1e-13 {
            return x;
        }
        if (fx < 0.0) == neg_a {
            (a, fa) = (x, fx);
            fb *= 0.5;
        } else {
            (b, fb) = (x, fx);
            fa *= 0.5;
        }
    }
    0.5 * (a + b)
}

pub fn plane_surface(p: &Plane, s: &Surface) -> SurfaceSurface {
    match s {
        Surface::Plane(q) => plane_plane(p, q),
        Surface::Cylinder(c) => plane_cylinder(p, c),
        Surface::Cone(c) => plane_cone(p, c),
        Surface::Sphere(sp) => plane_sphere(p, sp),
        Surface::Torus(t) => plane_torus(p, t),
        Surface::Procedural(_) => SurfaceSurface::NotAnalytic,
    }
}

/// Whether two planes are one: `plane_plane` is Coincident, without making the line of two
/// that are not.
pub fn planes_coincident(p: &Plane, q: &Plane) -> bool {
    p.normal_vec().cross(q.normal_vec()).length() <= tol::ANGULAR
        && p.signed_distance(q.frame.origin).abs() <= tol::LINEAR
}

pub fn plane_plane(p: &Plane, q: &Plane) -> SurfaceSurface {
    let (n1, n2) = (p.normal_vec(), q.normal_vec());
    let u = n1.cross(n2);
    if u.length() <= tol::ANGULAR {
        return if p.signed_distance(q.frame.origin).abs() <= tol::LINEAR {
            SurfaceSurface::Coincident
        } else {
            SurfaceSurface::Empty
        };
    }
    let d1 = n1.dot(p.frame.origin.to_vec());
    let d2 = n2.dot(q.frame.origin.to_vec());
    let o = (n2.get().cross(u) * d1 + u.cross(n1.get()) * d2) / u.length_sq();
    let line = Line::new(
        Point3::new(o.x, o.y, o.z),
        u.normalize().expect("non-parallel planes"),
    );
    SurfaceSurface::Curves(vec![(Curve::Line(line), Contact::Crossing)])
}

/// Where the axis through `origin` along `axis` meets the plane, or None if parallel.
fn axis_hit(p: &Plane, origin: Point3, axis: UnitVec3) -> Option<Point3> {
    let denom = p.normal_vec().dot(axis);
    if denom.abs() <= tol::ANGULAR {
        return None;
    }
    Some(origin + axis * (-p.signed_distance(origin) / denom))
}

pub fn plane_cylinder(p: &Plane, c: &Cylinder) -> SurfaceSurface {
    let n = p.normal_vec();
    let a = c.frame.z();
    let r = c.radius;
    let cos = n.dot(a).abs();
    // Parallel when tilting by the residual angle cannot move the far end of a
    // model-sized line by LINEAR.
    if cos <= tol::ANGULAR {
        let d = p.signed_distance(c.frame.origin);
        let side = a.cross(n).normalize().expect("axis in plane direction");
        let foot = c.frame.origin - n * d;
        if d.abs() > r + tol::LINEAR {
            return SurfaceSurface::Empty;
        }
        if (d.abs() - r).abs() <= tol::LINEAR {
            return SurfaceSurface::Curves(vec![(Curve::Line(Line::new(foot, a)), Contact::Touching)]);
        }
        let h = (r * r - d * d).sqrt();
        return SurfaceSurface::Curves(vec![
            (Curve::Line(Line::new(foot - side * h, a)), Contact::Crossing),
            (Curve::Line(Line::new(foot + side * h, a)), Contact::Crossing),
        ]);
    }
    // A hair off parallel the ellipse is millions of radii long and straight anywhere a part
    // reaches: lines tangent to it beside the plane's origin (off by (t cos)^2 / r at t from there).
    if cos <= 1e-6 {
        let q = c.frame.origin + a * a.dot(p.frame.origin - c.frame.origin);
        let d = p.signed_distance(q);
        if d.abs() < 0.999 * r {
            let side = a.cross(n).normalize().expect("axis in plane direction");
            let foot = q - n * d;
            let h = (r * r - d * d).sqrt();
            let lines: Option<Vec<_>> = [-h, h]
                .into_iter()
                .map(|w| {
                    let x = foot + side * w;
                    let dir = n.cross((x - q) / r).normalize()?;
                    let dir = if dir.dot(a.get()) < 0.0 { -dir } else { dir };
                    Some((Curve::Line(Line::new(x, dir)), Contact::Crossing))
                })
                .collect();
            if let Some(lines) = lines {
                return SurfaceSurface::Curves(lines);
            }
        }
    }
    let center = axis_hit(p, c.frame.origin, a).expect("checked not parallel");
    // A circle when the ellipse's two axes differ by less than LINEAR.
    if r * (1.0 / cos - 1.0) <= tol::LINEAR {
        let frame = Frame::from_z(center, n);
        return SurfaceSurface::Curves(vec![(
            Curve::Circle(Circle { frame, radius: r }),
            Contact::Crossing,
        )]);
    }
    let minor_dir = a.cross(n).normalize().expect("not perpendicular");
    let major_dir = n.cross(minor_dir);
    let frame = Frame::from_zx(center, n, major_dir).expect("major axis lies in the plane");
    SurfaceSurface::Curves(vec![(
        Curve::Ellipse(Ellipse {
            frame,
            major: r / cos,
            minor: r,
        }),
        Contact::Crossing,
    )])
}

pub fn plane_sphere(p: &Plane, s: &Sphere) -> SurfaceSurface {
    let d = p.signed_distance(s.frame.origin);
    let r = s.radius;
    if d.abs() > r + tol::LINEAR {
        return SurfaceSurface::Empty;
    }
    let foot = s.frame.origin - p.normal_vec() * d;
    if (d.abs() - r).abs() <= tol::LINEAR {
        return SurfaceSurface::Points(vec![foot]);
    }
    let frame = Frame::from_z(foot, p.normal_vec());
    let radius = (r * r - d * d).sqrt();
    SurfaceSurface::Curves(vec![(Curve::Circle(Circle { frame, radius }), Contact::Crossing)])
}

pub fn plane_torus(p: &Plane, t: &Torus) -> SurfaceSurface {
    let n = p.normal_vec();
    let a = t.frame.z();
    let (big, r) = (t.major, t.minor);
    if n.cross(a).length() <= tol::ANGULAR {
        // perpendicular to the axis: circles of latitude
        let h = p.signed_distance(t.frame.origin);
        let center = t.frame.origin - n * h;
        let frame = Frame::from_z(center, a);
        if h.abs() > r + tol::LINEAR {
            return SurfaceSurface::Empty;
        }
        if (h.abs() - r).abs() <= tol::LINEAR {
            return SurfaceSurface::Curves(vec![(
                Curve::Circle(Circle { frame, radius: big }),
                Contact::Touching,
            )]);
        }
        let w = (r * r - h * h).sqrt();
        return SurfaceSurface::Curves(vec![
            (
                Curve::Circle(Circle {
                    frame,
                    radius: big - w,
                }),
                Contact::Crossing,
            ),
            (
                Curve::Circle(Circle {
                    frame,
                    radius: big + w,
                }),
                Contact::Crossing,
            ),
        ]);
    }
    if n.dot(a).abs() <= tol::ANGULAR && p.signed_distance(t.frame.origin).abs() <= tol::LINEAR {
        // contains the axis: two cross-sections of the tube
        let radial = a.cross(n).normalize().expect("axis not along normal");
        let circles = [-1.0, 1.0].map(|side| {
            let center = t.frame.origin + radial * (side * big);
            let frame = Frame::from_zx(center, n, radial.get()).expect("radial lies in the plane");
            (Curve::Circle(Circle { frame, radius: r }), Contact::Crossing)
        });
        return SurfaceSurface::Curves(circles.to_vec());
    }
    SurfaceSurface::NotAnalytic
}

/// A plane cuts the half-cone in a circle, ellipse, parabola or hyperbola
/// branch; through the apex it gives the apex point or one or two generators.
pub fn plane_cone(p: &Plane, c: &Cone) -> SurfaceSurface {
    let f = &c.frame;
    let n = f.vec_to_local(p.normal_vec().get());
    let n = n.normalize().expect("unit normal");
    let k = c.tan_a();
    let apex = Vec3::new(0.0, 0.0, c.apex_v());
    let plane_o = f.point_to_local(p.frame.origin).to_vec();
    let apex_dist = n.dot(apex - plane_o);
    let to_world = |v: Vec3| f.point_to_world(Point3::new(v.x, v.y, v.z));

    // Plane perpendicular to the axis: a circle of latitude.
    if n.get().x.abs() <= tol::ANGULAR && n.get().y.abs() <= tol::ANGULAR {
        let h = n.dot(plane_o) / n.get().z;
        let rho = c.radius_at(h);
        if rho < -tol::LINEAR {
            return SurfaceSurface::Empty;
        }
        let center = to_world(Vec3::new(0.0, 0.0, h));
        if rho <= tol::LINEAR {
            return SurfaceSurface::Points(vec![center]);
        }
        let frame = Frame::from_z(center, p.normal_vec());
        return SurfaceSurface::Curves(vec![(
            Curve::Circle(Circle { frame, radius: rho }),
            Contact::Crossing,
        )]);
    }

    // In-plane axes: e2 perpendicular to the cone axis, e1 along the axis's
    // shadow in the plane, oriented up the cone.
    let zl = Vec3::Z;
    let e2 = n.cross(zl).normalize().expect("plane not perpendicular to axis");
    let mut e1 = e2.cross(n);
    if e1.z < 0.0 {
        e1 = -e1;
    }
    let e1z = e1.z;

    if apex_dist.abs() <= tol::LINEAR {
        // Through the apex: generators where the in-plane direction makes the half-angle with the axis.
        let apex_w = to_world(apex);
        let cos_phi = c.cos_a() / e1z.max(f64::MIN_POSITIVE);
        if cos_phi > 1.0 + tol::ANGULAR {
            return SurfaceSurface::Points(vec![apex_w]);
        }
        if cos_phi >= 1.0 - tol::ANGULAR {
            let dir = f.unit_to_world(UnitVec3::new_unchecked(e1));
            return SurfaceSurface::Curves(vec![(Curve::Line(Line::new(apex_w, dir)), Contact::Touching)]);
        }
        let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
        let lines = [-1.0, 1.0].map(|side| {
            let d = (e1 * cos_phi + e2 * (side * sin_phi))
                .normalize()
                .expect("unit combination");
            (
                Curve::Line(Line::new(apex_w, f.unit_to_world(d))),
                Contact::Crossing,
            )
        });
        return SurfaceSurface::Curves(lines.to_vec());
    }

    // Origin of plane coordinates: the plane point nearest the apex.
    let p0 = apex - n * apex_dist;
    // Cone implicit x^2 + y^2 - (radius + k z)^2 on p0 + s e1 + t e2 (e2 has no z part):
    //   A s^2 + t^2 + D s + E t + F = 0
    let rk = c.radius + k * p0.z;
    let a = 1.0 - (1.0 + k * k) * e1z * e1z;
    let d = 2.0 * (p0.dot(e1) - p0.z * e1z) - 2.0 * rk * k * e1z;
    let e = 2.0 * p0.dot(e2.get());
    let fc = p0.length_sq() - p0.z * p0.z - rk * rk;
    let t_shift = -e / 2.0;
    let world_dir = |v: Vec3| f.unit_to_world(UnitVec3::new_unchecked(v));
    let plane_pt = |s: f64, t: f64| to_world(p0 + e1 * s + e2 * t);
    let normal_w = p.normal_vec();

    if a.abs() <= 1e-12 {
        // parabola: t'^2 + D s + F' = 0
        let f2 = fc - e * e / 4.0;
        if d.abs() <= f64::MIN_POSITIVE {
            return SurfaceSurface::Empty;
        }
        let s_vertex = -f2 / d;
        if p0.z + s_vertex * e1z < c.apex_v() - tol::LINEAR {
            return SurfaceSurface::Empty; // on the mirror nappe
        }
        let open = if d > 0.0 { -e1 } else { e1 };
        let frame = Frame::from_zx(plane_pt(s_vertex, t_shift), normal_w, world_dir(open).get())
            .expect("opening direction lies in the plane");
        return SurfaceSurface::Curves(vec![(
            Curve::Parabola(Parabola {
                frame,
                focal: d.abs() / 4.0,
            }),
            Contact::Crossing,
        )]);
    }

    let s_center = -d / (2.0 * a);
    let g = d * d / (4.0 * a) + e * e / 4.0 - fc;
    let center = plane_pt(s_center, t_shift);
    if a > 0.0 {
        // ellipse: A s'^2 + t'^2 = G
        if g < 0.0 {
            return SurfaceSurface::Empty;
        }
        if p0.z + s_center * e1z < c.apex_v() {
            return SurfaceSurface::Empty; // on the mirror nappe
        }
        let along_e1 = (g / a).sqrt();
        let along_e2 = g.sqrt();
        if along_e1.max(along_e2) <= tol::LINEAR {
            return SurfaceSurface::Points(vec![center]);
        }
        let (major, minor, xdir) = if along_e1 >= along_e2 {
            (along_e1, along_e2, e1)
        } else {
            (along_e2, along_e1, e2.get())
        };
        let frame = Frame::from_zx(center, normal_w, world_dir(xdir).get()).expect("axis lies in the plane");
        if major - minor <= tol::LINEAR {
            return SurfaceSurface::Curves(vec![(
                Curve::Circle(Circle {
                    frame,
                    radius: 0.5 * (major + minor),
                }),
                Contact::Crossing,
            )]);
        }
        return SurfaceSurface::Curves(vec![(
            Curve::Ellipse(Ellipse { frame, major, minor }),
            Contact::Crossing,
        )]);
    }
    // hyperbola: |A| s'^2 - t'^2 = -G, transverse axis along e1 (G < 0 off the apex)
    if g >= 0.0 {
        return SurfaceSurface::NotAnalytic;
    }
    let semi_a = (-g / -a).sqrt();
    let semi_b = (-g).sqrt();
    // keep the branch on the upper nappe
    let up_vertex_z = p0.z + (s_center + semi_a) * e1z;
    let xdir = if up_vertex_z >= c.apex_v() { e1 } else { -e1 };
    let frame = Frame::from_zx(center, normal_w, world_dir(xdir).get()).expect("axis lies in the plane");
    SurfaceSurface::Curves(vec![(
        Curve::Hyperbola(Hyperbola {
            frame,
            a: semi_a,
            b: semi_b,
        }),
        Contact::Crossing,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use forge_math::rng::Rng;
    use forge_math::{Point2, dmath};

    /// A spindle's quartic also holds its other sheet (apple for lemon): a line hits only the used one.
    #[test]
    fn a_line_hits_one_sheet_of_a_spindle() {
        let line = Line::new(Point3::new(-40.0, 0.0, 0.0), Vec3::X.normalize().unwrap());
        for (major, rho) in [(-5.0, 8.0), (5.0, 18.0)] {
            let t = Torus::spindle(Frame::WORLD, major, 13.0).unwrap();
            let LineSurface::Hits(h) = line_torus(&line, &t) else {
                panic!("hits expected");
            };
            let at: Vec<f64> = h.iter().map(|h| h.point.x).collect();
            assert_eq!(at.len(), 2, "{at:?}");
            assert!((at[0] + rho).abs() < 1e-9 && (at[1] - rho).abs() < 1e-9, "{at:?}");
        }
    }

    /// Rounding puts the plane a hair off the circle's top: a touch all the same.
    #[test]
    fn a_circle_within_tolerance_of_a_plane_touches_it() {
        let c = Curve::Circle(Circle {
            frame: Frame::WORLD,
            radius: 12.7,
        });
        let range = Interval::new(0.0, dmath::TAU);
        for off in [4e-15, 1e-9, 5e-6] {
            let f = Frame::from_z(Point3::new(0.0, 12.7 + off, 0.0), Vec3::Y.normalize().unwrap());
            let CurveSurface::Hits(h) = curve_surface(&c, range, &Surface::Plane(Plane::new(f))) else {
                panic!("hits expected");
            };
            assert_eq!(h.len(), 1, "{off}: {h:?}");
            assert_eq!(h[0].contact, Contact::Touching);
            assert!((h[0].t - dmath::PI / 2.0).abs() < 1e-3, "{off}: {h:?}");
        }
        let f = Frame::from_z(Point3::new(0.0, 12.7 + 1e-3, 0.0), Vec3::Y.normalize().unwrap());
        assert_eq!(
            curve_surface(&c, range, &Surface::Plane(Plane::new(f))),
            CurveSurface::Hits(vec![])
        );
    }

    fn hits(r: LineSurface) -> Vec<LineHit> {
        match r {
            LineSurface::Hits(h) => h,
            LineSurface::OnSurface => panic!("unexpected OnSurface"),
        }
    }

    #[test]
    fn a_crossing_inside_a_bracket() {
        // an end on the surface is returned exactly, at either end
        assert_eq!(bisect_sign(&|t: f64| t - 1.0, 0.5, 1.0), 1.0);
        assert_eq!(bisect_sign(&|t: f64| t - 0.5, 0.5, 1.0), 0.5);
        // straight, bent, shallow and steep, and a jump (a pipe's value across its medial axis)
        let bent = |t: f64| (t - 0.3) * (1.0 + 40.0 * t * t);
        let shallow = |t: f64| 1e-3 * (t - 0.3);
        let jump = |t: f64| if t < 0.3 { -1.0 } else { 1.0 };
        assert!(bent(bisect_sign(&bent, 0.0, 1.0)).abs() <= 1e-13);
        assert!(shallow(bisect_sign(&shallow, 0.0, 1.0)).abs() <= 1e-13);
        assert!((bisect_sign(&jump, 0.0, 1.0) - 0.3).abs() <= 1e-12);
    }

    #[test]
    fn line_hits_lie_on_both() {
        let mut r = Rng::new(300);
        for _ in 0..3_000 {
            for s in rand_surfaces(&mut r) {
                // a line through a known surface point, in a random direction
                let uv = rand_uv(&mut r, &s);
                let p = s.eval(uv);
                let line = Line::new(p - rand_unit(&mut r) * 0.0, rand_unit(&mut r));
                let line = Line::new(line.origin - line.dir * r.range(0.0, 20.0), line.dir);
                let found = hits(line_surface(&line, &s));
                // p is a hit, or lies inside the tolerance band of a touching contact
                let band = |h: &LineHit| {
                    h.contact == Contact::Touching
                        && (0..=8).all(|k| s.distance(h.point.lerp(p, k as f64 / 8.0)) <= 1.01 * tol::LINEAR)
                };
                assert!(
                    found.iter().any(|h| h.point.distance(p) < 1e-6 || band(h)),
                    "{s:?} {line:?}: {found:?} missing {p:?}"
                );
                for h in &found {
                    let tol = if h.contact == Contact::Touching {
                        1.01 * tol::LINEAR
                    } else {
                        1e-6
                    };
                    assert!(s.distance(h.point) < tol, "{s:?} {h:?}");
                    assert!(line.eval(h.t).distance(h.point) == 0.0);
                }
            }
        }
    }

    #[test]
    fn tangent_lines_touch() {
        let mut r = Rng::new(301);
        for _ in 0..3_000 {
            for s in rand_surfaces(&mut r) {
                let uv = rand_uv(&mut r, &s);
                let d = s.deriv(uv);
                // Tangent along the direction of zero normal curvature fails for saddles;
                // pick convex spots: sphere anywhere, cylinder/cone along u, torus outer half along u.
                let dir = match s {
                    Surface::Plane(_) => continue,
                    Surface::Torus(_) if dmath::cos(uv.y) < 0.3 => continue,
                    Surface::Sphere(_) => {
                        let ang = r.range(0.0, dmath::TAU);
                        let (sn, cs) = dmath::sin_cos(ang);
                        (d.du.normalize().unwrap() * cs + d.dv.normalize().unwrap() * sn)
                            .normalize()
                            .unwrap()
                    }
                    _ => d.du.normalize().unwrap(),
                };
                let line = Line::new(d.p - dir * r.range(1.0, 10.0), dir);
                let found = hits(line_surface(&line, &s));
                let near: Vec<_> = found.iter().filter(|h| h.point.distance(d.p) < 1e-3).collect();
                assert_eq!(near.len(), 1, "{s:?} {line:?} {uv:?}: {found:?}");
                assert_eq!(near[0].contact, Contact::Touching, "{s:?}: {found:?}");
                assert!(near[0].point.distance(d.p) < 1e-6);
            }
        }
    }

    #[test]
    fn line_counts_match_sign_changes() {
        // Brute force: sample the line densely, count crossings of the surface by
        // the sign of (point - nearest).normal; compare with the crossing hits.
        let mut r = Rng::new(302);
        for _ in 0..400 {
            for s in rand_surfaces(&mut r) {
                if matches!(s, Surface::Cone(_)) {
                    continue; // signed side is ill-defined across the apex
                }
                let line = Line::new(rand_point(&mut r, 40.0), rand_unit(&mut r));
                let found = match line_surface(&line, &s) {
                    LineSurface::Hits(h) => h,
                    LineSurface::OnSurface => continue,
                };
                let side = |t: f64| {
                    let q = line.eval(t);
                    let uv = s.project(q);
                    let n = s.normal(uv).unwrap();
                    n.dot(q - s.eval(uv))
                };
                let (lo, hi) = (-200.0, 200.0);
                let n = 40_000;
                let mut count = 0;
                let mut prev = side(lo);
                for i in 1..=n {
                    let t = lo + (hi - lo) * i as f64 / n as f64;
                    let cur = side(t);
                    if (prev < 0.0) != (cur < 0.0) && (prev.abs() > 1e-6 || cur.abs() > 1e-6) {
                        count += 1;
                    }
                    prev = cur;
                }
                let crossings = found
                    .iter()
                    .filter(|h| h.contact == Contact::Crossing && h.t > lo && h.t < hi)
                    .count();
                assert_eq!(crossings, count, "{s:?} {line:?} {found:?}");
            }
        }
    }

    #[test]
    fn line_in_surface() {
        let p = Plane::new(Frame::WORLD);
        let l = Line::new(Point3::new(1.0, 2.0, 0.0), UnitVec3::X);
        assert_eq!(line_plane(&l, &p), LineSurface::OnSurface);
        let c = Cylinder {
            frame: Frame::WORLD,
            radius: 3.0,
        };
        let l = Line::new(Point3::new(3.0, 0.0, 5.0), UnitVec3::Z);
        assert_eq!(line_cylinder(&l, &c), LineSurface::OnSurface);
        let cone = Cone::new(Frame::WORLD, 2.0, 0.5).unwrap();
        let g = cone.deriv(Point2::new(1.0, 3.0));
        let l = Line::new(g.p, g.dv.normalize().unwrap());
        assert_eq!(line_cone(&l, &cone), LineSurface::OnSurface);
    }

    fn check_curves(a: &Surface, b: &Surface, res: &SurfaceSurface) {
        if let SurfaceSurface::Curves(cs) = res {
            for (c, contact) in cs {
                let tol = if *contact == Contact::Touching {
                    1.01 * tol::LINEAR
                } else {
                    1e-6
                };
                // generators through a cone's apex are rays starting there
                let ray = matches!((c, b), (Curve::Line(_), Surface::Cone(_)));
                let range = if ray {
                    Interval::new(0.0, 30.0)
                } else {
                    c.full_range(30.0)
                };
                for i in 0..=50 {
                    let t = range.lo + range.width() * i as f64 / 50.0;
                    let q = c.eval(t);
                    assert!(a.distance(q) < tol, "{a:?} {b:?} {c:?} t={t}: {}", a.distance(q));
                    assert!(b.distance(q) < tol, "{a:?} {b:?} {c:?} t={t}: {}", b.distance(q));
                }
            }
        }
        if let SurfaceSurface::Points(ps) = res {
            for q in ps {
                assert!(a.distance(*q) < 1e-6 && b.distance(*q) < 1e-6);
            }
        }
    }

    #[test]
    fn plane_sections_lie_on_both() {
        let mut r = Rng::new(303);
        let mut kinds = std::collections::BTreeMap::new();
        for i in 0..4_000 {
            for s in rand_surfaces(&mut r) {
                // planes through a surface point so most cases intersect; every
                // eighth plane is aligned with the surface frame to hit special cases
                let uv = rand_uv(&mut r, &s);
                let q = s.eval(uv);
                let sf = match &s {
                    Surface::Plane(p) => p.frame,
                    Surface::Cylinder(c) => c.frame,
                    Surface::Cone(c) => c.frame,
                    Surface::Sphere(c) => c.frame,
                    Surface::Torus(c) => c.frame,
                    Surface::Procedural(_) => continue,
                };
                let n = match i % 8 {
                    0 => sf.z(),
                    1 => sf.x(),
                    _ => rand_unit(&mut r),
                };
                let plane = Plane::new(Frame::from_z(q, n));
                let res = plane_surface(&plane, &s);
                let key = match &res {
                    SurfaceSurface::Curves(cs) => cs
                        .iter()
                        .map(|(c, _)| match c {
                            Curve::Line(_) => "line",
                            Curve::Circle(_) => "circle",
                            Curve::Ellipse(_) => "ellipse",
                            Curve::Parabola(_) => "parabola",
                            Curve::Hyperbola(_) => "hyperbola",
                            Curve::Intersection(_) => "traced",
                            Curve::Procedural(_) => "procedural",
                        })
                        .collect::<Vec<_>>()
                        .join("+"),
                    other => format!("{other:?}").chars().take(12).collect(),
                };
                *kinds.entry(key).or_insert(0) += 1;
                check_curves(&Surface::Plane(plane), &s, &res);
                if let SurfaceSurface::Curves(cs) = &res {
                    // q itself must be on one of the curves;
                    // a touching curve may sit up to sqrt(2 R LINEAR) beside the exact contact
                    let on_some = cs.iter().any(|(c, contact)| {
                        let t = c.closest(q, c.full_range(1e4));
                        let tol = if *contact == Contact::Touching { 0.05 } else { 1e-6 };
                        c.eval(t).distance(q) < tol
                    });
                    assert!(on_some, "{s:?} {plane:?} {res:?}");
                }
            }
        }
        for k in ["ellipse", "circle", "hyperbola", "line", "circle+circle"] {
            assert!(kinds.contains_key(k), "never produced {k}: {kinds:?}");
        }
    }

    #[test]
    fn cone_special_sections() {
        let cone = Cone::new(Frame::WORLD, 1.0, dmath::FRAC_PI_2 / 2.0).unwrap(); // 45 degrees, apex at z = -1
        // plane parallel to a generator: parabola
        let n = Vec3::new(1.0, 0.0, -1.0).normalize().unwrap();
        let plane = Plane::new(Frame::from_z(Point3::new(0.0, 0.0, 0.0), n));
        let res = plane_cone(&plane, &cone);
        assert!(
            matches!(&res, SurfaceSurface::Curves(c) if matches!(c[0].0, Curve::Parabola(_))),
            "{res:?}"
        );
        check_curves(&Surface::Plane(plane), &Surface::Cone(cone), &res);
        // the same plane shifted so it only meets the mirror nappe: nothing
        let plane = Plane::new(Frame::from_z(Point3::new(2.0, 0.0, 0.0), n));
        assert_eq!(plane_cone(&plane, &cone), SurfaceSurface::Empty);
        // vertical plane through the apex: two generators
        let plane = Plane::new(Frame::from_z(Point3::new(0.0, 0.0, -1.0), UnitVec3::Y));
        let res = plane_cone(&plane, &cone);
        assert!(
            matches!(&res, SurfaceSurface::Curves(c) if c.len() == 2),
            "{res:?}"
        );
        check_curves(&Surface::Plane(plane), &Surface::Cone(cone), &res);
        // horizontal plane below the apex: nothing
        let plane = Plane::new(Frame::from_z(Point3::new(0.0, 0.0, -3.0), UnitVec3::Z));
        assert_eq!(plane_cone(&plane, &cone), SurfaceSurface::Empty);
        // flat plane through the apex: the apex point
        let plane = Plane::new(Frame::from_z(Point3::new(0.0, 0.0, -1.0), UnitVec3::Z));
        assert!(matches!(plane_cone(&plane, &cone), SurfaceSurface::Points(_)));
        // a hyperbola keeps only the upper branch
        let plane = Plane::new(Frame::from_z(Point3::new(0.5, 0.0, 0.0), UnitVec3::X));
        let res = plane_cone(&plane, &cone);
        if let SurfaceSurface::Curves(c) = &res {
            let Curve::Hyperbola(h) = c[0].0 else {
                panic!("{res:?}")
            };
            for t in [-3.0, 0.0, 3.0] {
                assert!(h.eval(t).z >= -1.0);
            }
        } else {
            panic!("{res:?}");
        }
    }

    #[test]
    fn plane_cases() {
        let a = Plane::new(Frame::WORLD);
        let b = Plane::new(Frame::from_z(Point3::new(0.0, 0.0, 5.0), UnitVec3::Z));
        assert_eq!(plane_plane(&a, &b), SurfaceSurface::Empty);
        assert_eq!(plane_plane(&a, &a), SurfaceSurface::Coincident);
        let s = Sphere {
            frame: Frame::from_z(Point3::new(0.0, 0.0, 2.0), UnitVec3::Z),
            radius: 2.0,
        };
        assert!(matches!(plane_sphere(&a, &s), SurfaceSurface::Points(_)));
        let c = Cylinder {
            frame: Frame::from_z(Point3::new(0.0, 0.0, 2.0), UnitVec3::X),
            radius: 2.0,
        };
        let res = plane_cylinder(&a, &c);
        assert!(
            matches!(&res, SurfaceSurface::Curves(v) if v.len() == 1 && v[0].1 == Contact::Touching),
            "{res:?}"
        );
        let _ = Interval::point(0.0);
    }

    #[test]
    fn curve_surface_matches_brute_force() {
        let mut r = Rng::new(310);
        let mut total = 0;
        for _ in 0..100 {
            let f = rand_frame(&mut r);
            let curves = [
                Curve::Circle(Circle {
                    frame: f,
                    radius: r.range(1.0, 30.0),
                }),
                Curve::Ellipse(Ellipse {
                    frame: f,
                    major: r.range(10.0, 30.0),
                    minor: r.range(1.0, 9.0),
                }),
                Curve::Line(Line::new(rand_point(&mut r, 30.0), rand_unit(&mut r))),
            ];
            for s in rand_surfaces(&mut r) {
                for c in &curves {
                    let range = if matches!(c, Curve::Line(_)) {
                        Interval::new(-60.0, 60.0)
                    } else {
                        Interval::new(0.0, dmath::TAU)
                    };
                    let CurveSurface::Hits(hits) = curve_surface(c, range, &s) else {
                        continue;
                    };
                    let g = |t: f64| s.implicit(c.eval(t));
                    let n = 20_000;
                    let mut changes = 0;
                    let mut prev = g(range.lo);
                    for k in 1..=n {
                        let cur = g(range.lo + range.width() * k as f64 / n as f64);
                        if (prev < 0.0) != (cur < 0.0) && prev.abs().max(cur.abs()) > 2.0 * tol::LINEAR {
                            changes += 1;
                        }
                        prev = cur;
                    }
                    let crossings = hits.iter().filter(|h| h.contact == Contact::Crossing).count();
                    assert_eq!(crossings, changes, "{c:?} {s:?} {hits:?}");
                    for h in &hits {
                        assert!(s.distance(h.point) <= 1.01 * tol::LINEAR, "{h:?} {s:?}");
                    }
                    total += hits.len();
                }
            }
        }
        assert!(total > 300, "too few hits exercised: {total}");
    }

    #[test]
    fn planes_coincident_is_plane_plane_coincident() {
        // planes a hair apart in angle or offset, either side of the tolerances
        let mut r = Rng::new(312);
        let mut both = [0, 0];
        for _ in 0..2000 {
            let p = Plane::new(rand_frame(&mut r));
            let tilt = [0.0, 0.5, 2.0, 1e3][r.below(4)] * tol::ANGULAR;
            let shift = [0.0, 0.5, 2.0, 1e3][r.below(4)] * tol::LINEAR;
            let n = (p.normal_vec().get() + p.frame.x().get() * tilt)
                .normalize()
                .unwrap();
            let q = Plane::new(Frame::from_z(p.frame.origin + p.normal_vec().get() * shift, n));
            let one = planes_coincident(&p, &q);
            assert_eq!(
                one,
                plane_plane(&p, &q) == SurfaceSurface::Coincident,
                "{p:?} {q:?}"
            );
            both[one as usize] += 1;
        }
        assert!(both[0] > 100 && both[1] > 100, "{both:?}");
    }

    #[test]
    fn closed_forms_never_ask_for_the_box() {
        // a boolean's trim asks for its edge's box only for a subdivision: building it per edge
        // for lines and circles cost coplanar 0.8M instructions
        let mut r = Rng::new(311);
        for _ in 0..50 {
            let f = rand_frame(&mut r);
            let circle = Curve::Circle(Circle {
                frame: f,
                radius: r.range(1.0, 30.0),
            });
            let line = Curve::Line(Line::new(rand_point(&mut r, 30.0), rand_unit(&mut r)));
            for s in rand_surfaces(&mut r)
                .into_iter()
                .filter(|s| !matches!(s, Surface::Torus(_)))
            {
                for (c, range) in [
                    (&circle, Interval::new(0.0, dmath::TAU)),
                    (&line, Interval::new(-60.0, 60.0)),
                ] {
                    let asked = std::cell::Cell::new(false);
                    let got = curve_surface_in(c, range, &s, || {
                        asked.set(true);
                        None
                    });
                    assert!(!asked.get(), "{c:?} {s:?}");
                    assert_eq!(got, curve_surface(c, range, &s));
                }
            }
        }
    }

    #[test]
    fn curve_on_surface() {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 3.0,
        });
        let c = Curve::Circle(Circle {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 5.0)),
            radius: 3.0,
        });
        assert_eq!(
            curve_surface(&c, Interval::new(0.0, dmath::TAU), &cyl),
            CurveSurface::OnSurface
        );
    }

    #[test]
    fn conic_crossing_next_to_half_turn() {
        // a crossing a hair before t = pi, where tan(t/2) runs off to infinity
        let c = Curve::Circle(Circle {
            frame: Frame::WORLD,
            radius: 5.0,
        });
        for &t0 in &[
            dmath::PI - 1e-7,
            dmath::PI + 1e-9,
            dmath::PI,
            1e-8,
            dmath::FRAC_PI_2,
        ] {
            let d = c.deriv(t0);
            let plane = Surface::Plane(Plane::new(Frame::from_z(d.p, d.d1.normalize().unwrap())));
            let CurveSurface::Hits(h) = curve_surface(&c, Interval::new(0.0, dmath::TAU), &plane) else {
                panic!()
            };
            assert!(h.iter().any(|h| h.point.distance(d.p) < 1e-9), "t0 {t0}: {h:?}");
        }
    }
}
