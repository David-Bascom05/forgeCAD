//! A part's thinnest wall: from each triangle, straight in against its normal to the next
//! surface the ray leaves the solid through. `forge_agent::measure::wall` tries every triangle
//! against every other (minutes on a welded frame: 165k triangles); here each ray tries only the
//! triangles whose boxes it passes through before the thinnest wall so far, with the same tests
//! in the same order, so the answer is the same to the bit (its tests hold it to that one).
//!
//! Its own crate, not forge-agent's: forge-lang links forge-agent, and new code there moved how
//! the language's binaries are compiled (a few hundred instructions on the speed gate's lines);
//! only the app and the CLI link this.

use forge_agent::select;
use forge_math::{Point3, Vec3};
use forge_mesh::TriMesh;
use forge_topo::Body;

/// The thinnest wall: from each triangle, straight in against its normal to the next surface
/// the ray leaves the solid through. The thickness and where it is.
pub fn wall(body: &Body, m: &TriMesh) -> Option<(f64, Point3)> {
    let mut w = WallSearch::new(body, m);
    while !w.step(usize::MAX) {}
    w.found()
}

/// `wall`, a few triangles at a time: a caller that may not block for it all (a page's one
/// thread) steps it between frames, shows how far it is, and drops it to cancel. The same rays
/// in the same order as `wall`, so the same answer.
pub struct WallSearch<M: std::borrow::Borrow<TriMesh>> {
    m: M,
    /// Each face's faces met at an edge: a wedge there, not a wall.
    touching: std::collections::HashMap<u32, Vec<u32>>,
    normals: Vec<Option<Vec3>>,
    tree: Tree,
    best: Option<(f64, Point3)>,
    /// The next triangle's ray.
    next: usize,
    near: Vec<u32>,
    stack: Vec<usize>,
    seen: Vec<u32>,
}

impl<M: std::borrow::Borrow<TriMesh>> WallSearch<M> {
    pub fn new(body: &Body, m: M) -> Self {
        // faces meeting at an edge make a wedge there, not a wall
        let mut touching: std::collections::HashMap<u32, Vec<u32>> = Default::default();
        for e in body.edge_ids() {
            let fs = select::faces_of_edge(body, e);
            for &a in &fs {
                touching.entry(a.0).or_default().extend(fs.iter().map(|b| b.0));
            }
        }
        for t in touching.values_mut() {
            t.sort_unstable();
            t.dedup();
        }
        let mesh = m.borrow();
        let tri = |t: &[u32; 3]| t.map(|i| mesh.positions[i as usize]);
        let normals: Vec<Option<Vec3>> = mesh
            .triangles
            .iter()
            .map(|t| mesh.triangle_normal(*t).normalize().map(|n| n.get()))
            .collect();
        // each ray tries only the triangles whose boxes it passes through before the thinnest
        // wall so far (every triangle against every other took minutes on a welded frame); the
        // same tests on those, so the same answer: a triangle left out could not have been nearer
        let tree = Tree::new(&mesh.triangles.iter().map(tri).collect::<Vec<_>>());
        let seen = vec![u32::MAX; mesh.triangles.len()];
        WallSearch {
            m,
            touching,
            normals,
            tree,
            best: None,
            next: 0,
            near: vec![],
            stack: vec![],
            seen,
        }
    }

    /// The next `rays` triangles' rays (at most); whether every one is done.
    pub fn step(&mut self, rays: usize) -> bool {
        let m = self.m.borrow();
        let tri = |t: &[u32; 3]| t.map(|i| m.positions[i as usize]);
        let end = self.next.saturating_add(rays).min(m.triangles.len());
        for i in self.next..end {
            let Some(n) = self.normals[i] else { continue };
            let [a, b, c] = tri(&m.triangles[i]);
            let o = a + ((b - a) + (c - a)) * (1.0 / 3.0);
            let d = -n;
            let wedge = self.touching.get(&m.face_of[i]).map_or(&[][..], |t| &t[..]);
            self.tree.along(
                (o, d, self.best.map(|b| b.0)),
                (&mut self.near, &mut self.stack),
                (&mut self.seen, i as u32),
            );
            for &j in &self.near {
                let j = j as usize;
                // only the far side: a face the ray leaves through looks along it
                if j == i
                    || self.normals[j].is_none_or(|nj| nj.dot(d) <= 1e-9)
                    || wedge.binary_search(&m.face_of[j]).is_ok()
                {
                    continue;
                }
                if let Some(s) = hit(o, d, tri(&m.triangles[j]))
                    && s > 1e-7
                    && self.best.is_none_or(|b| s < b.0)
                {
                    self.best = Some((s, o + d * (s / 2.0)));
                }
            }
        }
        self.next = end;
        self.next == m.triangles.len()
    }

