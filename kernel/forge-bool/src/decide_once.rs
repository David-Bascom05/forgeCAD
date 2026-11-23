//! The decide-once flat-face boolean's numeric bridge (research/kernel-audit/decide-once-spec.md
//! §2): it fills the [`Ledger`] the combinatorial core (`forge-bool-core`) reads, each fact decided
//! once, exactly, in the representative surface, and builds the fresh [`Body`] the core's answer
//! describes. Feature `decide_once`; flat faces only (every face a plane, every edge a line).
//!
//! The steps are the exact reference's (verify/forge-exact/ForgeExact/Ref, Snap.lean), so a
//! decision here is the one the reference makes:
//! 1. **Carriers** (`Snap.tolEq`): two faces' planes are one surface iff their grown boxes meet
//!    and each plane is within LINEAR of the other over both faces' extent; the transitive closure
//!    by union–find; the representative is the least (body, face); past the 2·LINEAR spread cap the
//!    input is ambiguous and the old path runs.
//! 2. **Vertices recomputed exactly** (`Snap.vertexAt`) from the snapped planes round them.
//! 3. **Classes**: the distinct snapped planes. A cell is its sign against each, its covector.
//! 4. **Pieces**: on each class plane, a box section split by every class plane crossing it; each
//!    vertex's covector is decided once (exact dyadic signs, `forge_math::exact`), a facet's and an
//!    edge cell's are read off their vertices'; a facet is a piece of a face of that plane whose
//!    polygon holds it (the reference's ray parity, exactly).
//! 5. **Witnesses**: one ray per flood class (`forge_bool_core::witnesses`), from a point just off
//!    a facet, against the body's snapped faces (`Ref.Face.pierced`).
//! 6. **The core**, then the body it names, written fresh: the representative's surfaces, vertices
//!    rounded once from their exact points, edges as straight runs, loops walked round each face.

use crate::boolean::{BoolError, Op};
use forge_bool_core::{self as core, BodyId, CellId, FaceId as CFace, Ledger, Outcome, Refusal, Rule, Sgn};
use forge_geom::{Curve, Line, Surface};
use forge_math::exact::{Dy, DyPlane, DyV3, HPoint, meet3};
use forge_math::{Interval, Point3};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::cell::RefCell;
use std::collections::HashMap;

/// Why the bridge leaves a boolean to the old path.
#[derive(Debug, Clone, PartialEq)]
pub enum Skip {
    /// A face is not a plane or an edge is not a line.
    NotFlat,
    /// §6.1 says ambiguous: a class past the spread cap, or a vertex its snapped planes do not
    /// settle.
    Ambiguous(&'static str),
}

/// A face of an operand as the bridge reads it.
struct In {
    body: usize,
    face: FaceId,
    /// its own plane, exactly (the kernel's frame)
    plane: DyPlane,
    /// its box, from the kernel's vertices (f64, exact as dyadics)
    lo: [f64; 3],
    hi: [f64; 3],
    loops: Vec<Vec<VertexId>>,
    /// each loop's box, from the kernel's vertices
    lboxes: Vec<([f64; 3], [f64; 3])>,
}

fn lin() -> f64 {
    forge_math::tol::LINEAR
}

/// The faces of the operands, in arena order (body 0's first), or `NotFlat`.
fn inputs(bodies: &[&Body]) -> Result<Vec<In>, Skip> {
    let mut out = Vec::new();
    for (k, b) in bodies.iter().enumerate() {
        for e in b.edge_ids() {
            if !matches!(b.edge(e).curve, Curve::Line(_)) {
                return Err(Skip::NotFlat);
            }
        }
        for f in b.face_ids() {
            let face = b.face(f);
            let Surface::Plane(pl) = &face.surface else {
                return Err(Skip::NotFlat);
            };
            let o = pl.frame.origin;
            let n = pl.frame.z().get();
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            let mut loops = Vec::new();
            let mut lboxes = Vec::new();
            for &lp in face.loops.iter() {
                let mut vs = Vec::new();
                let (mut llo, mut lhi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                for c in b.lp(lp).coedges() {
                    let v = b.coedge_start(c);
                    let p = b.vertex(v).point;
                    for (a, x) in [p.x, p.y, p.z].into_iter().enumerate() {
                        lo[a] = lo[a].min(x);
                        hi[a] = hi[a].max(x);
                        llo[a] = llo[a].min(x);
                        lhi[a] = lhi[a].max(x);
                    }
                    vs.push(v);
                }
                loops.push(vs);
                lboxes.push((llo, lhi));
            }
            if loops.is_empty() {
                continue;
            }
            out.push(In {
                body: k,
                face: f,
                plane: DyPlane::from_frame([o.x, o.y, o.z], [n.x, n.y, n.z]),
                lo,
                hi,
                loops,
                lboxes,
            });
        }
    }
    Ok(out)
}

/// The corners of the box `[lo, hi]`, exactly.
fn corners(lo: &[Dy; 3], hi: &[Dy; 3]) -> Vec<DyV3> {
    let mut out = Vec::new();
    for x in [&lo[0], &hi[0]] {
        for y in [&lo[1], &hi[1]] {
            for z in [&lo[2], &hi[2]] {
                out.push(DyV3::new(x.clone(), y.clone(), z.clone()));
            }
        }
    }
    out
}

/// `Snap.near S T R d`: `T` within `d` of `S` at the projection onto `S` of every corner of `R`,
/// exactly: `(T(c)|nS|² − S(c)(nT·nS))² ≤ d² |nT|² |nS|⁴`.
fn near(s: &DyPlane, t: &DyPlane, cs: &[DyV3], d: &Dy) -> bool {
    let ns2 = s.n.dot(&s.n);
    let nt2 = t.n.dot(&t.n);
    let nts = t.n.dot(&s.n);
    let rhs = d.mul(d).mul(&nt2).mul(&ns2).mul(&ns2);
    cs.iter().all(|c| {
        let e = t.eval(c).mul(&ns2).sub(&s.eval(c).mul(&nts));
        e.mul(&e).cmp_dy(&rhs) != std::cmp::Ordering::Greater
    })
}

/// The grown boxes' overlap is not empty (`region` nonempty), and the extent's corners, exactly.
fn region_extent(a: &In, b: &In, l: f64) -> Option<Vec<DyV3>> {
    let g = Dy::from_f64(l);
    let mut lo = [Dy::zero(), Dy::zero(), Dy::zero()];
    let mut hi = [Dy::zero(), Dy::zero(), Dy::zero()];
    for k in 0..3 {
        let (alo, ahi) = (Dy::from_f64(a.lo[k]).sub(&g), Dy::from_f64(a.hi[k]).add(&g));
        let (blo, bhi) = (Dy::from_f64(b.lo[k]).sub(&g), Dy::from_f64(b.hi[k]).add(&g));
        // region: max lo ≤ min hi
        let rlo = if alo.cmp_dy(&blo).is_ge() { &alo } else { &blo };
        let rhi = if ahi.cmp_dy(&bhi).is_le() { &ahi } else { &bhi };
        if rlo.cmp_dy(rhi).is_gt() {
            return None;
        }
        lo[k] = if alo.cmp_dy(&blo).is_le() { alo } else { blo };
        hi[k] = if ahi.cmp_dy(&bhi).is_ge() { ahi } else { bhi };
    }
    Some(corners(&lo, &hi))
}

/// A plain union–find, root the least index.
struct Uf(Vec<usize>);

impl Uf {
    fn new(n: usize) -> Uf {
        Uf((0..n).collect())
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a < b {
            self.0[b] = a;
        } else if b < a {
            self.0[a] = b;
        }
    }
}

/// f64 screen: some corner of the extent has the planes plainly over `k`·LINEAR apart, or their
/// normals plainly not parallel over it, so they are not tolerance-equal (exact test skipped).
fn plainly_apart(a: &In, b: &In, k: f64) -> bool {
    let l = lin();
    let f = |p: &DyPlane| {
        let n = [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()];
        (n, p.c.to_f64())
    };
    let (na, ca) = f(&a.plane);
    let (nb, cb) = f(&b.plane);
    let la = (na[0] * na[0] + na[1] * na[1] + na[2] * na[2]).sqrt();
    let lb = (nb[0] * nb[0] + nb[1] * nb[1] + nb[2] * nb[2]).sqrt();
    for x in [a.lo[0].min(b.lo[0]) - l, a.hi[0].max(b.hi[0]) + l] {
        for y in [a.lo[1].min(b.lo[1]) - l, a.hi[1].max(b.hi[1]) + l] {
            for z in [a.lo[2].min(b.lo[2]) - l, a.hi[2].max(b.hi[2]) + l] {
                let da = (na[0] * x + na[1] * y + na[2] * z - ca) / la;
                let db = (nb[0] * x + nb[1] * y + nb[2] * z - cb) / lb;
                // the corner's distances to the two planes, either sense
                let apart = (da - db).abs().min((da + db).abs());
                if apart > 2.0 * k * l + 1e-9 * (1.0 + x.abs() + y.abs() + z.abs()) {
                    return true;
                }
            }
        }
    }
    false
}

/// §6.1 step 1: per face, its snapped plane (the representative's, in its own sense) and its
/// representative's index; or ambiguous past the spread cap.
fn snap(fs: &[In]) -> Result<(Vec<DyPlane>, Vec<usize>), Skip> {
    let l = lin();
    let ld = Dy::from_f64(l);
    let cap = ld.scale2(1); // K = 2
    let n = fs.len();
    // candidate pairs: grown boxes overlapping in x (sweep), then the exact test
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| fs[i].lo[0].total_cmp(&fs[j].lo[0]).then(i.cmp(&j)));
    let mut pairs = Vec::new();
    for (oi, &i) in order.iter().enumerate() {
        for &j in &order[oi + 1..] {
            if fs[j].lo[0] - l > fs[i].hi[0] + l {
                break;
            }
            let (a, b) = (i.min(j), i.max(j));
            if plainly_apart(&fs[a], &fs[b], 1.0) {
                continue;
            }
            if let Some(cs) = region_extent(&fs[a], &fs[b], l) {
                pairs.push((a, b, cs));
            }
        }
    }
    let mut uf = Uf::new(n);
    let mut eq = Vec::new();
    for (a, b, cs) in &pairs {
        if near(&fs[*a].plane, &fs[*b].plane, cs, &ld) && near(&fs[*b].plane, &fs[*a].plane, cs, &ld) {
            uf.union(*a, *b);
            eq.push((*a, *b));
        }
    }
    // the spread cap: every pair of a class whose region is not empty within 2·LINEAR
    let mut members: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = uf.find(i);
        members.entry(r).or_default().push(i);
    }
    for ms in members.values() {
        if ms.len() < 3 {
            continue; // a pair is within LINEAR by its own test
        }
        for (x, &a) in ms.iter().enumerate() {
            for &b in &ms[x + 1..] {
                if let Some(cs) = region_extent(&fs[a], &fs[b], l)
                    && !(near(&fs[a].plane, &fs[b].plane, &cs, &cap)
                        && near(&fs[b].plane, &fs[a].plane, &cs, &cap))
                {
                    return Err(Skip::Ambiguous(
                        "a class of tolerance-equal surfaces spreads past the cap",
                    ));
                }
            }
        }
    }
    // the representative: the least (body, face) name, which is the least index (arena order)
    let mut planes = Vec::with_capacity(n);
    let mut reps = Vec::with_capacity(n);
    for (i, f) in fs.iter().enumerate() {
        let r = uf.find(i);
        let rp = &fs[r].plane;
        planes.push(if f.plane.n.dot(&rp.n).signum() < 0 {
            rp.flipped()
        } else {
            rp.clone()
        });
        reps.push(r);
    }
    Ok((planes, reps))
}

