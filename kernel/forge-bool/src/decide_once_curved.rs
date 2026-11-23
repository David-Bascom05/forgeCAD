//! The decide-once boolean for quadric carriers (planes, cylinders, cones, spheres) on
//! formula-carrying bodies: research/decide-once/curved-design.md (D2, D4) and the Rust plan of
//! research/decide-once/curved-l2-plan.md §3. Behind the `decide_once_curved` feature, off by
//! default; nothing here is reached from `boolean.rs` yet, so with the feature off or on every
//! boolean is the old path's, bit for bit.
//!
//! **One to one with the Lean L2** (verify/forge-exact/ForgeExact/DecideOnce): each section below
//! is one Lean module, and each function the Lean definition of the same name, so that L4 can
//! refine one to the other.
//!
//! | Section | Lean | What |
//! |---|---|---|
//! | `QLine` | QLine.lean | `along_line`, `cov_on`: the exact covector of a point on a rational line |
//! | `QSep` | QSep.lean | the Tier-0 seams from a face's frame, and their classes |
//! | `QEdge` | QEdge.lean | edge samples on a plane `x = c`, their on-curve and Bezout certificates, `edge_cov` |
//! | `QPropose` | QPropose.lean | `elim`, `mk_edge`, `propose_edge`, the linear case `elim_lin`/`propose_edge_lin`, `propose_edge_any` (complete, by `roots_of`), the shear |
//! | `QVert` | QVert.lean | vertex samples, `vert_cov`, and `bezout3` (completeness); the proposer is next |
//! | `QForm` | QForm.lean | `SForm` and `on_cov`: a body's sign formula on a covector |
//!
//! Root isolation is `forge_math::exact::alg::roots_of` (RootCover.lean): complete by a checked
//! cover, or `None`. Still to come: the vertex proposer, the cell enumerator (Tier 1 is being
//! redesigned by dol-curved, curved-design.md §2.3), `QSample`, `QLedger`, then the write.
//!
//! **Numbers.** Exact arithmetic is `forge_math::exact::alg` (rationals and `Num` over the one
//! bignum, `Big`). Every untrusted step (resultants, root isolation, cofactors) produces a
//! certificate that a checker here verifies, as the Lean does; a failed check is
//! [`Skip::Uncertified`], spent fuel [`Skip::Budget`], never a wrong sign.
//!
//! **Float filters** ([`FILTERS`]): each sign is first decided on an outward-rounded interval
//! (`forge_math::Interval`, one ulp outward per operation), and only an interval that holds 0
//! goes to the exact path. A filter's answer is the true sign, which is the exact path's answer
//! whenever that one answers (`sign_spec`), so the filter never changes a decision; where the
//! exact path would run out of fuel, the filter's answer stands (strictly more decided).
//!
//! **Style** (research/kernel-audit/decide-once/aeneas/README.md): no struct holds a borrow, no
//! `break`, no `return` inside a loop, no `Ordering`, `while` loops over indices.

use forge_bool_core::Sgn;
use forge_math::Interval;
use forge_math::exact::alg::{
    Coef, Num, Poly, Q, eqv, gcd, is_zero_p, padd, pdiv, pmul, pneg, pscale, psub, roots_of,
};

/// Why a boolean is not decided here, and goes to the old path, counted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skip {
    /// Outside design D4: a carrier that is not a quadric, or a body without a sign formula.
    OutOfDomain(&'static str),
    /// An untrusted step's certificate failed to check (or no certificate could be proposed).
    Uncertified(&'static str),
    /// §6.1 says ambiguous.
    Ambiguous(&'static str),
    /// A decision ran out of its fixed fuel (curved-l2-plan.md §2).
    Budget(&'static str),
}

// =============================================================================================
// Fuel (curved-l2-plan.md §2, QEdge.lean): one fixed constant per call site
// =============================================================================================

/// `signFuel`: `Num.sign`'s bisections, at `cov_on`, `edge_cov`, `ESample::d_ok`, `vert_cov`.
pub const SIGN_FUEL: usize = 200;
/// `rootFuel`: root isolation depth (`QRef.realRoots`' default).
pub const ROOT_FUEL: usize = 60;

// =============================================================================================
// The float filters, listed for the IEEE session (L5)
// =============================================================================================

/// Every float filter on the curved path, with its stated bound. Each is decided on
/// `forge_math::Interval`, whose operations widen each bound by one ulp outward
/// (`Interval::out`); under round-to-nearest the error of one IEEE operation is at most half an
/// ulp, so each enclosure holds the exact value. Preconditions: every bound finite (checked; a
/// non-finite bound sends the decision to the exact path).
pub const FILTERS: &[(&str, &str)] = &[(
    "sign_filter (poly sign at a Num)",
    "q's coefficients and the Num's interval [a, b] (or point c) enclosed by floats checked \
         exactly against the rationals (Q::lo_f64 ≤ v ≤ Q::hi_f64); Horner in Interval; \
         an enclosure excluding 0 is the sign of q on all of [a, b], so at the root",
)];

/// Floats enclosing a rational, checked exactly.
fn enclose(q: &Q) -> Interval {
    Interval {
        lo: q.lo_f64(),
        hi: q.hi_f64(),
    }
}

/// An enclosure of `p` over `x`, by Horner in outward-rounded intervals; `None` if a bound is
/// not finite.
fn horner_iv(p: &[Q], x: &Interval) -> Option<Interval> {
    let mut acc = Interval::point(0.0);
    let mut ok = x.lo.is_finite() && x.hi.is_finite();
    let mut i = p.len();
    while ok && i > 0 {
        i -= 1;
        let c = enclose(&p[i]);
        ok = c.lo.is_finite() && c.hi.is_finite();
        if ok {
            acc = c + *x * acc;
            ok = acc.lo.is_finite() && acc.hi.is_finite();
        }
    }
    if ok { Some(acc) } else { None }
}

/// The sign of `p` at the float `x`, when an enclosure decides it.
fn fsign(p: &[Q], x: f64) -> Option<i32> {
    sign_filter_iv(&Interval::point(x), p)
}

/// Bisections spent tightening a number's enclosure ([`tighten`]).
pub const TIGHTEN_STEPS: usize = 60;

/// Floats enclosing the number. For an isolated root of `p` in `(a, b)` the enclosure is
/// tightened by bisection in floats: `lo`, `hi` with `a ≤ lo < hi ≤ b` at which enclosures give
/// `p` opposite signs hold the root (it is the one root in `(a, b)`); a midpoint whose sign no
/// enclosure decides ends the bisection. No exact arithmetic beyond the endpoints' rounding.
pub fn tighten(n: &Num) -> Interval {
    match n {
        Num::Rat(c) => enclose(c),
        Num::Iso(r) => {
            let outer = Interval {
                lo: r.a.lo_f64(),
                hi: r.b.hi_f64(),
            };
            let mut lo = r.a.hi_f64();
            let mut hi = r.b.lo_f64();
            let sl = fsign(&r.p, lo);
            let sh = fsign(&r.p, hi);
            let good = lo < hi
                && lo.is_finite()
                && hi.is_finite()
                && sl.is_some()
                && sh.is_some()
                && sl == sh.map(|v| -v);
            if !good {
                outer
            } else {
                let s_lo = sl.unwrap_or(0);
                let mut k = 0;
                let mut go = true;
                while go && k < TIGHTEN_STEPS {
                    let m = 0.5 * lo + 0.5 * hi;
                    if !(lo < m && m < hi) {
                        go = false;
                    } else {
                        match fsign(&r.p, m) {
                            Some(s) => {
                                if s == s_lo {
                                    lo = m;
                                } else {
                                    hi = m;
                                }
                            }
                            None => go = false,
                        }
                    }
                    k += 1;
                }
                Interval { lo, hi }
            }
        }
    }
}

/// **The sign filter**: the sign of `q` on the enclosure `x` when an interval evaluation excludes
/// 0, else `None` (the exact path decides).
pub fn sign_filter_iv(x: &Interval, q: &[Q]) -> Option<i32> {
    match horner_iv(q, x) {
        Some(v) => {
            if v.lo > 0.0 {
                Some(1)
            } else if v.hi < 0.0 {
                Some(-1)
            } else {
                None
            }
        }
        None => None,
    }
}

/// The sign filter on the number's untightened enclosure.
pub fn sign_filter(n: &Num, q: &[Q]) -> Option<i32> {
    let x = match n {
        Num::Rat(c) => enclose(c),
        Num::Iso(r) => Interval {
            lo: r.a.lo_f64(),
            hi: r.b.hi_f64(),
        },
    };
    sign_filter_iv(&x, q)
}

/// The sign of `q` at the root `n` denotes, where `t` encloses it: the filter on `t`, then
/// `Num::sign` on `n` with `fuel`.
pub fn sign_at2(n: &Num, t: &Interval, q: &[Q], fuel: usize) -> Option<i32> {
    match sign_filter_iv(t, q) {
        Some(s) => {
            debug_assert!(
                n.sign(q, fuel).is_none_or(|e| e == s),
                "sign filter against the exact sign"
            );
            Some(s)
        }
        None => n.sign(q, fuel),
    }
}

/// The sign of `q` at `n`: the filter, then `Num::sign` with `fuel`.
pub fn sign_at(n: &Num, q: &[Q], fuel: usize) -> Option<i32> {
    sign_at2(n, &tighten(n), q, fuel)
}

// =============================================================================================
// Points, planes, quadrics (Ref/Geom.lean, Quad/Basic.lean)
// =============================================================================================

/// `Ref.V3` over ℚ.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct V3 {
    pub x: Q,
    pub y: Q,
    pub z: Q,
}

impl V3 {
    pub fn new(x: Q, y: Q, z: Q) -> V3 {
        V3 { x, y, z }
    }
    pub fn from_f64(p: [f64; 3]) -> Option<V3> {
        match (Q::from_f64(p[0]), Q::from_f64(p[1]), Q::from_f64(p[2])) {
            (Some(x), Some(y), Some(z)) => Some(V3 { x, y, z }),
            _ => None,
        }
    }
    pub fn add(&self, o: &V3) -> V3 {
        V3::new(self.x.add(&o.x), self.y.add(&o.y), self.z.add(&o.z))
    }
    pub fn sub(&self, o: &V3) -> V3 {
        V3::new(self.x.sub(&o.x), self.y.sub(&o.y), self.z.sub(&o.z))
    }
    pub fn smul(&self, k: &Q) -> V3 {
        V3::new(k.mul(&self.x), k.mul(&self.y), k.mul(&self.z))
    }
    pub fn dot(&self, o: &V3) -> Q {
        self.x.mul(&o.x).add(&self.y.mul(&o.y)).add(&self.z.mul(&o.z))
    }
    pub fn cross(&self, o: &V3) -> V3 {
        V3::new(
            self.y.mul(&o.z).sub(&self.z.mul(&o.y)),
            self.z.mul(&o.x).sub(&self.x.mul(&o.z)),
            self.x.mul(&o.y).sub(&self.y.mul(&o.x)),
        )
    }
    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }
    pub fn norm2(&self) -> Q {
        self.dot(self)
    }
}

/// `Ref.Plane`: `n · x = c`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Plane {
    pub n: V3,
    pub c: Q,
}

