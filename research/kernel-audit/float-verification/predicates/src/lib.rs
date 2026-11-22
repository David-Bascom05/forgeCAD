//! `cross_sign` and `orient2d` from `kernel/forge-math/src/predicates.rs`, with the same
//! arithmetic in the same order, spelled so that Aeneas (2e3d448) can translate them:
//!
//! 1. `f64` operators and comparisons are trait-method calls (`a.mul(b)`, `a.gt(&b)`): Aeneas
//!    rejects `f64` binary operations ("Invalid inputs for binop") but translates calls of the
//!    `Add`/`Sub`/`Mul`/`Neg`/`PartialOrd`/`PartialEq` impls of `f64` as opaque functions.
//! 2. `f64` literals are `f64::from_bits(..)` with the literal's encoding: Aeneas cannot extract
//!    float literals.
//! 3. `sum_into`'s `next` closure is inlined, its `for _ in 1..total` and `scale_into`'s
//!    `for &ei in rest` are `while` loops over an index, and `estimate`'s `e.iter().sum()` and
//!    `e.last().copied().unwrap_or(0.0)` are written out.
//! 4. `exact_diffs` (a const-generic array with `iter_mut().zip`) is written for the four
//!    differences `orient2d_exact` asks it for.
//! 5. `Point2` is a local struct with the same fields.
//! 6. The fallback for a NaN slow-path result (`Big`, `cross_big`, `orient2d_big`) is the
//!    kernel's code verbatim, in module `big`, which Aeneas keeps opaque. `e.is_nan()` is
//!    written `e.ne(&e)`. The kernel's `debug_assert!`s are left out (no effect in release).
//!
//! `tests` checks every function against the kernel's, bit for bit.

use core::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

/// `0.0`
#[inline]
pub fn zero() -> f64 {
    f64::from_bits(0x0)
}
/// `SPLITTER = 2^27 + 1`
#[inline]
pub fn splitter() -> f64 {
    f64::from_bits(0x41a0000002000000)
}
/// `CCW_BOUND = (3 + 16 EPS) EPS`
#[inline]
pub fn ccw_bound() -> f64 {
    f64::from_bits(0x3cb8000000000004)
}
/// `CCW_BOUND_B = (2 + 12 EPS) EPS`
#[inline]
pub fn ccw_bound_b() -> f64 {
    f64::from_bits(0x3cb0000000000003)
}
/// `CROSS_BOUND = (2 + 8 EPS) EPS`
#[inline]
pub fn cross_bound() -> f64 {
    f64::from_bits(0x3cb0000000000002)
}

/// The kernel's exact integer fallback for NaN results, verbatim. `gen.sh` keeps this module opaque
/// (`charon --opaque fv_predicates::big`): Aeneas does not translate it, and the Lean side
/// gives it no meaning (`FunsExternal.lean`), so no theorem can say anything about inputs
/// that reach it.
pub mod big {
    use super::Point2;

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
    pub fn cross_big(a: f64, d: f64, b: f64, c: f64) -> f64 {
        if ![a, b, c, d].iter().all(|x| x.is_finite()) {
            return f64::NAN;
        }
        type B = Big<68>;
        B::of(a).mul(B::of(d)).sub(B::of(b).mul(B::of(c))).to_f64(2148)
    }

    /// `orient2d` exactly, for any finite coordinates.
    #[cold]
    pub fn orient2d_big(a: Point2, b: Point2, c: Point2) -> f64 {
        if ![a.x, a.y, b.x, b.y, c.x, c.y].iter().all(|x| x.is_finite()) {
            return f64::NAN;
        }
        type B = Big<68>;
        let (acx, acy) = (B::of(a.x).sub(B::of(c.x)), B::of(a.y).sub(B::of(c.y)));
        let (bcx, bcy) = (B::of(b.x).sub(B::of(c.x)), B::of(b.y).sub(B::of(c.y)));
        acx.mul(bcy).sub(acy.mul(bcx)).to_f64(2148)
    }
}

pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a.add(b);
    let bv = x.sub(a);
    let av = x.sub(bv);
    (x, (a.sub(av)).add(b.sub(bv)))
}

pub fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let x = a.sub(b);
    let bv = a.sub(x);
    let av = x.add(bv);
    (x, (a.sub(av)).add(bv.sub(b)))
}

pub fn split(a: f64) -> (f64, f64) {
    let c = splitter().mul(a);
    let hi = c.sub(c.sub(a));
    (hi, a.sub(hi))
}

pub fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a.mul(b);
    let (ahi, alo) = split(a);
    let (bhi, blo) = split(b);
    let err = ((x.sub(ahi.mul(bhi))).sub(alo.mul(bhi))).sub(ahi.mul(blo));
    (x, (alo.mul(blo)).sub(err))
}

/// The sign of an expansion is the sign of its largest component.
pub fn estimate(e: &[f64]) -> f64 {
    let top = if e.len() == 0 { zero() } else { e[e.len() - 1] };
    // `e.iter().sum()`: a fold from -0.0 (`Sum for f64` starts at -0.0)
    let mut sum = f64::from_bits(0x8000000000000000);
    let mut i = 0;
    while i < e.len() {
        sum = sum.add(e[i]);
        i += 1;
    }
    if (sum.gt(&zero()) == top.gt(&zero())) && (sum.lt(&zero()) == top.lt(&zero())) {
        sum
    } else {
        top
    }
}

/// `e + f` into `h`, both inputs nonoverlapping and in increasing magnitude;
/// zero components dropped. Returns the length used. No allocation.
pub fn sum_into(e: &[f64], f: &[f64], h: &mut [f64]) -> usize {
    let mut ei = 0;
    let mut fi = 0;
    let mut n = 0;
    let total = e.len() + f.len();
    if total == 0 {
        return 0;
    }
    // `next()`, inlined
    let mut q;
    if fi >= f.len() || (ei < e.len() && e[ei].abs().lt(&f[fi].abs())) {
        ei += 1;
        q = e[ei - 1];
    } else {
        fi += 1;
        q = f[fi - 1];
    }
    let mut k = 1;
    while k < total {
        let nx;
        if fi >= f.len() || (ei < e.len() && e[ei].abs().lt(&f[fi].abs())) {
            ei += 1;
            nx = e[ei - 1];
        } else {
            fi += 1;
            nx = f[fi - 1];
        }
        let (s, err) = two_sum(q, nx);
        if err.ne(&zero()) {
            h[n] = err;
            n += 1;
        }
        q = s;
        k += 1;
    }
    if q.ne(&zero()) || n == 0 {
        h[n] = q;
        n += 1;
    }
    n
}

/// a*d - b*c exactly, as up to four components.
pub fn cross_exact(a: f64, d: f64, b: f64, c: f64, h: &mut [f64; 4]) -> usize {
    let (x1, y1) = two_product(a, d);
    let (x2, y2) = two_product(b, c);
    sum_into(&[y1, x1], &[y2.neg(), x2.neg()], h)
}

/// a*b - c*d with the right sign (zero exactly when it is zero).
pub fn cross_sign(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let l = a.mul(b);
    let r = c.mul(d);
    let det = l.sub(r);
    if det.abs().gt(&cross_bound().mul(l.abs().add(r.abs()))) {
        return det;
    }
    let mut h = [zero(); 4];
    let n = cross_exact(a, b, c, d, &mut h);
    let e = estimate(&h[..n]);
    // the kernel's `e.is_nan()`
    if e.ne(&e) { big::cross_big(a, b, c, d) } else { e }
}

