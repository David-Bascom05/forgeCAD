//! Constrained Delaunay triangulation of polygonal regions (outer boundaries
//! and holes) with optional interior points. Points are inserted one by one in
//! input order with Lawson flips, constraint edges are recovered by flipping
//! the edges they cross, and triangles outside the region are dropped by an
//! even-odd flood fill across constraint edges. All decisions use the exact
//! predicates, so the result depends only on the input.

use forge_math::Point2;
use forge_math::predicates::{certainly_in_circle, orient2d};

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
    /// Refinement stopped with chords or triangles still asking to be split: past its rounds, or
    /// past its budget of points. A mesh that may break the tolerance is never returned as good.
    Unsettled {
        rounds: u32,
        points: u32,
    },
    /// The chord asks for more points than a face may have (`CEILING`): about `required` at
    /// `chord` (`Refine::need`). A coarser chord meshes it.
    TooFine {
        chord: f64,
        required: u64,
    },
    /// An invariant broke; a bug, not bad input.
    Internal(&'static str),
}

/// Rounds of refinement before it counts as unsettled.
const ROUNDS: usize = 200;
/// The most points one face's triangulation may have, whatever its chord asks.
pub const CEILING: usize = 1 << 23;
/// A refinement's points over what its chord asks of an ideal mesh (`Refine::need`): splitting at
/// the middles of edges and triangles makes about twice as many.
const SAFETY: u64 = 4;
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
    /// The constraint the last `insert_constraint` to fail ran into.
    hit: (u32, u32),
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
/// Two constraints a-b and another cross (a, b less `shift`, as the caller numbers them).
fn crossing(a: u32, b: u32, shift: u32) -> CdtError {
    CdtError::ConstraintsIntersect {
        a: a.wrapping_sub(shift),
        b: b.wrapping_sub(shift),
    }
}

/// The constraint a-b passes through `point` (all less `shift`).
fn through(a: u32, b: u32, point: u32, shift: u32) -> CdtError {
    CdtError::ConstraintThroughPoint {
        a: a.wrapping_sub(shift),
        b: b.wrapping_sub(shift),
        point: point.wrapping_sub(shift),
    }
}

/// Whether l-r and a-b are one chord, either way round.
fn same_chord(l: u32, r: u32, a: u32, b: u32) -> bool {
    (l == a && r == b) || (l == b && r == a)
}

/// Where the chord a-b is, or goes, in `uses` (sorted by lower end, then upper): its index, and
/// whether it is there.
fn find_use(uses: &[(u32, u32, u32)], a: u32, b: u32) -> (usize, bool) {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    // the first entry not below (lo, hi)
    let (mut l, mut r) = (0, uses.len());
    while l < r {
        let m = l + (r - l) / 2;
        let (x, y, _) = uses[m];
        if x < lo || (x == lo && y < hi) {
            l = m + 1;
        } else {
            r = m;
        }
    }
    (l, l < uses.len() && uses[l].0 == lo && uses[l].1 == hi)
}

/// One more use of the chord a-b.
fn count_use(uses: &mut Vec<(u32, u32, u32)>, a: u32, b: u32) {
    let (k, there) = find_use(uses, a, b);
    if there {
        uses[k].2 += 1;
    } else {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        uses.insert(k, (lo, hi, 1));
    }
}

/// The chord a-b's uses, taken off the count (one if it has none: a chord drawn once).
fn take_uses(uses: &mut Vec<(u32, u32, u32)>, a: u32, b: u32) -> u32 {
    let (k, there) = find_use(uses, a, b);
    if there { uses.remove(k).2 } else { 1 }
}

/// Where x is in a triangle's three vertices or neighbours.
fn slot(a: [u32; 3], x: u32) -> Option<usize> {
    if a[0] == x {
        Some(0)
    } else if a[1] == x {
        Some(1)
    } else if a[2] == x {
        Some(2)
    } else {
        None
    }
}

impl Mesh {
    fn p(&self, v: u32) -> Point2 {
        self.pts[v as usize]
    }

    fn write(&mut self, t: u32, tri: Tri) {
        self.vtri[tri.v[0] as usize] = t;
        self.vtri[tri.v[1] as usize] = t;
        self.vtri[tri.v[2] as usize] = t;
        self.tris[t as usize] = tri;
    }

    fn relink(&mut self, n: u32, old: u32, new: u32) {
        if n != NONE {
            let tri = &mut self.tris[n as usize];
            if let Some(k) = slot(tri.n, old) {
                tri.n[k] = new;
            }
        }
    }

    /// None only if the point is outside every triangle, which the enclosing super-triangle rules out.
    fn locate(&mut self, p: Point2) -> Option<Location> {
        let mut t = self.last;
        let limit = 4 * self.tris.len() + 64;
        let mut step = 0;
        let mut found = false;
        while !found && step < limit {
            let tri = self.tris[t as usize];
            let mut next = NONE;
            let mut k = 0;
            while next == NONE && k < 3 {
                let i = (k + step) % 3;
                let (a, b) = (self.p(tri.v[nx(i)]), self.p(tri.v[pv(i)]));
                if orient2d(a, b, p) < 0.0 && tri.n[i] != NONE {
                    next = tri.n[i];
                }
                k += 1;
            }
            if next == NONE {
                found = true;
            } else {
                t = next;
                step += 1;
            }
        }
        // The visibility walk can cycle in a constrained triangulation; fall back to a scan.
        if !found {
            t = 0;
            while !found && (t as usize) < self.tris.len() {
                let tri = self.tris[t as usize];
                let (a, b, c) = (self.p(tri.v[0]), self.p(tri.v[1]), self.p(tri.v[2]));
                if orient2d(b, c, p) >= 0.0 && orient2d(c, a, p) >= 0.0 && orient2d(a, b, p) >= 0.0 {
                    found = true;
                } else {
                    t += 1;
                }
            }
        }
        if found { Some(self.classify(t, p)) } else { None }
    }

