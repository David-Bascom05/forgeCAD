//! Exact signs for region.rs's three squared comparisons (verify/sheet's Region.lean: `near`,
//! `within`, `inDisc`): a float filter where its error bound proves the sign, else the value
//! recomputed exactly with floating-point expansions, built from forge-math predicates.rs's own
//! helpers, the ones orient2d's exact stage uses (Shewchuk 1997: `two_diff`, `two_product`,
//! `sum_into`, `scale_into`, `estimate`), so their proofs cover this. Every input is a double, so
//! a dyadic rational, and every expansion here is exact; the sign of a nonoverlapping expansion
//! is its largest component's (`estimate` has it).
//!
//! The filters: each value is a sum of products of rounded differences of doubles, `v`, with an
//! absolute error at most `FILTER · m`, `m` the sum of the products' magnitudes (each difference
//! rounds once, relatively, each product and sum once more: well under 8 ε of `m`; 16 ε is
//! taken). Where `|v| > FILTER · m` the float sign is the exact one; else the exact path decides.
//! Like predicates.rs, this assumes no product underflows inexactly (inputs within its windows)
//! and every operation is finite.

use forge_math::predicates::{estimate, scale_into, sum_into, two_diff, two_product};

const EPS: f64 = f64::EPSILON * 0.5;
/// The filters' relative bound.
const FILTER: f64 = 16.0 * EPS;

/// `a − b` exactly: one or two components.
fn diff(a: f64, b: f64) -> Vec<f64> {
    let (x, y) = two_diff(a, b);
    if y == 0.0 { vec![x] } else { vec![y, x] }
}

/// `a · b` exactly: one or two components.
fn product(a: f64, b: f64) -> Vec<f64> {
    let (x, y) = two_product(a, b);
    if y == 0.0 { vec![x] } else { vec![y, x] }
}

/// `e + f` exactly.
fn add(e: &[f64], f: &[f64]) -> Vec<f64> {
    let mut h = vec![0.0; e.len() + f.len()];
    let n = sum_into(e, f, &mut h);
    h.truncate(n);
    h
}

fn neg(e: &[f64]) -> Vec<f64> {
    e.iter().map(|x| -x).collect()
}

/// `e · f` exactly: `e` scaled by each component of `f`, summed.
fn mul(e: &[f64], f: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut t = vec![0.0; 2 * e.len()];
    for &b in f {
        let n = scale_into(e, b, &mut t);
        out = add(&out, &t[..n]);
    }
    out
}

/// The sign of an expansion: −1, 0 or 1.
fn sign(e: &[f64]) -> f64 {
    let s = estimate(e);
    if s > 0.0 {
        1.0
    } else if s < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// The sign of |p − c|² − r², exactly.
pub fn disc(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> f64 {
    let (dx, dy) = (px - cx, py - cy);
    let v = dx * dx + dy * dy - r * r;
    if v.abs() > FILTER * (dx * dx + dy * dy + r * r) {
        return v.signum();
    }
    let (ex, ey) = (diff(px, cx), diff(py, cy));
    let s = add(&mul(&ex, &ex), &mul(&ey, &ey));
    sign(&add(&s, &neg(&product(r, r))))
}

/// The sign of orient(a, b, c)² − l² |b − a|², exactly.
pub fn line_gap(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, l: f64) -> f64 {
    let (ux, uy, vx, vy) = (bx - ax, by - ay, cx - ax, cy - ay);
    let o = ux * vy - uy * vx;
    let n = ux * ux + uy * uy;
    let v = o * o - l * l * n;
    let m = (ux * vy).abs() + (uy * vx).abs();
    if v.abs() > FILTER * (m * m + l * l * n) {
        return v.signum();
    }
    let (eux, euy, evx, evy) = (diff(bx, ax), diff(by, ay), diff(cx, ax), diff(cy, ay));
    let o = add(&mul(&eux, &evy), &neg(&mul(&euy, &evx)));
    let n = add(&mul(&eux, &eux), &mul(&euy, &euy));
    let ln = mul(&product(l, l), &n);
    sign(&add(&mul(&o, &o), &neg(&ln)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ties_are_exact() {
        // on the circle exactly
        assert_eq!(disc(3.0, 4.0, 0.0, 0.0, 5.0), 0.0);
        assert_eq!(disc(0.0, 0.0, 0.0, 0.0, 1.0), -1.0);
        // off it by a rounding the floats lose: (0.1 + 0.2) - 0.1 is not 0.2, exactly
        let p = 0.1 + 0.2;
        assert_eq!(disc(p, 0.0, 0.1, 0.0, 0.2), 1.0);
        // exactly on the band's edge: orient 2, |b - a| 1, l 2
        assert_eq!(line_gap(0.0, 0.0, 1.0, 0.0, 5.0, 2.0, 2.0), 0.0);
        assert_eq!(line_gap(0.0, 0.0, 1.0, 0.0, 5.0, 2.0, 2.5), -1.0);
        assert_eq!(line_gap(0.0, 0.0, 1.0, 0.0, 5.0, 2.0, 1.5), 1.0);
        // a tie the floats would miss: 1e-5 off a line of length 3 in x at 1e-5 exactly
        let l = 1e-5;
        let s = line_gap(0.0, 0.0, 3.0, 0.0, 1.0, l, l);
        assert_eq!(s, 0.0);
    }
}
