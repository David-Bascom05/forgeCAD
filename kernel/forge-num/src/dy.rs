//! Exact dyadic rationals: every f64 is `m · 2^e` with integer `m`, and sums and products of such
//! numbers are again such numbers, so a sign of a polynomial in f64 inputs is computed with no
//! rounding at all and no precondition on the inputs' range.

use crate::int::Big;
use core::cmp::Ordering;

/// An exact dyadic rational `m · 2^e`. The ring operations do not normalise (a trailing zero bit
/// of `m` may stay): they are the fast path, and nothing they return depends on the form.
#[derive(Clone, Debug, Default)]
pub struct Dy {
    pub m: Big,
    pub e: i64,
}

impl Dy {
    #[inline]
    pub fn zero() -> Dy {
        Dy::default()
    }

    /// The f64 exactly (finite inputs only).
    #[inline]
    pub fn from_f64(x: f64) -> Dy {
        assert!(x.is_finite(), "an exact number from a non-finite f64");
        if x == 0.0 {
            return Dy::zero();
        }
        let b = x.to_bits();
        let neg = b >> 63 == 1;
        let ex = ((b >> 52) & 0x7ff) as i64;
        let fr = (b & ((1u64 << 52) - 1)) as i64;
        let (mut m, mut e) = if ex == 0 {
            (fr, -1074)
        } else {
            (fr | (1i64 << 52), ex - 1075)
        };
        let tz = m.trailing_zeros();
        m >>= tz;
        e += tz as i64;
        Dy {
            m: Big::from_i64(if neg { -m } else { m }),
            e,
        }
    }

    #[inline]
    pub fn from_i64(x: i64) -> Dy {
        Dy {
            m: Big::from_i64(x),
            e: 0,
        }
    }

    #[inline]
    pub fn signum(&self) -> i32 {
        self.m.signum()
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.m.is_zero()
    }

    #[inline]
    pub fn neg(&self) -> Dy {
        Dy {
            m: self.m.neg(),
            e: self.e,
        }
    }

    #[inline]
    pub fn abs(&self) -> Dy {
        Dy {
            m: self.m.abs(),
            e: self.e,
        }
    }

    #[inline]
    pub fn add(&self, o: &Dy) -> Dy {
        if self.is_zero() {
            return o.clone();
        }
        if o.is_zero() {
            return self.clone();
        }
        let e = if self.e < o.e { self.e } else { o.e };
        let a = self.m.shl((self.e - e) as u32);
        let b = o.m.shl((o.e - e) as u32);
        Dy { m: a.add(&b), e }
    }

    #[inline]
    pub fn sub(&self, o: &Dy) -> Dy {
        self.add(&o.neg())
    }

    #[inline]
    pub fn mul(&self, o: &Dy) -> Dy {
        if self.is_zero() || o.is_zero() {
            return Dy::zero();
        }
        Dy {
            m: self.m.mul(&o.m),
            e: self.e + o.e,
        }
    }

    /// `self · 2^k`.
    #[inline]
    pub fn scale2(&self, k: i64) -> Dy {
        Dy {
            m: self.m.clone(),
            e: self.e + k,
        }
    }

    #[inline]
    pub fn cmp_dy(&self, o: &Dy) -> Ordering {
        match self.sub(o).signum() {
            1 => Ordering::Greater,
            -1 => Ordering::Less,
            _ => Ordering::Equal,
        }
    }

    /// The nearest f64 or one next to it (for output only, never for a decision): the top 64
    /// bits with a sticky bit, rounded to f64, then scaled.
    #[inline]
    pub fn to_f64_near(&self) -> f64 {
        let (t, s) = self.m.top64();
        let v = t as f64 * pow2(s as i64 + self.e);
        if self.m.signum() < 0 { -v } else { v }
    }
}

/// `2^k` as an f64, through the subnormal and overflow ranges.
#[inline]
pub fn pow2(k: i64) -> f64 {
    if k > 1023 {
        return f64::INFINITY;
    }
    if k >= -1022 {
        return f64::from_bits(((k + 1023) as u64) << 52);
    }
    if k >= -1074 {
        return f64::from_bits(1u64 << (k + 1074));
    }
    // a product of two in range, so values just below the subnormal edge still round there
    if k >= -2000 {
        return pow2(k + 1000) * pow2(-1000);
    }
    0.0
}

/// `a / b` as an f64 (for output only).
#[inline]
pub fn ratio_f64(a: &Dy, b: &Dy) -> f64 {
    let (ta, sa) = a.m.top64();
    let (tb, sb) = b.m.top64();
    let q = ta as f64 / tb as f64;
    let v = q * pow2(sa as i64 + a.e - sb as i64 - b.e);
    if (a.signum() < 0) != (b.signum() < 0) {
        -v
    } else {
        v
    }
}

/// The 2×2 determinant `a d − b c`.
#[inline]
pub fn det2(a: &Dy, b: &Dy, c: &Dy, d: &Dy) -> Dy {
    a.mul(d).sub(&b.mul(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyadic_round_trips() {
        for x in [
            1.0,
            -0.1,
            3.5e-300,
            1e300,
            5e-324,
            core::f64::consts::FRAC_1_SQRT_2,
            12345.678,
        ] {
            assert_eq!(Dy::from_f64(x).to_f64_near(), x);
        }
    }

    #[test]
    fn arithmetic_is_exact() {
        // 0.1 + 0.2 - 0.3 is not zero in dyadics, and is exactly the f64 rounding error
        let s = Dy::from_f64(0.1).add(&Dy::from_f64(0.2)).sub(&Dy::from_f64(0.3));
        assert_eq!(s.signum(), 1);
        let a = Dy::from_f64(1e20);
        let b = Dy::from_f64(1e-20);
        assert_eq!(a.add(&b).sub(&a).cmp_dy(&b), Ordering::Equal);
        assert_eq!(a.mul(&b).signum(), 1);
        assert_eq!(ratio_f64(&Dy::from_f64(3.0), &Dy::from_f64(-4.0)), -0.75);
    }
}
