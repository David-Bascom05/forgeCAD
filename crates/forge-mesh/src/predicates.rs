//! Sign-exact geometric predicates. A plain f64 evaluation is used when its
//! error bound proves the sign; otherwise the determinant is recomputed exactly
//! with floating-point expansions (Shewchuk, "Adaptive Precision Floating-Point
//! Arithmetic and Fast Robust Geometric Predicates", 1997). The returned value
//! always has the correct sign; its magnitude is an approximation.

use forge_math::Point2;

const EPS: f64 = f64::EPSILON * 0.5;
const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1
const CCW_BOUND: f64 = (3.0 + 16.0 * EPS) * EPS;
const ICC_BOUND: f64 = (10.0 + 96.0 * EPS) * EPS;

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

/// Positive if a, b, c run counter-clockwise, negative if clockwise, zero if collinear.
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> f64 {
    let left = (a.x - c.x) * (b.y - c.y);
    let right = (a.y - c.y) * (b.x - c.x);
    let det = left - right;
    let bound = CCW_BOUND * (left.abs() + right.abs());
    if det > bound || -det > bound {
        return det;
    }
    orient2d_exact(a, b, c)
}

fn orient2d_exact(a: Point2, b: Point2, c: Point2) -> f64 {
    let (acx, acy) = (diff(a.x, c.x), diff(a.y, c.y));
    let (bcx, bcy) = (diff(b.x, c.x), diff(b.y, c.y));
    estimate(&expansion_sum(&mul(&acx, &bcy), &neg(&mul(&acy, &bcx))))
}

/// Positive if d lies inside the circle through a, b, c (taken counter-clockwise),
/// negative if outside, zero if on it.
pub fn incircle(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
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
    incircle_exact(a, b, c, d)
}

fn incircle_exact(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    let (adx, ady) = (diff(a.x, d.x), diff(a.y, d.y));
    let (bdx, bdy) = (diff(b.x, d.x), diff(b.y, d.y));
    let (cdx, cdy) = (diff(c.x, d.x), diff(c.y, d.y));
    let lift = |x: &[f64], y: &[f64]| expansion_sum(&mul(x, x), &mul(y, y));
    let cross =
        |x1: &[f64], y1: &[f64], x2: &[f64], y2: &[f64]| expansion_sum(&mul(x1, y2), &neg(&mul(x2, y1)));
    let ta = mul(&lift(&adx, &ady), &cross(&bdx, &bdy, &cdx, &cdy));
    let tb = mul(&lift(&bdx, &bdy), &cross(&cdx, &cdy, &adx, &ady));
    let tc = mul(&lift(&cdx, &cdy), &cross(&adx, &ady, &bdx, &bdy));
    estimate(&expansion_sum(&expansion_sum(&ta, &tb), &tc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;

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
}
