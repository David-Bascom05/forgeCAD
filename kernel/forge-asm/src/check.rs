//! Interference and clearance between placed occurrences (prior-art §1.8, decisions 2026-11-03):
//! a BVH broad phase over occurrence boxes, a conservative narrow phase of triangle tests on each
//! part's cached mesh, and exact confirmation through kernel booleans only for pairs that hit.
//! Narrow results are cached by (part, part, relative pose); after a drag only moved occurrences
//! are tested again.

use crate::diag::{Diagnostic, Intent};
use crate::model::{DocId, Feature, Fit, FitMatch, FitRule, PartRef, Parts, Path};
use crate::solve::Solution;
use forge_math::{Box3, Frame, Point3, Vec3, tol};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

/// A part's cached mesh in part coordinates, and how far it may lie from the true surface.
/// Triangles turn counter-clockwise seen from outside.
#[derive(Clone, Debug, Default)]
pub struct PartMesh {
    pub positions: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    pub tolerance: f64,
    bvh: OnceLock<Bvh>,
    boxes: OnceLock<Vec<Box3>>,
    sharp: OnceLock<Vec<[bool; 3]>>,
}

impl PartMesh {
    pub fn new(positions: Vec<Point3>, triangles: Vec<[u32; 3]>, tolerance: f64) -> PartMesh {
        PartMesh {
            positions,
            triangles,
            tolerance,
            bvh: OnceLock::new(),
            boxes: OnceLock::new(),
            sharp: OnceLock::new(),
        }
    }
    pub fn from_trimesh(m: &forge_mesh::TriMesh, tolerance: f64) -> PartMesh {
        PartMesh::new(m.positions.clone(), m.triangles.clone(), tolerance)
    }
    pub fn bounds(&self) -> Box3 {
        self.bvh().nodes.first().map_or(Box3::EMPTY, |n| n.bbox)
    }
    fn tri(&self, t: usize) -> [Point3; 3] {
        self.triangles[t].map(|i| self.positions[i as usize])
    }
    /// Each triangle edge (i to i+1) that is a feature: open, or turning more than 20° to its
    /// neighbour. Touching along such an edge is contact, not a crossing.
    fn sharp(&self) -> &[[bool; 3]] {
        self.sharp.get_or_init(|| {
            let mut by: BTreeMap<(u32, u32), Vec<(usize, usize)>> = BTreeMap::new();
            for (t, tri) in self.triangles.iter().enumerate() {
                for i in 0..3 {
                    let (a, b) = (tri[i], tri[(i + 1) % 3]);
                    by.entry((a.min(b), a.max(b))).or_default().push((t, i));
                }
            }
            let mut out = vec![[true; 3]; self.triangles.len()];
            let cos = forge_math::dmath::cos(20f64.to_radians());
            for uses in by.values() {
                if let [(t1, i1), (t2, i2)] = uses[..] {
                    let (n1, n2) = (normal(&self.tri(t1)), normal(&self.tri(t2)));
                    let l = n1.length() * n2.length();
                    let smooth = l > 0.0 && n1.dot(n2) >= cos * l;
                    out[t1][i1] = !smooth;
                    out[t2][i2] = !smooth;
                }
            }
            out
        })
    }
    fn bvh(&self) -> &Bvh {
        self.bvh.get_or_init(|| Bvh::new(self.tri_boxes()))
    }
    /// Each triangle's box, built once: the narrow phase queries them per sample point.
    fn tri_boxes(&self) -> &[Box3] {
        self.boxes.get_or_init(|| {
            (0..self.triangles.len())
                .map(|t| Box3::from_points(self.tri(t)))
                .collect()
        })
    }
}

// ---- bounding volume hierarchy ----

#[derive(Clone, Debug)]
struct Node {
    bbox: Box3,
    /// Leaf: items `items[lo..hi]`; inner: children `lo` and `hi`.
    lo: usize,
    hi: usize,
    leaf: bool,
}

#[derive(Clone, Debug, Default)]
struct Bvh {
    nodes: Vec<Node>,
    items: Vec<usize>,
}

const LEAF: usize = 4;

impl Bvh {
    /// Median splits on the widest axis of the centres; ties by index, so the tree depends on the
    /// boxes only.
    fn new(boxes: &[Box3]) -> Bvh {
        let mut b = Bvh {
            nodes: Vec::new(),
            items: (0..boxes.len()).collect(),
        };
        if boxes.is_empty() {
            return b;
        }
        let centres: Vec<Point3> = boxes.iter().map(Box3::center).collect();
        // (node, lo, hi) still to split
        b.nodes.push(Node {
            bbox: Box3::EMPTY,
            lo: 0,
            hi: boxes.len(),
            leaf: true,
        });
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let (lo, hi) = (b.nodes[n].lo, b.nodes[n].hi);
            let mut bb = Box3::EMPTY;
            let mut cb = Box3::EMPTY;
            for &i in &b.items[lo..hi] {
                bb = bb.hull(&boxes[i]);
                cb.include(centres[i]);
            }
            b.nodes[n].bbox = bb;
            if hi - lo <= LEAF {
                continue;
            }
            let w = [cb.x.width(), cb.y.width(), cb.z.width()];
            let axis = if w[0] >= w[1] && w[0] >= w[2] {
                0
            } else if w[1] >= w[2] {
                1
            } else {
                2
            };
            b.items[lo..hi].sort_by(|&p, &q| centres[p][axis].total_cmp(&centres[q][axis]).then(p.cmp(&q)));
            let mid = (lo + hi) / 2;
            let l = b.nodes.len();
            b.nodes.push(Node {
                bbox: Box3::EMPTY,
                lo,
                hi: mid,
                leaf: true,
            });
            b.nodes.push(Node {
                bbox: Box3::EMPTY,
                lo: mid,
                hi,
                leaf: true,
            });
            b.nodes[n] = Node {
                bbox: bb,
                lo: l,
                hi: l + 1,
                leaf: false,
            };
            stack.push(l + 1);
            stack.push(l);
        }
        b
    }

    /// Items whose boxes the segment `p` to `q` passes through (slab test per node, so a long ray
    /// visits only the boxes on its way, not every box in its bounding box).
    fn segment(&self, boxes: &[Box3], p: Point3, q: Point3, out: &mut Vec<usize>) {
        if self.nodes.is_empty() {
            return;
        }
        let d = q - p;
        let hits = |b: &Box3| -> bool {
            let (mut lo, mut hi) = (0.0f64, 1.0f64);
            for (o, di, iv) in [(p.x, d.x, b.x), (p.y, d.y, b.y), (p.z, d.z, b.z)] {
                if iv.lo > iv.hi {
                    return false;
                }
                if di == 0.0 {
                    if o < iv.lo || o > iv.hi {
                        return false;
                    }
                    continue;
                }
                let (mut t0, mut t1) = ((iv.lo - o) / di, (iv.hi - o) / di);
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                lo = lo.max(t0);
                hi = hi.min(t1);
                if lo > hi {
                    return false;
                }
            }
            true
        };
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if !hits(&node.bbox) {
                continue;
            }
            if node.leaf {
                out.extend(
                    self.items[node.lo..node.hi]
                        .iter()
                        .copied()
                        .filter(|&i| hits(&boxes[i])),
                );
            } else {
                stack.push(node.hi);
                stack.push(node.lo);
            }
        }
    }

    /// Items whose boxes overlap `q`.
    fn query(&self, boxes: &[Box3], q: &Box3, out: &mut Vec<usize>) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if !node.bbox.overlaps(q) {
                continue;
            }
            if node.leaf {
                out.extend(
                    self.items[node.lo..node.hi]
                        .iter()
                        .copied()
                        .filter(|&i| boxes[i].overlaps(q)),
                );
            } else {
                stack.push(node.hi);
                stack.push(node.lo);
            }
        }
    }
}

