//! Arithmetic as the Python importers did it, so a file translates to the same text: CPython 3.12's
//! `sum` (Neumaier), `math.hypot`/`math.dist` (its vector_norm), float `%`, and the number formats.
//! `sin`, `cos` and `atan2` are dmath's, as the Python's are (recreate/dmath.py): glibc's round their own
//! way, and an ulp off decides a near-tangent crossing or which of two equal circles comes first.

use forge_math::dmath;

pub type V2 = (f64, f64);
pub type V3 = (f64, f64, f64);

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = 2.0 * PI;

/// `sum(xs)` of floats, started from the int 0.
pub fn sum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = xs.into_iter();
    let Some(first) = it.next() else {
        return 0.0;
    };
    let (mut f, mut c) = (0.0 + first, 0.0);
    for x in it {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let split = |x: f64| {
        let c = 134_217_729.0 * x;
        let h = c - (c - x);
        (h, x - h)
    };
    let p = a * b;
    let ((ah, al), (bh, bl)) = (split(a), split(b));
    (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
}

fn fast_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let z = x - a;
    (x, b - z)
}

fn frexp_e(x: f64) -> i32 {
    let e = ((x.to_bits() >> 52) & 0x7ff) as i32;
    if e == 0 {
        // subnormal
        frexp_e(x * f64::from_bits(0x4350_0000_0000_0000)) - 54
    } else {
        e - 1022
    }
}

fn ldexp1(e: i32) -> f64 {
    f64::from_bits(((e + 1023) as u64) << 52)
}

/// CPython's vector_norm: `math.hypot(*v)`.
pub fn hypot(v: &[f64]) -> f64 {
    let mut a: Vec<f64> = v.iter().map(|x| x.abs()).collect();
    let max = a.iter().fold(0.0f64, |m, &x| if x > m { x } else { m });
    if a.iter().any(|x| x.is_nan()) {
        return if max.is_infinite() { max } else { f64::NAN };
    }
    norm(&mut a, max)
}

fn norm(a: &mut [f64], max: f64) -> f64 {
    if max.is_infinite() || max == 0.0 || a.len() <= 1 {
        return max;
    }
    let e = frexp_e(max);
    if e < -1023 {
        let m = f64::MIN_POSITIVE;
        a.iter_mut().for_each(|x| *x /= m);
        return m * norm(a, max / m);
    }
    let scale = ldexp1(-e);
    let (mut csum, mut frac1, mut frac2) = (1.0, 0.0, 0.0);
    for &x in a.iter() {
        let x = x * scale;
        let pr = two_prod(x, x);
        let sm = fast_sum(csum, pr.0);
        csum = sm.0;
        frac1 += pr.1;
        frac2 += sm.1;
    }
    let mut h = (csum - 1.0 + (frac1 + frac2)).sqrt();
    let pr = two_prod(-h, h);
    let sm = fast_sum(csum, pr.0);
    csum = sm.0;
    frac1 += pr.1;
    frac2 += sm.1;
    let x = csum - 1.0 + (frac1 + frac2);
    h += x / (2.0 * h);
    h / scale
}

pub fn dist2(a: V2, b: V2) -> f64 {
    hypot(&[a.0 - b.0, a.1 - b.1])
}

pub fn dist3(a: V3, b: V3) -> f64 {
    hypot(&[a.0 - b.0, a.1 - b.1, a.2 - b.2])
}

/// Python's float `%`: the sign of the divisor.
pub fn pmod(x: f64, w: f64) -> f64 {
    let m = x % w;
    if m != 0.0 {
        if (w < 0.0) != (m < 0.0) { m + w } else { m }
    } else {
        0.0f64.copysign(w)
    }
}

/// Python's `max(a, b)`: the first when equal.
pub fn pmax(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

pub fn pmin(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

pub fn atan2(y: f64, x: f64) -> f64 {
    dmath::atan2(y, x)
}

pub fn cos(x: f64) -> f64 {
    dmath::cos(x)
}

pub fn sin(x: f64) -> f64 {
    dmath::sin(x)
}

pub fn degrees(x: f64) -> f64 {
    x * (180.0 / PI)
}

fn fixed(x: f64, p: usize) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.into();
    }
    format!("{x:.p$}")
}

/// `f"{x:.10f}"`.
pub fn f10(x: f64) -> String {
    fixed(x, 10)
}

fn trimmed(s: String) -> String {
    let t = s.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-0" {
        "0".into()
    } else {
        t.into()
    }
}

/// cadfs.num: ten decimals, rounded as `round(x, 10)` does, zeros trimmed.
pub fn num(x: f64) -> String {
    let mut r = if x.is_finite() {
        f10(x).parse().unwrap_or(x)
    } else {
        x
    };
    if r == 0.0 {
        r = 0.0;
    }
    trimmed(fixed(r, 10))
}

/// cadfs.fine: fifteen decimals.
pub fn fine(x: f64) -> String {
    trimmed(fixed(x, 15))
}

pub fn vtext(v: V3) -> String {
    format!("{} {} {}", num(v.0), num(v.1), num(v.2))
}

pub fn vfine(v: V3) -> String {
    format!("{} {} {}", fine(v.0), fine(v.1), fine(v.2))
}

/// `round(x, 4)` as a key: equal keys for equal rounded values.
pub fn key4(x: f64) -> String {
    let s = fixed(x, 4);
    if s == "-0.0000" { "0.0000".into() } else { s }
}

pub fn add(a: V3, b: V3, s: f64) -> V3 {
    (a.0 + b.0 * s, a.1 + b.1 * s, a.2 + b.2 * s)
}

pub fn dot(a: V3, b: V3) -> f64 {
    sum([a.0 * b.0, a.1 * b.1, a.2 * b.2])
}

pub fn neg(a: V3) -> V3 {
    (-a.0, -a.1, -a.2)
}

pub fn cross(a: V3, b: V3) -> V3 {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

/// cadfs.unit; None where Python divides by zero.
pub fn unit(a: V3) -> Option<V3> {
    let l = dot(a, a).sqrt();
    (l != 0.0).then(|| (a.0 / l, a.1 / l, a.2 / l))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_python_did() {
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-1e-11), "0");
        assert_eq!(num(0.00048828125), "0.0004882812");
        assert_eq!(num(-2.5), "-2.5");
        assert_eq!(fine(0.1 + 0.2), "0.3");
        assert_eq!(f10(-1e-12), "-0.0000000000");
        assert_eq!(key4(-0.00001), "0.0000");
    }

    #[test]
    fn sums_and_norms_as_cpython() {
        // values checked against CPython 3.12
        assert_eq!(sum([1e16, 1.0, -1e16]), 1.0);
        assert_eq!(sum([0.1, 0.2, 0.3]), 0.6);
        assert_eq!(hypot(&[3.0, 4.0]), 5.0);
        assert_eq!(hypot(&[0.1, 0.2]), 0.223_606_797_749_979);
        assert_eq!(hypot(&[1e-320, 1e-320]), 1.414e-320);
        assert_eq!(pmod(-1.0, TAU), TAU - 1.0);
        assert_eq!(pmod(-0.0, TAU).to_bits(), 0.0f64.to_bits());
    }
}
