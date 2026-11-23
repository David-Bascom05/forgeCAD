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
//! Root isolation is `forge_alg::roots_of` (RootCover.lean): complete by a checked
//! cover, or `None`. Still to come: the vertex proposer, the cell enumerator (Tier 1 is being
//! redesigned by dol-curved, curved-design.md §2.3), `QSample`, `QLedger`, then the write.
//!
//! **Numbers.** Exact arithmetic is `forge_alg` (rationals and `Num` over the one
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

use forge_alg::{
    Coef, Num, Poly, Q, eqv, gcd, is_zero_p, padd, pdiv, pmul, pneg, ppow, propose_sqfree, pscale, pshift_up,
    psub, roots_of, xgcd,
};
use forge_bool_core::Sgn;
use forge_math::Interval;

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

/// `axisPlane2 o z`: the second axial plane, normal `z × (z × offAxis z)`, orthogonal to the
/// first (`axisPlane2_on`, `axisPlanes_orth`).
pub fn axis_plane2(o: &V3, z: &V3) -> Plane {
    let n = z.cross(&z.cross(&off_axis(z)));
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
        Kind::Cyl => vec![axis_plane(&f.o, &f.z), axis_plane2(&f.o, &f.z)],
        Kind::Cone => vec![axis_plane(&f.o, &f.z), axis_plane2(&f.o, &f.z), nappe_plane(f)],
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
    elim_t(&restr(qj, c), &restr(qk, c))
}

