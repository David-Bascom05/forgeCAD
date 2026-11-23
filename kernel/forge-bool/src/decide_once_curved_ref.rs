//! The curved decide-once **L2 reference**, executable: the Rust mirror of dol-curved's
//! QSweep.lean (event assembly), QSample.lean (the samples), QLedger.lean (the ledger) and
//! QArc.lean (crossings on a box's faces), one function per Lean definition.
//!
//! It is the reference the L6 differential judges against, run here because `lake` is not (the
//! master, 2026-11-23). It is **not** the production path: compiled only for tests and under the
//! `curved_reference` feature (for fuzzing), never reached from `boolean.rs`, and not written
//! for speed. The production builder is A8 v2's (research/decide-once/face-arrangement.md),
//! with the exact proposers of `decide_once_curved` as its fallback.

use crate::decide_once_curved::*;
use forge_bool_core::Sgn;
use forge_math::exact::alg::{Num, Poly, Q, is_zero_p, pmul, pscale, psub, roots_of};

/// A covector (Lean's `Cov`).
pub type Cov = Vec<Sgn>;

// =============================================================================================
// QSweep.lean: event assembly, complete by certificate
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

/// `tri qs`: every triple, in the Lean's order.
pub fn tri(qs: &[Quadric]) -> Vec<(Quadric, Quadric, Quadric)> {
    let mut out: Vec<(Quadric, Quadric, Quadric)> = Vec::new();
    let mut a = 0;
    while a < qs.len() {
        let mut b = a + 1;
        while b < qs.len() {
            let mut c = b + 1;
            while c < qs.len() {
                out.push((qs[a].clone(), qs[b].clone(), qs[c].clone()));
                c += 1;
            }
            b += 1;
        }
        a += 1;
    }
    out
}

/// `pairs qs`.
pub fn pairs(qs: &[Quadric]) -> Vec<(Quadric, Quadric)> {
    let mut out: Vec<(Quadric, Quadric)> = Vec::new();
    let mut a = 0;
    while a < qs.len() {
        let mut b = a + 1;
        while b < qs.len() {
            out.push((qs[a].clone(), qs[b].clone()));
            b += 1;
        }
        a += 1;
    }
    out
}

/// `eventTriples`: vertices, curve turning points, surface critical points.
pub fn event_triples(qs: &[Quadric]) -> Vec<(Quadric, Quadric, Quadric)> {
    let mut out = tri(qs);
    let ps = pairs(qs);
    let mut i = 0;
    while i < ps.len() {
        let k = crit_q(&ps[i].0, &ps[i].1);
        out.push((ps[i].0.clone(), ps[i].1.clone(), k));
        i += 1;
    }
    let mut j = 0;
    while j < qs.len() {
        out.push((qs[j].clone(), grad_plane_y(&qs[j]), grad_plane_z(&qs[j])));
        j += 1;
    }
    out
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

/// `rawEvents`: every triple's `x`s, flattened, or `None` if any triple is undecided.
pub fn raw_events(qs: &[Quadric], fuel: usize) -> Option<Vec<Num>> {
    let ts = event_triples(qs);
    let mut out: Vec<Num> = Vec::new();
    let mut ok = true;
    let mut i = 0;
    while ok && i < ts.len() {
        match propose_triple_x(&ts[i].0, &ts[i].1, &ts[i].2, fuel) {
            Some(xs) => {
                let mut j = 0;
                while j < xs.len() {
                    out.push(xs[j].clone());
                    j += 1;
                }
            }
            None => ok = false,
        }
        i += 1;
    }
    if ok { Some(out) } else { None }
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

/// **`eventsOf`**: the sweep's events, sorted and separated (`eventsOf_complete`).
pub fn events_of(qs: &[Quadric], fuel: usize) -> Option<Vec<Num>> {
    let raw = raw_events(qs, fuel)?;
    pick_events(&raw)
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
// QSample.lean: the samples
// =============================================================================================

/// `Tag`: where a sample came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Line,
    Edge,
    Vert,
}

/// `QS`: a sample, its tag and its exact covector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QS {
    pub tag: Tag,
    pub cov: Cov,
}

/// `yTurn q c`: where `q`'s trace on `x = c` turns in `y`, meets `z = ±∞`, or is `y = const`.
pub fn y_turn(q: &Quadric, c: &Q) -> Poly {
    let r = restr(q, c);
    if !q.zz.is_zero() {
        psub(&pmul(&r.1, &r.1), &pscale(&Q::from_i64(4).mul(&q.zz), &r.0))
    } else if !is_zero_p(&r.1) {
        r.1
    } else {
        r.0
    }
}

fn flatten<T: Clone>(v: Vec<Vec<T>>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let mut j = 0;
        while j < v[i].len() {
            out.push(v[i][j].clone());
            j += 1;
        }
        i += 1;
    }
    out
}

