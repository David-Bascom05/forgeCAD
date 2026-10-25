use crate::{Point2, Point3, UnitVec3, Vec3};

/// A right-handed orthonormal coordinate system placed in space. Also serves as
/// the rigid transform from its local coordinates to world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: Point3,
    x: UnitVec3,
    y: UnitVec3,
    z: UnitVec3,
}

impl Default for Frame {
    fn default() -> Self {
        Frame::WORLD
    }
}

impl Frame {
    pub const WORLD: Frame = Frame {
        origin: Point3::ORIGIN,
        x: UnitVec3::X,
        y: UnitVec3::Y,
        z: UnitVec3::Z,
    };

    /// z is kept exactly; x is the part of `x_hint` perpendicular to z.
    pub fn from_zx(origin: Point3, z: UnitVec3, x_hint: Vec3) -> Option<Frame> {
        // Gram-Schmidt twice: one pass loses perpendicularity when the hint is nearly along z.
        let x1 = x_hint - z * z.dot(x_hint);
        let x = (x1 - z * z.dot(x1)).normalize()?;
        let y = z.cross(x).normalize()?;
        Some(Frame { origin, x, y, z })
    }

    /// x is picked deterministically from z alone.
    pub fn from_z(origin: Point3, z: UnitVec3) -> Frame {
        let x = z.any_perpendicular();
        let y = z.cross(x).normalize().expect("perpendicular unit vectors");
        Frame { origin, x, y, z }
    }

    pub fn x(&self) -> UnitVec3 {
        self.x
    }
    pub fn y(&self) -> UnitVec3 {
        self.y
    }
    pub fn z(&self) -> UnitVec3 {
        self.z
    }

    pub fn vec_to_world(&self, v: Vec3) -> Vec3 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
    pub fn vec_to_local(&self, v: Vec3) -> Vec3 {
        Vec3::new(self.x.dot(v), self.y.dot(v), self.z.dot(v))
    }
    pub fn point_to_world(&self, p: Point3) -> Point3 {
        self.origin + self.vec_to_world(p.to_vec())
    }
    pub fn point_to_local(&self, p: Point3) -> Point3 {
        let v = self.vec_to_local(p - self.origin);
        Point3::new(v.x, v.y, v.z)
    }
    /// A point of the frame's xy plane.
    pub fn plane_point(&self, p: Point2) -> Point3 {
        self.origin + self.x * p.x + self.y * p.y
    }
    pub fn unit_to_world(&self, u: UnitVec3) -> UnitVec3 {
        UnitVec3::new_unchecked(self.vec_to_world(u.get()))
    }

    /// `self` applied after `inner`: local coordinates of `inner` go to world.
    pub fn compose(&self, inner: &Frame) -> Frame {
        Frame {
            origin: self.point_to_world(inner.origin),
            x: self.unit_to_world(inner.x),
            y: self.unit_to_world(inner.y),
            z: self.unit_to_world(inner.z),
        }
    }

    pub fn inverse(&self) -> Frame {
        let ex = UnitVec3::new_unchecked(Vec3::new(self.x.get().x, self.y.get().x, self.z.get().x));
        let ey = UnitVec3::new_unchecked(Vec3::new(self.x.get().y, self.y.get().y, self.z.get().y));
        let ez = UnitVec3::new_unchecked(Vec3::new(self.x.get().z, self.y.get().z, self.z.get().z));
        let o = self.vec_to_local(-self.origin.to_vec());
        Frame {
            origin: Point3::new(o.x, o.y, o.z),
            x: ex,
            y: ey,
            z: ez,
        }
    }

    /// Same origin and z, rotated about z so x points along `angle`.
    pub fn rotated_about_z(&self, angle: f64) -> Frame {
        let (s, c) = crate::dmath::sin_cos(angle);
        let x = (self.x * c + self.y * s)
            .normalize()
            .expect("rotation keeps length");
        let y = self.z.cross(x).normalize().expect("perpendicular unit vectors");
        Frame {
            origin: self.origin,
            x,
            y,
            z: self.z,
        }
    }

    pub fn with_origin(&self, origin: Point3) -> Frame {
        Frame { origin, ..*self }
    }

    /// z flipped, x kept, y flipped: still right-handed.
    pub fn flipped(&self) -> Frame {
        Frame {
            origin: self.origin,
            x: self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn rand_frame(r: &mut Rng) -> Frame {
        let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
            .normalize()
            .unwrap();
        let o = Point3::new(
            r.range(-100.0, 100.0),
            r.range(-100.0, 100.0),
            r.range(-100.0, 100.0),
        );
        Frame::from_zx(
            o,
            z,
            Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)),
        )
        .unwrap_or_else(|| Frame::from_z(o, z))
    }

    #[test]
    fn orthonormal_and_invertible() {
        let mut r = Rng::new(10);
        for _ in 0..10_000 {
            let f = rand_frame(&mut r);
            assert!((f.x.get().length() - 1.0).abs() < 1e-14);
            assert!(f.x.dot(f.y).abs() < 1e-15 && f.y.dot(f.z).abs() < 1e-14);
            assert!((f.x.cross(f.y) - f.z.get()).length() < 1e-14);
            let p = Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), r.range(-50.0, 50.0));
            assert!(f.point_to_local(f.point_to_world(p)).distance(p) < 1e-12);
            let inv = f.inverse();
            assert!(inv.point_to_world(f.point_to_world(p)).distance(p) < 1e-12);
            let g = rand_frame(&mut r);
            assert!(
                f.compose(&g)
                    .point_to_world(p)
                    .distance(f.point_to_world(g.point_to_world(p)))
                    < 1e-12
            );
        }
    }
}
