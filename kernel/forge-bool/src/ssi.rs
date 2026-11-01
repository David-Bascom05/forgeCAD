use forge_geom::intersect::{Contact, SurfaceSurface, plane_surface};
use forge_geom::{Circle, Curve, Cylinder, Ellipse, Line, Sphere, Surface};
use forge_math::{Box3, Frame, Interval, Point3, tol};

/// Intersection of two surfaces within `region`: closed form where the pair
/// allows, otherwise traced (an exact curve defined by both surfaces).
pub fn surfaces_in(a: &Surface, b: &Surface, region: Box3) -> SurfaceSurface {
    match closed_or_flush(a, b, &region) {
        SurfaceSurface::NotAnalytic => searched(a, b, region),
        r => r,
    }
}

/// Where two surfaces meet, when that has a closed form (NotAnalytic otherwise): cheap, unlike
/// `searched`.
pub fn closed_form(a: &Surface, b: &Surface) -> SurfaceSurface {
    let closed = match (a, b) {
        (Surface::Plane(p), s) => plane_surface(p, s),
        (s, Surface::Plane(p)) => plane_surface(p, s),
        (Surface::Cylinder(c), Surface::Cylinder(d)) => cylinder_cylinder(c, d),
        (Surface::Sphere(s), Surface::Sphere(t)) => sphere_sphere(s, t),
        _ => SurfaceSurface::NotAnalytic,
    };
    match closed {
        SurfaceSurface::NotAnalytic => forge_geom::screw::meet_coaxial(a, b)
            .or_else(|| forge_geom::ruled::meet_ruled(a, b))
            .unwrap_or(SurfaceSurface::NotAnalytic),
        r => r,
    }
}

/// `closed_form`, except surfaces it has crossing that are `nearly_flush` over `region`.
#[inline]
pub(crate) fn closed_or_flush(a: &Surface, b: &Surface, region: &Box3) -> SurfaceSurface {
    let r = closed_form(a, b);
    let near = match (a, b, &r) {
        // a plane all but tangent to a cylinder crosses it in a long thin ellipse
        (Surface::Plane(_), Surface::Cylinder(_), SurfaceSurface::Curves(cs))
        | (Surface::Cylinder(_), Surface::Plane(_), SurfaceSurface::Curves(cs)) => {
            matches!(cs[..], [(Curve::Ellipse(_), _)])
        }
        (Surface::Plane(p), Surface::Plane(q), SurfaceSurface::Curves(_)) => {
            p.normal_vec().dot(q.normal_vec()).abs() > 0.5
        }
        _ => false,
    };
    if near && let Some(f) = nearly_flush(a, b, region) {
        return f;
    }
    r
}

/// Surfaces too nearly flush for an angle test to see, as a model rounded to 9 digits has them: a
/// plane on a plane, or tangent to a cylinder along its length, to within LINEAR all over `region`.
/// Left to the closed forms, the plane crossing the other in a line or a cylinder in two hairs
/// apart gives edges the tolerance cannot tell apart.
#[cold]
fn nearly_flush(a: &Surface, b: &Surface, region: &Box3) -> Option<SurfaceSurface> {
    const NEAR: f64 = 1e-4;
    let corners = || {
        [
            (region.x.lo, region.y.lo, region.z.lo),
            (region.x.hi, region.y.lo, region.z.lo),
            (region.x.lo, region.y.hi, region.z.lo),
            (region.x.hi, region.y.hi, region.z.lo),
            (region.x.lo, region.y.lo, region.z.hi),
            (region.x.hi, region.y.lo, region.z.hi),
            (region.x.lo, region.y.hi, region.z.hi),
            (region.x.hi, region.y.hi, region.z.hi),
        ]
        .map(|(x, y, z)| Point3::new(x, y, z))
    };
    match (a, b) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            let (n, m) = (p.normal_vec(), q.normal_vec());
            let d = n.dot(m);
            let flip = if d < 0.0 { -1.0 } else { 1.0 };
            let c = n.cross(m).length();
            (c > tol::ANGULAR
                && c <= NEAR
                && corners()
                    .iter()
                    .all(|&k| (p.signed_distance(k) - flip * q.signed_distance(k)).abs() <= tol::LINEAR))
            .then_some(SurfaceSurface::Coincident)
        }
        (Surface::Plane(p), Surface::Cylinder(c)) | (Surface::Cylinder(c), Surface::Plane(p)) => {
            let (n, ax) = (p.normal_vec(), c.frame.z());
            let tilt = n.dot(ax).abs();
            if tilt <= tol::ANGULAR || tilt > NEAR {
                return None;
            }
            // the plane's distance from the axis, where the region lies along the axis
            let along = corners().map(|k| ax.dot(k - c.frame.origin));
            let (lo, hi) = along
                .iter()
                .fold((f64::MAX, f64::MIN), |m, &t| (m.0.min(t), m.1.max(t)));
            let at = |t: f64| p.signed_distance(c.frame.origin + ax * t);
            let touching = (at(lo).abs() - c.radius).abs() <= tol::LINEAR
                && (at(hi).abs() - c.radius).abs() <= tol::LINEAR;
            touching.then(|| {
                let foot = c.frame.origin + ax * lo - n * at(lo);
                SurfaceSurface::Curves(vec![(Curve::Line(Line::new(foot, ax)), Contact::Touching)])
            })
        }
        _ => None,
    }
}

