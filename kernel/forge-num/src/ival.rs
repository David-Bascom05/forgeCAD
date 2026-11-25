//! f64 intervals with outward rounding: every operation's true result lies in the interval it
//! returns. IEEE operations are correctly rounded, so one ulp outward on each side suffices.
//!
//! This is the filter in front of the exact sign, and the arithmetic of the clearance checker.
//! An empty or undefined result is the whole line ([-inf, inf]): never a narrower wrong answer.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ival {
    pub lo: f64,
    pub hi: f64,
}

/// Whether the build takes the strict rules (feature `ival-strict`: no exact-zero rule), for the
/// digests pinned per rule.
pub const STRICT: bool = cfg!(feature = "ival-strict");

pub const WHOLE: Ival = Ival {
    lo: f64::NEG_INFINITY,
    hi: f64::INFINITY,
};

#[inline]
fn down(x: f64) -> f64 {
    if x.is_nan() {
        f64::NEG_INFINITY
    } else {
        x.next_down()
    }
}
#[inline]
fn up(x: f64) -> f64 {
    if x.is_nan() { f64::INFINITY } else { x.next_up() }
}

#[inline]
fn min4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    if b < m {
        m = b;
    }
    if c < m {
        m = c;
    }
    if d < m {
        m = d;
    }
    m
}
#[inline]
fn max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    if b > m {
        m = b;
    }
    if c > m {
        m = c;
    }
    if d > m {
        m = d;
    }
    m
}

