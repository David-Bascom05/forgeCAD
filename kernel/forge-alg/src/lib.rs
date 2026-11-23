//! Exact rationals, rational polynomials and real algebraic numbers over [`Big`]: the Rust side
//! of verify/forge-exact's `ForgeExact.Alg` (Poly.lean, Model.lean, Real.lean), the parts of
//! `ForgeExact.QRef.Decide` the curved decide-once bridge calls (`between`, `simplest`,
//! `rootBound`), and `DecideOnce/RootCover.lean` (`rootsOf`: roots complete by certificate).
//! `QRef.realRoots` is deliberately not mirrored: it drops an interval still being split when
//! its fuel runs out, so it can lose roots silently.
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
//! `split_gap`, `ray_bound`, `propose_sqfree`) are untrusted, as in the Lean: whatever they return is checked before a decision
//! rests on it. L4 for this module is **interim** (design D6): a differential test against the
//! Lean L2; the blocker is Aeneas and bignum arithmetic.
//!
//! **Style** (research/kernel-audit/decide-once/aeneas/README.md): no struct holds a borrow, no
//! `break`, no `return` inside a loop, no `Ordering` (comparisons answer −1, 0 or 1), loops are
//! `while` loops over indices. Only [`Big`]'s public API is used; this crate is separate from `forge-math` so that nothing in
//! it can move `forge-math`'s code generation (see `div`).

mod div;
mod fguide;
pub use div::{divrem, gcd as big_gcd};
use forge_math::exact::{Big, Dy};

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

/// `(q, r)` with `a = q b + r` and `0 ≤ r < b`, for `a ≥ 0` and `b > 0` (`divrem`).
pub fn divmod_nat(a: &Big, b: &Big) -> (Big, Big) {
    divrem(a, b)
}

