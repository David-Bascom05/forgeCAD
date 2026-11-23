//! forge-sheet's combinatorial core (verify/sheet/Sheet/Classes.lean and Region.lean, L4 of
//! research/PROOF-STANDARD.md): what region.rs decides once its geometric facts are decided.
//!
//! * [`classes`]: vertex classes from the decided pairs (`near`, Region.lean), a union–find whose
//!   root is the least member, so each class is its least-named point when points come in name
//!   order (region.rs: loop, piece, fraction); classes numbered by their least members.
//!   Classes.lean `cls`/`rep`.
//! * [`chain`]: kept pieces (each from one vertex class to another) into loops; a vertex with
//!   unequal in and out is `Unchained`, one with more than one is a pinch (every pinch reported).
//!   region.rs `chain`'s combinatorics, decision for decision.
//! * [`owner`]: a hole's owner by Region.lean's rule: the first outer loop holding the hole's
//!   vertex such that every other outer loop holding it holds that loop's vertex (the
//!   innermost).
//!
//! Written for Aeneas (research/kernel-audit/decide-once/aeneas/README.md): no floats, no
//! closures, no iterators, no trait objects, no `break` and no `return` inside a loop (loops exit
//! through their guard), no `Ordering`, no struct holding a borrow, no `unsafe`.

#![forbid(unsafe_code)]

pub mod place;
mod union_find;

use union_find::Uf;

/// Each point's class: `class[i]` for point `i` of `0..n`, classes numbered in the order of their
/// least members. `pairs` are the decided `near` pairs (any order, either way round, repeats
/// allowed); every index must be below `n`.
pub fn classes(n: usize, pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut uf = Uf::new(n);
    let mut k = 0;
    while k < pairs.len() {
        let a = pairs[k].0 as usize;
        let b = pairs[k].1 as usize;
        uf.union(a, b);
        k += 1;
    }
    let mut number: Vec<u32> = Vec::with_capacity(n);
    let mut class: Vec<u32> = Vec::with_capacity(n);
    let mut next: u32 = 0;
    let mut i = 0;
    while i < n {
        let r = uf.find(i);
        if r == i {
            number.push(next);
            class.push(next);
            next += 1;
        } else {
            number.push(0);
            let c = number[r];
            class.push(c);
        }
        i += 1;
    }
    class
}

/// Why kept pieces do not chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainError {
    /// At `vertex`, `leaving` pieces leave and `arriving` arrive (they differ, or a walk did not
    /// close); `piece` is one of them.
    Unchained {
        vertex: u32,
        piece: u32,
        leaving: u32,
        arriving: u32,
    },
    /// Vertices with more than one piece in and out: each with its first two arriving pieces.
    Pinched { pinches: Vec<(u32, u32, u32)> },
}

/// The loops: `pieces[starts[k]..starts[k + 1]]` is loop `k`, in walking order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loops {
    pub pieces: Vec<u32>,
    pub starts: Vec<u32>,
}

/// Chain kept pieces into loops. Piece `e` runs from vertex `from[e]` to `to[e]` (each below
/// `nv`). region.rs `chain`: degrees first (the first vertex in order with unequal degrees is
/// `Unchained`; otherwise every vertex of degree past one is a pinch), then walks from each
/// unused piece in order, always along the first piece leaving the vertex reached.
pub fn chain(nv: usize, from: &[u32], to: &[u32]) -> Result<Loops, ChainError> {
    let m = from.len();
    // out_of and into as counts, first and second entries (the walk needs only the first leaving
    // piece; a pinch report the first two arriving)
    let mut n_out: Vec<u32> = vec![0; nv];
    let mut n_in: Vec<u32> = vec![0; nv];
    let mut first_out: Vec<u32> = vec![u32::MAX; nv];
    let mut first_in: Vec<u32> = vec![u32::MAX; nv];
    let mut second_in: Vec<u32> = vec![u32::MAX; nv];
    let mut e = 0;
    while e < m {
        let f = from[e] as usize;
        let t = to[e] as usize;
        if n_out[f] == 0 {
            first_out[f] = e as u32;
        }
        n_out[f] += 1;
        if n_in[t] == 0 {
            first_in[t] = e as u32;
        } else if n_in[t] == 1 {
            second_in[t] = e as u32;
        }
        n_in[t] += 1;
        e += 1;
    }
    let mut bad: Option<ChainError> = None;
    let mut pinches: Vec<(u32, u32, u32)> = Vec::new();
    let mut v = 0;
    while v < nv && bad.is_none() {
        if n_out[v] != n_in[v] {
            let piece = if n_out[v] > 0 { first_out[v] } else { first_in[v] };
            bad = Some(ChainError::Unchained {
                vertex: v as u32,
                piece,
                leaving: n_out[v],
                arriving: n_in[v],
            });
        } else if n_out[v] > 1 {
            pinches.push((v as u32, first_in[v], second_in[v]));
        }
        v += 1;
    }
    if let Some(b) = bad {
        return Err(b);
    }
    if !pinches.is_empty() {
        return Err(ChainError::Pinched { pinches });
    }
    let mut used: Vec<bool> = vec![false; m];
    let mut pieces: Vec<u32> = Vec::with_capacity(m);
    let mut starts: Vec<u32> = Vec::new();
    let mut start = 0;
    while start < m && bad.is_none() {
        if !used[start] {
            starts.push(pieces.len() as u32);
            let mut at = start;
            while !used[at] {
                used[at] = true;
                pieces.push(at as u32);
                at = first_out[to[at] as usize] as usize;
            }
            if at != start {
                bad = Some(ChainError::Unchained {
                    vertex: from[at],
                    piece: at as u32,
                    leaving: 1,
                    arriving: 1,
                });
            }
        }
        start += 1;
    }
    if let Some(b) = bad {
        return Err(b);
    }
    starts.push(pieces.len() as u32);
    Ok(Loops { pieces, starts })
}

/// A hole's owner among `k` outer loops: `holds[o]` is whether outer `o` holds the hole's vertex,
/// `nest[o * k + p]` whether outer `p` holds outer `o`'s vertex (each decided exactly, Region.lean
/// `inLoop`). The first `o` holding the hole's vertex such that every other `p` holding it holds
/// `o`'s vertex; `None` when there is none (region.rs `HoleOutside`).
pub fn owner(k: usize, holds: &[bool], nest: &[bool]) -> Option<u32> {
    let mut found: Option<u32> = None;
    let mut o = 0;
    while o < k && found.is_none() {
        if holds[o] {
            let mut ok = true;
            let mut p = 0;
            while p < k {
                if p != o && holds[p] && !nest[o * k + p] {
                    ok = false;
                }
                p += 1;
            }
            if ok {
                found = Some(o as u32);
            }
        }
        o += 1;
    }
    found
}