/// The box of `b` moved by `f` (its corners' box: conservative).
fn moved_box(b: &Box3, f: &Frame) -> Box3 {
    if b.is_empty() {
        return *b;
    }
    let mut out = Box3::EMPTY;
    for x in [b.x.lo, b.x.hi] {
        for y in [b.y.lo, b.y.hi] {
            for z in [b.z.lo, b.z.hi] {
                out.include(f.point_to_world(Point3::new(x, y, z)));
            }
        }
    }
    out
}

fn box_gap(a: &Box3, b: &Box3) -> f64 {
    let g = |p: forge_math::Interval, q: forge_math::Interval| (q.lo - p.hi).max(p.lo - q.hi).max(0.0);
    let (x, y, z) = (g(a.x, b.x), g(a.y, b.y), g(a.z, b.z));
    (x * x + y * y + z * z).sqrt()
}

// ---- triangle geometry ----

fn normal(t: &[Point3; 3]) -> Vec3 {
    (t[1] - t[0]).cross(t[2] - t[0])
}

/// Whether segment pq passes through triangle t, reaching more than `depth` behind its face
/// (inside the solid it bounds), and more than `depth` clear of t's feature edges.
fn pierces(p: Point3, q: Point3, t: &[Point3; 3], sharp: [bool; 3], depth: f64) -> bool {
    let n = normal(t);
    let len = n.length();
    if len == 0.0 {
        return false;
    }
    let n = n / len;
    let (dp, dq) = ((p - t[0]).dot(n), (q - t[0]).dot(n));
    if (dp > 0.0) == (dq > 0.0) || dp == dq {
        return false;
    }
    if -dp.min(dq) <= depth {
        return false;
    }
    let x = p + (q - p) * (dp / (dp - dq));
    // inside the triangle: same side of each edge
    let e = |a: Point3, b: Point3| (b - a).cross(x - a).dot(n);
    let (u, v, w) = (e(t[0], t[1]), e(t[1], t[2]), e(t[2], t[0]));
    let inside = (u >= 0.0 && v >= 0.0 && w >= 0.0) || (u <= 0.0 && v <= 0.0 && w <= 0.0);
    inside
        && (0..3).all(|i| {
            !sharp[i] || {
                let (_, c) = closest_segments(x, x, t[i], t[(i + 1) % 3]);
                x.distance(c) > depth
            }
        })
}

/// Two triangles cross deeper than `depth`: an edge of one through the other.
fn crosses(a: &[Point3; 3], sa: [bool; 3], b: &[Point3; 3], sb: [bool; 3], depth: f64) -> bool {
    (0..3).any(|i| pierces(a[i], a[(i + 1) % 3], b, sb, depth) || pierces(b[i], b[(i + 1) % 3], a, sa, depth))
}

/// The point of triangle t nearest p (Ericson, Real-Time Collision Detection 5.1.5).
fn closest_on_tri(p: Point3, t: &[Point3; 3]) -> Point3 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let den = 1.0 / (va + vb + vc);
    a + ab * (vb * den) + ac * (vc * den)
}

/// Nearest points of segments p1q1 and p2q2 (Ericson 5.1.9).
fn closest_segments(p1: Point3, q1: Point3, p2: Point3, q2: Point3) -> (Point3, Point3) {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t);
    if a <= 1e-300 && e <= 1e-300 {
        return (p1, p2);
    }
    if a <= 1e-300 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-300 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let den = a * e - b * b;
            let s0 = if den != 0.0 {
                ((b * f - c * e) / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            } else {
                t = t0;
                s = s0;
            }
        }
    }
    (p1 + d1 * s, p2 + d2 * t)
}

/// Distance between two triangles (0 if they cross), with the nearest points.
fn tri_distance(a: &[Point3; 3], b: &[Point3; 3]) -> (f64, Point3, Point3) {
    if crosses(a, [false; 3], b, [false; 3], -1.0) {
        return (0.0, a[0], a[0]);
    }
    let mut best = (f64::INFINITY, a[0], b[0]);
    let mut take = |p: Point3, q: Point3| {
        let d = p.distance(q);
        if d < best.0 {
            best = (d, p, q);
        }
    };
    for i in 0..3 {
        take(a[i], closest_on_tri(a[i], b));
        take(closest_on_tri(b[i], a), b[i]);
        for j in 0..3 {
            let (p, q) = closest_segments(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3]);
            take(p, q);
        }
    }
    best
}

// ---- mesh against mesh ----

/// What the triangle tests say about two meshes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Narrow {
    Clear,
    /// Surfaces cross deeper than the tolerance.
    Cross,
    /// One lies inside the other.
    Inside,
}

/// B's positions and node boxes taken into A's frame.
struct Placed<'a> {
    a: &'a PartMesh,
    b: &'a PartMesh,
    pos: Vec<Point3>,
    boxes: Vec<Box3>,
}

impl<'a> Placed<'a> {
    fn new(a: &'a PartMesh, b: &'a PartMesh, rel: &Frame) -> Placed<'a> {
        Placed {
            a,
            b,
            pos: b.positions.iter().map(|&p| rel.point_to_world(p)).collect(),
            boxes: b.bvh().nodes.iter().map(|n| moved_box(&n.bbox, rel)).collect(),
        }
    }
    fn tri_b(&self, t: usize) -> [Point3; 3] {
        self.b.triangles[t].map(|i| self.pos[i as usize])
    }
    /// Node pairs whose boxes come within `reach`, leaves first-found order deterministic.
    fn walk(&self, reach: f64, mut leaf: impl FnMut(usize, usize) -> bool) {
        let (ba, bb) = (self.a.bvh(), self.b.bvh());
        if ba.nodes.is_empty() || bb.nodes.is_empty() {
            return;
        }
        let mut stack = vec![(0usize, 0usize)];
        while let Some((na, nb)) = stack.pop() {
            let (x, y) = (&ba.nodes[na], &bb.nodes[nb]);
            if box_gap(&x.bbox, &self.boxes[nb]) > reach {
                continue;
            }
            match (x.leaf, y.leaf) {
                (true, true) => {
                    for &ta in &ba.items[x.lo..x.hi] {
                        for &tb in &bb.items[y.lo..y.hi] {
                            if leaf(ta, tb) {
                                return;
                            }
                        }
                    }
                }
                (false, true) => {
                    stack.push((x.hi, nb));
                    stack.push((x.lo, nb));
                }
                (true, false) => {
                    stack.push((na, y.hi));
                    stack.push((na, y.lo));
                }
                (false, false) => {
                    stack.push((x.hi, y.hi));
                    stack.push((x.hi, y.lo));
                    stack.push((x.lo, y.hi));
                    stack.push((x.lo, y.lo));
                }
            }
        }
    }
}