/// Positive if a, b, c run counter-clockwise, negative if clockwise, zero if collinear.
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> f64 {
    let left = (a.x.sub(c.x)).mul(b.y.sub(c.y));
    let right = (a.y.sub(c.y)).mul(b.x.sub(c.x));
    let det = left.sub(right);
    let detsum = left.abs().add(right.abs());
    let bound = ccw_bound().mul(detsum);
    if det.gt(&bound) || det.neg().gt(&bound) {
        return det;
    }
    orient2d_adapt(a, b, c, detsum)
}

pub fn orient2d_adapt(a: Point2, b: Point2, c: Point2, detsum: f64) -> f64 {
    let acx = a.x.sub(c.x);
    let acy = a.y.sub(c.y);
    let bcx = b.x.sub(c.x);
    let bcy = b.y.sub(c.y);
    let mut h = [zero(); 4];
    let n = cross_exact(acx, bcy, acy, bcx, &mut h);
    let det = estimate(&h[..n]);
    if det.ge(&ccw_bound_b().mul(detsum)) || det.neg().ge(&ccw_bound_b().mul(detsum)) {
        return det;
    }
    let e = orient2d_exact(a, b, c);
    if e.ne(&e) { big::orient2d_big(a, b, c) } else { e }
}

/// `exact_diffs` for the four differences of `orient2d_exact`.
pub fn exact_diffs4(a: Point2, b: Point2, c: Point2) -> Option<[f64; 4]> {
    let (x0, t0) = two_diff(a.x, c.x);
    if t0.ne(&zero()) {
        return None;
    }
    let (x1, t1) = two_diff(a.y, c.y);
    if t1.ne(&zero()) {
        return None;
    }
    let (x2, t2) = two_diff(b.x, c.x);
    if t2.ne(&zero()) {
        return None;
    }
    let (x3, t3) = two_diff(b.y, c.y);
    if t3.ne(&zero()) {
        return None;
    }
    Some([x0, x1, x2, x3])
}

pub fn orient2d_exact(a: Point2, b: Point2, c: Point2) -> f64 {
    // usual case: the coordinate differences are exact, so four components suffice
    if let Some([acx, acy, bcx, bcy]) = exact_diffs4(a, b, c) {
        let mut h = [zero(); 4];
        let n = cross_exact(acx, bcy, acy, bcx, &mut h);
        return estimate(&h[..n]);
    }
    let acx = diff2(a.x, c.x);
    let acy = diff2(a.y, c.y);
    let bcx = diff2(b.x, c.x);
    let bcy = diff2(b.y, c.y);
    let mut h = [zero(); 16];
    let n = cross_two(&acx, &bcy, &acy, &bcx, &mut h);
    estimate(&h[..n])
}

/// a - b as a two-component expansion (zero tail dropped).
pub fn diff2(a: f64, b: f64) -> ([f64; 2], usize) {
    let (x, y) = two_diff(a, b);
    if y.eq(&zero()) {
        ([x, zero()], 1)
    } else {
        ([y, x], 2)
    }
}

/// `e * b` into `h` (needs 2 * e.len() slots). No allocation.
pub fn scale_into(e: &[f64], b: f64, h: &mut [f64]) -> usize {
    if e.len() == 0 {
        return 0;
    }
    let first = e[0];
    let mut n = 0;
    let (mut q, lo) = two_product(first, b);
    if lo.ne(&zero()) {
        h[n] = lo;
        n += 1;
    }
    let mut i = 1;
    while i < e.len() {
        let ei = e[i];
        let (p1, p0) = two_product(ei, b);
        let (s, err) = two_sum(q, p0);
        if err.ne(&zero()) {
            h[n] = err;
            n += 1;
        }
        let (s2, err2) = two_sum(p1, s);
        if err2.ne(&zero()) {
            h[n] = err2;
            n += 1;
        }
        q = s2;
        i += 1;
    }
    if q.ne(&zero()) || n == 0 {
        h[n] = q;
        n += 1;
    }
    n
}

