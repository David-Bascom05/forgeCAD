//! Exact rationals, rational polynomials and real algebraic numbers over [`Big`]: the Rust side
//! of verify/forge-exact's `ForgeExact.Alg` (Poly.lean, Model.lean, Real.lean) and the parts of
//! `ForgeExact.QRef.Decide` the curved decide-once bridge calls (`realRoots`, `between`,
//! `simplest`, `rootBound`).
//!
//! **One to one with the Lean.** Every function here is the Lean definition of the same name
//! (snake-cased), written so that its result is the Lean's on every input, list lengths included:
//! `padd` keeps the longer list's tail, `pmul` is the Lean recursion unfolded, `divmod` and `xgcd`
//! are the same loops. A rational is kept normalised (denominator > 0, coprime), as Lean's `Rat`
//! is, so equal values are equal structurally, and division by zero is 0, as in Lean.
//!
//! **What is proven, and where.** The checks (`iso_ok`, `Num::sign`, `Num::cmp_rat`, `Num::cmp`,
//! `eqv`) are the ones `sign_spec`, `cmpRat_spec`, `cmp_spec` and `eqv_spec` are about: spent fuel
//! answers `None`, never a wrong sign. The proposers (`divmod`, `xgcd`, `gcd`, `sqfree`,
//! `real_roots`) are untrusted, as in the Lean: whatever they return is checked before a decision
//! rests on it. L4 for this module is **interim** (design D6): a differential test against the
//! Lean L2; the blocker is Aeneas and bignum arithmetic.
//!
//! **Style** (research/kernel-audit/decide-once/aeneas/README.md): no struct holds a borrow, no
//! `break`, no `return` inside a loop, no `Ordering` (comparisons answer −1, 0 or 1), loops are
//! `while` loops over indices. Only [`Big`]'s public API is used.

use super::{Big, Dy};

// ---------------------------------------------------------------------------------------------
// Natural-number helpers on Big (public API only: add, sub, mul, shl, bits, signum, abs)
// ---------------------------------------------------------------------------------------------

/// `a` against `b`: −1, 0 or 1.
pub fn bcmp(a: &Big, b: &Big) -> i32 {
    a.sub(b).signum()
}

fn big1() -> Big {
    Big::from_i64(1)
}

/// A float near `a / b` (both nonzero), from their top bits.
fn ratio_approx(a: &Big, b: &Big) -> f64 {
    super::ratio_f64(&Dy { m: a.clone(), e: 0 }, &Dy { m: b.clone(), e: 0 })
}

/// Quotients below this are estimated in one float division.
const EST_LIMIT: f64 = 4503599627370496.0; // 2^52

/// `(q, r)` with `a = q b + r` and `0 ≤ r < b`, for `a ≥ 0` and `b > 0`. The quotient is
/// estimated by a float division of the top bits and corrected exactly; a quotient past 2⁵² is
/// taken 50 bits at a time against `b` shifted. The result is exact whatever the estimate.
pub fn divmod_nat(a: &Big, b: &Big) -> (Big, Big) {
    if bcmp(a, b) < 0 {
        (Big::zero(), a.clone())
    } else {
        let est = ratio_approx(a, b);
        if est.is_finite() && est < EST_LIMIT {
            let mut q = Big::from_i64(est.floor() as i64);
            let mut r = a.sub(&q.mul(b));
            while r.signum() < 0 {
                q = q.sub(&big1());
                r = r.add(b);
            }
            while bcmp(&r, b) >= 0 {
                q = q.add(&big1());
                r = r.sub(b);
            }
            (q, r)
        } else {
            let k = a.bits() - b.bits() - 50;
            let t = b.shl(k);
            let (q1, r1) = divmod_nat(a, &t);
            let (q2, r2) = divmod_nat(&r1, b);
            (q1.shl(k).add(&q2), r2)
        }
    }
}

/// The gcd of `|a|` and `|b|` (Euclid).
pub fn gcd_big(a: &Big, b: &Big) -> Big {
    let mut x = a.abs();
    let mut y = b.abs();
    while !y.is_zero() {
        let r = divmod_nat(&x, &y).1;
        x = y;
        y = r;
    }
    x
}

/// `⌊√a⌋` for `a ≥ 0`, bit by bit.
pub fn isqrt_big(a: &Big) -> Big {
    let mut r = Big::zero();
    if a.signum() > 0 {
        let mut k: i64 = (a.bits() as i64) / 2;
        while k >= 0 {
            let c = r.add(&big1().shl(k as u32));
            if bcmp(&c.mul(&c), a) <= 0 {
                r = c;
            }
            k -= 1;
        }
    }
    r
}

// ---------------------------------------------------------------------------------------------
// Rationals (Lean's `Rat`)
// ---------------------------------------------------------------------------------------------

