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
}

/// Where the tests put what they find (`check`'s problem list).
pub trait Sink {
    fn stray(&mut self, v: usize);
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
