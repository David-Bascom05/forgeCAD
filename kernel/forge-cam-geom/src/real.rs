//! Exact reals: an expression DAG over exact dyadic inputs with + − × ÷ √, whose sign is decided
//! exactly.
//!
//! The rule, the only one: a sign is read from the node's f64 interval when that excludes zero
//! (the filter, made as each node is built). Otherwise the DAG is evaluated in big-float intervals
//! at doubling precision, until the interval excludes zero, or lies inside the BFMSS separation
//! bound (Burnikel, Fleischer, Mehlhorn, Schirra, Algorithmica 27, 2000): a non-zero value of the
//! expression is at least `1 / (u^(D-1) l)` in magnitude, D = 2^(distinct square roots), so an
//! interval inside that bound proves the value is zero. No epsilon, no tie broken by a guess.
//!
//! Division by a value that is zero is a construction error of the caller (every divisor here is
//! decided non-zero first); a divisor whose interval holds zero only widens the result to the
//! whole line until the precision separates it.
use crate::big::Nat;
use crate::ival::Ival;
use std::cmp::Ordering;

/// An exact dyadic rational: (-1)^neg · mag · 2^exp. Zero has mag = 0, neg = false, exp = 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dy {
    pub neg: bool,
    pub mag: Nat,
    pub exp: i64,
}

impl Dy {
    pub fn zero() -> Dy {
        Dy {
            neg: false,
            mag: Nat::zero(),
            exp: 0,
        }
    }

    fn norm(neg: bool, mag: Nat, exp: i64) -> Dy {
        if mag.is_zero() {
            return Dy::zero();
        }
        // strip trailing zero bits
        let mut k = 0u64;
        while !mag.bit(k) {
            k += 1;
        }
        if k > 0 {
            return Dy {
                neg,
                mag: mag.shr(k),
                exp: exp + k as i64,
            };
        }
        Dy { neg, mag, exp }
    }

    /// The exact value of a finite f64.
    pub fn from_f64(x: f64) -> Dy {
        assert!(x.is_finite(), "non-finite input");
        if x == 0.0 {
            return Dy::zero();
        }
        let b = x.to_bits();
        let neg = b >> 63 == 1;
        let e = ((b >> 52) & 0x7ff) as i64;
        let f = b & ((1u64 << 52) - 1);
        let (m, ex) = if e == 0 {
            (f, -1074)
        } else {
            (f | (1u64 << 52), e - 1075)
        };
        Dy::norm(neg, Nat::from_u64(m), ex)
    }

    pub fn is_zero(&self) -> bool {
        self.mag.is_zero()
    }

    pub fn neg(&self) -> Dy {
        if self.is_zero() {
            return Dy::zero();
        }
        Dy {
            neg: !self.neg,
            mag: self.mag.clone(),
            exp: self.exp,
        }
    }

    fn add_mag(a: &Dy, b: &Dy) -> Dy {
        // same sign assumed by the caller when used for magnitudes
        let e = if a.exp < b.exp { a.exp } else { b.exp };
        let am = a.mag.shl((a.exp - e) as u64);
        let bm = b.mag.shl((b.exp - e) as u64);
        if a.neg == b.neg {
            return Dy::norm(a.neg, am.add(&bm), e);
        }
        match am.cmp(&bm) {
            Ordering::Equal => Dy::zero(),
            Ordering::Greater => Dy::norm(a.neg, am.sub(&bm), e),
            Ordering::Less => Dy::norm(b.neg, bm.sub(&am), e),
        }
    }

    pub fn add(&self, o: &Dy) -> Dy {
        if self.is_zero() {
            return o.clone();
        }
        if o.is_zero() {
            return self.clone();
        }
        Dy::add_mag(self, o)
    }

    pub fn sub(&self, o: &Dy) -> Dy {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &Dy) -> Dy {
        if self.is_zero() || o.is_zero() {
            return Dy::zero();
        }
        Dy::norm(self.neg != o.neg, self.mag.mul(&o.mag), self.exp + o.exp)
    }

    /// Exact halving.
    pub fn half(&self) -> Dy {
        if self.is_zero() {
            return Dy::zero();
        }
        Dy {
            neg: self.neg,
            mag: self.mag.clone(),
            exp: self.exp - 1,
        }
    }