/// A rational `n / d`, normalised: `d > 0`, `gcd(n, d) = 1`, and 0 is `0 / 1`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Q {
    pub n: Big,
    pub d: Big,
}

impl Default for Q {
    fn default() -> Q {
        Q::zero()
    }
}

impl Q {
    pub fn zero() -> Q {
        Q {
            n: Big::zero(),
            d: big1(),
        }
    }

    pub fn one() -> Q {
        Q::from_i64(1)
    }

    pub fn from_i64(x: i64) -> Q {
        Q {
            n: Big::from_i64(x),
            d: big1(),
        }
    }

    pub fn from_big(n: Big) -> Q {
        Q { n, d: big1() }
    }

    /// `n / d`, normalised; `d = 0` gives 0, as Lean's division does.
    pub fn ratio(n: &Big, d: &Big) -> Q {
        if d.is_zero() || n.is_zero() {
            Q::zero()
        } else {
            let (n1, d1) = if d.signum() < 0 {
                (n.neg(), d.neg())
            } else {
                (n.clone(), d.clone())
            };
            if bcmp(&d1, &big1()) == 0 {
                Q { n: n1, d: d1 }
            } else {
                let g = gcd_big(&n1, &d1);
                if bcmp(&g, &big1()) == 0 {
                    Q { n: n1, d: d1 }
                } else {
                    let nn = if n1.signum() < 0 {
                        divmod_nat(&n1.abs(), &g).0.neg()
                    } else {
                        divmod_nat(&n1, &g).0
                    };
                    Q {
                        n: nn,
                        d: divmod_nat(&d1, &g).0,
                    }
                }
            }
        }
    }

    /// The exact value of a finite f64 (every f64 is dyadic); `None` for NaN and infinities.
    pub fn from_f64(x: f64) -> Option<Q> {
        if !x.is_finite() {
            None
        } else {
            let dy = Dy::from_f64(x);
            Some(Q::from_dy(&dy))
        }
    }

    /// The exact value of a dyadic `m · 2^e`.
    pub fn from_dy(x: &Dy) -> Q {
        if x.e >= 0 {
            Q::from_big(x.m.shl(x.e as u32))
        } else {
            Q::ratio(&x.m, &big1().shl((-x.e) as u32))
        }
    }

    pub fn is_zero(&self) -> bool {
        self.n.is_zero()
    }

    pub fn signum(&self) -> i32 {
        self.n.signum()
    }

    pub fn neg(&self) -> Q {
        Q {
            n: self.n.neg(),
            d: self.d.clone(),
        }
    }

    pub fn abs(&self) -> Q {
        Q {
            n: self.n.abs(),
            d: self.d.clone(),
        }
    }

    pub fn add(&self, o: &Q) -> Q {
        if self.is_zero() {
            o.clone()
        } else if o.is_zero() {
            self.clone()
        } else if bcmp(&self.d, &o.d) == 0 {
            Q::ratio(&self.n.add(&o.n), &self.d)
        } else {
            Q::ratio(&self.n.mul(&o.d).add(&o.n.mul(&self.d)), &self.d.mul(&o.d))
        }
    }

    pub fn sub(&self, o: &Q) -> Q {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &Q) -> Q {
        if self.is_zero() || o.is_zero() {
            Q::zero()
        } else {
            Q::ratio(&self.n.mul(&o.n), &self.d.mul(&o.d))
        }
    }

    /// `self / o`; 0 when `o = 0`, as in Lean.
    pub fn div(&self, o: &Q) -> Q {
        if o.is_zero() {
            Q::zero()
        } else {
            Q::ratio(&self.n.mul(&o.d), &self.d.mul(&o.n))
        }
    }

    /// `1 / self`; 0 for 0.
    pub fn inv(&self) -> Q {
        Q::ratio(&self.d, &self.n)
    }

    /// `self` against `o`: −1, 0 or 1.
    pub fn cmp_q(&self, o: &Q) -> i32 {
        if bcmp(&self.d, &o.d) == 0 {
            bcmp(&self.n, &o.n)
        } else {
            bcmp(&self.n.mul(&o.d), &o.n.mul(&self.d))
        }
    }

    pub fn lt(&self, o: &Q) -> bool {
        self.cmp_q(o) < 0
    }

    pub fn le(&self, o: &Q) -> bool {
        self.cmp_q(o) <= 0
    }

    /// `⌊self⌋` (toward −∞), Lean's `Rat.floor`.
    pub fn floor(&self) -> Big {
        if self.n.signum() >= 0 {
            divmod_nat(&self.n, &self.d).0
        } else {
            let (q, r) = divmod_nat(&self.n.abs(), &self.d);
            if r.is_zero() {
                q.neg()
            } else {
                q.neg().sub(&big1())
            }
        }
    }

    /// `(self + o) / 2`.
    pub fn mid(&self, o: &Q) -> Q {
        Q::ratio(&self.n.mul(&o.d).add(&o.n.mul(&self.d)), &self.d.mul(&o.d).shl(1))
    }

