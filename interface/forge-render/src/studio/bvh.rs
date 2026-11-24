//! Bounding volume hierarchies for the tracer: one per part over its triangles (binned SAH,
//! Wald 2007, "On fast Construction of SAH-based Bounding Volume Hierarchies"), one over the
//! placements' boxes. Flat arrays the GPU reads as they are: a node is 32 bytes, its two children
//! side by side.

use super::v3::{V3, v3};

/// A node: its box, and either its first child (`count` 0, children at `first` and `first + 1`)
/// or its primitives `first..first + count`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Node {
    pub min: V3,
    pub first: u32,
    pub max: V3,
    pub count: u32,
}

impl Node {
    /// As the GPU's two vec4s.
    pub fn words(&self) -> [u32; 8] {
        [
            self.min.x.to_bits(),
            self.min.y.to_bits(),
            self.min.z.to_bits(),
            self.first,
            self.max.x.to_bits(),
            self.max.y.to_bits(),
            self.max.z.to_bits(),
            self.count,
        ]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: V3::splat(f32::INFINITY),
        max: V3::splat(f32::NEG_INFINITY),
    };
    pub fn grow(&mut self, p: V3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    pub fn join(&mut self, b: &Aabb) {
        self.min = self.min.min(b.min);
        self.max = self.max.max(b.max);
    }
    pub fn area(&self) -> f32 {
        let d = self.max - self.min;
        if d.x < 0.0 {
            return 0.0;
        }
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
    pub fn centre(&self) -> V3 {
        (self.min + self.max) * 0.5
    }
}

const BINS: usize = 16;
pub const LEAF: usize = 4;

/// A hierarchy over boxes: its nodes, and the boxes' order its leaves refer to.
#[derive(Clone, Debug, Default)]
pub struct Bvh {
    pub nodes: Vec<Node>,
    pub order: Vec<u32>,
}

impl Bvh {
    /// Over these boxes, leaves of at most `leaf`.
    pub fn build(boxes: &[Aabb], leaf: usize) -> Bvh {
        let n = boxes.len();
        let mut order: Vec<u32> = (0..n as u32).collect();
        let centres: Vec<V3> = boxes.iter().map(Aabb::centre).collect();
        let mut nodes = vec![Node::default(); 2];
        // the root, then an empty slot so children pairs stay aligned at even indices
        let mut stack = vec![(0usize, 0usize, n)];
        while let Some((at, lo, hi)) = stack.pop() {
            let mut bound = Aabb::EMPTY;
            let mut cb = Aabb::EMPTY;
            for &i in &order[lo..hi] {
                bound.join(&boxes[i as usize]);
                cb.grow(centres[i as usize]);
            }
            let count = hi - lo;
            let mut split = None;
            if count > leaf {
                split = best_split(boxes, &centres, &order[lo..hi], &bound, &cb);
            }
            let Some((axis, pos)) = split else {
                nodes[at] = Node {
                    min: bound.min,
                    first: lo as u32,
                    max: bound.max,
                    count: count as u32,
                };
                if count == 0 {
                    nodes[at].min = V3::ZERO;
                    nodes[at].max = V3::ZERO;
                }
                continue;
            };
            // partition in place about the plane
            let (mut i, mut j) = (lo, hi);
            while i < j {
                if centres[order[i] as usize].get(axis) < pos {
                    i += 1;
                } else {
                    j -= 1;
                    order.swap(i, j);
                }
            }
            let mid = if i == lo || i == hi { (lo + hi) / 2 } else { i };
            let left = nodes.len();
            nodes.push(Node::default());
            nodes.push(Node::default());
            nodes[at] = Node {
                min: bound.min,
                first: left as u32,
                max: bound.max,
                count: 0,
            };
            stack.push((left + 1, mid, hi));
            stack.push((left, lo, mid));
        }
        Bvh { nodes, order }
    }
}

/// The cheapest of 16 bins on each axis, by the surface area heuristic; none when a leaf is
/// cheaper.
fn best_split(boxes: &[Aabb], centres: &[V3], ids: &[u32], bound: &Aabb, cb: &Aabb) -> Option<(usize, f32)> {
    let mut best = (f32::INFINITY, 0usize, 0.0f32);
    for axis in 0..3 {
        let (lo, hi) = (cb.min.get(axis), cb.max.get(axis));
        if hi - lo <= 0.0 {
            continue;
        }
        let scale = BINS as f32 / (hi - lo);
        let mut bins = [(Aabb::EMPTY, 0u32); BINS];
        for &i in ids {
            let b = (((centres[i as usize].get(axis) - lo) * scale) as usize).min(BINS - 1);
            bins[b].0.join(&boxes[i as usize]);
            bins[b].1 += 1;
        }
        let mut right_area = [0.0f32; BINS];
        let mut right_count = [0u32; BINS];
        let mut acc = Aabb::EMPTY;
        let mut c = 0;
        for b in (1..BINS).rev() {
            acc.join(&bins[b].0);
            c += bins[b].1;
            right_area[b] = acc.area();
            right_count[b] = c;
        }
        let mut acc = Aabb::EMPTY;
        let mut c = 0;
        for b in 0..BINS - 1 {
            acc.join(&bins[b].0);
            c += bins[b].1;
            let cost = acc.area() * c as f32 + right_area[b + 1] * right_count[b + 1] as f32;
            if cost < best.0 && c > 0 && right_count[b + 1] > 0 {
                best = (cost, axis, lo + (b + 1) as f32 / scale);
            }
        }
    }
    // a leaf costs its primitives; a split one traversal step plus its sides
    let leaf_cost = ids.len() as f32 * bound.area();
    (best.0.is_finite() && best.0 < leaf_cost + bound.area()).then_some((best.1, best.2))
        .or_else(|| (ids.len() > 4 * LEAF).then(|| {
            // nothing pays but the leaf is too big: halve the longest side
            let d = cb.max - cb.min;
            let axis = if d.x >= d.y && d.x >= d.z { 0 } else if d.y >= d.z { 1 } else { 2 };
            (axis, cb.centre().get(axis))
        }))
}

/// A ray's hit on a triangle (Möller & Trumbore 1997) given as its first corner and two edges:
/// t, u, v.
pub fn hit_triangle(o: V3, d: V3, p0: V3, e1: V3, e2: V3, t_max: f32) -> Option<(f32, f32, f32)> {
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-20 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - p0;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.0 && t < t_max).then_some((t, u, v))
}

/// Where a ray enters a box, if before `t_max` (the slab test with a precomputed inverse).
pub fn hit_box(o: V3, inv: V3, min: V3, max: V3, t_max: f32) -> Option<f32> {
    let t0 = (min - o).times(inv);
    let t1 = (max - o).times(inv);
    let near = t0.min(t1).max_elem().max(0.0);
    let far = t0.max(t1);
    let far = far.x.min(far.y).min(far.z).min(t_max);
    (near <= far).then_some(near)
}

pub fn inverse(d: V3) -> V3 {
    let f = |x: f32| if x.abs() < 1e-30 { 1e30f32.copysign(x) } else { 1.0 / x };
    v3(f(d.x), f(d.y), f(d.z))
}

/// The nearest hit in a hierarchy: its distance and primitive, `hit` testing one primitive.
/// Child links in `nodes` are absolute; the walk starts at `root`.
pub fn closest(
    nodes: &[Node],
    root: u32,
    o: V3,
    d: V3,
    mut t_max: f32,
    mut hit: impl FnMut(u32, f32) -> Option<f32>,
) -> Option<(f32, u32)> {
    let inv = inverse(d);
    let mut best = None;
    let mut stack = [0u32; 64];
    let mut sp = 0;
    let mut at = root;
    let r = nodes[root as usize];
    hit_box(o, inv, r.min, r.max, t_max)?;
    loop {
        let n = nodes[at as usize];
        if n.count > 0 {
            for k in n.first..n.first + n.count {
                if let Some(t) = hit(k, t_max) {
                    t_max = t;
                    best = Some((t, k));
                }
            }
        } else {
            let (a, b) = (n.first, n.first + 1);
            let ta = hit_box(o, inv, nodes[a as usize].min, nodes[a as usize].max, t_max);
            let tb = hit_box(o, inv, nodes[b as usize].min, nodes[b as usize].max, t_max);
            match (ta, tb) {
                (Some(x), Some(y)) => {
                    let (near, far) = if x <= y { (a, b) } else { (b, a) };
                    stack[sp] = far;
                    sp += 1;
                    at = near;
                    continue;
                }
                (Some(_), None) => {
                    at = a;
                    continue;
                }
                (None, Some(_)) => {
                    at = b;
                    continue;
                }
                _ => {}
            }
        }
        if sp == 0 {
            break;
        }
        sp -= 1;
        at = stack[sp];
    }
    best
}
