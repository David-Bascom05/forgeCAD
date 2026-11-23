//! Natural numbers of any size, little-endian base 2^32 limbs, always normalised (no zero limb on
//! top; zero is the empty vector).
//!
//! Written in the style Aeneas takes: plain `while` loops over indices, no closures, no iterator
//! chains, no u128. Every operation is exact.
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Nat {
    pub limbs: Vec<u32>,
}

fn trim(v: &mut Vec<u32>) {
    while !v.is_empty() && v[v.len() - 1] == 0 {
        v.pop();
    }
}

impl Nat {
    pub fn zero() -> Nat {
        Nat { limbs: Vec::new() }
    }

    pub fn from_u64(x: u64) -> Nat {
        let mut v = vec![x as u32, (x >> 32) as u32];
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// The number of bits: 0 for zero, else floor(log2) + 1.
    pub fn bits(&self) -> u64 {
        let n = self.limbs.len();
        if n == 0 {
            return 0;
        }
        let top = self.limbs[n - 1];
        (n as u64 - 1) * 32 + (32 - top.leading_zeros() as u64)
    }

    /// Bit `i` (0 = least significant).
    pub fn bit(&self, i: u64) -> bool {
        let w = (i / 32) as usize;
        if w >= self.limbs.len() {
            return false;
        }
        (self.limbs[w] >> (i % 32)) & 1 == 1
    }

    /// Whether any of the `k` low bits is set.
    pub fn low_bits_nonzero(&self, k: u64) -> bool {
        let full = (k / 32) as usize;
        let mut i = 0;
        while i < full && i < self.limbs.len() {
            if self.limbs[i] != 0 {
                return true;
            }
            i += 1;
        }
        let rem = k % 32;
        if rem > 0 && full < self.limbs.len() {
            let mask = (1u32 << rem) - 1;
            if self.limbs[full] & mask != 0 {
                return true;
            }
        }
        false
    }

    pub fn cmp(&self, o: &Nat) -> Ordering {
        let (a, b) = (self.limbs.len(), o.limbs.len());
        if a != b {
            return a.cmp(&b);
        }
        let mut i = a;
        while i > 0 {
            i -= 1;
            if self.limbs[i] != o.limbs[i] {
                return self.limbs[i].cmp(&o.limbs[i]);
            }
        }
        Ordering::Equal
    }

    pub fn add(&self, o: &Nat) -> Nat {
        let n = self.limbs.len().max(o.limbs.len());
        let mut v = Vec::with_capacity(n + 1);
        let mut carry = 0u64;
        let mut i = 0;
        while i < n {
            let a = if i < self.limbs.len() {
                self.limbs[i] as u64
            } else {
                0
            };
            let b = if i < o.limbs.len() { o.limbs[i] as u64 } else { 0 };
            let s = a + b + carry;
            v.push(s as u32);
            carry = s >> 32;
            i += 1;
        }
        if carry > 0 {
            v.push(carry as u32);
        }
        Nat { limbs: v }
    }

    /// self - o, for self >= o.
    pub fn sub(&self, o: &Nat) -> Nat {
        debug_assert!(self.cmp(o) != Ordering::Less);
        let mut v = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        let mut i = 0;
        while i < self.limbs.len() {
            let a = self.limbs[i] as i64;
            let b = if i < o.limbs.len() { o.limbs[i] as i64 } else { 0 };
            let mut d = a - b - borrow;
            if d < 0 {
                d += 1i64 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            v.push(d as u32);
            i += 1;
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn mul(&self, o: &Nat) -> Nat {
        if self.is_zero() || o.is_zero() {
            return Nat::zero();
        }
        let (n, m) = (self.limbs.len(), o.limbs.len());
        let mut v = vec![0u32; n + m];
        let mut i = 0;
        while i < n {
            let a = self.limbs[i] as u64;
            let mut carry = 0u64;
            let mut j = 0;
            while j < m {
                let t = a * (o.limbs[j] as u64) + (v[i + j] as u64) + carry;
                v[i + j] = t as u32;
                carry = t >> 32;
                j += 1;
            }
            let mut k = i + m;
            while carry > 0 {
                let t = (v[k] as u64) + carry;
                v[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
            i += 1;
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn mul_small(&self, m: u32) -> Nat {
        let mut v = Vec::with_capacity(self.limbs.len() + 1);
        let mut carry = 0u64;
        let mut i = 0;
        while i < self.limbs.len() {
            let t = (self.limbs[i] as u64) * (m as u64) + carry;
            v.push(t as u32);
            carry = t >> 32;
            i += 1;
        }
        if carry > 0 {
            v.push(carry as u32);
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn shl(&self, k: u64) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let w = (k / 32) as usize;
        let b = (k % 32) as u32;
        let mut v = vec![0u32; w];
        if b == 0 {
            let mut i = 0;
            while i < self.limbs.len() {
                v.push(self.limbs[i]);
                i += 1;
            }
        } else {
            let mut carry = 0u32;
            let mut i = 0;
            while i < self.limbs.len() {
                let x = self.limbs[i];
                v.push((x << b) | carry);
                carry = x >> (32 - b);
                i += 1;
            }
            if carry > 0 {
                v.push(carry);
            }
        }
        Nat { limbs: v }
    }

    /// floor(self / 2^k).
    pub fn shr(&self, k: u64) -> Nat {
        let w = (k / 32) as usize;
        if w >= self.limbs.len() {
            return Nat::zero();
        }
        let b = (k % 32) as u32;
        let mut v = Vec::with_capacity(self.limbs.len() - w);
        let mut i = w;
        while i < self.limbs.len() {
            let lo = self.limbs[i] >> b;
            let hi = if b > 0 && i + 1 < self.limbs.len() {
                self.limbs[i + 1] << (32 - b)
            } else {
                0
            };
            v.push(lo | hi);
            i += 1;
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    /// (floor(self / d), self mod d), d > 0. Knuth's algorithm D.
    pub fn divrem(&self, d: &Nat) -> (Nat, Nat) {
        assert!(!d.is_zero(), "division by zero");
        if self.cmp(d) == Ordering::Less {
            return (Nat::zero(), self.clone());
        }
        if d.limbs.len() == 1 {
            let dv = d.limbs[0] as u64;
            let mut q = vec![0u32; self.limbs.len()];
            let mut r = 0u64;
            let mut i = self.limbs.len();
            while i > 0 {
                i -= 1;
                let cur = (r << 32) | self.limbs[i] as u64;
                q[i] = (cur / dv) as u32;
                r = cur % dv;
            }
            trim(&mut q);
            return (Nat { limbs: q }, Nat::from_u64(r));
        }
        // normalise so the divisor's top limb has its top bit set
        let s = d.limbs[d.limbs.len() - 1].leading_zeros() as u64;
        let dn = d.shl(s);
        let mut un = self.shl(s).limbs;
        un.push(0);
        let n = dn.limbs.len();
        let m = un.len() - n - 1;
        let mut q = vec![0u32; m + 1];
        let dtop = dn.limbs[n - 1] as u64;
        let dnext = dn.limbs[n - 2] as u64;
        let b = 1u64 << 32;
        let mut j = m + 1;
        while j > 0 {
            j -= 1;
            let num = ((un[j + n] as u64) << 32) | un[j + n - 1] as u64;
            let mut qhat = num / dtop;
            let mut rhat = num % dtop;
            while qhat >= b || qhat * dnext > ((rhat << 32) | un[j + n - 2] as u64) {
                qhat -= 1;
                rhat += dtop;
                if rhat >= b {
                    break;
                }
            }
            // multiply and subtract
            let mut borrow = 0i64;
            let mut carry = 0u64;
            let mut i = 0;
            while i < n {
                let p = qhat * dn.limbs[i] as u64 + carry;
                carry = p >> 32;
                let t = un[i + j] as i64 - borrow - (p & 0xffff_ffff) as i64;
                if t < 0 {
                    un[i + j] = (t + (1i64 << 32)) as u32;
                    borrow = 1;
                } else {
                    un[i + j] = t as u32;
                    borrow = 0;
                }
                i += 1;
            }
            let t = un[j + n] as i64 - borrow - carry as i64;
            if t < 0 {
                un[j + n] = (t + (1i64 << 32)) as u32;
                // add back
                qhat -= 1;
                let mut c = 0u64;
                let mut i = 0;
                while i < n {
                    let s2 = un[i + j] as u64 + dn.limbs[i] as u64 + c;
                    un[i + j] = s2 as u32;
                    c = s2 >> 32;
                    i += 1;
                }
                un[j + n] = (un[j + n] as u64 + c) as u32;
            } else {
                un[j + n] = t as u32;
            }
            q[j] = qhat as u32;
        }
        trim(&mut q);
        un.truncate(n);
        trim(&mut un);
        let r = Nat { limbs: un }.shr(s);
        (Nat { limbs: q }, r)
    }

    /// floor(sqrt(self)), by Newton's iteration from above.
    pub fn isqrt(&self) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let mut x = Nat::from_u64(1).shl(self.bits().div_ceil(2));
        loop {
            let (q, _) = self.divrem(&x);
            let y = x.add(&q).shr(1);
            if y.cmp(&x) != Ordering::Less {
                return x;
            }
            x = y;
        }
    }

    /// The value as an f64 rounded toward zero, with the exponent it must be scaled by
    /// (value = m * 2^e, m < 2^53): for log estimates only.
    pub fn top53(&self) -> (u64, i64) {
        let b = self.bits();
        if b <= 53 {
            let mut v = 0u64;
            let mut i = self.limbs.len();
            while i > 0 {
                i -= 1;
                v = (v << 32) | self.limbs[i] as u64;
            }
            return (v, 0);
        }
        let s = self.shr(b - 53);
        let (m, _) = s.top53();
        (m, (b - 53) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: u64) -> Nat {
        Nat::from_u64(x)
    }

    #[test]
    fn arithmetic_matches_u64() {
        let xs = [
            0u64,
            1,
            2,
            7,
            0xffff_ffff,
            0x1_0000_0000,
            123_456_789_012,
            u64::MAX >> 3,
        ];
        for &a in &xs {
            for &b in &xs {
                let (x, y) = (n(a), n(b));
                if let Some(c) = a.checked_add(b) {
                    assert_eq!(x.add(&y), n(c));
                }
                if a >= b {
                    assert_eq!(x.sub(&y), n(a - b));
                }
                if let Some(p) = a.checked_mul(b) {
                    assert_eq!(x.mul(&y), n(p));
                }
                if b > 0 {
                    let (q, r) = x.divrem(&y);
                    assert_eq!((q, r), (n(a / b), n(a % b)));
                }
                assert_eq!(x.cmp(&y), a.cmp(&b));
            }
            assert_eq!(n(a).isqrt(), n((a as f64).sqrt() as u64).fix_isqrt(a));
        }
    }

    impl Nat {
        fn fix_isqrt(self, a: u64) -> Nat {
            // the true floor sqrt, by correcting a float estimate
            let mut r = self.limbs.first().copied().unwrap_or(0) as u64
                | ((self.limbs.get(1).copied().unwrap_or(0) as u64) << 32);
            while (r as u128) * (r as u128) > a as u128 {
                r -= 1;
            }
            while ((r + 1) as u128) * ((r + 1) as u128) <= a as u128 {
                r += 1;
            }
            Nat::from_u64(r)
        }
    }

    #[test]
    fn long_division_round_trips() {
        // (a * b + c) / b = a rem c, for multi-limb values
        let mut a = n(0x1234_5678_9abc_def1);
        let mut b = n(0xfedc_ba98_7654_3211);
        for i in 0..20u64 {
            a = a.mul(&n(0x9e37_79b9_7f4a_7c15 ^ i)).add(&n(i));
            b = b.mul(&n(0x7f4a_7c15 + i)).add(&n(3));
            let c = b.sub(&n(1)).shr(i % 7);
            let x = a.mul(&b).add(&c);
            let (q, r) = x.divrem(&b);
            assert_eq!(q, a);
            assert_eq!(r, c);
        }
        let big = n(3).shl(400).add(&n(5));
        let s = big.isqrt();
        assert!(s.mul(&s).cmp(&big) != Ordering::Greater);
        let s1 = s.add(&n(1));
        assert!(s1.mul(&s1).cmp(&big) == Ordering::Greater);
    }
}