    pub fn cmp(&self, o: &Dy) -> Ordering {
        let d = self.sub(o);
        if d.is_zero() {
            Ordering::Equal
        } else if d.neg {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    /// floor(log2 |x|) + 1 (the magnitude's binary order); for zero, i64::MIN.
    pub fn order(&self) -> i64 {
        if self.is_zero() {
            return i64::MIN;
        }
        self.mag.bits() as i64 + self.exp
    }

    /// Rounded to `p` significant bits, toward -inf (`up` false) or +inf (`up` true).
    pub fn round(&self, p: u64, up: bool) -> Dy {
        let b = self.mag.bits();
        if b <= p {
            return self.clone();
        }
        let k = b - p;
        let t = self.mag.shr(k);
        let lost = self.mag.low_bits_nonzero(k);
        // away from zero when rounding outward on this side
        let away = lost && (up != self.neg);
        let m = if away { t.add(&Nat::from_u64(1)) } else { t };
        Dy::norm(self.neg, m, self.exp + k as i64)
    }

    /// self / o rounded to about `p` bits, toward -inf or +inf. o != 0.
    pub fn div(&self, o: &Dy, p: u64, up: bool) -> Dy {
        if self.is_zero() {
            return Dy::zero();
        }
        let ab = self.mag.bits() as i64;
        let bb = o.mag.bits() as i64;
        let mut s = p as i64 + bb - ab + 2;
        if s < 0 {
            s = 0;
        }
        let (q, r) = self.mag.shl(s as u64).divrem(&o.mag);
        let neg = self.neg != o.neg;
        let away = !r.is_zero() && (up != neg);
        let m = if away { q.add(&Nat::from_u64(1)) } else { q };
        Dy::norm(neg, m, self.exp - s - o.exp)
    }

    /// sqrt(self) for self >= 0, rounded to about `p` bits toward -inf or +inf.
    pub fn sqrt(&self, p: u64, up: bool) -> Dy {
        if self.is_zero() || self.neg {
            return Dy::zero();
        }
        let b = self.mag.bits() as i64;
        let mut s = 2 * p as i64 - b + 2;
        if s < 0 {
            s = 0;
        }
        if (self.exp - s) % 2 != 0 {
            s += 1;
        }
        let m = self.mag.shl(s as u64);
        let q = m.isqrt();
        let exact = q.mul(&q).cmp(&m) == Ordering::Equal;
        let r = if up && !exact { q.add(&Nat::from_u64(1)) } else { q };
        Dy::norm(false, r, (self.exp - s) / 2)
    }

    /// The nearest f64 toward -inf or +inf (saturating to ±inf past the range).
    pub fn to_f64(&self, up: bool) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let r = self.round(53, up);
        let (m, e) = r.mag.top53();
        let ex = e + r.exp;
        let mut v = m as f64;
        // scale by 2^ex exactly where in range
        let mut k = ex;
        while k > 0 {
            let step = if k > 1000 { 1000 } else { k };
            v *= pow2(step);
            k -= step;
        }
        while k < 0 {
            let step = if k < -1000 { 1000 } else { -k };
            // division by a power of two is exact until subnormal; then outward by one ulp
            let w = v / pow2(step);
            v = if w * pow2(step) == v {
                w
            } else if up != self.neg {
                w.next_up()
            } else {
                w.next_down()
            };
            k += step;
        }
        if self.neg { -v } else { v }
    }
}

/// The f64 nearest a dyadic (ties to even, by choosing between the two directed roundings).
/// An exact rational n / d, d > 0 (`Ctx::exact_sign`).
#[derive(Clone, Debug)]
struct Q {
    n: Dy,
    d: Dy,
}

impl Q {
    fn zero() -> Q {
        Q::dy(Dy::zero())
    }
    fn one() -> Q {
        Q::dy(Dy::from_f64(1.0))
    }
    fn dy(n: Dy) -> Q {
        Q {
            n,
            d: Dy::from_f64(1.0),
        }
    }
    fn is_zero(&self) -> bool {
        self.n.is_zero()
    }
    fn sign(&self) -> Ordering {
        if self.n.is_zero() {
            Ordering::Equal
        } else if self.n.neg {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
    fn neg(&self) -> Q {
        Q {
            n: self.n.neg(),
            d: self.d.clone(),
        }
    }
    fn add(&self, o: &Q) -> Q {
        if self.d == o.d {
            return Q {
                n: self.n.add(&o.n),
                d: self.d.clone(),
            };
        }
        Q {
            n: self.n.mul(&o.d).add(&o.n.mul(&self.d)),
            d: self.d.mul(&o.d),
        }
    }
    fn mul(&self, o: &Q) -> Q {
        Q {
            n: self.n.mul(&o.n),
            d: self.d.mul(&o.d),
        }
    }
    /// None for a zero divisor.
    fn div(&self, o: &Q) -> Option<Q> {
        if o.n.is_zero() {
            return None;
        }
        // keep the denominator positive
        let (n, d) = (self.n.mul(&o.d), self.d.mul(&o.n));
        Some(if d.neg {
            Q {
                n: n.neg(),
                d: d.neg(),
            }
        } else {
            Q { n, d }
        })
    }
    fn bits(&self) -> u64 {
        self.n.mag.bits().max(self.d.mag.bits())
    }
}

/// a + b√c, c the one radicand of `Ctx::exact_sign` (b is 0 until its root is met).
#[derive(Clone, Debug)]
struct Quad {
    a: Q,
    b: Q,
}

impl Quad {
    fn rat(a: Q) -> Quad {
        Quad { a, b: Q::zero() }
    }
    fn neg(&self) -> Quad {
        Quad {
            a: self.a.neg(),
            b: self.b.neg(),
        }
    }
    fn add(&self, o: &Quad) -> Quad {
        Quad {
            a: self.a.add(&o.a),
            b: if self.b.is_zero() {
                o.b.clone()
            } else if o.b.is_zero() {
                self.b.clone()
            } else {
                self.b.add(&o.b)
            },
        }
    }
    /// (a1 + b1√c)(a2 + b2√c) = a1 a2 + b1 b2 c + (a1 b2 + a2 b1)√c.
    fn mul(&self, o: &Quad, c: Option<&Q>) -> Quad {
        match c {
            Some(c) if !(self.b.is_zero() && o.b.is_zero()) => Quad {
                a: self.a.mul(&o.a).add(&self.b.mul(&o.b).mul(c)),
                b: self.a.mul(&o.b).add(&o.a.mul(&self.b)),
            },
            _ => Quad::rat(self.a.mul(&o.a)),
        }
    }
    /// x / y = x (a2 − b2√c) / (a2² − b2² c); None where that norm is 0 (y is 0, or √c is
    /// rational and y a multiple of a2 − b2√c's conjugate).
    fn div(&self, o: &Quad, c: Option<&Q>) -> Option<Quad> {
        if o.b.is_zero() {
            return Some(Quad {
                a: self.a.div(&o.a)?,
                b: if self.b.is_zero() {
                    Q::zero()
                } else {
                    self.b.div(&o.a)?
                },
            });
        }
        let c = c?;
        let norm = o.a.mul(&o.a).add(&o.b.mul(&o.b).mul(c).neg());
        let conj = Quad {
            a: o.a.clone(),
            b: o.b.neg(),
        };
        let num = self.mul(&conj, Some(c));
        Some(Quad {
            a: num.a.div(&norm)?,
            b: num.b.div(&norm)?,
        })
    }
    /// The exact sign of a + b√c (√c ≥ 0).
    fn sign(&self, c: Option<&Q>) -> Ordering {
        let (sa, sb) = (self.a.sign(), self.b.sign());
        let sc = c.map_or(Ordering::Equal, |c| c.sign());
        if sb == Ordering::Equal || sc == Ordering::Equal {
            return sa;
        }
        if sa == Ordering::Equal || sa == sb {
            return sb;
        }
        // opposite signs: the term with the larger square wins
        let c = c.expect("a root was met");
        let t = self.a.mul(&self.a).add(&self.b.mul(&self.b).mul(c).neg());
        match t.sign() {
            Ordering::Greater => sa,
            Ordering::Less => sb,
            Ordering::Equal => Ordering::Equal,
        }
    }
    fn bits(&self) -> u64 {
        self.a.bits().max(self.b.bits())
    }
}

fn nearest(d: &Dy) -> f64 {
    let (lo, hi) = (d.to_f64(false), d.to_f64(true));
    if lo == hi {
        return lo;
    }
    let mid = Dy::from_f64(lo).add(&Dy::from_f64(hi)).half();
    match d.cmp(&mid) {
        Ordering::Less => lo,
        Ordering::Greater => hi,
        Ordering::Equal => {
            if lo.to_bits() & 1 == 0 {
                lo
            } else {
                hi
            }
        }
    }
}

fn pow2(k: i64) -> f64 {
    let mut v = 1.0f64;
    let mut i = 0;
    while i < k {
        v *= 2.0;
        i += 1;
    }
    v
}

/// A big-float interval; `None` bounds are infinite.
#[derive(Clone, Debug)]
pub struct BigIval {
    pub lo: Option<Dy>,
    pub hi: Option<Dy>,
}

impl BigIval {
    fn whole() -> BigIval {
        BigIval { lo: None, hi: None }
    }
    fn pt(d: &Dy) -> BigIval {
        BigIval {
            lo: Some(d.clone()),
            hi: Some(d.clone()),
        }
    }
    fn contains_zero(&self) -> bool {
        let lo_ok = match &self.lo {
            None => true,
            Some(d) => d.is_zero() || d.neg,
        };
        let hi_ok = match &self.hi {
            None => true,
            Some(d) => d.is_zero() || !d.neg,
        };
        lo_ok && hi_ok
    }
}

fn badd(a: &BigIval, b: &BigIval, p: u64) -> BigIval {
    let lo = match (&a.lo, &b.lo) {
        (Some(x), Some(y)) => Some(x.add(y).round(p, false)),
        _ => None,
    };
    let hi = match (&a.hi, &b.hi) {
        (Some(x), Some(y)) => Some(x.add(y).round(p, true)),
        _ => None,
    };
    BigIval { lo, hi }
}

fn bneg(a: &BigIval) -> BigIval {
    BigIval {
        lo: a.hi.as_ref().map(Dy::neg),
        hi: a.lo.as_ref().map(Dy::neg),
    }
}

fn bmul(a: &BigIval, b: &BigIval, p: u64) -> BigIval {
    match (&a.lo, &a.hi, &b.lo, &b.hi) {
        (Some(al), Some(ah), Some(bl), Some(bh)) => {
            let c = [al.mul(bl), al.mul(bh), ah.mul(bl), ah.mul(bh)];
            let mut lo = 0usize;
            let mut hi = 0usize;
            let mut i = 1;
            while i < 4 {
                if c[i].cmp(&c[lo]) == Ordering::Less {
                    lo = i;
                }
                if c[i].cmp(&c[hi]) == Ordering::Greater {
                    hi = i;
                }
                i += 1;
            }
            BigIval {
                lo: Some(c[lo].round(p, false)),
                hi: Some(c[hi].round(p, true)),
            }
        }
        _ => BigIval::whole(),
    }
}

fn bdiv(a: &BigIval, b: &BigIval, p: u64) -> BigIval {
    if b.contains_zero() {
        return BigIval::whole();
    }
    match (&a.lo, &a.hi, &b.lo, &b.hi) {
        (Some(al), Some(ah), Some(bl), Some(bh)) => {
            let lo_c = [
                al.div(bl, p, false),
                al.div(bh, p, false),
                ah.div(bl, p, false),
                ah.div(bh, p, false),
            ];
            let hi_c = [
                al.div(bl, p, true),
                al.div(bh, p, true),
                ah.div(bl, p, true),
                ah.div(bh, p, true),
            ];
            let mut lo = 0usize;
            let mut hi = 0usize;
            let mut i = 1;
            while i < 4 {
                if lo_c[i].cmp(&lo_c[lo]) == Ordering::Less {
                    lo = i;
                }
                if hi_c[i].cmp(&hi_c[hi]) == Ordering::Greater {
                    hi = i;
                }
                i += 1;
            }
            BigIval {
                lo: Some(lo_c[lo].clone()),
                hi: Some(hi_c[hi].clone()),
            }
        }
        _ => BigIval::whole(),
    }
}

fn bsqrt(a: &BigIval, p: u64) -> BigIval {
    let lo = match &a.lo {
        None => Some(Dy::zero()),
        Some(d) => Some(if d.neg { Dy::zero() } else { d.sqrt(p, false) }),
    };
    let hi = match &a.hi {
        None => None,
        Some(d) => Some(if d.neg { Dy::zero() } else { d.sqrt(p, true) }),
    };
    BigIval { lo, hi }
}

#[derive(Clone, Debug)]
pub enum Op {
    Lit(Dy),
    Add(u32, u32),
    Sub(u32, u32),
    Mul(u32, u32),
    Div(u32, u32),
    Sqrt(u32),
    Neg(u32),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub op: Op,
    /// The filter: an enclosure of the exact value.
    pub iv: Ival,
    /// log2 of BFMSS's u and l, rounded up.
    lu: f64,
    ll: f64,
}

/// A handle to a node of a `Ctx`.
pub type R = u32;

/// A node's identity, for sharing: equal keys are the same exact value, built once.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Key {
    Lit(bool, Vec<u32>, i64),
    Op(u8, u32, u32),
}

/// The arena of exact reals. Nodes are only appended, so children precede parents; `mark` and
/// `release` drop temporaries in a stack discipline.
///
/// Nodes are shared by an index on each node's key (a hash map with a fixed, unseeded hasher:
/// which node a key finds is the same on every target and run; only lookups go through it, never
/// its order): an expression built twice is one node, and constants fold: an
/// operation on two literals that is exact in dyadics is a literal. So `x − x` is the literal 0
/// by construction, and the separation bound counts each distinct radical once.
#[derive(Clone, Debug, Default)]
pub struct Ctx {
    pub nodes: Vec<Node>,
    /// How many exact (big-float) evaluations were needed: for tracing the filter's rate.
    pub exact_evals: u64,
    memo: Memo,
    keys: Vec<Key>,
}

/// The node memo: key to node, lookups only. It must never be iterated (a hash map's order is
/// not a result to depend on), so it offers get, insert and remove and nothing else.
#[derive(Clone, Debug, Default)]
struct Memo(std::collections::HashMap<Key, R, std::hash::BuildHasherDefault<Fx>>);

impl Memo {
    fn get(&self, k: &Key) -> Option<&R> {
        self.0.get(k)
    }
    fn insert(&mut self, k: Key, r: R) {
        self.0.insert(k, r);
    }
    fn remove(&mut self, k: &Key) {
        self.0.remove(k);
    }
}

/// A fixed (unseeded) multiplicative hasher for the memo.
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);
impl std::hash::Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(5) ^ *b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as u64)
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64)
    }
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }
}