    /// How far it is, 0 to 1.
    pub fn done(&self) -> f32 {
        let n = self.m.borrow().triangles.len();
        if n == 0 { 1.0 } else { self.next as f32 / n as f32 }
    }

    /// The thinnest wall so far: the answer once every ray is done.
    pub fn found(&self) -> Option<(f64, Point3)> {
        self.best
    }
}

/// Triangles' boxes in a tree, to find those a ray may hit.
struct Tree {
    /// (box min, box max, first, count): a leaf's triangles (a long one's pieces each name it)
    /// are `order[first..first + count]`;
    /// an inner node's children are the next node and node `first`.
    nodes: Vec<([f64; 3], [f64; 3], u32, u32)>,
    order: Vec<u32>,
}

impl Tree {
    const LEAF: usize = 4;

    fn new(tris: &[[Point3; 3]]) -> Tree {
        let extent = |lo: [f64; 3], hi: [f64; 3]| (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max);
        let box_of = |t: &[Point3; 3]| {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in t {
                for (k, x) in [p.x, p.y, p.z].into_iter().enumerate() {
                    lo[k] = lo[k].min(x);
                    hi[k] = hi[k].max(x);
                }
            }
            (lo, hi)
        };
        // a long sliver's box (a member's side, on the slant) lies across every ray near it: such
        // a triangle goes in as pieces no longer than some typical triangles, each its own box
        let mut sizes: Vec<f64> = tris
            .iter()
            .map(|t| {
                let (lo, hi) = box_of(t);
                extent(lo, hi)
            })
            .collect();
        let longest = if sizes.is_empty() {
            f64::INFINITY
        } else {
            let mid = sizes.len() / 2;
            16.0 * *sizes.select_nth_unstable_by(mid, f64::total_cmp).1
        };
        let mut boxes: Vec<([f64; 3], [f64; 3])> = vec![];
        let mut of: Vec<u32> = vec![];
        let mut pieces = vec![];
        for (k, t) in tris.iter().enumerate() {
            let (lo, hi) = box_of(t);
            // a little wider than the triangle: `hit` takes points just outside its edges
            let pad: [f64; 3] = std::array::from_fn(|x| 1e-6 * (hi[x] - lo[x]).abs().max(1.0));
            pieces.clear();
            pieces.push((*t, 0));
            while let Some((p, depth)) = pieces.pop() {
                let (lo, hi) = box_of(&p);
                if depth < 8 && extent(lo, hi) > longest {
                    // halved across its longest side
                    let side = (0..3)
                        .max_by(|&a, &b| {
                            let l = |e: usize| (p[(e + 1) % 3] - p[e]).length();
                            l(a).total_cmp(&l(b))
                        })
                        .unwrap_or(0);
                    let (a, b, c) = (p[side], p[(side + 1) % 3], p[(side + 2) % 3]);
                    let m = a + (b - a) * 0.5;
                    pieces.push(([a, m, c], depth + 1));
                    pieces.push(([m, b, c], depth + 1));
                    continue;
                }
                boxes.push((
                    std::array::from_fn(|x| lo[x] - pad[x]),
                    std::array::from_fn(|x| hi[x] + pad[x]),
                ));
                of.push(k as u32);
            }
        }
        let mut t = Tree {
            nodes: Vec::with_capacity(2 * boxes.len() / Self::LEAF + 1),
            order: (0..boxes.len() as u32).collect(),
        };
        if !boxes.is_empty() {
            t.split(&boxes, 0, boxes.len());
        }
        // the leaves name triangles, not pieces
        for k in &mut t.order {
            *k = of[*k as usize];
        }
        t
    }

    fn split(&mut self, boxes: &[([f64; 3], [f64; 3])], from: usize, to: usize) {
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for &k in &self.order[from..to] {
            let (a, b) = boxes[k as usize];
            for x in 0..3 {
                lo[x] = lo[x].min(a[x]);
                hi[x] = hi[x].max(b[x]);
            }
        }
        let me = self.nodes.len();
        self.nodes.push((lo, hi, from as u32, (to - from) as u32));
        if to - from <= Self::LEAF {
            return;
        }
        let axis = (0..3)
            .max_by(|&a, &b| (hi[a] - lo[a]).total_cmp(&(hi[b] - lo[b])))
            .unwrap_or(0);
        let mid = (from + to) / 2;
        let centre = |k: u32| boxes[k as usize].0[axis] + boxes[k as usize].1[axis];
        self.order[from..to].select_nth_unstable_by(mid - from, |&a, &b| centre(a).total_cmp(&centre(b)));
        self.split(boxes, from, mid);
        let second = self.nodes.len() as u32;
        self.split(boxes, mid, to);
        self.nodes[me].2 = second;
        self.nodes[me].3 = 0;
    }

