//! f64 intervals with outward rounding: every operation's true result lies in the interval it
//! returns. IEEE operations are correctly rounded, so one ulp outward on each side suffices.
//!
//! This is the filter in front of the exact sign, and the arithmetic of the clearance checker.
//! An empty or undefined result is the whole line ([-inf, inf]): never a narrower wrong answer.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ival {
    pub lo: f64,
    pub hi: f64,
}

pub const WHOLE: Ival = Ival {
    lo: f64::NEG_INFINITY,
    hi: f64::INFINITY,
};

#[inline]
fn down(x: f64) -> f64 {
    if x.is_nan() {
        f64::NEG_INFINITY
    } else {
        x.next_down()
    }
}
#[inline]
fn up(x: f64) -> f64 {
    if x.is_nan() { f64::INFINITY } else { x.next_up() }
}

#[inline]
fn min4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    if b < m {
        m = b;
    }
    if c < m {
        m = c;
    }
    if d < m {
        m = d;
    }
    m
}
#[inline]
fn max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    if b > m {
        m = b;
    }
    if c > m {
        m = c;
    }
    if d > m {
        m = d;
    }
    m
}

impl Ival {
    /// An exact point (any f64 is an exact real).
    #[inline]
    pub fn pt(x: f64) -> Ival {
        Ival { lo: x, hi: x }
    }
    /// [a, b] for a <= b, exactly.
    #[inline]
    pub fn new(a: f64, b: f64) -> Ival {
        if a <= b {
            Ival { lo: a, hi: b }
        } else {
            Ival { lo: b, hi: a }
        }
    }
    fn fix(lo: f64, hi: f64) -> Ival {
        if lo.is_nan() || hi.is_nan() {
            return WHOLE;
        }
        Ival { lo, hi }
    }
    #[inline]
    pub fn add(self, o: Ival) -> Ival {
        let lo = if self.lo == f64::NEG_INFINITY || o.lo == f64::NEG_INFINITY {
            f64::NEG_INFINITY
        } else {
            down(self.lo + o.lo)
        };
        let hi = if self.hi == f64::INFINITY || o.hi == f64::INFINITY {
            f64::INFINITY
        } else {
            up(self.hi + o.hi)
        };
        Ival::fix(lo, hi)
    }
    #[inline]
    pub fn neg(self) -> Ival {
        Ival {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    #[inline]
    pub fn sub(self, o: Ival) -> Ival {
        self.add(o.neg())
    }
    #[inline]
    pub fn mul(self, o: Ival) -> Ival {
        if self.lo == self.hi && o.lo == o.hi {
            let p = self.lo * o.lo;
            if p.is_nan() {
                return WHOLE;
            }
            // exact when the product is exact; else one ulp each way
            return Ival::fix(down(p), up(p));
        }
        let (a, b, c, d) = (self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi);
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
            return WHOLE;
        }
        Ival::fix(down(min4(a, b, c, d)), up(max4(a, b, c, d)))
    }
    #[inline]
    pub fn scale(self, k: f64) -> Ival {
        self.mul(Ival::pt(k))
    }
    /// The square, which is never negative.
    #[inline]
    pub fn sqr(self) -> Ival {
        let (a, b) = (self.lo.abs(), self.hi.abs());
        let (mut m, mut x) = if a < b { (a, b) } else { (b, a) };
        if self.lo <= 0.0 && self.hi >= 0.0 {
            m = 0.0;
        }
        if x.is_nan() {
            x = f64::INFINITY;
        }
        let lo = if m == 0.0 { 0.0 } else { down(m * m).max(0.0) };
        Ival::fix(lo, up(x * x))
    }
    #[inline]
    pub fn div(self, o: Ival) -> Ival {
        if o.lo <= 0.0 && o.hi >= 0.0 {
            return WHOLE;
        }
        let (a, b, c, d) = (self.lo / o.lo, self.lo / o.hi, self.hi / o.lo, self.hi / o.hi);
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
            return WHOLE;
        }
        Ival::fix(down(min4(a, b, c, d)), up(max4(a, b, c, d)))
    }
    /// The square root of the non-negative part (the argument is non-negative by construction
    /// wherever this is called on an exact value).
    #[inline]
    pub fn sqrt(self) -> Ival {
        let lo = if self.lo <= 0.0 {
            0.0
        } else {
            down(self.lo.sqrt()).max(0.0)
        };
        let hi = if self.hi < 0.0 { 0.0 } else { up(self.hi.sqrt()) };
        Ival::fix(lo, hi)
    }
    #[inline]
    pub fn min(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo < o.lo { self.lo } else { o.lo },
            hi: if self.hi < o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn max(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo > o.lo { self.lo } else { o.lo },
            hi: if self.hi > o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn abs(self) -> Ival {
        if self.lo >= 0.0 {
            self
        } else if self.hi <= 0.0 {
            self.neg()
        } else {
            Ival {
                lo: 0.0,
                hi: if -self.lo > self.hi { -self.lo } else { self.hi },
            }
        }
    }
    #[inline]
    pub fn hull(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo < o.lo { self.lo } else { o.lo },
            hi: if self.hi > o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn mid(self) -> f64 {
        if self.lo == f64::NEG_INFINITY || self.hi == f64::INFINITY {
            return 0.0;
        }
        0.5 * self.lo + 0.5 * self.hi
    }
    #[inline]
    pub fn width(self) -> f64 {
        if self.hi == self.lo {
            return 0.0;
        }
        up(self.hi - self.lo)
    }
    /// -1, 0 (undecided: zero inside) or 1.
    #[inline]
    pub fn sign(self) -> i32 {
        if self.lo > 0.0 {
            1
        } else if self.hi < 0.0 {
            -1
        } else {
            0
        }
    }
    #[inline]
    pub fn split(self) -> (Ival, Ival) {
        let m = self.mid();
        (Ival { lo: self.lo, hi: m }, Ival { lo: m, hi: self.hi })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encloses() {
        let a = Ival::pt(0.1);
        let b = Ival::pt(0.2);
        let s = a.add(b);
        assert!(s.lo <= 0.30000000000000004 && s.hi >= 0.3);
        let q = Ival::pt(2.0).sqrt();
        assert!(q.lo * q.lo <= 2.0 && q.hi * q.hi >= 2.0);
        assert_eq!(Ival::new(-1.0, 2.0).sqr().lo, 0.0);
        assert_eq!(Ival::pt(1.0).div(Ival::new(-1.0, 1.0)), WHOLE);
    }
}
