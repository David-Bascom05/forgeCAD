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
use forge_math::exact::{Dy, DyPlane, DyV3, HPoint, meet3, orient2};
use forge_math::{Interval, Point3};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
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
            for &lp in face.loops.iter() {
                let mut vs = Vec::new();
                for c in b.lp(lp).coedges() {
                    let v = b.coedge_start(c);
                    let p = b.vertex(v).point;
                    for (a, x) in [p.x, p.y, p.z].into_iter().enumerate() {
                        lo[a] = lo[a].min(x);
                        hi[a] = hi[a].max(x);
                    }
                    vs.push(v);
                }
                loops.push(vs);
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
fn vertices(bodies: &[&Body], fs: &[In], planes: &[DyPlane]) -> Result<Vec<HashMap<VertexId, HPoint>>, Skip> {
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
    for rk in round {
        let mut m = HashMap::new();
        let mut keys: Vec<_> = rk.keys().copied().collect();
        keys.sort();
        for v in keys {
            let ps: Vec<&DyPlane> = rk[&v].iter().map(|&i| &planes[i]).collect();
            let mut p = None;
            'find: for a in &ps {
                for b in &ps {
                    for c in &ps {
                        if let Some(q) = meet3(a, b, c) {
                            p = Some(q);
                            break 'find;
                        }
                    }
                }
            }
            let Some(p) = p else {
                return Err(Skip::Ambiguous(
                    "a vertex is not one point of the planes round it",
                ));
            };
            if ps.iter().any(|q| q.side(&p) != 0) {
                return Err(Skip::Ambiguous(
                    "a vertex is not one point of the planes round it",
                ));
            }
            m.insert(v, p);
        }
        out.push(m);
    }
    Ok(out)
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
    of_face: Vec<u32>,
    rep: Vec<usize>,
}