/// `turnsOn`.
pub fn turns_on(qs: &[Quadric], c: &Q) -> Option<Vec<Num>> {
    let mut out: Vec<Vec<Num>> = Vec::new();
    let mut i = 0;
    while i < qs.len() {
        out.push(roots_of(&y_turn(&qs[i], c), ROOT_FUEL)?);
        i += 1;
    }
    Some(flatten(out))
}

/// `edgesOn`.
pub fn edges_on(qs: &[Quadric], c: &Q) -> Option<Vec<ESample>> {
    let ps = pairs(qs);
    let mut out: Vec<Vec<ESample>> = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        out.push(propose_edge_any(&ps[i].0, &ps[i].1, c, ROOT_FUEL)?);
        i += 1;
    }
    Some(flatten(out))
}

/// `lineSamples qs c c'`: the samples of the line `(c, c', t)`.
pub fn line_samples(qs: &[Quadric], c: &Q, c2: &Q) -> Option<Vec<QS>> {
    let p = V3::new(c.clone(), c2.clone(), Q::zero());
    let d = V3::new(Q::zero(), Q::zero(), Q::one());
    let mut roots: Vec<Vec<Num>> = Vec::new();
    let mut i = 0;
    while i < qs.len() {
        roots.push(roots_of(&along_line(&qs[i], &p, &d), ROOT_FUEL)?);
        i += 1;
    }
    let ez = pick_events(&flatten(roots))?;
    let mut out: Vec<QS> = Vec::new();
    let mut k = 0;
    while k < ez.len() {
        out.push(QS {
            tag: Tag::Line,
            cov: cov_on(qs, &p, &d, &ez[k], SIGN_FUEL)?,
        });
        k += 1;
    }
    let ts = planes_of(&ez);
    let mut j = 0;
    while j < ts.len() {
        out.push(QS {
            tag: Tag::Line,
            cov: cov_on(qs, &p, &d, &Num::Rat(ts[j].clone()), SIGN_FUEL)?,
        });
        j += 1;
    }
    Some(out)
}

/// `edgeSample`.
pub fn edge_sample(qs: &[Quadric], e: &ESample) -> Option<QS> {
    if iso_ok_n(&e.y) && e.d_ok(SIGN_FUEL) {
        edge_cov(qs, e, SIGN_FUEL).map(|x| QS {
            tag: Tag::Edge,
            cov: x,
        })
    } else {
        None
    }
}

/// `planeSamples qs c`: the samples of the sweep plane `x = c`.
pub fn plane_samples(qs: &[Quadric], c: &Q) -> Option<Vec<QS>> {
    let es = edges_on(qs, c)?;
    let mut ev = turns_on(qs, c)?;
    let mut i = 0;
    while i < es.len() {
        ev.push(es[i].y.clone());
        i += 1;
    }
    let ey = pick_events(&ev)?;
    let ps = planes_of(&ey);
    let mut lines: Vec<Vec<QS>> = Vec::new();
    let mut j = 0;
    while j < ps.len() {
        lines.push(line_samples(qs, c, &ps[j])?);
        j += 1;
    }
    let mut out = flatten(lines);
    let mut k = 0;
    while k < es.len() {
        out.push(edge_sample(qs, &es[k])?);
        k += 1;
    }
    Some(out)
}

/// `numYQ`.
pub fn num_y_q(a: &Quadric, b: &Quadric, c: &Quadric) -> Q {
    a.lx.mul(&b.c.mul(&c.lz).sub(&b.lz.mul(&c.c)))
        .sub(&a.c.mul(&b.lx.mul(&c.lz).sub(&b.lz.mul(&c.lx))))
        .add(&a.lz.mul(&b.lx.mul(&c.c).sub(&b.c.mul(&c.lx))))
        .neg()
}

