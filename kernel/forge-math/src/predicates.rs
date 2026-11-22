//! Sign-exact geometric predicates. A plain f64 evaluation is used when its
//! error bound proves the sign; otherwise the determinant is recomputed exactly
//! with floating-point expansions (Shewchuk, "Adaptive Precision Floating-Point
//! Arithmetic and Fast Robust Geometric Predicates", 1997). The returned value
//! has the correct sign; its magnitude is an approximation.
//!
//! Shewchuk's bounds and expansions assume no underflow and no overflow. Here:
//! - `orient2d`'s and `dot3`'s filters carry an absolute slack (`SLACK`), which covers the
//!   absolute error of a subnormal product, and `cross_sign`'s is sound by monotonicity: a fast
//!   decision of these three is right for every finite input (proven for `orient2d` and
//!   `cross_sign`, research/kernel-audit/float-verification.md). The slack changes no decision
//!   unless the bound is below about 2^-1017. `triple` and the in-circle tests multiply rounded
//!   products, so an absolute error is amplified: for them the window below is the whole
//!   predicate's precondition, and inside it no product's rounding is absolute (every product
//!   is a multiple of 2^-1074), so their filters need no slack;
//! - the slow paths' expansions are exact when every nonzero input lies in the predicate's
//!   window (`DEG2_LO`..): that is each predicate's precondition for an exact answer, checked by
//!   `debug_assert!`. A slow-path result that comes out NaN (a product overflowed) is recomputed
//!   exactly with fixed-width integers (`Big`).

use crate::{Point2, Vec2, Vec3};
use std::cmp::Ordering;

const EPS: f64 = f64::EPSILON * 0.5;
const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1
const CCW_BOUND: f64 = (3.0 + 16.0 * EPS) * EPS;
const ICC_BOUND: f64 = (10.0 + 96.0 * EPS) * EPS;
// Shewchuk's stage-B bounds: the determinant of the rounded differences,
// computed exactly, has the right sign when it is at least this times the permanent.
const CCW_BOUND_B: f64 = (2.0 + 12.0 * EPS) * EPS;
const ICC_BOUND_B: f64 = (4.0 + 48.0 * EPS) * EPS;
// derived and tested in research/predicate-bounds/
/// Absolute slack on the fast filters' bounds: `16 · 2^-1074`, well above the absolute rounding
/// error (at most `2^-1075` each) of the few subnormal products a filter can meet.
const SLACK: f64 = f64::from_bits(16);
// The expansion paths are exact when every nonzero input lies in the predicate's window: then all
// inputs are multiples of `2^(lo - 52)`, every product of `deg` of them is a multiple of
// `2^-1074` (so its rounding error is representable even when it is subnormal), and nothing
// reaches `2^996` (Veltkamp's split) or overflows. deg 2: 2·(480+52) ≤ 1074; deg 3: 3·(300+52)
// ≤ 1074; deg 4: 4·(210+52) ≤ 1074.
const DEG2_LO: f64 = f64::from_bits((1023 - 480) << 52);
const DEG2_HI: f64 = f64::from_bits((1023 + 500) << 52);
const DEG3_LO: f64 = f64::from_bits((1023 - 300) << 52);
const DEG3_HI: f64 = f64::from_bits((1023 + 330) << 52);
const DEG4_LO: f64 = f64::from_bits((1023 - 210) << 52);
const DEG4_HI: f64 = f64::from_bits((1023 + 250) << 52);

/// Whether every value is zero or within `[lo, hi]` in magnitude (false for NaN and infinities):
/// the slow paths' precondition, checked in debug builds (`debug_assert!` compiles it away in
/// release).
fn within(v: &[f64], lo: f64, hi: f64) -> bool {
    v.iter().all(|&x| x == 0.0 || (x.abs() >= lo && x.abs() <= hi))
}

/// Exact signed integers of `N` 64-bit limbs (two's complement, little-endian): the last resort
/// for inputs outside the expansion windows. `of(x)` is `x · 2^1074`, an integer for every finite
/// double (below 2^2099), so each determinant scaled by a power of 2^1074 is a polynomial in
/// these. `N` is chosen per predicate so the polynomial cannot overflow.
#[derive(Clone, Copy)]
struct Big<const N: usize>([u64; N]);

impl<const N: usize> Big<N> {
    fn of(x: f64) -> Self {
        let bits = x.to_bits();
        let e = ((bits >> 52) & 0x7ff) as usize;
        let f = bits & ((1 << 52) - 1);
        // x = m · 2^(sh - 1074)
        let (m, sh) = if e == 0 { (f, 0) } else { (f | (1 << 52), e - 1) };
        let mut r = [0u64; N];
        let (limb, bit) = (sh / 64, sh % 64);
        r[limb] = m << bit;
        if bit > 0 {
            r[limb + 1] = m >> (64 - bit);
        }
        let v = Big(r);
        if bits >> 63 == 1 { v.neg() } else { v }
    }

    fn neg(self) -> Self {
        let mut r = [0u64; N];
        let mut carry = 1u64;
        for (o, &x) in r.iter_mut().zip(&self.0) {
            let (s, c) = (!x).overflowing_add(carry);
            *o = s;
            carry = c as u64;
        }
        Big(r)
    }

    fn add(self, o: Self) -> Self {
        let mut r = [0u64; N];
        let mut carry = false;
        for ((d, &x), &y) in r.iter_mut().zip(&self.0).zip(&o.0) {
            let (s1, c1) = x.overflowing_add(y);
            let (s2, c2) = s1.overflowing_add(carry as u64);
            *d = s2;
            carry = c1 || c2;
        }
        Big(r)
    }

    fn sub(self, o: Self) -> Self {
        self.add(o.neg())
    }

    /// The product modulo 2^(64N): the signed product, when it fits.
    fn mul(self, o: Self) -> Self {
        let mut r = [0u64; N];
        for i in 0..N {
            if self.0[i] == 0 {
                continue;
            }
            let mut carry = 0u128;
            for j in 0..N - i {
                let t = r[i + j] as u128 + self.0[i] as u128 * o.0[j] as u128 + carry;
                r[i + j] = t as u64;
                carry = t >> 64;
            }
        }
        Big(r)
    }

    fn negative(&self) -> bool {
        self.0[N - 1] >> 63 == 1
    }

