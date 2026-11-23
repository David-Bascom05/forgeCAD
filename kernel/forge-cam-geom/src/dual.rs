//! Interval forward-mode derivatives: a value enclosure with enclosures of its partial
//! derivatives in three parameters, over a box. With them the mean-value form
//! f(X) ⊆ f(c) + Σ ∂f(X)·(X − c) encloses a function to second order near a minimum, which is
//! what a tangent contact needs.
use crate::ival::Ival;

/// The arithmetic the parametrisations are written in, once, for both types.
pub trait Num: Copy {
    fn c(x: f64) -> Self;
    fn add(self, o: Self) -> Self;
    fn sub(self, o: Self) -> Self;
    fn mul(self, o: Self) -> Self;
    fn div(self, o: Self) -> Self;
    fn sqr(self) -> Self;
}

impl Num for Ival {
    #[inline]
    fn c(x: f64) -> Ival {
        Ival::pt(x)
    }
    #[inline]
    fn add(self, o: Ival) -> Ival {
        Ival::add(self, o)
    }
    #[inline]
    fn sub(self, o: Ival) -> Ival {
        Ival::sub(self, o)
    }
    #[inline]
    fn mul(self, o: Ival) -> Ival {
        Ival::mul(self, o)
    }
    #[inline]
    fn div(self, o: Ival) -> Ival {
        Ival::div(self, o)
    }
    #[inline]
    fn sqr(self) -> Ival {
        Ival::sqr(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct D {
    pub v: Ival,
    pub g: [Ival; 3],
}

const Z: Ival = Ival { lo: 0.0, hi: 0.0 };

impl D {
    /// The k-th parameter over the box x.
    pub fn var(x: Ival, k: usize) -> D {
        let mut g = [Z; 3];
        g[k] = Ival::pt(1.0);
        D { v: x, g }
    }
}

impl Num for D {
    #[inline]
    fn c(x: f64) -> D {
        D {
            v: Ival::pt(x),
            g: [Z; 3],
        }
    }
    #[inline]
    fn add(self, o: D) -> D {
        D {
            v: self.v.add(o.v),
            g: [
                self.g[0].add(o.g[0]),
                self.g[1].add(o.g[1]),
                self.g[2].add(o.g[2]),
            ],
        }
    }
    #[inline]
    fn sub(self, o: D) -> D {
        D {
            v: self.v.sub(o.v),
            g: [
                self.g[0].sub(o.g[0]),
                self.g[1].sub(o.g[1]),
                self.g[2].sub(o.g[2]),
            ],
        }
    }
    #[inline]
    fn mul(self, o: D) -> D {
        let mut g = [Z; 3];
        let mut k = 0;
        while k < 3 {
            g[k] = self.g[k].mul(o.v).add(self.v.mul(o.g[k]));
            k += 1;
        }
        D {
            v: self.v.mul(o.v),
            g,
        }
    }
    #[inline]
    fn div(self, o: D) -> D {
        // (a/b)' = (a' − (a/b) b') / b
        let q = self.v.div(o.v);
        let mut g = [Z; 3];
        let mut k = 0;
        while k < 3 {
            g[k] = self.g[k].sub(q.mul(o.g[k])).div(o.v);
            k += 1;
        }
        D { v: q, g }
    }
    #[inline]
    fn sqr(self) -> D {
        let two = self.v.scale(2.0);
        D {
            v: self.v.sqr(),
            g: [two.mul(self.g[0]), two.mul(self.g[1]), two.mul(self.g[2])],
        }
    }
}

/// The mean-value enclosure over the box `x` of a function whose value at the box's centre is
/// `at_c` and whose derivative enclosure over the box is `d.g`, intersected with the natural
/// enclosure `d.v`.
pub fn mean_value(d: &D, at_c: Ival, x: [Ival; 3]) -> Ival {
    let mut acc = at_c;
    let mut k = 0;
    while k < 3 {
        let c = x[k].mid();
        let dx = x[k].sub(Ival::pt(c));
        acc = acc.add(d.g[k].mul(dx));
        k += 1;
    }
    let lo = if acc.lo > d.v.lo { acc.lo } else { d.v.lo };
    let hi = if acc.hi < d.v.hi { acc.hi } else { d.v.hi };
    if lo <= hi { Ival { lo, hi } } else { d.v }
}
