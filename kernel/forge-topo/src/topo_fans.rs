//! `check_fans`'s test, as `topo.rs`'s are (and translated with them): in a module of its own so
//! that adding it left the code of the other tests as it was (rustc places generic code by module).
//!
//! Round each vertex, coedge to coedge: from one leaving it, back to the one arriving that closes
//! its corner of the face, then across to that one's partner, which leaves it again; one cycle is
//! one fan (`Body::fans`, which the booleans use, is the same walk). Only the number of fans is
//! kept: the coedges walked, in order, are `seen`, so the fans' faces are read from it.

use crate::topo::Topo;

/// What `fans` reads besides: each coedge's sense, and the corners of a face at a vertex.
pub trait FanTopo: Topo {
    /// Whether coedge `c` runs along its edge.
    fn coedge_forward(&self, c: usize) -> bool;
    /// `Body::corners`: where face `f` meets vertex `v` more than once, each coedge leaving `v`
    /// with the arriving one that closes its corner (geometry: taken as given).
    fn corners(&self, f: usize, v: usize) -> Vec<(usize, usize)>;
}

/// Where `fans` puts what it finds.
pub trait FanSink {
    /// Vertex `v` has `n` fans, more than one.
    fn vertex_fans(&mut self, v: usize, n: usize);
}

/// No coedge (an edge with no other use).
const NONE: usize = usize::MAX;

#[inline]
fn start<T: FanTopo>(t: &T, c: usize) -> usize {
    let (a, b) = t.edge_ends(t.coedge_edge(c));
    if t.coedge_forward(c) { a } else { b }
}

#[inline]
fn stop<T: FanTopo>(t: &T, c: usize) -> usize {
    let (a, b) = t.edge_ends(t.coedge_edge(c));
    if t.coedge_forward(c) { b } else { a }
}

#[inline]
fn face_of<T: FanTopo>(t: &T, c: usize) -> usize {
    t.loop_face(t.coedge_loop(c))
}

/// The coedge before `c` in its loop (`c` one of its loop's coedges).
#[inline]
fn prev_of<T: FanTopo>(t: &T, c: usize) -> usize {
    let l = t.coedge_loop(c);
    let first = t.loop_first(l);
    if c == first {
        first + t.loop_len(l) - 1
    } else {
        c - 1
    }
}

/// The first use of `c`'s edge that is not `c`, or `NONE`.
#[inline]
fn partner<T: FanTopo>(t: &T, c: usize) -> usize {
    let e = t.coedge_edge(c);
    let n = t.edge_n_uses(e);
    let mut i = 0;
    let mut found = NONE;
    while found == NONE && i < n {
        let o = t.edge_use(e, i);
        if o != c {
            found = o;
        }
        i += 1;
    }
    found
}

/// Whether `x` is among the first `n` of `xs`.
#[inline]
fn contains(xs: &[usize], n: usize, x: usize) -> bool {
    let mut i = 0;
    while i < n {
        if xs[i] == x {
            return true;
        }
        i += 1;
    }
    false
}

/// The faces met, each once in the order met, with their corners at `v`: face `cf[k]`'s are
/// `cl[cs[k]..cs[k + 1]]`.
struct Closes {
    cf: Vec<usize>,
    cl: Vec<(usize, usize)>,
    cs: Vec<usize>,
}

/// The coedge that closes `c`'s corner: from the corners where they have `c`, else the one before
/// it in its loop.
fn before<T: FanTopo>(t: &T, closes: &Closes, by_corners: bool, c: usize) -> usize {
    let mut into = 0;
    let mut found = false;
    if by_corners {
        let f = face_of(t, c);
        let nf = closes.cf.len();
        let mut k = 0;
        while k < nf {
            if closes.cf[k] == f {
                let mut j = closes.cs[k];
                let end = closes.cs[k + 1];
                while !found && j < end {
                    let (x, y) = closes.cl[j];
                    if x == c {
                        into = y;
                        found = true;
                    }
                    j += 1;
                }
                k = nf;
            } else {
                k += 1;
            }
        }
    }
    if found { into } else { prev_of(t, c) }
}

