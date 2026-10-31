//! Where a screw face with a straight section meets a plane square to its axis, or a cylinder or
//! cone about it, in closed form. At each turn angle u the section meets the other surface's
//! profile (a line in (r, h)) in one point, which moves linearly with u: the curve is a helix
//! (on a cylinder), a conical helix (on a cone) or a flat spiral (on a plane), all the screw's
//! own helix curve. Where the section runs along the profile it meets the surface at one angle,
//! along the section line there. Two screw faces are met in `pair`.

use super::{Helix, Motion, Screw, Section};
use crate::curve::{Line, ProcCurve};
use crate::intersect::{Contact, SurfaceSurface};
use crate::surface::Surface;
use forge_math::{Interval, Point3, Vec2, tol};
use std::sync::Arc;

/// The closed form for a screw face and a coaxial plane, cylinder or cone, in either order;
/// None for any other pair.
pub fn meet_coaxial(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    match (screw(a), screw(b)) {
        (Some(s), None) => s.meet_coaxial(b),
        (None, Some(s)) => s.meet_coaxial(a),
        (Some(s), Some(t)) => s.meet_screw(t),
        _ => None,
    }
}

impl Screw {
    /// A plane through the axis holds the section at every angle it passes, a half turn apart
    /// (its two half planes): there the face meets it along the section line.
    fn meet_axial_plane(
        &self,
        p: &crate::surface::Plane,
        a: forge_math::Point2,
        b: forge_math::Point2,
    ) -> Option<SurfaceSurface> {
        let f = &self.m.frame;
        let n = p.frame.z();
        if n.dot(f.z().get()).abs() > tol::ANGULAR {
            return None;
        }
        // the half plane at angle th has e(th) square to n
        let th = forge_math::dmath::atan2(-n.dot(f.x().get()), n.dot(f.y().get()));
        let half = forge_math::dmath::PI;
        let k0 = ((self.ue.lo - th) / half).ceil() as i64;
        let k1 = ((self.ue.hi - th) / half).floor() as i64;
        let mut curves = Vec::new();
        for k in k0..=k1 {
            let u = th + half * k as f64;
            let (e, _, _) = self.m.axes(u);
            let at =
                |q: forge_math::Point2| f.origin + e * (q.x + self.m.a * u) + f.z() * (q.y + self.m.b * u);
            if let Some(l) = Line::through(at(a), at(b)) {
                curves.push((crate::Curve::Line(l), Contact::Crossing));
            }
        }
        Some(if curves.is_empty() {
            SurfaceSurface::Empty
        } else {
            SurfaceSurface::Curves(curves)
        })
    }
}

pub(super) fn screw(s: &Surface) -> Option<&Screw> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Screw>()),
        _ => None,
    }
}