    /// An f64 near the value (a report or a filter's centre, never a decision).
    pub fn approx(&self) -> f64 {
        super::ratio_f64(
            &Dy {
                m: self.n.clone(),
                e: 0,
            },
            &Dy {
                m: self.d.clone(),
                e: 0,
            },
        )
    }

    /// The value against the finite float `f`: −1, 0 or 1, by `n` against `m 2^e d` (no gcd).
    pub fn cmp_f64(&self, f: f64) -> i32 {
        let dy = Dy::from_f64(f);
        if dy.e >= 0 {
            bcmp(&self.n, &dy.m.mul(&self.d).shl(dy.e as u32))
        } else {
            bcmp(&self.n.shl((-dy.e) as u32), &dy.m.mul(&self.d))
        }
    }

    /// A float `≤` the value, checked exactly (`-∞` if none is found).
    pub fn lo_f64(&self) -> f64 {
        let mut f = self.approx();
        let mut ok = false;
        let mut k = 0;
        while !ok && k < 4 {
            if !f.is_finite() {
                f = f64::NEG_INFINITY;
                ok = true;
            } else if self.cmp_f64(f) >= 0 {
                ok = true;
            } else {
                f = f.next_down();
            }
            k += 1;
        }
        if ok { f } else { f64::NEG_INFINITY }
    }

    /// A float `≥` the value, checked exactly (`+∞` if none is found).
    pub fn hi_f64(&self) -> f64 {
        let mut f = self.approx();
        let mut ok = false;
        let mut k = 0;
        while !ok && k < 4 {
            if !f.is_finite() {
                f = f64::INFINITY;
                ok = true;
            } else if self.cmp_f64(f) <= 0 {
                ok = true;
            } else {
                f = f.next_up();
            }
            k += 1;
        }
        if ok { f } else { f64::INFINITY }
    }
}

/// `ForgeExact.Ref.qabs`.
pub fn qabs(q: &Q) -> Q {
    if q.signum() < 0 { q.neg() } else { q.clone() }
}

/// `ForgeExact.Ref.qmax`.
pub fn qmax(a: &Q, b: &Q) -> Q {
    if a.le(b) { b.clone() } else { a.clone() }
}

/// `ForgeExact.Alg.qsgn`.
pub fn qsgn(c: &Q) -> i32 {
    c.signum()
}

// ---------------------------------------------------------------------------------------------
// Polynomials (Alg/Poly.lean): coefficient lists, constant term first
// ---------------------------------------------------------------------------------------------

/// A rational polynomial, `p[0] + p[1] x + …` (Lean's `Poly := List Rat`).
pub type Poly = Vec<Q>;

/// `padd`: coefficientwise, the longer list's tail kept.
pub fn padd(p: &[Q], q: &[Q]) -> Poly {
    let n = if p.len() >= q.len() { p.len() } else { q.len() };
    let mut out: Poly = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        if i < p.len() && i < q.len() {
            out.push(p[i].add(&q[i]));
        } else if i < p.len() {
            out.push(p[i].clone());
        } else {
            out.push(q[i].clone());
        }
        i += 1;
    }
    out
}

/// `pscale c p`.
pub fn pscale(c: &Q, p: &[Q]) -> Poly {
    let mut out: Poly = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        out.push(c.mul(&p[i]));
        i += 1;
    }
    out
}

/// `pneg p`.
pub fn pneg(p: &[Q]) -> Poly {
    let mut out: Poly = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        out.push(p[i].neg());
        i += 1;
    }
    out
}

/// `psub p q = padd p (pneg q)`.
pub fn psub(p: &[Q], q: &[Q]) -> Poly {
    padd(p, &pneg(q))
}

/// `pmul`: `[] * q = []`, `(a :: p) * q = padd (a q) (0 :: p q)`, unfolded from the back.
pub fn pmul(p: &[Q], q: &[Q]) -> Poly {
    let mut r: Poly = Vec::new();
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        let mut sh: Poly = Vec::with_capacity(r.len() + 1);
        sh.push(Q::zero());
        let mut k = 0;
        while k < r.len() {
            sh.push(r[k].clone());
            k += 1;
        }
        r = padd(&pscale(&p[i], q), &sh);
    }
    r
}

/// `pshiftUp k p`: `k` zeros, then `p`.
pub fn pshift_up(k: usize, p: &[Q]) -> Poly {
    let mut out: Poly = Vec::with_capacity(k + p.len());
    let mut i = 0;
    while i < k {
        out.push(Q::zero());
        i += 1;
    }
    let mut j = 0;
    while j < p.len() {
        out.push(p[j].clone());
        j += 1;
    }
    out
}