pub fn mul_small(e: &[f64], f: &[f64], h: &mut [f64; 8]) -> usize {
    let mut p = [zero(); 4];
    let mut q = [zero(); 4];
    let np = scale_into(e, f[0], &mut p);
    if f.len() == 1 {
        let mut i = 0;
        while i < np {
            h[i] = p[i];
            i += 1;
        }
        return np;
    }
    let nq = scale_into(e, f[1], &mut q);
    sum_into(&p[..np], &q[..nq], h)
}

/// x1 * y2 - x2 * y1 for two-component inputs: at most 16 components.
pub fn cross_two(
    x1: &([f64; 2], usize),
    y2: &([f64; 2], usize),
    x2: &([f64; 2], usize),
    y1: &([f64; 2], usize),
    h: &mut [f64; 16],
) -> usize {
    let mut p = [zero(); 8];
    let mut q = [zero(); 8];
    let np = mul_small(&x1.0[..x1.1], &y2.0[..y2.1], &mut p);
    let nq = mul_small(&x2.0[..x2.1], &y1.0[..y1.1], &mut q);
    let mut i = 0;
    while i < nq {
        q[i] = q[i].neg();
        i += 1;
    }
    sum_into(&p[..np], &q[..nq], h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::predicates as k;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// random doubles of every scale, with many near-collinear and repeated values
        fn f(&mut self, base: f64) -> f64 {
            match self.next() % 6 {
                0 => f64::from_bits(self.next()),
                1 => base,
                2 => base * (1.0 + (self.next() % 7) as f64 * f64::EPSILON),
                3 => (self.next() % 9) as f64 - 4.0,
                4 => base + 2f64.powi((self.next() % 60) as i32 - 30),
                _ => (self.next() as f64 / u64::MAX as f64 - 0.5) * 2f64.powi((self.next() % 40) as i32 - 20),
            }
        }
    }

    fn same(x: f64, y: f64) -> bool {
        x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
    }

    #[test]
    fn matches_the_kernel() {
        let mut r = Rng(0x9e3779b97f4a7c15);
        for i in 0..2_000_000 {
            // every scale, the extremes included (the integer fallback)
            let base = r.f(1.0) * 2f64.powi((i % 41) as i32 * 50 - 1000);
            let (a, b, c, d) = (r.f(base), r.f(base), r.f(base), r.f(base));
            assert!(
                same(cross_sign(a, b, c, d), k::cross_sign(a, b, c, d)),
                "cross_sign {a:e} {b:e} {c:e} {d:e}"
            );
            let (e, g) = (r.f(base), r.f(base));
            let p = |x: f64, y: f64| (Point2 { x, y }, forge_math::Point2::new(x, y));
            let (pa, ka) = p(a, b);
            let (pb, kb) = p(c, d);
            let (pc, kc) = p(e, g);
            assert!(
                same(orient2d(pa, pb, pc), k::orient2d(ka, kb, kc)),
                "orient2d {a:e} {b:e} {c:e} {d:e} {e:e} {g:e}"
            );
        }
    }

    #[test]
    fn orient2d_underflow_counterexample() {
        // the exact determinant is positive; the filter answers the least negative subnormal
        let a = Point2 {
            x: 5.394455326111622e-162,
            y: 1.6245062719943995e-164,
        };
        let b = Point2 {
            x: 1.3762009331530288e-157,
            y: 4.1443425003634566e-160,
        };
        let c = Point2 {
            x: -4.608837897596252e-178,
            y: 0.0,
        };
        // the exact determinant is positive, but the products underflow (the precondition
        // fails) and the fast filter answers -5e-324, as the kernel does
        assert_eq!(orient2d(a, b, c), -5e-324);
        let k = |p: Point2| forge_math::Point2::new(p.x, p.y);
        assert_eq!(
            k::orient2d(k(a), k(b), k(c)).to_bits(),
            orient2d(a, b, c).to_bits()
        );
    }
}