/// §6.1 step 2 (`Snap.vertexAt`): every vertex of every body from the snapped planes of the faces
/// round it, the first independent three, which every one of them must hold.
fn vertices(bodies: &[&Body], fs: &[In], planes: &[DyPlane], reg: &Region) -> Result<Vec<Pts>, Skip> {
    let need = &reg.near;
    let g = 4.0 * lin();
    let in_region = |p: Point3| match reg.bx {
        None => true,
        Some((lo, hi)) => {
            let q = [p.x, p.y, p.z];
            (0..3).all(|k| lo[k] - g <= q[k] && q[k] <= hi[k] + g)
        }
    };
    let mut round: Vec<HashMap<VertexId, Vec<usize>>> = vec![HashMap::new(); bodies.len()];
    for (i, f) in fs.iter().enumerate() {
        for lp in &f.loops {
            for &v in lp {
                let r = round[f.body].entry(v).or_default();
                if !r.contains(&i) {
                    r.push(i);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (k, rk) in round.into_iter().enumerate() {
        let pts = Pts {
            exact: RefCell::new(HashMap::new()),
            f: RefCell::new(HashMap::new()),
            round: rk,
        };
        // the near faces' vertices in the region now, each checked (§6.1's ambiguity is decided
        // there); the others when a parity test or a ray first needs them
        let mut keys: Vec<VertexId> = pts
            .round
            .iter()
            .filter(|(v, fsr)| fsr.iter().any(|&i| need[i]) && in_region(bodies[k].vertex(**v).point))
            .map(|(&v, _)| v)
            .collect();
        keys.sort();
        for v in keys {
            let ps: Vec<&DyPlane> = pts.round[&v].iter().map(|&i| &planes[i]).collect();
            let p = vertex_at(&ps).ok_or(Skip::Ambiguous(
                "a vertex is not one point of the planes round it",
            ))?;
            pts.f.borrow_mut().insert(v, p.to_f64());
            pts.exact.borrow_mut().insert(v, p);
        }
        out.push(pts);
    }
    Ok(out)
}

/// `Snap.vertexAt`: the first independent three of the planes meet in a point that every one of
/// them holds.
fn vertex_at(ps: &[&DyPlane]) -> Option<HPoint> {
    let mut p = None;
    'find: for a in ps {
        for b in ps {
            for c in ps {
                if let Some(q) = meet3(a, b, c) {
                    p = Some(q);
                    break 'find;
                }
            }
        }
    }
    let p = p?;
    ps.iter().all(|q| q.side(&p) == 0).then_some(p)
}

/// An exact rational `n / d`.
#[derive(Clone)]
struct Q {
    n: Dy,
    d: Dy,
}

impl Q {
    fn of(h: &HPoint, axis: usize) -> Q {
        Q {
            n: h.p.get(axis).clone(),
            d: h.w.clone(),
        }
    }
    fn sub(&self, o: &Q) -> Q {
        Q {
            n: self.n.mul(&o.d).sub(&o.n.mul(&self.d)),
            d: self.d.mul(&o.d),
        }
    }
    fn mul(&self, o: &Q) -> Q {
        Q {
            n: self.n.mul(&o.n),
            d: self.d.mul(&o.d),
        }
    }
    fn sgn(&self) -> i32 {
        self.n.signum() * self.d.signum()
    }
}

/// `Ref.crossesYZ` in axes `(s, t)`: does the ray from `q` toward +s (its origin perturbed to
/// `q + (ε, ε²)`) cross segment `ab`? Exact.
fn crosses(q: &HPoint, a: &HPoint, b: &HPoint, s: usize, t: usize) -> bool {
    let (qs, qt) = (Q::of(q, s), Q::of(q, t));
    let (a_s, at) = (Q::of(a, s), Q::of(a, t));
    let (bs, bt) = (Q::of(b, s), Q::of(b, t));
    let aa = at.sub(&qt).sgn() > 0;
    let ab = bt.sub(&qt).sgn() > 0;
    if aa == ab {
        return false;
    }
    let dz = bt.sub(&at);
    let d0 = bs.sub(&a_s).mul(&qt.sub(&at)).sub(&qs.sub(&a_s).mul(&dz));
    let sg = if d0.sgn() != 0 { d0.sgn() } else { -dz.sgn() };
    sg * dz.sgn() > 0
}

/// The class planes, the classes of each face, and per class its representative input face.
struct Classes {
    planes: Vec<DyPlane>,
    /// per input face, its class (NONE for a far face)
    of_face: Vec<u32>,
    rep: Vec<usize>,
    /// the region's walls, as classes carrying no face (none when the region is everything), and
    /// the side of each that is inside the region
    walls: Vec<u32>,
    wall_in: Vec<Sgn>,
}

const NONE: u32 = u32::MAX;

/// The distinct snapped planes of the near faces, in arena order of their first face.
fn classes(snapped: &[DyPlane], reps: &[usize], near: &[bool]) -> Classes {
    let mut planes: Vec<DyPlane> = Vec::new();
    let mut rep = Vec::new();
    let mut of_face = Vec::with_capacity(snapped.len());
    // by an f64 key first, so the exact comparison runs only against likely equals: one point set
    // gives keys at most one step apart in each component, so its neighbours are looked at too
    let mut by_key: HashMap<(i64, i64, i64, i64), Vec<u32>> = HashMap::new();
    for (i, p) in snapped.iter().enumerate() {
        if !near[i] {
            of_face.push(NONE);
            continue;
        }
        let n = [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        // canonical sense: first nonzero component positive
        let sg = if n[0] != 0.0 {
            n[0].signum()
        } else if n[1] != 0.0 {
            n[1].signum()
        } else {
            n[2].signum()
        };
        let q = |x: f64| (x * sg / len * 1e6).round() as i64;
        let key = (
            q(n[0]),
            q(n[1]),
            q(n[2]),
            (p.c.to_f64() * sg / len * 1e3).round() as i64,
        );
        let mut found = None;
        'look: for d0 in -1..=1 {
            for d1 in -1..=1 {
                for d2 in -1..=1 {
                    for d3 in -1..=1 {
                        let k = (key.0 + d0, key.1 + d1, key.2 + d2, key.3 + d3);
                        if let Some(cs) = by_key.get(&k) {
                            for &c in cs {
                                if planes[c as usize].same_set(p) {
                                    found = Some(c);
                                    break 'look;
                                }
                            }
                        }
                    }
                }
            }
        }
        let c = match found {
            Some(c) => c,
            None => {
                planes.push(p.clone());
                rep.push(reps[i]);
                by_key.entry(key).or_default().push(planes.len() as u32 - 1);
                planes.len() as u32 - 1
            }
        };
        of_face.push(c);
    }
    Classes {
        planes,
        of_face,
        rep,
        walls: Vec::new(),
        wall_in: Vec::new(),
    }
}

/// The region of §3a: the tools' box grown by `g`, and which faces are near; or everything.
struct Region {
    /// lo, hi of the closed box; None for everything
    bx: Option<([f64; 3], [f64; 3])>,
    near: Vec<bool>,
    /// outside the region the result is body 0 (else empty)
    keeps_target: bool,
}

/// The rule at a membership vector.
fn rule_at(rule: &Rule, m: &[bool]) -> bool {
    let ins = m.iter().filter(|&&b| b).count();
    match rule {
        Rule::Union => ins > 0,
        Rule::Inter => !m.is_empty() && ins == m.len(),
        Rule::Diff => !m.is_empty() && m[0] && ins == 1,
        Rule::Table(t) => {
            let row: usize = m
                .iter()
                .enumerate()
                .filter(|(_, b)| **b)
                .map(|(i, _)| 1usize << i)
                .sum();
            row < t.len() && t[row]
        }
    }
}

/// §3a: the region round the tools, decided once from the inputs.
fn region(bodies: &[&Body], fs: &[In], snapped: &[DyPlane], rule: &Rule) -> Region {
    let all = Region {
        bx: None,
        near: vec![true; fs.len()],
        keeps_target: true,
    };
    let nb = bodies.len();
    if nb < 2 || rule_at(rule, &vec![false; nb]) {
        return all;
    }
    let mut e0 = vec![false; nb];
    e0[0] = true;
    let keeps_target = rule_at(rule, &e0);
    let (mut blo, mut bhi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for f in fs.iter().filter(|f| f.body > 0) {
        for k in 0..3 {
            blo[k] = blo[k].min(f.lo[k]);
            bhi[k] = bhi[k].max(f.hi[k]);
        }
    }
    if !blo[0].is_finite() {
        return all;
    }
    let l = lin();
    let mut g = 1.0f64;
    while g / 2.0 >= 8.0 * l {
        g /= 2.0;
    }
    while g < 8.0 * l {
        g *= 2.0;
    }
    let wall = |axis: usize, x: f64| {
        let mut n = [0.0; 3];
        n[axis] = 1.0;
        let mut o = [0.0; 3];
        o[axis] = x;
        DyPlane::from_frame(o, n)
    };
    for _ in 0..40 {
        let lo = [blo[0] - g, blo[1] - g, blo[2] - g];
        let hi = [bhi[0] + g, bhi[1] + g, bhi[2] + g];
        // no wall a carrier, and no vertex near a wall's plane within the box's span
        let mut clash = false;
        'check: for (i, f) in fs.iter().enumerate() {
            let p = &snapped[i];
            let nf = [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()];
            for a in 0..3 {
                if nf[(a + 1) % 3] == 0.0 && nf[(a + 2) % 3] == 0.0 {
                    for x in [lo[a], hi[a]] {
                        if wall(a, x).same_set(p) {
                            clash = true;
                            break 'check;
                        }
                    }
                }
            }
            for lp in &f.loops {
                for &v in lp {
                    let q = bodies[f.body].vertex(v).point;
                    let q = [q.x, q.y, q.z];
                    let within = |a: usize| q[a] >= lo[a] - 4.0 * l && q[a] <= hi[a] + 4.0 * l;
                    for a in 0..3 {
                        if within((a + 1) % 3)
                            && within((a + 2) % 3)
                            && [lo[a], hi[a]].iter().any(|&x| (q[a] - x).abs() <= 4.0 * l)
                        {
                            clash = true;
                            break 'check;
                        }
                    }
                }
            }
        }
        if clash {
            g *= 2.0;
            continue;
        }
        let gr = 4.0 * l;
        let near: Vec<bool> = fs
            .iter()
            .map(|f| f.body > 0 || (0..3).all(|k| f.lo[k] - gr <= hi[k] && f.hi[k] + gr >= lo[k]))
            .collect();
        // a far face the snap moves is not unchanged: the region is everything
        if fs
            .iter()
            .enumerate()
            .any(|(i, f)| !near[i] && !snapped[i].same_set(&f.plane))
        {
            return all;
        }
        return Region {
            bx: Some((lo, hi)),
            near,
            keeps_target,
        };
    }
    all
}

/// The six walls of the box as classes: lo walls (inside on their positive side), hi walls (on
/// their negative side).
fn add_walls(cls: &mut Classes, lo: [f64; 3], hi: [f64; 3]) {
    for a in 0..3 {
        for (x, inside) in [(lo[a], Sgn::Pos), (hi[a], Sgn::Neg)] {
            let mut n = [0.0; 3];
            n[a] = 1.0;
            let mut o = [0.0; 3];
            o[a] = x;
            cls.planes.push(DyPlane::from_frame(o, n));
            cls.rep.push(usize::MAX);
            cls.walls.push(cls.planes.len() as u32 - 1);
            cls.wall_in.push(inside);
        }
    }
}

/// A vertex of the arrangement on one class plane: its exact point and, for a meet of three
/// classes, its covector.
struct Vx {
    p: HPoint,
    /// its f64 shadow, for screening
    f: [f64; 3],
    cov: Option<Vec<Sgn>>,
}

/// A convex polygon: (vertex, plane of the edge to the next vertex).
type Poly = Vec<(u32, Pl)>;

/// A plane a polygon's edge lies on: a class, or a wall of the working box.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Pl {
    Class(u32),
    Wall(u8),
}

/// The arrangement on one class plane: vertices, and convex cells as (vertex, plane of the edge
/// to the next vertex) lists.
struct Plane2 {
    vx: Vec<Vx>,
    cells: Vec<Vec<(u32, Pl)>>,
}

/// The sign of a class plane at an exact point, f64-screened.
fn side_at(plane: &DyPlane, pf: &[f64; 3], pff: &PlaneF, h: &HPoint) -> Sgn {
    let e = pff.n[0] * pf[0] + pff.n[1] * pf[1] + pff.n[2] * pf[2] - pff.c;
    let mag = pff.n[0].abs() * pf[0].abs()
        + pff.n[1].abs() * pf[1].abs()
        + pff.n[2].abs() * pf[2].abs()
        + pff.c.abs();
    let s = if e.abs() > 1e-11 * mag {
        if e > 0.0 { 1 } else { -1 }
    } else {
        plane.side(h)
    };
    match s {
        1 => Sgn::Pos,
        -1 => Sgn::Neg,
        _ => Sgn::Zero,
    }
}

/// A plane's f64 shadow, for screening.
struct PlaneF {
    n: [f64; 3],
    c: f64,
}

fn plane_f(p: &DyPlane) -> PlaneF {
    PlaneF {
        n: [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()],
        c: p.c.to_f64(),
    }
}

fn covector(cls: &Classes, pf: &[PlaneF], h: &HPoint, zero: &[u32]) -> Vec<Sgn> {
    let p = h.to_f64();
    (0..cls.planes.len())
        .map(|k| {
            if zero.contains(&(k as u32)) {
                Sgn::Zero
            } else {
                side_at(&cls.planes[k], &p, &pf[k], h)
            }
        })
        .collect()
}

/// The cells of class plane `c` within the box `[lo, hi]`: its section split by every class
/// plane crossing it.
fn arrange(cls: &Classes, pf: &[PlaneF], c: u32, lo: [f64; 3], hi: [f64; 3]) -> Plane2 {
    let pc = &cls.planes[c as usize];
    let nf = &pf[c as usize].n;
    let a = (0..3)
        .max_by(|&i, &j| nf[i].abs().total_cmp(&nf[j].abs()).then(j.cmp(&i)))
        .unwrap();
    let (u, v) = ((a + 1) % 3, (a + 2) % 3);
    let wall = |axis: usize, x: f64| {
        let mut n = [0.0; 3];
        n[axis] = 1.0;
        let mut o = [0.0; 3];
        o[axis] = x;
        DyPlane::from_frame(o, n)
    };
    // walls: 0 u lo, 1 u hi, 2 v lo, 3 v hi, 4 a lo, 5 a hi
    let walls = [
        wall(u, lo[u]),
        wall(u, hi[u]),
        wall(v, lo[v]),
        wall(v, hi[v]),
        wall(a, lo[a]),
        wall(a, hi[a]),
    ];
    let wf: Vec<PlaneF> = walls.iter().map(plane_f).collect();
    let plane_of = |p: Pl| -> &DyPlane {
        match p {
            Pl::Class(k) => &cls.planes[k as usize],
            Pl::Wall(w) => &walls[w as usize],
        }
    };
    let mut vx: Vec<Vx> = Vec::new();
    // a vertex by the two planes (besides c) it is the meet of, and class vertices by covector
    let mut by_pair: HashMap<(Pl, Pl), u32> = HashMap::new();
    let mut by_cov: HashMap<Vec<Sgn>, u32> = HashMap::new();
    let mut vertex = |x: Pl, y: Pl, vx: &mut Vec<Vx>| -> u32 {
        let key = if x <= y { (x, y) } else { (y, x) };
        if let Some(&i) = by_pair.get(&key) {
            return i;
        }
        let h = meet3(pc, plane_of(x), plane_of(y)).expect("a polygon's edge crosses the plane splitting it");
        let id = match (x, y) {
            (Pl::Class(i), Pl::Class(j)) => {
                let cov = covector(cls, pf, &h, &[c, i, j]);
                if let Some(&k) = by_cov.get(&cov) {
                    k
                } else {
                    let f = h.to_f64();
                    vx.push(Vx {
                        p: h,
                        f,
                        cov: Some(cov.clone()),
                    });
                    by_cov.insert(cov, vx.len() as u32 - 1);
                    vx.len() as u32 - 1
                }
            }
            _ => {
                let f = h.to_f64();
                vx.push(Vx { p: h, f, cov: None });
                vx.len() as u32 - 1
            }
        };
        by_pair.insert(key, id);
        id
    };
    let side = |vx: &Vec<Vx>, i: u32, p: Pl| -> i32 {
        let x = &vx[i as usize];
        match (p, &x.cov) {
            (Pl::Class(k), Some(cv)) => match cv[k as usize] {
                Sgn::Pos => 1,
                Sgn::Neg => -1,
                Sgn::Zero => 0,
            },
            _ => {
                let pl = plane_of(p);
                let pff = match p {
                    Pl::Class(k) => &pf[k as usize],
                    Pl::Wall(w) => &wf[w as usize],
                };
                match side_at(pl, &x.f, pff, &x.p) {
                    Sgn::Pos => 1,
                    Sgn::Neg => -1,
                    Sgn::Zero => 0,
                }
            }
        }
    };
    // the quad on the u and v walls, counter-clockwise in (u, v)
    let q = [
        vertex(Pl::Wall(0), Pl::Wall(2), &mut vx),
        vertex(Pl::Wall(1), Pl::Wall(2), &mut vx),
        vertex(Pl::Wall(1), Pl::Wall(3), &mut vx),
        vertex(Pl::Wall(0), Pl::Wall(3), &mut vx),
    ];
    let mut poly: Vec<(u32, Pl)> = vec![
        (q[0], Pl::Wall(2)),
        (q[1], Pl::Wall(1)),
        (q[2], Pl::Wall(3)),
        (q[3], Pl::Wall(0)),
    ];
    // split: (part with side ≥ 0, part with side ≤ 0)
    let mut split = |poly: &[(u32, Pl)], p: Pl, vx: &mut Vec<Vx>| -> (Poly, Poly) {
        let n = poly.len();
        let s: Vec<i32> = poly.iter().map(|&(i, _)| side(vx, i, p)).collect();
        let (mut pos, mut neg) = (Vec::new(), Vec::new());
        for k in 0..n {
            let (vi, line) = poly[k];
            let (fa, fb) = (s[k], s[(k + 1) % n]);
            // the edge from vertex k to k+1 lies on `line`; a part's edge out of a vertex on the
            // splitting plane runs along that plane when the part turns there
            if fa >= 0 {
                pos.push((vi, line));
            }
            if fa <= 0 {
                neg.push((vi, line));
            }
            if (fa > 0 && fb < 0) || (fa < 0 && fb > 0) {
                let w = vertex(line, p, vx);
                pos.push((w, line));
                neg.push((w, line));
            }
        }
        // the edge plane of each part's vertex is the plane the part runs on to its next vertex:
        // where a part's two consecutive vertices both lie on `p`, that edge is on `p`
        let fix = |part: &mut Vec<(u32, Pl)>, vx: &Vec<Vx>| {
            let m = part.len();
            for k in 0..m {
                let (i, j) = (part[k].0, part[(k + 1) % m].0);
                if side(vx, i, p) == 0 && side(vx, j, p) == 0 {
                    part[k].1 = p;
                }
            }
        };
        fix(&mut pos, vx);
        fix(&mut neg, vx);
        (pos, neg)
    };
    for w in [Pl::Wall(4), Pl::Wall(5)] {
        let (p, n) = split(&poly, w, &mut vx);
        poly = if w == Pl::Wall(4) { p } else { n };
        if poly.len() < 3 {
            return Plane2 {
                vx,
                cells: Vec::new(),
            };
        }
    }
    let mut cells = vec![poly];
    for j in 0..cls.planes.len() as u32 {
        if j == c {
            continue;
        }
        let pj = Pl::Class(j);
        let first = &cells;
        let all_pos = first
            .iter()
            .all(|cl| cl.iter().all(|&(i, _)| side(&vx, i, pj) > 0));
        let all_neg = first
            .iter()
            .all(|cl| cl.iter().all(|&(i, _)| side(&vx, i, pj) < 0));
        if all_pos || all_neg {
            continue;
        }
        let mut next = Vec::with_capacity(cells.len() + 4);
        for cl in &cells {
            let (p, n) = split(cl, pj, &mut vx);
            if p.len() >= 3 {
                next.push(p);
            }
            if n.len() >= 3 {
                next.push(n);
            }
        }
        cells = next;
    }
    Plane2 { vx, cells }
}

/// Is the exact point inside face `f`'s polygon (its loops, with exact vertices), by the parity
/// of the perturbed ray toward +s in the plane's projection `(s, t)`.
fn in_face(q: &HPoint, f: &In, pts: &Pts, planes: &[DyPlane], s: usize, t: usize) -> bool {
    let qf = q.to_f64();
    // (a loop's exact polygon is within a few LINEAR of its kernel box)
    let g = 4.0 * lin() + 1e-9 * (1.0 + qf[s].abs() + qf[t].abs());
    let mut k = 0;
    for (lp, (llo, lhi)) in f.loops.iter().zip(&f.lboxes) {
        if qf[t] < llo[t] - g || qf[t] > lhi[t] + g || qf[s] > lhi[s] + g {
            continue;
        }
        let n = lp.len();
        for i in 0..n {
            let (a, b) = (lp[i], lp[(i + 1) % n]);
            let c = match crosses_f(qf, pts.pf(a, planes), pts.pf(b, planes), s, t) {
                Some(c) => c,
                None => crosses(q, &pts.ph(a, planes), &pts.ph(b, planes), s, t),
            };
            if c {
                k += 1;
            }
        }
    }
    k % 2 == 1
}

/// The exact vertices of one body, and their f64 shadows (for screening only): the near faces'
/// worked out first, any other's when a ray first needs it.
pub(crate) struct Pts {
    exact: RefCell<HashMap<VertexId, HPoint>>,
    f: RefCell<HashMap<VertexId, [f64; 3]>>,
    /// per vertex, the faces round it
    round: HashMap<VertexId, Vec<usize>>,
}

impl Pts {
    fn ensure(&self, v: VertexId, planes: &[DyPlane]) {
        if self.exact.borrow().contains_key(&v) {
            return;
        }
        let ps: Vec<&DyPlane> = self.round[&v].iter().map(|&i| &planes[i]).collect();
        // (a far vertex its planes do not settle: the first meet, as the reference's vertexAt
        // would have refused the input; a far face is crossed only by rays)
        let p = vertex_at(&ps)
            .or_else(|| {
                let mut q = None;
                'find: for a in &ps {
                    for b in &ps {
                        for c in &ps {
                            if let Some(x) = meet3(a, b, c) {
                                q = Some(x);
                                break 'find;
                            }
                        }
                    }
                }
                q
            })
            .expect("three independent planes round a vertex");
        self.f.borrow_mut().insert(v, p.to_f64());
        self.exact.borrow_mut().insert(v, p);
    }
    fn pf(&self, v: VertexId, planes: &[DyPlane]) -> [f64; 3] {
        self.ensure(v, planes);
        self.f.borrow()[&v]
    }
    fn ph(&self, v: VertexId, planes: &[DyPlane]) -> HPoint {
        self.ensure(v, planes);
        self.exact.borrow()[&v].clone()
    }
}

/// `crosses`, screened in f64: the answer where every sign it takes is plainly away from zero, else
/// `None` (the exact test decides). The f64 points are within a few ulps of the exact ones.
fn crosses_f(q: [f64; 3], a: [f64; 3], b: [f64; 3], s: usize, t: usize) -> Option<bool> {
    const E: f64 = 1e-12;
    let scale = 1.0
        + q[s]
            .abs()
            .max(q[t].abs())
            .max(a[s].abs())
            .max(a[t].abs())
            .max(b[s].abs())
            .max(b[t].abs());
    let above = |x: f64| -> Option<bool> {
        let d = x - q[t];
        if d.abs() > E * scale { Some(d > 0.0) } else { None }
    };
    let aa = above(a[t])?;
    let ab = above(b[t])?;
    if aa == ab {
        return Some(false);
    }
    // b above and a not: dz > 0; and the other way round
    let dz_pos = ab;
    let d0 = (b[s] - a[s]) * (q[t] - a[t]) - (q[s] - a[s]) * (b[t] - a[t]);
    let mag = ((b[s] - a[s]).abs() + (q[s] - a[s]).abs() + (q[t] - a[t]).abs() + (b[t] - a[t]).abs()) * scale;
    if d0.abs() <= E * mag {
        return None;
    }
    Some((d0 > 0.0) == dz_pos)
}

/// `Ref.Face.pierced`: does the ray from `p` along +x (origin perturbed) pierce face `f`?
fn pierced(f: &In, plane: &DyPlane, pts: &Pts, planes: &[DyPlane], p: &HPoint) -> bool {
    let nx = plane.n.x.signum();
    if nx == 0 {
        return false;
    }
    // sgn (c − n·p) · sgn nx > 0: the face ahead along +x
    let side = match side_at(plane, &p.to_f64(), &plane_f(plane), p) {
        Sgn::Pos => 1,
        Sgn::Neg => -1,
        Sgn::Zero => 0,
    };
    if -side * nx <= 0 {
        return false;
    }
    in_face(p, f, pts, planes, 1, 2)
}

/// A facet's geometry: its plane's class, its polygon (vertex cells), an interior point.
struct Facet {
    class: u32,
    /// counter-clockwise about the class plane's normal
    poly: Vec<u32>,
    mid: HPoint,
}

/// Everything the bridge decided, kept to build the body.
struct Built {
    ledger: Ledger,
    /// per cell id: its facet geometry, if a facet
    facet: HashMap<u32, Facet>,
    /// per vertex cell: its exact point
    point: HashMap<u32, HPoint>,
    /// per edge cell: its two vertex cells
    ends: HashMap<u32, (u32, u32)>,
    /// per side cell: a facet it is a side of and which side
    side_of: HashMap<u32, (u32, i32)>,
    /// per side cell: every (facet, side) it is a side of
    sides: HashMap<u32, Vec<(u32, i32)>>,
    /// per facet: the input faces it is a piece of
    holders: HashMap<u32, Vec<usize>>,
    /// cell ids by covector
    by: HashMap<Vec<Sgn>, u32>,
}

/// A table of cells by covector.
#[derive(Default)]
struct CellTab {
    cells: Vec<Vec<Sgn>>,
    by: HashMap<Vec<Sgn>, u32>,
}

impl CellTab {
    fn id(&mut self, c: Vec<Sgn>) -> u32 {
        if let Some(&i) = self.by.get(&c) {
            return i;
        }
        self.cells.push(c.clone());
        self.by.insert(c, self.cells.len() as u32 - 1);
        self.cells.len() as u32 - 1
    }
}

/// The covector of the cell spanned by vertices: on each class the side of any vertex off it.
fn join(covs: &[&Vec<Sgn>]) -> Vec<Sgn> {
    let n = covs[0].len();
    (0..n)
        .map(|k| {
            covs.iter()
                .map(|c| c[k])
                .find(|&s| s != Sgn::Zero)
                .unwrap_or(Sgn::Zero)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_ledger(
    bodies: &[&Body],
    fs: &[In],
    cls: &Classes,
    pts: &[Pts],
    snapped: &[DyPlane],
    bx: Option<([f64; 3], [f64; 3])>,
) -> Built {
    // a cell of the open region: strictly inside every wall
    let open = |cv: &[Sgn]| {
        cls.walls
            .iter()
            .zip(&cls.wall_in)
            .all(|(&w, &s)| cv[w as usize] == s)
    };
    let on_wall = |cv: &[Sgn]| cls.walls.iter().any(|&w| cv[w as usize] == Sgn::Zero);
    let pf: Vec<PlaneF> = cls.planes.iter().map(plane_f).collect();
    let mut tab = CellTab::default();
    let mut facet = HashMap::new();
    let mut point = HashMap::new();
    let mut ends = HashMap::new();
    let mut side_of = HashMap::new();
    let mut sides: HashMap<u32, Vec<(u32, i32)>> = HashMap::new();
    let mut holders_of: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut pieces: Vec<(u32, u32)> = Vec::new(); // (face, cell)
    let mut edges: Vec<u32> = Vec::new();
    let mut verts: Vec<u32> = Vec::new();
    let mut is_e: HashMap<u32, ()> = HashMap::new();
    let mut is_v: HashMap<u32, ()> = HashMap::new();
    // the faces of each class, in arena order
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); cls.planes.len()];
    for (i, &c) in cls.of_face.iter().enumerate() {
        if c != NONE {
            on[c as usize].push(i);
        }
    }
    let mut per_face: Vec<Vec<u32>> = vec![Vec::new(); fs.len()];
    for (c, faces) in on.iter().enumerate() {
        if faces.is_empty() {
            continue;
        }
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &i in faces {
            for lp in &fs[i].loops {
                for v in lp {
                    let p = pts[fs[i].body].pf(*v, snapped);
                    for k in 0..3 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
            }
        }
        for k in 0..3 {
            let g = 1.0 + 1e-3 * (hi[k] - lo[k]).abs();
            lo[k] = (lo[k] - g).floor();
            hi[k] = (hi[k] + g).ceil();
            // (only the region's cells are listed: the section need reach no further)
            if let Some((rlo, rhi)) = bx {
                let m = (rhi[k] - rlo[k]) / 64.0 + 1.0 / 1024.0;
                lo[k] = lo[k].max(rlo[k] - m);
                hi[k] = hi[k].min(rhi[k] + m);
            }
        }
        if (0..3).any(|k| lo[k] >= hi[k]) {
            continue;
        }
        let pl = arrange(cls, &pf, c as u32, lo, hi);
        let nf = &pf[c].n;
        let a = (0..3)
            .max_by(|&i, &j| nf[i].abs().total_cmp(&nf[j].abs()).then(j.cmp(&i)))
            .unwrap();
        let (s, t) = ((a + 1) % 3, (a + 2) % 3);
        let flip = nf[a] < 0.0;
        for cell in &pl.cells {
            // a cell inside a face is bounded by classes only
            if cell.iter().any(|&(i, _)| pl.vx[i as usize].cov.is_none()) {
                continue;
            }
            let mid = HPoint::mean(&[
                &pl.vx[cell[0].0 as usize].p,
                &pl.vx[cell[1].0 as usize].p,
                &pl.vx[cell[2].0 as usize].p,
            ]);
            let midf = mid.to_f64();
            let holders: Vec<usize> = faces
                .iter()
                .copied()
                .filter(|&i| {
                    // (a face's snapped polygon is within a few LINEAR of its kernel box)
                    let g = 4.0 * lin() + 1e-9 * (1.0 + midf[0].abs() + midf[1].abs() + midf[2].abs());
                    let f = &fs[i];
                    (0..3).all(|k| f.lo[k] - g <= midf[k] && midf[k] <= f.hi[k] + g)
                        && in_face(&mid, f, &pts[f.body], snapped, s, t)
                })
                .collect();
            if holders.is_empty() {
                continue;
            }
            let covs: Vec<&Vec<Sgn>> = cell
                .iter()
                .map(|&(i, _)| pl.vx[i as usize].cov.as_ref().unwrap())
                .collect();
            let fc = join(&covs);
            if !open(&fc) {
                continue;
            }
            let id = tab.id(fc.clone());
            // its vertices and edges, as cells
            let mut poly = Vec::new();
            for k in 0..cell.len() {
                let (i, j) = (cell[k].0 as usize, cell[(k + 1) % cell.len()].0 as usize);
                let vi = tab.id(pl.vx[i].cov.clone().unwrap());
                point.entry(vi).or_insert_with(|| pl.vx[i].p.clone());
                let vj = tab.id(pl.vx[j].cov.clone().unwrap());
                let e = tab.id(join(&[
                    pl.vx[i].cov.as_ref().unwrap(),
                    pl.vx[j].cov.as_ref().unwrap(),
                ]));
                ends.entry(e).or_insert((vi, vj));
                poly.push(vi);
                let _ = vj;
            }
            if flip {
                poly.reverse();
            }
            for &i in &holders {
                per_face[i].push(id);
                let h = holders_of.entry(id).or_default();
                if !h.contains(&i) {
                    h.push(i);
                }
            }
            facet.entry(id).or_insert(Facet {
                class: c as u32,
                poly,
                mid,
            });
        }
    }
    // pieces in face arena order; then their edge and vertex cells in that order
    for (i, ids) in per_face.iter().enumerate() {
        for &id in ids {
            pieces.push((i as u32, id));
        }
    }
    for &(_, id) in &pieces {
        let poly = facet[&id].poly.clone();
        for k in 0..poly.len() {
            let v = poly[k];
            if !on_wall(&tab.cells[v as usize]) && is_v.insert(v, ()).is_none() {
                verts.push(v);
            }
            let w = poly[(k + 1) % poly.len()];
            let ec = join(&[&tab.cells[v as usize].clone(), &tab.cells[w as usize].clone()]);
            let wall_edge = on_wall(&ec);
            let e = tab.id(ec);
            if !wall_edge && is_e.insert(e, ()).is_none() {
                edges.push(e);
            }
        }
    }
    // the facets' sides
    let mut facets_seen: Vec<u32> = pieces.iter().map(|p| p.1).collect();
    facets_seen.dedup();
    let mut done: HashMap<u32, ()> = HashMap::new();
    for &id in &facets_seen {
        if done.insert(id, ()).is_some() {
            continue;
        }
        let c = facet[&id].class as usize;
        for (sg, s) in [(Sgn::Pos, 1), (Sgn::Neg, -1)] {
            let mut cv = tab.cells[id as usize].clone();
            cv[c] = sg;
            let t = tab.id(cv);
            side_of.entry(t).or_insert((id, s));
            sides.entry(t).or_default().push((id, s));
        }
    }
    let nb = bodies.len() as u32;
    let ledger = Ledger {
        nb,
        face_body: fs.iter().map(|f| BodyId(f.body as u32)).collect(),
        face_class: cls.of_face.clone(),
        cells: tab.cells.clone(),
        pieces: pieces.iter().map(|&(f, c)| (CellId(c), CFace(f))).collect(),
        inside: Vec::new(),
        edges: edges.into_iter().map(CellId).collect(),
        verts: verts.into_iter().map(CellId).collect(),
    };
    Built {
        ledger,
        facet,
        point,
        ends,
        side_of,
        sides,
        holders: holders_of,
        by: tab.by,
    }
}

/// A point strictly inside side cell `t`: just off its facet's interior point, toward its side.
fn witness_point(b: &Built, cls: &Classes, t: u32) -> HPoint {
    let (f, s) = b.side_of[&t];
    let fc = &b.facet[&f];
    let cov = &b.ledger.cells[f as usize];
    let c = fc.class as usize;
    let n = &cls.planes[c].n;
    let mut k = 4;
    loop {
        let d = Dy::from_i64(s as i64).scale2(-k);
        let p = fc.mid.offset(n, &d);
        let pfl = p.to_f64();
        let ok = cls.planes.iter().enumerate().all(|(j, pl)| {
            if j == c {
                return true;
            }
            side_at(pl, &pfl, &plane_f(pl), &p) == cov[j]
        });
        if ok && cls.planes[c].side(&p) == s {
            return p;
        }
        k += 6;
        assert!(k < 4000, "no point off a facet inside its side");
    }
}

/// The op as the core's rule.
fn rule_of(op: Op) -> Rule {
    match op {
        Op::Union => Rule::Union,
        Op::Subtract => Rule::Diff,
        Op::Intersect => Rule::Inter,
    }
}

/// What the bridge made of a boolean: the ledger, the core's outcome and the body.
pub struct Decided {
    pub ledger: Ledger,
    pub outcome: Outcome,
}

/// What `fill` keeps: the faces, their snapped planes, the classes, the exact vertices, the ledger.
type Filled = (Vec<In>, Vec<DyPlane>, Classes, Vec<Pts>, Built, Region);

/// The ledger of `bodies`, filled, with the witnesses answered; and what the bridge kept.
fn fill(bodies: &[&Body], rule: &Rule) -> Result<Filled, Skip> {
    let fs = inputs(bodies)?;
    let (snapped, reps) = snap(&fs)?;
    let reg = region(bodies, &fs, &snapped, rule);
    let pts = vertices(bodies, &fs, &snapped, &reg)?;
    let mut cls = classes(&snapped, &reps, &reg.near);
    if let Some((lo, hi)) = reg.bx {
        add_walls(&mut cls, lo, hi);
    }
    decided!(log(bodies, &fs, &cls, &pts, &snapped));
    let mut built = build_ledger(bodies, &fs, &cls, &pts, &snapped, reg.bx);
    // an operand whose own faces overlap (instances of a pattern in one body, say) is not a
    // solid the ledger can describe: the old path takes it
    if why_split(&built.ledger).is_some() {
        return Err(Skip::Ambiguous("an operand's own faces overlap"));
    }
    let ws = core::witnesses(&built.ledger);
    // per face: its outward normal along its class plane's (+1) or against it (-1)
    let out_sign: Vec<i32> = fs
        .iter()
        .enumerate()
        .map(|(g, f)| {
            if cls.of_face[g] == NONE {
                return 0;
            }
            let along = f.plane.n.dot(&cls.planes[cls.of_face[g] as usize].n).signum();
            if bodies[f.body].face(f.face).same_sense {
                along
            } else {
                -along
            }
        })
        .collect();
    // per body: the box of its faces, grown
    let g = 4.0 * lin();
    let mut bbox = vec![([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]); bodies.len()];
    for f in &fs {
        for k in 0..3 {
            bbox[f.body].0[k] = bbox[f.body].0[k].min(f.lo[k] - g);
            bbox[f.body].1[k] = bbox[f.body].1[k].max(f.hi[k] + g);
        }
    }
    let mut inside = Vec::new();
    for (t, i) in ws {
        let body = i.0 as usize;
        if witness_inside(&built, &cls, &fs, &snapped, &pts, &out_sign, &bbox, t.0, body) {
            inside.push((t, i));
        }
    }
    built.ledger.inside = inside;
    Ok((fs, snapped, cls, pts, built, reg))
}

/// Under the decision log: the facts the ledger decides that the old path's sites decide too, each
/// recorded once, where it is decided: two faces one surface (one class), for every pair of faces of
/// different bodies whose grown boxes meet and whose planes are within the band; and vertex on
/// surface (on the face's class plane, exactly), for every vertex within the band of another body's
/// face.
#[cfg(feature = "decisions")]
fn log(bodies: &[&Body], fs: &[In], cls: &Classes, pts: &[Pts], snapped: &[DyPlane]) {
    use crate::decisions::{BAND, Ent, Kind, Site, decide, key2, pair};
    let l = lin();
    let grown = |f: &In, g: f64, p: [f64; 3]| (0..3).all(|k| f.lo[k] - g <= p[k] && p[k] <= f.hi[k] + g);
    for (i, a) in fs.iter().enumerate() {
        for (j, b) in fs.iter().enumerate().skip(i + 1) {
            if a.body == b.body || region_extent(a, b, l).is_none() || plainly_apart(a, b, BAND / (2.0 * l)) {
                continue;
            }
            if cls.of_face[i] == NONE || cls.of_face[j] == NONE {
                continue;
            }
            let key = pair(Ent::f(bodies[a.body], a.face), Ent::f(bodies[b.body], b.face));
            let one = cls.of_face[i] == cls.of_face[j];
            decide(
                Kind::Coincident,
                key,
                None,
                None,
                one as u8,
                Site::Named("decide-once: surface table"),
                f64::NAN,
                l,
            );
        }
    }
    let _ = snapped;
    for (k, vs) in pts.iter().enumerate() {
        let vs = vs.exact.borrow();
        let mut ids: Vec<_> = vs.keys().copied().collect();
        ids.sort();
        for v in ids {
            let h = &vs[&v];
            let p = h.to_f64();
            for (j, f) in fs.iter().enumerate() {
                if f.body == k || cls.of_face[j] == NONE || !grown(f, BAND, p) {
                    continue;
                }
                let pl = &cls.planes[cls.of_face[j] as usize];
                let pf = plane_f(pl);
                let len = (pf.n[0] * pf.n[0] + pf.n[1] * pf.n[1] + pf.n[2] * pf.n[2]).sqrt();
                let d = (pf.n[0] * p[0] + pf.n[1] * p[1] + pf.n[2] * p[2] - pf.c) / len;
                if d.abs() > BAND {
                    continue;
                }
                let on = pl.side(h) == 0;
                let key = key2(Ent::v(bodies[k], v), Ent::f(bodies[f.body], f.face));
                let at = Some(Point3::new(p[0], p[1], p[2]));
                decide(
                    Kind::OnSurface,
                    key,
                    at,
                    None,
                    on as u8,
                    Site::Named("decide-once: side table"),
                    d.abs(),
                    l,
                );
            }
        }
    }
}

/// Whether side cell `t` is inside body `body` (a witness answer), decided by the first of:
/// - outside the body's box (grown) is outside;
/// - beside one of the body's own faces, it is inside iff on that face's inner side (the oriented
///   face of a closed, valid body says exactly what the ray parity says);
/// - otherwise one ray (`Ref.Face.pierced`), from a point strictly inside the cell.
#[allow(clippy::too_many_arguments)]
fn witness_inside(
    b: &Built,
    cls: &Classes,
    fs: &[In],
    snapped: &[DyPlane],
    pts: &[Pts],
    out_sign: &[i32],
    bbox: &[([f64; 3], [f64; 3])],
    t: u32,
    body: usize,
) -> bool {
    let (f0, _) = b.side_of[&t];
    let m = b.facet[&f0].mid.to_f64();
    let (lo, hi) = &bbox[body];
    if (0..3).any(|k| m[k] < lo[k] || m[k] > hi[k]) {
        return false;
    }
    for &(f, s) in &b.sides[&t] {
        if let Some(hs) = b.holders.get(&f) {
            for &g in hs {
                if fs[g].body == body {
                    return s != out_sign[g];
                }
            }
        }
    }
    let p = witness_point(b, cls, t);
    let pf = p.to_f64();
    let gr = 4.0 * lin();
    let k = fs
        .iter()
        .enumerate()
        .filter(|(fi, f)| {
            f.body == body
                && f.hi[0] + gr >= pf[0]
                && (1..3).all(|a| f.lo[a] - gr <= pf[a] && pf[a] <= f.hi[a] + gr)
                && pierced(f, &snapped[*fi], &pts[body], snapped, &p)
        })
        .count();
    k % 2 == 1
}

/// The ledger and the core's answer for a boolean of flat bodies (for tests and the comparison).
pub fn decide(bodies: &[&Body], rule: &Rule) -> Result<Decided, Skip> {
    let (_, _, _, _, built, _) = fill(bodies, rule)?;
    let outcome = core::core(&built.ledger, rule, &core::MetricLedger::default());
    Ok(Decided {
        ledger: built.ledger,
        outcome,
    })
}

/// The boolean of two flat bodies by the decide-once path: `Ok(None)` where it does not apply
/// (not flat, or ambiguous under §6.1), so the old path runs.
pub fn boolean(a: &Body, b: &Body, op: Op) -> Result<Option<Body>, BoolError> {
    nary(&[a, b], &rule_of(op))
}

/// Every face a plane and every edge a line.
pub fn flat(b: &Body) -> bool {
    b.edge_ids().all(|e| matches!(b.edge(e).curve, Curve::Line(_)))
        && b.face_ids()
            .all(|f| matches!(b.face(f).surface, Surface::Plane(_)))
}

/// A history `((t0 op0 t1) op1 t2) ..` of flat tools as one call: its pointwise table.
pub fn history(tools: &[&Body], ops: &[Op]) -> Result<Option<Body>, BoolError> {
    if tools.len() > 16 || tools.len() != ops.len() + 1 || !tools.iter().all(|b| flat(b)) {
        return Ok(None);
    }
    let nb = tools.len();
    let table = (0..1usize << nb)
        .map(|m| {
            let mut r = m & 1 == 1;
            for (i, op) in ops.iter().enumerate() {
                let x = (m >> (i + 1)) & 1 == 1;
                r = match op {
                    Op::Union => r || x,
                    Op::Subtract => r && !x,
                    Op::Intersect => r && x,
                };
            }
            r
        })
        .collect();
    nary(tools, &Rule::Table(table))
}

/// Any number of flat bodies under one pointwise rule, in one call.
pub fn nary(bodies: &[&Body], rule: &Rule) -> Result<Option<Body>, BoolError> {
    #[cfg(feature = "decisions")]
    let _scope = crate::decisions::scope("decide-once", String::from("plain"));
    let (fs, snapped, cls, pts, built, reg) = match fill(bodies, rule) {
        Ok(x) => x,
        Err(_) => return Ok(None),
    };
    let outcome = core::core(&built.ledger, rule, &core::MetricLedger::default());
    if std::env::var_os("FORGE_DECIDE_STATS").is_some() {
        eprintln!(
            "decide-once: {} bodies, {} faces ({} near), {} classes, {} cells, {} pieces, {} edges, {} verts, {} witnesses",
            bodies.len(),
            fs.len(),
            reg.near.iter().filter(|&&n| n).count(),
            cls.planes.len(),
            built.ledger.cells.len(),
            built.ledger.pieces.len(),
            built.ledger.edges.len(),
            built.ledger.verts.len(),
            built.ledger.inside.len()
        );
    }
    let w = Wr {
        bodies,
        fs: &fs,
        cls: &cls,
        built: &built,
        pts: &pts,
        snapped: &snapped,
        reg: &reg,
        pf: cls.planes.iter().map(plane_f).collect(),
        at: RefCell::new(HashMap::new()),
    };
    let written = |out: Option<&core::Body>| {
        write(&w, out).unwrap_or_else(|| panic!("decide-once: the result's segments do not close into loops"))
    };
    match outcome {
        Outcome::Empty => Ok(Some(written(None))),
        Outcome::Refused(Refusal::ZeroThickness(c)) => Err(BoolError::ZeroThickness {
            at: cell_point(&built, c.0),
        }),
        Outcome::Refused(r) => panic!(
            "the decide-once core refused {r:?}: its ledger is not consistent ({})",
            why_split(&built.ledger).unwrap_or_default()
        ),
        Outcome::Ok(out) => {
            // §6.1 step 3: a sub-LINEAR feature of the exact result is refused where it lies
            if let Some(at) = sliver(&cls, &built, &out) {
                return Err(BoolError::ZeroThickness { at });
            }
            Ok(Some(written(Some(&out))))
        }
    }
}

/// Which of `splitOk`'s conditions a ledger breaks, in words (a bridge bug's report).
fn why_split(l: &Ledger) -> Option<String> {
    let mut seen: HashMap<(u32, u32), u32> = HashMap::new();
    for &(c, f) in &l.pieces {
        let x = &l.cells[c.0 as usize];
        let zeros: Vec<usize> = (0..x.len()).filter(|&k| x[k] == Sgn::Zero).collect();
        if zeros.len() != 1 {
            return Some(format!(
                "piece of face {} has zeros {zeros:?}, face class {}",
                f.0, l.face_class[f.0 as usize]
            ));
        }
        if zeros[0] as u32 != l.face_class[f.0 as usize] {
            return Some(format!(
                "piece of face {} on class {}, face class {}",
                f.0, zeros[0], l.face_class[f.0 as usize]
            ));
        }
        let b = l.face_body[f.0 as usize].0;
        if let Some(&g) = seen.get(&(c.0, b))
            && g != f.0
        {
            return Some(format!("facet {} inside faces {g} and {} of body {b}", c.0, f.0));
        }
        seen.insert((c.0, b), f.0);
    }
    None
}

/// §6.1 step 3 on the core's body: an edge shorter than LINEAR, or a vertex within LINEAR of a
/// face it is not on (over the face's box), is a sub-LINEAR feature of the exact result. It is not
/// collapsed: the boolean refuses there (ZeroThickness), the one answer besides a valid body.
fn sliver(cls: &Classes, b: &Built, out: &core::Body) -> Option<Point3> {
    let l = lin();
    let pt = |v: u32| {
        let p = b.point[&v].to_f64();
        Point3::new(p[0], p[1], p[2])
    };
    for e in &out.edges {
        let mut count: HashMap<u32, u32> = HashMap::new();
        for c in &e.cells {
            let (u, v) = b.ends[&c.0];
            *count.entry(u).or_default() += 1;
            *count.entry(v).or_default() += 1;
        }
        let ends: Vec<u32> = count.iter().filter(|(_, n)| **n == 1).map(|(v, _)| *v).collect();
        if ends.len() == 2 && pt(ends[0]).distance(pt(ends[1])) < l {
            let (p, q) = (pt(ends[0]), pt(ends[1]));
            return Some(p + (q - p) * 0.5);
        }
    }
    // each face's vertices and box
    let verts: Vec<(u32, Point3)> = out.verts.iter().map(|v| (v.0, pt(v.0))).collect();
    for f in &out.faces {
        let mut on: HashMap<u32, ()> = HashMap::new();
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for c in &f.facets {
            for &v in &b.facet[&c.0].poly {
                on.insert(v, ());
                let p = b.point[&v].to_f64();
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        let pf = plane_f(&cls.planes[f.plane as usize]);
        let len = (pf.n[0] * pf.n[0] + pf.n[1] * pf.n[1] + pf.n[2] * pf.n[2]).sqrt();
        for &(v, p) in &verts {
            if on.contains_key(&v) {
                continue;
            }
            let q = [p.x, p.y, p.z];
            if (0..3).any(|k| q[k] < lo[k] - l || q[k] > hi[k] + l) {
                continue;
            }
            let d = (pf.n[0] * q[0] + pf.n[1] * q[1] + pf.n[2] * q[2] - pf.c) / len;
            if d.abs() < l && cls.planes[f.plane as usize].side(&b.point[&v]) != 0 {
                return Some(p);
            }
        }
    }
    None
}

/// `name`, or a piece of it no entity of the body has yet.
fn unique(used: &mut HashMap<Name, ()>, name: Name) -> Name {
    let mut n = name;
    let mut k = 0u64;
    while used.contains_key(&n) {
        k += 1;
        n = name.piece(Name::NONE.with_name(name).with(k));
    }
    used.insert(n, ());
    n
}

/// A point of a cell, for a refusal's locus.
fn cell_point(b: &Built, c: u32) -> Point3 {
    let f = |h: &HPoint| {
        let p = h.to_f64();
        Point3::new(p[0], p[1], p[2])
    };
    if let Some(h) = b.point.get(&c) {
        return f(h);
    }
    if let Some(&(u, v)) = b.ends.get(&c) {
        let (p, q) = (f(&b.point[&u]), f(&b.point[&v]));
        return p + (q - p) * 0.5;
    }
    Point3::new(0.0, 0.0, 0.0)
}

/// A point of the result being written: a vertex cell of the arrangement, or a vertex of body 0
/// kept from outside the region.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum PK {
    Cell(u32),
    Orig(VertexId),
}

/// A directed boundary segment of a final face, `a` to `b` with the face on its left about the
/// outward normal, and the input face it comes from (its name).
#[derive(Clone, Copy, Debug)]
struct Seg {
    a: PK,
    b: PK,
    src: usize,
}

/// A pool of segments that forms one or more final faces: a class plane and outward sense, or a
/// far face kept whole.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Grp {
    Plane(u32, i8),
    Far(usize),
}

/// A final face before it is built.
struct Fin {
    grp: Grp,
    /// loops of segments (outer first)
    loops: Vec<Vec<Seg>>,
    /// the least input face it holds a segment of
    src: usize,
}

/// What the write reads.
struct Wr<'a> {
    bodies: &'a [&'a Body],
    fs: &'a [In],
    cls: &'a Classes,
    built: &'a Built,
    pts: &'a [Pts],
    snapped: &'a [DyPlane],
    reg: &'a Region,
    pf: Vec<PlaneF>,
    at: RefCell<HashMap<PK, [f64; 3]>>,
}

impl Wr<'_> {
    fn pos(&self, k: PK) -> [f64; 3] {
        if let Some(p) = self.at.borrow().get(&k) {
            return *p;
        }
        let p = match k {
            PK::Cell(c) => self.built.point[&c].to_f64(),
            PK::Orig(v) => {
                let q = self.bodies[0].vertex(v).point;
                [q.x, q.y, q.z]
            }
        };
        self.at.borrow_mut().insert(k, p);
        p
    }

    /// The outward normal of a group, in f64.
    fn normal(&self, g: Grp) -> [f64; 3] {
        let (n, s) = match g {
            Grp::Plane(c, o) => (self.pf[c as usize].n, o as f64),
            Grp::Far(i) => {
                let pf = plane_f(&self.fs[i].plane);
                let ss = self.bodies[self.fs[i].body].face(self.fs[i].face).same_sense;
                (pf.n, if ss { 1.0 } else { -1.0 })
            }
        };
        [n[0] * s, n[1] * s, n[2] * s]
    }

    /// The vertex cell where an input edge on faces `g` and `k` (both near) crosses wall `w`.
    fn crossing(&self, g: usize, k: usize, w: u32) -> Option<PK> {
        let (cg, ck) = (self.cls.of_face[g], self.cls.of_face[k]);
        if cg == NONE || ck == NONE {
            return None;
        }
        let h = meet3(
            &self.cls.planes[cg as usize],
            &self.cls.planes[ck as usize],
            &self.cls.planes[w as usize],
        )?;
        let cv = covector(self.cls, &self.pf, &h, &[cg, ck, w]);
        self.built.by.get(&cv).map(|&c| PK::Cell(c))
    }
}