fn upadd(a: f64, b: f64) -> f64 {
    (a + b).next_up()
}

impl Ctx {
    pub fn new() -> Ctx {
        Ctx::default()
    }

    pub fn mark(&self) -> usize {
        self.nodes.len()
    }
    pub fn release(&mut self, m: usize) {
        while self.keys.len() > m {
            let k = self.keys.pop().unwrap();
            self.memo.remove(&k);
        }
        self.nodes.truncate(m);
    }

    fn push(&mut self, key: Key, op: Op, iv: Ival, lu: f64, ll: f64) -> R {
        self.nodes.push(Node { op, iv, lu, ll });
        let r = (self.nodes.len() - 1) as R;
        self.memo.insert(key.clone(), r);
        self.keys.push(key);
        r
    }

    fn find(&self, k: &Key) -> Option<R> {
        self.memo.get(k).copied()
    }

    pub fn iv(&self, a: R) -> Ival {
        self.nodes[a as usize].iv
    }

    /// The literal at a, if it is one.
    pub fn lit_of(&self, a: R) -> Option<&Dy> {
        match &self.nodes[a as usize].op {
            Op::Lit(d) => Some(d),
            _ => None,
        }
    }

    fn is_lit(&self, a: R, v: i64) -> bool {
        match self.lit_of(a) {
            Some(d) => {
                if v == 0 {
                    d.is_zero()
                } else {
                    !d.neg == (v > 0) && d.exp == 0 && d.mag == Nat::from_u64(v.unsigned_abs())
                }
            }
            None => false,
        }
    }

