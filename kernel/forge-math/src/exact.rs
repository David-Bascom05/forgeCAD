//! Exact dyadic arithmetic: every f64 is `m · 2^e` with integer `m`, and sums and products of
//! such numbers are again such numbers, so the signs the decide-once boolean's ledger needs (a
//! plane at the meet of three others, a point against a polygon's edge) are computed with no
//! rounding at all and no precondition on the inputs' range: [`Dy`] is a big integer times a power
//! of two. Slow next to the floating filters of `predicates`; callers filter first.

use core::cmp::Ordering;

/// Rationals, polynomials and real algebraic numbers over [`Big`] (the curved decide-once side).
pub mod alg;

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
    fn top64(&self) -> (u64, i32) {
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
                let dropped =
                    mag[..words].iter().any(|&w| w != 0) || (bits > 0 && acc & ((1u128 << bits) - 1) != 0);
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