/// Projection axes of a plane with outward normal `n`, so that (s, t) runs counter-clockwise about
/// it.
fn axes_of(n: [f64; 3]) -> (usize, usize) {
    let a = (0..3)
        .max_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()).then(j.cmp(&i)))
        .unwrap();
    let (u, v) = ((a + 1) % 3, (a + 2) % 3);
    if n[a] > 0.0 { (u, v) } else { (v, u) }
}

/// Chains a pool's segments into loops; at a point the boundary passes more than once, the next
/// segment is the first clockwise from the way back.
fn chain(w: &Wr, segs: &[Seg], n: [f64; 3]) -> Option<Vec<Vec<Seg>>> {
    let ax = axes_of(n);
    let xy = |k: PK| {
        let p = w.pos(k);
        (p[ax.0], p[ax.1])
    };
    let mut from: std::collections::BTreeMap<PK, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, s) in segs.iter().enumerate() {
        from.entry(s.a).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for s0 in 0..segs.len() {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let mut lp = vec![segs[s0]];
        let start = segs[s0].a;
        let mut cur = s0;
        let mut guard = 0;
        while segs[cur].b != start || lp.is_empty() {
            let at = segs[cur].b;
            let cands: Vec<usize> = from.get(&at)?.iter().copied().filter(|&j| !used[j]).collect();
            let next = match cands.len() {
                0 => return None,
                1 => cands[0],
                _ => {
                    let (pv, pu) = (xy(at), xy(segs[cur].a));
                    let back = forge_math::dmath::atan2(pu.1 - pv.1, pu.0 - pv.0);
                    let mut best = (f64::INFINITY, cands[0]);
                    for &j in &cands {
                        let pw = xy(segs[j].b);
                        let out = forge_math::dmath::atan2(pw.1 - pv.1, pw.0 - pv.0);
                        let mut turn = back - out;
                        while turn <= 0.0 {
                            turn += std::f64::consts::TAU;
                        }
                        if turn < best.0 {
                            best = (turn, j);
                        }
                    }
                    best.1
                }
            };
            used[next] = true;
            lp.push(segs[next]);
            cur = next;
            guard += 1;
            if guard > segs.len() {
                return None;
            }
        }
        loops.push(lp);
    }
    Some(loops)
}

