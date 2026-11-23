//! Where a section's offset folds. The section lies in its xy, x the distance from an axis along
//! y (a `Spun` wall's), and moves `d` along its normal (y', -x') / |C'|. Moved, it folds where it
//! turns back on itself (1 + d k <= 0, k = (x'y'' - y'x'') / |C'|^3 its signed curvature) or where
//! the circle a point of it turns on about the axis closes up (x + d y' / |C'| <= 0): the two
//! principal curvatures of the wall it turns into. Decided from Bernstein forms of the two sides
//! of each, squared to lose the root of |C'|, with a bound on each coefficient's rounding; not
//! sampled.

use super::{H, Spline, WORK, bern_mul};
use forge_math::Interval;

/// A rounding bound per unit of a coefficient's terms' size: products of degree up to 46 chained
/// six deep, and halvings 36 deep, each a few dozen roundings of eps = 1.1e-16, are well inside it.
const ROUNDING: f64 = 1e-12;

/// How finely a stretch is halved before what is left of it is called undecided: 2^-36 of a piece.
const FINEST: f64 = 1.0 / (1u64 << 36) as f64;

/// A Bernstein polynomial over [0, 1], and with each coefficient the size of the terms summed to
/// make it, which bounds its rounding.
#[derive(Clone)]
struct Poly {
    c: Vec<f64>,
    m: Vec<f64>,
}

impl Poly {
    fn exact(c: Vec<f64>) -> Poly {
        let m = c.iter().map(|x| x.abs()).collect();
        Poly { c, m }
    }
    fn degree(&self) -> usize {
        self.c.len() - 1
    }
    /// The derivative in its own parameter (one degree less; a constant's is 0).
    fn hodograph(&self) -> Poly {
        let n = self.degree();
        if n == 0 {
            return Poly {
                c: vec![0.0],
                m: vec![0.0],
            };
        }
        let k = n as f64;
        Poly {
            c: self.c.windows(2).map(|w| k * (w[1] - w[0])).collect(),
            m: self.m.windows(2).map(|w| k * (w[1] + w[0])).collect(),
        }
    }
    fn mul(&self, o: &Poly) -> Poly {
        let n = self.c.len() + o.c.len() - 1;
        let (mut c, mut m) = (vec![0.0; n], vec![0.0; n]);
        bern_mul(&self.c, &o.c, &mut c);
        bern_mul(&self.m, &o.m, &mut m);
        Poly { c, m }
    }
    /// Raised to degree `n` (by the product with 1 of the degree it lacks).
    fn elevated(&self, n: usize) -> Poly {
        if self.degree() >= n {
            return self.clone();
        }
        self.mul(&Poly::exact(vec![1.0; n - self.degree() + 1]))
    }
    fn add(&self, o: &Poly, k: f64) -> Poly {
        let n = self.degree().max(o.degree());
        let (a, b) = (self.elevated(n), o.elevated(n));
        Poly {
            c: a.c.iter().zip(&b.c).map(|(x, y)| x + k * y).collect(),
            m: a.m.iter().zip(&b.m).map(|(x, y)| x + k.abs() * y).collect(),
        }
    }
    fn scaled(&self, k: f64) -> Poly {
        Poly {
            c: self.c.iter().map(|x| x * k).collect(),
            m: self.m.iter().map(|x| x * k.abs()).collect(),
        }
    }
    /// Divided by s (`end` 0) or by 1 - s (`end` 1), its coefficient there being 0: the root a
    /// pole puts there taken out (b_j n / (j + 1) of b_(j+1), or b_j n / (n - j)).
    fn root_out(&self, end: usize) -> Poly {
        let n = self.degree();
        let k = n as f64;
        let take = |v: &[f64]| -> Vec<f64> {
            (0..n)
                .map(|j| {
                    if end == 0 {
                        v[j + 1] * k / (j + 1) as f64
                    } else {
                        v[j] * k / (n - j) as f64
                    }
                })
                .collect()
        };
        Poly {
            c: take(&self.c),
            m: take(&self.m),
        }
    }
    /// The two halves (de Casteljau, convex combinations: the sizes halve alike and still bound).
    fn halves(&self) -> (Poly, Poly) {
        fn split(c: &[f64]) -> (Vec<f64>, Vec<f64>) {
            let n = c.len();
            let mut w = c.to_vec();
            let (mut lo, mut hi) = (Vec::with_capacity(n), vec![0.0; n]);
            for r in 0..n {
                lo.push(w[0]);
                hi[n - 1 - r] = w[n - 1 - r];
                for j in 0..n - 1 - r {
                    w[j] = 0.5 * (w[j] + w[j + 1]);
                }
            }
            (lo, hi)
        }
        let ((c0, c1), (m0, m1)) = (split(&self.c), split(&self.m));
        (Poly { c: c0, m: m0 }, Poly { c: c1, m: m1 })
    }
    /// Negative over the stretch: every coefficient below zero by more than its rounding.
    fn below(&self) -> bool {
        self.c.iter().zip(&self.m).all(|(c, m)| c + ROUNDING * m < 0.0)
    }
    /// Positive over the stretch: every coefficient above zero by more than its rounding.
    fn above(&self) -> bool {
        self.c.iter().zip(&self.m).all(|(c, m)| c - ROUNDING * m > 0.0)
    }
}