/// `ppow p k`.
pub fn ppow(p: &[Q], k: usize) -> Poly {
    let mut r: Poly = vec![Q::one()];
    let mut i = 0;
    while i < k {
        r = pmul(p, &r);
        i += 1;
    }
    r
}

/// `pcomp p q = p (q x)`, Horner: `foldr (fun c acc => padd [c] (pmul q acc)) []`.
pub fn pcomp(p: &[Q], q: &[Q]) -> Poly {
    let mut acc: Poly = Vec::new();
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        acc = padd(&[p[i].clone()], &pmul(q, &acc));
    }
    acc
}

/// `Poly.ev p x`, Horner.
pub fn ev(p: &[Q], x: &Q) -> Q {
    let mut acc = Q::zero();
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        acc = p[i].add(&x.mul(&acc));
    }
    acc
}

/// `Poly.trim`: trailing zeros dropped.
pub fn trim(p: &[Q]) -> Poly {
    let mut n = p.len();
    while n > 0 && p[n - 1].is_zero() {
        n -= 1;
    }
    p[..n].to_vec()
}

/// `Poly.isZero`.
pub fn is_zero_p(p: &[Q]) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i < p.len() {
        ok = p[i].is_zero();
        i += 1;
    }
    ok
}

/// `Poly.eqv`: equal as polynomials (the checked identity of `eqv_spec`).
pub fn eqv(p: &[Q], q: &[Q]) -> bool {
    is_zero_p(&psub(p, q))
}

/// `Poly.deg` (0 for the zero polynomial, as Lean's `Nat` subtraction gives).
pub fn deg(p: &[Q]) -> usize {
    let t = trim(p);
    if t.is_empty() { 0 } else { t.len() - 1 }
}

/// `Poly.lead`.
pub fn lead(p: &[Q]) -> Q {
    let t = trim(p);
    if t.is_empty() {
        Q::zero()
    } else {
        t[t.len() - 1].clone()
    }
}

/// `Poly.divmod`: untrusted long division, `p = q d + r`.
pub fn divmod(p: &[Q], d0: &[Q]) -> (Poly, Poly) {
    let d = trim(d0);
    if d.is_empty() {
        (Vec::new(), p.to_vec())
    } else {
        let m = d.len() - 1;
        let ld = d[m].clone();
        let mut r = trim(p);
        let qlen = r.len().saturating_sub(m);
        let mut q: Poly = vec![Q::zero(); qlen];
        let mut it = 0;
        let mut go = true;
        while go && it < p.len() + 1 {
            if r.len() < m + 1 {
                go = false;
            } else {
                let k = r.len() - 1;
                let c = r[k].div(&ld);
                if k - m < q.len() {
                    q[k - m] = c.clone();
                }
                let s = psub(&r, &pscale(&c, &pshift_up(k - m, &d)));
                let mut s2 = s;
                s2.pop();
                r = trim(&s2);
            }
            it += 1;
        }
        (trim(&q), trim(&r))
    }
}

/// `Poly.pmod`.
pub fn pmod(p: &[Q], d: &[Q]) -> Poly {
    divmod(p, d).1
}

/// `Poly.pdiv`.
pub fn pdiv(p: &[Q], d: &[Q]) -> Poly {
    divmod(p, d).0
}

/// `Poly.monic`.
pub fn monic(p: &[Q]) -> Poly {
    let l = lead(p);
    if l.is_zero() {
        p.to_vec()
    } else {
        pscale(&Q::one().div(&l), &trim(p))
    }
}

/// `Poly.deriv`.
pub fn deriv(p: &[Q]) -> Poly {
    let mut out: Poly = Vec::new();
    let mut i = 1;
    while i < p.len() {
        out.push(p[i].mul(&Q::from_i64(i as i64)));
        i += 1;
    }
    out
}

/// `Poly.xgcd`: `(g, u, v)` with `g = u p + v q`, `g` monic. Untrusted.
pub fn xgcd(p: &[Q], q: &[Q]) -> (Poly, Poly, Poly) {
    let mut r0 = trim(p);
    let mut r1 = trim(q);
    let mut u0: Poly = vec![Q::one()];
    let mut v0: Poly = Vec::new();
    let mut u1: Poly = Vec::new();
    let mut v1: Poly = vec![Q::one()];
    let mut it = 0;
    let n = p.len() + q.len() + 2;
    let mut go = true;
    while go && it < n {
        if is_zero_p(&r1) {
            go = false;
        } else {
            let (qq, rr) = divmod(&r0, &r1);
            let u2 = trim(&psub(&u0, &pmul(&qq, &u1)));
            let v2 = trim(&psub(&v0, &pmul(&qq, &v1)));
            r0 = r1;
            r1 = rr;
            u0 = u1;
            v0 = v1;
            u1 = u2;
            v1 = v2;
        }
        it += 1;
    }
    let l = lead(&r0);
    if l.is_zero() {
        (r0, u0, v0)
    } else {
        let s = Q::one().div(&l);
        (pscale(&s, &r0), pscale(&s, &u0), pscale(&s, &v0))
    }
}

