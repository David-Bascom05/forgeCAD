//! The write's combinatorial part (decide-once-spec §3a, the stitch): the result's loops round
//! the region, from ids alone. The bridge decides every geometric fact once, exactly, and hands
//! it over as an id or a rank: the turn order at a point the boundary passes more than once,
//! each loop's sense and each hole's holder. What is here reads no point and no number: it pools
//! the boundary sides, cancels the ones a face uses twice or that lie on a wall, chains the rest
//! into loops, and gathers each outer loop with its holes.
//!
//! In the style of the rest of the crate (for Aeneas): plain loops, no closures or iterators, no
//! `Ordering`, no maps (sorted lists and binary search), no floats, no return inside a loop.

use crate::sort::sort_triples;
use crate::v2::Csr;

/// No id.
pub const NONE: u32 = u32::MAX;

/// A side of a core face's facet: from point `u` to point `v` along edge cell `e`, with the face
/// on its left about the face's outward normal; `wall`: the edge cell lies on a wall of the region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Side {
    pub face: u32,
    pub u: u32,
    pub v: u32,
    pub e: u32,
    pub wall: bool,
}

/// A directed boundary segment of the result, `a` to `b`, in group `grp` (a carrier class and an
/// outward sense, as one id), from input face `src`; `keep`: the whole of a kept edge of body 0
/// (NONE: none); `cf`: the core face it bounds (NONE: a near face's remainder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seg {
    pub a: u32,
    pub b: u32,
    pub grp: u32,
    pub src: u32,
    pub keep: u32,
    pub cf: u32,
}

/// Stage 1, the cancel: the sides of the core faces (`face_grp[f]`, `face_src[f]` per core face)
/// that bound the face, as segments in the order given. A side is dropped when its face uses its
/// edge cell twice (it is inside the face) or when its edge cell lies on a wall (the frontier,
/// where the region's outside takes over).
pub fn cancel(sides: &[Side], face_grp: &[u32], face_src: &[u32]) -> Vec<Seg> {
    let n = sides.len();
    // (face, edge cell, side) sorted: a run of equal (face, edge cell) is that face's uses of it
    let mut keys: Vec<(u32, u32, u32)> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        keys.push((sides[i].face, sides[i].e, i as u32));
        i += 1;
    }
    sort_triples(&mut keys);
    let mut once = vec![false; n];
    let mut k = 0;
    while k < n {
        let mut j = k + 1;
        while j < n && keys[j].0 == keys[k].0 && keys[j].1 == keys[k].1 {
            j += 1;
        }
        if j == k + 1 {
            once[keys[k].2 as usize] = true;
        }
        k = j;
    }
    let mut out: Vec<Seg> = Vec::new();
    let mut i = 0;
    while i < n {
        let s = sides[i];
        if once[i] && !s.wall {
            out.push(Seg {
                a: s.u,
                b: s.v,
                grp: face_grp[s.face as usize],
                src: face_src[s.face as usize],
                keep: NONE,
                cf: s.face,
            });
        }
        i += 1;
    }
    out
}

/// The turn order at the points a boundary passes more than once: row `r` of `ends` belongs to
/// (group, point) `keys[r]` (sorted, no repeats), and lists the segment ends there in their
/// clockwise order about the group's outward normal, each as `2 · segment` (it leaves the point)
/// or `2 · segment + 1` (it arrives), as the bridge decides it exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turns {
    pub keys: Vec<(u32, u32)>,
    pub ends: Csr,
}