    /// About `self · 2^-scale`, with the exact sign: never zero unless `self` is, never
    /// infinite (clamped to the least subnormal and to `f64::MAX`).
    fn to_f64(self, scale: i32) -> f64 {
        let neg = self.negative();
        let m = if neg { self.neg() } else { self };
        let Some(k) = m.0.iter().rposition(|&l| l != 0) else {
            return 0.0;
        };
        let lo = if k > 0 { m.0[k - 1] } else { 0 };
        let mut v = m.0[k] as f64 * f64::from_bits((1023 + 64) << 52) + lo as f64;
        let mut e = 64 * (k as i32 - 1) - scale;
        while e > 0 && v.is_finite() {
            let s = e.min(1000);
            v *= f64::from_bits(((1023 + s) as u64) << 52);
            e -= s;
        }
        while e < 0 && v != 0.0 {
            let s = (-e).min(1000);
            v *= f64::from_bits(((1023 - s) as u64) << 52);
            e += s;
        }
        let v = v.clamp(f64::from_bits(1), f64::MAX);
        if neg { -v } else { v }
    }
}

/// The exact determinant `a*d - b*c`'s sign, for any finite inputs.
#[cold]
fn cross_big(a: f64, d: f64, b: f64, c: f64) -> f64 {
    if ![a, b, c, d].iter().all(|x| x.is_finite()) {
        return f64::NAN;
    }
    type B = Big<68>;
    B::of(a).mul(B::of(d)).sub(B::of(b).mul(B::of(c))).to_f64(2148)
}
/// a*b - c*d evaluated in f64 is within this times |a*b| + |c*d| (as evaluated) of the truth.
const CROSS_BOUND: f64 = (2.0 + 8.0 * EPS) * EPS;
/// The same for a 3-term dot product, relative to the sum of the products' magnitudes.
const DOT3_BOUND: f64 = (3.0 + 32.0 * EPS) * EPS;
/// The same for a triple product a . (b x c), relative to its permanent.
const TRIPLE_BOUND: f64 = (5.0 + 64.0 * EPS) * EPS;

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let x = a - b;
    let bv = a - x;
    let av = x + bv;
    (x, (a - av) + (bv - b))
}

fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ahi, alo) = split(a);
    let (bhi, blo) = split(b);
    let err = ((x - ahi * bhi) - alo * bhi) - ahi * blo;
    (x, alo * blo - err)
}

/// The sign of an expansion is the sign of its largest component.
fn estimate(e: &[f64]) -> f64 {
    let top = e.last().copied().unwrap_or(0.0);
    let sum: f64 = e.iter().sum();
    if (sum > 0.0) == (top > 0.0) && (sum < 0.0) == (top < 0.0) {
        sum
    } else {
        top
    }
}

/// `e + f` into `h`, both inputs nonoverlapping and in increasing magnitude;
/// zero components dropped. Returns the length used. No allocation.
fn sum_into(e: &[f64], f: &[f64], h: &mut [f64]) -> usize {
    let (mut ei, mut fi, mut n) = (0, 0, 0);
    let mut next = || {
        if fi >= f.len() || (ei < e.len() && e[ei].abs() < f[fi].abs()) {
            ei += 1;
            e[ei - 1]
        } else {
            fi += 1;
            f[fi - 1]
        }
    };
    let total = e.len() + f.len();
    if total == 0 {
        return 0;
    }
    let mut q = next();
    for _ in 1..total {
        let (s, err) = two_sum(q, next());
        if err != 0.0 {
            h[n] = err;
            n += 1;
        }
        q = s;
    }
    if q != 0.0 || n == 0 {
        h[n] = q;
        n += 1;
    }
    n
}

/// `e * b` into `h` (needs 2 * e.len() slots). No allocation.
fn scale_into(e: &[f64], b: f64, h: &mut [f64]) -> usize {
    let Some((&first, rest)) = e.split_first() else {
        return 0;
    };
    let mut n = 0;
    let (mut q, lo) = two_product(first, b);
    if lo != 0.0 {
        h[n] = lo;
        n += 1;
    }
    for &ei in rest {
        let (p1, p0) = two_product(ei, b);
        let (s, err) = two_sum(q, p0);
        if err != 0.0 {
            h[n] = err;
            n += 1;
        }
        let (s2, err2) = two_sum(p1, s);
        if err2 != 0.0 {
            h[n] = err2;
            n += 1;
        }
        q = s2;
    }
    if q != 0.0 || n == 0 {
        h[n] = q;
        n += 1;
    }
    n
}

/// a*d - b*c exactly, as up to four components.
fn cross_exact(a: f64, d: f64, b: f64, c: f64, h: &mut [f64; 4]) -> usize {
    let (x1, y1) = two_product(a, d);
    let (x2, y2) = two_product(b, c);
    sum_into(&[y1, x1], &[-y2, -x2], h)
}

fn exact_diffs<const N: usize>(pairs: [(f64, f64); N]) -> Option<[f64; N]> {
    let mut out = [0.0; N];
    for (o, (a, b)) in out.iter_mut().zip(pairs) {
        let (x, tail) = two_diff(a, b);
        if tail != 0.0 {
            return None;
        }
        *o = x;
    }
    Some(out)
}

/// Positive if a, b, c run counter-clockwise, negative if clockwise, zero if collinear.
#[inline]
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> f64 {
    let left = (a.x - c.x) * (b.y - c.y);
    let right = (a.y - c.y) * (b.x - c.x);
    let det = left - right;
    let detsum = left.abs() + right.abs();
    let bound = CCW_BOUND * detsum + SLACK;
    if det > bound || -det > bound {
        return det;
    }
    orient2d_adapt(a, b, c, detsum)
}

#[cold]
fn orient2d_adapt(a: Point2, b: Point2, c: Point2, detsum: f64) -> f64 {
    debug_assert!(
        within(&[a.x, a.y, b.x, b.y, c.x, c.y], DEG2_LO, DEG2_HI),
        "orient2d outside its window"
    );
    let (acx, acy, bcx, bcy) = (a.x - c.x, a.y - c.y, b.x - c.x, b.y - c.y);
    let mut h = [0.0; 4];
    let n = cross_exact(acx, bcy, acy, bcx, &mut h);
    let det = estimate(&h[..n]);
    if det >= CCW_BOUND_B * detsum || -det >= CCW_BOUND_B * detsum {
        return det;
    }
    let e = orient2d_exact(a, b, c);
    if e.is_nan() { orient2d_big(a, b, c) } else { e }
}

fn orient2d_exact(a: Point2, b: Point2, c: Point2) -> f64 {
    // usual case: the coordinate differences are exact, so four components suffice
    if let Some([acx, acy, bcx, bcy]) = exact_diffs([(a.x, c.x), (a.y, c.y), (b.x, c.x), (b.y, c.y)]) {
        let mut h = [0.0; 4];
        let n = cross_exact(acx, bcy, acy, bcx, &mut h);
        return estimate(&h[..n]);
    }
    let (acx, acy) = (diff2(a.x, c.x), diff2(a.y, c.y));
    let (bcx, bcy) = (diff2(b.x, c.x), diff2(b.y, c.y));
    let mut h = [0.0; 16];
    let n = cross_two(&acx, &bcy, &acy, &bcx, &mut h);
    estimate(&h[..n])
}