/// Twice a loop's signed area in the projection, counter-clockwise positive.
fn area(w: &Wr, lp: &[Seg], ax: (usize, usize)) -> f64 {
    lp.iter()
        .map(|s| {
            let (p, q) = (w.pos(s.a), w.pos(s.b));
            p[ax.0] * q[ax.1] - q[ax.0] * p[ax.1]
        })
        .sum()
}

/// Is the point inside the loop (projected; f64, for telling holes their face).
fn inside_loop(w: &Wr, lp: &[Seg], ax: (usize, usize), q: (f64, f64)) -> bool {
    let mut c = false;
    for s in lp {
        let (p, r) = (w.pos(s.a), w.pos(s.b));
        let (a, b) = ((p[ax.0], p[ax.1]), (r[ax.0], r[ax.1]));
        if (a.1 > q.1) != (b.1 > q.1) && q.0 < a.0 + (q.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
            c = !c;
        }
    }
    c
}

/// The body the core's answer describes, with everything outside the region passed through
/// (§3a's write), written fresh. `None` only if the segments do not close into loops, which a
/// valid ledger never gives.
fn write(w: &Wr, out: Option<&core::Body>) -> Option<Body> {
    let (bodies, fs, cls, built, reg) = (w.bodies, w.fs, w.cls, w.built, w.reg);
    let l = &built.ledger;
    let on_wall = |cv: &[Sgn]| cls.walls.iter().any(|&x| cv[x as usize] == Sgn::Zero);
    let mut pools: std::collections::BTreeMap<Grp, Vec<Seg>> = std::collections::BTreeMap::new();
    // body 0's global face index by FaceId
    let mut gidx: HashMap<FaceId, usize> = HashMap::new();
    for (g, f) in fs.iter().enumerate() {
        if f.body == 0 {
            gidx.insert(f.face, g);
        }
    }
    // 1. outside the region: far faces whole, near faces clipped at the walls
    if let Some((lo, hi)) = reg.bx
        && reg.keeps_target
    {
        let b0 = bodies[0];
        let inside = |p: [f64; 3]| (0..3).all(|k| lo[k] < p[k] && p[k] < hi[k]);
        for (g, f) in fs.iter().enumerate() {
            if f.body != 0 {
                continue;
            }
            let face = b0.face(f.face);
            for &lp in face.loops.iter() {
                for c in b0.lp(lp).coedges() {
                    let (u, v) = (b0.coedge_start(c), b0.coedge_end(c));
                    if !reg.near[g] {
                        pools.entry(Grp::Far(g)).or_default().push(Seg {
                            a: PK::Orig(u),
                            b: PK::Orig(v),
                            src: g,
                        });
                        continue;
                    }
                    let grp = Grp::Plane(cls.of_face[g], w_out(w, g) as i8);
                    let (pu, pv) = (w.pos(PK::Orig(u)), w.pos(PK::Orig(v)));
                    let (iu, iv) = (inside(pu), inside(pv));
                    if iu && iv {
                        continue;
                    }
                    // the segment's span in the box (Liang–Barsky), and the walls it enters and
                    // leaves by
                    let (mut t0, mut t1) = (0.0f64, 1.0f64);
                    let (mut w0, mut w1) = (None, None);
                    let mut miss = false;
                    for k in 0..3 {
                        let d = pv[k] - pu[k];
                        if d == 0.0 {
                            if pu[k] <= lo[k] || pu[k] >= hi[k] {
                                miss = true;
                            }
                            continue;
                        }
                        let (ta, tb) = ((lo[k] - pu[k]) / d, (hi[k] - pu[k]) / d);
                        let (wa, wb) = (cls.walls[2 * k], cls.walls[2 * k + 1]);
                        let ((tn, wn), (tx, wx)) = if ta < tb {
                            ((ta, wa), (tb, wb))
                        } else {
                            ((tb, wb), (ta, wa))
                        };
                        if tn > t0 {
                            t0 = tn;
                            w0 = Some(wn);
                        }
                        if tx < t1 {
                            t1 = tx;
                            w1 = Some(wx);
                        }
                    }
                    let other = || -> Option<usize> {
                        let e = b0.coedge(c).edge;
                        let pc = b0.edge(e).coedges.iter().copied().find(|&x| x != c)?;
                        gidx.get(&b0.coedge_face(pc)).copied()
                    };
                    let pool = pools.entry(grp).or_default();
                    if miss || t0 >= t1 {
                        pool.push(Seg {
                            a: PK::Orig(u),
                            b: PK::Orig(v),
                            src: g,
                        });
                        continue;
                    }
                    let k = other()?;
                    if !iu {
                        let x = w.crossing(g, k, w0?)?;
                        pool.push(Seg {
                            a: PK::Orig(u),
                            b: x,
                            src: g,
                        });
                    }
                    if !iv {
                        let x = w.crossing(g, k, w1?)?;
                        pool.push(Seg {
                            a: x,
                            b: PK::Orig(v),
                            src: g,
                        });
                    }
                }
            }
        }
    }
    // 2. inside: the core's faces, their boundary off the walls
    if let Some(out) = out {
        for f in &out.faces {
            let grp = Grp::Plane(f.plane, if f.orient == Sgn::Pos { 1 } else { -1 });
            let mut count: HashMap<u32, u32> = HashMap::new();
            let mut segs = Vec::new();
            for c in &f.facets {
                let mut poly = built.facet[&c.0].poly.clone();
                if f.orient == Sgn::Neg {
                    poly.reverse();
                }
                for k in 0..poly.len() {
                    let (u, v) = (poly[k], poly[(k + 1) % poly.len()]);
                    let e = edge_cell(built, u, v)?;
                    *count.entry(e).or_default() += 1;
                    segs.push((u, v, e));
                }
            }
            let pool = pools.entry(grp).or_default();
            for (u, v, e) in segs {
                if count[&e] == 1 && !on_wall(&l.cells[e as usize]) {
                    pool.push(Seg {
                        a: PK::Cell(u),
                        b: PK::Cell(v),
                        src: f.origin.0 as usize,
                    });
                }
            }
        }
    }
    // 3. the final faces: each pool's loops, holes given to the outer loop round them
    let mut fins: Vec<Fin> = Vec::new();
    for (&grp, segs) in &pools {
        if segs.is_empty() {
            continue;
        }
        let n = w.normal(grp);
        let ax = axes_of(n);
        let loops = chain(w, segs, n)?;
        let src_of = |lp: &[Seg]| lp.iter().map(|s| s.src).min().unwrap_or(usize::MAX);
        if let Grp::Far(g) = grp {
            let mut loops = loops;
            let mut best = 0;
            for k in 1..loops.len() {
                if area(w, &loops[k], ax) > area(w, &loops[best], ax) {
                    best = k;
                }
            }
            loops.swap(0, best);
            fins.push(Fin { grp, loops, src: g });
            continue;
        }
        let (mut outer, mut holes) = (Vec::new(), Vec::new());
        for lp in loops {
            if area(w, &lp, ax) > 0.0 {
                outer.push(lp);
            } else {
                holes.push(lp);
            }
        }
        let mut faces: Vec<Vec<Vec<Seg>>> = outer.into_iter().map(|o| vec![o]).collect();
        for h in holes {
            let (p, q) = (w.pos(h[0].a), w.pos(h[0].b));
            let m = ((p[ax.0] + q[ax.0]) / 2.0, (p[ax.1] + q[ax.1]) / 2.0);
            let mut best: Option<(f64, usize)> = None;
            for (k, fl) in faces.iter().enumerate() {
                if inside_loop(w, &fl[0], ax, m) {
                    let a = area(w, &fl[0], ax);
                    if best.is_none_or(|(b, _)| a < b) {
                        best = Some((a, k));
                    }
                }
            }
            faces[best?.1].push(h);
        }
        for fl in faces {
            let src = fl.iter().map(|lp| src_of(lp)).min().unwrap_or(usize::MAX);
            fins.push(Fin { grp, loops: fl, src });
        }
    }
    // in input arena order (of the least input face each holds)
    fins.sort_by_key(|f| f.src);
    if fins.is_empty() {
        return Some(Body::default());
    }
    build_body(w, &fins)
}