/// `Poly.gcd`.
pub fn gcd(p: &[Q], q: &[Q]) -> Poly {
    xgcd(p, q).0
}

/// `Poly.sqfree`: the square-free part. Untrusted.
pub fn sqfree(p: &[Q]) -> Poly {
    let g = gcd(p, &deriv(p));
    if deg(&g) == 0 {
        monic(p)
    } else {
        monic(&pdiv(p, &g))
    }
}

/// A polynomial from small integers (tests and constants).
pub fn poly_i(cs: &[i64]) -> Poly {
    let mut out: Poly = Vec::with_capacity(cs.len());
    let mut i = 0;
    while i < cs.len() {
        out.push(Q::from_i64(cs[i]));
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Descartes' two cases (Alg/Model.lean) and the Möbius transform (Alg/Real.lean)
// ---------------------------------------------------------------------------------------------

/// `noPosRoot`: the nonzero coefficients have one sign, and one is nonzero.
pub fn no_pos_root(p: &[Q]) -> bool {
    let mut all_nn = true;
    let mut all_np = true;
    let mut any_p = false;
    let mut any_n = false;
    let mut i = 0;
    while i < p.len() {
        let s = p[i].signum();
        if s < 0 {
            all_nn = false;
            any_n = true;
        }
        if s > 0 {
            all_np = false;
            any_p = true;
        }
        i += 1;
    }
    (all_nn && any_p) || (all_np && any_n)
}

/// `patOK p k`: the first `k` coefficients `≥ 0`, the rest `≤ 0`.
fn pat_ok(p: &[Q], k: usize) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i < p.len() {
        ok = if i < k {
            p[i].signum() >= 0
        } else {
            p[i].signum() <= 0
        };
        i += 1;
    }
    ok
}

/// `patStrict p k`: one of the first `k` coefficients is `> 0`.
fn pat_strict(p: &[Q], k: usize) -> bool {
    let mut any = false;
    let mut i = 0;
    while !any && i < p.len() && i < k {
        any = p[i].signum() > 0;
        i += 1;
    }
    any
}

/// `onePattern`.
fn one_pattern(p: &[Q]) -> bool {
    let mut any = false;
    let mut k = 0;
    while !any && k < p.len() + 1 {
        any = pat_ok(p, k) && pat_strict(p, k);
        k += 1;
    }
    any
}

/// `onePosRoot`: at most one positive root, certified (one sign change, for `p` or `−p`).
pub fn one_pos_root(p: &[Q]) -> bool {
    one_pattern(p) || one_pattern(&pneg(p))
}

/// `moebius a b p = (1 + x)^(n−1) p((a + b x)/(1 + x))`, by the Lean recursion
/// `moebius a b (c :: p) = padd (c (1 + x)^|p|) ([a, b] · moebius a b p)`, from the back.
pub fn moebius(a: &Q, b: &Q, p: &[Q]) -> Poly {
    let one_x: Poly = vec![Q::one(), Q::one()];
    let ab: Poly = vec![a.clone(), b.clone()];
    let mut acc: Poly = Vec::new();
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        let tail_len = p.len() - 1 - i;
        acc = padd(&pscale(&p[i], &ppow(&one_x, tail_len)), &pmul(&ab, &acc));
    }
    acc
}

// ---------------------------------------------------------------------------------------------
// Real algebraic numbers (Alg/Real.lean)
// ---------------------------------------------------------------------------------------------

/// `Iso`: a polynomial with an interval meant to isolate one of its roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Iso {
    pub p: Poly,
    pub a: Q,
    pub b: Q,
}

impl Iso {
    /// `Iso.ok`: the isolation certificate (`a < b`, a sign change, one root).
    pub fn ok(&self) -> bool {
        self.a.lt(&self.b)
            && ev(&self.p, &self.a).mul(&ev(&self.p, &self.b)).signum() < 0
            && one_pos_root(&moebius(&self.a, &self.b, &self.p))
    }

    /// `Iso.zeroAt`: `q` vanishes at the root, certified by a common factor that changes sign.
    pub fn zero_at(&self, q: &[Q]) -> bool {
        let g = gcd(&self.p, q);
        eqv(&self.p, &pmul(&g, &pdiv(&self.p, &g)))
            && eqv(q, &pmul(&g, &pdiv(q, &g)))
            && ev(&g, &self.a).mul(&ev(&g, &self.b)).signum() < 0
    }
}

/// `Num`: a rational, or an isolated root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Num {
    Rat(Q),
    Iso(Iso),
}