/// Signed side of a point against a closed mesh by ray parity: Some(true) inside by more than
/// `depth`, Some(false) outside, None when too close to call.
fn inside(m: &PartMesh, p: Point3, depth: f64) -> Option<bool> {
    let bvh = m.bvh();
    // nearest surface first: a point within `depth` of it is not decided
    let mut near = f64::INFINITY;
    let mut stack = vec![0usize];
    while let Some(n) = stack.pop() {
        let node = &bvh.nodes[n];
        if box_gap(&node.bbox, &Box3::from_point(p)) > near.min(depth) {
            continue;
        }
        if node.leaf {
            for &t in &bvh.items[node.lo..node.hi] {
                near = near.min(p.distance(closest_on_tri(p, &m.tri(t))));
            }
        } else {
            stack.push(node.hi);
            stack.push(node.lo);
        }
    }
    if near <= depth {
        return None;
    }
    // a ray in a direction no model axis takes
    let d = Vec3::new(0.571_428_57, 0.618_034, 0.538_461_5);
    let far = p + d * (2.0 * (m.bounds().diagonal() + p.to_vec().length() + 1.0));
    let mut hits = 0usize;
    let mut out = Vec::new();
    bvh.segment(m.tri_boxes(), p, far, &mut out);
    for t in out {
        let tri = m.tri(t);
        let n = normal(&tri);
        let (dp, dq) = ((p - tri[0]).dot(n), (far - tri[0]).dot(n));
        if (dp > 0.0) == (dq > 0.0) {
            continue;
        }
        let x = p + (far - p) * (dp / (dp - dq));
        let e = |a: Point3, b: Point3| (b - a).cross(x - a).dot(n);
        let (u, v, w) = (e(tri[0], tri[1]), e(tri[1], tri[2]), e(tri[2], tri[0]));
        if u == 0.0 || v == 0.0 || w == 0.0 {
            return None;
        }
        if (u > 0.0 && v > 0.0 && w > 0.0) || (u < 0.0 && v < 0.0 && w < 0.0) {
            hits += 1;
        }
    }
    Some(hits % 2 == 1)
}

fn narrow(a: &PartMesh, b: &PartMesh, rel: &Frame, depth: f64) -> Narrow {
    // the answer is symmetric; placing the smaller mesh in the larger one's frame moves fewer points
    if b.positions.len() > a.positions.len() {
        return narrow(b, a, &rel.inverse(), depth);
    }
    let pl = Placed::new(a, b, rel);
    let mut hit = false;
    pl.walk(0.0, |ta, tb| {
        hit = crosses(&a.tri(ta), a.sharp()[ta], &pl.tri_b(tb), b.sharp()[tb], depth);
        hit
    });
    if hit {
        return Narrow::Cross;
    }
    // no clean crossing: a face of one may still lie inside the other (one inside the other, or
    // surfaces that cross only along coincident feature edges). In the shared region, the sampled
    // mesh's triangles farther than `depth` from the tested mesh's surface fall into connected
    // pieces (by shared vertices) that cannot pass through that surface, so each piece lies wholly
    // on one side of it: one parity ray per piece decides it. Triangles that come within `depth`
    // are tested one by one at their centres, as before.
    let inv = rel.inverse();
    let bb = moved_box(&b.bounds(), rel);
    let ab = a.bounds();
    let meet = |p: forge_math::Interval, q: forge_math::Interval| {
        let (lo, hi) = (p.lo.max(q.lo), p.hi.min(q.hi));
        (lo <= hi).then(|| forge_math::Interval::new(lo, hi))
    };
    let (Some(x), Some(y), Some(z)) = (meet(ab.x, bb.x), meet(ab.y, bb.y), meet(ab.z, bb.z)) else {
        return Narrow::Clear;
    };
    let shared = Box3 { x, y, z }.grow(depth);
    // (mesh tested, mesh sampled, sampled into tested's frame, the shared region in tested's frame,
    // tested's frame into sampled's)
    let in_b = moved_box(&shared, &inv);
    for (m, other, map, region, back) in [(a, b, rel, shared, inv), (b, a, &inv, in_b, *rel)] {
        let ob = other.bvh();
        let mut found = Vec::new();
        ob.query(other.tri_boxes(), &moved_box(&region, &back), &mut found);
        found.sort_unstable();
        let placed: Vec<[Point3; 3]> = found
            .iter()
            .map(|&t| other.tri(t).map(|p| map.point_to_world(p)))
            .collect();
        let far: Vec<bool> = placed.iter().map(|tri| !near_surface(m, tri, depth)).collect();
        // a triangle that comes near the surface may still reach deep through it (a coarse face
        // whose edge lies on the other's face): its own centre is a sample, as a piece of one
        let near = (0..found.len()).filter(|&k| !far[k]).map(|k| vec![k]);
        for piece in pieces(other, &found, &far).into_iter().chain(near) {
            for k in piece {
                let tri = &placed[k];
                let c = Point3::new(
                    (tri[0].x + tri[1].x + tri[2].x) / 3.0,
                    (tri[0].y + tri[1].y + tri[2].y) / 3.0,
                    (tri[0].z + tri[1].z + tri[2].z) / 3.0,
                );
                match inside(m, c, depth) {
                    Some(true) => return Narrow::Inside,
                    Some(false) => break,
                    None => {}
                }
            }
        }
    }
    Narrow::Clear
}

/// True when some triangle of `m` comes within `depth` of `t` (in `m`'s frame).
fn near_surface(m: &PartMesh, t: &[Point3; 3], depth: f64) -> bool {
    let bvh = m.bvh();
    if bvh.nodes.is_empty() {
        return false;
    }
    let tb = Box3::from_points(*t);
    let mut stack = vec![0usize];
    while let Some(n) = stack.pop() {
        let node = &bvh.nodes[n];
        if box_gap(&node.bbox, &tb) > depth {
            continue;
        }
        if node.leaf {
            for &i in &bvh.items[node.lo..node.hi] {
                if box_gap(&m.tri_boxes()[i], &tb) <= depth && tri_distance(&m.tri(i), t).0 <= depth {
                    return true;
                }
            }
        } else {
            stack.push(node.hi);
            stack.push(node.lo);
        }
    }
    false
}

/// The connected pieces (by shared vertices) of the triangles `found[k]` with `keep[k]`, each as
/// indices into `found` in ascending order, pieces ordered by their first index.
fn pieces(m: &PartMesh, found: &[usize], keep: &[bool]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..found.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut first: BTreeMap<u32, usize> = BTreeMap::new();
    for (k, &t) in found.iter().enumerate() {
        if !keep[k] {
            continue;
        }
        for v in m.triangles[t] {
            match first.get(&v) {
                Some(&j) => {
                    let (x, y) = (root(&mut parent, j), root(&mut parent, k));
                    if x != y {
                        parent[x.max(y)] = x.min(y);
                    }
                }
                None => {
                    first.insert(v, k);
                }
            }
        }
    }
    let mut by: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for k in (0..found.len()).filter(|&k| keep[k]) {
        let r = root(&mut parent, k);
        by.entry(r).or_default().push(k);
    }
    by.into_values().collect()
}