impl Ival {
    /// An exact point (any f64 is an exact real).
    #[inline]
    pub fn pt(x: f64) -> Ival {
        Ival { lo: x, hi: x }
    }
    /// [a, b] for a <= b, exactly.
    #[inline]
    pub fn new(a: f64, b: f64) -> Ival {
        if a <= b {
            Ival { lo: a, hi: b }
        } else {
            Ival { lo: b, hi: a }
        }
    }
    fn fix(lo: f64, hi: f64) -> Ival {
        if lo.is_nan() || hi.is_nan() {
            return WHOLE;
        }
        Ival { lo, hi }
    }
    /// The sum, outward. `down`/`up` alone give every case the branches on infinities once did:
    /// −∞ + x and −∞ + ∞ (NaN) round down to −∞, +∞ + x and NaN up to +∞, and neither returns
    /// NaN, so no fix is needed (`tests::add_and_mul_are_the_reference_bit_for_bit`).
    #[inline]
    pub fn add(self, o: Ival) -> Ival {
        // x + 0 = x exactly: an exact zero adds nothing, so the other operand is the sum (an
        // ordered interval: NaN or inverted ends take the general rule). Sound, and tighter than
        // rounding outward; it keeps a derivative that is exactly zero exactly zero
        // (verify/forge-cam ForgeCam/IvalZero.lean `add_zero_encl`). Feature `ival-strict`: the
        // rule as it was, for a train's comparison.
        #[cfg(not(feature = "ival-strict"))]
        {
            if o.lo == 0.0 && o.hi == 0.0 && self.lo <= self.hi {
                return self;
            }
            if self.lo == 0.0 && self.hi == 0.0 && o.lo <= o.hi {
                return o;
            }
        }
        self.add_strict(o)
    }
    /// The sum, outward, with no exact-zero rule.
    #[inline]
    pub fn add_strict(self, o: Ival) -> Ival {
        Ival {
            lo: down(self.lo + o.lo),
            hi: up(self.hi + o.hi),
        }
    }
    #[inline]
    pub fn neg(self) -> Ival {
        Ival {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    #[inline]
    pub fn sub(self, o: Ival) -> Ival {
        self.add(o.neg())
    }
    /// The product, outward. With every end finite and each interval ordered (lo <= hi) no product
    /// is NaN, and rounding is monotone, so the operands' signs pick which of the four end products are the least and the greatest
    /// (two products, not four and their comparisons); any other case is the general rule. Every
    /// result is the general rule's, bit for bit (`tests::add_and_mul_are_the_reference_bit_for_bit`).
    #[inline]
    pub fn mul(self, o: Ival) -> Ival {
        // x · 0 = 0 exactly for every real x: an exact zero times a finite, ordered interval is
        // exactly [0, 0] (an unbounded or undefined operand takes the general rule, so WHOLE is
        // never hidden). Sound, and tighter than rounding outward
        // (verify/forge-cam ForgeCam/IvalZero.lean `mul_zero_encl`). Feature `ival-strict`: the
        // rule as it was, for a train's comparison.
        #[cfg(not(feature = "ival-strict"))]
        {
            let finite = |x: Ival| x.lo.is_finite() && x.hi.is_finite() && x.lo <= x.hi;
            if (self.lo == 0.0 && self.hi == 0.0 && finite(o)) || (o.lo == 0.0 && o.hi == 0.0 && finite(self))
            {
                return Ival { lo: 0.0, hi: 0.0 };
            }
        }
        self.mul_strict(o)
    }
    /// The product, outward, with no exact-zero rule.
    #[inline]
    pub fn mul_strict(self, o: Ival) -> Ival {
        let (a0, a1, b0, b1) = (self.lo, self.hi, o.lo, o.hi);
        if a0.is_finite() && a1.is_finite() && b0.is_finite() && b1.is_finite() && a0 <= a1 && b0 <= b1 {
            if a0 == a1 && b0 == b1 {
                // exact when the product is exact; else one ulp each way
                let p = a0 * b0;
                return Ival {
                    lo: down(p),
                    hi: up(p),
                };
            }
            let (lo, hi) = if a0 >= 0.0 {
                if b0 >= 0.0 {
                    (a0 * b0, a1 * b1)
                } else if b1 <= 0.0 {
                    (a1 * b0, a0 * b1)
                } else {
                    (a1 * b0, a1 * b1)
                }
            } else if a1 <= 0.0 {
                if b0 >= 0.0 {
                    (a0 * b1, a1 * b0)
                } else if b1 <= 0.0 {
                    (a1 * b1, a0 * b0)
                } else {
                    (a0 * b1, a0 * b0)
                }
            } else if b0 >= 0.0 {
                (a0 * b1, a1 * b1)
            } else if b1 <= 0.0 {
                (a1 * b0, a0 * b0)
            } else {
                let (l1, l2, h1, h2) = (a0 * b1, a1 * b0, a0 * b0, a1 * b1);
                (if l2 < l1 { l2 } else { l1 }, if h2 > h1 { h2 } else { h1 })
            };
            return Ival {
                lo: down(lo),
                hi: up(hi),
            };
        }
        if a0 == a1 && b0 == b1 {
            let p = a0 * b0;
            if p.is_nan() {
                return WHOLE;
            }
            return Ival::fix(down(p), up(p));
        }
        let (a, b, c, d) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
            return WHOLE;
        }
        Ival::fix(down(min4(a, b, c, d)), up(max4(a, b, c, d)))
    }
    #[inline]
    pub fn scale(self, k: f64) -> Ival {
        self.mul(Ival::pt(k))
    }
    /// The square, which is never negative.
    #[inline]
    pub fn sqr(self) -> Ival {
        let (a, b) = (self.lo.abs(), self.hi.abs());
        let (mut m, mut x) = if a < b { (a, b) } else { (b, a) };
        if self.lo <= 0.0 && self.hi >= 0.0 {
            m = 0.0;
        }
        if x.is_nan() {
            x = f64::INFINITY;
        }
        let lo = if m == 0.0 { 0.0 } else { down(m * m).max(0.0) };
        Ival::fix(lo, up(x * x))
    }
    #[inline]
    pub fn div(self, o: Ival) -> Ival {
        if o.lo <= 0.0 && o.hi >= 0.0 {
            return WHOLE;
        }
        let (a, b, c, d) = (self.lo / o.lo, self.lo / o.hi, self.hi / o.lo, self.hi / o.hi);
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
            return WHOLE;
        }
        Ival::fix(down(min4(a, b, c, d)), up(max4(a, b, c, d)))
    }
    /// The square root of the non-negative part (the argument is non-negative by construction
    /// wherever this is called on an exact value).
    #[inline]
    pub fn sqrt(self) -> Ival {
        let lo = if self.lo <= 0.0 {
            0.0
        } else {
            down(self.lo.sqrt()).max(0.0)
        };
        let hi = if self.hi < 0.0 { 0.0 } else { up(self.hi.sqrt()) };
        Ival::fix(lo, hi)
    }
    #[inline]
    pub fn min(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo < o.lo { self.lo } else { o.lo },
            hi: if self.hi < o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn max(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo > o.lo { self.lo } else { o.lo },
            hi: if self.hi > o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn abs(self) -> Ival {
        if self.lo >= 0.0 {
            self
        } else if self.hi <= 0.0 {
            self.neg()
        } else {
            Ival {
                lo: 0.0,
                hi: if -self.lo > self.hi { -self.lo } else { self.hi },
            }
        }
    }
    #[inline]
    pub fn hull(self, o: Ival) -> Ival {
        Ival {
            lo: if self.lo < o.lo { self.lo } else { o.lo },
            hi: if self.hi > o.hi { self.hi } else { o.hi },
        }
    }
    #[inline]
    pub fn mid(self) -> f64 {
        if self.lo == f64::NEG_INFINITY || self.hi == f64::INFINITY {
            return 0.0;
        }
        0.5 * self.lo + 0.5 * self.hi
    }
    #[inline]
    pub fn width(self) -> f64 {
        if self.hi == self.lo {
            return 0.0;
        }
        up(self.hi - self.lo)
    }
    /// -1, 0 (undecided: zero inside) or 1.
    #[inline]
    pub fn sign(self) -> i32 {
        if self.lo > 0.0 {
            1
        } else if self.hi < 0.0 {
            -1
        } else {
            0
        }
    }
    #[inline]
    pub fn split(self) -> (Ival, Ival) {
        let m = self.mid();
        (Ival { lo: self.lo, hi: m }, Ival { lo: m, hi: self.hi })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules as they were before the fast paths (verbatim): the references.
    fn add_ref(x: Ival, o: Ival) -> Ival {
        let lo = if x.lo == f64::NEG_INFINITY || o.lo == f64::NEG_INFINITY {
            f64::NEG_INFINITY
        } else {
            down(x.lo + o.lo)
        };
        let hi = if x.hi == f64::INFINITY || o.hi == f64::INFINITY {
            f64::INFINITY
        } else {
            up(x.hi + o.hi)
        };
        Ival::fix(lo, hi)
    }
    fn mul_ref(x: Ival, o: Ival) -> Ival {
        if x.lo == x.hi && o.lo == o.hi {
            let p = x.lo * o.lo;
            if p.is_nan() {
                return WHOLE;
            }
            return Ival::fix(down(p), up(p));
        }
        let (a, b, c, d) = (x.lo * o.lo, x.lo * o.hi, x.hi * o.lo, x.hi * o.hi);
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
            return WHOLE;
        }
        Ival::fix(down(min4(a, b, c, d)), up(max4(a, b, c, d)))
    }
    fn bits(x: Ival) -> (u64, u64) {
        (x.lo.to_bits(), x.hi.to_bits())
    }
    fn same(op: &str, x: Ival, y: Ival) {
        // the strict rules are the references bit for bit
        let (got, want) = match op {
            "add" => (x.add_strict(y), add_ref(x, y)),
            _ => (x.mul_strict(y), mul_ref(x, y)),
        };
        assert_eq!(bits(got), bits(want), "{op} {x:?} {y:?}: {got:?} vs {want:?}");
        // the rules in use: the strict ones, or (an exact zero) a result inside them and exact
        let used = match op {
            "add" => x.add(y),
            _ => x.mul(y),
        };
        if bits(used) != bits(got) {
            let zero = |v: Ival| v.lo == 0.0 && v.hi == 0.0;
            assert!(zero(x) || zero(y), "{op} {x:?} {y:?}: {used:?} vs {got:?}");
            assert!(
                got.lo <= used.lo && used.hi <= got.hi,
                "{op} {x:?} {y:?}: {used:?} not in {got:?}"
            );
            match op {
                "add" => assert!(used == if zero(y) { x } else { y }),
                _ => assert!(zero(used)),
            }
        }
    }

    /// Every pair of intervals over the special values (both zeros, the denormal limits, ±1,
    /// halves, the largest finite, the infinities, NaN, small and large magnitudes; each end any
    /// of them, inverted ones included): the fast add and mul are the references bit for bit.
    #[test]
    fn add_and_mul_are_the_reference_bit_for_bit() {
        let base = [
            0.0,
            f64::MIN_POSITIVE,
            5e-324,
            1e-300,
            0.1,
            0.5,
            1.0,
            1.5,
            2.0,
            3.0,
            1e10,
            1e300,
            f64::MAX,
            f64::INFINITY,
        ];
        let mut vals: Vec<f64> = Vec::new();
        for &v in &base {
            vals.push(v);
            vals.push(-v);
        }
        vals.push(f64::NAN);
        let ivs: Vec<Ival> = vals
            .iter()
            .flat_map(|&lo| vals.iter().map(move |&hi| Ival { lo, hi }))
            .collect();
        for &x in &ivs {
            for &y in &ivs {
                same("add", x, y);
                same("mul", x, y);
            }
        }
        // random intervals, ends from random bit patterns and from ordinary ranges, sorted and not
        let mut s = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let val = |k: u64, r: u64| match k % 4 {
            0 => f64::from_bits(r),
            1 => (r % 2001) as f64 / 16.0 - 62.5,
            2 => ((r % 1_000_003) as f64 - 500_000.0) * 1e-7,
            _ => vals[(r % vals.len() as u64) as usize],
        };
        for _ in 0..2_000_000 {
            let (k, r1, r2, r3, r4) = (next(), next(), next(), next(), next());
            let (a, b) = (val(k, r1), val(k >> 2, r2));
            let (c, d) = (val(k >> 4, r3), val(k >> 6, r4));
            let x = if k & (1 << 10) == 0 {
                Ival::new(a, b)
            } else {
                Ival { lo: a, hi: b }
            };
            let y = if k & (1 << 11) == 0 {
                Ival::new(c, d)
            } else {
                Ival { lo: c, hi: d }
            };
            same("add", x, y);
            same("mul", x, y);
        }
    }

    #[test]
    fn encloses() {
        let a = Ival::pt(0.1);
        let b = Ival::pt(0.2);
        let s = a.add(b);
        assert!(s.lo <= 0.30000000000000004 && s.hi >= 0.3);
        let q = Ival::pt(2.0).sqrt();
        assert!(q.lo * q.lo <= 2.0 && q.hi * q.hi >= 2.0);
        assert_eq!(Ival::new(-1.0, 2.0).sqr().lo, 0.0);
        assert_eq!(Ival::pt(1.0).div(Ival::new(-1.0, 1.0)), WHOLE);
    }
}