impl Plane {
    /// `Plane.eval`: `n · x − c`.
    pub fn eval(&self, x: &V3) -> Q {
        self.n.dot(x).sub(&self.c)
    }
    /// `Plane.canon`: scaled so the first nonzero normal coefficient is 1.
    pub fn canon(&self) -> Plane {
        let s = if !self.n.x.is_zero() {
            self.n.x.clone()
        } else if !self.n.y.is_zero() {
            self.n.y.clone()
        } else {
            self.n.z.clone()
        };
        let k = Q::one().div(&s);
        Plane {
            n: self.n.smul(&k),
            c: self.c.div(&s),
        }
    }
    /// The plane as a quadric (its linear terms).
    pub fn quadric(&self) -> Quadric {
        Quadric {
            lx: self.n.x.clone(),
            ly: self.n.y.clone(),
            lz: self.n.z.clone(),
            c: self.c.neg(),
            ..Quadric::default()
        }
    }
}

/// `Quad.Quadric`: `xx x² + yy y² + zz z² + xy xy + xz xz + yz yz + lx x + ly y + lz z + c`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Quadric {
    pub xx: Q,
    pub yy: Q,
    pub zz: Q,
    pub xy: Q,
    pub xz: Q,
    pub yz: Q,
    pub lx: Q,
    pub ly: Q,
    pub lz: Q,
    pub c: Q,
}

impl Quadric {
    /// `Quadric.eval`.
    pub fn eval(&self, p: &V3) -> Q {
        let (x, y, z) = (&p.x, &p.y, &p.z);
        self.xx
            .mul(x)
            .mul(x)
            .add(&self.yy.mul(y).mul(y))
            .add(&self.zz.mul(z).mul(z))
            .add(&self.xy.mul(x).mul(y))
            .add(&self.xz.mul(x).mul(z))
            .add(&self.yz.mul(y).mul(z))
            .add(&self.lx.mul(x))
            .add(&self.ly.mul(y))
            .add(&self.lz.mul(z))
            .add(&self.c)
    }
    /// `Quadric.grad`.
    pub fn grad(&self, p: &V3) -> V3 {
        let two = Q::from_i64(2);
        V3::new(
            two.mul(&self.xx)
                .mul(&p.x)
                .add(&self.xy.mul(&p.y))
                .add(&self.xz.mul(&p.z))
                .add(&self.lx),
            two.mul(&self.yy)
                .mul(&p.y)
                .add(&self.xy.mul(&p.x))
                .add(&self.yz.mul(&p.z))
                .add(&self.ly),
            two.mul(&self.zz)
                .mul(&p.z)
                .add(&self.xz.mul(&p.x))
                .add(&self.yz.mul(&p.y))
                .add(&self.lz),
        )
    }
    /// `Quadric.quad`: the quadratic part at `d`.
    pub fn quad(&self, d: &V3) -> Q {
        let (x, y, z) = (&d.x, &d.y, &d.z);
        self.xx
            .mul(x)
            .mul(x)
            .add(&self.yy.mul(y).mul(y))
            .add(&self.zz.mul(z).mul(z))
            .add(&self.xy.mul(x).mul(y))
            .add(&self.xz.mul(x).mul(z))
            .add(&self.yz.mul(y).mul(z))
    }
    /// Every coefficient divided by `s`.
    fn scaled(&self, s: &Q) -> Quadric {
        Quadric {
            xx: self.xx.div(s),
            yy: self.yy.div(s),
            zz: self.zz.div(s),
            xy: self.xy.div(s),
            xz: self.xz.div(s),
            yz: self.yz.div(s),
            lx: self.lx.div(s),
            ly: self.ly.div(s),
            lz: self.lz.div(s),
            c: self.c.div(s),
        }
    }
    fn coeffs(&self) -> [&Q; 10] {
        [
            &self.xx, &self.yy, &self.zz, &self.xy, &self.xz, &self.yz, &self.lx, &self.ly, &self.lz, &self.c,
        ]
    }
    /// `QRef.canon`: scaled so the first nonzero coefficient is 1, and the scale's sign.
    pub fn canon(&self) -> (Quadric, bool) {
        let a = self.coeffs();
        let mut k = 0;
        while k < 10 && a[k].is_zero() {
            k += 1;
        }
        if k == 10 {
            (self.clone(), true)
        } else {
            let s = a[k].clone();
            (self.scaled(&s), s.signum() > 0)
        }
    }
}

// =============================================================================================
// Faces (QRef/Faces.lean)
// =============================================================================================

/// `QRef.Kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plane,
    Cyl,
    Cone,
    Sphere,
    Torus,
}

/// `QRef.QFace`: a face's surface as stored (frame, radius, cone slope `s` and `c`), exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QFace {
    pub solid: usize,
    pub index: usize,
    pub kind: Kind,
    pub o: V3,
    pub z: V3,
    pub r: Q,
    pub s: Q,
    pub c: Q,
}

/// `centred`: `(p − o)ᵀ S (p − o) + L · (p − o) + k`.
#[allow(clippy::too_many_arguments)]
pub fn centred(sxx: &Q, syy: &Q, szz: &Q, sxy: &Q, sxz: &Q, syz: &Q, l: &V3, k: &Q, o: &V3) -> Quadric {
    let two = Q::from_i64(2);
    let gx = two.mul(sxx).mul(&o.x).add(&sxy.mul(&o.y)).add(&sxz.mul(&o.z));
    let gy = two.mul(syy).mul(&o.y).add(&sxy.mul(&o.x)).add(&syz.mul(&o.z));
    let gz = two.mul(szz).mul(&o.z).add(&sxz.mul(&o.x)).add(&syz.mul(&o.y));
    let qo = sxx
        .mul(&o.x)
        .mul(&o.x)
        .add(&syy.mul(&o.y).mul(&o.y))
        .add(&szz.mul(&o.z).mul(&o.z))
        .add(&sxy.mul(&o.x).mul(&o.y))
        .add(&sxz.mul(&o.x).mul(&o.z))
        .add(&syz.mul(&o.y).mul(&o.z));
    Quadric {
        xx: sxx.clone(),
        yy: syy.clone(),
        zz: szz.clone(),
        xy: sxy.clone(),
        xz: sxz.clone(),
        yz: syz.clone(),
        lx: l.x.sub(&gx),
        ly: l.y.sub(&gy),
        lz: l.z.sub(&gz),
        c: qo.sub(&l.dot(o)).add(k),
    }
}

impl QFace {
    /// `QFace.quadric`: the solid is where it is negative.
    pub fn quadric(&self) -> Quadric {
        let z = &self.z;
        let zero = Q::zero();
        let zv = V3::default();
        match self.kind {
            Kind::Plane => centred(&zero, &zero, &zero, &zero, &zero, &zero, z, &zero, &self.o),
            Kind::Sphere => {
                let one = Q::one();
                centred(
                    &one,
                    &one,
                    &one,
                    &zero,
                    &zero,
                    &zero,
                    &zv,
                    &self.r.mul(&self.r).neg(),
                    &self.o,
                )
            }
            Kind::Cyl => {
                let n = z.norm2();
                let m2 = Q::from_i64(-2);
                centred(
                    &n.sub(&z.x.mul(&z.x)),
                    &n.sub(&z.y.mul(&z.y)),
                    &n.sub(&z.z.mul(&z.z)),
                    &m2.mul(&z.x).mul(&z.y),
                    &m2.mul(&z.x).mul(&z.z),
                    &m2.mul(&z.y).mul(&z.z),
                    &zv,
                    &self.r.mul(&self.r).mul(&n).neg(),
                    &self.o,
                )
            }
            Kind::Cone => {
                let c2 = self.c.mul(&self.c);
                let s2 = self.s.mul(&self.s);
                let k = c2.add(&s2);
                let m2k = Q::from_i64(-2).mul(&k);
                let lin = z.smul(&Q::from_i64(-2).mul(&self.r).mul(&self.c).mul(&self.s));
                centred(
                    &c2.sub(&k.mul(&z.x).mul(&z.x)),
                    &c2.sub(&k.mul(&z.y).mul(&z.y)),
                    &c2.sub(&k.mul(&z.z).mul(&z.z)),
                    &m2k.mul(&z.x).mul(&z.y),
                    &m2k.mul(&z.x).mul(&z.z),
                    &m2k.mul(&z.y).mul(&z.z),
                    &lin,
                    &self.r.mul(&self.r).mul(&c2).neg(),
                    &self.o,
                )
            }
            Kind::Torus => centred(&zero, &zero, &zero, &zero, &zero, &zero, &zv, &Q::one(), &self.o),
        }
    }

    /// `QFace.nappe`: the cone's nappe, as a quadric negative on it.
    pub fn nappe(&self) -> Option<Quadric> {
        match self.kind {
            Kind::Cone => {
                let zero = Q::zero();
                Some(centred(
                    &zero,
                    &zero,
                    &zero,
                    &zero,
                    &zero,
                    &zero,
                    &self.z.smul(&self.s.neg()),
                    &self.r.mul(&self.c).neg(),
                    &self.o,
                ))
            }
            _ => None,
        }
    }

