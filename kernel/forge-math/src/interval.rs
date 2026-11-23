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
    pub fn point(v: f64) -> Self {
        Interval { lo: v, hi: v }
    }
    fn out(lo: f64, hi: f64) -> Self {
        Interval {
            lo: lo.next_down(),
            hi: hi.next_up(),
        }
    }
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
    /// sin over the interval: the endpoints' values, and 1 wherever a point (k + 1/2) pi with k even
    /// may lie inside, -1 where one with k odd may.
    pub fn sin(&self) -> Interval {
        self.trig(dmath::sin, 0.5)
    }
    /// cos over the interval: the endpoints' values, and 1 wherever a multiple k pi with k even may
    /// lie inside, -1 where one with k odd may.
    pub fn cos(&self) -> Interval {
        self.trig(dmath::cos, 0.0)
    }
    /// sin and cos over the interval together, each as `sin` and `cos` give it (one dmath call
    /// per end for both).
    pub fn sin_cos(&self) -> (Interval, Interval) {
        let (a, b) = (dmath::sin_cos(self.lo), dmath::sin_cos(self.hi));
        (self.trig_at(a.0, b.0, 0.5), self.trig_at(a.1, b.1, 0.0))
    }
    /// The range of sin or cos (`f`, its extremes at (k + `half`) pi, the even k maxima).
    ///
    /// The contract (proven on verify/ieee754's model by the IEEE work, Dmath.sin_err and cos_err):
    /// for finite |x| <= TRIG_MOST = 2048, dmath's sin and cos are within an absolute TRIG_ERR =
    /// 2^-52 of the true values. So each end's value, pushed out by TRIG_ERR and then rounded
    /// outward, encloses the true one; the error is absolute, not relative (dmath is not within a
    /// few ulps of tiny values: near the zeros, ulps are far below its error). A point (k + half) pi
    /// is counted inside when its enclosure [(k + half) PI, (k + half) PI_UP], rounded outward,
    /// meets the interval: PI < pi < PI_UP, so a point that may be inside is never missed. The
    /// result is clamped to [-1, 1]. Past TRIG_MOST, or across 6 (less than a turn), it is [-1, 1].
    fn trig(&self, f: fn(f64) -> f64, half: f64) -> Interval {
        if self.is_empty() || !(self.lo.is_finite() && self.hi.is_finite()) {
            return self.trig_at(f64::NAN, f64::NAN, half);
        }
        self.trig_at(f(self.lo), f(self.hi), half)
    }
    /// `trig` from the function's values at the ends, `fa` and `fb`.
    fn trig_at(&self, fa: f64, fb: f64, half: f64) -> Interval {
        const WHOLE: Interval = Interval { lo: -1.0, hi: 1.0 };
        let (a, b) = (self.lo, self.hi);
        if self.is_empty() {
            return *self;
        }
        if !(a.is_finite() && b.is_finite()) || a.abs().max(b.abs()) > TRIG_MOST || b - a >= 6.0 {
            return WHOLE;
        }
        let (mut lo, mut hi) = ((fa.min(fb) - TRIG_ERR).next_down(), (fa.max(fb) + TRIG_ERR).next_up());
        // the extremes' indices k with (k + half) pi possibly in [a, b]: |k| <= TRIG_MOST / pi + 1,
        // exact as floats, and (k + half) exact too; the width < 6 leaves at most three
        let first = (a / dmath::PI - half).floor() - 1.0;
        let mut k = first;
        while k <= first + 4.0 {
            let m = k + half;
            // m pi lies in [m PI, m PI_UP] (m >= 0) or [m PI_UP, m PI] (m < 0), each product rounded out
            let (p, q) = ((m * dmath::PI), (m * PI_UP));
            let (pl, ph) = (p.min(q).next_down(), p.max(q).next_up());
            if pl <= b && ph >= a {
                if (k as i64).rem_euclid(2) == 0 {
                    hi = 1.0;
                } else {
                    lo = -1.0;
                }
            }
            k += 1.0;
        }
        Interval {
            lo: lo.max(-1.0),
            hi: hi.min(1.0),
        }
    }
}

/// The float just above pi (dmath::PI is the one just below).
const PI_UP: f64 = 3.141_592_653_589_793_6;
/// dmath::sin and cos's error bound is proven up to here.
const TRIG_MOST: f64 = 2048.0;
/// dmath sin and cos's proven absolute error bound for |x| <= TRIG_MOST: 2^-52.
const TRIG_ERR: f64 = f64::EPSILON;