    /// Into `out`, the triangles whose boxes the ray from `o` along `d` meets, no further than
    /// `upto` (with a margin) when given; each once (`seen` marks those taken for ray `ray`).
    fn along(
        &self,
        (o, d, upto): (Point3, Vec3, Option<f64>),
        (out, stack): (&mut Vec<u32>, &mut Vec<usize>),
        (seen, ray): (&mut [u32], u32),
    ) {
        out.clear();
        if self.nodes.is_empty() {
            return;
        }
        let far = upto.map_or(f64::INFINITY, |s| s * (1.0 + 1e-9) + 1e-9);
        // plain arrays: the vectors' indexing was a third of the time, not inlined
        let (o, d) = ([o.x, o.y, o.z], [d.x, d.y, d.z]);
        let inv = d.map(|x| 1.0 / x);
        let meets = |lo: &[f64; 3], hi: &[f64; 3]| {
            let (mut t0, mut t1) = (0.0f64, far);
            for k in 0..3 {
                if d[k].abs() < 1e-300 {
                    if o[k] < lo[k] || o[k] > hi[k] {
                        return false;
                    }
                    continue;
                }
                let (a, b) = ((lo[k] - o[k]) * inv[k], (hi[k] - o[k]) * inv[k]);
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                t0 = t0.max(a);
                t1 = t1.min(b);
                if t0 > t1 {
                    return false;
                }
            }
            true
        };
        stack.clear();
        stack.push(0);
        while let Some(n) = stack.pop() {
            let (lo, hi, first, count) = &self.nodes[n];
            if !meets(lo, hi) {
                continue;
            }
            if *count > 0 {
                // a triangle met in more than one of its pieces, taken once
                for &k in &self.order[*first as usize..(*first + *count) as usize] {
                    if seen[k as usize] != ray {
                        seen[k as usize] = ray;
                        out.push(k);
                    }
                }
            } else {
                stack.push(*first as usize);
                stack.push(n + 1);
            }
        }
    }
}

/// `forge_agent::measure`'s ray test, the same arithmetic (the answers are compared to the bit).
fn hit(o: Point3, d: Vec3, [a, b, c]: [Point3; 3]) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let tv = o - a;
    let u = tv.dot(p) / det;
    let q = tv.cross(e1);
    let v = d.dot(q) / det;
    if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    Some(e2.dot(q) / det)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::{Frame, Point2};
    use forge_ops::{Profile, extrude};

    fn bits(w: Option<(f64, Point3)>) -> Option<[u64; 4]> {
        w.map(|(t, p)| [t.to_bits(), p.x.to_bits(), p.y.to_bits(), p.z.to_bits()])
    }

    #[test]
    fn the_tree_finds_the_wall_every_pair_does_bit_for_bit() {
        let parts = [
            // a plate with two holes: its 6 mm thickness
            Profile {
                plane: Frame::WORLD,
                loops: vec![
                    Profile::rectangle(Point2::new(0.0, 0.0), 60.0, 40.0, 0),
                    Profile::circle(Point2::new(15.0, 20.0), 2.5, 100),
                    Profile::circle(Point2::new(45.0, 20.0), 4.0, 200),
                ],
            },
            // a tube: its 1.5 mm wall between round faces
            Profile {
                plane: Frame::WORLD,
                loops: vec![
                    Profile::circle(Point2::new(0.0, 0.0), 10.0, 0),
                    Profile::circle(Point2::new(0.0, 0.0), 8.5, 100),
                ],
            },
        ];
        for (k, p) in parts.iter().enumerate() {
            let b = extrude(p, 6.0, 1).expect("a part");
            let m = forge_mesh::tessellate(&b, &forge_mesh::Quality::default()).expect("a mesh");
            let (fast, slow) = (wall(&b, &m), forge_agent::measure::wall(&b, &m));
            assert!(fast.is_some(), "part {k}: no wall");
            assert_eq!(bits(fast), bits(slow), "part {k}: {fast:?} vs {slow:?}");
            // a few rays at a time, as a page steps it between frames: the same
            let mut w = WallSearch::new(&b, &m);
            while !w.step(7) {}
            assert_eq!(bits(w.found()), bits(slow), "part {k}, stepped");
        }
    }
}
