//! Signed integers of any size, with a machine fast path.

use crate::nat::Nat;
use core::cmp::Ordering;

/// A signed big integer: inline while it fits under 2¹²⁶ in magnitude (the common case: the
/// kernel's numbers are short dyadics), else its sign and magnitude. One representation per value
/// (a value under 2¹²⁶ is always inline, a magnitude is always normalised), so equality is
/// structural.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Big {
    S(i128),
    L { neg: bool, mag: Nat },
}

impl Default for Big {
    fn default() -> Big {
        Big::S(0)
    }
}

/// Inline values stay below this magnitude, so a sum of two never overflows i128.
pub const SMALL_BITS: u32 = 126;

// The machine-integer builtins the crate uses, each named once (for Aeneas: one place to model).

/// `|v|`, exactly (as u128: `i128::MIN` too).
#[inline]
fn abs128(v: i128) -> u128 {
    v.unsigned_abs()
}

/// The number of leading zero bits of `v`.
#[inline]
fn lz128(v: u128) -> u32 {
    v.leading_zeros()
}

/// `a · b`, or `None` past i128.
#[inline]
fn mul128(a: i128, b: i128) -> Option<i128> {
    a.checked_mul(b)
}

#[inline]
fn fits(v: i128) -> bool {
    abs128(v) < (1u128 << SMALL_BITS)
}

impl Big {
    #[inline]
    pub fn zero() -> Big {
        Big::S(0)
    }

    #[inline]
    pub fn from_i64(x: i64) -> Big {
        Big::S(x as i128)
    }

    /// The value from sign and magnitude, inline if it fits.
    #[inline]
    pub fn from_nat(neg: bool, mag: Nat) -> Big {
        if mag.limbs.len() <= 4 {
            let v = mag.to_u128();
            if v < (1u128 << SMALL_BITS) {
                let v = v as i128;
                return Big::S(if neg { -v } else { v });
            }
        }
        Big::L { neg, mag }
    }

    /// Sign and magnitude.
    #[inline]
    pub fn to_nat(&self) -> (bool, Nat) {
        match self {
            Big::S(v) => (*v < 0, Nat::from_u128(abs128(*v))),
            Big::L { neg, mag } => (*neg, mag.clone()),
        }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        matches!(self, Big::S(0))
    }

    /// -1, 0 or 1.
    #[inline]
    pub fn signum(&self) -> i32 {
        match self {
            Big::S(v) => {
                if *v > 0 {
                    1
                } else if *v < 0 {
                    -1
                } else {
                    0
                }
            }
            Big::L { neg, .. } => {
                if *neg {
                    -1
                } else {
                    1
                }
            }
        }
    }

    #[inline]
    pub fn neg(&self) -> Big {
        match self {
            Big::S(v) => Big::S(-*v),
            Big::L { neg, mag } => Big::L {
                neg: !*neg,
                mag: mag.clone(),
            },
        }
    }

    #[inline]
    pub fn abs(&self) -> Big {
        match self {
            Big::S(v) => Big::S(if *v < 0 { -*v } else { *v }),
            Big::L { mag, .. } => Big::L {
                neg: false,
                mag: mag.clone(),
            },
        }
    }

    #[inline]
    pub fn add(&self, o: &Big) -> Big {
        if let (Big::S(a), Big::S(b)) = (self, o) {
            let s = *a + *b; // both under 2^126: no overflow
            if fits(s) {
                return Big::S(s);
            }
        }
        let (an, am) = self.to_nat();
        let (bn, bm) = o.to_nat();
        if an == bn {
            return Big::from_nat(an, am.add(&bm));
        }
        match am.cmp(&bm) {
            Ordering::Equal => Big::zero(),
            Ordering::Greater => Big::from_nat(an, am.sub(&bm)),
            Ordering::Less => Big::from_nat(bn, bm.sub(&am)),
        }
    }

    #[inline]
    pub fn sub(&self, o: &Big) -> Big {
        self.add(&o.neg())
    }

    #[inline]
    pub fn mul(&self, o: &Big) -> Big {
        if let (Big::S(a), Big::S(b)) = (self, o)
            && let Some(p) = mul128(*a, *b)
            && fits(p)
        {
            return Big::S(p);
        }
        let (an, am) = self.to_nat();
        let (bn, bm) = o.to_nat();
        let m = am.mul(&bm);
        let neg = an != bn && !m.is_zero();
        Big::from_nat(neg, m)
    }

