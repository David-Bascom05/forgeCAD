//! forge-num's `Big` and `Dy` against the implementation they replaced (forge-math's exact.rs as
//! of d20dddfa, kept verbatim below as the reference): the same signs, exponents, top bits,
//! conversions to f64 (bit for bit) and inline-or-limb representation, on random dyadics in both
//! the inline and the limb regime. Guards the bit identity of decide-once's exact geometry.

#[allow(dead_code, clippy::all)]
mod reference {
    use core::cmp::Ordering;

    /// A signed big integer: inline while it fits under 2¹²⁶ in magnitude (the common case: the
    /// kernel's numbers are short dyadics), else its magnitude in base 2³² limbs, least significant
    /// first, with no trailing zero limb. One representation per value, so equality is structural.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum Big {
        S(i128),
        L { neg: bool, mag: Vec<u32> },
    }

    impl Default for Big {
        fn default() -> Big {
            Big::S(0)
        }
    }

    /// Inline values stay below this magnitude, so a sum of two never overflows i128.
    const SMALL_BITS: u32 = 126;

    fn fits(v: i128) -> bool {
        v.unsigned_abs() < (1u128 << SMALL_BITS)
    }

    fn trim(v: &mut Vec<u32>) {
        while v.last() == Some(&0) {
            v.pop();
        }
    }

    fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
        if a.len() != b.len() {
            return a.len().cmp(&b.len());
        }
        for i in (0..a.len()).rev() {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }
        Ordering::Equal
    }

    fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
        let mut out = Vec::with_capacity(a.len() + 1);
        let mut carry = 0u64;
        for (i, &x) in a.iter().enumerate() {
            let s = x as u64 + *b.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry != 0 {
            out.push(carry as u32);
        }
        out
    }

    /// `a - b` for `|a| >= |b|`.
    fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow = 0i64;
        for (i, &x) in a.iter().enumerate() {
            let mut d = x as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(d as u32);
        }
        trim(&mut out);
        out
    }

    fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0u32; a.len() + b.len()];
        for (i, &x) in a.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &y) in b.iter().enumerate() {
                let t = out[i + j] as u64 + x as u64 * y as u64 + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            let mut k = i + b.len();
            while carry != 0 {
                let t = out[k] as u64 + carry;
                out[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
        }
        trim(&mut out);
        out
    }

    fn shl_mag(a: &[u32], bits: u32) -> Vec<u32> {
        if a.is_empty() {
            return Vec::new();
        }
        let (words, bits) = ((bits / 32) as usize, bits % 32);
        let mut out = vec![0u32; words];
        if bits == 0 {
            out.extend_from_slice(a);
        } else {
            let mut carry = 0u32;
            for &x in a {
                out.push((x << bits) | carry);
                carry = x >> (32 - bits);
            }
            if carry != 0 {
                out.push(carry);
            }
        }
        out
    }

    fn mag_of(v: u128) -> Vec<u32> {
        let mut m = Vec::new();
        let mut v = v;
        while v != 0 {
            m.push(v as u32);
            v >>= 32;
        }
        m
    }

    impl Big {
        pub fn zero() -> Big {
            Big::S(0)
        }

        pub fn from_i64(x: i64) -> Big {
            Big::S(x as i128)
        }

        /// The value from sign and limbs, inline if it fits.
        fn from_l(neg: bool, mut mag: Vec<u32>) -> Big {
            trim(&mut mag);
            if mag.len() <= 4 {
                let mut v = 0u128;
                for (i, &w) in mag.iter().enumerate() {
                    v |= (w as u128) << (32 * i);
                }
                if v < (1u128 << SMALL_BITS) {
                    let v = v as i128;
                    return Big::S(if neg { -v } else { v });
                }
            }
            Big::L {
                neg: neg && !mag.is_empty(),
                mag,
            }
        }

        /// Sign and limbs.
        fn to_l(&self) -> (bool, Vec<u32>) {
            match self {
                Big::S(v) => (*v < 0, mag_of(v.unsigned_abs())),
                Big::L { neg, mag } => (*neg, mag.clone()),
            }
        }

        pub fn is_zero(&self) -> bool {
            matches!(self, Big::S(0))
        }

        /// -1, 0 or 1.
        pub fn signum(&self) -> i32 {
            match self {
                Big::S(v) => v.signum() as i32,
                Big::L { neg, .. } => {
                    if *neg {
                        -1
                    } else {
                        1
                    }
                }
            }
        }

        pub fn neg(&self) -> Big {
            match self {
                Big::S(v) => Big::S(-v),
                Big::L { neg, mag } => Big::L {
                    neg: !neg,
                    mag: mag.clone(),
                },
            }
        }

        pub fn abs(&self) -> Big {
            match self {
                Big::S(v) => Big::S(v.abs()),
                Big::L { mag, .. } => Big::L {
                    neg: false,
                    mag: mag.clone(),
                },
            }
        }

        pub fn add(&self, o: &Big) -> Big {
            if let (Big::S(a), Big::S(b)) = (self, o) {
                let s = a + b; // both under 2^126: no overflow
                if fits(s) {
                    return Big::S(s);
                }
            }
            let ((an, am), (bn, bm)) = (self.to_l(), o.to_l());
            if an == bn {
                return Big::from_l(an, add_mag(&am, &bm));
            }
            match cmp_mag(&am, &bm) {
                Ordering::Equal => Big::zero(),
                Ordering::Greater => Big::from_l(an, sub_mag(&am, &bm)),
                Ordering::Less => Big::from_l(bn, sub_mag(&bm, &am)),
            }
        }

        pub fn sub(&self, o: &Big) -> Big {
            self.add(&o.neg())
        }

        pub fn mul(&self, o: &Big) -> Big {
            if let (Big::S(a), Big::S(b)) = (self, o)
                && let Some(p) = a.checked_mul(*b)
                && fits(p)
            {
                return Big::S(p);
            }
            let ((an, am), (bn, bm)) = (self.to_l(), o.to_l());
            Big::from_l(an != bn, mul_mag(&am, &bm))
        }

        pub fn shl(&self, bits: u32) -> Big {
            if let Big::S(v) = self {
                if *v == 0 {
                    return Big::S(0);
                }
                let used = 128 - v.unsigned_abs().leading_zeros();
                if used + bits < SMALL_BITS {
                    return Big::S(v << bits);
                }
            }
            let (n, m) = self.to_l();
            Big::from_l(n, shl_mag(&m, bits))
        }

        /// The number of significant bits of the magnitude.
        pub fn bits(&self) -> u32 {
            match self {
                Big::S(v) => 128 - v.unsigned_abs().leading_zeros(),
                Big::L { mag, .. } => match mag.last() {
                    None => 0,
                    Some(&t) => 32 * (mag.len() as u32 - 1) + (32 - t.leading_zeros()),
                },
            }
        }

        /// The top 64 bits of the magnitude and the shift they stand at: `|self| ≈ top · 2^shift`,
        /// with a sticky low bit for what was dropped (so rounding to f64 sees it).
        pub fn top64(&self) -> (u64, i32) {
            match self {
                Big::S(v) => {
                    let a = v.unsigned_abs();
                    let b = 128 - a.leading_zeros();
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
                    let (words, bits) = ((s / 32) as usize, s % 32);
                    let mut acc = 0u128;
                    for i in 0..3 {
                        if let Some(&w) = mag.get(words + i) {
                            acc |= (w as u128) << (32 * i);
                        }
                    }
                    let mut t = (acc >> bits) as u64;
                    let dropped = mag[..words].iter().any(|&w| w != 0)
                        || (bits > 0 && acc & ((1u128 << bits) - 1) != 0);
                    if dropped {
                        t |= 1;
                    }
                    (t, s as i32)
                }
            }
        }

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

    /// An exact dyadic rational `m · 2^e`.
    #[derive(Clone, Debug, Default)]
    pub struct Dy {
        pub m: Big,
        pub e: i32,
    }

    impl Dy {
        pub fn zero() -> Dy {
            Dy::default()
        }

        /// The f64 exactly (finite inputs only).
        pub fn from_f64(x: f64) -> Dy {
            assert!(x.is_finite(), "an exact number from a non-finite f64");
            if x == 0.0 {
                return Dy::zero();
            }
            let b = x.to_bits();
            let neg = b >> 63 == 1;
            let ex = ((b >> 52) & 0x7ff) as i32;
            let fr = (b & ((1u64 << 52) - 1)) as i64;
            let (mut m, mut e) = if ex == 0 {
                (fr, -1074)
            } else {
                (fr | (1i64 << 52), ex - 1075)
            };
            let tz = m.trailing_zeros();
            m >>= tz;
            e += tz as i32;
            Dy {
                m: Big::from_i64(if neg { -m } else { m }),
                e,
            }
        }

        pub fn from_i64(x: i64) -> Dy {
            Dy {
                m: Big::from_i64(x),
                e: 0,
            }
        }

        pub fn signum(&self) -> i32 {
            self.m.signum()
        }

        pub fn is_zero(&self) -> bool {
            self.m.is_zero()
        }

        pub fn neg(&self) -> Dy {
            Dy {
                m: self.m.neg(),
                e: self.e,
            }
        }

        pub fn abs(&self) -> Dy {
            Dy {
                m: self.m.abs(),
                e: self.e,
            }
        }

        pub fn add(&self, o: &Dy) -> Dy {
            if self.is_zero() {
                return o.clone();
            }
            if o.is_zero() {
                return self.clone();
            }
            let e = self.e.min(o.e);
            let a = self.m.shl((self.e - e) as u32);
            let b = o.m.shl((o.e - e) as u32);
            Dy { m: a.add(&b), e }
        }

        pub fn sub(&self, o: &Dy) -> Dy {
            self.add(&o.neg())
        }

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
        pub fn scale2(&self, k: i32) -> Dy {
            Dy {
                m: self.m.clone(),
                e: self.e + k,
            }
        }

        pub fn cmp_dy(&self, o: &Dy) -> Ordering {
            match self.sub(o).signum() {
                1 => Ordering::Greater,
                -1 => Ordering::Less,
                _ => Ordering::Equal,
            }
        }

        /// The nearest f64 or one next to it (for output only, never for a decision).
        pub fn to_f64(&self) -> f64 {
            let (t, s) = self.m.top64();
            let v = t as f64 * pow2(s + self.e);
            if self.m.signum() < 0 { -v } else { v }
        }
    }

    /// `2^k` as an f64, through the subnormal and overflow ranges.
    fn pow2(k: i32) -> f64 {
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
    pub fn ratio_f64(a: &Dy, b: &Dy) -> f64 {
        let (ta, sa) = a.m.top64();
        let (tb, sb) = b.m.top64();
        let q = ta as f64 / tb as f64;
        let v = q * pow2(sa + a.e - sb - b.e);
        if (a.signum() < 0) != (b.signum() < 0) {
            -v
        } else {
            v
        }
    }

    /// The 2×2 determinant `a d − b c`.
    pub fn det2(a: &Dy, b: &Dy, c: &Dy, d: &Dy) -> Dy {
        a.mul(d).sub(&b.mul(c))
    }
}

use forge_math::exact::{Big, Dy, ratio_f64};

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// A dyadic over a wide range of exponents, now and then a subnormal.
fn pick(s: &mut u64) -> f64 {
    if rng(s).is_multiple_of(7) {
        return f64::from_bits(rng(s) % (1u64 << 52));
    }
    let m = (rng(s) >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
    let k = (rng(s) % 400) as i32 - 200;
    m * 2f64.powi(k)
}

#[test]
fn same_as_the_reference() {
    let mut s = 0x1234_5678_9abc_def1u64;
    for it in 0..10_000 {
        let (a, b, c) = (pick(&mut s), pick(&mut s), pick(&mut s));
        let (na, nb, nc) = (Dy::from_f64(a), Dy::from_f64(b), Dy::from_f64(c));
        let (oa, ob, oc) = (
            reference::Dy::from_f64(a),
            reference::Dy::from_f64(b),
            reference::Dy::from_f64(c),
        );
        let mut n = na.mul(&nb).add(&nc).sub(&na.mul(&nc));
        let mut o = oa.mul(&ob).add(&oc).sub(&oa.mul(&oc));
        // squarings carry the magnitude through hundreds of bits into the limb form
        for _ in 0..(it % 4) {
            n = n.mul(&n).sub(&na);
            o = o.mul(&o).sub(&oa);
        }
        assert_eq!(n.signum(), o.signum());
        assert_eq!(n.e, o.e as i64);
        assert_eq!(n.m.bits(), o.m.bits());
        assert_eq!(n.m.top64(), o.m.top64());
        assert_eq!(n.to_f64_near().to_bits(), o.to_f64().to_bits(), "{a} {b} {c}");
        assert_eq!(
            ratio_f64(&n, &na).to_bits(),
            reference::ratio_f64(&o, &oa).to_bits()
        );
        assert_eq!(n.cmp_dy(&nb), o.cmp_dy(&ob));
        let k = (rng(&mut s) % 300) as u32;
        assert_eq!(n.m.shl(k).top64(), o.m.shl(k).top64());
        assert_eq!(n.m.cmp_big(&nc.m), o.m.cmp_big(&oc.m));
        assert_eq!(matches!(n.m, Big::S(_)), matches!(o.m, reference::Big::S(_)));
    }
}
