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
use forge_bool_core::row::{row_get, row_set};
use forge_bool_core::{
    self as core, BodyId, CellId, FaceId as CFace, Ledger, Outcome, Refusal, Row, Rule, Sgn,
};
use forge_geom::{Curve, Line, Surface};
use forge_math::exact::{Dy, DyPlane, DyV3, HPoint, meet3};
use forge_math::{Interval, Point3};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::cell::RefCell;
use std::hash::{BuildHasherDefault, Hasher};

/// A fast, deterministic hasher for the bridge's keyed tables (covectors, ids).
#[derive(Default)]
pub(crate) struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u8(&mut self, b: u8) {
        self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u32(&mut self, b: u32) {
        self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u64(&mut self, b: u64) {
        self.0 = (self.0.rotate_left(5) ^ b).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_usize(&mut self, b: usize) {
        self.write_u64(b as u64);
    }
}

type FxMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<Fx>>;
/// Every keyed table of the bridge: Fx-hashed, so fast and the same from run to run.
type HashMap<K, V> = FxMap<K, V>;

/// Why the bridge leaves a boolean to the old path.
#[derive(Debug, Clone, PartialEq)]
pub enum Skip {
    /// A face is not a plane or an edge is not a line.
    NotFlat,
    /// §6.1 says ambiguous: a class past the spread cap, or a vertex its snapped planes do not
    /// settle.
    Ambiguous(&'static str),
    /// More classes than a packed covector holds (counted in `SKIP_WIDE`).
    Wide,
}

/// A face of an operand as the bridge reads it.
struct In {
    body: usize,
    face: FaceId,
    /// its own plane, exactly (the kernel's frame)
    plane: DyPlane,
    /// that plane in f64 (the frame's normal exactly, the offset rounded), for screening
    pf: PlaneF,
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
#[inline(never)]
fn inputs(bodies: &[&Body]) -> Result<Vec<In>, Skip> {
    if !bodies.iter().all(|b| flat(b)) {
        return Err(Skip::NotFlat);
    }
    let mut out = Vec::with_capacity(bodies.iter().map(|b| b.face_ids().count()).sum());
    let mut vs = Vec::new();
    for (k, b) in bodies.iter().enumerate() {
        for f in b.face_ids() {
            let face = b.face(f);
            let Surface::Plane(pl) = &face.surface else {
                return Err(Skip::NotFlat);
            };
            let o = pl.frame.origin;
            let n = pl.frame.z().get();
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            let mut loops = Vec::with_capacity(face.loops.len());
            let mut lboxes = Vec::with_capacity(face.loops.len());
            for &lp in face.loops.iter() {
                vs.clear();
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
                loops.push(vs.clone());
                lboxes.push((llo, lhi));
            }
            if loops.is_empty() {
                continue;
            }
            let plane = DyPlane::from_frame([o.x, o.y, o.z], [n.x, n.y, n.z]);
            out.push(In {
                body: k,
                face: f,
                // (the frame's normal is the exact one's f64)
                pf: PlaneF {
                    n: [n.x, n.y, n.z],
                    c: plane.c.to_f64(),
                },
                plane,
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

/// `region_extent` screened in f64: `Some(None)` when the grown boxes plainly miss, `Some(Some(
/// (lo, hi)))` (the extent, each bound within an ulp of the exact one) when they plainly overlap,
/// None when the exact test decides (a gap or overlap within 1e-12 of the coordinates' size; the
/// f64 bounds are each one rounding from the exact ones).
fn region_f(a: &In, b: &In, l: f64) -> Option<Option<([f64; 3], [f64; 3])>> {
    let (mut lo, mut hi) = ([0.0; 3], [0.0; 3]);
    for k in 0..3 {
        let (alo, ahi) = (a.lo[k] - l, a.hi[k] + l);
        let (blo, bhi) = (b.lo[k] - l, b.hi[k] + l);
        let (rlo, rhi) = (alo.max(blo), ahi.min(bhi));
        if (rlo - rhi).abs() <= 1e-12 * (1.0 + rlo.abs() + rhi.abs()) {
            return None;
        }
        if rlo > rhi {
            return Some(None);
        }
        lo[k] = alo.min(blo);
        hi[k] = ahi.max(bhi);
    }
    Some(Some((lo, hi)))
}

/// `near` screened in f64 at the corners of `[lo, hi]` (each within an ulp of the exact extent's):
/// `Some` where every corner is plainly within, or one plainly past, `d` (the f64 value within
/// 1e-12 of the terms' size of the exact one: the planes' normals are f64 exactly, their offsets
/// one rounding off, and the corners an ulp; the margin covers a few roundings a hundredfold), else
/// None.
fn near_f(s: &PlaneF, t: &PlaneF, lo: [f64; 3], hi: [f64; 3], d: f64) -> Option<bool> {
    let dot = |u: [f64; 3], v: [f64; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let (ns2, nt2, nts) = (dot(s.n, s.n), dot(t.n, t.n), dot(t.n, s.n));
    // (the abs-dot, not |nts|: nts's rounding is relative to it, and it cancels for normals near
    // perpendicular)
    let nts_abs = s.n[0].abs() * t.n[0].abs() + s.n[1].abs() * t.n[1].abs() + s.n[2].abs() * t.n[2].abs();
    let r = d * nt2.sqrt() * ns2;
    let mut all = true;
    for x in [lo[0], hi[0]] {
        for y in [lo[1], hi[1]] {
            for z in [lo[2], hi[2]] {
                let c = [x, y, z];
                let ca = [x.abs(), y.abs(), z.abs()];
                let na = |n: [f64; 3]| [n[0].abs(), n[1].abs(), n[2].abs()];
                let sc = dot(s.n, c) - s.c;
                let tc = dot(t.n, c) - t.c;
                let e = tc * ns2 - sc * nts;
                let m = (dot(na(t.n), ca) + t.c.abs()) * ns2 + (dot(na(s.n), ca) + s.c.abs()) * nts_abs;
                let err = 1e-12 * m;
                if e.abs() - err > r * (1.0 + 1e-12) {
                    return Some(false);
                }
                if e.abs() + err >= r * (1.0 - 1e-12) {
                    all = false;
                }
            }
        }
    }
    all.then_some(true)
}

/// Are `a`'s and `b`'s planes tolerance-equal within `k`·LINEAR over their grown boxes: `None`
/// when the boxes' region is empty, else whether each is `near` the other. f64 first, exact where
/// the screens cannot tell.
fn tol_eq(a: &In, b: &In, k: f64) -> Option<bool> {
    let l = lin();
    let d = l * k;
    if let Some(r) = region_f(a, b, l) {
        let (lo, hi) = r?;
        if let (Some(x), Some(y)) = (near_f(&a.pf, &b.pf, lo, hi, d), near_f(&b.pf, &a.pf, lo, hi, d)) {
            return Some(x && y);
        }
    }
    let cs = region_extent(a, b, l)?;
    let dd = Dy::from_f64(d);
    Some(near(&a.plane, &b.plane, &cs, &dd) && near(&b.plane, &a.plane, &cs, &dd))
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
    let (na, ca) = (a.pf.n, a.pf.c);
    let (nb, cb) = (b.pf.n, b.pf.c);
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
#[inline(never)]
fn snap(fs: &[In]) -> Result<(Vec<DyPlane>, Vec<usize>), Skip> {
    let l = lin();
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
            // (grown boxes plainly apart in y or z: an empty region)
            let gap = |k: usize| {
                let (lo, hi) = (fs[a].lo[k].max(fs[b].lo[k]) - l, fs[a].hi[k].min(fs[b].hi[k]) + l);
                lo - hi > 1e-12 * (1.0 + lo.abs() + hi.abs())
            };
            if gap(1) || gap(2) {
                continue;
            }
            if plainly_apart(&fs[a], &fs[b], 1.0) {
                continue;
            }
            if tol_eq(&fs[a], &fs[b], 1.0) == Some(true) {
                pairs.push((a, b));
            }
        }
    }
    let mut uf = Uf::new(n);
    for &(a, b) in &pairs {
        uf.union(a, b);
    }
    // the spread cap: every pair of a class whose region is not empty within 2·LINEAR
    let mut members: HashMap<usize, Vec<usize>> = HashMap::default();
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
                if tol_eq(&fs[a], &fs[b], 2.0) == Some(false) {
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
        // (a face its own representative keeps its plane)
        planes.push(if r != i && f.plane.n.dot(&rp.n).signum() < 0 {
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
#[inline(never)]
fn vertices(
    bodies: &[&Body],
    fs: &[In],
    planes: &[DyPlane],
    reps: &[usize],
    reg: &Region,
) -> Result<Vec<Pts>, Skip> {
    let need = &reg.near;
    let g = 4.0 * lin();
    let in_region = |p: Point3| match reg.bx {
        None => true,
        Some((lo, hi)) => {
            let q = [p.x, p.y, p.z];
            (0..3).all(|k| lo[k] - g <= q[k] && q[k] <= hi[k] + g)
        }
    };
    // the snapped planes in f64: a face its own representative keeps its own shadow
    let spf: Vec<PlaneF> = planes
        .iter()
        .enumerate()
        .map(|(i, p)| if reps[i] == i { fs[i].pf } else { plane_f(p) })
        .collect();
    // per body: the faces round each vertex, as a table by vertex index (a counting sort, faces
    // in arena order within a vertex)
    let mut tabs: Vec<(Vec<u32>, Vec<u32>)> = Vec::with_capacity(bodies.len());
    for k in 0..bodies.len() {
        let mut count: Vec<u32> = Vec::new();
        for f in fs.iter().filter(|f| f.body == k) {
            for lp in &f.loops {
                for &v in lp {
                    if count.len() <= v.index() + 1 {
                        count.resize(v.index() + 2, 0);
                    }
                    count[v.index() + 1] += 1;
                }
            }
        }
        for x in 1..count.len() {
            count[x] += count[x - 1];
        }
        let mut at = count.clone();
        let mut faces = vec![u32::MAX; *count.last().unwrap_or(&0) as usize];
        for (i, f) in fs.iter().enumerate().filter(|(_, f)| f.body == k) {
            for lp in &f.loops {
                for &v in lp {
                    // (a face once per vertex)
                    let (s0, a) = (count[v.index()] as usize, at[v.index()] as usize);
                    if a == s0 || faces[a - 1] != i as u32 {
                        faces[a] = i as u32;
                        at[v.index()] += 1;
                    }
                }
            }
        }
        // compact out the slots a repeat left
        let nv = count.len().saturating_sub(1);
        let mut start = vec![0u32; nv + 1];
        let mut out = Vec::with_capacity(faces.len());
        for v in 0..nv {
            out.extend_from_slice(&faces[count[v] as usize..at[v] as usize]);
            start[v + 1] = out.len() as u32;
        }
        tabs.push((start, out));
    }
    let mut out = Vec::new();
    for (k, (start, faces)) in tabs.into_iter().enumerate() {
        let nv = start.len().saturating_sub(1);
        let pts = Pts {
            exact: RefCell::new(vec![None; nv]),
            f: RefCell::new(vec![None; nv]),
            spf: spf.clone(),
            start,
            faces,
        };
        // the near faces' vertices in the region now, each checked (§6.1's ambiguity is decided
        // there); the others when a parity test or a ray first needs them
        for v in 0..nv as u32 {
            let r = pts.round(VertexId(v));
            if r.is_empty()
                || !r.iter().any(|&i| need[i as usize])
                || !in_region(bodies[k].vertex(VertexId(v)).point)
            {
                continue;
            }
            let ps: Vec<&DyPlane> = r.iter().map(|&i| &planes[i as usize]).collect();
            let p = vertex_at(&ps).ok_or(Skip::Ambiguous(
                "a vertex is not one point of the planes round it",
            ))?;
            pts.f.borrow_mut()[v as usize] = Some(p.to_f64());
            pts.exact.borrow_mut()[v as usize] = Some(p);
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
#[inline(never)]
fn classes(snapped: &[DyPlane], reps: &[usize], near: &[bool]) -> Classes {
    let mut planes: Vec<DyPlane> = Vec::new();
    let mut rep = Vec::new();
    let mut of_face = Vec::with_capacity(snapped.len());
    // by an f64 key first, so the exact comparison runs only against likely equals: one point set
    // gives keys at most one step apart in each component, so its neighbours are looked at too
    let mut by_key: HashMap<(i64, i64, i64, i64), Vec<u32>> = HashMap::default();
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
#[inline(never)]
fn region(bodies: &[&Body], fs: &[In], snapped: &[DyPlane], reps: &[usize], rule: &Rule) -> Region {
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
            let pf = &fs[reps[i]].pf;
            let nf = pf.n;
            for a in 0..3 {
                if nf[(a + 1) % 3] == 0.0 && nf[(a + 2) % 3] == 0.0 {
                    for x in [lo[a], hi[a]] {
                        // (f64 screen: the offset of the rep's plane, rounded once from the exact
                        // one, is within an ulp of it; a wall plainly off it is not that plane)
                        let off = (pf.c - nf[a] * x).abs();
                        if off > 1e-9 * (pf.c.abs() + (nf[a] * x).abs()) {
                            continue;
                        }
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
            .any(|(i, f)| !near[i] && reps[i] != i && !snapped[i].same_set(&f.plane))
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
#[inline(never)]
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
    /// its exact point, worked out when first needed (a piece's vertex, or a sign the f64 point
    /// cannot settle)
    p: RefCell<Option<Box<HPoint>>>,
    /// the two planes (besides the section's) it is the meet of
    meet: (Pl, Pl),
    /// its f64 point and a bound on each coordinate's distance from the exact one
    f: [f64; 3],
    err: f64,
    /// where its covector starts in `Plane2::covs`, or NONE (a vertex on a wall)
    cov: u32,
}

/// A plane as one number (classes below walls), for keys.
fn pl_code(p: Pl) -> u32 {
    match p {
        Pl::Class(k) => k,
        Pl::Wall(w) => u32::MAX - 8 + w as u32,
    }
}

/// A plane a polygon's edge lies on: a class, or a wall of the working box.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Pl {
    Class(u32),
    Wall(u8),
}

/// The arrangement on one class plane: vertices, and convex cells as (vertex, plane of the edge
/// to the next vertex) lists, flat: cell `k` is `cv[cs[k]..cs[k + 1]]`.
struct Plane2 {
    vx: Vec<Vx>,
    /// the class vertices' covectors
    covs: Vec<Row>,
    cs: Vec<u32>,
    cv: Vec<(u32, Pl)>,
    /// the section's plane and its box walls, to work out exact points
    pc: DyPlane,
    walls: Vec<DyPlane>,
}

impl Plane2 {
    fn exact(&self, cls: &Classes, i: usize) -> HPoint {
        exact_vx(&self.vx[i], &self.pc, &self.walls, cls)
    }
    fn cov(&self, i: u32) -> Option<Row> {
        let o = self.vx[i as usize].cov;
        (o != NONE).then(|| self.covs[o as usize])
    }
    fn ncells(&self) -> usize {
        self.cs.len().saturating_sub(1)
    }
    fn cell(&self, k: usize) -> &[(u32, Pl)] {
        &self.cv[self.cs[k] as usize..self.cs[k + 1] as usize]
    }
}

/// A vertex's exact point (memoised).
fn exact_vx(x: &Vx, pc: &DyPlane, walls: &[DyPlane], cls: &Classes) -> HPoint {
    if let Some(h) = x.p.borrow().as_ref() {
        return (**h).clone();
    }
    let of = |p: Pl| match p {
        Pl::Class(k) => &cls.planes[k as usize],
        Pl::Wall(w) => &walls[w as usize],
    };
    let h = meet3(pc, of(x.meet.0), of(x.meet.1)).expect("a vertex is a meet of three planes");
    *x.p.borrow_mut() = Some(Box::new(h.clone()));
    h
}

/// Rounding slack of the f64 evaluations below (generous: each is a handful of operations).
const GAMMA: f64 = 64.0 * f64::EPSILON;

/// The meet of three planes in f64, with a bound on each coordinate's error; None when the system
/// is too near singular for the bound to hold (the exact meet decides then).
fn meet_f(a: &PlaneF, b: &PlaneF, c: &PlaneF) -> Option<([f64; 3], f64)> {
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let cabs = |u: [f64; 3], v: [f64; 3]| {
        [
            (u[1] * v[2]).abs() + (u[2] * v[1]).abs(),
            (u[2] * v[0]).abs() + (u[0] * v[2]).abs(),
            (u[0] * v[1]).abs() + (u[1] * v[0]).abs(),
        ]
    };
    let dot = |u: [f64; 3], v: [f64; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let dabs = |u: [f64; 3], v: [f64; 3]| u[0].abs() * v[0] + u[1].abs() * v[1] + u[2].abs() * v[2];
    let (bc, ca, ab) = (cross(b.n, c.n), cross(c.n, a.n), cross(a.n, b.n));
    let (bca, caa, aba) = (cabs(b.n, c.n), cabs(c.n, a.n), cabs(a.n, b.n));
    let det = dot(a.n, bc);
    let edet = GAMMA * dabs(a.n, bca);
    if det.abs() <= 4.0 * edet {
        return None;
    }
    let mut p = [0.0; 3];
    let mut err: f64 = 0.0;
    for k in 0..3 {
        let num = a.c * bc[k] + b.c * ca[k] + c.c * ab[k];
        let mnum = a.c.abs() * bca[k] + b.c.abs() * caa[k] + c.c.abs() * aba[k];
        p[k] = num / det;
        let e = (GAMMA * mnum + p[k].abs() * edet) / (det.abs() - edet) + p[k].abs() * GAMMA;
        err = err.max(e);
    }
    p.iter().all(|x| x.is_finite()).then_some((p, err))
}

/// A plane's sign at a point known to within `err` per coordinate in f64; `exact` when the f64
/// value cannot settle it.
fn side_err(plane: &DyPlane, pff: &PlaneF, pf: &[f64; 3], err: f64, exact: impl FnOnce() -> HPoint) -> Sgn {
    let e = pff.n[0] * pf[0] + pff.n[1] * pf[1] + pff.n[2] * pf[2] - pff.c;
    let mag = pff.n[0].abs() * pf[0].abs()
        + pff.n[1].abs() * pf[1].abs()
        + pff.n[2].abs() * pf[2].abs()
        + pff.c.abs();
    let bound = GAMMA * mag + (pff.n[0].abs() + pff.n[1].abs() + pff.n[2].abs()) * err;
    let s = if e.abs() > bound {
        if e > 0.0 { 1 } else { -1 }
    } else {
        plane.side(&exact())
    };
    match s {
        1 => Sgn::Pos,
        -1 => Sgn::Neg,
        _ => Sgn::Zero,
    }
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
#[derive(Clone, Copy)]
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

fn covector(cls: &Classes, pf: &[PlaneF], h: &HPoint, zero: &[u32]) -> Row {
    let p = h.to_f64();
    let mut r = Row::default();
    for (k, (pl, pfk)) in cls.planes.iter().zip(pf).enumerate() {
        if !zero.contains(&(k as u32)) {
            r = row_set(r, k, side_at(pl, &p, pfk, h));
        }
    }
    r
}

/// A row as a map key.
fn rk(r: Row) -> (u64, u64) {
    (r.pos, r.neg)
}

/// The covector of the cell spanned by two: on each class the first's sign, or the second's where
/// the first is zero.
fn row_join(a: Row, b: Row) -> Row {
    let z = !(a.pos | a.neg);
    Row {
        pos: a.pos | (b.pos & z),
        neg: a.neg | (b.neg & z),
    }
}

/// The working state of `arrange`: the vertices so far, found by the planes they meet and (class
/// vertices) by covector.
struct Arr<'a> {
    cls: &'a Classes,
    pf: &'a [PlaneF],
    c: u32,
    pc: &'a DyPlane,
    walls: [DyPlane; 6],
    wf: [PlaneF; 6],
    vx: Vec<Vx>,
    covs: Vec<Row>,
    by_pair: HashMap<u64, u32>,
    /// class vertices by covector
    by_cov: HashMap<(u64, u64), u32>,
}

impl Arr<'_> {
    fn plane(&self, p: Pl) -> &DyPlane {
        match p {
            Pl::Class(k) => &self.cls.planes[k as usize],
            Pl::Wall(w) => &self.walls[w as usize],
        }
    }
    fn planef(&self, p: Pl) -> &PlaneF {
        match p {
            Pl::Class(k) => &self.pf[k as usize],
            Pl::Wall(w) => &self.wf[w as usize],
        }
    }
    fn cov(&self, i: u32) -> Option<Row> {
        let o = self.vx[i as usize].cov;
        (o != NONE).then(|| self.covs[o as usize])
    }
    /// The vertex where planes `x` and `y` cross the section: an existing one with that pair, or
    /// (class vertices) that covector, else a new one.
    fn vertex(&mut self, x: Pl, y: Pl) -> u32 {
        let (cx, cy) = (pl_code(x), pl_code(y));
        let key = if cx <= cy {
            (cx as u64) << 32 | cy as u64
        } else {
            (cy as u64) << 32 | cx as u64
        };
        if let Some(&i) = self.by_pair.get(&key) {
            return i;
        }
        let mut x0 = Vx {
            p: RefCell::new(None),
            meet: (x, y),
            f: [0.0; 3],
            err: 0.0,
            cov: NONE,
        };
        match meet_f(&self.pf[self.c as usize], self.planef(x), self.planef(y)) {
            Some((f, e)) => {
                x0.f = f;
                x0.err = e;
            }
            None => {
                let h = meet3(self.pc, self.plane(x), self.plane(y))
                    .expect("a polygon's edge crosses the plane splitting it");
                x0.f = h.to_f64();
                x0.err = 0.0;
                x0.p = RefCell::new(Some(Box::new(h)));
            }
        }
        let id = match (x, y) {
            (Pl::Class(i), Pl::Class(j)) => {
                let nc = self.cls.planes.len();
                let mut r = Row::default();
                for k in 0..nc {
                    if !(k as u32 == self.c || k as u32 == i || k as u32 == j) {
                        let s = side_err(&self.cls.planes[k], &self.pf[k], &x0.f, x0.err, || {
                            exact_vx(&x0, self.pc, &self.walls, self.cls)
                        });
                        r = row_set(r, k, s);
                    }
                }
                match self.by_cov.get(&rk(r)) {
                    Some(&found) => found,
                    None => {
                        x0.cov = self.covs.len() as u32;
                        self.covs.push(r);
                        let id = self.vx.len() as u32;
                        self.vx.push(x0);
                        self.by_cov.insert(rk(r), id);
                        id
                    }
                }
            }
            _ => {
                self.vx.push(x0);
                self.vx.len() as u32 - 1
            }
        };
        self.by_pair.insert(key, id);
        id
    }
    /// The side of plane `p` at vertex `i`: -1, 0 or 1.
    fn side(&self, i: u32, p: Pl) -> i32 {
        let x = &self.vx[i as usize];
        // (a vertex is on the planes it is the meet of)
        if p == x.meet.0 || p == x.meet.1 {
            return 0;
        }
        if let (Pl::Class(k), Some(cv)) = (p, self.cov(i)) {
            return match row_get(cv, k as usize) {
                Sgn::Pos => 1,
                Sgn::Neg => -1,
                Sgn::Zero => 0,
            };
        }
        match side_err(self.plane(p), self.planef(p), &x.f, x.err, || {
            exact_vx(x, self.pc, &self.walls, self.cls)
        }) {
            Sgn::Pos => 1,
            Sgn::Neg => -1,
            Sgn::Zero => 0,
        }
    }
    /// Split `poly` by `p` into (part with side ≥ 0, part with side ≤ 0), appended to `pos` and
    /// `neg`; `s` is scratch. Unless `all`, `known` holds the sides of `p` at the vertices made
    /// before this pass (those made in it lie on `p`).
    #[allow(clippy::too_many_arguments)]
    fn split(
        &mut self,
        poly: &[(u32, Pl)],
        p: Pl,
        known: &[i8],
        all: bool,
        s: &mut Vec<i32>,
        pos: &mut Vec<(u32, Pl)>,
        neg: &mut Vec<(u32, Pl)>,
    ) {
        let n = poly.len();
        s.clear();
        for &(i, _) in poly {
            s.push(if all {
                self.side(i, p)
            } else if (i as usize) < known.len() {
                known[i as usize] as i32
            } else {
                0
            });
        }
        let (p0, n0) = (pos.len(), neg.len());
        // which pushed vertices lie on `p` (a crossing's new vertex does, by construction)
        let (mut zp, mut zn) = (0u64, 0u64);
        let (mut kp, mut kn) = (0u32, 0u32);
        let mut zpv: Vec<bool> = Vec::new();
        let mut znv: Vec<bool> = Vec::new();
        let big = n > 30;
        let mark = |z: &mut u64, zv: &mut Vec<bool>, k: &mut u32, on: bool| {
            if big {
                zv.push(on);
            } else if on {
                *z |= 1 << *k;
            }
            *k += 1;
        };
        for k in 0..n {
            let (vi, line) = poly[k];
            let (fa, fb) = (s[k], s[(k + 1) % n]);
            // the edge from vertex k to k+1 lies on `line`; a part's edge out of a vertex on the
            // splitting plane runs along that plane when the part turns there
            if fa >= 0 {
                pos.push((vi, line));
                mark(&mut zp, &mut zpv, &mut kp, fa == 0);
            }
            if fa <= 0 {
                neg.push((vi, line));
                mark(&mut zn, &mut znv, &mut kn, fa == 0);
            }
            if (fa > 0 && fb < 0) || (fa < 0 && fb > 0) {
                let w = self.vertex(line, p);
                pos.push((w, line));
                neg.push((w, line));
                mark(&mut zp, &mut zpv, &mut kp, true);
                mark(&mut zn, &mut znv, &mut kn, true);
            }
        }
        // the edge plane of each part's vertex is the plane the part runs on to its next vertex:
        // where a part's two consecutive vertices both lie on `p`, that edge is on `p`
        for (part, from, z, zv) in [(pos, p0, zp, &zpv), (neg, n0, zn, &znv)] {
            let m = part.len() - from;
            let on = |k: usize| if big { zv[k] } else { z >> k & 1 == 1 };
            for k in 0..m {
                if on(k) && on((k + 1) % m) {
                    part[from + k].1 = p;
                }
            }
        }
    }
}

/// The cells of class plane `c` within the box `[lo, hi]`: its section split by every class
/// plane crossing it.
#[inline(never)]
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
    let wf = [0, 1, 2, 3, 4, 5].map(|k| plane_f(&walls[k]));
    let nc = cls.planes.len();
    let mut ar = Arr {
        cls,
        pf,
        c,
        pc,
        walls,
        wf,
        vx: Vec::new(),
        covs: Vec::new(),
        by_pair: HashMap::default(),
        by_cov: HashMap::default(),
    };
    let done = |ar: Arr, cs: Vec<u32>, cv: Vec<(u32, Pl)>| Plane2 {
        vx: ar.vx,
        covs: ar.covs,
        cs,
        cv,
        pc: pc.clone(),
        walls: ar.walls.to_vec(),
    };
    // the quad on the u and v walls, counter-clockwise in (u, v)
    let q = [
        ar.vertex(Pl::Wall(0), Pl::Wall(2)),
        ar.vertex(Pl::Wall(1), Pl::Wall(2)),
        ar.vertex(Pl::Wall(1), Pl::Wall(3)),
        ar.vertex(Pl::Wall(0), Pl::Wall(3)),
    ];
    let mut poly: Vec<(u32, Pl)> = vec![
        (q[0], Pl::Wall(2)),
        (q[1], Pl::Wall(1)),
        (q[2], Pl::Wall(3)),
        (q[3], Pl::Wall(0)),
    ];
    let mut s = Vec::new();
    for w in [Pl::Wall(4), Pl::Wall(5)] {
        let (mut p, mut n) = (Vec::new(), Vec::new());
        ar.split(&poly, w, &[], true, &mut s, &mut p, &mut n);
        poly = if w == Pl::Wall(4) { p } else { n };
        if poly.len() < 3 {
            return done(ar, Vec::new(), Vec::new());
        }
    }
    // the section clipped to the box: every cell lies in it, so a plane strictly to one side of
    // all its vertices is to that side of every cell
    let hull: Vec<u32> = poly.iter().map(|&(i, _)| i).collect();
    // the cells, flat, double-buffered
    let mut cs: Vec<u32> = vec![0, poly.len() as u32];
    let mut cv: Vec<(u32, Pl)> = poly;
    let (mut cs2, mut cv2): (Vec<u32>, Vec<(u32, Pl)>) = (Vec::new(), Vec::new());
    let (mut pp, mut nn) = (Vec::new(), Vec::new());
    let mut sj: Vec<i8> = Vec::new();
    for j in 0..nc as u32 {
        if j == c {
            continue;
        }
        let pj = Pl::Class(j);
        // (a plane missing the clipped section leaves every cell whole)
        let mut all_pos = true;
        let mut all_neg = true;
        for &i in &hull {
            let x = ar.side(i, pj);
            all_pos &= x > 0;
            all_neg &= x < 0;
        }
        if all_pos || all_neg {
            continue;
        }
        // the plane's side at every vertex so far, once
        sj.clear();
        for i in 0..ar.vx.len() as u32 {
            sj.push(ar.side(i, pj) as i8);
        }
        cs2.clear();
        cv2.clear();
        cs2.push(0);
        for k in 0..cs.len() - 1 {
            let cl = &cv[cs[k] as usize..cs[k + 1] as usize];
            // (a cell the plane does not cross stays whole)
            let (mut cp, mut cn) = (true, true);
            for &(i, _) in cl {
                let x = sj[i as usize];
                cp &= x > 0;
                cn &= x < 0;
            }
            if cp || cn {
                cv2.extend_from_slice(cl);
                cs2.push(cv2.len() as u32);
                continue;
            }
            pp.clear();
            nn.clear();
            ar.split(cl, pj, &sj, false, &mut s, &mut pp, &mut nn);
            if pp.len() >= 3 {
                cv2.extend_from_slice(&pp);
                cs2.push(cv2.len() as u32);
            }
            if nn.len() >= 3 {
                cv2.extend_from_slice(&nn);
                cs2.push(cv2.len() as u32);
            }
        }
        std::mem::swap(&mut cs, &mut cs2);
        std::mem::swap(&mut cv, &mut cv2);
    }
    done(ar, cs, cv)
}

/// Is the exact point inside face `f`'s polygon (its loops, with exact vertices), by the parity
/// of the perturbed ray toward +s in the plane's projection `(s, t)`.
#[inline(never)]
fn in_face(q: &HPoint, f: &In, pts: &Pts, planes: &[DyPlane], s: usize, t: usize) -> bool {
    in_face_q(q.to_f64(), &|| q.clone(), f, pts, planes, s, t)
}

/// `in_face` at a point given by its f64 shadow `qf` (its `s` and `t` within a few ulps of the
/// exact point's) and the exact point on demand.
fn in_face_q(
    qf: [f64; 3],
    q: &dyn Fn() -> HPoint,
    f: &In,
    pts: &Pts,
    planes: &[DyPlane],
    s: usize,
    t: usize,
) -> bool {
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
                None => crosses(&q(), &pts.ph(a, planes), &pts.ph(b, planes), s, t),
            };
            if c {
                k += 1;
            }
        }
    }
    k % 2 == 1
}

/// The exact vertices of one body, and their f64 shadows (for screening only): the near faces'
/// worked out first, any other's when a ray first needs it. Tables by vertex index.
pub(crate) struct Pts {
    exact: RefCell<Vec<Option<HPoint>>>,
    f: RefCell<Vec<Option<[f64; 3]>>>,
    /// the snapped planes in f64, by face
    spf: Vec<PlaneF>,
    /// the faces round vertex `v`: `faces[start[v]..start[v + 1]]`, in arena order
    start: Vec<u32>,
    faces: Vec<u32>,
}

impl Pts {
    /// The faces round `v`.
    fn round(&self, v: VertexId) -> &[u32] {
        let v = v.index();
        if v + 1 >= self.start.len() {
            return &[];
        }
        &self.faces[self.start[v] as usize..self.start[v + 1] as usize]
    }
    /// The exact points worked out so far, by vertex.
    #[allow(dead_code)]
    fn known(&self) -> Vec<(VertexId, HPoint)> {
        let e = self.exact.borrow();
        (0..e.len())
            .filter_map(|i| e[i].as_ref().map(|h| (VertexId(i as u32), h.clone())))
            .collect()
    }
    fn ensure(&self, v: VertexId, planes: &[DyPlane]) {
        if self.exact.borrow()[v.index()].is_some() {
            return;
        }
        let ps: Vec<&DyPlane> = self.round(v).iter().map(|&i| &planes[i as usize]).collect();
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
        self.f.borrow_mut()[v.index()] = Some(p.to_f64());
        self.exact.borrow_mut()[v.index()] = Some(p);
    }
    /// The vertex in f64, within 1e-13 of the exact one relative to its size (for screening;
    /// `crosses_f`'s margin, 1e-12 of the scale, covers twice that plus its own rounding):
    /// the f64 meet of the three planes `ensure` would meet, where its bound says so; else the
    /// exact point rounded.
    fn pf(&self, v: VertexId, planes: &[DyPlane]) -> [f64; 3] {
        if let Some(p) = self.f.borrow()[v.index()] {
            return p;
        }
        if let Some(p) = self.first_meet_f(v, planes) {
            self.f.borrow_mut()[v.index()] = Some(p);
            return p;
        }
        self.ensure(v, planes);
        self.f.borrow()[v.index()].unwrap()
    }
    /// The f64 meet of the first independent three of the planes round `v` (their order as
    /// `vertex_at`'s), if its forward bound is within 1e-13 of its size; None to go exact (an
    /// earlier three the f64 cannot tell from dependent, or a loose bound).
    fn first_meet_f(&self, v: VertexId, planes: &[DyPlane]) -> Option<[f64; 3]> {
        let r = self.round(v);
        for (x, &a) in r.iter().enumerate() {
            for (y, &b) in r.iter().enumerate() {
                for (z, &c) in r.iter().enumerate() {
                    if x == y || y == z || x == z {
                        continue; // (a repeated plane: dependent)
                    }
                    let (a, b, c) = (a as usize, b as usize, c as usize);
                    match meet_f(&self.spf[a], &self.spf[b], &self.spf[c]) {
                        Some((p, e)) => {
                            let m = 1.0 + p[0].abs().max(p[1].abs()).max(p[2].abs());
                            return (e <= 1e-13 * m).then_some(p);
                        }
                        None => {
                            // dependent exactly, or the exact meet decides
                            let (pa, pb, pc) = (&planes[a], &planes[b], &planes[c]);
                            if !pa.n.dot(&pb.n.cross(&pc.n)).is_zero() {
                                return None;
                            }
                        }
                    }
                }
            }
        }
        None
    }
    fn ph(&self, v: VertexId, planes: &[DyPlane]) -> HPoint {
        self.ensure(v, planes);
        self.exact.borrow()[v.index()].clone().unwrap()
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
    /// `poly_e[k]`: the edge cell from `poly[k]` to `poly[k + 1]`
    poly_e: Vec<u32>,
    /// an interior point: exactly, once worked out (`Built::mid`); until then, the f64 point
    /// `mid_f` whose projection along axis `mid_a` onto the plane is strictly inside
    mid: std::cell::OnceCell<HPoint>,
    mid_f: [f64; 3],
    mid_a: usize,
}

/// A table by cell id (dense: ids are the ledger's, 0..n).
pub(crate) struct Dense<T>(Vec<Option<T>>);

impl<T> Default for Dense<T> {
    fn default() -> Self {
        Dense(Vec::new())
    }
}

impl<T> Dense<T> {
    fn get(&self, k: &u32) -> Option<&T> {
        self.0.get(*k as usize).and_then(|x| x.as_ref())
    }
    /// The entry of `k`, made by `f` if empty.
    fn or_insert_with(&mut self, k: u32, f: impl FnOnce() -> T) -> &mut T {
        let k = k as usize;
        if self.0.len() <= k {
            self.0.resize_with(k + 1, || None);
        }
        self.0[k].get_or_insert_with(f)
    }
}

impl<T> std::ops::Index<&u32> for Dense<T> {
    type Output = T;
    fn index(&self, k: &u32) -> &T {
        self.get(k).expect("no entry for that cell")
    }
}

/// Everything the bridge decided, kept to build the body. Each table is by cell id.
struct Built {
    ledger: Ledger,
    /// a facet's geometry
    facet: Dense<Facet>,
    /// a vertex cell's exact point
    point: Dense<HPoint>,
    /// a vertex cell's point in f64 (the exact one rounded once), for the write
    #[allow(dead_code)]
    pointf: Dense<[f64; 3]>,
    /// an edge cell's two vertex cells
    ends: Dense<(u32, u32)>,
    /// a side cell: a facet it is a side of and which side
    side_of: Dense<(u32, i32)>,
    /// a side cell: every (facet, side) it is a side of
    sides: Dense<Vec<(u32, i32)>>,
    /// a facet: the input faces it is a piece of
    holders: Dense<Vec<usize>>,
    /// per cell: its covector
    cells: Vec<Row>,
    /// per cell: is it on a wall (some wall class zero); for the write
    #[allow(dead_code)]
    wall: Vec<bool>,
    /// cell ids by covector (`rk`)
    by: FxMap<(u64, u64), u32>,
}

impl Built {
    /// Facet `f`'s interior point, exactly.
    fn mid(&self, cls: &Classes, f: u32) -> &HPoint {
        let fc = &self.facet[&f];
        fc.mid.get_or_init(|| {
            project(cls, fc.class as usize, fc.mid_a, fc.mid_f)
                .expect("a facet's plane is not parallel to its axis")
        })
    }
}

/// A table of cells by covector.
#[derive(Default)]
struct CellTab {
    cells: Vec<Row>,
    by: FxMap<(u64, u64), u32>,
}

impl CellTab {
    /// The id of `c`, added if new.
    fn id_of(&mut self, c: Row) -> u32 {
        if let Some(&i) = self.by.get(&rk(c)) {
            return i;
        }
        self.cells.push(c);
        self.by.insert(rk(c), self.cells.len() as u32 - 1);
        self.cells.len() as u32 - 1
    }
}

/// A point strictly inside the facet of class plane `c` with covector `fc`, near the f64 point
/// `m`: `m`'s coordinates off axis `a` exactly, and the one on `a` that puts it on the plane
/// (`W = n_a`, so its terms stay small). None where that point is not strictly inside (a facet
/// too thin for `m`): the mean of its vertices is then.
#[inline(never)]
fn interior(cls: &Classes, pf: &[PlaneF], c: usize, a: usize, m: [f64; 3], fc: Row) -> Option<HPoint> {
    let h = project(cls, c, a, m)?;
    let hf = h.to_f64();
    let inside =
        (0..cls.planes.len()).all(|k| k == c || side_at(&cls.planes[k], &hf, &pf[k], &h) == row_get(fc, k));
    inside.then_some(h)
}

/// `m`'s coordinates off axis `a` exactly, and the one on `a` that puts it on class plane `c`
/// (`W = n_a`); None if the plane is parallel to `a`.
fn project(cls: &Classes, c: usize, a: usize, m: [f64; 3]) -> Option<HPoint> {
    let pc = &cls.planes[c];
    let (s, t) = ((a + 1) % 3, (a + 2) % 3);
    let (ms, mt) = (Dy::from_f64(m[s]), Dy::from_f64(m[t]));
    let w = pc.n.get(a).clone();
    if w.is_zero() {
        return None;
    }
    let pa = pc.c.sub(&pc.n.get(s).mul(&ms)).sub(&pc.n.get(t).mul(&mt));
    let mut xyz = [Dy::zero(), Dy::zero(), Dy::zero()];
    xyz[a] = pa;
    xyz[s] = ms.mul(&w);
    xyz[t] = mt.mul(&w);
    let [x, y, z] = xyz;
    Some(HPoint {
        p: DyV3::new(x, y, z),
        w,
    })
}

/// `interior` screened in f64: is `project(m)` strictly inside the facet (`Some(true)`), plainly
/// not (`Some(false)`), or does the exact test decide (None)? The projection's coordinates off
/// `a` are `m`'s exactly; on `a` it is `(c − n_s m_s − n_t m_t) / n_a` in f64, within
/// `e_a = γ·(|c| + |n_s m_s| + |n_t m_t|)/|n_a| + γ·|p_a|` of the exact one (`c` one rounding off
/// the exact offset), and each sign is `side_err`'s with that bound.
fn interior_f(cls: &Classes, pf: &[PlaneF], c: usize, a: usize, m: [f64; 3], fc: Row) -> Option<bool> {
    let (s, t) = ((a + 1) % 3, (a + 2) % 3);
    let n = &pf[c];
    if n.n[a] == 0.0 {
        return None;
    }
    let (ps, pt) = (n.n[s] * m[s], n.n[t] * m[t]);
    let mut p = m;
    p[a] = (n.c - ps - pt) / n.n[a];
    let e = GAMMA * (n.c.abs() + ps.abs() + pt.abs()) / n.n[a].abs() + GAMMA * p[a].abs();
    if !p[a].is_finite() || !e.is_finite() {
        return None;
    }
    let mut undecided = false;
    for (k, pk) in pf.iter().enumerate().take(cls.planes.len()) {
        if k == c {
            continue;
        }
        let v = pk.n[0] * p[0] + pk.n[1] * p[1] + pk.n[2] * p[2] - pk.c;
        let mag =
            pk.n[0].abs() * p[0].abs() + pk.n[1].abs() * p[1].abs() + pk.n[2].abs() * p[2].abs() + pk.c.abs();
        let bound = GAMMA * mag + (pk.n[0].abs() + pk.n[1].abs() + pk.n[2].abs()) * e;
        if v.abs() <= bound {
            undecided = true;
            continue;
        }
        let sg = if v > 0.0 { Sgn::Pos } else { Sgn::Neg };
        if sg != row_get(fc, k) {
            return Some(false);
        }
    }
    (!undecided).then_some(true)
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn build_ledger(
    bodies: &[&Body],
    fs: &[In],
    cls: &Classes,
    pts: &[Pts],
    snapped: &[DyPlane],
    bx: Option<([f64; 3], [f64; 3])>,
) -> Built {
    // a cell of the open region: strictly inside every wall (as masks)
    let (mut wpos, mut wneg, mut wall_mask) = (0u64, 0u64, 0u64);
    for (&w, &s) in cls.walls.iter().zip(&cls.wall_in) {
        let b = core::row::bit(w as usize);
        wall_mask |= b;
        match s {
            Sgn::Pos => wpos |= b,
            Sgn::Neg => wneg |= b,
            Sgn::Zero => {}
        }
    }
    let open = |cv: Row| (cv.pos & wpos) == wpos && (cv.neg & wneg) == wneg;
    let on_wall = |cv: Row| !(cv.pos | cv.neg) & wall_mask != 0;
    let pf: Vec<PlaneF> = cls.planes.iter().map(plane_f).collect();
    let mut tab = CellTab::default();
    let mut facet: Dense<Facet> = Dense::default();
    let mut point: Dense<HPoint> = Dense::default();
    let mut pointf: Dense<[f64; 3]> = Dense::default();
    let mut ends: Dense<(u32, u32)> = Dense::default();
    let mut side_of: Dense<(u32, i32)> = Dense::default();
    let mut sides: Dense<Vec<(u32, i32)>> = Dense::default();
    let mut holders_of: Dense<Vec<usize>> = Dense::default();
    let mut pieces: Vec<(u32, u32)> = Vec::new(); // (face, cell)
    let mut edges: Vec<u32> = Vec::new();
    let mut verts: Vec<u32> = Vec::new();
    let mut is_e: HashMap<u32, ()> = HashMap::default();
    let mut is_v: HashMap<u32, ()> = HashMap::default();
    // the faces of each class, in arena order
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); cls.planes.len()];
    for (i, &c) in cls.of_face.iter().enumerate() {
        if c != NONE {
            on[c as usize].push(i);
        }
    }
    let mut per_face: Vec<Vec<u32>> = vec![Vec::new(); fs.len()];
    // scratch: a facet's covector, an edge's, the faces boxing a facet
    let mut boxed: Vec<usize> = Vec::new();
    for (c, faces) in on.iter().enumerate() {
        if faces.is_empty() {
            continue;
        }
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &i in faces {
            // (the kernel's box: the section's box is grown well past it below)
            for k in 0..3 {
                lo[k] = lo[k].min(fs[i].lo[k]);
                hi[k] = hi[k].max(fs[i].hi[k]);
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
        // per arrangement vertex: its cell id, once interned
        let mut vid = vec![NONE; pl.vx.len()];
        for ck in 0..pl.ncells() {
            let cell = pl.cell(ck);
            // a cell inside a face is bounded by classes only
            if cell.iter().any(|&(i, _)| pl.cov(i).is_none()) {
                continue;
            }
            // its covector: on each class the side of any vertex off it
            let mut fc = pl.cov(cell[0].0).unwrap();
            for &(i, _) in &cell[1..] {
                fc = row_join(fc, pl.cov(i).unwrap());
            }
            if !open(fc) {
                continue;
            }
            // the faces whose box holds the cell's middle (in f64, within a margin), and only then
            // the exact middle and its parity
            let (a0, a1, a2) = (
                pl.vx[cell[0].0 as usize].f,
                pl.vx[cell[1].0 as usize].f,
                pl.vx[cell[2].0 as usize].f,
            );
            let _ = ck;
            let midf = [0, 1, 2].map(|k| (a0[k] + a1[k] + a2[k]) / 3.0);
            let g = 4.0 * lin() + 1e-9 * (1.0 + midf[0].abs() + midf[1].abs() + midf[2].abs());
            boxed.clear();
            boxed.extend(
                faces
                    .iter()
                    .copied()
                    .filter(|&i| (0..3).all(|k| fs[i].lo[k] - g <= midf[k] && midf[k] <= fs[i].hi[k] + g)),
            );
            if boxed.is_empty() {
                continue;
            }
            // its interior point: midf projected onto the plane, certified in f64 where it can be
            // (built exactly only when asked), else exactly, else the mean of three vertices
            let mid = std::cell::OnceCell::new();
            if interior_f(cls, &pf, c, a, midf, fc) != Some(true) {
                let h = interior(cls, &pf, c, a, midf, fc).unwrap_or_else(|| {
                    let (e0, e1, e2) = (
                        pl.exact(cls, cell[0].0 as usize),
                        pl.exact(cls, cell[1].0 as usize),
                        pl.exact(cls, cell[2].0 as usize),
                    );
                    HPoint::mean(&[&e0, &e1, &e2])
                });
                let _ = mid.set(h);
            }
            let holders: Vec<usize> = match mid.get() {
                Some(h) => boxed
                    .iter()
                    .copied()
                    .filter(|&i| in_face(h, &fs[i], &pts[fs[i].body], snapped, s, t))
                    .collect(),
                None => {
                    let q = || project(cls, c, a, midf).expect("a facet's plane is not parallel to its axis");
                    boxed
                        .iter()
                        .copied()
                        .filter(|&i| in_face_q(midf, &q, &fs[i], &pts[fs[i].body], snapped, s, t))
                        .collect()
                }
            };
            if holders.is_empty() {
                continue;
            }
            let id = tab.id_of(fc);
            // its vertices and edges, as cells
            let mut poly = Vec::with_capacity(cell.len());
            let mut poly_e = Vec::with_capacity(cell.len());
            for k in 0..cell.len() {
                let (i, j) = (cell[k].0, cell[(k + 1) % cell.len()].0);
                let mut vertex_id = |i: u32, tab: &mut CellTab| {
                    if vid[i as usize] == NONE {
                        vid[i as usize] = tab.id_of(pl.cov(i).unwrap());
                    }
                    vid[i as usize]
                };
                let vi = vertex_id(i, &mut tab);
                if pointf.get(&vi).is_none() {
                    let hf = point.or_insert_with(vi, || pl.exact(cls, i as usize)).to_f64();
                    pointf.or_insert_with(vi, || hf);
                }
                let vj = vertex_id(j, &mut tab);
                let e = tab.id_of(row_join(pl.cov(i).unwrap(), pl.cov(j).unwrap()));
                ends.or_insert_with(e, || (vi, vj));
                poly.push(vi);
                poly_e.push(e);
            }
            if flip {
                // (reversed, the edge from new k to k + 1 is old n - 2 - k's)
                poly.reverse();
                let n = poly_e.len();
                poly_e = (0..n).map(|k| poly_e[(2 * n - 2 - k) % n]).collect();
            }
            for &i in &holders {
                per_face[i].push(id);
                let h = holders_of.or_insert_with(id, Vec::new);
                if !h.contains(&i) {
                    h.push(i);
                }
            }
            facet.or_insert_with(id, || Facet {
                class: c as u32,
                poly,
                poly_e,
                mid,
                mid_f: midf,
                mid_a: a,
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
        let fc = &facet[&id];
        for k in 0..fc.poly.len() {
            let v = fc.poly[k];
            if !on_wall(tab.cells[v as usize]) && is_v.insert(v, ()).is_none() {
                verts.push(v);
            }
            let e = fc.poly_e[k];
            if !on_wall(tab.cells[e as usize]) && is_e.insert(e, ()).is_none() {
                edges.push(e);
            }
        }
    }
    // the facets' sides
    let mut facets_seen: Vec<u32> = pieces.iter().map(|p| p.1).collect();
    facets_seen.dedup();
    let mut done: HashMap<u32, ()> = HashMap::default();
    for &id in &facets_seen {
        if done.insert(id, ()).is_some() {
            continue;
        }
        let c = facet[&id].class as usize;
        for (sg, s) in [(Sgn::Pos, 1), (Sgn::Neg, -1)] {
            let t = tab.id_of(row_set(tab.cells[id as usize], c, sg));
            side_of.or_insert_with(t, || (id, s));
            sides.or_insert_with(t, Vec::new).push((id, s));
        }
    }
    let nb = bodies.len() as u32;
    let ledger = Ledger {
        nb,
        face_body: fs.iter().map(|f| BodyId(f.body as u32)).collect(),
        face_class: cls.of_face.clone(),
        width: cls.planes.len() as u32,
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
        pointf,
        ends,
        side_of,
        sides,
        holders: holders_of,
        wall: tab.cells.iter().map(|&cv| on_wall(cv)).collect(),
        cells: tab.cells,
        by: tab.by,
    }
}

/// A point strictly inside side cell `t`: just off its facet's interior point, toward its side.
fn witness_point(b: &Built, cls: &Classes, t: u32) -> HPoint {
    let (f, s) = b.side_of[&t];
    let fc = &b.facet[&f];
    let cov = b.cells[f as usize];
    let c = fc.class as usize;
    let n = &cls.planes[c].n;
    let mut k = 4;
    loop {
        let d = Dy::from_i64(s as i64).scale2(-k);
        let p = b.mid(cls, f).offset(n, &d);
        let pfl = p.to_f64();
        let ok = cls.planes.iter().enumerate().all(|(j, pl)| {
            if j == c {
                return true;
            }
            side_at(pl, &pfl, &plane_f(pl), &p) == row_get(cov, j)
        });
        if ok && cls.planes[c].side(&p) == s {
            return p;
        }
        k += 6;
        assert!(k < 4000, "no point off a facet inside its side");
    }
}

/// `LedgerWF` (ForgeBoolCore/Specs.lean), the core's precondition: bodies, distinct cells, ids in
/// range, distinct edge ids, and the counts the core's u32 ids and usize sizes can hold.
fn ledger_wf(l: &Ledger) -> bool {
    let n = l.cells.len();
    let distinct = |xs: Vec<&core::Row>| {
        let mut xs: Vec<(u64, u64)> = xs.iter().map(|r| (r.pos, r.neg)).collect();
        xs.sort_unstable();
        xs.windows(2).all(|w| w[0] != w[1])
    };
    let mut es: Vec<u32> = l.edges.iter().map(|e| e.0).collect();
    es.sort_unstable();
    let total = n as u128 + 2 * l.pieces.len() as u128 + 1;
    let wmask = core::row::width_mask(l.width as usize);
    l.nb > 0
        && l.width <= core::row::MAX_WIDTH
        && l.cells
            .iter()
            .all(|r| r.pos & r.neg == 0 && (r.pos | r.neg) & !wmask == 0)
        && distinct(l.cells.iter().collect())
        && l.pieces.iter().all(|(c, _)| (c.0 as usize) < n)
        && l.inside.iter().all(|(c, _)| (c.0 as usize) < n)
        && l.edges.iter().all(|c| (c.0 as usize) < n)
        && l.verts.iter().all(|c| (c.0 as usize) < n)
        && es.windows(2).all(|w| w[0] != w[1])
        && l.face_body.iter().all(|b| b.0 < u32::MAX)
        && l.pieces.iter().all(|(_, f)| f.0 < u32::MAX)
        && total < u32::MAX as u128
        && (l.edges.len() as u128) < u32::MAX as u128
        && (l.verts.len() as u128) < u32::MAX as u128
        && 3 * total <= usize::MAX as u128
}

/// How many booleans the core skipped for width (more classes than a packed row holds), for the
/// gate's report.
pub static SKIP_WIDE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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
    let reg = region(bodies, &fs, &snapped, &reps, rule);
    let pts = vertices(bodies, &fs, &snapped, &reps, &reg)?;
    let mut cls = classes(&snapped, &reps, &reg.near);
    if let Some((lo, hi)) = reg.bx {
        add_walls(&mut cls, lo, hi);
    }
    // (rows hold 64 classes: past that the old path, counted)
    if cls.planes.len() > core::row::MAX_WIDTH as usize {
        SKIP_WIDE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return Err(Skip::Wide);
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
    debug_assert!(
        ledger_wf(&built.ledger),
        "the bridge's ledger is not well formed (LedgerWF)"
    );
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
        for (v, h) in vs.known() {
            let h = &h;
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
#[inline(never)]
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
    let m = b.mid(cls, f0).to_f64();
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
        Err(e) => {
            if std::env::var_os("FORGE_DECIDE_STATS").is_some() {
                eprintln!("decide-once skip: {e:?}");
            }
            return Ok(None);
        }
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
        at: RefCell::new(HashMap::default()),
    };
    let written = |out: Option<&core::Body>| {
        write(&w, out).unwrap_or_else(|| panic!("decide-once: the result's segments do not close into loops"))
    };
    match outcome {
        // (more classes than a row holds: the old path, counted)
        Outcome::Skip(_) => {
            SKIP_WIDE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(None)
        }
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
    let mut seen: HashMap<(u32, u32), u32> = HashMap::default();
    for &(c, f) in &l.pieces {
        let x = l.cells[c.0 as usize];
        let zeros: Vec<usize> = (0..l.width as usize)
            .filter(|&k| core::row::row_get(x, k) == Sgn::Zero)
            .collect();
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
        let mut count: HashMap<u32, u32> = HashMap::default();
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
        let mut on: HashMap<u32, ()> = HashMap::default();
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
        self.built.by.get(&rk(cv)).map(|&c| PK::Cell(c))
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
    let mut pools: std::collections::BTreeMap<Grp, Vec<Seg>> = std::collections::BTreeMap::new();
    // body 0's global face index by FaceId
    let mut gidx: HashMap<FaceId, usize> = HashMap::default();
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
            let mut count: HashMap<u32, u32> = HashMap::default();
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
                if count[&e] == 1 && !built.wall[e as usize] {
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
    let mut per_src: HashMap<usize, u64> = HashMap::default();
    for f in fins {
        *per_src.entry(f.src).or_default() += 1;
    }
    let mut seen: HashMap<usize, u64> = HashMap::default();
    let mut used: HashMap<Name, ()> = HashMap::default();
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
    let mut orig_v: HashMap<u32, (usize, VertexId)> = HashMap::default();
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
                    if let Some(&c) = built.by.get(&rk(cv)) {
                        orig_v.entry(c).or_insert((k, v));
                    }
                }
            }
        }
        let _ = b;
    }
    let mut body = Body::default();
    let mut vid: HashMap<PK, VertexId> = HashMap::default();
    let mut vused: HashMap<Name, ()> = HashMap::default();
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
    let placed: RefCell<HashMap<(usize, VertexId), Option<PK>>> = RefCell::new(HashMap::default());
    let place = |b: usize, v: VertexId| -> Option<PK> {
        if let Some(&p) = placed.borrow().get(&(b, v)) {
            return p;
        }
        let p = if b == 0 && vid_of_orig(w, v) {
            Some(PK::Orig(v))
        } else {
            let h = w.pts[b].ph(v, w.snapped);
            let cv = covector(cls, &w.pf, &h, &[]);
            built.by.get(&rk(cv)).map(|&c| PK::Cell(c))
        };
        placed.borrow_mut().insert((b, v), p);
        p
    };
    let mut orig_e: HashMap<(PK, PK), Name> = HashMap::default();
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
    let mut shell_of: HashMap<usize, forge_topo::ShellId> = HashMap::default();
    let mut eid: HashMap<(PK, PK), (EdgeId, PK)> = HashMap::default();
    let mut eused: HashMap<Name, ()> = HashMap::default();
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
    b.by.get(&rk(row_join(b.cells[u as usize], b.cells[v as usize])))
        .copied()
}