impl Num {
    /// `Num.cmpRat`: the number against the rational `c`, −1, 0 or 1.
    pub fn cmp_rat(&self, c: &Q) -> i32 {
        match self {
            Num::Rat(d) => d.cmp_q(c),
            Num::Iso(r) => {
                if c.le(&r.a) {
                    1
                } else if r.b.le(c) {
                    -1
                } else {
                    let v = ev(&r.p, c);
                    if v.is_zero() {
                        0
                    } else if v.mul(&ev(&r.p, &r.b)).signum() < 0 {
                        1
                    } else {
                        -1
                    }
                }
            }
        }
    }

    /// `Num.bisect`: the interval halved, the root kept; a rational if it is the midpoint.
    pub fn bisect(&self) -> Num {
        match self {
            Num::Rat(c) => Num::Rat(c.clone()),
            Num::Iso(r) => {
                let m = r.a.mid(&r.b);
                let v = ev(&r.p, &m);
                if v.is_zero() {
                    Num::Rat(m)
                } else if ev(&r.p, &r.a).mul(&v).signum() < 0 {
                    Num::Iso(Iso {
                        p: r.p.clone(),
                        a: r.a.clone(),
                        b: m,
                    })
                } else {
                    Num::Iso(Iso {
                        p: r.p.clone(),
                        a: m,
                        b: r.b.clone(),
                    })
                }
            }
        }
    }

    /// `Num.width`.
    pub fn width(&self) -> Q {
        match self {
            Num::Rat(_) => Q::zero(),
            Num::Iso(r) => r.b.sub(&r.a),
        }
    }

    /// `Num.lo`.
    pub fn lo(&self) -> Q {
        match self {
            Num::Rat(c) => c.clone(),
            Num::Iso(r) => r.a.clone(),
        }
    }

    /// `Num.hi`.
    pub fn hi(&self) -> Q {
        match self {
            Num::Rat(c) => c.clone(),
            Num::Iso(r) => r.b.clone(),
        }
    }

    /// `Num.refine k`.
    pub fn refine(&self, k: usize) -> Num {
        let mut n = self.clone();
        let mut i = 0;
        while i < k {
            n = n.bisect();
            i += 1;
        }
        n
    }

    /// `Num.signNZ q fuel`: bisect until `q` has no root in the interval, then read `q` at the
    /// midpoint. `None` when the fuel runs out.
    pub fn sign_nz(&self, q: &[Q], fuel: usize) -> Option<i32> {
        let mut n = self.clone();
        let mut f = fuel;
        let mut out: Option<i32> = None;
        let mut done = false;
        while !done {
            match &n {
                Num::Rat(c) => {
                    out = Some(qsgn(&ev(q, c)));
                    done = true;
                }
                Num::Iso(r) => {
                    if f == 0 {
                        done = true;
                    } else if no_pos_root(&moebius(&r.a, &r.b, q)) {
                        out = Some(qsgn(&ev(q, &r.a.mid(&r.b))));
                        done = true;
                    } else {
                        f -= 1;
                        n = n.bisect();
                    }
                }
            }
        }
        out
    }

    /// **`Num.sign q fuel`**: the sign of `q` at the number, or `None` when the fuel runs out
    /// (`sign_spec`: never a wrong sign).
    pub fn sign(&self, q: &[Q], fuel: usize) -> Option<i32> {
        match self {
            Num::Rat(c) => Some(qsgn(&ev(q, c))),
            Num::Iso(r) => {
                if r.zero_at(q) {
                    Some(0)
                } else {
                    self.sign_nz(q, fuel)
                }
            }
        }
    }