impl Screw {
    fn meet_coaxial(&self, other: &Surface) -> Option<SurfaceSurface> {
        let f = &self.m.frame;
        let z = f.z();
        let h = |p: Point3| z.dot(p - f.origin);
        let coaxial = |o: Point3, d: forge_math::UnitVec3| {
            let q = o - f.origin;
            d.cross(z).length() <= tol::ANGULAR && (q - z * z.dot(q)).length() <= tol::LINEAR
        };
        // the other surface's profile in (r, h): n . q = c
        let (n, c) = match other {
            Surface::Plane(p) if p.frame.z().cross(z).length() <= tol::ANGULAR => {
                (Vec2::new(0.0, 1.0), h(p.frame.origin))
            }
            Surface::Cylinder(cy) if coaxial(cy.frame.origin, cy.frame.z()) => {
                (Vec2::new(1.0, 0.0), cy.radius)
            }
            Surface::Cone(co) if coaxial(co.frame.origin, co.frame.z()) => {
                let up = co.frame.z().dot(z.get()).signum();
                let n = Vec2::new(up * co.cos_a(), -co.sin_a());
                (n, n.dot(Vec2::new(co.radius, h(co.frame.origin))))
            }
            Surface::Plane(p)
                if coaxial(p.frame.origin, p.frame.x()) || coaxial(p.frame.origin, p.frame.y()) =>
            {
                let Section::Line { a: p0, b: p1 } = self.ring.pieces[0].s else {
                    return None;
                };
                return self.meet_axial_plane(p, p0, p1);
            }
            _ => return None,
        };
        // the section's box carried over the face's angles: a profile clear of it is never met
        // (a coil round a rod it does not touch, whatever its section)
        let piece = &self.ring.pieces[0];
        let (rr, hh) = (piece.rr + self.u * self.m.a, piece.hh + self.u * self.m.b);
        let reach = rr * n.x + hh * n.y;
        if c < reach.lo - tol::LINEAR || c > reach.hi + tol::LINEAR {
            return Some(SurfaceSurface::Empty);
        }
        let Section::Line { a: p0, b: p1 } = piece.s else {
            return None;
        };
        let p0 = Vec2::new(p0.x, p0.y);
        let d = Vec2::new(p1.x, p1.y) - p0;
        let ab = Vec2::new(self.m.a, self.m.b);
        let (nd, nab, gap) = (n.dot(d), n.dot(ab), c - n.dot(p0));
        if nd.abs() <= 1e-12 * d.length() {
            // the section along the profile: met at the one angle where the motion brings it on
            if nab.abs() <= 1e-12 {
                return if gap.abs() <= tol::LINEAR {
                    None
                } else {
                    Some(SurfaceSurface::Empty)
                };
            }
            let u = gap / nab;
            if !self.ue.contains(u) {
                return Some(SurfaceSurface::Empty);
            }
            let (e, _, _) = self.m.axes(u);
            let at = |q: Vec2| f.origin + e * (q.x + self.m.a * u) + z * (q.y + self.m.b * u);
            let line = Line::through(at(p0), at(p0 + d))?;
            return Some(SurfaceSurface::Curves(vec![(
                crate::Curve::Line(line),
                Contact::Crossing,
            )]));
        }
        // s(u) = (gap - nab u) / nd along the section, the point q0 + v u in (r, h)
        let q0 = p0 + d * (gap / nd);
        let v = ab - d * (nab / nd);
        // the angles where s stays on the section, carried a quarter on past its ends
        let (s0, s1) = (-0.25, 1.25);
        let (mut lo, mut hi) = if nab == 0.0 {
            if !(s0..=s1).contains(&(gap / nd)) {
                return Some(SurfaceSurface::Empty);
            }
            (self.ue.lo, self.ue.hi)
        } else {
            let at = |s: f64| (gap - s * nd) / nab;
            let (x, y) = (at(s0), at(s1));
            (x.min(y).max(self.ue.lo), x.max(y).min(self.ue.hi))
        };
        // and off the axis (past it the point is the mirror image, on no face)
        if v.x > 0.0 {
            lo = lo.max(-q0.x / v.x);
        } else if v.x < 0.0 {
            hi = hi.min(-q0.x / v.x);
        } else if q0.x <= 0.0 {
            return Some(SurfaceSurface::Empty);
        }
        if lo >= hi {
            return Some(SurfaceSurface::Empty);
        }
        let range = Interval::new(lo, hi);
        let helix = Helix {
            m: Motion {
                frame: *f,
                a: v.x,
                b: v.y,
            },
            p: forge_math::Point2::new(q0.x, q0.y),
            u: range,
            fence: None,
        };
        Some(SurfaceSurface::Curves(vec![(
            crate::Curve::Procedural(ProcCurve(Arc::new(helix))),
            Contact::Crossing,
        )]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveGeom;
    use crate::surface::{Cone, Cylinder, Plane, SurfaceGeom};
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point2};

    /// Every point of every closed-form curve lies on both surfaces.
    #[test]
    fn curves_lie_on_both_surfaces() {
        let mut r = Rng::new(733);
        let mut checked = 0;
        for _ in 0..60 {
            let f = rand_frame(&mut r);
            let (a, b) = (r.range(-0.2, 0.2), r.range(-0.5, 0.5));
            let (r0, h0) = (r.range(2.0, 4.0), r.range(-1.0, 1.0));
            let (w, h) = (r.range(0.5, 1.5), r.range(0.3, 1.0));
            let c = [
                Point2::new(r0, h0),
                Point2::new(r0 + w, h0 + r.range(-0.3, 0.3)),
                Point2::new(r0 + w, h0 + h),
                Point2::new(r0, h0 + h),
            ];
            let loop4: Vec<Section> = (0..4)
                .map(|k| Section::Line {
                    a: c[k],
                    b: c[(k + 1) % 4],
                })
                .collect();
            let Some(faces) = Screw::loop_faces(f, &loop4, a, b, Interval::new(0.0, r.range(2.0, 9.0)))
            else {
                continue;
            };
            let up = |o: f64| f.with_origin(f.origin + f.z() * o);
            let others = [
                Surface::Plane(Plane::new(up(r.range(-1.0, 3.0)))),
                Surface::Cylinder(Cylinder {
                    frame: up(r.range(-2.0, 2.0)),
                    radius: r.range(2.0, 6.0),
                }),
                Surface::Cone(
                    Cone::new(up(r.range(-1.0, 1.0)), r.range(2.0, 5.0), r.range(0.2, 1.2)).expect("a cone"),
                ),
                Surface::Cone(
                    Cone::new(
                        Frame::from_zx(f.origin, -f.z(), f.x().get()).expect("axes"),
                        r.range(2.0, 5.0),
                        r.range(0.2, 1.2),
                    )
                    .expect("a cone"),
                ),
            ];
            for face in faces {
                let s = face.clone().surface();
                for o in &others {
                    let Some(SurfaceSurface::Curves(cs)) = meet_coaxial(&s, o) else {
                        continue;
                    };
                    for (curve, _) in cs {
                        // a section line at one angle meets it too; the helices are checked here
                        let crate::Curve::Procedural(pc) = &curve else {
                            continue;
                        };
                        let d = pc.domain();
                        for i in 0..=10 {
                            let t = d.lo + (d.hi - d.lo) * i as f64 / 10.0;
                            let p = curve.eval(t);
                            assert!(o.distance(p) < 1e-9, "off the other surface by {}", o.distance(p));
                            // the point at angle t: its section point lies on the section's line
                            let l = f.point_to_local(p);
                            let (sn, cs) = forge_math::dmath::sin_cos(t);
                            let rho = l.x * cs + l.y * sn;
                            assert!((l.y * cs - l.x * sn).abs() < 1e-9 && rho > 0.0);
                            let q = Point2::new(rho - a * t, l.z - b * t);
                            let Section::Line { a: s0, b: s1 } = face.ring.pieces[0].s else {
                                unreachable!()
                            };
                            let dir = (s1 - s0).normalize().expect("a section");
                            assert!(
                                (q - s0).cross(dir).abs() < 1e-9,
                                "off the screw by {}",
                                (q - s0).cross(dir)
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 300, "{checked}");
    }
}