/// Least distance between two meshes, B placed by `rel` in A's frame; None past `limit`.
fn distance(a: &PartMesh, b: &PartMesh, rel: &Frame, limit: f64) -> Option<(f64, Point3, Point3)> {
    let pl = Placed::new(a, b, rel);
    let mut best: Option<(f64, Point3, Point3)> = None;
    let mut reach = limit;
    // best-first would prune more; a depth-first walk with a shrinking reach is deterministic
    let (ba, bb) = (a.bvh(), b.bvh());
    if ba.nodes.is_empty() || bb.nodes.is_empty() {
        return None;
    }
    let mut stack = vec![(0usize, 0usize)];
    while let Some((na, nb)) = stack.pop() {
        let (x, y) = (&ba.nodes[na], &bb.nodes[nb]);
        if box_gap(&x.bbox, &pl.boxes[nb]) > reach {
            continue;
        }
        match (x.leaf, y.leaf) {
            (true, true) => {
                for &ta in &ba.items[x.lo..x.hi] {
                    for &tb in &bb.items[y.lo..y.hi] {
                        let (d, p, q) = tri_distance(&a.tri(ta), &pl.tri_b(tb));
                        if d <= reach && best.is_none_or(|b| d < b.0) {
                            best = Some((d, p, q));
                            reach = d;
                        }
                    }
                }
            }
            (false, true) => {
                stack.push((x.hi, nb));
                stack.push((x.lo, nb));
            }
            (true, false) => {
                stack.push((na, y.hi));
                stack.push((na, y.lo));
            }
            (false, false) => {
                stack.push((x.hi, y.hi));
                stack.push((x.hi, y.lo));
                stack.push((x.lo, y.hi));
                stack.push((x.lo, y.lo));
            }
        }
    }
    best
}

// ---- exact confirmation ----

/// Why an exact overlap volume could not be had.
#[derive(Clone, Debug, PartialEq)]
pub enum ExactFail {
    /// No kernel solid for one of the parts.
    NoSolid,
    /// The kernel's intersection (or its mass properties) failed: its reason.
    Boolean(String),
}

/// The exact overlap volume of two parts, B placed by `b_in_a` in A's frame.
pub trait Exact: Sync {
    fn overlap_volume(&self, a: &PartRef, b: &PartRef, b_in_a: &Frame) -> Result<f64, ExactFail>;
}

impl<T: Exact + ?Sized> Exact for &T {
    fn overlap_volume(&self, a: &PartRef, b: &PartRef, b_in_a: &Frame) -> Result<f64, ExactFail> {
        (**self).overlap_volume(a, b, b_in_a)
    }
}

impl<T: Exact + ?Sized + Send> Exact for Arc<T> {
    fn overlap_volume(&self, a: &PartRef, b: &PartRef, b_in_a: &Frame) -> Result<f64, ExactFail> {
        (**self).overlap_volume(a, b, b_in_a)
    }
}

/// No exact geometry: mesh hits are reported unconfirmed.
pub struct NoExact;

impl Exact for NoExact {
    fn overlap_volume(&self, _: &PartRef, _: &PartRef, _: &Frame) -> Result<f64, ExactFail> {
        Err(ExactFail::NoSolid)
    }
}

/// Exact volumes by the kernel's boolean intersection and mass properties.
#[derive(Default)]
pub struct BrepExact {
    pub bodies: BTreeMap<(DocId, u64), forge_topo::Body>,
}

impl Exact for BrepExact {
    fn overlap_volume(&self, a: &PartRef, b: &PartRef, b_in_a: &Frame) -> Result<f64, ExactFail> {
        let ba = self
            .bodies
            .get(&(a.doc.clone(), a.body))
            .ok_or(ExactFail::NoSolid)?;
        let bb = self
            .bodies
            .get(&(b.doc.clone(), b.body))
            .ok_or(ExactFail::NoSolid)?;
        let moved = rigid_copy(bb, b_in_a);
        match forge_bool::boolean::boolean(ba, &moved, forge_bool::boolean::Op::Intersect, 0) {
            Ok(c) if c.face_ids().next().is_none() => Ok(0.0),
            Ok(c) => forge_topo::mass::mass(&c)
                .map(|m| m.volume.abs())
                .map_err(|e| ExactFail::Boolean(format!("mass of the intersection: {e:?}"))),
            Err(e) => Err(ExactFail::Boolean(format!("{e:?}"))),
        }
    }
}

/// A copy of `b` moved rigidly by `f`, names kept.
pub fn rigid_copy(b: &forge_topo::Body, f: &Frame) -> forge_topo::Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    use forge_topo::body::{EdgeId, ShellId, VertexId};
    let mut out = forge_topo::Body::new();
    out.joins = b.joins.clone();
    let mut vmap: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; b.edges.len()];
    let mut smap: Vec<Option<ShellId>> = vec![None; b.shells.len()];
    for fid in b.face_ids() {
        let face = b.face(fid);
        let shell = *smap[face.shell.index()].get_or_insert_with(|| out.add_shell());
        let nf = out.add_face(shell, face.surface.transformed(f), face.same_sense, face.name);
        for &l in &face.loops {
            let mut uses = Vec::new();
            for c in b.lp(l).coedges() {
                let co = b.coedge(c);
                let e = match emap[co.edge.index()] {
                    Some(e) => e,
                    None => {
                        let edge = b.edge(co.edge);
                        let mut vert = |v: VertexId| {
                            *vmap[v.index()].get_or_insert_with(|| {
                                let x = b.vertex(v);
                                out.add_tolerant_vertex(f.point_to_world(x.point), x.tol, x.name)
                            })
                        };
                        let (s, t) = (vert(edge.start), vert(edge.end));
                        let e = out.add_edge(edge.curve.transformed(f), edge.range, s, t, edge.name);
                        emap[co.edge.index()] = Some(e);
                        e
                    }
                };
                uses.push((e, co.forward));
            }
            out.add_loop(nf, &uses);
        }
    }
    for &w in &b.split {
        out.note_split(w);
    }
    out
}

// ---- the check ----

/// Two occurrences that share volume: a warning, never an error (contact is sometimes meant).
#[derive(Clone, Debug, PartialEq)]
pub struct Interference {
    pub a: Path,
    pub b: Path,
    /// Exact shared volume (mm³), where kernel solids confirmed it.
    pub volume: Option<f64>,
    /// False when no exact geometry was there: the meshes cross, deeper than `tolerance`.
    pub confirmed: bool,
    /// The narrow phase's tolerance for this pair: overlaps shallower than this may be missed.
    pub tolerance: f64,
    /// When unconfirmed because the kernel's intersection failed: its reason.
    pub boolean_failed: Option<String>,
}

/// Contact that a declared fit or a fit rule says is meant.
#[derive(Clone, Debug, PartialEq)]
pub struct IntendedContact {
    pub a: Path,
    pub b: Path,
    pub by: Intent,
}

/// The pairs one fit rule covered.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleUse {
    pub rule: u64,
    pub pairs: Vec<(Path, Path)>,
}

