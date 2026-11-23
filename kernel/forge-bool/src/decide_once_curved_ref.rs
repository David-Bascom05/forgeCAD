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
use forge_alg::{Num, Poly, Q, is_zero_p, pmul, pscale, psub, roots_of};
use forge_bool_core::Sgn;

/// A covector (Lean's `Cov`).
pub type Cov = Vec<Sgn>;

// =============================================================================================
// QSweep.lean: event assembly over the whole arrangement (the reference's)
// =============================================================================================

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

/// **`eventsOf`**: the sweep's events, sorted and separated (`eventsOf_complete`).
pub fn events_of(qs: &[Quadric], fuel: usize) -> Option<Vec<Num>> {
    let raw = raw_events(qs, fuel)?;
    pick_events(&raw)
}

// =============================================================================================
// QSample.lean (at 7ab0596): the samples
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

/// `zeroOn q c`: `q` vanishes on the whole plane `x = c`.
pub fn zero_on(q: &Quadric, c: &Q) -> bool {
    let r = restr(q, c);
    is_zero_p(&r.0) && is_zero_p(&r.1) && is_zero_p(&r.2)
}

/// `activeOn qs c`: the carriers not vanishing on the plane `x = c`.
pub fn active_on(qs: &[Quadric], c: &Q) -> Vec<Quadric> {
    qs.iter().filter(|q| !zero_on(q, c)).cloned().collect()
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

/// `lineSamples qs c c'`: the samples of the line `(c, c', t)`, roots of the active carriers.
pub fn line_samples(qs: &[Quadric], c: &Q, c2: &Q) -> Option<Vec<QS>> {
    let p = V3::new(c.clone(), c2.clone(), Q::zero());
    let d = V3::new(Q::zero(), Q::zero(), Q::one());
    let act = active_on(qs, c);
    let mut roots: Vec<Vec<Num>> = Vec::new();
    let mut i = 0;
    while i < act.len() {
        roots.push(roots_of(&along_line(&act[i], &p, &d), ROOT_FUEL)?);
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

/// `edgeSample qs tg e`.
pub fn edge_sample(qs: &[Quadric], tg: Tag, e: &ESample) -> Option<QS> {
    if iso_ok_n(&e.y) && e.d_ok(SIGN_FUEL) {
        edge_cov(qs, e, SIGN_FUEL).map(|x| QS { tag: tg, cov: x })
    } else {
        None
    }
}

/// `planeSamples qs onEvent c`: the samples of the plane `x = c`. On an event plane, the
/// crossings are points of three carriers, tagged `vert`.
pub fn plane_samples(qs: &[Quadric], on_event: bool, c: &Q) -> Option<Vec<QS>> {
    let act = active_on(qs, c);
    let nx: Vec<Quadric> = act.iter().filter(|q| !is_x_plane(q)).cloned().collect();
    let es = edges_on(&nx, c)?;
    let mut ev = turns_on(&act, c)?;
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
    let tg = if on_event { Tag::Vert } else { Tag::Edge };
    let mut k = 0;
    while k < es.len() {
        out.push(edge_sample(qs, tg, &es[k])?);
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

/// `lineVerts qs a b q`: the vertex samples on the line of two planes.
pub fn line_verts(qs: &[Quadric], a: &Quadric, b: &Quadric, q: &Quadric) -> Option<Vec<QS>> {
    let l = line_of(a, b);
    if !line_ok(a, b, &l) {
        return None;
    }
    let ns = roots_of(&along_line(q, &l.0, &l.1), ROOT_FUEL)?;
    if !ns.iter().all(iso_ok_n) {
        return None;
    }
    let mut out: Vec<QS> = Vec::with_capacity(ns.len());
    let mut i = 0;
    while i < ns.len() {
        out.push(QS {
            tag: Tag::Vert,
            cov: cov_on(qs, &l.0, &l.1, &ns[i], SIGN_FUEL)?,
        });
        i += 1;
    }
    Some(out)
}

/// `lineTripleS`.
pub fn line_triple_s(qs: &[Quadric], a: &Quadric, b: &Quadric, c: &Quadric) -> Option<Vec<QS>> {
    if line_ok(a, b, &line_of(a, b)) {
        line_verts(qs, a, b, c)
    } else if line_ok(a, c, &line_of(a, c)) {
        line_verts(qs, a, c, b)
    } else if line_ok(b, c, &line_of(b, c)) {
        line_verts(qs, b, c, a)
    } else {
        None
    }
}

fn nzc(q: &Quadric) -> bool {
    is_const_q(q) && !q.c.is_zero()
}

/// `tripleSamples`: the vertex samples of a triple.
pub fn triple_samples(qs: &[Quadric], a: &Quadric, b: &Quadric, c: &Quadric) -> Option<Vec<QS>> {
    if nzc(a) || nzc(b) || nzc(c) || parallel_apart(a, b) || parallel_apart(a, c) || parallel_apart(b, c) {
        return Some(Vec::new());
    }
    let lt = line_triple_s(qs, a, b, c);
    if lt.is_some() {
        lt
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

// =============================================================================================
// QEnum.lean: every common point of a triple, enumerated
// =============================================================================================

/// `PtD`: a point descriptor, a vertex sample or a root on two planes' line.
#[derive(Clone, Debug)]
pub enum PtD {
    Vs(VSample),
    Ln(V3, V3, Num),
}

/// **`enumTriple`**: the descriptors of a triple's common points (`None`: undecided).
/// `enumTriple_complete`: every common point is named by one.
pub fn enum_triple(a: &Quadric, b: &Quadric, c: &Quadric, fuel: usize) -> Option<Vec<PtD>> {
    let ln = |x: &Quadric, y: &Quadric, q: &Quadric| -> Option<Vec<PtD>> {
        let l = line_of(x, y);
        roots_of(&along_line(q, &l.0, &l.1), fuel).map(|ns| {
            ns.into_iter()
                .map(|n| PtD::Ln(l.0.clone(), l.1.clone(), n))
                .collect()
        })
    };
    if nzc(a) || nzc(b) || nzc(c) || parallel_apart(a, b) || parallel_apart(a, c) || parallel_apart(b, c) {
        Some(Vec::new())
    } else if line_ok(a, b, &line_of(a, b)) {
        ln(a, b, c)
    } else if line_ok(a, c, &line_of(a, c)) {
        ln(a, c, b)
    } else if line_ok(b, c, &line_of(b, c)) {
        ln(b, c, a)
    } else if is_plane_q(a) && is_plane_q(b) && is_plane_q(c) {
        None
    } else {
        propose_triple(a, b, c, fuel).map(|vs| vs.into_iter().map(PtD::Vs).collect())
    }
}

impl PtD {
    /// `PtD.cov`: the descriptor's covector (`cov_spec`).
    pub fn cov(&self, qs: &[Quadric]) -> Option<Cov> {
        match self {
            PtD::Vs(v) => {
                if iso_ok_n(&v.x) && v.d_ok(SIGN_FUEL) {
                    vert_cov(qs, v, SIGN_FUEL)
                } else {
                    None
                }
            }
            PtD::Ln(p, d, n) => {
                if iso_ok_n(n) {
                    cov_on(qs, p, d, n, SIGN_FUEL)
                } else {
                    None
                }
            }
        }
    }
}

/// `critTriples`: every pair with its turning-point quadric.
pub fn crit_triples(qs: &[Quadric]) -> Vec<(Quadric, Quadric, Quadric)> {
    pairs(qs)
        .into_iter()
        .map(|(a, b)| {
            let c = crit_q(&a, &b);
            (a, b, c)
        })
        .collect()
}

/// `enumSamples`: a triple's vertex samples, one per common point.
pub fn enum_samples(qs: &[Quadric], t: &(Quadric, Quadric, Quadric)) -> Option<Vec<QS>> {
    let ds = enum_triple(&t.0, &t.1, &t.2, ROOT_FUEL)?;
    let mut out: Vec<QS> = Vec::with_capacity(ds.len());
    let mut i = 0;
    while i < ds.len() {
        out.push(QS {
            tag: Tag::Vert,
            cov: ds[i].cov(qs)?,
        });
        i += 1;
    }
    Some(out)
}

/// **`vertSamples`**: every common point of three carriers, and every turn and tangency of two.
pub fn vert_samples(qs: &[Quadric]) -> Option<Vec<QS>> {
    let mut ts = tri(qs);
    ts.extend(crit_triples(qs));
    let mut out: Vec<Vec<QS>> = Vec::new();
    let mut i = 0;
    while i < ts.len() {
        out.push(enum_samples(qs, &ts[i])?);
        i += 1;
    }
    Some(flatten(out))
}

/// `ratEvents`: the rational events.
pub fn rat_events(e: &[Num]) -> Vec<Q> {
    let mut out: Vec<Q> = Vec::new();
    let mut i = 0;
    while i < e.len() {
        if let Num::Rat(q) = &e[i] {
            out.push(q.clone());
        }
        i += 1;
    }
    out
}

/// `samplesAt`: the samples in the frame the carriers are given in.
pub fn samples_at(qs: &[Quadric]) -> Option<Vec<QS>> {
    let e = events_of(qs, ROOT_FUEL)?;
    let ps = planes_of(&e);
    let mut out: Vec<Vec<QS>> = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        out.push(plane_samples(qs, false, &ps[i])?);
        i += 1;
    }
    let re = rat_events(&e);
    let mut k = 0;
    while k < re.len() {
        if qs.iter().any(|q| zero_on(q, &re[k])) {
            out.push(plane_samples(qs, true, &re[k])?);
        }
        k += 1;
    }
    let mut all = flatten(out);
    let vs = vert_samples(qs)?;
    let mut j = 0;
    while j < vs.len() {
        all.push(vs[j].clone());
        j += 1;
    }
    Some(all)
}

/// **`samplesOfF`**: the samples in the first frame that passes `frame_ok` and `sing_all` and
/// where every step succeeds, with that frame.
pub fn samples_of_f(qs: &[Quadric]) -> Option<(Frame, Vec<QS>)> {
    let fs = frames();
    let mut out: Option<(Frame, Vec<QS>)> = None;
    let mut i = 0;
    while out.is_none() && i < fs.len() {
        let fq: Vec<Quadric> = qs.iter().map(|q| in_frame(&fs[i], q)).collect();
        if frame_ok(&fq) && sing_all(&fq) {
            out = samples_at(&fq).map(|ss| (fs[i].clone(), ss));
        }
        i += 1;
    }
    out
}

/// `samplesOf`: the samples, without their frame.
pub fn samples_of(qs: &[Quadric]) -> Option<Vec<QS>> {
    samples_of_f(qs).map(|p| p.1)
}

// =============================================================================================
// QSing.lean: every carrier's singular zeros lie on other carriers, certified
// =============================================================================================

/// `gradQ q p`.
pub fn grad_q(q: &Quadric, p: &V3) -> V3 {
    let two = Q::from_i64(2);
    V3::new(
        two.mul(&q.xx)
            .mul(&p.x)
            .add(&q.xy.mul(&p.y))
            .add(&q.xz.mul(&p.z))
            .add(&q.lx),
        q.xy.mul(&p.x)
            .add(&two.mul(&q.yy).mul(&p.y))
            .add(&q.yz.mul(&p.z))
            .add(&q.ly),
        q.xz.mul(&p.x)
            .add(&q.yz.mul(&p.y))
            .add(&two.mul(&q.zz).mul(&p.z))
            .add(&q.lz),
    )
}

/// `hessQ q w`.
pub fn hess_q(q: &Quadric, w: &V3) -> V3 {
    let two = Q::from_i64(2);
    V3::new(
        two.mul(&q.xx).mul(&w.x).add(&q.xy.mul(&w.y)).add(&q.xz.mul(&w.z)),
        q.xy.mul(&w.x).add(&two.mul(&q.yy).mul(&w.y)).add(&q.yz.mul(&w.z)),
        q.xz.mul(&w.x).add(&q.yz.mul(&w.y)).add(&two.mul(&q.zz).mul(&w.z)),
    )
}

/// `detH q`.
pub fn det_h(q: &Quadric) -> Q {
    let two = Q::from_i64(2);
    let (a, b, c) = (two.mul(&q.xx), two.mul(&q.yy), two.mul(&q.zz));
    a.mul(&b.mul(&c).sub(&q.yz.mul(&q.yz)))
        .sub(&q.xy.mul(&q.xy.mul(&c).sub(&q.yz.mul(&q.xz))))
        .add(&q.xz.mul(&q.xy.mul(&q.yz).sub(&b.mul(&q.xz))))
}

/// `SingCert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SingCert {
    Plane,
    Point(V3),
    Apex(V3, usize),
    None_(V3),
}

/// `SingCert.ok qs j q`.
pub fn sing_ok(qs: &[Quadric], j: usize, q: &Quadric, c: &SingCert) -> bool {
    match c {
        SingCert::Plane => is_plane_q(q) && !(q.lx.is_zero() && q.ly.is_zero() && q.lz.is_zero()),
        SingCert::Point(p0) => grad_q(q, p0).is_zero() && !q.eval(p0).is_zero(),
        SingCert::Apex(p0, m) => {
            grad_q(q, p0).is_zero()
                && !det_h(q).is_zero()
                && *m != j
                && *m < qs.len()
                && qs[*m].eval(p0).is_zero()
        }
        SingCert::None_(w) => {
            hess_q(q, w).is_zero() && !w.x.mul(&q.lx).add(&w.y.mul(&q.ly)).add(&w.z.mul(&q.lz)).is_zero()
        }
    }
}

/// `solve3 A b`: `A p = b` solved by row reduction of `[A | b | I]` (free variables zero), or a
/// `y` with `y A = 0` and `y · b ≠ 0`.
#[allow(clippy::result_large_err)]
pub fn solve3(a: &[[Q; 3]; 3], b: &[Q; 3]) -> Result<V3, V3> {
    let mut m: Vec<Vec<Q>> = Vec::with_capacity(3);
    let mut i = 0;
    while i < 3 {
        let mut row: Vec<Q> = vec![a[i][0].clone(), a[i][1].clone(), a[i][2].clone(), b[i].clone()];
        let mut k = 0;
        while k < 3 {
            row.push(if k == i { Q::one() } else { Q::zero() });
            k += 1;
        }
        m.push(row);
        i += 1;
    }
    let mut row = 0;
    let mut piv: Vec<(usize, usize)> = Vec::new();
    let mut col = 0;
    while col < 3 {
        if row < 3 {
            let mut k = 0;
            let mut found: Option<usize> = None;
            while found.is_none() && k < 3 - row {
                if !m[row + k][col].is_zero() {
                    found = Some(k);
                }
                k += 1;
            }
            if let Some(k) = found {
                let r = row + k;
                m.swap(row, r);
                let pv = m[row][col].clone();
                let mut t = 0;
                while t < 7 {
                    m[row][t] = m[row][t].div(&pv);
                    t += 1;
                }
                let mut i2 = 0;
                while i2 < 3 {
                    if i2 != row {
                        let f = m[i2][col].clone();
                        let mut t2 = 0;
                        while t2 < 7 {
                            let v = m[i2][t2].sub(&f.mul(&m[row][t2]));
                            m[i2][t2] = v;
                            t2 += 1;
                        }
                    }
                    i2 += 1;
                }
                piv.push((row, col));
                row += 1;
            }
        }
        col += 1;
    }
    let mut i3 = row;
    let mut bad: Option<V3> = None;
    while bad.is_none() && i3 < 3 {
        if !m[i3][3].is_zero() {
            bad = Some(V3::new(m[i3][4].clone(), m[i3][5].clone(), m[i3][6].clone()));
        }
        i3 += 1;
    }
    match bad {
        Some(y) => Err(y),
        None => {
            let mut sol = [Q::zero(), Q::zero(), Q::zero()];
            let mut k = 0;
            while k < piv.len() {
                sol[piv[k].1] = m[piv[k].0][3].clone();
                k += 1;
            }
            let [x, y, z] = sol;
            Ok(V3::new(x, y, z))
        }
    }
}

/// `proposeSing qs j q`.
pub fn propose_sing(qs: &[Quadric], j: usize, q: &Quadric) -> SingCert {
    if is_plane_q(q) {
        return SingCert::Plane;
    }
    let two = Q::from_i64(2);
    let h = [
        [two.mul(&q.xx), q.xy.clone(), q.xz.clone()],
        [q.xy.clone(), two.mul(&q.yy), q.yz.clone()],
        [q.xz.clone(), q.yz.clone(), two.mul(&q.zz)],
    ];
    match solve3(&h, &[q.lx.neg(), q.ly.neg(), q.lz.neg()]) {
        Err(w) => SingCert::None_(w),
        Ok(p0) => {
            if !q.eval(&p0).is_zero() {
                SingCert::Point(p0)
            } else {
                let mut m = 0;
                let mut found: Option<usize> = None;
                while found.is_none() && m < qs.len() {
                    if m != j && qs[m].eval(&p0).is_zero() {
                        found = Some(m);
                    }
                    m += 1;
                }
                match found {
                    Some(m) => SingCert::Apex(p0, m),
                    None => SingCert::Point(p0),
                }
            }
        }
    }
}

/// **`singAll`**.
pub fn sing_all(qs: &[Quadric]) -> bool {
    let mut ok = true;
    let mut j = 0;
    while ok && j < qs.len() {
        ok = sing_ok(qs, j, &qs[j], &propose_sing(qs, j, &qs[j]));
        j += 1;
    }
    ok
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

/// `zeroIdxs`: a covector's zero positions.
pub fn zero_idxs(x: &[Sgn]) -> Vec<usize> {
    (0..x.len()).filter(|&i| x[i] == Sgn::Zero).collect()
}

/// `uTriple`: the triple whose common points hold every point of the vertex candidate `v`: three
/// of its zero carriers, or, with two zeros, the pair and its turning-point quadric.
pub fn u_triple(qs: &[Quadric], v: &[Sgn]) -> Option<(Quadric, Quadric, Quadric)> {
    let z = zero_idxs(v);
    let g = |i: usize| qs.get(i).cloned().unwrap_or_default();
    if z.len() >= 3 {
        Some((g(z[0]), g(z[1]), g(z[2])))
    } else if z.len() == 2 {
        let (a, b) = (g(z[0]), g(z[1]));
        let c = crit_q(&a, &b);
        Some((a, b, c))
    } else {
        None
    }
}

/// **`vertOneAt`, (U)**: the triple's complete list has a covector for every point, and exactly
/// one carries `v` (`vertOneAt_spec`: two points with covector `v` on the triple are equal).
pub fn vert_one_at(qs: &[Quadric], v: &[Sgn]) -> bool {
    match u_triple(qs, v) {
        Some(t) => match enum_triple(&t.0, &t.1, &t.2, ROOT_FUEL) {
            Some(ds) => {
                let cs: Vec<Option<Cov>> = ds.iter().map(|d| d.cov(qs)).collect();
                cs.iter().all(|c| c.is_some()) && cs.iter().filter(|c| c.as_deref() == Some(v)).count() == 1
            }
            None => false,
        },
        None => false,
    }
}

/// **`qledger qs forms`**: the curved ledger, checked (`splitOk`, `sidesOk`, and (U) on every
/// vertex candidate in the sampling frame), or `None`.
pub fn qledger(qs: &[Quadric], forms: &[SForm]) -> Option<RefLedger> {
    let (f, ss) = samples_of_f(qs)?;
    let l = curved_ledger(qs.len(), forms, &ss);
    let fq: Vec<Quadric> = qs.iter().map(|q| in_frame(&f, q)).collect();
    if l.split_ok() && sides_ok(&l, &ss) && l.verts.iter().all(|v| vert_one_at(&fq, v)) {
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

// =============================================================================================
// QBound.lean: the bounding sphere, certified
// =============================================================================================

/// `qadd`.
pub fn qadd(p: &Quadric, q: &Quadric) -> Quadric {
    Quadric {
        xx: p.xx.add(&q.xx),
        yy: p.yy.add(&q.yy),
        zz: p.zz.add(&q.zz),
        xy: p.xy.add(&q.xy),
        xz: p.xz.add(&q.xz),
        yz: p.yz.add(&q.yz),
        lx: p.lx.add(&q.lx),
        ly: p.ly.add(&q.ly),
        lz: p.lz.add(&q.lz),
        c: p.c.add(&q.c),
    }
}

/// `qscale k q`.
pub fn qscale(k: &Q, q: &Quadric) -> Quadric {
    Quadric {
        xx: k.mul(&q.xx),
        yy: k.mul(&q.yy),
        zz: k.mul(&q.zz),
        xy: k.mul(&q.xy),
        xz: k.mul(&q.xz),
        yz: k.mul(&q.yz),
        lx: k.mul(&q.lx),
        ly: k.mul(&q.ly),
        lz: k.mul(&q.lz),
        c: k.mul(&q.c),
    }
}

/// `qsumL`.
pub fn qsum_l(l: &[Quadric]) -> Quadric {
    let mut acc = Quadric::default();
    let mut i = l.len();
    while i > 0 {
        i -= 1;
        acc = qadd(&l[i], &acc);
    }
    acc
}

fn qr(n: i64, d: i64) -> Q {
    Q::from_i64(n).div(&Q::from_i64(d))
}

/// `bCentre`.
pub fn b_centre() -> (Q, Q, Q) {
    (qr(1, 7), qr(1, 11), qr(1, 13))
}

/// `sphereR R2`: `|p − c|² − R2`, `c = bCentre`.
pub fn sphere_r(r2: &Q) -> Quadric {
    let (a, b, c) = b_centre();
    let m2 = Q::from_i64(-2);
    Quadric {
        xx: Q::one(),
        yy: Q::one(),
        zz: Q::one(),
        lx: m2.mul(&a),
        ly: m2.mul(&b),
        lz: m2.mul(&c),
        c: a.mul(&a).add(&b.mul(&b)).add(&c.mul(&c)).sub(r2),
        ..Quadric::default()
    }
}

/// `conjs`: the top-level conjuncts.
pub fn conjs(f: &SForm) -> Vec<SForm> {
    match f {
        SForm::And(a, b) => {
            let mut v = conjs(a);
            v.extend(conjs(b));
            v
        }
        g => vec![g.clone()],
    }
}

/// `BTerm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BTerm {
    One(usize, Sgn),
    Two(usize, usize, Sgn, Sgn),
}

impl BTerm {
    /// `BTerm.lits`.
    pub fn lits(&self) -> Vec<SForm> {
        match self {
            BTerm::One(j, s) => vec![SForm::Lit(*j, *s)],
            BTerm::Two(j, k, sj, sk) => vec![SForm::Lit(*j, *sj), SForm::Lit(*k, *sk)],
        }
    }
    /// `BTerm.quad`.
    pub fn quad(&self, qs: &[Quadric]) -> Option<Quadric> {
        match self {
            BTerm::One(j, s) => lit_q(qs, *j, *s),
            BTerm::Two(j, k, sj, sk) => {
                let f = lit_aff(qs, *j, *sj)?;
                let g = lit_aff(qs, *k, *sk)?;
                Some(qscale(&Q::from_i64(-1), &aff_mul(&f, &g)))
            }
        }
    }
}

/// `litQ`.
pub fn lit_q(qs: &[Quadric], j: usize, s: Sgn) -> Option<Quadric> {
    if j >= qs.len() {
        return None;
    }
    match s {
        Sgn::Neg => Some(qs[j].clone()),
        Sgn::Pos => Some(qscale(&Q::from_i64(-1), &qs[j])),
        Sgn::Zero => None,
    }
}

/// `litAff`.
pub fn lit_aff(qs: &[Quadric], j: usize, s: Sgn) -> Option<Aff> {
    if j >= qs.len() || !is_plane_q(&qs[j]) {
        return None;
    }
    let q = &qs[j];
    match s {
        Sgn::Neg => Some((q.lx.clone(), q.ly.clone(), q.lz.clone(), q.c.clone())),
        Sgn::Pos => Some((q.lx.neg(), q.ly.neg(), q.lz.neg(), q.c.neg())),
        Sgn::Zero => None,
    }
}

/// `BCert`.
#[derive(Clone, Debug)]
pub struct BCert {
    pub terms: Vec<(Q, BTerm)>,
    pub sos: Vec<(Q, Aff)>,
}

/// **`certOk`**: the body lies strictly inside the bounding sphere.
pub fn cert_ok(qs: &[Quadric], f: &SForm, r2: &Q, c: &BCert) -> bool {
    let cs = conjs(f);
    if !(c.terms.iter().all(|t| t.0.signum() >= 0)
        && c.terms.iter().any(|t| t.0.signum() > 0)
        && c.sos.iter().all(|t| t.0.signum() >= 0)
        && c.terms.iter().all(|t| t.1.lits().iter().all(|g| cs.contains(g))))
    {
        return false;
    }
    let mut qts: Vec<Quadric> = Vec::with_capacity(c.terms.len());
    let mut i = 0;
    while i < c.terms.len() {
        match c.terms[i].1.quad(qs) {
            Some(q) => qts.push(qscale(&c.terms[i].0, &q)),
            None => return false,
        }
        i += 1;
    }
    let lhs = qsub(&qsum_l(&qts), &sphere_r(r2));
    let sq: Vec<Quadric> = c.sos.iter().map(|t| qscale(&t.0, &aff_mul(&t.1, &t.1))).collect();
    lhs == qsum_l(&sq)
}

/// `ldl A`: `LDLᵀ` of a symmetric 4×4 matrix, the pivots and the unit lower columns.
pub fn ldl(a: &[[Q; 4]; 4]) -> Option<Vec<(Q, Aff)>> {
    let n = 4;
    let mut l: Vec<Vec<Q>> = vec![vec![Q::zero(); n]; n];
    let mut d: Vec<Q> = vec![Q::zero(); n];
    let mut i = 0;
    while i < n {
        let mut di = a[i][i].clone();
        let mut k = 0;
        while k < i {
            di = di.sub(&l[i][k].mul(&l[i][k]).mul(&d[k]));
            k += 1;
        }
        d[i] = di.clone();
        l[i][i] = Q::one();
        let mut j = i + 1;
        while j < n {
            let mut v = a[j][i].clone();
            let mut k2 = 0;
            while k2 < i {
                v = v.sub(&l[j][k2].mul(&l[i][k2]).mul(&d[k2]));
                k2 += 1;
            }
            if di.is_zero() {
                if !v.is_zero() {
                    return None;
                }
                l[j][i] = Q::zero();
            } else {
                l[j][i] = v.div(&di);
            }
            j += 1;
        }
        i += 1;
    }
    if d.iter().any(|x| x.signum() < 0) {
        return None;
    }
    let mut out: Vec<(Q, Aff)> = Vec::with_capacity(n);
    let mut c = 0;
    while c < n {
        out.push((
            d[c].clone(),
            (l[0][c].clone(), l[1][c].clone(), l[2][c].clone(), l[3][c].clone()),
        ));
        c += 1;
    }
    Some(out)
}

/// `qmat q`.
pub fn qmat(q: &Quadric) -> [[Q; 4]; 4] {
    let h = qr(1, 2);
    [
        [q.xx.clone(), q.xy.mul(&h), q.xz.mul(&h), q.lx.mul(&h)],
        [q.xy.mul(&h), q.yy.clone(), q.yz.mul(&h), q.ly.mul(&h)],
        [q.xz.mul(&h), q.yz.mul(&h), q.zz.clone(), q.lz.mul(&h)],
        [q.lx.mul(&h), q.ly.mul(&h), q.lz.mul(&h), q.c.clone()],
    ]
}

/// `bodyTerms`: the non-plane literals, and the pairs of parallel plane literals.
pub fn body_terms(qs: &[Quadric], f: &SForm) -> Vec<BTerm> {
    let mut lits: Vec<(usize, Sgn)> = Vec::new();
    for g in conjs(f) {
        if let SForm::Lit(j, s) = g
            && s != Sgn::Zero
        {
            lits.push((j, s));
        }
    }
    let mut ones: Vec<BTerm> = Vec::new();
    let mut pl: Vec<(usize, Sgn)> = Vec::new();
    for &(j, s) in &lits {
        if j < qs.len() {
            if is_plane_q(&qs[j]) {
                pl.push((j, s));
            } else {
                ones.push(BTerm::One(j, s));
            }
        }
    }
    let mut twos: Vec<BTerm> = Vec::new();
    let mut i = 0;
    while i < pl.len() {
        let mut k = i + 1;
        while k < pl.len() {
            let (j, sj) = pl[i];
            let (kk, sk) = pl[k];
            if nrm(&qs[j]).cross(&nrm(&qs[kk])).is_zero() {
                twos.push(BTerm::Two(j, kk, sj, sk));
            }
            k += 1;
        }
        i += 1;
    }
    ones.extend(twos);
    ones
}

/// `proposeCert`: weights from the grid, then `ldl`, checked by `cert_ok`.
pub fn propose_cert(qs: &[Quadric], f: &SForm, r2: &Q) -> Option<BCert> {
    let ts = body_terms(qs, f);
    let was = [
        qr(1, 1),
        qr(5, 4),
        qr(3, 2),
        qr(2, 1),
        qr(3, 1),
        qr(4, 1),
        qr(8, 1),
    ];
    let wbs = [
        qr(1, 1),
        qr(3, 2),
        qr(2, 1),
        qr(4, 1),
        qr(8, 1),
        qr(16, 1),
        qr(32, 1),
    ];
    for wa in &was {
        for wb in &wbs {
            let terms: Vec<(Q, BTerm)> = ts
                .iter()
                .map(|t| {
                    let w = match t {
                        BTerm::One(..) => wa.clone(),
                        BTerm::Two(..) => wb.clone(),
                    };
                    (w, t.clone())
                })
                .collect();
            let mut qts: Vec<Quadric> = Vec::new();
            let mut ok = true;
            for t in &terms {
                match t.1.quad(qs) {
                    Some(q) => qts.push(qscale(&t.0, &q)),
                    None => ok = false,
                }
            }
            if !ok {
                continue;
            }
            let n = qsub(&qsum_l(&qts), &sphere_r(r2));
            if let Some(sos) = ldl(&qmat(&n)) {
                let c = BCert { terms, sos };
                if cert_ok(qs, f, r2, &c) {
                    return Some(c);
                }
            }
        }
    }
    None
}

/// `boundForms`: each body conjoined with "inside `B`" (carrier `nc`).
pub fn bound_forms(nc: usize, forms: &[SForm]) -> Vec<SForm> {
    forms
        .iter()
        .map(|f| SForm::And(Box::new(f.clone()), Box::new(SForm::Lit(nc, Sgn::Neg))))
        .collect()
}

/// **`qledgerB`**: the bounded curved ledger.
pub fn qledger_b(qs: &[Quadric], forms: &[SForm], r2: &Q) -> Option<RefLedger> {
    let mut certs: Vec<BCert> = Vec::new();
    for f in forms {
        certs.push(propose_cert(qs, f, r2)?);
    }
    if !forms.iter().zip(certs.iter()).all(|(f, c)| cert_ok(qs, f, r2, c)) {
        return None;
    }
    let mut q2 = qs.to_vec();
    q2.push(sphere_r(r2));
    qledger(&q2, &bound_forms(qs.len(), forms))
}

/// `nzQ`.
pub fn nz_q(q: &Quadric) -> bool {
    *q != Quadric::default()
}

/// `belowB`.
pub fn below_b(n: usize, f: &SForm) -> bool {
    f.below(n)
}

/// **`certified`**.
pub fn certified(qs: &[Quadric], forms: &[SForm], r2: &Q) -> bool {
    let mut q2 = qs.to_vec();
    q2.push(sphere_r(r2));
    qledger_b(qs, forms, r2).is_some() && q2.iter().all(nz_q) && forms.iter().all(|f| below_b(qs.len(), f))
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

    fn plane_q(a: i64, b: i64, c: i64, d: (i64, i64)) -> Quadric {
        Quadric {
            lx: qi(a),
            ly: qi(b),
            lz: qi(c),
            c: q(d.0, d.1),
            ..Quadric::default()
        }
    }
    fn unit_box() -> (Vec<Quadric>, SForm) {
        let qs = vec![
            plane_q(1, 0, 0, (0, 1)),
            plane_q(1, 0, 0, (-1, 1)),
            plane_q(0, 1, 0, (0, 1)),
            plane_q(0, 1, 0, (-1, 1)),
            plane_q(0, 0, 1, (0, 1)),
            plane_q(0, 0, 1, (-1, 1)),
        ];
        (qs, box_f())
    }
    fn lit(j: usize, s: Sgn) -> SForm {
        SForm::Lit(j, s)
    }
    fn and(a: SForm, b: SForm) -> SForm {
        SForm::And(Box::new(a), Box::new(b))
    }
    fn box_f() -> SForm {
        and(
            lit(0, Sgn::Pos),
            and(
                lit(1, Sgn::Neg),
                and(
                    lit(2, Sgn::Pos),
                    and(lit(3, Sgn::Neg), and(lit(4, Sgn::Pos), lit(5, Sgn::Neg))),
                ),
            ),
        )
    }
    fn counts(l: &RefLedger) -> (usize, usize, usize) {
        (l.facets().len(), l.edges.len(), l.verts.len())
    }

    /// QLedger.lean's #guards: the ball cut by y = 0 and z = 0; the unit box.
    #[test]
    fn qledger_guards() {
        let plane_y0 = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        // cut by y = 0 and z = 0 (not by its own seam): the covector (0, 0, 0) is two points,
        // (±1, 0, 0), so the vertex certificate (U) refuses, as it must
        assert!(qledger(&[sphere1(), plane_y0, plane_z()], &[SForm::Lit(0, Sgn::Neg)]).is_none());
        let (b, f) = unit_box();
        assert_eq!(qledger(&b, &[f]).map(|l| counts(&l)), Some((6, 12, 8)));
    }

    /// QSing.lean's #guards.
    #[test]
    fn qsing_guards() {
        let cone = quad([1, 1, -1, 0, 0, 0, 0, 0, 0, 0]);
        let py = quad([0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
        assert!(sing_all(&[cone.clone(), py]));
        assert!(!sing_all(&[cone]));
        let cyl = Quadric {
            xx: qi(1),
            yy: qi(1),
            c: q(-1, 4),
            ..Quadric::default()
        };
        assert!(sing_all(&[
            sphere1(),
            cyl,
            quad([0, 0, 0, 0, 0, 0, 1, 2, 3, 4]),
            quad([1, 1, 1, 0, 0, 0, 0, 0, 0, -9])
        ]));
    }

    /// QBound.lean's #guards.
    #[test]
    fn qbound_guards() {
        let (b, f) = unit_box();
        assert!(propose_cert(&b, &f, &qi(4)).is_some());
        assert!(propose_cert(&[sphere1()], &lit(0, Sgn::Neg), &qi(2)).is_some());
        assert!(propose_cert(&b, &f, &qi(1)).is_none());
        let cyl = Quadric {
            xx: qi(1),
            yy: qi(1),
            c: q(-1, 4),
            ..Quadric::default()
        };
        let cap = [cyl, plane_q(0, 0, 1, (1, 1)), plane_q(0, 0, 1, (-1, 1))];
        let capf = and(lit(0, Sgn::Neg), and(lit(1, Sgn::Pos), lit(2, Sgn::Neg)));
        assert!(propose_cert(&cap, &capf, &qi(4)).is_some());
        assert_eq!(
            qledger_b(&b, &[box_f()], &qi(4)).map(|l| counts(&l)),
            Some((6, 12, 8))
        );
    }

    /// QGuards.lean: the five representative inputs, certified at R² = 16, with their counts.
    /// Minutes in release (the cone alone ~40 s), as the Lean's `curved-guards` executable is:
    /// `cargo test --release -p forge-bool --features curved_reference qguards -- --ignored`
    /// (`QGUARD=cone` and so on runs one).
    #[test]
    #[ignore = "minutes; run in release with --ignored"]
    fn qguards() {
        let box3 = vec![
            plane_q(1, 0, 0, (1, 1)),
            plane_q(1, 0, 0, (-1, 1)),
            plane_q(0, 1, 0, (1, 1)),
            plane_q(0, 1, 0, (-1, 1)),
            plane_q(0, 0, 1, (1, 1)),
            plane_q(0, 0, 1, (-1, 1)),
        ];
        let cyl = Quadric {
            xx: qi(1),
            yy: qi(1),
            c: q(-1, 4),
            ..Quadric::default()
        };
        let seams = vec![plane_q(0, 1, 0, (0, 1)), plane_q(-1, 0, 0, (0, 1))];
        let cyl_cap = and(lit(6, Sgn::Neg), and(lit(4, Sgn::Pos), lit(5, Sgn::Neg)));
        let cat = |a: &[Quadric], b: &[Quadric], c: &[Quadric]| -> Vec<Quadric> {
            let mut v = a.to_vec();
            v.extend_from_slice(b);
            v.extend_from_slice(c);
            v
        };
        let box_minus_cyl = (
            cat(&box3, std::slice::from_ref(&cyl), &seams),
            vec![box_f(), cyl_cap.clone()],
        );
        let plate = vec![
            plane_q(1, 0, 0, (1, 1)),
            plane_q(1, 0, 0, (-1, 1)),
            plane_q(0, 1, 0, (1, 1)),
            plane_q(0, 1, 0, (-1, 1)),
            plane_q(0, 0, 1, (0, 1)),
            plane_q(0, 0, 1, (-1, 4)),
        ];
        let plate_hole = (
            cat(&plate, std::slice::from_ref(&cyl), &seams),
            vec![box_f(), cyl_cap],
        );
        let sphere_box = (
            cat(
                &box3,
                &[Quadric {
                    xx: qi(1),
                    yy: qi(1),
                    zz: qi(1),
                    c: q(-9, 4),
                    ..Quadric::default()
                }],
                &[
                    plane_q(1, 0, 0, (0, 1)),
                    plane_q(0, 1, 0, (0, 1)),
                    plane_q(0, 0, 1, (0, 1)),
                ],
            ),
            vec![box_f(), lit(6, Sgn::Neg)],
        );
        let capped = and(lit(0, Sgn::Neg), and(lit(3, Sgn::Pos), lit(4, Sgn::Neg)));
        let cone = (
            cat(
                &[quad([1, 1, -1, 0, 0, 0, 0, 0, 0, 0])],
                &seams,
                &[plane_q(0, 0, 1, (0, 1)), plane_q(0, 0, 1, (-1, 1))],
            ),
            vec![capped.clone()],
        );
        let seamed_cyl = (
            cat(
                &[cyl],
                &seams,
                &[plane_q(0, 0, 1, (1, 1)), plane_q(0, 0, 1, (-1, 1))],
            ),
            vec![capped],
        );
        let r2 = qi(16);
        let want = [
            ("box - cylinder", &box_minus_cyl, (28, 52, 26)),
            ("plate with a hole", &plate_hole, (28, 52, 26)),
            ("sphere ∩ box", &sphere_box, (48, 86, 36)),
            ("cone", &cone, (8, 12, 6)),
            ("seamed cylinder", &seamed_cyl, (12, 20, 10)),
        ];
        let only = std::env::var("QGUARD").ok();
        for (name, (qs, fs), c) in want {
            if only.as_deref().is_some_and(|o| !name.starts_with(o)) {
                continue;
            }
            assert!(certified(qs, fs, &r2), "{name}: certified");
            let l = qledger_b(qs, fs, &r2).unwrap();
            assert_eq!(counts(&l), c, "{name}");
        }
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