/// A near face's outward sense against its class plane.
fn w_out(w: &Wr, g: usize) -> i32 {
    let f = &w.fs[g];
    let along = f.plane.n.dot(&w.cls.planes[w.cls.of_face[g] as usize].n).signum();
    if w.bodies[f.body].face(f.face).same_sense {
        along
    } else {
        -along
    }
}

/// The body of the final faces: vertices where three or more faces meet, an edge per straight run
/// between two, shells by adjacency; names kept where the entity is the input's.
fn build_body(w: &Wr, fins: &[Fin]) -> Option<Body> {
    let (bodies, fs, cls, built) = (w.bodies, w.fs, w.cls, w.built);
    let face_name = |g: usize| bodies[fs[g].body].face(fs[g].face).name;
    // the faces round each point
    let mut round: std::collections::BTreeMap<PK, Vec<usize>> = std::collections::BTreeMap::new();
    for (fi, f) in fins.iter().enumerate() {
        for lp in &f.loops {
            for s in lp {
                let r = round.entry(s.a).or_default();
                if !r.contains(&fi) {
                    r.push(fi);
                }
            }
        }
    }
    let is_vert = |k: PK| round.get(&k).is_some_and(|r| r.len() >= 3);
    // runs: each loop cut at its vertices
    struct Run {
        a: PK,
        b: PK,
    }
    let mut face_runs: Vec<Vec<Vec<Run>>> = Vec::new();
    let mut edge_faces: std::collections::BTreeMap<(PK, PK), Vec<usize>> = std::collections::BTreeMap::new();
    for (fi, f) in fins.iter().enumerate() {
        let mut lruns = Vec::new();
        for lp in &f.loops {
            let first = lp.iter().position(|s| is_vert(s.a))?;
            let n = lp.len();
            let mut runs = Vec::new();
            let mut a = lp[first].a;
            for k in 0..n {
                let s = &lp[(first + k) % n];
                if is_vert(s.b) {
                    runs.push(Run { a, b: s.b });
                    a = s.b;
                }
            }
            for r in &runs {
                let key = if r.a <= r.b { (r.a, r.b) } else { (r.b, r.a) };
                edge_faces.entry(key).or_default().push(fi);
            }
            lruns.push(runs);
        }
        face_runs.push(lruns);
    }
    // shells: faces joined across edges
    let mut uf = Uf::new(fins.len());
    for fsx in edge_faces.values() {
        if fsx.len() != 2 {
            return None;
        }
        uf.union(fsx[0], fsx[1]);
    }
    // names
    let mut per_src: HashMap<usize, u64> = HashMap::new();
    for f in fins {
        *per_src.entry(f.src).or_default() += 1;
    }
    let mut seen: HashMap<usize, u64> = HashMap::new();
    let mut used: HashMap<Name, ()> = HashMap::new();
    let mut fnames = Vec::new();
    for f in fins {
        let base = face_name(f.src);
        let name = if per_src[&f.src] == 1 {
            base
        } else {
            let k = seen.entry(f.src).or_default();
            *k += 1;
            base.piece(Name::NONE.with_name(base).with(*k))
        };
        fnames.push(unique(&mut used, name));
    }
    // the input's vertices by their place in the result, for points and names kept
    let mut orig_v: HashMap<u32, (usize, VertexId)> = HashMap::new();
    for (k, b) in bodies.iter().enumerate() {
        for (g, f) in fs.iter().enumerate() {
            if f.body != k || cls.of_face[g] == NONE {
                continue;
            }
            for lp in &f.loops {
                for &v in lp {
                    // (only a vertex in the region can be a cell)
                    if k == 0 && vid_of_orig(w, v) {
                        continue;
                    }
                    let h = w.pts[k].ph(v, w.snapped);
                    let cv = covector(cls, &w.pf, &h, &[]);
                    if let Some(&c) = built.by.get(&cv) {
                        orig_v.entry(c).or_insert((k, v));
                    }
                }
            }
        }
        let _ = b;
    }
    let mut body = Body::default();
    let mut vid: HashMap<PK, VertexId> = HashMap::new();
    let mut vused: HashMap<Name, ()> = HashMap::new();
    let mut vertex = |body: &mut Body, k: PK| -> VertexId {
        if let Some(&v) = vid.get(&k) {
            return v;
        }
        let (p, name) = match k {
            PK::Orig(v) => {
                let x = bodies[0].vertex(v);
                (x.point, x.name)
            }
            PK::Cell(c) => match orig_v.get(&c) {
                Some(&(b, v)) => {
                    let x = bodies[b].vertex(v);
                    (x.point, x.name)
                }
                None => {
                    let q = w.pos(k);
                    (
                        Point3::new(q[0], q[1], q[2]),
                        Name::NONE.with_str("decide-once vertex").with(c as u64),
                    )
                }
            },
        };
        let v = body.add_vertex(p, unique(&mut vused, name));
        vid.insert(k, v);
        v
    };
    // the input's edges by their two vertices' places, for names kept
    let placed: RefCell<HashMap<(usize, VertexId), Option<PK>>> = RefCell::new(HashMap::new());
    let place = |b: usize, v: VertexId| -> Option<PK> {
        if let Some(&p) = placed.borrow().get(&(b, v)) {
            return p;
        }
        let p = if b == 0 && vid_of_orig(w, v) {
            Some(PK::Orig(v))
        } else {
            let h = w.pts[b].ph(v, w.snapped);
            let cv = covector(cls, &w.pf, &h, &[]);
            built.by.get(&cv).map(|&c| PK::Cell(c))
        };
        placed.borrow_mut().insert((b, v), p);
        p
    };
    let mut orig_e: HashMap<(PK, PK), Name> = HashMap::new();
    let gof: HashMap<(usize, FaceId), usize> = fs
        .iter()
        .enumerate()
        .map(|(g, f)| ((f.body, f.face), g))
        .collect();
    for (k, b) in bodies.iter().enumerate() {
        for e in b.edge_ids() {
            let ed = b.edge(e);
            let touches_near = ed.coedges.iter().any(|&c| {
                gof.get(&(k, b.coedge_face(c)))
                    .is_some_and(|&g| w.reg.near[g] || k == 0)
            });
            if !touches_near {
                continue;
            }
            if let (Some(a), Some(z)) = (place(k, ed.start), place(k, ed.end)) {
                let key = if a <= z { (a, z) } else { (z, a) };
                orig_e.entry(key).or_insert(ed.name);
            }
        }
    }
    let mut shell_of: HashMap<usize, forge_topo::ShellId> = HashMap::new();
    let mut eid: HashMap<(PK, PK), (EdgeId, PK)> = HashMap::new();
    let mut eused: HashMap<Name, ()> = HashMap::new();
    for (fi, f) in fins.iter().enumerate() {
        let root = uf.find(fi);
        let shell = *shell_of.entry(root).or_insert_with(|| body.add_shell());
        let (surface, same_sense) = match f.grp {
            Grp::Far(g) => {
                let x = bodies[fs[g].body].face(fs[g].face);
                (x.surface.clone(), x.same_sense)
            }
            Grp::Plane(c, o) => {
                let cp = &cls.planes[c as usize];
                let src = if fs[f.src].plane.same_set(cp) {
                    f.src
                } else {
                    cls.rep[c as usize]
                };
                let along = fs[src].plane.n.dot(&cp.n).signum() > 0;
                (
                    bodies[fs[src].body].face(fs[src].face).surface.clone(),
                    along == (o > 0),
                )
            }
        };
        let face = body.add_face(shell, surface, same_sense, fnames[fi]);
        for runs in &face_runs[fi] {
            let mut uses = Vec::new();
            for r in runs {
                let key = if r.a <= r.b { (r.a, r.b) } else { (r.b, r.a) };
                let (e, start) = match eid.get(&key) {
                    Some(&x) => x,
                    None => {
                        let (va, vb) = (vertex(&mut body, r.a), vertex(&mut body, r.b));
                        let (pa, pb) = (body.vertex(va).point, body.vertex(vb).point);
                        let line = Line::through(pa, pb)?;
                        let name = match orig_e.get(&key) {
                            Some(&n) => n,
                            None => {
                                let names: Vec<Name> = edge_faces[&key].iter().map(|&x| fnames[x]).collect();
                                Name::NONE.with_str("decide-once edge").with_set(&names)
                            }
                        };
                        let e = body.add_edge(
                            Curve::Line(line),
                            Interval::new(0.0, pa.distance(pb)),
                            va,
                            vb,
                            unique(&mut eused, name),
                        );
                        eid.insert(key, (e, r.a));
                        (e, r.a)
                    }
                };
                uses.push((e, start == r.a));
            }
            body.add_loop(face, &uses);
        }
    }
    // joins: the inputs', and an input face whose name no final face has went to the first that
    // holds a segment of it
    for b in bodies {
        body.joins.extend(&b.joins);
    }
    let mut named: HashMap<Name, ()> = fnames.iter().map(|&n| (n, ())).collect();
    for (fi, f) in fins.iter().enumerate() {
        for lp in &f.loops {
            for s in lp {
                let n = face_name(s.src);
                if named.insert(n, ()).is_none() {
                    body.joins.aliases.push((n, fnames[fi]));
                }
            }
        }
    }
    Some(body)
}

/// Is body 0's vertex `v` outside the region (a point the write keeps by its own identity)?
fn vid_of_orig(w: &Wr, v: VertexId) -> bool {
    match w.reg.bx {
        None => false,
        Some((lo, hi)) => {
            let q = w.bodies[0].vertex(v).point;
            let q = [q.x, q.y, q.z];
            !(0..3).all(|k| lo[k] < q[k] && q[k] < hi[k])
        }
    }
}

/// The edge cell between two vertex cells of a facet.
fn edge_cell(b: &Built, u: u32, v: u32) -> Option<u32> {
    let cu = &b.ledger.cells[u as usize];
    let cv = &b.ledger.cells[v as usize];
    b.by.get(&join(&[cu, cv])).copied()
}
