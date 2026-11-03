//! Each joint kind as a fixed bundle of primitive frame constraints (prior-art §5.3), its closed
//! form motion for tree placement, and its rows with their derivatives for loop blocks.
//!
//! Conventions: A is the moving end, B the reference. With `flip`, A's frame is turned half a turn
//! about its x first (Frame::flipped). The joint holds when `B⁻¹ ∘ A` equals the kind's motion
//! `M(values)`:
//! ```text
//! fastened     I                      slider       T(0,0,d)
//! revolute     Rz(θ)                  cylindrical  T(0,0,d) Rz(θ)
//! pin_slot     T(d,0,0) Rz(θ)         planar       T(u,v,0) Rz(θ)
//! ball         any rotation, origins coincident
//! ```

use crate::model::{ConstraintKind, Freedom, JointKind};
use forge_math::{Frame, Point3, Vec3, dmath};

pub(crate) fn effective(a: Frame, flip: bool) -> Frame {
    if flip { a.flipped() } else { a }
}

/// `B⁻¹ ∘ A`: A's frame in B's coordinates.
pub(crate) fn relative(a: &Frame, b: &Frame) -> Frame {
    b.inverse().compose(a)
}

fn angle_of(rel: &Frame) -> f64 {
    let x = rel.x().get();
    dmath::atan2(x.y, x.x)
}

/// The value of one freedom read off a relative frame (angles in (-π, π]).
pub(crate) fn measure(f: Freedom, rel: &Frame) -> f64 {
    match f {
        Freedom::Tx => rel.origin.x,
        Freedom::Ty => rel.origin.y,
        Freedom::Tz => rel.origin.z,
        Freedom::Rz => angle_of(rel),
        // a ball's rotation is kept whole, never as values
        Freedom::Rx | Freedom::Ry => 0.0,
    }
}

/// The motion `M` for these values (one per freedom of the kind, in its order). A ball's motion
/// is `ball` with its origin moved to zero.
pub(crate) fn motion(kind: JointKind, v: &[f64], ball: &Frame) -> Frame {
    let rz = |t: f64| Frame::WORLD.rotated_about_z(t);
    let at = |x: f64, y: f64, z: f64, f: Frame| f.with_origin(Point3::new(x, y, z));
    match kind {
        JointKind::Fastened => Frame::WORLD,
        JointKind::Revolute => rz(v[0]),
        JointKind::Slider => at(0.0, 0.0, v[0], Frame::WORLD),
        JointKind::Cylindrical => at(0.0, 0.0, v[0], rz(v[1])),
        JointKind::PinSlot => at(v[0], 0.0, 0.0, rz(v[1])),
        JointKind::Planar => at(v[0], v[1], 0.0, rz(v[2])),
        JointKind::Ball => ball.with_origin(Point3::ORIGIN),
    }
}

/// An angle moved by whole turns to lie within half a turn of `near`.
pub(crate) fn unwrap_near(a: f64, near: f64) -> f64 {
    let d = a - near;
    let turns = (d / dmath::TAU).round();
    if turns == 0.0 { a } else { a - dmath::TAU * turns }
}

/// One row: its residual and its derivative with respect to each end's cluster increment
/// `(ω, v)`, where a cluster moves as `R ← exp(ω̂) R`, `t ← t + v`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Row {
    pub r: f64,
    pub ga: [f64; 6],
    pub gb: [f64; 6],
    /// An angle-like row (a dot of two unit axes or an angle): scaled by the block size.
    pub angular: bool,
}

/// The two ends in world coordinates, with each end's cluster origin.
pub(crate) struct Ends {
    pub a: Frame,
    pub b: Frame,
    pub ta: Point3,
    pub tb: Point3,
}

fn put(g: &mut [f64; 6], w: Vec3, v: Vec3) {
    *g = [w.x, w.y, w.z, v.x, v.y, v.z];
}

impl Ends {
    /// `(oA − oB)·n` with `n` an axis of B.
    fn along(&self, n: Vec3) -> Row {
        let d = self.a.origin - self.b.origin;
        let ra = self.a.origin - self.ta;
        let rb = self.b.origin - self.tb;
        let mut row = Row {
            r: d.dot(n),
            ..Row::default()
        };
        put(&mut row.ga, ra.cross(n), n);
        put(&mut row.gb, n.cross(d) - rb.cross(n), -n);
        row
    }
    /// `(oA − oB)·e` with `e` a fixed world axis.
    fn coord(&self, e: Vec3) -> Row {
        let d = self.a.origin - self.b.origin;
        let ra = self.a.origin - self.ta;
        let rb = self.b.origin - self.tb;
        let mut row = Row {
            r: d.dot(e),
            ..Row::default()
        };
        put(&mut row.ga, ra.cross(e), e);
        put(&mut row.gb, -rb.cross(e), -e);
        row
    }
    /// `u·n`, `u` on A, `n` on B.
    fn dot(&self, u: Vec3, n: Vec3) -> Row {
        let mut row = Row {
            r: u.dot(n),
            angular: true,
            ..Row::default()
        };
        let c = u.cross(n);
        put(&mut row.ga, c, Vec3::ZERO);
        put(&mut row.gb, -c, Vec3::ZERO);
        row
    }