    /// `QFace.WF`: a nonzero axis for the axial kinds, and a cone's `s ≠ 0`.
    pub fn wf(&self) -> bool {
        let axial = self.kind != Kind::Plane && self.kind != Kind::Sphere;
        (!axial || !self.z.is_zero()) && (self.kind != Kind::Cone || !self.s.is_zero())
    }
}

// =============================================================================================
// QLine.lean: the exact covector of a point on a rational line
// =============================================================================================

/// `QRef.alongLine`: `q` along `P + t D`, as a polynomial in `t`.
pub fn along_line(q: &Quadric, p: &V3, d: &V3) -> Poly {
    vec![q.eval(p), q.grad(p).dot(d), q.quad(d)]
}

/// `sgnOfInt`.
pub fn sgn_of_int(s: i32) -> Sgn {
    if s == 1 {
        Sgn::Pos
    } else if s == -1 {
        Sgn::Neg
    } else {
        Sgn::Zero
    }
}

/// **`covOn`**: the covector of the point `n` names on `P + t D` over `qs`, decided exactly
/// (`None`: fuel spent). `covOn_spec`: it is the point's covector, in every model.
pub fn cov_on(qs: &[Quadric], p: &V3, d: &V3, n: &Num, fuel: usize) -> Option<Vec<Sgn>> {
    let mut out: Vec<Sgn> = Vec::with_capacity(qs.len());
    let t = tighten(n);
    let mut ok = true;
    let mut i = 0;
    while ok && i < qs.len() {
        match sign_at2(n, &t, &along_line(&qs[i], p, d), fuel) {
            Some(s) => out.push(sgn_of_int(s)),
            None => ok = false,
        }
        i += 1;
    }
    if ok { Some(out) } else { None }
}

// =============================================================================================
// QSep.lean: the Tier-0 seams
// =============================================================================================

fn ex() -> V3 {
    V3::new(Q::one(), Q::zero(), Q::zero())
}
fn ey() -> V3 {
    V3::new(Q::zero(), Q::one(), Q::zero())
}
fn ez() -> V3 {
    V3::new(Q::zero(), Q::zero(), Q::one())
}

/// `offAxis`: the first coordinate axis not parallel to `z`.
pub fn off_axis(z: &V3) -> V3 {
    if !z.cross(&ex()).is_zero() {
        ex()
    } else if !z.cross(&ey()).is_zero() {
        ey()
    } else {
        ez()
    }
}

/// `axisPlane o z`: the plane through the axis `o + t z` (`axisPlane_on`).
pub fn axis_plane(o: &V3, z: &V3) -> Plane {
    let n = z.cross(&off_axis(z));
    let c = n.dot(o);
    Plane { n, c }
}

/// `nappePlane`: `s z·(p − o) + r c = 0`, the cone's nappe plane, through the apex.
pub fn nappe_plane(f: &QFace) -> Plane {
    let n = f.z.smul(&f.s);
    let c = n.dot(&f.o).sub(&f.r.mul(&f.c));
    Plane { n, c }
}

/// `centrePlane`: `x = o.x`.
pub fn centre_plane(o: &V3) -> Plane {
    Plane {
        n: ex(),
        c: o.x.clone(),
    }
}

/// `equatorPlane`: `z·(p − o) = 0`.
pub fn equator_plane(o: &V3, z: &V3) -> Plane {
    Plane {
        n: z.clone(),
        c: z.dot(o),
    }
}

/// `seamsOf`: a face's Tier-0 seams.
pub fn seams_of(f: &QFace) -> Vec<Plane> {
    match f.kind {
        Kind::Plane => Vec::new(),
        Kind::Cyl => vec![axis_plane(&f.o, &f.z)],
        Kind::Cone => vec![axis_plane(&f.o, &f.z), nappe_plane(f)],
        Kind::Sphere => vec![centre_plane(&f.o)],
        Kind::Torus => vec![axis_plane(&f.o, &f.z), equator_plane(&f.o, &f.z)],
    }
}

/// `classesOf` (Classes.lean): keep a plane iff no kept plane has its canonical form.
pub fn classes_of(ps: &[Plane]) -> Vec<Plane> {
    let mut acc: Vec<Plane> = Vec::new();
    let mut keys: Vec<Plane> = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        let k = ps[i].canon();
        let mut seen = false;
        let mut j = 0;
        while !seen && j < keys.len() {
            seen = keys[j] == k;
            j += 1;
        }
        if !seen {
            acc.push(ps[i].clone());
            keys.push(k);
        }
        i += 1;
    }
    acc
}

/// `seams`: the seams of all faces, one per point set (`noScaled_seams`).
pub fn seams(fs: &[QFace]) -> Vec<Plane> {
    let mut all: Vec<Plane> = Vec::new();
    let mut i = 0;
    while i < fs.len() {
        let s = seams_of(&fs[i]);
        let mut j = 0;
        while j < s.len() {
            all.push(s[j].clone());
            j += 1;
        }
        i += 1;
    }
    classes_of(&all)
}

// =============================================================================================
// QEdge.lean: edge samples on a plane x = c
// =============================================================================================

/// `restr q c`: `q` on `x = c` as `a₀(y) + a₁(y) z + a₂(y) z²`.
pub fn restr(q: &Quadric, c: &Q) -> (Poly, Poly, Poly) {
    (
        vec![
            q.xx.mul(c).mul(c).add(&q.lx.mul(c)).add(&q.c),
            q.xy.mul(c).add(&q.ly),
            q.yy.clone(),
        ],
        vec![q.xz.mul(c).add(&q.lz), q.yz.clone()],
        vec![q.zz.clone()],
    )
}

/// `homog q c N D = D² · q(c, y, −N/D)`, a polynomial in `y` (`homog_ev`).
pub fn homog(q: &Quadric, c: &Q, n: &[Q], d: &[Q]) -> Poly {
    let (a0, a1, a2) = restr(q, c);
    psub(
        &padd(&pmul(&pmul(d, d), &a0), &pmul(&pmul(n, n), &a2)),
        &pmul(&pmul(d, n), &a1),
    )
}

/// `ESample`: a point of `q_j = q_k = 0` on `x = c`: `y = α` a root of `R`, `z = −N(α)/D(α)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ESample {
    pub c: Q,
    pub r: Poly,
    pub n: Poly,
    pub d: Poly,
    /// the root `α` of `R`
    pub y: Num,
    pub qj: Poly,
    pub qk: Poly,
}

impl ESample {
    /// `ESample.onCurve`: `homog q_j = R Q_j` and `homog q_k = R Q_k` (`onCurve_spec`).
    pub fn on_curve(&self, qj: &Quadric, qk: &Quadric) -> bool {
        eqv(&homog(qj, &self.c, &self.n, &self.d), &pmul(&self.r, &self.qj))
            && eqv(&homog(qk, &self.c, &self.n, &self.d), &pmul(&self.r, &self.qk))
    }

    /// `ESample.dOk`: `D(α) ≠ 0`, decided.
    pub fn d_ok(&self, fuel: usize) -> bool {
        match sign_at2(&self.y, &tighten(&self.y), &self.d, fuel) {
            Some(s) => s != 0,
            None => false,
        }
    }
}

/// **`edgeCov`**: the sample's covector over `qs` (`None`: fuel spent). `edgeCov_spec`.
pub fn edge_cov(qs: &[Quadric], e: &ESample, fuel: usize) -> Option<Vec<Sgn>> {
    let mut out: Vec<Sgn> = Vec::with_capacity(qs.len());
    let t = tighten(&e.y);
    let mut ok = true;
    let mut i = 0;
    while ok && i < qs.len() {
        match sign_at2(&e.y, &t, &homog(&qs[i], &e.c, &e.n, &e.d), fuel) {
            Some(s) => out.push(sgn_of_int(s)),
            None => ok = false,
        }
        i += 1;
    }
    if ok { Some(out) } else { None }
}

/// `Alg.BPoly`: a polynomial in `z` with coefficients in `ℚ[y]` (`YPoly`'s zero is `[]`, as the
/// list instance of `Coef`).
pub type BPoly = Vec<Poly>;

/// `bzero`: every coefficient of every coefficient is zero.
pub fn bzero(s: &[Poly]) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i < s.len() {
        ok = is_zero_p(&s[i]);
        i += 1;
    }
    ok
}

/// `restrB q c`: `q` on `x = c`, as a polynomial in `z` over `ℚ[y]`.
pub fn restr_b(q: &Quadric, c: &Q) -> BPoly {
    let (a0, a1, a2) = restr(q, c);
    vec![a0, a1, a2]
}

/// `liftY R`: a polynomial in `y` as a bivariate constant.
pub fn lift_y(r: &[Q]) -> BPoly {
    vec![r.to_vec()]
}

/// **`bezout`**: the certificate `A q_j + B q_k = R` on `x = c` (`bezout_spec`: every crossing of
/// the curve with the plane is a root of `R`).
pub fn bezout(qj: &Quadric, qk: &Quadric, c: &Q, a: &[Poly], b: &[Poly], r: &[Q]) -> bool {
    bzero(&psub(
        &padd(&pmul(a, &restr_b(qj, c)), &pmul(b, &restr_b(qk, c))),
        &lift_y(r),
    ))
}

// =============================================================================================
// QPropose.lean: the edge proposer, complete, and the shear
// =============================================================================================

/// `Elim`: the elimination data for `q_j`, `q_k` on `x = c`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elim {
    pub r: Poly,
    pub n: Poly,
    pub d: Poly,
    pub u: BPoly,
    pub v: BPoly,
}

/// `elim qj qk c`: the resultant `R`, the subresultant `D z + N`, and the Bezout cofactors.
pub fn elim(qj: &Quadric, qk: &Quadric, c: &Q) -> Elim {
    elim_of(&restr(qj, c), &restr(qk, c))
}

