//! Exact signs of region.rs's squared comparisons (verify/sheet Region.lean `near`, `within`,
//! `inDisc`; Exact.lean, Filter16.lean), forge-sheet's exact.rs decision for decision, written
//! for Aeneas: a float filter where its error bound proves the sign (Filter16.lean
//! `disc_filter_sound`, `line_gap_filter_sound`), else the value recomputed exactly as a
//! floating-point expansion (Shewchuk 1997: two-sum, Dekker's two-product, grow-expansion with
//! zero elimination), its sign the largest component's.
//!
//! Every function here is the same sequence of float operations as forge-sheet's exact.rs, so the
//! results are bit-identical (tests/core.rs); only the loops are spelled as `while`.

/// `16 ε`, `ε = 2⁻⁵³`: the filters' relative bound.
const FILTER: f64 = 16.0 * (f64::EPSILON * 0.5);
/// Dekker's splitter, `2²⁷ + 1`.
const SPLITTER: f64 = 134_217_729.0;

/// `a + b` and its rounding error, exactly (Knuth).
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

/// Veltkamp's split of `a` into two 26-bit halves.
pub fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// `a b` and its rounding error, exactly (Dekker).
pub fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let err = x - ah * bh - al * bh - ah * bl;
    (x, al * bl - err)
}

/// The sign of a float: `1`, `-1` or `0` (`signum` on a nonzero value).
pub fn sgn(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// The expansion of one double: empty for zero.
pub fn exp_of(x: f64) -> Vec<f64> {
    let mut e = Vec::new();
    if x != 0.0 {
        e.push(x);
    }
    e
}

/// `e + b` exactly (Grow-Expansion with zero elimination).
pub fn grow(e: &[f64], b: f64) -> Vec<f64> {
    let mut q = b;
    let mut out = Vec::new();
    let mut i = 0;
    while i < e.len() {
        let (s, lo) = two_sum(q, e[i]);
        if lo != 0.0 {
            out.push(lo);
        }
        q = s;
        i += 1;
    }
    if q != 0.0 {
        out.push(q);
    }
    out
}

/// `a − b` exactly.
pub fn exp_diff(a: f64, b: f64) -> Vec<f64> {
    let e = exp_of(a);
    grow(&e, -b)
}

/// `e + o` exactly: each of `o`'s components grown in.
pub fn exp_add(e: &[f64], o: &[f64]) -> Vec<f64> {
    let mut s = e.to_vec();
    let mut i = 0;
    while i < o.len() {
        s = grow(&s, o[i]);
        i += 1;
    }
    s
}

/// `−e`.
pub fn exp_neg(e: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < e.len() {
        out.push(-e[i]);
        i += 1;
    }
    out
}

/// `e o` exactly: every pair's two-product, grown in (error, then product).
pub fn exp_mul(e: &[f64], o: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < e.len() {
        let mut j = 0;
        while j < o.len() {
            let (x, y) = two_product(e[i], o[j]);
            out = grow(&out, y);
            out = grow(&out, x);
            j += 1;
        }
        i += 1;
    }
    out
}

/// An expansion's sign: its last (largest) nonzero component's.
pub fn exp_sign(e: &[f64]) -> f64 {
    let mut s = 0.0;
    let mut i = e.len();
    while i > 0 && s == 0.0 {
        i -= 1;
        s = sgn(e[i]);
    }
    s
}

/// The sign of `|p − c|² − r²`, exactly.
pub fn disc(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> f64 {
    let dx = px - cx;
    let dy = py - cy;
    let v = dx * dx + dy * dy - r * r;
    if v.abs() > FILTER * (dx * dx + dy * dy + r * r) {
        sgn(v)
    } else {
        let ex = exp_diff(px, cx);
        let ey = exp_diff(py, cy);
        let er = exp_of(r);
        let s = exp_mul(&ex, &ex);
        let s = exp_add(&s, &exp_mul(&ey, &ey));
        let s = exp_add(&s, &exp_neg(&exp_mul(&er, &er)));
        exp_sign(&s)
    }
}

/// The sign of `orient(a, b, c)² − l² |b − a|²`, exactly.
pub fn line_gap(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, l: f64) -> f64 {
    let ux = bx - ax;
    let uy = by - ay;
    let vx = cx - ax;
    let vy = cy - ay;
    let o = ux * vy - uy * vx;
    let n = ux * ux + uy * uy;
    let v = o * o - l * l * n;
    let m = (ux * vy).abs() + (uy * vx).abs();
    if v.abs() > FILTER * (m * m + l * l * n) {
        sgn(v)
    } else {
        let eux = exp_diff(bx, ax);
        let euy = exp_diff(by, ay);
        let evx = exp_diff(cx, ax);
        let evy = exp_diff(cy, ay);
        let o = exp_mul(&eux, &evy);
        let o = exp_add(&o, &exp_neg(&exp_mul(&euy, &evx)));
        let n = exp_mul(&eux, &eux);
        let n = exp_add(&n, &exp_mul(&euy, &euy));
        let el = exp_of(l);
        let s = exp_mul(&o, &o);
        let s = exp_add(&s, &exp_neg(&exp_mul(&exp_mul(&el, &el), &n)));
        exp_sign(&s)
    }
}