/// `elimT`: `elim` on two restrictions `f = a₀ + a₁t + a₂t²`, `g = b₀ + b₁t + b₂t²` with coefficients in
/// `ℚ[s]`: the resultant in `t`, the subresultant `D t + N`, and the cofactors `U f + V g = R`.
pub fn elim_t(a: &(Poly, Poly, Poly), b: &(Poly, Poly, Poly)) -> Elim {
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
// QVertPlane.lean: the vertex proposer for a plane and two quadrics
// =============================================================================================

/// `substZ q α₀ α₁ α₂`: `q(x, y, α₀ + α₁x + α₂y)` as coefficients in `y` over `ℚ[x]`.
pub fn subst_z_q(q: &Quadric, a0: &Q, a1: &Q, a2: &Q) -> (Poly, Poly, Poly) {
    let two = Q::from_i64(2);
    (
        vec![
            q.zz.mul(a0).mul(a0).add(&q.lz.mul(a0)).add(&q.c),
            two.mul(&q.zz)
                .mul(a0)
                .mul(a1)
                .add(&q.xz.mul(a0))
                .add(&q.lx)
                .add(&q.lz.mul(a1)),
            q.xx.add(&q.zz.mul(a1).mul(a1)).add(&q.xz.mul(a1)),
        ],
        vec![
            two.mul(&q.zz)
                .mul(a2)
                .mul(a0)
                .add(&q.yz.mul(a0))
                .add(&q.ly)
                .add(&q.lz.mul(a2)),
            q.xy.add(&two.mul(&q.zz).mul(a2).mul(a1))
                .add(&q.xz.mul(a2))
                .add(&q.yz.mul(a1)),
        ],
        vec![q.yy.add(&q.zz.mul(a2).mul(a2)).add(&q.yz.mul(a2))],
    )
}

/// `planeZOf P`: the plane's `z = α₀ + α₁x + α₂y`.
pub fn plane_z_of(p: &Quadric) -> (Q, Q, Q) {
    (p.c.neg().div(&p.lz), p.lx.neg().div(&p.lz), p.ly.neg().div(&p.lz))
}

/// `onPlane q P`.
pub fn on_plane(q: &Quadric, p: &Quadric) -> (Poly, Poly, Poly) {
    let (a0, a1, a2) = plane_z_of(p);
    subst_z_q(q, &a0, &a1, &a2)
}

/// `mkVertP qa qb P E n`: `y = −N/D`, `z` from the plane.
pub fn mk_vert_p(qa: &Quadric, qb: &Quadric, p: &Quadric, e: &Elim, n: &Num) -> VSample {
    let x: Poly = vec![Q::zero(), Q::one()];
    let nz = padd(
        &psub(&pscale(&p.lx, &pmul(&x, &e.d)), &pscale(&p.ly, &e.n)),
        &pscale(&p.c, &e.d),
    );
    let dz = pscale(&p.lz, &e.d);
    VSample {
        r: e.r.clone(),
        ny: e.n.clone(),
        dy: e.d.clone(),
        nz: nz.clone(),
        dz: dz.clone(),
        x: n.clone(),
        qa: pdiv(&homog3(qa, &e.n, &e.d, &nz, &dz), &e.r),
        qb: pdiv(&homog3(qb, &e.n, &e.d, &nz, &dz), &e.r),
        qc: Vec::new(),
    }
}

/// `quotP q P`: `q = K P + f`, `K = k₀ + k₁z` over `ℚ[x][y]` (checked through `bezout3`).
pub fn quot_p(q: &Quadric, p: &Quadric) -> TPoly {
    let t = restr_t(q);
    let inv: Poly = vec![Q::one().div(&p.lz)];
    let p0: BPoly = vec![vec![p.c.clone(), p.lx.clone()], vec![p.ly.clone()]];
    let k1 = pscale(&inv, &t[2]);
    let k0 = pscale(&inv, &psub(&t[1], &pmul(&k1, &p0)));
    vec![k0, k1]
}

/// **`proposeVertPlane qa qb P fuel`**: every vertex of the plane `P` and the quadrics, each
/// certified, or `None` (`proposeVertPlane_complete`). `A = U`, `B = V`,
/// `C = −(U K_a + V K_b)`.
pub fn propose_vert_plane(qa: &Quadric, qb: &Quadric, p: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    if p.lz.is_zero() {
        return None;
    }
    let e = elim_t(&on_plane(qa, p), &on_plane(qb, p));
    let a: TPoly = vec![e.u.clone()];
    let b: TPoly = vec![e.v.clone()];
    let c = pneg(&padd(&pmul(&a, &quot_p(qa, p)), &pmul(&b, &quot_p(qb, p))));
    if !bezout3(qa, qb, p, &a, &b, &c, &e.r) {
        return None;
    }
    let ns = roots_of(&e.r, fuel)?;
    let mut vs: Vec<VSample> = Vec::with_capacity(ns.len());
    let mut ok = true;
    let mut i = 0;
    while i < ns.len() {
        let v = mk_vert_p(qa, qb, p, &e, &ns[i]);
        ok = ok && v.on_vert(qa, qb, p) && v.d_ok(SIGN_FUEL);
        vs.push(v);
        i += 1;
    }
    if ok { Some(vs) } else { None }
}

impl Quadric {
    /// `Quadric.swapYZ`: `q(x, z, y)` (`swapYZ_ev`).
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

impl VSample {
    /// `VSample.swapBack`: a sample of the exchanged problem, read back.
    pub fn swap_back(&self) -> VSample {
        VSample {
            r: self.r.clone(),
            ny: self.nz.clone(),
            dy: self.dz.clone(),
            nz: self.ny.clone(),
            dz: self.dy.clone(),
            x: self.x.clone(),
            qa: self.qa.clone(),
            qb: self.qb.clone(),
            qc: self.qc.clone(),
        }
    }
}

/// **`proposeVertPlaneAny`**: `lz ≠ 0` directly; else `ly ≠ 0` through the exchange, every
/// `on_vert` and `d_ok` rerun on the original quadrics; else `None`
/// (`proposeVertPlaneAny_complete`).
pub fn propose_vert_plane_any(qa: &Quadric, qb: &Quadric, p: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    if !p.lz.is_zero() {
        propose_vert_plane(qa, qb, p, fuel)
    } else if !p.ly.is_zero() {
        let vs = propose_vert_plane(&qa.swap_yz(), &qb.swap_yz(), &p.swap_yz(), fuel)?;
        let mut ws: Vec<VSample> = Vec::with_capacity(vs.len());
        let mut ok = true;
        let mut i = 0;
        while i < vs.len() {
            let w = vs[i].swap_back();
            ok = ok && w.on_vert(qa, qb, p) && w.d_ok(SIGN_FUEL);
            ws.push(w);
            i += 1;
        }
        if ok { Some(ws) } else { None }
    } else {
        None
    }
}

/// `gcdOk G s t R_A R_B`: `G = s R_A + t R_B` (`gcd_cover`: a common root of `R_A` and `R_B` is
/// a root of `G`).
pub fn gcd_ok(g: &[Q], s: &[Q], t: &[Q], ra: &[Q], rb: &[Q]) -> bool {
    eqv(g, &padd(&pmul(s, ra), &pmul(t, rb)))
}

/// Trailing zero coefficients dropped.
fn btrim(b: &[Poly]) -> BPoly {
    let mut n = b.len();
    while n > 0 && is_zero_p(&b[n - 1]) {
        n -= 1;
    }
    b[..n].to_vec()
}

// =============================================================================================
// QVert3.lean: the vertex proposer for three quadrics
// =============================================================================================

/// `zcoef q k`: the `z^k` coefficient of `q` over `ℚ[x][y]`.
pub fn zcoef(q: &Quadric, k: usize) -> BPoly {
    let t = restr_t(q);
    if k < t.len() { t[k].clone() } else { Vec::new() }
}

/// `elimZ qa qb`: QPropose's closed-form elimination over `ℚ[x][y]`.
pub fn elim_z(qa: &Quadric, qb: &Quadric) -> ElimG<BPoly> {
    elim_gen(
        &(zcoef(qa, 0), zcoef(qa, 1), zcoef(qa, 2)),
        &(zcoef(qb, 0), zcoef(qb, 1), zcoef(qb, 2)),
    )
}

/// `bdeg`.
pub fn bdeg(f: &[Poly]) -> usize {
    let t = btrim(f);
    if t.is_empty() { 0 } else { t.len() - 1 }
}

/// `blc`.
pub fn blc(f: &[Poly]) -> Poly {
    let t = btrim(f);
    if t.is_empty() {
        Vec::new()
    } else {
        t[t.len() - 1].clone()
    }
}

/// `bzeroQ`.
pub fn bzero_q(f: &[Poly]) -> bool {
    btrim(f).is_empty()
}

/// `bcontent`: the `ℚ[x]`-content of a list of bivariates.
pub fn bcontent(fs: &[&BPoly]) -> Poly {
    let mut g: Poly = Vec::new();
    let mut i = 0;
    while i < fs.len() {
        let mut j = 0;
        while j < fs[i].len() {
            if !is_zero_p(&fs[i][j]) {
                g = gcd(&g, &fs[i][j]);
            }
            j += 1;
        }
        i += 1;
    }
    g
}

/// `bdivc c f`.
pub fn bdivc(c: &[Q], f: &[Poly]) -> BPoly {
    let mut out: BPoly = Vec::with_capacity(f.len());
    let mut i = 0;
    while i < f.len() {
        out.push(pdiv(&f[i], c));
        i += 1;
    }
    out
}

/// `lg a − lf yᵏ b`, the step of `prem`.
fn prem_step(lg: &[Q], lf: &[Q], k: usize, a: &[Poly], b: &[Poly]) -> BPoly {
    psub(&pscale(&lg.to_vec(), a), &pscale(&lf.to_vec(), &pshift_up(k, b)))
}

/// `prem g gs gt n f fs ft`: reduce `f` (with cofactors) by `g` until `deg f < deg g`, the
/// common content divided out at every step.
#[allow(clippy::too_many_arguments)]
pub fn prem(
    g: &[Poly],
    gs: &[Poly],
    gt: &[Poly],
    n: usize,
    f0: &[Poly],
    fs0: &[Poly],
    ft0: &[Poly],
) -> (BPoly, BPoly, BPoly) {
    let mut f = f0.to_vec();
    let mut fs = fs0.to_vec();
    let mut ft = ft0.to_vec();
    let mut k2 = n;
    while k2 > 0 && !(bzero_q(&f) || bdeg(&f) < bdeg(g)) {
        let k = bdeg(&f) - bdeg(g);
        let lg = blc(g);
        let lf = blc(&f);
        let f1 = prem_step(&lg, &lf, k, &f, g);
        let fs1 = prem_step(&lg, &lf, k, &fs, gs);
        let ft1 = prem_step(&lg, &lf, k, &ft, gt);
        let c = bcontent(&[&f1, &fs1, &ft1]);
        if deg_p(&c) == 0 {
            f = f1;
            fs = fs1;
            ft = ft1;
        } else {
            f = bdivc(&c, &f1);
            fs = bdivc(&c, &fs1);
            ft = bdivc(&c, &ft1);
        }
        k2 -= 1;
    }
    (f, fs, ft)
}

/// `Chain`: `R = S R₁ + T R₂` free of `y`, and the last linear remainder `L = LS R₁ + LT R₂`.
#[derive(Clone, Debug)]
pub struct Chain {
    pub r: Poly,
    pub s: BPoly,
    pub t: BPoly,
    pub l: BPoly,
    pub ls: BPoly,
    pub lt: BPoly,
}

/// The remainder sequence's fuel (`chain 16`) and `prem`'s (`prem … 64`).
pub const CHAIN_FUEL: usize = 16;
pub const PREM_FUEL: usize = 64;

/// `chain`: the remainder sequence, to the last remainder constant in `y` and the last linear one.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn chain(
    fuel: usize,
    f0: BPoly,
    fs0: BPoly,
    ft0: BPoly,
    g0: BPoly,
    gs0: BPoly,
    gt0: BPoly,
) -> Option<Chain> {
    let (mut f, mut fs, mut ft, mut g, mut gs, mut gt) = (f0, fs0, ft0, g0, gs0, gt0);
    let mut lin: Option<(BPoly, BPoly, BPoly)> = None;
    let mut out: Option<Chain> = None;
    let mut done = false;
    let mut n = fuel;
    while !done && n > 0 {
        if bzero_q(&g) {
            done = true;
        } else if bdeg(&g) == 0 {
            let lin2 = if bdeg(&f) == 1 {
                Some((f.clone(), fs.clone(), ft.clone()))
            } else {
                lin.clone()
            };
            out = lin2.map(|(l, ls, lt)| {
                let tg = btrim(&g);
                Chain {
                    r: if tg.is_empty() { Vec::new() } else { tg[0].clone() },
                    s: gs.clone(),
                    t: gt.clone(),
                    l: btrim(&l),
                    ls,
                    lt,
                }
            });
            done = true;
        } else {
            let (r, rs, rt) = prem(&g, &gs, &gt, PREM_FUEL, &f, &fs, &ft);
            if bdeg(&g) == 1 {
                lin = Some((g.clone(), gs.clone(), gt.clone()));
            }
            f = g;
            fs = gs;
            ft = gt;
            g = r;
            gs = rs;
            gt = rt;
        }
        n -= 1;
    }
    out
}

/// `Proj`: one projection.
#[derive(Clone, Debug)]
pub struct Proj {
    pub r: Poly,
    pub a: TPoly,
    pub b: TPoly,
    pub c: TPoly,
    pub l: BPoly,
    pub al: TPoly,
    pub bl: TPoly,
    pub cl: TPoly,
    pub n: BPoly,
    pub d: BPoly,
}

/// `(S U₁ + T U₂, S V₁, T V₂)`.
fn cof(s: &[Poly], t: &[Poly], e1: &ElimG<BPoly>, e2: &ElimG<BPoly>) -> (TPoly, TPoly, TPoly) {
    let s1: TPoly = vec![s.to_vec()];
    let t1: TPoly = vec![t.to_vec()];
    (
        padd(&pmul(&s1, &e1.u), &pmul(&t1, &e2.u)),
        pmul(&s1, &e1.v),
        pmul(&t1, &e2.v),
    )
}

/// **`proj`**: the two eliminations and the chain.
pub fn proj(qa: &Quadric, qb: &Quadric, qc: &Quadric) -> Option<Proj> {
    let e1 = elim_z(qa, qb);
    let e2 = elim_z(qa, qc);
    let one: BPoly = vec![vec![Q::one()]];
    let ch = if bdeg(&e1.r) >= bdeg(&e2.r) {
        chain(
            CHAIN_FUEL,
            e1.r.clone(),
            one.clone(),
            Vec::new(),
            e2.r.clone(),
            Vec::new(),
            one,
        )
    } else {
        chain(
            CHAIN_FUEL,
            e2.r.clone(),
            Vec::new(),
            one.clone(),
            e1.r.clone(),
            one,
            Vec::new(),
        )
    }?;
    let (a, b, c) = cof(&ch.s, &ch.t, &e1, &e2);
    let (al, bl, cl) = cof(&ch.ls, &ch.lt, &e1, &e2);
    Some(Proj {
        r: ch.r,
        a,
        b,
        c,
        l: ch.l,
        al,
        bl,
        cl,
        n: e1.n,
        d: e1.d,
    })
}

/// **`comb3`**: `L = A q_a + B q_b + C q_c`, checked.
#[allow(clippy::too_many_arguments)]
pub fn comb3(
    qa: &Quadric,
    qb: &Quadric,
    qc: &Quadric,
    a: &[BPoly],
    b: &[BPoly],
    c: &[BPoly],
    l: &[Poly],
) -> bool {
    tzero(&psub(
        &padd(
            &padd(&pmul(a, &restr_t(qa)), &pmul(b, &restr_t(qb))),
            &pmul(c, &restr_t(qc)),
        ),
        &[l.to_vec()],
    ))
}

/// **`linOk`**: `b₂ q_a − a₂ q_b = N + D z`, checked.
pub fn lin_ok(qa: &Quadric, qb: &Quadric, n: &[Poly], d: &[Poly]) -> bool {
    tzero(&psub(
        &padd(
            &pmul(&[zcoef(qb, 2)], &restr_t(qa)),
            &pmul(&[pneg(&zcoef(qa, 2))], &restr_t(qb)),
        ),
        &[n.to_vec(), d.to_vec()],
    ))
}

/// `homSub p0 p1 f = f(x, −p₀/p₁) p₁^(len f − 1)` (Horner).
pub fn hom_sub(p0: &[Q], p1: &[Q], f: &[Poly]) -> Poly {
    let mp0 = pneg(p0);
    let mut acc: Poly = Vec::new();
    let mut i = f.len();
    while i > 0 {
        i -= 1;
        acc = padd(&pmul(&f[i], &ppow(p1, f.len() - 1 - i)), &pmul(&mp0, &acc));
    }
    acc
}

/// `bpad m f`: `f` padded with zero coefficients to length `m`.
pub fn bpad(m: usize, f: &[Poly]) -> BPoly {
    let mut out = f.to_vec();
    while out.len() < m {
        out.push(Vec::new());
    }
    out
}

/// `tryShear`: the projection after the shear `λ`, if its chain checks.
pub fn try_shear(qa: &Quadric, qb: &Quadric, qc: &Quadric, l: &Q) -> Option<(Q, Proj)> {
    let (sa, sb, sc) = (qa.shear(l), qb.shear(l), qc.shear(l));
    let p = proj(&sa, &sb, &sc)?;
    if bezout3(&sa, &sb, &sc, &p.a, &p.b, &p.c, &p.r) {
        Some((l.clone(), p))
    } else {
        None
    }
}

/// `secondProj`: the first shear `λ ≠ 0` whose chain checks.
pub fn second_proj(qa: &Quadric, qb: &Quadric, qc: &Quadric) -> Option<(Q, Proj)> {
    let ls = shears();
    let mut out: Option<(Q, Proj)> = None;
    let mut i = 0;
    while out.is_none() && i < ls.len() {
        if !ls[i].is_zero() {
            out = try_shear(qa, qb, qc, &ls[i]);
        }
        i += 1;
    }
    out
}

/// **`proposeVert3`**: every common point of three quadrics, each certified, or `None`
/// (`proposeVert3_complete`).
pub fn propose_vert3(qa: &Quadric, qb: &Quadric, qc: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    let pa = proj(qa, qb, qc)?;
    if !(bezout3(qa, qb, qc, &pa.a, &pa.b, &pa.c, &pa.r)
        && comb3(qa, qb, qc, &pa.al, &pa.bl, &pa.cl, &pa.l)
        && lin_ok(qa, qb, &pa.n, &pa.d))
    {
        return None;
    }
    let (_, pb) = second_proj(qa, qb, qc)?;
    let (g, u, v) = xgcd(&pa.r, &pb.r);
    if !gcd_ok(&g, &u, &v, &pa.r, &pb.r) {
        return None;
    }
    let sp = propose_sqfree(&g)?;
    let ns = roots_of(&g, fuel)?;
    let p0 = if pa.l.is_empty() {
        Vec::new()
    } else {
        pa.l[0].clone()
    };
    let p1 = if pa.l.len() > 1 {
        pa.l[1].clone()
    } else {
        Vec::new()
    };
    let m = if pa.n.len() >= pa.d.len() {
        pa.n.len()
    } else {
        pa.d.len()
    };
    let nz = hom_sub(&p0, &p1, &bpad(m, &pa.n));
    let dz = hom_sub(&p0, &p1, &bpad(m, &pa.d));
    let ha = homog3(qa, &p0, &p1, &nz, &dz);
    let hb = homog3(qb, &p0, &p1, &nz, &dz);
    let hc = homog3(qc, &p0, &p1, &nz, &dz);
    let wa = pdiv(&ha, &sp);
    let wb = pdiv(&hb, &sp);
    let wc = pdiv(&hc, &sp);
    // `on_vert` reads only what every sample shares (R, the representation, the quotients), so
    // it is decided once, on the `homog3`s already built: the same value as the Lean's
    // per-sample `vs.all (onVert ..)`
    let vert_ok = eqv(&ha, &pmul(&sp, &wa)) && eqv(&hb, &pmul(&sp, &wb)) && eqv(&hc, &pmul(&sp, &wc));
    let mut vs: Vec<VSample> = Vec::with_capacity(ns.len());
    let mut ok = pa.l.len() <= 2;
    let mut k = 0;
    while k < ns.len() {
        let vsm = VSample {
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
        ok = ok && vert_ok && vsm.d_ok(SIGN_FUEL);
        debug_assert!(!ok || vsm.on_vert(qa, qb, qc));
        vs.push(vsm);
        k += 1;
    }
    if ok { Some(vs) } else { None }
}

/// `deg` over ℚ.
fn deg_p(p: &[Q]) -> usize {
    forge_alg::deg(p)
}

// =============================================================================================
// QSweep.lean: the events of a curve (turning points), and their sorting and separation
// =============================================================================================

/// An affine form `a x + b y + c z + d`.
pub type Aff = (Q, Q, Q, Q);

/// `gradY q`: `∂_y q` as an affine form.
pub fn grad_y(q: &Quadric) -> Aff {
    (
        q.xy.clone(),
        Q::from_i64(2).mul(&q.yy),
        q.yz.clone(),
        q.ly.clone(),
    )
}

/// `gradZ q`: `∂_z q` as an affine form.
pub fn grad_z(q: &Quadric) -> Aff {
    (
        q.xz.clone(),
        q.yz.clone(),
        Q::from_i64(2).mul(&q.zz),
        q.lz.clone(),
    )
}

/// `affMul f g`: the product of two affine forms, as a quadric.
pub fn aff_mul(f: &Aff, g: &Aff) -> Quadric {
    let (a, b, c, d) = f;
    let (a2, b2, c2, d2) = g;
    Quadric {
        xx: a.mul(a2),
        yy: b.mul(b2),
        zz: c.mul(c2),
        xy: a.mul(b2).add(&b.mul(a2)),
        xz: a.mul(c2).add(&c.mul(a2)),
        yz: b.mul(c2).add(&c.mul(b2)),
        lx: a.mul(d2).add(&d.mul(a2)),
        ly: b.mul(d2).add(&d.mul(b2)),
        lz: c.mul(d2).add(&d.mul(c2)),
        c: d.mul(d2),
    }
}

/// `qsub p q`.
pub fn qsub(p: &Quadric, q: &Quadric) -> Quadric {
    Quadric {
        xx: p.xx.sub(&q.xx),
        yy: p.yy.sub(&q.yy),
        zz: p.zz.sub(&q.zz),
        xy: p.xy.sub(&q.xy),
        xz: p.xz.sub(&q.xz),
        yz: p.yz.sub(&q.yz),
        lx: p.lx.sub(&q.lx),
        ly: p.ly.sub(&q.ly),
        lz: p.lz.sub(&q.lz),
        c: p.c.sub(&q.c),
    }
}

/// `critQ qj qk = (∇q_j × ∇q_k)ₓ`: zero at the `x`-turning points of `q_j ∩ q_k` (`critQ_ev`).
pub fn crit_q(qj: &Quadric, qk: &Quadric) -> Quadric {
    qsub(
        &aff_mul(&grad_y(qj), &grad_z(qk)),
        &aff_mul(&grad_z(qj), &grad_y(qk)),
    )
}

fn aff_plane(f: &Aff) -> Quadric {
    Quadric {
        lx: f.0.clone(),
        ly: f.1.clone(),
        lz: f.2.clone(),
        c: f.3.clone(),
        ..Quadric::default()
    }
}

/// `gradPlaneY q`: `∂_y q = 0` as a plane.
pub fn grad_plane_y(q: &Quadric) -> Quadric {
    aff_plane(&grad_y(q))
}

/// `gradPlaneZ q`: `∂_z q = 0` as a plane.
pub fn grad_plane_z(q: &Quadric) -> Quadric {
    aff_plane(&grad_z(q))
}

/// `isPlaneQ`: no quadratic term.
pub fn is_plane_q(q: &Quadric) -> bool {
    q.xx.is_zero() && q.yy.is_zero() && q.zz.is_zero() && q.xy.is_zero() && q.xz.is_zero() && q.yz.is_zero()
}

/// **`proposeTriple`**: the vertex proposer for a triple, by its planes, in the Lean's role order.
pub fn propose_triple(a: &Quadric, b: &Quadric, c: &Quadric, fuel: usize) -> Option<Vec<VSample>> {
    let (pa, pb, pc) = (is_plane_q(a), is_plane_q(b), is_plane_q(c));
    if !pa && !pb && !pc {
        propose_vert3(a, b, c, fuel)
    } else if pc && !pa {
        propose_vert_plane_any(a, b, c, fuel)
    } else if pc && !pb {
        propose_vert_plane_any(b, a, c, fuel)
    } else if pb && !pa {
        propose_vert_plane_any(a, c, b, fuel)
    } else if pb && !pc {
        propose_vert_plane_any(c, a, b, fuel)
    } else if pa && !pb {
        propose_vert_plane_any(b, c, a, fuel)
    } else if pa && !pc {
        propose_vert_plane_any(c, b, a, fuel)
    } else {
        None
    }
}

/// `isConstQ`: no `x`, `y`, `z` term at all.
pub fn is_const_q(q: &Quadric) -> bool {
    is_plane_q(q) && q.lx.is_zero() && q.ly.is_zero() && q.lz.is_zero()
}

/// `det3Q`.
pub fn det3_q(a: &Quadric, b: &Quadric, c: &Quadric) -> Q {
    a.lx.mul(&b.ly.mul(&c.lz).sub(&b.lz.mul(&c.ly)))
        .sub(&a.ly.mul(&b.lx.mul(&c.lz).sub(&b.lz.mul(&c.lx))))
        .add(&a.lz.mul(&b.lx.mul(&c.ly).sub(&b.ly.mul(&c.lx))))
}

/// `numXQ`: Cramer's numerator for `x`.
pub fn num_x_q(a: &Quadric, b: &Quadric, c: &Quadric) -> Q {
    a.c.mul(&b.ly.mul(&c.lz).sub(&b.lz.mul(&c.ly)))
        .sub(&a.ly.mul(&b.c.mul(&c.lz).sub(&b.lz.mul(&c.c))))
        .add(&a.lz.mul(&b.c.mul(&c.ly).sub(&b.ly.mul(&c.c))))
        .neg()
}

/// A nonzero constant: no zeros at all.
fn nz_const(q: &Quadric) -> bool {
    is_const_q(q) && !q.c.is_zero()
}

/// **`proposeTripleX`**: the `x` of every common point of a triple (`proposeTripleX_complete`).
pub fn propose_triple_x(a: &Quadric, b: &Quadric, c: &Quadric, fuel: usize) -> Option<Vec<Num>> {
    if nz_const(a) || nz_const(b) || nz_const(c) {
        Some(Vec::new())
    } else if is_plane_q(a) && is_plane_q(b) && is_plane_q(c) {
        let d = det3_q(a, b, c);
        if !d.is_zero() {
            Some(vec![Num::Rat(num_x_q(a, b, c).div(&d))])
        } else {
            None
        }
    } else {
        let vs = propose_triple(a, b, c, fuel)?;
        let mut out: Vec<Num> = Vec::with_capacity(vs.len());
        let mut i = 0;
        while i < vs.len() {
            out.push(vs[i].x.clone());
            i += 1;
        }
        Some(out)
    }
}

/// `QRef.sortNums`: insertion sort by `Num.cmp 200`, equal ones dropped; `None` if a comparison
/// fails.
pub fn sort_nums(ns: &[Num]) -> Option<Vec<Num>> {
    let mut out: Vec<Num> = Vec::new();
    let mut failed = false;
    let mut i = 0;
    while !failed && i < ns.len() {
        let mut pos = out.len();
        let mut dup = false;
        let mut found = false;
        let mut k = 0;
        while !found && !failed && k < out.len() {
            match Num::cmp(200, &ns[i], &out[k]) {
                Some(-1) => {
                    pos = k;
                    found = true;
                }
                Some(0) => {
                    dup = true;
                    found = true;
                }
                Some(_) => {}
                None => failed = true,
            }
            k += 1;
        }
        if !failed && !dup {
            out.insert(pos, ns[i].clone());
        }
        i += 1;
    }
    if failed { None } else { Some(out) }
}

/// `separate k u rest`: refine consecutive numbers until their intervals separate (untrusted).
pub fn separate(k0: usize, u0: &Num, rest0: &[Num]) -> Vec<Num> {
    let mut out: Vec<Num> = Vec::new();
    let mut k = k0;
    let mut u = u0.clone();
    let mut rest = rest0.to_vec();
    let mut i = 0;
    while i < rest.len() {
        if k == 0 || u.hi().lt(&rest[i].lo()) {
            out.push(u);
            u = rest[i].clone();
            i += 1;
        } else {
            k -= 1;
            u = u.bisect();
            rest[i] = rest[i].bisect();
        }
    }
    out.push(u);
    out
}

/// `isoOkN`.
pub fn iso_ok_n(n: &Num) -> bool {
    match n {
        Num::Rat(_) => true,
        Num::Iso(r) => r.ok(),
    }
}

/// `sepOk`: consecutive numbers' intervals are apart.
pub fn sep_ok(l: &[Num]) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i + 1 < l.len() {
        ok = l[i].hi().lt(&l[i + 1].lo());
        i += 1;
    }
    ok
}

/// `spreadE`.
pub fn spread_e(l: &[Num]) -> Vec<Num> {
    if l.is_empty() {
        Vec::new()
    } else {
        separate(400, &l[0], &l[1..])
    }
}

/// **`checkEvents`**: every raw event equals a listed one, every listed one is a checked
/// isolation, and consecutive ones are separated.
pub fn check_events(raw: &[Num], e: &[Num]) -> bool {
    let mut ok = true;
    let mut i = 0;
    while ok && i < raw.len() {
        let mut any = false;
        let mut j = 0;
        while !any && j < e.len() {
            any = Num::cmp(200, &raw[i], &e[j]) == Some(0);
            j += 1;
        }
        ok = any;
        i += 1;
    }
    let mut k = 0;
    while ok && k < e.len() {
        ok = iso_ok_n(&e[k]);
        k += 1;
    }
    ok && sep_ok(e)
}

/// `pickEvents`.
pub fn pick_events(raw: &[Num]) -> Option<Vec<Num>> {
    let sorted = sort_nums(raw)?;
    let e = spread_e(&sorted);
    if check_events(raw, &e) { Some(e) } else { None }
}

/// `planesOf`: a rational sample plane between consecutive events, and one outside each end.
pub fn planes_of(e: &[Num]) -> Vec<Q> {
    if e.is_empty() {
        vec![Q::zero()]
    } else {
        let mut out: Vec<Q> = Vec::with_capacity(e.len() + 1);
        out.push(e[0].lo().sub(&Q::one()));
        let mut i = 0;
        while i + 1 < e.len() {
            out.push(e[i].hi().mid(&e[i + 1].lo()));
            i += 1;
        }
        out.push(e[e.len() - 1].hi().add(&Q::one()));
        out
    }
}

// =============================================================================================
// The builder, stage (a): the operands' faces, exactly, their carrier classes, and the near pairs
// =============================================================================================

/// A face of an operand as the curved builder reads it.
#[derive(Clone, Debug)]
pub struct CIn {
    pub body: usize,
    pub face: forge_topo::FaceId,
    /// the surface as stored, exactly (`QFace`); `QFace::quadric` is negative inside the solid
    /// when `outward`, positive inside when not (a face with the solid outside: a hole's wall)
    pub q: QFace,
    pub outward: bool,
    /// the face's box (f64, as the kernel keeps it)
    pub lo: [f64; 3],
    pub hi: [f64; 3],
}

fn v3_of(p: forge_math::Point3) -> Option<V3> {
    V3::from_f64([p.x, p.y, p.z])
}

fn vv_of(v: forge_math::Vec3) -> Option<V3> {
    V3::from_f64([v.x, v.y, v.z])
}

fn q_of(x: f64) -> Option<Q> {
    Q::from_f64(x)
}

/// **Stage (a)**: every face of the operands, in arena order (body 0's first), read exactly from
/// its stored frame, as exact-curved's harness reads it (verify/forge-exact/fuzz/src/curved.rs,
/// `face_of`), but keeping a face whose solid lies outside it (`outward = false`). Tori and
/// procedural faces are outside the Q1 domain (design D4).
pub fn read_faces(bodies: &[&forge_topo::Body]) -> Result<Vec<CIn>, Skip> {
    use forge_geom::Surface;
    let mut out: Vec<CIn> = Vec::new();
    let mut k = 0;
    while k < bodies.len() {
        let b = bodies[k];
        let ids: Vec<forge_topo::FaceId> = b.face_ids().collect();
        let mut i = 0;
        while i < ids.len() {
            let f = ids[i];
            let face = b.face(f);
            let bb = b.face_box(f);
            let lo = [bb.x.lo, bb.y.lo, bb.z.lo];
            let hi = [bb.x.hi, bb.y.hi, bb.z.hi];
            let zero = Q::zero();
            let read: Option<(Kind, V3, V3, Q, Q, Q, bool)> = match &face.surface {
                Surface::Plane(pl) => {
                    let z = pl.frame.z().get();
                    let n = if face.same_sense { z } else { -z };
                    match (v3_of(pl.frame.origin), vv_of(n)) {
                        (Some(o), Some(n)) => {
                            Some((Kind::Plane, o, n, zero.clone(), zero.clone(), zero.clone(), true))
                        }
                        _ => None,
                    }
                }
                Surface::Cylinder(c) => {
                    match (v3_of(c.frame.origin), vv_of(c.frame.z().get()), q_of(c.radius)) {
                        (Some(o), Some(z), Some(r)) => {
                            Some((Kind::Cyl, o, z, r, zero.clone(), zero.clone(), face.same_sense))
                        }
                        _ => None,
                    }
                }
                Surface::Sphere(s) => match (v3_of(s.frame.origin), q_of(s.radius)) {
                    (Some(o), Some(r)) => Some((
                        Kind::Sphere,
                        o,
                        V3::default(),
                        r,
                        zero.clone(),
                        zero.clone(),
                        face.same_sense,
                    )),
                    _ => None,
                },
                Surface::Cone(c) => match (
                    v3_of(c.frame.origin),
                    vv_of(c.frame.z().get()),
                    q_of(c.radius),
                    q_of(c.sin_a()),
                    q_of(c.cos_a()),
                ) {
                    (Some(o), Some(z), Some(r), Some(s), Some(cc)) => {
                        Some((Kind::Cone, o, z, r, s, cc, face.same_sense))
                    }
                    _ => None,
                },
                Surface::Torus(_) => return Err(Skip::OutOfDomain("a torus face")),
                Surface::Procedural(_) => return Err(Skip::OutOfDomain("a procedural face")),
            };
            match read {
                None => return Err(Skip::OutOfDomain("a non-finite frame")),
                Some((kind, o, z, r, s, c, outward)) => {
                    let q = QFace {
                        solid: k,
                        index: f.index(),
                        kind,
                        o,
                        z,
                        r,
                        s,
                        c,
                    };
                    if !q.wf() {
                        return Err(Skip::OutOfDomain("a degenerate frame"));
                    }
                    out.push(CIn {
                        body: k,
                        face: f,
                        q,
                        outward,
                        lo,
                        hi,
                    });
                }
            }
            i += 1;
        }
        k += 1;
    }
    Ok(out)
}

/// The carrier classes: faces on one quadric (equal up to scale, `Quadric::canon`) share one.
/// Returns, per face, its class, and per class its canonical quadric.
pub fn carrier_classes(fs: &[CIn]) -> (Vec<usize>, Vec<Quadric>) {
    let mut cls: Vec<usize> = Vec::with_capacity(fs.len());
    let mut keys: Vec<Quadric> = Vec::new();
    let mut i = 0;
    while i < fs.len() {
        let (k, _) = fs[i].q.quadric().canon();
        let mut j = 0;
        let mut found = keys.len();
        while found == keys.len() && j < keys.len() {
            if keys[j] == k {
                found = j;
            }
            j += 1;
        }
        if found == keys.len() {
            keys.push(k);
        }
        cls.push(found);
        i += 1;
    }
    (cls, keys)
}

/// The near pairs: faces on distinct carriers whose boxes overlap within LINEAR. Only these can
/// cross, so only these are given to the proposers.
pub fn near_pairs(fs: &[CIn], cls: &[usize]) -> Vec<(usize, usize)> {
    let t = forge_math::tol::LINEAR;
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < fs.len() {
        let mut j = i + 1;
        while j < fs.len() {
            let mut apart = cls[i] == cls[j];
            let mut a = 0;
            while !apart && a < 3 {
                apart = fs[i].hi[a] + t < fs[j].lo[a] || fs[j].hi[a] + t < fs[i].lo[a];
                a += 1;
            }
            if !apart {
                out.push((i, j));
            }
            j += 1;
        }
        i += 1;
    }
    out
}

// =============================================================================================
// The builder, stage (b): the crossing curve of a near pair, as certified slab samples
// =============================================================================================

/// The crossing curve `q_j = q_k = 0` of a near pair, sampled between its `x`-events. In frame
/// `λ` (`shears`), the curve's turning points in `x` are the `events`. On each sweep plane
/// `x = planes[s]` (one between consecutive events, one outside each end: `planes_of`), the
/// curve's crossings are `slabs[s]`, certified (`propose_edge_any`) and sorted by `y`. Between
/// consecutive events every branch of the curve is a graph over `x` (no turning point), so it
/// crosses each slab's plane once: branch `b` of slab `s` is one arc piece.
#[derive(Clone, Debug)]
pub struct PairCurve {
    pub shear: Q,
    pub events: Vec<Num>,
    pub planes: Vec<Q>,
    pub slabs: Vec<Vec<ESample>>,
}

/// The `x`s of every common point of a plane `P` and two quadrics, complete by certificate,
/// with no per-point representation: `elim_t` on the plane, `bezout3`, `roots_of R` (the `y`/`z`
/// exchange keeps `x`). A superset of the points' `x`s is all a sweep's events need.
pub fn vert_xs_plane(qa: &Quadric, qb: &Quadric, p: &Quadric, fuel: usize) -> Option<Vec<Num>> {
    if !p.lz.is_zero() {
        let e = elim_t(&on_plane(qa, p), &on_plane(qb, p));
        let a: TPoly = vec![e.u.clone()];
        let b: TPoly = vec![e.v.clone()];
        let c = pneg(&padd(&pmul(&a, &quot_p(qa, p)), &pmul(&b, &quot_p(qb, p))));
        if bezout3(qa, qb, p, &a, &b, &c, &e.r) {
            roots_of(&e.r, fuel)
        } else {
            None
        }
    } else if !p.ly.is_zero() {
        vert_xs_plane(&qa.swap_yz(), &qb.swap_yz(), &p.swap_yz(), fuel)
    } else {
        None
    }
}

/// The `x`s of every common point of three quadrics, complete by certificate (`bezout3` on two
/// projections, `gcd_ok`, `roots_of G`), with no per-point representation. Two common points
/// with one `x` (a pair mirrored in `z`) make `propose_vert3` answer `None`; their `x` is still
/// a root of `G`.
pub fn vert_xs3(qa: &Quadric, qb: &Quadric, qc: &Quadric, fuel: usize) -> Option<Vec<Num>> {
    let pa = proj(qa, qb, qc)?;
    if !bezout3(qa, qb, qc, &pa.a, &pa.b, &pa.c, &pa.r) {
        return None;
    }
    let (_, pb) = second_proj(qa, qb, qc)?;
    let (g, u, v) = xgcd(&pa.r, &pb.r);
    if !gcd_ok(&g, &u, &v, &pa.r, &pb.r) || is_zero_p(&g) {
        return None;
    }
    roots_of(&g, fuel)
}

/// **The events' `x`s of a triple**, complete by certificate: as `propose_triple_x`, but through
/// `vert_xs_plane` and `vert_xs3`, which need no per-point representation (a superset of the
/// `x`s; an extra event only adds a sweep plane). Proposed to dol-curved for QSweep.lean.
pub fn propose_triple_xs(a: &Quadric, b: &Quadric, c: &Quadric, fuel: usize) -> Option<Vec<Num>> {
    if nz_const(a) || nz_const(b) || nz_const(c) {
        Some(Vec::new())
    } else if is_plane_q(a) && is_plane_q(b) && is_plane_q(c) {
        let d = det3_q(a, b, c);
        if !d.is_zero() {
            Some(vec![Num::Rat(num_x_q(a, b, c).div(&d))])
        } else {
            None
        }
    } else {
        let (pa, pb, pc) = (is_plane_q(a), is_plane_q(b), is_plane_q(c));
        if !pa && !pb && !pc {
            vert_xs3(a, b, c, fuel)
        } else if pc && !pa {
            vert_xs_plane(a, b, c, fuel)
        } else if pc && !pb {
            vert_xs_plane(b, a, c, fuel)
        } else if pb && !pa {
            vert_xs_plane(a, c, b, fuel)
        } else if pb && !pc {
            vert_xs_plane(c, a, b, fuel)
        } else if pa && !pb {
            vert_xs_plane(b, c, a, fuel)
        } else if pa && !pc {
            vert_xs_plane(c, b, a, fuel)
        } else {
            None
        }
    }
}

/// The `x`s of a pair's turning points (`(∇q_j × ∇q_k)ₓ = 0` on the curve), sorted and separated.
pub fn pair_events(qj: &Quadric, qk: &Quadric) -> Option<Vec<Num>> {
    let raw = propose_triple_xs(qj, qk, &crit_q(qj, qk), ROOT_FUEL)?;
    pick_events(&raw)
}

/// A slab's crossings sorted by their `y` (`sort_nums` order; equal `y`s answer `None`: the
/// projection is not generic, and the next shear is tried).
fn sorted_by_y(es: Vec<ESample>) -> Option<Vec<ESample>> {
    let mut ys: Vec<Num> = Vec::with_capacity(es.len());
    let mut i = 0;
    while i < es.len() {
        ys.push(es[i].y.clone());
        i += 1;
    }
    let order = sort_nums(&ys)?;
    if order.len() != es.len() {
        return None;
    }
    let mut out: Vec<ESample> = Vec::with_capacity(es.len());
    let mut k = 0;
    while k < order.len() {
        let mut j = 0;
        let mut found = false;
        while !found && j < es.len() {
            if es[j].y == order[k] {
                out.push(es[j].clone());
                found = true;
            }
            j += 1;
        }
        k += 1;
    }
    if out.len() == es.len() { Some(out) } else { None }
}

/// The curve in one frame, or `None` if a step is undecided there.
fn pair_curve_at(qj: &Quadric, qk: &Quadric, l: &Q) -> Option<PairCurve> {
    let (a, b) = (qj.shear(l), qk.shear(l));
    let events = pair_events(&a, &b)?;
    let planes = planes_of(&events);
    let mut slabs: Vec<Vec<ESample>> = Vec::with_capacity(planes.len());
    let mut i = 0;
    while i < planes.len() {
        let es = propose_edge_any(&a, &b, &planes[i], ROOT_FUEL)?;
        slabs.push(sorted_by_y(es)?);
        i += 1;
    }
    Some(PairCurve {
        shear: l.clone(),
        events,
        planes,
        slabs,
    })
}

/// **Stage (b)**: a near pair's crossing curve, in the first frame of `shears` where every
/// step is certified (the decided rule, as `samplesOf`'s), or [`Skip::Budget`].
pub fn pair_curve(qj: &Quadric, qk: &Quadric) -> Result<PairCurve, Skip> {
    let ls = shears();
    let mut out: Option<PairCurve> = None;
    let mut i = 0;
    while out.is_none() && i < ls.len() {
        out = pair_curve_at(qj, qk, &ls[i]);
        i += 1;
    }
    match out {
        Some(c) => Ok(c),
        None => Err(Skip::Budget("a crossing curve undecided in every frame")),
    }
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
    use forge_alg::{Iso, poly_i};
    use forge_math::exact::Big;

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

    /// QVertPlane.lean's #guards, and the covectors of what is proposed.
    #[test]
    fn qvertplane_guards() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let tilt = Quadric {
            lx: qi(1),
            ly: qi(1),
            lz: qi(2),
            c: q(-1, 2),
            ..Quadric::default()
        };
        let cyl_z = Quadric {
            xx: qi(1),
            yy: qi(1),
            c: q(-1, 4),
            ..Quadric::default()
        };
        let vs = propose_vert_plane(&sphere1(), &plane_y0, &plane_z(), 30).unwrap();
        assert_eq!(vs.len(), 2);
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), plane_y0.clone(), plane_z()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        let vs = propose_vert_plane(&sphere1(), &cyl_z, &tilt, 30).unwrap();
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), cyl_z.clone(), tilt.clone()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        assert!(propose_vert_plane(&sphere1(), &plane_z(), &plane_y0, 30).is_none());
        let vs = propose_vert_plane_any(&sphere1(), &plane_z(), &plane_y0, 30).unwrap();
        assert_eq!(vs.len(), 2);
        let mut i = 0;
        while i < vs.len() {
            assert_eq!(
                vert_cov(&[sphere1(), plane_z(), plane_y0.clone()], &vs[i], SIGN_FUEL),
                Some(vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
            );
            i += 1;
        }
        // kind (c): the sphere with its partials ∂y = 2y (lz = 0, exchanged) and ∂z = 2z
        let dy = quad([0, 0, 0, 0, 0, 0, 0, 2, 0, 0]);
        let dz = quad([0, 0, 0, 0, 0, 0, 0, 0, 2, 0]);
        assert_eq!(
            propose_vert_plane_any(&sphere1(), &dz, &dy, 30).map(|v| v.len()),
            Some(2)
        );
        // x = const: refused
        let px = quad([0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(propose_vert_plane_any(&sphere1(), &plane_z(), &px, 30).is_none());
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
            if let Some(c) = proj(&s0, &s1, &cyl) {
                let mut n = 0;
                let mut bits = 0;
                let ts = [&c.a, &c.b, &c.c];
                let mut i = 0;
                while i < 3 {
                    for b in ts[i].iter() {
                        for p in b.iter() {
                            for x in p.iter() {
                                n += 1;
                                bits = bits.max(x.n.bits().max(x.d.bits()));
                            }
                        }
                    }
                    i += 1;
                }
                let mut rb = 0;
                for x in c.r.iter() {
                    rb = rb.max(x.n.bits().max(x.d.bits()));
                }
                println!(
                    "R_A degree {} (max {} bits); A, B, C: {} rationals, max {} bits",
                    forge_alg::deg(&c.r),
                    rb,
                    n,
                    bits
                );
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

    /// Stage (b): the unit circle in z = 0 turns at x = ±1, and crosses the middle plane twice.
    #[test]
    fn stage_b_circle() {
        let c = pair_curve(&sphere1(), &plane_z()).unwrap();
        assert_eq!(c.events.len(), 2);
        let counts: Vec<usize> = c.slabs.iter().map(|s| s.len()).collect();
        assert_eq!(counts, vec![0, 2, 0]);
        // two unit spheres a half apart in x: their circle lies in x = 1/4 (a plane x = const:
        // no turning point in x, the whole curve at one event), as the proposers find it
        let s2 = QFace {
            solid: 0,
            index: 0,
            kind: Kind::Sphere,
            o: V3::new(q(1, 2), qi(0), qi(0)),
            z: V3::default(),
            r: qi(1),
            s: qi(0),
            c: qi(0),
        }
        .quadric();
        let r = pair_curve(&sphere1(), &s2);
        assert!(r.is_ok() || matches!(r, Err(Skip::Budget(_))));
        // a cylinder along z through the sphere: two circles, each turning twice
        let cyl = QFace {
            solid: 0,
            index: 0,
            kind: Kind::Cyl,
            o: V3::new(q(1, 4), q(1, 8), qi(0)),
            z: V3::new(qi(0), qi(0), qi(1)),
            r: q(1, 2),
            s: qi(0),
            c: qi(0),
        }
        .quadric();
        let c = pair_curve(&sphere1(), &cyl).unwrap();
        assert_eq!(c.slabs.len(), c.events.len() + 1);
        assert!(c.slabs.iter().all(|s| s.len() % 2 == 0));
        assert!(c.slabs.iter().any(|s| s.len() == 4));
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
        // coaxial cylinders share both axial seams; the two are orthogonal and hold the axis
        assert_eq!(seams(&[cyl(qi(1)), cyl(qi(2)), other]).len(), 2);
        let ap2 = axis_plane2(&o, &z);
        assert!(ap2.eval(&o.add(&z.smul(&qi(7)))).is_zero() && ap.n.dot(&ap2.n).is_zero());
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