/// Recent pairs marched: a boolean meets one pair several times over the same region (and a
/// regen, over the same surfaces, many times more).
type Met = (Surface, Surface, Box3, SurfaceSurface);
thread_local! {
    static MET: std::cell::RefCell<std::collections::VecDeque<Met>> = const {
        std::cell::RefCell::new(std::collections::VecDeque::new())
    };
}

/// Where two surfaces with no closed form meet in `region`: by marching or tracing.
pub fn searched(a: &Surface, b: &Surface, region: Box3) -> SurfaceSurface {
    // of a spindle only the part off the axis is there, and neither the closed forms nor the tracer
    // know: a clear refusal rather than a boolean that quietly misses (research/build-failures)
    if [a, b]
        .iter()
        .any(|s| matches!(s, Surface::Torus(t) if t.is_spindle()))
    {
        return SurfaceSurface::NotAnalytic;
    }
    if forge_geom::screw::misses(a, b, region) {
        return SurfaceSurface::Empty;
    }
    // a box whose bounds differ only by rounding asks the same question
    let same = |x: Interval, y: Interval| (x.lo - y.lo).abs() <= 1e-11 && (x.hi - y.hi).abs() <= 1e-11;
    let near = |r: &Box3| same(r.x, region.x) && same(r.y, region.y) && same(r.z, region.z);
    let hit = |e: &Met| near(&e.2) && e.0 == *a && e.1 == *b;
    if let Some(r) = MET.with(|m| m.borrow().iter().find(|e| hit(e)).map(|e| e.3.clone())) {
        return r;
    }
    let met = forge_geom::meet::intersect(a, b, region).or_else(|| forge_geom::loftmeet::meet(a, b, region));
    let r = match met {
        Some(r) if r != SurfaceSurface::NotAnalytic => r,
        _ => match forge_geom::intcurve::trace(a, b, region) {
            Some(cs) if cs.is_empty() => SurfaceSurface::Empty,
            Some(cs) => SurfaceSurface::Curves(
                cs.into_iter()
                    .map(|c| (Curve::Intersection(std::sync::Arc::new(c)), Contact::Crossing))
                    .collect(),
            ),
            // a screw face the tracer cannot follow across its carried track's corners
            None => forge_geom::screw::meet_general(a, b, region).unwrap_or(SurfaceSurface::NotAnalytic),
        },
    };
    MET.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() == 32 {
            m.pop_front();
        }
        m.push_back((a.clone(), b.clone(), region, r.clone()));
    });
    r
}

/// Whether two surfaces are the same surface.
pub fn coincident(a: &Surface, b: &Surface) -> bool {
    match (a, b) {
        (Surface::Plane(p), Surface::Plane(q)) => forge_geom::intersect::planes_coincident(p, q),
        (Surface::Plane(p), s) | (s, Surface::Plane(p)) => plane_surface(p, s) == SurfaceSurface::Coincident,
        (Surface::Cylinder(c), Surface::Cylinder(d)) => cylinder_cylinder(c, d) == SurfaceSurface::Coincident,
        (Surface::Sphere(s), Surface::Sphere(t)) => sphere_sphere(s, t) == SurfaceSurface::Coincident,
        _ => forge_geom::meet::coaxial(a, b) == Some(SurfaceSurface::Coincident),
    }
}

/// Parallel axes give lines (or the same cylinder), equal ones with crossing axes two
/// ellipses; anything else is a quartic space curve, left to the general path.
fn cylinder_cylinder(c: &Cylinder, d: &Cylinder) -> SurfaceSurface {
    let a = c.frame.z();
    if a.cross(d.frame.z()).length() > tol::ANGULAR {
        return equal_crossing(c, d).unwrap_or(SurfaceSurface::NotAnalytic);
    }
    // work in c's cross-section plane
    let off = d.frame.origin - c.frame.origin;
    let off = off - a * a.dot(off);
    let dist = off.length();
    let (r1, r2) = (c.radius, d.radius);
    if dist <= tol::LINEAR {
        return if (r1 - r2).abs() <= tol::LINEAR {
            SurfaceSurface::Coincident
        } else {
            SurfaceSurface::Empty
        };
    }
    if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
        return SurfaceSurface::Empty;
    }
    let ex = off / dist;
    let ey = a.cross(ex);
    // circle-circle in the section: x along the centre line
    let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
    let h2 = r1 * r1 - x * x;
    let at = |y: f64| c.frame.origin + ex * x + ey * y;
    let dir = a;
    if h2 <= (2.0 * tol::LINEAR * r1).max(0.0) {
        return SurfaceSurface::Curves(vec![(Curve::Line(Line::new(at(0.0), dir)), Contact::Touching)]);
    }
    let h = h2.sqrt();
    SurfaceSurface::Curves(vec![
        (Curve::Line(Line::new(at(-h), dir)), Contact::Crossing),
        (Curve::Line(Line::new(at(h), dir)), Contact::Crossing),
    ])
}