/// The fans the coedges `items[lo..hi]` leaving a vertex lie in, counted; the coedges walked are
/// the first `sl` of `seen` (a buffer kept between vertices: its length is not the count), `sl`
/// returned with the count.
fn count_fans<T: FanTopo>(
    t: &T,
    items: &[usize],
    lo: usize,
    hi: usize,
    closes: &Closes,
    by_corners: bool,
    seen: &mut Vec<usize>,
) -> (usize, usize) {
    let mut count = 0;
    let mut sl = 0;
    let mut i = lo;
    while i < hi {
        let c0 = items[i];
        if !contains(seen, sl, c0) {
            count += 1;
            let mut c = c0;
            let mut go = true;
            while go && !contains(seen, sl, c) {
                if sl < seen.len() {
                    seen[sl] = c;
                } else {
                    seen.push(c);
                }
                sl += 1;
                let p = partner(t, before(t, closes, by_corners, c));
                if p == NONE {
                    go = false;
                } else {
                    c = p;
                }
            }
        }
        i += 1;
    }
    (count, sl)
}

/// The fans at a vertex, by loop order; where that leaves two or more, again with the corners of
/// each face in them, unless every face's corners are empty or they do not close each leaving
/// coedge with a distinct use, of its edge, arriving there.
fn n_fans<T: FanTopo>(t: &T, items: &[usize], lo: usize, hi: usize, seen: &mut Vec<usize>) -> usize {
    let mut closes = Closes { cf: Vec::new(), cl: Vec::new(), cs: Vec::new() };
    let (count, sl) = count_fans(t, items, lo, hi, &closes, false, seen);
    if count < 2 {
        return count;
    }
    let v = start(t, items[lo]);
    closes.cs.push(0);
    let mut any = false;
    let ns = sl;
    let mut k = 0;
    while k < ns {
        let f = face_of(t, seen[k]);
        if !contains(&closes.cf, closes.cf.len(), f) {
            closes.cf.push(f);
            let cr = t.corners(f, v);
            let m = cr.len();
            if m > 0 {
                any = true;
            }
            let mut j = 0;
            while j < m {
                closes.cl.push(cr[j]);
                j += 1;
            }
            closes.cs.push(closes.cl.len());
        }
        k += 1;
    }
    if !any {
        return count;
    }
    // the corners close each leaving coedge with a distinct use, of its edge, arriving at v
    let nc = t.n_coedges();
    let ne = t.n_edges();
    let mut into: Vec<usize> = Vec::new();
    let mut ok = true;
    let mut i = lo;
    while ok && i < hi {
        let a = before(t, &closes, true, items[i]);
        if a >= nc {
            ok = false;
        } else {
            let e = t.coedge_edge(a);
            if e >= ne || stop(t, a) != v {
                ok = false;
            } else {
                let mut used = false;
                let nu = t.edge_n_uses(e);
                let mut j = 0;
                while j < nu {
                    if t.edge_use(e, j) == a {
                        used = true;
                    }
                    j += 1;
                }
                if !used || contains(&into, into.len(), a) {
                    ok = false;
                } else {
                    into.push(a);
                }
            }
        }
        i += 1;
    }
    if !ok {
        return count;
    }
    count_fans(t, items, lo, hi, &closes, true, seen).0
}

/// `check_fans`: the vertices round which the faces lie in more than one fan, with how many.
pub fn fans<T: FanTopo, S: FanSink>(t: &T, out: &mut S) {
    let nv = t.n_vertices();
    let ne = t.n_edges();
    // the coedges leaving each vertex, edge by edge, each edge's uses in order: vertex `v`'s are
    // `items[off[v]..off[v + 1]]`
    let mut off: Vec<usize> = vec![0; nv + 1];
    let mut e = 0;
    while e < ne {
        let n = t.edge_n_uses(e);
        let mut i = 0;
        while i < n {
            let s = start(t, t.edge_use(e, i));
            off[s + 1] += 1;
            i += 1;
        }
        e += 1;
    }
    let mut v = 0;
    while v < nv {
        off[v + 1] += off[v];
        v += 1;
    }
    // where each vertex's next coedge goes
    let mut at: Vec<usize> = off.clone();
    let mut items: Vec<usize> = vec![0; off[nv]];
    let mut e = 0;
    while e < ne {
        let n = t.edge_n_uses(e);
        let mut i = 0;
        while i < n {
            let c = t.edge_use(e, i);
            let s = start(t, c);
            items[at[s]] = c;
            at[s] += 1;
            i += 1;
        }
        e += 1;
    }
    let mut seen: Vec<usize> = Vec::new();
    let mut v = 0;
    while v < nv {
        let n = n_fans(t, &items, off[v], off[v + 1], &mut seen);
        if n > 1 {
            out.vertex_fans(v, n);
        }
        v += 1;
    }
}