#[derive(Clone, Debug, PartialEq)]
enum PairHit {
    Interference(Interference),
    /// With the engagement and its minimum where a rule's threads engage too little.
    Intended(IntendedContact, Option<Measured>),
    /// The meshes crossed but the exact solids only touch.
    Touch(Path, Path),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterferenceReport {
    /// Unintended shared volume, by path: warnings.
    pub interferences: Vec<Interference>,
    /// Contact covered by a fit or a fit rule, by path.
    pub intended: Vec<IntendedContact>,
    /// Pairs whose solids only touch.
    pub touching: Vec<(Path, Path)>,
    /// Each fit rule that covered something, and what.
    pub rules: Vec<RuleUse>,
    /// Rule-covered threads that engage too little: warnings.
    pub shallow: Vec<ShallowEngagement>,
    /// Every rule-covered thread pair's engagement, short or not.
    pub engagements: Vec<Engagement>,
    /// Occurrences with no mesh, not checked.
    pub unchecked: Vec<Path>,
    pub candidates: usize,
    pub narrow_tests: usize,
    pub cache_hits: usize,
    pub exact_runs: usize,
    pub reused: usize,
}

impl InterferenceReport {
    /// As typed diagnostics: interference a warning, intended contact for information.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out: Vec<Diagnostic> = self
            .interferences
            .iter()
            .map(|i| Diagnostic::Interference {
                a: i.a.clone(),
                b: i.b.clone(),
                volume: i.volume,
                confirmed: i.confirmed,
                boolean_failed: i.boolean_failed.clone(),
            })
            .collect();
        out.extend(self.shallow.iter().map(|x| Diagnostic::ShallowEngagement {
            a: x.a.clone(),
            b: x.b.clone(),
            rule: x.rule,
            engaged: x.engaged,
            min: x.min,
        }));
        out.extend(self.intended.iter().map(|i| Diagnostic::IntendedContact {
            a: i.a.clone(),
            b: i.b.clone(),
            by: i.by,
        }));
        out
    }
}

/// Whether an occurrence matches one side of a fit rule; for a feature, its anchors' world frames.
fn feature_axes<P: Parts>(parts: &P, o: &Occ, m: &FitMatch) -> Option<Vec<(Frame, Feature)>> {
    match m {
        FitMatch::Part { doc, body } => {
            (o.part.doc == *doc && body.is_none_or(|b| b == o.part.body)).then(Vec::new)
        }
        FitMatch::Feature { kind, size } => {
            let info = parts.part(&o.part)?;
            let axes: Vec<(Frame, Feature)> = info
                .features
                .iter()
                .filter(|(_, f)| f.kind == *kind && size.is_none_or(|s| (s - f.size).abs() <= 1e-9))
                .filter_map(|(n, f)| {
                    let r = info.refs.get(n)?.as_ref().ok()?;
                    Some((o.pose.compose(&r.frame), f.clone()))
                })
                .collect();
            (!axes.is_empty()).then_some(axes)
        }
    }
}

/// Two anchors on one axis (either way round), within the narrow phase's tolerance.
fn coaxial(p: &Frame, q: &Frame, tol: f64) -> bool {
    let (zp, zq) = (p.z().get(), q.z().get());
    zp.cross(zq).length() <= 1e-9 && (p.origin - q.origin).cross(zq).length() <= tol
}

/// How far two coaxial threads overlap along their axis: each runs `depth` from its anchor's
/// origin along its z.
pub fn engagement(p: &Frame, dp: f64, q: &Frame, dq: f64) -> f64 {
    let zq = q.z().get();
    let s0 = (p.origin - q.origin).dot(zq);
    let s1 = (p.origin + p.z().get() * dp - q.origin).dot(zq);
    let (lo, hi) = (s0.min(s1), s0.max(s1));
    (hi.min(dq) - lo.max(0.0)).max(0.0)
}

/// Two threads' engagement as measured, with the minimum it was held to (None: no minimum
/// applies) and whether it fell short.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    pub engaged: f64,
    pub min: Option<f64>,
    pub short: bool,
}

/// None: the rule does not cover the pair. Some(None): covered, nothing to measure (no threads
/// with depths on one axis). Some(Some(m)): covered, its threads' engagement measured.
type Covered = Option<Option<Measured>>;

fn rule_covers<P: Parts>(parts: &P, r: &FitRule, x: &Occ, y: &Occ, tol: f64) -> Covered {
    let one = |x: &Occ, y: &Occ| -> Covered {
        let (ax, ay) = (feature_axes(parts, x, &r.a)?, feature_axes(parts, y, &r.b)?);
        if ax.is_empty() || ay.is_empty() {
            return Some(None);
        }
        // features on both sides: they must meet on one axis; the best engagement is reported
        let mut covered: Covered = None;
        for (p, fp) in &ax {
            for (q, fq) in &ay {
                if !coaxial(p, q, tol) {
                    continue;
                }
                let (Some(dp), Some(dq)) = (fp.depth, fq.depth) else {
                    covered = covered.or(Some(None));
                    continue;
                };
                let threads = fp.kind.is_thread() && fq.kind.is_thread();
                let engaged = engagement(p, dp, q, dq);
                let min = match r.min_engagement {
                    Some(crate::model::MinEngagement::Length(m)) => Some(m),
                    // the internal thread's whole depth: the nut's, the tapped hole's
                    Some(crate::model::MinEngagement::Full) => {
                        Some(if fp.kind == crate::model::FeatureKind::InternalThread {
                            dp
                        } else {
                            dq
                        })
                    }
                    None => threads.then(|| crate::model::SHOP_THREAD_ENGAGEMENT * fp.size.min(fq.size)),
                };
                let m = Measured {
                    engaged,
                    min,
                    short: min.is_some_and(|m| engaged < m - tol),
                };
                let better = match covered {
                    Some(Some(c)) => (c.short && !m.short) || (c.short == m.short && m.engaged > c.engaged),
                    _ => true,
                };
                if better {
                    covered = Some(Some(m));
                }
            }
        }
        covered
    };
    one(x, y).or_else(|| one(y, x))
}

/// A rule's threads that engage less than its minimum: a warning.
#[derive(Clone, Debug, PartialEq)]
pub struct ShallowEngagement {
    pub a: Path,
    pub b: Path,
    pub rule: u64,
    pub engaged: f64,
    pub min: f64,
}

/// Every rule-covered thread pair's engagement as measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Engagement {
    pub a: Path,
    pub b: Path,
    pub rule: u64,
    pub engaged: f64,
    pub min: Option<f64>,
    pub short: bool,
}

type Key = (PartKey, PartKey, [u64; 9]);
type PairResults = BTreeMap<(usize, usize), Option<PairHit>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PartKey {
    Hash([u8; 32]),
    Doc(DocId, u64),
}

#[derive(Clone, Debug)]
struct Occ {
    path: Path,
    part: PartRef,
    pose: Frame,
    mesh: Option<Arc<PartMesh>>,
    key: PartKey,
}

/// Interference and clearance over a live assembly, keeping its caches between calls.
pub struct Checker<P: Parts, E: Exact> {
    parts: P,
    exact: E,
    pub threads: usize,
    cache: BTreeMap<Key, (Narrow, Result<f64, ExactFail>)>,
    last: Option<(Vec<Occ>, PairResults)>,
}

fn pose_bits(f: &Frame) -> [u64; 9] {
    let (o, x, z) = (f.origin, f.x().get(), f.z().get());
    [o.x, o.y, o.z, x.x, x.y, x.z, z.x, z.y, z.z].map(f64::to_bits)
}