fn classes(fs: &[In], snapped: &[DyPlane], reps: &[usize]) -> Classes {
    let mut planes: Vec<DyPlane> = Vec::new();
    let mut rep = Vec::new();
    let mut of_face = Vec::with_capacity(fs.len());
    // by an f64 key first, so the exact comparison runs only against likely equals
    let mut by_key: HashMap<(i64, i64, i64, i64), Vec<u32>> = HashMap::new();
    for (i, p) in snapped.iter().enumerate() {
        let n = [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        // canonical sense: first nonzero component positive
        let s = if n[0] != 0.0 {
            n[0].signum()
        } else if n[1] != 0.0 {
            n[1].signum()
        } else {
            n[2].signum()
        };
        let q = |x: f64| (x * s / len * 1e6).round() as i64;
        let key = (
            q(n[0]),
            q(n[1]),
            q(n[2]),
            (p.c.to_f64() * s / len * 1e3).round() as i64,
        );
        let mut found = None;
        'look: for dk in [(0, 0, 0, 0)] {
            let _ = dk;
            if let Some(cs) = by_key.get(&key) {
                for &c in cs {
                    if planes[c as usize].same_set(p) {
                        found = Some(c);
                        break 'look;
                    }
                }
            }
        }
        // (a key on a rounding boundary: fall back to every class with a near normal)
        if found.is_none() {
            for (c, cp) in planes.iter().enumerate() {
                if cp.same_set(p) {
                    found = Some(c as u32);
                    break;
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
    Classes { planes, of_face, rep }
}

/// A vertex of the arrangement on one class plane: its exact point and, for a meet of three
/// classes, its covector.
struct Vx {
    p: HPoint,
    cov: Option<Vec<Sgn>>,
}

/// A convex polygon: (vertex, plane of the edge to the next vertex).
type Poly = Vec<(u32, Pl)>;

/// A plane a polygon's edge lies on: a class, or a wall of the working box.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
        let key = if format!("{x:?}") <= format!("{y:?}") {
            (x, y)
        } else {
            (y, x)
        };
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
                    vx.push(Vx {
                        p: h,
                        cov: Some(cov.clone()),
                    });
                    by_cov.insert(cov, vx.len() as u32 - 1);
                    vx.len() as u32 - 1
                }
            }
            _ => {
                vx.push(Vx { p: h, cov: None });
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
            _ => plane_of(p).side(&x.p),
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
fn in_face(q: &HPoint, f: &In, pts: &HashMap<VertexId, HPoint>, s: usize, t: usize) -> bool {
    let mut k = 0;
    for lp in &f.loops {
        let n = lp.len();
        for i in 0..n {
            if crosses(q, &pts[&lp[i]], &pts[&lp[(i + 1) % n]], s, t) {
                k += 1;
            }
        }
    }
    k % 2 == 1
}

/// `Ref.Face.pierced`: does the ray from `p` along +x (origin perturbed) pierce face `f`?
fn pierced(f: &In, plane: &DyPlane, pts: &HashMap<VertexId, HPoint>, p: &HPoint) -> bool {
    let nx = plane.n.x.signum();
    if nx == 0 {
        return false;
    }
    // sgn (c − n·p) · sgn nx > 0: the face ahead along +x
    if -plane.side(p) * nx <= 0 {
        return false;
    }
    in_face(p, f, pts, 1, 2)
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

fn build_ledger(bodies: &[&Body], fs: &[In], cls: &Classes, pts: &[HashMap<VertexId, HPoint>]) -> Built {
    let pf: Vec<PlaneF> = cls.planes.iter().map(plane_f).collect();
    let mut tab = CellTab::default();
    let mut facet = HashMap::new();
    let mut point = HashMap::new();
    let mut ends = HashMap::new();
    let mut side_of = HashMap::new();
    let mut pieces: Vec<(u32, u32)> = Vec::new(); // (face, cell)
    let mut edges: Vec<u32> = Vec::new();
    let mut verts: Vec<u32> = Vec::new();
    let mut is_e: HashMap<u32, ()> = HashMap::new();
    let mut is_v: HashMap<u32, ()> = HashMap::new();
    // the faces of each class, in arena order
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); cls.planes.len()];
    for (i, &c) in cls.of_face.iter().enumerate() {
        on[c as usize].push(i);
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
                    let p = pts[fs[i].body][v].to_f64();
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
            let holders: Vec<usize> = faces
                .iter()
                .copied()
                .filter(|&i| in_face(&mid, &fs[i], &pts[fs[i].body], s, t))
                .collect();
            if holders.is_empty() {
                continue;
            }
            let covs: Vec<&Vec<Sgn>> = cell
                .iter()
                .map(|&(i, _)| pl.vx[i as usize].cov.as_ref().unwrap())
                .collect();
            let fc = join(&covs);
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
            if is_v.insert(v, ()).is_none() {
                verts.push(v);
            }
            let w = poly[(k + 1) % poly.len()];
            let e = tab.id(join(&[
                &tab.cells[v as usize].clone(),
                &tab.cells[w as usize].clone(),
            ]));
            if is_e.insert(e, ()).is_none() {
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
        }
    }
    let nb = bodies.len() as u32;
    let ledger = Ledger {
        nb,
        face_body: fs.iter().map(|f| BodyId(f.body as u32)).collect(),
        face_class: cls.of_face.clone(),
        cells: tab.cells,
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
        let ok = cls.planes.iter().enumerate().all(|(j, pl)| {
            if j == c {
                return true;
            }
            let want = match cov[j] {
                Sgn::Pos => 1,
                Sgn::Neg => -1,
                Sgn::Zero => 0,
            };
            pl.side(&p) == want
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
type Filled = (
    Vec<In>,
    Vec<DyPlane>,
    Classes,
    Vec<HashMap<VertexId, HPoint>>,
    Built,
);

/// The ledger of `bodies`, filled, with the witnesses answered; and what the bridge kept.
fn fill(bodies: &[&Body]) -> Result<Filled, Skip> {
    let fs = inputs(bodies)?;
    let (snapped, reps) = snap(&fs)?;
    let pts = vertices(bodies, &fs, &snapped)?;
    let cls = classes(&fs, &snapped, &reps);
    let mut built = build_ledger(bodies, &fs, &cls, &pts);
    let ws = core::witnesses(&built.ledger);
    let mut inside = Vec::new();
    for (t, i) in ws {
        let p = witness_point(&built, &cls, t.0);
        let body = i.0 as usize;
        let k = fs
            .iter()
            .enumerate()
            .filter(|(fi, f)| f.body == body && pierced(f, &snapped[*fi], &pts[body], &p))
            .count();
        if k % 2 == 1 {
            inside.push((t, i));
        }
    }
    built.ledger.inside = inside;
    Ok((fs, snapped, cls, pts, built))
}

/// The ledger and the core's answer for a boolean of flat bodies (for tests and the comparison).
pub fn decide(bodies: &[&Body], rule: &Rule) -> Result<Decided, Skip> {
    let (_, _, _, _, built) = fill(bodies)?;
    let outcome = core::core(&built.ledger, rule);
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

/// Any number of flat bodies under one pointwise rule, in one call.
pub fn nary(bodies: &[&Body], rule: &Rule) -> Result<Option<Body>, BoolError> {
    let (fs, _snapped, cls, _pts, built) = match fill(bodies) {
        Ok(x) => x,
        Err(_) => return Ok(None),
    };
    let outcome = core::core(&built.ledger, rule);
    match outcome {
        Outcome::Empty => Ok(Some(Body::default())),
        Outcome::Refused(Refusal::ZeroThickness(c)) => Err(BoolError::ZeroThickness {
            at: cell_point(&built, c.0),
        }),
        Outcome::Refused(r) => panic!("the decide-once core refused {r:?}: its ledger is not consistent"),
        Outcome::Ok(out) => Ok(write(bodies, &fs, &cls, &built, &out)),
    }
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

/// The body the core's answer describes, written fresh (`None` if it cannot be walked into
/// loops: left to the old path).
fn write(bodies: &[&Body], fs: &[In], cls: &Classes, built: &Built, out: &core::Body) -> Option<Body> {
    let l = &built.ledger;
    let mut body = Body::default();
    // vertices, by vertex cell
    let mut vid: HashMap<u32, VertexId> = HashMap::new();
    // the input vertices by covector, for their names
    let mut in_vname: HashMap<u32, Name> = HashMap::new();
    for (k, b) in bodies.iter().enumerate() {
        let _ = k;
        for v in b.vertex_ids() {
            let p = b.vertex(v).point;
            let _ = p;
        }
    }
    let _ = &mut in_vname;
    let face_name = |g: usize| -> Name { bodies[fs[g].body].face(fs[g].face).name };
    // faces: names and surfaces
    let mut per_origin: HashMap<u32, usize> = HashMap::new();
    for f in &out.faces {
        *per_origin.entry(f.origin.0).or_default() += 1;
    }
    let mut seen_origin: HashMap<u32, u64> = HashMap::new();
    let mut fnames = Vec::new();
    for f in &out.faces {
        let base = face_name(f.origin.0 as usize);
        let name = if per_origin[&f.origin.0] == 1 {
            base
        } else {
            let k = seen_origin.entry(f.origin.0).or_default();
            *k += 1;
            base.piece(Name::NONE.with(*k))
        };
        fnames.push(name);
    }
    // which face each result facet is in
    let mut face_of: HashMap<u32, usize> = HashMap::new();
    for (i, f) in out.faces.iter().enumerate() {
        for c in &f.facets {
            face_of.insert(c.0, i);
        }
    }
    // edges: each run's cells chained from one end to the other
    let mut edge_runs: Vec<Vec<(u32, u32, u32)>> = Vec::new(); // (cell, from, to) in run order
    let mut run_of: HashMap<u32, usize> = HashMap::new();
    for (ri, e) in out.edges.iter().enumerate() {
        let mut count: HashMap<u32, u32> = HashMap::new();
        for c in &e.cells {
            let (u, v) = built.ends[&c.0];
            *count.entry(u).or_default() += 1;
            *count.entry(v).or_default() += 1;
            run_of.insert(c.0, ri);
        }
        let mut endv: Vec<u32> = count.iter().filter(|(_, n)| **n == 1).map(|(v, _)| *v).collect();
        endv.sort();
        if endv.len() != 2 {
            return None;
        }
        let mut at = endv[0];
        let mut left: Vec<u32> = e.cells.iter().map(|c| c.0).collect();
        let mut chain = Vec::new();
        while !left.is_empty() {
            let k = left.iter().position(|c| {
                let (u, v) = built.ends[c];
                u == at || v == at
            })?;
            let c = left.swap_remove(k);
            let (u, v) = built.ends[&c];
            let next = if u == at { v } else { u };
            chain.push((c, at, next));
            at = next;
        }
        edge_runs.push(chain);
    }
    for &v in &out.verts {
        let p = built.point[&v.0].to_f64();
        let name = Name::NONE.with_str("decide-once vertex").with(v.0 as u64);
        vid.insert(v.0, body.add_vertex(Point3::new(p[0], p[1], p[2]), name));
    }
    // per face: its boundary uses (run, forward), from its facets' directed edges
    let mut uses: Vec<Vec<(usize, bool)>> = vec![Vec::new(); out.faces.len()];
    for (fi, f) in out.faces.iter().enumerate() {
        let mut seen: HashMap<usize, ()> = HashMap::new();
        for c in &f.facets {
            let fc = &built.facet[&c.0];
            let mut poly = fc.poly.clone();
            if f.orient == Sgn::Neg {
                poly.reverse();
            }
            for k in 0..poly.len() {
                let (u, v) = (poly[k], poly[(k + 1) % poly.len()]);
                let cv = core::zero_idx(&l.cells[u as usize]);
                let _ = cv;
                let e = edge_cell(built, u, v)?;
                let Some(&ri) = run_of.get(&e) else { continue };
                // a run of this face's boundary (not one inside it)
                if !out.edges[ri].faces.iter().any(|h| face_of[&h.0] == fi) {
                    continue;
                }
                if seen.insert(ri, ()).is_some() {
                    continue;
                }
                let (_, from, to) = *edge_runs[ri].iter().find(|x| x.0 == e)?;
                let forward = from == u && to == v;
                if !forward && !(from == v && to == u) {
                    return None;
                }
                uses[fi].push((ri, forward));
            }
        }
    }
    // shells: faces joined across edges
    let mut uf = Uf::new(out.faces.len());
    for e in &out.edges {
        if e.faces.len() == 2 {
            uf.union(face_of[&e.faces[0].0], face_of[&e.faces[1].0]);
        }
    }
    let mut shell_of: HashMap<usize, forge_topo::ShellId> = HashMap::new();
    // the edges, made when their first face's loops are
    let mut eid: Vec<Option<EdgeId>> = vec![None; out.edges.len()];
    let mut edge_names: HashMap<Name, u64> = HashMap::new();
    for (fi, f) in out.faces.iter().enumerate() {
        let root = uf.find(fi);
        let shell = *shell_of.entry(root).or_insert_with(|| body.add_shell());
        let g = f.origin.0 as usize;
        let class_plane = &cls.planes[f.plane as usize];
        let own = &fs[g];
        let src = if own.plane.same_set(class_plane) {
            g
        } else {
            cls.rep[f.plane as usize]
        };
        let surface = bodies[fs[src].body].face(fs[src].face).surface.clone();
        let along = fs[src].plane.n.dot(&class_plane.n).signum() > 0;
        let same_sense = along == (f.orient == Sgn::Pos);
        let face = body.add_face(shell, surface, same_sense, fnames[fi]);
        // walk the uses into loops
        let mut left = uses[fi].clone();
        let mut loops: Vec<Vec<(usize, bool)>> = Vec::new();
        while let Some(first) = left.first().copied() {
            left.remove(0);
            let start = run_start(&edge_runs[first.0], first.1);
            let mut lp = vec![first];
            let mut at = run_end(&edge_runs[first.0], first.1);
            let mut guard = 0;
            while at != start {
                let cands: Vec<usize> = (0..left.len())
                    .filter(|&k| run_start(&edge_runs[left[k].0], left[k].1) == at)
                    .collect();
                let k = match cands.len() {
                    0 => return None,
                    1 => cands[0],
                    _ => pick_turn(built, &edge_runs, &lp, &left, &cands, cls, f)?,
                };
                let u = left.remove(k);
                at = run_end(&edge_runs[u.0], u.1);
                lp.push(u);
                guard += 1;
                if guard > 1_000_000 {
                    return None;
                }
            }
            loops.push(lp);
        }
        // the outer loop first: the one with the largest area about the outward normal
        let area = |lp: &Vec<(usize, bool)>| loop_area(built, &edge_runs, lp, cls, f);
        let mut best = 0;
        for k in 1..loops.len() {
            if area(&loops[k]) > area(&loops[best]) {
                best = k;
            }
        }
        loops.swap(0, best);
        for lp in loops {
            let mut ce = Vec::new();
            for (ri, fwd) in lp {
                let e = match eid[ri] {
                    Some(e) => e,
                    None => {
                        let ch = &edge_runs[ri];
                        let (s, t) = if fwd {
                            (ch[0].1, ch[ch.len() - 1].2)
                        } else {
                            (ch[ch.len() - 1].2, ch[0].1)
                        };
                        let (vs, vt) = (*vid.get(&s)?, *vid.get(&t)?);
                        let (ps, pt) = (body.vertex(vs).point, body.vertex(vt).point);
                        let line = Line::through(ps, pt)?;
                        let names: Vec<Name> = out.edges[ri]
                            .faces
                            .iter()
                            .map(|h| fnames[face_of[&h.0]])
                            .collect();
                        let base = Name::NONE.with_str("decide-once edge").with_set(&names);
                        let k = edge_names.entry(base).or_insert(0u64);
                        *k += 1;
                        let name = if *k == 1 {
                            base
                        } else {
                            base.piece(Name::NONE.with(*k))
                        };
                        let e = body.add_edge(
                            Curve::Line(line),
                            Interval::new(0.0, ps.distance(pt)),
                            vs,
                            vt,
                            name,
                        );
                        eid[ri] = Some(e);
                        // the edge runs the way this first face walks it
                        let _ = fwd;
                        e
                    }
                };
                let ed = body.edge(e);
                let ch = &edge_runs[ri];
                let s = if fwd { ch[0].1 } else { ch[ch.len() - 1].2 };
                let forward = vid.get(&s) == Some(&ed.start);
                ce.push((e, forward));
            }
            body.add_loop(face, &ce);
        }
    }
    for b in bodies {
        body.joins.extend(&b.joins);
    }
    Some(body)
}

fn run_start(ch: &[(u32, u32, u32)], fwd: bool) -> u32 {
    if fwd { ch[0].1 } else { ch[ch.len() - 1].2 }
}

fn run_end(ch: &[(u32, u32, u32)], fwd: bool) -> u32 {
    if fwd { ch[ch.len() - 1].2 } else { ch[0].1 }
}

/// The edge cell between two vertex cells of a facet.
fn edge_cell(b: &Built, u: u32, v: u32) -> Option<u32> {
    let cu = &b.ledger.cells[u as usize];
    let cv = &b.ledger.cells[v as usize];
    let j = join(&[cu, cv]);
    b.ledger.cells.iter().position(|c| *c == j).map(|i| i as u32)
}

/// Projection axes of a face's plane, oriented so (s, t) is counter-clockwise about its outward
/// normal.
fn axes(cls: &Classes, f: &core::OutFace) -> (usize, usize) {
    let n = plane_f(&cls.planes[f.plane as usize]).n;
    let a = (0..3)
        .max_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()).then(j.cmp(&i)))
        .unwrap();
    let (u, v) = ((a + 1) % 3, (a + 2) % 3);
    let out_pos = (n[a] > 0.0) == (f.orient == Sgn::Pos);
    if out_pos { (u, v) } else { (v, u) }
}

fn xy(b: &Built, v: u32, ax: (usize, usize)) -> (f64, f64) {
    let p = b.point[&v].to_f64();
    (p[ax.0], p[ax.1])
}

/// Twice the signed area of a loop in the face's projection, counter-clockwise positive.
fn loop_area(
    b: &Built,
    runs: &[Vec<(u32, u32, u32)>],
    lp: &[(usize, bool)],
    cls: &Classes,
    f: &core::OutFace,
) -> f64 {
    let ax = axes(cls, f);
    let mut s = 0.0;
    for &(ri, fwd) in lp {
        for &(_, u, v) in &runs[ri] {
            let (u, v) = if fwd { (u, v) } else { (v, u) };
            let (p, q) = (xy(b, u, ax), xy(b, v, ax));
            s += p.0 * q.1 - q.0 * p.1;
        }
    }
    s
}

/// At a vertex where a face's boundary passes twice, the next use: the first clockwise from the
/// way back along the arriving one.
fn pick_turn(
    b: &Built,
    runs: &[Vec<(u32, u32, u32)>],
    lp: &[(usize, bool)],
    left: &[(usize, bool)],
    cands: &[usize],
    cls: &Classes,
    f: &core::OutFace,
) -> Option<usize> {
    let ax = axes(cls, f);
    let &(ri, fwd) = lp.last()?;
    let ch = &runs[ri];
    // the arriving segment's last piece
    let (u, v) = if fwd {
        (ch[ch.len() - 1].1, ch[ch.len() - 1].2)
    } else {
        (ch[0].2, ch[0].1)
    };
    let (pv, pu) = (xy(b, v, ax), xy(b, u, ax));
    let back = forge_math::dmath::atan2(pu.1 - pv.1, pu.0 - pv.0);
    let mut best = None;
    let mut best_turn = f64::INFINITY;
    for &k in cands {
        let (rj, fj) = left[k];
        let cj = &runs[rj];
        let w = if fj { cj[0].2 } else { cj[cj.len() - 1].1 };
        let pw = xy(b, w, ax);
        let out = forge_math::dmath::atan2(pw.1 - pv.1, pw.0 - pv.0);
        // clockwise from `back`: back − out in (0, 2π]
        let mut turn = back - out;
        while turn <= 0.0 {
            turn += std::f64::consts::TAU;
        }
        if turn < best_turn {
            best_turn = turn;
            best = Some(k);
        }
    }
    let _ = orient2;
    best
}