/// Equal cylinders whose axes cross (a T or + drilling) meet in two ellipses, in the planes
/// through the crossing that bisect the axes, centred on it: each plane holds the common
/// perpendicular, along which both ellipses reach r, where they cross and the cylinders touch.
fn equal_crossing(c: &Cylinder, d: &Cylinder) -> Option<SurfaceSurface> {
    let r = c.radius;
    let (a, b) = (c.frame.z().get(), d.frame.z().get());
    let n = a.cross(b);
    let w = d.frame.origin - c.frame.origin;
    let len = n.length();
    if (r - d.radius).abs() > tol::LINEAR || (w.dot(n) / len).abs() > tol::LINEAR {
        return None;
    }
    let o = c.frame.origin + a * (w.cross(b).dot(n) / (len * len));
    let m = n / len;
    let mut out = Vec::with_capacity(2);
    for u in [a + b, a - b] {
        let x = u.normalize()?.get();
        // a point x t from the crossing is t |x × a| from either axis
        let frame = Frame::from_axes(o, x, m, x.cross(m))?;
        let major = r / x.cross(a).length();
        out.push((
            Curve::Ellipse(Ellipse {
                frame,
                major,
                minor: r,
            }),
            Contact::Crossing,
        ));
    }
    Some(SurfaceSurface::Curves(out))
}

fn sphere_sphere(s: &Sphere, t: &Sphere) -> SurfaceSurface {
    let d = t.frame.origin - s.frame.origin;
    let dist = d.length();
    let (r1, r2) = (s.radius, t.radius);
    if dist <= tol::LINEAR {
        return if (r1 - r2).abs() <= tol::LINEAR {
            SurfaceSurface::Coincident
        } else {
            SurfaceSurface::Empty
        };
    }
    if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
        return SurfaceSurface::Empty;
    }
    let n = d.normalize().expect("distinct centres");
    let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
    let centre = s.frame.origin + n * x;
    let h2 = r1 * r1 - x * x;
    if h2 <= 2.0 * tol::LINEAR * r1 {
        return SurfaceSurface::Points(vec![centre]);
    }
    SurfaceSurface::Curves(vec![(
        Curve::Circle(Circle {
            frame: Frame::from_z(centre, n),
            radius: h2.sqrt(),
        }),
        Contact::Crossing,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::testing::{rand_frame, rand_unit};
    use forge_geom::{CurveGeom, SurfaceGeom};
    use forge_math::rng::Rng;

    fn surfaces(a: &Surface, b: &Surface) -> SurfaceSurface {
        let far = forge_math::Point3::new(100.0, 100.0, 100.0);
        surfaces_in(
            a,
            b,
            Box3::from_points([far, forge_math::Point3::ORIGIN - far.to_vec()]),
        )
    }

    fn on_both(a: &Surface, b: &Surface) {
        if let SurfaceSurface::Curves(cs) = surfaces(a, b) {
            for (c, contact) in cs {
                let tolerance = if contact == Contact::Touching {
                    1.01 * tol::LINEAR
                } else {
                    1e-7
                };
                let range = c.full_range(20.0);
                for i in 0..=20 {
                    let p = c.eval((range.lo + range.width() * i as f64 / 20.0).min(range.hi));
                    assert!(
                        a.distance(p) <= tolerance && b.distance(p) <= tolerance,
                        "{a:?} {b:?} {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn parallel_cylinders_and_spheres() {
        let mut r = Rng::new(900);
        let mut curves = 0;
        for _ in 0..2000 {
            let f = rand_frame(&mut r);
            let g = f.with_origin(f.origin + f.x() * r.range(0.0, 8.0) + f.z() * r.range(-5.0, 5.0));
            let c = Surface::Cylinder(Cylinder {
                frame: f,
                radius: r.range(1.0, 5.0),
            });
            let d = Surface::Cylinder(Cylinder {
                frame: g,
                radius: r.range(1.0, 5.0),
            });
            on_both(&c, &d);
            let s = Surface::Sphere(Sphere {
                frame: f,
                radius: r.range(1.0, 5.0),
            });
            let t = Surface::Sphere(Sphere {
                frame: g.with_origin(g.origin + rand_unit(&mut r) * 1.0),
                radius: r.range(1.0, 5.0),
            });
            on_both(&s, &t);
            if matches!(surfaces(&c, &d), SurfaceSurface::Curves(_)) {
                curves += 1;
            }
        }
        assert!(curves > 500);
        let c = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        assert_eq!(surfaces(&c, &c), SurfaceSurface::Coincident);
    }
}
