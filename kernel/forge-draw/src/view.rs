use forge_math::{Point2, Point3, UnitVec3, Vec2, Vec3};

/// An orthographic view: the direction toward the eye and the screen's right and up. A model
/// point at `p` is drawn at `(p . right, p . up)`, in model millimetres (scale is the sheet's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub toward: UnitVec3,
    pub right: UnitVec3,
    pub up: UnitVec3,
}

/// The standard orientations, as Inventor names them (`iso` is its "iso top right").
pub const NAMED: [&str; 8] = [
    "front", "back", "top", "bottom", "right", "left", "iso", "iso_left",
];

impl View {
    /// The view looking back along `toward` with `up` (made square to it) up the screen.
    pub fn new(toward: Vec3, up: Vec3) -> Option<View> {
        let toward = toward.normalize()?;
        let right = up.cross(toward.get()).normalize()?;
        let up = toward.cross(right.get()).normalize()?;
        Some(View { toward, right, up })
    }

    pub fn named(name: &str) -> Option<View> {
        let (t, u) = match name {
            "front" => (Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
            "back" => (Vec3::Y, Vec3::Z),
            "right" => (Vec3::X, Vec3::Z),
            "left" => (Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
            "top" => (Vec3::Z, Vec3::Y),
            "bottom" => (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)),
            "iso" => (Vec3::new(1.0, -1.0, 1.0), Vec3::Z),
            "iso_left" => (Vec3::new(-1.0, -1.0, 1.0), Vec3::Z),
            _ => return None,
        };
        View::new(t, u)
    }

    pub fn project(&self, p: Point3) -> Point2 {
        let v = p.to_vec();
        Point2::new(v.dot(self.right.get()), v.dot(self.up.get()))
    }

    pub fn project_vec(&self, v: Vec3) -> Vec2 {
        Vec2::new(v.dot(self.right.get()), v.dot(self.up.get()))
    }

    /// How near the eye a point is: larger is nearer.
    pub fn depth(&self, p: Point3) -> f64 {
        p.to_vec().dot(self.toward.get())
    }

    /// The model direction a sheet direction is.
    pub fn unproject_vec(&self, v: Vec2) -> Vec3 {
        self.right.get() * v.x + self.up.get() * v.y
    }

    /// The view seen from the side `dx, dy` points to on the sheet (one of the four ways, or a
    /// diagonal for an isometric child): third angle sees the side it is placed on, first angle the
    /// opposite one.
    pub fn projected(&self, dx: f64, dy: f64, third_angle: bool) -> Option<View> {
        let s = if third_angle { 1.0 } else { -1.0 };
        let (r, u, t) = (self.right.get(), self.up.get(), self.toward.get());
        let diagonal = dx.abs() > 1e-9 && dy.abs() > 1e-9;
        if diagonal {
            let toward = t + (r * dx.signum() + u * dy.signum()) * s;
            return View::new(toward, u);
        }
        if dx.abs() >= dy.abs() {
            View::new(r * (dx.signum() * s), u)
        } else {
            View::new(u * (dy.signum() * s), t * -(dy.signum() * s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_angle_children() {
        let front = View::named("front").unwrap();
        let right = front.projected(1.0, 0.0, true).unwrap();
        assert!((right.toward.get() - Vec3::X).length() < 1e-12);
        let top = front.projected(0.0, 1.0, true).unwrap();
        assert!((top.toward.get() - Vec3::Z).length() < 1e-12);
        assert!((top.up.get() - Vec3::Y).length() < 1e-12);
        // first angle: the view placed right is seen from the left
        let left = front.projected(1.0, 0.0, false).unwrap();
        assert!((left.toward.get() - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-12);
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(front.project(p), Point2::new(1.0, 3.0));
        assert_eq!(top.project(p), Point2::new(1.0, 2.0));
        assert_eq!(right.project(p), Point2::new(2.0, 3.0));
    }
}
