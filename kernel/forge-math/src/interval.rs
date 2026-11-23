use crate::dmath;
use crate::{Point2, Point3};
use core::ops::{Add, Mul, Neg, Sub};

/// A closed range of reals that is guaranteed to contain the true result of
/// every operation applied to it: each bound is pushed outward one ulp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub const EMPTY: Interval = Interval {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    };

    pub fn new(lo: f64, hi: f64) -> Self {
        Interval {
            lo: lo.min(hi),
            hi: lo.max(hi),
        }
    }
    #[inline]
    pub fn point(v: f64) -> Self {
        Interval { lo: v, hi: v }
    }
    #[inline]
    fn out(lo: f64, hi: f64) -> Self {
        Interval {
            lo: lo.next_down(),
            hi: hi.next_up(),
        }
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.lo > self.hi
    }
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }
    pub fn mid(&self) -> f64 {
        0.5 * (self.lo + self.hi)
    }
    pub fn contains(&self, v: f64) -> bool {
        self.lo <= v && v <= self.hi
    }
    pub fn contains_zero(&self) -> bool {
        self.contains(0.0)
    }
    pub fn overlaps(&self, o: &Interval) -> bool {
        self.lo <= o.hi && o.lo <= self.hi
    }
    pub fn hull(&self, o: &Interval) -> Interval {
        Interval {
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
        }
    }
    pub fn include(&mut self, v: f64) {
        self.lo = self.lo.min(v);
        self.hi = self.hi.max(v);
    }
    pub fn grow(&self, d: f64) -> Interval {
        Interval {
            lo: self.lo - d,
            hi: self.hi + d,
        }
    }
    pub fn split(&self) -> (Interval, Interval) {
        let m = self.mid();
        (Interval { lo: self.lo, hi: m }, Interval { lo: m, hi: self.hi })
    }
    pub fn sqr(&self) -> Interval {
        let (a, b) = (self.lo * self.lo, self.hi * self.hi);
        if self.contains_zero() {
            Interval {
                lo: 0.0,
                hi: a.max(b).next_up(),
            }
        } else {
            Interval::out(a.min(b), a.max(b)).clamp_lo(0.0)
        }
    }
    fn clamp_lo(self, v: f64) -> Interval {
        Interval {
            lo: self.lo.max(v),
            hi: self.hi,
        }
    }
    pub fn sqrt(&self) -> Interval {
        Interval::out(self.lo.max(0.0).sqrt(), self.hi.max(0.0).sqrt()).clamp_lo(0.0)
    }
    /// Division by an interval containing zero gives the whole line.
    pub fn div(&self, d: &Interval) -> Interval {
        if d.contains_zero() {
            return Interval {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        let c = [self.lo / d.lo, self.lo / d.hi, self.hi / d.lo, self.hi / d.hi];
        Interval::out(min4(c), max4(c))
    }
    /// atan2 over the box y × x: the angles of the box's points, in (−π, π].
    ///
    /// Contract (dmath::atan2's absolute error bound, 2^-50, proven on verify/ieee754's model by
    /// the IEEE work for finite arguments other than y = −0 with x < 0, which dmath takes to −π):
    /// −0 is normalised to +0 first (the same point), then for finite ends dmath::atan2 at the
    /// corners, each widened by an absolute 2^-50 and rounded outward, clamped to [−PI_UP, PI_UP].
    /// The corners suffice: off the origin and the cut the angle is continuous on the box, which
    /// is convex and seen from the origin within less than a half turn, so its least and greatest
    /// angles are at extreme points (an axis crossing inside it is between two corners' angles).
    /// [−PI_UP, PI_UP] when the box meets the origin or the branch cut (x ≤ 0 with
    /// y.lo ≤ 0 ≤ y.hi), or an end is not finite: refused. Empty for an empty box.
    pub fn atan2(y: Interval, x: Interval) -> Interval {
        /// The float just above pi (dmath::PI is the one just below).
        const PI_UP: f64 = 3.141_592_653_589_793_6;
        /// dmath atan2's proven absolute error bound: 2^-50.
        const ATAN_ERR: f64 = 4.0 * f64::EPSILON;
        let whole = Interval {
            lo: -PI_UP,
            hi: PI_UP,
        };
        if y.is_empty() || x.is_empty() {
            return Interval::EMPTY;
        }
        let finite = [y.lo, y.hi, x.lo, x.hi].iter().all(|v| v.is_finite());
        // the origin and the cut alike: some x <= 0 with y = 0
        if !finite || (x.lo <= 0.0 && y.lo <= 0.0 && 0.0 <= y.hi) {
            return whole;
        }
        // -0 is +0: dmath's -pi at (-0, x < 0) is outside its contract
        let z = |v: f64| v + 0.0;
        let c = [
            dmath::atan2(z(y.lo), z(x.lo)),
            dmath::atan2(z(y.lo), z(x.hi)),
            dmath::atan2(z(y.hi), z(x.lo)),
            dmath::atan2(z(y.hi), z(x.hi)),
        ];
        Interval {
            lo: (min4(c) - ATAN_ERR).next_down().max(-PI_UP),
            hi: (max4(c) + ATAN_ERR).next_up().min(PI_UP),
        }
    }
    /// dmath::sin is within 1 ulp, so the bounds get two extra ulps of slack.
    pub fn sin(&self) -> Interval {
        self.shifted_cos(-dmath::FRAC_PI_2)
    }
    pub fn cos(&self) -> Interval {
        self.shifted_cos(0.0)
    }
    /// cos(x + shift) over the interval: extremes at the endpoints or at
    /// multiples of pi inside it.
    fn shifted_cos(&self, shift: f64) -> Interval {
        // also covers non-finite bounds and ones so large that k += 1 stops counting
        let finite = self.lo.is_finite() && self.hi.is_finite();
        if !finite || self.width() >= dmath::TAU || self.lo.abs().max(self.hi.abs()) > 1e15 {
            return Interval { lo: -1.0, hi: 1.0 };
        }
        let a = self.lo + shift;
        let b = self.hi + shift;
        let (ca, cb) = (dmath::cos(a), dmath::cos(b));
        let mut lo = ca.min(cb);
        let mut hi = ca.max(cb);
        let mut k = (a / dmath::PI).ceil();
        // width < 2pi, so at most three multiples of pi fall inside
        for _ in 0..3 {
            if k * dmath::PI > b {
                break;
            }
            if (k as i64).rem_euclid(2) == 0 {
                hi = 1.0;
            } else {
                lo = -1.0;
            }
            k += 1.0;
        }
        Interval {
            lo: (lo.next_down().next_down().next_down()).max(-1.0),
            hi: (hi.next_up().next_up().next_up()).min(1.0),
        }
    }
}

#[inline]
fn min4(c: [f64; 4]) -> f64 {
    c[0].min(c[1]).min(c[2]).min(c[3])
}
#[inline]
fn max4(c: [f64; 4]) -> f64 {
    c[0].max(c[1]).max(c[2]).max(c[3])
}

impl Add for Interval {
    type Output = Interval;
    #[inline]
    fn add(self, o: Interval) -> Interval {
        Interval::out(self.lo + o.lo, self.hi + o.hi)
    }
}
impl Sub for Interval {
    type Output = Interval;
    #[inline]
    fn sub(self, o: Interval) -> Interval {
        Interval::out(self.lo - o.hi, self.hi - o.lo)
    }
}
impl Neg for Interval {
    type Output = Interval;
    #[inline]
    fn neg(self) -> Interval {
        Interval {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}
impl Mul for Interval {
    type Output = Interval;
    #[inline]
    fn mul(self, o: Interval) -> Interval {
        let c = [self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi];
        Interval::out(min4(c), max4(c))
    }
}
impl Mul<f64> for Interval {
    type Output = Interval;
    #[inline]
    fn mul(self, s: f64) -> Interval {
        self * Interval::point(s)
    }
}
impl Add<f64> for Interval {
    type Output = Interval;
    #[inline]
    fn add(self, s: f64) -> Interval {
        self + Interval::point(s)
    }
}

/// Axis-aligned box in 3D.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub x: Interval,
    pub y: Interval,
    pub z: Interval,
}

impl Box3 {
    pub const EMPTY: Box3 = Box3 {
        x: Interval::EMPTY,
        y: Interval::EMPTY,
        z: Interval::EMPTY,
    };

    pub fn from_point(p: Point3) -> Self {
        Box3 {
            x: Interval::point(p.x),
            y: Interval::point(p.y),
            z: Interval::point(p.z),
        }
    }
    pub fn from_points(ps: impl IntoIterator<Item = Point3>) -> Self {
        let mut b = Box3::EMPTY;
        for p in ps {
            b.include(p);
        }
        b
    }
    pub fn include(&mut self, p: Point3) {
        self.x.include(p.x);
        self.y.include(p.y);
        self.z.include(p.z);
    }
    pub fn is_empty(&self) -> bool {
        self.x.is_empty() || self.y.is_empty() || self.z.is_empty()
    }
    pub fn contains(&self, p: Point3) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y) && self.z.contains(p.z)
    }
    pub fn overlaps(&self, o: &Box3) -> bool {
        self.x.overlaps(&o.x) && self.y.overlaps(&o.y) && self.z.overlaps(&o.z)
    }
    pub fn hull(&self, o: &Box3) -> Box3 {
        Box3 {
            x: self.x.hull(&o.x),
            y: self.y.hull(&o.y),
            z: self.z.hull(&o.z),
        }
    }
    pub fn grow(&self, d: f64) -> Box3 {
        Box3 {
            x: self.x.grow(d),
            y: self.y.grow(d),
            z: self.z.grow(d),
        }
    }
    pub fn center(&self) -> Point3 {
        Point3::new(self.x.mid(), self.y.mid(), self.z.mid())
    }
    pub fn diagonal(&self) -> f64 {
        let (a, b, c) = (self.x.width(), self.y.width(), self.z.width());
        (a * a + b * b + c * c).sqrt()
    }
}

