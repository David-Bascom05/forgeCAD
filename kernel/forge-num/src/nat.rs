//! Natural numbers of any size, little-endian base 2^32 limbs, always normalised (no zero limb on
//! top; zero is the empty vector).
//!
//! Written in the style Aeneas takes: plain `while` loops over indices, no closures, no iterator
//! chains. Every operation is exact.
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Nat {
    pub limbs: Limbs,
}

/// How many limbs a [`Nat`] holds inline (no allocation): 512 bits.
pub const INLINE: usize = 16;

/// A natural's limbs: inline up to [`INLINE`], on the heap past it. Exact arithmetic at the
/// precisions the exact reals start at (128 bits, products to a few hundred) then allocates
/// nothing; it reads as a slice either way.
#[derive(Clone)]
pub enum Limbs {
    In { len: u8, buf: [u32; INLINE] },
    Heap(Vec<u32>),
}

impl Limbs {
    #[inline]
    pub fn new() -> Limbs {
        Limbs::In {
            len: 0,
            buf: [0; INLINE],
        }
    }

    #[inline]
    pub fn with_capacity(n: usize) -> Limbs {
        if n <= INLINE {
            Limbs::new()
        } else {
            Limbs::Heap(Vec::with_capacity(n))
        }
    }

    /// `n` zero limbs.
    #[inline]
    pub fn zeroed(n: usize) -> Limbs {
        if n <= INLINE {
            Limbs::In {
                len: n as u8,
                buf: [0; INLINE],
            }
        } else {
            Limbs::Heap(vec![0; n])
        }
    }

    #[inline]
    pub fn from_slice(v: &[u32]) -> Limbs {
        let mut l = Limbs::with_capacity(v.len());
        let mut i = 0;
        while i < v.len() {
            l.push(v[i]);
            i += 1;
        }
        l
    }

    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        match self {
            Limbs::In { len, buf } => &buf[..*len as usize],
            Limbs::Heap(v) => v,
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        match self {
            Limbs::In { len, buf } => &mut buf[..*len as usize],
            Limbs::Heap(v) => v,
        }
    }

    #[inline]
    pub fn push(&mut self, x: u32) {
        match self {
            Limbs::In { len, buf } => {
                let n = *len as usize;
                if n < INLINE {
                    buf[n] = x;
                    *len += 1;
                } else {
                    let mut v = Vec::with_capacity(2 * INLINE);
                    v.extend_from_slice(&buf[..]);
                    v.push(x);
                    *self = Limbs::Heap(v);
                }
            }
            Limbs::Heap(v) => v.push(x),
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<u32> {
        match self {
            Limbs::In { len, buf } => {
                if *len == 0 {
                    None
                } else {
                    *len -= 1;
                    Some(buf[*len as usize])
                }
            }
            Limbs::Heap(v) => v.pop(),
        }
    }

    #[inline]
    pub fn truncate(&mut self, n: usize) {
        match self {
            Limbs::In { len, .. } => {
                if n < *len as usize {
                    *len = n as u8;
                }
            }
            Limbs::Heap(v) => v.truncate(n),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0)
    }

    /// The limbs as a vector (a copy).
    pub fn to_vec(&self) -> Vec<u32> {
        self.as_slice().to_vec()
    }
}

impl Default for Limbs {
    #[inline]
    fn default() -> Limbs {
        Limbs::new()
    }
}

impl core::ops::Deref for Limbs {
    type Target = [u32];
    #[inline]
    fn deref(&self) -> &[u32] {
        self.as_slice()
    }
}

impl core::ops::DerefMut for Limbs {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u32] {
        self.as_mut_slice()
    }
}

impl PartialEq for Limbs {
    #[inline]
    fn eq(&self, o: &Limbs) -> bool {
        self.as_slice() == o.as_slice()
    }
}

impl Eq for Limbs {}

impl core::hash::Hash for Limbs {
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.as_slice().hash(h)
    }
}

impl core::fmt::Debug for Limbs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

fn trim(v: &mut Limbs) {
    while !v.is_empty() && v[v.len() - 1] == 0 {
        v.pop();
    }
}

impl Nat {
    pub fn zero() -> Nat {
        Nat { limbs: Limbs::new() }
    }