/// `numZQ`.
pub fn num_z_q(a: &Quadric, b: &Quadric, c: &Quadric) -> Q {
    a.lx.mul(&b.ly.mul(&c.c).sub(&b.c.mul(&c.ly)))
        .sub(&a.ly.mul(&b.lx.mul(&c.c).sub(&b.c.mul(&c.lx))))
        .add(&a.c.mul(&b.lx.mul(&c.ly).sub(&b.ly.mul(&c.lx))))
        .neg()
}

/// `vertSample`.
pub fn vert_sample(qs: &[Quadric], v: &VSample) -> Option<QS> {
    if iso_ok_n(&v.x) && v.d_ok(SIGN_FUEL) {
        vert_cov(qs, v, SIGN_FUEL).map(|x| QS {
            tag: Tag::Vert,
            cov: x,
        })
    } else {
        None
    }
}

/// `tripleSamples`: the vertex samples of a triple.
pub fn triple_samples(qs: &[Quadric], a: &Quadric, b: &Quadric, c: &Quadric) -> Option<Vec<QS>> {
    let nzc = |q: &Quadric| is_const_q(q) && !q.c.is_zero();
    if nzc(a) || nzc(b) || nzc(c) {
        Some(Vec::new())
    } else if is_plane_q(a) && is_plane_q(b) && is_plane_q(c) {
        let d = det3_q(a, b, c);
        if !d.is_zero() {
            let p = V3::new(
                num_x_q(a, b, c).div(&d),
                num_y_q(a, b, c).div(&d),
                num_z_q(a, b, c).div(&d),
            );
            cov_on(qs, &p, &V3::default(), &Num::Rat(Q::zero()), SIGN_FUEL).map(|x| {
                vec![QS {
                    tag: Tag::Vert,
                    cov: x,
                }]
            })
        } else {
            None
        }
    } else {
        let vs = propose_triple(a, b, c, ROOT_FUEL)?;
        let mut out: Vec<QS> = Vec::with_capacity(vs.len());
        let mut i = 0;
        while i < vs.len() {
            out.push(vert_sample(qs, &vs[i])?);
            i += 1;
        }
        Some(out)
    }
}

/// `vertSamples`.
pub fn vert_samples(qs: &[Quadric]) -> Option<Vec<QS>> {
    let ts = tri(qs);
    let mut out: Vec<Vec<QS>> = Vec::new();
    let mut i = 0;
    while i < ts.len() {
        out.push(triple_samples(qs, &ts[i].0, &ts[i].1, &ts[i].2)?);
        i += 1;
    }
    Some(flatten(out))
}

/// `samplesAt`: the samples in the frame the carriers are given in.
pub fn samples_at(qs: &[Quadric]) -> Option<Vec<QS>> {
    let e = events_of(qs, ROOT_FUEL)?;
    let ps = planes_of(&e);
    let mut out: Vec<Vec<QS>> = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        out.push(plane_samples(qs, &ps[i])?);
        i += 1;
    }
    let mut all = flatten(out);
    let vs = vert_samples(qs)?;
    let mut k = 0;
    while k < vs.len() {
        all.push(vs[k].clone());
        k += 1;
    }
    Some(all)
}

/// **`samplesOf`**: the samples in the first sheared frame where every step succeeds.
pub fn samples_of(qs: &[Quadric]) -> Option<Vec<QS>> {
    let ls = shears();
    let mut out: Option<Vec<QS>> = None;
    let mut i = 0;
    while out.is_none() && i < ls.len() {
        let mut sq: Vec<Quadric> = Vec::with_capacity(qs.len());
        let mut j = 0;
        while j < qs.len() {
            sq.push(qs[j].shear(&ls[i]));
            j += 1;
        }
        out = samples_at(&sq);
        i += 1;
    }
    out
}

// =============================================================================================
// QLedger.lean (and Core.lean's covector operations): the ledger
// =============================================================================================

/// `Cov.le`: `X` is in the closure of `Y`.
pub fn cov_le(x: &[Sgn], y: &[Sgn]) -> bool {
    let mut ok = x.len() == y.len();
    let mut i = 0;
    while ok && i < x.len() {
        ok = x[i] == Sgn::Zero || x[i] == y[i];
        i += 1;
    }
    ok
}

/// `Cov.zeroIdx`: the first zero (the length if none).
pub fn zero_idx(x: &[Sgn]) -> usize {
    let mut i = 0;
    while i < x.len() && x[i] != Sgn::Zero {
        i += 1;
    }
    i
}

fn count_zero(x: &[Sgn]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < x.len() {
        if x[i] == Sgn::Zero {
            n += 1;
        }
        i += 1;
    }
    n
}

