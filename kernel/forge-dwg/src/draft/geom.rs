//! Points, 2D affine maps, boxes and the tolerances drafting works to.

use forge_math::dmath;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

pub const PI: f64 = dmath::PI;
pub const TAU: f64 = dmath::TAU;

/// A point or vector in the plane.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

pub const fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);
    pub fn dot(self, o: V2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    /// The z of the 3D cross product: positive when `o` is counter-clockwise of `self`.
    pub fn cross(self, o: V2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len2(self) -> f64 {
        self.dot(self)
    }
    pub fn len(self) -> f64 {
        self.len2().sqrt()
    }
    pub fn dist(self, o: V2) -> f64 {
        (self - o).len()
    }
    /// A quarter turn counter-clockwise.
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    pub fn unit(self) -> Option<V2> {
        let l = self.len();
        (l > 0.0 && l.is_finite()).then(|| self / l)
    }
    /// The unit vector, or zero.
    pub fn unit_or_zero(self) -> V2 {
        self.unit().unwrap_or(V2::ZERO)
    }
    pub fn angle(self) -> f64 {
        dmath::atan2(self.y, self.x)
    }
    pub fn polar(angle: f64, r: f64) -> V2 {
        let (s, c) = dmath::sin_cos(angle);
        v2(r * c, r * s)
    }
    pub fn lerp(self, o: V2, t: f64) -> V2 {
        self + (o - self) * t
    }
    pub fn mid(self, o: V2) -> V2 {
        v2(0.5 * (self.x + o.x), 0.5 * (self.y + o.y))
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
    pub fn abs_max(self) -> f64 {
        self.x.abs().max(self.y.abs())
    }
    pub fn rotate(self, angle: f64) -> V2 {
        let (s, c) = dmath::sin_cos(angle);
        v2(c * self.x - s * self.y, s * self.x + c * self.y)
    }
    pub fn arr(self) -> [f64; 2] {
        [self.x, self.y]
    }
    pub fn arr3(self, z: f64) -> [f64; 3] {
        [self.x, self.y, z]
    }
}

impl From<[f64; 2]> for V2 {
    fn from(a: [f64; 2]) -> V2 {
        v2(a[0], a[1])
    }
}
impl From<[f64; 3]> for V2 {
    fn from(a: [f64; 3]) -> V2 {
        v2(a[0], a[1])
    }
}
impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        *self = *self + o;
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, o: V2) {
        *self = *self - o;
    }
}
impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        v2(self.x * k, self.y * k)
    }
}
impl Div<f64> for V2 {
    type Output = V2;
    fn div(self, k: f64) -> V2 {
        v2(self.x / k, self.y / k)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}

/// An angle in [0, 2pi).
pub fn wrap(a: f64) -> f64 {
    dmath::wrap_tau(a)
}

/// An angle in (-pi, pi].
pub fn wrap_pi(a: f64) -> f64 {
    let w = wrap(a);
    if w > PI { w - TAU } else { w }
}

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bbox {
    pub min: V2,
    pub max: V2,
}

impl Bbox {
    pub const EMPTY: Bbox = Bbox {
        min: v2(f64::INFINITY, f64::INFINITY),
        max: v2(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };
    pub fn of(pts: impl IntoIterator<Item = V2>) -> Bbox {
        let mut b = Bbox::EMPTY;
        for p in pts {
            b.add(p);
        }
        b
    }
    pub fn add(&mut self, p: V2) {
        self.min = v2(self.min.x.min(p.x), self.min.y.min(p.y));
        self.max = v2(self.max.x.max(p.x), self.max.y.max(p.y));
    }
    pub fn union(self, o: Bbox) -> Bbox {
        Bbox {
            min: v2(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            max: v2(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        }
    }
    pub fn is_empty(&self) -> bool {
        !(self.min.x <= self.max.x && self.min.y <= self.max.y)
    }
    pub fn grow(self, d: f64) -> Bbox {
        Bbox {
            min: self.min - v2(d, d),
            max: self.max + v2(d, d),
        }
    }
    pub fn overlaps(&self, o: &Bbox) -> bool {
        self.min.x <= o.max.x && o.min.x <= self.max.x && self.min.y <= o.max.y && o.min.y <= self.max.y
    }
    pub fn contains(&self, p: V2) -> bool {
        self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y
    }
    /// The larger side (0 for a point; infinite for unbounded boxes).
    pub fn size(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            (self.max.x - self.min.x).max(self.max.y - self.min.y)
        }
    }
    /// The largest coordinate magnitude.
    pub fn reach(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.min.abs_max().max(self.max.abs_max())
        }
    }
    /// The distance from a point to the box (0 inside).
    pub fn dist(&self, p: V2) -> f64 {
        let dx = (self.min.x - p.x).max(p.x - self.max.x).max(0.0);
        let dy = (self.min.y - p.y).max(p.y - self.max.y).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
    pub fn center(&self) -> V2 {
        self.min.mid(self.max)
    }
}

/// The tolerances an operation works to, from the size of the geometry it works on: lengths
/// under `lin` are zero, and two points closer than it are the same.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tol {
    /// Length (drawing units).
    pub lin: f64,
    /// Angle (radians).
    pub ang: f64,
}

impl Tol {
    /// For geometry of `size` (its larger extent) at most `reach` from the origin: a part in 1e9 of
    /// the size, but not below what f64 resolves at that distance from the origin.
    pub fn for_size(size: f64, reach: f64) -> Tol {
        let s = if size.is_finite() { size } else { reach };
        let lin = (s * 1e-9).max(reach * 256.0 * f64::EPSILON).max(1e-12);
        Tol { lin, ang: 1e-10 }
    }
    pub fn of_box(b: &Bbox) -> Tol {
        let finite = Bbox {
            min: v2(clampf(b.min.x), clampf(b.min.y)),
            max: v2(clampf(b.max.x), clampf(b.max.y)),
        };
        Tol::for_size(finite.size(), finite.reach())
    }
}

fn clampf(x: f64) -> f64 {
    if x.is_finite() { x } else { 0.0 }
}

/// A 2D affine map: p -> (a x + b y + tx, c x + d y + ty).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub m: [[f64; 2]; 2],
    pub t: V2,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        m: [[1.0, 0.0], [0.0, 1.0]],
        t: V2::ZERO,
    };
    pub fn translate(d: V2) -> Affine {
        Affine {
            t: d,
            ..Affine::IDENTITY
        }
    }
    /// A turn of `angle` (counter-clockwise) about `about`.
    pub fn rotate(about: V2, angle: f64) -> Affine {
        let (s, c) = dmath::sin_cos(angle);
        let m = [[c, -s], [s, c]];
        Affine::linear_about(m, about)
    }
    /// Uniform scale by `k` about `base`.
    pub fn scale(base: V2, k: f64) -> Affine {
        Affine::linear_about([[k, 0.0], [0.0, k]], base)
    }
    /// Scale by `kx`, `ky` along the axes about `base`.
    pub fn scale_xy(base: V2, kx: f64, ky: f64) -> Affine {
        Affine::linear_about([[kx, 0.0], [0.0, ky]], base)
    }
    /// The reflection in the line through `a` and `b`.
    pub fn mirror(a: V2, b: V2) -> Affine {
        let u = (b - a).unit_or_zero();
        let (c2, s2) = (u.x * u.x - u.y * u.y, 2.0 * u.x * u.y);
        Affine::linear_about([[c2, s2], [s2, -c2]], a)
    }
    fn linear_about(m: [[f64; 2]; 2], p: V2) -> Affine {
        let l = Affine { m, t: V2::ZERO };
        Affine { m, t: p - l.vec(p) }
    }
    pub fn apply(&self, p: V2) -> V2 {
        self.vec(p) + self.t
    }
    /// The linear part on a vector.
    pub fn vec(&self, v: V2) -> V2 {
        v2(
            self.m[0][0] * v.x + self.m[0][1] * v.y,
            self.m[1][0] * v.x + self.m[1][1] * v.y,
        )
    }
    pub fn det(&self) -> f64 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }
    /// `self` after `o`.
    pub fn then(&self, o: &Affine) -> Affine {
        // o(self(p))
        let a = &o.m;
        let b = &self.m;
        let m = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        Affine {
            m,
            t: o.apply(self.t),
        }
    }
    pub fn inverse(&self) -> Option<Affine> {
        let d = self.det();
        if d == 0.0 || !d.is_finite() {
            return None;
        }
        let m = [
            [self.m[1][1] / d, -self.m[0][1] / d],
            [-self.m[1][0] / d, self.m[0][0] / d],
        ];
        let l = Affine { m, t: V2::ZERO };
        Some(Affine { m, t: -l.vec(self.t) })
    }
    /// The uniform scale when the map is a similarity (a rotation, possibly a reflection, and a
    /// uniform scale) to a relative `tol`.
    pub fn similarity_scale(&self, tol: f64) -> Option<f64> {
        let c0 = v2(self.m[0][0], self.m[1][0]);
        let c1 = v2(self.m[0][1], self.m[1][1]);
        let (l0, l1) = (c0.len(), c1.len());
        let k = l0.max(l1);
        if k == 0.0 {
            return None;
        }
        ((l0 - l1).abs() <= tol * k && c0.dot(c1).abs() <= tol * k * k).then_some(0.5 * (l0 + l1))
    }
    pub fn is_reflection(&self) -> bool {
        self.det() < 0.0
    }
}

