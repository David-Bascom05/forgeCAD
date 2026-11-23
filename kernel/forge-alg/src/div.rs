//! `divrem` (Knuth's algorithm D) and `gcd` (Lehmer) on [`Big`], by its public variants and
//! methods only. They live in this crate, not in `forge_math::exact`, because any code added to
//! `forge-math` moves rustc's codegen-unit partitioning of that crate, and with it the inlining of
//! `Big`'s existing callers and the speed gate's exact counts (measured, 2026-11-23).

use core::cmp::Ordering;
use forge_math::exact::{Big, INL};

// Copies of `exact`'s private limb helpers, by `Big`'s public variants.

/// `|x|`'s limbs, trimmed, and its sign (`exact`'s `view`, by the public variants).
fn mag(x: &Big) -> (bool, Vec<u64>) {
    match x {
        Big::S(v) => {
            let a = v.unsigned_abs();
            let mut m = vec![a as u64, (a >> 64) as u64];
            while m.last() == Some(&0) {
                m.pop();
            }
            (*v < 0, m)
        }
        Big::M { neg, n, w } => (*neg, w[..*n as usize].to_vec()),
        Big::L { neg, mag } => (*neg, mag.clone()),
    }
}

/// The value from sign and limbs, in its one representation (`exact`'s `from_mag`).
fn canon(neg: bool, m0: &[u64]) -> Big {
    let mut n = m0.len();
    while n > 0 && m0[n - 1] == 0 {
        n -= 1;
    }
    let m = &m0[..n];
    if m.len() <= 2 {
        let v = m.first().copied().unwrap_or(0) as u128 | (m.get(1).copied().unwrap_or(0) as u128) << 64;
        if v < (1u128 << 126) {
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

/// `exact`'s `cmp_mag`, on trimmed magnitudes.
fn cmp_mag(a: &[u64], b: &[u64]) -> Ordering {
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
        let (_, am) = mag(a);
        let (_, bm) = mag(b);
        match cmp_mag(&am, &bm) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

/// `⌊|x| / 2^sh⌋` truncated to 64 bits (the leading digits Lehmer's step reads).
fn top_at(x: &Big, sh: u32) -> u64 {
    let (_, m) = mag(x);
    let (w, b) = ((sh / 64) as usize, sh % 64);
    let lo = m.get(w).copied().unwrap_or(0) as u128;
    let hi = m.get(w + 1).copied().unwrap_or(0) as u128;
    (((hi << 64) | lo) >> b) as u64
}

/// `(q, r)` with `a = q o + r`, `q` truncated toward zero and `r` of `a`'s sign
/// (`|r| < |o|`); `o = 0` gives `(0, a)`.
pub fn divrem(a: &Big, o: &Big) -> (Big, Big) {
    if o.is_zero() {
        return (Big::zero(), a.clone());
    }
    if let (Big::S(x), Big::S(y)) = (a, o) {
        return (Big::S(x / y), Big::S(x % y));
    }
    let (an, am) = mag(a);
    let (bn, bm) = mag(o);
    let (q, r) = divrem_mag(&am, &bm);
    (canon(an != bn, &q), canon(an, &r))
}

/// The gcd of `|self|` and `|o|` (`gcd(0, 0) = 0`). Lehmer's algorithm (TAOCP 4.5.2, L) while
/// the larger is past 2¹²⁶: the leading 64 bits of both decide a run of Euclid steps, applied
/// at once as a unimodular 2×2 matrix (so the gcd is kept whatever the run), a full division
/// when the run is empty; then binary on `u128`.
pub fn gcd(a0: &Big, o: &Big) -> Big {
    let mut x = a0.abs();
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
            let r = divrem(&x, &y).1;
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