    pub fn dy(&mut self, d: Dy) -> R {
        let d = Dy::norm(d.neg, d.mag, d.exp);
        let key = Key::Lit(d.neg, d.mag.limbs.clone(), d.exp);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let (lu, ll) = if d.is_zero() {
            (0.0, 0.0)
        } else {
            let b = d.mag.bits() as f64;
            if d.exp >= 0 {
                (b + d.exp as f64, 0.0)
            } else {
                (b, -(d.exp as f64))
            }
        };
        let iv = Ival {
            lo: d.to_f64(false),
            hi: d.to_f64(true),
        };
        self.push(key, Op::Lit(d), iv, lu, ll)
    }

    pub fn lit(&mut self, x: f64) -> R {
        self.dy(Dy::from_f64(x))
    }

    fn lits(&self, a: R, b: R) -> Option<(Dy, Dy)> {
        match (self.lit_of(a), self.lit_of(b)) {
            (Some(x), Some(y)) => Some((x.clone(), y.clone())),
            _ => None,
        }
    }

    pub fn add(&mut self, a: R, b: R) -> R {
        if self.is_lit(a, 0) {
            return b;
        }
        if self.is_lit(b, 0) {
            return a;
        }
        if let Some((x, y)) = self.lits(a, b) {
            return self.dy(x.add(&y));
        }
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let key = Key::Op(1, a, b);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let (na, nb) = (&self.nodes[a as usize], &self.nodes[b as usize]);
        let lu = upadd(
            if upadd(na.lu, nb.ll) > upadd(nb.lu, na.ll) {
                upadd(na.lu, nb.ll)
            } else {
                upadd(nb.lu, na.ll)
            },
            1.0,
        );
        let ll = upadd(na.ll, nb.ll);
        let iv = na.iv.add(nb.iv);
        self.push(key, Op::Add(a, b), iv, lu, ll)
    }
    pub fn sub(&mut self, a: R, b: R) -> R {
        if a == b {
            return self.dy(Dy::zero());
        }
        if self.is_lit(b, 0) {
            return a;
        }
        if let Some((x, y)) = self.lits(a, b) {
            return self.dy(x.sub(&y));
        }
        let key = Key::Op(2, a, b);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let (na, nb) = (&self.nodes[a as usize], &self.nodes[b as usize]);
        let lu = upadd(
            if upadd(na.lu, nb.ll) > upadd(nb.lu, na.ll) {
                upadd(na.lu, nb.ll)
            } else {
                upadd(nb.lu, na.ll)
            },
            1.0,
        );
        let ll = upadd(na.ll, nb.ll);
        let iv = na.iv.sub(nb.iv);
        self.push(key, Op::Sub(a, b), iv, lu, ll)
    }
    pub fn mul(&mut self, a: R, b: R) -> R {
        if self.is_lit(a, 0) || self.is_lit(b, 0) {
            return self.dy(Dy::zero());
        }
        if self.is_lit(a, 1) {
            return b;
        }
        if self.is_lit(b, 1) {
            return a;
        }
        if let Some((x, y)) = self.lits(a, b) {
            return self.dy(x.mul(&y));
        }
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let key = Key::Op(3, a, b);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let (na, nb) = (&self.nodes[a as usize], &self.nodes[b as usize]);
        let (lu, ll) = (upadd(na.lu, nb.lu), upadd(na.ll, nb.ll));
        let iv = if a == b { na.iv.sqr() } else { na.iv.mul(nb.iv) };
        self.push(key, Op::Mul(a, b), iv, lu, ll)
    }
    pub fn div(&mut self, a: R, b: R) -> R {
        if self.is_lit(a, 0) {
            return self.dy(Dy::zero());
        }
        if self.is_lit(b, 1) || a == b && self.nodes[a as usize].iv.sign() != 0 {
            if a == b {
                return self.dy(Dy::from_f64(1.0));
            }
            return a;
        }
        if let Some((x, y)) = self.lits(a, b)
            && y.mag == Nat::from_u64(1)
        {
            // a power of two: exact
            let inv = Dy {
                neg: y.neg,
                mag: Nat::from_u64(1),
                exp: -y.exp,
            };
            return self.dy(x.mul(&inv));
        }
        let key = Key::Op(4, a, b);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let (na, nb) = (&self.nodes[a as usize], &self.nodes[b as usize]);
        let (lu, ll) = (upadd(na.lu, nb.ll), upadd(na.ll, nb.lu));
        let iv = na.iv.div(nb.iv);
        self.push(key, Op::Div(a, b), iv, lu, ll)
    }
    /// The square root of a value the caller knows is non-negative.
    pub fn sqrt(&mut self, a: R) -> R {
        if let Some(d) = self.lit_of(a) {
            // a perfect square of a dyadic: exact
            if d.is_zero() {
                return a;
            }
            if !d.neg {
                let (m, e) = if d.exp % 2 == 0 {
                    (d.mag.clone(), d.exp)
                } else {
                    (d.mag.shl(1), d.exp - 1)
                };
                let q = m.isqrt();
                if q.mul(&q) == m {
                    return self.dy(Dy {
                        neg: false,
                        mag: q,
                        exp: e / 2,
                    });
                }
            }
        }
        let key = Key::Op(5, a, 0);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let na = &self.nodes[a as usize];
        let lu = if na.lu > 0.0 { (0.5 * na.lu).next_up() } else { 0.0 };
        let ll = if na.ll > 0.0 { (0.5 * na.ll).next_up() } else { 0.0 };
        let iv = na.iv.sqrt();
        self.push(key, Op::Sqrt(a), iv, lu, ll)
    }
    pub fn neg(&mut self, a: R) -> R {
        if let Some(d) = self.lit_of(a) {
            let n = d.neg();
            return self.dy(n);
        }
        if let Op::Neg(x) = self.nodes[a as usize].op {
            return x;
        }
        let key = Key::Op(6, a, 0);
        if let Some(r) = self.find(&key) {
            return r;
        }
        let na = &self.nodes[a as usize];
        let (lu, ll, iv) = (na.lu, na.ll, na.iv.neg());
        self.push(key, Op::Neg(a), iv, lu, ll)
    }
    pub fn sqr(&mut self, a: R) -> R {
        self.mul(a, a)
    }
    pub fn half(&mut self, a: R) -> R {
        let h = self.lit(0.5);
        self.mul(a, h)
    }