    /// `Num.cmp fuel n m`: −1, 0 or 1, or `None` when the fuel runs out (`cmp_spec`).
    pub fn cmp(fuel: usize, n: &Num, m: &Num) -> Option<i32> {
        let mut u = n.clone();
        let mut v = m.clone();
        let mut f = fuel;
        let mut out: Option<i32> = None;
        let mut done = false;
        while !done {
            match (&u, &v) {
                (_, Num::Rat(c)) => {
                    out = Some(u.cmp_rat(c));
                    done = true;
                }
                (Num::Rat(c), _) => {
                    out = Some(-v.cmp_rat(c));
                    done = true;
                }
                (Num::Iso(r), Num::Iso(s)) => {
                    if f == 0 {
                        done = true;
                    } else if r.b.le(&s.a) {
                        out = Some(-1);
                        done = true;
                    } else if s.b.le(&r.a) {
                        out = Some(1);
                        done = true;
                    } else if u.sign(&s.p, f - 1) == Some(0) && u.cmp_rat(&s.a) == 1 && u.cmp_rat(&s.b) == -1
                    {
                        out = Some(0);
                        done = true;
                    } else {
                        f -= 1;
                        let u2 = u.bisect();
                        let v2 = v.bisect();
                        u = u2;
                        v = v2;
                    }
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// From QRef/Decide.lean: root isolation and rationals between numbers (untrusted proposers)
// ---------------------------------------------------------------------------------------------

/// `simplest a b`: the simplest rational in `[a, b]` (Stern–Brocot), for `a ≤ b`.
pub fn simplest(a: &Q, b: &Q) -> Q {
    if a.signum() <= 0 && b.signum() >= 0 {
        Q::zero()
    } else if b.signum() < 0 {
        simplest(&b.neg(), &a.neg()).neg()
    } else {
        let fa = Q::from_big(a.floor());
        if fa.cmp_q(a) == 0 {
            a.clone()
        } else {
            let fa1 = fa.add(&Q::one());
            if fa1.le(b) {
                fa1
            } else {
                fa.add(&Q::one().div(&simplest(&Q::one().div(&b.sub(&fa)), &Q::one().div(&a.sub(&fa)))))
            }
        }
    }
}

/// `rootBound`: Cauchy's bound on the roots' magnitude.
pub fn root_bound(p: &[Q]) -> Q {
    let t = trim(p);
    if t.is_empty() {
        Q::one()
    } else {
        let l = t[t.len() - 1].clone();
        let mut m = Q::zero();
        let mut i = 0;
        while i < t.len() {
            m = qmax(&m, &qabs(&t[i].div(&l)));
            i += 1;
        }
        Q::one().add(&m)
    }
}

/// `ratSqrt?`: the square root of a rational, if it is rational.
pub fn rat_sqrt(q: &Q) -> Option<Q> {
    if q.signum() < 0 {
        None
    } else {
        let sn = isqrt_big(&q.n);
        let sd = isqrt_big(&q.d);
        if bcmp(&sn.mul(&sn), &q.n) == 0 && bcmp(&sd.mul(&sd), &q.d) == 0 {
            Some(Q::ratio(&sn, &sd))
        } else {
            None
        }
    }
}

/// The isolation fuel's work cap (`realRoots`' `for _ in [0:4000]`).
pub const REAL_ROOTS_STEPS: usize = 4000;

/// `realRoots p fuel`: the real roots of `p` (assumed square-free), each isolated or rational,
/// in the Lean's order. Untrusted: every `Iso` returned has passed `Iso::ok`.
pub fn real_roots(p0: &[Q], fuel: usize) -> Vec<Num> {
    let p = trim(p0);
    let mut out: Vec<Num> = Vec::new();
    let dg = deg(&p);
    if dg == 0 {
        return out;
    }
    if dg == 1 {
        out.push(Num::Rat(p[0].neg().div(&p[1])));
        return out;
    }
    if dg == 2 {
        let (c, b, a) = (p[0].clone(), p[1].clone(), p[2].clone());
        let disc = b.mul(&b).sub(&Q::from_i64(4).mul(&a).mul(&c));
        if disc.signum() < 0 {
            return out;
        }
        let a2 = Q::from_i64(2).mul(&a);
        if disc.is_zero() {
            out.push(Num::Rat(b.neg().div(&a2)));
            return out;
        }
        if let Some(r) = rat_sqrt(&disc) {
            out.push(Num::Rat(b.neg().sub(&r).div(&a2)));
            out.push(Num::Rat(b.neg().add(&r).div(&a2)));
            return out;
        }
    }
    let bnd = root_bound(&p);
    // the stack, top last (Lean's list head)
    let mut stack: Vec<(Q, Q, usize)> = vec![(bnd.neg(), bnd.clone(), 0)];
    if ev(&p, &bnd.neg()).is_zero() {
        out.push(Num::Rat(bnd.neg()));
    }
    if ev(&p, &bnd).is_zero() {
        out.push(Num::Rat(bnd.clone()));
    }
    let mut it = 0;
    while it < REAL_ROOTS_STEPS && !stack.is_empty() {
        let (a, b, d) = stack.pop().unwrap_or((Q::zero(), Q::zero(), 0));
        let tr = moebius(&a, &b, &p);
        if !no_pos_root(&tr) {
            let iso = Iso {
                p: p.clone(),
                a: a.clone(),
                b: b.clone(),
            };
            if one_pos_root(&tr) && iso.ok() {
                let w = b.sub(&a).div(&Q::from_i64(8));
                let s = simplest(&a.add(&w), &b.sub(&w));
                if a.lt(&s) && s.lt(&b) && ev(&p, &s).is_zero() {
                    out.push(Num::Rat(s));
                } else {
                    out.push(Num::Iso(iso));
                }
            } else if d < fuel {
                let m = a.mid(&b);
                if ev(&p, &m).is_zero() {
                    out.push(Num::Rat(m.clone()));
                }
                // Lean pushes (a, m) then (m, b) at the head: (a, m) is taken first
                stack.push((m.clone(), b, d + 1));
                stack.push((a, m, d + 1));
            }
        }
        it += 1;
    }
    out
}

/// `between n m`: a rational strictly between two numbers known to be in order, or `None`.
pub fn between(n: &Num, m: &Num) -> Option<Q> {
    let mut u = n.clone();
    let mut v = m.clone();
    let mut out: Option<Q> = None;
    let mut i = 0;
    while out.is_none() && i < 200 {
        let (uh, vl) = (u.hi(), v.lo());
        if uh.lt(&vl) {
            out = Some(uh.mid(&vl));
        } else {
            u = u.bisect();
            v = v.bisect();
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i64, d: i64) -> Q {
        Q::ratio(&Big::from_i64(n), &Big::from_i64(d))
    }

    #[test]
    fn rationals_normalise() {
        assert_eq!(q(6, -4), q(-3, 2));
        assert_eq!(q(0, 5), Q::zero());
        assert_eq!(q(1, 3).add(&q(1, 6)), q(1, 2));
        assert_eq!(q(7, 2).floor(), Big::from_i64(3));
        assert_eq!(q(-7, 2).floor(), Big::from_i64(-4));
        assert_eq!(q(1, 0), Q::zero());
        assert_eq!(Q::from_f64(0.375), Some(q(3, 8)));
        assert!(q(1, 3).lo_f64() <= 1.0 / 3.0 && q(1, 3).hi_f64() >= 1.0 / 3.0);
        // a long one: (2^200 + 1) / 3^50 normalises and round-trips
        let big = Big::from_i64(1).shl(200).add(&Big::from_i64(1));
        let mut t = Big::from_i64(1);
        let mut i = 0;
        while i < 50 {
            t = t.mul(&Big::from_i64(3));
            i += 1;
        }
        let x = Q::ratio(&big, &t);
        assert_eq!(x.mul(&Q::from_big(t.clone())), Q::from_big(big.clone()));
        assert_eq!(isqrt_big(&Big::from_i64(99)), Big::from_i64(9));
    }

    #[test]
    fn polys_as_the_lean() {
        // pmul's length: [a] * [] = [0], as the Lean recursion gives
        assert_eq!(pmul(&poly_i(&[3]), &[]).len(), 1);
        assert_eq!(pmul(&poly_i(&[1, 1]), &poly_i(&[-1, 1])), poly_i(&[-1, 0, 1]));
        let (qq, r) = divmod(&poly_i(&[-1, 0, 1]), &poly_i(&[-1, 1]));
        assert_eq!(qq, poly_i(&[1, 1]));
        assert!(r.is_empty());
        // gcd of (x−1)(x−2) and (x−1)(x+3) is x − 1
        let a = pmul(&poly_i(&[-1, 1]), &poly_i(&[-2, 1]));
        let b = pmul(&poly_i(&[-1, 1]), &poly_i(&[3, 1]));
        assert_eq!(gcd(&a, &b), poly_i(&[-1, 1]));
        // sqfree of (x−1)²(x+1) is (x−1)(x+1)
        let s = pmul(&pmul(&poly_i(&[-1, 1]), &poly_i(&[-1, 1])), &poly_i(&[1, 1]));
        assert_eq!(sqfree(&s), poly_i(&[-1, 0, 1]));
    }

    #[test]
    fn roots_and_signs() {
        // x² − 2: two irrational roots, isolated
        let p = poly_i(&[-2, 0, 1]);
        let rs = real_roots(&poly_i(&[-2, 0, 0, 1]), 60);
        assert_eq!(rs.len(), 1); // x³ − 2
        // the quadratic case finds no rational root and falls through to isolation, as the Lean
        assert_eq!(real_roots(&p, 60).len(), 2);
        let r3 = real_roots(&pmul(&p, &poly_i(&[-3, 1])), 60);
        assert_eq!(r3.len(), 3);
        let mut i = 0;
        while i < r3.len() {
            if let Num::Iso(iso) = &r3[i] {
                assert!(iso.ok());
            }
            i += 1;
        }
        // sign of x − 1 at the root √2 of x² − 2 is +; of x² − 2 itself, 0
        let s2 = Num::Iso(Iso {
            p: p.clone(),
            a: Q::one(),
            b: Q::from_i64(2),
        });
        assert_eq!(s2.sign(&poly_i(&[-1, 1]), 200), Some(1));
        assert_eq!(s2.sign(&p, 200), Some(0));
        assert_eq!(s2.cmp_rat(&q(3, 2)), -1);
        assert_eq!(s2.cmp_rat(&q(7, 5)), 1);
        let three = Num::Rat(Q::from_i64(3));
        assert_eq!(Num::cmp(200, &s2, &three), Some(-1));
        assert_eq!(Num::cmp(200, &s2, &s2), Some(0));
        let b = between(&s2, &three).unwrap();
        assert!(s2.cmp_rat(&b) < 0 && b.lt(&Q::from_i64(3)));
        assert_eq!(simplest(&q(1, 3), &q(1, 2)), q(1, 2));
        assert_eq!(simplest(&q(3, 10), &q(2, 5)), q(1, 3));
    }
}
