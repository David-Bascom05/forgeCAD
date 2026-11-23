//! `check_edges`'s and `check_loops`'s tests, as `topo.rs`'s are (and translated with them): the
//! use counts and senses of edges, the faces with no loops, the empty loops and the loops that do
//! not close. The one test of geometry among them (an edge's curve degenerate) is the view's
//! `edge_degenerate`, which the translation takes as given. A module of its own, as
//! `topo_shell.rs`, so the code of the tests already translated is as it was.

use crate::topo::{Sink, Topo};

/// What `edges` and `loops` read besides.
pub trait EdgeTopo: Topo {
    /// Whether coedge `c` runs along its edge.
    fn coedge_forward(&self, c: usize) -> bool;
    /// Whether two-use edge `e`'s curve is degenerate over its range (check.rs; floats).
    fn edge_degenerate(&self, e: usize) -> bool;
}

/// `check_edges`: each live edge has two uses, in opposite senses, and a curve that is not
/// degenerate.
pub fn edges<T: EdgeTopo, S: Sink>(t: &T, out: &mut S) {
    let ne = t.n_edges();
    let mut e = 0;
    while e < ne {
        if t.edge_alive(e) {
            let n = t.edge_n_uses(e);
            if n != 2 {
                out.edge_uses(e, n);
            } else {
                if t.coedge_forward(t.edge_use(e, 0)) == t.coedge_forward(t.edge_use(e, 1)) {
                    out.edge_same_way(e);
                }
                if t.edge_degenerate(e) {
                    out.edge_degenerate(e);
                }
            }
        }
        e += 1;
    }
}

/// The vertex coedge `c` leaves from.
#[inline]
fn coedge_start<T: EdgeTopo>(t: &T, c: usize) -> usize {
    let (a, b) = t.edge_ends(t.coedge_edge(c));
    if t.coedge_forward(c) { a } else { b }
}

/// The vertex coedge `c` arrives at.
#[inline]
fn coedge_end<T: EdgeTopo>(t: &T, c: usize) -> usize {
    let (a, b) = t.edge_ends(t.coedge_edge(c));
    if t.coedge_forward(c) { b } else { a }
}

/// `check_loops`: each live face has a loop; each loop of a live face has a coedge, and each of
/// its coedges ends where the next (cyclically) starts.
pub fn loops<T: EdgeTopo, S: Sink>(t: &T, out: &mut S) {
    let nf = t.n_faces();
    let mut f = 0;
    while f < nf {
        if t.face_alive(f) && t.face_n_loops(f) == 0 {
            out.face_no_loops(f);
        }
        f += 1;
    }
    let nl = t.n_loops();
    let mut l = 0;
    while l < nl {
        if t.face_alive(t.loop_face(l)) {
            let first = t.loop_first(l);
            let n = t.loop_len(l);
            if n == 0 {
                out.loop_empty(l);
            } else {
                let mut i = 0;
                while i < n {
                    // (the next coedge, cyclically: `(i + 1) % n`)
                    let j = if i + 1 < n { i + 1 } else { 0 };
                    if coedge_end(t, first + i) != coedge_start(t, first + j) {
                        out.loop_broken(l, first + i);
                    }
                    i += 1;
                }
            }
        }
        l += 1;
    }
}