/// `Cov.isFacet`: exactly one zero.
pub fn is_facet(x: &[Sgn]) -> bool {
    count_zero(x) == 1
}

fn set_at(x: &[Sgn], i: usize, s: Sgn) -> Cov {
    let mut out = x.to_vec();
    if i < out.len() {
        out[i] = s;
    }
    out
}

/// `Cov.plus`.
pub fn plus(x: &[Sgn]) -> Cov {
    set_at(x, zero_idx(x), Sgn::Pos)
}

/// `Cov.minus`.
pub fn minus(x: &[Sgn]) -> Cov {
    set_at(x, zero_idx(x), Sgn::Neg)
}

/// `dedup`: later repeats dropped, first occurrences kept in order.
pub fn dedup<T: Clone + PartialEq>(xs: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    let mut i = 0;
    while i < xs.len() {
        if !out.contains(&xs[i]) {
            out.push(xs[i].clone());
        }
        i += 1;
    }
    out
}

/// Lean's `Ledger` (Core.lean), as the reference builds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefLedger {
    pub nb: usize,
    pub face_body: Vec<usize>,
    pub face_cls: Vec<usize>,
    pub pieces: Vec<(Cov, usize)>,
    pub inside: Vec<(Cov, usize)>,
    pub edges: Vec<Cov>,
    pub verts: Vec<Cov>,
}

impl RefLedger {
    fn body_of(&self, f: usize) -> usize {
        if f < self.face_body.len() {
            self.face_body[f]
        } else {
            0
        }
    }
    fn cls_of(&self, f: usize) -> usize {
        if f < self.face_cls.len() {
            self.face_cls[f]
        } else {
            0
        }
    }
    /// `facets`: every piece, once.
    pub fn facets(&self) -> Vec<Cov> {
        let ps: Vec<Cov> = self.pieces.iter().map(|p| p.0.clone()).collect();
        dedup(&ps)
    }
    /// `topes`: the side cells of the pieces.
    pub fn topes(&self) -> Vec<Cov> {
        let fs = self.facets();
        let mut ts: Vec<Cov> = Vec::new();
        let mut i = 0;
        while i < fs.len() {
            ts.push(plus(&fs[i]));
            ts.push(minus(&fs[i]));
            i += 1;
        }
        dedup(&ts)
    }
    /// `splitOk`.
    pub fn split_ok(&self) -> bool {
        let mut ok = true;
        let mut i = 0;
        while ok && i < self.pieces.len() {
            let p = &self.pieces[i];
            ok = is_facet(&p.0) && zero_idx(&p.0) == self.cls_of(p.1) && self.body_of(p.1) < self.nb;
            let mut j = 0;
            while ok && j < self.pieces.len() {
                let q = &self.pieces[j];
                ok = !(q.0 == p.0 && self.body_of(q.1) == self.body_of(p.1)) || q.1 == p.1;
                j += 1;
            }
            i += 1;
        }
        ok
    }
}

/// `formOf`: body `i`'s formula (`ff` past the list).
pub fn form_of(forms: &[SForm], i: usize) -> SForm {
    if i < forms.len() {
        forms[i].clone()
    } else {
        SForm::Ff
    }
}

