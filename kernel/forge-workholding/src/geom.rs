//! The bodies' primitives, rigid poses, and what the consumers take: the certified checker's
//! faces (forge-cam-geom `Face`), boxes, and distances (the envelope test, the reach report).
//!
//! Every body is a union of blocks (oriented boxes) and cylinders: exact for the checker (planes
//! and cylinders are its surfaces), for the machine simulation (boxes and cylinders) and for the
//! renderer. A body's keep-out grows it: a block by `k` on every side (its corners stay sharp, so
//! the grown block holds every point within `k` of the block), a cylinder by `k` in radius and at
//! both ends (likewise a superset). A superset is the safe side for a clearance check.
use forge_cam_geom::check::{Face, Surf, TSeg, Trim};
use forge_cam_geom::ival::Ival;

pub type V3 = [f64; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prim {
    /// A block: corner `o`, edges along the orthonormal `axes` (u, v, w), of lengths `size`.
    Block { o: V3, axes: [V3; 3], size: V3 },
    /// A cylinder: the centre of its base, the unit axis, the radius and the height along it.
    Cyl { base: V3, axis: V3, r: f64, h: f64 },
}

pub const IDENT: [V3; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl Prim {
    /// An axis-aligned box from its corners.
    pub fn aabb(min: V3, max: V3) -> Prim {
        Prim::Block {
            o: min,
            axes: IDENT,
            size: sub(max, min),
        }
    }

    /// A cylinder standing on z (axis +z) at (x, y), from z0 to z1.
    pub fn post(x: f64, y: f64, r: f64, z0: f64, z1: f64) -> Prim {
        Prim::Cyl {
            base: [x, y, z0],
            axis: [0.0, 0.0, 1.0],
            r,
            h: z1 - z0,
        }
    }

    /// The prim grown by `k` (see the module note: a superset of every point within `k`).
    pub fn grown(&self, k: f64) -> Prim {
        if k == 0.0 {
            return *self;
        }
        match *self {
            Prim::Block { o, axes, size } => Prim::Block {
                o: sub(o, add(add(mul(axes[0], k), mul(axes[1], k)), mul(axes[2], k))),
                axes,
                size: [size[0] + 2.0 * k, size[1] + 2.0 * k, size[2] + 2.0 * k],
            },
            Prim::Cyl { base, axis, r, h } => Prim::Cyl {
                base: sub(base, mul(axis, k)),
                axis,
                r: r + k,
                h: h + 2.0 * k,
            },
        }
    }

    /// Its axis-aligned bounds (exact for a block; for a cylinder the bound of its two end
    /// disks, which is its bound).
    pub fn bounds(&self) -> (V3, V3) {
        match *self {
            Prim::Block { o, axes, size } => {
                let mut lo = o;
                let mut hi = o;
                for k in 0..3 {
                    for (a, s) in axes.iter().zip(size) {
                        let d = a[k] * s;
                        if d < 0.0 {
                            lo[k] += d;
                        } else {
                            hi[k] += d;
                        }
                    }
                }
                (lo, hi)
            }
            Prim::Cyl { base, axis, r, h } => {
                let top = add(base, mul(axis, h));
                let mut lo = [0.0; 3];
                let mut hi = [0.0; 3];
                for k in 0..3 {
                    let e = r * (1.0 - axis[k] * axis[k]).max(0.0).sqrt();
                    lo[k] = base[k].min(top[k]) - e;
                    hi[k] = base[k].max(top[k]) + e;
                }
                (lo, hi)
            }
        }
    }

    pub fn volume(&self) -> f64 {
        match *self {
            Prim::Block { size, .. } => size[0] * size[1] * size[2],
            Prim::Cyl { r, h, .. } => std::f64::consts::PI * r * r * h,
        }
    }

    /// How far a point lies outside (0 inside or on).
    pub fn outside(&self, x: V3) -> f64 {
        match *self {
            Prim::Block { o, axes, size } => {
                let d = sub(x, o);
                let mut s = 0.0;
                for k in 0..3 {
                    let t = dot(d, axes[k]);
                    let e = if t < 0.0 {
                        -t
                    } else if t > size[k] {
                        t - size[k]
                    } else {
                        0.0
                    };
                    s += e * e;
                }
                s.sqrt()
            }
            Prim::Cyl { base, axis, r, h } => {
                let d = sub(x, base);
                let t = dot(d, axis);
                let rad = sub(d, mul(axis, t));
                let rr = dot(rad, rad).sqrt();
                let er = (rr - r).max(0.0);
                let et = if t < 0.0 {
                    -t
                } else if t > h {
                    t - h
                } else {
                    0.0
                };
                (er * er + et * et).sqrt()
            }
        }
    }

    /// The prim moved by a pose.
    pub fn posed(&self, p: &Pose) -> Prim {
        match *self {
            Prim::Block { o, axes, size } => Prim::Block {
                o: p.point(o),
                axes: [p.dir(axes[0]), p.dir(axes[1]), p.dir(axes[2])],
                size,
            },
            Prim::Cyl { base, axis, r, h } => Prim::Cyl {
                base: p.point(base),
                axis: p.dir(axis),
                r,
                h,
            },
        }
    }

    /// The checker's faces of the prim, ids `id0..`; `mu` the faces' own error bound.
    pub fn faces(&self, id0: u32, mu: f64) -> Vec<Face> {
        match *self {
            Prim::Block { o, axes, size } => {
                let rect = |w: f64, h: f64| Trim {
                    segs: vec![
                        TSeg::Line { a: [0.0, 0.0], b: [w, 0.0] },
                        TSeg::Line { a: [w, 0.0], b: [w, h] },
                        TSeg::Line { a: [w, h], b: [0.0, h] },
                        TSeg::Line { a: [0.0, h], b: [0.0, 0.0] },
                    ],
                };
                let mk = |o: V3, i: usize, j: usize, id: u32| {
                    Face::new(
                        Surf::Plane {
                            o,
                            u: axes[i],
                            v: axes[j],
                        },
                        Ival::new(0.0, size[i]),
                        Ival::new(0.0, size[j]),
                        Some(rect(size[i], size[j])),
                        mu,
                        id,
                    )
                };
                let far = |k: usize| add(o, mul(axes[k], size[k]));
                vec![
                    mk(o, 0, 1, id0),
                    mk(far(2), 0, 1, id0 + 1),
                    mk(o, 0, 2, id0 + 2),
                    mk(far(1), 0, 2, id0 + 3),
                    mk(o, 1, 2, id0 + 4),
                    mk(far(0), 1, 2, id0 + 5),
                ]
            }
            Prim::Cyl { base, axis, r, h } => {
                let (e1, e2) = perp(axis);
                // four quarter pieces about e1, e2, −e1, −e2: p = tan of half the angle from the
                // middle, |p| ≤ tan(π/8) = √2 − 1 (widened by 1e-12 relative, as stock.rs does)
                let th = (2f64.sqrt() - 1.0) * (1.0 + 1e-12);
                let mut out = Vec::new();
                for (k, x) in [e1, e2, mul(e1, -1.0), mul(e2, -1.0)].into_iter().enumerate() {
                    out.push(Face::new(
                        Surf::Cyl {
                            o: base,
                            x,
                            y: cross(axis, x),
                            ax: axis,
                            r,
                        },
                        Ival::new(-th, th),
                        Ival::new(0.0, h),
                        None,
                        mu + 1e-12 * r,
                        id0 + k as u32,
                    ));
                }
                // the end disks: a plane trimmed by four quarter arcs, centred
                let q = |a: [f64; 2], b: [f64; 2]| TSeg::Arc {
                    a,
                    b,
                    c: [0.0, 0.0],
                    r,
                    ccw: true,
                };
                let disk = Trim {
                    segs: vec![
                        q([r, 0.0], [0.0, r]),
                        q([0.0, r], [-r, 0.0]),
                        q([-r, 0.0], [0.0, -r]),
                        q([0.0, -r], [r, 0.0]),
                    ],
                };
                for (k, c) in [base, add(base, mul(axis, h))].into_iter().enumerate() {
                    out.push(Face::new(
                        Surf::Plane { o: c, u: e1, v: e2 },
                        Ival::new(-r, r),
                        Ival::new(-r, r),
                        Some(disk.clone()),
                        mu,
                        id0 + 4 + k as u32,
                    ));
                }
                out
            }
        }
    }
}

/// Two unit vectors completing a unit axis to a right-handed frame (exact for the axis
/// directions ±x, ±y, ±z).
pub fn perp(a: V3) -> (V3, V3) {
    let pick = if a[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let e2 = cross(a, pick);
    let n = dot(e2, e2).sqrt();
    let e2 = mul(e2, 1.0 / n);
    let e1 = cross(e2, a);
    (e1, e2)
}

/// A rigid pose: x ↦ m·x + t, `m` orthonormal with det +1 (rows).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub m: [V3; 3],
    pub t: V3,
}

impl Pose {
    pub const ID: Pose = Pose {
        m: IDENT,
        t: [0.0; 3],
    };

    pub fn shift(t: V3) -> Pose {
        Pose { m: IDENT, t }
    }

    /// A turn about z by a whole number of quarter turns (exact), then a shift.
    pub fn quarter_z(k: i32, t: V3) -> Pose {
        let (c, s) = match k.rem_euclid(4) {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 0.0),
            _ => (0.0, -1.0),
        };
        Pose {
            m: [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
            t,
        }
    }

    /// The pose taking the unit direction `d` to +z by a quarter turn about x or y (d one of
    /// ±x, ±y, ±z), pivoting about `c`: the frame in which a tool along `d` comes down −z.
    pub fn tool_along(d: V3, c: V3) -> Option<Pose> {
        let m: [V3; 3] = match d {
            [0.0, 0.0, 1.0] => IDENT,
            [0.0, 0.0, -1.0] => [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
            // +x → +z: a turn about y by −90°
            [1.0, 0.0, 0.0] => [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            [-1.0, 0.0, 0.0] => [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
            // +y → +z: a turn about x by +90°
            [0.0, 1.0, 0.0] => [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
            [0.0, -1.0, 0.0] => [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
            _ => return None,
        };
        let p = Pose { m, t: [0.0; 3] };
        let mc = p.dir(c);
        Some(Pose { m, t: sub(c, mc) })
    }

    pub fn dir(&self, v: V3) -> V3 {
        [dot(self.m[0], v), dot(self.m[1], v), dot(self.m[2], v)]
    }
    pub fn point(&self, x: V3) -> V3 {
        add(self.dir(x), self.t)
    }
    /// self ∘ other (other first).
    pub fn then(&self, after: &Pose) -> Pose {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = (0..3).map(|k| after.m[i][k] * self.m[k][j]).sum();
            }
        }
        Pose {
            m,
            t: after.point(self.t),
        }
    }
}

/// The distance in xy from a point to an axis-aligned rectangle (0 inside).
pub fn xy_dist(p: [f64; 2], lo: [f64; 2], hi: [f64; 2]) -> f64 {
    let dx = (lo[0] - p[0]).max(p[0] - hi[0]).max(0.0);
    let dy = (lo[1] - p[1]).max(p[1] - hi[1]).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prims_bound_grow_and_pose() {
        let b = Prim::aabb([0.0, 0.0, 0.0], [10.0, 20.0, 5.0]);
        assert_eq!(b.bounds(), ([0.0, 0.0, 0.0], [10.0, 20.0, 5.0]));
        assert_eq!(b.grown(1.0).bounds(), ([-1.0, -1.0, -1.0], [11.0, 21.0, 6.0]));
        assert_eq!(b.outside([5.0, 5.0, 2.0]), 0.0);
        assert_eq!(b.outside([13.0, 24.0, 2.0]), 5.0);
        let c = Prim::post(0.0, 0.0, 5.0, 0.0, 10.0);
        assert_eq!(c.bounds(), ([-5.0, -5.0, 0.0], [5.0, 5.0, 10.0]));
        assert_eq!(c.outside([8.0, 0.0, 14.0]), 5.0);
        let p = Pose::quarter_z(1, [100.0, 0.0, 0.0]);
        assert_eq!(p.point([1.0, 0.0, 0.0]), [100.0, 1.0, 0.0]);
        assert_eq!(b.posed(&p).bounds(), ([80.0, 0.0, 0.0], [100.0, 10.0, 5.0]));
        // a tool along +x: the point 10 out along +x from c rises 10 along z
        let t = Pose::tool_along([1.0, 0.0, 0.0], [1.0, 2.0, 3.0]).unwrap();
        assert_eq!(t.point([11.0, 2.0, 3.0]), [1.0, 2.0, 13.0]);
        assert_eq!(t.point([1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
        let u = Pose::tool_along([0.0, -1.0, 0.0], [0.0; 3]).unwrap();
        assert_eq!(u.point([0.0, -4.0, 0.0]), [0.0, 0.0, 4.0]);
        let id = p.then(&Pose::quarter_z(-1, [0.0; 3]));
        assert_eq!(id.m, IDENT);
    }

    #[test]
    fn faces_of_a_cylinder_and_a_block() {
        let c = Prim::post(1.0, 2.0, 3.0, 0.0, 4.0);
        let f = c.faces(7, 1e-9);
        assert_eq!(f.len(), 6);
        assert_eq!(f[0].id, 7);
        // every face's enclosure lies in the cylinder's bound (with its own slack)
        let (lo, hi) = c.bounds();
        for x in &f {
            for k in 0..3 {
                assert!(x.bbox[k].lo >= lo[k] - 1e-6 && x.bbox[k].hi <= hi[k] + 1e-6, "{k} {:?}", x.bbox);
            }
        }
        assert_eq!(Prim::aabb([0.0; 3], [1.0; 3]).faces(0, 1e-9).len(), 6);
    }
}