    /// The nodes `a` depends on, in increasing order (children first).
    fn closure(&self, a: R) -> Vec<u32> {
        let n = a as usize + 1;
        let mut need = vec![false; n];
        need[a as usize] = true;
        let mut i = n;
        while i > 0 {
            i -= 1;
            if !need[i] {
                continue;
            }
            match &self.nodes[i].op {
                Op::Lit(_) => {}
                Op::Add(x, y) | Op::Sub(x, y) | Op::Mul(x, y) | Op::Div(x, y) => {
                    need[*x as usize] = true;
                    need[*y as usize] = true;
                }
                Op::Sqrt(x) | Op::Neg(x) => need[*x as usize] = true,
            }
        }
        let mut out = Vec::new();
        let mut j = 0;
        while j < n {
            if need[j] {
                out.push(j as u32);
            }
            j += 1;
        }
        out
    }

    /// The node evaluated in big-float intervals at `p` bits.
    pub fn eval(&self, a: R, p: u64) -> BigIval {
        let order = self.closure(a);
        let mut slot: Vec<Option<BigIval>> = vec![None; a as usize + 1];
        let mut k = 0;
        while k < order.len() {
            let i = order[k] as usize;
            let v = match &self.nodes[i].op {
                Op::Lit(d) => BigIval::pt(d),
                Op::Add(x, y) => badd(
                    slot[*x as usize].as_ref().unwrap(),
                    slot[*y as usize].as_ref().unwrap(),
                    p,
                ),
                Op::Sub(x, y) => {
                    let ny = bneg(slot[*y as usize].as_ref().unwrap());
                    badd(slot[*x as usize].as_ref().unwrap(), &ny, p)
                }
                Op::Mul(x, y) => {
                    if x == y {
                        let s = slot[*x as usize].as_ref().unwrap();
                        let m = bmul(s, s, p);
                        // a square is never negative
                        let lo = match (&m.lo, s.contains_zero()) {
                            (_, true) => Some(Dy::zero()),
                            (Some(l), false) => Some(if l.neg { Dy::zero() } else { l.clone() }),
                            (None, false) => Some(Dy::zero()),
                        };
                        BigIval { lo, hi: m.hi }
                    } else {
                        bmul(
                            slot[*x as usize].as_ref().unwrap(),
                            slot[*y as usize].as_ref().unwrap(),
                            p,
                        )
                    }
                }
                Op::Div(x, y) => bdiv(
                    slot[*x as usize].as_ref().unwrap(),
                    slot[*y as usize].as_ref().unwrap(),
                    p,
                ),
                Op::Sqrt(x) => bsqrt(slot[*x as usize].as_ref().unwrap(), p),
                Op::Neg(x) => bneg(slot[*x as usize].as_ref().unwrap()),
            };
            slot[i] = Some(v);
            k += 1;
        }
        slot[a as usize].take().unwrap()
    }