    /// A primitive constraint's rows, in a fixed order.
    pub fn rows(&self, kind: ConstraintKind, out: &mut Vec<Row>) {
        let (xa, za) = (self.a.x().get(), self.a.z().get());
        let (xb, yb, zb) = (self.b.x().get(), self.b.y().get(), self.b.z().get());
        let coincident = |out: &mut Vec<Row>| {
            for e in [Vec3::X, Vec3::Y, Vec3::Z] {
                out.push(self.coord(e));
            }
        };
        let parallel = |out: &mut Vec<Row>| {
            out.push(self.dot(za, xb));
            out.push(self.dot(za, yb));
        };
        let on_line = |out: &mut Vec<Row>| {
            out.push(self.along(xb));
            out.push(self.along(yb));
        };
        let offset = |mut r: Row, o: f64| {
            r.r -= o;
            r
        };
        match kind {
            ConstraintKind::Coincident => coincident(out),
            ConstraintKind::PointOnPlane { offset: o } => out.push(offset(self.along(zb), o)),
            ConstraintKind::PointOnLine => on_line(out),
            ConstraintKind::PlanePlane { offset: o } => {
                parallel(out);
                out.push(offset(self.along(zb), o));
            }
            ConstraintKind::Concentric => {
                parallel(out);
                on_line(out);
            }
            ConstraintKind::Insert { offset: o } => {
                parallel(out);
                on_line(out);
                out.push(offset(self.along(zb), o));
            }
            ConstraintKind::Parallel => parallel(out),
            ConstraintKind::Perpendicular => out.push(self.dot(za, zb)),
            ConstraintKind::Angle { angle } => out.push(offset(self.dot(za, zb), dmath::cos(angle))),
            ConstraintKind::Distance { distance } => {
                let d = self.a.origin - self.b.origin;
                let l = d.length();
                let n = if l > 0.0 { d / l } else { Vec3::X };
                let mut r = self.coord(n);
                r.r = l - distance;
                out.push(r);
            }
            ConstraintKind::Lock => {
                coincident(out);
                parallel(out);
                out.push(self.dot(xa, yb));
            }
            ConstraintKind::Twist { angle } => out.push(self.angle_row(angle)),
        }
    }

    /// The largest residual of a primitive here (angles × `size`).
    pub fn gap(&self, kind: ConstraintKind, size: f64) -> f64 {
        let mut rows = Vec::new();
        self.rows(kind, &mut rows);
        rows.iter()
            .map(|r| r.r.abs() * if r.angular { size } else { 1.0 })
            .fold(0.0, f64::max)
    }

    /// One value's row, `value − target`, angles taken within half a turn of `target`.
    pub fn value_row(&self, f: Freedom, target: f64) -> Row {
        let (xb, yb, zb) = (self.b.x().get(), self.b.y().get(), self.b.z().get());
        let mut row = match f {
            Freedom::Tx => self.along(xb),
            Freedom::Ty => self.along(yb),
            Freedom::Tz => self.along(zb),
            _ => return self.angle_row(target),
        };
        row.r -= target;
        row
    }

    /// The value of a freedom here, and its row as `value − 0` (angles unwrapped near `near`).
    pub fn value(&self, f: Freedom, near: f64) -> Row {
        match f {
            Freedom::Rz | Freedom::Rx | Freedom::Ry => {
                let xa = self.a.x().get();
                let (xb, yb) = (self.b.x().get(), self.b.y().get());
                let (p, q) = (xa.dot(xb), xa.dot(yb));
                let n2 = (p * p + q * q).max(1e-300);
                let dp = self.dot(xa, xb);
                let dq = self.dot(xa, yb);
                let mut row = Row {
                    r: unwrap_near(dmath::atan2(q, p), near),
                    angular: true,
                    ..Row::default()
                };
                for k in 0..6 {
                    row.ga[k] = (p * dq.ga[k] - q * dp.ga[k]) / n2;
                    row.gb[k] = (p * dq.gb[k] - q * dp.gb[k]) / n2;
                }
                row
            }
            _ => self.value_row(f, 0.0),
        }
    }

    /// `θ − target` for the angle of A's x about B's z, as `xA · (cosθ yB − sinθ xB)`, which
    /// is smooth everywhere and zero exactly at the target.
    fn angle_row(&self, target: f64) -> Row {
        let (s, c) = dmath::sin_cos(target);
        let xa = self.a.x().get();
        let m = self.b.y().get() * c - self.b.x().get() * s;
        let mut row = self.dot(xa, m);
        // the dot's derivative: B's axes are rotated as one, so m moves with B as n does
        row.r = xa.dot(m);
        row
    }
}

/// How far apart two frames are: origin distance, and axis gaps times `size`.
pub(crate) fn frame_gap(p: &Frame, q: &Frame, size: f64) -> f64 {
    let d = (p.origin - q.origin).length();
    let ax = (p.x().get() - q.x().get())
        .length()
        .max((p.z().get() - q.z().get()).length());
    d.max(ax * size)
}

/// `v` turned by the rotation vector `w` (Rodrigues).
pub(crate) fn rotate(v: Vec3, w: Vec3) -> Vec3 {
    let t2 = w.dot(w);
    if t2 == 0.0 {
        return v;
    }
    let t = t2.sqrt();
    let k = w / t;
    let (s, c) = dmath::sin_cos(t);
    v * c + k.cross(v) * s + k * (k.dot(v) * (1.0 - c))
}

/// A frame moved by the increment `(ω, v)` about its own origin.
pub(crate) fn step(f: &Frame, w: Vec3, v: Vec3) -> Frame {
    let z = rotate(f.z().get(), w);
    let x = rotate(f.x().get(), w);
    let z = z.normalize().unwrap_or(f.z());
    Frame::from_zx(f.origin + v, z, x).unwrap_or(*f)
}
