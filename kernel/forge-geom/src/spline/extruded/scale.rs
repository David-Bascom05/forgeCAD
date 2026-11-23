//! A spline wall scaled about a point (forge_topo::similar): its own data, lengths times k.
use super::Extruded;
use crate::screw::fresh_id;
use forge_math::{Box3, Interval, Point3};
use std::sync::Arc;

impl Extruded {
    /// The wall scaled by `k` (> 0) about `c`: the section's control points scaled in the frame
    /// (about its origin), the frame's origin scaled about c, the heights times k.
    pub fn scaled(&self, c: Point3, k: f64) -> Extruded {
        let mut s = self.clone();
        s.id = fresh_id();
        s.frame = self.frame.with_origin(c + (self.frame.origin - c) * k);
        s.c = Arc::new(self.c.mapped(|p| Point3::new(p.x * k, p.y * k, p.z * k)));
        s.height = Interval::new(self.height.lo * k, self.height.hi * k);
        s.whole = Box3 {
            x: Interval::new(self.whole.x.lo * k, self.whole.x.hi * k),
            y: Interval::new(self.whole.y.lo * k, self.whole.y.hi * k),
            z: Interval::new(self.whole.z.lo * k, self.whole.z.hi * k),
        };
        s.radius = self.radius * k;
        s.speed = self.speed * k;
        s
    }
}