/// `elim` on two restrictions `f = a₀ + a₁t + a₂t²`, `g = b₀ + b₁t + b₂t²` with coefficients in
/// `ℚ[s]`: the resultant in `t`, the subresultant `D t + N`, and the cofactors `U f + V g = R`.
pub fn elim_of(a: &(Poly, Poly, Poly), b: &(Poly, Poly, Poly)) -> Elim {
    let e = elim_gen(a, b);
    Elim {
        r: e.r,
        n: e.n,
        d: e.d,
        u: e.u,
        v: e.v,
    }
}

/// `elim` over any coefficient ring (`Poly` for samples on a plane, `BPoly` = `ℚ[x][y]` for the
/// three-quadric vertex): the resultant in `t`, `D t + N`, and `U f + V g = R`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElimG<T> {
    pub r: T,
    pub n: T,
    pub d: T,
    pub u: Vec<T>,
    pub v: Vec<T>,
}

fn cm<T: Coef>(p: &T, q: &T) -> T {
    p.c_mul(q)
}
fn cs<T: Coef>(p: &T, q: &T) -> T {
    p.c_add(&q.c_neg())
}
fn ca<T: Coef>(p: &T, q: &T) -> T {
    p.c_add(q)
}

/// The closed forms of QPropose's `elim`, over `T`.
pub fn elim_gen<T: Coef>(a: &(T, T, T), b: &(T, T, T)) -> ElimG<T> {
    let (a0, a1, a2) = (&a.0, &a.1, &a.2);
    let (b0, b1, b2) = (&b.0, &b.1, &b.2);
    let x = cs(&cm(a2, b0), &cm(a0, b2));
    let y = cs(&cm(a2, b1), &cm(a1, b2));
    let z = cs(&cm(a1, b0), &cm(a0, b1));
    let u0 = ca(
        &cs(
            &cs(&cm(a0, &cm(b2, b2)), &cm(a1, &cm(b1, b2))),
            &cm(a2, &cm(b0, b2)),
        ),
        &cm(a2, &cm(b1, b1)),
    );
    let u1 = ca(&cm(a1, &cm(b2, b2)).c_neg(), &cm(a2, &cm(b1, b2)));
    let v0 = ca(
        &cs(
            &ca(&cm(a0, &cm(a2, b2)).c_neg(), &cm(a1, &cm(a1, b2))),
            &cm(a1, &cm(a2, b1)),
        ),
        &cm(a2, &cm(a2, b0)),
    );
    let v1 = cs(&cm(a1, &cm(a2, b2)), &cm(a2, &cm(a2, b1)));
    ElimG {
        r: cs(&cm(&x, &x), &cm(&y, &z)),
        n: cs(&cm(b2, a0), &cm(a2, b0)),
        d: cs(&cm(b2, a1), &cm(a2, b1)),
        u: vec![u0, u1],
        v: vec![v0, v1],
    }
}

/// `mkEdge`: the sample for the root `n` (quotients `a₂`, `b₂`: `homog q_j = a₂ R`).
pub fn mk_edge(qj: &Quadric, qk: &Quadric, c: &Q, e: &Elim, n: &Num) -> ESample {
    ESample {
        c: c.clone(),
        r: e.r.clone(),
        n: e.n.clone(),
        d: e.d.clone(),
        y: n.clone(),
        qj: restr(qj, c).2,
        qk: restr(qk, c).2,
    }
}

/// **`proposeEdge qj qk c fuel`**: every crossing of `q_j = q_k = 0` with `x = c`, each
/// certified (`bezout`, `rootsOf`, `onCurve`, `dOk`), or `None` (`Skip::Budget`).
/// `proposeEdge_complete`: every crossing is a proposed sample's point.
pub fn propose_edge(qj: &Quadric, qk: &Quadric, c: &Q, fuel: usize) -> Option<Vec<ESample>> {
    let e = elim(qj, qk, c);
    if !bezout(qj, qk, c, &e.u, &e.v, &e.r) {
        None
    } else {
        match roots_of(&e.r, fuel) {
            None => None,
            Some(ns) => {
                let mut es: Vec<ESample> = Vec::with_capacity(ns.len());
                let mut ok = true;
                let mut i = 0;
                while i < ns.len() {
                    let s = mk_edge(qj, qk, c, &e, &ns[i]);
                    ok = ok && s.on_curve(qj, qk) && s.d_ok(SIGN_FUEL);
                    es.push(s);
                    i += 1;
                }
                if ok { Some(es) } else { None }
            }
        }
    }
}

/// `elimLin qj qk c`: both quadrics without `z²`, so `q_j = a₁z + a₀`, `q_k = b₁z + b₀` on the
/// plane, `b₁ q_j − a₁ q_k = R := b₁a₀ − a₁b₀`, and `z = −a₀/a₁`.
pub fn elim_lin(qj: &Quadric, qk: &Quadric, c: &Q) -> Elim {
    let a = restr(qj, c);
    let b = restr(qk, c);
    Elim {
        r: psub(&pmul(&b.1, &a.0), &pmul(&a.1, &b.0)),
        n: a.0.clone(),
        d: a.1.clone(),
        u: vec![b.1.clone()],
        v: vec![pneg(&a.1)],
    }
}

/// `proposeEdgeLin`: the linear case's crossings, each certified, or `None`
/// (`proposeEdgeLin_complete`).
pub fn propose_edge_lin(qj: &Quadric, qk: &Quadric, c: &Q, fuel: usize) -> Option<Vec<ESample>> {
    let e = elim_lin(qj, qk, c);
    if !bezout(qj, qk, c, &e.u, &e.v, &e.r) {
        None
    } else {
        match roots_of(&e.r, fuel) {
            None => None,
            Some(ns) => {
                let qk_q = pneg(&restr(qj, c).1);
                let mut es: Vec<ESample> = Vec::with_capacity(ns.len());
                let mut ok = true;
                let mut i = 0;
                while i < ns.len() {
                    let s = ESample {
                        c: c.clone(),
                        r: e.r.clone(),
                        n: e.n.clone(),
                        d: e.d.clone(),
                        y: ns[i].clone(),
                        qj: Vec::new(),
                        qk: qk_q.clone(),
                    };
                    ok = ok && s.on_curve(qj, qk) && s.d_ok(SIGN_FUEL);
                    es.push(s);
                    i += 1;
                }
                if ok { Some(es) } else { None }
            }
        }
    }
}

/// **`proposeEdgeAny`**: the proposer for any pair, linear when neither has `z²`, solving for
/// `z` from a quadric with a `z` term on the plane (`proposeEdgeAny_complete`).
pub fn propose_edge_any(qj: &Quadric, qk: &Quadric, c: &Q, fuel: usize) -> Option<Vec<ESample>> {
    if qj.zz.is_zero() && qk.zz.is_zero() {
        if is_zero_p(&restr(qj, c).1) {
            propose_edge_lin(qk, qj, c, fuel)
        } else {
            propose_edge_lin(qj, qk, c, fuel)
        }
    } else {
        propose_edge(qj, qk, c, fuel)
    }
}

/// `shears`: the generic projections tried, in order; the decided rule is the first `λ` for
/// which every proposal of the boolean succeeds, and past the list `Skip::Budget`.
pub fn shears() -> Vec<Q> {
    vec![
        Q::from_i64(0),
        Q::from_i64(1),
        Q::from_i64(2),
        Q::from_i64(3),
        Q::from_i64(-1),
        Q::one().div(&Q::from_i64(2)),
        Q::from_i64(5),
        Q::from_i64(-3),
        Q::from_i64(7),
        Q::one().div(&Q::from_i64(3)),
    ]
}

impl Quadric {
    /// `Quadric.shear l`: `q(x, y + λz, z)` (`shear_ev`).
    pub fn shear(&self, l: &Q) -> Quadric {
        let two = Q::from_i64(2);
        Quadric {
            xx: self.xx.clone(),
            yy: self.yy.clone(),
            zz: self.zz.add(&l.mul(l).mul(&self.yy)).add(&l.mul(&self.yz)),
            xy: self.xy.clone(),
            xz: self.xz.add(&l.mul(&self.xy)),
            yz: self.yz.add(&two.mul(l).mul(&self.yy)),
            lx: self.lx.clone(),
            ly: self.ly.clone(),
            lz: self.lz.add(&l.mul(&self.ly)),
            c: self.c.clone(),
        }
    }
}

// =============================================================================================
// QVert.lean: vertex samples
// =============================================================================================

/// `homog3 q = (D_y D_z)² q(x, −N_y/D_y, −N_z/D_z)`, a polynomial in `x` (`homog3_ev`).
pub fn homog3(q: &Quadric, ny: &[Q], dy: &[Q], nz: &[Q], dz: &[Q]) -> Poly {
    let x: Poly = vec![Q::zero(), Q::one()];
    let y2 = pmul(dy, dy);
    let z2 = pmul(dz, dz);
    let yz = pmul(dy, dz);
    let terms: Vec<Poly> = vec![
        pscale(&q.xx, &pmul(&pmul(&x, &x), &pmul(&y2, &z2))),
        pscale(&q.yy, &pmul(&pmul(ny, ny), &z2)),
        pscale(&q.zz, &pmul(&pmul(nz, nz), &y2)),
        pscale(&q.xy.neg(), &pmul(&pmul(&x, ny), &pmul(dy, &z2))),
        pscale(&q.xz.neg(), &pmul(&pmul(&x, nz), &pmul(dz, &y2))),
        pscale(&q.yz, &pmul(&pmul(ny, nz), &yz)),
        pscale(&q.lx, &pmul(&x, &pmul(&y2, &z2))),
        pscale(&q.ly.neg(), &pmul(ny, &pmul(dy, &z2))),
        pscale(&q.lz.neg(), &pmul(nz, &pmul(dz, &y2))),
        pscale(&q.c, &pmul(&y2, &z2)),
    ];
    // terms.foldr padd []
    let mut acc: Poly = Vec::new();
    let mut i = terms.len();
    while i > 0 {
        i -= 1;
        acc = padd(&terms[i], &acc);
    }
    acc
}