/// The row of (group, point) in `t`, or NONE.
fn turn_row(t: &Turns, grp: u32, p: u32) -> u32 {
    let (mut lo, mut hi) = (0usize, t.keys.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (g, q) = t.keys[mid];
        if g < grp || (g == grp && q < p) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo < t.keys.len() && t.keys[lo] == (grp, p) { lo as u32 } else { NONE }
}

/// The segments by start point within their group: (grp, a, segment) sorted.
fn by_start(segs: &[Seg]) -> Vec<(u32, u32, u32)> {
    let mut v: Vec<(u32, u32, u32)> = Vec::with_capacity(segs.len());
    let mut i = 0;
    while i < segs.len() {
        v.push((segs[i].grp, segs[i].a, i as u32));
        i += 1;
    }
    sort_triples(&mut v);
    v
}

/// The first position in `starts` of (grp, a).
fn first_at(starts: &[(u32, u32, u32)], grp: u32, a: u32) -> usize {
    let (mut lo, mut hi) = (0usize, starts.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (g, q, _) = starts[mid];
        if g < grp || (g == grp && q < a) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The segment that follows `cur`: the one unused segment of its group leaving its end; where
/// several leave, the first of them clockwise from `cur`'s arrival in the turn order. NONE: none
/// leaves (or the turn order does not list the arrival).
fn next_seg(segs: &[Seg], starts: &[(u32, u32, u32)], used: &[bool], turns: &Turns, cur: u32) -> u32 {
    let (grp, at) = (segs[cur as usize].grp, segs[cur as usize].b);
    let lo = first_at(starts, grp, at);
    let mut count = 0u32;
    let mut only = NONE;
    let mut k = lo;
    while k < starts.len() && starts[k].0 == grp && starts[k].1 == at {
        if !used[starts[k].2 as usize] {
            count += 1;
            only = starts[k].2;
        }
        k += 1;
    }
    if count < 2 {
        return only;
    }
    let r = turn_row(turns, grp, at);
    if r == NONE {
        return NONE;
    }
    let (s, e) = (turns.ends.start[r as usize] as usize, turns.ends.start[r as usize + 1] as usize);
    let len = e - s;
    // the arrival's place in the row
    let mut pos = len;
    let mut i = 0;
    while i < len {
        if pos == len && turns.ends.ids[s + i] == 2 * cur + 1 {
            pos = i;
        }
        i += 1;
    }
    if pos == len {
        return NONE;
    }
    // the first leaving end after it, clockwise, of an unused segment
    let mut pick = NONE;
    let mut step = 1;
    while step < len {
        let x = turns.ends.ids[s + (pos + step) % len];
        if pick == NONE && x & 1 == 0 && !used[(x / 2) as usize] && segs[(x / 2) as usize].a == at {
            pick = x / 2;
        }
        step += 1;
    }
    pick
}

/// Stage 2, the chain: the segments as closed loops, each a list of segment ids in order. A loop
/// starts at the least unused segment and follows `next_seg` until it closes. None: a segment has
/// no successor or a loop does not close within all the segments (a ledger the bridge got wrong).
pub fn chain(segs: &[Seg], turns: &Turns) -> Option<Vec<Vec<u32>>> {
    let n = segs.len();
    let starts = by_start(segs);
    let mut used = vec![false; n];
    let mut loops: Vec<Vec<u32>> = Vec::new();
    let mut ok = true;
    let mut s0 = 0;
    while ok && s0 < n {
        if !used[s0] {
            used[s0] = true;
            let start = segs[s0].a;
            let mut lp: Vec<u32> = vec![s0 as u32];
            let mut cur = s0 as u32;
            let mut steps = 0;
            while ok && segs[cur as usize].b != start {
                let nx = next_seg(segs, &starts, &used, turns, cur);
                if nx == NONE || steps > n {
                    ok = false;
                } else {
                    used[nx as usize] = true;
                    lp.push(nx);
                    cur = nx;
                    steps += 1;
                }
            }
            if ok {
                loops.push(lp);
            }
        }
        s0 += 1;
    }
    if ok { Some(loops) } else { None }
}

/// A final face before its edges: its group, its least input face, and its loops (outer first),
/// each a loop index (the chained loops first, then the kept ones).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceOut {
    pub grp: u32,
    pub src: u32,
    pub loops: Vec<u32>,
}

/// Stage 3, the faces: per loop (the chained ones, then the kept ones) its group `grp[l]`, least
/// input face `src[l]`, whether it is outer `outer[l]`, and for a hole its holder `holder[l]` (an
/// outer loop of its group, decided by the bridge). Each outer loop makes a face, its holes after
/// it in loop order; the faces in order of their least input face, then of their outer loop. None:
/// a hole with no holder, or a holder that is not an outer loop.
pub fn faces(grp: &[u32], src: &[u32], outer: &[bool], holder: &[u32]) -> Option<Vec<FaceOut>> {
    let n = grp.len();
    // the face of each outer loop
    let mut face_of = vec![NONE; n];
    let mut out: Vec<FaceOut> = Vec::new();
    let mut l = 0;
    while l < n {
        if outer[l] {
            face_of[l] = out.len() as u32;
            out.push(FaceOut {
                grp: grp[l],
                src: src[l],
                loops: vec![l as u32],
            });
        }
        l += 1;
    }
    let mut ok = true;
    let mut l = 0;
    while l < n {
        if !outer[l] {
            let h = holder[l];
            if h == NONE || h as usize >= n || face_of[h as usize] == NONE || grp[h as usize] != grp[l] {
                ok = false;
            } else {
                let f = face_of[h as usize] as usize;
                out[f].loops.push(l as u32);
                if src[l] < out[f].src {
                    out[f].src = src[l];
                }
            }
        }
        l += 1;
    }
    if !ok {
        return None;
    }
    // (src, outer loop, face) sorted: the faces' order
    let mut keys: Vec<(u32, u32, u32)> = Vec::with_capacity(out.len());
    let mut f = 0;
    while f < out.len() {
        keys.push((out[f].src, out[f].loops[0], f as u32));
        f += 1;
    }
    sort_triples(&mut keys);
    let mut sorted: Vec<FaceOut> = Vec::with_capacity(out.len());
    let mut k = 0;
    while k < keys.len() {
        sorted.push(out[keys[k].2 as usize].clone());
        k += 1;
    }
    Some(sorted)
}

/// Stage 4a, the corners: of each point `0..npts`, whether it is a vertex of the result: three or
/// more final faces round it. `pf`: (point, final face) of every loop segment's start, sorted, no
/// repeats; `extra[p]`: the faces round `p` the loops do not show (body 0's far faces kept round
/// an input vertex, counted by the bridge), added where the loops show fewer than three.
pub fn corners(npts: usize, pf: &[(u32, u32)], extra: &[u32]) -> Vec<bool> {
    let mut round = vec![0u32; npts];
    let mut i = 0;
    while i < pf.len() {
        round[pf[i].0 as usize] += 1;
        i += 1;
    }
    let mut out = vec![false; npts];
    let mut p = 0;
    while p < npts {
        let mut n = round[p];
        if n < 3 {
            n += extra[p];
        }
        out[p] = n >= 3;
        p += 1;
    }
    out
}

/// A run of a loop: from point `a` to point `b`, the stretch between two cuts; `keep`: the whole
/// of a kept edge of body 0 it is (NONE: none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub a: u32,
    pub b: u32,
    pub keep: u32,
}

/// Stage 4b, the runs: each loop of `loops` (its segments' start points in order, and per
/// segment `keep`, the kept edge it is or NONE, parallel to `loops.ids`) cut at its corners
/// (`corner`) and either side of a kept edge. A loop's runs start at its first cut and keep the
/// loop's order; row `l` of the answer's `start` is loop `l`'s. None: a loop with no cut (a loop
/// with no corner and no kept edge, which the ledger does not make).
pub fn runs(loops: &Csr, keep: &[u32], corner: &[bool]) -> Option<(Vec<u32>, Vec<Run>)> {
    let nl = loops.start.len().saturating_sub(1);
    let mut start: Vec<u32> = Vec::with_capacity(nl + 1);
    let mut out: Vec<Run> = Vec::new();
    let mut ok = true;
    let mut l = 0;
    while ok && l < nl {
        start.push(out.len() as u32);
        let (s, e) = (loops.start[l] as usize, loops.start[l + 1] as usize);
        let n = e - s;
        // a cut before segment k: a corner at its start, or a kept edge on either side
        let mut first = n;
        let mut k = 0;
        while first == n && k < n {
            if cut_at(loops, keep, corner, s, n, k) {
                first = k;
            }
            k += 1;
        }
        if n == 0 || first == n {
            ok = false;
        } else {
            let mut j = 0;
            while j < n {
                let k = (first + j) % n;
                let b = loops.ids[s + (k + 1) % n];
                if j == 0 || cut_at(loops, keep, corner, s, n, k) {
                    out.push(Run {
                        a: loops.ids[s + k],
                        b,
                        keep: keep[s + k],
                    });
                } else {
                    let last = out.len() - 1;
                    out[last].b = b;
                }
                j += 1;
            }
        }
        l += 1;
    }
    start.push(out.len() as u32);
    if ok { Some((start, out)) } else { None }
}

/// Whether a loop (its segments `s..s + n` of `loops`) is cut before its segment `k`.
fn cut_at(loops: &Csr, keep: &[u32], corner: &[bool], s: usize, n: usize, k: usize) -> bool {
    corner[loops.ids[s + k] as usize] || keep[s + k] != NONE || keep[s + (k + n - 1) % n] != NONE
}

/// The edges of `edges`: each one's key (least end, other end), sorted, and its two (final face,
/// run).
pub type Edges = (Vec<(u32, u32)>, Vec<[(u32, u32); 2]>);

/// Stage 4c, the edges: the runs that are no kept edge (`runs`, row `l` of `start` loop `l`, of
/// final face `face_of[l]`) by their ends either way round: per edge its key (least end, other
/// end), sorted, and its two (final face, run). None: an edge with other than two runs (two runs
/// of one pair of ends in a face pair, or one alone: a ledger the bridge got wrong).
pub fn edges(start: &[u32], runs: &[Run], face_of: &[u32]) -> Option<Edges> {
    // (least end, other end, run) of every run not kept, sorted; the run's face by its loop
    let mut by_ends: Vec<(u32, u32, u32)> = Vec::new();
    let mut run_face = vec![NONE; runs.len()];
    let nl = start.len().saturating_sub(1);
    let mut l = 0;
    while l < nl {
        let mut r = start[l] as usize;
        while r < start[l + 1] as usize {
            run_face[r] = face_of[l];
            if runs[r].keep == NONE {
                let (a, b) = (runs[r].a, runs[r].b);
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                by_ends.push((lo, hi, r as u32));
            }
            r += 1;
        }
        l += 1;
    }
    sort_triples(&mut by_ends);
    let mut keys: Vec<(u32, u32)> = Vec::new();
    let mut pairs: Vec<[(u32, u32); 2]> = Vec::new();
    let mut ok = true;
    let mut k = 0;
    while ok && k < by_ends.len() {
        let mut j = k + 1;
        while j < by_ends.len() && by_ends[j].0 == by_ends[k].0 && by_ends[j].1 == by_ends[k].1 {
            j += 1;
        }
        if j - k != 2 {
            ok = false;
        } else {
            let (r0, r1) = (by_ends[k].2, by_ends[k + 1].2);
            keys.push((by_ends[k].0, by_ends[k].1));
            pairs.push([(run_face[r0 as usize], r0), (run_face[r1 as usize], r1)]);
        }
        k = j;
    }
    if ok { Some((keys, pairs)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(face: u32, u: u32, v: u32, e: u32) -> Side {
        Side {
            face,
            u,
            v,
            e,
            wall: false,
        }
    }

    #[test]
    fn two_facets_one_face_cancel_their_shared_side_and_chain_to_one_loop() {
        // a unit square split by its diagonal into two triangles of one face: 0 1 2 and 0 2 3
        let sides = [
            side(0, 0, 1, 10),
            side(0, 1, 2, 11),
            side(0, 2, 0, 12),
            side(0, 0, 2, 12),
            side(0, 2, 3, 13),
            side(0, 3, 0, 14),
        ];
        let segs = cancel(&sides, &[7], &[3]);
        assert_eq!(segs.len(), 4);
        assert!(segs.iter().all(|s| s.grp == 7 && s.src == 3 && s.cf == 0 && s.keep == NONE));
        let loops = chain(&segs, &Turns::default()).expect("closes");
        assert_eq!(loops, vec![vec![0, 1, 2, 3]]);
        let fs = faces(&[7], &[3], &[true], &[NONE]).expect("one face");
        assert_eq!(fs, vec![FaceOut { grp: 7, src: 3, loops: vec![0] }]);
    }

    #[test]
    fn a_wall_side_is_dropped_and_an_open_chain_is_refused() {
        let mut sides = vec![side(0, 0, 1, 10), side(0, 1, 2, 11), side(0, 2, 0, 12)];
        sides[1].wall = true;
        let segs = cancel(&sides, &[0], &[0]);
        assert_eq!(segs.len(), 2);
        assert_eq!(chain(&segs, &Turns::default()), None);
    }

    #[test]
    fn at_a_point_passed_twice_the_turn_order_picks_the_way() {
        // two squares touching at point 0, one boundary: 0 1 2 3 0 and 0 4 5 6 0, both through 0
        let mk = |a: u32, b: u32| Seg {
            a,
            b,
            grp: 0,
            src: 0,
            keep: NONE,
            cf: NONE,
        };
        // the loop starts at point 1, so it meets point 0 mid-way: 1 2 3 0, then 0 4 5 6 0 or 0 1
        let segs = vec![mk(1, 2), mk(2, 3), mk(3, 0), mk(0, 1), mk(0, 4), mk(4, 5), mk(5, 6), mk(6, 0)];
        // at point 0, clockwise: the arrival of 2, the way out by 4, the arrival of 7, the way out
        // by 3
        let turns = Turns {
            keys: vec![(0, 0)],
            ends: Csr {
                start: vec![0, 4],
                ids: vec![2 * 2 + 1, 2 * 4, 2 * 7 + 1, 2 * 3],
            },
        };
        let loops = chain(&segs, &turns).expect("closes");
        // arriving at 0 by 2, the first way out clockwise is 4's; back at 0 by 7, only 3 is left
        assert_eq!(loops, vec![vec![0, 1, 2, 4, 5, 6, 7, 3]]);
        // the other turn order takes the short way first
        let turns = Turns {
            keys: vec![(0, 0)],
            ends: Csr {
                start: vec![0, 4],
                ids: vec![2 * 2 + 1, 2 * 3, 2 * 7 + 1, 2 * 4],
            },
        };
        assert_eq!(chain(&segs, &turns).expect("closes"), vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]]);
    }

    #[test]
    fn corners_count_the_faces_round_a_point_and_the_ones_the_bridge_adds() {
        // point 0 has faces 0 1 2; point 1 faces 0 1, and one more the loops do not show; 2: one
        let pf = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (2, 0)];
        assert_eq!(corners(3, &pf, &[0, 1, 1]), vec![true, true, false]);
        assert_eq!(corners(3, &pf, &[0, 0, 0]), vec![true, false, false]);
    }

    #[test]
    fn a_loop_is_cut_at_its_corners_and_beside_kept_edges_and_runs_pair_into_edges() {
        // two final faces sharing the run 2 -> 0 (a square 0 1 2 3 cut by its diagonal: face 0
        // the loop 0 1 2, face 1 the loop 2 3 0, as point ids); corners 0 and 2 only
        let loops = Csr {
            start: vec![0, 3, 6],
            ids: vec![0, 1, 2, 2, 3, 0],
        };
        let keep = [NONE; 6];
        let corner = [true, false, true, false];
        let (start, rs) = runs(&loops, &keep, &corner).expect("cut");
        assert_eq!(start, vec![0, 2, 4]);
        let r = |a, b| Run { a, b, keep: NONE };
        assert_eq!(rs, vec![r(0, 2), r(2, 0), r(2, 0), r(0, 2)]);
        // every run has ends 0 and 2: four runs on one pair of ends, refused
        assert_eq!(edges(&start, &rs, &[0, 1]), None);
        // a lens of two faces, each one run either way between 0 and 2: one edge, its two runs
        let (keys, pairs) = edges(&[0, 1, 2], &[r(0, 2), r(2, 0)], &[0, 1]).expect("two runs");
        assert_eq!(keys, vec![(0, 2)]);
        assert_eq!(pairs, vec![[(0, 0), (1, 1)]]);
    }

    #[test]
    fn a_kept_edge_is_a_run_of_its_own_and_not_an_edge_and_a_loop_with_no_cut_is_refused() {
        // a triangle 0 1 2 with its first side kept (edge 9), no corners
        let loops = Csr {
            start: vec![0, 3],
            ids: vec![0, 1, 2],
        };
        let (start, rs) = runs(&loops, &[9, NONE, NONE], &[false; 3]).expect("cut by the kept edge");
        assert_eq!(start, vec![0, 2]);
        assert_eq!(rs, vec![Run { a: 0, b: 1, keep: 9 }, Run { a: 1, b: 0, keep: NONE }]);
        assert_eq!(runs(&loops, &[NONE; 3], &[false; 3]), None);
    }

    #[test]
    fn holes_go_to_their_holder_and_faces_come_by_least_source() {
        // loops: 0 outer (src 5), 1 outer (src 2), 2 a hole of 0 (src 1)
        let fs = faces(&[0, 0, 0], &[5, 2, 1], &[true, true, false], &[NONE, NONE, 0]).expect("ok");
        assert_eq!(
            fs,
            vec![
                FaceOut { grp: 0, src: 1, loops: vec![0, 2] },
                FaceOut { grp: 0, src: 2, loops: vec![1] },
            ]
        );
        assert_eq!(faces(&[0, 0], &[0, 0], &[true, false], &[NONE, NONE]), None);
    }
}