/// `orient2d` exactly, for any finite coordinates.
#[cold]
fn orient2d_big(a: Point2, b: Point2, c: Point2) -> f64 {
    if ![a.x, a.y, b.x, b.y, c.x, c.y].iter().all(|x| x.is_finite()) {
        return f64::NAN;
    }
    type B = Big<68>;
    let (acx, acy) = (B::of(a.x).sub(B::of(c.x)), B::of(a.y).sub(B::of(c.y)));
    let (bcx, bcy) = (B::of(b.x).sub(B::of(c.x)), B::of(b.y).sub(B::of(c.y)));
    acx.mul(bcy).sub(acy.mul(bcx)).to_f64(2148)
}

/// a - b as a two-component expansion (zero tail dropped).
fn diff2(a: f64, b: f64) -> ([f64; 2], usize) {
    let (x, y) = two_diff(a, b);
    if y == 0.0 { ([x, 0.0], 1) } else { ([y, x], 2) }
}

fn mul_small(e: &[f64], f: &[f64], h: &mut [f64; 8]) -> usize {
    let (mut p, mut q) = ([0.0; 4], [0.0; 4]);
    let np = scale_into(e, f[0], &mut p);
    if f.len() == 1 {
        h[..np].copy_from_slice(&p[..np]);
        return np;
    }
    let nq = scale_into(e, f[1], &mut q);
    sum_into(&p[..np], &q[..nq], h)
}

/// x1 * y2 - x2 * y1 for two-component inputs: at most 16 components.
fn cross_two(
    x1: &([f64; 2], usize),
    y2: &([f64; 2], usize),
    x2: &([f64; 2], usize),
    y1: &([f64; 2], usize),
    h: &mut [f64; 16],
) -> usize {
    let (mut p, mut q) = ([0.0; 8], [0.0; 8]);
    let np = mul_small(&x1.0[..x1.1], &y2.0[..y2.1], &mut p);
    let nq = mul_small(&x2.0[..x2.1], &y1.0[..y1.1], &mut q);
    for v in &mut q[..nq] {
        *v = -*v;
    }
    sum_into(&p[..np], &q[..nq], h)
}

/// a*b - c*d with the right sign (zero exactly when it is zero).
#[inline]
pub fn cross_sign(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let (l, r) = (a * b, c * d);
    let det = l - r;
    // sound for every finite input: rounding is monotone, so `det` never has the wrong sign,
    // only, at worst, zero (research/kernel-audit/float-verification.md)
    if det.abs() > CROSS_BOUND * (l.abs() + r.abs()) {
        return det;
    }
    debug_assert!(
        within(&[a, b, c, d], DEG2_LO, DEG2_HI),
        "cross_sign outside its window"
    );
    let mut h = [0.0; 4];
    let n = cross_exact(a, b, c, d, &mut h);
    let e = estimate(&h[..n]);
    if e.is_nan() { cross_big(a, b, c, d) } else { e }
}

/// The 2D cross product u x v with the right sign: positive when v is counter-clockwise of u.
#[inline]
pub fn cross2(u: Vec2, v: Vec2) -> f64 {
    cross_sign(u.x, v.y, u.y, v.x)
}

/// u . v with the right sign.
#[inline]
pub fn dot3(u: Vec3, v: Vec3) -> f64 {
    let (x, y, z) = (u.x * v.x, u.y * v.y, u.z * v.z);
    let dot = x + y + z;
    if dot.abs() > DOT3_BOUND * (x.abs() + y.abs() + z.abs()) + SLACK {
        return dot;
    }
    debug_assert!(
        within(&[u.x, u.y, u.z, v.x, v.y, v.z], DEG2_LO, DEG2_HI),
        "dot3 outside its window"
    );
    let (mut p, mut q) = ([0.0; 4], [0.0; 6]);
    let (x1, x0) = two_product(u.x, v.x);
    let (y1, y0) = two_product(u.y, v.y);
    let (z1, z0) = two_product(u.z, v.z);
    let np = sum_into(&[x0, x1], &[y0, y1], &mut p);
    let nq = sum_into(&p[..np], &[z0, z1], &mut q);
    let e = estimate(&q[..nq]);
    if e.is_nan() { dot3_big(u, v) } else { e }
}

#[cold]
fn dot3_big(u: Vec3, v: Vec3) -> f64 {
    if ![u.x, u.y, u.z, v.x, v.y, v.z].iter().all(|x| x.is_finite()) {
        return f64::NAN;
    }
    type B = Big<68>;
    let p = |a: f64, b: f64| B::of(a).mul(B::of(b));
    p(u.x, v.x).add(p(u.y, v.y)).add(p(u.z, v.z)).to_f64(2148)
}

