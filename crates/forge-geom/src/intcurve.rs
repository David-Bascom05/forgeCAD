//! Curves defined as "where surface A meets surface B", for pairs with no
//! closed form (a sphere and a cylinder meet in a quartic space curve).
//!
//! The curve keeps both surfaces and a dense chain of points lying on both.
//! The parameter runs along the chain (point i at t = i); every evaluation
//! starts from the chain and is pulled exactly onto both surfaces by Newton,
//! so the chain only guides — the curve is exact.

use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point3, Vec3, tol};

#[derive(Clone, Debug, PartialEq)]
pub struct IntCurve {
    pub a: Surface,
    pub b: Surface,
    /// Points on both surfaces, in order along the curve.
    pub pts: Vec<Point3>,
    /// The last point connects back to the first.
    pub closed: bool,
    /// Largest distance from a chain segment to the true curve between its ends.
    pub sag: f64,
}

/// Pulls `p` onto both surfaces: the minimum-norm Newton step for the two
/// implicit equations, repeated. None when the surfaces are tangent there.
pub fn refine(a: &Surface, b: &Surface, mut p: Point3) -> Option<Point3> {
    for _ in 0..40 {
        let (ga, gb) = (a.implicit(p), b.implicit(p));
        if ga.abs() <= 1e-3 * tol::LINEAR && gb.abs() <= 1e-3 * tol::LINEAR {
            return Some(p);
        }
        let (na, nb) = (a.gradient(p), b.gradient(p));
        // dp = J^T (J J^T)^-1 (-g) with J = [na; nb]
        let (aa, ab, bb) = (na.dot(na), na.dot(nb), nb.dot(nb));
        let det = aa * bb - ab * ab;
        if det <= 1e-12 * aa * bb {
            return None;
        }
        let la = (-ga * bb + gb * ab) / det;
        let lb = (-gb * aa + ga * ab) / det;
        p += na * la + nb * lb;
    }
    let ok = a.implicit(p).abs() <= tol::LINEAR && b.implicit(p).abs() <= tol::LINEAR;
    ok.then_some(p)
}

/// Unit direction of the curve at p (na x nb), or None where the surfaces are tangent.
pub fn direction(a: &Surface, b: &Surface, p: Point3) -> Option<Vec3> {
    a.gradient(p).cross(b.gradient(p)).normalize().map(|u| u.get())
}

impl IntCurve {
    pub fn segments(&self) -> usize {
        if self.closed {
            self.pts.len()
        } else {
            self.pts.len() - 1
        }
    }

    pub fn domain_hi(&self) -> f64 {
        self.segments() as f64
    }

    fn chain_point(&self, t: f64) -> Point3 {
        let n = self.pts.len();
        let seg = self.segments();
        let t = if self.closed {
            t.rem_euclid(seg as f64)
        } else {
            t.clamp(0.0, seg as f64)
        };
        let i = (t.floor() as usize).min(seg.saturating_sub(1));
        let f = t - i as f64;
        self.pts[i].lerp(self.pts[(i + 1) % n], f)
    }

    pub fn eval(&self, t: f64) -> Point3 {
        let guess = self.chain_point(t);
        refine(&self.a, &self.b, guess).unwrap_or(guess)
    }

    /// First and second derivative with respect to t, by central differences
    /// of exact points (the chain spacing sets the scale of t).
    pub fn derivs(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let h = 1e-4;
        let (p0, p, p1) = (self.eval(t - h), self.eval(t), self.eval(t + h));
        (p, (p1 - p0) / (2.0 * h), ((p1 - p) - (p - p0)) / (h * h))
    }

    /// Bound on |dP/dt| over the curve.
    pub fn speed_bound(&self) -> f64 {
        let n = self.pts.len();
        (0..self.segments())
            .map(|i| self.pts[i].distance(self.pts[(i + 1) % n]))
            .fold(0.0, f64::max)
            * 1.5
            + 2.0 * self.sag
    }