    /// `self · 2^bits`.
    #[inline]
    pub fn shl(&self, bits: u32) -> Big {
        if let Big::S(v) = self {
            if *v == 0 {
                return Big::S(0);
            }
            let used = 128 - lz128(abs128(*v));
            if used + bits < SMALL_BITS {
                return Big::S(*v << bits);
            }
        }
        let (n, m) = self.to_nat();
        Big::from_nat(n, m.shl(bits as u64))
    }

    /// The number of significant bits of the magnitude.
    #[inline]
    pub fn bits(&self) -> u32 {
        match self {
            Big::S(v) => 128 - lz128(abs128(*v)),
            Big::L { mag, .. } => mag.bits() as u32,
        }
    }

    /// The top 64 bits of the magnitude and the shift they stand at: `|self| ≈ top · 2^shift`,
    /// with a sticky low bit for what was dropped (so rounding to f64 sees it).
    #[inline]
    pub fn top64(&self) -> (u64, i32) {
        match self {
            Big::S(v) => {
                let a = abs128(*v);
                let b = 128 - lz128(a);
                if b <= 64 {
                    return (a as u64, 0);
                }
                let s = b - 64;
                let mut t = (a >> s) as u64;
                if a & ((1u128 << s) - 1) != 0 {
                    t |= 1;
                }
                (t, s as i32)
            }
            Big::L { mag, .. } => {
                let b = self.bits();
                let s = b - 64;
                let words = (s / 32) as usize;
                let bits = s % 32;
                let mut acc = 0u128;
                let mut i = 0;
                while i < 3 {
                    if words + i < mag.limbs.len() {
                        acc |= (mag.limbs[words + i] as u128) << (32 * i);
                    }
                    i += 1;
                }
                let mut t = (acc >> bits) as u64;
                let mut dropped = bits > 0 && acc & ((1u128 << bits) - 1) != 0;
                let mut k = 0;
                while !dropped && k < words {
                    dropped = mag.limbs[k] != 0;
                    k += 1;
                }
                if dropped {
                    t |= 1;
                }
                (t, s as i32)
            }
        }
    }

    #[inline]
    pub fn cmp_big(&self, o: &Big) -> Ordering {
        if let (Big::S(a), Big::S(b)) = (self, o) {
            return a.cmp(b);
        }
        match self.sub(o).signum() {
            1 => Ordering::Greater,
            -1 => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng(s: &mut u64) -> u64 {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    }

    /// Values on both sides of the inline bound, through every operation, against i128 wherever
    /// the result fits there, and against the ring laws past it.
    #[test]
    fn inline_and_limbs_agree() {
        let mut s = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..20000 {
            let a = (rng(&mut s) as i64 >> (rng(&mut s) % 63)) as i128;
            let b = (rng(&mut s) as i64 >> (rng(&mut s) % 63)) as i128;
            let k = (rng(&mut s) % 70) as u32;
            let (x, y) = (Big::S(a).shl(k), Big::S(b).shl(70 - k));
            let (xa, yb) = (Big::S(a), Big::S(b));
            assert_eq!(xa.add(&yb), Big::S(a + b));
            assert_eq!(xa.mul(&yb), Big::S(a * b));
            assert_eq!(x.add(&y).sub(&y), x);
            assert_eq!(x.mul(&y).mul(&x), x.mul(&x.mul(&y)));
            assert_eq!(x.sub(&x), Big::zero());
            assert_eq!(x.neg().neg(), x);
            assert_eq!(x.mul(&y).signum(), x.signum() * y.signum());
            assert_eq!(x.cmp_big(&y), x.sub(&y).signum().cmp(&0));
            // one representation per value: a product back under the bound is inline
            let p = x.mul(&y);
            if p.bits() < SMALL_BITS {
                assert!(matches!(p, Big::S(_)));
            }
        }
    }

    #[test]
    fn top64_is_truncation_with_sticky() {
        let x = Big::S(1).shl(200).add(&Big::S(1));
        let (t, s) = x.top64();
        assert_eq!((t, s), ((1u64 << 63) | 1, 137));
        let y = Big::S(0xffff_ffff_ffff_ffffi128 << 10);
        assert_eq!(y.top64(), (0xffff_ffff_ffff_ffff, 10));
    }
}
