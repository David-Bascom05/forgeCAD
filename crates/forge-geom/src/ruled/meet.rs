//! Where a ruled wall between two copies of one curve meets a surface in closed form: walls
//! between a parabola and its translate (a parabolic cylinder) cut by a plane of the parabola's
//! own direction, in the parabola moved along the rulings; walls between one wrapped curve at
//! two radii cut by a cylinder about the same axis, in the curve wrapped at that radius.

use super::{Rail, Ruled};
use crate::curve::Parabola;
use crate::intersect::{Contact, SurfaceSurface};
use crate::surface::Surface;
use forge_math::tol;

/// The closed form for such a wall and a plane or cylinder, in either order; None otherwise.
pub fn meet_ruled(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    match (ruled(a), ruled(b)) {
        (Some(r), None) => r.meet(b),
        (None, Some(r)) => r.meet(a),
        _ => None,
    }
}

fn ruled(s: &Surface) -> Option<&Ruled> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Ruled>()),
        _ => None,
    }
}

impl Ruled {
    fn meet(&self, other: &Surface) -> Option<SurfaceSurface> {
        let one = |c: crate::Curve| Some(SurfaceSurface::Curves(vec![(c, Contact::Crossing)]));
        match (self.a, self.b, other) {
            (
                Rail::Parabola { curve: ca, range: ra },
                Rail::Parabola { curve: cb, range: rb },
                Surface::Plane(p),
            ) => {
                let (fa, fb) = (&ca.frame, &cb.frame);
                let same = ra == rb
                    && ca.focal == cb.focal
                    && (fa.x().get() - fb.x().get()).length() <= tol::ANGULAR
                    && (fa.y().get() - fb.y().get()).length() <= tol::ANGULAR;
                let n = p.frame.z();
                if !same || n.cross(fa.z().get()).length() > tol::ANGULAR {
                    return None;
                }
                let d = fb.origin - fa.origin;
                let along = n.dot(d);
                if along.abs() <= tol::LINEAR {
                    return None;
                }
                let s = n.dot(p.frame.origin - fa.origin) / along;
                one(crate::Curve::Parabola(Parabola {
                    frame: fa.with_origin(fa.origin + d * s),
                    focal: ca.focal,
                }))
            }
            (Rail::Wrap(wa), Rail::Wrap(wb), Surface::Cylinder(c)) => {
                let (fa, fb) = (&wa.axis, &wb.axis);
                let z = fa.z();
                let off = c.frame.origin - fa.origin;
                let same = wa.flat == wb.flat
                    && wa.unroll == wb.unroll
                    && fa.origin.distance(fb.origin) <= tol::LINEAR
                    && (fa.x().get() - fb.x().get()).length() <= tol::ANGULAR
                    && (z.get() - fb.z().get()).length() <= tol::ANGULAR;
                if !same
                    || c.frame.z().cross(z.get()).length() > tol::ANGULAR
                    || (off - z * z.dot(off)).length() > tol::LINEAR
                {
                    return None;
                }
                one(wa.at_radius(c.radius).curve())
            }
            _ => None,
        }
    }
}