/// `VSample`: `x = α` a root of `R`, `y = −N_y(α)/D_y(α)`, `z = −N_z(α)/D_z(α)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VSample {
    pub r: Poly,
    pub ny: Poly,
    pub dy: Poly,
    pub nz: Poly,
    pub dz: Poly,
    /// the root `α` of `R`
    pub x: Num,
    pub qa: Poly,
    pub qb: Poly,
    pub qc: Poly,
}

impl VSample {
    /// `VSample.onVert` (`onVert_spec`: all three quadrics vanish at the sample).
    pub fn on_vert(&self, qa: &Quadric, qb: &Quadric, qc: &Quadric) -> bool {
        eqv(
            &homog3(qa, &self.ny, &self.dy, &self.nz, &self.dz),
            &pmul(&self.r, &self.qa),
        ) && eqv(
            &homog3(qb, &self.ny, &self.dy, &self.nz, &self.dz),
            &pmul(&self.r, &self.qb),
        ) && eqv(
            &homog3(qc, &self.ny, &self.dy, &self.nz, &self.dz),
            &pmul(&self.r, &self.qc),
        )
    }

    /// `VSample.dOk`: `D_y(α) D_z(α) ≠ 0`, decided.
    pub fn d_ok(&self, fuel: usize) -> bool {
        match sign_at2(&self.x, &tighten(&self.x), &pmul(&self.dy, &self.dz), fuel) {
            Some(s) => s != 0,
            None => false,
        }
    }
}

/// **`vertCov`**: the vertex sample's covector over `qs` (`None`: fuel spent). `vertCov_spec`.
pub fn vert_cov(qs: &[Quadric], v: &VSample, fuel: usize) -> Option<Vec<Sgn>> {
    let mut out: Vec<Sgn> = Vec::with_capacity(qs.len());
    let t = tighten(&v.x);
    let mut ok = true;
    let mut i = 0;
    while ok && i < qs.len() {
        match sign_at2(&v.x, &t, &homog3(&qs[i], &v.ny, &v.dy, &v.nz, &v.dz), fuel) {
            Some(s) => out.push(sgn_of_int(s)),
            None => ok = false,
        }
        i += 1;
    }
    if ok { Some(out) } else { None }
}

/// `TPoly`: a polynomial in `z` over `ℚ[x][y]` (`ZC` is a `BPoly` whose inner variable is `x`).
pub type TPoly = Vec<BPoly>;

/// `restrT q`: `q` as a polynomial in `z` over `ℚ[x][y]`.
pub fn restr_t(q: &Quadric) -> TPoly {
    vec![
        vec![
            vec![q.c.clone(), q.lx.clone(), q.xx.clone()],
            vec![q.ly.clone(), q.xy.clone()],
            vec![q.yy.clone()],
        ],
        vec![vec![q.lz.clone(), q.xz.clone()], vec![q.yz.clone()]],
        vec![vec![q.zz.clone()]],
    ]
}

/// `liftR R`: a polynomial in `x` as a trivariate constant.
pub fn lift_r(r: &[Q]) -> TPoly {
    vec![vec![r.to_vec()]]
}

/// `tzero`: every coefficient is zero.
pub fn tzero(t: &[BPoly]) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i < t.len() {
        ok = bzero(&t[i]);
        i += 1;
    }
    ok
}

/// **`bezout3`**: the certificate `A q_a + B q_b + C q_c = R(x)` (`bezout3_spec`: every common
/// zero of the three has its `x` a root of `R`).
#[allow(clippy::too_many_arguments)]
pub fn bezout3(
    qa: &Quadric,
    qb: &Quadric,
    qc: &Quadric,
    a: &[BPoly],
    b: &[BPoly],
    c: &[BPoly],
    r: &[Q],
) -> bool {
    tzero(&psub(
        &padd(
            &padd(&pmul(a, &restr_t(qa)), &pmul(b, &restr_t(qb))),
            &pmul(c, &restr_t(qc)),
        ),
        &lift_r(r),
    ))
}

// =============================================================================================
// The vertex proposer, one plane and two quadrics (untrusted; checked by `bezout3`, `roots_of`,
// `on_vert`, `d_ok`). Drafted here for dol-curved's Lean `proposeVert`.
// =============================================================================================

/// The elimination data for a vertex: `R(x)`, `y = −N_y/D_y`, `z = −N_z/D_z`, and the cofactors
/// `A q_a + B q_b + C P = R` (`bezout3`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElimV {
    pub r: Poly,
    pub ny: Poly,
    pub dy: Poly,
    pub nz: Poly,
    pub dz: Poly,
    pub a: TPoly,
    pub b: TPoly,
    pub c: TPoly,
}

impl Quadric {
    /// `q(x, z, y)`: the roles of `y` and `z` exchanged.
    pub fn swap_yz(&self) -> Quadric {
        Quadric {
            xx: self.xx.clone(),
            yy: self.zz.clone(),
            zz: self.yy.clone(),
            xy: self.xz.clone(),
            xz: self.xy.clone(),
            yz: self.yz.clone(),
            lx: self.lx.clone(),
            ly: self.lz.clone(),
            lz: self.ly.clone(),
            c: self.c.clone(),
        }
    }
}

/// A polynomial in `z` over `ℚ[x][y]` with `y` and `z` exchanged.
pub fn tswap_yz(t: &[BPoly]) -> TPoly {
    let mut ny = 0;
    let mut k = 0;
    while k < t.len() {
        if t[k].len() > ny {
            ny = t[k].len();
        }
        k += 1;
    }
    let mut out: TPoly = Vec::with_capacity(ny);
    let mut j = 0;
    while j < ny {
        let mut row: BPoly = Vec::with_capacity(t.len());
        let mut k2 = 0;
        while k2 < t.len() {
            if j < t[k2].len() {
                row.push(t[k2][j].clone());
            } else {
                row.push(Vec::new());
            }
            k2 += 1;
        }
        out.push(row);
        j += 1;
    }
    out
}

/// `t(x, y, Z(x, y))` for `t` a polynomial in `z` over `ℚ[x][y]` and `Z` a polynomial in `y`
/// over `ℚ[x]`: Horner in `z`.
pub fn subst_z(t: &[BPoly], zl: &[Poly]) -> BPoly {
    let mut acc: BPoly = Vec::new();
    let mut i = t.len();
    while i > 0 {
        i -= 1;
        acc = padd(&t[i], &pmul(zl, &acc));
    }
    acc
}

/// The quotient `K` of a quadric `t = t₀ + t₁z + t₂z²` by `P = p₀ + n_z z`, with `inv = 1/n_z`:
/// `k₁ = t₂/n_z`, `k₀ = (t₁ − k₁p₀)/n_z`, so `t = K P + (t₀ − k₀p₀)`.
fn quot_plane(t: &[BPoly], p0: &[Poly], inv: &Q) -> TPoly {
    let sc: Poly = vec![inv.clone()];
    let k1 = pscale(&sc, &t[2]);
    let k0 = pscale(&sc, &psub(&t[1], &pmul(&k1, p0)));
    vec![k0, k1]
}

/// Trailing zero coefficients dropped.
fn btrim(b: &[Poly]) -> BPoly {
    let mut n = b.len();
    while n > 0 && is_zero_p(&b[n - 1]) {
        n -= 1;
    }
    b[..n].to_vec()
}

fn coef_or_empty(b: &[Poly], i: usize) -> Poly {
    if i < b.len() { b[i].clone() } else { Vec::new() }
}

/// `elimVert` for a plane with `n.z ≠ 0`: `z = (c − n.x x − n.y y)/n.z` substituted, the two
/// conics eliminated in `y` (`elim_of`), and the cofactors lifted through the quotients by `P`:
/// `q = K P + f` (division in `z` by `P`, whose `z` coefficient is the constant `n.z`), so
/// `R = U f_a + V f_b = U q_a + V q_b − (U K_a + V K_b) P`.
fn elim_vert_z(pl: &Plane, qa: &Quadric, qb: &Quadric) -> Option<ElimV> {
    let nzc = pl.n.z.clone();
    if nzc.is_zero() {
        None
    } else {
        let inv = Q::one().div(&nzc);
        // Z(x, y) = c/nz − (nx/nz) x − (ny/nz) y, as a polynomial in y over Q[x]
        let zl: BPoly = vec![
            vec![pl.c.mul(&inv), pl.n.x.neg().mul(&inv)],
            vec![pl.n.y.neg().mul(&inv)],
        ];
        let ta = restr_t(qa);
        let tb = restr_t(qb);
        let fa = btrim(&subst_z(&ta, &zl));
        let fb = btrim(&subst_z(&tb, &zl));
        if fa.len() > 3 || fb.len() > 3 {
            None
        } else {
            let a3 = (
                coef_or_empty(&fa, 0),
                coef_or_empty(&fa, 1),
                coef_or_empty(&fa, 2),
            );
            let b3 = (
                coef_or_empty(&fb, 0),
                coef_or_empty(&fb, 1),
                coef_or_empty(&fb, 2),
            );
            let e = elim_of(&a3, &b3);
            // P = p0 + nz z, p0 = nx x + ny y − c (the plane's quadric)
            let p0 = restr_t(&pl.quadric())[0].clone();
            let ka = quot_plane(&ta, &p0, &inv);
            let kb = quot_plane(&tb, &p0, &inv);
            let u: TPoly = vec![e.u.clone()];
            let v: TPoly = vec![e.v.clone()];
            let c = pneg(&padd(&pmul(&u, &ka), &pmul(&v, &kb)));
            // z = [(c − nx x) D + ny N] / (nz D)
            let cx: Poly = vec![pl.c.clone(), pl.n.x.neg()];
            let nz_p = pneg(&padd(&pmul(&cx, &e.d), &pscale(&pl.n.y, &e.n)));
            let dz_p = pscale(&nzc, &e.d);
            Some(ElimV {
                r: e.r,
                ny: e.n,
                dy: e.d,
                nz: nz_p,
                dz: dz_p,
                a: u,
                b: v,
                c,
            })
        }
    }
}

