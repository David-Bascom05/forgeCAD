//! One set of formulas for points and for enclosures: a scalar that is either a double or an
//! outward-rounded interval (forge-math `Interval`), with the bend's stable trigonometry.
use core::ops::{Add, Mul, Neg, Sub};
use forge_math::{Interval, dmath};

/// A scalar the fold formulas run on: `f64` for points, `Interval` for enclosures.
pub trait Sc:
    Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Neg<Output = Self>
    + Add<f64, Output = Self>
    + Mul<f64, Output = Self>
{
    fn c(v: f64) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn sqr(self) -> Self;
    /// 1 / x (x away from 0).
    fn recip(self) -> Self;
    /// sin x / x (1 at 0).
    fn sinc(self) -> Self;
    /// (1 - cos x) / x (0 at 0), as x/2 sinc(x/2)^2 so it is exact near 0.
    fn vsc(self) -> Self {
        let h = self * 0.5;
        h * h.sinc().sqr()
    }
    /// 1 - cos x, as 2 sin(x/2)^2.
    fn vers(self) -> Self {
        (self * 0.5).sin().sqr() * 2.0
    }
}

/// Values of sinc below this use the series.
const SERIES: f64 = 1e-4;

pub fn sinc(x: f64) -> f64 {
    if x.abs() < SERIES {
        1.0 - x * x / 6.0
    } else {
        dmath::sin(x) / x
    }
}

impl Sc for f64 {
    fn c(v: f64) -> f64 {
        v
    }
    fn sin(self) -> f64 {
        dmath::sin(self)
    }
    fn cos(self) -> f64 {
        dmath::cos(self)
    }
    fn sqr(self) -> f64 {
        self * self
    }
    fn recip(self) -> f64 {
        1.0 / self
    }
    fn sinc(self) -> f64 {
        sinc(self)
    }
}

/// Absolute slack on interval sine and cosine.
const TRIG_SLACK: f64 = 1e-15;

fn widen(i: Interval, by: f64, cap: f64) -> Interval {
    Interval {
        lo: (i.lo - by).max(-cap),
        hi: (i.hi + by).min(cap),
    }
}

/// sinc's error from dmath's sine (1 ulp), the division and the series' remainder, widened.
const SINC_SLACK: f64 = 1e-15;

impl Sc for Interval {
    fn c(v: f64) -> Interval {
        Interval::point(v)
    }
    // forge-math's interval sine is a shifted cosine, whose rounding (of pi/2) is absolute, about
    // 6e-17: past its ulp slack near 0. Both widened by TRIG_SLACK.
    fn sin(self) -> Interval {
        widen(Interval::sin(&self), TRIG_SLACK, 1.0)
    }
    fn cos(self) -> Interval {
        widen(Interval::cos(&self), TRIG_SLACK, 1.0)
    }
    fn sqr(self) -> Interval {
        Interval::sqr(&self)
    }
    fn recip(self) -> Interval {
        Interval::point(1.0).div(&self)
    }
    fn sinc(self) -> Interval {
        // even, and decreasing in |x| on [0, pi]
        let (a, b) = (self.lo.abs(), self.hi.abs());
        let big = a.max(b);
        let small = if self.contains_zero() { 0.0 } else { a.min(b) };
        if big <= dmath::PI {
            Interval {
                lo: sinc(big) - SINC_SLACK,
                hi: (sinc(small) + SINC_SLACK).min(1.0),
            }
        } else if !self.contains_zero() {
            Interval::sin(&self).div(&self)
        } else {
            // |sin x / x| <= 1 everywhere; sinc >= -0.2173 (its least value)
            Interval { lo: -0.22, hi: 1.0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enclosures_hold_the_points() {
        for k in 0..200 {
            let lo = k as f64 * 0.0157;
            let w = 0.003 * (1 + k % 7) as f64;
            let i = Interval::new(lo, lo + w);
            for s in 0..=10 {
                let x = lo + w * s as f64 / 10.0;
                assert!(i.sinc().contains(x.sinc()), "sinc {x}");
                assert!(i.vsc().contains(x.vsc()), "vsc {x}");
                assert!(i.vers().contains(x.vers()), "vers {x}");
            }
        }
    }

    #[test]
    fn stable_at_zero() {
        assert_eq!(0.0f64.sinc(), 1.0);
        assert_eq!(0.0f64.vsc(), 0.0);
        let x = 1e-9f64;
        assert!((x.vsc() - x / 2.0).abs() < 1e-24);
    }
}
