//! Exact dyadic arithmetic: every f64 is `m · 2^e` with integer `m`, and sums and products of
//! such numbers are again such numbers, so the signs the decide-once boolean's ledger needs (a
//! plane at the meet of three others, a point against a polygon's edge) are computed with no
//! rounding at all and no precondition on the inputs' range: [`Dy`] is a big integer times a power
//! of two. Slow next to the floating filters of `predicates`; callers filter first.

use core::cmp::Ordering;

/// Rationals, polynomials and real algebraic numbers over [`Big`] (the curved decide-once side).
pub mod alg;

/// A signed big integer, one representation per value (so equality is structural):
/// - `S`: inline while it fits under 2¹²⁶ in magnitude (the common case: the kernel's numbers are
///   short dyadics);
/// - `M`: its magnitude in up to [`INL`] base 2⁶⁴ limbs held inline, least significant first,
///   unused limbs zero (the products the ledger's signs take: a plane at a meet of three is ~320
///   bits);
/// - `L`: past that, the limbs on the heap, with no trailing zero limb.
///
/// No arithmetic below `L`'s range allocates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Big {
    S(i128),
    M { neg: bool, n: u8, w: [u64; INL] },
    L { neg: bool, mag: Vec<u64> },
}

impl Default for Big {
    fn default() -> Big {
        Big::S(0)
    }
}

/// Limbs held inline (512 bits).
pub const INL: usize = 8;

/// Inline values stay below this magnitude, so a sum of two never overflows i128.
const SMALL_BITS: u32 = 126;

fn fits(v: i128) -> bool {
    v.unsigned_abs() < (1u128 << SMALL_BITS)
}

fn trimmed(a: &[u64]) -> &[u64] {
    let mut n = a.len();
    while n > 0 && a[n - 1] == 0 {
        n -= 1;
    }
    &a[..n]
}