fn par<T: Send>(n: usize, threads: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    #[cfg(target_family = "wasm")]
    let threads = {
        let _ = threads;
        1
    };
    if threads <= 1 || n < 64 {
        return (0..n).map(f).collect();
    }
    let chunk = n.div_ceil(threads);
    std::thread::scope(|s| {
        let hs: Vec<_> = (0..n)
            .step_by(chunk)
            .map(|lo| {
                let f = &f;
                s.spawn(move || (lo..(lo + chunk).min(n)).map(f).collect::<Vec<T>>())
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("a check")).collect()
    })
}

/// Below this exact volume (mm³) two solids only touch.
pub const TOUCH_VOLUME: f64 = 1e-6;

impl<P: Parts, E: Exact> Checker<P, E> {
    pub fn parts(&self) -> &P {
        &self.parts
    }
    pub fn exact(&self) -> &E {
        &self.exact
    }

    pub fn new(parts: P, exact: E) -> Self {
        Checker {
            parts,
            exact,
            threads: crate::solve::Options::default().threads,
            cache: BTreeMap::new(),
            last: None,
        }
    }

    fn occurrences(&self, sol: &Solution) -> Vec<Occ> {
        sol.occurrences
            .iter()
            .map(|o| {
                let info = self.parts.part(&o.part);
                Occ {
                    path: o.path.clone(),
                    part: o.part.clone(),
                    pose: o.pose,
                    mesh: info.and_then(|i| i.mesh.clone()),
                    key: match info.and_then(|i| i.hash) {
                        Some(h) => PartKey::Hash(h.0),
                        None => PartKey::Doc(o.part.doc.clone(), o.part.body),
                    },
                }
            })
            .collect()
    }

    /// Every pair of occurrences whose solids meet: unintended ones as interference (warnings),
    /// the rest as intended contact by a declared fit or a fit rule. Occurrences that did not move
    /// since the last call keep their pair results; only pairs with a moved occurrence are tested.
    pub fn interference(&mut self, sol: &Solution, fits: &[Fit], rules: &[FitRule]) -> InterferenceReport {
        let occ = self.occurrences(sol);
        let mut rep = InterferenceReport::default();
        let n = occ.len();
        let boxes: Vec<Box3> = occ
            .iter()
            .map(|o| {
                o.mesh
                    .as_ref()
                    .map_or(Box3::EMPTY, |m| moved_box(&m.bounds(), &o.pose))
            })
            .collect();
        for o in occ.iter().filter(|o| o.mesh.is_none()) {
            rep.unchecked.push(o.path.clone());
        }
        // which occurrences moved since the last call (all, when the set changed)
        let prev = self.last.take();
        let same_set = prev.as_ref().is_some_and(|(p, _)| {
            p.len() == n
                && p.iter()
                    .zip(&occ)
                    .all(|(x, y)| x.path == y.path && x.part == y.part)
        });
        let moved: Vec<bool> = match (&prev, same_set) {
            (Some((p, _)), true) => p
                .iter()
                .zip(&occ)
                .map(|(x, y)| pose_bits(&x.pose) != pose_bits(&y.pose))
                .collect(),
            _ => vec![true; n],
        };
        let bvh = Bvh::new(&boxes);
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut found = Vec::new();
        for i in (0..n).filter(|&i| moved[i] && !boxes[i].is_empty()) {
            found.clear();
            bvh.query(&boxes, &boxes[i], &mut found);
            for &j in &found {
                if j != i && (!moved[j] || j > i) {
                    pairs.push((i.min(j), i.max(j)));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        rep.candidates = pairs.len();

        // narrow phase, cached by parts and relative pose
        let keyed: Vec<(Key, Frame, bool)> = pairs
            .iter()
            .map(|&(i, j)| {
                let (x, y, swap) = if occ[i].key <= occ[j].key {
                    (i, j, false)
                } else {
                    (j, i, true)
                };
                let rel = occ[x].pose.inverse().compose(&occ[y].pose);
                (
                    (occ[x].key.clone(), occ[y].key.clone(), pose_bits(&rel)),
                    rel,
                    swap,
                )
            })
            .collect();
        let todo: Vec<usize> = (0..pairs.len())
            .filter(|&k| !self.cache.contains_key(&keyed[k].0))
            .collect();
        rep.cache_hits = pairs.len() - todo.len();
        rep.narrow_tests = todo.len();
        let narrowed = par(todo.len(), self.threads, |t| {
            let k = todo[t];
            let (i, j) = pairs[k];
            let (x, y) = if keyed[k].2 { (j, i) } else { (i, j) };
            let (ma, mb) = (
                occ[x].mesh.as_ref().expect("boxed"),
                occ[y].mesh.as_ref().expect("boxed"),
            );
            let depth = ma.tolerance + mb.tolerance + tol::LINEAR;
            narrow(ma, mb, &keyed[k].1, depth)
        });
        let fit_at: BTreeMap<(&Path, &Path), usize> = fits
            .iter()
            .enumerate()
            .map(|(i, f)| (if f.a <= f.b { (&f.a, &f.b) } else { (&f.b, &f.a) }, i))
            .collect();
        let mut rules_sorted: Vec<&FitRule> = rules.iter().collect();
        rules_sorted.sort_by_key(|r| r.id);
        let intent_of = |i: usize, j: usize| -> Option<(Intent, Option<Measured>)> {
            let (a, b) = (&occ[i].path, &occ[j].path);
            if let Some(&index) = fit_at.get(&if a <= b { (a, b) } else { (b, a) }) {
                return Some((Intent::Fit { index }, None));
            }
            let tol = occ[i].mesh.as_ref().map_or(0.0, |m| m.tolerance)
                + occ[j].mesh.as_ref().map_or(0.0, |m| m.tolerance)
                + tol::LINEAR;
            rules_sorted.iter().find_map(|r| {
                rule_covers(&self.parts, r, &occ[i], &occ[j], tol).map(|s| (Intent::Rule { rule: r.id }, s))
            })
        };
        let intent = |i: usize, j: usize| intent_of(i, j).map(|x| x.0);
        // exact confirmation for hits that are not intended
        let confirm: Vec<usize> = todo
            .iter()
            .zip(&narrowed)
            .filter(|(k, r)| **r != Narrow::Clear && intent(pairs[**k].0, pairs[**k].1).is_none())
            .map(|(k, _)| *k)
            .collect();
        rep.exact_runs = confirm.len();
        let volumes = par(confirm.len(), self.threads, |t| {
            let k = confirm[t];
            let (i, j) = pairs[k];
            let (x, y) = if keyed[k].2 { (j, i) } else { (i, j) };
            self.exact.overlap_volume(&occ[x].part, &occ[y].part, &keyed[k].1)
        });
        for (k, r) in todo.iter().zip(&narrowed) {
            let vol = match confirm.iter().position(|c| c == k) {
                Some(t) => volumes[t].clone(),
                None => Err(ExactFail::NoSolid),
            };
            self.cache.insert(keyed[*k].0.clone(), (*r, vol));
        }

        let mut results: PairResults = BTreeMap::new();
        if let (Some((_, old)), true) = (&prev, same_set) {
            for (&(i, j), r) in old {
                if !moved[i] && !moved[j] {
                    results.insert((i, j), r.clone());
                    rep.reused += 1;
                }
            }
        }
        for (k, &(i, j)) in pairs.iter().enumerate() {
            let (r, vol) = self.cache[&keyed[k].0].clone();
            let hit = if r == Narrow::Clear {
                None
            } else {
                let (ma, mb) = (
                    occ[i].mesh.as_ref().expect("boxed"),
                    occ[j].mesh.as_ref().expect("boxed"),
                );
                let (a, b) = (occ[i].path.clone(), occ[j].path.clone());
                Some(match intent_of(i, j) {
                    Some((by, short)) => PairHit::Intended(IntendedContact { a, b, by }, short),
                    None => match vol {
                        Ok(v) if v <= TOUCH_VOLUME => PairHit::Touch(a, b),
                        _ => PairHit::Interference(Interference {
                            a,
                            b,
                            volume: vol.as_ref().ok().copied(),
                            confirmed: vol.is_ok(),
                            tolerance: ma.tolerance + mb.tolerance + tol::LINEAR,
                            boolean_failed: match vol {
                                Err(ExactFail::Boolean(why)) => Some(why),
                                _ => None,
                            },
                        }),
                    },
                })
            };
            results.insert((i, j), hit);
        }
        let mut by_rule: BTreeMap<u64, Vec<(Path, Path)>> = BTreeMap::new();
        for h in results.values().flatten() {
            match h {
                PairHit::Interference(x) => rep.interferences.push(x.clone()),
                PairHit::Intended(x, short) => {
                    if let Intent::Rule { rule } = x.by {
                        by_rule.entry(rule).or_default().push((x.a.clone(), x.b.clone()));
                        if let Some(m) = *short {
                            rep.engagements.push(Engagement {
                                a: x.a.clone(),
                                b: x.b.clone(),
                                rule,
                                engaged: m.engaged,
                                min: m.min,
                                short: m.short,
                            });
                            if let (true, Some(min)) = (m.short, m.min) {
                                rep.shallow.push(ShallowEngagement {
                                    a: x.a.clone(),
                                    b: x.b.clone(),
                                    rule,
                                    engaged: m.engaged,
                                    min,
                                });
                            }
                        }
                    }
                    rep.intended.push(x.clone());
                }
                PairHit::Touch(a, b) => rep.touching.push((a.clone(), b.clone())),
            }
        }
        rep.interferences.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
        rep.intended.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
        rep.touching.sort();
        rep.rules = by_rule
            .into_iter()
            .map(|(rule, mut pairs)| {
                pairs.sort();
                RuleUse { rule, pairs }
            })
            .collect();
        // forget clear pairs: only hits need carrying over
        results.retain(|_, r| r.is_some());
        self.last = Some((occ, results));
        rep
    }

    /// The least distance between two occurrences' meshes (0 when they meet), within `limit`.
    pub fn clearance(&self, sol: &Solution, a: &Path, b: &Path, limit: f64) -> Option<Clearance> {
        let occ = self.occurrences(sol);
        let find = |p: &Path| occ.iter().find(|o| &o.path == p);
        let (x, y) = (find(a)?, find(b)?);
        let (ma, mb) = (x.mesh.as_ref()?, y.mesh.as_ref()?);
        let rel = x.pose.inverse().compose(&y.pose);
        let (d, p, q) = distance(ma, mb, &rel, limit)?;
        Some(Clearance {
            distance: d,
            on_a: x.pose.point_to_world(p),
            on_b: x.pose.point_to_world(q),
            tolerance: ma.tolerance + mb.tolerance,
        })
    }

    /// Every pair of occurrences closer than `limit`, with its clearance, by path.
    pub fn clearances(&self, sol: &Solution, limit: f64) -> Vec<(Path, Path, Clearance)> {
        let occ = self.occurrences(sol);
        let boxes: Vec<Box3> = occ
            .iter()
            .map(|o| {
                o.mesh
                    .as_ref()
                    .map_or(Box3::EMPTY, |m| moved_box(&m.bounds(), &o.pose).grow(limit))
            })
            .collect();
        let bvh = Bvh::new(&boxes);
        let mut out = Vec::new();
        let mut found = Vec::new();
        for i in 0..occ.len() {
            found.clear();
            bvh.query(&boxes, &boxes[i], &mut found);
            found.sort_unstable();
            for &j in found.iter().filter(|&&j| j > i) {
                if let Some(c) = self.clearance(sol, &occ[i].path, &occ[j].path, limit) {
                    out.push((occ[i].path.clone(), occ[j].path.clone(), c));
                }
            }
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clearance {
    /// Between the meshes: within `tolerance` of the true distance.
    pub distance: f64,
    pub on_a: Point3,
    pub on_b: Point3,
    pub tolerance: f64,
}

/// One pose of a motion sweep.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepStep {
    pub value: f64,
    /// Interference at this pose (not touches, fits or rules).
    pub overlaps: Vec<Interference>,
    /// The watched pairs' clearances (None: farther than the limit).
    pub clearances: Vec<Option<f64>>,
    /// Errors of the solve at this pose (a loop that cannot reach it, a limit).
    pub errors: usize,
}

/// Drives one joint value over `range` in `steps` equal steps, re-solving and checking at each
/// pose; the joint's value goes back to where it was after.
#[allow(clippy::too_many_arguments)]
pub fn sweep<S: Parts, P: Parts, E: Exact>(
    solver: &mut crate::Solver<S>,
    checker: &mut Checker<P, E>,
    joint: crate::JointId,
    freedom: crate::Freedom,
    range: (f64, f64),
    steps: usize,
    watch: &[(Path, Path)],
    limit: f64,
) -> Vec<SweepStep> {
    let before = solver
        .assembly()
        .joints
        .iter()
        .find(|j| j.id == joint)
        .and_then(|j| j.drive.iter().find(|d| d.0 == freedom).map(|d| d.1));
    let fits = solver.assembly().fits.clone();
    let rules = solver.assembly().fit_rules.clone();
    let mut out = Vec::new();
    for i in 0..=steps {
        let v = range.0 + (range.1 - range.0) * i as f64 / steps.max(1) as f64;
        let sol = solver.set_drive(joint, freedom, v).clone();
        let rep = checker.interference(&sol, &fits, &rules);
        out.push(SweepStep {
            value: v,
            overlaps: rep.interferences.clone(),
            clearances: watch
                .iter()
                .map(|(a, b)| checker.clearance(&sol, a, b, limit).map(|c| c.distance))
                .collect(),
            errors: sol.errors().count(),
        });
    }
    if let Some(v) = before {
        solver.set_drive(joint, freedom, v);
    }
    out
}

#[cfg(test)]
mod narrow_tests {
    use super::*;

    /// An axis-aligned box mesh from `lo` to `hi`, each face split n x n (2 n^2 triangles), outward.
    fn box_mesh(lo: Point3, hi: Point3, n: usize) -> PartMesh {
        let mut pos: Vec<Point3> = Vec::new();
        let mut tris: Vec<[u32; 3]> = Vec::new();
        let size = [hi.x - lo.x, hi.y - lo.y, hi.z - lo.z];
        // (normal axis, side, u axis, v axis) with u x v along the outward normal
        for (ax, side, u, w) in [
            (0, 1.0, 1, 2),
            (0, 0.0, 2, 1),
            (1, 1.0, 2, 0),
            (1, 0.0, 0, 2),
            (2, 1.0, 0, 1),
            (2, 0.0, 1, 0),
        ] {
            let base = pos.len() as u32;
            for i in 0..=n {
                for j in 0..=n {
                    let mut c = [lo.x, lo.y, lo.z];
                    c[ax] += side * size[ax];
                    c[u] += size[u] * i as f64 / n as f64;
                    c[w] += size[w] * j as f64 / n as f64;
                    pos.push(Point3::new(c[0], c[1], c[2]));
                }
            }
            let id = |i: usize, j: usize| base + (i * (n + 1) + j) as u32;
            for i in 0..n {
                for j in 0..n {
                    tris.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
                    tris.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
                }
            }
        }
        PartMesh::new(pos, tris, 1e-6)
    }

    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }

    fn turned(o: Point3, a: f64) -> Frame {
        Frame::WORLD.rotated_about_z(a).with_origin(o)
    }

    /// The inside phase as it was: every centre in the shared region gets a parity ray.
    fn narrow_every_centre(a: &PartMesh, b: &PartMesh, rel: &Frame, depth: f64) -> Narrow {
        let pl = Placed::new(a, b, rel);
        let mut hit = false;
        pl.walk(0.0, |ta, tb| {
            hit = crosses(&a.tri(ta), a.sharp()[ta], &pl.tri_b(tb), b.sharp()[tb], depth);
            hit
        });
        if hit {
            return Narrow::Cross;
        }
        let inv = rel.inverse();
        for (m, other, map) in [(a, b, *rel), (b, a, inv)] {
            for t in 0..other.triangles.len() {
                let tri = other.tri(t);
                let c = Point3::new(
                    (tri[0].x + tri[1].x + tri[2].x) / 3.0,
                    (tri[0].y + tri[1].y + tri[2].y) / 3.0,
                    (tri[0].z + tri[1].z + tri[2].z) / 3.0,
                );
                if inside(m, map.point_to_world(c), depth) == Some(true) {
                    return Narrow::Inside;
                }
            }
        }
        Narrow::Clear
    }

    #[test]
    fn segment_visits_exactly_the_boxes_the_segment_meets() {
        let mut s = 12345u64;
        let mut r = || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let boxes: Vec<Box3> = (0..500)
            .map(|_| {
                let c = p(r() * 100.0, r() * 100.0, r() * 100.0);
                Box3::from_points([c, p(c.x + r() * 5.0, c.y + r() * 5.0, c.z + r() * 5.0)])
            })
            .collect();
        let bvh = Bvh::new(&boxes);
        for _ in 0..200 {
            let (a, b) = (
                p(r() * 120.0 - 10.0, r() * 120.0 - 10.0, r() * 120.0 - 10.0),
                p(r() * 120.0 - 10.0, r() * 120.0 - 10.0, r() * 120.0 - 10.0),
            );
            let mut got = Vec::new();
            bvh.segment(&boxes, a, b, &mut got);
            got.sort_unstable();
            // brute force: sample the segment densely against each box, and the slab test agrees
            let want: Vec<usize> = (0..boxes.len())
                .filter(|&i| {
                    let bx = boxes[i];
                    (0..=20000).any(|k| bx.contains(a + (b - a) * (k as f64 / 20000.0)))
                })
                .collect();
            for i in &want {
                assert!(got.contains(i), "segment missed box {i}");
            }
            // anything extra may only graze (no sample inside): at most a hair from the segment
            for i in got.iter().filter(|i| !want.contains(i)) {
                let bx = boxes[*i].grow(0.02 * (b - a).length() / 1000.0 + 1e-9);
                assert!(
                    (0..=20000).any(|k| bx.contains(a + (b - a) * (k as f64 / 20000.0))),
                    "box {i} far from the segment"
                );
            }
        }
    }

    #[test]
    fn pieces_join_by_shared_vertices_and_skip_dropped_triangles() {
        // box_mesh gives each face its own vertices: six pieces
        let m = box_mesh(p(0.0, 0.0, 0.0), p(1.0, 1.0, 1.0), 2);
        let all: Vec<usize> = (0..m.triangles.len()).collect();
        assert_eq!(pieces(&m, &all, &vec![true; all.len()]).len(), 6);
        // a strip of two triangles sharing an edge, and one sharing only a vertex, is one piece
        let strip = PartMesh::new(
            vec![
                p(0.0, 0.0, 0.0),
                p(1.0, 0.0, 0.0),
                p(1.0, 1.0, 0.0),
                p(0.0, 1.0, 0.0),
                p(2.0, 2.0, 0.0),
                p(2.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3], [2, 5, 4]],
            1e-6,
        );
        assert_eq!(pieces(&strip, &[0, 1, 2], &[true; 3]), vec![vec![0, 1, 2]]);
        assert_eq!(pieces(&strip, &[0, 1, 2], &[true, false, true]), vec![vec![0, 2]]);
        assert_eq!(pieces(&strip, &[0, 1, 2], &[false, true, true]), vec![vec![1, 2]]);
        // keep only top and bottom
        let n = m.triangles.len() / 6;
        let keep: Vec<bool> = (0..all.len()).map(|t| t / n >= 4).collect();
        let ps = pieces(&m, &all, &keep);
        assert_eq!(ps.len(), 2);
        assert!(ps.iter().all(|p| p.len() == n));
    }

    #[test]
    fn inside_touching_and_crossing_boxes_classify_as_before() {
        let coarse = box_mesh(p(0.0, 0.0, 0.0), p(40.0, 40.0, 40.0), 1);
        let block = box_mesh(p(0.0, 0.0, 0.0), p(10.0, 10.0, 10.0), 1);
        assert_eq!(
            inside(&coarse, p(15.0, 15.0, 15.0), 1e-6),
            Some(true),
            "coarse parity"
        );
        assert_eq!(
            narrow(&coarse, &block, &turned(p(10.0, 10.0, 10.0), 0.0), 3e-6),
            Narrow::Inside,
            "coarse"
        );
        // two coarse boxes 2 deep with four faces coplanar: no clean crossing, the end face is inside
        assert_eq!(
            narrow(&block, &block, &turned(p(8.0, 0.0, 0.0), 0.0), 3e-6),
            Narrow::Inside,
            "coplanar sides"
        );
        let big = box_mesh(p(0.0, 0.0, 0.0), p(10.0, 10.0, 10.0), 6);
        let small = box_mesh(p(0.0, 0.0, 0.0), p(2.0, 2.0, 2.0), 3);
        let d = 3e-6;
        // wholly inside
        let r = turned(p(4.0, 4.0, 4.0), 0.3);
        assert_eq!(narrow(&big, &small, &r, d), Narrow::Inside);
        assert_eq!(narrow(&small, &big, &r.inverse(), d), Narrow::Inside);
        // inside, one face on the big box's floor (coincident faces: no crossing)
        assert_eq!(
            narrow(&big, &small, &turned(p(3.0, 3.0, 0.0), 0.0), d),
            Narrow::Inside
        );
        // outside, flush on the big box's top
        assert_eq!(
            narrow(&big, &small, &turned(p(3.0, 3.0, 10.0), 0.0), d),
            Narrow::Clear
        );
        // apart
        assert_eq!(
            narrow(&big, &small, &turned(p(20.0, 3.0, 3.0), 0.0), d),
            Narrow::Clear
        );
        // half in
        assert_eq!(
            narrow(&big, &small, &turned(p(9.0, 3.0, 3.0), 0.2), d),
            Narrow::Cross
        );
        // the same answers as testing every centre, over many poses
        let mut s = 7u64;
        let mut r = || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..300 {
            let mut c = [0.0f64; 3];
            for x in c.iter_mut() {
                let v = r() * 12.0 - 2.0;
                *x = if r() < 0.3 { (v * 2.0).round() / 2.0 } else { v };
            }
            let o = p(c[0], c[1], c[2]);
            let a = if r() < 0.5 { 0.0 } else { r() * 6.0 };
            let f = turned(o, a);
            assert_eq!(
                narrow(&big, &small, &f, d),
                narrow_every_centre(&big, &small, &f, d),
                "at {o:?} turned {a}"
            );
        }
    }
}