    pub fn closest(&self, p: Point3, range: Interval) -> f64 {
        let seg = self.segments();
        let (lo, hi) = (
            range.lo.max(if self.closed { f64::NEG_INFINITY } else { 0.0 }),
            range.hi,
        );
        let hi = if self.closed { hi } else { hi.min(seg as f64) };
        // nearest chain sample in range, then golden-section on the two neighbouring segments
        let mut best = (f64::INFINITY, lo);
        let start = lo.ceil() as i64;
        let end = hi.floor() as i64;
        for k in start..=end {
            let d = self.chain_point(k as f64).distance_sq(p);
            if d < best.0 {
                best = (d, k as f64);
            }
        }
        for t in [lo, hi] {
            let d = self.eval(t).distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        let (mut a, mut b) = ((best.1 - 1.0).max(lo), (best.1 + 1.0).min(hi));
        let g = 0.618_033_988_749_895;
        let f = |t: f64| self.eval(t).distance_sq(p);
        let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
        let (mut fc, mut fd) = (f(c), f(d));
        for _ in 0..60 {
            if fc < fd {
                b = d;
                d = c;
                fd = fc;
                c = b - g * (b - a);
                fc = f(c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + g * (b - a);
                fd = f(d);
            }
        }
        let t = 0.5 * (a + b);
        if f(t) <= best.0 { t } else { best.1 }
    }

    pub fn bbox(&self, range: Interval) -> Box3 {
        let mut b = Box3::from_points([self.eval(range.lo), self.eval(range.hi)]);
        let n = self.pts.len();
        let (s, e) = (range.lo.ceil() as i64, range.hi.floor() as i64);
        for k in s..=e {
            b.include(self.pts[(k.rem_euclid(n as i64)) as usize]);
        }
        b.grow(self.sag + tol::LINEAR)
    }

    pub fn transformed(&self, f: &Frame) -> IntCurve {
        use crate::surface::SurfaceGeom;
        IntCurve {
            a: self.a.transformed(f),
            b: self.b.transformed(f),
            pts: self.pts.iter().map(|&p| f.point_to_world(p)).collect(),
            closed: self.closed,
            sag: self.sag,
        }
    }
}

/// Every branch of the intersection of two surfaces inside `region`.
///
/// Seeds come from an octree over the region: a cell can hold no point of
/// the intersection if either surface's distance at its centre exceeds the
/// cell's half-diagonal (both implicits are 1-Lipschitz). Each seed is pulled
/// onto both surfaces and traced both ways along na x nb, with a Newton
/// correction at every step, until the branch leaves the region or closes.
/// Seeds already lying on a traced branch are skipped. None if the surfaces
/// are tangent somewhere the tracing needs to pass.
pub fn trace(a: &Surface, b: &Surface, region: Box3) -> Option<Vec<IntCurve>> {
    let size = region.diagonal().max(1e-6);
    let leaf = (size / 64.0).max(10.0 * tol::LINEAR);
    let step = leaf * 0.5;
    let mut seeds: Vec<Point3> = Vec::new();
    let mut stack = vec![(region.center(), 0.5 * size)];
    while let Some((c, r)) = stack.pop() {
        if a.implicit(c).abs() > r || b.implicit(c).abs() > r {
            continue;
        }
        if 2.0 * r <= leaf {
            seeds.push(c);
            continue;
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
    let grown = region.grow(step);
    let mut curves: Vec<IntCurve> = Vec::new();
    for s in seeds {
        let Some(p0) = refine(a, b, s) else { continue };
        if !grown.contains(p0) {
            continue;
        }
        if curves.iter().any(|c| near_chain(c, p0, step)) {
            continue;
        }
        let forward = march(a, b, p0, 1.0, step, &grown)?;
        if forward.1 {
            let mut pts = forward.0;
            pts.pop(); // the closing point repeats the first
            curves.push(finish(a, b, pts, true, step));
            continue;
        }
        let mut back = march(a, b, p0, -1.0, step, &grown)?.0;
        back.reverse();
        back.pop();
        back.extend(forward.0);
        if back.len() >= 2 {
            curves.push(finish(a, b, back, false, step));
        }
    }
    Some(curves)
}

fn near_chain(c: &IntCurve, p: Point3, step: f64) -> bool {
    c.pts.iter().any(|q| q.distance(p) <= 1.5 * step)
}

/// Points from p0 in direction `sign`, until leaving `bounds` (true) or
/// closing back on p0 (returns closed = true).
fn march(
    a: &Surface,
    b: &Surface,
    p0: Point3,
    sign: f64,
    step: f64,
    bounds: &Box3,
) -> Option<(Vec<Point3>, bool)> {
    let mut pts = vec![p0];
    let mut p = p0;
    let max = (4.0 * bounds.diagonal() / step) as usize + 16;
    for k in 0..max {
        let d = direction(a, b, p)? * sign;
        // a predictor along the tangent, then back onto both surfaces in the plane across it
        let guess = p + d * step;
        let q = refine(a, b, guess)?;
        if (q - p).dot(d) <= 0.0 {
            return None;
        }
        if k > 2 && q.distance(p0) <= 0.75 * step {
            pts.push(p0);
            return Some((pts, true));
        }
        pts.push(q);
        if !bounds.contains(q) {
            return Some((pts, false));
        }
        p = q;
    }
    None
}

fn finish(a: &Surface, b: &Surface, pts: Vec<Point3>, closed: bool, step: f64) -> IntCurve {
    // measured: how far each chord's midpoint sits from the curve
    let n = pts.len();
    let segs = if closed { n } else { n - 1 };
    let sag = (0..segs)
        .map(|i| {
            let m = pts[i].lerp(pts[(i + 1) % n], 0.5);
            refine(a, b, m).map_or(step, |q| q.distance(m))
        })
        .fold(0.0, f64::max);
    IntCurve {
        a: *a,
        b: *b,
        pts,
        closed,
        sag: 1.5 * sag + tol::LINEAR,
    }
}

impl crate::curve::CurveGeom for IntCurve {
    fn deriv(&self, t: f64) -> crate::curve::CurveDeriv {
        let (p, d1, d2) = self.derivs(t);
        crate::curve::CurveDeriv { p, d1, d2 }
    }
    fn eval(&self, t: f64) -> Point3 {
        IntCurve::eval(self, t)
    }
    fn domain(&self) -> crate::curve::Domain1 {
        let hi = self.domain_hi();
        crate::curve::Domain1 {
            lo: 0.0,
            hi,
            period: self.closed.then_some(hi),
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        IntCurve::closest(self, p, range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        IntCurve::bbox(self, range)
    }
    fn transformed(&self, f: &Frame) -> Self {
        IntCurve::transformed(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveGeom;
    use crate::surface::{Cone, Cylinder, Sphere, SurfaceGeom, Torus};
    use forge_math::{Frame, UnitVec3};

    fn on_both(c: &IntCurve) {
        for i in 0..=200 {
            let t = c.domain_hi() * i as f64 / 200.0;
            let p = CurveGeom::eval(c, t);
            assert!(
                c.a.distance(p) <= tol::LINEAR && c.b.distance(p) <= tol::LINEAR,
                "{t}"
            );
        }
    }

    #[test]
    fn sphere_through_cylinder_gives_two_loops() {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.5, 0.0, 1.0)),
            radius: 4.0,
        });
        let region = Box3::from_points([Point3::new(-6.0, -6.0, -6.0), Point3::new(6.0, 6.0, 8.0)]);
        let cs = trace(&cyl, &sph, region).unwrap();
        assert_eq!(cs.len(), 2, "{}", cs.len());
        for c in &cs {
            assert!(c.closed);
            on_both(c);
        }
    }

    #[test]
    fn crossing_cylinders_and_a_cone_with_a_torus() {
        let a = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let x = Frame::from_z(Point3::new(0.0, 0.5, 0.0), UnitVec3::X);
        let b = Surface::Cylinder(Cylinder {
            frame: x,
            radius: 1.2,
        });
        let region = Box3::from_points([Point3::new(-5.0, -5.0, -5.0), Point3::new(5.0, 5.0, 5.0)]);
        let cs = trace(&a, &b, region).unwrap();
        assert_eq!(cs.len(), 2);
        cs.iter().for_each(on_both);
        let cone = Surface::Cone(
            Cone::new(Frame::WORLD.with_origin(Point3::new(4.5, 0.0, -3.0)), 0.5, 0.4).unwrap(),
        );
        let torus = Surface::Torus(Torus::new(Frame::WORLD, 5.0, 1.0).unwrap());
        let region = Box3::from_points([Point3::new(-7.0, -7.0, -2.0), Point3::new(7.0, 7.0, 2.0)]);
        let cs = trace(&cone, &torus, region).unwrap();
        assert!(!cs.is_empty());
        cs.iter().for_each(on_both);
        let _ = cs[0].a.project(Point3::ORIGIN);
    }
}
