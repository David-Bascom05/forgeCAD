//! The float-free part of `check`: the tests it makes of a body's topology alone, over a
//! read-only view of it (`Topo`), so Charon/Aeneas can take them to Lean as they ship
//! (verify/aeneas/forge-topo) and the translation be proven to refine
//! verify/forge-topo-check/lean's model. `Body` is the one instance; the functions are generic
//! only so the translation never sees a float, and monomorphise to what they were.
//!
//! Written in the subset Aeneas takes: indices and `while` loops, `Vec` push and index.

/// What `check`'s topological tests read of a body, by index.
pub trait Topo {
    fn n_vertices(&self) -> usize;
    fn n_edges(&self) -> usize;
    /// Whether edge `e` is live (`Edge::alive`).
    fn edge_alive(&self, e: usize) -> bool;
    /// Edge `e`'s start and end vertices.
    fn edge_ends(&self, e: usize) -> (usize, usize);
    /// How many uses (coedges) edge `e` lists, and its `i`th.
    fn edge_n_uses(&self, e: usize) -> usize;
    fn edge_use(&self, e: usize, i: usize) -> usize;
    fn n_coedges(&self) -> usize;
    /// Coedge `c`'s edge and loop.
    fn coedge_edge(&self, c: usize) -> usize;
    fn coedge_loop(&self, c: usize) -> usize;
    fn n_loops(&self) -> usize;
    /// Loop `l`'s first coedge and count (its coedges are `first..first + len`), and its face.
    fn loop_first(&self, l: usize) -> usize;
    fn loop_len(&self, l: usize) -> usize;
    fn loop_face(&self, l: usize) -> usize;
    fn n_faces(&self) -> usize;
    fn face_alive(&self, f: usize) -> bool;
    /// How many loops face `f` lists, and its `i`th.
    fn face_n_loops(&self, f: usize) -> usize;
    fn face_loop(&self, f: usize, i: usize) -> usize;
}

/// Where the tests put what they find (`check`'s problem list).
pub trait Sink {
    /// Vertex `v` is on no edge.
    fn stray(&mut self, v: usize);
    /// The links of face (`what` 0), edge (1) or loop (2) `id` are broken.
    fn back_link(&mut self, what: u8, id: usize);
    /// Shell `s` (or, `s` the shell count, a shell out of range) lists its faces wrongly.
    fn shell_faces(&mut self, s: usize);
    /// Shell `s`'s faces are not all joined through edges.
    fn shell_disconnected(&mut self, s: usize);
}

/// `check_strays`: the vertices no edge (live or left over) starts or ends at, in order.
pub fn strays<T: Topo, S: Sink>(t: &T, out: &mut S) {
    let nv = t.n_vertices();
    let mut used: Vec<bool> = vec![false; nv];
    let ne = t.n_edges();
    let mut e = 0;
    while e < ne {
        let (a, b) = t.edge_ends(e);
        if a < nv {
            used[a] = true;
        }
        if b < nv {
            used[b] = true;
        }
        e += 1;
    }
    let mut v = 0;
    while v < nv {
        if !used[v] {
            out.stray(v);
        }
        v += 1;
    }
}

/// Whether edge `e` lists coedge `c` among its uses.
fn uses<T: Topo>(t: &T, e: usize, c: usize) -> bool {
    let n = t.edge_n_uses(e);
    let mut i = 0;
    while i < n {
        if t.edge_use(e, i) == c {
            return true;
        }
        i += 1;
    }
    false
}

/// `check_links`: the links between entities (`WF` of verify/forge-topo-check/lean/TopoCheck/
/// Spec.lean), and that every id is in range: each live face's loops are its own; each loop of a
/// live face is listed by it once, and its coedges name it and are uses of their (live) edges;
/// each live edge's uses are coedges of it in such loops, and its ends are vertices. The faces at
/// fault, then the edges, then the loops.
pub fn links<T: Topo, S: Sink>(t: &T, out: &mut S) {
    let nv = t.n_vertices();
    let ne = t.n_edges();
    let nc = t.n_coedges();
    let nl = t.n_loops();
    let nf = t.n_faces();
    // how often each loop's face lists it, up to 2: only none, once and more are told apart
    let mut listed: Vec<u32> = vec![0; nl];
    let mut f = 0;
    while f < nf {
        if t.face_alive(f) {
            let mut ok = true;
            let n = t.face_n_loops(f);
            let mut i = 0;
            while i < n {
                let l = t.face_loop(f, i);
                if l >= nl || t.loop_face(l) != f {
                    ok = false;
                } else if listed[l] < 2 {
                    listed[l] += 1;
                }
                i += 1;
            }
            if !ok {
                out.back_link(0, f);
            }
        }
        f += 1;
    }
    let mut e = 0;
    while e < ne {
        if t.edge_alive(e) {
            let (a, b) = t.edge_ends(e);
            let mut ok = a < nv && b < nv;
            let n = t.edge_n_uses(e);
            let mut i = 0;
            while ok && i < n {
                let c = t.edge_use(e, i);
                ok = c < nc && t.coedge_edge(c) == e && {
                    let l = t.coedge_loop(c);
                    l < nl && listed[l] == 1 && {
                        let first = t.loop_first(l);
                        first <= c && c - first < t.loop_len(l)
                    }
                };
                i += 1;
            }
            if !ok {
                out.back_link(1, e);
            }
        }
        e += 1;
    }
    let mut l = 0;
    while l < nl {
        let f = t.loop_face(l);
        // left over from an in-place edit (a dead face's, or one `remove_loop` took off): no part
        // of the body, left for `compact`
        let skip = f < nf && (!t.face_alive(f) || listed[l] == 0);
        if !skip {
            let first = t.loop_first(l);
            let len = t.loop_len(l);
            // (its coedges in range first, so `first + i` cannot overflow)
            let mut ok = f < nf && listed[l] == 1 && (len == 0 || (first < nc && len <= nc - first));
            let mut i = 0;
            while ok && i < len {
                let c = first + i;
                ok = t.coedge_loop(c) == l && {
                    let e = t.coedge_edge(c);
                    e < ne && t.edge_alive(e) && uses(t, e, c)
                };
                i += 1;
            }
            if !ok {
                out.back_link(2, l);
            }
        }
        l += 1;
    }
}