/// The forms over one stretch whose signs say the offset does not fold there: in the section,
/// R < 0 or D = |P|^6 - R^2 > 0, R = -d det[A, A', A''] W^3 (1 + d k > 0, as |P|^3 > R); about
/// the axis, X > 0 and R2 < 0 or D2 = X^2 |P|^2 - R2^2 > 0, R2 = -d P_y W (x + d y' / |C'| > 0,
/// as X |P| > R2), where A = (X, Y, W) and P = A'W - AW' (W = 1, P = A', with no weights).
struct Forms([Poly; 5]);

impl Forms {
    fn new(w: &[H], rational: bool, d: f64) -> Forms {
        let col = |k: usize| Poly::exact(w.iter().map(|q| q[k]).collect());
        let x0 = col(0);
        // P and det[A, A', A''] do not change when the stretch moves (by multiples of W): taken
        // from its first point, their terms are the size of the stretch, not of where it is, and
        // so is their rounding (a difference of two floats rounds to its own size)
        let o = (w[0][0] / w[0][3], w[0][1] / w[0][3]);
        let moved = |k: usize, c: f64| Poly::exact(w.iter().map(|q| q[k] - c * q[3]).collect());
        let (x, y) = (moved(0, o.0), moved(1, o.1));
        let (x1, y1) = (x.hodograph(), y.hodograph());
        let (x2, y2) = (x1.hodograph(), y1.hodograph());
        let (px, py, r, weight) = if rational {
            let wt = col(3);
            let w1 = wt.hodograph();
            let w2 = w1.hodograph();
            let px = x1.mul(&wt).add(&x.mul(&w1), -1.0);
            let py = y1.mul(&wt).add(&y.mul(&w1), -1.0);
            let a = wt.mul(&x1.mul(&y2).add(&y1.mul(&x2), -1.0));
            let b = w1.mul(&x.mul(&y2).add(&y.mul(&x2), -1.0));
            let c = w2.mul(&x.mul(&y1).add(&y.mul(&x1), -1.0));
            let det = a.add(&b, -1.0).add(&c, 1.0);
            let w3 = wt.mul(&wt).mul(&wt);
            (px, py, det.mul(&w3).scaled(-d), Some(wt))
        } else {
            let det = x1.mul(&y2).add(&y1.mul(&x2), -1.0);
            (x1, y1, det.scaled(-d), None)
        };
        // where the section meets the axis square to it (a pole: x = 0 and y' = 0 there, both
        // exactly), the circles' test is 0 there over 0: both sides lose their common root, and
        // what is left is its limit (the section's own bend at the pole, as on a sphere)
        let (mut x0, mut py_axis) = (x0, py.clone());
        for end in [0, 1] {
            let at = |p: &Poly| if end == 0 { p.c[0] } else { p.c[p.degree()] };
            if x0.degree() > 0 && py_axis.degree() > 0 && at(&x0) == 0.0 && at(&py_axis) == 0.0 {
                x0 = x0.root_out(end);
                py_axis = py_axis.root_out(end);
            }
        }
        let b = px.mul(&px).add(&py.mul(&py), 1.0);
        let dd = b.mul(&b).mul(&b).add(&r.mul(&r), -1.0);
        let r2 = match &weight {
            Some(wt) => py_axis.mul(wt).scaled(-d),
            None => py_axis.scaled(-d),
        };
        let d2 = x0.mul(&x0).mul(&b).add(&r2.mul(&r2), -1.0);
        Forms([r, dd, x0, r2, d2])
    }
    fn settled(&self) -> bool {
        let [r, dd, x, r2, d2] = &self.0;
        (r.below() || dd.above()) && x.above() && (r2.below() || d2.above())
    }
    fn halves(&self) -> (Forms, Forms) {
        let hs = self.0.clone().map(|p| p.halves());
        (Forms(hs.clone().map(|h| h.0)), Forms(hs.map(|h| h.1)))
    }
}

impl Spline {
    /// Where the section moved `d` folds over `range` (the module's two tests): None where it
    /// provably folds nowhere; Some(t) where it folds at t, or where that cannot be decided (a point
    /// on the axis, a touch within rounding, a degree past the products' reach). Each piece's
    /// stretch is halved where the forms' signs do not settle it, and a halving point where the
    /// section folds is returned.
    pub fn offset_fold(&self, d: f64, range: Interval) -> Option<f64> {
        let folds_at = |t: f64| {
            let q = self.deriv(t);
            let (x1, y1) = (q.d1.x, q.d1.y);
            let l = (x1 * x1 + y1 * y1).sqrt();
            let k = (x1 * q.d2.y - y1 * q.d2.x) / (l * l * l);
            !(1.0 + d * k > 0.0 && q.p.x + d * y1 / l > 0.0)
        };
        let p = self.degree;
        // the highest degree the products reach: (d det W^3)^2, or |P|^6
        let top = if self.rational { 12 * p - 6 } else { 6 * p - 6 };
        if top >= WORK {
            return Some(range.mid());
        }
        for (i, s0, s1) in self.over(range) {
            if s1 <= s0 {
                continue;
            }
            let (a0, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            let w = self.part(i, s0, s1);
            let mut todo = vec![(0.0, 1.0, Forms::new(&w[..=p], self.rational, d))];
            while let Some((a, b, f)) = todo.pop() {
                if f.settled() {
                    continue;
                }
                let m = 0.5 * (a + b);
                let t = a0 + (s0 + (s1 - s0) * m) * h;
                if folds_at(t) || b - a <= FINEST {
                    return Some(t);
                }
                let (lo, hi) = f.halves();
                todo.push((m, b, hi));
                todo.push((a, m, lo));
            }
        }
        None
    }
}
