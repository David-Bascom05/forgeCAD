//! Constrained Delaunay triangulation of polygonal regions (outer boundaries
//! and holes) with optional interior points. Points are inserted one by one in
//! input order with Lawson flips, constraint edges are recovered by flipping
//! the edges they cross, and triangles outside the region are dropped by an
//! even-odd flood fill across constraint edges. All decisions use the exact
//! predicates, so the result depends only on the input.

use crate::predicates::{certainly_in_circle, orient2d};
use forge_math::Point2;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub struct Triangulation {
    /// The input points, then the interior points that were kept.
    pub points: Vec<Point2>,
    /// Counter-clockwise.
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CdtError {
    NonFinite {
        point: u32,
    },
    BadIndex {
        index: u32,
    },
    /// A loop with fewer than three points, or the same point twice in a row.
    BadLoop {
        loop_index: usize,
    },
    DuplicatePoint {
        a: u32,
        b: u32,
    },
    /// Two constraint edges cross, overlap or repeat.
    ConstraintsIntersect {
        a: u32,
        b: u32,
    },
    /// The constraint edge a-b passes exactly through another point.
    ConstraintThroughPoint {
        a: u32,
        b: u32,
        point: u32,
    },
    /// An invariant broke; a bug, not bad input.
    Internal(&'static str),
}

const NONE: u32 = u32::MAX;
const SUPER: u32 = 3;

#[derive(Clone, Copy, Debug)]
struct Tri {
    v: [u32; 3],
    /// n[i] is the neighbour across the edge opposite v[i].
    n: [u32; 3],
    /// c[i] marks the edge opposite v[i] as a constraint.
    c: [bool; 3],
}

struct Mesh {
    pts: Vec<Point2>,
    tris: Vec<Tri>,
    /// Some triangle using each vertex.
    vtri: Vec<u32>,
    last: u32,
    /// Per triangle, inside the region; filled once the boundary is in, then
    /// kept up to date as interior points split triangles (a split inherits it;
    /// flips never cross constraints, so never mix inside with outside).
    inside: Vec<bool>,
    /// Reused work list for legalize, so inserting a point allocates nothing.
    stack: Vec<(u32, usize)>,
}

enum Location {
    Inside(u32),
    OnEdge(u32, usize),
    Vertex(u32),
}

fn nx(i: usize) -> usize {
    (i + 1) % 3
}
fn pv(i: usize) -> usize {
    (i + 2) % 3
}

impl Mesh {
    fn p(&self, v: u32) -> Point2 {
        self.pts[v as usize]
    }

    fn write(&mut self, t: u32, tri: Tri) {
        for &v in &tri.v {
            self.vtri[v as usize] = t;
        }
        self.tris[t as usize] = tri;
    }

    fn relink(&mut self, n: u32, old: u32, new: u32) {
        if n == NONE {
            return;
        }
        let tri = &mut self.tris[n as usize];
        for k in 0..3 {
            if tri.n[k] == old {
                tri.n[k] = new;
                return;
            }
        }
    }

    /// None only if the point is outside every triangle, which the enclosing super-triangle rules out.
    fn locate(&mut self, p: Point2) -> Option<Location> {
        let mut t = self.last;
        let limit = 4 * self.tris.len() + 64;
        for step in 0..limit {
            let tri = self.tris[t as usize];
            let mut moved = false;
            for k in 0..3 {
                let i = (k + step) % 3;
                let (a, b) = (self.p(tri.v[nx(i)]), self.p(tri.v[pv(i)]));
                if orient2d(a, b, p) < 0.0 && tri.n[i] != NONE {
                    t = tri.n[i];
                    moved = true;
                    break;
                }
            }
            if !moved {
                return Some(self.classify(t, p));
            }
        }
        // The visibility walk can cycle in a constrained triangulation; fall back to a scan.
        for t in 0..self.tris.len() as u32 {
            let tri = self.tris[t as usize];
            if (0..3).all(|i| orient2d(self.p(tri.v[nx(i)]), self.p(tri.v[pv(i)]), p) >= 0.0) {
                return Some(self.classify(t, p));
            }
        }
        None
    }

    fn classify(&self, t: u32, p: Point2) -> Location {
        let tri = self.tris[t as usize];
        for &v in &tri.v {
            if self.p(v) == p {
                return Location::Vertex(v);
            }
        }
        for i in 0..3 {
            if orient2d(self.p(tri.v[nx(i)]), self.p(tri.v[pv(i)]), p) == 0.0 {
                return Location::OnEdge(t, i);
            }
        }
        Location::Inside(t)
    }

    fn insert(&mut self, v: u32, loc: Location) {
        match loc {
            Location::Inside(t) => self.split_inside(t, v),
            Location::OnEdge(t, i) => self.split_edge(t, i, v),
            Location::Vertex(_) => unreachable!("caller rejects duplicates"),
        }
    }

    fn split_inside(&mut self, t: u32, p: u32) {
        let Tri {
            v: [a, b, c],
            n: [na, nb, nc],
            c: [ca, cb, cc],
        } = self.tris[t as usize];
        let t1 = self.tris.len() as u32;
        let t2 = t1 + 1;
        self.tris.push(self.tris[t as usize]);
        self.tris.push(self.tris[t as usize]);
        if !self.inside.is_empty() {
            let f = self.inside[t as usize];
            self.inside.extend([f, f]);
        }
        self.write(
            t,
            Tri {
                v: [p, b, c],
                n: [na, t1, t2],
                c: [ca, false, false],
            },
        );
        self.write(
            t1,
            Tri {
                v: [p, c, a],
                n: [nb, t2, t],
                c: [cb, false, false],
            },
        );
        self.write(
            t2,
            Tri {
                v: [p, a, b],
                n: [nc, t, t1],
                c: [cc, false, false],
            },
        );
        self.relink(nb, t, t1);
        self.relink(nc, t, t2);
        self.last = t;
        self.legalize(&[(t, 0), (t1, 0), (t2, 0)]);
    }

    fn split_edge(&mut self, t: u32, i: usize, p: u32) {
        let tt = self.tris[t as usize];
        let (a, b, c) = (tt.v[i], tt.v[nx(i)], tt.v[pv(i)]);
        let (n_ca, n_ab) = (tt.n[nx(i)], tt.n[pv(i)]);
        let (c_ca, c_ab, c_bc) = (tt.c[nx(i)], tt.c[pv(i)], tt.c[i]);
        let u = tt.n[i];
        let uu = self.tris[u as usize];
        let j = (0..3).find(|&k| uu.n[k] == t).expect("neighbour links back");
        let d = uu.v[j];
        let (n_bd, n_dc) = (uu.n[nx(j)], uu.n[pv(j)]);
        let (c_bd, c_dc) = (uu.c[nx(j)], uu.c[pv(j)]);
        let t1 = self.tris.len() as u32;
        let u1 = t1 + 1;
        self.tris.push(tt);
        self.tris.push(tt);
        if !self.inside.is_empty() {
            let (ft, fu) = (self.inside[t as usize], self.inside[u as usize]);
            self.inside.extend([ft, fu]);
        }
        self.write(
            t,
            Tri {
                v: [a, b, p],
                n: [u1, t1, n_ab],
                c: [c_bc, false, c_ab],
            },
        );
        self.write(
            t1,
            Tri {
                v: [a, p, c],
                n: [u, n_ca, t],
                c: [c_bc, c_ca, false],
            },
        );
        self.write(
            u,
            Tri {
                v: [d, c, p],
                n: [t1, u1, n_dc],
                c: [c_bc, false, c_dc],
            },
        );
        self.write(
            u1,
            Tri {
                v: [d, p, b],
                n: [t, n_bd, u],
                c: [c_bc, c_bd, false],
            },
        );
        self.relink(n_ca, t, t1);
        self.relink(n_bd, u, u1);
        self.last = t;
        self.legalize(&[(t, 2), (t1, 1), (u, 2), (u1, 1)]);
    }

    /// Flips the edge opposite v[i] of t. Afterwards t = [v[i], a, q] and the
    /// neighbour u = [v[i], q, b], where a-b was the old edge and q the far vertex.
    fn flip(&mut self, t: u32, i: usize) -> u32 {
        let tt = self.tris[t as usize];
        let (o, a, b) = (tt.v[i], tt.v[nx(i)], tt.v[pv(i)]);
        let u = tt.n[i];
        let uu = self.tris[u as usize];
        let j = (0..3).find(|&k| uu.n[k] == t).expect("neighbour links back");
        let q = uu.v[j];
        let (n_bo, n_oa) = (tt.n[nx(i)], tt.n[pv(i)]);
        let (c_bo, c_oa) = (tt.c[nx(i)], tt.c[pv(i)]);
        let (n_aq, n_qb) = (uu.n[nx(j)], uu.n[pv(j)]);
        let (c_aq, c_qb) = (uu.c[nx(j)], uu.c[pv(j)]);
        self.write(
            t,
            Tri {
                v: [o, a, q],
                n: [n_aq, u, n_oa],
                c: [c_aq, false, c_oa],
            },
        );
        self.write(
            u,
            Tri {
                v: [o, q, b],
                n: [n_qb, n_bo, t],
                c: [c_qb, c_bo, false],
            },
        );
        self.relink(n_aq, u, t);
        self.relink(n_bo, t, u);
        u
    }

    fn legalize(&mut self, start: &[(u32, usize)]) {
        let mut stack = std::mem::take(&mut self.stack);
        stack.clear();
        stack.extend_from_slice(start);
        while let Some((t, i)) = stack.pop() {
            let tt = self.tris[t as usize];
            let u = tt.n[i];
            if u == NONE || tt.c[i] {
                continue;
            }
            let uu = self.tris[u as usize];
            let Some(j) = (0..3).find(|&k| uu.n[k] == t) else {
                continue;
            };
            let q = uu.v[j];
            if certainly_in_circle(self.p(tt.v[0]), self.p(tt.v[1]), self.p(tt.v[2]), self.p(q)) {
                let u = self.flip(t, i);
                stack.push((t, 0));
                stack.push((u, 0));
            }
        }
        self.stack = stack;
    }

    /// A triangle holding the directed edge a -> b, and the index of the vertex opposite it.
    fn find_edge(&self, a: u32, b: u32) -> Option<(u32, usize)> {
        let start = self.vtri[a as usize];
        let mut t = start;
        for _ in 0..self.tris.len() {
            let tt = self.tris[t as usize];
            let ia = (0..3).find(|&k| tt.v[k] == a)?;
            if tt.v[nx(ia)] == b {
                return Some((t, pv(ia)));
            }
            // next triangle counter-clockwise around a: across the edge a -> v[pv]
            t = tt.n[nx(ia)];
            if t == NONE || t == start {
                return None;
            }
        }
        None
    }

    fn has_edge(&self, a: u32, b: u32) -> Option<(u32, usize)> {
        self.find_edge(a, b).or_else(|| self.find_edge(b, a))
    }

    fn insert_constraint(&mut self, a: u32, b: u32, ext: impl Fn(u32) -> u32) -> Result<(), CdtError> {
        let err_cross = || CdtError::ConstraintsIntersect { a: ext(a), b: ext(b) };
        if let Some((t, i)) = self.has_edge(a, b) {
            return self.mark(t, i, err_cross);
        }
        let (pa, pb) = (self.p(a), self.p(b));

        // the triangle around a whose far edge the segment leaves through
        let start = self.vtri[a as usize];
        let mut t = start;
        let mut crossed: VecDeque<(u32, u32)> = VecDeque::new();
        let mut cur: Option<(u32, u32, u32)> = None;
        for _ in 0..=self.tris.len() {
            let tt = self.tris[t as usize];
            let ia = (0..3)
                .find(|&k| tt.v[k] == a)
                .ok_or(CdtError::Internal("fan lost a"))?;
            let (v1, v2) = (tt.v[nx(ia)], tt.v[pv(ia)]);
            let o1 = orient2d(pa, pb, self.p(v1));
            let o2 = orient2d(pa, pb, self.p(v2));
            for (o, v) in [(o1, v1), (o2, v2)] {
                if o == 0.0 && (self.p(v) - pa).dot(pb - pa) > 0.0 {
                    return Err(CdtError::ConstraintThroughPoint {
                        a: ext(a),
                        b: ext(b),
                        point: ext(v),
                    });
                }
            }
            if o1 < 0.0 && o2 > 0.0 {
                cur = Some((t, v2, v1));
                break;
            }
            t = tt.n[nx(ia)];
            if t == start {
                break;
            }
        }
        let Some((mut t, mut left, mut right)) = cur else {
            return Err(CdtError::Internal("no start wedge"));
        };

        loop {
            let (tc, ic) = self
                .find_edge(right, left)
                .ok_or(CdtError::Internal("walk lost edge"))?;
            debug_assert_eq!(tc, t);
            if self.tris[tc as usize].c[ic] {
                return Err(err_cross());
            }
            crossed.push_back((left, right));
            let u = self.tris[tc as usize].n[ic];
            let uu = self.tris[u as usize];
            let j = (0..3)
                .find(|&k| uu.n[k] == tc)
                .ok_or(CdtError::Internal("broken link"))?;
            let q = uu.v[j];
            if q == b {
                break;
            }
            let oq = orient2d(pa, pb, self.p(q));
            if oq == 0.0 {
                return Err(CdtError::ConstraintThroughPoint {
                    a: ext(a),
                    b: ext(b),
                    point: ext(q),
                });
            }
            if oq > 0.0 {
                left = q;
            } else {
                right = q;
            }
            t = u;
        }

        let mut fresh: Vec<(u32, u32)> = Vec::new();
        let mut budget = 64 * (crossed.len() + 4) * (crossed.len() + 4);
        while let Some((x, y)) = crossed.pop_front() {
            budget = budget
                .checked_sub(1)
                .ok_or(CdtError::Internal("constraint recovery did not converge"))?;
            let (t, i) = self
                .find_edge(x, y)
                .ok_or(CdtError::Internal("crossed edge vanished"))?;
            let tt = self.tris[t as usize];
            let u = tt.n[i];
            let uu = self.tris[u as usize];
            let j = (0..3)
                .find(|&k| uu.n[k] == t)
                .ok_or(CdtError::Internal("broken link"))?;
            let (p, q) = (tt.v[i], uu.v[j]);
            let (pp, pq) = (self.p(p), self.p(q));
            let convex = {
                let s1 = orient2d(pp, pq, self.p(x));
                let s2 = orient2d(pp, pq, self.p(y));
                (s1 > 0.0 && s2 < 0.0) || (s1 < 0.0 && s2 > 0.0)
            };
            if !convex {
                crossed.push_back((x, y));
                continue;
            }
            self.flip(t, i);
            let still_crosses = p != a && p != b && q != a && q != b && {
                let s1 = orient2d(pa, pb, pp);
                let s2 = orient2d(pa, pb, pq);
                (s1 > 0.0 && s2 < 0.0) || (s1 < 0.0 && s2 > 0.0)
            };
            if still_crosses {
                crossed.push_back((p, q));
            } else {
                fresh.push((p, q));
            }
        }

        let (t, i) = self
            .has_edge(a, b)
            .ok_or(CdtError::Internal("constraint not recovered"))?;
        self.mark(t, i, err_cross)?;

        // restore the Delaunay property around the new edges
        let mut changed = true;
        let mut rounds = 0;
        while changed && rounds < 64 {
            changed = false;
            rounds += 1;
            for edge in fresh.iter_mut() {
                let (p, q) = *edge;
                let Some((t, i)) = self.has_edge(p, q) else {
                    continue;
                };
                let tt = self.tris[t as usize];
                if tt.c[i] {
                    continue;
                }
                let u = tt.n[i];
                let uu = self.tris[u as usize];
                let Some(j) = (0..3).find(|&k| uu.n[k] == t) else {
                    continue;
                };
                let r = uu.v[j];
                if certainly_in_circle(self.p(tt.v[0]), self.p(tt.v[1]), self.p(tt.v[2]), self.p(r)) {
                    let o = tt.v[i];
                    self.flip(t, i);
                    *edge = (o, r);
                    changed = true;
                }
            }
        }
        Ok(())
    }

    fn mark(&mut self, t: u32, i: usize, err: impl Fn() -> CdtError) -> Result<(), CdtError> {
        if self.tris[t as usize].c[i] {
            return Err(err());
        }
        self.tris[t as usize].c[i] = true;
        let u = self.tris[t as usize].n[i];
        if u != NONE {
            let k = (0..3)
                .find(|&k| self.tris[u as usize].n[k] == t)
                .ok_or(CdtError::Internal("broken link"))?;
            self.tris[u as usize].c[k] = true;
        }
        Ok(())
    }
}

/// Insertion order for incremental Delaunay: a fixed-seed shuffle split into
/// rounds that double in size, each round sorted along a Z-order curve. The
/// shuffle keeps the expected flips per insertion constant (points arriving in
/// order along a line otherwise flip the whole fan each time); the curve keeps
/// consecutive points close, so the location walk stays short.
fn brio_order(points: &[Point2], lo: Point2, size: f64) -> Vec<usize> {
    let n = points.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut rng = forge_math::rng::Rng::new(0x5eed_cd71);
    for i in (1..n).rev() {
        order.swap(i, rng.below(i + 1));
    }
    let scale = 65535.0 / size;
    let q = |v: f64| (v * scale).clamp(0.0, 65535.0) as u32;
    let keys: Vec<u64> = points
        .iter()
        .map(|p| morton(q(p.x - lo.x), q(p.y - lo.y)))
        .collect();
    let mut start = 0;
    let mut round = 16usize;
    while start < n {
        let end = (start + round).min(n);
        // the last round takes everything left so rounds keep doubling
        let end = if n - end < round { n } else { end };
        order[start..end].sort_unstable_by_key(|&i| (keys[i], i));
        start = end;
        round *= 2;
    }
    order
}

/// Position along a Z-order (Morton) curve: the bits of x and y interleaved.
#[inline]
fn morton(x: u32, y: u32) -> u64 {
    fn spread(v: u32) -> u64 {
        let mut v = u64::from(v & 0xffff);
        v = (v | (v << 8)) & 0x00ff_00ff;
        v = (v | (v << 4)) & 0x0f0f_0f0f;
        v = (v | (v << 2)) & 0x3333_3333;
        (v | (v << 1)) & 0x5555_5555
    }
    spread(x) | (spread(y) << 1)
}

fn segment_distance_sq(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l = ab.length_sq();
    let t = if l > 0.0 {
        ((p - a).dot(ab) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length_sq()
}

pub fn triangulate(
    points: &[Point2],
    loops: &[Vec<u32>],
    interior: &[Point2],
) -> Result<Triangulation, CdtError> {
    let n = points.len() as u32;
    for (i, p) in points.iter().enumerate() {
        if !(p.x.is_finite() && p.y.is_finite()) {
            return Err(CdtError::NonFinite { point: i as u32 });
        }
    }
    for (li, lp) in loops.iter().enumerate() {
        if lp.len() < 3 {
            return Err(CdtError::BadLoop { loop_index: li });
        }
        for k in 0..lp.len() {
            if lp[k] >= n {
                return Err(CdtError::BadIndex { index: lp[k] });
            }
            if lp[k] == lp[(k + 1) % lp.len()] {
                return Err(CdtError::BadLoop { loop_index: li });
            }
        }
    }

    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    if points.is_empty() {
        return Ok(Triangulation {
            points: vec![],
            triangles: vec![],
        });
    }
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(f64::MIN_POSITIVE);
    let eps = 1e-12 * size;
    let c = Point2::new(0.5 * (lo.x + hi.x), 0.5 * (lo.y + hi.y));
    let m = 100.0 * size;

    let kept: Vec<Point2> = interior
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .filter(|&p| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y)
        .collect();

    let mut pts = Vec::with_capacity(SUPER as usize + points.len() + kept.len());
    pts.push(Point2::new(c.x - m, c.y - m));
    pts.push(Point2::new(c.x + m, c.y - m));
    pts.push(Point2::new(c.x, c.y + m));
    let mut mesh = Mesh {
        pts,
        tris: vec![Tri {
            v: [0, 1, 2],
            n: [NONE; 3],
            c: [false; 3],
        }],
        vtri: vec![0; 3],
        last: 0,
        inside: Vec::new(),
        stack: Vec::new(),
    };
    let ext = |v: u32| v.wrapping_sub(SUPER);

    mesh.pts.extend_from_slice(points);
    mesh.vtri.resize(mesh.pts.len(), 0);
    for i in brio_order(points, lo, size) {
        let p = points[i];
        let v = SUPER + i as u32;
        match mesh
            .locate(p)
            .ok_or(CdtError::Internal("point location failed"))?
        {
            Location::Vertex(w) => {
                return Err(CdtError::DuplicatePoint {
                    a: ext(w),
                    b: i as u32,
                });
            }
            loc => mesh.insert(v, loc),
        }
    }
    for lp in loops {
        for k in 0..lp.len() {
            let a = lp[k] + SUPER;
            let b = lp[(k + 1) % lp.len()] + SUPER;
            mesh.insert_constraint(a, b, ext)?;
        }
    }

    // even-odd flood fill from the super triangle's corner
    let nt = mesh.tris.len();
    let mut parity: Vec<Option<bool>> = vec![None; nt];
    let seed = mesh.vtri[0];
    parity[seed as usize] = Some(false);
    let mut queue = VecDeque::from([seed]);
    while let Some(t) = queue.pop_front() {
        let tt = mesh.tris[t as usize];
        let pt = parity[t as usize].expect("queued triangles have parity");
        for i in 0..3 {
            let u = tt.n[i];
            if u == NONE || parity[u as usize].is_some() {
                continue;
            }
            parity[u as usize] = Some(pt ^ tt.c[i]);
            queue.push_back(u);
        }
    }
    mesh.inside = parity.iter().map(|p| *p == Some(true)).collect();

    // interior points go in only where they land inside, clear of the boundary
    let kept: Vec<Point2> = brio_order(&kept, lo, size).into_iter().map(|i| kept[i]).collect();
    for &p in &kept {
        let loc = mesh
            .locate(p)
            .ok_or(CdtError::Internal("point location failed"))?;
        let near = |w: u32, mesh: &Mesh| (mesh.p(w) - p).length_sq() <= eps * eps;
        let near_constraint = |t: u32, mesh: &Mesh| {
            let tt = mesh.tris[t as usize];
            (0..3).any(|i| {
                tt.c[i] && segment_distance_sq(p, mesh.p(tt.v[nx(i)]), mesh.p(tt.v[pv(i)])) <= eps * eps
            })
        };
        let skip = match &loc {
            Location::Vertex(_) => true,
            Location::Inside(t) => {
                !mesh.inside[*t as usize]
                    || near_constraint(*t, &mesh)
                    || mesh.tris[*t as usize].v.iter().any(|&w| near(w, &mesh))
            }
            Location::OnEdge(t, i) => {
                let tt = mesh.tris[*t as usize];
                let u = tt.n[*i];
                tt.c[*i]
                    || !mesh.inside[*t as usize]
                    || near_constraint(*t, &mesh)
                    || near_constraint(u, &mesh)
                    || tt
                        .v
                        .iter()
                        .chain(mesh.tris[u as usize].v.iter())
                        .any(|&w| near(w, &mesh))
            }
        };
        if skip {
            continue;
        }
        let v = mesh.pts.len() as u32;
        mesh.pts.push(p);
        mesh.vtri.push(0);
        mesh.insert(v, loc);
    }

    let triangles = mesh
        .tris
        .iter()
        .zip(&mesh.inside)
        .filter(|(t, inside)| **inside && t.v.iter().all(|&v| v >= SUPER))
        .map(|(t, _)| t.v.map(|v| v - SUPER))
        .collect();
    Ok(Triangulation {
        points: mesh.pts.split_off(SUPER as usize),
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::dmath;
    use forge_math::rng::Rng;

    fn shoelace(pts: &[Point2], lp: &[u32]) -> f64 {
        (0..lp.len())
            .map(|k| {
                let a = pts[lp[k] as usize];
                let b = pts[lp[(k + 1) % lp.len()] as usize];
                a.x * b.y - a.y * b.x
            })
            .sum::<f64>()
            * 0.5
    }

    /// Checks every invariant; `area` is the region's expected area.
    fn check(tr: &Triangulation, loops: &[Vec<u32>], area: f64) {
        let p = &tr.points;
        let mut edges: Vec<(u32, u32, usize)> = Vec::new();
        let mut total = 0.0;
        for (ti, t) in tr.triangles.iter().enumerate() {
            let o = orient2d(p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
            assert!(o > 0.0, "triangle {t:?} not counter-clockwise");
            let (a, b, c) = (p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
            total += 0.5 * (b - a).cross(c - a);
            for i in 0..3 {
                edges.push((t[i], t[(i + 1) % 3], ti));
            }
        }
        assert!(
            (total - area).abs() <= 1e-9 * area.abs(),
            "area {total} vs {area}"
        );
        edges.sort();
        let find = |a: u32, b: u32| {
            edges
                .binary_search_by(|e| (e.0, e.1).cmp(&(a, b)))
                .ok()
                .map(|i| edges[i].2)
        };
        let mut constrained: Vec<(u32, u32)> = Vec::new();
        for lp in loops {
            for k in 0..lp.len() {
                let (a, b) = (lp[k], lp[(k + 1) % lp.len()]);
                assert!(
                    find(a, b).is_some() || find(b, a).is_some(),
                    "constraint {a}-{b} missing"
                );
                constrained.push((a.min(b), a.max(b)));
            }
        }
        constrained.sort();
        for &(a, b, ti) in &edges {
            let Some(tj) = find(b, a) else {
                assert!(
                    constrained.binary_search(&(a.min(b), a.max(b))).is_ok(),
                    "open edge {a}-{b}"
                );
                continue;
            };
            if constrained.binary_search(&(a.min(b), a.max(b))).is_ok() {
                continue;
            }
            let t = tr.triangles[ti];
            let far = *tr.triangles[tj].iter().find(|&&v| v != a && v != b).unwrap();
            // Delaunay up to cases the filter cannot decide (near-cocircular)
            let inside = certainly_in_circle(
                p[t[0] as usize],
                p[t[1] as usize],
                p[t[2] as usize],
                p[far as usize],
            );
            assert!(!inside, "edge {a}-{b} not Delaunay");
        }
        let mut used = vec![false; p.len()];
        for t in &tr.triangles {
            for &v in t {
                used[v as usize] = true;
            }
        }
        assert!(used.iter().all(|&u| u), "unused point");
    }

    fn circle_loop(pts: &mut Vec<Point2>, c: Point2, r: f64, n: usize, ccw: bool) -> Vec<u32> {
        let start = pts.len() as u32;
        for k in 0..n {
            let a = dmath::TAU * k as f64 / n as f64 * if ccw { 1.0 } else { -1.0 };
            let (s, co) = dmath::sin_cos(a);
            pts.push(Point2::new(c.x + r * co, c.y + r * s));
        }
        (start..start + n as u32).collect()
    }

    #[test]
    fn square_with_circular_holes() {
        let mut pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let mut loops = vec![vec![0, 1, 2, 3]];
        loops.push(circle_loop(&mut pts, Point2::new(3.0, 3.0), 1.5, 40, false));
        loops.push(circle_loop(&mut pts, Point2::new(7.0, 6.5), 2.0, 64, true));
        let area = 100.0 - shoelace(&pts, &loops[1]).abs() - shoelace(&pts, &loops[2]).abs();
        let mut interior = Vec::new();
        for i in 0..20 {
            for j in 0..20 {
                interior.push(Point2::new(0.25 + 0.5 * i as f64, 0.25 + 0.5 * j as f64));
            }
        }
        let tr = triangulate(&pts, &loops, &interior).unwrap();
        check(&tr, &loops, area);
        assert!(
            tr.points.len() > pts.len() + 300,
            "holes should drop only some interior points"
        );
        // nothing kept inside a hole
        for q in &tr.points[pts.len()..] {
            assert!(q.distance(Point2::new(3.0, 3.0)) > 1.4 && q.distance(Point2::new(7.0, 6.5)) > 1.9);
        }
    }

    #[test]
    fn random_stars_with_holes() {
        let mut r = Rng::new(600);
        for _ in 0..60 {
            let mut pts = Vec::new();
            let n = 12 + r.below(60);
            let c = Point2::new(r.range(-100.0, 100.0), r.range(-100.0, 100.0));
            for k in 0..n {
                let a = dmath::TAU * (k as f64 + r.range(0.0, 0.9)) / n as f64;
                let rad = r.range(10.0, 20.0);
                let (s, co) = dmath::sin_cos(a);
                pts.push(Point2::new(c.x + rad * co, c.y + rad * s));
            }
            let mut loops = vec![(0..n as u32).collect::<Vec<_>>()];
            if r.bool() {
                loops[0].reverse();
            }
            let hole = circle_loop(&mut pts, c, r.range(2.0, 8.0), 3 + r.below(30), r.bool());
            let area = shoelace(&pts, &loops[0]).abs() - shoelace(&pts, &hole).abs();
            loops.push(hole);
            let interior: Vec<Point2> = (0..r.below(200))
                .map(|_| Point2::new(c.x + r.range(-20.0, 20.0), c.y + r.range(-20.0, 20.0)))
                .collect();
            let tr = triangulate(&pts, &loops, &interior).unwrap();
            check(&tr, &loops, area);
        }
    }

    #[test]
    fn interior_points_all_used_when_inside() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        ];
        let loops = vec![vec![0, 1, 2, 3]];
        let mut r = Rng::new(601);
        let interior: Vec<Point2> = (0..500)
            .map(|_| Point2::new(r.range(0.1, 3.9), r.range(0.1, 2.9)))
            .collect();
        let tr = triangulate(&pts, &loops, &interior).unwrap();
        assert_eq!(tr.points.len(), 504);
        check(&tr, &loops, 12.0);
        // outside, on-edge and duplicate points are dropped
        let extra = [
            Point2::new(5.0, 1.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 1.0),
        ];
        let tr = triangulate(&pts, &loops, &extra).unwrap();
        assert_eq!(tr.points.len(), 5);
        check(&tr, &loops, 12.0);
    }

    #[test]
    fn long_thin_rectangle() {
        // like an unrolled cylinder face: 1000 x 0.01 with 200 points on each long side
        let mut pts = Vec::new();
        for k in 0..200 {
            pts.push(Point2::new(1000.0 * k as f64 / 199.0, 0.0));
        }
        for k in 0..200 {
            pts.push(Point2::new(1000.0 * (199 - k) as f64 / 199.0, 0.01));
        }
        let loops = vec![(0..400).collect::<Vec<u32>>()];
        let tr = triangulate(&pts, &loops, &[]).unwrap();
        check(&tr, &loops, 10.0);
        assert_eq!(tr.triangles.len(), 398);
    }

    #[test]
    fn collinear_boundary_points() {
        let mut pts = Vec::new();
        let per = 50;
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        for s in 0..4 {
            let (a, b) = (corners[s], corners[(s + 1) % 4]);
            for k in 0..per {
                let t = k as f64 / per as f64;
                pts.push(Point2::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        let loops = vec![(0..pts.len() as u32).collect::<Vec<_>>()];
        let tr = triangulate(&pts, &loops, &[]).unwrap();
        check(&tr, &loops, 1.0);
        assert_eq!(tr.triangles.len(), pts.len() - 2);
    }

    #[test]
    fn hole_through_dense_cocircular_grid() {
        // boundary points come first, so the hole's edges are recovered through a
        // grid of interior points where every quad is cocircular
        let mut pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(6.0, 0.0),
            Point2::new(6.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        let hole = circle_loop(&mut pts, Point2::new(3.0, 3.0), 2.2, 200, false);
        let area = 36.0 - shoelace(&pts, &hole).abs();
        let loops = vec![vec![0, 1, 2, 3], hole];
        let interior: Vec<Point2> = (1..60)
            .flat_map(|i| (1..60).map(move |j| Point2::new(0.1 * i as f64, 0.1 * j as f64)))
            .collect();
        let tr = triangulate(&pts, &loops, &interior).unwrap();
        check(&tr, &loops, area);
    }

    #[test]
    fn deterministic() {
        let mut pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 0.0),
            Point2::new(5.0, 5.0),
            Point2::new(0.0, 5.0),
        ];
        let hole = circle_loop(&mut pts, Point2::new(2.5, 2.5), 1.0, 17, true);
        let loops = vec![vec![0, 1, 2, 3], hole];
        let interior: Vec<Point2> = (0..50)
            .map(|k| Point2::new(0.1 * k as f64, 0.37 * (k % 13) as f64))
            .collect();
        let a = triangulate(&pts, &loops, &interior).unwrap();
        let b = triangulate(&pts, &loops, &interior).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn errors() {
        let sq = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let bowtie = triangulate(&sq, &[vec![0, 2, 1, 3]], &[]);
        assert!(
            matches!(bowtie, Err(CdtError::ConstraintsIntersect { .. })),
            "{bowtie:?}"
        );
        let mut dup = sq.clone();
        dup.push(Point2::new(1.0, 1.0));
        assert!(matches!(
            triangulate(&dup, &[vec![0, 1, 2, 3]], &[]),
            Err(CdtError::DuplicatePoint { .. })
        ));
        let mut through = sq.clone();
        through.push(Point2::new(0.5, 0.0));
        through.push(Point2::new(0.5, 0.5));
        through.push(Point2::new(0.4, 0.5));
        let r = triangulate(&through, &[vec![0, 1, 2, 3], vec![4, 5, 6]], &[]);
        assert!(
            matches!(r, Err(CdtError::ConstraintThroughPoint { point: 4, .. })),
            "{r:?}"
        );
        assert!(matches!(
            triangulate(&sq, &[vec![0, 1]], &[]),
            Err(CdtError::BadLoop { .. })
        ));
        assert!(matches!(
            triangulate(&sq, &[vec![0, 1, 9]], &[]),
            Err(CdtError::BadIndex { index: 9 })
        ));
    }
}