/// **`elimVert`**: a plane with `n.z ≠ 0` directly; with `n.z = 0 ≠ n.y`, the same with `y` and
/// `z` exchanged and mapped back; a plane `x = const` is not proposed (the shear rule).
pub fn elim_vert(pl: &Plane, qa: &Quadric, qb: &Quadric) -> Option<ElimV> {
    if !pl.n.z.is_zero() {
        elim_vert_z(pl, qa, qb)
    } else if !pl.n.y.is_zero() {
        let sp = Plane {
            n: V3::new(pl.n.x.clone(), pl.n.z.clone(), pl.n.y.clone()),
            c: pl.c.clone(),
        };
        match elim_vert_z(&sp, &qa.swap_yz(), &qb.swap_yz()) {
            None => None,
            Some(e) => Some(ElimV {
                r: e.r,
                ny: e.nz,
                dy: e.dz,
                nz: e.ny,
                dz: e.dy,
                a: tswap_yz(&e.a),
                b: tswap_yz(&e.b),
                c: tswap_yz(&e.c),
            }),
        }
    } else {
        None
    }
}

/// **`proposeVert`**: every common point of the plane and the two quadrics, each certified, or
/// `None` (`Skip::Budget`). Checked on the inputs as given: `bezout3` (completeness), `roots_of`
/// (every root of `R`), and per sample `on_vert` and `d_ok`.
pub fn propose_vert(pl: &Plane, qa: &Quadric, qb: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    let pq = pl.quadric();
    match elim_vert(pl, qa, qb) {
        None => None,
        Some(e) => {
            if !bezout3(qa, qb, &pq, &e.a, &e.b, &e.c, &e.r) {
                None
            } else {
                match roots_of(&e.r, fuel) {
                    None => None,
                    Some(ns) => {
                        let ha = homog3(qa, &e.ny, &e.dy, &e.nz, &e.dz);
                        let hb = homog3(qb, &e.ny, &e.dy, &e.nz, &e.dz);
                        let hc = homog3(&pq, &e.ny, &e.dy, &e.nz, &e.dz);
                        let (wa, wb, wc) = (pdiv(&ha, &e.r), pdiv(&hb, &e.r), pdiv(&hc, &e.r));
                        let mut vs: Vec<VSample> = Vec::with_capacity(ns.len());
                        let mut ok = true;
                        let mut i = 0;
                        while i < ns.len() {
                            let v = VSample {
                                r: e.r.clone(),
                                ny: e.ny.clone(),
                                dy: e.dy.clone(),
                                nz: e.nz.clone(),
                                dz: e.dz.clone(),
                                x: ns[i].clone(),
                                qa: wa.clone(),
                                qb: wb.clone(),
                                qc: wc.clone(),
                            };
                            ok = ok && v.on_vert(qa, qb, &pq) && v.d_ok(SIGN_FUEL);
                            vs.push(v);
                            i += 1;
                        }
                        if ok { Some(vs) } else { None }
                    }
                }
            }
        }
    }
}

// =============================================================================================
// The vertex proposer, three quadrics (untrusted; checked by `bezout3` twice, a gcd identity,
// `roots_of`, `on_vert`, `d_ok`). Drafted here for dol-curved's Lean mirror.
// =============================================================================================

/// `q` as a quadratic in `z` over `ℚ[x][y]`.
fn ztriple(q: &Quadric) -> (BPoly, BPoly, BPoly) {
    let t = restr_t(q);
    (t[0].clone(), t[1].clone(), t[2].clone())
}

/// The `y`-polynomial `y^k p`.
fn yshift(k: usize, p: &[Poly]) -> BPoly {
    let mut out: BPoly = Vec::with_capacity(k + p.len());
    let mut i = 0;
    while i < k {
        out.push(Vec::new());
        i += 1;
    }
    let mut j = 0;
    while j < p.len() {
        out.push(p[j].clone());
        j += 1;
    }
    out
}

/// Every coefficient divided by `c` (exact by construction; anything else fails a later check).
fn bdiv(p: &[Poly], c: &[Q]) -> BPoly {
    let mut out: BPoly = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        out.push(pdiv(&p[i], c));
        i += 1;
    }
    out
}

/// The gcd (in `ℚ[x]`) of every coefficient of `ps`.
fn content(ps: &[&BPoly]) -> Poly {
    let mut g: Poly = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        let mut j = 0;
        while j < ps[i].len() {
            if !is_zero_p(&ps[i][j]) {
                g = if g.is_empty() {
                    ps[i][j].clone()
                } else {
                    gcd(&g, &ps[i][j])
                };
            }
            j += 1;
        }
        i += 1;
    }
    g
}

/// One projection: `R(x)` with the cofactors of `A q_a + B q_b + C q_c = R`, and the last
/// remainder of `y`-degree 1, `p₁ y + p₀`, when the sequence has one.
#[derive(Clone, Debug)]
pub struct Chain {
    pub r: Poly,
    pub a: TPoly,
    pub b: TPoly,
    pub c: TPoly,
    pub lin: Option<(Poly, Poly)>,
    /// `D z + N` from the elimination of `z` between `q_a` and `q_b`, over `ℚ[x][y]`
    pub dz: BPoly,
    pub nz: BPoly,
}

/// Steps the remainder sequence may take (the `y`-degree is at most 4, so 8 is ample).
pub const CHAIN_STEPS: usize = 8;

/// **The chain**: `R₁ = Res_z(q_a, q_b) = U₁q_a + V₁q_b` and `R₂ = Res_z(q_a, q_c) = U₂q_a + V₂q_c`
/// over `ℚ[x][y]` (`elim_gen`), then a pseudo-remainder sequence in `y` that tracks cofactors,
/// `f = s R₁ + t R₂` (each step `f ← lc(g) f − lc(f) yᵏ g`, the common content in `ℚ[x]` divided
/// out), down to `R = S R₁ + T R₂` free of `y`. So `A = S U₁ + T U₂`, `B = S V₁`, `C = T V₂`.
/// `None` if a remainder vanishes (`R₁`, `R₂` share a factor: a common curve).
pub fn chain(qa: &Quadric, qb: &Quadric, qc: &Quadric) -> Option<Chain> {
    let ea = elim_gen(&ztriple(qa), &ztriple(qb));
    let eb = elim_gen(&ztriple(qa), &ztriple(qc));
    let r1 = btrim(&ea.r);
    let r2 = btrim(&eb.r);
    if r1.is_empty() || r2.is_empty() {
        return None;
    }
    let one: BPoly = vec![vec![Q::one()]];
    let (mut f, mut sf, mut tf, mut g, mut sg, mut tg) = if r1.len() >= r2.len() {
        (r1, one.clone(), Vec::new(), r2, Vec::new(), one)
    } else {
        (r2, Vec::new(), one.clone(), r1, one, Vec::new())
    };
    let mut ok = true;
    let mut it = 0;
    while ok && g.len() > 1 && it < CHAIN_STEPS {
        // reduce f by g
        let mut k2 = 0;
        while !f.is_empty() && f.len() >= g.len() && k2 < 16 {
            let k = f.len() - g.len();
            let lf = f[f.len() - 1].clone();
            let lg = g[g.len() - 1].clone();
            f = btrim(&psub(&pscale(&lg, &f), &pscale(&lf, &yshift(k, &g))));
            sf = psub(&pscale(&lg, &sf), &pscale(&lf, &yshift(k, &sg)));
            tf = psub(&pscale(&lg, &tf), &pscale(&lf, &yshift(k, &tg)));
            k2 += 1;
        }
        let c = content(&[&f, &sf, &tf]);
        if deg_p(&c) > 0 {
            f = bdiv(&f, &c);
            sf = bdiv(&sf, &c);
            tf = bdiv(&tf, &c);
        }
        if f.is_empty() {
            ok = false;
        } else {
            let (f0, s0, t0) = (g, sg, tg);
            g = f;
            sg = sf;
            tg = tf;
            f = f0;
            sf = s0;
            tf = t0;
        }
        it += 1;
    }
    if !ok || g.len() != 1 {
        None
    } else {
        let lin = if f.len() == 2 {
            Some((f[0].clone(), f[1].clone()))
        } else {
            None
        };
        let s1: TPoly = vec![sg];
        let t1: TPoly = vec![tg];
        Some(Chain {
            r: g[0].clone(),
            a: padd(&pmul(&s1, &ea.u), &pmul(&t1, &eb.u)),
            b: pmul(&s1, &ea.v),
            c: pmul(&t1, &eb.v),
            lin,
            dz: btrim(&ea.d),
            nz: btrim(&ea.n),
        })
    }
}

/// `deg` over ℚ (local name, to keep the import list short).
fn deg_p(p: &[Q]) -> usize {
    forge_math::exact::alg::deg(p)
}

/// `F(x, −p₀/p₁) p₁^m` for a `y`-polynomial `F` of degree `≤ m`.
fn hom_y(f: &[Poly], p0: &[Q], p1: &[Q], m: usize) -> Poly {
    let mp0 = pneg(p0);
    let mut acc: Poly = Vec::new();
    let mut i = 0;
    while i < f.len() {
        let mut t = f[i].clone();
        let mut k = 0;
        while k < i {
            t = pmul(&t, &mp0);
            k += 1;
        }
        while k < m {
            t = pmul(&t, p1);
            k += 1;
        }
        acc = padd(&acc, &t);
        i += 1;
    }
    acc
}