/// **`curvedLedger nc forms ss`**.
pub fn curved_ledger(nc: usize, forms: &[SForm], ss: &[QS]) -> RefLedger {
    let nb = forms.len();
    let all: Vec<Cov> = ss.iter().map(|s| s.cov.clone()).collect();
    let covs = dedup(&all);
    let mut pieces: Vec<(Cov, usize)> = Vec::new();
    let mut k = 0;
    while k < covs.len() {
        let phi = &covs[k];
        if is_facet(phi) {
            let mut i = 0;
            while i < nb {
                let f = form_of(forms, i);
                if f.on_cov(&plus(phi)) != f.on_cov(&minus(phi)) {
                    pieces.push((phi.clone(), i * nc + zero_idx(phi)));
                }
                i += 1;
            }
        }
        k += 1;
    }
    let mut face_body: Vec<usize> = Vec::with_capacity(nb * nc);
    let mut face_cls: Vec<usize> = Vec::with_capacity(nb * nc);
    let mut f = 0;
    while f < nb * nc {
        face_body.push(f / nc);
        face_cls.push(f % nc);
        f += 1;
    }
    let mut l = RefLedger {
        nb,
        face_body,
        face_cls,
        pieces,
        inside: Vec::new(),
        edges: Vec::new(),
        verts: Vec::new(),
    };
    let ts = l.topes();
    let mut inside: Vec<(Cov, usize)> = Vec::new();
    let mut t = 0;
    while t < ts.len() {
        let mut i = 0;
        while i < nb {
            if form_of(forms, i).on_cov(&ts[t]) {
                inside.push((ts[t].clone(), i));
            }
            i += 1;
        }
        t += 1;
    }
    let fs = l.facets();
    let below = |e: &Cov| fs.iter().any(|ph| cov_le(e, ph));
    let ecand: Vec<Cov> = ss
        .iter()
        .filter(|s| s.tag != Tag::Vert && count_zero(&s.cov) >= 2)
        .map(|s| s.cov.clone())
        .collect();
    let edges: Vec<Cov> = dedup(&ecand).into_iter().filter(|e| below(e)).collect();
    let vcand: Vec<Cov> = ss
        .iter()
        .filter(|s| s.tag == Tag::Vert)
        .map(|s| s.cov.clone())
        .collect();
    let verts: Vec<Cov> = dedup(&vcand)
        .into_iter()
        .filter(|v| !ss.iter().any(|s| s.tag != Tag::Vert && s.cov == *v) && below(v))
        .collect();
    l.inside = inside;
    l.edges = edges;
    l.verts = verts;
    l
}

/// `sidesOk`: both sides of every facet are sampled.
pub fn sides_ok(l: &RefLedger, ss: &[QS]) -> bool {
    let fs = l.facets();
    fs.iter().all(|ph| {
        let (p, m) = (plus(ph), minus(ph));
        ss.iter().any(|s| s.cov == p) && ss.iter().any(|s| s.cov == m)
    })
}

/// **`qledger qs forms`**: the curved ledger, checked (`splitOk`, `sidesOk`), or `None`.
pub fn qledger(qs: &[Quadric], forms: &[SForm]) -> Option<RefLedger> {
    let ss = samples_of(qs)?;
    let l = curved_ledger(qs.len(), forms, &ss);
    if l.split_ok() && sides_ok(&l, &ss) {
        Some(l)
    } else {
        None
    }
}

// =============================================================================================
// QArc.lean: crossings on a vertex box's faces
// =============================================================================================

impl Quadric {
    /// `Quadric.swapXY`: `q(y, x, z)`.
    pub fn swap_xy(&self) -> Quadric {
        Quadric {
            xx: self.yy.clone(),
            yy: self.xx.clone(),
            zz: self.zz.clone(),
            xy: self.xy.clone(),
            xz: self.yz.clone(),
            yz: self.xz.clone(),
            lx: self.ly.clone(),
            ly: self.lx.clone(),
            lz: self.lz.clone(),
            c: self.c.clone(),
        }
    }
    /// `Quadric.swapXZ`: `q(z, y, x)`.
    pub fn swap_xz(&self) -> Quadric {
        Quadric {
            xx: self.zz.clone(),
            yy: self.yy.clone(),
            zz: self.xx.clone(),
            xy: self.yz.clone(),
            xz: self.xz.clone(),
            yz: self.xy.clone(),
            lx: self.lz.clone(),
            ly: self.ly.clone(),
            lz: self.lx.clone(),
            c: self.c.clone(),
        }
    }
}

/// `Axis`: a box face's normal axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// `Axis.perm`: the swap that makes the face `a = c` the plane `x = c`.
pub fn axis_perm(a: Axis, q: &Quadric) -> Quadric {
    match a {
        Axis::X => q.clone(),
        Axis::Y => q.swap_xy(),
        Axis::Z => q.swap_xz(),
    }
}

/// `faceCrossings a c qj qk fuel`: the first shear for which the face's crossings are proposed.
pub fn face_crossings(a: Axis, c: &Q, qj: &Quadric, qk: &Quadric, fuel: usize) -> Option<(Q, Vec<ESample>)> {
    let ls = shears();
    let mut out: Option<(Q, Vec<ESample>)> = None;
    let mut i = 0;
    while out.is_none() && i < ls.len() {
        out = propose_edge_any(
            &axis_perm(a, qj).shear(&ls[i]),
            &axis_perm(a, qk).shear(&ls[i]),
            c,
            fuel,
        )
        .map(|es| (ls[i].clone(), es));
        i += 1;
    }
    out
}

