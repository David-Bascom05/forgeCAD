//! Exact signs for region.rs's three squared comparisons (verify/sheet's Region.lean: `near`,
//! `within`, `inDisc`): a float filter where its error bound proves the sign, else the value
//! recomputed exactly with floating-point expansions (Shewchuk 1997: two-sum, Dekker's
//! two-product, grow-expansion). Every input is a double, so a dyadic rational, and every
//! expansion here is exact; the sign of a nonoverlapping expansion is its largest component's.
//!
//! The filters: each value is a sum of products of rounded differences of doubles, `v`, with an
//! absolute error at most `FILTER · m`, `m` the sum of the products' magnitudes (each difference
//! rounds once, relatively, each product and sum once more: well under 8 ε of `m`; 16 ε is
//! taken). Where `|v| > FILTER · m` the float sign is the exact one; else the exact path decides.
//! Like predicates.rs, this assumes no product underflows inexactly (inputs within its windows).

const EPS: f64 = f64::EPSILON * 0.5;
/// The filters' relative bound.
const FILTER: f64 = 16.0 * EPS;
const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let err = x - ah * bh - al * bh - ah * bl;
    (x, al * bl - err)
}

/// An exact value: a sum of doubles, nonoverlapping, increasing in magnitude (zeros dropped).
#[derive(Clone, Debug, Default)]
struct Exp(Vec<f64>);

impl Exp {
    fn of(x: f64) -> Exp {
        Exp(if x == 0.0 { vec![] } else { vec![x] })
    }
    /// `a - b` exactly.
    fn diff(a: f64, b: f64) -> Exp {
        let mut e = Exp::of(a);
        e.grow(-b);
        e
    }
    /// Adds `b` exactly (grow-expansion).
    fn grow(&mut self, b: f64) {
        let mut q = b;
        let mut out = Vec::with_capacity(self.0.len() + 1);
        for &e in &self.0 {
            let (s, lo) = two_sum(q, e);
            if lo != 0.0 {
                out.push(lo);
            }
            q = s;
        }
        if q != 0.0 {
            out.push(q);
        }
        self.0 = out;
    }
    fn add(&mut self, o: &Exp) {
        for &x in &o.0 {
            self.grow(x);
        }
    }
    fn neg(&self) -> Exp {
        Exp(self.0.iter().map(|x| -x).collect())
    }
    /// The product exactly: every pair's two-product, grown in.
    fn mul(&self, o: &Exp) -> Exp {
        let mut out = Exp::default();
        for &a in &self.0 {
            for &b in &o.0 {
                let (x, y) = two_product(a, b);
                out.grow(y);
                out.grow(x);
            }
        }
        out
    }
    /// The sign: the largest component's.
    fn sign(&self) -> f64 {
        self.0
            .iter()
            .rev()
            .find(|x| **x != 0.0)
            .map_or(0.0, |x| x.signum())
    }
}

/// The sign of |p − c|² − r², exactly.
pub fn disc(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> f64 {
    let (dx, dy) = (px - cx, py - cy);
    let v = dx * dx + dy * dy - r * r;
    if v.abs() > FILTER * (dx * dx + dy * dy + r * r) {
        return v.signum();
    }
    let (ex, ey, er) = (Exp::diff(px, cx), Exp::diff(py, cy), Exp::of(r));
    let mut s = ex.mul(&ex);
    s.add(&ey.mul(&ey));
    s.add(&er.mul(&er).neg());
    s.sign()
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
    let (eux, euy, evx, evy) = (
        Exp::diff(bx, ax),
        Exp::diff(by, ay),
        Exp::diff(cx, ax),
        Exp::diff(cy, ay),
    );
    let mut o = eux.mul(&evy);
    o.add(&euy.mul(&evx).neg());
    let mut n = eux.mul(&eux);
    n.add(&euy.mul(&euy));
    let el = Exp::of(l);
    let mut s = o.mul(&o);
    s.add(&el.mul(&el).mul(&n).neg());
    s.sign()
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
