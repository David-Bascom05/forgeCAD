//! The used sheet of a spindle torus in a surface-surface answer: an equation in the torus's own
//! terms holds the other sheet too (apple for lemon). With S = |q|^2 + R^2 - r^2 (q in the torus
//! frame) a point of the quartic is on the used sheet exactly when R S > 0, and the sheets meet only
//! at the two apices, so a branch that misses both is on one sheet all along and one point of it
//! decides (research/spindle-priorart: sheet.py proves these).
use crate::curve::CurveGeom;
use crate::intersect::SurfaceSurface;
use crate::surface::SurfaceGeom;
use crate::{Surface, Torus};
use forge_math::{Point3, tol};

/// R S at `p`: positive on the used sheet, negative on the other, 0 at an apex.
pub fn side(t: &Torus, p: Point3) -> f64 {
    let q = t.frame.point_to_local(p).to_vec();
    t.major * (q.dot(q) + t.major * t.major - t.minor * t.minor)
}

/// A spindle's two apices, where its sheets meet on the axis.
pub fn apices(t: &Torus) -> [Point3; 2] {
    let h = (t.minor * t.minor - t.major * t.major).max(0.0).sqrt();
    let z = t.frame.z().get();
    [t.frame.origin + z * h, t.frame.origin - z * h]
}

/// `r`, the answer for `t` and `other`, kept to `t`'s used sheet when `t` is a spindle: NotAnalytic
/// when `other` passes through an apex (a branch may change sheets there, and curves cannot end at
/// an apex vertex yet; the search goes on to the tracer unless the apex is in its region) or a curve
/// is not one that can be sampled whole.
pub fn used(t: &Torus, other: &Surface, r: SurfaceSurface) -> SurfaceSurface {
    if !t.is_spindle() || matches!(r, SurfaceSurface::Empty | SurfaceSurface::NotAnalytic) {
        return r;
    }
    if apices(t).iter().any(|&a| other.distance(a) <= tol::LINEAR) {
        return SurfaceSurface::NotAnalytic;
    }
    match r {
        SurfaceSurface::Points(ps) => {
            let ps: Vec<Point3> = ps.into_iter().filter(|&p| side(t, p) > 0.0).collect();
            if ps.is_empty() {
                SurfaceSurface::Empty
            } else {
                SurfaceSurface::Points(ps)
            }
        }
        SurfaceSurface::Curves(cs) => {
            let mut kept = Vec::new();
            for (c, contact) in cs {
                let d = c.domain();
                if !(d.lo.is_finite() && d.hi.is_finite()) {
                    return SurfaceSurface::NotAnalytic;
                }
                // the point farthest from the axis, where |S| = 2 |R| rho leaves the most margin
                let far = (0..=64)
                    .map(|k| c.eval(d.lo + (d.hi - d.lo) * k as f64 / 64.0))
                    .max_by(|p, q| axis_distance(t, *p).total_cmp(&axis_distance(t, *q)))
                    .expect("samples");
                if side(t, far) > 0.0 {
                    kept.push((c, contact));
                }
            }
            if kept.is_empty() {
                SurfaceSurface::Empty
            } else {
                SurfaceSurface::Curves(kept)
            }
        }
        r => r,
    }
}

/// `used` for whichever of `a`, `b` is a spindle torus (both, if both are).
pub fn used_by(a: &Surface, b: &Surface, mut r: SurfaceSurface) -> SurfaceSurface {
    if let Surface::Torus(t) = a {
        r = used(t, b, r);
    }
    if let Surface::Torus(t) = b {
        r = used(t, a, r);
    }
    r
}

/// Whether either is a spindle torus with the other through an apex inside `region`: the one case the
/// tracer may not take. Its implicit, sqrt((rho - R)^2 + z^2) - r with rho not negative, vanishes on the
/// used sheet only, so away from an apex it stays there (research/spindle-priorart).
pub fn apex_met(a: &Surface, b: &Surface, region: &forge_math::Box3) -> bool {
    let met = |s: &Surface, o: &Surface| matches!(s, Surface::Torus(t) if t.is_spindle() && apices(t).iter().any(|&p| region.grow(tol::LINEAR).contains(p) && o.distance(p) <= tol::LINEAR));
    met(a, b) || met(b, a)
}

/// Whether either is a spindle torus.
pub fn spindle_in(a: &Surface, b: &Surface) -> bool {
    [a, b]
        .iter()
        .any(|s| matches!(s, Surface::Torus(t) if t.is_spindle()))
}

fn axis_distance(t: &Torus, p: Point3) -> f64 {
    let q = t.frame.point_to_local(p);
    q.x * q.x + q.y * q.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cylinder, Plane};
    use forge_math::{Box3, Frame, UnitVec3};

    /// Every point of every curve kept for a spindle is on its used sheet (or an apex).
    #[test]
    fn what_is_kept_is_on_the_used_sheet() {
        let region = Box3::from_point(Point3::ORIGIN).grow(30.0);
        for major in [-5.0, 5.0, -12.0, 0.5] {
            let t = Torus::spindle(Frame::WORLD, major, 13.0).expect("a spindle");
            let s = Surface::Torus(t);
            let others = [
                Surface::Plane(Plane::new(Frame::from_z(Point3::new(0.0, 3.0, 0.0), UnitVec3::Y))),
                Surface::Plane(Plane::new(Frame::from_z(
                    Point3::new(0.0, 0.0, -6.0),
                    UnitVec3::Z,
                ))),
                Surface::Cylinder(Cylinder {
                    frame: Frame::from_z(Point3::new(0.0, 3.0, 1.0), UnitVec3::X),
                    radius: 2.0,
                }),
            ];
            for o in &others {
                let mut r = crate::meet::intersect(o, &s, region);
                if let Surface::Plane(p) = o {
                    r = r.or(Some(crate::intersect::plane_torus(p, &t)));
                }
                let Some(r) = r else { continue };
                let SurfaceSurface::Curves(cs) = used_by(o, &s, r) else {
                    continue;
                };
                for (c, _) in cs {
                    let d = c.domain();
                    for k in 0..=200 {
                        let p = c.eval(d.lo + (d.hi - d.lo) * k as f64 / 200.0);
                        let apex = apices(&t).iter().any(|a| a.distance(p) <= tol::LINEAR);
                        assert!(side(&t, p) > 0.0 || apex, "{major} {o:?} {p:?}");
                    }
                }
            }
        }
    }
}