/// The triple product a . (b x c) with the right sign: positive when a, b, c form a
/// right-handed set.
#[inline]
pub fn triple(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    // the whole predicate's precondition: products here multiply rounded products, so an
    // absolute (subnormal) error is amplified, and only the window rules it out
    debug_assert!(
        within(&[a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z], DEG3_LO, DEG3_HI),
        "triple outside its window"
    );
    let (yz, zy) = (b.y * c.z, b.z * c.y);
    let (zx, xz) = (b.z * c.x, b.x * c.z);
    let (xy, yx) = (b.x * c.y, b.y * c.x);
    let det = a.x * (yz - zy) + a.y * (zx - xz) + a.z * (xy - yx);
    let permanent = a.x.abs() * (yz.abs() + zy.abs())
        + a.y.abs() * (zx.abs() + xz.abs())
        + a.z.abs() * (xy.abs() + yx.abs());
    if det.abs() > TRIPLE_BOUND * permanent {
        return det;
    }
    let mut parts = [[0.0; 8]; 3];
    let mut lens = [0; 3];
    for (k, (ai, (p, q, r, s))) in [
        (a.x, (b.y, c.z, b.z, c.y)),
        (a.y, (b.z, c.x, b.x, c.z)),
        (a.z, (b.x, c.y, b.y, c.x)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut cr = [0.0; 4];
        let n = cross_exact(p, q, r, s, &mut cr);
        lens[k] = scale_into(&cr[..n], ai, &mut parts[k]);
    }
    let (mut ab, mut all) = ([0.0; 16], [0.0; 24]);
    let n = sum_into(&parts[0][..lens[0]], &parts[1][..lens[1]], &mut ab);
    let m = sum_into(&ab[..n], &parts[2][..lens[2]], &mut all);
    let e = estimate(&all[..m]);
    if e.is_nan() { triple_big(a, b, c) } else { e }
}

#[cold]
fn triple_big(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    if ![a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z]
        .iter()
        .all(|x| x.is_finite())
    {
        return f64::NAN;
    }
    type B = Big<104>;
    let cr = |p: f64, q: f64, r: f64, s: f64| B::of(p).mul(B::of(q)).sub(B::of(r).mul(B::of(s)));
    let x = B::of(a.x).mul(cr(b.y, c.z, b.z, c.y));
    let y = B::of(a.y).mul(cr(b.z, c.x, b.x, c.z));
    let z = B::of(a.z).mul(cr(b.x, c.y, b.y, c.x));
    x.add(y).add(z).to_f64(3222)
}

/// Orders two nonzero directions by their angle in (-pi, pi] (atan2's range), exactly.
#[inline]
pub fn angle_cmp(u: Vec2, v: Vec2) -> Ordering {
    // lower half, the positive x axis, upper half, the negative x axis
    let half = |w: Vec2| match (w.y < 0.0, w.y == 0.0, w.x >= 0.0) {
        (true, _, _) => 0,
        (_, true, true) => 1,
        (_, true, false) => 3,
        _ => 2,
    };
    half(u).cmp(&half(v)).then_with(|| {
        let c = cross2(u, v);
        if c > 0.0 {
            Ordering::Less
        } else if c < 0.0 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    })
}

/// Whether p is inside the closed polygon, by the parity of its edges' crossings with the ray
/// from p towards +x (an end exactly at p's height counts on one side only), exactly.
pub fn inside_polygon(p: Point2, poly: &[Point2]) -> bool {
    let Some(&last) = poly.last() else {
        return false;
    };
    let (mut a, mut a_above) = (last, last.y > p.y);
    let mut inside = false;
    for &b in poly {
        let b_above = b.y > p.y;
        if a_above != b_above {
            let o = orient2d(a, b, p);
            if if b_above { o > 0.0 } else { o < 0.0 } {
                inside = !inside;
            }
        }
        (a, a_above) = (b, b_above);
    }
    inside
}

/// True only when d is certainly strictly inside the circle through a, b, c
/// (counter-clockwise). Undecided cases, including exactly cocircular points
/// such as samples of one arc, answer false. For Delaunay flips this is
/// enough: flipping only on certainty still terminates, and leaving a
/// near-cocircular edge unflipped changes triangle shape, never validity.
#[inline]
pub fn certainly_in_circle(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    // the whole predicate's precondition (lifts multiply rounded products; see `triple`)
    debug_assert!(
        within(&[a.x, a.y, b.x, b.y, c.x, c.y, d.x, d.y], DEG4_LO, DEG4_HI),
        "certainly_in_circle outside its window: {a:?} {b:?} {c:?} {d:?}"
    );
    let (adx, ady) = (a.x - d.x, a.y - d.y);
    let (bdx, bdy) = (b.x - d.x, b.y - d.y);
    let (cdx, cdy) = (c.x - d.x, c.y - d.y);
    let (bdxcdy, cdxbdy) = (bdx * cdy, cdx * bdy);
    let (cdxady, adxcdy) = (cdx * ady, adx * cdy);
    let (adxbdy, bdxady) = (adx * bdy, bdx * ady);
    let alift = adx * adx + ady * ady;
    let blift = bdx * bdx + bdy * bdy;
    let clift = cdx * cdx + cdy * cdy;
    let det = alift * (bdxcdy - cdxbdy) + blift * (cdxady - adxcdy) + clift * (adxbdy - bdxady);
    let permanent = (bdxcdy.abs() + cdxbdy.abs()) * alift
        + (cdxady.abs() + adxcdy.abs()) * blift
        + (adxbdy.abs() + bdxady.abs()) * clift;
    det > ICC_BOUND * permanent
}

/// Positive if d lies inside the circle through a, b, c (taken counter-clockwise),
/// negative if outside, zero if on it.
pub fn incircle(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    // the whole predicate's precondition (lifts multiply rounded products; see `triple`)
    debug_assert!(
        within(&[a.x, a.y, b.x, b.y, c.x, c.y, d.x, d.y], DEG4_LO, DEG4_HI),
        "incircle outside its window: {a:?} {b:?} {c:?} {d:?}"
    );
    let (adx, ady) = (a.x - d.x, a.y - d.y);
    let (bdx, bdy) = (b.x - d.x, b.y - d.y);
    let (cdx, cdy) = (c.x - d.x, c.y - d.y);
    let (bdxcdy, cdxbdy) = (bdx * cdy, cdx * bdy);
    let (cdxady, adxcdy) = (cdx * ady, adx * cdy);
    let (adxbdy, bdxady) = (adx * bdy, bdx * ady);
    let alift = adx * adx + ady * ady;
    let blift = bdx * bdx + bdy * bdy;
    let clift = cdx * cdx + cdy * cdy;
    let det = alift * (bdxcdy - cdxbdy) + blift * (cdxady - adxcdy) + clift * (adxbdy - bdxady);
    let permanent = (bdxcdy.abs() + cdxbdy.abs()) * alift
        + (cdxady.abs() + adxcdy.abs()) * blift
        + (adxbdy.abs() + bdxady.abs()) * clift;
    let bound = ICC_BOUND * permanent;
    if det > bound || -det > bound {
        return det;
    }

    // stage B: exact for the rounded differences; their rounding is within the bound
    let det = incircle_exact_diffs([adx, ady, bdx, bdy, cdx, cdy]);
    let exact = [
        (a.x, d.x),
        (a.y, d.y),
        (b.x, d.x),
        (b.y, d.y),
        (c.x, d.x),
        (c.y, d.y),
    ]
    .iter()
    .all(|&(p, q)| two_diff(p, q).1 == 0.0);
    if exact || det >= ICC_BOUND_B * permanent || -det >= ICC_BOUND_B * permanent {
        return det;
    }
    let e = incircle_exact(a, b, c, d);
    if e.is_nan() { incircle_big(a, b, c, d) } else { e }
}

/// `incircle` exactly, for any finite coordinates.
#[cold]
fn incircle_big(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    if ![a.x, a.y, b.x, b.y, c.x, c.y, d.x, d.y]
        .iter()
        .all(|x| x.is_finite())
    {
        return f64::NAN;
    }
    type B = Big<136>;
    let df = |p: f64, q: f64| B::of(p).sub(B::of(q));
    let (adx, ady) = (df(a.x, d.x), df(a.y, d.y));
    let (bdx, bdy) = (df(b.x, d.x), df(b.y, d.y));
    let (cdx, cdy) = (df(c.x, d.x), df(c.y, d.y));
    let lift = |x: B, y: B| x.mul(x).add(y.mul(y));
    let ta = lift(adx, ady).mul(bdx.mul(cdy).sub(cdx.mul(bdy)));
    let tb = lift(bdx, bdy).mul(cdx.mul(ady).sub(adx.mul(cdy)));
    let tc = lift(cdx, cdy).mul(adx.mul(bdy).sub(bdx.mul(ady)));
    ta.add(tb).add(tc).to_f64(4296)
}

fn incircle_exact(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    if let Some(dx) = exact_diffs([
        (a.x, d.x),
        (a.y, d.y),
        (b.x, d.x),
        (b.y, d.y),
        (c.x, d.x),
        (c.y, d.y),
    ]) {
        return incircle_exact_diffs(dx);
    }
    incircle_exact_general(a, b, c, d)
}

/// The full determinant from exact differences, on the stack (at most 96 components).
fn incircle_exact_diffs([adx, ady, bdx, bdy, cdx, cdy]: [f64; 6]) -> f64 {
    let term = |x: f64, y: f64, cr: &[f64], out: &mut [f64; 32]| -> usize {
        let (mut t8, mut t16a, mut t16b) = ([0.0; 8], [0.0; 16], [0.0; 16]);
        let n8 = scale_into(cr, x, &mut t8);
        let na = scale_into(&t8[..n8], x, &mut t16a);
        let n8 = scale_into(cr, y, &mut t8);
        let nb = scale_into(&t8[..n8], y, &mut t16b);
        sum_into(&t16a[..na], &t16b[..nb], out)
    };
    let (mut bc, mut ca, mut ab) = ([0.0; 4], [0.0; 4], [0.0; 4]);
    let nbc = cross_exact(bdx, cdy, cdx, bdy, &mut bc);
    let nca = cross_exact(cdx, ady, adx, cdy, &mut ca);
    let nab = cross_exact(adx, bdy, bdx, ady, &mut ab);
    let (mut ta, mut tb, mut tc) = ([0.0; 32], [0.0; 32], [0.0; 32]);
    let na = term(adx, ady, &bc[..nbc], &mut ta);
    let nb = term(bdx, bdy, &ca[..nca], &mut tb);
    let nc = term(cdx, cdy, &ab[..nab], &mut tc);
    let mut tab = [0.0; 64];
    let nab2 = sum_into(&ta[..na], &tb[..nb], &mut tab);
    let mut fin = [0.0; 96];
    let n = sum_into(&tab[..nab2], &tc[..nc], &mut fin);
    estimate(&fin[..n])
}

fn incircle_exact_general(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    let (adx, ady) = (diff2(a.x, d.x), diff2(a.y, d.y));
    let (bdx, bdy) = (diff2(b.x, d.x), diff2(b.y, d.y));
    let (cdx, cdy) = (diff2(c.x, d.x), diff2(c.y, d.y));
    // lift (16) times cross (16): at most 512 components per term
    let term = |x: &([f64; 2], usize), y: &([f64; 2], usize), cr: &[f64], out: &mut [f64; 512]| -> usize {
        let (mut xx, mut yy, mut lift) = ([0.0; 8], [0.0; 8], [0.0; 16]);
        let nx = mul_small(&x.0[..x.1], &x.0[..x.1], &mut xx);
        let ny = mul_small(&y.0[..y.1], &y.0[..y.1], &mut yy);
        let nl = sum_into(&xx[..nx], &yy[..ny], &mut lift);
        let mut acc = [0.0; 512];
        let mut na = 0;
        let mut part = [0.0; 32];
        for &c in cr {
            let np = scale_into(&lift[..nl], c, &mut part);
            na = sum_into(&acc[..na], &part[..np], out);
            acc[..na].copy_from_slice(&out[..na]);
        }
        na
    };
    let (mut bc, mut ca, mut ab) = ([0.0; 16], [0.0; 16], [0.0; 16]);
    let nbc = cross_two(&bdx, &cdy, &cdx, &bdy, &mut bc);
    let nca = cross_two(&cdx, &ady, &adx, &cdy, &mut ca);
    let nab = cross_two(&adx, &bdy, &bdx, &ady, &mut ab);
    let (mut ta, mut tb, mut tc) = ([0.0; 512], [0.0; 512], [0.0; 512]);
    let na = term(&adx, &ady, &bc[..nbc], &mut ta);
    let nb = term(&bdx, &bdy, &ca[..nca], &mut tb);
    let nc = term(&cdx, &cdy, &ab[..nab], &mut tc);
    let mut tab = [0.0; 1024];
    let nab2 = sum_into(&ta[..na], &tb[..nb], &mut tab);
    let mut fin = [0.0; 1536];
    let n = sum_into(&tab[..nab2], &tc[..nc], &mut fin);
    estimate(&fin[..n])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// Integer-valued points scaled by 2^-shift: f64 holds them exactly and i128 gives exact signs.
    fn pt(x: i64, y: i64, shift: i32) -> Point2 {
        let s = f64::from_bits(((1023 - shift) as u64) << 52);
        Point2::new(x as f64 * s, y as f64 * s)
    }

    fn orient_ref(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> i32 {
        let v = (a.0 - c.0) as i128 * (b.1 - c.1) as i128 - (a.1 - c.1) as i128 * (b.0 - c.0) as i128;
        v.signum() as i32
    }

    fn incircle_ref(a: (i64, i64), b: (i64, i64), c: (i64, i64), d: (i64, i64)) -> i32 {
        let (adx, ady) = ((a.0 - d.0) as i128, (a.1 - d.1) as i128);
        let (bdx, bdy) = ((b.0 - d.0) as i128, (b.1 - d.1) as i128);
        let (cdx, cdy) = ((c.0 - d.0) as i128, (c.1 - d.1) as i128);
        let v = (adx * adx + ady * ady) * (bdx * cdy - cdx * bdy)
            + (bdx * bdx + bdy * bdy) * (cdx * ady - adx * cdy)
            + (cdx * cdx + cdy * cdy) * (adx * bdy - bdx * ady);
        v.signum() as i32
    }

    fn sign(v: f64) -> i32 {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    }

    /// `x · 2^k`, exactly (the result normal or zero).
    fn sc(x: i64, k: i32) -> f64 {
        let (mut v, mut k) = (x as f64, k);
        while k != 0 {
            let s = k.clamp(-1000, 1000);
            v *= f64::from_bits(((1023 + s) as u64) << 52);
            k -= s;
        }
        v
    }

    /// Outside the window (here the products underflow) the slow path is not exact, so this runs
    /// in release only: the fast filter, before the slack, answered -5e-324 for a positive
    /// determinant (research/kernel-audit/float-verification.md); now it does not decide.
    #[test]
    #[cfg(not(debug_assertions))]
    fn orient2d_underflow_counterexample() {
        let a = Point2::new(5.394455326111622e-162, 1.6245062719943995e-164);
        let b = Point2::new(1.3762009331530288e-157, 4.1443425003634566e-160);
        let c = Point2::new(-4.608837897596252e-178, 0.0);
        assert!(orient2d(a, b, c) >= 0.0);
    }

    /// Products that overflow: the expansions answer NaN, and the integer evaluation decides.
    /// Outside the window, so release only.
    #[test]
    #[cfg(not(debug_assertions))]
    fn overflow_is_exact() {
        let p = |x: f64, y: f64| Point2::new(x, y);
        assert!(cross_sign(1e300, 1e10, 1e300, 1e9) > 0.0);
        assert!(cross_sign(1e200, 1e200, 1e200, 1e199) > 0.0);
        let v = orient2d(p(1e300, 0.0), p(0.0, 1e300), p(-1e300, -1e300));
        assert!(v > 0.0 && v.is_finite());
        assert!(orient2d(p(1e300, 1e300), p(2e300, 2e300), p(0.0, 1e-300)) > 0.0);
        assert!(dot3(Vec3::new(1e300, 1.0, 0.0), Vec3::new(1e300, 0.0, 0.0)) > 0.0);
        assert!(
            triple(
                Vec3::new(1e300, 0.0, 0.0),
                Vec3::new(0.0, 1e300, 0.0),
                Vec3::new(0.0, 0.0, 1e10)
            ) > 0.0
        );
    }

    #[test]
    fn lattice_at_every_scale() {
        // near-degenerate integer configurations scaled by 2^k, exact signs from i128, at scales
        // inside every window (incircle's is the narrowest: [2^-210, 2^250]); the degree-2
        // predicates again at their own window's edges in `degree2_at_window_edges`
        let mut r = Rng::new(505);
        for k in [-210, -150, -60, 0, 60, 150, 228] {
            for _ in 0..400 {
                let a = (r.below(1 << 20) as i64 - (1 << 19), r.below(1 << 20) as i64);
                let (dx, dy) = (r.below(1 << 10) as i64 + 1, r.below(1 << 10) as i64);
                let b = (a.0 + dx, a.1 + dy);
                let m = r.below(7) as i64 - 3;
                let c = (
                    a.0 + dx * m + (r.below(3) as i64 - 1),
                    a.1 + dy * m + (r.below(3) as i64 - 1),
                );
                let q = |v: (i64, i64)| Point2::new(sc(v.0, k), sc(v.1, k));
                assert_eq!(
                    sign(orient2d(q(a), q(b), q(c))),
                    orient_ref(a, b, c),
                    "2^{k} {a:?} {b:?} {c:?}"
                );
                let i = |x: i64| x as i128;
                let want = (i(a.0) * i(b.1) - i(a.1) * i(b.0)).signum() as i32;
                assert_eq!(
                    sign(cross_sign(sc(a.0, k), sc(b.1, k), sc(a.1, k), sc(b.0, k))),
                    want,
                    "2^{k}"
                );
                // cocircular: a square's corners, the last nudged
                let s = r.below(1 << 8) as i64 + 1;
                let (cx, cy) = (r.below(1 << 12) as i64, r.below(1 << 12) as i64);
                let mut d = (cx, cy + s);
                d.0 += r.below(3) as i64 - 1;
                let sq = [(cx, cy), (cx + s, cy), (cx + s, cy + s), d];
                assert_eq!(
                    sign(incircle(q(sq[0]), q(sq[1]), q(sq[2]), q(sq[3]))),
                    incircle_ref(sq[0], sq[1], sq[2], sq[3]),
                    "2^{k} {sq:?}"
                );
                // coplanar vectors, nudged
                let v3 = |r: &mut Rng| {
                    [
                        r.below(1 << 12) as i64 - (1 << 11),
                        r.below(1 << 12) as i64 - (1 << 11),
                        r.below(1 << 12) as i64 - (1 << 11),
                    ]
                };
                let (va, vb) = (v3(&mut r), v3(&mut r));
                let mm = r.below(5) as i64 - 2;
                let vc = [
                    va[0] * mm + vb[0],
                    va[1] * mm + vb[1],
                    va[2] * mm + vb[2] + (r.below(3) as i64 - 1),
                ];
                let f = |v: [i64; 3]| Vec3::new(sc(v[0], k), sc(v[1], k), sc(v[2], k));
                let cr = [
                    i(vb[1]) * i(vc[2]) - i(vb[2]) * i(vc[1]),
                    i(vb[2]) * i(vc[0]) - i(vb[0]) * i(vc[2]),
                    i(vb[0]) * i(vc[1]) - i(vb[1]) * i(vc[0]),
                ];
                let want = (i(va[0]) * cr[0] + i(va[1]) * cr[1] + i(va[2]) * cr[2]).signum() as i32;
                assert_eq!(
                    sign(triple(f(va), f(vb), f(vc))),
                    want,
                    "2^{k} {va:?} {vb:?} {vc:?}"
                );
                let w = [cr[0] as i64 % (1 << 20), cr[1] as i64 % (1 << 20), 0];
                let want = (i(va[0]) * i(w[0]) + i(va[1]) * i(w[1])).signum() as i32;
                assert_eq!(sign(dot3(f(va), f(w))), want, "2^{k}");
            }
        }
    }

    #[test]
    fn degree2_at_window_edges() {
        let mut r = Rng::new(506);
        for k in [-480, -400, -300, 300, 400, 478] {
            for _ in 0..400 {
                let a = (r.below(1 << 20) as i64 - (1 << 19), r.below(1 << 20) as i64);
                let (dx, dy) = (r.below(1 << 10) as i64 + 1, r.below(1 << 10) as i64);
                let b = (a.0 + dx, a.1 + dy);
                let m = r.below(7) as i64 - 3;
                let c = (
                    a.0 + dx * m + (r.below(3) as i64 - 1),
                    a.1 + dy * m + (r.below(3) as i64 - 1),
                );
                let q = |v: (i64, i64)| Point2::new(sc(v.0, k), sc(v.1, k));
                assert_eq!(sign(orient2d(q(a), q(b), q(c))), orient_ref(a, b, c), "2^{k}");
                let i = |x: i64| x as i128;
                let want = (i(a.0) * i(b.1) - i(a.1) * i(b.0)).signum() as i32;
                assert_eq!(
                    sign(cross_sign(sc(a.0, k), sc(b.1, k), sc(a.1, k), sc(b.0, k))),
                    want
                );
                let want = (i(a.0) * i(c.0) + i(a.1) * i(c.1)).signum() as i32;
                let f = |v: (i64, i64)| Vec3::new(sc(v.0, k), sc(v.1, k), 0.0);
                assert_eq!(sign(dot3(f(a), f(c))), want);
            }
        }
    }

    #[test]
    fn orient_near_collinear() {
        let mut r = Rng::new(500);
        for _ in 0..20_000 {
            // c within one unit of the line through a and b; with offsets near 2^50 the
            // products lose far more than the determinant's size to rounding
            let a = (r.below(1 << 20) as i64 - (1 << 19), r.below(1 << 20) as i64);
            let (dx, dy) = ((r.next_u64() >> 14) as i64 + 1, (r.next_u64() >> 14) as i64);
            let b = (a.0 + dx, a.1 + dy);
            let k = r.below(7) as i64 - 3;
            let c = (
                a.0 + dx * k + (r.below(3) as i64 - 1),
                a.1 + dy * k + (r.below(3) as i64 - 1),
            );
            let shift = r.below(40) as i32;
            let got = orient2d(pt(a.0, a.1, shift), pt(b.0, b.1, shift), pt(c.0, c.1, shift));
            assert_eq!(sign(got), orient_ref(a, b, c), "{a:?} {b:?} {c:?}");
        }
    }

    #[test]
    fn incircle_near_cocircular() {
        let mut r = Rng::new(501);
        let circle = [
            (5, 0),
            (3, 4),
            (4, 3),
            (0, 5),
            (-3, 4),
            (-4, -3),
            (0, -5),
            (3, -4),
            (-5, 0),
        ];
        for _ in 0..20_000 {
            let s = r.below(1 << 22) as i64 + 1;
            let (cx, cy) = (r.below(1 << 26) as i64 - (1 << 25), r.below(1 << 26) as i64);
            let mut q = [(0i64, 0i64); 4];
            for p in q.iter_mut() {
                let (x, y) = circle[r.below(circle.len())];
                *p = (cx + x * s, cy + y * s);
            }
            if q.windows(2).any(|w| w[0] == w[1]) || q[0] == q[2] || q[0] == q[3] || q[1] == q[3] {
                continue;
            }
            q[3].0 += r.below(3) as i64 - 1;
            q[3].1 += r.below(3) as i64 - 1;
            let shift = r.below(30) as i32;
            let p: Vec<Point2> = q.iter().map(|&(x, y)| pt(x, y, shift)).collect();
            let got = incircle(p[0], p[1], p[2], p[3]);
            assert_eq!(sign(got), incircle_ref(q[0], q[1], q[2], q[3]), "{q:?}");
        }
    }

    #[test]
    fn random_agree_with_reference() {
        let mut r = Rng::new(502);
        for _ in 0..20_000 {
            let mut g = || {
                (
                    r.below(1 << 24) as i64 - (1 << 23),
                    r.below(1 << 24) as i64 - (1 << 23),
                )
            };
            let (a, b, c, d) = (g(), g(), g(), g());
            let p = |q: (i64, i64)| pt(q.0, q.1, 10);
            assert_eq!(sign(orient2d(p(a), p(b), p(c))), orient_ref(a, b, c));
            assert_eq!(sign(incircle(p(a), p(b), p(c), p(d))), incircle_ref(a, b, c, d));
        }
    }

    #[test]
    fn small_predicates_agree_with_integers() {
        let mut r = Rng::new(503);
        let v3 = |r: &mut Rng| {
            let mut g = || (r.below(1 << 26) as i64 - (1 << 25)) as f64;
            Vec3::new(g(), g(), g())
        };
        for _ in 0..20_000 {
            let (a, b, mut c) = (v3(&mut r), v3(&mut r), v3(&mut r));
            // often nearly coplanar: c a mix of a and b plus a nudge
            if r.bool() {
                let k = r.below(5) as f64 - 2.0;
                c = Vec3::new(
                    a.x * k + b.x,
                    a.y * k + b.y,
                    a.z * k + b.z + (r.below(3) as f64 - 1.0),
                );
            }
            let i = |x: f64| x as i128;
            let cx = i(b.y) * i(c.z) - i(b.z) * i(c.y);
            let cy = i(b.z) * i(c.x) - i(b.x) * i(c.z);
            let cz = i(b.x) * i(c.y) - i(b.y) * i(c.x);
            let want = (i(a.x) * cx + i(a.y) * cy + i(a.z) * cz).signum() as i32;
            assert_eq!(sign(triple(a, b, c)), want, "{a:?} {b:?} {c:?}");
            let want = (i(a.x) * i(c.x) + i(a.y) * i(c.y) + i(a.z) * i(c.z)).signum() as i32;
            assert_eq!(sign(dot3(a, c)), want);
            let want = (i(a.x) * i(b.y) - i(a.y) * i(b.x)).signum() as i32;
            assert_eq!(sign(cross2(Vec2::new(a.x, a.y), Vec2::new(b.x, b.y))), want);
        }
        // products whose rounding hides the difference entirely
        let x = 1.0 + f64::EPSILON;
        assert!(cross_sign(x, x, 1.0, 1.0 + 2.0 * f64::EPSILON) > 0.0);
        assert_eq!(cross_sign(3.0, 5.0, 5.0, 3.0), 0.0);
    }

    #[test]
    fn angles_order_like_atan2() {
        let mut r = Rng::new(504);
        let mut dirs: Vec<Vec2> = (0..400)
            .map(|_| Vec2::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0)))
            .collect();
        dirs.extend([
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, -1.0),
        ]);
        for u in &dirs {
            for v in &dirs {
                let (a, b) = (crate::dmath::atan2(u.y, u.x), crate::dmath::atan2(v.y, v.x));
                if (a - b).abs() > 1e-12 {
                    assert_eq!(angle_cmp(*u, *v), a.total_cmp(&b), "{u:?} {v:?}");
                }
            }
        }
        // the same direction at two lengths is one angle
        assert_eq!(
            angle_cmp(Vec2::new(0.3, 0.7), Vec2::new(0.6, 1.4)),
            Ordering::Equal
        );
    }

    #[test]
    fn exact_paths_directly() {
        let a = Point2::new(0.5, 0.5);
        let b = Point2::new(12.0, 12.0);
        let c = Point2::new(24.0, 24.0);
        assert_eq!(orient2d_exact(a, b, c), 0.0);
        let c2 = Point2::new(24.0, 24.0f64.next_up());
        assert!(orient2d_exact(a, b, c2) > 0.0);
        assert!(orient2d(a, b, c2) > 0.0);
        let sq = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        assert_eq!(incircle_exact(sq[0], sq[1], sq[2], sq[3]), 0.0);
        assert_eq!(incircle(sq[0], sq[1], sq[2], sq[3]), 0.0);
        let inside = Point2::new(0.0, 1.0f64.next_down());
        assert!(incircle(sq[0], sq[1], sq[2], inside) > 0.0);
    }

    #[test]
    fn stack_path_matches_general_path() {
        let mut r = Rng::new(77);
        let sign = |v: f64| (v > 0.0) as i32 - (v < 0.0) as i32;
        for _ in 0..20_000 {
            // near-cocircular: points on a circle, rounded, with tiny nudges
            let c = Point2::new(r.range(-10.0, 10.0), r.range(-10.0, 10.0));
            let rad = r.range(0.1, 10.0);
            let p: Vec<Point2> = (0..4)
                .map(|_| {
                    let a = r.range(0.0, crate::dmath::TAU);
                    let q = crate::predicates::tests::on_circle(c, rad, a);
                    if r.bool() {
                        Point2::new(q.x.next_up(), q.y)
                    } else {
                        q
                    }
                })
                .collect();
            let dx = [
                p[0].x - p[3].x,
                p[0].y - p[3].y,
                p[1].x - p[3].x,
                p[1].y - p[3].y,
                p[2].x - p[3].x,
                p[2].y - p[3].y,
            ];
            if exact_diffs([
                (p[0].x, p[3].x),
                (p[0].y, p[3].y),
                (p[1].x, p[3].x),
                (p[1].y, p[3].y),
                (p[2].x, p[3].x),
                (p[2].y, p[3].y),
            ])
            .is_some()
            {
                assert_eq!(
                    sign(incircle_exact_diffs(dx)),
                    sign(incircle_exact_general(p[0], p[1], p[2], p[3]))
                );
            }
        }
    }

    fn on_circle(c: Point2, r: f64, a: f64) -> Point2 {
        let (s, co) = crate::dmath::sin_cos(a);
        Point2::new(c.x + r * co, c.y + r * s)
    }

    mod reference {
        //! The original allocating expansion code, kept as an independent check.
        use super::super::{estimate, two_diff, two_product, two_sum};
        use crate::Point2;

        /// Sum of two nonoverlapping expansions (components in increasing magnitude),
        /// zero components removed.
        fn expansion_sum(e: &[f64], f: &[f64]) -> Vec<f64> {
            let mut h = Vec::with_capacity(e.len() + f.len());
            let (mut ei, mut fi) = (0, 0);
            let next = |ei: &mut usize, fi: &mut usize| {
                if *fi >= f.len() || (*ei < e.len() && e[*ei].abs() < f[*fi].abs()) {
                    *ei += 1;
                    e[*ei - 1]
                } else {
                    *fi += 1;
                    f[*fi - 1]
                }
            };
            if e.is_empty() && f.is_empty() {
                return h;
            }
            let mut q = next(&mut ei, &mut fi);
            while ei < e.len() || fi < f.len() {
                let n = next(&mut ei, &mut fi);
                let (s, err) = two_sum(q, n);
                if err != 0.0 {
                    h.push(err);
                }
                q = s;
            }
            if q != 0.0 || h.is_empty() {
                h.push(q);
            }
            h
        }

        fn scale_expansion(e: &[f64], b: f64) -> Vec<f64> {
            let mut h = Vec::with_capacity(2 * e.len());
            let Some((&first, rest)) = e.split_first() else {
                return h;
            };
            let (mut q, lo) = two_product(first, b);
            if lo != 0.0 {
                h.push(lo);
            }
            for &ei in rest {
                let (p1, p0) = two_product(ei, b);
                let (s, err) = two_sum(q, p0);
                if err != 0.0 {
                    h.push(err);
                }
                let (s2, err2) = two_sum(p1, s);
                if err2 != 0.0 {
                    h.push(err2);
                }
                q = s2;
            }
            if q != 0.0 || h.is_empty() {
                h.push(q);
            }
            h
        }

        fn mul(e: &[f64], f: &[f64]) -> Vec<f64> {
            f.iter().fold(Vec::new(), |acc, &fi| {
                expansion_sum(&acc, &scale_expansion(e, fi))
            })
        }

        fn neg(e: &[f64]) -> Vec<f64> {
            e.iter().map(|x| -x).collect()
        }

        fn diff(a: f64, b: f64) -> Vec<f64> {
            let (x, y) = two_diff(a, b);
            expansion_sum(&[y], &[x])
        }

        pub fn orient2d_exact(a: Point2, b: Point2, c: Point2) -> f64 {
            let (acx, acy) = (diff(a.x, c.x), diff(a.y, c.y));
            let (bcx, bcy) = (diff(b.x, c.x), diff(b.y, c.y));
            estimate(&expansion_sum(&mul(&acx, &bcy), &neg(&mul(&acy, &bcx))))
        }

        pub fn incircle_exact(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
            let (adx, ady) = (diff(a.x, d.x), diff(a.y, d.y));
            let (bdx, bdy) = (diff(b.x, d.x), diff(b.y, d.y));
            let (cdx, cdy) = (diff(c.x, d.x), diff(c.y, d.y));
            let lift = |x: &[f64], y: &[f64]| expansion_sum(&mul(x, x), &mul(y, y));
            let cross = |x1: &[f64], y1: &[f64], x2: &[f64], y2: &[f64]| {
                expansion_sum(&mul(x1, y2), &neg(&mul(x2, y1)))
            };
            let ta = mul(&lift(&adx, &ady), &cross(&bdx, &bdy, &cdx, &cdy));
            let tb = mul(&lift(&bdx, &bdy), &cross(&cdx, &cdy, &adx, &ady));
            let tc = mul(&lift(&cdx, &cdy), &cross(&adx, &ady, &bdx, &bdy));
            estimate(&expansion_sum(&expansion_sum(&ta, &tb), &tc))
        }
    }

    #[test]
    fn inexact_differences_match_reference() {
        let mut r = Rng::new(78);
        let sign = |v: f64| (v > 0.0) as i32 - (v < 0.0) as i32;
        for _ in 0..20_000 {
            // coordinates of very different magnitude so differences round, near-degenerate by construction
            let base = Point2::new(r.range(-1e6, 1e6), r.range(-1e6, 1e6));
            let rad = r.range(1e-3, 10.0);
            let p: Vec<Point2> = (0..4)
                .map(|_| {
                    let q = on_circle(base, rad, r.range(0.0, crate::dmath::TAU));
                    Point2::new(q.x + r.range(-1e-12, 1e-12), q.y)
                })
                .collect();
            assert_eq!(
                sign(incircle_exact_general(p[0], p[1], p[2], p[3])),
                sign(reference::incircle_exact(p[0], p[1], p[2], p[3]))
            );
            assert_eq!(
                sign(incircle(p[0], p[1], p[2], p[3])),
                sign(reference::incircle_exact(p[0], p[1], p[2], p[3]))
            );
            let l = Point2::new(
                p[0].x + (p[1].x - p[0].x) * 0.37,
                p[0].y + (p[1].y - p[0].y) * 0.37,
            );
            assert_eq!(
                sign(orient2d(p[0], p[1], l)),
                sign(reference::orient2d_exact(p[0], p[1], l))
            );
        }
    }
}