    pub fn from_u64(x: u64) -> Nat {
        let mut v = Limbs::from_slice(&[x as u32, (x >> 32) as u32]);
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn from_u128(x: u128) -> Nat {
        let mut v = Limbs::from_slice(&[x as u32, (x >> 32) as u32, (x >> 64) as u32, (x >> 96) as u32]);
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
        let (a, b) = (self.limbs.as_slice(), o.limbs.as_slice());
        if a.len() != b.len() {
            return a.len().cmp(&b.len());
        }
        let mut i = a.len();
        while i > 0 {
            i -= 1;
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }
        Ordering::Equal
    }

    pub fn add(&self, o: &Nat) -> Nat {
        // the longer operand first
        let (a, b) = if self.limbs.len() >= o.limbs.len() {
            (self.limbs.as_slice(), o.limbs.as_slice())
        } else {
            (o.limbs.as_slice(), self.limbs.as_slice())
        };
        let (n, m) = (a.len(), b.len());
        let mut v = Limbs::zeroed(n + 1);
        {
            let out = v.as_mut_slice();
            let mut carry = 0u64;
            let mut i = 0;
            while i < m {
                let t = a[i] as u64 + b[i] as u64 + carry;
                out[i] = t as u32;
                carry = t >> 32;
                i += 1;
            }
            while i < n {
                let t = a[i] as u64 + carry;
                out[i] = t as u32;
                carry = t >> 32;
                i += 1;
            }
            out[n] = carry as u32;
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    /// self - o, for self >= o.
    pub fn sub(&self, o: &Nat) -> Nat {
        debug_assert!(self.cmp(o) != Ordering::Less);
        let (a, b) = (self.limbs.as_slice(), o.limbs.as_slice());
        let (n, m) = (a.len(), b.len());
        let mut v = Limbs::zeroed(n);
        {
            let out = v.as_mut_slice();
            // t wraps: its top bit is the borrow, its low half the limb
            let mut borrow = 0u64;
            let mut i = 0;
            while i < m {
                let t = (a[i] as u64).wrapping_sub(b[i] as u64).wrapping_sub(borrow);
                out[i] = t as u32;
                borrow = t >> 63;
                i += 1;
            }
            while i < n {
                let t = (a[i] as u64).wrapping_sub(borrow);
                out[i] = t as u32;
                borrow = t >> 63;
                i += 1;
            }
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn mul(&self, o: &Nat) -> Nat {
        if self.is_zero() || o.is_zero() {
            return Nat::zero();
        }
        let (a, b) = (self.limbs.as_slice(), o.limbs.as_slice());
        let (n, m) = (a.len(), b.len());
        let mut v = Limbs::zeroed(n + m);
        {
            let out = v.as_mut_slice();
            let mut i = 0;
            while i < n {
                let ai = a[i] as u64;
                // row i adds a[i]·b at out[i..]: out[i + m] is still zero, so the carry lands there
                let row = &mut out[i..i + m + 1];
                let mut carry = 0u64;
                let mut j = 0;
                while j < m {
                    let t = ai * (b[j] as u64) + (row[j] as u64) + carry;
                    row[j] = t as u32;
                    carry = t >> 32;
                    j += 1;
                }
                row[m] = carry as u32;
                i += 1;
            }
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    pub fn mul_small(&self, m: u32) -> Nat {
        let mut v = Limbs::with_capacity(self.limbs.len() + 1);
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
        let n = self.limbs.len();
        // the shifted limbs and a carry limb, written in place (dropped below when zero)
        let mut v = Limbs::zeroed(w + n + 1);
        {
            let a = self.limbs.as_slice();
            let o = v.as_mut_slice();
            if b == 0 {
                o[w..w + n].copy_from_slice(a);
            } else {
                let mut carry = 0u32;
                let mut i = 0;
                while i < n {
                    let x = a[i];
                    o[w + i] = (x << b) | carry;
                    carry = x >> (32 - b);
                    i += 1;
                }
                o[w + n] = carry;
            }
        }
        trim(&mut v);
        Nat { limbs: v }
    }

    /// floor(self / 2^k).
    /// The number of trailing zero bits (0 for zero).
    pub fn trailing_zeros(&self) -> u64 {
        let a = self.limbs.as_slice();
        let mut i = 0;
        while i < a.len() {
            if a[i] != 0 {
                return 32 * i as u64 + a[i].trailing_zeros() as u64;
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
        {
            let a = self.limbs.as_mut_slice();
            let mut i = w;
            if b == 0 {
                while i < n {
                    a[i - w] = a[i];
                    i += 1;
                }
            } else {
                while i + 1 < n {
                    a[i - w] = (a[i] >> b) | (a[i + 1] << (32 - b));
                    i += 1;
                }
                a[n - 1 - w] = a[n - 1] >> b;
            }
        }
        self.limbs.truncate(n - w);
        trim(&mut self.limbs);
    }

    pub fn shr(&self, k: u64) -> Nat {
        let w = (k / 32) as usize;
        let a = self.limbs.as_slice();
        let n = a.len();
        if w >= n {
            return Nat::zero();
        }
        let b = (k % 32) as u32;
        let mut v = Limbs::zeroed(n - w);
        {
            let out = v.as_mut_slice();
            if b == 0 {
                out.copy_from_slice(&a[w..]);
            } else {
                let mut i = w;
                while i + 1 < n {
                    out[i - w] = (a[i] >> b) | (a[i + 1] << (32 - b));
                    i += 1;
                }
                out[n - 1 - w] = a[n - 1] >> b;
            }
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
            let mut q = Limbs::zeroed(self.limbs.len());
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
        let mut unl = self.shl(s).limbs;
        unl.push(0);
        let un = unl.as_mut_slice();
        let n = dn.limbs.len();
        let m = un.len() - n - 1;
        let mut q = Limbs::zeroed(m + 1);
        let dv = dn.limbs.as_slice();
        let dtop = dv[n - 1] as u64;
        let dnext = dv[n - 2] as u64;
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
            // multiply and subtract (t below wraps: its top bit is the borrow, its low half the limb)
            let mut borrow = 0u64;
            let mut carry = 0u64;
            {
                let w = &mut un[j..j + n];
                let mut i = 0;
                while i < n {
                    let p = qhat * dv[i] as u64 + carry;
                    carry = p >> 32;
                    let t = (w[i] as u64).wrapping_sub(p & 0xffff_ffff).wrapping_sub(borrow);
                    w[i] = t as u32;
                    borrow = t >> 63;
                    i += 1;
                }
            }
            let t = un[j + n] as i64 - borrow as i64 - carry as i64;
            if t < 0 {
                un[j + n] = (t + (1i64 << 32)) as u32;
                // add back
                qhat -= 1;
                let mut c = 0u64;
                let w = &mut un[j..j + n];
                let mut i = 0;
                while i < n {
                    let s2 = w[i] as u64 + dv[i] as u64 + c;
                    w[i] = s2 as u32;
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
        unl.truncate(n);
        trim(&mut unl);
        let r = Nat { limbs: unl }.shr(s);
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
            let mut x = Nat {
                limbs: Limbs::from_slice(&limbs),
            };
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

    /// Values whose limb counts sit at the inline bound (INLINE - 1, INLINE, INLINE + 1 and
    /// neighbours), and results that cross it: every operation gives the same limbs whether its
    /// operands are held inline or on the heap, and the ring and division identities hold.
    #[test]
    fn inline_and_heap_agree_at_the_bound() {
        fn rng(s: &mut u64) -> u64 {
            *s ^= *s << 13;
            *s ^= *s >> 7;
            *s ^= *s << 17;
            *s
        }
        fn pick(s: &mut u64, n: usize) -> Vec<u32> {
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                let r = rng(s);
                v.push(if r.is_multiple_of(5) { u32::MAX } else { r as u32 });
            }
            if let Some(t) = v.last_mut()
                && *t == 0
            {
                *t = 1;
            }
            v
        }
        let mut s = 0x2545_f491_4f6c_dd1du64;
        let both = |v: &[u32]| {
            let inline = Nat {
                limbs: Limbs::from_slice(v),
            };
            let heap = Nat {
                limbs: Limbs::Heap(v.to_vec()),
            };
            (inline, heap)
        };
        let lo = INLINE - 3;
        let hi = INLINE + 3;
        for na in lo..=hi {
            for nb in 1..=hi {
                for _ in 0..20 {
                    let (av, bv) = (pick(&mut s, na), pick(&mut s, nb));
                    let ((a, ah), (b, bh)) = (both(&av), both(&bv));
                    let same = |x: Nat, y: Nat| assert_eq!(x.limbs.as_slice(), y.limbs.as_slice());
                    same(a.add(&b), ah.add(&bh));
                    same(a.mul(&b), ah.mul(&bh));
                    let k = rng(&mut s) % 100;
                    same(a.shl(k), ah.shl(k));
                    same(a.shr(k), ah.shr(k));
                    let (mut x, mut y) = (a.clone(), ah.clone());
                    x.shr_assign(k);
                    y.shr_assign(k);
                    same(x, y);
                    assert_eq!(a.cmp(&b), ah.cmp(&bh));
                    assert_eq!(a.trailing_zeros(), ah.trailing_zeros());
                    let (q, r) = a.divrem(&b);
                    let (qh, rh) = ah.divrem(&bh);
                    assert_eq!(q.limbs.as_slice(), qh.limbs.as_slice());
                    assert_eq!(r.limbs.as_slice(), rh.limbs.as_slice());
                    assert_eq!(q.mul(&b).add(&r), a);
                    assert_eq!(r.cmp(&b), Ordering::Less);
                    let (big, small) = if a.cmp(&b) == Ordering::Less {
                        (&b, &a)
                    } else {
                        (&a, &b)
                    };
                    assert_eq!(big.sub(small).add(small), *big);
                    let t = a.isqrt();
                    assert_eq!(t.limbs.as_slice(), ah.isqrt().limbs.as_slice());
                    assert!(t.mul(&t).cmp(&a) != Ordering::Greater);
                    let t1 = t.add(&Nat::from_u64(1));
                    assert_eq!(t1.mul(&t1).cmp(&a), Ordering::Greater);
                }
            }
        }
    }
}