fn min4(c: [f64; 4]) -> f64 {
    c[0].min(c[1]).min(c[2]).min(c[3])
}
fn max4(c: [f64; 4]) -> f64 {
    c[0].max(c[1]).max(c[2]).max(c[3])
}

impl Add for Interval {
    type Output = Interval;
    fn add(self, o: Interval) -> Interval {
        Interval::out(self.lo + o.lo, self.hi + o.hi)
    }
}
impl Sub for Interval {
    type Output = Interval;
    fn sub(self, o: Interval) -> Interval {
        Interval::out(self.lo - o.hi, self.hi - o.lo)
    }
}
impl Neg for Interval {
    type Output = Interval;
    fn neg(self) -> Interval {
        Interval {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}
impl Mul for Interval {
    type Output = Interval;
    fn mul(self, o: Interval) -> Interval {
        let c = [self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi];
        Interval::out(min4(c), max4(c))
    }
}
impl Mul<f64> for Interval {
    type Output = Interval;
    fn mul(self, s: f64) -> Interval {
        self * Interval::point(s)
    }
}
impl Add<f64> for Interval {
    type Output = Interval;
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

    /// Double-double arithmetic: a reference for sin and cos to ~1e-30, from IEEE + - * only.
    mod dd {
        pub type D = (f64, f64);
        pub fn two_sum(a: f64, b: f64) -> D {
            let s = a + b;
            let v = s - a;
            (s, (a - (s - v)) + (b - v))
        }
        fn split(a: f64) -> (f64, f64) {
            let c = 134_217_729.0 * a;
            let h = c - (c - a);
            (h, a - h)
        }
        pub fn two_prod(a: f64, b: f64) -> D {
            let p = a * b;
            let ((ah, al), (bh, bl)) = (split(a), split(b));
            (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
        }
        pub fn add(a: D, b: D) -> D {
            let (s, e) = two_sum(a.0, b.0);
            let e = e + a.1 + b.1;
            two_sum(s, e)
        }
        pub fn mul(a: D, b: D) -> D {
            let (p, e) = two_prod(a.0, b.0);
            two_sum(p, e + a.0 * b.1 + a.1 * b.0)
        }
        pub fn div_f(a: D, d: f64) -> D {
            let q = a.0 / d;
            let (p, e) = two_prod(q, d);
            let r = ((a.0 - p) - e + a.1) / d;
            two_sum(q, r)
        }
        /// pi/2 in three parts (each the next 53 bits).
        const P1: f64 = crate::dmath::FRAC_PI_2;
        const P2: f64 = 6.123_233_995_736_766e-17;
        const P3: f64 = -1.497_384_904_859_169_8e-33;
        /// x - k pi/2 for k = round(x 2/pi), as a double-double, and k mod 4.
        fn reduce(x: f64) -> (D, i64) {
            let k = (x / P1).round();
            let mut r = two_sum(x, 0.0);
            for p in [P1, P2, P3] {
                let (h, l) = two_prod(k, p);
                r = add(r, (-h, -l));
            }
            (r, (k as i64).rem_euclid(4))
        }
        fn series(r: D, odd: bool) -> D {
            let r2 = mul(r, r);
            let mut term = if odd { r } else { (1.0, 0.0) };
            let mut sum = term;
            let mut n = if odd { 1.0 } else { 0.0 };
            for _ in 0..30 {
                term = div_f(mul(term, r2), -(n + 1.0) * (n + 2.0));
                sum = add(sum, term);
                n += 2.0;
            }
            sum
        }
        pub fn sin(x: f64) -> D {
            let (r, q) = reduce(x);
            let (s, c) = (series(r, true), series(r, false));
            match q {
                0 => s,
                1 => c,
                2 => (-s.0, -s.1),
                _ => (-c.0, -c.1),
            }
        }
        pub fn cos(x: f64) -> D {
            let (r, q) = reduce(x);
            let (s, c) = (series(r, true), series(r, false));
            match q {
                0 => c,
                1 => (-s.0, -s.1),
                2 => (-c.0, -c.1),
                _ => s,
            }
        }
        /// Whether lo <= v <= hi for the double-double v.
        pub fn within(v: D, lo: f64, hi: f64) -> bool {
            (v.0 > lo || (v.0 == lo && v.1 >= 0.0)) && (v.0 < hi || (v.0 == hi && v.1 <= 0.0))
        }
        /// |f - v| in ulps of v.
        pub fn ulps(f: f64, v: D) -> f64 {
            let e = add((f, 0.0), (-v.0, -v.1)).0.abs();
            let u = (v.0.abs().next_up() - v.0.abs()).max(f64::from_bits(1));
            e / u
        }
    }

    /// The test points: dense about 0, the multiples of pi/2 up to TRIG_MOST, and spread between.
    fn trig_points() -> Vec<f64> {
        let mut xs = Vec::new();
        let mut r = Rng::new(7);
        for _ in 0..200_000 {
            xs.push(r.range(-TRIG_MOST, TRIG_MOST));
            xs.push(r.range(-10.0, 10.0));
        }
        let mut k = -(TRIG_MOST / dmath::FRAC_PI_2).floor();
        let mut ks = Vec::new();
        while k * dmath::FRAC_PI_2 <= TRIG_MOST {
            ks.push(k);
            k += if k.abs() < 64.0 { 1.0 } else { 37.0 };
        }
        for k in ks {
            let c = k * dmath::FRAC_PI_2;
            let mut x = c;
            let mut y = c;
            for _ in 0..12 {
                xs.push(x);
                xs.push(y);
                x = x.next_up();
                y = y.next_down();
            }
            for e in [1e-15, 1e-12, 1e-9, 1e-6, 1e-3] {
                xs.push(c + e * c.abs().max(1.0));
                xs.push(c - e * c.abs().max(1.0));
            }
        }
        for e in [
            0.0, 5e-324, 1e-300, 1e-20, 1e-17, 6e-17, 1e-10, 1e-8, 7.45e-9, 7.46e-9,
        ] {
            xs.push(e);
            xs.push(-e);
        }
        xs
    }

    #[test]
    fn trig_within_the_contract() {
        // the reference: sin(1) = 0.84147098480789650665250232163030 (0.8414709848078965 + 1.7768450929355360e-18), and sin of the float nearest
        // pi is pi - PI = 1.2246467991473531772260659322750e-16 to 30 digits
        let s1 = dd::sin(1.0);
        assert_eq!(s1.0, 0.841_470_984_807_896_5);
        assert!((s1.1 - 1.776_845_092_935_536e-18).abs() < 1e-31, "{s1:?}");
        let sp = dd::sin(dmath::PI);
        assert!((sp.0 - 1.224_646_799_147_353_2e-16).abs() < 1e-31, "{sp:?}");
        // the bound TRIG_ERR is (proven on the IEEE model), measured against the double-double
        // reference: absolute, and in ulps for the record
        let (mut worst, mut ulps): (f64, f64) = (0.0, 0.0);
        for x in trig_points() {
            for (f, v) in [(dmath::sin(x), dd::sin(x)), (dmath::cos(x), dd::cos(x))] {
                worst = worst.max(dd::add((f, 0.0), (-v.0, -v.1)).0.abs());
                ulps = ulps.max(dd::ulps(f, v));
            }
        }
        println!("dmath sin/cos: at most {worst:e} off ({ulps} ulps)");
        assert!(worst <= TRIG_ERR, "dmath is {worst:e} off");
    }

    #[test]
    fn trig_encloses_the_true_value() {
        assert_eq!(PI_UP, dmath::PI.next_up());
        let xs = trig_points();
        let mut r = Rng::new(9);
        for (i, &x) in xs.iter().enumerate() {
            // the point itself, and an interval about it of a width from nothing to a turn
            let w = [0.0, 1e-12, 1e-6, 0.1, 1.0, 3.0, 6.5][i % 7] * r.unit();
            for iv in [
                Interval::point(x),
                Interval::new(x - w, x + w),
                Interval::new(x, x + w),
            ] {
                let (s, c) = (iv.sin(), iv.cos());
                for y in [iv.lo, iv.hi, iv.mid()] {
                    assert!(dd::within(dd::sin(y), s.lo, s.hi), "sin {iv:?} at {y}: {s:?}");
                    assert!(dd::within(dd::cos(y), c.lo, c.hi), "cos {iv:?} at {y}: {c:?}");
                }
            }
        }
        for (i, &x) in xs.iter().enumerate().step_by(7) {
            let iv = Interval::new(x, x + [0.0, 1e-9, 0.3, 2.0][i % 4]);
            assert_eq!(iv.sin_cos(), (iv.sin(), iv.cos()), "{iv:?}");
        }
        // sin near 0 (the bound that was 6e-17 high), and the extremes inside
        assert!(Interval::point(0.0).sin().lo <= 0.0);
        assert!(Interval::new(-1e-300, 1e-300).sin().contains(0.0));
        assert_eq!(Interval::new(1.5, 1.6).sin().hi, 1.0);
        assert_eq!(Interval::new(-1.6, -1.5).sin().lo, -1.0);
        assert_eq!(Interval::new(3.1, 3.2).cos().lo, -1.0);
        assert_eq!(Interval::point(dmath::PI).cos().lo, -1.0);
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