/// **`proposeVert3`**: every common point of three quadrics, each certified, or `None`.
/// - Projection A on the quadrics as given, projection B after the first shear `λ ≠ 0` of
///   `shears()` whose chain exists. A shear keeps `x`, so a vertex's `x` is a root of both `R_A`
///   and `R_B` (`bezout3` on each; B's on the sheared quadrics, by `shear_ev`).
/// - `G = gcd(R_A, R_B)`, checked as `G = u R_A + v R_B`: every vertex's `x` is a root of `G`.
/// - Samples on `roots_of(sqfree G)`: `y = −p₀/p₁` from A's linear remainder, `z = −N(x, y)/D(x, y)`
///   from A's `z`-elimination, homogenised; each checked by `on_vert` and `d_ok`.
pub fn propose_vert3(qa: &Quadric, qb: &Quadric, qc: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    let ca_ = chain(qa, qb, qc)?;
    if !bezout3(qa, qb, qc, &ca_.a, &ca_.b, &ca_.c, &ca_.r) {
        return None;
    }
    let ls = shears();
    let mut cb: Option<Chain> = None;
    let mut i = 1;
    while cb.is_none() && i < ls.len() {
        let (sa, sb, sc) = (qa.shear(&ls[i]), qb.shear(&ls[i]), qc.shear(&ls[i]));
        if let Some(c) = chain(&sa, &sb, &sc)
            && bezout3(&sa, &sb, &sc, &c.a, &c.b, &c.c, &c.r)
        {
            cb = Some(c);
        }
        i += 1;
    }
    let cb = cb?;
    let (g, u, v) = forge_math::exact::alg::xgcd(&ca_.r, &cb.r);
    if !eqv(&g, &padd(&pmul(&u, &ca_.r), &pmul(&v, &cb.r))) {
        return None;
    }
    if is_zero_p(&g) {
        return None;
    }
    if deg_p(&g) == 0 {
        return Some(Vec::new());
    }
    let (p0, p1) = ca_.lin.clone()?;
    let sp = forge_math::exact::alg::propose_sqfree(&g)?;
    let ns = roots_of(&sp, fuel)?;
    let m = if ca_.dz.len() >= ca_.nz.len() {
        ca_.dz.len()
    } else {
        ca_.nz.len()
    };
    let m = if m == 0 { 0 } else { m - 1 };
    let nz = hom_y(&ca_.nz, &p0, &p1, m);
    let dz = hom_y(&ca_.dz, &p0, &p1, m);
    let wa = pdiv(&homog3(qa, &p0, &p1, &nz, &dz), &sp);
    let wb = pdiv(&homog3(qb, &p0, &p1, &nz, &dz), &sp);
    let wc = pdiv(&homog3(qc, &p0, &p1, &nz, &dz), &sp);
    let mut vs: Vec<VSample> = Vec::with_capacity(ns.len());
    let mut ok = true;
    let mut k = 0;
    while k < ns.len() {
        let v = VSample {
            r: sp.clone(),
            ny: p0.clone(),
            dy: p1.clone(),
            nz: nz.clone(),
            dz: dz.clone(),
            x: ns[k].clone(),
            qa: wa.clone(),
            qb: wb.clone(),
            qc: wc.clone(),
        };
        ok = ok && v.on_vert(qa, qb, qc) && v.d_ok(SIGN_FUEL);
        vs.push(v);
        k += 1;
    }
    if ok { Some(vs) } else { None }
}

// =============================================================================================
// QForm.lean: a body's sign formula
// =============================================================================================

/// `SForm`: a sign formula over carrier indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SForm {
    Lit(usize, Sgn),
    Tt,
    Ff,
    And(Box<SForm>, Box<SForm>),
    Or(Box<SForm>, Box<SForm>),
    Not(Box<SForm>),
}

impl SForm {
    /// `SForm.onCov`: the formula's value on a covector (`den_cell`: on its cell).
    pub fn on_cov(&self, x: &[Sgn]) -> bool {
        match self {
            SForm::Lit(j, s) => *j < x.len() && x[*j] == *s,
            SForm::Tt => true,
            SForm::Ff => false,
            SForm::And(a, b) => a.on_cov(x) && b.on_cov(x),
            SForm::Or(a, b) => a.on_cov(x) || b.on_cov(x),
            SForm::Not(a) => !a.on_cov(x),
        }
    }