    /// The number of distinct square-root nodes `a` depends on.
    fn radicals(&self, a: R) -> u32 {
        let order = self.closure(a);
        let mut k = 0;
        let mut c = 0u32;
        while k < order.len() {
            if let Op::Sqrt(_) = self.nodes[order[k] as usize].op {
                c += 1;
            }
            k += 1;
        }
        c
    }

    /// The exact sign of `a`.
    pub fn sign(&mut self, a: R) -> Ordering {
        self.sign_with(a, true)
    }

    /// `sign`, with or without the exact field route (`exact_sign`; the tests compare the two).
    fn sign_with(&mut self, a: R, exact: bool) -> Ordering {
        let s = self.nodes[a as usize].iv.sign();
        if s > 0 {
            return Ordering::Greater;
        }
        if s < 0 {
            return Ordering::Less;
        }
        if let Op::Lit(d) = &self.nodes[a as usize].op {
            return if d.is_zero() {
                Ordering::Equal
            } else if d.neg {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        self.exact_evals += 1;
        // the separation bound, in bits: (D-1) lu + ll
        let k = self.radicals(a);
        let node = &self.nodes[a as usize];
        let d = if k >= 60 {
            f64::INFINITY
        } else {
            (1u64 << k) as f64
        };
        let bits = ((d - 1.0) * node.lu + node.ll).next_up().ceil() + 1.0;
        assert!(
            bits.is_finite() && bits < 1e9,
            "separation bound out of reach ({k} radicals)"
        );
        let need = bits as i64;
        let mut p = 128u64;
        let dbg = std::env::var("FORGE_CAM_DEBUG").is_ok();
        loop {
            if dbg {
                eprintln!("sign: node {a} radicals {k} bound {need} bits, p {p}");
            }
            let v = self.eval(a, p);
            if let Some(lo) = &v.lo
                && !lo.is_zero()
                && !lo.neg
            {
                return Ordering::Greater;
            }
            if let Some(hi) = &v.hi
                && !hi.is_zero()
                && hi.neg
            {
                return Ordering::Less;
            }
            if let (Some(lo), Some(hi)) = (&v.lo, &v.hi) {
                let small = |x: &Dy| x.is_zero() || x.order() <= -need;
                if small(lo) && small(hi) {
                    if dbg && need > 5000 {
                        eprintln!("ZERO {}", std::backtrace::Backtrace::force_capture());
                    }
                    return Ordering::Equal;
                }
            }
            // past the first precision, an expression with at most one distinct square root is
            // decided exactly in its field Q(√c) (`exact_sign`), which gives the same sign the
            // doubling below would reach: most of these are zeros, which the doubling proves only
            // at the separation bound (thousands of bits, each precision evaluated in turn)
            if exact
                && p == 128
                && k <= 1
                && let Some(o) = self.exact_sign(&order, (2 * need).max(4096) as u64)
            {
                return o;
            }
            p *= 2;
            assert!(p < (1u64 << 26), "precision out of reach");
        }
    }

    /// The exact sign of the last node of `order` (a closure, increasing), when its closure has
    /// at most one distinct square root: every value is then a + b√c with a, b, c rational and
    /// √c that one root's value (c ≥ 0: its operand, rational, the only radical below it), and
    /// +, −, ×, ÷ stay in that field exactly. The sign of a + b√c: of a where b or c is 0, of b
    /// where a is 0, their common sign where they agree, else of a's or b's term, whichever has
    /// the larger square (a² against b²c, exactly). None (the caller goes on as before) where a
    /// number grows past `cap` bits, a divisor's norm is 0, or the radicand is negative: never a
    /// wrong sign, only no answer.
    fn exact_sign(&self, order: &[u32], cap: u64) -> Option<Ordering> {
        let mut val: Vec<Quad> = Vec::with_capacity(order.len());
        let at = |x: &u32| order.binary_search(x).expect("an operand is in the closure");
        // the radicand c, once its root is met
        let mut c: Option<Q> = None;
        for &i in order {
            let v = match &self.nodes[i as usize].op {
                Op::Lit(d) => Quad::rat(Q::dy(d.clone())),
                Op::Add(x, y) => val[at(x)].add(&val[at(y)]),
                Op::Sub(x, y) => val[at(x)].add(&val[at(y)].neg()),
                Op::Neg(x) => val[at(x)].neg(),
                Op::Mul(x, y) => val[at(x)].mul(&val[at(y)], c.as_ref()),
                Op::Div(x, y) => val[at(x)].div(&val[at(y)], c.as_ref())?,
                Op::Sqrt(x) => {
                    let r = &val[at(x)];
                    // the one radical: its operand has no root in it, so it is rational
                    if !r.b.is_zero() || c.is_some() || r.a.sign() == Ordering::Less {
                        return None;
                    }
                    c = Some(r.a.clone());
                    Quad {
                        a: Q::zero(),
                        b: Q::one(),
                    }
                }
            };
            if v.bits() > cap {
                return None;
            }
            val.push(v);
        }
        let v = val.pop()?;
        Some(v.sign(c.as_ref()))
    }

    /// sign(a - b), without keeping the difference.
    pub fn cmp(&mut self, a: R, b: R) -> Ordering {
        let (ia, ib) = (self.iv(a), self.iv(b));
        if ia.hi < ib.lo {
            return Ordering::Less;
        }
        if ia.lo > ib.hi {
            return Ordering::Greater;
        }
        let m = self.mark();
        let d = self.sub(a, b);
        let s = self.sign(d);
        self.release(m);
        s
    }

    /// A dyadic strictly between a and b, given a < b (decided exactly by the caller).
    pub fn between(&mut self, a: R, b: R) -> Dy {
        let (ia, ib) = (self.iv(a), self.iv(b));
        if ia.hi < ib.lo {
            let m = Dy::from_f64(ia.hi).add(&Dy::from_f64(ib.lo)).half();
            return m;
        }
        let mut p = 128u64;
        loop {
            let (va, vb) = (self.eval(a, p), self.eval(b, p));
            if let (Some(ah), Some(bl)) = (&va.hi, &vb.lo)
                && ah.cmp(bl) == Ordering::Less
            {
                return ah.add(bl).half();
            }
            p *= 2;
            assert!(p < (1u64 << 26), "between: a < b does not hold");
        }
    }

    /// A close f64 to the exact value (nearest at 2^-60 relative, enough for output).
    pub fn approx(&self, a: R) -> f64 {
        let iv = self.iv(a);
        if iv.lo == iv.hi {
            return iv.lo;
        }
        let mut p = 96u64;
        loop {
            let v = self.eval(a, p);
            if let (Some(l), Some(h)) = (&v.lo, &v.hi) {
                // both ends round to the same nearest f64: that is the answer
                let (a1, b1) = (nearest(l), nearest(h));
                if a1 == b1 || p >= 1024 {
                    return a1;
                }
            }
            if p >= 1024 {
                return iv.mid();
            }
            p *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyadic_exact() {
        for &x in &[0.0, 1.0, -0.1, 3.5, 1e-300, -2.5e300, f64::MIN_POSITIVE / 8.0] {
            let d = Dy::from_f64(x);
            assert_eq!(d.to_f64(false), x);
            assert_eq!(d.to_f64(true), x);
        }
        let s = Dy::from_f64(0.1).add(&Dy::from_f64(0.2));
        assert!(s.to_f64(false) <= 0.30000000000000004 && s.to_f64(true) >= 0.30000000000000004);
    }

    struct Rng(u64);
    impl Rng {
        fn u(&mut self, n: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) % n
        }
    }

    /// The field route (`exact_sign`, at most one distinct root) gives the sign the doubling gives,
    /// on random expressions over small literals and one root, and on zeros built so the filter
    /// cannot see them: (x + y)(x − y) − (x² − y²), (x / y)·y − x, x·√c·√c − x·c, and the norm
    /// (a + b√c)(a − b√c) − (a² − b²c).
    #[test]
    fn the_field_route_agrees_with_the_doubling() {
        let mut g = Rng(3);
        let (mut zeros, mut nonzeros, mut routed) = (0, 0, 0);
        for case in 0..400 {
            let mut c = Ctx::new();
            let lit = |c: &mut Ctx, g: &mut Rng| {
                let v = (g.u(2001) as f64 - 1000.0) / 64.0 + if g.u(2) == 0 { 0.1 } else { 0.0 };
                c.lit(v)
            };
            // the radicand and its root (a square's root folds to a literal: then no radical)
            let cv = {
                let v = (g.u(50) + 1) as f64 / 8.0;
                c.lit(v)
            };
            let rad = c.sqrt(cv);
            let mut pool: Vec<R> = (0..4).map(|_| lit(&mut c, &mut g)).collect();
            if case % 2 == 0 {
                pool.push(rad);
            }
            for _ in 0..8 {
                let (x, y) = (
                    pool[g.u(pool.len() as u64) as usize],
                    pool[g.u(pool.len() as u64) as usize],
                );
                let v = match g.u(4) {
                    0 => c.add(x, y),
                    1 => c.sub(x, y),
                    2 => c.mul(x, y),
                    _ => {
                        if c.sign_with(y, false) == Ordering::Equal {
                            continue;
                        }
                        c.div(x, y)
                    }
                };
                pool.push(v);
            }
            let (x, y) = (pool[pool.len() - 1], pool[pool.len() - 2]);
            // the zeros
            let xpy = c.add(x, y);
            let xmy = c.sub(x, y);
            let pr = c.mul(xpy, xmy);
            let (x2, y2) = (c.sqr(x), c.sqr(y));
            let d2 = c.sub(x2, y2);
            let z1 = c.sub(pr, d2);
            let mut tests = vec![x, y, xpy, z1];
            if c.sign_with(y, false) != Ordering::Equal {
                let q = c.div(x, y);
                let qy = c.mul(q, y);
                tests.push(c.sub(qy, x));
            }
            let xr = c.mul(x, rad);
            let xrr = c.mul(xr, rad);
            let xc = c.mul(x, cv);
            tests.push(c.sub(xrr, xc));
            let br = c.mul(y, rad);
            let (p1, p2) = (c.add(x, br), c.sub(x, br));
            let nm = c.mul(p1, p2);
            let yc = c.mul(y2, cv);
            let n2 = c.sub(x2, yc);
            tests.push(c.sub(nm, n2));
            for t in tests {
                let want = c.sign_with(t, false);
                let order = c.closure(t);
                if c.radicals(&order) <= 1
                    && let Some(got) = c.exact_sign(&order, 1 << 20)
                {
                    routed += 1;
                    assert_eq!(got, want, "case {case}");
                }
                assert_eq!(c.sign(t), want, "case {case}");
                if want == Ordering::Equal {
                    zeros += 1;
                } else {
                    nonzeros += 1;
                }
            }
        }
        assert!(
            zeros > 800 && nonzeros > 800 && routed > 2000,
            "{zeros} {nonzeros} {routed}"
        );
    }

    #[test]
    fn decides_ties_exactly() {
        let mut c = Ctx::new();
        // sqrt(2) * sqrt(2) - 2 == 0
        let two = c.lit(2.0);
        let r = c.sqrt(two);
        let rr = c.mul(r, r);
        let z = c.sub(rr, two);
        assert_eq!(c.sign(z), Ordering::Equal);
        // (sqrt(3) + sqrt(2))^2 - (5 + 2 sqrt 6) == 0, with three radicals
        let three = c.lit(3.0);
        let six = c.lit(6.0);
        let five = c.lit(5.0);
        let (r3, r6) = (c.sqrt(three), c.sqrt(six));
        let s = c.add(r3, r);
        let s2 = c.sqr(s);
        let t = c.mul(two, r6);
        let rhs = c.add(five, t);
        let z2 = c.sub(s2, rhs);
        assert_eq!(c.sign(z2), Ordering::Equal);
        // a near-tie that is not one: sqrt(1e30 + 1) - 1e15 > 0
        let big = c.lit(1e30);
        let one = c.lit(1.0);
        let b1 = c.add(big, one);
        let sb = c.sqrt(b1);
        let e15 = c.lit(1e15);
        let d = c.sub(sb, e15);
        assert_eq!(c.sign(d), Ordering::Greater);
        // 0.1 + 0.2 != 0.3 exactly (as the doubles they are)
        let (a, b, cc) = (c.lit(0.1), c.lit(0.2), c.lit(0.3));
        let ab = c.add(a, b);
        assert_eq!(c.cmp(ab, cc), Ordering::Greater);
        // division
        let third = c.div(one, three);
        let back = c.mul(third, three);
        assert_eq!(c.cmp(back, one), Ordering::Equal);
    }

    #[test]
    fn between_is_strict() {
        let mut c = Ctx::new();
        let two = c.lit(2.0);
        let r = c.sqrt(two);
        let one = c.lit(1.0);
        let e = c.lit(f64::EPSILON);
        let r2 = c.add(r, e);
        let m = c.between(r, r2);
        let mn = c.dy(m);
        assert_eq!(c.cmp(r, mn), Ordering::Less);
        assert_eq!(c.cmp(mn, r2), Ordering::Less);
        let _ = one;
    }
}