/// `boxFaces lo hi`.
pub fn box_faces(lo: &V3, hi: &V3) -> Vec<(Axis, Q)> {
    vec![
        (Axis::X, lo.x.clone()),
        (Axis::X, hi.x.clone()),
        (Axis::Y, lo.y.clone()),
        (Axis::Y, hi.y.clone()),
        (Axis::Z, lo.z.clone()),
        (Axis::Z, hi.z.clone()),
    ]
}

/// A face's crossings: its axis and plane, the shear used, and the samples.
pub type FaceCut = (Axis, Q, Q, Vec<ESample>);

/// **`boxCrossings`**: the crossings of `q_j ∩ q_k` with every face of the box
/// (`boxCrossings_complete`).
pub fn box_crossings(qj: &Quadric, qk: &Quadric, lo: &V3, hi: &V3, fuel: usize) -> Option<Vec<FaceCut>> {
    let fs = box_faces(lo, hi);
    let mut out: Vec<FaceCut> = Vec::with_capacity(6);
    let mut i = 0;
    while i < fs.len() {
        let (l, es) = face_crossings(fs[i].0, &fs[i].1, qj, qk, fuel)?;
        out.push((fs[i].0, fs[i].1.clone(), l, es));
        i += 1;
    }
    Some(out)
}

/// `faceCov`: a crossing's covector over the carriers, in the face's frame.
pub fn face_cov(qs: &[Quadric], a: Axis, l: &Q, e: &ESample) -> Option<Cov> {
    if iso_ok_n(&e.y) && e.d_ok(SIGN_FUEL) {
        let fq: Vec<Quadric> = qs.iter().map(|q| axis_perm(a, q).shear(l)).collect();
        edge_cov(&fq, e, SIGN_FUEL)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn sphere1() -> Quadric {
        quad([1, 1, 1, 0, 0, 0, 0, 0, 0, -1])
    }
    fn plane_z() -> Quadric {
        quad([0, 0, 0, 0, 0, 0, 0, 0, 1, 0])
    }

    /// QSweep.lean's #guards.
    #[test]
    fn qsweep_guards() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        assert_eq!(
            crit_q(&sphere1(), &plane_z()),
            quad([0, 0, 0, 0, 0, 0, 0, 2, 0, 0])
        );
        let e = events_of(&[sphere1(), plane_y0.clone(), plane_z()], 30).unwrap();
        let lohi: Vec<(Q, Q)> = e.iter().map(|n| (n.lo(), n.hi())).collect();
        assert_eq!(lohi, vec![(qi(-1), qi(-1)), (qi(1), qi(1))]);
        assert_eq!(planes_of(&e), vec![qi(-2), qi(0), qi(2)]);
    }

    /// QSample.lean's #guard: 27 distinct covectors, one a vertex sample with covector (0, 0, 0).
    #[test]
    fn qsample_guard() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let ss = samples_of(&[sphere1(), plane_y0, plane_z()]).unwrap();
        let covs: Vec<Cov> = ss.iter().map(|s| s.cov.clone()).collect();
        assert_eq!(dedup(&covs).len(), 27);
        assert!(
            ss.iter()
                .any(|s| s.tag == Tag::Vert && s.cov == vec![Sgn::Zero, Sgn::Zero, Sgn::Zero])
        );
    }

    /// QLedger.lean's #guards: the ball cut by y = 0 and z = 0 has 4 facets, 4 edges, 1 vertex.
    #[test]
    fn qledger_guards() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        let l = qledger(&[sphere1(), plane_y0, plane_z()], &[SForm::Lit(0, Sgn::Neg)]).unwrap();
        assert_eq!(l.facets().len(), 4);
        assert_eq!(l.edges.len(), 4);
        assert_eq!(l.verts.len(), 1);
    }

    /// QArc.lean's #guard: the unit circle in z = 0 against the box [−1/2, 3/2] × [−1/2, 1/2]².
    #[test]
    fn qarc_guard() {
        let lo = V3::new(q(-1, 2), q(-1, 2), q(-1, 2));
        let hi = V3::new(q(3, 2), q(1, 2), q(1, 2));
        let cs = box_crossings(&sphere1(), &plane_z(), &lo, &hi, ROOT_FUEL).unwrap();
        let lens: Vec<usize> = cs.iter().map(|c| c.3.len()).collect();
        assert_eq!(lens, vec![2, 0, 2, 2, 0, 0]);
        assert!(cs.iter().all(|c| c.2.is_zero()));
    }
}