    fn classify(&self, t: u32, p: Point2) -> Location {
        let tri = self.tris[t as usize];
        let (a, b, c) = (self.p(tri.v[0]), self.p(tri.v[1]), self.p(tri.v[2]));
        if a == p {
            Location::Vertex(tri.v[0])
        } else if b == p {
            Location::Vertex(tri.v[1])
        } else if c == p {
            Location::Vertex(tri.v[2])
        } else if orient2d(b, c, p) == 0.0 {
            Location::OnEdge(t, 0)
        } else if orient2d(c, a, p) == 0.0 {
            Location::OnEdge(t, 1)
        } else if orient2d(a, b, p) == 0.0 {
            Location::OnEdge(t, 2)
        } else {
            Location::Inside(t)
        }
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
        let j = slot(uu.n, t).expect("neighbour links back");
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
        let j = slot(uu.n, t).expect("neighbour links back");
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
            let Some(j) = slot(uu.n, t) else {
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
        let mut steps = 0;
        let found = self.find_edge_counted(a, b, &mut steps);
        // a fan of thousands, asked of again and again, is the one cost here that grows unseen
        if steps > 64 {
            forge_geom::budget::spend_many(steps);
        }
        found
    }

    fn find_edge_counted(&self, a: u32, b: u32, steps: &mut u64) -> Option<(u32, usize)> {
        let start = self.vtri[a as usize];
        let mut found = None;
        // the search is over: found, a lost, or the fan closed
        let mut done = false;
        // counter-clockwise around a (across a -> v[pv]); a super-triangle corner's fan ends at
        // the hull, so the rest of it is searched clockwise (across a -> v[nx])
        let mut pass = 0;
        while !done && pass < 2 {
            let ccw = pass == 0;
            let mut t = start;
            let mut on = true;
            let mut k = 0;
            while on && k < self.tris.len() {
                *steps += 1;
                let tt = self.tris[t as usize];
                match slot(tt.v, a) {
                    None => done = true,
                    Some(ia) => {
                        if tt.v[nx(ia)] == b {
                            found = Some((t, pv(ia)));
                            done = true;
                        } else {
                            t = if ccw { tt.n[nx(ia)] } else { tt.n[pv(ia)] };
                            if t == start {
                                done = true;
                            } else if t == NONE {
                                on = false;
                            }
                        }
                    }
                }
                on = on && !done;
                k += 1;
            }
            pass += 1;
        }
        found
    }

    fn has_edge(&self, a: u32, b: u32) -> Option<(u32, usize)> {
        match self.find_edge(a, b) {
            Some(e) => Some(e),
            None => self.find_edge(b, a),
        }
    }

    /// The constraint a-b, recovered by flipping the edges it crosses. Errors name points less
    /// `shift` (the super-triangle's corners, or none).
    // (Aeneas: no is_multiple_of, research/aeneas/STYLE.md)
    #[allow(clippy::manual_is_multiple_of)]
    fn insert_constraint(&mut self, a: u32, b: u32, shift: u32) -> Result<(), CdtError> {
        if let Some((t, i)) = self.has_edge(a, b) {
            self.hit = (a, b);
            return self.mark(t, i, crossing(a, b, shift));
        }
        let (pa, pb) = (self.p(a), self.p(b));
        let mut err: Option<CdtError> = None;

        // the triangle around a whose far edge the segment leaves through
        let start = self.vtri[a as usize];
        let mut t = start;
        let mut cur: Option<(u32, u32, u32)> = None;
        let mut on = true;
        let mut k = 0;
        while on && k <= self.tris.len() {
            let tt = self.tris[t as usize];
            match slot(tt.v, a) {
                None => err = Some(CdtError::Internal("fan lost a")),
                Some(ia) => {
                    let (v1, v2) = (tt.v[nx(ia)], tt.v[pv(ia)]);
                    let o1 = orient2d(pa, pb, self.p(v1));
                    let o2 = orient2d(pa, pb, self.p(v2));
                    if o1 == 0.0 && (self.p(v1) - pa).dot(pb - pa) > 0.0 {
                        err = Some(through(a, b, v1, shift));
                    } else if o2 == 0.0 && (self.p(v2) - pa).dot(pb - pa) > 0.0 {
                        err = Some(through(a, b, v2, shift));
                    } else if o1 < 0.0 && o2 > 0.0 {
                        cur = Some((t, v2, v1));
                        on = false;
                    } else {
                        t = tt.n[nx(ia)];
                        on = t != start;
                    }
                }
            }
            on = on && err.is_none();
            k += 1;
        }
        if let Some(e) = err {
            return Err(e);
        }
        let Some((mut t, mut left, mut right)) = cur else {
            return Err(CdtError::Internal("no start wedge"));
        };

        // the edges it crosses, in order from a
        let mut crossed: Vec<(u32, u32)> = Vec::new();
        let mut walking = true;
        while walking && err.is_none() {
            match self.find_edge(right, left) {
                None => err = Some(CdtError::Internal("walk lost edge")),
                Some((tc, ic)) => {
                    debug_assert_eq!(tc, t);
                    if self.tris[tc as usize].c[ic] {
                        self.hit = (left, right);
                        err = Some(crossing(a, b, shift));
                    } else {
                        crossed.push((left, right));
                        let u = self.tris[tc as usize].n[ic];
                        let uu = self.tris[u as usize];
                        match slot(uu.n, tc) {
                            None => err = Some(CdtError::Internal("broken link")),
                            Some(j) => {
                                let q = uu.v[j];
                                if q == b {
                                    walking = false;
                                } else {
                                    let oq = orient2d(pa, pb, self.p(q));
                                    if oq == 0.0 {
                                        err = Some(through(a, b, q, shift));
                                    } else if oq > 0.0 {
                                        left = q;
                                    } else {
                                        right = q;
                                    }
                                    t = u;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(e) = err {
            return Err(e);
        }

        // flipped away in turn, a queue (a ring over `crossed`: each edge taken off it puts at
        // most one back); an edge whose quadrilateral is not convex waits its turn again
        let ring = crossed.len();
        let mut head = 0;
        let mut queued = ring;
        let mut fresh: Vec<(u32, u32)> = Vec::new();
        let mut budget = 64 * (ring + 4) * (ring + 4);
        while queued > 0 && err.is_none() {
            let (x, y) = crossed[head];
            head = if head + 1 == ring { 0 } else { head + 1 };
            queued -= 1;
            let back = if head + queued >= ring {
                head + queued - ring
            } else {
                head + queued
            };
            if budget == 0 {
                err = Some(CdtError::Internal("constraint recovery did not converge"));
            } else {
                budget -= 1;
                if budget % 1024 == 0 {
                    forge_geom::budget::spend_many(16 * 1024);
                    if forge_geom::budget::over() {
                        err = Some(CdtError::Internal("the boundary would take too long"));
                    }
                }
            }
            if err.is_none() {
                match self.find_edge(x, y) {
                    None => err = Some(CdtError::Internal("crossed edge vanished")),
                    Some((t, i)) => {
                        let tt = self.tris[t as usize];
                        let u = tt.n[i];
                        let uu = self.tris[u as usize];
                        match slot(uu.n, t) {
                            None => err = Some(CdtError::Internal("broken link")),
                            Some(j) => {
                                let (p, q) = (tt.v[i], uu.v[j]);
                                let (pp, pq) = (self.p(p), self.p(q));
                                let convex = {
                                    let s1 = orient2d(pp, pq, self.p(x));
                                    let s2 = orient2d(pp, pq, self.p(y));
                                    (s1 > 0.0 && s2 < 0.0) || (s1 < 0.0 && s2 > 0.0)
                                };
                                if !convex {
                                    crossed[back] = (x, y);
                                    queued += 1;
                                } else {
                                    self.flip(t, i);
                                    let still_crosses = p != a && p != b && q != a && q != b && {
                                        let s1 = orient2d(pa, pb, pp);
                                        let s2 = orient2d(pa, pb, pq);
                                        (s1 > 0.0 && s2 < 0.0) || (s1 < 0.0 && s2 > 0.0)
                                    };
                                    if still_crosses {
                                        crossed[back] = (p, q);
                                        queued += 1;
                                    } else {
                                        fresh.push((p, q));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(e) = err {
            return Err(e);
        }

        let Some((t, i)) = self.has_edge(a, b) else {
            return Err(CdtError::Internal("constraint not recovered"));
        };
        self.mark(t, i, crossing(a, b, shift))?;

        // restore the Delaunay property around the new edges
        let mut changed = true;
        let mut rounds = 0;
        while changed && rounds < 64 {
            changed = false;
            rounds += 1;
            let mut e = 0;
            while e < fresh.len() {
                let (p, q) = fresh[e];
                if let Some((t, i)) = self.has_edge(p, q) {
                    let tt = self.tris[t as usize];
                    if !tt.c[i] {
                        let u = tt.n[i];
                        let uu = self.tris[u as usize];
                        if let Some(j) = slot(uu.n, t) {
                            let r = uu.v[j];
                            if certainly_in_circle(
                                self.p(tt.v[0]),
                                self.p(tt.v[1]),
                                self.p(tt.v[2]),
                                self.p(r),
                            ) {
                                let o = tt.v[i];
                                self.flip(t, i);
                                fresh[e] = (o, r);
                                changed = true;
                            }
                        }
                    }
                }
                e += 1;
            }
        }
        // each a search round a vertex: a constraint across thousands of triangles is the dear part
        let work = fresh.len() * rounds;
        if work > 64 {
            forge_geom::budget::spend_many(16 * work as u64);
        }
        Ok(())
    }

    /// A boundary that crosses, overlaps and repeats itself, drawn in as it is: chords are cut where
    /// they cross or pass through a point. Each chord's uses are counted, and one used an even
    /// number of times bounds nothing (a slit, a loop there and back).
    fn insert_tangled(&mut self, chords: Vec<(u32, u32)>) -> Result<(), CdtError> {
        // each chord's uses, sorted by its ends (lower first)
        let mut uses: Vec<(u32, u32, u32)> = Vec::new();
        let mut budget = 64 * chords.len() + 1024;
        let mut todo = chords;
        todo.reverse();
        let mut err: Option<CdtError> = None;
        while err.is_none() && !todo.is_empty() {
            let (a, b) = todo[todo.len() - 1];
            todo.pop();
            if a != b {
                if budget == 0 {
                    err = Some(CdtError::Internal("a tangled boundary did not untangle"));
                } else {
                    budget -= 1;
                    if forge_geom::budget::over() {
                        err = Some(CdtError::Internal("the boundary would take too long"));
                    }
                }
            }
            if a != b && err.is_none() {
                match self.insert_constraint(a, b, 0) {
                    Ok(()) => count_use(&mut uses, a, b),
                    Err(CdtError::ConstraintThroughPoint { point, .. }) => {
                        todo.push((point, b));
                        todo.push((a, point));
                    }
                    Err(CdtError::ConstraintsIntersect { .. }) => {
                        let (l, r) = self.hit;
                        if same_chord(l, r, a, b) {
                            // the same chord again
                            count_use(&mut uses, a, b);
                        } else {
                            match self.cut(a, b, l, r, &mut uses) {
                                Err(e) => err = Some(e),
                                Ok((w, n)) => {
                                    todo.push((w, b));
                                    todo.push((a, w));
                                    let mut k = 0;
                                    while k < n {
                                        todo.push((w, r));
                                        todo.push((l, w));
                                        k += 1;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => err = Some(e),
                }
            }
        }
        let mut k = 0;
        while err.is_none() && k < uses.len() {
            let (a, b, n) = uses[k];
            if n % 2 == 0 {
                match self.has_edge(a, b) {
                    None => err = Some(CdtError::Internal("a boundary chord vanished")),
                    Some((t, i)) => self.unmark(t, i),
                }
            }
            k += 1;
        }
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// The chord a-b crosses the constraint l-r: a point where they cross (or the corner it
    /// rounds to), l-r unmarked and its uses taken off the count. With the point and those uses.
    fn cut(
        &mut self,
        a: u32,
        b: u32,
        l: u32,
        r: u32,
        uses: &mut Vec<(u32, u32, u32)>,
    ) -> Result<(u32, u32), CdtError> {
        let (pa, pb, pl, pr) = (self.p(a), self.p(b), self.p(l), self.p(r));
        let (dl, dr) = (orient2d(pa, pb, pl), orient2d(pa, pb, pr));
        let x = pl.lerp(pr, dl / (dl - dr));
        let Some((t, i)) = self.has_edge(l, r) else {
            return Err(CdtError::Internal("crossed edge vanished"));
        };
        self.unmark(t, i);
        let n = take_uses(uses, l, r);
        let Some(loc) = self.locate(x) else {
            return Err(CdtError::Internal("point location failed"));
        };
        let w = match loc {
            // on a corner, to the last bit: the chords go through it
            Location::Vertex(w) => w,
            loc => {
                let w = self.pts.len() as u32;
                self.pts.push(x);
                self.vtri.push(0);
                self.insert(w, loc);
                w
            }
        };
        Ok((w, n))
    }

    fn unmark(&mut self, t: u32, i: usize) {
        self.tris[t as usize].c[i] = false;
        let u = self.tris[t as usize].n[i];
        if u != NONE
            && let Some(k) = slot(self.tris[u as usize].n, t)
        {
            self.tris[u as usize].c[k] = false;
        }
    }

    /// Inserts an interior point where it lands inside the region, clear of the boundary and of
    /// existing points by `eps`; elsewhere it is skipped.
    fn insert_interior(&mut self, p: Point2, eps: f64) -> Result<(), CdtError> {
        let loc = self
            .locate(p)
            .ok_or(CdtError::Internal("point location failed"))?;
        let skip = match &loc {
            Location::Vertex(_) => true,
            Location::Inside(t) => {
                !self.inside[*t as usize] || self.near_constraint(*t, p, eps) || self.near_corner(*t, p, eps)
            }
            Location::OnEdge(t, i) => {
                let tt = self.tris[*t as usize];
                let u = tt.n[*i];
                tt.c[*i]
                    || !self.inside[*t as usize]
                    || self.near_constraint(*t, p, eps)
                    || self.near_constraint(u, p, eps)
                    || self.near_corner(*t, p, eps)
                    || self.near_corner(u, p, eps)
            }
        };
        if !skip {
            let v = self.pts.len() as u32;
            self.pts.push(p);
            self.vtri.push(0);
            self.insert(v, loc);
        }
        Ok(())
    }

    /// Whether p is within eps of a corner of t, in the corners' order.
    fn near_corner(&self, t: u32, p: Point2, eps: f64) -> bool {
        let v = self.tris[t as usize].v;
        (self.p(v[0]) - p).length_sq() <= eps * eps
            || (self.p(v[1]) - p).length_sq() <= eps * eps
            || (self.p(v[2]) - p).length_sq() <= eps * eps
    }

    /// Whether p is within eps of a constraint edge of t, in the edges' order.
    fn near_constraint(&self, t: u32, p: Point2, eps: f64) -> bool {
        let tt = self.tris[t as usize];
        let (a, b, c) = (self.p(tt.v[0]), self.p(tt.v[1]), self.p(tt.v[2]));
        (tt.c[0] && segment_distance_sq(p, b, c) <= eps * eps)
            || (tt.c[1] && segment_distance_sq(p, c, a) <= eps * eps)
            || (tt.c[2] && segment_distance_sq(p, a, b) <= eps * eps)
    }

    fn mark(&mut self, t: u32, i: usize, err: CdtError) -> Result<(), CdtError> {
        if self.tris[t as usize].c[i] {
            return Err(err);
        }
        self.tris[t as usize].c[i] = true;
        let u = self.tris[t as usize].n[i];
        if u != NONE {
            let Some(k) = slot(self.tris[u as usize].n, t) else {
                return Err(CdtError::Internal("broken link"));
            };
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
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        order.push(i);
        i += 1;
    }
    let mut rng = forge_math::rng::Rng::new(0x5eed_cd71);
    let mut i = n;
    while i > 1 {
        i -= 1;
        order.swap(i, rng.below(i + 1));
    }
    let scale = 65535.0 / size;
    let mut keys: Vec<u64> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let p = points[i];
        keys.push(morton(grid(p.x - lo.x, scale), grid(p.y - lo.y, scale)));
        i += 1;
    }
    let mut run: Vec<u64> = Vec::new();
    let mut tmp: Vec<u64> = Vec::new();
    let mut start = 0;
    let mut round = 16usize;
    while start < n {
        let end = if start + round < n { start + round } else { n };
        // the last round takes everything left so rounds keep doubling
        let end = if n - end < round { n } else { end };
        // by key, then by index: a key below 2^32 and an index below 2^32, one u64
        run.clear();
        let mut k = start;
        while k < end {
            run.push(keys[order[k]] << 32 | order[k] as u64);
            k += 1;
        }
        sort_keys(&mut run, &mut tmp);
        let mut k = start;
        while k < end {
            order[k] = (run[k - start] & 0xffff_ffff) as usize;
            k += 1;
        }
        start = end;
        round *= 2;
    }
    order
}

/// A coordinate offset on the 65536-step grid of `scale` steps a unit, clamped to it.
fn grid(v: f64, scale: f64) -> u32 {
    let x = v * scale;
    // (f64::clamp: the same two comparisons, in this order)
    let x = if x < 0.0 { 0.0 } else { x };
    let x = if x > 65535.0 { 65535.0 } else { x };
    x as u32
}

/// Sorts v ascending: insertion sorts of runs of 16, merged bottom-up through `tmp`.
fn sort_keys(v: &mut [u64], tmp: &mut Vec<u64>) {
    let n = v.len();
    let mut s = 0;
    while s < n {
        let e = if s + 16 < n { s + 16 } else { n };
        let mut i = s + 1;
        while i < e {
            let x = v[i];
            let mut j = i;
            while j > s && v[j - 1] > x {
                v[j] = v[j - 1];
                j -= 1;
            }
            v[j] = x;
            i += 1;
        }
        s = e;
    }
    if n > 16 {
        tmp.clear();
        tmp.resize(n, 0);
        let mut w = 16;
        let mut in_v = true;
        while w < n {
            if in_v {
                merge_runs(v, tmp, w);
            } else {
                merge_runs(tmp, v, w);
            }
            in_v = !in_v;
            w *= 2;
        }
        if !in_v {
            let mut i = 0;
            while i < n {
                v[i] = tmp[i];
                i += 1;
            }
        }
    }
}

/// One bottom-up merge pass: each two sorted runs of `w` in `src`, merged into `dst`.
fn merge_runs(src: &[u64], dst: &mut [u64], w: usize) {
    let n = src.len();
    let mut s = 0;
    while s < n {
        let m = if s + w < n { s + w } else { n };
        let e = if m + w < n { m + w } else { n };
        let (mut i, mut j, mut k) = (s, m, s);
        while k < e {
            if j >= e || (i < m && src[i] <= src[j]) {
                dst[k] = src[i];
                i += 1;
            } else {
                dst[k] = src[j];
                j += 1;
            }
            k += 1;
        }
        s = e;
    }
}

/// Position along a Hilbert curve over a 65536 x 65536 grid: neighbours along it are
/// neighbours in the plane (a Z-order curve jumps across at each power of two).
// (Aeneas: no mem::swap, research/aeneas/STYLE.md)
#[allow(clippy::manual_swap)]
fn hilbert(mut x: u32, mut y: u32) -> u64 {
    let mut d = 0u64;
    let mut s = 1u32 << 15;
    while s > 0 {
        let rx = u32::from(x & s > 0);
        let ry = u32::from(y & s > 0);
        d += u64::from(s) * u64::from(s) * u64::from((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = 0xffff - x;
                y = 0xffff - y;
            }
            let t = x;
            x = y;
            y = t;
        }
        s >>= 1;
    }
    d
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
        // (f64::clamp: the same two comparisons, in this order)
        let t = (p - a).dot(ab) / l;
        let t = if t < 0.0 { 0.0 } else { t };
        if t > 1.0 { 1.0 } else { t }
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
    triangulate_refined(points, loops, interior, None::<&dyn Refine>)
}

/// What refinement splits (point indices as in the result, and their positions).
pub trait Refine {
    /// Whether an unconstrained edge gets a point at its middle.
    fn edge(&self, a: u32, b: u32, pa: Point2, pb: Point2) -> bool;
    /// Whether a triangle gets a point at its centroid; never asked unless `triangles`.
    fn triangle(&self, v: [u32; 3], p: [Point2; 3]) -> bool;
    fn triangles(&self) -> bool;
    /// About how many points the refinement's tolerance asks for: asked once, only of a
    /// refinement past the budget a face's boundary alone gives it.
    fn need(&self) -> Option<Need> {
        None
    }
}

/// What a refinement's chord asks for: about `points` points (an ideal mesh's, of triangles
/// sagging `chord` each).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Need {
    pub chord: f64,
    pub points: u64,
}

/// A polygon with no inner points, zipped from its leftmost point along both sides, each step the
/// shorter chord: no triangulation to keep up. Triangles all counter-clockwise inside a simple
/// boundary cover it once. None where a step has no such triangle, or a chord or triangle fails
/// `refine`.
// (!(x > 0.0), not x <= 0.0: a NaN takes the same side as before)
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn zip<R: Refine + ?Sized>(points: &[Point2], lp: &[u32], refine: &R) -> Option<Vec<[u32; 3]>> {
    let n = lp.len();
    if n < 3 {
        return None;
    }
    // the leftmost point (the lowest y of those), the first of equals
    let mut s = 0;
    let mut k = 1;
    while k < n {
        let (m, x) = (at(points, lp, s), at(points, lp, k));
        if m.x > x.x || (m.x == x.x && m.y > x.y) {
            s = k;
        }
        k += 1;
    }
    let mut out = Vec::with_capacity(n - 2);
    let (mut lower, mut upper) = (s + n + 1, s + 2 * n - 1);
    let mut ok = zip_tri(points, lp, refine, [s + n, lower, upper], &mut out);
    if ok
        && lower + 1 < upper
        && refine.edge(
            lp[lower % n],
            lp[upper % n],
            at(points, lp, lower),
            at(points, lp, upper),
        )
    {
        ok = false;
    }
    while ok && lower + 1 < upper {
        let up_first = lower + 2 < upper
            && at(points, lp, lower).distance(at(points, lp, upper - 1))
                < at(points, lp, lower + 1).distance(at(points, lp, upper));
        // the rest of the polygon is the triangle left: its chords are the sides
        let take_up = if up_first {
            orient2d(
                at(points, lp, lower),
                at(points, lp, upper - 1),
                at(points, lp, upper),
            ) > 0.0
        } else {
            !(orient2d(
                at(points, lp, lower),
                at(points, lp, lower + 1),
                at(points, lp, upper),
            ) > 0.0)
        };
        let (a, b) = if take_up {
            (lower, upper - 1)
        } else {
            (lower + 1, upper)
        };
        if a + 1 < b && refine.edge(lp[a % n], lp[b % n], at(points, lp, a), at(points, lp, b)) {
            ok = false;
        } else {
            let v = if take_up {
                [lower, upper - 1, upper]
            } else {
                [lower, lower + 1, upper]
            };
            ok = zip_tri(points, lp, refine, v, &mut out);
            (lower, upper) = (a, b);
        }
    }
    if ok { Some(out) } else { None }
}

/// The point at place k of the loop lp, round and round.
fn at(points: &[Point2], lp: &[u32], k: usize) -> Point2 {
    points[lp[k % lp.len()] as usize]
}

/// `zip`'s triangle of loop places v, kept in `out` unless it is a sliver or `refine` splits it.
fn zip_tri<R: Refine + ?Sized>(
    points: &[Point2],
    lp: &[u32],
    refine: &R,
    v: [usize; 3],
    out: &mut Vec<[u32; 3]>,
) -> bool {
    let n = lp.len();
    let t = [lp[v[0] % n], lp[v[1] % n], lp[v[2] % n]];
    // a sliver the triangulation's choice of chords may well avoid: its call, not ours
    let (a, b, c) = (
        points[t[0] as usize],
        points[t[1] as usize],
        points[t[2] as usize],
    );
    let side = (a.distance(b)).max(b.distance(c)).max(c.distance(a));
    if orient2d(a, b, c) <= 1e-5 * side * side || (refine.triangles() && refine.triangle(t, [a, b, c])) {
        false
    } else {
        out.push(t);
        true
    }
}

/// `triangulate`, then the points `refine` asks for inserted into the same triangulation, round
/// after round, until it asks for none.
pub fn triangulate_refined<R: Refine + ?Sized>(
    points: &[Point2],
    loops: &[Vec<u32>],
    interior: &[Point2],
    refine: Option<&R>,
) -> Result<Triangulation, CdtError> {
    triangulate_within(points, loops, interior, refine, false)
}

/// `triangulate_refined`. `tangled`: a boundary that crosses or repeats itself or has two points in one place is no error.
/// The region is what an odd number of chords parts from the outside, its chords cut where they
/// cross, with a point there (after the input points) that the face's neighbours do not have.
pub fn triangulate_within<R: Refine + ?Sized>(
    points: &[Point2],
    loops: &[Vec<u32>],
    interior: &[Point2],
    refine: Option<&R>,
    tangled: bool,
) -> Result<Triangulation, CdtError> {
    match within(points, loops, interior, refine, tangled, ROUNDS, false) {
        Ok((t, _)) => Ok(t),
        Err(e) => Err(e),
    }
}

/// `triangulate_within`, but a refinement that does not settle (`CdtError::Unsettled`) gives the
/// triangulation as far as it got, with `false`: for a file's body, drawn as it is, its caller
/// to say so (`TriMesh::unsettled`).
pub fn triangulate_as_far<R: Refine + ?Sized>(
    points: &[Point2],
    loops: &[Vec<u32>],
    interior: &[Point2],
    refine: Option<&R>,
    tangled: bool,
) -> Result<(Triangulation, bool), CdtError> {
    within(points, loops, interior, refine, tangled, ROUNDS, true)
}

/// `triangulate_within`, refining for at most `rounds` rounds; with `as_far`, an unsettled
/// refinement stops where it is instead of failing. With whether it settled.
#[inline(never)]
fn within<R: Refine + ?Sized>(
    points: &[Point2],
    loops: &[Vec<u32>],
    interior: &[Point2],
    refine: Option<&R>,
    tangled: bool,
    rounds: usize,
    as_far: bool,
) -> Result<(Triangulation, bool), CdtError> {
    if let Some(e) = bad_input(points, loops, tangled) {
        return Err(e);
    }
    if points.is_empty() {
        return Ok((
            Triangulation {
                points: vec![],
                triangles: vec![],
            },
            true,
        ));
    }
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut k = 0;
    while k < points.len() {
        let p = points[k];
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        k += 1;
    }
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(f64::MIN_POSITIVE);
    let eps = 1e-12 * size;
    let c = Point2::new(0.5 * (lo.x + hi.x), 0.5 * (lo.y + hi.y));
    let m = 100.0 * size;

    let mut kept: Vec<Point2> = Vec::new();
    let mut k = 0;
    while k < interior.len() {
        let p = interior[k];
        if p.x.is_finite() && p.y.is_finite() && p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y {
            kept.push(p);
        }
        k += 1;
    }

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
        hit: (NONE, NONE),
    };
    mesh.pts.extend_from_slice(points);
    mesh.vtri.resize(mesh.pts.len(), 0);
    let same_as = mesh.insert_points(points, lo, size, tangled)?;
    if tangled {
        let mut chords: Vec<(u32, u32)> = Vec::new();
        let mut l = 0;
        while l < loops.len() {
            let lp = &loops[l];
            let mut k = 0;
            while k < lp.len() {
                let b = if k + 1 == lp.len() { lp[0] } else { lp[k + 1] };
                chords.push((one(&same_as, lp[k] + SUPER), one(&same_as, b + SUPER)));
                k += 1;
            }
            l += 1;
        }
        mesh.insert_tangled(chords)?;
    } else {
        mesh.insert_loops(loops)?;
    }
    mesh.flood();

    // interior points go in only where they land inside, clear of the boundary
    let order = brio_order(&kept, lo, size);
    let mut k = 0;
    while k < order.len() {
        mesh.insert_interior(kept[order[k]], eps)?;
        k += 1;
    }
    let mut settled = true;
    if let Some(refine) = refine {
        settled = mesh.refine(refine, points.len() + kept.len(), lo, size, eps, rounds, as_far)?;
    }

    let mut triangles = Vec::new();
    let mut t = 0;
    while t < mesh.tris.len() {
        let v = mesh.tris[t].v;
        if mesh.inside[t] && v[0] >= SUPER && v[1] >= SUPER && v[2] >= SUPER {
            triangles.push([v[0] - SUPER, v[1] - SUPER, v[2] - SUPER]);
        }
        t += 1;
    }
    Ok((
        Triangulation {
            points: mesh.pts.split_off(SUPER as usize),
            triangles,
        },
        settled,
    ))
}

/// The first thing wrong with the input, if anything is: a point not finite, a loop index out of
/// range, a loop too short or repeating a point (unless `tangled`).
fn bad_input(points: &[Point2], loops: &[Vec<u32>], tangled: bool) -> Option<CdtError> {
    let n = points.len() as u32;
    let mut err = None;
    let mut i = 0;
    while err.is_none() && i < points.len() {
        let p = points[i];
        if !(p.x.is_finite() && p.y.is_finite()) {
            err = Some(CdtError::NonFinite { point: i as u32 });
        }
        i += 1;
    }
    let mut li = 0;
    while err.is_none() && li < loops.len() {
        let lp = &loops[li];
        if lp.len() < 3 && !tangled {
            err = Some(CdtError::BadLoop { loop_index: li });
        }
        let mut k = 0;
        while err.is_none() && k < lp.len() {
            let next = if k + 1 == lp.len() { lp[0] } else { lp[k + 1] };
            if lp[k] >= n {
                err = Some(CdtError::BadIndex { index: lp[k] });
            } else if lp[k] == next && !tangled {
                err = Some(CdtError::BadLoop { loop_index: li });
            }
            k += 1;
        }
        li += 1;
    }
    err
}

/// The point a tangled boundary's second point in one place stands for: the first
/// (`same_as`, sorted: each a point above, the one it stands for below).
fn one(same_as: &[u64], v: u32) -> u32 {
    let (mut l, mut r) = (0, same_as.len());
    while l < r {
        let m = l + (r - l) / 2;
        if ((same_as[m] >> 32) as u32) < v {
            l = m + 1;
        } else {
            r = m;
        }
    }
    if l < same_as.len() && (same_as[l] >> 32) as u32 == v {
        (same_as[l] & 0xffff_ffff) as u32
    } else {
        v
    }
}

/// The point budget's error: unsettled after `rounds` rounds, with the points it has.
#[allow(clippy::implicit_saturating_sub)]
fn unsettled(rounds: usize, mesh: &Mesh) -> CdtError {
    let n = mesh.pts.len() as u32;
    CdtError::Unsettled {
        rounds: rounds as u32,
        points: if n > SUPER { n - SUPER } else { 0 },
    }
}

impl Mesh {
    /// The input points, in BRIO order. A point where one already is: for a tangled boundary, a
    /// stand-in for that one (returned, sorted, as `one` reads them); otherwise an error.
    fn insert_points(
        &mut self,
        points: &[Point2],
        lo: Point2,
        size: f64,
        tangled: bool,
    ) -> Result<Vec<u64>, CdtError> {
        let mut same_as: Vec<u64> = Vec::new();
        let order = brio_order(points, lo, size);
        let mut err = None;
        let mut k = 0;
        while err.is_none() && k < order.len() {
            let i = order[k];
            let v = SUPER + i as u32;
            match self.locate(points[i]) {
                None => err = Some(CdtError::Internal("point location failed")),
                Some(Location::Vertex(w)) => {
                    if tangled {
                        same_as.push(u64::from(v) << 32 | u64::from(w));
                    } else {
                        err = Some(CdtError::DuplicatePoint {
                            a: w.wrapping_sub(SUPER),
                            b: i as u32,
                        });
                    }
                }
                Some(loc) => self.insert(v, loc),
            }
            k += 1;
        }
        match err {
            Some(e) => Err(e),
            None => {
                let mut tmp = Vec::new();
                sort_keys(&mut same_as, &mut tmp);
                Ok(same_as)
            }
        }
    }

    /// Each loop's edges, as constraints, in order.
    fn insert_loops(&mut self, loops: &[Vec<u32>]) -> Result<(), CdtError> {
        let mut err = None;
        let mut l = 0;
        while err.is_none() && l < loops.len() {
            let lp = &loops[l];
            if forge_geom::budget::over() {
                err = Some(CdtError::Internal("the boundary would take too long"));
            }
            let mut k = 0;
            while err.is_none() && k < lp.len() {
                if k % 256 == 255 && forge_geom::budget::over() {
                    err = Some(CdtError::Internal("the boundary would take too long"));
                } else {
                    let a = lp[k] + SUPER;
                    let b = if k + 1 == lp.len() { lp[0] } else { lp[k + 1] } + SUPER;
                    if let Err(e) = self.insert_constraint(a, b, SUPER) {
                        err = Some(e);
                    }
                }
                k += 1;
            }
            l += 1;
        }
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Which triangles are inside: an even-odd flood fill from the super triangle's corner,
    /// across constraint edges.
    fn flood(&mut self) {
        let nt = self.tris.len();
        let mut parity: Vec<u8> = vec![2; nt];
        let seed = self.vtri[0];
        parity[seed as usize] = 0;
        let mut queue: Vec<u32> = vec![seed];
        let mut head = 0;
        while head < queue.len() {
            let t = queue[head];
            head += 1;
            let tt = self.tris[t as usize];
            let pt = parity[t as usize];
            let mut i = 0;
            while i < 3 {
                let u = tt.n[i];
                if u != NONE && parity[u as usize] == 2 {
                    parity[u as usize] = pt ^ u8::from(tt.c[i]);
                    queue.push(u);
                }
                i += 1;
            }
        }
        self.inside = Vec::with_capacity(nt);
        let mut t = 0;
        while t < nt {
            self.inside.push(parity[t] == 1);
            t += 1;
        }
    }

    /// Then the points the caller asks for, round after round, into the same triangulation; each
    /// round looks only at the triangles the last one made (each made triangle holds a new
    /// point). With whether it settled.
    #[allow(clippy::too_many_arguments)]
    fn refine<R: Refine + ?Sized>(
        &mut self,
        refine: &R,
        given: usize,
        lo: Point2,
        size: f64,
        eps: f64,
        rounds: usize,
        as_far: bool,
    ) -> Result<bool, CdtError> {
        let mut mids: Vec<Point2> = Vec::new();
        let mut keys: Vec<u64> = Vec::new();
        let mut tmp: Vec<u64> = Vec::new();
        let mut fresh_from = SUPER;
        let mut seen = vec![false; self.tris.len()];
        let mut split = vec![false; self.tris.len()];
        let mut look: Vec<u64> = Vec::with_capacity(self.tris.len());
        let mut t = 0;
        while t < self.tris.len() {
            look.push(t as u64);
            t += 1;
        }
        let triangles = refine.triangles();
        // a cascade that never ends stops at a number of rounds (each halves what it splits:
        // two hundred halvings are past any double's precision), one that only grows at a budget
        // of points: a quarter of a million, or four for each point given, and past that what
        // the chord asks for (`Refine::need`, asked then), four times over, up to `CEILING`;
        // either fails, typed (past the ceiling, as `TooFine`)
        let mut budget = if 4 * given > 1 << 18 { 4 * given } else { 1 << 18 };
        let mut asked = false;
        let mut need: Option<Need> = None;
        let mut err: Option<CdtError> = None;
        let mut settled = false;
        let mut stop = false;
        let mut round = 0;
        while !stop && err.is_none() && round < rounds {
            if self.pts.len() > budget && !asked {
                asked = true;
                need = refine.need();
                if let Some(n) = need {
                    // (an ideal mesh past the ceiling: no refinement fits under it)
                    if n.points > CEILING as u64 && !as_far {
                        err = Some(CdtError::TooFine {
                            chord: n.chord,
                            required: n.points,
                        });
                    } else {
                        let want = if n.points > u64::MAX / SAFETY {
                            u64::MAX
                        } else {
                            n.points * SAFETY
                        };
                        let want = if want < CEILING as u64 {
                            want as usize
                        } else {
                            CEILING
                        };
                        if want > budget {
                            budget = want;
                        }
                    }
                }
            }
            if err.is_none() && self.pts.len() > budget {
                if as_far {
                    stop = true;
                } else {
                    err = Some(match need {
                        Some(n) if budget >= CEILING => CdtError::TooFine {
                            chord: n.chord,
                            required: n.points,
                        },
                        _ => unsettled(round, self),
                    });
                }
            }
            if !stop && err.is_none() && forge_geom::budget::over() {
                err = Some(CdtError::Internal("refinement would take too long"));
            }
            if !stop && err.is_none() {
                mids.clear();
                let first_new = self.pts.len() as u32;
                seen.resize(self.tris.len(), false);
                split.resize(self.tris.len(), false);
                let mut k = 0;
                while k < look.len() {
                    seen[look[k] as usize] = true;
                    k += 1;
                }
                // a triangle only once none of its edges is split (its middle sags by a third
                // more than they do at most, and splitting them may be all it needs): its edges
                // are looked at from the lower of their two triangles, and `look` runs upwards
                let mut k = 0;
                while k < look.len() {
                    let t = look[k] as u32;
                    self.asks(refine, t, fresh_from, triangles, &seen, &mut split, &mut mids);
                    k += 1;
                }
                let mut k = 0;
                while k < look.len() {
                    seen[look[k] as usize] = false;
                    split[look[k] as usize] = false;
                    k += 1;
                }
                if mids.is_empty() {
                    settled = true;
                    stop = true;
                } else {
                    // along a Hilbert curve, so each walk starts beside the point before (by
                    // key, then by place: a key below 2^32 and a place below 2^32, one u64)
                    let scale = 65535.0 / size;
                    keys.clear();
                    let mut k = 0;
                    while k < mids.len() {
                        let p = mids[k];
                        let h = hilbert(grid(p.x - lo.x, scale), grid(p.y - lo.y, scale));
                        keys.push(h << 32 | k as u64);
                        k += 1;
                    }
                    sort_keys(&mut keys, &mut tmp);
                    let mut k = 0;
                    while err.is_none() && k < keys.len() {
                        let p = mids[(keys[k] & 0xffff_ffff) as usize];
                        if let Err(e) = self.insert_interior(p, eps) {
                            err = Some(e);
                        }
                        k += 1;
                    }
                }
                if !stop && err.is_none() {
                    // the triangles round each new point (whose fan closes: it lies inside the
                    // region)
                    look.clear();
                    seen.resize(self.tris.len(), false);
                    let mut v = first_new;
                    while v < self.pts.len() as u32 {
                        self.fan_into(v, &mut seen, &mut look);
                        v += 1;
                    }
                    let mut k = 0;
                    while k < look.len() {
                        seen[look[k] as usize] = false;
                        k += 1;
                    }
                    sort_keys(&mut look, &mut tmp);
                    fresh_from = first_new;
                }
            }
            if !stop && err.is_none() {
                round += 1;
            }
        }
        if let Some(e) = err {
            return Err(e);
        }
        if !settled && !as_far {
            return Err(unsettled(rounds, self));
        }
        Ok(settled)
    }

    /// What `refine` asks of triangle t: the middles of its edges that must be split (each once,
    /// from the lower of its two triangles), then its centroid if no edge of it is split.
    #[allow(clippy::too_many_arguments)]
    fn asks<R: Refine + ?Sized>(
        &self,
        refine: &R,
        t: u32,
        fresh_from: u32,
        triangles: bool,
        seen: &[bool],
        split: &mut [bool],
        mids: &mut Vec<Point2>,
    ) {
        let tt = self.tris[t as usize];
        let [v0, v1, v2] = tt.v;
        if self.inside[t as usize]
            && !(v0 < SUPER || v1 < SUPER || v2 < SUPER)
            && !(v0 < fresh_from && v1 < fresh_from && v2 < fresh_from)
        {
            let mut i = 0;
            while i < 3 {
                let u = tt.n[i];
                let (va, vb) = (tt.v[nx(i)], tt.v[pv(i)]);
                if !(tt.c[i] || (u != NONE && u < t && seen[u as usize] && self.inside[u as usize]))
                    && !(va < fresh_from && vb < fresh_from)
                    && refine.edge(va - SUPER, vb - SUPER, self.p(va), self.p(vb))
                {
                    mids.push(self.p(va).lerp(self.p(vb), 0.5));
                    split[t as usize] = true;
                    if u != NONE && seen[u as usize] {
                        split[u as usize] = true;
                    }
                }
                i += 1;
            }
            if triangles && !split[t as usize] {
                let p = [self.p(v0), self.p(v1), self.p(v2)];
                if refine.triangle([v0 - SUPER, v1 - SUPER, v2 - SUPER], p) {
                    mids.push(Point2::new(
                        (p[0].x + p[1].x + p[2].x) / 3.0,
                        (p[0].y + p[1].y + p[2].y) / 3.0,
                    ));
                }
            }
        }
    }

    /// The triangles round vertex v not yet `seen`, onto `look` (and seen).
    fn fan_into(&self, v: u32, seen: &mut [bool], look: &mut Vec<u64>) {
        let start = self.vtri[v as usize];
        let mut t = start;
        let mut on = true;
        let mut k = 0;
        while on && k < self.tris.len() {
            if !seen[t as usize] {
                seen[t as usize] = true;
                look.push(u64::from(t));
            }
            let tt = self.tris[t as usize];
            match slot(tt.v, v) {
                None => on = false,
                Some(i) => {
                    t = tt.n[nx(i)];
                    on = t != start && t != NONE;
                }
            }
            k += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::dmath;
    use forge_math::rng::Rng;

    fn tangled(pts: &[(f64, f64)], loops: &[&[u32]]) -> f64 {
        let points: Vec<Point2> = pts.iter().map(|p| Point2::new(p.0, p.1)).collect();
        let loops: Vec<Vec<u32>> = loops.iter().map(|l| l.to_vec()).collect();
        let t = triangulate_within(&points, &loops, &[], None::<&dyn Refine>, true).unwrap();
        t.triangles
            .iter()
            .map(|v| {
                let [a, b, c] = v.map(|i| t.points[i as usize]);
                assert!(orient2d(a, b, c) > 0.0);
                0.5 * (b - a).cross(c - a)
            })
            .sum()
    }

    /// A boundary's cost counts against a file's budget: past it, a clean error.
    #[test]
    fn sort_keys_sorts() {
        let mut rng = Rng::new(7);
        let mut tmp = Vec::new();
        for n in [0, 1, 2, 15, 16, 17, 31, 32, 33, 100, 1000, 4097] {
            for dup in [false, true] {
                let mut v: Vec<u64> = (0..n)
                    .map(|_| if dup { rng.below(8) as u64 } else { rng.next_u64() })
                    .collect();
                let mut w = v.clone();
                w.sort_unstable();
                sort_keys(&mut v, &mut tmp);
                assert_eq!(v, w, "n {n}");
            }
        }
    }

    #[test]
    fn a_boundary_past_its_budget_fails_cleanly() {
        let n = 2048;
        let points: Vec<Point2> = (0..n)
            .map(|k| {
                let (s, c) = dmath::sin_cos(dmath::TAU * k as f64 / n as f64);
                Point2::new(c, s)
            })
            .collect();
        let lp: Vec<u32> = (0..n).collect();
        let got = {
            let _b = forge_geom::budget::scoped(0);
            triangulate_refined(&points, &[lp], &[], None::<&dyn Refine>)
        };
        assert!(matches!(got, Err(CdtError::Internal(_))), "{got:?}");
    }

    #[test]
    fn a_tangled_boundary_is_what_an_odd_number_of_chords_parts() {
        let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} {b}");
        // a bow tie: two triangles of a quarter each
        close(
            tangled(&[(0., 0.), (1., 1.), (1., 0.), (0., 1.)], &[&[0, 1, 2, 3]]),
            0.5,
        );
        // a square with a slit in and out
        close(
            tangled(
                &[(0., 0.), (2., 0.), (2., 2.), (1., 2.), (1., 1.), (0., 2.)],
                &[&[0, 1, 2, 3, 4, 3, 5]],
            ),
            4.0,
        );
        // a loop there and back bounds nothing, whatever else there is
        let square = [(0., 0.), (1., 0.), (1., 1.), (0., 1.), (0.2, 0.2), (0.8, 0.8)];
        close(tangled(&square, &[&[0, 1, 2, 3], &[4, 5]]), 1.0);
        // a corner of a hole on the boundary, and a chord through a point
        close(
            tangled(
                &[
                    (0., 0.),
                    (2., 0.),
                    (2., 2.),
                    (0., 2.),
                    (1., 0.),
                    (1.5, 1.),
                    (0.5, 1.),
                ],
                &[&[0, 1, 2, 3], &[4, 5, 6]],
            ),
            3.5,
        );
        // two points in one place
        close(
            tangled(
                &[(0., 0.), (1., 0.), (1., 1.), (1., 1.), (0., 1.)],
                &[&[0, 1, 2, 3, 4]],
            ),
            1.0,
        );
        // a hole out past the boundary, overlapped by half
        close(
            tangled(
                &[
                    (0., 0.),
                    (2., 0.),
                    (2., 2.),
                    (0., 2.),
                    (1., 0.5),
                    (3., 0.5),
                    (3., 1.5),
                    (1., 1.5),
                ],
                &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
            ),
            4.0 - 1.0 + 1.0,
        );
    }

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

    struct Asks(bool);
    impl Refine for Asks {
        fn edge(&self, _: u32, _: u32, _: Point2, _: Point2) -> bool {
            self.0
        }
        fn triangle(&self, _: [u32; 3], _: [Point2; 3]) -> bool {
            false
        }
        fn triangles(&self) -> bool {
            false
        }
    }

    #[test]
    fn zip_covers_a_polygon_once_or_declines() {
        let mut r = Rng::new(41);
        let mut zipped = 0;
        for round in 0..300 {
            // two wavy rails, far apart or nearly touching, so some zips must decline
            let (n, m) = (3 + r.below(40), 3 + r.below(40));
            let (amp, gap) = (r.range(0.0, 1.0), r.range(0.1, 2.0));
            let rail = |i: usize, n: usize, y: f64, k: f64| {
                let x = 10.0 * i as f64 / (n - 1) as f64;
                Point2::new(x, y + amp * dmath::sin_cos(k * x).0)
            };
            let k = (r.range(0.2, 3.0), r.range(0.2, 3.0));
            let mut pts: Vec<Point2> = (0..n).map(|i| rail(i, n, 0.0, k.0)).collect();
            pts.extend((0..m).rev().map(|i| rail(i, m, gap, k.1)));
            let lp: Vec<u32> = (0..pts.len() as u32).collect();
            // rails that cross are no polygon: the Delaunay path refuses those too
            let Ok(cdt) = triangulate(&pts, std::slice::from_ref(&lp), &[]) else {
                continue;
            };
            let Some(triangles) = zip(&pts, &lp, &Asks(false)) else {
                continue;
            };
            zipped += 1;
            assert_eq!(triangles.len(), cdt.triangles.len(), "round {round}");
            // all counter-clockwise and the polygon's area, each edge once each way or on the
            // boundary
            let mut area = 0.0;
            let mut edges = std::collections::HashSet::new();
            for t in &triangles {
                let [a, b, c] = t.map(|i| pts[i as usize]);
                assert!(orient2d(a, b, c) > 0.0, "round {round}");
                area += 0.5 * (b - a).cross(c - a);
                for k in 0..3 {
                    assert!(edges.insert((t[k], t[(k + 1) % 3])), "round {round}");
                }
            }
            let want = shoelace(&pts, &lp);
            assert!((area - want).abs() <= 1e-9 * want, "round {round}: {area} {want}");
            let on_loop = |a: u32, b: u32| (a + 1) % lp.len() as u32 == b;
            for &(a, b) in &edges {
                assert!(on_loop(a, b) || edges.contains(&(b, a)), "round {round}");
            }
            assert!(zip(&pts, &lp, &Asks(true)).is_none() || pts.len() == 3);
        }
        assert!(zipped > 50, "{zipped}");
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
    fn planar_face_from_the_torture_plate() {
        // a pocket floor with a short arc and a hole; the constraint walk once lost its edge
        let pts: Vec<Point2> = [
            (-13.147907446062607, -2.5149650614533767),
            (-0.08156110173683118, -2.514965061453423),
            (-0.04370783220592134, -1.8875417826545902),
            (0.07261642051883, -1.2698351297676056),
            (0.26557089467565465, -0.6716199413147774),
            (0.5321022010575742, -0.10236261504552899),
            (0.8679926410710936, 0.42892869214884466),
            (1.2679269492953997, 0.9138466129333672),
            (1.7255764043800383, 1.3447176110912267),
            (2.2336989772771303, 1.7147234106475286),
            (2.7842539320217625, 2.018008891010458),
            (3.368529065572016, 2.2497747407546775),
            (3.9772785732140035, 2.4063534038555576),
            (4.6008693578944, 2.4852671165709017),
            (5.229433468222707, 2.4852671165708884),
            (5.853024252903083, 2.4063534038555208),
            (6.4617737605450785, 2.2497747407546735),
            (7.0460488940953265, 2.018008891010424),
            (7.596603848839967, 1.7147234106475397),
            (8.104726421737054, 1.3447176110912809),
            (8.562375876821687, 0.9138466129334049),
            (8.962310185045993, 0.42892869214889684),
            (9.298200625059524, -0.10236261504551303),
            (9.564731931441441, -0.6716199413147304),
            (9.757686405598275, -1.2698351297675994),
            (9.874010658323023, -1.8875417826545844),
            (9.91186392785392, -2.5149650614534),
            (12.220945028588936, -2.514965061453402),
            (13.147907446062588, -2.514965061453415),
            (13.147907446062602, -0.9686298901924761),
            (13.147907446062607, 2.5149650614533767),
            (12.723825901115738, 2.5149650614533856),
            (12.604006266651902, 2.5149650614533874),
            (-13.147907446062588, 2.514965061453415),
            (-8.226878659744951, -2.155643866400904),
            (-8.576638408546751, -1.9050154615057284),
            (-8.874389984420485, -1.5943862962830786),
            (-9.109993814563431, -1.2343344743960558),
            (-9.275426693115708, -0.8371211288043909),
            (-9.365055001813134, -0.41627288358738923),
            (-9.375826556100261, 0.013878779015978004),
            (-9.307374543614943, 0.43868556004577597),
            (-9.162030015543186, 0.843681174073672),
            (-8.944742505466982, 1.2150739812025453),
            (-8.662911478934268, 1.5402166448223027),
            (-8.326134353534618, 1.8080368214889195),
            (-7.945879670353846, 2.0094142163009776),
            (-7.535096546562451, 2.1374911636036584),
            (-7.107773708761077, 2.1879061565680233),
            (-6.678463123673843, 2.1589423730747086),
            (-6.261784448374547, 2.0515861400323923),
            (-5.871927175399572, 1.8694933451976892),
            (-5.522167426597774, 1.6188649403025133),
            (-5.2244158507240375, 1.3082357750798634),
            (-4.9888120205810935, 0.9481839531928413),
            (-4.823379142028816, 0.5509706076011753),
            (-4.733750833331389, 0.1301223623841743),
            (-4.722979279044263, -0.3000293002191925),
            (-4.79143129152958, -0.7248360812489907),
            (-4.936775819601337, -1.129831695276887),
            (-5.154063329677542, -1.5012245024057602),
            (-5.435894356210256, -1.8263671660255174),
            (-5.772671481609905, -2.094187342692134),
            (-6.15292616479069, -2.2955647375041854),
            (-6.563709288582073, -2.423641684806874),
            (-6.991032126383447, -2.474056677771238),
            (-7.4203427114706795, -2.445092894277924),
            (-7.837021386769977, -2.3377366612356076),
        ]
        .iter()
        .map(|&(x, y)| Point2::new(x, y))
        .collect();
        let loops: Vec<Vec<u32>> = vec![(0..34).collect(), (34..68).collect()];
        let tr = triangulate(&pts, &loops, &[]).unwrap();
        let area: f64 = loops.iter().map(|l| shoelace(&pts, l)).sum();
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

    /// A refinement that never settles (every edge split, every round) stops at the point budget
    /// with a clean error, not after 200 rounds of doubling.
    /// A refinement past the budget its boundary gives it gets what its chord asks for, four times
    /// over, up to the ceiling; past the ceiling it is refused, typed, with the chord and the count.
    #[test]
    fn the_budget_is_what_the_chord_asks_for() {
        struct Asks(usize, u64);
        impl Refine for Asks {
            // every edge longer than 1/k of the square split: about k² triangles
            fn edge(&self, _: u32, _: u32, a: Point2, b: Point2) -> bool {
                a.distance(b) > 1.0 / self.0 as f64
            }
            fn triangle(&self, _: [u32; 3], _: [Point2; 3]) -> bool {
                false
            }
            fn triangles(&self) -> bool {
                false
            }
            fn need(&self) -> Option<Need> {
                Some(Need {
                    chord: 1e-3,
                    points: self.1,
                })
            }
        }
        // the unit square, its sides cut at 1/k (k = 256)
        let k = 256;
        let mut square = Vec::new();
        for (o, d) in [
            ((0.0, 0.0), (1.0, 0.0)),
            ((1.0, 0.0), (0.0, 1.0)),
            ((1.0, 1.0), (-1.0, 0.0)),
            ((0.0, 1.0), (0.0, -1.0)),
        ] {
            for i in 0..k {
                let t = i as f64 / k as f64;
                square.push(Point2::new(o.0 + d.0 * t, o.1 + d.1 * t));
            }
        }
        let lp = vec![(0..square.len() as u32).collect::<Vec<_>>()];
        // past 2^18 points, and given what it needs it settles
        let t = triangulate_within(&square, &lp, &[], Some(&Asks(k, 200_000)), false).unwrap();
        assert!(t.points.len() > 1 << 18, "{}", t.points.len());
        // what it asks for told too small: unsettled, as before
        let small = triangulate_within(&square, &lp, &[], Some(&Asks(k, 1000)), false);
        assert!(matches!(small, Err(CdtError::Unsettled { .. })), "{small:?}");
        // an ideal mesh past the ceiling: refused at once, with the chord and the count
        let fine = triangulate_within(&square, &lp, &[], Some(&Asks(k, 1 << 24)), false);
        assert_eq!(
            fine.map(|_| ()),
            Err(CdtError::TooFine {
                chord: 1e-3,
                required: 1 << 24
            })
        );
    }

    #[test]
    fn refinement_that_never_settles_fails() {
        struct Always;
        impl Refine for Always {
            fn edge(&self, _: u32, _: u32, _: Point2, _: Point2) -> bool {
                true
            }
            fn triangle(&self, _: [u32; 3], _: [Point2; 3]) -> bool {
                true
            }
            fn triangles(&self) -> bool {
                false
            }
        }
        let sq = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        // past the budget of points
        assert!(matches!(
            triangulate_refined(&sq, &[vec![0, 1, 2, 3]], &[], Some(&Always)),
            Err(CdtError::Unsettled { .. })
        ));
        // past the rounds, with points to spare: an error too, never a mesh that breaks the test
        let got = within(&sq, &[vec![0, 1, 2, 3]], &[], Some(&Always), false, 3, false);
        assert!(
            matches!(got, Err(CdtError::Unsettled { rounds: 3, .. })),
            "{got:?}"
        );
    }
}
