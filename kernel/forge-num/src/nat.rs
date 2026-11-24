//! Natural numbers of any size, little-endian base 2^32 limbs, always normalised (no zero limb on
//! top; zero is the empty vector).
//!
//! Written in the style Aeneas takes: plain `while` loops over indices, no closures, no iterator
//! chains. Every operation is exact.
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
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

    pub fn from_u128(x: u128) -> Nat {
        let mut v = vec![x as u32, (x >> 32) as u32, (x >> 64) as u32, (x >> 96) as u32];
        trim(&mut v);
        Nat { limbs: v }
    }

    /// The value, when it has at most 128 bits.
    pub fn to_u128(&self) -> u128 {
        let mut v = 0u128;
        let mut i = self.limbs.len();
        while i > 0 {
            i -= 1;
            v = (v << 32) | self.limbs[i] as u128;
        }
        v
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
        // room for the shifted limbs and a carry, so the pushes below never reallocate
        let mut v = Vec::with_capacity(w + self.limbs.len() + 1);
        v.resize(w, 0u32);
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
    /// The number of trailing zero bits (0 for zero).
    pub fn trailing_zeros(&self) -> u64 {
        let mut i = 0;
        while i < self.limbs.len() {
            if self.limbs[i] != 0 {
                return 32 * i as u64 + self.limbs[i].trailing_zeros() as u64;
            }
            i += 1;
        }
        0
    }

    /// `self.shr(k)` in place (its limbs reused).
    pub fn shr_assign(&mut self, k: u64) {
        let w = (k / 32) as usize;
        if w >= self.limbs.len() {
            self.limbs.clear();
            return;
        }
        let b = (k % 32) as u32;
        let n = self.limbs.len();
        let mut i = w;
        while i < n {
            let lo = self.limbs[i] >> b;
            let hi = if b > 0 && i + 1 < n {
                self.limbs[i + 1] << (32 - b)
            } else {
                0
            };
            self.limbs[i - w] = lo | hi;
            i += 1;
        }
        self.limbs.truncate(n - w);
        trim(&mut self.limbs);
    }

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
    ///
    /// The iteration y = ⌊(x + ⌊n/x⌋)/2⌋ from any x ≥ ⌊√n⌋ stays ≥ ⌊√n⌋ and decreases until
    /// x = ⌊√n⌋, where it first fails to decrease: so the result is ⌊√n⌋ whatever the start, as
    /// long as the start is at least that. The start is (⌊√(n / 4^k)⌋ + 1) · 2^k, k a quarter of
    /// n's bits (recursively): above √n, since ⌊√(n / 4^k)⌋ + 1 > √(n / 4^k), and within a
    /// relative 2^(k − bits/2 + 1) of it, so one or two full-size divisions finish (from
    /// 2^⌈bits/2⌉ it took about log₂ bits). Small values start from 2^⌈bits/2⌉ as before.
    pub fn isqrt(&self) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let b = self.bits();
        if b <= 128 {
            return Nat::from_u128(isqrt_u128(self.to_u128()));
        }
        let mut x = if b > 128 {
            let k = b / 4;
            self.shr(2 * k).isqrt().add(&Nat::from_u64(1)).shl(k)
        } else {
            Nat::from_u64(1).shl(b.div_ceil(2))
        };
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

/// ⌊√v⌋ in machine integers: an f64 estimate pushed above the root (its relative error is
/// under 2^-50, so est · (1 + 2^-40) + 2 is an upper bound), then Newton's step, which from any
/// upper bound decreases to ⌊√v⌋ and stops there (as `Nat::isqrt`).
fn isqrt_u128(v: u128) -> u128 {
    if v < 2 {
        return v;
    }
    let est = (v as f64).sqrt() as u128;
    let mut x = (est + (est >> 40) + 2).min(1u128 << 64);
    loop {
        let y = (x + v / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
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
                if let (Some(q0), Some(r0)) = (a.checked_div(b), a.checked_rem(b)) {
                    let (q, r) = x.divrem(&y);
                    assert_eq!((q, r), (n(q0), n(r0)));
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
        // the seeded start agrees with the plain iteration from 2^⌈bits/2⌉ (the old start)
        let plain = |x: &Nat| {
            let mut r = n(1).shl(x.bits().div_ceil(2));
            loop {
                let (q, _) = x.divrem(&r);
                let y = r.add(&q).shr(1);
                if y.cmp(&r) != Ordering::Less {
                    return r;
                }
                r = y;
            }
        };
        let mut x = n(0x9e37_79b9_7f4a_7c15);
        for i in 0..400u64 {
            x = x.mul(&n(0x7f4a_7c15_9e37_79b9 ^ i)).add(&n(i * 7919));
            // squares, squares less one and plus one: the edges of ⌊√⌋
            let s = x.shr(i % 97);
            let sq = s.mul(&s);
            for v in [x.clone(), sq.clone(), sq.add(&n(1)), sq.sub(&n(1)).add(&n(0))] {
                if v.is_zero() {
                    continue;
                }
                assert_eq!(v.isqrt(), plain(&v), "{i}");
            }
            if !sq.is_zero() {
                assert_eq!(sq.isqrt(), s);
                assert_eq!(sq.sub(&n(1)).isqrt(), s.sub(&n(1)));
            }
            if x.bits() > 3000 {
                x = x.shr(2900);
            }
        }
        let big = n(3).shl(400).add(&n(5));
        let s = big.isqrt();
        assert!(s.mul(&s).cmp(&big) != Ordering::Greater);
        let s1 = s.add(&n(1));
        assert!(s1.mul(&s1).cmp(&big) == Ordering::Greater);
    }

    /// shr_assign and trailing_zeros agree with shr and bit-by-bit counting (Dy::norm uses
    /// them), on numbers of 1 to 6 limbs with runs of zero bits, every shift up to past the end.
    #[test]
    fn shift_in_place_is_shr() {
        let mut seed = 0x2545f4914f6cdd1du64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..300 {
            let len = 1 + (rnd() % 6) as usize;
            let mut limbs: Vec<u32> = (0..len).map(|_| rnd() as u32).collect();
            // zero limbs and low bits at the bottom, sometimes
            let z = (rnd() % 3) as usize;
            for l in limbs.iter_mut().take(z.min(len - 1)) {
                *l = 0;
            }
            limbs[0] &= !((1u32 << (rnd() % 31)) - 1);
            let mut x = Nat { limbs };
            trim(&mut x.limbs);
            if x.is_zero() {
                continue;
            }
            let mut k = 0u64;
            while !x.bit(k) {
                k += 1;
            }
            assert_eq!(x.trailing_zeros(), k);
            let mut sh = 0;
            while sh <= 32 * len as u64 + 33 {
                let mut y = x.clone();
                y.shr_assign(sh);
                assert_eq!(y, x.shr(sh), "{x:?} >> {sh}");
                sh += 1 + rnd() % 7;
            }
        }
    }

    /// The machine-integer root (values of at most 128 bits): s² ≤ v < (s + 1)² for 0..2000,
    /// squares and their neighbours up to the top of u128, and random values.
    #[test]
    fn isqrt_in_machine_integers() {
        let check = |v: u128| {
            let s = isqrt_u128(v);
            let (sn, vn) = (Nat::from_u128(s), Nat::from_u128(v));
            assert!(sn.mul(&sn).cmp(&vn) != Ordering::Greater, "{v}: {s}");
            let s1 = sn.add(&Nat::from_u64(1));
            assert!(s1.mul(&s1).cmp(&vn) == Ordering::Greater, "{v}: {s}");
            assert_eq!(Nat::from_u128(v).isqrt(), sn);
        };
        for v in 0..2000u128 {
            check(v);
        }
        let mut r = 1u128;
        while r < (1u128 << 64) {
            for d in [-1i128, 0, 1] {
                let v = (r * r) as i128 + d;
                if v >= 0 {
                    check(v as u128);
                }
            }
            r = r * 3 + 1;
        }
        check(u128::MAX);
        check(u128::MAX - 1);
        let top = (1u128 << 64) - 1;
        check(top * top);
        check(top * top - 1);
        let mut seed = 0x853c49e6748fea9bu64;
        for _ in 0..2000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let v = ((seed as u128) << 64 | (seed.rotate_left(17) as u128)) >> (seed % 120);
            check(v);
        }
    }
}