fn cmp_mag(a: &[u64], b: &[u64]) -> Ordering {
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

/// `out = a + b`; `out` is zeroed and at least `max(len) + 1` long.
fn add_into(a: &[u64], b: &[u64], out: &mut [u64]) {
    let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut carry = 0u64;
    for i in 0..a.len() {
        let (s1, c1) = a[i].overflowing_add(if i < b.len() { b[i] } else { 0 });
        let (s2, c2) = s1.overflowing_add(carry);
        out[i] = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
    out[a.len()] = carry;
}

/// `out = a - b` for `|a| >= |b|`; `out` at least `a.len()` long.
fn sub_into(a: &[u64], b: &[u64], out: &mut [u64]) {
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let (d1, b1) = a[i].overflowing_sub(if i < b.len() { b[i] } else { 0 });
        let (d2, b2) = d1.overflowing_sub(borrow);
        out[i] = d2;
        borrow = (b1 as u64) + (b2 as u64);
    }
}

/// `out = a · b`; `out` is zeroed and at least `a.len() + b.len()` long.
fn mul_into(a: &[u64], b: &[u64], out: &mut [u64]) {
    for i in 0..a.len() {
        let mut carry = 0u128;
        let x = a[i] as u128;
        for j in 0..b.len() {
            let t = out[i + j] as u128 + x * b[j] as u128 + carry;
            out[i + j] = t as u64;
            carry = t >> 64;
        }
        out[i + b.len()] = carry as u64;
    }
}

/// `out = a · 2^bits`; `out` is zeroed and at least `a.len() + bits / 64 + 1` long.
fn shl_into(a: &[u64], bits: u32, out: &mut [u64]) {
    let (words, bits) = ((bits / 64) as usize, bits % 64);
    if bits == 0 {
        out[words..words + a.len()].copy_from_slice(a);
    } else {
        let mut carry = 0u64;
        for i in 0..a.len() {
            out[words + i] = (a[i] << bits) | carry;
            carry = a[i] >> (64 - bits);
        }
        out[words + a.len()] = carry;
    }
}

/// Run `f` on a zeroed scratch of `len` limbs: on the stack when short.
fn scratch<R>(len: usize, f: impl FnOnce(&mut [u64]) -> R) -> R {
    const STACK: usize = 2 * INL + 2;
    if len <= STACK {
        let mut b = [0u64; STACK];
        f(&mut b[..len])
    } else {
        let mut v = vec![0u64; len];
        f(&mut v)
    }
}

impl Big {
    pub fn zero() -> Big {
        Big::S(0)
    }

    pub fn from_i64(x: i64) -> Big {
        Big::S(x as i128)
    }

    /// The value from sign and limbs, in its one representation.
    fn from_mag(neg: bool, m: &[u64]) -> Big {
        let m = trimmed(m);
        if m.len() <= 2 {
            let v = m.first().copied().unwrap_or(0) as u128 | (m.get(1).copied().unwrap_or(0) as u128) << 64;
            if v < (1u128 << SMALL_BITS) {
                let v = v as i128;
                return Big::S(if neg { -v } else { v });
            }
        }
        if m.len() <= INL {
            let mut w = [0u64; INL];
            w[..m.len()].copy_from_slice(m);
            Big::M {
                neg,
                n: m.len() as u8,
                w,
            }
        } else {
            Big::L { neg, mag: m.to_vec() }
        }
    }

    /// Sign and limbs (trimmed), an inline value's in `tmp`.
    fn view<'a>(&'a self, tmp: &'a mut [u64; 2]) -> (bool, &'a [u64]) {
        match self {
            Big::S(v) => {
                let a = v.unsigned_abs();
                tmp[0] = a as u64;
                tmp[1] = (a >> 64) as u64;
                (*v < 0, trimmed(&tmp[..]))
            }
            Big::M { neg, n, w } => (*neg, &w[..*n as usize]),
            Big::L { neg, mag } => (*neg, &mag[..]),
        }
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Big::S(0))
    }

    /// -1, 0 or 1.
    pub fn signum(&self) -> i32 {
        match self {
            Big::S(v) => v.signum() as i32,
            Big::M { neg, .. } | Big::L { neg, .. } => {
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
            Big::M { neg, n, w } => Big::M {
                neg: !neg,
                n: *n,
                w: *w,
            },
            Big::L { neg, mag } => Big::L {
                neg: !neg,
                mag: mag.clone(),
            },
        }
    }

    pub fn abs(&self) -> Big {
        match self {
            Big::S(v) => Big::S(v.abs()),
            Big::M { n, w, .. } => Big::M {
                neg: false,
                n: *n,
                w: *w,
            },
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
        let (mut ta, mut tb) = ([0u64; 2], [0u64; 2]);
        let ((an, am), (bn, bm)) = (self.view(&mut ta), o.view(&mut tb));
        if an == bn {
            return scratch(am.len().max(bm.len()) + 1, |out| {
                add_into(am, bm, out);
                Big::from_mag(an, out)
            });
        }
        match cmp_mag(am, bm) {
            Ordering::Equal => Big::zero(),
            Ordering::Greater => scratch(am.len(), |out| {
                sub_into(am, bm, out);
                Big::from_mag(an, out)
            }),
            Ordering::Less => scratch(bm.len(), |out| {
                sub_into(bm, am, out);
                Big::from_mag(bn, out)
            }),
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
        if self.is_zero() || o.is_zero() {
            return Big::zero();
        }
        let (mut ta, mut tb) = ([0u64; 2], [0u64; 2]);
        let ((an, am), (bn, bm)) = (self.view(&mut ta), o.view(&mut tb));
        scratch(am.len() + bm.len(), |out| {
            mul_into(am, bm, out);
            Big::from_mag(an != bn, out)
        })
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
        let mut t = [0u64; 2];
        let (n, m) = self.view(&mut t);
        scratch(m.len() + (bits / 64) as usize + 1, |out| {
            shl_into(m, bits, out);
            Big::from_mag(n, out)
        })
    }

    /// The number of significant bits of the magnitude.
    pub fn bits(&self) -> u32 {
        let mut t = [0u64; 2];
        let (_, m) = self.view(&mut t);
        match m.last() {
            None => 0,
            Some(&top) => 64 * (m.len() as u32 - 1) + (64 - top.leading_zeros()),
        }
    }

    /// The top 64 bits of the magnitude and the shift they stand at: `|self| ≈ top · 2^shift`,
    /// with a sticky low bit for what was dropped (so rounding to f64 sees it).
    fn top64(&self) -> (u64, i32) {
        let b = self.bits();
        let mut t = [0u64; 2];
        let (_, m) = self.view(&mut t);
        if b <= 64 {
            return (m.first().copied().unwrap_or(0), 0);
        }
        let s = b - 64;
        let (words, bits) = ((s / 64) as usize, s % 64);
        let acc = m[words] as u128 | (m.get(words + 1).copied().unwrap_or(0) as u128) << 64;
        let mut top = (acc >> bits) as u64;
        let dropped = m[..words].iter().any(|&w| w != 0) || (bits > 0 && acc & ((1u128 << bits) - 1) != 0);
        if dropped {
            top |= 1;
        }
        (top, s as i32)
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

/// `a · 2^s` for `s < 64`, one limb longer (the top limb may be zero).
fn shl_small(a: &[u64], s: u32) -> Vec<u64> {
    let mut out = vec![0u64; a.len() + 1];
    if s == 0 {
        out[..a.len()].copy_from_slice(a);
    } else {
        let mut carry = 0u64;
        for i in 0..a.len() {
            out[i] = (a[i] << s) | carry;
            carry = a[i] >> (64 - s);
        }
        out[a.len()] = carry;
    }
    out
}

/// `(q, r)` with `a = q b + r`, `r < b`, for trimmed magnitudes with `b` nonzero: Knuth's
/// algorithm D (TAOCP 4.3.1) on base-2⁶⁴ limbs (double words in `u128`), a short division for a
/// one-limb `b`.
fn divrem_mag(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    if cmp_mag(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    if b.len() == 1 {
        let d = b[0] as u128;
        let mut q = vec![0u64; a.len()];
        let mut r = 0u128;
        let mut i = a.len();
        while i > 0 {
            i -= 1;
            let cur = (r << 64) | a[i] as u128;
            q[i] = (cur / d) as u64;
            r = cur % d;
        }
        return (q, vec![r as u64]);
    }
    let s = b[b.len() - 1].leading_zeros();
    let mut v = shl_small(b, s);
    v.pop(); // b's top limb, shifted, does not carry out
    let mut u = shl_small(a, s);
    let n = v.len();
    let m = u.len() - n;
    let mut q = vec![0u64; m];
    let base = 1u128 << 64;
    let (v1, v2) = (v[n - 1] as u128, v[n - 2] as u128);
    let mut j = m;
    while j > 0 {
        j -= 1;
        let num = ((u[j + n] as u128) << 64) | u[j + n - 1] as u128;
        let mut qhat = num / v1;
        let mut rhat = num % v1;
        while qhat >= base || qhat * v2 > ((rhat << 64) | u[j + n - 2] as u128) {
            qhat -= 1;
            rhat += v1;
            if rhat >= base {
                break;
            }
        }
        // u[j..=j+n] -= qhat · v
        let mut borrow = 0u64;
        let mut carry = 0u128;
        for i in 0..n {
            let p = qhat * v[i] as u128 + carry;
            carry = p >> 64;
            let (d1, b1) = u[i + j].overflowing_sub(p as u64);
            let (d2, b2) = d1.overflowing_sub(borrow);
            u[i + j] = d2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        let (d1, b1) = u[j + n].overflowing_sub(carry as u64);
        let (d2, b2) = d1.overflowing_sub(borrow);
        u[j + n] = d2;
        if b1 || b2 {
            // the estimate was one too big: add v back
            qhat -= 1;
            let mut c = 0u64;
            for i in 0..n {
                let (s1, c1) = u[i + j].overflowing_add(v[i]);
                let (s2, c2) = s1.overflowing_add(c);
                u[i + j] = s2;
                c = (c1 as u64) + (c2 as u64);
            }
            u[j + n] = u[j + n].wrapping_add(c);
        }
        q[j] = qhat as u64;
    }
    // the remainder: u[0..n] shifted back
    let mut r = vec![0u64; n];
    if s == 0 {
        r.copy_from_slice(&u[..n]);
    } else {
        for i in 0..n {
            let hi = if i + 1 < n { u[i + 1] << (64 - s) } else { 0 };
            r[i] = (u[i] >> s) | hi;
        }
    }
    (q, r)
}

/// The gcd of two magnitudes below 2¹²⁸ (binary).
fn gcd_u128(a: u128, b: u128) -> u128 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let shift = (a | b).trailing_zeros();
    let mut a = a >> a.trailing_zeros();
    let mut b = b;
    while b != 0 {
        b >>= b.trailing_zeros();
        if a > b {
            core::mem::swap(&mut a, &mut b);
        }
        b -= a;
    }
    a << shift
}

/// `|a|` against `|b|`: −1, 0 or 1.
fn bcmp_abs(a: &Big, b: &Big) -> i32 {
    let (ba, bb) = (a.bits(), b.bits());
    if ba != bb {
        if ba < bb { -1 } else { 1 }
    } else {
        let (mut ta, mut tb) = ([0u64; 2], [0u64; 2]);
        let (_, am) = a.view(&mut ta);
        let (_, bm) = b.view(&mut tb);
        match cmp_mag(am, bm) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

/// `⌊|x| / 2^sh⌋` truncated to 64 bits (the leading digits Lehmer's step reads).
fn top_at(x: &Big, sh: u32) -> u64 {
    let mut t = [0u64; 2];
    let (_, m) = x.view(&mut t);
    let (w, b) = ((sh / 64) as usize, sh % 64);
    let lo = m.get(w).copied().unwrap_or(0) as u128;
    let hi = m.get(w + 1).copied().unwrap_or(0) as u128;
    (((hi << 64) | lo) >> b) as u64
}

impl Big {
    /// `(q, r)` with `self = q o + r`, `q` truncated toward zero and `r` of `self`'s sign
    /// (`|r| < |o|`); `o = 0` gives `(0, self)`.
    pub fn divrem(&self, o: &Big) -> (Big, Big) {
        if o.is_zero() {
            return (Big::zero(), self.clone());
        }
        if let (Big::S(a), Big::S(b)) = (self, o) {
            return (Big::S(a / b), Big::S(a % b));
        }
        let (mut ta, mut tb) = ([0u64; 2], [0u64; 2]);
        let (an, am) = self.view(&mut ta);
        let (bn, bm) = o.view(&mut tb);
        let (q, r) = divrem_mag(am, bm);
        (Big::from_mag(an != bn, &q), Big::from_mag(an, &r))
    }

    /// The gcd of `|self|` and `|o|` (`gcd(0, 0) = 0`). Lehmer's algorithm (TAOCP 4.5.2, L) while
    /// the larger is past 2¹²⁶: the leading 64 bits of both decide a run of Euclid steps, applied
    /// at once as a unimodular 2×2 matrix (so the gcd is kept whatever the run), a full division
    /// when the run is empty; then binary on `u128`.
    pub fn gcd(&self, o: &Big) -> Big {
        let mut x = self.abs();
        let mut y = o.abs();
        if bcmp_abs(&x, &y) < 0 {
            core::mem::swap(&mut x, &mut y);
        }
        loop {
            if let (Big::S(a), Big::S(b)) = (&x, &y) {
                return Big::S(gcd_u128(a.unsigned_abs(), b.unsigned_abs()) as i128);
            }
            if y.is_zero() {
                return x;
            }
            let sh = x.bits() - 64;
            let xh = top_at(&x, sh) as i128;
            let yh = top_at(&y, sh) as i128;
            let (mut a, mut b, mut c, mut d) = (1i128, 0i128, 0i128, 1i128);
            let (mut u, mut v) = (xh, yh);
            loop {
                if v + c == 0 || v + d == 0 {
                    break;
                }
                let q = (u + a) / (v + c);
                let q2 = (u + b) / (v + d);
                if q != q2 {
                    break;
                }
                let t = a - q * c;
                a = c;
                c = t;
                let t = b - q * d;
                b = d;
                d = t;
                let t = u - q * v;
                u = v;
                v = t;
            }
            if b == 0 {
                let r = x.divrem(&y).1;
                x = y;
                y = r;
            } else {
                let nx = x.mul(&Big::S(a)).add(&y.mul(&Big::S(b))).abs();
                let ny = x.mul(&Big::S(c)).add(&y.mul(&Big::S(d))).abs();
                x = nx;
                y = ny;
                if bcmp_abs(&x, &y) < 0 {
                    core::mem::swap(&mut x, &mut y);
                }
            }
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
    // (zero is +0.0, whatever the sign of `b`: one f64 per value)
    if a.is_zero() {
        return 0.0;
    }
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

/// An exact 3-vector.
#[derive(Clone, Debug, Default)]
pub struct DyV3 {
    pub x: Dy,
    pub y: Dy,
    pub z: Dy,
}

impl DyV3 {
    pub fn new(x: Dy, y: Dy, z: Dy) -> DyV3 {
        DyV3 { x, y, z }
    }

    pub fn from_f64(x: f64, y: f64, z: f64) -> DyV3 {
        DyV3::new(Dy::from_f64(x), Dy::from_f64(y), Dy::from_f64(z))
    }

    pub fn dot(&self, o: &DyV3) -> Dy {
        self.x.mul(&o.x).add(&self.y.mul(&o.y)).add(&self.z.mul(&o.z))
    }

    pub fn cross(&self, o: &DyV3) -> DyV3 {
        DyV3::new(
            det2(&self.y, &self.z, &o.y, &o.z),
            det2(&self.z, &self.x, &o.z, &o.x),
            det2(&self.x, &self.y, &o.x, &o.y),
        )
    }

    pub fn scale(&self, k: &Dy) -> DyV3 {
        DyV3::new(self.x.mul(k), self.y.mul(k), self.z.mul(k))
    }

    pub fn add(&self, o: &DyV3) -> DyV3 {
        DyV3::new(self.x.add(&o.x), self.y.add(&o.y), self.z.add(&o.z))
    }

    pub fn sub(&self, o: &DyV3) -> DyV3 {
        DyV3::new(self.x.sub(&o.x), self.y.sub(&o.y), self.z.sub(&o.z))
    }

    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }

    pub fn get(&self, axis: usize) -> &Dy {
        match axis {
            0 => &self.x,
            1 => &self.y,
            _ => &self.z,
        }
    }
}

/// A plane `n · x = c`, exactly.
#[derive(Clone, Debug, Default)]
pub struct DyPlane {
    pub n: DyV3,
    pub c: Dy,
}

/// A point in homogeneous form `(X, Y, Z) / W`, `W ≠ 0`: the exact meet of three planes, or a
/// combination of such meets.
#[derive(Clone, Debug, Default)]
pub struct HPoint {
    pub p: DyV3,
    pub w: Dy,
}

impl DyPlane {
    /// The plane through `o` with normal `n`, from the kernel's f64 frame, exactly.
    pub fn from_frame(o: [f64; 3], n: [f64; 3]) -> DyPlane {
        let n = DyV3::from_f64(n[0], n[1], n[2]);
        let o = DyV3::from_f64(o[0], o[1], o[2]);
        let c = n.dot(&o);
        DyPlane { n, c }
    }

    pub fn flipped(&self) -> DyPlane {
        DyPlane {
            n: DyV3::new(self.n.x.neg(), self.n.y.neg(), self.n.z.neg()),
            c: self.c.neg(),
        }
    }

    /// `n · x − c` at an exact point.
    pub fn eval(&self, x: &DyV3) -> Dy {
        self.n.dot(x).sub(&self.c)
    }

    /// The sign of the plane at a homogeneous point (`n · X − c W`, times the sign of `W`).
    pub fn side(&self, h: &HPoint) -> i32 {
        self.n.dot(&h.p).sub(&self.c.mul(&h.w)).signum() * h.w.signum()
    }

    /// The same point set (normals parallel, offsets in proportion).
    pub fn same_set(&self, o: &DyPlane) -> bool {
        if !self.n.cross(&o.n).is_zero() {
            return false;
        }
        // n₁ = k n₂: then c₁ = k c₂, i.e. c₁ n₂ = c₂ n₁ componentwise
        self.n.scale(&o.c).sub(&o.n.scale(&self.c)).is_zero()
    }
}

/// The point on three planes, if they meet in one (`meet3` of the reference).
pub fn meet3(a: &DyPlane, b: &DyPlane, c: &DyPlane) -> Option<HPoint> {
    let bc = b.n.cross(&c.n);
    let d = a.n.dot(&bc);
    if d.is_zero() {
        return None;
    }
    let p = bc
        .scale(&a.c)
        .add(&c.n.cross(&a.n).scale(&b.c))
        .add(&a.n.cross(&b.n).scale(&c.c));
    Some(HPoint { p, w: d })
}

impl HPoint {
    /// The point exactly (`W = 1`).
    pub fn from_dy(p: DyV3) -> HPoint {
        HPoint {
            p,
            w: Dy::from_i64(1),
        }
    }

    /// The nearest f64 point, or next to it (output only).
    pub fn to_f64(&self) -> [f64; 3] {
        [
            ratio_f64(&self.p.x, &self.w),
            ratio_f64(&self.p.y, &self.w),
            ratio_f64(&self.p.z, &self.w),
        ]
    }

    /// The sign of `self.axis − o.axis`.
    pub fn cmp_axis(&self, o: &HPoint, axis: usize) -> i32 {
        let a = self.p.get(axis).mul(&o.w);
        let b = o.p.get(axis).mul(&self.w);
        a.sub(&b).signum() * self.w.signum() * o.w.signum()
    }

    /// The same point.
    pub fn same(&self, o: &HPoint) -> bool {
        (0..3).all(|a| self.cmp_axis(o, a) == 0)
    }

    /// The mean of points.
    pub fn mean(ps: &[&HPoint]) -> HPoint {
        let mut w = Dy::from_i64(ps.len() as i64);
        for q in ps {
            w = w.mul(&q.w);
        }
        let mut p = DyV3::default();
        for (i, q) in ps.iter().enumerate() {
            let mut k = Dy::from_i64(1);
            for (j, r) in ps.iter().enumerate() {
                if i != j {
                    k = k.mul(&r.w);
                }
            }
            p = p.add(&q.p.scale(&k));
        }
        HPoint { p, w }
    }

    /// `self + t · d` (d a direction, t a dyadic).
    pub fn offset(&self, d: &DyV3, t: &Dy) -> HPoint {
        HPoint {
            p: self.p.add(&d.scale(&t.mul(&self.w))),
            w: self.w.clone(),
        }
    }
}

/// The sign of the 2D orientation of three homogeneous points projected to axes `(u, v)`:
/// positive counter-clockwise.
pub fn orient2(a: &HPoint, b: &HPoint, c: &HPoint, u: usize, v: usize) -> i32 {
    // det [[Au Av Aw] [Bu Bv Bw] [Cu Cv Cw]] · sign(Aw Bw Cw)
    let (au, av, aw) = (a.p.get(u), a.p.get(v), &a.w);
    let (bu, bv, bw) = (b.p.get(u), b.p.get(v), &b.w);
    let (cu, cv, cw) = (c.p.get(u), c.p.get(v), &c.w);
    let d = au
        .mul(&det2(bv, bw, cv, cw))
        .sub(&av.mul(&det2(bu, bw, cu, cw)))
        .add(&aw.mul(&det2(bu, bv, cu, cv)));
    d.signum() * aw.signum() * bw.signum() * cw.signum()
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
            assert_eq!(Dy::from_f64(x).to_f64(), x);
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
    }

    /// Ring laws on numbers whose exponents span hundreds of bits, so sums and products cross
    /// between the inline and the limb forms both ways.
    #[test]
    fn inline_and_limbs_agree() {
        let mut r = crate::rng::Rng::new(7);
        let mut pick = || {
            let m = r.range(-1.0, 1.0);
            let e = (r.range(-200.0, 200.0)) as i32;
            Dy::from_f64(m).scale2(e)
        };
        for _ in 0..3000 {
            let (x, y, z) = (pick(), pick(), pick());
            assert_eq!(x.add(&y).sub(&y).cmp_dy(&x), Ordering::Equal);
            assert_eq!(x.mul(&y).mul(&z).cmp_dy(&x.mul(&y.mul(&z))), Ordering::Equal);
            assert_eq!(
                x.mul(&y.add(&z)).cmp_dy(&x.mul(&y).add(&x.mul(&z))),
                Ordering::Equal
            );
            assert_eq!(x.sub(&x).signum(), 0);
            let (fx, fy) = (x.to_f64(), y.to_f64());
            if fx.is_finite() && fy.is_finite() && (fx - fy).abs() > 1e-9 * (fx.abs() + fy.abs()) {
                assert_eq!(x.cmp_dy(&y), fx.partial_cmp(&fy).unwrap());
            }
            let big = x.mul(&x).mul(&x).mul(&x);
            assert_eq!(big.sub(&big).signum(), 0);
            assert_eq!(big.signum(), if x.is_zero() { 0 } else { 1 });
        }
    }

    /// The same laws with exponents a thousand bits apart (heap limbs both ways), and small
    /// integers against i128.
    #[test]
    fn wide_and_small_agree() {
        let mut r = crate::rng::Rng::new(11);
        for _ in 0..2000 {
            let mut pick = || {
                let m = r.range(-1.0, 1.0);
                let e = (r.range(-1000.0, 1000.0)) as i32;
                Dy::from_f64(m).scale2(e)
            };
            let (x, y, z) = (pick(), pick(), pick());
            assert_eq!(x.add(&y).sub(&y).cmp_dy(&x), Ordering::Equal);
            assert_eq!(x.mul(&y).mul(&z).cmp_dy(&x.mul(&y.mul(&z))), Ordering::Equal);
            assert_eq!(
                x.mul(&y.add(&z)).cmp_dy(&x.mul(&y).add(&x.mul(&z))),
                Ordering::Equal
            );
            let big = x.mul(&x).mul(&x).mul(&x).mul(&y).mul(&y);
            assert_eq!(big.sub(&big).signum(), 0);
            assert_eq!(big.neg().add(&big).signum(), 0);
        }
        for _ in 0..2000 {
            let a = (r.range(-1.0, 1.0) * 9e18) as i64;
            let b = (r.range(-1.0, 1.0) * 9e18) as i64;
            let (x, y) = (Big::from_i64(a), Big::from_i64(b));
            let p = x.mul(&y).mul(&x);
            let q = p.add(&p).add(&p).sub(&p.shl(1));
            assert_eq!(q.cmp_big(&p), Ordering::Equal);
            assert_eq!(x.mul(&y), Big::S(a as i128 * b as i128));
            assert_eq!(p.signum(), (a.signum() * a.signum() * b.signum()) as i32);
        }
    }

    #[test]
    fn zero_is_positive_zero() {
        let z = ratio_f64(&Dy::zero(), &Dy::from_i64(-3));
        assert!(z == 0.0 && z.is_sign_positive());
        assert!(Dy::zero().to_f64().is_sign_positive());
    }

    #[test]
    fn three_planes_meet() {
        let x = DyPlane::from_frame([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let y = DyPlane::from_frame([0.0, 2.0, 0.0], [0.0, 1.0, 0.0]);
        let z = DyPlane::from_frame([0.0, 0.0, 3.0], [0.0, 0.0, 1.0]);
        let p = meet3(&x, &y, &z).unwrap();
        assert_eq!(p.to_f64(), [1.0, 2.0, 3.0]);
        let tilt = DyPlane::from_frame([1.0, 2.0, 3.0], [0.6, 0.8, 0.0]);
        assert_eq!(tilt.side(&p), 0);
        let off = DyPlane::from_frame([1.0, 2.0, 3.0 + 1e-15], [0.0, 0.0, 1.0]);
        assert_eq!(off.side(&p), -1);
        assert!(x.same_set(&x.flipped()));
        assert!(!x.same_set(&y));
    }
}
