//! A body's real geometry for the collision check: one box per compiled face, a bounding-volume
//! tree over them, and points on the body.
//!
//! The faces come from the shared compiled-face pipeline (forge-machine-lang / the workholding
//! compiler: exact faces, faces under a proven enclosure, or certified triangles carrying their
//! deviation mu). Each face's box is its conservative bounds grown by its mu, so the body's
//! surface lies inside the union of the boxes: the posed-interval gap between two such unions
//! (collide's own test, box against box) proves a clearance whenever it reports one, and is far
//! tighter than a record's envelope. What the boxes do not cover is a body's inside: one body
//! wholly inside another touches no surface, so collide asks, at each span's start, that a point
//! of each body lies outside the other's envelope (which holds the other's solid); when it does
//! not, that span falls back to the envelopes (conservative, never a missed contact).
use crate::kin::V3;

/// One node: its box (base frame, home pose); an inner node's two children, a leaf's range of
/// face boxes.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub lo: V3,
    pub hi: V3,
    pub kids: Option<(usize, usize)>,
    pub first: usize,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    /// The face boxes (lo, hi), in leaf order.
    pub boxes: Vec<(V3, V3)>,
    /// nodes[0] is the root.
    pub nodes: Vec<Node>,
    /// Points on the body (at least one on each of its solids), for the inside test.
    pub probes: Vec<V3>,
    /// A closed triangle mesh of the body, within the boxes' bands of its surface (each triangle
    /// inside a face box): its ray parity is the solid's at every point outside all the boxes.
    pub tris: Vec<[V3; 3]>,
}

/// Faces per leaf.
const LEAF: usize = 4;

impl Tree {
    /// The tree over `boxes` (each `lo <= hi`, finite), with `probes` on the body.
    pub fn new(mut boxes: Vec<(V3, V3)>, probes: Vec<V3>, tris: Vec<[V3; 3]>) -> Result<Tree, String> {
        if boxes.is_empty() {
            return Err("no faces".into());
        }
        if probes.is_empty() {
            return Err("no point on the body".into());
        }
        for (lo, hi) in &boxes {
            if (0..3).any(|k| !(lo[k].is_finite() && hi[k].is_finite() && lo[k] <= hi[k])) {
                return Err(format!("a face box {lo:?} {hi:?} that is not a box"));
            }
        }
        let mut nodes = Vec::with_capacity(2 * boxes.len() / LEAF + 1);
        build(&mut boxes, 0, &mut nodes);
        Ok(Tree {
            boxes,
            nodes,
            probes,
            tris,
        })
    }

    /// Whether `p` (body frame) is in a face box: near the surface, where the mesh's parity may
    /// differ from the solid's.
    pub fn near(&self, p: V3) -> bool {
        let inb = |lo: &V3, hi: &V3| (0..3).all(|k| lo[k] <= p[k] && p[k] <= hi[k]);
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let nd = &self.nodes[n];
            if !inb(&nd.lo, &nd.hi) {
                continue;
            }
            match nd.kids {
                Some((a, b)) => stack.extend([a, b]),
                None => {
                    if self.boxes[nd.first..nd.first + nd.count]
                        .iter()
                        .any(|(lo, hi)| inb(lo, hi))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether `p` is inside the solid: Some by ray parity on the mesh where `p` is away from the
    /// surface (in no face box), None where it is near it or every ray tried grazes an edge.
    pub fn inside(&self, p: V3) -> Option<bool> {
        if self.tris.is_empty() || self.near(p) {
            return None;
        }
        // directions with no simple ratio to the axes or each other (a ray along an edge or a
        // face is unlikely; parity needs no unit length)
        let dirs = [
            [0.613, 0.5791, 0.5372],
            [0.2673, 0.5345, 0.8018],
            [-0.6911, 0.4083, 0.5974],
        ];
        'dir: for d in dirs {
            let mut n = 0u32;
            for t in &self.tris {
                match crossing(p, d, t) {
                    Cross::No => {}
                    Cross::Yes => n += 1,
                    Cross::Edge => continue 'dir,
                }
            }
            return Some(n % 2 == 1);
        }
        None
    }
}

enum Cross {
    No,
    Yes,
    /// Too close to an edge or the plane to call.
    Edge,
}

/// Whether the ray p + t d (t > 0) crosses triangle `t` (Möller-Trumbore), with a margin.
fn crossing(p: V3, d: V3, t: &[V3; 3]) -> Cross {
    use crate::kin::{cross, dot, sub};
    const EPS: f64 = 1e-9;
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let h = cross(d, e2);
    let a = dot(e1, h);
    let scale = dot(e1, e1).max(dot(e2, e2));
    if a.abs() <= EPS * scale {
        return Cross::No; // parallel to the plane: a ray in a triangle's plane misses its interior
    }
    let f = 1.0 / a;
    let s = sub(p, t[0]);
    let u = f * dot(s, h);
    let q = cross(s, e1);
    let v = f * dot(d, q);
    let w = f * dot(e2, q);
    if u < -EPS || v < -EPS || u + v > 1.0 + EPS || w < 0.0 {
        return Cross::No;
    }
    if u < EPS || v < EPS || u + v > 1.0 - EPS || w < EPS {
        return Cross::Edge;
    }
    Cross::Yes
}

fn bounds(b: &[(V3, V3)]) -> (V3, V3) {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for (l, h) in b {
        for k in 0..3 {
            lo[k] = lo[k].min(l[k]);
            hi[k] = hi[k].max(h[k]);
        }
    }
    (lo, hi)
}

/// Builds the subtree over `b` (whose first box is at `offset` in the final order); returns its
/// node's index.
fn build(b: &mut [(V3, V3)], offset: usize, nodes: &mut Vec<Node>) -> usize {
    let (lo, hi) = bounds(b);
    let me = nodes.len();
    nodes.push(Node {
        lo,
        hi,
        kids: None,
        first: offset,
        count: b.len(),
    });
    if b.len() <= LEAF {
        return me;
    }
    // split at the median centre along the widest axis
    let k = (0..3)
        .max_by(|&x, &y| (hi[x] - lo[x]).total_cmp(&(hi[y] - lo[y])))
        .unwrap_or(0);
    let mid = b.len() / 2;
    b.select_nth_unstable_by(mid, |p, q| (p.0[k] + p.1[k]).total_cmp(&(q.0[k] + q.1[k])));
    let (l, r) = b.split_at_mut(mid);
    let a = build(l, offset, nodes);
    let c = build(r, offset + mid, nodes);
    nodes[me].kids = Some((a, c));
    me
}