/// The arbitrary-axis algorithm: the OCS x and y axes for an extrusion (normal) direction.
pub fn ocs_axes(n: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = if l > 0.0 {
        [n[0] / l, n[1] / l, n[2] / l]
    } else {
        [0.0, 0.0, 1.0]
    };
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let ax = if n[0].abs() < 1.0 / 64.0 && n[1].abs() < 1.0 / 64.0 {
        cross([0.0, 1.0, 0.0], n)
    } else {
        cross([0.0, 0.0, 1.0], n)
    };
    let la = (ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]).sqrt();
    let ax = [ax[0] / la, ax[1] / la, ax[2] / la];
    let ay = cross(n, ax);
    (ax, ay, n)
}

/// Solves the 2x2 system [a b; c d] x = [e f]; None when singular (relative to `eps`).
pub fn solve2(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, eps: f64) -> Option<(f64, f64)> {
    let det = a * d - b * c;
    let scale = (a.abs() + b.abs()) * (c.abs() + d.abs());
    if det.abs() <= eps * scale || det == 0.0 {
        return None;
    }
    Some(((e * d - b * f) / det, (a * f - e * c) / det))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_maps() {
        let r = Affine::rotate(v2(1.0, 1.0), PI / 2.0);
        let p = r.apply(v2(2.0, 1.0));
        assert!(p.dist(v2(1.0, 2.0)) < 1e-12);
        let m = Affine::mirror(v2(0.0, 0.0), v2(1.0, 1.0));
        assert!(m.apply(v2(1.0, 0.0)).dist(v2(0.0, 1.0)) < 1e-12);
        assert!(m.is_reflection());
        let s = Affine::scale(v2(1.0, 0.0), 2.0);
        assert!(s.apply(v2(2.0, 0.0)).dist(v2(3.0, 0.0)) < 1e-12);
        let c = r.then(&s);
        let inv = c.inverse().unwrap();
        let q = v2(3.5, -2.0);
        assert!(inv.apply(c.apply(q)).dist(q) < 1e-12);
        assert!((c.similarity_scale(1e-12).unwrap() - 2.0).abs() < 1e-12);
        assert!(
            Affine::scale_xy(V2::ZERO, 1.0, 2.0)
                .similarity_scale(1e-9)
                .is_none()
        );
    }

    #[test]
    fn arbitrary_axis() {
        let (ax, ay, _) = ocs_axes([0.0, 0.0, 1.0]);
        assert_eq!((ax, ay), ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
        let (ax, ay, _) = ocs_axes([0.0, 0.0, -1.0]);
        assert_eq!(ax, [-1.0, 0.0, 0.0]);
        assert_eq!(ay, [0.0, 1.0, 0.0]);
    }
}