/// Axis-aligned box in a 2D parameter space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub u: Interval,
    pub v: Interval,
}

impl Box2 {
    pub fn new(u: Interval, v: Interval) -> Self {
        Box2 { u, v }
    }
    pub fn contains(&self, p: Point2) -> bool {
        self.u.contains(p.x) && self.v.contains(p.y)
    }
    pub fn center(&self) -> Point2 {
        Point2::new(self.u.mid(), self.v.mid())
    }
    /// Halves the longer side.
    pub fn split(&self) -> (Box2, Box2) {
        if self.u.width() >= self.v.width() {
            let (a, b) = self.u.split();
            (Box2::new(a, self.v), Box2::new(b, self.v))
        } else {
            let (a, b) = self.v.split();
            (Box2::new(self.u, a), Box2::new(self.u, b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    #[test]
    // the platform's atan2 as a reference independent of dmath's
    #[allow(clippy::disallowed_methods)]
    fn atan2_encloses_the_angle_or_refuses() {
        let whole = Interval::new(-3.141_592_653_589_793_6, 3.141_592_653_589_793_6);
        let mut r = Rng::new(21);
        let mut refused = 0;
        for _ in 0..50_000 {
            let y = Interval::new(r.range(-4.0, 4.0), r.range(-4.0, 4.0));
            let x = Interval::new(r.range(-4.0, 4.0), r.range(-4.0, 4.0));
            let a = Interval::atan2(y, x);
            if a == whole {
                refused += 1;
                // refused only where the box meets the origin or the cut
                assert!(x.lo <= 0.0 && y.contains(0.0));
                continue;
            }
            for _ in 0..8 {
                let (py, px) = (r.range(y.lo, y.hi), r.range(x.lo, x.hi));
                assert!(a.contains(py.atan2(px)), "{y:?} {x:?} {a:?}");
            }
            // the ends at least are the true ones within the contract's width
            assert!(a.hi - a.lo < 4.0);
        }
        assert!(refused > 0 && refused < 50_000);
        // -0 is +0: right of the origin, the angle 0, not -pi or a refusal
        let a = Interval::atan2(Interval::new(-0.0, 1.0), Interval::new(1.0, 2.0));
        assert!(a.contains(0.0) && a.lo > -1e-15, "{a:?}");
        // touching the cut from above is the cut: refused
        let a = Interval::atan2(Interval::new(0.0, 1.0), Interval::new(-2.0, -1.0));
        assert_eq!(a, whole);
        // across the cut, and round the origin: the whole range
        assert_eq!(
            Interval::atan2(Interval::new(-1.0, 1.0), Interval::new(-2.0, -1.0)),
            whole
        );
        assert_eq!(
            Interval::atan2(Interval::new(-1.0, 1.0), Interval::new(-1.0, 1.0)),
            whole
        );
        assert_eq!(
            Interval::atan2(Interval::new(-1.0, 0.0), Interval::new(-2.0, -1.0)),
            whole
        );
        // right of the origin, across y = 0: no cut there
        let a = Interval::atan2(Interval::new(-1.0, 1.0), Interval::new(1.0, 2.0));
        assert!(
            a.contains(-core::f64::consts::FRAC_PI_4)
                && a.contains(core::f64::consts::FRAC_PI_4)
                && a.hi < 0.8
        );
        assert!(Interval::atan2(Interval::EMPTY, Interval::point(1.0)).is_empty());
    }

    #[test]
    fn interval_contains_true_values() {
        let mut r = Rng::new(20);
        for _ in 0..50_000 {
            let a = Interval::new(r.range(-10.0, 10.0), r.range(-10.0, 10.0));
            let b = Interval::new(r.range(-10.0, 10.0), r.range(-10.0, 10.0));
            let x = r.range(a.lo, a.hi);
            let y = r.range(b.lo, b.hi);
            assert!((a + b).contains(x + y));
            assert!((a - b).contains(x - y));
            assert!((a * b).contains(x * y));
            assert!(a.sqr().contains(x * x));
            assert!(a.sin().contains(dmath::sin(x)), "{a:?} {x}");
            assert!(a.cos().contains(dmath::cos(x)), "{a:?} {x}");
            if !b.contains_zero() {
                assert!(a.div(&b).contains(x / y));
            }
        }
    }

    #[test]
    fn cos_extrema() {
        let i = Interval::new(-0.1, 0.1).cos();
        assert_eq!(i.hi, 1.0);
        let i = Interval::new(3.0, 3.3).cos();
        assert_eq!(i.lo, -1.0);
        let i = Interval::new(1.0, 1.2).sin();
        assert!(i.hi < 1.0);
    }
}