/// The gcd of `|a|` and `|b|` (`big_gcd`).
pub fn gcd_big(a: &Big, b: &Big) -> Big {
    big_gcd(a, b)
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

    /// Henrici's sum: with `g = gcd(d₁, d₂)`, `t = n₁(d₂/g) + n₂(d₁/g)` and `h = gcd(t, g)`,
    /// the normal form is `(t/h) / ((d₁/g)(d₂/h))`: gcds of the small operands only.
    pub fn add(&self, o: &Q) -> Q {
        if self.is_zero() {
            o.clone()
        } else if o.is_zero() {
            self.clone()
        } else {
            let g = big_gcd(&self.d, &o.d);
            if bcmp(&g, &big1()) == 0 {
                Q {
                    n: self.n.mul(&o.d).add(&o.n.mul(&self.d)),
                    d: self.d.mul(&o.d),
                }
            } else {
                let d1g = divrem(&self.d, &g).0;
                let d2g = divrem(&o.d, &g).0;
                let t = self.n.mul(&d2g).add(&o.n.mul(&d1g));
                if t.is_zero() {
                    Q::zero()
                } else {
                    let h = big_gcd(&t, &g);
                    if bcmp(&h, &big1()) == 0 {
                        Q {
                            n: t,
                            d: d1g.mul(&o.d),
                        }
                    } else {
                        Q {
                            n: divrem(&t, &h).0,
                            d: d1g.mul(&divrem(&o.d, &h).0),
                        }
                    }
                }
            }
        }
    }

    pub fn sub(&self, o: &Q) -> Q {
        self.add(&o.neg())
    }

    /// Henrici's product: `g₁ = gcd(n₁, d₂)`, `g₂ = gcd(n₂, d₁)`, and the normal form is
    /// `(n₁/g₁)(n₂/g₂) / ((d₁/g₂)(d₂/g₁))`.
    pub fn mul(&self, o: &Q) -> Q {
        if self.is_zero() || o.is_zero() {
            Q::zero()
        } else {
            let g1 = big_gcd(&self.n, &o.d);
            let g2 = big_gcd(&o.n, &self.d);
            let one = big1();
            let (n1, d2) = if bcmp(&g1, &one) == 0 {
                (self.n.clone(), o.d.clone())
            } else {
                (divrem(&self.n, &g1).0, divrem(&o.d, &g1).0)
            };
            let (n2, d1) = if bcmp(&g2, &one) == 0 {
                (o.n.clone(), self.d.clone())
            } else {
                (divrem(&o.n, &g2).0, divrem(&self.d, &g2).0)
            };
            Q {
                n: n1.mul(&n2),
                d: d1.mul(&d2),
            }
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
        forge_math::exact::ratio_f64(
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

/// A coefficient ring for the list polynomials: `Q`, and lists of a coefficient ring (so
/// `Vec<Q>` is Lean's `Poly`, `Vec<Vec<Q>>` its `BPoly` over `YPoly`, and `Vec<Vec<Vec<Q>>>` its
/// `TPoly` over `ZC`). The list instance's zero is `[]` and its one `[1]`, as `YPoly`'s and `ZC`'s.
pub trait Coef: Clone {
    fn c_zero() -> Self;
    fn c_one() -> Self;
    fn c_add(&self, o: &Self) -> Self;
    fn c_mul(&self, o: &Self) -> Self;
    fn c_neg(&self) -> Self;
}

impl Coef for Q {
    fn c_zero() -> Q {
        Q::zero()
    }
    fn c_one() -> Q {
        Q::one()
    }
    fn c_add(&self, o: &Q) -> Q {
        self.add(o)
    }
    fn c_mul(&self, o: &Q) -> Q {
        self.mul(o)
    }
    fn c_neg(&self) -> Q {
        self.neg()
    }
}

impl<T: Coef> Coef for Vec<T> {
    fn c_zero() -> Vec<T> {
        Vec::new()
    }
    fn c_one() -> Vec<T> {
        vec![T::c_one()]
    }
    fn c_add(&self, o: &Vec<T>) -> Vec<T> {
        padd(self, o)
    }
    fn c_mul(&self, o: &Vec<T>) -> Vec<T> {
        pmul(self, o)
    }
    fn c_neg(&self) -> Vec<T> {
        pneg(self)
    }
}

/// A rational polynomial, `p[0] + p[1] x + …` (Lean's `Poly := List Rat`).
pub type Poly = Vec<Q>;

/// `padd`: coefficientwise, the longer list's tail kept.
pub fn padd<T: Coef>(p: &[T], q: &[T]) -> Vec<T> {
    let n = if p.len() >= q.len() { p.len() } else { q.len() };
    let mut out: Vec<T> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        if i < p.len() && i < q.len() {
            out.push(p[i].c_add(&q[i]));
        } else if i < p.len() {
            out.push(p[i].clone());
        } else {
            out.push(q[i].clone());
        }
        i += 1;
    }
    out
}

/// `pscale c p = p.map (c * ·)`.
pub fn pscale<T: Coef>(c: &T, p: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        out.push(c.c_mul(&p[i]));
        i += 1;
    }
    out
}

/// `pneg p`.
pub fn pneg<T: Coef>(p: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        out.push(p[i].c_neg());
        i += 1;
    }
    out
}

/// `psub p q = padd p (pneg q)`.
pub fn psub<T: Coef>(p: &[T], q: &[T]) -> Vec<T> {
    padd(p, &pneg(q))
}

/// `pmul`: `[] * q = []`, `(a :: p) * q = padd (a q) (0 :: p q)`, unfolded from the back.
pub fn pmul<T: Coef>(p: &[T], q: &[T]) -> Vec<T> {
    let mut r: Vec<T> = Vec::new();
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        let mut sh: Vec<T> = Vec::with_capacity(r.len() + 1);
        sh.push(T::c_zero());
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
pub fn pshift_up<T: Coef>(k: usize, p: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(k + p.len());
    let mut i = 0;
    while i < k {
        out.push(T::c_zero());
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
pub fn ppow<T: Coef>(p: &[T], k: usize) -> Vec<T> {
    let mut r: Vec<T> = vec![T::c_one()];
    let mut i = 0;
    while i < k {
        r = pmul(p, &r);
        i += 1;
    }
    r
}

/// `pcomp p q = p (q x)`, Horner: `foldr (fun c acc => padd [c] (pmul q acc)) []`.
pub fn pcomp<T: Coef>(p: &[T], q: &[T]) -> Vec<T> {
    let mut acc: Vec<T> = Vec::new();
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

// ---------------------------------------------------------------------------------------------
// DecideOnce/RootCover.lean: a certified cover of a polynomial's real roots
// ---------------------------------------------------------------------------------------------

/// `gapOk`: the gap `(a, b)` is root-free or isolates one root.
pub fn gap_ok(p: &[Q], a: &Q, b: &Q) -> bool {
    a.lt(b)
        && (no_pos_root(&moebius(a, b, p))
            || Iso {
                p: p.to_vec(),
                a: a.clone(),
                b: b.clone(),
            }
            .ok())
}

/// `gapsOk p t0 ts`: every gap of the breakpoints `t0 :: ts` checks.
pub fn gaps_ok(p: &[Q], t0: &Q, ts: &[Q]) -> bool {
    let mut ok = true;
    let mut a = t0.clone();
    let mut i = 0;
    while ok && i < ts.len() {
        ok = gap_ok(p, &a, &ts[i]);
        a = ts[i].clone();
        i += 1;
    }
    ok
}

/// `lastOf t0 ts`.
pub fn last_of(t0: &Q, ts: &[Q]) -> Q {
    if ts.is_empty() {
        t0.clone()
    } else {
        ts[ts.len() - 1].clone()
    }
}

/// **`coverOk`**: both rays root-free (Descartes on `p(t₀ − s)` and `p(tₘ + s)`), every gap
/// certified. `cover_complete`: then every root of `p` is in `cover_roots`.
pub fn cover_ok(p: &[Q], t0: &Q, ts: &[Q]) -> bool {
    no_pos_root(&pcomp(p, &[t0.clone(), Q::from_i64(-1)]))
        && no_pos_root(&pcomp(p, &[last_of(t0, ts), Q::one()]))
        && gaps_ok(p, t0, ts)
}

/// `gapRoots`: each gap that is not root-free, as its isolated root.
pub fn gap_roots(p: &[Q], t0: &Q, ts: &[Q]) -> Vec<Num> {
    let mut out: Vec<Num> = Vec::new();
    let mut a = t0.clone();
    let mut i = 0;
    while i < ts.len() {
        if !no_pos_root(&moebius(&a, &ts[i], p)) {
            out.push(Num::Iso(Iso {
                p: p.to_vec(),
                a: a.clone(),
                b: ts[i].clone(),
            }));
        }
        a = ts[i].clone();
        i += 1;
    }
    out
}

/// `coverRoots`: the breakpoints where `p` vanishes, then the gaps' isolated roots.
pub fn cover_roots(p: &[Q], t0: &Q, ts: &[Q]) -> Vec<Num> {
    let mut out: Vec<Num> = Vec::new();
    if ev(p, t0).is_zero() {
        out.push(Num::Rat(t0.clone()));
    }
    let mut i = 0;
    while i < ts.len() {
        if ev(p, &ts[i]).is_zero() {
            out.push(Num::Rat(ts[i].clone()));
        }
        i += 1;
    }
    let g = gap_roots(p, t0, ts);
    let mut j = 0;
    while j < g.len() {
        out.push(g[j].clone());
        j += 1;
    }
    out
}

/// `splitGap p n a b`: split `(a, b)` until every gap checks; the interior breakpoints, in order
/// (untrusted).
pub fn split_gap(p: &[Q], n: usize, a: &Q, b: &Q) -> Option<Vec<Q>> {
    if gap_ok(p, a, b) {
        Some(Vec::new())
    } else if n == 0 {
        None
    } else {
        let m = a.mid(b);
        match (split_gap(p, n - 1, a, &m), split_gap(p, n - 1, &m, b)) {
            (Some(l), Some(r)) => {
                let mut out = l;
                out.push(m);
                let mut i = 0;
                while i < r.len() {
                    out.push(r[i].clone());
                    i += 1;
                }
                Some(out)
            }
            _ => None,
        }
    }
}

/// `rayBound p n B`: the bound doubled until both rays check (untrusted).
pub fn ray_bound(p: &[Q], n: usize, b0: &Q) -> Option<Q> {
    let mut b = b0.clone();
    let mut k = n;
    let mut out: Option<Q> = None;
    while out.is_none() && k > 0 {
        if no_pos_root(&pcomp(p, &[b.neg(), Q::from_i64(-1)]))
            && no_pos_root(&pcomp(p, &[b.clone(), Q::one()]))
        {
            out = Some(b.clone());
        } else {
            b = Q::from_i64(2).mul(&b);
        }
        k -= 1;
    }
    out
}

/// The float guide's cover of `(−b, b)`: breakpoints between approximate roots; a gap that does
/// not check (a complex pair near it, or two roots the guide merged) is broken at the critical
/// points inside it, and a piece that still fails is bisected alone (`split_gap`). Untrusted:
/// `propose_cover` checks the whole cover with `cover_ok`.
fn guided_cover(p: &[Q], b: &Q, fuel: usize) -> Option<Vec<Q>> {
    let pf: Vec<f64> = p.iter().map(|c| c.approx()).collect();
    let (pts, crit) = fguide::cover_points(&pf, b.approx())?;
    let mut guide: Vec<Q> = Vec::with_capacity(pts.len() + 1);
    let mut i = 0;
    while i < pts.len() {
        let q = Q::from_f64(pts[i])?;
        if !(b.neg().lt(&q) && q.lt(b)) {
            return None;
        }
        guide.push(q);
        i += 1;
    }
    guide.push(b.clone());
    let mut ts: Vec<Q> = Vec::with_capacity(guide.len());
    let mut a = b.neg();
    let mut k = 0;
    while k < guide.len() {
        if !gap_ok(p, &a, &guide[k]) {
            // the critical points strictly inside, as breakpoints
            let (al, bl) = (a.approx(), guide[k].approx());
            let mut sub: Vec<Q> = Vec::new();
            let mut j = 0;
            while j < crit.len() {
                if crit[j] > al
                    && crit[j] < bl
                    && let Some(c) = Q::from_f64(crit[j]).filter(|c| a.lt(c) && c.lt(&guide[k]))
                {
                    sub.push(c);
                }
                j += 1;
            }
            sub.push(guide[k].clone());
            let mut lo = a.clone();
            let mut m = 0;
            while m < sub.len() {
                if !gap_ok(p, &lo, &sub[m]) {
                    ts.extend(split_gap(p, fuel, &lo, &sub[m])?);
                }
                if m + 1 < sub.len() {
                    ts.push(sub[m].clone());
                }
                lo = sub[m].clone();
                m += 1;
            }
        }
        ts.push(guide[k].clone());
        a = guide[k].clone();
        k += 1;
    }
    Some(ts)
}

/// `proposeCover p fuel`: a cover `(t₀, ts)`, checked by `cover_ok`, or `None`. The float guide
/// first (`guided_cover`), then bisection (`split_gap`).
pub fn propose_cover(p: &[Q], fuel: usize) -> Option<(Q, Vec<Q>)> {
    // Fujiwara's bound from the float guide when both rays check (usually far tighter than
    // Cauchy's, so the gaps' Möbius transforms are smaller), else Cauchy's doubled
    let pf: Vec<f64> = p.iter().map(|c| c.approx()).collect();
    let cb = root_bound(p);
    // (only where it is much tighter: its two ray checks cost more than they save otherwise)
    let fb = fguide::fujiwara(&pf)
        .filter(|f| 4.0 * f < cb.approx())
        .and_then(Q::from_f64)
        .filter(|f| {
            no_pos_root(&pcomp(p, &[f.neg(), Q::from_i64(-1)]))
                && no_pos_root(&pcomp(p, &[f.clone(), Q::one()]))
        });
    let b = match fb {
        Some(f) => f,
        None => ray_bound(p, fuel, &cb)?,
    };
    if let Some(ts) = guided_cover(p, &b, fuel)
        && cover_ok(p, &b.neg(), &ts)
    {
        return Some((b.neg(), ts));
    }
    let mut ts = split_gap(p, fuel, &b.neg(), &b)?;
    ts.push(b.clone());
    if cover_ok(p, &b.neg(), &ts) {
        Some((b.neg(), ts))
    } else {
        None
    }
}

/// `certifiedRoots p fuel`: the roots of a square-free `p`, complete by certificate, or `None`.
pub fn certified_roots(p: &[Q], fuel: usize) -> Option<Vec<Num>> {
    propose_cover(p, fuel).map(|c| cover_roots(p, &c.0, &c.1))
}

/// `sqfreeOk p sp k w h`: `sp^k = p w` and `p = sp h` (`sqfree_roots`: the same roots).
pub fn sqfree_ok(p: &[Q], sp: &[Q], k: usize, w: &[Q], h: &[Q]) -> bool {
    eqv(&ppow(sp, k), &pmul(p, w)) && eqv(p, &pmul(sp, h))
}

/// `proposeSqfree p`: the square-free part, with its certificate checked, or `None`.
pub fn propose_sqfree(p: &[Q]) -> Option<Poly> {
    let sp = sqfree(p);
    let k = deg(p) + 1;
    let w = pdiv(&ppow(&sp, k), p);
    let h = pdiv(p, &sp);
    if sqfree_ok(p, &sp, k, &w, &h) {
        Some(sp)
    } else {
        None
    }
}

/// **`rootsOf p fuel`**: the real roots of any polynomial, complete by certificate
/// (`rootsOf_complete`), or `None` (the bridge's `Skip::Budget`).
pub fn roots_of(p: &[Q], fuel: usize) -> Option<Vec<Num>> {
    match propose_sqfree(p) {
        None => None,
        Some(sp) => certified_roots(&sp, fuel),
    }
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
        // RootCover.lean's #guards
        let n = |v: Option<Vec<Num>>| v.map(|l| l.len());
        assert_eq!(n(roots_of(&poly_i(&[2, -3, 0, 1]), 30)), Some(2));
        assert_eq!(n(certified_roots(&poly_i(&[-1, 0, 1]), 30)), Some(2));
        assert_eq!(n(certified_roots(&poly_i(&[0, -2, 0, 1]), 30)), Some(3));
        assert_eq!(n(certified_roots(&poly_i(&[1, 0, 1]), 30)), Some(0));
        assert_eq!(n(roots_of(&poly_i(&[-2, 0, 0, 1]), 60)), Some(1));
        let r3 = roots_of(&pmul(&p, &poly_i(&[-3, 1])), 60).unwrap();
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

    /// The shift-and-subtract division, kept as the oracle for `Big::divrem`.
    fn divmod_slow(a: &Big, b: &Big) -> (Big, Big) {
        let mut q = Big::zero();
        let mut r = a.clone();
        if bcmp(a, b) >= 0 {
            let mut k: i64 = a.bits() as i64 - b.bits() as i64;
            while k >= 0 {
                let t = b.shl(k as u32);
                if bcmp(&t, &r) <= 0 {
                    r = r.sub(&t);
                    q = q.add(&Big::from_i64(1).shl(k as u32));
                }
                k -= 1;
            }
        }
        (q, r)
    }

    #[test]
    fn divrem_and_gcd_against_the_oracle() {
        let mut s: u64 = 0x1234_5678_9abc_def1;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let mut k = 0;
        while k < 3000 {
            // magnitudes of 1..600 bits, built from random 63-bit words
            let wa = 1 + (next() % 10) as usize;
            let wb = 1 + (next() % 10) as usize;
            let mut a = Big::zero();
            let mut b = Big::zero();
            let mut i = 0;
            while i < wa {
                a = a.shl(63).add(&Big::from_i64((next() >> 1) as i64));
                i += 1;
            }
            i = 0;
            while i < wb {
                b = b.shl(63).add(&Big::from_i64((next() >> 1) as i64));
                i += 1;
            }
            // sometimes a long run of high limbs, to stress qhat's correction
            if k % 7 == 0 {
                b = Big::from_i64(1).shl(32 * wb as u32).sub(&Big::from_i64(1));
            }
            if b.is_zero() {
                b = Big::from_i64(3);
            }
            let (q, r) = divrem(&a, &b);
            assert_eq!((q.clone(), r.clone()), divmod_slow(&a, &b), "divrem {k}");
            assert_eq!(q.mul(&b).add(&r), a);
            // signs: truncated
            let (qn, rn) = divrem(&a.neg(), &b);
            assert_eq!((qn, rn), (q.neg(), r.neg()));
            // gcd against Euclid by the oracle
            let g = big_gcd(&a, &b);
            let (mut x, mut y) = (a.clone(), b.clone());
            while !y.is_zero() {
                let t = divmod_slow(&x, &y).1;
                x = y;
                y = t;
            }
            assert_eq!(g, x, "gcd {k}");
            k += 1;
        }
        assert_eq!(big_gcd(&Big::from_i64(12), &Big::from_i64(-18)), Big::from_i64(6));
        assert_eq!(big_gcd(&Big::zero(), &Big::zero()), Big::zero());
    }

    #[test]
    fn henrici_against_ratio() {
        let mut st: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            st ^= st << 13;
            st ^= st >> 7;
            st ^= st << 17;
            st
        };
        let mut big = |w: usize| {
            let mut x = Big::from_i64(1 + (next() % 1000) as i64);
            let mut i = 0;
            while i < w {
                x = x.mul(&Big::from_i64(1 + (next() % 4_000_000_000) as i64));
                i += 1;
            }
            if next() % 2 == 0 { x.neg() } else { x }
        };
        let mut k = 0;
        while k < 2000 {
            let w = (k % 9) as usize;
            let a = Q::ratio(&big(w), &big(w).abs());
            let b = Q::ratio(&big(w / 2), &big(w).abs());
            let sum = Q::ratio(&a.n.mul(&b.d).add(&b.n.mul(&a.d)), &a.d.mul(&b.d));
            let prod = Q::ratio(&a.n.mul(&b.n), &a.d.mul(&b.d));
            assert_eq!(a.add(&b), sum, "add {k}");
            assert_eq!(a.mul(&b), prod, "mul {k}");
            assert_eq!(a.add(&a.neg()), Q::zero());
            k += 1;
        }
    }
}
