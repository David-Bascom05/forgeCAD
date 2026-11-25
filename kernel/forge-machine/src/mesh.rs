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
}

/// Faces per leaf.
const LEAF: usize = 4;

impl Tree {
    /// The tree over `boxes` (each `lo <= hi`, finite), with `probes` on the body.
    pub fn new(mut boxes: Vec<(V3, V3)>, probes: Vec<V3>) -> Result<Tree, String> {
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
        })
    }

    /// The root's box.
    pub fn bounds(&self) -> (V3, V3) {
        (self.nodes[0].lo, self.nodes[0].hi)
    }
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