    /// `SForm.Below n`: every literal's index is below `n`.
    pub fn below(&self, n: usize) -> bool {
        match self {
            SForm::Lit(j, _) => *j < n,
            SForm::Tt | SForm::Ff => true,
            SForm::And(a, b) | SForm::Or(a, b) => a.below(n) && b.below(n),
            SForm::Not(a) => a.below(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::exact::Big;
    use forge_math::exact::alg::{Iso, poly_i};

    fn q(n: i64, d: i64) -> Q {
        Q::ratio(&Big::from_i64(n), &Big::from_i64(d))
    }
    fn qi(n: i64) -> Q {
        Q::from_i64(n)
    }
    fn quad(c: [i64; 10]) -> Quadric {
        Quadric {
            xx: qi(c[0]),
            yy: qi(c[1]),
            zz: qi(c[2]),
            xy: qi(c[3]),
            xz: qi(c[4]),
            yz: qi(c[5]),
            lx: qi(c[6]),
            ly: qi(c[7]),
            lz: qi(c[8]),
            c: qi(c[9]),
        }
    }
    // QEdge.lean's #guard data
    fn sphere1() -> Quadric {
        quad([1, 1, 1, 0, 0, 0, 0, 0, 0, -1])
    }
    fn plane_z() -> Quadric {
        quad([0, 0, 0, 0, 0, 0, 0, 0, 1, 0])
    }
    fn plane_yh() -> Quadric {
        Quadric {
            ly: qi(1),
            c: q(-1, 2),
            ..Quadric::default()
        }
    }

    /// QEdge.lean's `#guard`s.
    #[test]
    fn qedge_guards() {
        let e = ESample {
            c: qi(0),
            r: poly_i(&[-1, 0, 1]),
            n: poly_i(&[0]),
            d: poly_i(&[1]),
            y: Num::Rat(qi(1)),
            qj: poly_i(&[1]),
            qk: poly_i(&[0]),
        };
        assert!(e.on_curve(&sphere1(), &plane_z()));
        assert!(e.d_ok(SIGN_FUEL));
        assert_eq!(
            edge_cov(&[sphere1(), plane_z(), plane_yh()], &e, SIGN_FUEL),
            Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Pos])
        );
        let a: BPoly = vec![poly_i(&[1])];
        let b: BPoly = vec![Vec::new(), poly_i(&[-1])];
        assert!(bezout(
            &sphere1(),
            &plane_z(),
            &qi(0),
            &a,
            &b,
            &poly_i(&[-1, 0, 1])
        ));
    }

    /// QVert.lean's `#guard`s.
    #[test]
    fn qvert_guards() {
        let plane_x1 = quad([0, 0, 0, 0, 0, 0, 1, 0, 0, -1]);
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let v = VSample {
            r: poly_i(&[-1, 1]),
            ny: poly_i(&[0]),
            dy: poly_i(&[1]),
            nz: poly_i(&[0]),
            dz: poly_i(&[1]),
            x: Num::Rat(qi(1)),
            qa: poly_i(&[1, 1]),
            qb: poly_i(&[0]),
            qc: poly_i(&[0]),
        };
        assert!(v.on_vert(&sphere1(), &plane_y0, &plane_z()));
        assert!(v.d_ok(SIGN_FUEL));
        assert_eq!(
            vert_cov(
                &[sphere1(), plane_y0, plane_z(), plane_x1, plane_yh()],
                &v,
                SIGN_FUEL
            ),
            Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero, Sgn::Zero, Sgn::Neg])
        );
    }

    /// QPropose.lean's #guards, and the covectors of what it proposes.
    #[test]
    fn qpropose_guards() {
        let es = propose_edge(&sphere1(), &plane_z(), &qi(0), 30).unwrap();
        assert_eq!(es.len(), 2);
        let mut i = 0;
        while i < es.len() {
            assert_eq!(
                edge_cov(&[sphere1(), plane_z()], &es[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // the cylinder x² + z² = 1/4: unsheared, two crossings share a y, refused
        let cyl_y = Quadric {
            xx: qi(1),
            zz: qi(1),
            c: q(-1, 4),
            ..Quadric::default()
        };
        assert!(propose_edge(&sphere1(), &cyl_y, &qi(0), 30).is_none());
        // the linear case: y = 1/2 and z = 0 on x = 0 cross once
        let es = propose_edge_any(&plane_yh(), &plane_z(), &qi(0), 30).unwrap();
        assert_eq!(es.len(), 1);
        assert_eq!(
            edge_cov(&[plane_yh(), plane_z(), sphere1()], &es[0], SIGN_FUEL),
            Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Neg])
        );
        assert!(propose_edge(&plane_yh(), &plane_z(), &qi(0), 30).is_none());
        let l = qi(1);
        let (s1, c1) = (sphere1().shear(&l), cyl_y.shear(&l));
        let es = propose_edge(&s1, &c1, &qi(0), 30).unwrap();
        assert_eq!(es.len(), 4);
        let mut i = 0;
        while i < es.len() {
            assert_eq!(
                edge_cov(&[s1.clone(), c1.clone()], &es[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // x = 1/3 on the sphere and z = 0: y = ±√(8/9), sides of y = 1/2 differ
        let es = propose_edge(&sphere1(), &plane_z(), &q(1, 3), 30).unwrap();
        assert_eq!(es.len(), 2);
        assert!(edge_cov(&[plane_yh()], &es[0], SIGN_FUEL) != edge_cov(&[plane_yh()], &es[1], SIGN_FUEL));
    }

    /// QVert.lean's bezout3 #guard: x² − 1 = 1·sphere + (−y)·y + (−z)·z.
    #[test]
    fn qvert_bezout3() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let a: TPoly = vec![vec![poly_i(&[1])]];
        let b: TPoly = vec![vec![Vec::new(), poly_i(&[-1])]];
        let c: TPoly = vec![vec![], vec![poly_i(&[-1])]];
        assert!(bezout3(
            &sphere1(),
            &plane_y0,
            &plane_z(),
            &a,
            &b,
            &c,
            &poly_i(&[-1, 0, 1])
        ));
        assert!(!bezout3(
            &sphere1(),
            &plane_y0,
            &plane_z(),
            &a,
            &b,
            &c,
            &poly_i(&[-2, 0, 1])
        ));
    }

    /// The vertex proposer: a plane and two quadrics, directly and with y, z exchanged.
    #[test]
    fn propose_vert_plane() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let pz = Plane {
            n: V3::new(qi(0), qi(0), qi(1)),
            c: qi(0),
        };
        // the plane z = 0, the sphere and y = 0: (±1, 0, 0)
        let vs = propose_vert(&pz, &sphere1(), &plane_y0, 30).unwrap();
        assert_eq!(vs.len(), 2);
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), plane_y0.clone(), plane_z()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // a tilted plane x + y + 2z = 1/2, the sphere and a cylinder along z: generic
        let tilt = Plane {
            n: V3::new(qi(1), qi(1), qi(2)),
            c: q(1, 2),
        };
        let cyl = QFace {
            solid: 0,
            index: 0,
            kind: Kind::Cyl,
            o: V3::new(q(1, 4), q(1, 8), qi(0)),
            z: V3::new(qi(0), qi(0), qi(1)),
            r: q(3, 4),
            s: qi(0),
            c: qi(0),
        }
        .quadric();
        let vs = propose_vert(&tilt, &sphere1(), &cyl, 60).unwrap();
        assert!(!vs.is_empty());
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), cyl.clone(), tilt.quadric()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // n.z = 0: the plane y = 1/2 (exchanged), the sphere and z = 0: (±√3/2, 1/2, 0)
        let py = Plane {
            n: V3::new(qi(0), qi(1), qi(0)),
            c: q(1, 2),
        };
        let vs = propose_vert(&py, &sphere1(), &plane_z(), 30).unwrap();
        assert_eq!(vs.len(), 2);
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), plane_z(), plane_yh()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // x = const is not proposed (the shear rule takes it)
        let px = Plane {
            n: V3::new(qi(1), qi(0), qi(0)),
            c: qi(0),
        };
        assert!(propose_vert(&px, &sphere1(), &plane_z(), 30).is_none());
    }

    /// The three-quadric vertex proposer: two spheres and a cylinder.
    #[test]
    fn propose_vert_three() {
        let sph = |o: V3, r: Q| {
            QFace {
                solid: 0,
                index: 0,
                kind: Kind::Sphere,
                o,
                z: V3::default(),
                r,
                s: qi(0),
                c: qi(0),
            }
            .quadric()
        };
        let s0 = sphere1();
        let s1 = sph(V3::new(q(1, 2), q(1, 3), q(1, 5)), qi(1));
        let cyl = QFace {
            solid: 0,
            index: 0,
            kind: Kind::Cyl,
            o: V3::new(q(1, 4), q(1, 8), qi(0)),
            z: V3::new(qi(0), qi(0), qi(1)),
            r: q(3, 4),
            s: qi(0),
            c: qi(0),
        }
        .quadric();
        let t0 = std::time::Instant::now();
        let vs = propose_vert3(&s0, &s1, &cyl, 60);
        if std::env::var("VDBG").is_ok() {
            println!("{:?} in {:?}", vs.as_ref().map(|v| v.len()), t0.elapsed());
            if let Some(c) = chain(&s0, &s1, &cyl) {
                println!("R_A degree {}", forge_math::exact::alg::deg(&c.r));
            }
        }
        let vs = vs.unwrap();
        assert!(!vs.is_empty());
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[s0.clone(), s1.clone(), cyl.clone()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
    }

    /// QLine: the covector along a line through the unit sphere.
    #[test]
    fn qline_cov_on() {
        // the x axis: sphere's restriction t² − 1, roots ±1; at t = √2 (root of t² − 2) outside
        let p = V3::default();
        let d = V3::new(qi(1), qi(0), qi(0));
        let r2 = Num::Iso(Iso {
            p: poly_i(&[-2, 0, 1]),
            a: qi(1),
            b: qi(2),
        });
        assert_eq!(
            cov_on(&[sphere1(), plane_z()], &p, &d, &r2, SIGN_FUEL),
            Some(vec![Sgn::Pos, Sgn::Zero])
        );
        assert_eq!(
            cov_on(&[sphere1()], &p, &d, &Num::Rat(qi(1)), SIGN_FUEL),
            Some(vec![Sgn::Zero])
        );
    }

    /// QSep: seams hold the axis and the centre; coaxial faces share their seam class.
    #[test]
    fn qsep_seams() {
        let o = V3::new(qi(1), qi(2), qi(3));
        let z = V3::new(qi(0), qi(0), qi(2));
        let ap = axis_plane(&o, &z);
        assert!(ap.eval(&o).is_zero() && ap.eval(&o.add(&z.smul(&qi(5)))).is_zero());
        assert!(!ap.n.is_zero());
        let cyl = |r: Q| QFace {
            solid: 0,
            index: 0,
            kind: Kind::Cyl,
            o: o.clone(),
            z: z.clone(),
            r,
            s: qi(0),
            c: qi(0),
        };
        let other = QFace {
            o: o.add(&z),
            z: z.smul(&qi(-3)),
            ..cyl(qi(2))
        };
        assert_eq!(seams(&[cyl(qi(1)), cyl(qi(2)), other]).len(), 1);
        let cone = QFace {
            kind: Kind::Cone,
            s: qi(1),
            c: qi(1),
            r: qi(1),
            ..cyl(qi(1))
        };
        // the nappe plane is the nappe quadric's zero set, negated
        let np = nappe_plane(&cone);
        let nq = cone.nappe().unwrap();
        let pt = V3::new(qi(7), q(-1, 3), qi(5));
        assert_eq!(nq.eval(&pt), np.eval(&pt).neg());
        assert!(centre_plane(&o).eval(&o).is_zero());
        assert!(
            cone.wf()
                && !QFace {
                    s: qi(0),
                    ..cone.clone()
                }
                .wf()
        );
    }

    #[test]
    fn qform_on_cov() {
        let f = SForm::And(
            Box::new(SForm::Lit(0, Sgn::Neg)),
            Box::new(SForm::Not(Box::new(SForm::Lit(1, Sgn::Neg)))),
        );
        assert!(f.on_cov(&[Sgn::Neg, Sgn::Pos]));
        assert!(!f.on_cov(&[Sgn::Neg, Sgn::Neg]));
        assert!(f.below(2) && !f.below(1));
    }

    #[test]
    fn sign_filter_agrees() {
        let r2 = Num::Iso(Iso {
            p: poly_i(&[-2, 0, 1]),
            a: qi(1),
            b: qi(2),
        });
        // x − 1 is 0 at the interval's end, so only the exact path decides it; 2x − 1 is not
        assert_eq!(sign_filter(&r2, &poly_i(&[-1, 1])), None);
        assert_eq!(sign_at(&r2, &poly_i(&[-1, 1]), SIGN_FUEL), Some(1));
        assert_eq!(sign_filter(&r2, &poly_i(&[-1, 2])), Some(1));
        assert_eq!(sign_filter(&r2, &poly_i(&[-2, 0, 1])), None);
        assert_eq!(sign_at(&r2, &poly_i(&[-2, 0, 1]), SIGN_FUEL), Some(0));
        assert_eq!(sign_filter(&Num::Rat(q(1, 3)), &poly_i(&[-1, 3])), None);
        assert_eq!(sign_at(&Num::Rat(q(1, 3)), &poly_i(&[-1, 3]), SIGN_FUEL), Some(0));
    }
}

/// The committed digest of the exact path over a fixed family (interim L4, design D6: the numeric
/// parts are checked by a differential digest until Aeneas takes bignums). The same test runs
/// natively and under wasm32-wasip1 (tools/test-wasm.sh), so it is also the native = wasm check.
#[cfg(test)]
mod digest {
    use super::*;
    use forge_math::exact::Big;

    fn fnv(h: u64, s: &str) -> u64 {
        let mut h = h;
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            h ^= b[i] as u64;
            h = h.wrapping_mul(0x100000001b3);
            i += 1;
        }
        h
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> i64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) % 9) as i64 - 4
        }
    }

    fn q(n: i64, d: i64) -> Q {
        Q::ratio(&Big::from_i64(n), &Big::from_i64(d))
    }

    /// A small quadric family: spheres, cylinders and cones on small dyadic frames, and planes.
    fn family(g: &mut Lcg) -> Vec<Quadric> {
        let mut out: Vec<Quadric> = Vec::new();
        let mut i = 0;
        while i < 12 {
            let o = V3::new(q(g.next(), 4), q(g.next(), 4), q(g.next(), 4));
            let mut z = V3::new(
                Q::from_i64(g.next()),
                Q::from_i64(g.next()),
                Q::from_i64(g.next()),
            );
            if z.is_zero() {
                z = V3::new(Q::zero(), Q::zero(), Q::one());
            }
            let kind = match i % 4 {
                0 => Kind::Sphere,
                1 => Kind::Cyl,
                2 => Kind::Cone,
                _ => Kind::Plane,
            };
            let f = QFace {
                solid: 0,
                index: i,
                kind,
                o,
                z,
                r: q(g.next().abs() + 1, 2),
                s: Q::one(),
                c: q(1, 2),
            };
            out.push(f.quadric());
            i += 1;
        }
        out
    }

    #[test]
    fn exact_digest() {
        let mut g = Lcg(0x5eed);
        let qs = family(&mut g);
        let mut h: u64 = 0xcbf29ce484222325;
        let mut i = 0;
        while i < qs.len() {
            let mut j = i + 1;
            while j < qs.len() {
                let c = q(g.next(), 8);
                let s = match propose_edge_any(&qs[i], &qs[j], &c, ROOT_FUEL) {
                    Some(es) => {
                        let mut t = format!("{}:", es.len());
                        let mut k = 0;
                        while k < es.len() {
                            let cov = edge_cov(&qs, &es[k], SIGN_FUEL);
                            t = format!("{t}{:?}{:?};", es[k].y, cov);
                            k += 1;
                        }
                        t
                    }
                    None => "none".to_string(),
                };
                h = fnv(h, &s);
                j += 1;
            }
            i += 1;
        }
        // root isolation over random cubics and quartics
        let mut k = 0;
        while k < 40 {
            let deg = 3 + (k % 2) as usize;
            let mut p: Poly = Vec::new();
            let mut t = 0;
            while t <= deg {
                p.push(Q::from_i64(g.next()));
                t += 1;
            }
            let rs = roots_of(&p, ROOT_FUEL);
            h = fnv(h, &format!("{rs:?}"));
            k += 1;
        }
        println!("exact digest {h:#018x}");
        assert_eq!(h, DIGEST, "the exact path's digest changed: {h:#018x}");
    }

    const DIGEST: u64 = 0xe2c8e3e43408b4a8;
}
